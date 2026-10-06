//! The iced application: the `WinampPlayer` app struct, its `Message` event
//! type, and the `update`/`view` loop `init_ui` hands to iced. Widget
//! construction lives in the `views` submodule (browse lists, Now Playing bar,
//! transport controls) and the Previous/Next stepping arithmetic in
//! `transport`; this module wires those to the shared `AppState` and the
//! `AppleMusicService` seam.

use iced::{Element, Task, widget::Column};
use std::{future::Future, sync::Arc};
use tokio::sync::Mutex;

mod transport;
mod views;

use crate::{
    apple_music::AppleMusicService,
    library::{Album, Artist, Song},
    state::{self, AppState},
};

/// Runs the UI, blocking until the window is closed.
///
/// `main` must call this from a plain (non-async) context: iced drives its
/// event loop synchronously on the calling thread, and `update`/`view` use
/// `blocking_lock` on the shared state, which panics inside a runtime.
pub fn init_ui(state: Arc<Mutex<AppState>>) -> iced::Result {
    iced::application(move || boot(state.clone()), update, view)
        .title("nostalgia")
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    PlayPause,
    Stop,
    VolumeChange(f32),
    NextTrack,
    PreviousTrack,
    TrackSelected(String),
    ArtistSelected(String),
    AlbumSelected(String),
    Back,
    LoadArtists,
    ArtistsLoaded(Vec<Artist>),
    AlbumsLoaded(Vec<Album>),
    SongsLoaded(Vec<Song>),
    TrackPlayed,
}

struct WinampPlayer {
    state: Arc<Mutex<AppState>>,
    apple_music_service: AppleMusicService,
    current_view: CurrentView,
    artists: Vec<Artist>,
    albums: Vec<Album>,
    songs: Vec<Song>,
    /// Every song the player has loaded across all browsed albums, so the Now
    /// Playing bar can still name the playing track after the user browses to
    /// a different album (whose list replaces `songs`).
    known_songs: Vec<Song>,
}

/// Which browse screen is showing. Payload-free: the albums and songs the
/// view renders come from the loaded `albums`/`songs` buffers, so carrying
/// the selected ids here (as earlier versions did) only duplicated
/// state nothing read.
enum CurrentView {
    Artists,
    Albums,
    Songs,
}

/// The initial player over the given shared state: nothing selected, no
/// lists loaded. Both `boot` and the tests start a player this way, so the
/// starting shape lives here instead of being repeated at each site — a new
/// field has only one spot to get its startup value.
impl WinampPlayer {
    fn new(state: Arc<Mutex<AppState>>) -> Self {
        let service = AppleMusicService::new(state.clone());
        Self {
            state,
            apple_music_service: service,
            current_view: CurrentView::Artists,
            artists: Vec::new(),
            albums: Vec::new(),
            songs: Vec::new(),
            known_songs: Vec::new(),
        }
    }

    /// The Now Playing bar's label for `current_track`, resolved against the
    /// accumulated [`Self::known_songs`] rather than the currently-browsed
    /// album's `songs`: `view` renders the bar from this, so the bar keeps
    /// naming a playing track even after a browse to another album replaced
    /// `songs`. The browse-away regression test asserts this same path.
    fn now_playing_label(&self, current_track: Option<&str>) -> String {
        views::now_playing_label(&self.known_songs, current_track)
    }
}

fn boot(state: Arc<Mutex<AppState>>) -> (WinampPlayer, Task<Message>) {
    (WinampPlayer::new(state), Task::done(Message::LoadArtists))
}

/// Maps a library-fetch `Result` to its matching `*Loaded` message: the
/// fetched items on success, an empty list on error — after handing the
/// error to `report_error`, so a failed fetch is never dropped silently.
/// Shared by the artists, albums, and songs load paths so the error fallback
/// (and its reporting) stays identical in all three. `report_error` is
/// injected rather than hardcoded so the reporting contract is testable
/// without capturing stderr.
fn loaded_or_empty<T, E>(
    result: Result<Vec<T>, E>,
    loaded: impl Fn(Vec<T>) -> Message,
    report_error: impl FnOnce(&E),
) -> Message {
    match result {
        Ok(items) => loaded(items),
        Err(err) => {
            report_error(&err);
            loaded(Vec::new())
        }
    }
}

/// Maps a playback `Result` to the `TrackPlayed` completion message, handing
/// any error to `report_error` first so a failed play is never dropped
/// silently — a backend that rejects a track would otherwise look like the
/// button did nothing. The playback twin of [`loaded_or_empty`], which
/// cannot serve here: it maps `Result<Vec<T>, _>` onto a `*Loaded` message,
/// while a play has no payload to load, only a completion. `report_error`
/// is injected rather than hardcoded so the reporting contract is testable
/// without capturing stderr.
fn played_or_reported<E>(result: Result<(), E>, report_error: impl FnOnce(&E)) -> Message {
    if let Err(err) = result {
        report_error(&err);
    }
    Message::TrackPlayed
}

