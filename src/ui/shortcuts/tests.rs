use super::*;

/// Builds a `KeyPressed` event for `key` with `modifiers`. The fields the
/// shortcut mapper ignores (`modified_key`, `physical_key`, `location`,
/// `text`, `repeat`) are filled with neutral values, so each mapping test
/// names only the key and modifiers it cares about.
fn key_pressed(key: Key, modifiers: Modifiers) -> Event {
    Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: key::Physical::Code(key::Code::KeyA),
        location: iced::keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    }
}

/// Like `key_pressed`, but at `Location::Numpad`.
fn numpad_pressed(key: Key, modifiers: Modifiers) -> Event {
    Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: key::Physical::Code(key::Code::Numpad4),
        location: Location::Numpad,
        modifiers,
        text: None,
        repeat: false,
    }
}

/// Like `key_pressed`, but with an explicit physical key and location.
fn digit_pressed(key: Key, code: key::Code, location: Location, modifiers: Modifiers) -> Event {
    Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: key::Physical::Code(code),
        location,
        modifiers,
        text: None,
        repeat: false,
    }
}

#[test]
fn options_shortcut_maps_alt_o_and_rejects_other_chords() {
    let physical = key::Physical::Code(key::Code::KeyO);
    assert!(matches!(
        options_shortcut(&physical, Modifiers::ALT),
        Some(Message::ToggleOptions)
    ));
    assert!(matches!(
        message_for(digit_pressed(
            Key::Character("o".into()),
            key::Code::KeyO,
            Location::Standard,
            Modifiers::ALT
        )),
        Some(Message::ToggleOptions)
    ));
    for modifiers in [
        Modifiers::NONE,
        Modifiers::SHIFT,
        Modifiers::CTRL,
        Modifiers::ALT | Modifiers::CTRL,
        Modifiers::ALT | Modifiers::SHIFT,
        Modifiers::ALT | Modifiers::LOGO,
    ] {
        assert!(options_shortcut(&physical, modifiers).is_none());
    }
    assert!(options_shortcut(&key::Physical::Code(key::Code::KeyP), Modifiers::ALT).is_none());
}

#[test]
fn message_for_options_maps_up_down_enter_and_falls_back() {
    for modifiers in [Modifiers::NONE, Modifiers::SHIFT] {
        assert!(matches!(
            message_for_options(key_pressed(Key::Named(key::Named::ArrowUp), modifiers)),
            Some(Message::OptionsSelectionUp)
        ));
        assert!(matches!(
            message_for_options(key_pressed(Key::Named(key::Named::ArrowDown), modifiers)),
            Some(Message::OptionsSelectionDown)
        ));
        assert!(matches!(
            message_for_options(key_pressed(Key::Named(key::Named::Enter), modifiers)),
            Some(Message::OptionsActivate)
        ));
    }
    // Ctrl+Up and Ctrl+Down step the browse selection through the
    // fallback (`message_for`); Enter stays closed to the menu, and a
    // Logo chord is left to the OS as before.
    for named in [key::Named::ArrowUp, key::Named::ArrowDown] {
        assert!(matches!(
            message_for_options(key_pressed(Key::Named(named), Modifiers::CTRL)),
            Some(Message::MoveBrowseSelectionUp | Message::MoveBrowseSelectionDown)
        ));
    }
    for named in [
        key::Named::ArrowUp,
        key::Named::ArrowDown,
        key::Named::Enter,
    ] {
        assert!(message_for_options(key_pressed(Key::Named(named), Modifiers::LOGO)).is_none(),);
    }
    assert!(
        message_for_options(key_pressed(Key::Named(key::Named::Enter), Modifiers::CTRL)).is_none()
    );
    assert!(matches!(
        message_for_options(key_pressed(Key::Character("x".into()), Modifiers::NONE)),
        Some(Message::Play)
    ));
    assert!(matches!(
        message_for_options(digit_pressed(
            Key::Character("o".into()),
            key::Code::KeyO,
            Location::Standard,
            Modifiers::ALT
        )),
        Some(Message::ToggleOptions)
    ));
    assert!(matches!(
        message_for(key_pressed(
            Key::Named(key::Named::ArrowUp),
            Modifiers::NONE
        )),
        Some(Message::VolumeUp)
    ));
    assert!(matches!(
        message_for(key_pressed(Key::Named(key::Named::Enter), Modifiers::NONE)),
        Some(Message::OpenSelected)
    ));
}

