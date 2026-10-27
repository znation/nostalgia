//! Recording and failing `AudioOutput` fakes shared by the audio, service, and `ui` tests.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use crate::apple_music::AppleMusicError;
use crate::audio::{AudioOutput, EqSnapshot, PlaybackFormat, Spectrum};
use crate::equalizer::BAND_COUNT;

use super::PREVIEW_URL;

/// The commands a [`RecordingAudio`] recorded, in order, so a test can pin
/// exactly what the service handed the audio backend. The derive drops `Eq`
/// because [`AudioCall::SetVolume`], [`AudioCall::SetBalance`], and
/// [`AudioCall::SetEqualizer`] carry an `f32`, which is only `PartialEq`; the
/// only uses compare whole `Vec<AudioCall>`s with `assert_eq!`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AudioCall {
    /// `play` was called with this URL.
    Play(String),
    /// `pause` was called.
    Pause,
    /// `resume` was called.
    Resume,
    /// `stop` was called.
    Stop,
    /// `set_volume` was called with this gain.
    SetVolume(f32),
    /// `set_balance` was called with this balance.
    SetBalance(f32),
    /// `seek_by` was called with this signed offset in seconds.
    SeekBy(i64),
    /// `seek_to` was called with this absolute position.
    SeekTo(Duration),
    /// `set_equalizer` was called with this curve.
    SetEqualizer(EqSnapshot),
}

/// Asserts `calls` recorded the four [`AudioCall`]s that starting a preview
/// always emits — the play-time volume set to the default `0.5`, the balance
/// set to the default `0.0`, the default flat/disabled equalizer, then
/// [`PREVIEW_URL`] — followed by `expected`, the calls under test. The
/// `apple_music` service tests and the `ui` transport tests both need that
/// prefix, so it lives here once.
pub(crate) fn assert_preview_calls(calls: &[AudioCall], expected: &[AudioCall]) {
    let mut wanted = vec![
        AudioCall::SetVolume(0.5),
        AudioCall::SetBalance(0.0),
        AudioCall::SetEqualizer(EqSnapshot::default()),
        AudioCall::Play(PREVIEW_URL.to_string()),
    ];
    wanted.extend_from_slice(expected);
    assert_eq!(calls, wanted);
}

/// The fixed spectrum [`RecordingAudio`] reports: one distinctive nonzero
/// band, so a test can pin that the UI draws what the backend's loaded preview
/// reports rather than an all-zero fallback.
pub(crate) fn recording_spectrum() -> Spectrum {
    let mut bands = [0.0; BAND_COUNT];
    bands[2] = 0.5;
    Spectrum { bands }
}

/// Appends `call` to a fake backend's recorded calls and reports success. The
/// recording [`AudioOutput`] fakes in this module and the `apple_music` tests
/// all share this lock-and-push body, so a change to how a command is logged
/// happens once.
pub(crate) fn record_call(
    calls: &Mutex<Vec<AudioCall>>,
    call: AudioCall,
) -> Result<(), AppleMusicError> {
    calls.lock().unwrap().push(call);
    Ok(())
}

/// A recording [`AudioOutput`] so a test observes the commands issued to the
/// backend without touching an audio device. The `apple_music`, `ui`, and
/// `audio` suites all need one, so the fake lives here once.
#[derive(Debug, Default)]
pub(crate) struct RecordingAudio {
    calls: Mutex<Vec<AudioCall>>,
    finished: AtomicBool,
}

impl RecordingAudio {
    /// A snapshot of the recorded calls, cloned out from under the mutex.
    pub(crate) fn calls(&self) -> Vec<AudioCall> {
        self.calls.lock().unwrap().clone()
    }

    /// Appends `call` to the recorded calls and reports success, the body every
    /// accepted [`AudioOutput`] command shares.
    fn record(&self, call: AudioCall) -> Result<(), AppleMusicError> {
        record_call(&self.calls, call)
    }

    /// Sets the finished state [`AudioOutput::is_finished`] reports, so a test
    /// can drive both values of a preview's completion without a real player.
    pub(crate) fn set_finished(&self, finished: bool) {
        self.finished.store(finished, Ordering::Relaxed);
    }
}

impl AudioOutput for RecordingAudio {
    fn play(&self, url: &str) -> Result<(), AppleMusicError> {
        self.record(AudioCall::Play(url.to_string()))
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        self.record(AudioCall::Pause)
    }

    fn resume(&self) -> Result<(), AppleMusicError> {
        self.record(AudioCall::Resume)
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        self.record(AudioCall::Stop)
    }

    fn set_volume(&self, volume: f32) -> Result<(), AppleMusicError> {
        self.record(AudioCall::SetVolume(volume))
    }

    fn set_balance(&self, balance: f32) -> Result<(), AppleMusicError> {
        self.record(AudioCall::SetBalance(balance))
    }

    fn seek_by(&self, offset_secs: i64) -> Result<(), AppleMusicError> {
        self.record(AudioCall::SeekBy(offset_secs))
    }

    fn seek_to(&self, position: Duration) -> Result<(), AppleMusicError> {
        self.record(AudioCall::SeekTo(position))
    }

    /// Reports a fixed stub position; the recording fake models no real
    /// playback clock, so tests that need a known elapsed value use this one.
    fn position(&self) -> Option<Duration> {
        Some(Duration::from_secs(7))
    }

