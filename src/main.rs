use std::sync::Arc;
use tokio::sync::Mutex;

mod apple_music;
mod state;
mod ui;

use crate::state::AppState;

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(AppState::default()));

    // Initialize UI
    ui::init_ui(state.clone()).await;

    // Initialize Apple Music service
    apple_music::init_service(state.clone()).await;

    println!("WinAmp-style Apple Music Player started!");
}
