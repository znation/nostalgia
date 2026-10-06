//! The nostalgia binary: it builds the shared `AppState`, wires it into the
//! stub Apple Music service, and hands both to the iced application.
//!
//! `unsafe` is forbidden crate-wide: the player is safe Rust, so an `unsafe`
//! block added here is a mistake worth failing the build for rather than
//! catching in review.
#![forbid(unsafe_code)]

use std::sync::Arc;
use tokio::sync::Mutex;

mod apple_music;
mod clamp;
mod equalizer;
mod library;
mod sample_library;
mod state;
#[cfg(test)]
mod test_support;
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
