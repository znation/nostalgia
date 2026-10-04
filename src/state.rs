//! Shared application state, the single source of truth for playback
//! status. Both the Apple Music service and the UI read and mutate it,
//! so it lives in its own module rather than in the binary's entry point.

/// Global playback state shared between the Apple Music service and the UI.
pub struct AppState {
    pub current_track: Option<String>,
    pub is_playing: bool,
    pub volume: f32,
}
