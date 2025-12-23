use std::sync::Arc;
use tokio::sync::Mutex;
use iced::{
    Application, Command, Element, Settings,
    executor, Subscription, Theme
};
use iced_native::keyboard;

use crate::AppState;

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
}

struct WinampPlayer {
    state: Arc<Mutex<AppState>>,
}

impl Application for WinampPlayer {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                state: Arc::new(Mutex::new(AppState {
                    current_track: None,
                    is_playing: false,
                    volume: 0.5,
                })),
            },
            Command::none(),
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
            Message::TrackSelected(track) => {
                let mut state = self.state.lock().await;
                state.current_track = Some(track);
            }
        }
        Command::none()
    }

    fn view(&self, state: &AppState) -> Element<Message> {
        // WinAmp-style UI layout
        iced::Column::new()
            .push(
                // Main display area (like WinAmp's main window)
                iced::Container::new(
                    iced::Column::new()
                        .push(
                            // Album art and track info
                            iced::Row::new()
                                .push(/* album art */)
                                .push(/* track info */),
                        )
                        .push(
                            // Playback controls (WinAmp-style buttons)
                            iced::Row::new()
                                .push(/* previous button */)
                                .push(/* play/pause button */)
                                .push(/* next button */),
                        ),
                )
                .width(iced::Length::Fill)
                .height(iced::Length::FillPortion(3)),
            )
            .push(
                // Playlist area (like WinAmp's playlist window)
                iced::Container::new(
                    /* playlist content */
                )
                .width(iced::Length::Fill)
                .height(iced::Length::FillPortion(1)),
            )
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}