/// Asserts that pressing `key` with `modifiers` maps to the message `pattern`.
macro_rules! assert_maps {
    ($key:expr, $modifiers:expr => $pattern:pat) => {
        assert!(matches!(
            message_for(key_pressed($key, $modifiers)),
            Some($pattern)
        ))
    };
}

/// Asserts that pressing `key` with `modifiers` maps to no message.
macro_rules! assert_unbound {
    ($key:expr, $modifiers:expr) => {{
        let (key, modifiers) = ($key, $modifiers);
        assert!(
            message_for(key_pressed(key.clone(), modifiers)).is_none(),
            "{key:?} with {modifiers:?} must stay unbound"
        )
    }};
}

/// Asserts each classic letter binding maps to its transport message, by
/// driving `map` with every letter. The tests below pass different mappers:
/// one wraps the letter in a `KeyPressed` event for `message_for`, the other
/// feeds `shortcut` an uppercase letter with Shift held.
fn assert_letter_bindings(map: impl Fn(&str) -> Option<Message>) {
    assert!(matches!(map("z"), Some(Message::PreviousTrack)));
    assert!(matches!(map("x"), Some(Message::Play)));
    assert!(matches!(map("c"), Some(Message::Pause)));
    assert!(matches!(map("b"), Some(Message::NextTrack)));
    assert!(matches!(map("r"), Some(Message::ToggleRepeat)));
    assert!(matches!(map("s"), Some(Message::ToggleShuffle)));
    assert!(matches!(map("a"), Some(Message::ToggleAlwaysOnTop)));
    assert!(matches!(map("t"), Some(Message::ToggleTimeDisplay)));
    assert!(matches!(map("e"), Some(Message::ToggleEqualizerPanel)));
}

#[test]
fn message_for_maps_each_bound_letter() {
    assert_letter_bindings(|letter| {
        message_for(key_pressed(Key::Character(letter.into()), Modifiers::NONE))
    });
    assert_maps!(Key::Character("v".into()), Modifiers::NONE => Message::Stop);
}

#[test]
fn message_for_maps_shift_v_to_fade_out_and_stop() {
    for character in ["v", "V"] {
        assert_maps!(Key::Character(character.into()), Modifiers::SHIFT => Message::FadeOutAndStop);
    }
}

#[test]
fn message_for_maps_the_arrow_keys() {
    assert_maps!(Key::Named(key::Named::ArrowUp), Modifiers::NONE => Message::VolumeUp);
    assert_maps!(Key::Named(key::Named::ArrowDown), Modifiers::NONE => Message::VolumeDown);
    assert_maps!(Key::Named(key::Named::ArrowLeft), Modifiers::NONE => Message::SeekBackward);
    assert_maps!(Key::Named(key::Named::ArrowRight), Modifiers::NONE => Message::SeekForward);
}

#[test]
fn message_for_maps_enter_to_open_selected() {
    for modifiers in [Modifiers::NONE, Modifiers::SHIFT] {
        assert_maps!(Key::Named(key::Named::Enter), modifiers => Message::OpenSelected);
    }
}

#[test]
fn message_for_maps_backspace_to_back() {
    for modifiers in [Modifiers::NONE, Modifiers::SHIFT] {
        assert_maps!(Key::Named(key::Named::Backspace), modifiers => Message::Back);
    }
}

#[test]
fn message_for_maps_delete_to_remove_selected() {
    // Only plain Delete maps in the plain-key match: a Shift+Delete chord
    // is the Control block's clear (see
    // `message_for_maps_ctrl_shift_delete_to_remove_all`), and every other
    // chord modifier is a window-manager key on this gate.
    assert_maps!(Key::Named(key::Named::Delete), Modifiers::NONE => Message::RemoveSelectedBrowse);
    for modifiers in [
        Modifiers::SHIFT,
        Modifiers::CTRL,
        Modifiers::ALT,
        Modifiers::LOGO,
        Modifiers::ALT | Modifiers::SHIFT,
        Modifiers::LOGO | Modifiers::SHIFT,
    ] {
        assert_unbound!(Key::Named(key::Named::Delete), modifiers);
    }
}

