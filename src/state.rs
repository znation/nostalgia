//! Shared application state, the single source of truth for playback
//! status. Both the Apple Music service and the UI read and mutate it,
//! so it lives in its own module rather than in the binary's entry point.

use crate::clamp;
use crate::equalizer;

/// Global playback state shared between the Apple Music service and the UI.
///
/// `Debug` and `PartialEq` support equality assertions on whole `AppState`
/// values, such as the default-state tests below.
#[derive(Debug, PartialEq)]
pub struct AppState {
    pub current_track: Option<String>,
    pub is_playing: bool,
    /// Whether Previous/Next wrap around the current album's ends (Repeat on)
    /// or stop at the edge (Repeat off). Starts off, as in Winamp.
    pub repeat: bool,
    /// Playback volume in `[0.0, 1.0]`, never NaN. Kept private — unlike the
    /// other fields, which have no validity range — so the only way to change
    /// it is [`AppState::set_volume`], which clamps. Read it with
    /// [`AppState::volume`].
    volume: f32,
    /// Whether the equalizer is engaged. Starts off, as in Winamp; the band
    /// gains below are stored regardless so turning it back on restores them.
    pub eq_enabled: bool,
    /// The preamp gain in decibels applied ahead of the bands, in
    /// `[equalizer::GAIN_MIN_DB, equalizer::GAIN_MAX_DB]`. Kept private like
    /// `volume`: [`AppState::set_eq_preamp`] is its only writer and clamps,
    /// and [`AppState::eq_preamp`] is its only reader.
    eq_preamp: f32,
    /// The gain in decibels of each of the [`equalizer::BAND_COUNT`] bands,
    /// low frequency to high; flat (all `0.0`) by default. Kept private like
    /// `volume`: [`AppState::set_eq_band`] is its only writer and clamps, and
    /// [`AppState::eq_bands`] is its only reader.
    eq_bands: [f32; equalizer::BAND_COUNT],
}

/// The one production `AppState` is built in the app's entry point (`main`)
/// and shared (as an `Arc<Mutex<_>>`) with both the UI and the Apple Music
/// service; the test modules build their own. A single `Default` keeps every
/// construction site in sync: a new field's startup value is set here, in one
/// place, instead of in a struct literal repeated at each site. The starting
/// volume is 0.5, not the derived 0.0, so a manual impl is required.
///
/// Initial state: nothing loaded, stopped, Repeat off, at 50% volume, and the
/// equalizer off with a flat (all-zero) curve.
impl Default for AppState {
    fn default() -> Self {
        Self {
            current_track: None,
            is_playing: false,
            repeat: false,
            volume: 0.5,
            eq_enabled: false,
            eq_preamp: 0.0,
            eq_bands: [0.0; equalizer::BAND_COUNT],
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

    /// Flip the equalizer's on/off flag in place. The UI's EQ button is the
    /// only toggle caller; keeping the flip here (rather than inlined at the
    /// call site) puts the toggling semantics next to the field they mutate,
    /// like `toggle_repeat`. The stored preamp and band gains are left alone,
    /// so re-enabling the equalizer restores the curve the user dialed in.
    pub fn toggle_equalizer(&mut self) {
        self.eq_enabled = !self.eq_enabled;
    }

    /// Store a preamp gain, clamped to the valid range via
    /// [`equalizer::clamp_gain`]. The UI's preamp slider is the only caller;
    /// the field is private and this is its only writer, so the clamp holds
    /// no matter which caller stores a gain.
    pub fn set_eq_preamp(&mut self, gain: f32) {
        self.eq_preamp = equalizer::clamp_gain(gain);
    }

    /// Store a clamped gain into band `band`. An out-of-range `band` index
    /// (a stale slider message after the band count shrinks, say) is ignored
    /// rather than panicking: `get_mut` yields `None` and nothing is stored,
    /// so a bad index can't take the window down mid-drag. The field is
    /// private and this is its only writer, so every stored band gain is
    /// clamped.
    pub fn set_eq_band(&mut self, band: usize, gain: f32) {
        if let Some(slot) = self.eq_bands.get_mut(band) {
            *slot = equalizer::clamp_gain(gain);
        }
    }

    /// The current playback volume, always in `[0.0, 1.0]` (see
    /// [`AppState::set_volume`]). The UI's `view` reads it for the slider.
    #[must_use]
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// The current preamp gain in decibels, always within
    /// `[equalizer::GAIN_MIN_DB, equalizer::GAIN_MAX_DB]` (see
    /// [`AppState::set_eq_preamp`]). The UI's `view` reads it for the preamp
    /// slider.
    #[must_use]
    pub fn eq_preamp(&self) -> f32 {
        self.eq_preamp
    }

    /// The current per-band gains in decibels, low frequency to high, each
    /// always within `[equalizer::GAIN_MIN_DB, equalizer::GAIN_MAX_DB]` (see
    /// [`AppState::set_eq_band`]). The UI's `view` reads them for the band
    /// sliders.
    #[must_use]
    pub fn eq_bands(&self) -> [f32; equalizer::BAND_COUNT] {
        self.eq_bands
    }

    /// Stores `volume`, clamped to `[0.0, 1.0]` with NaN mapped to silence by
    /// [`clamp_volume`]. The field is private and this is its only writer, so
    /// the clamped invariant holds no matter which caller (today, the UI's
    /// `VolumeChange` arm) sets it.
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = clamp_volume(volume);
    }
}

/// Clamps a volume value to the valid `[0.0, 1.0]` range, mapping a NaN to
/// `0.0` (silence).
///
/// iced's slider can emit a value outside the range (a drag beyond the ends,
/// or a stale in-flight change), and the UI must never store an unclamped
/// volume in `AppState`. [`AppState::set_volume`] is the field's only writer
/// and calls this, so the rule lives here as a pure function beside the state
/// it protects rather than at each call site. The NaN-and-bounds behaviour
/// itself is [`crate::clamp::clamp_with_nan_fallback`], shared with
/// `equalizer::clamp_gain`; this wrapper only names volume's range and its
/// silence fallback.
#[must_use]
pub fn clamp_volume(volume: f32) -> f32 {
    clamp::clamp_with_nan_fallback(volume, 0.0, 1.0, 0.0)
}

#[cfg(test)]
mod tests {
    use super::{AppState, clamp_volume};
    use crate::equalizer::{BAND_COUNT, GAIN_MAX_DB, GAIN_MIN_DB};

