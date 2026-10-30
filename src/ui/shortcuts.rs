//! The classic Winamp keyboard bindings: mapping an iced key event to the
//! transport `Message` it triggers.
//!
//! The player's keys are a player concern, so the key-to-message table lives
//! here as pure functions, mirroring `views`, `transport`, and `loading`. They
//! know the `Message` type and iced's key event, not `WinampPlayer` or the
//! update loop, so `init_ui` installs [`message_for`] as the window
//! subscription's mapper. Its unit tests live in the `tests` submodule.

use iced::keyboard::{Event, Key, Location, Modifiers, key};

use super::Message;

/// Maps a keyboard event to the classic Winamp transport shortcut, or `None`
/// for every event that binds nothing: a key release, a modifier-only change,
/// an unbound key, or a key pressed with a chord modifier. The bound keys
/// include R for Repeat, S for Shuffle, A for always-on-top, T for the
/// elapsed/remaining readout, E for the equalizer panel's show/hide, Enter to
/// open the selected browse row,
/// Backspace to step the browse list back one level, Delete to remove the
/// showing songs list's selected row, Home and End to snap it
/// to its first and last rows, Page Up and Page Down to scroll it one page at
/// a time, F5 to refetch the library and return to the Artists list, Ctrl+F
/// to focus the `Search library` box, Ctrl+Z and Ctrl+B to play the first and
/// last song of the showing list, Ctrl+R to reverse the showing list,
/// Ctrl+Shift+R to shuffle it, Ctrl+W to toggle window-shade mode, Ctrl+J to
/// flip the Jump to time panel, Ctrl+A and Ctrl+T to repeat the classic
/// always-on-top and elapsed/remaining toggles, Ctrl+F1 to open the About
/// panel, plain J to flip the Jump to File panel,
/// Ctrl+Shift+1 to sort it by title (see [`playlist_sort_shortcut`]), Alt+3
/// to flip the File info panel
/// (see [`file_info_shortcut`]), Alt+E and Alt+G to show or hide the library
/// and equalizer panels (see [`panel_shortcut`]), and the system media keys
/// (Play/Pause, Play, Pause, Stop, Next track, and Previous track) to their
/// transport messages: Play, Pause, Stop, Next, and Previous match the
/// on-screen controls and classic letters, while Play/Pause is its own toggle
/// (the one Control chord Winamp-style play uses).
/// `init_ui` installs this as the window subscription's mapper, so the
/// key-to-message table lives here once and the subscription stays a plain
/// `filter_map`. With `NumLock` on, the keypad digits 4, 5, 6, 7, 9, 8, and 2
/// also drive the transport and volume (see [`numpad_shortcut`]), and Alt+Up
/// and Alt+Down move the selected browse row (see [`move_row_shortcut`]).
pub(super) fn message_for(event: Event) -> Option<Message> {
    match event {
        Event::KeyPressed {
            key,
            modifiers,
            physical_key,
            location,
            ..
        } => playlist_sort_shortcut(&physical_key, location, modifiers)
            .or_else(|| file_info_shortcut(&physical_key, location, modifiers))
            .or_else(|| panel_shortcut(&physical_key, modifiers))
            .or_else(|| options_shortcut(&physical_key, modifiers))
            .or_else(|| move_row_shortcut(&key, modifiers))
            .or_else(|| numpad_shortcut(&key, location, modifiers))
            .or_else(|| shortcut(&key, modifiers)),
        Event::KeyReleased { .. } | Event::ModifiersChanged(_) => None,
    }
}

/// Options captures Up, Down, and Enter without Control, Alt, or Logo;
/// other events retain their normal bindings.
pub(super) fn message_for_options(event: Event) -> Option<Message> {
    if let Event::KeyPressed {
        key: Key::Named(named),
        modifiers,
        ..
    } = &event
        && !modifiers.control()
        && !modifiers.alt()
        && !modifiers.logo()
    {
        match named {
            key::Named::ArrowUp => return Some(Message::OptionsSelectionUp),
            key::Named::ArrowDown => return Some(Message::OptionsSelectionDown),
            key::Named::Enter => return Some(Message::OptionsActivate),
            _ => {}
        }
    }
    message_for(event)
}