#[test]
fn message_for_maps_ctrl_shift_delete_to_remove_all() {
    // Ctrl+Shift+Delete is the clear (Winamp's playlist editor's Remove
    // all): it rides the Control block's no-Alt, no-Logo gate like
    // Ctrl+R, and claims the Delete chord the plain-key gate withholds.
    assert_maps!(
        Key::Named(key::Named::Delete),
        Modifiers::CTRL | Modifiers::SHIFT => Message::RemoveAllBrowse
    );
    // Every other modifier combination on Delete stays where Delete's own
    // mapping or the gate put it: plain is Remove selected, and the rest
    // (including a Control without Shift) is left to the window manager.
    assert_maps!(Key::Named(key::Named::Delete), Modifiers::NONE => Message::RemoveSelectedBrowse);
    for modifiers in [
        Modifiers::SHIFT,
        Modifiers::CTRL,
        Modifiers::ALT,
        Modifiers::LOGO,
        Modifiers::ALT | Modifiers::SHIFT,
        Modifiers::LOGO | Modifiers::SHIFT,
        Modifiers::CTRL | Modifiers::ALT,
        Modifiers::CTRL | Modifiers::LOGO,
        Modifiers::CTRL | Modifiers::ALT | Modifiers::SHIFT,
    ] {
        assert_unbound!(Key::Named(key::Named::Delete), modifiers);
    }
}

#[test]
fn message_for_maps_home_and_end_to_browse_scroll() {
    for modifiers in [Modifiers::NONE, Modifiers::SHIFT] {
        assert_maps!(Key::Named(key::Named::Home), modifiers => Message::ScrollBrowseToTop);
        assert_maps!(Key::Named(key::Named::End), modifiers => Message::ScrollBrowseToBottom);
    }
}

#[test]
fn message_for_maps_page_keys_to_browse_paging() {
    for modifiers in [Modifiers::NONE, Modifiers::SHIFT] {
        assert_maps!(Key::Named(key::Named::PageDown), modifiers => Message::ScrollBrowsePageDown);
        assert_maps!(Key::Named(key::Named::PageUp), modifiers => Message::ScrollBrowsePageUp);
    }
}

#[test]
fn message_for_maps_f5_to_reload() {
    for modifiers in [Modifiers::NONE, Modifiers::SHIFT] {
        assert_maps!(Key::Named(key::Named::F5), modifiers => Message::ReloadLibrary);
    }
}

#[test]
fn message_for_maps_media_keys_to_the_transport() {
    // The system media keys drive the same explicit Play, Pause, Stop,
    // Next, and Previous messages as the on-screen controls and classic
    // letters, plus their own Play/Pause toggle, plain or with Shift held.
    for modifiers in [Modifiers::NONE, Modifiers::SHIFT] {
        assert_maps!(Key::Named(key::Named::MediaPlayPause), modifiers => Message::PlayPause);
        assert_maps!(Key::Named(key::Named::MediaPlay), modifiers => Message::Play);
        assert_maps!(Key::Named(key::Named::MediaPause), modifiers => Message::Pause);
        assert_maps!(Key::Named(key::Named::MediaStop), modifiers => Message::Stop);
        assert_maps!(Key::Named(key::Named::MediaTrackNext), modifiers => Message::NextTrack);
        assert_maps!(Key::Named(key::Named::MediaTrackPrevious), modifiers => Message::PreviousTrack);
    }
}

#[test]
fn message_for_maps_a_and_t_to_the_classic_toggles() {
    for modifiers in [Modifiers::NONE, Modifiers::SHIFT] {
        assert_maps!(Key::Character("a".into()), modifiers => Message::ToggleAlwaysOnTop);
        assert_maps!(Key::Character("t".into()), modifiers => Message::ToggleTimeDisplay);
    }
}

#[test]
fn message_for_maps_ctrl_v_to_stop_after_current() {
    for modifiers in [Modifiers::CTRL, Modifiers::CTRL | Modifiers::SHIFT] {
        for character in ["v", "V"] {
            assert_maps!(Key::Character(character.into()), modifiers => Message::ToggleStopAfterCurrent);
        }
    }
    for modifiers in [
        Modifiers::CTRL | Modifiers::ALT,
        Modifiers::CTRL | Modifiers::LOGO,
    ] {
        assert_unbound!(Key::Character("v".into()), modifiers);
    }
}

#[test]
fn message_for_maps_ctrl_a_and_ctrl_t_to_the_classic_toggles() {
    // Winamp's Ctrl+A and Ctrl+T repeat the plain A and T toggles, with or
    // without Shift and regardless of the character's case.
    for modifiers in [Modifiers::CTRL, Modifiers::CTRL | Modifiers::SHIFT] {
        for character in ["a", "A"] {
            assert_maps!(Key::Character(character.into()), modifiers => Message::ToggleAlwaysOnTop);
        }
        for character in ["t", "T"] {
            assert_maps!(Key::Character(character.into()), modifiers => Message::ToggleTimeDisplay);
        }
    }

    // Alt or Logo held with Control stays a window-manager chord.
    for modifiers in [
        Modifiers::CTRL | Modifiers::ALT,
        Modifiers::CTRL | Modifiers::LOGO,
    ] {
        for character in ["a", "t"] {
            assert_unbound!(Key::Character(character.into()), modifiers);
        }
    }
}

