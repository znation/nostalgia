use iced::{
    Element, Length, Task,
    widget::{Button, Column, Row, Slider, Space, Text},
};
use std::sync::Arc;
use tokio::sync::Mutex;

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
        Message::NextTrack | Message::PreviousTrack => Task::none(),
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
            let service = player.apple_music_service.clone();
            Task::perform(
                async move { service.get_albums_by_artist(&artist_id).await },
                |result| match result {
                    Ok(albums) => Message::AlbumsLoaded(albums),
                    Err(_) => Message::AlbumsLoaded(Vec::new()),
                },
            )
        }
        Message::AlbumSelected(album_id) => {
            player.current_view = CurrentView::Songs;
            let service = player.apple_music_service.clone();
            Task::perform(
                async move { service.get_songs_from_album(&album_id).await },
                |result| match result {
                    Ok(songs) => Message::SongsLoaded(songs),
                    Err(_) => Message::SongsLoaded(Vec::new()),
                },
            )
        }
        Message::LoadArtists => {
            let service = player.apple_music_service.clone();
            Task::perform(
                async move { service.get_favorite_artists().await },
                |result| match result {
                    Ok(artists) => Message::ArtistsLoaded(artists),
                    Err(_) => Message::ArtistsLoaded(Vec::new()),
                },
            )
        }
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

fn view(player: &WinampPlayer) -> Element<'_, Message> {
    let state = player.state.blocking_lock();
    let now_playing = state
        .current_track
        .clone()
        .unwrap_or_else(|| "Nothing".to_string());
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
    fn new_player_starts_at_artists_with_nothing_selected() {
        let (player, _state) = test_player();

        assert!(matches!(player.current_view, CurrentView::Artists));
        assert!(player.artists.is_empty());
        assert!(player.albums.is_empty());
        assert!(player.songs.is_empty());
    }
}
