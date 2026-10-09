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
//! the worker rather than surfacing as a caller error. Each download is bounded
//! by [`PREVIEW_TIMEOUT`], so a stalled server cannot block the worker — and
//! with it every later play, pause, stop, and volume command — forever.

use std::io::Cursor;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use crate::music_error::AppleMusicError;

/// The end-to-end bound on one preview download, from DNS lookup through
/// reading the response body. `ureq` defaults every network timeout to `None`,
/// so without this a server that accepts the connection and then stalls would
/// block the audio worker forever; because the worker serves commands one at a
/// time, every later play, pause, stop, and volume command would queue behind
/// it. Matches the REST client's `REQUEST_TIMEOUT` so both of the app's network
/// calls share one bound.
const PREVIEW_TIMEOUT: Duration = Duration::from_secs(30);

/// The audio-output seam: a backend that can play, pause, and stop a track
/// and set its output gain.
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

    /// Resumes a paused player.
    ///
    /// Has no effect when nothing is loaded — this resumes a paused player, it
    /// does not restart a stopped one (see [`AudioOutput::stop`]).
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the backend cannot accept the
    /// request (see [`AudioOutput::play`]).
    fn resume(&self) -> Result<(), AppleMusicError>;

    /// Stops the current audio and discards it.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the backend cannot accept the
    /// request (see [`AudioOutput::play`]).
    fn stop(&self) -> Result<(), AppleMusicError>;

    /// Sets the output gain, where `1.0` is full volume.
    ///
    /// Applies to the current player, if one is loaded, and is remembered so
    /// the next [`play`](Self::play) starts at the same gain. This is how the
    /// volume slider reaches a preview, and how a preview starts at the
    /// slider's value rather than the backend's default.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the backend cannot accept the
    /// request (see [`AudioOutput::play`]).
    fn set_volume(&self, volume: f32) -> Result<(), AppleMusicError>;
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

    fn resume(&self) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; nothing to resume");
        Ok(())
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; nothing to stop");
        Ok(())
    }

    fn set_volume(&self, volume: f32) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; volume set to {volume}");
        Ok(())
    }
}

/// A command sent from [`RodioOutput`] to its worker thread.
enum Command {
    /// Download, decode, and play the URL, replacing the current player.
    Play(String),
    /// Pause the current player.
    Pause,
    /// Resume the current player, if one is loaded.
    Resume,
    /// Stop and discard the current player.
    Stop,
    /// Set the output gain (`1.0` is full volume).
    SetVolume(f32),
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

    fn resume(&self) -> Result<(), AppleMusicError> {
        self.send(Command::Resume)
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        self.send(Command::Stop)
    }