/// Alt+O uses the physical letter position, without additional modifiers.
fn options_shortcut(physical_key: &key::Physical, modifiers: Modifiers) -> Option<Message> {
    (matches!(physical_key, key::Physical::Code(key::Code::KeyO))
        && modifiers.alt()
        && !modifiers.control()
        && !modifiers.shift()
        && !modifiers.logo())
    .then_some(Message::ToggleOptions)
}

/// The keyboard mapper used while the Jump to File panel is open: Up and Down
/// move the panel's highlight instead of nudging the volume, and Page Up and
/// Page Down move it one page instead of scrolling the browse list behind the
/// panel. A key press with Control, Alt, and Logo released maps `ArrowUp`,
/// `ArrowDown`, `PageUp`, and `PageDown` (Shift allowed, like the letter
/// bindings) to [`Message::JumpFileSelectionUp`],
/// [`Message::JumpFileSelectionDown`], [`Message::JumpFileSelectionPageUp`],
/// and [`Message::JumpFileSelectionPageDown`]. Every other event falls through
/// to [`message_for`], so J, Ctrl+J, and the rest keep their mappings; Home and
/// End stay with the focused filter box. iced's `TextInput` leaves Up, Down,
/// Page Up, and Page Down to the listener, so the filter box keeps focus while
/// they move the highlight.
pub(super) fn message_for_jump_file(event: Event) -> Option<Message> {
    match &event {
        Event::KeyPressed {
            key: Key::Named(key::Named::ArrowUp),
            modifiers,
            ..
        } if !modifiers.control() && !modifiers.alt() && !modifiers.logo() => {
            Some(Message::JumpFileSelectionUp)
        }
        Event::KeyPressed {
            key: Key::Named(key::Named::ArrowDown),
            modifiers,
            ..
        } if !modifiers.control() && !modifiers.alt() && !modifiers.logo() => {
            Some(Message::JumpFileSelectionDown)
        }
        Event::KeyPressed {
            key: Key::Named(key::Named::PageUp),
            modifiers,
            ..
        } if !modifiers.control() && !modifiers.alt() && !modifiers.logo() => {
            Some(Message::JumpFileSelectionPageUp)
        }
        Event::KeyPressed {
            key: Key::Named(key::Named::PageDown),
            modifiers,
            ..
        } if !modifiers.control() && !modifiers.alt() && !modifiers.logo() => {
            Some(Message::JumpFileSelectionPageDown)
        }
        _ => message_for(event),
    }
}

/// Winamp's Ctrl+Shift+1, "Sort list by title", sending the same message as the
/// `Sort` button. It matches the top-row `1` by physical key, not by logical
/// character: with Shift held the logical key is `!` on US layouts and another
/// character elsewhere, but the key position is the same. Control and Shift
/// must both be held without Alt or Logo, and the keypad `1` (a different
/// physical key at `Location::Numpad`) never matches.
fn playlist_sort_shortcut(
    physical_key: &key::Physical,
    location: Location,
    modifiers: Modifiers,
) -> Option<Message> {
    (modifiers.control()
        && modifiers.shift()
        && !modifiers.alt()
        && !modifiers.logo()
        && location == Location::Standard
        && *physical_key == key::Physical::Code(key::Code::Digit1))
    .then_some(Message::SortBrowse)
}

/// Winamp's Alt+3, "File info", flipping the File info panel. Like
/// [`playlist_sort_shortcut`] it matches the top-row `3` by physical key, and
/// requires Alt held with Control, Shift, and Logo released; the keypad `3`
/// (`Location::Numpad`) never matches.
fn file_info_shortcut(
    physical_key: &key::Physical,
    location: Location,
    modifiers: Modifiers,
) -> Option<Message> {
    (modifiers.alt()
        && !modifiers.control()
        && !modifiers.shift()
        && !modifiers.logo()
        && location == Location::Standard
        && *physical_key == key::Physical::Code(key::Code::Digit3))
    .then_some(Message::ToggleFileInfo)
}

