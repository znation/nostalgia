//! Event filters for the always-on and panel-scoped iced listeners: they turn
//! Escape, mixer-navigation keys, and wheel rolls into the messages that
//! dismiss a panel or end a mixer readout.
//!
//! [`super::subscriptions`] installs each one through `iced::event::listen_with`
//! only while it applies.

use iced::keyboard;

use crate::ui::Message;

/// Subscribed only while a transient panel is open. A plain or Shift-held
/// Escape press maps to [`Message::EscapePressed`]; Escape with Control, Alt,
/// or Logo held, an Escape release, and every other event map to nothing. The
/// status is ignored deliberately: the press must be seen even when a focused
/// `TextInput` captures Escape and clears its own focus.
pub(in crate::ui) fn escape_key_message(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    use keyboard::key::Named;
    match event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(Named::Escape),
            modifiers,
            ..
        }) if !modifiers.control() && !modifiers.alt() && !modifiers.logo() => {
            Some(Message::EscapePressed)
        }
        _ => None,
    }
}

/// Always subscribed, since a slider's own change message sets the readout
/// during the same event this must then see. Ends the readout on an arrow,
/// Home/End, or Page key press, which a focused slider turns into a change
/// message with no drag and so no release. Other keys and every wheel roll
/// send nothing, so typing and list scrolling rebuild no screen.
pub(in crate::ui) fn mixer_key_message(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    use keyboard::key::Named;
    match event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key:
                keyboard::Key::Named(
                    Named::ArrowUp
                    | Named::ArrowDown
                    | Named::ArrowLeft
                    | Named::ArrowRight
                    | Named::Home
                    | Named::End
                    | Named::PageUp
                    | Named::PageDown,
                ),
            ..
        }) => Some(Message::MixerReleased),
        _ => None,
    }
}

/// Subscribed only while a readout is showing, so wheel rolls rebuild the
/// screen only then. The Ctrl+wheel change that sets a readout arrives while
/// this is absent; the next wheel roll or navigation key clears it.
pub(in crate::ui) fn mixer_wheel_message(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Mouse(iced::mouse::Event::WheelScrolled { .. }) => {
            Some(Message::MixerReleased)
        }
        _ => None,
    }
}