    fn set_volume(&self, volume: f32) -> Result<(), AppleMusicError> {
        self.send(Command::SetVolume(volume))
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
    // The gain the output should hold. Remembered across players so a volume
    // set while stopped still applies to the next `Play`, and a new player
    // starts at it rather than rodio's full-volume default.
    let mut volume = 1.0_f32;
    while let Ok(command) = receiver.recv() {
        match command {
            Command::Play(url) => match download_and_decode(preview_agent(), &url) {
                Ok(decoder) => {
                    // A fresh player per track replaces the previous one, so a
                    // new selection does not queue behind the old track. It
                    // starts at the remembered gain rather than rodio's
                    // full-volume default.
                    let next = rodio::Player::connect_new(device.mixer());
                    next.set_volume(volume);
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
            Command::Resume => {
                if let Some(player) = &player {
                    player.play();
                }
            }
            Command::Stop => {
                if let Some(player) = &player {
                    player.stop();
                }
                player = None;
            }
            Command::SetVolume(new_volume) => {
                volume = new_volume;
                if let Some(player) = &player {
                    player.set_volume(volume);
                }
            }
        }
    }
}

/// The process-wide [`ureq::Agent`] the audio worker downloads previews
/// through, so consecutive plays reuse its pooled connection. It carries
/// [`PREVIEW_TIMEOUT`] as its global timeout, so a stalled server cannot block
/// the worker forever.
fn preview_agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| agent_with_timeout(PREVIEW_TIMEOUT))
}

/// Builds an agent with `timeout` as its global bound and no redirect
/// following. Split out from [`preview_agent`] so a test can bound a download
/// against a stalled loopback server without waiting out the production 30
/// seconds.
///
/// The agent disables `ureq`'s status-as-error shortcut
/// (`http_status_as_error(false)`), so a 4xx/5xx response reaches
/// [`download_and_decode`] as an `Ok` response and its own status check
/// reports "the preview download returned HTTP …" instead of `ureq`'s bare
/// status error.
///
/// The agent follows no redirects (`max_redirects(0)`), matching the REST
/// agent. `preview_url_problem` validates the preview URL's host once, before
/// the fetch, and `ureq` follows redirects by default; a `Location` to an
/// address that validation refused (an internal address literal, say) would
/// otherwise be fetched, so refusing to follow one keeps the fetch on the
/// single URL that passed validation.
fn agent_with_timeout(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .into()
}

/// Downloads `url` through `agent` and decodes it as an `MP4`/`AAC` preview.
///
/// The Apple Music preview is an `M4A` (AAC in an `MP4` container), so the
/// decoder is hinted with that format. The whole asset is buffered in memory
/// before decoding because the preview is short and the decoder needs a
/// `Seek` source; `ureq` caps that buffer at its 10 MiB body limit.
///
/// # Errors
///
/// Returns an [`AppleMusicError`] when the download fails (including its
/// timeout), the response is not a success status, the body cannot be read, or
/// the bytes do not decode.
fn download_and_decode(
    agent: &ureq::Agent,
    url: &str,
) -> Result<rodio::Decoder<Cursor<Vec<u8>>>, AppleMusicError> {
    let mut response = agent.get(url).call().map_err(|error| {
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
    use crate::test_support::{
        AudioCall, PREVIEW_URL, RecordingAudio, loopback_listener, read_some_request,
        serve_one_response,
    };
    use std::time::Instant;

    #[test]
    fn silent_output_reports_success_for_every_command() {
        let output = SilentOutput;
        assert!(output.play(PREVIEW_URL).is_ok());
        assert!(output.pause().is_ok());
        assert!(output.resume().is_ok());
        assert!(output.stop().is_ok());
        assert!(output.set_volume(0.4).is_ok());
    }

    #[test]
    fn audio_with_fallback_returns_the_opened_output() {
        let recording = Arc::new(RecordingAudio::default());
        let opened = Arc::clone(&recording) as Arc<dyn AudioOutput>;

        let output = audio_with_fallback(|| Ok(opened));
        output.play(PREVIEW_URL).unwrap();

        assert_eq!(
            recording.calls(),
            vec![AudioCall::Play(PREVIEW_URL.to_string())]
        );
    }

    #[test]
    fn audio_with_fallback_falls_back_to_silence_when_open_fails() {
        let output = audio_with_fallback(|| Err(AppleMusicError::new("no device")));

        assert!(output.play(PREVIEW_URL).is_ok());
        assert!(output.pause().is_ok());
        assert!(output.resume().is_ok());
        assert!(output.stop().is_ok());
        assert!(output.set_volume(0.4).is_ok());
    }

    // `ureq` defaults every network timeout to `None`, so a preview server
    // that accepts the connection and never responds used to block the audio
    // worker forever, and every later play/pause/stop command queued behind
    // it. The bound is asserted from a worker thread with `recv_timeout`, so
    // an unbounded download fails this test at 5s instead of hanging the
    // suite.
    #[test]
    fn a_stalled_preview_download_is_bounded_by_the_timeout() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let agent = agent_with_timeout(Duration::from_millis(200));
        let url = format!("http://{address}/stalled");
        let (sender, receiver) = mpsc::channel();
        let started = Instant::now();
        std::thread::spawn(move || {
            let _ = sender.send(download_and_decode(&agent, &url));
        });
        let result = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("the download must return within its bound");
        assert!(result.is_err(), "a stalled preview download must error");
        // A connect failure would also return `Err`, instantly; require that
        // the call actually waited for the bound, so this test only passes
        // because the stalled response was timed out.
        assert!(
            started.elapsed() >= Duration::from_millis(100),
            "returned before the 200ms bound: {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn rodio_output_new_reports_the_device_result_without_panicking() {
        // A machine with an output device gets `Ok`; one without gets `Err`.
        // Either outcome is acceptable here — this pins only that opening the
        // device is reported through the `Result` rather than a panic.
        let _ = RodioOutput::new();
    }

    #[test]
    fn rodio_output_maps_each_seam_method_to_its_worker_command() {
        // Hold the receiver open so the sends succeed, then read back the
        // command each public seam method put on the worker channel.
        let (commands, receiver) = std::sync::mpsc::channel();
        let output = RodioOutput {
            commands: Mutex::new(commands),
        };

        output.play(PREVIEW_URL).unwrap();
        output.pause().unwrap();
        output.resume().unwrap();
        output.stop().unwrap();
        output.set_volume(0.4).unwrap();

        match receiver.try_recv() {
            Ok(Command::Play(url)) => assert_eq!(url, PREVIEW_URL),
            _ => panic!("play did not send Command::Play"),
        }
        assert!(matches!(receiver.try_recv(), Ok(Command::Pause)));
        assert!(matches!(receiver.try_recv(), Ok(Command::Resume)));
        assert!(matches!(receiver.try_recv(), Ok(Command::Stop)));
        assert!(matches!(receiver.try_recv(), Ok(Command::SetVolume(v)) if v == 0.4));
    }

    #[test]
    fn rodio_output_reports_a_closed_worker_for_every_command() {
        // Dropping the receiver stands in for a worker thread that has ended,
        // so every seam method must surface the closed channel as an error
        // rather than panicking.
        let (commands, receiver) = std::sync::mpsc::channel();
        drop(receiver);
        let output = RodioOutput {
            commands: Mutex::new(commands),
        };

        assert_eq!(
            output.play(PREVIEW_URL).unwrap_err().to_string(),
            "the audio thread is gone"
        );
        assert_eq!(
            output.pause().unwrap_err().to_string(),
            "the audio thread is gone"
        );
        assert_eq!(
            output.resume().unwrap_err().to_string(),
            "the audio thread is gone"
        );
        assert_eq!(
            output.stop().unwrap_err().to_string(),
            "the audio thread is gone"
        );
        assert_eq!(
            output.set_volume(0.4).unwrap_err().to_string(),
            "the audio thread is gone"
        );
    }

    // A preview URL that answers with a non-2xx status must be reported as a
    // status failure, before its body reaches the decoder. The agent disables
    // `ureq`'s status-as-error shortcut (`http_status_as_error(false)`), so
    // this 404 arrives as an `Ok` response and `download_and_decode`'s own
    // status check runs. The body is deliberately non-empty: were that check
    // removed, these bytes would go to `new_mp4` and the function would report
    // a decode failure, so pinning the exact status message keeps the check
    // ahead of the decoder.
    #[test]
    fn a_preview_download_with_an_error_status_is_reported() {
        let body = "<html>captive portal, not a preview</html>";
        let address = serve_one_response(&format!(
            "HTTP/1.1 404 Not Found\r\nContent-Type: audio/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ));
        let agent = agent_with_timeout(Duration::from_secs(5));
        let url = format!("http://{address}/missing.m4a");

        let error = download_and_decode(&agent, &url)
            .err()
            .expect("a 404 preview must not decode");
        assert_eq!(
            error.to_string(),
            "the preview download returned HTTP 404 Not Found"
        );
    }

    // A preview URL's host is validated before it reaches this backend, but a
    // hostile or compromised API reply can point a public preview URL at a
    // server that answers 302 to an internal address, which the one-time host
    // check cannot see. `ureq` follows redirects by default, so without this
    // guard the fetch would reach an address the preview validation refused.
    // Pin that the preview agent refuses to follow one, mirroring the REST
    // agent.
    #[test]
    fn a_redirecting_preview_is_not_followed() {
        let (foreign, foreign_addr) = loopback_listener();

        // The redirecting server answers the one request with a 302 pointing
        // at the "foreign" server, exactly what a hostile or compromised API
        // reply could return.
        let redirector_addr = serve_one_response(&format!(
            "HTTP/1.1 302 Found\r\nLocation: http://{foreign_addr}/preview.m4a\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ));

        // The foreign server records whether any redirected request arrived. A
        // one-second poll makes "no request" a bounded, observable result
        // rather than a hang.
        let foreign_thread = std::thread::spawn(move || {
            foreign.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + Duration::from_secs(1);
            loop {
                match foreign.accept() {
                    Ok((mut stream, _)) => return Some(read_some_request(&mut stream)),
                    Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if Instant::now() >= deadline {
                            return None;
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("foreign listener failed: {error}"),
                }
            }
        });

        let agent = agent_with_timeout(Duration::from_secs(2));
        let url = format!("http://{redirector_addr}/preview.m4a");
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(download_and_decode(&agent, &url));
        });
        let result = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("the redirecting preview must return within its bound");
        let error = result.err().expect("a 302 preview must not decode");
        assert_eq!(
            error.to_string(),
            "the preview download returned HTTP 302 Found"
        );

        let followed = foreign_thread.join().unwrap();
        assert!(
            followed.is_none(),
            "the preview fetch reached the redirect target: {followed:?}"
        );
    }

    // A 2xx response whose body is not an MP4/AAC preview (a captive portal's
    // HTML, a truncated download) must be reported as a decode failure, not
    // panic the worker or play noise.
    #[test]
    fn a_preview_download_that_does_not_decode_is_reported() {
        let address = serve_one_response(
            "HTTP/1.1 200 OK\r\nContent-Type: audio/mp4\r\nContent-Length: 4\r\nConnection: close\r\n\r\nJUNK",
        );
        let agent = agent_with_timeout(Duration::from_secs(5));
        let url = format!("http://{address}/not-a-preview.m4a");

        let error = download_and_decode(&agent, &url)
            .err()
            .expect("garbage bytes must not decode as an MP4 preview");
        assert!(
            error.to_string().contains("decoding the preview failed"),
            "unexpected error: {error}"
        );
    }
}
