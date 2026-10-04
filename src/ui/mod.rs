use iced::{Application, Element, Settings, executor, widget::Column};
use iced_native::Command;
use std::sync::Arc;
use tokio::sync::Mutex;

mod views;

use crate::{
    apple_music::{Album, AppleMusicService, Artist, Song},
    state::AppState,
};

pub fn init_ui(state: Arc<Mutex<AppState>>) {
    WinampPlayer::run(Settings::default()).expect("Failed to start UI");
}

#[derive(Debug, Clone)]
enum Message {
    PlayPause,
    NextTrack,
    PreviousTrack,
    VolumeChange(f32),
    TrackSelected(String),
    ArtistSelected(String),
    AlbumSelected(String),
    LoadArtists,
    ArtistsLoaded(Vec<Artist>),
    AlbumsLoaded(Vec<Album>),
    SongsLoaded(Vec<Song>),
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

impl Application for WinampPlayer {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let state = Arc::new(Mutex::new(AppState::default()));

        (
            Self {
                state: state.clone(),
                apple_music_service: AppleMusicService::new(state),
                current_view: CurrentView::Artists,
                artists: Vec::new(),
                albums: Vec::new(),
                songs: Vec::new(),
                selected_artist: None,
                selected_album: None,
            },
            Command::perform(async {}, |_| Message::LoadArtists),
        )
    }

    async fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::PlayPause => {
                let mut state = self.state.lock().await;
                state.is_playing = !state.is_playing;
            }
            Message::NextTrack => {
                // Handle next track
            }
            Message::PreviousTrack => {
                // Handle previous track
            }
            Message::VolumeChange(volume) => {
                let mut state = self.state.lock().await;
                state.volume = volume.clamp(0.0, 1.0);
            }
            Message::TrackSelected(track_id) => {
                let _ = self.apple_music_service.play_track(&track_id).await;
            }
            Message::ArtistSelected(artist_id) => {
                self.selected_artist = Some(artist_id.clone());
                self.current_view = CurrentView::Albums(artist_id);
                return Command::perform(
                    self.apple_music_service.get_albums_by_artist(&artist_id),
                    |result| match result {
                        Ok(albums) => Message::AlbumsLoaded(albums),
                        Err(_) => Message::AlbumsLoaded(vec![]),
                    },
                );
            }
            Message::AlbumSelected(album_id) => {
                self.selected_album = Some(album_id.clone());
                let artist_id = self.selected_artist.as_ref().unwrap();
                self.current_view = CurrentView::Songs(artist_id.clone(), album_id);
                return Command::perform(
                    self.apple_music_service.get_songs_from_album(&album_id),
                    |result| match result {
                        Ok(songs) => Message::SongsLoaded(songs),
                        Err(_) => Message::SongsLoaded(vec![]),
                    },
                );
            }
            Message::LoadArtists => {
                return Command::perform(
                    self.apple_music_service.get_favorite_artists(),
                    |result| match result {
                        Ok(artists) => Message::ArtistsLoaded(artists),
                        Err(_) => Message::ArtistsLoaded(vec![]),
                    },
                );
            }
            Message::ArtistsLoaded(artists) => {
                self.artists = artists;
            }
            Message::AlbumsLoaded(albums) => {
                self.albums = albums;
            }
            Message::SongsLoaded(songs) => {
                self.songs = songs;
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let state = self.state.lock().unwrap();

        let main_content = match &self.current_view {
            CurrentView::Artists => views::view_artists(&self.artists),
            CurrentView::Albums(_) => views::view_albums(&self.albums),
            CurrentView::Songs(_, _) => views::view_songs(&self.songs),
        };

        Column::new()
            .push(views::view_now_playing(state.current_track.as_deref()))
            .push(views::view_transport_controls(state.is_playing))
            .push(main_content)
            .into()
    }
}
