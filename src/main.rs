use std::sync::Arc;
use tokio::sync::Mutex;

mod apple_music;
mod ui;

struct AppState {
    current_track: Option<String>,
    is_playing: bool,
    volume: f32,
}

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(AppState {
        current_track: None,
        is_playing: false,
        volume: 0.5,
    }));

    // Initialize UI
    ui::init_ui(state.clone()).await;

    // Initialize Apple Music service
    apple_music::init_service(state.clone()).await;

    println!("WinAmp-style Apple Music Player started!");
}
