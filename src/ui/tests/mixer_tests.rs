use super::*;

#[test]
fn toggle_equalizer_flips_shared_state() {
    assert_toggles_shared_state(Message::ToggleEqualizer, |state| state.eq_enabled);
}

#[test]
fn eq_preamp_change_clamps_value_before_storing() {
    assert_message_clamps(
        Message::EqPreampChange(99.0),
        |state| state.eq_preamp(),
        GAIN_MAX_DB,
    );
    assert_message_clamps(
        Message::EqPreampChange(-99.0),
        |state| state.eq_preamp(),
        GAIN_MIN_DB,
    );
}

#[test]
fn eq_band_change_clamps_value_before_storing() {
    assert_message_clamps(
        Message::EqBandChange(0, 99.0),
        |state| state.eq_bands()[0],
        GAIN_MAX_DB,
    );
    assert_message_clamps(
        Message::EqBandChange(1, -99.0),
        |state| state.eq_bands()[1],
        GAIN_MIN_DB,
    );
}

// The EqPresetSelected arm writes a whole curve and records the selection,
// which the one-value slider-arm tests above do not cover. Drive a preset
// through `update` and read all three shared-state fields back.
#[test]
fn eq_preset_selected_applies_the_curve_and_selection() {
    let (mut player, state) = test_player();
    let preset = rock_preset();

    let _ = update(
        &mut player,
        Message::EqPresetSelected(PresetChoice::BuiltIn(preset)),
    );

    let state = state.blocking_lock();
    assert_curve_applied(&state, preset);
    assert_eq!(state.eq_preset(), Some(preset));
}

#[test]
fn eq_scale_click_sets_every_band_and_keeps_the_preamp() {
    let (mut player, state) = test_player();
    let _ = update(
        &mut player,
        Message::EqPresetSelected(PresetChoice::BuiltIn(rock_preset())),
    );

    let _ = update(&mut player, Message::EqBandsSetAll(GAIN_MIN_DB));

    let state = state.blocking_lock();
    assert_eq!(state.eq_bands(), [GAIN_MIN_DB; BAND_COUNT]);
    assert_eq!(state.eq_preamp(), rock_preset().preamp);
    assert_eq!(state.eq_preset(), None);
    assert_eq!(player.mixer_drag, None);
}

#[test]
fn volume_change_clamps_value_before_storing() {
    // Out-of-range slider values are clamped by the update arm.
    assert_message_clamps(Message::VolumeChange(1.5), |state| state.volume(), 1.0);
    assert_message_clamps(Message::VolumeChange(-0.2), |state| state.volume(), 0.0);

    // An in-range value is stored as-is.
    assert_message_clamps(Message::VolumeChange(0.3), |state| state.volume(), 0.3);
}

#[test]
fn balance_change_clamps_value_before_storing() {
    // Out-of-range slider values are clamped by the update arm.
    assert_message_clamps(Message::BalanceChange(1.5), |state| state.balance(), 1.0);
    assert_message_clamps(Message::BalanceChange(-2.0), |state| state.balance(), -1.0);

    // An in-range value is stored as-is.
    assert_message_clamps(Message::BalanceChange(-0.4), |state| state.balance(), -0.4);
}
