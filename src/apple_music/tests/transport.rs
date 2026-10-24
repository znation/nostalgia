use super::*;

// `pause` pauses the injected backend as well as clearing the shared playing
// flag and setting the paused mode; the test pins both halves of that
// contract.
#[tokio::test]
async fn pause_pauses_the_audio_backend() {
    let (service, state, recording) = recording_service();

    service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap();
    service.pause().await.unwrap();

    assert_preview_calls(&recording.calls(), &[AudioCall::Pause]);
    assert_playback_state(&state, Some("song-1"), false).await;
}

#[tokio::test]
async fn pause_stops_playing_but_keeps_current_track() {
    let (service, state) = service_with_song_1_playing().await;

    service.pause().await.unwrap();

    assert_playback_state(&state, Some("song-1"), false).await;
    assert_transport_mode(&state, TransportMode::Paused).await;
}

// Pause forwards to the backend even when no preview is loaded, but the
// transport must not claim Paused: the audio worker ignores Pause with no
// player, and Play would have nothing to resume, leaving a Paused LCD state
// that only Stop could leave. A current track whose source supplied no preview
// is marked playing but has no backend player either, so pausing it stops the
// transport too.
#[tokio::test]
async fn pause_without_a_loaded_preview_leaves_the_transport_stopped() {
    let (service, state) = test_service_with_state();

    service.pause().await.unwrap();

    assert_playback_state(&state, None, false).await;
    assert_transport_mode(&state, TransportMode::Stopped).await;

    service.play_track("song-1", None, || true).await.unwrap();
    assert_transport_mode(&state, TransportMode::Playing).await;

    service.pause().await.unwrap();

    assert_playback_state(&state, Some("song-1"), false).await;
    assert_transport_mode(&state, TransportMode::Stopped).await;
}

// The Play/Pause toggle's pause half has the same no-preview case: a track
// that cannot sound is stopped rather than left claiming a paused preview.
#[tokio::test]
async fn toggle_without_a_loaded_preview_stops_the_transport() {
    let (service, state) = test_service_with_state();

    service.play_track("song-1", None, || true).await.unwrap();
    service.toggle_play_pause().await.unwrap();

    assert_playback_state(&state, Some("song-1"), false).await;
    assert_transport_mode(&state, TransportMode::Stopped).await;
}

// Stop discards the backend's player, so a Pause after it has nothing to hold
// paused. The track stays named, but the transport remains stopped rather than
// claiming a paused preview Play could not resume.
#[tokio::test]
async fn pause_after_stop_leaves_the_transport_stopped() {
    let (service, state, _) = recording_service_with_preview().await;

    service.stop().await.unwrap();
    service.pause().await.unwrap();

    assert_playback_state(&state, Some("song-1"), false).await;
    assert_transport_mode(&state, TransportMode::Stopped).await;
}

// `resume` drives the injected backend, sets the shared playing flag, and
// clears the paused flag, the other half of `pause`. A `Some` preview is
// loaded first so the backend has a paused player to resume (the service only
// resumes a loaded preview, or restarts a stopped one).
#[tokio::test]
async fn resume_resumes_the_audio_backend() {
    let (service, state, recording) = recording_service_with_preview().await;

    service.pause().await.unwrap();
    service.resume().await.unwrap();

    assert_preview_calls(&recording.calls(), &[AudioCall::Pause, AudioCall::Resume]);
    assert_playback_state(&state, Some("song-1"), true).await;
    assert_transport_mode(&state, TransportMode::Playing).await;
}

// `stop` drives the injected backend and clears the shared playing and paused
// flags while keeping `current_track`, exactly as `AppState::stop` does.
#[tokio::test]
async fn stop_stops_the_audio_backend() {
    let (service, state, recording) = recording_service_with_preview().await;

    service.stop().await.unwrap();

    assert_preview_calls(&recording.calls(), &[AudioCall::Stop]);
    assert_playback_state(&state, Some("song-1"), false).await;
    assert_transport_mode(&state, TransportMode::Stopped).await;
}

