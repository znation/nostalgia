use super::*;

#[tokio::test]
async fn play_track_sets_current_track_and_starts_playing() {
    let (service, state) = test_service_with_state();

    service.play_track("song-1", None, || true).await.unwrap();

    assert_playback_state(&state, Some("song-1"), true).await;
}

// The service hands a track's preview URL to the injected audio backend and
// commits the shared-state transition. The recording fake pins the exact URL,
// so a regression that dropped it (or reordered the arguments) fails here.
#[tokio::test]
async fn play_track_starts_the_preview_through_the_audio_backend() {
    let (service, state, recording) = recording_service();

    service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap();

    assert_preview_calls(&recording.calls(), &[]);
    assert_playback_state(&state, Some("song-1"), true).await;
}

// A song the library supplied without a preview URL still commits the state
// transition — the Now Playing bar must name it — but starts no new audio. It
// stops the previous preview, so the old track neither keeps sounding nor
// keeps answering the position query under the new track's name. The recording
// fake pins the exact sequence: the earlier preview, then `Stop`.
#[tokio::test]
async fn play_track_without_a_preview_url_stops_the_previous_preview() {
    let (service, state, recording) = recording_service_with_preview().await;

    service.play_track("song-2", None, || true).await.unwrap();

    assert_preview_calls(&recording.calls(), &[AudioCall::Stop]);
    assert_playback_state(&state, Some("song-2"), true).await;
}

// A `None` preview replaces the previous one by stopping the backend, and a
// backend that rejects that stop must surface the error rather than claim the
// new selection is playing: the old preview may still be sounding, so the
// service clears the playing and paused flags and forgets the remembered URL
// so a later Play cannot restart it under the new track's name.
// `current_track` stays committed as the rollback leaves it for a rejected
// `Some` preview, so the bar still names the track the user picked.
#[tokio::test]
async fn play_track_without_a_preview_url_reports_a_stop_failure_without_claiming_playback() {
    let audio = Arc::new(RejectingAudio::new(RejectedCommand::Stop));
    let (service, state) =
        service_with_preview_playing(Arc::clone(&audio) as Arc<dyn AudioOutput>).await;

    let error = service
        .play_track("song-2", None, || true)
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-2"), false).await;

    // The URL was forgotten before the stop, so Play cannot resurrect the
    // preview the failed stop left sounding.
    service.resume().await.unwrap();
    assert_eq!(
        audio.calls(),
        vec![
            AudioCall::SetVolume(0.5),
            AudioCall::SetBalance(0.0),
            AudioCall::SetEqualizer(EqSnapshot::default()),
            AudioCall::Play(PREVIEW_URL.to_string()),
            AudioCall::Stop,
        ]
    );
    assert_playback_state(&state, Some("song-2"), false).await;
}

// A `Some` preview URL that is blank can play nothing, so the seam rejects it
// before committing state — the same guard a blank track id gets. The REST
// layer never produces one (`Resource::into_song` maps a blank preview to
// `None`), so this pins the public seam's contract for a caller that hands it
// one directly. Both an empty and a whitespace-only URL are blank, and the
// error quotes the value with `{url:?}` so the whitespace case is visible.
// State must survive untouched, so the already-playing song keeps playing.
#[tokio::test]
async fn play_track_rejects_a_blank_preview_url_without_touching_state() {
    let (service, state) = service_with_song_1_playing().await;

    let error = service
        .play_track("song-2", Some(""), || true)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "preview URL must not be blank (got \"\")"
    );

    let error = service
        .play_track("song-2", Some("   "), || true)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "preview URL must not be blank (got \"   \")"
    );

    assert_playback_state(&state, Some("song-1"), true).await;
}

