use super::*;
use iced::keyboard::Modifiers;

#[test]
fn ctrl_click_previous_plays_the_first_song() {
    let (mut player, _state) = player_stepping_from(Some("song-2"));
    let _ = update(
        &mut player,
        Message::ModifiersHeld {
            control: true,
            shift: false,
        },
    );

    let task = update(&mut player, Message::PreviousButton);

    assert_track_selected(task, 0, 0);
}

#[test]
fn ctrl_click_next_plays_the_last_song() {
    let (mut player, _state) = player_stepping_from(Some("song-1"));
    let _ = update(
        &mut player,
        Message::ModifiersHeld {
            control: true,
            shift: false,
        },
    );

    let task = update(&mut player, Message::NextButton);

    assert_track_selected(task, 0, stepping_songs().len() - 1);
}

#[test]
fn plain_button_clicks_step_one_song() {
    let (mut plain, _state) = player_stepping_from(Some("song-1"));
    let (mut keyed, _keyed_state) = player_stepping_from(Some("song-1"));
    assert!(!plain.control_held);
    assert_track_selected(update(&mut plain, Message::NextButton), 0, 1);
    assert_track_selected(update(&mut keyed, Message::NextTrack), 0, 1);

    let (mut plain, _state) = player_stepping_from(Some("song-2"));
    let (mut keyed, _keyed_state) = player_stepping_from(Some("song-2"));
    assert_track_selected(update(&mut plain, Message::PreviousButton), 0, 0);
    assert_track_selected(update(&mut keyed, Message::PreviousTrack), 0, 0);
}

#[test]
fn control_release_restores_plain_steps() {
    let (mut player, _state) = player_stepping_from(Some("song-1"));
    let _ = update(
        &mut player,
        Message::ModifiersHeld {
            control: true,
            shift: false,
        },
    );
    let _ = update(
        &mut player,
        Message::ModifiersHeld {
            control: false,
            shift: false,
        },
    );

    let task = update(&mut player, Message::NextButton);

    assert_track_selected(task, 0, 1);
}

#[test]
fn modifiers_message_tracks_control_and_focus_loss() {
    use iced::event::Status;
    let id = iced::window::Id::unique();
    let modifiers = |m| iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(m));

    assert!(matches!(
        modifiers_message(modifiers(Modifiers::CTRL), Status::Ignored, id),
        Some(Message::ModifiersHeld {
            control: true,
            shift: false
        })
    ));
    assert!(matches!(
        modifiers_message(modifiers(Modifiers::SHIFT), Status::Ignored, id),
        Some(Message::ModifiersHeld {
            control: false,
            shift: true
        })
    ));
    assert!(matches!(
        modifiers_message(
            iced::Event::Window(iced::window::Event::Unfocused),
            Status::Ignored,
            id
        ),
        Some(Message::ModifiersHeld {
            control: false,
            shift: false
        })
    ));
    let key = iced::Event::Keyboard(iced::keyboard::Event::KeyReleased {
        key: iced::keyboard::Key::Character("a".into()),
        location: iced::keyboard::Location::Standard,
        modifiers: Modifiers::empty(),
        modified_key: iced::keyboard::Key::Character("a".into()),
        physical_key: iced::keyboard::key::Physical::Unidentified(
            iced::keyboard::key::NativeCode::Unidentified,
        ),
    });
    assert!(modifiers_message(key, Status::Ignored, id).is_none());
}

#[test]
fn window_moved_message_maps_only_move_events() {
    use iced::event::Status;
    let id = iced::window::Id::unique();
    let point = iced::Point::new(12.0, 34.0);

    assert!(matches!(
        window_moved_message(
            iced::Event::Window(iced::window::Event::Moved(point)),
            Status::Ignored,
            id
        ),
        Some(Message::WindowMoved(moved)) if moved == point
    ));
    assert!(
        window_moved_message(
            iced::Event::Window(iced::window::Event::Focused),
            Status::Ignored,
            id
        )
        .is_none()
    );
    let key = iced::Event::Keyboard(iced::keyboard::Event::KeyReleased {
        key: iced::keyboard::Key::Character("a".into()),
        location: iced::keyboard::Location::Standard,
        modifiers: Modifiers::empty(),
        modified_key: iced::keyboard::Key::Character("a".into()),
        physical_key: iced::keyboard::key::Physical::Unidentified(
            iced::keyboard::key::NativeCode::Unidentified,
        ),
    });
    assert!(window_moved_message(key, Status::Ignored, id).is_none());
}

#[test]
fn shift_click_previous_steps_one_song() {
    let (mut player, _state) = player_stepping_from(Some("song-2"));
    let _ = update(
        &mut player,
        Message::ModifiersHeld {
            control: false,
            shift: true,
        },
    );

    let task = update(&mut player, Message::PreviousButton);

    assert_track_selected(task, 0, 0);
}