/// Winamp's Alt+E and Alt+G, showing or hiding the library section (its
/// playlist editor) and the equalizer panel. The keys are matched by physical
/// position, not logical character, because on macOS Alt+E and Alt+G produce
/// other characters (a dead key, `©`). Alt must be held with Control, Shift,
/// and Logo released.
fn panel_shortcut(physical_key: &key::Physical, modifiers: Modifiers) -> Option<Message> {
    if !modifiers.alt() || modifiers.control() || modifiers.shift() || modifiers.logo() {
        return None;
    }
    match physical_key {
        key::Physical::Code(key::Code::KeyE) => Some(Message::ToggleLibraryPanel),
        key::Physical::Code(key::Code::KeyG) => Some(Message::ToggleEqualizerPanel),
        _ => None,
    }
}

/// Alt+Up and Alt+Down move the selected browse row one place up or down, like
/// Winamp's playlist editor. The logical arrow key is matched wherever it sits,
/// and Control, Shift, and Logo must be released; plain arrows stay volume.
fn move_row_shortcut(key: &Key, modifiers: Modifiers) -> Option<Message> {
    if !modifiers.alt() || modifiers.control() || modifiers.shift() || modifiers.logo() {
        return None;
    }
    match key {
        Key::Named(key::Named::ArrowUp) => Some(Message::MoveSelectedRowUp),
        Key::Named(key::Named::ArrowDown) => Some(Message::MoveSelectedRowDown),
        _ => None,
    }
}

/// Winamp's numeric-keypad bindings, for keys pressed at `Location::Numpad`
/// with `NumLock` on: 4 previous, 5 play, 6 next, 7 seek five seconds back, 9
/// seek five seconds forward, 8 volume up, and 2 volume down, sending the same
/// messages as Z, X, B, and the arrows. Shift is allowed; Control, Alt, or Logo
/// yields `None`. Keypad 1 and 3 jump ten songs back or forward. Keypad 0 and
/// the operators stay unbound, and top-row
/// digits never reach this function's bindings. With `NumLock` off the keypad
/// sends arrow keys, which `shortcut` already handles.
fn numpad_shortcut(key: &Key, location: Location, modifiers: Modifiers) -> Option<Message> {
    if location != Location::Numpad || modifiers.control() || modifiers.alt() || modifiers.logo() {
        return None;
    }
    let Key::Character(character) = key else {
        return None;
    };
    match character.as_str() {
        "4" => Some(Message::PreviousTrack),
        "5" => Some(Message::Play),
        "6" => Some(Message::NextTrack),
        "7" => Some(Message::SeekBackward),
        "9" => Some(Message::SeekForward),
        "8" => Some(Message::VolumeUp),
        "2" => Some(Message::VolumeDown),
        "1" => Some(Message::JumpTracksBack),
        "3" => Some(Message::JumpTracksForward),
        _ => None,
    }
}

