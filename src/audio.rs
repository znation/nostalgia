//! The audio-output seam: an injectable backend the Apple Music service hands
//! a playable asset URL to.
//!
//! The service never talks to an audio device directly. It holds an
//! `Arc<dyn AudioOutput>` and calls [`AudioOutput::play`] with a preview URL;
//! the production [`RodioOutput`] plays it through `rodio`, [`SilentOutput`]
//! logs and reports success when no device opens, and a test injects a
//! recording fake. Keeping the device behind this trait is what lets the
//! service tests run without sound, and what lets a device-less machine fall
//! back to silence instead of failing to start.
//!
//! `RodioOutput` opens the default device on a dedicated worker thread, which
//! then owns the `rodio` stream and player. Commands cross to that thread over
//! a channel, so the seam stays `Send + Sync` regardless of the platform
//! handle's own thread bounds, and a download or decode failure is logged on
//! the worker rather than surfacing as a caller error.

use std::io::Cursor;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender};

use crate::music_error::AppleMusicError;

/// The audio-output seam: a backend that can play, pause, and stop a track.
///
/// Implementations are shared behind an `Arc`, so every method takes `&self`
/// and the backend owns whatever interior state it needs. The trait is
/// `Send + Sync` because the service hands it across the async executor and
/// the playback worker.
pub trait AudioOutput: Send + Sync {
    /// Starts playing the audio at `url`, replacing whatever is playing now.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the backend cannot accept the
    /// request. [`RodioOutput`] reports one only when its worker thread has
    /// already ended; a download or decode failure on the worker is logged
    /// there instead.
    fn play(&self, url: &str) -> Result<(), AppleMusicError>;

    /// Pauses the current audio, keeping it ready to resume.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the backend cannot accept the
    /// request (see [`AudioOutput::play`]).
    fn pause(&self) -> Result<(), AppleMusicError>;

    /// Stops the current audio and discards it.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the backend cannot accept the
    /// request (see [`AudioOutput::play`]).
    fn stop(&self) -> Result<(), AppleMusicError>;
}

/// An [`AudioOutput`] that produces no sound: it logs each command and reports
/// success.
///
/// This is the stand-in when no output device opens (see
/// [`audio_with_fallback`]) and the backend the browse-only tests use, so the
/// service can be driven end to end without touching an audio device.
#[derive(Debug, Default, Clone, Copy)]
pub struct SilentOutput;

impl AudioOutput for SilentOutput {
    fn play(&self, url: &str) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; not playing {url:?}");
        Ok(())
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; nothing to pause");
        Ok(())
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; nothing to stop");
        Ok(())
    }
}

/// A command sent from [`RodioOutput`] to its worker thread.
enum Command {
    /// Download, decode, and play the URL, replacing the current player.
    Play(String),
    /// Pause the current player.
    Pause,
    /// Stop and discard the current player.
    Stop,
}

/// The production [`AudioOutput`], backed by `rodio`.
///
/// The default output device and the current `rodio` player live on a worker
/// thread; this type only holds the channel that reaches it, so it is
/// `Send + Sync` and cheap to clone behind an `Arc`.
pub struct RodioOutput {
    /// The worker's command channel; wrapped in a `Mutex` because the seam
    /// requires `Sync` and the standard sender is only `Send`.
    commands: Mutex<Sender<Command>>,
}

