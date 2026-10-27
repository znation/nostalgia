use super::*;

// `set_output_volume` forwards to the injected backend without touching the
// shared state: the UI stores the clamped value itself, so this method is only
// the seam to the output gain.
#[tokio::test]
async fn set_output_volume_forwards_to_the_audio_backend() {
    let recording = Arc::new(RecordingAudio::default());
    let (service, state) = service_with_audio(Arc::clone(&recording) as Arc<dyn AudioOutput>);

    service.set_output_volume(0.3).unwrap();

    assert_eq!(recording.calls(), vec![AudioCall::SetVolume(0.3)]);
    assert_eq!(state.lock().await.volume(), 0.5);
}

// `set_output_balance` forwards to the injected backend without touching the
// shared state: the UI stores the clamped value itself, so this method is only
// the seam to the output's stereo image.
#[tokio::test]
async fn set_output_balance_forwards_to_the_audio_backend() {
    let recording = Arc::new(RecordingAudio::default());
    let (service, state) = service_with_audio(Arc::clone(&recording) as Arc<dyn AudioOutput>);

    service.set_output_balance(-0.4).unwrap();

    assert_eq!(recording.calls(), vec![AudioCall::SetBalance(-0.4)]);
    assert_eq!(state.lock().await.balance(), 0.0);
}

// `seek_output` forwards a relative seek to the injected backend without
// touching the shared state: the position belongs to the playing preview, not
// to `AppState`.
#[tokio::test]
async fn seek_output_forwards_to_the_audio_backend() {
    let recording = Arc::new(RecordingAudio::default());
    let (service, state) = service_with_audio(Arc::clone(&recording) as Arc<dyn AudioOutput>);

    service.seek_output(-5).unwrap();

    assert_eq!(recording.calls(), vec![AudioCall::SeekBy(-5)]);
    assert_eq!(*state.lock().await, AppState::default());
}

/// Runs `seek` against a backend whose seek fails and asserts the failure
/// reaches the caller after exactly one attempt, leaving the shared state alone.
async fn assert_seek_failure_propagates(
    seek: impl FnOnce(&AppleMusicService) -> Result<(), AppleMusicError>,
) {
    let audio = Arc::new(FailingAudio::failing_seek());
    let (service, state) = service_with_audio(Arc::clone(&audio) as Arc<dyn AudioOutput>);

    let error = seek(&service).unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_eq!(audio.attempts(FailingCommand::Seek), 1);
    assert_eq!(*state.lock().await, AppState::default());
}

// A backend that cannot seek reports it through the seam, so the UI's seek
// path can log the failure instead of silently claiming the preview moved.
#[tokio::test]
async fn seek_output_propagates_an_audio_backend_failure() {
    assert_seek_failure_propagates(|service| service.seek_output(-5)).await;
}

// `playback_position` reports exactly what the injected backend answers, so
// the UI's readout has one seam to the loaded preview's clock.
#[tokio::test]
async fn playback_position_forwards_the_audio_output_position() {
    let (service, _state) = service_with_audio(Arc::new(RecordingAudio::default()));

    assert_eq!(service.playback_position(), Some(Duration::from_secs(7)));
}

// `playback_format` reports exactly what the injected backend answers, so the
// LCD's status row has one seam to the loaded preview's format.
#[tokio::test]
async fn playback_format_forwards_the_audio_output_format() {
    let (service, _state) = service_with_audio(Arc::new(RecordingAudio::default()));

    assert_eq!(
        service.playback_format(),
        Some(crate::audio::PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: Some(256),
        })
    );
}

// `playback_spectrum` reports exactly what the injected backend answers, so
// the LCD visualizer has one seam to the loaded preview's live bands.
#[tokio::test]
async fn playback_spectrum_forwards_the_audio_output_spectrum() {
    let (service, _state) = service_with_audio(Arc::new(RecordingAudio::default()));

    assert_eq!(
        service.playback_spectrum(),
        Some(crate::test_support::recording_spectrum())
    );
}

// `preview_finished` reports exactly what the injected backend answers and
// records no audio command, so the completion poll stays observational.
#[tokio::test]
async fn preview_finished_forwards_the_audio_output_state() {
    let recording = Arc::new(RecordingAudio::default());
    let (service, _state) = service_with_audio(Arc::clone(&recording) as Arc<dyn AudioOutput>);

    assert!(!service.preview_finished());
    recording.set_finished(true);
    assert!(service.preview_finished());
    recording.set_finished(false);
    assert!(!service.preview_finished());
    assert!(
        recording.calls().is_empty(),
        "reading the finished state must not record an audio command"
    );
}

// `seek_output_to` forwards an absolute seek to the injected backend without
// touching the shared state: the position belongs to the playing preview, not
// to `AppState`.
#[tokio::test]
async fn seek_output_to_forwards_an_absolute_position() {
    let recording = Arc::new(RecordingAudio::default());
    let (service, state) = service_with_audio(Arc::clone(&recording) as Arc<dyn AudioOutput>);

    service.seek_output_to(Duration::from_secs(12)).unwrap();

    assert_eq!(
        recording.calls(),
        vec![AudioCall::SeekTo(Duration::from_secs(12))]
    );
    assert_eq!(*state.lock().await, AppState::default());
}

// A backend that cannot seek reports the failure through the absolute-seek
// seam too, so the UI's release path logs it instead of claiming the preview
// moved.
#[tokio::test]
async fn seek_output_to_propagates_an_audio_backend_failure() {
    assert_seek_failure_propagates(|service| service.seek_output_to(Duration::from_secs(12))).await;
}

// `playback_duration` reports exactly what the injected backend answers, so
// the seek bar has one seam to the loaded preview's own length.
#[tokio::test]
async fn playback_duration_forwards_the_audio_output_duration() {
    let (service, _state) = service_with_audio(Arc::new(RecordingAudio::default()));

    assert_eq!(service.playback_duration(), Some(Duration::from_secs(30)));
}

// `set_output_equalizer` forwards to the injected backend without touching the
// shared state: the UI stores the clamped curve itself, so this method is only
// the seam to the output's equalizer.
#[tokio::test]
async fn set_output_equalizer_forwards_to_the_audio_backend() {
    let recording = Arc::new(RecordingAudio::default());
    let (service, state) = service_with_audio(Arc::clone(&recording) as Arc<dyn AudioOutput>);
    let snapshot = EqSnapshot {
        enabled: true,
        preamp_db: 3.0,
        bands_db: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0],
    };

    service.set_output_equalizer(snapshot).unwrap();

    assert_eq!(recording.calls(), vec![AudioCall::SetEqualizer(snapshot)]);
    let state = state.lock().await;
    assert!(!state.eq_enabled);
    assert_eq!(state.eq_preamp(), 0.0);
    assert_eq!(state.eq_bands(), [0.0; crate::equalizer::BAND_COUNT]);
}