/// The classic Winamp main-window bindings: Z previous, X play, C pause, V
/// stop, B next, R repeat, S shuffle, A toggles always-on-top, T switches the
/// large readout between elapsed and remaining time, E shows or hides the
/// equalizer panel, Enter opens the selected browse row, Backspace steps the
/// browse list back one level, Delete removes the showing songs list's
/// selected row (plain only: Shift, like the chord modifiers, stays out so
/// Delete cannot ride a chord the user meant for the window manager), Home and
/// End
/// snap it to its first and last rows, Page Up and Page Down scroll it one page
/// at a time, F5 refetches the library and returns to the Artists list, Ctrl+F
/// focuses
/// the `Search library` box, the up/down arrows nudge
/// the volume, the left/right arrows seek five seconds
/// back or forward, and the system media keys (Play/Pause, Play, Pause, Stop,
/// Next track, and Previous track) drive the transport. Apart from Ctrl+F,
/// Ctrl+Z (play the first song), Ctrl+B (play the last song), and Ctrl+R
/// (reverse the showing list), Ctrl+Shift+R (shuffle it), Ctrl+Shift+Delete (clear
/// the showing songs list), Ctrl+J (flip the Jump to
/// time panel), Ctrl+W (toggle window-shade mode), Ctrl+A and Ctrl+T (repeat
/// the classic always-on-top and elapsed/remaining toggles), and Ctrl+F1 (open
/// the About panel), a
/// Control, Alt, or
/// Logo chord is left to the OS and window manager, so those modifiers yield
/// `None`; Shift is allowed, and the letter match ignores ASCII case, so
/// Shift+Z steps back exactly like Z and Ctrl+Shift+F focuses the box too.
/// Plain J opens the transient Jump to File panel.
/// Alt+F and Logo+F stay unbound because they are chords for the window
/// manager, not the player. Keypad digits are handled by [`numpad_shortcut`],
/// so top-row digits stay unbound here (Ctrl+Shift+1 is handled by
/// [`playlist_sort_shortcut`]).
fn shortcut(key: &Key, modifiers: Modifiers) -> Option<Message> {
    // Ctrl+F is the one Control chord the player claims. Alt or Logo held with
    // it stays a window-manager chord, so only plain Control (plus optional
    // Shift) reaches the search box.
    if modifiers.control()
        && !modifiers.alt()
        && !modifiers.logo()
        && matches!(key, Key::Character(character) if character.eq_ignore_ascii_case("f"))
    {
        return Some(Message::FocusSearch);
    }

    // Ctrl+Z and Ctrl+B jump to the first and last song of the list Z and B step through, Ctrl+R
    // reverses the showing list, Ctrl+Shift+R shuffles it, Ctrl+Shift+Delete clears the
    // showing songs list, and Ctrl+W rolls the window up into shade mode and back,
    // Ctrl+J flips the Jump to time panel, Ctrl+A and Ctrl+T repeat the
    // classic always-on-top and elapsed/remaining toggles, Ctrl+V arms Stop after current, and
    // Ctrl+F1 opens the About panel, under the same no-Alt, no-Logo rule as Ctrl+F.
    if modifiers.control() && !modifiers.alt() && !modifiers.logo() {
        match key {
            Key::Character(character) if character.eq_ignore_ascii_case("z") => {
                return Some(Message::FirstTrack);
            }
            Key::Character(character) if character.eq_ignore_ascii_case("b") => {
                return Some(Message::LastTrack);
            }
            Key::Character(character)
                if character.eq_ignore_ascii_case("r") && !modifiers.shift() =>
            {
                return Some(Message::ReverseBrowse);
            }
            Key::Character(character) if character.eq_ignore_ascii_case("r") => {
                return Some(Message::RandomizeBrowse);
            }
            Key::Character(character) if character.eq_ignore_ascii_case("w") => {
                return Some(Message::ToggleWindowShade);
            }
            Key::Character(character)
                if character.eq_ignore_ascii_case("j") && !modifiers.shift() =>
            {
                return Some(Message::ToggleJumpTime);
            }
            Key::Character(character) if character.eq_ignore_ascii_case("a") => {
                return Some(Message::ToggleAlwaysOnTop);
            }
            Key::Character(character) if character.eq_ignore_ascii_case("t") => {
                return Some(Message::ToggleTimeDisplay);
            }
            Key::Character(character) if character.eq_ignore_ascii_case("v") => {
                return Some(Message::ToggleStopAfterCurrent);
            }
            // Ctrl+Shift+Delete clears the showing songs list (Winamp's
            // playlist editor's Remove all, beside plain Delete's Remove
            // selected), claiming the Delete chord the plain-key gate withholds.
            Key::Named(key::Named::Delete) if modifiers.shift() => {
                return Some(Message::RemoveAllBrowse);
            }
            Key::Named(key::Named::F1) => {
                return Some(Message::ToggleAbout);
            }
            _ => {}
        }
    }

    if modifiers.control() || modifiers.alt() || modifiers.logo() {
        return None;
    }

    match key {
        Key::Character(character) if character.eq_ignore_ascii_case("j") => {
            Some(Message::ToggleJumpFile)
        }
        Key::Character(character) if character.eq_ignore_ascii_case("z") => {
            Some(Message::PreviousTrack)
        }
        Key::Character(character) if character.eq_ignore_ascii_case("x") => Some(Message::Play),
        Key::Character(character) if character.eq_ignore_ascii_case("c") => Some(Message::Pause),
        Key::Character(character) if character.eq_ignore_ascii_case("v") && modifiers.shift() => {
            Some(Message::FadeOutAndStop)
        }
        Key::Character(character) if character.eq_ignore_ascii_case("v") => Some(Message::Stop),
        Key::Character(character) if character.eq_ignore_ascii_case("b") => {
            Some(Message::NextTrack)
        }
        Key::Character(character) if character.eq_ignore_ascii_case("r") => {
            Some(Message::ToggleRepeat)
        }
        Key::Character(character) if character.eq_ignore_ascii_case("s") => {
            Some(Message::ToggleShuffle)
        }
        Key::Character(character) if character.eq_ignore_ascii_case("a") => {
            Some(Message::ToggleAlwaysOnTop)
        }
        Key::Character(character) if character.eq_ignore_ascii_case("t") => {
            Some(Message::ToggleTimeDisplay)
        }
        Key::Character(character) if character.eq_ignore_ascii_case("e") => {
            Some(Message::ToggleEqualizerPanel)
        }
        Key::Named(key::Named::Enter) => Some(Message::OpenSelected),
        Key::Named(key::Named::Backspace) => Some(Message::Back),
        // Delete removes the showing songs list's selected row like Winamp's
        // playlist editor's Remove selected. Plain only: every other named-key
        // binding lets a Shift through with the no-chord gate, but a
        // Shift+Delete chord would read as an edit command, not a remove.
        Key::Named(key::Named::Delete) if !modifiers.shift() => Some(Message::RemoveSelectedBrowse),
        Key::Named(key::Named::Home) => Some(Message::ScrollBrowseToTop),
        Key::Named(key::Named::End) => Some(Message::ScrollBrowseToBottom),
        Key::Named(key::Named::PageDown) => Some(Message::ScrollBrowsePageDown),
        Key::Named(key::Named::PageUp) => Some(Message::ScrollBrowsePageUp),
        Key::Named(key::Named::F5) => Some(Message::ReloadLibrary),
        Key::Named(key::Named::ArrowUp) => Some(Message::VolumeUp),
        Key::Named(key::Named::ArrowDown) => Some(Message::VolumeDown),
        Key::Named(key::Named::ArrowLeft) => Some(Message::SeekBackward),
        Key::Named(key::Named::ArrowRight) => Some(Message::SeekForward),
        Key::Named(key::Named::MediaPlayPause) => Some(Message::PlayPause),
        Key::Named(key::Named::MediaPlay) => Some(Message::Play),
        Key::Named(key::Named::MediaPause) => Some(Message::Pause),
        Key::Named(key::Named::MediaStop) => Some(Message::Stop),
        Key::Named(key::Named::MediaTrackNext) => Some(Message::NextTrack),
        Key::Named(key::Named::MediaTrackPrevious) => Some(Message::PreviousTrack),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
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
        for modifiers in [Modifiers::CTRL, Modifiers::LOGO] {
            for named in [
                key::Named::ArrowUp,
                key::Named::ArrowDown,
                key::Named::Enter,
            ] {
                assert!(message_for_options(key_pressed(Key::Named(named), modifiers)).is_none());
            }
        }
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
        // Ctrl+F, Ctrl+Z, Ctrl+B, and Ctrl+R are the Control exceptions,
        // pinned by their own tests; Z and B stay unbound with Alt or Logo.
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
                Key::Named(key::Named::ArrowUp),
                Key::Named(key::Named::ArrowDown),
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
        // A Control chord is left to the OS, as `message_for` leaves it.
        assert!(
            message_for_jump_file(key_pressed(
                Key::Named(key::Named::ArrowUp),
                Modifiers::CTRL
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
        for modifiers in [
            Modifiers::ALT | Modifiers::CTRL,
            Modifiers::ALT | Modifiers::SHIFT,
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
        assert!(
            message_for(key_pressed(Key::Named(key::Named::Escape), Modifiers::NONE)).is_none()
        );

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
}