#[test]
fn message_for_maps_ctrl_f1_to_about() {
    // Winamp's Ctrl+F1 opens the About box, with or without Shift.
    for modifiers in [Modifiers::CTRL, Modifiers::CTRL | Modifiers::SHIFT] {
        assert_maps!(Key::Named(key::Named::F1), modifiers => Message::ToggleAbout);
    }

    // Plain F1 and the Alt/Logo chords stay unbound, as do Control+Alt and
    // Control+Logo.
    for modifiers in [
        Modifiers::NONE,
        Modifiers::ALT,
        Modifiers::LOGO,
        Modifiers::CTRL | Modifiers::ALT,
        Modifiers::CTRL | Modifiers::LOGO,
    ] {
        assert_unbound!(Key::Named(key::Named::F1), modifiers);
    }
}

#[test]
fn message_for_maps_ctrl_f_to_focus_search() {
    // Control plus F focuses the search box, with or without Shift and
    // regardless of the character's case.
    for modifiers in [Modifiers::CTRL, Modifiers::CTRL | Modifiers::SHIFT] {
        for character in ["f", "F"] {
            assert!(
                matches!(
                    message_for(key_pressed(Key::Character(character.into()), modifiers)),
                    Some(Message::FocusSearch)
                ),
                "Ctrl+F ({character}) must focus the search box"
            );
        }
    }

    // Plain F and the Alt/Logo chords keep their old behavior: unbound.
    for modifiers in [
        Modifiers::NONE,
        Modifiers::SHIFT,
        Modifiers::ALT,
        Modifiers::LOGO,
        Modifiers::CTRL | Modifiers::ALT,
        Modifiers::CTRL | Modifiers::LOGO,
    ] {
        assert_unbound!(Key::Character("f".into()), modifiers);
    }
}

#[test]
fn shortcut_matches_letters_case_insensitively() {
    // Uppercase characters map the same as lowercase, whether the case comes
    // from the character itself or the Shift modifier held over a lowercase
    // one; Shift is allowed, unlike the chord modifiers below.
    assert_letter_bindings(|letter| {
        shortcut(
            &Key::Character(letter.to_uppercase().into()),
            Modifiers::SHIFT,
        )
    });
}

#[test]
fn message_for_maps_ctrl_z_and_ctrl_b_to_the_list_edges() {
    for modifiers in [Modifiers::CTRL, Modifiers::CTRL | Modifiers::SHIFT] {
        for letter in ["z", "Z"] {
            assert_maps!(Key::Character(letter.into()), modifiers => Message::FirstTrack);
        }
        for letter in ["b", "B"] {
            assert_maps!(Key::Character(letter.into()), modifiers => Message::LastTrack);
        }
    }
    for modifiers in [
        Modifiers::ALT,
        Modifiers::LOGO,
        Modifiers::CTRL | Modifiers::ALT,
        Modifiers::CTRL | Modifiers::LOGO,
    ] {
        for letter in ["z", "b"] {
            assert_unbound!(Key::Character(letter.into()), modifiers);
        }
    }
}