/// Stores a freshly fetched list into the player's matching buffer, with no
/// further work. The `ArtistsLoaded` and `AlbumsLoaded` update arms both
/// just store; `SongsLoaded` folds the new songs into `known_songs` first
/// (see [`store_songs`]) and then ends in this same store. The
/// store-and-noop shape lives here once instead of in each arm.
fn store_loaded<T>(buffer: &mut Vec<T>, items: Vec<T>) -> Task<Message> {
    *buffer = items;
    Task::none()
}

/// Stores a freshly fetched song list into the player's current-album buffer
/// and folds it into the accumulated [`WinampPlayer::known_songs`], so a later
/// browse to a different album (which replaces `songs`) can't lose the title
/// of the playing track. Only `SongsLoaded` needs the extra fold — artists
/// and albums never appear in the Now Playing bar.
fn store_songs(player: &mut WinampPlayer, songs: Vec<Song>) -> Task<Message> {
    for song in &songs {
        if !player.known_songs.iter().any(|known| known.id == song.id) {
            player.known_songs.push(song.clone());
        }
    }
    store_loaded(&mut player.songs, songs)
}

/// Formats the browse-fetch failure report: names the fetch that failed
/// (e.g. "loading albums for artist \"artist-1\""), states the empty-list
/// fallback, and includes the underlying error. The play path names the
/// offending track; this names the offending query, so a failed browse tells
/// the user which fetch failed and what it was fetching. Kept as a pure
/// function so the report contract is testable without capturing stderr.
fn fetch_failure_report<E: std::fmt::Debug>(context: &str, err: &E) -> String {
    format!("music-library fetch failed ({context}); showing an empty list: {err:?}")
}

/// Runs a library-fetch future through iced's runtime, mapping its `Result`
/// onto the matching `*Loaded` message (empty list on error, via
/// [`loaded_or_empty`], with the error reported to stderr via
/// [`fetch_failure_report`]). `context` names the fetch — "loading favorite
/// artists", "loading albums for artist \"artist-1\"", or "loading songs from
/// album \"album-1\"" — so a failed browse reports *which* query failed and
/// what it was fetching, not just that a fetch failed. Shared by the artists,
/// albums, and songs load arms so none of them repeats the
/// clone-the-service-then-`Task::perform` boilerplate.
fn fetch_into<T, E, Fut>(
    service: &AppleMusicService,
    context: String,
    fetch: impl FnOnce(AppleMusicService) -> Fut + Send + 'static,
    loaded: impl Fn(Vec<T>) -> Message + Send + 'static,
) -> Task<Message>
where
    T: Send + 'static,
    E: std::fmt::Debug + Send + 'static,
    Fut: Future<Output = Result<Vec<T>, E>> + Send + 'static,
{
    let service = service.clone();
    Task::perform(async move { fetch(service).await }, move |result| {
        loaded_or_empty(result, loaded, |err| {
            eprintln!("{}", fetch_failure_report(&context, err))
        })
    })
}

/// The task the Next/Previous buttons schedule: step the current track
/// through the player's loaded songs in the given direction — via the
/// `transport::next_track_id`/`previous_track_id` stepping function passed
/// in — and schedule the landed song as `TrackSelected`, or no task when
/// there is nothing to step through. Both transport update arms used to
/// repeat this lock-then-dispatch block; the direction comes in as a function
/// so the stepping arithmetic stays in `transport` and the wiring lives here
/// once.
fn step_track(
    player: &WinampPlayer,
    step: fn(&[Song], Option<&str>) -> Option<String>,
) -> Task<Message> {
    // The stepped song is computed under the state lock, borrowing the
    // current track directly. The earlier version cloned the `current_track`
    // `String` only to borrow it via `as_deref` — the same clone-to-borrow
    // the per-frame now-playing path used to do — so the lock now covers the
    // pure stepping scan (fast, and `step` never locks anything itself).
    let state = player.state.blocking_lock();
    match step(&player.songs, state.current_track.as_deref()) {
        Some(track_id) => Task::done(Message::TrackSelected(track_id)),
        None => Task::none(),
    }
}

/// Locks the shared playback state, applies `mutation` to it, and returns no
/// task. The Play/Pause, Stop, and VolumeChange arms all repeat the same
/// synchronous shared-state update — `blocking_lock`, one mutation, then
/// `Task::none()` — so the lock-and-noop shape lives here once and each arm
/// only names its mutation. Asynchronous work (fetches) goes through
/// [`fetch_into`] instead.
fn mutate_state(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    let mut state = player.state.blocking_lock();
    mutation(&mut state);
    Task::none()
}

