use super::*;

/// Asserts the shared state holds `preset`'s preamp and band gains.
fn assert_curve_applied(state: &AppState, preset: crate::equalizer::Preset) {
    assert_eq!(state.eq_preamp(), preset.preamp);
    assert_eq!(state.eq_bands(), preset.bands);
}

#[test]
fn toggle_equalizer_flips_shared_state() {
    assert_toggles_shared_state(Message::ToggleEqualizer, |state| state.eq_enabled);
}

// The AUTO button flips only its own flag: the stored curve, preset, and
// equalizer on/off state stay put, and the toggle schedules no backend work.
#[test]
fn toggle_eq_auto_flips_only_the_auto_flag() {
    let (mut player, state) = test_player();
    let preset = rock_preset();
    let _ = update(&mut player, Message::EqPresetSelected(preset));

    assert_message_schedules_no_work(&mut player, Message::ToggleEqAuto);
    {
        let state = state.blocking_lock();
        assert!(state.eq_auto);
        assert!(!state.eq_enabled);
        assert_eq!(state.eq_preset(), Some(preset));
        assert_curve_applied(&state, preset);
    }

    assert_message_schedules_no_work(&mut player, Message::ToggleEqAuto);
    let state = state.blocking_lock();
    assert!(!state.eq_auto);
    assert_eq!(state.eq_preset(), Some(preset));
    assert_eq!(state.eq_bands(), preset.bands);
}

// AUTO tracks the selected song's primary genre through the same
// mutate-and-forward path as a pick-list pick: a trimmed, case-insensitive
// match applies the whole preset and reaches the audio backend, while an
// unmatched genre or AUTO off leaves the previous Rock curve and its
// forwarded calls alone.
#[test]
fn auto_on_applies_a_genre_matched_preset_on_track_selection() {
    let (recording, mut player, state) = player_with_recording();
    let mut songs = stepping_songs();
    songs[0].genre = "  rOcK  ".to_string();
    songs[1].genre = "Alternative".to_string();
    songs[2].genre = "Vocal".to_string();
    let _ = update(&mut player, Message::SongsLoaded(songs));

    let vocal = *crate::equalizer::PRESETS
        .iter()
        .find(|preset| preset.name == "Vocal")
        .unwrap();
    let _ = update(&mut player, Message::EqPresetSelected(vocal));
    state.blocking_lock().eq_auto = true;

    let before = recording.calls().len();
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    let rock = rock_preset();
    {
        let state = state.blocking_lock();
        assert_eq!(state.eq_preset(), Some(rock));
        assert_curve_applied(&state, rock);
    }
    let calls = recording.calls();
    assert_eq!(
        calls.len(),
        before + 1,
        "the matched preset is forwarded once"
    );
    match calls.last() {
        Some(AudioCall::SetEqualizer(snapshot)) => {
            assert_eq!(snapshot.preamp_db, rock.preamp);
            assert_eq!(snapshot.bands_db, rock.bands);
        }
        other => panic!("expected the Rock curve to reach the backend, got {other:?}"),
    }

    // An unmatched genre changes neither the stored curve nor the backend.
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 1 });
    assert_eq!(state.blocking_lock().eq_preset(), Some(rock));
    assert_eq!(recording.calls().len(), calls.len());

    // With AUTO off, even a genre that names another preset leaves the curve
    // and the backend alone.
    state.blocking_lock().eq_auto = false;
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 2 });
    let state = state.blocking_lock();
    assert_eq!(state.eq_preset(), Some(rock));
    assert_eq!(state.eq_bands(), rock.bands);
    assert_eq!(recording.calls().len(), calls.len());
}

#[test]
fn eq_preamp_change_clamps_value_before_storing() {
    assert_message_clamps(
        Message::EqPreampChange(99.0),
        |state| state.eq_preamp(),
        GAIN_MAX_DB,
    );
    assert_message_clamps(
        Message::EqPreampChange(-99.0),
        |state| state.eq_preamp(),
        GAIN_MIN_DB,
    );
}

