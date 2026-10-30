use super::*;

// The repeat, shuffle, and equalizer toggle arms share one shape: read the
// flag, update, read it again. `assert_toggles_shared_state` owns that shape,
// so each test only names its message and the flag it flips. All three use
// `blocking_lock`, which panics inside an async runtime, so they stay plain
// tests (no `#[tokio::test]`). The Play/Pause arm is asynchronous now (it
// drives the audio backend), so it has its own `#[tokio::test]` below.
#[tokio::test]
async fn play_pause_toggles_is_playing() {
    // Load a preview so the toggle has audio to pause and resume.
    let (mut player, state) = player_playing_unrecorded_preview().await;
    assert!(state.lock().await.is_playing);

    let task = update(&mut player, Message::PlayPause);
    assert_transport_settled(task, "pause toggle").await;
    assert!(!state.lock().await.is_playing);

    let task = update(&mut player, Message::PlayPause);
    assert_transport_settled(task, "resume toggle").await;
    assert!(state.lock().await.is_playing);
}

#[test]
fn toggle_repeat_flips_shared_state() {
    assert_toggles_shared_state(Message::ToggleRepeat, |state| state.repeat);
}

#[test]
fn shuffle_toggle_flips_the_shuffle_flag() {
    assert_toggles_shared_state(Message::ToggleShuffle, |state| state.shuffle);
}

// `Message::Stop` drives the audio backend through a `Task`, so it needs the
// async runtime and `drive_task`.
#[tokio::test]
async fn stop_clears_is_playing() {
    let (mut player, state) = player_playing_unrecorded_preview().await;
    assert!(state.lock().await.is_playing);

    let task = update(&mut player, Message::Stop);
    assert_transport_settled(task, "stop").await;
    assert!(!state.lock().await.is_playing);
}

#[tokio::test]
async fn play_and_pause_set_the_playback_flag_explicitly() {
    let (mut player, state) = player_playing_unrecorded_preview().await;

    // X restarts a song that is already playing, unlike the Play/Pause toggle.
    let task = update(&mut player, Message::Play);
    assert_transport_settled(task, "play").await;
    assert!(state.lock().await.is_playing);
    let task = update(&mut player, Message::Play);
    assert_transport_settled(task, "play again").await;
    assert!(state.lock().await.is_playing);

    // C pauses, unpauses a paused song, and pauses again.
    let task = update(&mut player, Message::Pause);
    assert_transport_settled(task, "pause").await;
    assert!(!state.lock().await.is_playing);
    let task = update(&mut player, Message::Pause);
    assert_transport_settled(task, "pause again").await;
    assert!(state.lock().await.is_playing);
    let task = update(&mut player, Message::Pause);
    assert_transport_settled(task, "pause a third time").await;
    assert!(!state.lock().await.is_playing);
}

// The four transport arms must reach the injected backend, not just flip the
// shared flag. Each message is driven over its own recording backend and the
// call it records is pinned: Play resumes a paused preview, Pause pauses, Stop
// stops, and PlayPause pauses while playing. A regression that left an arm on
// the synchronous `AppState` mutator would flip the flag but record no call.
#[tokio::test]
async fn transport_messages_drive_the_audio_backend() {
    // Play (the X key): resume a paused preview.
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_preview(Arc::clone(&recording)).await;
    player.apple_music_service.pause().await.unwrap();
    assert_transport_settled(update(&mut player, Message::Play), "play").await;
    assert_preview_calls(&recording.calls(), &[AudioCall::Pause, AudioCall::Resume]);
    assert!(state.lock().await.is_playing);

    // Play while playing: restarts the preview from zero.
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_preview(Arc::clone(&recording)).await;
    assert_transport_settled(update(&mut player, Message::Play), "restart").await;
    assert_preview_calls(&recording.calls(), &[AudioCall::SeekTo(Duration::ZERO)]);
    assert!(state.lock().await.is_playing);

    // Pause (the C key).
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_preview(Arc::clone(&recording)).await;
    assert_transport_settled(update(&mut player, Message::Pause), "pause").await;
    assert_preview_calls(&recording.calls(), &[AudioCall::Pause]);
    assert!(!state.lock().await.is_playing);

    // Stop.
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_preview(Arc::clone(&recording)).await;
    assert_transport_settled(update(&mut player, Message::Stop), "stop").await;
    assert_preview_calls(&recording.calls(), &[AudioCall::Stop]);
    assert!(!state.lock().await.is_playing);

    // PlayPause while playing: pauses.
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_preview(Arc::clone(&recording)).await;
    assert_transport_settled(update(&mut player, Message::PlayPause), "toggle").await;
    assert_preview_calls(&recording.calls(), &[AudioCall::Pause]);
    assert!(!state.lock().await.is_playing);
}

