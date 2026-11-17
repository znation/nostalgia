use super::*;

// A slider's own arrow-key or Ctrl+wheel change message arrives with no drag
// and no release; the key or wheel event that caused it ends the readout.
#[test]
fn a_key_or_wheel_event_ends_a_mixer_readout() {
    use iced::{Event, event::Status, keyboard, mouse, window};
    let (mut player, _state) = test_player();
    let wheel = Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
    });
    let key = Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Named(keyboard::key::Named::ArrowUp),
        modified_key: keyboard::Key::Named(keyboard::key::Named::ArrowUp),
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    });
    let _ = update(&mut player, Message::VolumeChange(0.4));
    assert!(player.mixer_drag.is_some());
    let message = mixer_key_message(key, Status::Captured, window::Id::unique())
        .expect("a navigation key maps to a message");
    let _ = update(&mut player, message);
    assert_eq!(player.mixer_drag, None);
    let _ = update(&mut player, Message::VolumeChange(0.4));
    let message = mixer_wheel_message(wheel.clone(), Status::Captured, window::Id::unique())
        .expect("a wheel event maps to a message");
    let _ = update(&mut player, message);
    assert_eq!(player.mixer_drag, None);
    // Plain scrolling and typing send nothing, so they rebuild no screen.
    assert!(mixer_key_message(wheel, Status::Ignored, window::Id::unique()).is_none());
    let letter = Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Character("a".into()),
        modified_key: keyboard::Key::Character("a".into()),
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    });
    assert!(mixer_key_message(letter, Status::Ignored, window::Id::unique()).is_none());
    let moved = Event::Mouse(mouse::Event::CursorMoved {
        position: iced::Point::ORIGIN,
    });
    assert!(mixer_wheel_message(moved, Status::Ignored, window::Id::unique()).is_none());
}

#[test]
fn only_the_slider_navigation_keys_end_a_mixer_readout() {
    use iced::keyboard::key::Named;
    use iced::{Event, event::Status, keyboard, window};
    let event = |named: Named, pressed: bool| {
        let key = keyboard::Key::Named(named);
        let physical_key =
            keyboard::key::Physical::Unidentified(keyboard::key::NativeCode::Unidentified);
        Event::Keyboard(if pressed {
            keyboard::Event::KeyPressed {
                key: key.clone(),
                modified_key: key,
                physical_key,
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::empty(),
                text: None,
                repeat: false,
            }
        } else {
            keyboard::Event::KeyReleased {
                key: key.clone(),
                modified_key: key,
                physical_key,
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::empty(),
            }
        })
    };
    let navigation = [
        Named::ArrowUp,
        Named::ArrowDown,
        Named::ArrowLeft,
        Named::ArrowRight,
        Named::Home,
        Named::End,
        Named::PageUp,
        Named::PageDown,
    ];
    for named in navigation {
        assert!(
            matches!(
                mixer_key_message(event(named, true), Status::Ignored, window::Id::unique()),
                Some(Message::MixerReleased)
            ),
            "{named:?} should end the readout"
        );
        assert!(
            mixer_key_message(event(named, false), Status::Ignored, window::Id::unique()).is_none(),
            "releasing {named:?} should send nothing"
        );
    }
    for named in [Named::Enter, Named::Space, Named::Tab, Named::Escape] {
        assert!(
            mixer_key_message(event(named, true), Status::Ignored, window::Id::unique()).is_none(),
            "{named:?} should send nothing"
        );
    }
}