#[test]
fn eq_band_change_clamps_value_before_storing() {
    assert_message_clamps(
        Message::EqBandChange(0, 99.0),
        |state| state.eq_bands()[0],
        GAIN_MAX_DB,
    );
    assert_message_clamps(
        Message::EqBandChange(1, -99.0),
        |state| state.eq_bands()[1],
        GAIN_MIN_DB,
    );
}

// The EqPresetSelected arm writes a whole curve and records the selection,
// which the one-value slider-arm tests above do not cover. Drive a preset
// through `update` and read all three shared-state fields back.
#[test]
fn eq_preset_selected_applies_the_curve_and_selection() {
    let (mut player, state) = test_player();
    let preset = rock_preset();

    let _ = update(&mut player, Message::EqPresetSelected(preset));

    let state = state.blocking_lock();
    assert_curve_applied(&state, preset);
    assert_eq!(state.eq_preset(), Some(preset));
}

#[test]
fn eq_scale_click_sets_every_band_and_keeps_the_preamp() {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::EqPresetSelected(rock_preset()));

    let _ = update(&mut player, Message::EqBandsSetAll(GAIN_MIN_DB));

    let state = state.blocking_lock();
    assert_eq!(state.eq_bands(), [GAIN_MIN_DB; BAND_COUNT]);
    assert_eq!(state.eq_preamp(), rock_preset().preamp);
    assert_eq!(state.eq_preset(), None);
    assert_eq!(player.mixer_drag, None);
}

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

#[test]
fn volume_change_clamps_value_before_storing() {
    // Out-of-range slider values are clamped by the update arm.
    assert_message_clamps(Message::VolumeChange(1.5), |state| state.volume(), 1.0);
    assert_message_clamps(Message::VolumeChange(-0.2), |state| state.volume(), 0.0);

    // An in-range value is stored as-is.
    assert_message_clamps(Message::VolumeChange(0.3), |state| state.volume(), 0.3);
}

