use super::{AppState, TransportMode, clamp_volume, snap_balance_to_center};
use crate::equalizer::{BAND_COUNT, GAIN_MAX_DB, GAIN_MIN_DB};
use crate::test_support::rock_preset;

/// Runs `mutation` on `state` and asserts it leaves `current_track` and
/// `volume` untouched. Every `AppState` mutation but `play`, `pause`, and
/// the volume setters (`set_volume`, `nudge_volume`) uses this helper or
/// its own isolation test — the playback setters (`toggle_playing`,
/// `stop`, `toggle_repeat`, `toggle_shuffle`) and the equalizer setters
/// (`toggle_equalizer`, `set_eq_preamp`, `set_eq_band`,
/// `apply_eq_preset`) use it directly,
/// while `set_balance` is pinned by `set_balance_changes_only_the_balance`
/// — so the snapshot-then-compare sequence lives here once instead of at
/// each call site. Each test pins its own field's new value separately;
/// this helper only pins the two fields the mutation must not disturb.
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

/// A state whose loaded track, playback flag, Repeat, equalizer flag,
/// preamp, and band-4 gain are all set to non-default values, so a slider
/// setter can be shown to write its own field and nothing else. The volume
/// and balance isolation tests share it, so the fixture stays in lockstep;
/// each caller sets the slider it tests (`set_balance`'s caller also sets
/// volume, to prove that survives).
fn state_with_every_other_field_set() -> AppState {
    let mut state = AppState {
        repeat: true,
        eq_enabled: true,
        ..playing_state()
    };
    state.set_eq_preamp(3.0);
    state.set_eq_band(4, 5.0);
    state
}

/// Asserts the fields [`state_with_every_other_field_set`] sets — the
/// loaded track, the playback flag, Repeat, the equalizer flag, the preamp,
/// and band 4's gain — still hold their fixture values. The two slider
/// isolation tests share it, so neither can stop checking a field the other
/// still covers.
fn assert_track_flags_and_eq_untouched(state: &AppState) {
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(state.is_playing);
    assert!(state.repeat);
    assert!(state.eq_enabled);
    assert_eq!(state.eq_preamp(), 3.0);
    assert_eq!(state.eq_bands()[4], 5.0);
}