#[test]
fn shortcut_ignores_chord_modifiers() {
    // Control, Alt, and Logo chords belong to the OS and window manager;
    // Ctrl+F, Ctrl+Z, Ctrl+B, Ctrl+R, and the Ctrl+arrow selection steps
    // are the Control exceptions, pinned by their own tests; Z and B stay
    // unbound with Alt or Logo.
    for modifiers in [Modifiers::ALT, Modifiers::LOGO] {
        for key in [Key::Character("z".into()), Key::Character("b".into())] {
            assert!(
                shortcut(&key, modifiers).is_none(),
                "{key:?} with {modifiers:?} must not map"
            );
        }
    }
    for modifiers in [Modifiers::CTRL, Modifiers::ALT, Modifiers::LOGO] {
        for key in [
            Key::Character("x".into()),
            Key::Character("s".into()),
            Key::Character("e".into()),
            Key::Named(key::Named::Enter),
            Key::Named(key::Named::Backspace),
            Key::Named(key::Named::Home),
            Key::Named(key::Named::End),
            Key::Named(key::Named::PageUp),
            Key::Named(key::Named::PageDown),
            Key::Named(key::Named::F5),
            // The up and down arrows are lifted out: Ctrl+Up and
            // Ctrl+Down now step the browse selection under plain Ctrl
            // (see the Ctrl-arrows test). ArrowLeft and ArrowRight, and
            // the arrows under Alt or Logo, stay OS territory.
            Key::Named(key::Named::ArrowLeft),
            Key::Named(key::Named::ArrowRight),
            Key::Named(key::Named::MediaPlayPause),
            Key::Named(key::Named::MediaPlay),
            Key::Named(key::Named::MediaPause),
            Key::Named(key::Named::MediaStop),
            Key::Named(key::Named::MediaTrackNext),
            Key::Named(key::Named::MediaTrackPrevious),
        ] {
            assert!(
                shortcut(&key, modifiers).is_none(),
                "{key:?} with {modifiers:?} must not map"
            );
        }
    }
    // The up and down arrows under Alt or Logo stay OS chord territory
    // in `shortcut` (the Alt arrows map in `alt_arrow_shortcut`, pinned
    // by its own test); their Ctrl chords are pinned by the
    // selection-step test.
    for modifiers in [Modifiers::ALT, Modifiers::LOGO] {
        for named in [key::Named::ArrowUp, key::Named::ArrowDown] {
            assert!(
                shortcut(&Key::Named(named), modifiers).is_none(),
                "Named({named:?}) with {modifiers:?} must stay unbound"
            );
        }
    }
    // A and T are also bound with Control alone (see the Ctrl+A/Ctrl+T
    // test), so only Alt and Logo are checked here.
    for modifiers in [Modifiers::ALT, Modifiers::LOGO] {
        for key in [Key::Character("a".into()), Key::Character("t".into())] {
            assert!(
                shortcut(&key, modifiers).is_none(),
                "{key:?} with {modifiers:?} must not map"
            );
        }
    }
    // R is also bound with Control alone (see the Ctrl+R test), so only
    // Alt and Logo are checked here.
    for modifiers in [Modifiers::ALT, Modifiers::LOGO] {
        assert!(shortcut(&Key::Character("r".into()), modifiers).is_none());
    }
}

#[test]
fn message_for_maps_ctrl_shift_1_to_sort() {
    for character in ["!", "1", "&"] {
        let event = digit_pressed(
            Key::Character(character.into()),
            key::Code::Digit1,
            Location::Standard,
            Modifiers::CTRL | Modifiers::SHIFT,
        );
        assert!(matches!(message_for(event), Some(Message::SortBrowse)));
    }
}

#[test]
fn ctrl_shift_1_needs_exactly_control_and_shift() {
    let one = || Key::Character("1".into());
    for modifiers in [
        Modifiers::CTRL,
        Modifiers::SHIFT,
        Modifiers::NONE,
        Modifiers::CTRL | Modifiers::SHIFT | Modifiers::ALT,
        Modifiers::CTRL | Modifiers::SHIFT | Modifiers::LOGO,
    ] {
        let event = digit_pressed(one(), key::Code::Digit1, Location::Standard, modifiers);
        assert!(message_for(event).is_none(), "1 with {modifiers:?}");
    }
    let both = Modifiers::CTRL | Modifiers::SHIFT;
    let digit2 = digit_pressed(
        Key::Character("2".into()),
        key::Code::Digit2,
        Location::Standard,
        both,
    );
    assert!(message_for(digit2).is_none());
    let keypad = digit_pressed(one(), key::Code::Numpad1, Location::Numpad, both);
    assert!(message_for(keypad).is_none());
}

#[test]
fn message_for_maps_ctrl_j_to_jump_time() {
    for character in ["j", "J"] {
        let event = key_pressed(Key::Character(character.into()), Modifiers::CTRL);
        assert!(matches!(message_for(event), Some(Message::ToggleJumpTime)));
    }
    for modifiers in [
        Modifiers::CTRL | Modifiers::SHIFT,
        Modifiers::CTRL | Modifiers::ALT,
        Modifiers::CTRL | Modifiers::LOGO,
    ] {
        let event = key_pressed(Key::Character("j".into()), modifiers);
        assert!(message_for(event).is_none(), "{modifiers:?}");
    }
}