impl RodioOutput {
    /// Opens the default output device on a dedicated worker thread.
    ///
    /// The worker reports whether the device opened back through a channel, so
    /// a machine with no output device gets `Err` rather than a panic. The
    /// worker then runs until this value is dropped, when its command sender
    /// closes.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the worker thread cannot be spawned
    /// or the default output device cannot be opened.
    pub fn new() -> Result<Self, AppleMusicError> {
        let (commands, receiver) = mpsc::channel();
        let (ready_sender, ready_receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("nostalgia-audio".to_string())
            .spawn(move || run_worker(receiver, ready_sender))
            .map_err(|error| {
                AppleMusicError::new(format!("spawning the audio thread failed: {error}"))
            })?;

        match ready_receiver.recv() {
            Ok(Ok(())) => Ok(Self {
                commands: Mutex::new(commands),
            }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(AppleMusicError::new(
                "the audio thread ended before opening a device",
            )),
        }
    }

    /// Sends `command` to the worker, mapping a closed channel to a seam error.
    fn send(&self, command: Command) -> Result<(), AppleMusicError> {
        self.commands
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send(command)
            .map_err(|_| AppleMusicError::new("the audio thread is gone"))
    }
}

impl AudioOutput for RodioOutput {
    fn play(&self, url: &str) -> Result<(), AppleMusicError> {
        self.send(Command::Play(url.to_string()))
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        self.send(Command::Pause)
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        self.send(Command::Stop)
    }
}

/// Opens the default device and serves [`Command`]s until the sender closes.
///
/// `ready` receives the open result before the command loop starts, so
/// [`RodioOutput::new`] can report a device failure to its caller. Every later
/// failure — a download, a decode — is logged here, because the caller is no
/// longer waiting on this thread.
fn run_worker(receiver: Receiver<Command>, ready: Sender<Result<(), AppleMusicError>>) {
    let device = match rodio::DeviceSinkBuilder::open_default_sink() {
        Ok(device) => device,
        Err(error) => {
            let _ = ready.send(Err(AppleMusicError::new(format!(
                "opening the default audio device failed: {error}"
            ))));
            return;
        }
    };
    let _ = ready.send(Ok(()));

    let mut player: Option<rodio::Player> = None;
    while let Ok(command) = receiver.recv() {
        match command {
            Command::Play(url) => match download_and_decode(&url) {
                Ok(decoder) => {
                    // A fresh player per track replaces the previous one, so a
                    // new selection does not queue behind the old track.
                    let next = rodio::Player::connect_new(device.mixer());
                    next.append(decoder);
                    player = Some(next);
                }
                Err(error) => eprintln!("audio playback failed for {url:?}: {error}"),
            },
            Command::Pause => {
                if let Some(player) = &player {
                    player.pause();
                }
            }
            Command::Stop => {
                if let Some(player) = &player {
                    player.stop();
                }
                player = None;
            }
        }
    }
}

/// Downloads `url` and decodes it as an `MP4`/`AAC` preview.
///
/// The Apple Music preview is an `M4A` (AAC in an `MP4` container), so the
/// decoder is hinted with that format. The whole asset is buffered in memory
/// before decoding because the preview is short and the decoder needs a
/// `Seek` source.
///
/// # Errors
///
/// Returns an [`AppleMusicError`] when the download fails, the response is not
/// a success status, the body cannot be read, or the bytes do not decode.
fn download_and_decode(url: &str) -> Result<rodio::Decoder<Cursor<Vec<u8>>>, AppleMusicError> {
    let mut response = ureq::get(url).call().map_err(|error| {
        AppleMusicError::new(format!("downloading the preview failed: {error}"))
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppleMusicError::new(format!(
            "the preview download returned HTTP {status}"
        )));
    }
    let bytes = response
        .body_mut()
        .read_to_vec()
        .map_err(|error| AppleMusicError::new(format!("reading the preview failed: {error}")))?;
    rodio::Decoder::new_mp4(Cursor::new(bytes))
        .map_err(|error| AppleMusicError::new(format!("decoding the preview failed: {error}")))
}

/// Runs `open` and returns its output, or a [`SilentOutput`] when it fails.
///
/// The service builds its [`RodioOutput`] through this so a machine with no
/// audio device still starts and browses; the failed opener is logged once.
/// Pure and injectable — the opener is a parameter — so the fallback is
/// testable without a device.
pub(crate) fn audio_with_fallback(
    open: impl FnOnce() -> Result<Arc<dyn AudioOutput>, AppleMusicError>,
) -> Arc<dyn AudioOutput> {
    match open() {
        Ok(output) => output,
        Err(error) => {
            eprintln!("audio output unavailable, falling back to silence: {error}");
            Arc::new(SilentOutput)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A recording [`AudioOutput`] so a test can observe which commands
    /// reached the backend without an audio device.
    #[derive(Debug, Default)]
    struct RecordingOutput {
        calls: Mutex<Vec<String>>,
    }

    impl AudioOutput for RecordingOutput {
        fn play(&self, url: &str) -> Result<(), AppleMusicError> {
            self.calls.lock().unwrap().push(format!("play:{url}"));
            Ok(())
        }

        fn pause(&self) -> Result<(), AppleMusicError> {
            self.calls.lock().unwrap().push("pause".to_string());
            Ok(())
        }

        fn stop(&self) -> Result<(), AppleMusicError> {
            self.calls.lock().unwrap().push("stop".to_string());
            Ok(())
        }
    }

    #[test]
    fn silent_output_reports_success_for_every_command() {
        let output = SilentOutput;
        assert!(output.play("https://example.test/preview.m4a").is_ok());
        assert!(output.pause().is_ok());
        assert!(output.stop().is_ok());
    }

    #[test]
    fn audio_with_fallback_returns_the_opened_output() {
        let recording = Arc::new(RecordingOutput::default());
        let opened = Arc::clone(&recording) as Arc<dyn AudioOutput>;

        let output = audio_with_fallback(|| Ok(opened));
        output.play("https://example.test/preview.m4a").unwrap();

        let calls = recording.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "play:https://example.test/preview.m4a");
    }

    #[test]
    fn audio_with_fallback_falls_back_to_silence_when_open_fails() {
        let output = audio_with_fallback(|| Err(AppleMusicError::new("no device")));

        assert!(output.play("https://example.test/preview.m4a").is_ok());
        assert!(output.pause().is_ok());
        assert!(output.stop().is_ok());
    }

    #[test]
    fn rodio_output_new_reports_the_device_result_without_panicking() {
        // A machine with an output device gets `Ok`; one without gets `Err`.
        // Either outcome is acceptable here — this pins only that opening the
        // device is reported through the `Result` rather than a panic.
        let _ = RodioOutput::new();
    }
}