// `pause` documents a backend-first order: a backend that cannot accept the
// pause reports it before shared state claims playback stopped. A regression
// that cleared the shared flag first would leave the Now Playing bar reading
// "paused" while the preview kept sounding, and only a failing backend can
// catch it.
#[tokio::test]
async fn pause_reports_a_backend_failure_without_clearing_playing() {
    let (service, state) =
        service_with_preview_playing(Arc::new(RejectingAudio::new(RejectedCommand::Pause))).await;

    let error = service.pause().await.unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-1"), true).await;
}

// `stop`'s backend-first order, the twin of the pause contract above: a
// rejected stop must not clear the playing flag, or the UI would report a
// stopped transport while the preview still sounds.
#[tokio::test]
async fn stop_reports_a_backend_failure_without_clearing_playing() {
    let (service, state) =
        service_with_preview_playing(Arc::new(RejectingAudio::new(RejectedCommand::Stop))).await;

    let error = service.stop().await.unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-1"), true).await;
}

// `resume_locked` calls the backend before it sets the playing flag, so a
// backend that rejects the resume leaves the transport paused rather than
// claiming a preview that never restarted. The starting point is a loaded but
// paused preview: play it, then pause it successfully.
#[tokio::test]
async fn resume_reports_a_backend_failure_without_claiming_playback() {
    let (service, state) =
        service_with_preview_playing(Arc::new(RejectingAudio::new(RejectedCommand::Resume))).await;
    service.pause().await.unwrap();

    let error = service.resume().await.unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-1"), false).await;
}

// The Play/Pause toggle pauses while playing.
#[tokio::test]
async fn toggle_play_pause_pauses_while_playing() {
    let (service, state, recording) = recording_service_with_preview().await;

    service.toggle_play_pause().await.unwrap();

    assert_preview_calls(&recording.calls(), &[AudioCall::Pause]);
    assert_playback_state(&state, Some("song-1"), false).await;
    assert_transport_mode(&state, TransportMode::Paused).await;
}

// `toggle_play_pause`'s pause arm keeps the same backend-first order as
// `pause`: a backend that rejects the pause must report it before the toggle
// flips the shared playing flag, or the Now Playing bar would read "paused"
// while the preview kept sounding. `pause_reports_a_backend_failure_without_clearing_playing`
// pins the plain path; this pins the toggle's separately written arm.
#[tokio::test]
async fn toggle_play_pause_reports_a_backend_failure_without_clearing_playing() {
    let audio = Arc::new(RejectingAudio::new(RejectedCommand::Pause));
    let (service, state) =
        service_with_preview_playing(Arc::clone(&audio) as Arc<dyn AudioOutput>).await;

    let error = service.toggle_play_pause().await.unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_preview_calls(&audio.calls(), &[AudioCall::Pause]);
    assert_playback_state(&state, Some("song-1"), true).await;
}

// The Play/Pause toggle resumes while paused.
#[tokio::test]
async fn toggle_play_pause_resumes_while_paused() {
    let (service, state, recording) = recording_service_with_preview().await;

    service.pause().await.unwrap();
    service.toggle_play_pause().await.unwrap();

    assert_preview_calls(&recording.calls(), &[AudioCall::Pause, AudioCall::Resume]);
    assert_playback_state(&state, Some("song-1"), true).await;
}

// Stop discards the backend's player, so `resume` cannot resume it; instead
// the service replays the remembered preview URL, restarting it from the
// beginning rather than claiming playback with nothing loaded. Without this,
// Play after Stop would set `is_playing` while the backend held no player —
// the desync this wiring exists to prevent.
#[tokio::test]
async fn resume_after_stop_restarts_the_preview() {
    let (service, state, recording) = recording_service_with_preview().await;

    service.stop().await.unwrap();
    service.resume().await.unwrap();

    assert_preview_calls(
        &recording.calls(),
        &[
            AudioCall::Stop,
            AudioCall::SetEqualizer(EqSnapshot::default()),
            AudioCall::Play(PREVIEW_URL.to_string()),
        ],
    );
    assert_playback_state(&state, Some("song-1"), true).await;
    assert_transport_mode(&state, TransportMode::Playing).await;
}

