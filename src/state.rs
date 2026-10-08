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
    /// The id of the track the player currently has loaded, or `None` before
    /// anything has played. The Now Playing bar names it and the Songs view
    /// marks its row; [`AppState::stop`] leaves it set so the bar keeps
    /// showing the interrupted track's title.
    pub current_track: Option<String>,
    /// Whether playback is running. The transport row's Play/Pause button
    /// reads it to choose its "Pause"/"Play" label, and [`AppState::stop`]
    /// clears it without clearing the current track.
    pub is_playing: bool,
    /// Whether Previous/Next wrap around the current album's ends (Repeat on)
    /// or stop at the edge (Repeat off). Starts off, as in Winamp.
    pub repeat: bool,
    /// Playback volume in `[0.0, 1.0]`, never NaN. Kept private so the only
    /// way to change it is [`AppState::set_volume`], which clamps. Read it
    /// with [`AppState::volume`].
    volume: f32,
    /// Whether the equalizer is engaged. Starts off, as in Winamp; the band
    /// gains below are stored regardless so turning it back on restores them.
    pub eq_enabled: bool,
    /// The preamp gain in decibels applied ahead of the bands, in
    /// `[equalizer::GAIN_MIN_DB, equalizer::GAIN_MAX_DB]`. Kept private like
    /// `volume`: [`AppState::set_eq_preamp`] and
    /// [`AppState::apply_eq_preset`] are its writers, both through
    /// [`equalizer::clamp_gain`], and [`AppState::eq_preamp`] is its only
    /// reader.
    eq_preamp: f32,
    /// The gain in decibels of each of the [`equalizer::BAND_COUNT`] bands,
    /// low frequency to high; flat (all `0.0`) by default. Kept private like
    /// `volume`: [`AppState::set_eq_band`] and
    /// [`AppState::apply_eq_preset`] are its writers, both through
    /// [`equalizer::clamp_gain`], and [`AppState::eq_bands`] is its only
    /// reader.
    eq_bands: [f32; equalizer::BAND_COUNT],
    /// The preset whose whole curve is currently applied, or `None` for a
    /// custom curve. [`AppState::apply_eq_preset`] is its only setter to
    /// `Some`; the hand-moved sliders (`set_eq_preamp`, `set_eq_band`) clear
    /// it to `None`, since a manual move makes the curve custom. Read it with
    /// [`AppState::eq_preset`].
    eq_preset: Option<equalizer::Preset>,
}

