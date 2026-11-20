use super::*;

/// Fires one `PositionTick` and returns the epoch it was issued at with every
/// reply its batched queries emit.
fn tick_replies(player: &mut WinampPlayer) -> (u64, Vec<Message>) {
    let epoch = player.position_epoch;
    let replies = task_outputs(update(player, Message::PositionTick), "position tick");
    (epoch, replies)
}

/// Asserts the tick's replies include a position query at `epoch`.
fn assert_position_queried(replies: &[Message], epoch: u64) {
    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackPosition { epoch: reply_epoch, .. } if *reply_epoch == epoch
        )),
        "the tick must query the position: {replies:?}"
    );
}

#[test]
fn position_tick_schedules_a_query() {
    let (mut player, _state) = test_player();
    assert_message_schedules_work(&mut player, Message::PositionTick);
}

// The subscription itself must emit the message its `update` arm expects: a
// stream that never ticks (or yields a different message) would leave the
// readout frozen even though `PositionTick` schedules the query correctly.
// Reading two ticks also pins that the timer keeps running after the first.
// The tick must ask the backend for both values: one query for the elapsed
// position and one for the status row's format. Yield every output the
// batched task emits and check both replies arrive.
#[test]
fn position_tick_queries_position_and_format() {
    let (mut player, _state) = test_player();
    let (epoch, replies) = tick_replies(&mut player);

    assert_position_queried(&replies, epoch);
    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackFormat { epoch: reply_epoch, .. } if *reply_epoch == epoch
        )),
        "the tick must query the format: {replies:?}"
    );
}

// The tick must also ask the backend for the preview's own length, so the
// seek bar has a scale. Drive the batched task's stream to its end and check
// the duration reply arrives beside the position.
#[test]
fn position_tick_queries_position_and_duration() {
    let (_recording, mut player, _state) = player_with_recording();
    let (epoch, replies) = tick_replies(&mut player);

    assert_position_queried(&replies, epoch);
    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackDuration { epoch: reply_epoch, duration: Some(duration) }
                if *reply_epoch == epoch && *duration == Duration::from_secs(30)
        )),
        "the tick must query the duration: {replies:?}"
    );
}

// The tick must also ask the backend whether the preview has reached its end,
// so the transport can advance off the existing one-second poll instead of a
// new timer. Drive the batched task's stream to its end and check the finished
// reply carries the tick's epoch and the backend's state.
#[test]
fn position_tick_queries_preview_finished() {
    let (recording, mut player, _state) = player_with_recording();
    for finished in [true, false] {
        recording.set_finished(finished);
        let (epoch, replies) = tick_replies(&mut player);

        assert!(
            replies.iter().any(|message| matches!(
                message,
                Message::PlaybackFinished { epoch: reply_epoch, finished: reply_finished }
                    if *reply_epoch == epoch && *reply_finished == finished
            )),
            "the tick must query the finished state {finished}: {replies:?}"
        );
    }
}

// The paused elapsed readout blinks on the existing position poll: each tick
// flips the phase while the transport is paused, and a tick after Play or Stop
// restores it. A stopped tick still holds the flag true, but the view draws the
// stopped readout transparent regardless of the phase except for the `--:--`
// placeholder. A loaded preview is the paused state Play can resume.
#[test]
fn position_tick_blinks_the_elapsed_readout_only_while_paused() {
    let (mut player, state) = player_with_audio(Arc::new(RecordingAudio::default()));
    futures::executor::block_on(player.apple_music_service.play_track(
        "song-1",
        Some(PREVIEW_URL),
        || true,
    ))
    .unwrap();
    state.blocking_lock().pause();
    assert_eq!(
        state.blocking_lock().transport_mode(),
        TransportMode::Paused
    );
    assert!(player.blink_visible);

    let _ = update(&mut player, Message::PositionTick);
    assert!(
        !player.blink_visible,
        "the first paused tick hides the readout"
    );
    let _ = update(&mut player, Message::PositionTick);
    assert!(
        player.blink_visible,
        "the second paused tick shows it again"
    );

    state.blocking_lock().play();
    let _ = update(&mut player, Message::PositionTick);
    assert!(
        player.blink_visible,
        "a playing tick always shows the readout"
    );

    state.blocking_lock().pause();
    let _ = update(&mut player, Message::PositionTick);
    assert!(!player.blink_visible, "a paused tick resumes the blink");
    state.blocking_lock().stop();
    let _ = update(&mut player, Message::PositionTick);
    assert!(
        player.blink_visible,
        "a stopped tick holds the blink phase visible, though the view clears a real readout"
    );
}

#[test]
fn position_tick_skips_the_format_and_length_once_both_are_known() {
    let (_recording, mut player, _state) = player_with_recording();
    player.playback_format = Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    });
    player.playback_duration = Some(Duration::from_secs(30));
    let (epoch, replies) = tick_replies(&mut player);

    assert_position_queried(&replies, epoch);
    assert!(
        !replies.iter().any(|message| matches!(
            message,
            Message::PlaybackFormat { .. } | Message::PlaybackDuration { .. }
        )),
        "a steady preview's format and length cannot change, so the tick must not query them: {replies:?}"
    );
}

// The two optional values are polled independently, so a preview whose source
// cannot supply one of them still polls for that one without re-querying the
// known one.
#[test]
fn position_tick_queries_only_the_missing_value() {
    let (_recording, mut player, _state) = player_with_recording();
    player.playback_format = Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    });
    let (epoch, replies) = tick_replies(&mut player);
    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackDuration { epoch: reply_epoch, duration: Some(_) }
                if *reply_epoch == epoch
        )),
        "an unknown length must still be queried: {replies:?}"
    );
    assert!(
        !replies
            .iter()
            .any(|message| matches!(message, Message::PlaybackFormat { .. })),
        "a known format must not be queried again: {replies:?}"
    );

    player.playback_format = None;
    player.playback_duration = Some(Duration::from_secs(30));
    let (epoch, replies) = tick_replies(&mut player);
    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackFormat { epoch: reply_epoch, .. } if *reply_epoch == epoch
        )),
        "an unknown format must still be queried: {replies:?}"
    );
    assert!(
        !replies
            .iter()
            .any(|message| matches!(message, Message::PlaybackDuration { .. })),
        "a known length must not be queried again: {replies:?}"
    );
}