// A restart after Stop builds a fresh player from the backend's remembered
// curve, so it must forward the curve the panel shows now even when that
// changed while the transport was stopped.
#[tokio::test]
async fn resume_after_stop_restarts_at_the_states_current_equalizer() {
    let (service, state, recording) = recording_service_with_preview().await;
    service.stop().await.unwrap();
    {
        let mut state = state.lock().await;
        state.toggle_equalizer();
        state.set_eq_preamp(3.0);
        state.set_eq_band(4, 5.0);
    }
    let mut bands_db = [0.0; crate::equalizer::BAND_COUNT];
    bands_db[4] = 5.0;
    let equalizer = EqSnapshot {
        enabled: true,
        preamp_db: 3.0,
        bands_db,
    };

    service.resume().await.unwrap();

    assert_preview_calls(
        &recording.calls(),
        &[
            AudioCall::Stop,
            AudioCall::SetEqualizer(equalizer),
            AudioCall::Play(PREVIEW_URL.to_string()),
        ],
    );
    assert_playback_state(&state, Some("song-1"), true).await;
}

// A restart after Stop is a fresh `play`, so a backend that rejects it must
// surface the error and leave the transport stopped; the service must not
// claim a loaded player or set the playing flag. A regression that marked the
// transport loaded before the backend accepted the restart would send the
// next resume as a `Resume` to a backend with no player, so the second attempt
// here must issue another `Play`.
#[tokio::test]
async fn resume_after_stop_reports_a_restart_failure_without_claiming_playback() {
    let audio = Arc::new(RejectingAudio::new(RejectedCommand::PlayAfterFirst));
    let (service, state) =
        service_with_preview_playing(Arc::clone(&audio) as Arc<dyn AudioOutput>).await;
    service.stop().await.unwrap();

    let error = service.resume().await.unwrap_err();
    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-1"), false).await;

    let error = service.resume().await.unwrap_err();
    assert_eq!(error.to_string(), "the audio device is gone");
    assert_eq!(
        audio.calls(),
        vec![
            AudioCall::SetVolume(0.5),
            AudioCall::SetBalance(0.0),
            AudioCall::SetEqualizer(EqSnapshot::default()),
            AudioCall::Play(PREVIEW_URL.to_string()),
            AudioCall::Stop,
            AudioCall::SetEqualizer(EqSnapshot::default()),
            AudioCall::Play(PREVIEW_URL.to_string()),
            AudioCall::SetEqualizer(EqSnapshot::default()),
            AudioCall::Play(PREVIEW_URL.to_string()),
        ]
    );
    assert_playback_state(&state, Some("song-1"), false).await;
}

// A restart after Stop forwards the panel's curve before it replays, so a
// backend that refuses that curve must surface the error and leave the
// transport stopped: replaying anyway would sound at the wrong curve, and
// marking the transport loaded first would send the next Play as a `Resume` to
// a backend with no player. The retry here must therefore issue another
// `SetEqualizer`, not a `Resume`.
#[tokio::test]
async fn resume_after_stop_reports_an_equalizer_failure_without_claiming_playback() {
    let audio = Arc::new(RejectingAudio::new(RejectedCommand::SetEqualizerAfterFirst));
    let (service, state) =
        service_with_preview_playing(Arc::clone(&audio) as Arc<dyn AudioOutput>).await;
    service.stop().await.unwrap();

    let error = service.resume().await.unwrap_err();
    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-1"), false).await;

    let error = service.resume().await.unwrap_err();
    assert_eq!(error.to_string(), "the audio device is gone");
    assert_eq!(
        audio.calls(),
        vec![
            AudioCall::SetVolume(0.5),
            AudioCall::SetBalance(0.0),
            AudioCall::SetEqualizer(EqSnapshot::default()),
            AudioCall::Play(PREVIEW_URL.to_string()),
            AudioCall::Stop,
            AudioCall::SetEqualizer(EqSnapshot::default()),
            AudioCall::SetEqualizer(EqSnapshot::default()),
        ]
    );
    assert_playback_state(&state, Some("song-1"), false).await;
}

