use super::*;

/// Asserts the shared state holds `preset`'s preamp and band gains.
fn assert_curve_applied(state: &AppState, preset: crate::equalizer::Preset) {
    assert_eq!(state.eq_preamp(), preset.preamp);
    assert_eq!(state.eq_bands(), preset.bands);
}

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

// `PresetChoice::Custom` applies the whole curve through the same
// mutate-and-forward path, and the per-band setters leave the shared
// selection `None`, so the closed pick-list label keeps reading `Custom`
// exactly as a hand-tuned curve does.
#[test]
fn a_custom_choice_applies_the_whole_curve_and_keeps_the_selection_none() {
    let (mut player, state) = test_player();
    let mut bands: [f32; BAND_COUNT] = [0.0; BAND_COUNT];
    bands[0] = GAIN_MAX_DB;
    bands[9] = GAIN_MIN_DB;
    let custom = CustomPreset::from_curve(String::from("Very Fuzzed"), 2.5, bands);

    let _ = update(
        &mut player,
        Message::EqPresetSelected(PresetChoice::Custom(custom)),
    );

    let state = state.blocking_lock();
    assert_eq!(state.eq_preamp(), 2.5);
    assert_eq!(state.eq_bands(), bands);
    assert_eq!(state.eq_preset(), None);
}

// The save row applies nothing: it opens the naming panel with an empty box,
// closing every other transient panel, and Enter when the panel is closed is
// a no-op.
#[test]
fn the_save_row_opens_the_panel_and_closes_the_other_panels() {
    let (mut player, state) = test_player();
    let _ = update(
        &mut player,
        Message::EqPresetSelected(PresetChoice::BuiltIn(rock_preset())),
    );

    let _ = update(&mut player, Message::EqPresetSelected(PresetChoice::Save));

    assert!(player.save_preset_open);
    assert!(player.transient_panel_open());
    assert_eq!(player.save_preset_name, "");
    assert_eq!(player.save_preset_note, None);
    let state = state.blocking_lock();
    assert_curve_applied(&state, rock_preset());
    assert_eq!(state.eq_preset(), Some(rock_preset()));

    player.jump_time_visible = true;
    player.save_preset_open = false;
    let _ = update(&mut player, Message::EqPresetSelected(PresetChoice::Save));
    assert!(player.save_preset_open);
    assert!(
        !player.jump_time_visible,
        "the save panel must close Jump to time on opening"
    );
}

#[test]
fn a_closed_save_panel_ignores_its_submit() {
    let (mut player, _state) = test_player();
    player.save_preset_name = "Very Fuzzed".to_string();

    let _ = update(&mut player, Message::SavePresetSubmitted);

    assert!(!player.save_preset_open);
    assert!(player.custom_eq_presets.is_empty());
}

// Enter saves the applied curve — the stored preamp and band gains — under
// the trimmed name and closes the panel. The saved curve is not applied by
// saving: the shared state already holds it.
#[test]
fn save_preset_submitted_saves_the_applied_curve_under_the_typed_name() {
    let (mut player, state) = test_player();
    let _ = update(
        &mut player,
        Message::EqPresetSelected(PresetChoice::BuiltIn(rock_preset())),
    );
    let _ = update(&mut player, Message::EqPresetSelected(PresetChoice::Save));
    let _ = update(
        &mut player,
        Message::SavePresetNameChanged("  Very Fuzzed ".to_string()),
    );

    let _ = update(&mut player, Message::SavePresetSubmitted);

    assert!(!player.save_preset_open);
    assert_eq!(player.save_preset_note, None);
    assert_eq!(player.custom_eq_presets.len(), 1);
    let saved = &player.custom_eq_presets[0];
    assert_eq!(saved.name, "Very Fuzzed");
    assert_eq!(saved.preamp, rock_preset().preamp);
    assert_eq!(saved.bands, rock_preset().bands);
    let state = state.blocking_lock();
    assert_eq!(state.eq_preset(), Some(rock_preset()));
}

// A keystroke clears the note, so the next submit is judged from a clean
// slate, and a name equal to an existing save (any case) updates only that
// save's curve: one row stays and its name is kept as first saved.
#[test]
fn saving_over_an_existing_custom_replaces_only_its_curve() {
    let (mut player, _state) = test_player();
    let first = {
        let mut bands = [0.0; BAND_COUNT];
        bands[0] = 1.0;
        CustomPreset::from_curve(String::from("Very Fuzzed"), 2.5, bands)
    };
    player.custom_eq_presets.push(first);

    let _ = update(&mut player, Message::EqPresetSelected(PresetChoice::Save));
    let _ = update(
        &mut player,
        Message::SavePresetNameChanged("A:name".to_string()),
    );
    let _ = update(&mut player, Message::SavePresetSubmitted);
    assert!(player.save_preset_open);

    let _ = update(
        &mut player,
        Message::SavePresetNameChanged("very fuzzed".to_string()),
    );
    assert_eq!(player.save_preset_note, None);
    let _ = update(&mut player, Message::SavePresetSubmitted);

    assert!(!player.save_preset_open);
    assert_eq!(player.custom_eq_presets.len(), 1);
    assert_eq!(player.custom_eq_presets[0].name, "Very Fuzzed");
    assert_eq!(player.custom_eq_presets[0].preamp, 0.0);
    assert_eq!(player.custom_eq_presets[0].bands, [0.0; BAND_COUNT]);
}