// Every transport arm shares `transport_into`, whose error branch logs the
// backend's rejection and then still emits `TransportSettled` so the view
// re-renders. No other UI test reaches that branch: the recording fake
// succeeds at every command. The service calls the backend before it writes
// shared state, so a rejected command must also leave the playing flag alone.
// `attempts` proves each arm reached a failing backend rather than skipping
// it, which a settled-and-unchanged assertion alone could not tell apart.
#[tokio::test]
async fn rejected_transport_commands_still_settle_and_keep_playing_state() {
    let (audio, mut player, state) = player_with_failing_audio(FailingAudio::failing_transport());
    player
        .apple_music_service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap();
    assert!(state.lock().await.is_playing);

    for (message, what) in [
        (Message::Pause, "rejected pause"),
        (Message::Stop, "rejected stop"),
        (Message::Play, "play restarts instead of resuming"),
        (Message::PlayPause, "rejected toggle"),
    ] {
        assert_transport_settled(update(&mut player, message), what).await;
        assert!(
            state.lock().await.is_playing,
            "{what} must leave the playing flag set"
        );
    }

    // Play on a playing preview restarts it with a seek, not a transport command.
    assert_eq!(audio.attempts(FailingCommand::Transport), 3);
}

// The UI must hand the selected song's preview URL to the service, so the
// audio backend plays the right asset. This drives `TrackSelected` through
// `update` and the returned play task to completion, then asserts the
// recording backend saw exactly that URL — the end-to-end wiring the one-line
// `preview_url.as_deref()` handoff exists for.
#[tokio::test]
async fn track_selected_plays_the_songs_preview_url() {
    let (recording, mut player, _state) = player_with_recording();

    player.songs.items = vec![Song {
        id: "song-1".to_string(),
        title: "One".to_string(),
        album: String::new(),
        preview_url: Some(PREVIEW_URL.to_string()),
        ..sample_song()
    }];

    let task = update(&mut player, Message::TrackSelected { epoch: 0, index: 0 });
    drive_task(task, "play", |_| {}).await;

    assert_preview_calls(&recording.calls(), &[]);
}

// The left and right arrow arms must reach the injected backend: a seek that
// only changed UI state would leave the playing preview at its old position.
// This drives both arms through `update` over a recording backend and pins
// the signed five-second offsets they forwarded. The expected offsets are
// literals rather than `views::SEEK_STEP_SECS`: the README promises the
// arrows seek five seconds, and an assertion written in terms of the same
// constant the arms read would still pass if that documented step changed.
#[test]
fn seek_messages_drive_the_audio_backend() {
    let (recording, mut player, state) = player_with_recording();

    assert_message_schedules_no_work(&mut player, Message::SeekBackward);
    assert_message_schedules_no_work(&mut player, Message::SeekForward);

    assert_eq!(
        recording.calls(),
        vec![AudioCall::SeekBy(-5), AudioCall::SeekBy(5)]
    );
    assert_eq!(*state.blocking_lock(), AppState::default());
}