#[test]
fn message_for_jump_file_maps_up_and_down_and_falls_back() {
    for modifiers in [Modifiers::NONE, Modifiers::SHIFT] {
        assert!(matches!(
            message_for_jump_file(key_pressed(Key::Named(key::Named::ArrowUp), modifiers)),
            Some(Message::JumpFileSelectionUp)
        ));
        assert!(matches!(
            message_for_jump_file(key_pressed(Key::Named(key::Named::ArrowDown), modifiers)),
            Some(Message::JumpFileSelectionDown)
        ));
        assert!(matches!(
            message_for_jump_file(key_pressed(Key::Named(key::Named::PageUp), modifiers)),
            Some(Message::JumpFileSelectionPageUp)
        ));
        assert!(matches!(
            message_for_jump_file(key_pressed(Key::Named(key::Named::PageDown), modifiers)),
            Some(Message::JumpFileSelectionPageDown)
        ));
    }
    // Everything else keeps `message_for`'s mapping: a plain J still opens
    // the panel, and Alt+Up still moves a browse row.
    assert!(matches!(
        message_for_jump_file(key_pressed(Key::Character("j".into()), Modifiers::NONE)),
        Some(Message::ToggleJumpFile)
    ));
    assert!(matches!(
        message_for_jump_file(key_pressed(Key::Named(key::Named::ArrowUp), Modifiers::ALT)),
        Some(Message::MoveSelectedRowUp)
    ));
    // Ctrl+Up and Ctrl+Down step the browse selection through the
    // fallback (`message_for`); a Logo chord is left to the OS as before.
    assert!(matches!(
        message_for_jump_file(key_pressed(
            Key::Named(key::Named::ArrowUp),
            Modifiers::CTRL
        )),
        Some(Message::MoveBrowseSelectionUp)
    ));
    assert!(
        message_for_jump_file(key_pressed(
            Key::Named(key::Named::ArrowUp),
            Modifiers::LOGO
        ))
        .is_none()
    );
    for modifiers in [Modifiers::CTRL, Modifiers::ALT, Modifiers::LOGO] {
        for key in [key::Named::PageUp, key::Named::PageDown] {
            assert!(
                message_for_jump_file(key_pressed(Key::Named(key), modifiers)).is_none(),
                "{key:?} with {modifiers:?}"
            );
        }
    }
}

#[test]
fn message_for_maps_j_to_jump_file() {
    for modifiers in [Modifiers::NONE, Modifiers::SHIFT] {
        for character in ["j", "J"] {
            let event = key_pressed(Key::Character(character.into()), modifiers);
            assert!(
                matches!(message_for(event), Some(Message::ToggleJumpFile)),
                "{modifiers:?}+{character} must open Jump to File"
            );
        }
    }
}

#[test]
fn message_for_maps_alt_e_and_alt_g_to_the_panel_toggles() {
    for logical in ["e", "\u{b4}"] {
        let event = digit_pressed(
            Key::Character(logical.into()),
            key::Code::KeyE,
            Location::Standard,
            Modifiers::ALT,
        );
        assert!(matches!(
            message_for(event),
            Some(Message::ToggleLibraryPanel)
        ));
    }
    for logical in ["g", "\u{a9}"] {
        let event = digit_pressed(
            Key::Character(logical.into()),
            key::Code::KeyG,
            Location::Standard,
            Modifiers::ALT,
        );
        assert!(matches!(
            message_for(event),
            Some(Message::ToggleEqualizerPanel)
        ));
    }
    for code in [key::Code::KeyE, key::Code::KeyG] {
        for modifiers in [
            Modifiers::CTRL | Modifiers::ALT,
            Modifiers::SHIFT | Modifiers::ALT,
            Modifiers::LOGO | Modifiers::ALT,
        ] {
            let event = digit_pressed(
                Key::Character("x".into()),
                code,
                Location::Standard,
                modifiers,
            );
            assert!(message_for(event).is_none(), "{code:?} with {modifiers:?}");
        }
    }
    assert_maps!(Key::Character("e".into()), Modifiers::NONE => Message::ToggleEqualizerPanel);
    assert!(message_for(key_pressed(Key::Character("g".into()), Modifiers::NONE)).is_none());
}

