//! Shared application state, the single source of truth for playback
//! status. Both the Apple Music service and the UI read and mutate it,
//! so it lives in its own module rather than in the binary's entry point.

/// Global playback state shared between the Apple Music service and the UI.
///
/// `Debug` lets the state be included in error messages and logs, `Clone`
/// allows snapshots (e.g. for tests), and `PartialEq` supports equality
/// assertions such as the default-state tests below.
#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    pub current_track: Option<String>,
    pub is_playing: bool,
    pub volume: f32,
}

/// The UI and the Apple Music service both construct an `AppState`; one shared
/// default keeps the two construction sites in sync. The starting volume is
/// 0.5, not the derived 0.0, so a manual impl is required.
///
/// Initial state: nothing loaded, stopped, at 50% volume. Keeping the
/// initial values in one place (rather than repeating the struct literal
/// at each construction site) means a new field has only one spot to be
/// given its startup value.
impl Default for AppState {
    fn default() -> Self {
        Self {
            current_track: None,
            is_playing: false,
            volume: 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppState;

    #[test]
    fn default_state_is_stopped_at_half_volume() {
        let state = AppState::default();
        assert_eq!(state.current_track, None);
        assert!(!state.is_playing);
        assert_eq!(state.volume, 0.5);
    }

    #[test]
    fn default_state_is_deterministic() {
        assert_eq!(AppState::default(), AppState::default());
    }
}