#[test]
fn balance_change_clamps_value_before_storing() {
    // Out-of-range slider values are clamped by the update arm.
    assert_message_clamps(Message::BalanceChange(1.5), |state| state.balance(), 1.0);
    assert_message_clamps(Message::BalanceChange(-2.0), |state| state.balance(), -1.0);

    // An in-range value is stored as-is.
    assert_message_clamps(Message::BalanceChange(-0.4), |state| state.balance(), -0.4);
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

// The three new wheel paths must reach the injected backend, not just shared
// state, exactly like the drags they mirror. Drive each through `update` over a
// recording backend and pin the forwarded value.
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

    assert_message_schedules_no_work(&mut player, Message::EqPresetSelected(rock_preset()));
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

// A roll over the drawn Now Playing seek slider seeks instead of nudging the
// volume.
#[test]
fn wheel_over_the_root_seek_slider_seeks_without_nudging_the_volume() {
    let (mut player, _state) = test_player();
    player.playback_position = Some(Duration::from_secs(45));
    player.playback_duration = Some(Duration::from_secs(180));
    let bar = captured_root_view_container(&player, views::SEEK_BAR_ID)
        .expect("a drawn seek slider is laid out");

    for (y, forward) in [(1.0, true), (-1.0, false)] {
        let messages = root_wheel_messages(&player, bar, y);
        let seeks: Vec<_> = messages
            .iter()
            .filter(|m| matches!(m, Message::SeekForward | Message::SeekBackward))
            .collect();
        assert_eq!(seeks.len(), 1, "exactly one seek expected: {messages:?}");
        assert_eq!(matches!(seeks[0], Message::SeekForward), forward);
        assert!(
            !messages
                .iter()
                .any(|m| matches!(m, Message::VolumeWheel(_))),
            "the roll must not nudge the volume: {messages:?}"
        );
    }
}

// With no position or length the strip is blank, so a roll over it is still
// the panel's volume nudge.
#[test]
fn wheel_over_a_blank_seek_strip_still_nudges_the_volume() {
    let (mut player, _state) = test_player();
    player.playback_position = None;
    player.playback_duration = None;
    assert!(captured_root_view_container(&player, views::SEEK_BAR_ID).is_none());
    let strip = captured_root_view_container(&player, views::SEEK_STRIP_ID)
        .expect("the blank strip is laid out");

    let messages = root_wheel_messages(&player, strip, 1.0);
    let volume = messages
        .iter()
        .filter(|m| matches!(m, Message::VolumeWheel(_)))
        .count();
    assert_eq!(volume, 1, "one volume nudge expected: {messages:?}");
    assert!(
        !messages
            .iter()
            .any(|m| matches!(m, Message::SeekForward | Message::SeekBackward)),
        "a blank strip must not seek: {messages:?}"
    );
}

// A plain roll over the balance slider moves the balance instead of the
// volume, in both directions.
#[test]
fn wheel_over_the_balance_slider_nudges_the_balance_not_the_volume() {
    let (player, _state) = test_player();
    let slider = captured_root_view_container(&player, views::BALANCE_SLIDER_ID)
        .expect("the balance slider is laid out");

    for (y, expected) in [(1.0, 1.0), (-1.0, -1.0)] {
        let messages = root_wheel_messages(&player, slider, y);
        let wheels: Vec<&f32> = messages
            .iter()
            .filter_map(|message| match message {
                Message::BalanceWheel(direction) => Some(direction),
                _ => None,
            })
            .collect();
        assert_eq!(
            wheels.len(),
            1,
            "exactly one balance roll expected: {messages:?}"
        );
        assert_eq!(*wheels[0], expected);
        assert!(
            !messages
                .iter()
                .any(|message| matches!(message, Message::VolumeWheel(_))),
            "the roll must not nudge the volume: {messages:?}"
        );
    }
}

// A plain roll over the equalizer's preamp or any band moves that slider
// instead of the volume.
#[test]
fn wheel_over_the_preamp_and_every_band_nudges_that_slider_not_the_volume() {
    let (mut player, _state) = test_player();
    player.eq_panel_visible = true;

    let preamp = captured_root_view_container(&player, views::EQ_PREAMP_SLIDER_ID)
        .expect("the preamp slider is laid out");
    // The slider itself is 16px wide at the column's left, narrower than the
    // band label that sets the column's width, so aim at the slider's centre
    // rather than the container's.
    let slider_area = |bounds: iced::Rectangle| iced::Rectangle {
        width: 16.0,
        height: 100.0,
        ..bounds
    };
    let messages = root_wheel_messages(&player, slider_area(preamp), 1.0);
    let preamp_wheels: Vec<&f32> = messages
        .iter()
        .filter_map(|message| match message {
            Message::EqPreampWheel(direction) => Some(direction),
            _ => None,
        })
        .collect();
    assert_eq!(
        preamp_wheels.len(),
        1,
        "exactly one preamp roll expected: {messages:?}"
    );
    assert_eq!(*preamp_wheels[0], 1.0);
    assert!(
        !messages
            .iter()
            .any(|message| matches!(message, Message::VolumeWheel(_))),
        "the preamp roll must not nudge the volume: {messages:?}"
    );

    for index in 0..BAND_COUNT {
        let band = captured_root_view_container(&player, &views::eq_band_slider_id(index))
            .unwrap_or_else(|| panic!("band {index} is laid out"));
        let messages = root_wheel_messages(&player, slider_area(band), 1.0);
        let band_wheels: Vec<(usize, f32)> = messages
            .iter()
            .filter_map(|message| match message {
                Message::EqBandWheel(moved, direction) => Some((*moved, *direction)),
                _ => None,
            })
            .collect();
        assert_eq!(
            band_wheels,
            vec![(index, 1.0)],
            "only band {index} moves: {messages:?}"
        );
        assert!(
            !messages
                .iter()
                .any(|message| matches!(message, Message::VolumeWheel(_))),
            "the band roll must not nudge the volume: {messages:?}"
        );
    }
}

// Dragging the volume slider swaps the LCD title for the live level, and the
// release brings the title back.
#[test]
fn dragging_volume_shows_the_level_until_release() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    assert!(root_view_draws(&player, "Tune"));

    let _ = update(&mut player, Message::VolumeChange(0.4));
    assert_eq!(player.mixer_drag, Some(views::MixerDrag::Volume));
    assert!(root_view_draws(&player, "Volume: 40%"));
    assert!(!root_view_draws(&player, "Tune"));

    let _ = update(&mut player, Message::MixerReleased);
    assert_eq!(player.mixer_drag, None);
    assert!(root_view_draws(&player, "Tune"));
    assert!(!root_view_draws(&player, "Volume: 40%"));
}

