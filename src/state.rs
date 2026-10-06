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
    /// Whether Previous/Next wrap around the current album's ends (Repeat on)
    /// or stop at the edge (Repeat off). Starts off, as in Winamp.
    pub repeat: bool,
    pub volume: f32,
}

/// The one production `AppState` is built in the app's entry point (`main`)
/// and shared (as an `Arc<Mutex<_>>`) with both the UI and the Apple Music
/// service; the test modules build their own. A single `Default` keeps every
/// construction site in sync: a new field's startup value is set here, in one
/// place, instead of in a struct literal repeated at each site. The starting
/// volume is 0.5, not the derived 0.0, so a manual impl is required.
///
/// Initial state: nothing loaded, stopped, Repeat off, at 50% volume.
impl Default for AppState {
    fn default() -> Self {
        Self {
            current_track: None,
            is_playing: false,
            repeat: false,
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

    /// Flip the Repeat flag in place. The UI's Repeat button is the only
    /// toggle caller; keeping the flip here (rather than inlined at the call
    /// site) puts the toggling semantics next to the field they mutate, like
    /// `toggle_playing`.
    pub fn toggle_repeat(&mut self) {
        self.repeat = !self.repeat;
    }

    /// Clear the playback flag in place, leaving `current_track` in place so
    /// the Now Playing bar keeps showing the interrupted track's title
    /// (matching Winamp, where Stop halts the music and the title stays in
    /// the display). The UI's Stop button is the only caller; keeping the
    /// semantics here (rather than inlined at the call site) puts them next
    /// to the field they mutate, like `toggle_playing`.
    pub fn stop(&mut self) {
        self.is_playing = false;
    }
}

/// Clamps a volume value to the valid `[0.0, 1.0]` range.
///
/// iced's slider can emit a value outside the range (a drag beyond the ends,
/// or a stale in-flight change), and the UI must never store an unclamped
/// volume in `AppState`. Volume is shared state, so the rule lives beside
/// `AppState` as a pure function the UI's `update` arm calls before storing.
///
/// `f32::clamp` passes NaN through unchanged, so a NaN volume is mapped to
/// `0.0` (silence) rather than being stored as-is — the safe outcome for a
/// value that is neither in range nor comparable to it. Non-finite values
/// that *are* comparable, `+inf` and `-inf`, clamp to the nearer bound like
/// any other out-of-range value: `+inf` to `1.0` (loudest), `-inf` to `0.0`
/// (silence).
///
/// `#[must_use]` guards the contract that a clamped value must be stored:
/// the function's entire purpose is its returned value, so a caller that
/// drops it — `state::clamp_volume(volume);` as a statement — has silently
/// done nothing, leaving the unclamped (possibly NaN) volume in shared
/// state with no error. Making the result `#[must_use]` turns that silent
/// no-op into a compile error, the same way the `f32::clamp` NaN hole is
/// caught by the function itself.
#[must_use]
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

    /// Runs `mutation` on `state` and asserts it leaves `current_track` and
    /// `volume` untouched. The playback mutations — `toggle_playing`, `stop`,
    /// and `toggle_repeat` — each contract that only their flag changes (the
    /// Now Playing bar keeps the interrupted track's title), so the
    /// snapshot-then-compare sequence lives here once instead of at each call
    /// site.
    fn assert_keeps_track_and_volume(state: &mut AppState, mutation: impl FnOnce(&mut AppState)) {
        let track = state.current_track.clone();
        let volume = state.volume;
        mutation(state);
        assert_eq!(state.current_track, track);
        assert_eq!(state.volume, volume);
    }

    #[test]
    fn default_state_is_stopped_at_half_volume() {
        let state = AppState::default();
        assert_eq!(state.current_track, None);
        assert!(!state.is_playing);
        assert!(!state.repeat);
        assert_eq!(state.volume, 0.5);
    }

    #[test]
    fn default_state_is_deterministic() {
        assert_eq!(AppState::default(), AppState::default());
    }

    #[test]
    fn toggle_playing_flips_only_the_playback_flag() {
        let mut state = AppState::default();
        assert!(!state.is_playing);

        assert_keeps_track_and_volume(&mut state, AppState::toggle_playing);
        assert!(state.is_playing);

        assert_keeps_track_and_volume(&mut state, AppState::toggle_playing);
        assert!(!state.is_playing);
    }

    #[test]
    fn toggle_repeat_flips_only_the_repeat_flag() {
        let mut state = AppState::default();
        assert!(!state.repeat);

        assert_keeps_track_and_volume(&mut state, AppState::toggle_repeat);
        assert!(state.repeat);
        assert!(!state.is_playing);

        assert_keeps_track_and_volume(&mut state, AppState::toggle_repeat);
        assert!(!state.repeat);
    }

    #[test]
    fn stop_clears_playing_flag_and_keeps_current_track() {
        let mut state = AppState {
            current_track: Some("song-1".to_string()),
            is_playing: true,
            ..Default::default()
        };
        assert!(state.is_playing);

        assert_keeps_track_and_volume(&mut state, AppState::stop);
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

    #[test]
    fn clamp_volume_caps_positive_infinity_at_loudest() {
        // `+inf` is non-finite but comparable (it exceeds every volume), so
        // `f32::clamp` maps it to the upper bound — full volume, not the
        // silence reserved for the incomparable NaN. Pinned so a refactor of
        // the non-finite handling can't silently change this branch.
        assert_eq!(clamp_volume(f32::INFINITY), 1.0);
    }

    #[test]
    fn clamp_volume_floors_negative_infinity_at_silence() {
        // The `-inf` twin of `+inf`: comparable and below every volume, so
        // it clamps to the lower bound (silence).
        assert_eq!(clamp_volume(f32::NEG_INFINITY), 0.0);
    }
}
