use iced::{
    Element, Length, Task,
    widget::{Button, Column, Row, Space, Text},
};
use std::sync::Arc;
use tokio::sync::Mutex;

mod views;

use crate::{
    apple_music::AppleMusicService,
    library::{Album, Artist, Song},
    state::AppState,
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
    selected_artist: Option<String>,
    selected_album: Option<String>,
}

enum CurrentView {
    Artists,
    Albums(String),
    Songs(String, String),
}

fn boot(state: Arc<Mutex<AppState>>) -> (WinampPlayer, Task<Message>) {
    let apple_music_service = AppleMusicService::new(state.clone());
    (
        WinampPlayer {
            state,
            apple_music_service,
            current_view: CurrentView::Artists,
            artists: Vec::new(),
            albums: Vec::new(),
            songs: Vec::new(),
            selected_artist: None,
            selected_album: None,
        },
        Task::done(Message::LoadArtists),
    )
}

fn update(player: &mut WinampPlayer, message: Message) -> Task<Message> {
    match message {
        Message::PlayPause => {
            let mut state = player.state.blocking_lock();
            state.toggle_playing();
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
            player.selected_artist = Some(artist_id.clone());
            player.current_view = CurrentView::Albums(artist_id.clone());
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
            player.selected_album = Some(album_id.clone());
            let artist_id = player.selected_artist.clone().unwrap_or_default();
            player.current_view = CurrentView::Songs(artist_id, album_id.clone());
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
    drop(state);

    let main_content = match &player.current_view {
        CurrentView::Artists => views::view_artists(&player.artists),
        CurrentView::Albums(_) => views::view_albums(&player.albums),
        CurrentView::Songs(_, _) => views::view_songs(&player.songs),
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
                .push(Button::new(Text::new("Next")).on_press(Message::NextTrack)),
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
        let player = WinampPlayer {
            state: state.clone(),
            apple_music_service: AppleMusicService::new(state.clone()),
            current_view: CurrentView::Artists,
            artists: Vec::new(),
            albums: Vec::new(),
            songs: Vec::new(),
            selected_artist: None,
            selected_album: None,
        };
        (player, state)
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
    fn artist_selected_sets_view_and_selection() {
        let (mut player, _state) = test_player();

        let _ = update(&mut player, Message::ArtistSelected("artist-1".to_string()));

        assert_eq!(player.selected_artist.as_deref(), Some("artist-1"));
        assert!(matches!(
            player.current_view,
            CurrentView::Albums(ref artist_id) if artist_id == "artist-1"
        ));
    }

    #[test]
    fn album_selected_sets_view_and_selection() {
        let (mut player, _state) = test_player();
        player.selected_artist = Some("artist-2".to_string());

        let _ = update(&mut player, Message::AlbumSelected("album-3".to_string()));

        assert_eq!(player.selected_album.as_deref(), Some("album-3"));
        assert!(matches!(
            player.current_view,
            CurrentView::Songs(ref artist_id, ref album_id)
                if artist_id == "artist-2" && album_id == "album-3"
        ));
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
}
