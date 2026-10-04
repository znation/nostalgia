//! Shared application state, the single source of truth for playback
//! status. Both the Apple Music service and the UI read and mutate it,
//! so it lives in its own module rather than in the binary's entry point.

/// Global playback state shared between the Apple Music service and the UI.
pub struct AppState {
    pub current_track: Option<String>,
    pub is_playing: bool,
    pub volume: f32,
}

impl Default for AppState {
    /// Initial state: nothing loaded, stopped, at 50% volume. Keeping the
    /// initial values in one place (rather than repeating the struct literal
    /// at each construction site) means a new field has only one spot to be
    /// given its startup value.
    fn default() -> Self {
        Self {
            current_track: None,
            is_playing: false,
            volume: 0.5,
        }
    }
}
