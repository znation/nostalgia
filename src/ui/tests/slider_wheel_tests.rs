use super::*;

#[test]
fn balance_wheel_nudges_by_one_step_and_ignores_zero_and_nan() {
    let (mut player, state) = test_player();

    let start = state.blocking_lock().balance();
    assert_message_schedules_no_work(&mut player, Message::BalanceWheel(1.0));
    let raised = state.blocking_lock().balance();
    assert!((raised - (start + views::BALANCE_STEP)).abs() < f32::EPSILON);

    assert_message_schedules_no_work(&mut player, Message::BalanceWheel(-1.0));
    let lowered = state.blocking_lock().balance();
    assert!((lowered - start).abs() < f32::EPSILON);

    for no_op in [0.0, f32::NAN] {
        assert_message_schedules_no_work(&mut player, Message::BalanceWheel(no_op));
        assert!((state.blocking_lock().balance() - start).abs() < f32::EPSILON);
    }

    for _ in 0..200 {
        assert_message_schedules_no_work(&mut player, Message::BalanceWheel(1.0));
    }
    assert_eq!(state.blocking_lock().balance(), 1.0);
    assert_eq!(player.mixer_drag, None);

    for _ in 0..300 {
        assert_message_schedules_no_work(&mut player, Message::BalanceWheel(-1.0));
    }
    assert_eq!(state.blocking_lock().balance(), -1.0);
    assert_eq!(player.mixer_drag, None);
}

#[test]
fn eq_preamp_wheel_nudges_by_one_db_and_ignores_zero_and_nan() {
    let (mut player, state) = test_player();

    let start = state.blocking_lock().eq_preamp();
    assert_message_schedules_no_work(&mut player, Message::EqPreampWheel(1.0));
    let raised = state.blocking_lock().eq_preamp();
    assert!((raised - (start + views::EQ_STEP)).abs() < f32::EPSILON);

    assert_message_schedules_no_work(&mut player, Message::EqPreampWheel(-1.0));
    let lowered = state.blocking_lock().eq_preamp();
    assert!((lowered - start).abs() < f32::EPSILON);

    for no_op in [0.0, f32::NAN] {
        assert_message_schedules_no_work(&mut player, Message::EqPreampWheel(no_op));
        assert!((state.blocking_lock().eq_preamp() - start).abs() < f32::EPSILON);
    }

    for _ in 0..200 {
        assert_message_schedules_no_work(&mut player, Message::EqPreampWheel(1.0));
    }
    assert_eq!(state.blocking_lock().eq_preamp(), GAIN_MAX_DB);
    assert_eq!(player.mixer_drag, None);

    for _ in 0..200 {
        assert_message_schedules_no_work(&mut player, Message::EqPreampWheel(-1.0));
    }
    assert_eq!(state.blocking_lock().eq_preamp(), GAIN_MIN_DB);
    assert_eq!(player.mixer_drag, None);
}

#[test]
fn eq_band_wheel_nudges_only_its_band_and_ignores_zero_nan_and_an_out_of_range_index() {
    let (mut player, state) = test_player();

    assert_message_schedules_no_work(&mut player, Message::EqBandWheel(1, 1.0));
    let bands = state.blocking_lock().eq_bands();
    assert!((bands[1] - views::EQ_STEP).abs() < f32::EPSILON);
    assert!(
        bands
            .iter()
            .enumerate()
            .all(|(index, gain)| index == 1 || *gain == 0.0),
        "only band 1 moves: {bands:?}"
    );

    assert_message_schedules_no_work(&mut player, Message::EqBandWheel(1, -1.0));
    assert!((state.blocking_lock().eq_bands()[1] - 0.0).abs() < f32::EPSILON);

    for no_op in [
        Message::EqBandWheel(1, 0.0),
        Message::EqBandWheel(1, f32::NAN),
        Message::EqBandWheel(BAND_COUNT, 1.0),
        Message::EqBandWheel(BAND_COUNT + 5, -1.0),
    ] {
        assert_message_schedules_no_work(&mut player, no_op);
        let bands = state.blocking_lock().eq_bands();
        assert!(
            bands.iter().all(|gain| *gain == 0.0),
            "no band moves: {bands:?}"
        );
    }

    for _ in 0..200 {
        assert_message_schedules_no_work(&mut player, Message::EqBandWheel(0, 1.0));
    }
    assert_eq!(state.blocking_lock().eq_bands()[0], GAIN_MAX_DB);
    assert_eq!(state.blocking_lock().eq_bands()[1], 0.0);
    assert_eq!(player.mixer_drag, None);
}

// These wheel paths must reach the injected backend, not just shared state.
#[test]
fn slider_wheels_forward_their_new_values_to_the_audio_backend() {
    let (recording, mut player, _state) = player_with_recording();

    assert_message_schedules_no_work(&mut player, Message::BalanceWheel(1.0));
    assert_message_schedules_no_work(&mut player, Message::EqPreampWheel(1.0));
    assert_message_schedules_no_work(&mut player, Message::EqBandWheel(2, 1.0));

    let calls = recording.calls();
    assert_eq!(calls.len(), 3, "one forward per roll: {calls:?}");
    assert!(
        matches!(calls[0], AudioCall::SetBalance(balance)
            if (balance - views::BALANCE_STEP).abs() < f32::EPSILON),
        "the balance roll forwards its new balance: {calls:?}"
    );
    assert!(
        matches!(calls[1], AudioCall::SetEqualizer(snapshot)
            if snapshot.preamp_db == views::EQ_STEP),
        "the preamp roll forwards the curve: {calls:?}"
    );
    assert!(
        matches!(calls[2], AudioCall::SetEqualizer(snapshot)
            if snapshot.bands_db[2] == views::EQ_STEP),
        "the band roll forwards the moved band: {calls:?}"
    );
}
