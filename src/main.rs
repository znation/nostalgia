use std::sync::Arc;
use tokio::sync::Mutex;

mod apple_music;
mod equalizer;
mod library;
mod sample_library;
mod state;
mod ui;

use crate::state::AppState;

fn main() -> iced::Result {
    let state = Arc::new(Mutex::new(AppState::default()));

    // Initialize the (stub) Apple Music service.
    apple_music::init_service(state.clone());

    println!("Winamp-style Apple Music Player started!");

    // Run the UI; blocks until the window is closed.
    ui::init_ui(state)
}