// With no preview ever played there is nothing to resume or restart, so Play
// leaves the transport stopped rather than claiming playback the backend
// cannot produce.
#[tokio::test]
async fn resume_without_a_preview_leaves_the_transport_stopped() {
    let (service, state, recording) = recording_service();

    service.resume().await.unwrap();

    assert!(recording.calls().is_empty());
    assert_playback_state(&state, None, false).await;
}

// `has_loaded_preview` runs on the UI's main thread each time `update`
// re-evaluates the timer subscriptions, and a transport transition may be
// holding the transport lock then. It must answer without waiting for that
// lock: blocking there would stall the thread that has to drive the
// transition to completion. While a transition holds the lock it reports
// `true`, so a paused preview's position poll stays subscribed instead of
// being dropped on a stale read; once the lock is free the real flag answers.
// With no preview ever played, that flag is false.
#[tokio::test]
async fn has_loaded_preview_answers_true_while_a_transition_holds_the_lock() {
    let (service, _state) = test_service_with_state();

    let held = service.transport.lock().await;
    assert!(
        service.has_loaded_preview(),
        "a held transport lock must report a loaded preview without waiting"
    );
    drop(held);

    assert!(
        !service.has_loaded_preview(),
        "with the lock free and no preview played, the real flag is false"
    );
}

// Two Play/Pause toggles can arrive close together (a double-click), and each
// runs as its own task. The service holds the transport lock across the flag
// read, the backend call, and the flag write, so the second toggle observes
// the first's write instead of both reading "playing" and both pausing. This
// test holds the first toggle inside the backend's `pause`, starts the second,
// and proves it has not entered the backend: were the lock dropped before the
// backend call, the second pause would enter concurrently, the gate would fire
// twice, and the call list would end with two `Pause`s and the wrong flag.
#[test]
fn concurrent_toggles_serialize_on_the_transport_lock() {
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let audio = Arc::new(GatedAudio {
        calls: std::sync::Mutex::new(Vec::new()),
        entered: entered_tx,
        release: std::sync::Mutex::new(release_rx),
    });
    let (service, state) = service_with_audio(Arc::clone(&audio) as Arc<dyn AudioOutput>);
    futures::executor::block_on(service.play_track("song-1", Some(PREVIEW_URL), || true)).unwrap();

    let first = service.clone();
    let first = std::thread::spawn(move || {
        futures::executor::block_on(first.toggle_play_pause()).unwrap();
    });
    entered_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the first toggle must reach the backend");

    let second = service.clone();
    let second = std::thread::spawn(move || {
        futures::executor::block_on(second.toggle_play_pause()).unwrap();
    });

    // A second entry while the first is held means the toggles overlapped in
    // the backend. `recv_timeout` returning `Ok` is that failure.
    assert!(
        entered_rx
            .recv_timeout(std::time::Duration::from_millis(200))
            .is_err(),
        "a second toggle entered the backend while the first was still in it"
    );

    release_tx.send(()).unwrap();
    first.join().unwrap();
    second.join().unwrap();

    assert_preview_calls(&audio.calls(), &[AudioCall::Pause, AudioCall::Resume]);
    assert!(futures::executor::block_on(state.lock()).is_playing);
}

// The `pause` test (`pause_pauses_the_audio_backend`) pins its state change;
// its two sibling stubs, `next_track` and `previous_track`, have no production
// caller at all.
// They are deliberately unimplemented no-ops — the transport buttons step
// through `transport::next_track_id`/`previous_track_id` instead — so pin
// the one contract they do carry: they report success and, unlike
// `pause`, leave the shared playback state untouched. A regression that
// wired either stub into `AppState` (or made it fail) would otherwise
// ship silently, since no caller reaches them today.
#[tokio::test]
async fn next_and_previous_track_stubs_succeed_without_touching_state() {
    let (service, state) = service_with_song_1_playing().await;

    service.next_track().await.unwrap();
    service.previous_track().await.unwrap();

    assert_playback_state(&state, Some("song-1"), true).await;
}
