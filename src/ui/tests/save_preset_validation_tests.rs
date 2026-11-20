use super::*;

// Typing is refused inline and saves nothing: an empty box keeps the panel
// open, a built-in's name in any case shows `Already a preset`, and a colon
// shows `Use it without a colon` because the settings line's separator is
// the colon.
#[test]
fn save_preset_submitted_refuses_empty_builtin_and_colon_names() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::EqPresetSelected(PresetChoice::Save));

    let _ = update(&mut player, Message::SavePresetSubmitted);
    assert!(player.save_preset_open);
    assert_eq!(player.save_preset_note, Some(views::EMPTY_NAME_NOTE));
    assert!(player.custom_eq_presets.is_empty());

    let _ = update(
        &mut player,
        Message::SavePresetNameChanged("rock".to_string()),
    );
    let _ = update(&mut player, Message::SavePresetSubmitted);
    assert!(player.save_preset_open);
    assert_eq!(player.save_preset_note, Some(views::BUILT_IN_NAME_NOTE));
    assert!(player.custom_eq_presets.is_empty());

    let _ = update(
        &mut player,
        Message::SavePresetNameChanged("A:name".to_string()),
    );
    let _ = update(&mut player, Message::SavePresetSubmitted);
    assert!(player.save_preset_open);
    assert_eq!(player.save_preset_note, Some(views::COLON_NAME_NOTE));
    assert!(player.custom_eq_presets.is_empty());
}

// A name carrying a line break (a multi-line paste can put one in the box)
// is refused inline and saves nothing, like a colon name: the settings
// file is line-based, so such a name would split its saved row in two and
// let the name's tail be read as settings lines of its own.
#[test]
fn save_preset_submitted_refuses_a_name_with_a_line_break() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::EqPresetSelected(PresetChoice::Save));

    let _ = update(
        &mut player,
        Message::SavePresetNameChanged("Fuzz\nBass".to_string()),
    );
    let _ = update(&mut player, Message::SavePresetSubmitted);
    assert!(player.save_preset_open);
    assert_eq!(player.save_preset_note, Some(views::LINE_BREAK_NAME_NOTE));
    assert!(player.custom_eq_presets.is_empty());

    let _ = update(
        &mut player,
        Message::SavePresetNameChanged("Fuzz\rBass".to_string()),
    );
    assert_eq!(player.save_preset_note, None);
    let _ = update(&mut player, Message::SavePresetSubmitted);
    assert!(player.save_preset_open);
    assert_eq!(player.save_preset_note, Some(views::LINE_BREAK_NAME_NOTE));
    assert!(player.custom_eq_presets.is_empty());
}
