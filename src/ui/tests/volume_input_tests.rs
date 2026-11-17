use super::*;

#[test]
fn volume_up_and_down_nudge_by_the_slider_step() {
    let (mut player, state) = test_player();

    let start = state.blocking_lock().volume();
    assert_message_schedules_no_work(&mut player, Message::VolumeUp);
    let raised = state.blocking_lock().volume();
    assert!((raised - (start + views::VOLUME_STEP)).abs() < f32::EPSILON);

    assert_message_schedules_no_work(&mut player, Message::VolumeDown);
    let lowered = state.blocking_lock().volume();
    assert!((lowered - start).abs() < f32::EPSILON);
}

#[test]
fn volume_keys_clamp_at_the_ends() {
    let (mut player, state) = test_player();

    for _ in 0..200 {
        assert_message_schedules_no_work(&mut player, Message::VolumeUp);
    }
    assert_eq!(state.blocking_lock().volume(), 1.0);

    for _ in 0..200 {
        assert_message_schedules_no_work(&mut player, Message::VolumeDown);
    }
    assert_eq!(state.blocking_lock().volume(), 0.0);
}

#[test]
fn volume_wheel_nudges_by_one_slider_step_and_ignores_zero() {
    let (mut player, state) = test_player();

    let start = state.blocking_lock().volume();
    assert_message_schedules_no_work(&mut player, Message::VolumeWheel(1.0));
    let raised = state.blocking_lock().volume();
    assert!((raised - (start + views::VOLUME_STEP)).abs() < f32::EPSILON);

    assert_message_schedules_no_work(&mut player, Message::VolumeWheel(-1.0));
    let lowered = state.blocking_lock().volume();
    assert!((lowered - start).abs() < f32::EPSILON);

    assert_message_schedules_no_work(&mut player, Message::VolumeWheel(0.0));
    assert!((state.blocking_lock().volume() - start).abs() < f32::EPSILON);
}

#[test]
fn volume_wheel_clamps_at_the_ends() {
    let (mut player, state) = test_player();

    for _ in 0..200 {
        assert_message_schedules_no_work(&mut player, Message::VolumeWheel(1.0));
    }
    assert_eq!(state.blocking_lock().volume(), 1.0);

    for _ in 0..200 {
        assert_message_schedules_no_work(&mut player, Message::VolumeWheel(-1.0));
    }
    assert_eq!(state.blocking_lock().volume(), 0.0);
}
