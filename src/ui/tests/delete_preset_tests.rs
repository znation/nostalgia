use super::*;

// The panel's `Delete` button removes the saved custom matching the typed
// name — any case — and closes the panel, keeping the other customs in
// stored order.
#[test]
fn delete_preset_submitted_removes_the_named_custom_preset() {
    let (mut player, _state) = test_player();
    player.custom_eq_presets.push(CustomPreset::from_curve(
        String::from("Very Fuzzed"),
        2.5,
        [0.0; BAND_COUNT],
    ));
    player.custom_eq_presets.push(CustomPreset::from_curve(
        String::from("Morning Drive"),
        -1.0,
        [0.0; BAND_COUNT],
    ));

    let _ = update(&mut player, Message::EqPresetSelected(PresetChoice::Save));
    let _ = update(
        &mut player,
        Message::SavePresetNameChanged("VERY FUZZED".to_string()),
    );
    let _ = update(&mut player, Message::DeletePresetSubmitted);

    assert!(!player.save_preset_open);
    assert_eq!(player.save_preset_note, None);
    assert_eq!(player.custom_eq_presets.len(), 1);
    assert_eq!(player.custom_eq_presets[0].name, "Morning Drive");
    assert_eq!(player.custom_eq_presets[0].preamp, -1.0);
    assert_eq!(player.custom_eq_presets[0].bands, [0.0; BAND_COUNT]);
}

// Deletion is refused inline and removes nothing, exactly matching the save
// path's refusal shapes: an empty name keeps the panel open with
// `Type a name`, a colon shows `Use it without a colon`, a line break shows
// `Use one line`, a built-in's name (any case) shows `Already a preset`
// because built-ins refuse deletion, and a name no saved custom carries
// shows `No such preset`.
#[test]
fn delete_preset_submitted_refuses_empty_builtin_colon_and_unknown_names() {
    let (mut player, _state) = test_player();
    player.custom_eq_presets.push(CustomPreset::from_curve(
        String::from("Morning Drive"),
        -1.0,
        [0.0; BAND_COUNT],
    ));
    let _ = update(&mut player, Message::EqPresetSelected(PresetChoice::Save));

    let _ = update(&mut player, Message::SavePresetNameChanged(String::new()));
    let _ = update(&mut player, Message::DeletePresetSubmitted);
    assert!(player.save_preset_open);
    assert_eq!(player.save_preset_note, Some(views::EMPTY_NAME_NOTE));
    assert_eq!(player.custom_eq_presets.len(), 1);

    let _ = update(
        &mut player,
        Message::SavePresetNameChanged("A:name".to_string()),
    );
    let _ = update(&mut player, Message::DeletePresetSubmitted);
    assert!(player.save_preset_open);
    assert_eq!(player.save_preset_note, Some(views::COLON_NAME_NOTE));
    assert_eq!(player.custom_eq_presets.len(), 1);

    let _ = update(
        &mut player,
        Message::SavePresetNameChanged("Fuzz\nBass".to_string()),
    );
    let _ = update(&mut player, Message::DeletePresetSubmitted);
    assert!(player.save_preset_open);
    assert_eq!(player.save_preset_note, Some(views::LINE_BREAK_NAME_NOTE));
    assert_eq!(player.custom_eq_presets.len(), 1);

    let _ = update(
        &mut player,
        Message::SavePresetNameChanged("rock".to_string()),
    );
    let _ = update(&mut player, Message::DeletePresetSubmitted);
    assert!(player.save_preset_open);
    assert_eq!(player.save_preset_note, Some(views::BUILT_IN_NAME_NOTE));
    assert_eq!(player.custom_eq_presets.len(), 1);

    let _ = update(
        &mut player,
        Message::SavePresetNameChanged("Never Saved".to_string()),
    );
    let _ = update(&mut player, Message::DeletePresetSubmitted);
    assert!(player.save_preset_open);
    assert_eq!(player.save_preset_note, Some(views::NO_SUCH_PRESET_NOTE));
    assert_eq!(player.custom_eq_presets.len(), 1);
}

// A keystroke clears the note before the next delete, and with the panel
// closed `Delete` does nothing at all.
#[test]
fn delete_preset_submitted_is_a_no_op_with_the_panel_closed() {
    let (mut player, _state) = test_player();
    player.custom_eq_presets.push(CustomPreset::from_curve(
        String::from("Morning Drive"),
        -1.0,
        [0.0; BAND_COUNT],
    ));
    player.save_preset_name = "Morning Drive".to_string();

    let _ = update(&mut player, Message::DeletePresetSubmitted);

    assert!(!player.save_preset_open);
    assert_eq!(player.custom_eq_presets.len(), 1);
    assert_eq!(player.custom_eq_presets[0].name, "Morning Drive");
}

// Deleting the currently applied custom's name touches only the saved
// list: the shared state keeps the applied curve, the pick-list label keeps
// reading `Custom`, and AUTO, `eq_preset=`, and the live sliders are
// untouched.
#[test]
fn deleting_the_applied_custom_leaves_the_applied_curve_untouched() {
    let (mut player, state) = test_player();
    let mut bands: [f32; BAND_COUNT] = [0.0; BAND_COUNT];
    bands[0] = GAIN_MAX_DB;
    let custom = CustomPreset::from_curve(String::from("Very Fuzzed"), 2.5, bands);
    let _ = update(
        &mut player,
        Message::EqPresetSelected(PresetChoice::Custom(custom.clone())),
    );
    player.custom_eq_presets.push(custom);
    let _ = update(&mut player, Message::EqPresetSelected(PresetChoice::Save));
    let _ = update(
        &mut player,
        Message::SavePresetNameChanged("Very Fuzzed".to_string()),
    );
    let _ = update(&mut player, Message::DeletePresetSubmitted);

    assert!(!player.save_preset_open);
    assert!(player.custom_eq_presets.is_empty());
    let state = state.blocking_lock();
    assert_eq!(state.eq_preamp(), 2.5);
    assert_eq!(state.eq_bands(), bands);
    assert_eq!(state.eq_preset(), None);
}