// A preview URL comes from the Apple Music API response and is handed to a
// network fetch by the audio backend, so the seam must reject anything that is
// not an https URL with an acceptable host: a cleartext URL, a
// loopback/private/link-local address literal (an SSRF target), a non-http
// scheme, a numeric host form that resolves to an address, and a value
// carrying a control character. Each must fail before shared state is
// committed and before the backend records a play, so a hostile or compromised
// API reply cannot reach the fetch through a cleartext, address-literal, or
// numeric-host URL.
#[tokio::test]
async fn play_track_rejects_a_preview_url_that_is_not_a_public_https_url() {
    let recording = Arc::new(RecordingAudio::default());
    let (service, state) = service_with_audio(Arc::clone(&recording) as Arc<dyn AudioOutput>);

    for url in [
        "http://127.0.0.1:8080/preview.m4a",
        "http://169.254.169.254/latest/meta-data/",
        "https://localhost/preview.m4a",
        "https://[::1]/preview.m4a",
        "https://10.0.0.5/preview.m4a",
        "https://192.168.1.1/preview.m4a",
        "https://2130706433/preview.m4a",
        "https://127.1/preview.m4a",
        "https://0x7f000001/preview.m4a",
        "https://0X7F000001/preview.m4a",
        "https://0x7f.0.0.1/preview.m4a",
        "https://0177.0.0.1/preview.m4a",
        "file:///etc/passwd",
        "https://example.test/\u{1b}[2Jpreview.m4a",
    ] {
        let error = service
            .play_track("song-1", Some(url), || true)
            .await
            .unwrap_err();
        assert!(
            error.to_string().starts_with("preview URL "),
            "unexpected error for {url:?}: {error}"
        );
    }

    assert!(recording.calls().is_empty());
    assert_playback_state(&state, None, false).await;
}

// The guard must not reject a legitimate preview URL, so Apple's CDN host, a
// plain public host, and a public address literal all still reach the backend.
// `play_track` first forwards the state's volume (the default 0.5) and
// balance (the default center) so the preview starts at the sliders' values,
// then starts the play.
#[tokio::test]
async fn play_track_accepts_a_public_https_preview_url() {
    for url in [
        "https://audio-ssl.itunes.apple.com/itunes-assets/AudioPreview/x.m4a",
        PREVIEW_URL,
        "https://93.184.216.34/preview.m4a",
    ] {
        let recording = Arc::new(RecordingAudio::default());
        let (service, _state) = service_with_audio(Arc::clone(&recording) as Arc<dyn AudioOutput>);
        service
            .play_track("song-1", Some(url), || true)
            .await
            .unwrap();
        assert_eq!(
            recording.calls(),
            vec![
                AudioCall::SetVolume(0.5),
                AudioCall::SetBalance(0.0),
                AudioCall::SetEqualizer(EqSnapshot::default()),
                AudioCall::Play(url.to_string()),
            ]
        );
    }
}

// The first preview must start at the balance already stored in shared state,
// not at the backend's centered default, just as it starts at the stored
// volume.
#[tokio::test]
async fn play_track_starts_the_preview_at_the_states_balance() {
    let (service, state, recording) = recording_service();
    state.lock().await.set_balance(-0.4);

    service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap();

    assert_eq!(
        recording.calls(),
        vec![
            AudioCall::SetVolume(0.5),
            AudioCall::SetBalance(-0.4),
            AudioCall::SetEqualizer(EqSnapshot::default()),
            AudioCall::Play(PREVIEW_URL.to_string()),
        ]
    );
}

// The first preview must start at the equalizer curve already stored in
// shared state, not at the backend's flat default, just as it starts at the
// stored volume and balance. Pin the whole snapshot `play_track` forwards.
#[tokio::test]
async fn play_track_starts_the_preview_at_the_states_equalizer() {
    let (service, state, recording) = recording_service();
    {
        let mut state = state.lock().await;
        state.toggle_equalizer();
        state.set_eq_preamp(3.0);
        state.set_eq_band(0, 4.0);
    }

    service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap();

    let mut bands_db = [0.0; crate::equalizer::BAND_COUNT];
    bands_db[0] = 4.0;
    assert_eq!(
        recording.calls(),
        vec![
            AudioCall::SetVolume(0.5),
            AudioCall::SetBalance(0.0),
            AudioCall::SetEqualizer(EqSnapshot {
                enabled: true,
                preamp_db: 3.0,
                bands_db,
            }),
            AudioCall::Play(PREVIEW_URL.to_string()),
        ]
    );
}