#[test]
fn default_state_is_stopped_at_half_volume() {
    let state = AppState::default();
    assert_eq!(state.current_track, None);
    assert!(!state.is_playing);
    assert!(!state.paused);
    assert_eq!(state.transport_mode(), TransportMode::Stopped);
    assert!(!state.repeat);
    assert!(!state.shuffle);
    assert_eq!(state.volume(), 0.5);
    assert_eq!(state.balance(), 0.0);
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
fn toggle_playing_switches_between_playing_and_paused() {
    let mut state = AppState::default();
    assert!(!state.is_playing);
    assert_eq!(state.transport_mode(), TransportMode::Stopped);

    assert_keeps_track_and_volume(&mut state, AppState::toggle_playing);
    assert!(state.is_playing);
    assert!(!state.paused);
    assert_eq!(state.transport_mode(), TransportMode::Playing);

    assert_keeps_track_and_volume(&mut state, AppState::toggle_playing);
    assert!(!state.is_playing);
    assert!(state.paused);
    assert_eq!(state.transport_mode(), TransportMode::Paused);

    assert_keeps_track_and_volume(&mut state, AppState::toggle_playing);
    assert!(state.is_playing);
    assert!(!state.paused);
    assert_eq!(state.transport_mode(), TransportMode::Playing);
}

#[test]
fn pause_sets_the_paused_mode_while_play_and_stop_clear_it() {
    let mut state = playing_state();
    assert_eq!(state.transport_mode(), TransportMode::Playing);

    state.pause();
    assert!(!state.is_playing);
    assert!(state.paused);
    assert_eq!(state.transport_mode(), TransportMode::Paused);

    state.play();
    assert!(state.is_playing);
    assert!(!state.paused);
    assert_eq!(state.transport_mode(), TransportMode::Playing);

    state.pause();
    state.stop();
    assert!(!state.is_playing);
    assert!(!state.paused);
    assert_eq!(state.transport_mode(), TransportMode::Stopped);
}

/// Flips a playback flag twice with `toggle`, reading it through `flag`:
/// off to on to off, with the track, volume, and playback state untouched.
/// The Repeat and Shuffle tests share it.
fn assert_toggle_flips_only_its_flag(toggle: fn(&mut AppState), flag: fn(&AppState) -> bool) {
    let mut state = AppState::default();
    assert!(!flag(&state));

    assert_keeps_track_and_volume(&mut state, toggle);
    assert!(flag(&state));
    assert!(!state.is_playing);

    assert_keeps_track_and_volume(&mut state, toggle);
    assert!(!flag(&state));
}

#[test]
fn toggle_repeat_flips_only_the_repeat_flag() {
    assert_toggle_flips_only_its_flag(AppState::toggle_repeat, |state| state.repeat);
}

#[test]
fn toggle_manual_advance_flips_only_the_manual_advance_flag() {
    assert_toggle_flips_only_its_flag(AppState::toggle_manual_advance, |state| {
        state.manual_advance
    });
}

#[test]
fn toggle_shuffle_flips_only_the_shuffle_flag() {
    assert_toggle_flips_only_its_flag(AppState::toggle_shuffle, |state| state.shuffle);
}

#[test]
fn toggle_eq_auto_flips_only_the_auto_flag_and_keeps_the_curve() {
    let mut state = AppState::default();
    assert!(!state.eq_auto);
    let preset = rock_preset();
    state.apply_eq_preset(preset);

    state.toggle_eq_auto();
    assert!(state.eq_auto);
    assert!(!state.eq_enabled);
    assert_eq!(state.eq_preset(), Some(preset));
    assert_eq!(state.eq_bands(), preset.bands);

    state.toggle_eq_auto();
    assert!(!state.eq_auto);
    assert_eq!(state.eq_preset(), Some(preset));
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
// untouched neighbors are pinned. A regression that stored the gain into
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
fn set_all_eq_bands_clamps_every_band_and_clears_the_preset() {
    let mut state = AppState::default();
    let preset = rock_preset();
    state.apply_eq_preset(preset);

    state.set_all_eq_bands(99.0);
    assert_eq!(
        state.eq_bands(),
        [crate::equalizer::GAIN_MAX_DB; crate::equalizer::BAND_COUNT]
    );
    assert_eq!(state.eq_preamp(), preset.preamp);
    assert_eq!(state.eq_preset(), None);

    state.set_all_eq_bands(0.0);
    assert_eq!(state.eq_bands(), [0.0; crate::equalizer::BAND_COUNT]);
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

/// Applies `rock_preset` and then runs `move_slider`, asserting the
/// selection clears: nudging either slider by hand makes the curve custom,
/// so the pick list must stop naming the preset. The preamp and band
/// setters share that apply→assert-selected→move→assert-cleared sequence,
/// so it lives here once and each call names its own setter.
fn assert_slider_clears_the_preset_selection(move_slider: impl FnOnce(&mut AppState)) {
    let mut state = AppState::default();
    let preset = rock_preset();

    state.apply_eq_preset(preset);
    assert_eq!(state.eq_preset(), Some(preset));
    move_slider(&mut state);
    assert_eq!(state.eq_preset(), None);
}

// Selecting a preset and then nudging either slider by hand makes the
// curve custom, so the selection must clear; otherwise the pick list
// would keep naming a preset the sliders no longer match. Cover both
// slider setters.
#[test]
fn moving_a_slider_clears_the_preset_selection() {
    assert_slider_clears_the_preset_selection(|state| state.set_eq_preamp(1.0));
    assert_slider_clears_the_preset_selection(|state| state.set_eq_band(0, 1.0));
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

#[test]
fn nudge_volume_with_a_nan_delta_falls_to_silence_like_set_volume() {
    // The sum is NaN, so `set_volume`'s clamp applies its silence
    // fallback rather than leaving the old volume or storing the NaN.
    let mut state = AppState::default();
    state.set_volume(0.5);

    state.nudge_volume(f32::NAN);
    assert_eq!(state.volume(), 0.0);
}

// `set_volume` is a mutation `assert_keeps_track_and_volume` cannot guard
// (neither can `nudge_volume`), because volume is the field that helper
// holds constant. Its three
// tests above assert only the stored volume, so a regression that also
// cleared `current_track` (blanking the Now Playing bar mid-drag) or reset
// a playback or EQ flag would pass all of them. Pin that the volume setter
// writes volume alone, against a state whose every other field is set.
#[test]
fn set_volume_changes_only_the_volume() {
    let mut state = state_with_every_other_field_set();

    state.set_volume(0.9);

    assert_eq!(state.volume(), 0.9);
    assert_track_flags_and_eq_untouched(&state);
}

#[test]
fn set_balance_stores_an_in_range_value() {
    let mut state = AppState::default();
    state.set_balance(-0.4);
    assert_eq!(state.balance(), -0.4);
}

#[test]
fn set_balance_clamps_out_of_range_values() {
    let mut state = AppState::default();
    state.set_balance(1.5);
    assert_eq!(state.balance(), 1.0);
    state.set_balance(-2.0);
    assert_eq!(state.balance(), -1.0);
}

#[test]
fn set_balance_maps_nan_to_center() {
    let mut state = AppState::default();
    state.set_balance(f32::NAN);
    assert_eq!(state.balance(), 0.0);
}

// `set_balance` is a mutation `assert_keeps_track_and_volume` cannot guard,
// because balance is not a field that helper holds constant. Its three
// tests above assert only the stored balance, so a regression that also
// cleared `current_track` or reset a playback or EQ flag would pass all of
// them. Pin that the balance setter writes balance alone, against a state
// whose every other field is set.
#[test]
fn set_balance_changes_only_the_balance() {
    let mut state = state_with_every_other_field_set();
    state.set_volume(0.9);

    state.set_balance(-0.5);

    assert_eq!(state.balance(), -0.5);
    assert_eq!(state.volume(), 0.9);
    assert_track_flags_and_eq_untouched(&state);
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

#[test]
fn snap_balance_to_center_snaps_an_inward_move_near_center() {
    assert_eq!(snap_balance_to_center(0.3, 0.05), 0.0);
    assert_eq!(snap_balance_to_center(-0.3, -0.09), 0.0);
    // The boundary itself is not snapped.
    assert_eq!(snap_balance_to_center(0.3, 0.1), 0.1);
}

#[test]
fn snap_balance_to_center_never_snaps_an_outward_move() {
    assert_eq!(snap_balance_to_center(0.0, 0.01), 0.01);
    assert_eq!(snap_balance_to_center(0.0, -0.05), -0.05);
    assert_eq!(snap_balance_to_center(0.05, 0.06), 0.06);
    assert_eq!(snap_balance_to_center(0.5, f32::NAN), 0.0);
    assert_eq!(snap_balance_to_center(0.5, 2.0), 1.0);
}