fn update(player: &mut WinampPlayer, message: Message) -> Task<Message> {
    match message {
        Message::PlayPause => mutate_state(player, AppState::toggle_playing),
        // Stop is synchronous, exactly like PlayPause: the stub has no
        // playback position yet, so its only observable effect is the cleared
        // playing flag — identical to Pause today. The seam is the
        // distinction: once real playback lands, Stop also resets the track
        // position while Pause keeps it.
        Message::Stop => mutate_state(player, AppState::stop),
        Message::VolumeChange(volume) => {
            mutate_state(player, |state| state.volume = state::clamp_volume(volume))
        }
        Message::NextTrack => step_track(player, transport::next_track_id),
        Message::PreviousTrack => step_track(player, transport::previous_track_id),
        Message::TrackSelected(track_id) => {
            let service = player.apple_music_service.clone();
            let id_for_report = track_id.clone();
            Task::perform(
                async move { service.play_track(&track_id).await },
                move |result| {
                    played_or_reported(result, |err| {
                        eprintln!("failed to play track {id_for_report:?}: {err:?}")
                    })
                },
            )
        }
        Message::ArtistSelected(artist_id) => {
            player.current_view = CurrentView::Albums;
            fetch_into(
                &player.apple_music_service,
                format!("loading albums for artist {artist_id:?}"),
                move |service| async move { service.get_albums_by_artist(&artist_id).await },
                Message::AlbumsLoaded,
            )
        }
        Message::AlbumSelected(album_id) => {
            player.current_view = CurrentView::Songs;
            fetch_into(
                &player.apple_music_service,
                format!("loading songs from album {album_id:?}"),
                move |service| async move { service.get_songs_from_album(&album_id).await },
                Message::SongsLoaded,
            )
        }
        // The browse hierarchy is navigable both ways: ArtistSelected and
        // AlbumSelected step down (Artists → Albums → Songs), Back steps up
        // again. The Back button is only rendered below the artist list, so
        // this arm mostly fires where a level exists to leave; the explicit
        // top-level arm keeps the no-op in the same place as the steps.
        Message::Back => {
            player.current_view = match &player.current_view {
                CurrentView::Songs => CurrentView::Albums,
                CurrentView::Albums => CurrentView::Artists,
                CurrentView::Artists => CurrentView::Artists,
            };
            Task::none()
        }
        Message::LoadArtists => fetch_into(
            &player.apple_music_service,
            "loading favorite artists".to_string(),
            |service| async move { service.get_favorite_artists().await },
            Message::ArtistsLoaded,
        ),
        Message::ArtistsLoaded(artists) => store_loaded(&mut player.artists, artists),
        Message::AlbumsLoaded(albums) => store_loaded(&mut player.albums, albums),
        Message::SongsLoaded(songs) => store_songs(player, songs),
        Message::TrackPlayed => Task::none(),
    }
}