// The options menu's Back 5 seconds and Fwd 5 seconds rows must reuse the
// arrow arms exactly: choosing either by mouse or by Enter closes the menu,
// schedules no work, and forwards the same signed five-second offset to the
// backend, pinned as a literal like `seek_messages_drive_the_audio_backend`
// above. Keyboard activation reaches the choice through the row's position
// in `OptionsRow::ALL` (`OptionsRow` has no `PartialEq`), so both routes are
// driven.
#[test]
fn options_back_and_fwd_seconds_rows_seek_like_the_arrows() {
    for (choice, expected) in [
        (OptionsChoice::SeekBackward, -5),
        (OptionsChoice::SeekForward, 5),
    ] {
        let (recording, mut player, state) = player_with_recording();

        // Mouse: the row's own message.
        player.options_visible = true;
        assert_message_schedules_no_work(&mut player, Message::OptionsChoice(choice));
        assert!(!player.options_visible, "{choice:?} must close the menu");

        // Keyboard: the highlighted row activated with Enter.
        player.options_visible = true;
        player.options_selected = OptionsRow::ALL
            .iter()
            .position(|row| {
                matches!(
                    (row, choice),
                    (OptionsRow::SeekBackward, OptionsChoice::SeekBackward)
                        | (OptionsRow::SeekForward, OptionsChoice::SeekForward)
                )
            })
            .unwrap();
        assert_message_schedules_no_work(&mut player, Message::OptionsActivate);
        assert!(!player.options_visible, "{choice:?} must close the menu");

        assert_eq!(
            recording.calls(),
            vec![AudioCall::SeekBy(expected), AudioCall::SeekBy(expected)]
        );
        assert_eq!(*state.blocking_lock(), AppState::default());
    }
}

// A rejection from the seek backend must only be logged: the seek arm still
// schedules no work and leaves shared state alone. The fake counts its
// rejected calls, proving the message reached the failing backend instead of
// skipping the forward.
#[test]
fn a_seek_backend_failure_still_schedules_no_work() {
    let (audio, mut player, state) = player_with_failing_audio(FailingAudio::failing_seek());

    assert_message_schedules_no_work(&mut player, Message::SeekForward);

    assert_eq!(audio.attempts(FailingCommand::Seek), 1);
    assert_eq!(*state.blocking_lock(), AppState::default());
}

// `Message::TrackSelected` wraps playback in a `Task`; the arm itself only
// schedules it, so the real behavior lives in the returned task. Drive that
// task to completion (as the iced runtime would) and assert the shared
// state it mutates through the service.
#[tokio::test]
async fn track_selected_starts_playback_of_the_selected_track() {
    let (mut player, state) = test_player();
    player.songs.items = vec![sample_song()];

    let task = update(&mut player, Message::TrackSelected { epoch: 0, index: 0 });
    drive_task(task, "playback", |message| match message {
        Message::TrackPlayed { committed, .. } => assert!(committed),
        other => panic!("unexpected playback task output: {other:?}"),
    })
    .await;

    let state = state.lock().await;
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(state.is_playing);
}

// At launch with a restored last-played track — the state carries its id but
// the service has neither a loaded preview nor a remembered URL — Play
// resolves the saved id through the service seam and replays it through the
// row-press machinery, exactly as a press on its row would: the state marks
// it playing and the Now Playing index learns its title. A relaunch where
// Play silently did nothing left the promise in docs/usage.md false.
// The replayed song's genre matches no AUTO preset, so the row-press body
// the reply drives stays lock-free enough for the async runtime — `song-1`'s
// Rock would take `apply_auto_preset`'s shared-state read, and identical
// behavior is already covered by the plain threading tests above.
#[tokio::test]
async fn play_replays_the_track_restored_at_boot() {
    let (recording, mut player, state) = player_with_recording();
    // The boot path restored this id; nothing has played since.
    state.lock().await.current_track = Some("song-4".to_string());
    assert!(!state.lock().await.is_playing);

    let resolve_task = update(&mut player, Message::Play);
    assert!(
        !state.lock().await.is_playing,
        "the resolve alone must not start playback"
    );

    // The resolve replies with the saved id's library song; feed the reply
    // through update the way the runtime would.
    let mut resolved = None;
    drive_task(resolve_task, "boot replay resolve", |message| {
        resolved = Some(message)
    })
    .await;
    let Some(Message::ReplayResolved { track_id, song }) = resolved else {
        panic!("the replay resolve did not reply with ReplayResolved");
    };
    assert_eq!(track_id, "song-4");
    assert!(song.is_some(), "the sample library must know the saved id");

    let task = update(&mut player, Message::ReplayResolved { track_id, song });
    drive_task(task, "boot replay play", |message| match message {
        Message::TrackPlayed { committed, .. } => assert!(committed),
        other => panic!("unexpected replay task output: {other:?}"),
    })
    .await;

    let state = state.lock().await;
    assert_eq!(state.current_track.as_deref(), Some("song-4"));
    assert!(state.is_playing, "the restored track must be playing");
    assert_eq!(
        player
            .known_tracks
            .get("song-4")
            .map(|known| known.title.as_str()),
        Some("B-side"),
        "the replay must record the title the row press would have"
    );
    // The asset-less sample song commits without a preview, exactly as a
    // press on its row does: the service stops the (empty) previous player
    // and starts nothing.
    assert_eq!(recording.calls(), &[AudioCall::Stop]);
}

