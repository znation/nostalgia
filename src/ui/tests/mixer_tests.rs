use super::*;

#[test]
fn toggle_equalizer_flips_shared_state() {
    assert_toggles_shared_state(Message::ToggleEqualizer, |state| state.eq_enabled);
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
    assert_eq!(state.eq_preamp(), preset.preamp);
    assert_eq!(state.eq_bands(), preset.bands);
    assert_eq!(state.eq_preset(), Some(preset));
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
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);

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
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);

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
    let audio = Arc::new(FailingAudio::failing_volume_and_balance());
    let (mut player, state) =
        player_with_audio(Arc::clone(&audio) as Arc<dyn crate::audio::AudioOutput>);

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
    let audio = Arc::new(FailingAudio::failing_volume_and_balance());
    let (mut player, state) =
        player_with_audio(Arc::clone(&audio) as Arc<dyn crate::audio::AudioOutput>);

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
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);

    fn snapshot(state: &AppState) -> crate::audio::EqSnapshot {
        crate::audio::EqSnapshot {
            enabled: state.eq_enabled,
            preamp_db: state.eq_preamp(),
            bands_db: state.eq_bands(),
        }
    }

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
    let audio = Arc::new(FailingAudio::failing_equalizer());
    let (mut player, state) =
        player_with_audio(Arc::clone(&audio) as Arc<dyn crate::audio::AudioOutput>);

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
