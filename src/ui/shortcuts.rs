//! The classic Winamp keyboard bindings: mapping an iced key event to the
//! transport `Message` it triggers.
//!
//! The player's keys are a player concern, so the key-to-message table lives
//! here as pure functions, mirroring `views`, `transport`, and `loading`. They
//! know the `Message` type and iced's key event, not `WinampPlayer` or the
//! update loop, so `init_ui` installs [`message_for`] as the window
//! subscription's mapper. Its unit tests live in the `tests` submodule.

use iced::keyboard::{Event, Key, Modifiers, key};

use super::Message;

/// Maps a keyboard event to the classic Winamp transport shortcut, or `None`
/// for every event that binds nothing: a key release, a modifier-only change,
/// an unbound key, or a key pressed with a chord modifier. `init_ui` installs
/// this as the window subscription's mapper, so the key-to-message table
/// lives here once and the subscription stays a plain `filter_map`.
pub(super) fn message_for(event: Event) -> Option<Message> {
    match event {
        Event::KeyPressed { key, modifiers, .. } => shortcut(&key, modifiers),
        Event::KeyReleased { .. } | Event::ModifiersChanged(_) => None,
    }
}

/// The classic Winamp main-window bindings: Z previous, X play, C pause, V
/// stop, B next, and the arrow keys nudge the volume. A Control, Alt, or Logo
/// chord is left to the OS and window manager, so those modifiers yield
/// `None`; Shift is allowed, and the letter match ignores ASCII case, so
/// Shift+Z steps back exactly like Z.
fn shortcut(key: &Key, modifiers: Modifiers) -> Option<Message> {
    if modifiers.control() || modifiers.alt() || modifiers.logo() {
        return None;
    }

    match key {
        Key::Character(character) if character.eq_ignore_ascii_case("z") => {
            Some(Message::PreviousTrack)
        }
        Key::Character(character) if character.eq_ignore_ascii_case("x") => Some(Message::Play),
        Key::Character(character) if character.eq_ignore_ascii_case("c") => Some(Message::Pause),
        Key::Character(character) if character.eq_ignore_ascii_case("v") => Some(Message::Stop),
        Key::Character(character) if character.eq_ignore_ascii_case("b") => {
            Some(Message::NextTrack)
        }
        Key::Named(key::Named::ArrowUp) => Some(Message::VolumeUp),
        Key::Named(key::Named::ArrowDown) => Some(Message::VolumeDown),
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

    /// Asserts each classic letter binding maps to its transport message, by
    /// driving `map` with every letter. The tests below pass different mappers:
    /// one wraps the letter in a `KeyPressed` event for `message_for`, the other
    /// feeds `shortcut` an uppercase letter with Shift held.
    fn assert_letter_bindings(map: impl Fn(&str) -> Option<Message>) {
        assert!(matches!(map("z"), Some(Message::PreviousTrack)));
        assert!(matches!(map("x"), Some(Message::Play)));
        assert!(matches!(map("c"), Some(Message::Pause)));
        assert!(matches!(map("v"), Some(Message::Stop)));
        assert!(matches!(map("b"), Some(Message::NextTrack)));
    }

    #[test]
    fn message_for_maps_each_bound_letter() {
        assert_letter_bindings(|letter| {
            message_for(key_pressed(Key::Character(letter.into()), Modifiers::NONE))
        });
    }

    #[test]
    fn message_for_maps_the_arrow_keys() {
        assert!(matches!(
            message_for(key_pressed(
                Key::Named(key::Named::ArrowUp),
                Modifiers::NONE
            )),
            Some(Message::VolumeUp)
        ));
        assert!(matches!(
            message_for(key_pressed(
                Key::Named(key::Named::ArrowDown),
                Modifiers::NONE
            )),
            Some(Message::VolumeDown)
        ));
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
    fn shortcut_ignores_chord_modifiers() {
        // Control, Alt, and Logo chords belong to the OS and window manager.
        for modifiers in [Modifiers::CTRL, Modifiers::ALT, Modifiers::LOGO] {
            for key in [
                Key::Character("z".into()),
                Key::Character("x".into()),
                Key::Named(key::Named::ArrowUp),
                Key::Named(key::Named::ArrowDown),
            ] {
                assert!(
                    shortcut(&key, modifiers).is_none(),
                    "{key:?} with {modifiers:?} must not map"
                );
            }
        }
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