#[test]
fn dragging_the_seek_slider_shows_the_target_until_release() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    player.playback_duration = Some(Duration::from_secs(30));
    let _ = update(&mut player, Message::SeekPreviewChanged(12.5));
    assert!(root_view_draws(&player, "Seek to: 00:12/00:30 (41%)"));
    assert!(!root_view_draws(&player, "Tune"));

    let _ = update(&mut player, Message::SeekPreviewReleased);
    assert!(root_view_draws(&player, "Tune"));
    assert!(!root_view_draws(&player, "Seek to: 00:12/00:30 (41%)"));
}

#[test]
fn a_seek_drag_without_a_duration_shows_no_seek_text() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    player.playback_duration = None;
    player.seek_preview = Some(Duration::from_secs(12));
    assert!(!root_view_draws(&player, "Seek to: 00:12/00:00 (0%)"));
    assert!(root_view_draws(&player, "Tune"));
}

#[test]
fn the_marquee_pauses_during_a_seek_drag() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "A Title Too Long For The LCD Bar");
    player.playback_duration = Some(Duration::from_secs(30));
    assert!(player.needs_marquee());
    let _ = update(&mut player, Message::SeekPreviewChanged(12.5));
    assert!(!player.needs_marquee());
    let _ = update(&mut player, Message::SeekPreviewReleased);
    assert!(player.needs_marquee());
}

#[test]
fn dragging_an_eq_band_shows_its_gain_until_release() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    let _ = update(&mut player, Message::EqBandChange(0, 5.0));
    assert_eq!(player.mixer_drag, Some(views::MixerDrag::EqBand(0)));
    assert!(root_view_draws(&player, "EQ: 60HZ +5 DB"));
    assert!(!root_view_draws(&player, "Tune"));
    let _ = update(&mut player, Message::MixerReleased);
    assert_eq!(player.mixer_drag, None);
    assert!(root_view_draws(&player, "Tune"));
}

#[test]
fn dragging_the_preamp_shows_its_gain() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    let _ = update(&mut player, Message::EqPreampChange(-3.0));
    assert_eq!(player.mixer_drag, Some(views::MixerDrag::EqPreamp));
    assert!(root_view_draws(&player, "Preamp: -3 DB"));
}

#[test]
fn eq_toggle_and_preset_do_not_show_the_readout() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::ToggleEqualizer);
    assert_eq!(player.mixer_drag, None);
    let _ = update(&mut player, Message::EqPresetSelected(rock_preset()));
    assert_eq!(player.mixer_drag, None);
}

#[test]
fn an_out_of_range_eq_band_shows_no_readout() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::EqBandChange(crate::equalizer::BAND_COUNT, 5.0),
    );
    assert_eq!(player.mixer_drag, None);
}

#[test]
fn dragging_balance_shows_the_side() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    let _ = update(&mut player, Message::BalanceChange(-0.25));
    assert_eq!(player.mixer_drag, Some(views::MixerDrag::Balance));
    assert!(root_view_draws(&player, "Balance: 25% Left"));
}

#[test]
fn dragging_balance_into_the_middle_snaps_to_center() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    let _ = update(&mut player, Message::BalanceChange(-0.4));
    let _ = update(&mut player, Message::BalanceChange(-0.05));
    assert_eq!(state.blocking_lock().balance(), 0.0);
    assert!(root_view_draws(&player, "Balance: Center"));
}

// A real drag sends many values; continuing inward through the zone stays
// snapped, while moving outward again leaves center.
#[test]
fn a_continuous_inward_balance_drag_stays_snapped() {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::BalanceChange(-0.4));
    for v in [-0.05, -0.04, -0.02, 0.01] {
        let _ = update(&mut player, Message::BalanceChange(v));
        assert_eq!(state.blocking_lock().balance(), 0.0, "at {v}");
    }
    let _ = update(&mut player, Message::BalanceChange(0.06));
    assert_eq!(state.blocking_lock().balance(), 0.06);
    let _ = update(&mut player, Message::MixerReleased);
}

