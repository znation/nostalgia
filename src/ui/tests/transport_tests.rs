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
    let (mut player, state) = player_playing_preview(Arc::new(RecordingAudio::default())).await;
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
    let (mut player, state) = player_playing_preview(Arc::new(RecordingAudio::default())).await;
    assert!(state.lock().await.is_playing);

    let task = update(&mut player, Message::Stop);
    assert_transport_settled(task, "stop").await;
    assert!(!state.lock().await.is_playing);
}

#[tokio::test]
async fn play_and_pause_set_the_playback_flag_explicitly() {
    let (mut player, state) = player_playing_preview(Arc::new(RecordingAudio::default())).await;

    // X plays even when already playing, unlike the Play/Pause toggle.
    let task = update(&mut player, Message::Play);
    assert_transport_settled(task, "play").await;
    assert!(state.lock().await.is_playing);
    let task = update(&mut player, Message::Play);
    assert_transport_settled(task, "play again").await;
    assert!(state.lock().await.is_playing);

    // C pauses even when already paused.
    let task = update(&mut player, Message::Pause);
    assert_transport_settled(task, "pause").await;
    assert!(!state.lock().await.is_playing);
    let task = update(&mut player, Message::Pause);
    assert_transport_settled(task, "pause again").await;
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
    let audio = Arc::new(FailingAudio::failing_transport());
    let (mut player, state) =
        player_with_audio(Arc::clone(&audio) as Arc<dyn crate::audio::AudioOutput>);
    player
        .apple_music_service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap();
    assert!(state.lock().await.is_playing);

    for (message, what) in [
        (Message::Pause, "rejected pause"),
        (Message::Stop, "rejected stop"),
        (Message::Play, "rejected resume"),
        (Message::PlayPause, "rejected toggle"),
    ] {
        assert_transport_settled(update(&mut player, message), what).await;
        assert!(
            state.lock().await.is_playing,
            "{what} must leave the playing flag set"
        );
    }

    assert_eq!(audio.attempts(FailingCommand::Transport), 4);
}

// The UI must hand the selected song's preview URL to the service, so the
// audio backend plays the right asset. This drives `TrackSelected` through
// `update` and the returned play task to completion, then asserts the
// recording backend saw exactly that URL — the end-to-end wiring the one-line
// `preview_url.as_deref()` handoff exists for.
#[tokio::test]
async fn track_selected_plays_the_songs_preview_url() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);

    player.songs.items = vec![Song {
        id: "song-1".to_string(),
        title: "One".to_string(),
        artist: "The Sample Band".to_string(),
        album: String::new(),
        album_id: "album-1".to_string(),
        duration_ms: 210_000,
        preview_url: Some(PREVIEW_URL.to_string()),
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
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);

    assert_message_schedules_no_work(&mut player, Message::SeekBackward);
    assert_message_schedules_no_work(&mut player, Message::SeekForward);

    assert_eq!(
        recording.calls(),
        vec![AudioCall::SeekBy(-5), AudioCall::SeekBy(5)]
    );
    assert_eq!(*state.blocking_lock(), AppState::default());
}

// A rejection from the seek backend must only be logged: the seek arm still
// schedules no work and leaves shared state alone. The fake counts its
// rejected calls, proving the message reached the failing backend instead of
// skipping the forward.
#[test]
fn a_seek_backend_failure_still_schedules_no_work() {
    let audio = Arc::new(FailingAudio::failing_seek());
    let (mut player, state) =
        player_with_audio(Arc::clone(&audio) as Arc<dyn crate::audio::AudioOutput>);

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
