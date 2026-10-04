use std::sync::Arc;
use tokio::sync::Mutex;
use iced::{
    Application, Element, Settings,
    executor, Length, widget::{
Column, Row, Text, Button, Scrollable, Space,
    }
};
use iced_native::Command;

use crate::{state::AppState, apple_music::{AppleMusicService, Artist, Album, Song}};

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
        let state = Arc::new(Mutex::new(AppState {
            current_track: None,
            is_playing: false,
            volume: 0.5,
        }));

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
                if state.is_playing {
                    // Start playback
                } else {
                    // Pause playback
                }
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
                    }
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
                    }
                );
            }
            Message::LoadArtists => {
                return Command::perform(
                    self.apple_music_service.get_favorite_artists(),
                    |result| match result {
                        Ok(artists) => Message::ArtistsLoaded(artists),
                        Err(_) => Message::ArtistsLoaded(vec![]),
                    }
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
            CurrentView::Artists => self.view_artists(),
            CurrentView::Albums(artist_id) => self.view_albums(artist_id),
            CurrentView::Songs(artist_id, album_id) => self.view_songs(artist_id, album_id),
        };

        Column::new()
            .push(
                Row::new()
                    .push(Text::new("Now Playing: ").size(20))
                    .push(match &state.current_track {
                        Some(track) => Text::new(track).size(20),
                        None => Text::new("Nothing").size(20),
                    })
            )
            .push(
                Row::new()
                    .push(Button::new(Text::new(if state.is_playing { "Pause" } else { "Play" }))
                        .on_press(Message::PlayPause))
                    .push(Space::with_width(Length::Units(20)))
                    .push(Button::new(Text::new("Previous"))
                        .on_press(Message::PreviousTrack))
                    .push(Space::with_width(Length::Units(20)))
                    .push(Button::new(Text::new("Next"))
                        .on_press(Message::NextTrack))
            )
            .push(main_content)
            .into()
    }
}

impl WinampPlayer {
    fn view_artists(&self) -> Element<Message> {
        let mut column = Column::new().padding(20);

        for artist in &self.artists {
            column = column.push(
                Button::new(
                    Row::new()
                        .push(Text::new(&artist.name).size(18))
                        .push(Space::with_width(Length::Units(10)))
                        .push(Text::new("View Albums").size(14))
                )
                .on_press(Message::ArtistSelected(artist.id.clone()))
            );
        }

        Scrollable::new(column)
            .width(Length::Fill)
            .height(Length::FillPortion(3))
            .into()
    }

    fn view_albums(&self, artist_id: &str) -> Element<Message> {
        let mut column = Column::new().padding(20);

        for album in &self.albums {
            column = column.push(
                Button::new(
                    Row::new()
                        .push(Text::new(&album.title).size(18))
                        .push(Space::with_width(Length::Units(10)))
                        .push(Text::new("View Songs").size(14))
                )
                .on_press(Message::AlbumSelected(album.id.clone()))
            );
        }

        Scrollable::new(column)
            .width(Length::Fill)
            .height(Length::FillPortion(3))
            .into()
    }

    fn view_songs(&self, artist_id: &str, album_id: &str) -> Element<Message> {
        let mut column = Column::new().padding(20);

        for song in &self.songs {
            column = column.push(
                Button::new(
                    Row::new()
                        .push(Text::new(&song.title).size(18))
                        .push(Space::with_width(Length::Units(10)))
                        .push(Text::new("Play").size(14))
                )
                .on_press(Message::TrackSelected(song.id.clone()))
            );
        }

        Scrollable::new(column)
            .width(Length::Fill)
            .height(Length::FillPortion(3))
            .into()
    }
}