    /// Reports a fixed stub format, so a test can pin that the UI displays
    /// what the backend's loaded preview reports.
    fn playback_format(&self) -> Option<PlaybackFormat> {
        Some(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: Some(256),
        })
    }

    /// Reports a fixed stub spectrum, so the LCD visualizer has known bands
    /// and a test can pin what the backend's loaded preview reports.
    fn spectrum(&self) -> Option<Spectrum> {
        Some(recording_spectrum())
    }

    /// Reports a fixed stub preview length, so the seek bar has a scale and a
    /// test can pin what the backend's loaded preview reports.
    fn duration(&self) -> Option<Duration> {
        Some(Duration::from_secs(30))
    }

    /// Reports the settable finished state, so a test can drive a preview's
    /// natural end without a real player.
    fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }

    fn set_equalizer(&self, equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
        self.record(AudioCall::SetEqualizer(equalizer))
    }
}

/// The commands an [`AudioOutput`] fake refuses, so a test can name the
/// backend failure it drives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FailingCommand {
    /// `play`.
    Play,
    /// `pause`, `resume`, and `stop`. The three transport commands share one
    /// counter, because a test that drives them wants one total.
    Transport,
    /// `set_volume`.
    Volume,
    /// `set_balance`.
    Balance,
    /// `seek_by`.
    Seek,
    /// `set_equalizer`.
    Equalizer,
}

/// An [`AudioOutput`] that refuses the commands a test names and accepts the
/// rest, so a test can pin that the service propagates the backend's error.
///
/// The `apple_music` and `ui` suites each need a backend that fails one
/// command — a play, a seek, the volume and balance forwards, the equalizer
/// forward, and the transport commands — and each of those fakes implemented
/// every [`AudioOutput`] method to accept the commands it did not fail.
/// One fake serves all of them here: a constructor names the commands it
/// refuses, and the per-command counters let a test prove its arm reached the
/// failing backend instead of skipping the forward, and that the other arm
/// stayed untouched.
#[derive(Debug)]
pub(crate) struct FailingAudio {
    /// The commands this fake refuses; every other command succeeds.
    failing: Vec<FailingCommand>,
    play_attempts: AtomicUsize,
    transport_attempts: AtomicUsize,
    volume_attempts: AtomicUsize,
    balance_attempts: AtomicUsize,
    seek_attempts: AtomicUsize,
    equalizer_attempts: AtomicUsize,
}

impl FailingAudio {
    /// A fake whose `play` fails, so a test can pin that the service
    /// propagates the backend's error.
    pub(crate) fn failing_play() -> Self {
        Self::refusing(&[FailingCommand::Play])
    }

    /// A fake whose `seek_by` fails, so a test can pin that the service
    /// propagates the backend's error and the UI logs it without scheduling
    /// work.
    pub(crate) fn failing_seek() -> Self {
        Self::refusing(&[FailingCommand::Seek])
    }

    /// A fake whose volume and balance commands fail, so a UI test can pin
    /// that each update arm logs the backend's error and still commits its
    /// state mutation.
    pub(crate) fn failing_volume_and_balance() -> Self {
        Self::refusing(&[FailingCommand::Volume, FailingCommand::Balance])
    }

    /// A fake whose equalizer command fails, so a UI test can pin that the
    /// equalizer update arm logs the backend's error and still commits its
    /// state mutation.
    pub(crate) fn failing_equalizer() -> Self {
        Self::refusing(&[FailingCommand::Equalizer])
    }

    /// A fake whose pause, resume, and stop commands fail, so a UI test can
    /// reach a loaded preview and then drive each transport arm through
    /// `loading::transport_into`'s error branch.
    pub(crate) fn failing_transport() -> Self {
        Self::refusing(&[FailingCommand::Transport])
    }

    /// A fake that refuses exactly the commands in `commands`.
    fn refusing(commands: &[FailingCommand]) -> Self {
        Self {
            failing: commands.to_vec(),
            play_attempts: AtomicUsize::new(0),
            transport_attempts: AtomicUsize::new(0),
            volume_attempts: AtomicUsize::new(0),
            balance_attempts: AtomicUsize::new(0),
            seek_attempts: AtomicUsize::new(0),
            equalizer_attempts: AtomicUsize::new(0),
        }
    }

    /// How many times the backend refused `command`. Only refusals are
    /// counted, so a command this fake accepts always reads zero.
    pub(crate) fn attempts(&self, command: FailingCommand) -> usize {
        self.counter(command).load(Ordering::SeqCst)
    }

    /// The counter that records the refusals of `command`.
    fn counter(&self, command: FailingCommand) -> &AtomicUsize {
        match command {
            FailingCommand::Play => &self.play_attempts,
            FailingCommand::Transport => &self.transport_attempts,
            FailingCommand::Volume => &self.volume_attempts,
            FailingCommand::Balance => &self.balance_attempts,
            FailingCommand::Seek => &self.seek_attempts,
            FailingCommand::Equalizer => &self.equalizer_attempts,
        }
    }

    /// Refuses `command` when this fake fails it, counting the refusal; the
    /// commands it does not fail succeed.
    fn attempt(&self, command: FailingCommand) -> Result<(), AppleMusicError> {
        if !self.failing.contains(&command) {
            return Ok(());
        }
        self.counter(command).fetch_add(1, Ordering::SeqCst);
        Err(AppleMusicError::new("the audio device is gone"))
    }
}

impl AudioOutput for FailingAudio {
    fn play(&self, _url: &str) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Play)
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Transport)
    }

    fn resume(&self) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Transport)
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Transport)
    }

    fn set_volume(&self, _volume: f32) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Volume)
    }

    fn set_balance(&self, _balance: f32) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Balance)
    }

    fn seek_by(&self, _offset_secs: i64) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Seek)
    }

    fn seek_to(&self, _position: Duration) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Seek)
    }

    fn set_equalizer(&self, _equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Equalizer)
    }
}