// After a release, a lone step out of center (an arrow key) is not snapped.
#[test]
fn a_balance_step_after_release_leaves_center() {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::BalanceChange(-0.4));
    let _ = update(&mut player, Message::BalanceChange(-0.05));
    let _ = update(&mut player, Message::MixerReleased);
    let _ = update(&mut player, Message::BalanceChange(0.01));
    assert_eq!(state.blocking_lock().balance(), 0.01);
}

// Keys and the wheel change the volume without showing the readout.
#[test]
fn volume_keys_and_wheel_do_not_show_the_readout() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::VolumeUp);
    assert_eq!(player.mixer_drag, None);
    let _ = update(&mut player, Message::VolumeWheel(1.0));
    assert_eq!(player.mixer_drag, None);
}

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

#[test]
fn the_marquee_pauses_during_a_mixer_drag() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "A Title Too Long For The LCD Bar");
    assert!(player.needs_marquee());
    let _ = update(&mut player, Message::VolumeChange(0.5));
    assert!(!player.needs_marquee());
    let _ = update(&mut player, Message::MixerReleased);
    assert!(player.needs_marquee());
}

#[test]
fn the_marquee_pauses_while_the_title_is_dragged_and_resumes_from_there() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "A Title Too Long For The LCD Bar");
    assert!(player.needs_marquee());
    let _ = update(&mut player, Message::TitleDragPressed);
    assert!(!player.needs_marquee());
    let _ = update(&mut player, Message::TitleDragMoved(100.0));
    assert_eq!(player.marquee_offset, 0, "the first move only anchors");
    let _ = update(&mut player, Message::TitleDragMoved(70.0));
    assert_eq!(player.marquee_offset, 3);
    let _ = update(&mut player, Message::TitleDragReleased);
    assert!(player.needs_marquee());
    assert_eq!(player.marquee_offset, 3);
    let _ = update(&mut player, Message::TitleDragMoved(10.0));
    assert_eq!(
        player.marquee_offset, 3,
        "a move without a press changes nothing"
    );
}

#[test]
fn dragging_the_title_right_wraps_backward_and_a_second_drag_starts_from_there() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "A Title Too Long For The LCD Bar");
    let cycle = {
        let current = state.blocking_lock().current_track.clone();
        let current = current.as_deref();
        views::marquee_cycle_len(
            &player.now_playing_label(current),
            &player.now_playing_artist(current),
            &player.now_playing_number(current),
        )
    };
    assert!(cycle > 3);

    let _ = update(&mut player, Message::TitleDragPressed);
    let _ = update(&mut player, Message::TitleDragMoved(100.0));
    let _ = update(&mut player, Message::TitleDragMoved(130.0));
    assert_eq!(
        player.marquee_offset,
        cycle - 3,
        "dragging right from the start wraps to the end of the cycle"
    );
    let _ = update(&mut player, Message::TitleDragReleased);

    let _ = update(&mut player, Message::TitleDragPressed);
    let _ = update(&mut player, Message::TitleDragMoved(50.0));
    assert_eq!(
        player.marquee_offset,
        cycle - 3,
        "a new press anchors afresh instead of jumping"
    );
    let _ = update(&mut player, Message::TitleDragMoved(20.0));
    assert_eq!(
        player.marquee_offset, 0,
        "dragging left three characters returns to the start of the cycle"
    );
}

#[test]
fn a_new_selection_clears_a_title_drag() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "A Title Too Long For The LCD Bar");
    let _ = update(&mut player, Message::TitleDragPressed);
    assert!(player.title_drag.is_some());
    let _ = update(
        &mut player,
        Message::SongsLoaded(crate::test_support::stepping_songs()),
    );
    let epoch = player.songs.epoch;
    let _ = update(&mut player, Message::TrackSelected { epoch, index: 0 });
    assert!(player.title_drag.is_none());
}