/// The one production `AppState` is built in the app's entry point (`main`)
/// and shared (as an `Arc<Mutex<_>>`) with both the UI and the Apple Music
/// service; the test modules build their own. A single `Default` keeps every
/// construction site in sync: a new field's startup value is set here, in one
/// place, instead of in a struct literal repeated at each site. The starting
/// volume is 0.5, not the derived 0.0, so a manual impl is required.
///
/// Initial state: nothing loaded, stopped, Repeat off, at 50% volume, and the
/// equalizer off with a flat (all-zero) curve and no preset selected.
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
            eq_preset: None,
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

    /// Set the playback flag, leaving `current_track` in place so the Now
    /// Playing bar keeps naming the track. The keyboard's X key calls this
    /// (through `Message::Play`); unlike [`AppState::toggle_playing`] it is an
    /// explicit, idempotent set, so a second X press keeps playing rather
    /// than pausing. Pause and [`AppState::stop`] are the same flag change
    /// today; the seam is the distinction — once real playback lands, Pause
    /// keeps the track position while Stop resets it.
    pub fn play(&mut self) {
        self.is_playing = true;
    }

    /// Clear the playback flag, leaving `current_track` in place so the Now
    /// Playing bar keeps showing the interrupted track's title (matching
    /// Winamp). The keyboard's C key calls this (through `Message::Pause`);
    /// unlike [`AppState::toggle_playing`] it is an explicit, idempotent set,
    /// so a second C press keeps the player paused rather than resuming.
    pub fn pause(&mut self) {
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
    /// the field is private and both writers go through `clamp_gain` (this
    /// setter and [`AppState::apply_eq_preset`]), so the clamp holds no
    /// matter which caller stores a gain. A hand-moved slider is a custom
    /// curve, so this clears any applied preset selection.
    pub fn set_eq_preamp(&mut self, gain: f32) {
        self.eq_preamp = equalizer::clamp_gain(gain);
        self.eq_preset = None;
    }

    /// Store a clamped gain into band `band`. An out-of-range `band` index
    /// (a stale slider message after the band count shrinks, say) is ignored
    /// rather than panicking: `get_mut` yields `None` and nothing is stored,
    /// so a bad index can't take the window down mid-drag. The field is
    /// private and both writers go through `clamp_gain` (this setter and
    /// [`AppState::apply_eq_preset`]), so every stored band gain is clamped.
    /// A hand-moved slider is a custom curve, so a stored gain clears
    /// any applied preset selection; an ignored out-of-range index stores
    /// nothing and leaves the selection alone.
    pub fn set_eq_band(&mut self, band: usize, gain: f32) {
        if let Some(slot) = self.eq_bands.get_mut(band) {
            *slot = equalizer::clamp_gain(gain);
            self.eq_preset = None;
        }
    }

    /// Apply a whole preset curve: store its preamp and every band gain
    /// (each still through [`equalizer::clamp_gain`], so a preset can never
    /// push shared state out of range) and remember the selection. The UI's
    /// equalizer pick list is the only caller; a later hand move of the
    /// preamp or a band clears the selection (see [`AppState::set_eq_preamp`]
    /// and [`AppState::set_eq_band`]).
    pub fn apply_eq_preset(&mut self, preset: equalizer::Preset) {
        self.eq_preamp = equalizer::clamp_gain(preset.preamp);
        self.eq_bands = preset.bands.map(equalizer::clamp_gain);
        self.eq_preset = Some(preset);
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

    /// The applied equalizer preset, or `None` for a custom curve. The UI's
    /// `view` reads it to show the selected preset's name in the pick list.
    #[must_use]
    pub fn eq_preset(&self) -> Option<equalizer::Preset> {
        self.eq_preset
    }

    /// Stores `volume`, clamped to `[0.0, 1.0]` with NaN mapped to silence by
    /// [`clamp_volume`]. The field is private and this is its only writer, so
    /// the clamped invariant holds no matter which caller (today, the UI's
    /// `VolumeChange` arm and [`AppState::nudge_volume`]) sets it.
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = clamp_volume(volume);
    }

    /// Moves the volume by `delta`, clamped to the valid range by
    /// [`AppState::set_volume`]. The UI's arrow-key arms call this with
    /// `+views::VOLUME_STEP` and `-views::VOLUME_STEP`, so the keyboard and
    /// the slider share one granularity and one clamp.
    pub fn nudge_volume(&mut self, delta: f32) {
        self.set_volume(self.volume + delta);
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
    use crate::test_support::rock_preset;

    /// Runs `mutation` on `state` and asserts it leaves `current_track` and
    /// `volume` untouched. Every `AppState` mutation but `play`, `pause`, and
    /// `set_volume` uses this helper — the playback setters (`toggle_playing`,
    /// `stop`, `toggle_repeat`) and the equalizer setters (`toggle_equalizer`,
    /// `set_eq_preamp`, `set_eq_band`, `apply_eq_preset`) — so the
    /// snapshot-then-compare sequence lives here once instead of at each call
    /// site. Each test pins
    /// its own field's new value separately; this helper only pins the two
    /// fields the mutation must not disturb.
    fn assert_keeps_track_and_volume(state: &mut AppState, mutation: impl FnOnce(&mut AppState)) {
        let track = state.current_track.clone();
        let volume = state.volume();
        mutation(state);
        assert_eq!(state.current_track, track);
        assert_eq!(state.volume(), volume);
    }

    /// A state with `song-1` loaded and playing — the non-default starting
    /// point the `stop` and `set_volume` isolation tests both need, so the
    /// mutation has a current track and a set playback flag to preserve.
    /// Building it here once keeps the two fixtures in lockstep.
    fn playing_state() -> AppState {
        AppState {
            current_track: Some("song-1".to_string()),
            is_playing: true,
            ..Default::default()
        }
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
        assert_eq!(state.eq_preset(), None);
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

    // `set_volume`'s NaN-to-silence mapping is pinned at the state level, but
    // the preamp setter's NaN mapping is only pinned in `clamp_gain`'s own
    // test. `set_eq_preamp` stores a caller-supplied gain, so pin the flat
    // fallback here too: a refactor that bypassed `clamp_gain` (an inline
    // `gain.clamp(..)`, say) would still pass the out-of-range tests above
    // while letting NaN into shared state and the preamp slider.
    #[test]
    fn set_eq_preamp_maps_nan_to_flat() {
        let mut state = AppState::default();
        state.set_eq_preamp(f32::NAN);
        assert_eq!(state.eq_preamp(), 0.0);
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

    // The out-of-range guard must leave the applied preset selection alone: a
    // stale slider message for a band index that no longer exists stores
    // nothing, so the pick list must keep naming the preset. The test above
    // starts with no selection, so a regression that cleared `eq_preset`
    // before the `get_mut` guard would pass it while silently deselecting the
    // preset mid-drag; pin the selection and the untouched curve here.
    #[test]
    fn set_eq_band_out_of_range_keeps_the_preset_selection() {
        let mut state = AppState::default();
        let preset = rock_preset();

        state.apply_eq_preset(preset);
        assert_keeps_track_and_volume(&mut state, |state| state.set_eq_band(BAND_COUNT, 5.0));

        assert_eq!(state.eq_preset(), Some(preset));
        assert_eq!(state.eq_bands(), preset.bands);
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

    // The band twin of `set_eq_preamp_maps_nan_to_flat`: `set_eq_band` stores
    // a caller-supplied gain, so pin that a NaN gain lands as flat rather
    // than in the stored curve.
    #[test]
    fn set_eq_band_maps_nan_to_flat() {
        let mut state = AppState::default();
        state.set_eq_band(0, f32::NAN);
        assert_eq!(state.eq_bands()[0], 0.0);
    }

    #[test]
    fn apply_eq_preset_stores_the_whole_curve_and_the_selection() {
        let mut state = AppState::default();
        let preset = rock_preset();

        assert_keeps_track_and_volume(&mut state, |state| state.apply_eq_preset(preset));

        assert_eq!(state.eq_preset(), Some(preset));
        assert_eq!(state.eq_preamp(), preset.preamp);
        assert_eq!(state.eq_bands(), preset.bands);
    }

    // Selecting a preset and then nudging either slider by hand makes the
    // curve custom, so the selection must clear; otherwise the pick list
    // would keep naming a preset the sliders no longer match. Cover both
    // slider setters.
    #[test]
    fn moving_a_slider_clears_the_preset_selection() {
        let mut state = AppState::default();
        let preset = rock_preset();

        state.apply_eq_preset(preset);
        assert_eq!(state.eq_preset(), Some(preset));
        state.set_eq_preamp(1.0);
        assert_eq!(state.eq_preset(), None);

        state.apply_eq_preset(preset);
        assert_eq!(state.eq_preset(), Some(preset));
        state.set_eq_band(0, 1.0);
        assert_eq!(state.eq_preset(), None);
    }

    #[test]
    fn stop_clears_playing_flag_and_keeps_current_track() {
        let mut state = playing_state();
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
    fn nudge_volume_moves_by_the_delta_and_clamps() {
        let mut state = AppState::default();
        state.set_volume(0.5);

        state.nudge_volume(0.01);
        assert_eq!(state.volume(), 0.51);
        state.nudge_volume(-0.01);
        assert_eq!(state.volume(), 0.5);

        state.nudge_volume(1.0);
        assert_eq!(state.volume(), 1.0);
        state.nudge_volume(-2.0);
        assert_eq!(state.volume(), 0.0);
    }

    // `set_volume` is the one mutation `assert_keeps_track_and_volume` cannot
    // guard, because volume is the field that helper holds constant. Its three
    // tests above assert only the stored volume, so a regression that also
    // cleared `current_track` (blanking the Now Playing bar mid-drag) or reset
    // a playback or EQ flag would pass all of them. Pin that the volume setter
    // writes volume alone, against a state whose every other field is set.
    #[test]
    fn set_volume_changes_only_the_volume() {
        let mut state = AppState {
            repeat: true,
            eq_enabled: true,
            ..playing_state()
        };
        state.set_eq_preamp(3.0);
        state.set_eq_band(4, 5.0);

        state.set_volume(0.9);

        assert_eq!(state.volume(), 0.9);
        assert_eq!(state.current_track.as_deref(), Some("song-1"));
        assert!(state.is_playing);
        assert!(state.repeat);
        assert!(state.eq_enabled);
        assert_eq!(state.eq_preamp(), 3.0);
        assert_eq!(state.eq_bands()[4], 5.0);
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
