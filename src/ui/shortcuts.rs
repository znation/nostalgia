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
/// and Alt+Down move the selected browse row one place while Shift+Alt+Up and
/// Shift+Alt+Down take it to the list's start or end
/// (see [`move_row_shortcut`]).
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
/// Winamp's playlist editor, and Shift+Alt+Up and Shift+Alt+Down take it all
/// the way to the list's first or last row. The logical arrow key is matched
/// wherever it sits; Alt must be held with Control and Logo released, while
/// Shift only widens the move; plain arrows stay volume.
fn move_row_shortcut(key: &Key, modifiers: Modifiers) -> Option<Message> {
    if !modifiers.alt() || modifiers.control() || modifiers.logo() {
        return None;
    }
    match (key, modifiers.shift()) {
        (Key::Named(key::Named::ArrowUp), false) => Some(Message::MoveSelectedRowUp),
        (Key::Named(key::Named::ArrowDown), false) => Some(Message::MoveSelectedRowDown),
        (Key::Named(key::Named::ArrowUp), true) => Some(Message::MoveSelectedRowToTop),
        (Key::Named(key::Named::ArrowDown), true) => Some(Message::MoveSelectedRowToBottom),
        // Shift+Alt+Left/Right stays unbound so a plain Shift+arrow keeps the
        // volume and seek arms unclaimed.
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
/// the classic always-on-top and elapsed/remaining toggles), Ctrl+Up and
/// Ctrl+Down (step the browse selection bar one row up or down over the
/// showing list, like the playlist editor's selection keys), and Ctrl+F1 (open
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
    // Ctrl+F is the one Control letter chord the player claims; Ctrl+Up and
    // Ctrl+Down, matched below, are the Control arrow ones, and every other
    // Control, Alt, or Logo chord is left to the OS and window manager. Alt
    // or Logo held with the chord stays a window-manager chord, so only plain
    // Control (plus optional Shift) reaches the search box.
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
            // Ctrl+Up and Ctrl+Down step the browse selection bar over the
            // showing list, Winamp's playlist editor's selection keys, beside
            // the Alt+arrow row moves. Shift held with the chord maps the
            // same way: the arrows carry no shift behavior anywhere.
            Key::Named(key::Named::ArrowUp) => {
                return Some(Message::MoveBrowseSelectionUp);
            }
            Key::Named(key::Named::ArrowDown) => {
                return Some(Message::MoveBrowseSelectionDown);
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
mod tests;