// The media-key Play/Pause reaches the same replay from its stopped branch,
// so the keyboard alone starts a relaunched session's restored track.
#[tokio::test]
async fn play_pause_toggles_start_of_a_replay_from_stopped() {
    let (recording, mut player, state) = player_with_recording();
    // `song-5`'s genre matches no AUTO preset, like the test above.
    state.lock().await.current_track = Some("song-5".to_string());

    let resolve_task = update(&mut player, Message::PlayPause);
    let mut resolved = None;
    drive_task(resolve_task, "boot replay resolve", |message| {
        resolved = Some(message)
    })
    .await;
    let Some(Message::ReplayResolved { track_id, song }) = resolved else {
        panic!("the replay resolve did not reply with ReplayResolved");
    };
    assert_eq!(track_id, "song-5");

    let task = update(&mut player, Message::ReplayResolved { track_id, song });
    drive_task(task, "boot replay play", |message| match message {
        Message::TrackPlayed { committed, .. } => assert!(committed),
        other => panic!("unexpected replay task output: {other:?}"),
    })
    .await;

    let state = state.lock().await;
    assert_eq!(state.current_track.as_deref(), Some("song-5"));
    assert!(state.is_playing);
    assert_eq!(recording.calls(), &[AudioCall::Stop]);
}

// A resolve reply the boot path no longer owns is dropped: the restored id
// must still be the shared state's current track, and nothing may be playing
// or loaded when it lands, so a track the user picked while the resolve was
// in flight cannot be knocked out by the replay.
#[tokio::test]
async fn a_replay_reply_a_newer_selection_superseded_is_dropped() {
    let (_, mut player, state) = player_with_recording();
    state.lock().await.current_track = Some("song-1".to_string());

    let task = update(
        &mut player,
        Message::ReplayResolved {
            track_id: "song-2".to_string(),
            song: Some(sample_song()),
        },
    );
    assert_no_task(task);

    assert!(!state.lock().await.is_playing);
}

// A restored id that names no song in the library (a stale id from a
// deleted track) leaves the transport stopped: the reply carries no song and
// no remembered URL covers it, so there is nothing to replay.
#[tokio::test]
async fn a_replay_of_a_stale_unresolvable_id_leaves_the_transport_stopped() {
    let (_, mut player, state) = player_with_recording();
    state.lock().await.current_track = Some("gone-track".to_string());

    let resolve_task = update(&mut player, Message::Play);
    let mut resolved = None;
    drive_task(resolve_task, "stale replay resolve", |message| {
        resolved = Some(message)
    })
    .await;
    let Some(Message::ReplayResolved { track_id, song }) = resolved else {
        panic!("the replay resolve did not reply with ReplayResolved");
    };
    assert_eq!(track_id, "gone-track");
    assert!(song.is_none(), "a stale id must not resolve");

    let task = update(&mut player, Message::ReplayResolved { track_id, song });
    assert_no_task(task);

    let state = state.lock().await;
    assert!(!state.is_playing);
    assert_eq!(state.current_track.as_deref(), Some("gone-track"));
}
