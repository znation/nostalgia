//! The iced application: the `WinampPlayer` app struct, its `Message` event
//! type, and the `update`/`view` loop `init_ui` hands to iced. Widget
//! construction lives in the `views` submodule (browse lists, Now Playing bar,
//! transport controls) and the Previous/Next stepping arithmetic in
//! `transport`; this module wires those to the shared `AppState` and the
//! `AppleMusicService` seam. Its unit tests live in the `tests` submodule.

use iced::{Element, Task, widget::Column};
use std::{collections::HashMap, future::Future, sync::Arc};
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
    ToggleRepeat,
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
    /// Each known song's id mapped to its title — the accumulated index of
    /// every song the player has loaded across all browsed albums, so the Now
    /// Playing bar can still name the playing track after the user browses to
    /// a different album (whose list replaces `songs`). `store_songs` records
    /// a freshly loaded album in O(1) per song, and `now_playing_label`
    /// resolves the per-frame bar label with a single get instead of scanning
    /// a growing list on every frame. The map is the whole accumulation: the
    /// buffer it indexes used to clone each whole `Song` into an unread list,
    /// so it is the only per-album cost of keeping a track title known.
    known_titles: HashMap<String, String>,
}

/// Which browse screen is showing. Payload-free: the albums and songs the
/// view renders come from the loaded `albums`/`songs` buffers, so carrying
/// the selected ids here (as earlier versions did) only duplicated
/// state nothing read.
#[derive(Debug, PartialEq, Eq)]
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
            known_titles: HashMap::new(),
        }
    }

    /// The Now Playing bar's label for `current_track`, resolved against the
    /// accumulated [`Self::known_titles`] index rather than the
    /// currently-browsed album's `songs`: `view` renders the bar from this, so
    /// the bar keeps naming a playing track even after a browse to another
    /// album replaced `songs`. The browse-away regression test asserts this
    /// same path.
    fn now_playing_label(&self, current_track: Option<&str>) -> String {
        views::now_playing_label(&self.known_titles, current_track)
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
/// just store; `SongsLoaded` records the new songs into `known_titles` first
/// (see [`store_songs`]) and then ends in this same store. The
/// store-and-noop shape lives here once instead of in each arm.
fn store_loaded<T>(buffer: &mut Vec<T>, items: Vec<T>) -> Task<Message> {
    *buffer = items;
    Task::none()
}

/// Stores a freshly fetched song list into the player's current-album buffer
/// and records each song's id→title pair in the accumulated
/// [`WinampPlayer::known_titles`] index, so a later browse to a different
/// album (which replaces `songs`) can't lose the title of the playing track.
/// The fold is O(1) per song — one map insert, with no `Song` clone and no
/// growing list to scan — rather than a linear scan of everything the player
/// has ever loaded. Only `SongsLoaded` needs the extra fold — artists and
/// albums never appear in the Now Playing bar.
fn store_songs(player: &mut WinampPlayer, songs: Vec<Song>) -> Task<Message> {
    for song in &songs {
        // A repeat id re-inserts an identical title, a no-op on the map's
        // contents — so revisiting an album never grows `known_titles`.
        player
            .known_titles
            .insert(song.id.clone(), song.title.clone());
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
/// once. The shared Repeat flag is read under the same lock and passed
/// through to the stepping function, so the wrap-vs-stop-at-the-edge decision
/// (see `transport::next_track_id`) stays in `transport` too.
fn step_track(
    player: &WinampPlayer,
    step: fn(&[Song], Option<&str>, bool) -> Option<String>,
) -> Task<Message> {
    // The stepped song is computed under the state lock, borrowing the
    // current track directly. The earlier version cloned the `current_track`
    // `String` only to borrow it via `as_deref` — the same clone-to-borrow
    // the per-frame now-playing path used to do — so the lock now covers the
    // pure stepping scan (fast, and `step` never locks anything itself).
    let state = player.state.blocking_lock();
    match step(&player.songs, state.current_track.as_deref(), state.repeat) {
        Some(track_id) => Task::done(Message::TrackSelected(track_id)),
        None => Task::none(),
    }
}

/// Locks the shared playback state, applies `mutation` to it, and returns no
/// task. The Play/Pause, Stop, ToggleRepeat, and VolumeChange arms all repeat
/// the same synchronous shared-state update — `blocking_lock`, one mutation,
/// then `Task::none()` — so the lock-and-noop shape lives here once and each
/// arm only names its mutation. Asynchronous work (fetches) goes through
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
        Message::ToggleRepeat => mutate_state(player, AppState::toggle_repeat),
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
    // borrowed `current_track` against the accumulated `known_titles` index
    // (see [`WinampPlayer::now_playing_label`]) — the label outlives the lock,
    // but the owned `String` clone of the current track is not needed, so the
    // per-frame path allocates only the resolved label.
    let (now_playing, is_playing, volume, repeat) = {
        let state = player.state.blocking_lock();
        (
            player.now_playing_label(state.current_track.as_deref()),
            state.is_playing,
            state.volume,
            state.repeat,
        )
    };

    let main_content = match &player.current_view {
        CurrentView::Artists => views::view_artists(&player.artists),
        CurrentView::Albums => views::view_albums(&player.albums),
        // A second short lock (like the label block above) hands the current
        // track's id to the Songs view so it can mark the playing row. The
        // borrowed id is compared inside `song_row` and the guard drops at
        // the end of this arm, so no per-frame `Option<String>` clone is
        // needed.
        CurrentView::Songs => {
            let state = player.state.blocking_lock();
            views::view_songs(&player.songs, state.current_track.as_deref())
        }
    };

    let mut column = Column::new()
        .push(views::view_now_playing(now_playing))
        .push(views::view_transport_controls(is_playing, volume, repeat));
    // The Back button sits above the list it navigates and exists only where
    // the hierarchy has a level above to return to (see `views::can_go_back`).
    if views::can_go_back(&player.current_view) {
        column = column.push(views::view_back_button());
    }
    column.push(main_content).into()
}

#[cfg(test)]
mod tests;