// A backend that cannot start the preview reports it through the seam, so the
// UI's play path can log the failure instead of silently claiming success.
// The preview never started, so shared state must not keep claiming it is
// playing: `current_track` records the selection but `is_playing` is rolled
// back.
#[tokio::test]
async fn play_track_propagates_an_audio_backend_failure() {
    let (service, state) = service_with_audio(Arc::new(FailingAudio::failing_play()));

    let error = service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-1"), false).await;
}

// A play can fail after a newer one has already committed: the user clicks
// song-1, then song-2, and song-1's backend call fails after song-2 is
// playing. The failure rollback must not clear the newer play's flag, or the
// UI would show song-2 stopped while it plays. The guard reports "current" at
// commit and "superseded" at rollback — the window in which a newer play can
// land — and the rollback then leaves the flag set.
#[tokio::test]
async fn play_track_rollback_leaves_a_newer_plays_flag_alone() {
    let (service, state) = service_with_audio(Arc::new(FailingAudio::failing_play()));

    // First call (the commit guard) reports current; the second (the rollback
    // guard) reports superseded.
    let calls = std::cell::Cell::new(0u32);
    let is_current = || {
        let call = calls.get();
        calls.set(call + 1);
        call == 0
    };

    let error = service
        .play_track("song-1", Some(PREVIEW_URL), is_current)
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-1"), true).await;
    // Two calls: the commit guard and the rollback guard. A missing rollback
    // would consult the guard once and leave the assertion above green.
    assert_eq!(calls.get(), 2);
}

// Plays "song-1" against a backend that rejects `command`, asserts the play
// fails with the backend's error and rolled back to "song-1 selected, not
// playing", and hands back the backend, service and state for further checks.
async fn play_rejected_by(
    command: RejectedCommand,
) -> (Arc<RejectingAudio>, AppleMusicService, Arc<Mutex<AppState>>) {
    let audio = Arc::new(RejectingAudio::new(command));
    let (service, state) = service_with_audio(Arc::clone(&audio) as Arc<dyn AudioOutput>);

    let error = service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-1"), false).await;
    (audio, service, state)
}

// `play_track` starts the preview only after the backend accepts the starting
// volume: `set_volume` and `play` are chained, so a backend that cannot set the
// gain must not start the preview at rodio's full-volume default. Pin both the
// surfaced error and the exact command sequence — a regression that issued
// `play` anyway (or set the volume best-effort) would sound the preview while
// reporting failure, and only the recorded calls can catch that.
#[tokio::test]
async fn play_track_does_not_start_the_preview_when_the_volume_cannot_be_set() {
    let (audio, _service, _state) = play_rejected_by(RejectedCommand::SetVolume).await;

    assert_eq!(audio.calls(), vec![AudioCall::SetVolume(0.5)]);
}

// `play_track` starts the preview as `set_volume -> set_balance ->
// set_equalizer -> play`, so the first play uses the sliders' values and the
// panel's curve rather than the backend's defaults. A backend that cannot set
// the balance must stop the chain there: starting the preview anyway would
// sound it at the wrong stereo image while reporting failure. The recorded
// calls pin the order and that nothing past the rejected balance ran; the
// shared-state rollback is the same one the volume and play failures pin. The
// failed start is not remembered either, so a later Play has nothing to
// resume and cannot claim playback the backend never produced.
#[tokio::test]
async fn play_track_does_not_start_the_preview_when_the_balance_cannot_be_set() {
    let (audio, service, state) = play_rejected_by(RejectedCommand::SetBalance).await;

    assert_eq!(
        audio.calls(),
        vec![AudioCall::SetVolume(0.5), AudioCall::SetBalance(0.0)],
        "the start chain must stop at the rejected balance"
    );

    service.resume().await.unwrap();
    assert_playback_state(&state, Some("song-1"), false).await;
    assert_eq!(
        audio.calls(),
        vec![AudioCall::SetVolume(0.5), AudioCall::SetBalance(0.0)],
        "a failed start must not be resumed later"
    );
}

