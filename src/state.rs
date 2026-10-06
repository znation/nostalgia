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

/// The one production `AppState` is built in the app's entry point (`main`)
/// and shared (as an `Arc<Mutex<_>>`) with both the UI and the Apple Music
/// service; the test modules build their own. A single `Default` keeps every
/// construction site in sync. The starting volume is 0.5, not the derived 0.0,
/// so a manual impl is required.
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

impl AppState {
    /// Flip the play/pause flag in place. The UI's Play/Pause button is the
    /// only toggle caller; keeping the flip here (rather than inlined at the
    /// call site) puts the toggling semantics next to the field they mutate.
    pub fn toggle_playing(&mut self) {
        self.is_playing = !self.is_playing;
    }
}

/// Clamps a volume value to the valid `[0.0, 1.0]` range.
///
/// iced's slider can emit a value outside the range (a drag beyond the ends,
/// or a stale in-flight change), and the UI must never store an unclamped
/// volume in `AppState`. Volume is shared state, so the rule lives beside
/// `AppState` as a pure function the UI's `update` arm calls before storing.
///
/// `f32::clamp` passes NaN through unchanged, so a non-finite volume is
/// mapped to `0.0` (silence) rather than being stored as-is — the safe
/// outcome for a value that is neither in range nor comparable to it.
pub fn clamp_volume(volume: f32) -> f32 {
    if volume.is_nan() {
        0.0
    } else {
        volume.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{AppState, clamp_volume};

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

    #[test]
    fn toggle_playing_flips_only_the_playback_flag() {
        let mut state = AppState::default();
        let track = state.current_track.clone();
        let volume = state.volume;
        assert!(!state.is_playing);

        state.toggle_playing();
        assert!(state.is_playing);
        assert_eq!(state.current_track, track);
        assert_eq!(state.volume, volume);

        state.toggle_playing();
        assert!(!state.is_playing);
    }

    #[test]
    fn clamp_volume_caps_above_one() {
        assert_eq!(clamp_volume(1.5), 1.0);
    }

    #[test]
    fn clamp_volume_floors_below_zero() {
        assert_eq!(clamp_volume(-0.2), 0.0);
    }

    #[test]
    fn clamp_volume_passes_through_in_range() {
        assert_eq!(clamp_volume(0.0), 0.0);
        assert_eq!(clamp_volume(0.3), 0.3);
        assert_eq!(clamp_volume(1.0), 1.0);
    }

    #[test]
    fn clamp_volume_treats_nan_as_silence() {
        // `f32::clamp` passes NaN through unchanged, so it must be handled
        // explicitly or a non-finite value lands in shared state.
        assert_eq!(clamp_volume(f32::NAN), 0.0);
    }
}
