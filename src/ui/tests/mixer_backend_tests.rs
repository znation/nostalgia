use super::*;

// The equalizer arms must reach the injected backend, not just write shared
// state: a toggle, slider drag, or preset pick that only moved `AppState`
// would leave the playing preview at its old curve. This drives every arm
// through `update` over a recording backend and pins each forwarded snapshot
// against the clamped values that same message left in shared state,
// including an out-of-range band gain and a `NaN` one that stores and forwards
// the flat clamp. Every arm must schedule no work: the running source reads
// the new curve without a new `Play`.
#[test]
fn equalizer_messages_forward_to_the_audio_backend() {
    fn snapshot(state: &AppState) -> crate::audio::EqSnapshot {
        crate::audio::EqSnapshot {
            enabled: state.eq_enabled,
            preamp_db: state.eq_preamp(),
            bands_db: state.eq_bands(),
        }
    }

    let (recording, mut player, state) = player_with_recording();

    let mut expected = Vec::new();

    assert_message_schedules_no_work(&mut player, Message::ToggleEqualizer);
    expected.push(snapshot(&state.blocking_lock()));

    assert_message_schedules_no_work(&mut player, Message::EqPreampChange(3.0));
    expected.push(snapshot(&state.blocking_lock()));

    assert_message_schedules_no_work(&mut player, Message::EqBandChange(0, 4.0));
    expected.push(snapshot(&state.blocking_lock()));

    assert_message_schedules_no_work(&mut player, Message::EqBandChange(1, 99.0));
    expected.push(snapshot(&state.blocking_lock()));

    assert_message_schedules_no_work(&mut player, Message::EqBandChange(2, f32::NAN));
    expected.push(snapshot(&state.blocking_lock()));

    assert_message_schedules_no_work(
        &mut player,
        Message::EqPresetSelected(PresetChoice::BuiltIn(rock_preset())),
    );
    expected.push(snapshot(&state.blocking_lock()));

    // The values the messages stored are the clamped ones, so the forwarded
    // curve and the shared state cannot drift apart.
    assert!(expected[0].enabled);
    assert_eq!(expected[1].preamp_db, 3.0);
    assert_eq!(expected[2].bands_db[0], 4.0);
    assert_eq!(expected[3].bands_db[1], GAIN_MAX_DB);
    assert_eq!(expected[4].bands_db[2], 0.0);
    assert_eq!(expected[5].preamp_db, rock_preset().preamp);
    assert_eq!(expected[5].bands_db, rock_preset().bands);

    assert_eq!(
        recording.calls(),
        expected
            .into_iter()
            .map(AudioCall::SetEqualizer)
            .collect::<Vec<_>>()
    );
}

// A backend rejection must not roll back the equalizer mutation: the state
// write happens before the forward, exactly as in the volume and balance
// helpers, so the panel keeps the user's choice and only logs the failure.
// The fake counts the rejected calls, proving the message reached the failing
// backend rather than skipping the forward.
#[test]
fn an_equalizer_backend_failure_still_commits_the_state() {
    let (audio, mut player, state) = player_with_failing_audio(FailingAudio::failing_equalizer());

    assert_message_schedules_no_work(&mut player, Message::EqBandChange(0, 4.0));

    assert_eq!(state.blocking_lock().eq_bands()[0], 4.0);
    assert_eq!(audio.attempts(FailingCommand::Equalizer), 1);
}

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
