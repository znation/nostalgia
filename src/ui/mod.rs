use iced::{
    Element, Length, Task,
    widget::{Button, Column, Row, Slider, Space, Text},
};
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
    VolumeChange(f32),
    NextTrack,
    PreviousTrack,
    TrackSelected(String),
    ArtistSelected(String),
    AlbumSelected(String),
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
        }
    }
}

fn boot(state: Arc<Mutex<AppState>>) -> (WinampPlayer, Task<Message>) {
    (WinampPlayer::new(state), Task::done(Message::LoadArtists))
}

/// Maps a library-fetch `Result` to its matching `*Loaded` message, falling
/// back to an empty list on error. Shared by the artists, albums, and songs
/// load paths so the error fallback stays identical in all three.
fn loaded_or_empty<T, E>(result: Result<Vec<T>, E>, loaded: impl Fn(Vec<T>) -> Message) -> Message {
    match result {
        Ok(items) => loaded(items),
        Err(_) => loaded(Vec::new()),
    }
}

/// Runs a library-fetch future through iced's runtime, mapping its `Result`
/// onto the matching `*Loaded` message (empty list on error, via
/// [`loaded_or_empty`]). Shared by the artists, albums, and songs load arms
/// so none of them repeats the clone-the-service-then-`Task::perform`
/// boilerplate.
fn fetch_into<T, E, Fut>(
    service: &AppleMusicService,
    fetch: impl FnOnce(AppleMusicService) -> Fut + Send + 'static,
    loaded: impl Fn(Vec<T>) -> Message + Send + 'static,
) -> Task<Message>
where
    T: Send + 'static,
    E: Send + 'static,
    Fut: Future<Output = Result<Vec<T>, E>> + Send + 'static,
{
    let service = service.clone();
    Task::perform(async move { fetch(service).await }, move |result| {
        loaded_or_empty(result, loaded)
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
    let current = player.state.blocking_lock().current_track.clone();
    match step(&player.songs, current.as_deref()) {
        Some(track_id) => Task::done(Message::TrackSelected(track_id)),
        None => Task::none(),
    }
}

fn update(player: &mut WinampPlayer, message: Message) -> Task<Message> {
    match message {
        Message::PlayPause => {
            let mut state = player.state.blocking_lock();
            state.toggle_playing();
            Task::none()
        }
        Message::VolumeChange(volume) => {
            let mut state = player.state.blocking_lock();
            state.volume = state::clamp_volume(volume);
            Task::none()
        }
        Message::NextTrack => step_track(player, transport::next_track_id),
        Message::PreviousTrack => step_track(player, transport::previous_track_id),
        Message::TrackSelected(track_id) => {
            let service = player.apple_music_service.clone();
            Task::perform(
                async move {
                    let _ = service.play_track(&track_id).await;
                },
                |_| Message::TrackPlayed,
            )
        }
        Message::ArtistSelected(artist_id) => {
            player.current_view = CurrentView::Albums;
            fetch_into(
                &player.apple_music_service,
                move |service| async move { service.get_albums_by_artist(&artist_id).await },
                Message::AlbumsLoaded,
            )
        }
        Message::AlbumSelected(album_id) => {
            player.current_view = CurrentView::Songs;
            fetch_into(
                &player.apple_music_service,
                move |service| async move { service.get_songs_from_album(&album_id).await },
                Message::SongsLoaded,
            )
        }
        Message::LoadArtists => fetch_into(
            &player.apple_music_service,
            |service| async move { service.get_favorite_artists().await },
            Message::ArtistsLoaded,
        ),
        Message::ArtistsLoaded(artists) => {
            player.artists = artists;
            Task::none()
        }
        Message::AlbumsLoaded(albums) => {
            player.albums = albums;
            Task::none()
        }
        Message::SongsLoaded(songs) => {
            player.songs = songs;
            Task::none()
        }
        Message::TrackPlayed => Task::none(),
    }
}

/// The Now Playing bar label: the title of `current_track` when it is one of
/// `songs`; the raw id when it isn't in `songs`; "Nothing" when stopped.
fn now_playing_label(songs: &[Song], current_track: Option<&str>) -> String {
    match current_track {
        Some(id) => songs
            .iter()
            .find(|song| song.id == id)
            .map(|song| song.title.clone())
            .unwrap_or_else(|| id.to_string()),
        None => "Nothing".to_string(),
    }
}

fn view(player: &WinampPlayer) -> Element<'_, Message> {
    let state = player.state.blocking_lock();
    let now_playing = now_playing_label(&player.songs, state.current_track.as_deref());
    let play_label = if state.is_playing { "Pause" } else { "Play" };
    let volume = state.volume;
    drop(state);

    let main_content = match &player.current_view {
        CurrentView::Artists => views::view_artists(&player.artists),
        CurrentView::Albums => views::view_albums(&player.albums),
        CurrentView::Songs => views::view_songs(&player.songs),
    };

    Column::new()
        .push(
            Row::new()
                .push(Text::new("Now Playing: ").size(20))
                .push(Text::new(now_playing).size(20)),
        )
        .push(
            Row::new()
                .push(Button::new(Text::new(play_label)).on_press(Message::PlayPause))
                .push(Space::new().width(Length::Fixed(20.0)))
                .push(Button::new(Text::new("Previous")).on_press(Message::PreviousTrack))
                .push(Space::new().width(Length::Fixed(20.0)))
                .push(Button::new(Text::new("Next")).on_press(Message::NextTrack))
                .push(Space::new().width(Length::Fixed(20.0)))
                .push(
                    Slider::new(0.0..=1.0, volume, Message::VolumeChange)
                        .step(0.01)
                        .width(Length::Fixed(100.0)),
                ),
        )
        .push(main_content)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh player over its own shared state, so a test can inspect the
    /// same `AppState` the player mutates.
    fn test_player() -> (WinampPlayer, Arc<Mutex<AppState>>) {
        let state = Arc::new(Mutex::new(AppState::default()));
        (WinampPlayer::new(state.clone()), state)
    }

    /// Drives an iced `Task` to completion and hands its single `Output`
    /// action to `check`. `update` only schedules work as a `Task`, so a test
    /// that wants to observe the resulting message — `TrackPlayed` after
    /// playback, a `*Loaded` message after a fetch — must run the task itself
    /// the way the iced runtime would. Both the playback and the fetch tests
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

    // The Previous/Next buttons read the player's songs buffer and the shared
    // current track, hand the direction to the transport helper, and schedule
    // the stepped track as `Message::TrackSelected` (or no task when there is
    // nothing to step through). The stepping arithmetic is tested in
    // `transport.rs`; these tests pin the arm's wiring — that it reads the
    // player's state and forwards the stepped track. The arms use
    // `blocking_lock`, which panics inside an async runtime, so each test
    // calls `update` on a plain thread and only then drives the returned
    // task's stream.
    fn songs_for_stepping() -> Vec<Song> {
        vec![
            Song {
                id: "song-1".to_string(),
                title: "One".to_string(),
                album_id: "album-1".to_string(),
            },
            Song {
                id: "song-2".to_string(),
                title: "Two".to_string(),
                album_id: "album-1".to_string(),
            },
            Song {
                id: "song-3".to_string(),
                title: "Three".to_string(),
                album_id: "album-1".to_string(),
            },
        ]
    }

    /// Drives `task` to its single output and asserts it is a `TrackSelected`
    /// for the given id, as the iced runtime would deliver the button's task.
    fn assert_track_selected(task: Task<Message>, expected: &str) {
        use futures::StreamExt;

        let mut stream =
            iced_runtime::task::into_stream(task).expect("stepping must schedule a task");
        let action = futures::executor::block_on(stream.next())
            .expect("stepping task must yield a completion message");
        match action {
            iced_runtime::Action::Output(Message::TrackSelected(id)) => assert_eq!(id, expected),
            other => panic!("unexpected stepping task output: {other:?}"),
        }
    }

    #[test]
    fn next_track_steps_to_the_following_song() {
        let (mut player, state) = test_player();
        player.songs = songs_for_stepping();
        state.blocking_lock().current_track = Some("song-1".to_string());

        let task = update(&mut player, Message::NextTrack);

        assert_track_selected(task, "song-2");
    }

    #[test]
    fn previous_track_steps_to_the_preceding_song() {
        let (mut player, state) = test_player();
        player.songs = songs_for_stepping();
        state.blocking_lock().current_track = Some("song-2".to_string());

        let task = update(&mut player, Message::PreviousTrack);

        assert_track_selected(task, "song-1");
    }

    #[test]
    fn next_track_with_no_current_track_starts_at_the_first_song() {
        let (mut player, _state) = test_player();
        player.songs = songs_for_stepping();

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

    #[test]
    fn artists_loaded_populates_list() {
        let (mut player, _state) = test_player();
        let artists = vec![Artist {
            id: "artist-1".to_string(),
            name: "The Sample Band".to_string(),
        }];

        let _ = update(&mut player, Message::ArtistsLoaded(artists.clone()));

        assert_eq!(player.artists, artists);
    }

    #[test]
    fn albums_loaded_populates_list() {
        let (mut player, _state) = test_player();
        let albums = vec![Album {
            id: "album-1".to_string(),
            title: "First Record".to_string(),
            artist_id: "artist-1".to_string(),
        }];

        let _ = update(&mut player, Message::AlbumsLoaded(albums.clone()));

        assert_eq!(player.albums, albums);
    }

    #[test]
    fn songs_loaded_populates_list() {
        let (mut player, _state) = test_player();
        let songs = vec![Song {
            id: "song-1".to_string(),
            title: "Opening".to_string(),
            album_id: "album-1".to_string(),
        }];

        let _ = update(&mut player, Message::SongsLoaded(songs.clone()));

        assert_eq!(player.songs, songs);
    }

    #[test]
    fn loaded_or_empty_maps_ok_and_err_to_loaded() {
        let albums = vec![Album {
            id: "album-1".to_string(),
            title: "First Record".to_string(),
            artist_id: "artist-1".to_string(),
        }];

        let ok_message =
            loaded_or_empty::<Album, String>(Ok(albums.clone()), Message::AlbumsLoaded);
        assert!(matches!(ok_message, Message::AlbumsLoaded(v) if v == albums));

        let err_message =
            loaded_or_empty::<Album, String>(Err("boom".to_string()), Message::AlbumsLoaded);
        assert!(matches!(err_message, Message::AlbumsLoaded(v) if v.is_empty()));
    }

    // `fetch_into` schedules the fetch as an iced `Task`; the arm itself only
    // builds it, so the real behavior lives in the returned task. Drive that
    // task to completion and assert the mapped `*Loaded` message, as the
    // load-path arms would produce it.
    #[tokio::test]
    async fn fetch_into_schedules_fetch_and_maps_result_to_loaded_message() {
        let (player, _state) = test_player();

        let task = fetch_into(
            &player.apple_music_service,
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

    #[test]
    fn new_player_starts_at_artists_with_nothing_selected() {
        let (player, _state) = test_player();

        assert!(matches!(player.current_view, CurrentView::Artists));
        assert!(player.artists.is_empty());
        assert!(player.albums.is_empty());
        assert!(player.songs.is_empty());
    }

    #[test]
    fn now_playing_label_shows_nothing_when_stopped() {
        let songs = vec![Song {
            id: "song-1".to_string(),
            title: "Opening".to_string(),
            album_id: "album-1".to_string(),
        }];
        assert_eq!(now_playing_label(&songs, None), "Nothing");
    }

    #[test]
    fn now_playing_label_resolves_known_track_to_title() {
        let songs = vec![Song {
            id: "song-1".to_string(),
            title: "Opening".to_string(),
            album_id: "album-1".to_string(),
        }];
        assert_eq!(now_playing_label(&songs, Some("song-1")), "Opening");
    }

    #[test]
    fn now_playing_label_falls_back_to_id_when_track_not_in_songs() {
        let songs = vec![Song {
            id: "song-1".to_string(),
            title: "Opening".to_string(),
            album_id: "album-1".to_string(),
        }];
        assert_eq!(
            now_playing_label(&songs, Some("no-such-song")),
            "no-such-song"
        );
    }

    #[test]
    fn now_playing_label_falls_back_to_id_when_no_songs_loaded() {
        assert_eq!(now_playing_label(&[], Some("song-1")), "song-1");
    }
}