    /// Runs `mutation` on `state` and asserts it leaves `current_track` and
    /// `volume` untouched. The playback mutations — `toggle_playing`, `stop`,
    /// and `toggle_repeat` — each contract that only their flag changes (the
    /// Now Playing bar keeps the interrupted track's title), so the
    /// snapshot-then-compare sequence lives here once instead of at each call
    /// site.
    fn assert_keeps_track_and_volume(state: &mut AppState, mutation: impl FnOnce(&mut AppState)) {
        let track = state.current_track.clone();
        let volume = state.volume();
        mutation(state);
        assert_eq!(state.current_track, track);
        assert_eq!(state.volume(), volume);
    }

    #[test]
    fn default_state_is_stopped_at_half_volume() {
        let state = AppState::default();
        assert_eq!(state.current_track, None);
        assert!(!state.is_playing);
        assert!(!state.repeat);
        assert_eq!(state.volume(), 0.5);
        assert!(!state.eq_enabled);
        assert_eq!(state.eq_preamp(), 0.0);
        assert_eq!(state.eq_bands(), [0.0; BAND_COUNT]);
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
    fn toggle_equalizer_flips_only_the_eq_flag() {
        let mut state = AppState::default();
        assert!(!state.eq_enabled);
        let preamp = state.eq_preamp();
        let bands = state.eq_bands();

        assert_keeps_track_and_volume(&mut state, AppState::toggle_equalizer);
        assert!(state.eq_enabled);
        assert!(!state.is_playing);

        assert_keeps_track_and_volume(&mut state, AppState::toggle_equalizer);
        assert!(!state.eq_enabled);
        // Toggling never disturbs the stored curve, so re-enabling restores it.
        assert_eq!(state.eq_preamp(), preamp);
        assert_eq!(state.eq_bands(), bands);
    }

    #[test]
    fn set_eq_preamp_clamps_and_keeps_track_and_volume() {
        let mut state = AppState::default();

        assert_keeps_track_and_volume(&mut state, |state| state.set_eq_preamp(99.0));
        assert_eq!(state.eq_preamp(), GAIN_MAX_DB);

        assert_keeps_track_and_volume(&mut state, |state| state.set_eq_preamp(-99.0));
        assert_eq!(state.eq_preamp(), GAIN_MIN_DB);

        assert_keeps_track_and_volume(&mut state, |state| state.set_eq_preamp(3.5));
        assert_eq!(state.eq_preamp(), 3.5);
    }

    #[test]
    fn set_eq_band_clamps_the_stored_gain() {
        let mut state = AppState::default();

        assert_keeps_track_and_volume(&mut state, |state| state.set_eq_band(0, 99.0));
        assert_eq!(state.eq_bands()[0], GAIN_MAX_DB);

        assert_keeps_track_and_volume(&mut state, |state| state.set_eq_band(9, -99.0));
        assert_eq!(state.eq_bands()[9], GAIN_MIN_DB);
    }

    #[test]
    fn set_eq_band_ignores_an_out_of_range_band_index() {
        let mut state = AppState::default();

        assert_keeps_track_and_volume(&mut state, |state| state.set_eq_band(BAND_COUNT, 5.0));
        assert_eq!(state.eq_bands(), [0.0; BAND_COUNT]);
    }

    // The `set_eq_band` tests above use only out-of-range gains and assert
    // only the addressed slot, so neither the in-range pass-through nor the
    // untouched neighbours are pinned. A regression that stored the gain into
    // every band (or wrote the preamp field instead) would clear those tests
    // while corrupting the whole curve, so pin the single-slot write: band 3
    // takes the in-range gain and the other nine bands and the preamp stay
    // flat.
    #[test]
    fn set_eq_band_writes_only_the_addressed_band() {
        let mut state = AppState::default();

        assert_keeps_track_and_volume(&mut state, |state| state.set_eq_band(3, 4.5));

        let mut expected = [0.0; BAND_COUNT];
        expected[3] = 4.5;
        assert_eq!(state.eq_bands(), expected);
        assert_eq!(state.eq_preamp(), 0.0);
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
    fn set_volume_stores_an_in_range_value() {
        let mut state = AppState::default();
        state.set_volume(0.3);
        assert_eq!(state.volume(), 0.3);
    }

    #[test]
    fn set_volume_clamps_out_of_range_values() {
        let mut state = AppState::default();
        state.set_volume(1.5);
        assert_eq!(state.volume(), 1.0);
        state.set_volume(-0.2);
        assert_eq!(state.volume(), 0.0);
    }

    #[test]
    fn set_volume_maps_nan_to_silence() {
        let mut state = AppState::default();
        state.set_volume(f32::NAN);
        assert_eq!(state.volume(), 0.0);
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
        // The shared clamp handles the NaN branch; this pins that
        // `clamp_volume` passes the silence fallback.
        assert_eq!(clamp_volume(f32::NAN), 0.0);
    }
}
