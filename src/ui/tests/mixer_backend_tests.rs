use super::*;

// The volume arms must reach the injected backend, not just write shared
// state: a slider drag or arrow key that only moved the `AppState` value would
// leave the playing preview at its old gain. This drives each arm through
// `update` over a recording backend and pins the clamped value it forwarded,
// including a `NaN` `VolumeChange` that stores and forwards the `0.0` clamp.
#[test]
fn volume_messages_forward_to_the_audio_backend() {
    let (recording, mut player, state) = player_with_recording();

    assert_message_schedules_no_work(&mut player, Message::VolumeChange(0.2));
    assert_eq!(state.blocking_lock().volume(), 0.2);

    assert_message_schedules_no_work(&mut player, Message::VolumeUp);
    assert_eq!(state.blocking_lock().volume(), 0.2 + views::VOLUME_STEP);

    assert_message_schedules_no_work(&mut player, Message::VolumeDown);
    assert_eq!(state.blocking_lock().volume(), 0.2);

    assert_message_schedules_no_work(&mut player, Message::VolumeChange(f32::NAN));
    assert_eq!(state.blocking_lock().volume(), 0.0);

    assert_eq!(
        recording.calls(),
        vec![
            AudioCall::SetVolume(0.2),
            AudioCall::SetVolume(0.2 + views::VOLUME_STEP),
            AudioCall::SetVolume(0.2),
            AudioCall::SetVolume(0.0),
        ]
    );
}

// The balance arm must reach the injected backend, not just write shared
// state: a slider drag that only moved the `AppState` value would leave the
// playing preview centered. This drives `BalanceChange` through `update` over
// a recording backend and pins the clamped value it forwarded, including an
// out-of-range value and a `NaN` that stores and forwards the center.
#[test]
fn balance_messages_forward_to_the_audio_backend() {
    let (recording, mut player, state) = player_with_recording();

    assert_message_schedules_no_work(&mut player, Message::BalanceChange(-0.4));
    assert_eq!(state.blocking_lock().balance(), -0.4);

    assert_message_schedules_no_work(&mut player, Message::BalanceChange(1.5));
    assert_eq!(state.blocking_lock().balance(), 1.0);

    assert_message_schedules_no_work(&mut player, Message::BalanceChange(f32::NAN));
    assert_eq!(state.blocking_lock().balance(), 0.0);

    assert_eq!(
        recording.calls(),
        vec![
            AudioCall::SetBalance(-0.4),
            AudioCall::SetBalance(1.0),
            AudioCall::SetBalance(0.0),
        ]
    );
}

// A backend rejection must not roll back the volume mutation: the state write
// happens before the forward, so the slider and arrow keys keep the user's
// choice and only log the failure. The fake counts the rejected calls, proving
// the message reached the failing backend rather than skipping the forward.
#[test]
fn a_volume_backend_failure_still_commits_the_state() {
    let (audio, mut player, state) =
        player_with_failing_audio(FailingAudio::failing_volume_and_balance());

    assert_message_schedules_no_work(&mut player, Message::VolumeChange(0.3));

    assert_eq!(state.blocking_lock().volume(), 0.3);
    assert_eq!(audio.attempts(FailingCommand::Volume), 1);
    assert_eq!(audio.attempts(FailingCommand::Balance), 0);
}

// The balance arm has the same commit-then-forward contract as volume and the
// equalizer: a rejected backend call leaves the centered image the user chose
// in shared state and only logs the failure.
#[test]
fn a_balance_backend_failure_still_commits_the_state() {
    let (audio, mut player, state) =
        player_with_failing_audio(FailingAudio::failing_volume_and_balance());

    assert_message_schedules_no_work(&mut player, Message::BalanceChange(-0.4));

    assert_eq!(state.blocking_lock().balance(), -0.4);
    assert_eq!(audio.attempts(FailingCommand::Balance), 1);
    assert_eq!(audio.attempts(FailingCommand::Volume), 0);
}