/// Assembles the app screen: the Now Playing bar and transport row — both
/// built in `views.rs` from the resolved title, playback state, and volume —
/// above the current browse list.
fn view(player: &WinampPlayer) -> Element<'_, Message> {
    // The now-playing title is resolved while the state lock is held, from a
    // borrowed `current_track` against the accumulated `known_songs` (see
    // [`WinampPlayer::now_playing_label`]) — the label outlives the lock, but
    // the owned `String` clone of the current track is not needed, so the
    // per-frame path allocates only the resolved label.
    let (now_playing, is_playing, volume) = {
        let state = player.state.blocking_lock();
        (
            player.now_playing_label(state.current_track.as_deref()),
            state.is_playing,
            state.volume,
        )
    };

    let main_content = match &player.current_view {
        CurrentView::Artists => views::view_artists(&player.artists),
        CurrentView::Albums => views::view_albums(&player.albums),
        CurrentView::Songs => views::view_songs(&player.songs),
    };

    let mut column = Column::new()
        .push(views::view_now_playing(now_playing))
        .push(views::view_transport_controls(is_playing, volume));
    // The Back button sits above the list it navigates and exists only where
    // the hierarchy has a level above to return to (see `views::can_go_back`).
    if views::can_go_back(&player.current_view) {
        column = column.push(views::view_back_button());
    }
    column.push(main_content).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{sample_album, sample_artist, sample_song, stepping_songs};

    /// A fresh player over its own shared state, so a test can inspect the
    /// same `AppState` the player mutates.
    fn test_player() -> (WinampPlayer, Arc<Mutex<AppState>>) {
        let state = Arc::new(Mutex::new(AppState::default()));
        (WinampPlayer::new(state.clone()), state)
    }

    /// Drives an iced `Task` to completion and hands its single `Output`
    /// action to `check`. `update` only schedules work as a `Task`, so a test
    /// that wants to observe the resulting message — `TrackPlayed` after
    /// playback, a `*Loaded` message after a fetch, a `TrackSelected` after a
    /// step — must run the task itself the way the iced runtime would. The
    /// playback, fetch, and (through `assert_track_selected`) stepping tests
    /// do exactly that, so the stream plumbing and the "exactly one `Output`"
    /// assertion live here once instead of at each call site.
    async fn drive_task(task: Task<Message>, what: &str, mut check: impl FnMut(Message)) {
        use futures::StreamExt;

        let mut stream =
            iced_runtime::task::into_stream(task).expect("task must schedule a stream");
        let action = stream
            .next()
            .await
            .expect("task must yield a completion message");
        match action {
            iced_runtime::Action::Output(message) => check(message),
            other => panic!("unexpected {what} task output: {other:?}"),
        }
    }

    /// Drives the task a browse arm schedules, feeds the fetched `*Loaded`
    /// message back through the update loop, and asserts the list lands in
    /// the player's matching buffer in library order. The three fetch tests
    /// — loading artists, albums by artist, and songs from album — all repeat
    /// this drive-then-feed-then-assert flow; only the message variant, the
    /// buffer, the id key, and the expected ids differ, so they come in as
    /// parameters and the flow lives here once.
    ///
    /// `clippy::too_many_arguments` is allowed: the eight parameters are the
    /// natural vocabulary of the three fetch tests — each one is supplied by
    /// every call site, and bundling them into a struct would only add a
    /// construction site per test — so the arity is the shape of the flow,
    /// not a readability smell.
    #[allow(clippy::too_many_arguments)]
    async fn drive_fetch_and_assert_loaded<T, Extract, Buffer, Key>(
        player: &mut WinampPlayer,
        task: Task<Message>,
        what: &str,
        extract: Extract,
        loaded: impl Fn(Vec<T>) -> Message,
        buffer: Buffer,
        key: Key,
        expected_ids: &[&str],
    ) where
        T: 'static,
        Extract: Fn(&Message) -> Option<Vec<T>>,
        Buffer: Fn(&mut WinampPlayer) -> &mut Vec<T>,
        Key: Fn(&T) -> &str,
    {
        let mut fetched: Option<Vec<T>> = None;
        drive_task(task, what, |message| {
            fetched = Some(
                extract(&message)
                    .unwrap_or_else(|| panic!("unexpected {what} task output: {message:?}")),
            );
        })
        .await;

        let items = fetched.expect("browse task must yield a *Loaded message");
        let _ = update(player, loaded(items));
        let ids: Vec<&str> = buffer(player).iter().map(key).collect();
        assert_eq!(ids, expected_ids);
    }

    // `Message::PlayPause` uses `blocking_lock`, which panics inside an async
    // runtime, so this stays a plain test (no `#[tokio::test]`).
    #[test]
    fn play_pause_toggles_is_playing() {
        let (mut player, state) = test_player();

        let _ = update(&mut player, Message::PlayPause);
        assert!(state.blocking_lock().is_playing);

        let _ = update(&mut player, Message::PlayPause);
        assert!(!state.blocking_lock().is_playing);
    }

    // `Message::Stop` uses `blocking_lock`, which panics inside an async
    // runtime, so this stays a plain test (no `#[tokio::test]`).
    #[test]
    fn stop_clears_is_playing() {
        let (mut player, state) = test_player();

        let _ = update(&mut player, Message::PlayPause);
        assert!(state.blocking_lock().is_playing);

        let _ = update(&mut player, Message::Stop);
        assert!(!state.blocking_lock().is_playing);
    }

    #[test]
    fn volume_change_clamps_value_before_storing() {
        let (mut player, state) = test_player();
        // Default volume is 0.5 (see `AppState::default`).
        assert_eq!(state.blocking_lock().volume, 0.5);

        // Out-of-range slider values are clamped by the update arm.
        let _ = update(&mut player, Message::VolumeChange(1.5));
        assert_eq!(state.blocking_lock().volume, 1.0);

        let _ = update(&mut player, Message::VolumeChange(-0.2));
        assert_eq!(state.blocking_lock().volume, 0.0);

        // An in-range value is stored as-is.
        let _ = update(&mut player, Message::VolumeChange(0.3));
        assert_eq!(state.blocking_lock().volume, 0.3);
    }

    #[test]
    fn artist_selected_flips_to_albums_view() {
        let (mut player, _state) = test_player();

        let _ = update(&mut player, Message::ArtistSelected("artist-1".to_string()));

        assert!(matches!(player.current_view, CurrentView::Albums));
    }

    #[test]
    fn album_selected_flips_to_songs_view() {
        let (mut player, _state) = test_player();

        let _ = update(&mut player, Message::AlbumSelected("album-3".to_string()));

        assert!(matches!(player.current_view, CurrentView::Songs));
    }

    // The browse hierarchy must be navigable back up (Songs → Albums →
    // Artists), not just down: without `Message::Back` a session is a one-way
    // corridor that ends stuck at the bottom of the hierarchy. These arms use
    // only field assignment (no `blocking_lock`), so they stay plain tests
    // like the view-flip tests above.
    #[test]
    fn back_from_albums_returns_to_artists() {
        let (mut player, _state) = test_player();

        let _ = update(&mut player, Message::ArtistSelected("artist-1".to_string()));

        assert!(matches!(player.current_view, CurrentView::Albums));

        let _ = update(&mut player, Message::Back);
        assert!(matches!(player.current_view, CurrentView::Artists));
    }

    #[test]
    fn back_from_songs_returns_to_albums() {
        let (mut player, _state) = test_player();

        let _ = update(&mut player, Message::ArtistSelected("artist-1".to_string()));
        let _ = update(&mut player, Message::AlbumSelected("album-1".to_string()));

        assert!(matches!(player.current_view, CurrentView::Songs));

        let _ = update(&mut player, Message::Back);
        assert!(matches!(player.current_view, CurrentView::Albums));
    }

    #[test]
    fn back_from_artists_is_a_noop() {
        let (mut player, _state) = test_player();
        assert!(matches!(player.current_view, CurrentView::Artists));

        let _ = update(&mut player, Message::Back);
        assert!(matches!(player.current_view, CurrentView::Artists));
    }

    // The browse arms do two things — flip the view and fetch the next
    // level's list — and the view-flip tests above stop at the first half.
    // These pin the fetch half: `ArtistSelected` must load the selected
    // artist's albums and `AlbumSelected` the selected album's songs, both
    // delivered as a `*Loaded` message that iced feeds back into the browse
    // buffer. A regression that fetched the wrong artist's albums (or the
    // wrong album's songs) would still flip the view, so the loaded contents
    // are asserted, not just the view.

    #[tokio::test]
    async fn artist_selected_fetches_the_artists_albums_into_the_player() {
        let (mut player, _state) = test_player();

        let task = update(&mut player, Message::ArtistSelected("artist-1".to_string()));
        drive_fetch_and_assert_loaded(
            &mut player,
            task,
            "load albums",
            |message| match message {
                Message::AlbumsLoaded(albums) => Some(albums.clone()),
                _ => None,
            },
            Message::AlbumsLoaded,
            |player| &mut player.albums,
            |album| album.id.as_str(),
            &["album-1", "album-2"],
        )
        .await;

        // The arm flips to the albums view before the fetched list is fed
        // back through the update loop.
        assert!(matches!(player.current_view, CurrentView::Albums));
    }

    #[tokio::test]
    async fn album_selected_fetches_the_albums_songs_into_the_player() {
        let (mut player, _state) = test_player();

        let task = update(&mut player, Message::AlbumSelected("album-3".to_string()));
        drive_fetch_and_assert_loaded(
            &mut player,
            task,
            "load songs",
            |message| match message {
                Message::SongsLoaded(songs) => Some(songs.clone()),
                _ => None,
            },
            Message::SongsLoaded,
            |player| &mut player.songs,
            |song| song.id.as_str(),
            &["song-5"],
        )
        .await;

        // As with the artist arm: the view flips to songs, and the fetched
        // list lands in the browse buffer in library order.
        assert!(matches!(player.current_view, CurrentView::Songs));
    }

    // Startup wiring: `boot` hands iced a fresh player plus the task that
    // loads the artist list, and `update`'s `LoadArtists` arm runs that fetch
    // into the player's `artists` buffer. A regression that stopped boot from
    // emitting the task — or pointed the arm at the wrong fetch — would open
    // the app with an empty browse list, so the wiring is pinned end to end.

    #[tokio::test]
    async fn boot_schedules_loading_the_artist_list() {
        let state = Arc::new(Mutex::new(AppState::default()));

        let (player, task) = boot(state);
        assert!(matches!(player.current_view, CurrentView::Artists));

        drive_task(task, "boot", |message| {
            assert!(matches!(message, Message::LoadArtists));
        })
        .await;
    }

    #[tokio::test]
    async fn load_artists_fetches_favorite_artists_into_the_player() {
        let (mut player, _state) = test_player();

        let task = update(&mut player, Message::LoadArtists);
        drive_fetch_and_assert_loaded(
            &mut player,
            task,
            "load artists",
            |message| match message {
                Message::ArtistsLoaded(artists) => Some(artists.clone()),
                _ => None,
            },
            Message::ArtistsLoaded,
            |player| &mut player.artists,
            |artist| artist.id.as_str(),
            &["artist-1", "artist-2", "artist-3"],
        )
        .await;
    }

    // `Message::TrackSelected` wraps playback in a `Task`; the arm itself only
    // schedules it, so the real behavior lives in the returned task. Drive that
    // task to completion (as the iced runtime would) and assert the shared
    // state it mutates through the service.
    #[tokio::test]
    async fn track_selected_starts_playback_of_the_selected_track() {
        let (mut player, state) = test_player();

        let task = update(&mut player, Message::TrackSelected("song-1".to_string()));
        drive_task(task, "playback", |message| {
            assert!(matches!(message, Message::TrackPlayed));
        })
        .await;

        let state = state.lock().await;
        assert_eq!(state.current_track.as_deref(), Some("song-1"));
        assert!(state.is_playing);
    }

    // `Message::TrackPlayed` is the other half of the `TrackSelected` handoff:
    // the playback task reports the track started, and `update`'s `TrackPlayed`
    // arm receives that completion. The test above only *produces* the message
    // (it drives the task and asserts its output); nothing feeds `TrackPlayed`
    // back through `update`, so this arm — the one update branch the suite
    // never reaches — could drift (e.g. scheduling a follow-up task or touching
    // shared state) with no test catching it. Pin the arm's no-op contract: it
    // schedules no work and leaves the shared state untouched. `blocking_lock`
    // panics inside an async runtime, so this stays a plain test.
    #[test]
    fn track_played_handoff_is_a_noop() {
        let (mut player, state) = test_player();

        // A track is mid-playback when the completion handoff arrives.
        {
            let mut state = state.blocking_lock();
            state.current_track = Some("song-1".to_string());
            state.is_playing = true;
            state.volume = 0.7;
        }

        let task = update(&mut player, Message::TrackPlayed);

        // The arm schedules no follow-up work...
        assert!(iced_runtime::task::into_stream(task).is_none());

        // ...and leaves the shared state exactly as it was.
        let state = state.blocking_lock();
        assert_eq!(state.current_track.as_deref(), Some("song-1"));
        assert!(state.is_playing);
        assert_eq!(state.volume, 0.7);
    }

    // The Previous/Next buttons read the player's songs buffer and the shared
    // current track, hand the direction to the transport helper, and schedule
    // the stepped track as `Message::TrackSelected` (or no task when there is
    // nothing to step through). The stepping arithmetic is tested in
    // `transport.rs`; these tests pin the arm's wiring — that it reads the
    // player's state and forwards the stepped track. The arms use
    // `blocking_lock`, which panics inside an async runtime, so each test
    // calls `update` on a plain thread and only then drives the returned
    // task's stream.
    /// Drives `task` to its single output and asserts it is a `TrackSelected`
    /// for the given id, as the iced runtime would deliver the button's task.
    /// The stepping tests drive their tasks exactly the way the playback and
    /// fetch tests do, so they route through [`drive_task`] instead of
    /// repeating the `into_stream`/`next` plumbing; this stays a plain
    /// function (rather than `async`) because the stepping arms use
    /// `blocking_lock`, which panics inside an async runtime.
    fn assert_track_selected(task: Task<Message>, expected: &str) {
        futures::executor::block_on(drive_task(task, "stepping", |message| match message {
            Message::TrackSelected(id) => assert_eq!(id, expected),
            other => panic!("unexpected stepping task output: {other:?}"),
        }));
    }

    #[test]
    fn next_track_steps_to_the_following_song() {
        let (mut player, state) = test_player();
        player.songs = stepping_songs();
        state.blocking_lock().current_track = Some("song-1".to_string());

        let task = update(&mut player, Message::NextTrack);

        assert_track_selected(task, "song-2");
    }

    #[test]
    fn previous_track_steps_to_the_preceding_song() {
        let (mut player, state) = test_player();
        player.songs = stepping_songs();
        state.blocking_lock().current_track = Some("song-2".to_string());

        let task = update(&mut player, Message::PreviousTrack);

        assert_track_selected(task, "song-1");
    }

    #[test]
    fn next_track_with_no_current_track_starts_at_the_first_song() {
        let (mut player, _state) = test_player();
        player.songs = stepping_songs();

        let task = update(&mut player, Message::NextTrack);

        assert_track_selected(task, "song-1");
    }

    #[test]
    fn next_track_with_no_songs_loaded_does_nothing() {
        let (mut player, _state) = test_player();

        let task = update(&mut player, Message::NextTrack);

        assert!(iced_runtime::task::into_stream(task).is_none());
    }

    #[test]
    fn previous_track_with_no_songs_loaded_does_nothing() {
        let (mut player, _state) = test_player();

        let task = update(&mut player, Message::PreviousTrack);

        assert!(iced_runtime::task::into_stream(task).is_none());
    }

    /// Feeds a `*Loaded` message built from `items` back through `update` and
    /// asserts the list lands in `buffer` unchanged. The three
    /// `*_loaded_populates_list` tests — artists, albums, songs — each used
    /// to repeat the same update-then-compare flow, differing only in the
    /// fixture, the `*Loaded` message variant, and the buffer it fills; the
    /// message constructor and the buffer come in as parameters so the flow
    /// lives here once and each test only names its fixture and target.
    fn assert_store_loaded<T: Clone + PartialEq + std::fmt::Debug>(
        player: &mut WinampPlayer,
        items: Vec<T>,
        loaded: impl Fn(Vec<T>) -> Message,
        buffer: impl Fn(&mut WinampPlayer) -> &mut Vec<T>,
    ) {
        let _ = update(player, loaded(items.clone()));
        assert_eq!(&*buffer(player), &items);
    }

    #[test]
    fn artists_loaded_populates_list() {
        let (mut player, _state) = test_player();
        let artists = vec![sample_artist()];

        assert_store_loaded(&mut player, artists, Message::ArtistsLoaded, |player| {
            &mut player.artists
        });
    }

    #[test]
    fn albums_loaded_populates_list() {
        let (mut player, _state) = test_player();
        let albums = vec![sample_album()];

        assert_store_loaded(&mut player, albums, Message::AlbumsLoaded, |player| {
            &mut player.albums
        });
    }

    #[test]
    fn songs_loaded_populates_list() {
        let (mut player, _state) = test_player();
        let songs = vec![sample_song()];

        assert_store_loaded(&mut player, songs, Message::SongsLoaded, |player| {
            &mut player.songs
        });
    }

    // `store_songs` folds a loaded song list into the accumulated
    // `known_songs` so the Now Playing bar keeps naming a playing track after
    // a browse away. The fold dedups: a song `known_songs` already holds must
    // not be pushed a second time — otherwise every revisit of an album grows
    // the buffer unboundedly, and since `view` scans it per frame, eventually
    // slows the bar. No other test loads overlapping lists:
    // `songs_loaded_populates_list` fills an empty buffer and the browse-away
    // test loads two disjoint albums, so the dedup branch (a song arriving
    // that `known_songs` already contains) is reachable only by loading the
    // same album twice — which this does. A regression that dropped the `any`
    // guard would pass every other test while duplicating songs here.
    #[test]
    fn songs_loaded_does_not_duplicate_already_known_songs() {
        let (mut player, _state) = test_player();

        let songs = stepping_songs();
        let _ = update(&mut player, Message::SongsLoaded(songs.clone()));

        // Re-load the same album — e.g. browsing back to it after stepping
        // away — and its songs must not appear twice in `known_songs`.
        let _ = update(&mut player, Message::SongsLoaded(songs));

        assert_eq!(player.known_songs.len(), 3);
        let known_ids: Vec<&str> = player
            .known_songs
            .iter()
            .map(|song| song.id.as_str())
            .collect();
        assert_eq!(known_ids, vec!["song-1", "song-2", "song-3"]);
    }

    // The Now Playing bar must keep naming the playing track, not its raw id,
    // after the user browses to a different album. `view` renders the bar's
    // label through `WinampPlayer::now_playing_label`, so asserting that same
    // resolution after a browse-away pins the actual bar path: if the label
    // were resolved against `songs` (the currently-browsed album's list, which
    // album-2's `SongsLoaded` just replaced) instead of the accumulated
    // `known_songs`, song-1 would no longer be "known" and the label would
    // fall back to the raw id "song-1", failing this test.
    #[test]
    fn now_playing_label_keeps_the_track_name_after_browsing_to_another_album() {
        let (mut player, state) = test_player();

        // Play song-1 (title "One") from album-1, then browse to album-2's
        // songs — the flow that used to leave the bar showing "song-1".
        let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
        state.blocking_lock().current_track = Some("song-1".to_string());
        let _ = update(
            &mut player,
            Message::SongsLoaded(vec![Song {
                id: "song-4".to_string(),
                title: "B-side".to_string(),
                album_id: "album-2".to_string(),
            }]),
        );

        let current_track = state.blocking_lock().current_track.clone();
        assert_eq!(player.now_playing_label(current_track.as_deref()), "One");
    }

    #[test]
    fn loaded_or_empty_maps_ok_and_err_to_loaded() {
        let albums = vec![sample_album()];

        let ok_message =
            loaded_or_empty::<Album, String>(Ok(albums.clone()), Message::AlbumsLoaded, |_| {
                unreachable!("ok path must not report an error")
            });
        assert!(matches!(ok_message, Message::AlbumsLoaded(v) if v == albums));

        let err_message = loaded_or_empty::<Album, String>(
            Err("boom".to_string()),
            Message::AlbumsLoaded,
            |_| {},
        );
        assert!(matches!(err_message, Message::AlbumsLoaded(v) if v.is_empty()));
    }

    #[test]
    fn loaded_or_empty_reports_the_error_before_falling_back_to_empty() {
        // A failed browse fetch must not vanish silently: the load path
        // reports the error (to stderr in production; here to a recording
        // closure) and still falls back to an empty list so the UI stays
        // usable. `loaded_or_empty` takes the reporter as a parameter so this
        // contract is testable without capturing stderr.
        let mut reported: Option<String> = None;
        let message = loaded_or_empty::<Album, String>(
            Err("boom".to_string()),
            Message::AlbumsLoaded,
            |err| reported = Some(err.clone()),
        );

        assert_eq!(reported.as_deref(), Some("boom"));
        assert!(matches!(message, Message::AlbumsLoaded(v) if v.is_empty()));
    }

    #[test]
    fn played_or_reported_reports_a_failed_play_and_still_completes() {
        // A failed play must not vanish silently: the playback path reports
        // the error (to stderr in production; here to a recording closure)
        // and still emits the `TrackPlayed` completion so the UI's handoff
        // stays intact. `played_or_reported` takes the reporter as a
        // parameter so this contract is testable without capturing stderr.
        let mut reported: Option<String> = None;
        let message = played_or_reported::<String>(Err("boom".to_string()), |err| {
            reported = Some(err.clone());
        });

        assert_eq!(reported.as_deref(), Some("boom"));
        assert!(matches!(message, Message::TrackPlayed));

        // The Ok path completes with the same message and reports nothing.
        let mut reported_ok: Option<String> = None;
        let ok_message = played_or_reported::<String>(Ok(()), |err| {
            reported_ok = Some(err.clone());
        });
        assert!(reported_ok.is_none());
        assert!(matches!(ok_message, Message::TrackPlayed));
    }

    // `fetch_into` schedules the fetch as an iced `Task`; the arm itself only
    // builds it, so the real behavior lives in the returned task. Drive that
    // task to completion and assert the mapped `*Loaded` message, as the
    // load-path arms would produce it.
    #[test]
    fn fetch_failure_report_names_the_fetch_and_includes_the_error() {
        // The browse error report must identify the failing query (the fetch
        // context plus the underlying error), not just say a fetch failed —
        // otherwise a broken backend would log three identical lines for the
        // artists, albums, and songs paths with no way to tell which query
        // failed. The format is pinned here so the contract can't drift.
        let report = fetch_failure_report("loading albums for artist \"artist-1\"", &"boom");
        assert_eq!(
            report,
            "music-library fetch failed (loading albums for artist \"artist-1\"); showing an empty list: \"boom\""
        );
    }

    #[tokio::test]
    async fn fetch_into_schedules_fetch_and_maps_result_to_loaded_message() {
        let (player, _state) = test_player();

        let task = fetch_into(
            &player.apple_music_service,
            "loading favorite artists".to_string(),
            |service| async move { service.get_favorite_artists().await },
            Message::ArtistsLoaded,
        );
        drive_task(task, "fetch", |message| {
            assert!(matches!(
                message,
                Message::ArtistsLoaded(artists) if !artists.is_empty()
            ));
        })
        .await;
    }

    // The success path above proves `fetch_into` delivers a `*Loaded` message
    // for a successful fetch; this pins the error path — a fetch that returns
    // `Err` must still yield the matching `*Loaded` message with an empty
    // list, so the browse view falls back to an empty list instead of waiting
    // forever on a list that never arrives. `loaded_or_empty`'s error branch
    // is tested directly above, but no test drives a failing fetch *through*
    // `fetch_into` — the real service always succeeds, so the error path is
    // reachable only by injecting a failing fetch closure, which is exactly
    // what this does. A regression that swallowed the error (or failed to
    // emit any message) would leave the browse view stuck, and only this
    // test would catch it.
    #[tokio::test]
    async fn fetch_into_maps_a_failed_fetch_to_an_empty_loaded_message() {
        let (player, _state) = test_player();

        let task = fetch_into(
            &player.apple_music_service,
            "loading albums for artist \"artist-1\"".to_string(),
            |_service| async { Err::<Vec<Album>, String>("boom".to_string()) },
            Message::AlbumsLoaded,
        );
        drive_task(task, "failed fetch", |message| {
            assert!(matches!(
                message,
                Message::AlbumsLoaded(albums) if albums.is_empty()
            ));
        })
        .await;
    }

    #[test]
    fn new_player_starts_at_artists_with_nothing_selected() {
        let (player, _state) = test_player();

        assert!(matches!(player.current_view, CurrentView::Artists));
        assert!(player.artists.is_empty());
        assert!(player.albums.is_empty());
        assert!(player.songs.is_empty());
    }

    // `view` is the per-frame assembly: it locks the shared state, resolves
    // the now-playing label, matches the current browse view onto its list
    // buffer, and stacks the Now Playing bar, the transport row, the optional
    // Back button (only below the artist list), and the browse list. No other
    // test reaches it — the `views.rs` tests stop at the individual builders
    // and the `update` tests stop before the view layer — so a regression
    // that made this assembly panic (a bad slider range, a `view_*` builder
    // failing over the input the app produces) would take the window down on
    // every refresh with no test catching it. iced `Element`s expose no tree
    // introspection, so the observable contract — as with the `views.rs`
    // builder tests — is that `view` builds its widget tree without panicking
    // over the space the app actually produces: every browse view (including
    // the pre-load empty buffers), both play states, the volume endpoints the
    // update arm can store, and each now-playing resolution. `view` uses
    // `blocking_lock`, which panics inside an async runtime, so this stays a
    // plain test.
    #[test]
    fn view_constructs_over_the_apps_full_input_space() {
        let (mut player, state) = test_player();

        // The pre-load shape: nothing fetched, no current track. This is what
        // the window shows before the first `LoadArtists` fetch lands, in
        // every browse view.
        for current_view in [
            CurrentView::Artists,
            CurrentView::Albums,
            CurrentView::Songs,
        ] {
            player.current_view = current_view;
            let _empty = view(&player);
        }

        // The loaded shape: artists, albums, and songs in the browse buffers,
        // rendered for every now-playing resolution a session can reach.
        player.artists = vec![sample_artist()];
        player.albums = vec![sample_album()];
        player.songs = vec![sample_song()];

        let now_playing_options = [None, Some("song-1"), Some("no-such-song")];
        let play_states = [false, true];
        let volumes = [0.0, 0.5, 1.0];

        for current_view in [
            CurrentView::Artists,
            CurrentView::Albums,
            CurrentView::Songs,
        ] {
            player.current_view = current_view;
            for current_track in now_playing_options {
                for is_playing in play_states {
                    for volume in volumes {
                        {
                            let mut state = state.blocking_lock();
                            state.current_track = current_track.map(str::to_string);
                            state.is_playing = is_playing;
                            state.volume = volume;
                        }
                        let _screen = view(&player);
                    }
                }
            }
        }
    }
}