#[test]
fn message_for_maps_alt_3_to_file_info() {
    let three = || Key::Character("3".into());
    let event = digit_pressed(
        three(),
        key::Code::Digit3,
        Location::Standard,
        Modifiers::ALT,
    );
    assert!(matches!(message_for(event), Some(Message::ToggleFileInfo)));

    for modifiers in [
        Modifiers::CTRL | Modifiers::ALT,
        Modifiers::SHIFT | Modifiers::ALT,
        Modifiers::LOGO | Modifiers::ALT,
        Modifiers::NONE,
    ] {
        let event = digit_pressed(three(), key::Code::Digit3, Location::Standard, modifiers);
        assert!(message_for(event).is_none(), "3 with {modifiers:?}");
    }
    let keypad = digit_pressed(
        three(),
        key::Code::Numpad3,
        Location::Numpad,
        Modifiers::ALT,
    );
    assert!(message_for(keypad).is_none());
}

#[test]
fn message_for_maps_ctrl_r_to_reverse_the_list() {
    for letter in ["r", "R"] {
        assert_maps!(Key::Character(letter.into()), Modifiers::CTRL => Message::ReverseBrowse);
    }
    for modifiers in [
        Modifiers::ALT,
        Modifiers::LOGO,
        Modifiers::CTRL | Modifiers::ALT,
        Modifiers::CTRL | Modifiers::LOGO,
    ] {
        assert_unbound!(Key::Character("r".into()), modifiers);
    }
}

#[test]
fn message_for_maps_ctrl_w_to_toggle_window_shade() {
    for modifiers in [Modifiers::CTRL, Modifiers::CTRL | Modifiers::SHIFT] {
        assert_maps!(Key::Character("w".into()), modifiers => Message::ToggleWindowShade);
    }
    assert_maps!(Key::Character("W".into()), Modifiers::CTRL => Message::ToggleWindowShade);
    for modifiers in [
        Modifiers::NONE,
        Modifiers::SHIFT,
        Modifiers::ALT,
        Modifiers::LOGO,
        Modifiers::CTRL | Modifiers::ALT,
        Modifiers::CTRL | Modifiers::LOGO,
    ] {
        assert_unbound!(Key::Character("w".into()), modifiers);
    }
}

#[test]
fn message_for_maps_ctrl_shift_r_to_randomize_the_list() {
    for letter in ["r", "R"] {
        assert_maps!(
            Key::Character(letter.into()),
            Modifiers::CTRL | Modifiers::SHIFT => Message::RandomizeBrowse
        );
    }
    for modifiers in [
        Modifiers::CTRL | Modifiers::SHIFT | Modifiers::ALT,
        Modifiers::CTRL | Modifiers::SHIFT | Modifiers::LOGO,
    ] {
        assert_unbound!(Key::Character("r".into()), modifiers);
    }
    // Shift alone still toggles Repeat.
    assert!(!matches!(
        message_for(key_pressed(Key::Character("R".into()), Modifiers::SHIFT)),
        Some(Message::RandomizeBrowse)
    ));
}

#[test]
fn message_for_maps_alt_arrows_to_move_the_selected_row() {
    assert_maps!(Key::Named(key::Named::ArrowUp), Modifiers::ALT => Message::MoveSelectedRowUp);
    assert_maps!(
        Key::Named(key::Named::ArrowDown), Modifiers::ALT => Message::MoveSelectedRowDown
    );
    // Shift held too widens the one-place move to the list's edge.
    assert_maps!(
        Key::Named(key::Named::ArrowUp), Modifiers::ALT | Modifiers::SHIFT => Message::MoveSelectedRowToTop
    );
    assert_maps!(
        Key::Named(key::Named::ArrowDown), Modifiers::ALT | Modifiers::SHIFT => Message::MoveSelectedRowToBottom
    );
    // Shift+Alt+Left/Right stays unbound so a plain Shift+arrow keeps the
    // volume and seek arms unclaimed.
    for named in [key::Named::ArrowLeft, key::Named::ArrowRight] {
        assert_unbound!(Key::Named(named), Modifiers::ALT | Modifiers::SHIFT);
    }
    // Control+Alt and Alt+Logo remain OS-chord territory, with or without
    // Shift.
    for modifiers in [
        Modifiers::ALT | Modifiers::CTRL,
        Modifiers::ALT | Modifiers::CTRL | Modifiers::SHIFT,
        Modifiers::ALT | Modifiers::LOGO,
    ] {
        for named in [key::Named::ArrowUp, key::Named::ArrowDown] {
            assert_unbound!(Key::Named(named), modifiers);
        }
    }
    assert_maps!(Key::Named(key::Named::ArrowUp), Modifiers::NONE => Message::VolumeUp);
    assert_maps!(Key::Named(key::Named::ArrowDown), Modifiers::NONE => Message::VolumeDown);
}