// The equalizer is the last value forwarded before `play`, so a backend that
// cannot apply the curve must not start the preview at a stale or default
// curve. Pin the exact prefix (volume and balance already applied, equalizer
// refused) and that `play` was never reached, plus the rollback the previous
// failure tests share.
#[tokio::test]
async fn play_track_does_not_start_the_preview_when_the_equalizer_cannot_be_set() {
    let (audio, _service, _state) = play_rejected_by(RejectedCommand::SetEqualizer).await;

    assert_eq!(
        audio.calls(),
        vec![
            AudioCall::SetVolume(0.5),
            AudioCall::SetBalance(0.0),
            AudioCall::SetEqualizer(EqSnapshot::default()),
        ],
        "the start chain must stop at the rejected equalizer"
    );
}

// The test above plays once from the default (no track), so it only
// proves the first selection is recorded. The Songs view lets the user
// click any row, so every later selection must replace `current_track`;
// a regression that only set it when none was playing (or that made
// `play_track` idempotent on the id) would clear that test while the
// Now Playing bar silently kept naming the first song. Pin the overwrite,
// and that playback stays on across it.
#[tokio::test]
async fn play_track_replaces_the_current_track_when_another_song_is_played() {
    let (service, state) = service_with_song_1_playing().await;

    service.play_track("song-2", None, || true).await.unwrap();

    assert_playback_state(&state, Some("song-2"), true).await;
}

// A play reply can complete out of order: the user clicks song-1, then
// song-2, and song-1's slower play lands last. Committing that older
// reply would replace the newer track in shared state. The guard is
// evaluated under the state lock, so a superseded play leaves the newer
// track untouched — the seam half of the UI's out-of-order-play guard.
#[tokio::test]
async fn a_superseded_play_leaves_the_newer_track_in_place() {
    let (service, state) = test_service_with_state();

    // The newer play commits, then the older one completes late with a
    // guard reporting it has been superseded.
    service.play_track("song-2", None, || true).await.unwrap();
    service.play_track("song-1", None, || false).await.unwrap();

    assert_playback_state(&state, Some("song-2"), true).await;
}

// A blank id can never name a track, so the seam rejects it instead of
// recording a blank `current_track` and marking nothing as playing. The
// error path the UI's `play_into` reports is pinned here, and
// shared state must be left untouched (the rejection happens before the
// state lock). Both an empty and a whitespace-only id are blank.
#[tokio::test]
async fn play_track_rejects_a_blank_track_id_without_touching_state() {
    let (service, state) = test_service_with_state();

    assert_blank_id_rejected(
        service.play_track("", None, || true),
        service.play_track("   ", None, || true),
        IdKind::Track,
    )
    .await;

    assert_playback_state(&state, None, false).await;
}

// The rejection test above starts from the default (no track, not
// playing), so a regression that cleared the shared state *while*
// rejecting the blank id would still leave it at `None`/`false` and pass
// unnoticed. The Songs view can be clicked mid-playback, so pin the
// rejection against a live track: the already-playing song must survive
// untouched rather than being cleared to "nothing is playing".
#[tokio::test]
async fn play_track_rejects_a_blank_track_id_while_a_song_is_playing() {
    let (service, state) = service_with_song_1_playing().await;

    let error = service.play_track("", None, || true).await.unwrap_err();

    assert_eq!(error.to_string(), "track id must not be blank (got \"\")");
    assert_playback_state(&state, Some("song-1"), true).await;
}

// A track id carrying a terminal control character would inject an escape
// sequence into the `Playing track: {id}` log line (and into the
// `current_track` the Now Playing bar later renders). The seam rejects it
// exactly as it rejects a blank id, leaving shared state untouched; the
// error message escapes the value with `{id:?}` so the report itself is
// safe to print.
#[tokio::test]
async fn play_track_rejects_a_control_character_track_id() {
    let (service, state) = service_with_song_1_playing().await;

    let id = "evil\u{1b}]0;pwnd\u{7}";
    assert_control_character_id_rejected(service.play_track(id, None, || true), IdKind::Track, id)
        .await;

    assert_playback_state(&state, Some("song-1"), true).await;
}