#[test]
fn message_for_maps_ctrl_arrows_to_move_the_browse_selection() {
    // Shift held with the chord maps the same way: the arrows carry no
    // shift behavior anywhere.
    for modifiers in [Modifiers::CTRL, Modifiers::CTRL | Modifiers::SHIFT] {
        assert_maps!(
            Key::Named(key::Named::ArrowUp), modifiers => Message::MoveBrowseSelectionUp
        );
        assert_maps!(
            Key::Named(key::Named::ArrowDown), modifiers => Message::MoveBrowseSelectionDown
        );
    }
    // The chord stays a window-manager one under Alt or Logo.
    for modifiers in [
        Modifiers::CTRL | Modifiers::ALT,
        Modifiers::CTRL | Modifiers::LOGO,
    ] {
        for named in [key::Named::ArrowUp, key::Named::ArrowDown] {
            assert_unbound!(Key::Named(named), modifiers);
        }
    }
    // The plain arrows still nudge the volume and the Alt arrows still
    // move the selected row.
    assert_maps!(Key::Named(key::Named::ArrowUp), Modifiers::NONE => Message::VolumeUp);
    assert_maps!(Key::Named(key::Named::ArrowDown), Modifiers::NONE => Message::VolumeDown);
    assert_maps!(Key::Named(key::Named::ArrowUp), Modifiers::ALT => Message::MoveSelectedRowUp);
    assert_maps!(
        Key::Named(key::Named::ArrowDown), Modifiers::ALT => Message::MoveSelectedRowDown
    );
}

#[test]
fn message_for_maps_the_numpad_transport_digits() {
    for modifiers in [Modifiers::NONE, Modifiers::SHIFT] {
        let map =
            |digit: &str| message_for(numpad_pressed(Key::Character(digit.into()), modifiers));
        assert!(matches!(map("4"), Some(Message::PreviousTrack)));
        assert!(matches!(map("5"), Some(Message::Play)));
        assert!(matches!(map("6"), Some(Message::NextTrack)));
        assert!(matches!(map("7"), Some(Message::SeekBackward)));
        assert!(matches!(map("9"), Some(Message::SeekForward)));
        assert!(matches!(map("8"), Some(Message::VolumeUp)));
        assert!(matches!(map("2"), Some(Message::VolumeDown)));
        assert!(matches!(map("1"), Some(Message::JumpTracksBack)));
        assert!(matches!(map("3"), Some(Message::JumpTracksForward)));
    }
}

#[test]
fn message_for_leaves_other_digits_unbound() {
    for key in ["0", "+"] {
        let event = numpad_pressed(Key::Character(key.into()), Modifiers::NONE);
        assert!(message_for(event).is_none(), "keypad {key}");
    }
    for key in ["1", "2", "3", "4", "5", "6", "7", "8", "9"] {
        let event = key_pressed(Key::Character(key.into()), Modifiers::NONE);
        assert!(message_for(event).is_none(), "top-row {key}");
    }
}

#[test]
fn message_for_ignores_numpad_chords() {
    for modifiers in [Modifiers::CTRL, Modifiers::ALT, Modifiers::LOGO] {
        let event = numpad_pressed(Key::Character("4".into()), modifiers);
        assert!(message_for(event).is_none());
    }
}

#[test]
fn message_for_maps_numpad_arrows_like_the_arrow_keys() {
    let left = numpad_pressed(Key::Named(key::Named::ArrowLeft), Modifiers::NONE);
    let up = numpad_pressed(Key::Named(key::Named::ArrowUp), Modifiers::NONE);
    assert!(matches!(message_for(left), Some(Message::SeekBackward)));
    assert!(matches!(message_for(up), Some(Message::VolumeUp)));
}

#[test]
fn message_for_ignores_unbound_keys_and_non_press_events() {
    // An unbound character and an unbound named key both map to nothing.
    assert!(message_for(key_pressed(Key::Character("q".into()), Modifiers::NONE)).is_none());
    assert!(message_for(key_pressed(Key::Named(key::Named::Escape), Modifiers::NONE)).is_none());

    // A key release and a modifier-only change bind nothing either.
    assert!(
        message_for(Event::KeyReleased {
            key: Key::Character("z".into()),
            modified_key: Key::Character("z".into()),
            physical_key: key::Physical::Code(key::Code::KeyZ),
            location: iced::keyboard::Location::Standard,
            modifiers: Modifiers::NONE,
        })
        .is_none()
    );
    assert!(message_for(Event::ModifiersChanged(Modifiers::SHIFT)).is_none());
}
