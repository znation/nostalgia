//! The `rodio`-backed output: its worker thread, command channel, and the
//! player the worker drives.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use rodio::Source;
use rodio::source::SeekError;

use crate::music_error::AppleMusicError;

use super::balance::BalanceSource;
use super::equalizer::{EqSettings, EqSnapshot, EqSource};
use super::meter::{MeterSource, SpectrumMeter};
use super::preview::{DecodedPreview, download_and_decode, preview_agent};
use super::{AudioOutput, PlaybackFormat, Spectrum};

/// The longest [`RodioOutput::new`] waits for the worker to report whether the
/// default audio device opened.
///
/// `rodio::DeviceSinkBuilder::open_default_sink` talks to the platform audio
/// daemon, which can block indefinitely when that daemon is wedged or
/// unreachable. `new` runs on the caller's thread — the binary builds the
/// service before it starts the UI — so an unbounded wait there would stall the
/// whole app at startup and the silent fallback would never get a chance to
/// stand in. Five seconds is far longer than a healthy open takes and short
/// enough that a wedged one is reported instead of hanging.
pub(super) const DEVICE_OPEN_TIMEOUT: Duration = Duration::from_secs(5);

/// The longest [`RodioOutput::position`], [`RodioOutput::playback_format`],
/// [`RodioOutput::spectrum`], [`RodioOutput::duration`], and
/// [`RodioOutput::is_finished`] wait for the worker to answer.
///
/// The worker handles commands in channel order, so a query issued while the
/// worker is downloading a preview is answered only after that download
/// installs the new player. A download can hold the worker for up to
/// [`super::preview::PREVIEW_TIMEOUT`], far longer than a UI poll should
/// block, so this bound is short: a query that misses it reports `None`
/// rather than stalling the caller, and the next poll asks again. It bounds
/// all five queued backend queries — the position, the format, the spectrum,
/// the preview's length, and whether the preview has finished — because the
/// same download sits in front of any of them.
pub(super) const WORKER_QUERY_TIMEOUT: Duration = Duration::from_millis(100);

/// A command sent from [`RodioOutput`] to its worker thread.
pub(super) enum Command {
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
    /// Set the stereo balance (`-1.0` hard left, `1.0` hard right).
    SetBalance(f32),
    /// Seek the current player by this many seconds (negative moves backward).
    SeekBy(i64),
    /// Seek the current player to this absolute position.
    SeekTo(Duration),
    /// Report the current player's position on `reply` (`None` when none is
    /// loaded).
    Position(Sender<Option<Duration>>),
    /// Report the loaded preview's own length on `reply` (`None` when none is
    /// loaded).
    Duration(Sender<Option<Duration>>),
    /// Report the loaded preview's sample rate and channel mode on `reply`
    /// (`None` when none is loaded).
    PlaybackFormat(Sender<Option<PlaybackFormat>>),
    /// Report the loaded preview's live spectrum on `reply` (`None` when none
    /// is loaded).
    Spectrum(Sender<Option<Spectrum>>),
    /// Report whether the loaded preview has played to its end on `reply`
    /// (`None` when none is loaded).
    Finished(Sender<Option<bool>>),
    /// Set the equalizer curve applied to the output.
    SetEqualizer(EqSnapshot),
}

/// The player handle [`serve_commands`] drives.
///
/// `rodio::Player` can only be built from an open output device, so the
/// command loop names this small seam rather than the concrete type:
/// production implements it for `rodio::Player` over the opened device, and a
/// test stands in a recording fake to exercise the loop's state transitions
/// (including the failed-preview path) without a device. The methods mirror
/// the `rodio::Player` operations the loop uses.
pub(super) trait WorkerPlayer {
    /// A decoded preview the player can hold.
    type Preview;

    /// Sets the player's output gain.
    fn set_volume(&self, volume: f32);
    /// Sets the player's stereo balance.
    fn set_balance(&self, balance: f32);
    /// Seeks the player relative to its current position.
    fn seek_by(&self, offset_secs: i64);
    /// Seeks the player to an absolute position.
    fn seek_to(&self, position: Duration);
    /// Reports the player's current position.
    fn position(&self) -> Duration;
    /// Reports the loaded preview's own length, when the player has one. The
    /// default answers `None`, so a fake without a length compiles.
    fn duration(&self) -> Option<Duration> {
        None
    }
    /// Reports the loaded preview's format, when the player has one. The
    /// default answers `None`, so a fake without a format compiles.
    fn playback_format(&self) -> Option<PlaybackFormat> {
        None
    }
    /// Reports the loaded preview's live spectrum, when the player has one.
    /// The default answers `None`, so a fake without a spectrum compiles.
    fn spectrum(&self) -> Option<Spectrum> {
        None
    }
    /// Reports whether the player's loaded preview has played to its end. The
    /// default answers `false`, so a fake without a finished state compiles.
    fn is_finished(&self) -> bool {
        false
    }
    /// Sets the player's equalizer curve.
    fn set_equalizer(&self, equalizer: EqSnapshot);
    /// Appends `preview` to the player's queue.
    fn append(&self, preview: Self::Preview);
    /// Pauses playback.
    fn pause(&self);
    /// Resumes playback.
    fn play(&self);
    /// Stops playback and clears the queue.
    fn stop(&self);
}

/// Reads `source`'s sample rate and channel count into a [`PlaybackFormat`],
/// the format the Now Playing LCD's status row shows, and computes its average
/// bitrate from `encoded_bytes` and the source's own length when it reports
/// one.
pub(super) fn format_for_source(source: &impl rodio::Source, encoded_bytes: u64) -> PlaybackFormat {
    PlaybackFormat {
        sample_rate_hz: source.sample_rate().get(),
        channels: source.channels().get(),
        bitrate_kbps: source
            .total_duration()
            .and_then(|duration| preview_bitrate_kbps(encoded_bytes, duration)),
    }
}

/// Computes the average encoded bitrate in kilobits per second for an
/// `encoded_bytes`-byte body spanning `duration`.
///
/// One bit per millisecond is one kilobit per second, so the byte count is
/// multiplied by eight and divided by the duration in milliseconds, rounded to
/// the nearest kilobit. Returns `None` when the duration is zero (nothing to
/// divide by) or the result does not fit a `u32`.
pub(super) fn preview_bitrate_kbps(encoded_bytes: u64, duration: Duration) -> Option<u32> {
    let millis = duration.as_millis();
    if millis == 0 {
        return None;
    }
    let bits = u128::from(encoded_bytes) * 8;
    let kbps = (bits + millis / 2) / millis;
    u32::try_from(kbps).ok()
}

/// Returns `position` moved by `offset_secs`: backward for a negative offset
/// (saturating at zero) and forward for a positive one (saturating at
/// [`Duration::MAX`]).
pub(super) fn seek_target(position: Duration, offset_secs: i64) -> Duration {
    let offset = Duration::from_secs(offset_secs.unsigned_abs());
    if offset_secs < 0 {
        position.saturating_sub(offset)
    } else {
        position.saturating_add(offset)
    }
}

/// Clamps `position` to the loaded preview's `duration`, or passes it through
/// when the backend reports no length.
pub(super) fn clamp_seek(position: Duration, duration: Option<Duration>) -> Duration {
    match duration {
        Some(duration) => position.min(duration),
        None => position,
    }
}

/// Reads `source`'s own length, the span the seek bar draws.
pub(super) fn duration_for_source(source: &impl rodio::Source) -> Option<Duration> {
    source.total_duration()
}

/// Reports `meter`'s live spectrum only once the player holds a preview.
///
/// `loaded` is the player's format cell: it is set only after a preview is
/// appended, so an unloaded player (before its append, or after a stop)
/// reports nothing rather than a window that still holds older audio.
pub(super) fn spectrum_for_loaded_player(loaded: bool, meter: &SpectrumMeter) -> Option<Spectrum> {
    loaded.then(|| meter.take())
}

/// The two player operations a relative seek needs, split from
/// [`RodioPlayer`] so the composition of reading the position and seeking to
/// [`seek_target`] from it is testable without an output device.
pub(super) trait SeekablePlayer {
    fn position(&self) -> Duration;
    fn seek_to(&self, position: Duration) -> Result<(), SeekError>;
}

impl SeekablePlayer for rodio::Player {
    fn position(&self) -> Duration {
        self.get_pos()
    }

    fn seek_to(&self, position: Duration) -> Result<(), SeekError> {
        self.try_seek(position)
    }
}

/// Seeks `player` by `offset_secs` from its current position, logging a
/// rejected seek without changing any state.
pub(super) fn seek_player(player: &impl SeekablePlayer, offset_secs: i64) {
    let position = player.position();
    if let Err(error) = player.seek_to(seek_target(position, offset_secs)) {
        eprintln!("seeking the preview failed: {error}");
    }
}

/// The production [`WorkerPlayer`]: a `rodio::Player` plus the balance and
/// equalizer shared with the source adapters wrapping each preview it appends.
pub(super) struct RodioPlayer {
    player: rodio::Player,
    balance: Arc<AtomicU32>,
    equalizer: Arc<EqSettings>,
    /// The format of the preview appended to this player, set once because
    /// each played track builds a fresh player.
    format: OnceLock<PlaybackFormat>,
    /// The appended preview's own length, set once beside the format. It can
    /// be absent even with a player loaded when the source cannot report it.
    duration: OnceLock<Option<Duration>>,
    /// The mono window every sample this player pulls is recorded into for
    /// the LCD visualizer.
    spectrum: Arc<SpectrumMeter>,
}

impl RodioPlayer {
    fn new(player: rodio::Player) -> Self {
        Self {
            player,
            balance: Arc::new(AtomicU32::new(0.0_f32.to_bits())),
            equalizer: Arc::new(EqSettings::default()),
            format: OnceLock::new(),
            duration: OnceLock::new(),
            spectrum: Arc::new(SpectrumMeter::default()),
        }
    }

    /// Wraps `source` in the meter and equalizer chain and appends it, setting
    /// the format and length cells from the source itself. Production appends
    /// a decoded preview through here; keeping the chain generic lets a test
    /// drive the same wiring with an in-memory source.
    pub(super) fn append_source<S>(&self, source: S, encoded_bytes: u64)
    where
        S: Source<Item = f32> + Send + 'static,
    {
        let _ = self.format.set(format_for_source(&source, encoded_bytes));
        let _ = self.duration.set(duration_for_source(&source));
        let metered = MeterSource::new(source, Arc::clone(&self.spectrum));
        let equalized = EqSource::new(metered, Arc::clone(&self.equalizer));
        self.player
            .append(BalanceSource::new(equalized, Arc::clone(&self.balance)));
    }
}

#[cfg(test)]
impl RodioPlayer {
    /// Builds a player without an output device, sharing `spectrum` with the
    /// caller and returning the queue its appended audio is pulled from. The
    /// format cell stays empty until an append, as it does in production.
    pub(super) fn without_device(
        spectrum: Arc<SpectrumMeter>,
    ) -> (Self, rodio::queue::SourcesQueueOutput) {
        let (player, queue) = rodio::Player::new();
        (
            Self {
                player,
                balance: Arc::new(AtomicU32::new(0.0_f32.to_bits())),
                equalizer: Arc::new(EqSettings::default()),
                format: OnceLock::new(),
                duration: OnceLock::new(),
                spectrum,
            },
            queue,
        )
    }

    /// Sets the format cell the way an append does, without needing a decoded
    /// preview.
    pub(super) fn mark_loaded_for_test(&self) {
        let _ = self.format.set(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 1,
            bitrate_kbps: None,
        });
    }
}

impl WorkerPlayer for RodioPlayer {
    type Preview = DecodedPreview;

    fn set_volume(&self, volume: f32) {
        self.player.set_volume(volume);
    }

    fn set_balance(&self, balance: f32) {
        self.balance.store(balance.to_bits(), Ordering::Relaxed);
    }

    fn seek_by(&self, offset_secs: i64) {
        seek_player(&self.player, offset_secs);
    }

    fn seek_to(&self, position: Duration) {
        let target = clamp_seek(position, self.duration());
        if let Err(error) = self.player.try_seek(target) {
            eprintln!("seeking the preview failed: {error}");
        }
    }

    fn position(&self) -> Duration {
        self.player.get_pos()
    }

    fn duration(&self) -> Option<Duration> {
        self.duration.get().copied().flatten()
    }

    fn playback_format(&self) -> Option<PlaybackFormat> {
        self.format.get().copied()
    }

    fn spectrum(&self) -> Option<Spectrum> {
        // The format cell is set only after a preview is appended, so a player
        // without one (before its append, or after a stop) reports nothing
        // rather than a window that still holds older audio.
        spectrum_for_loaded_player(self.format.get().is_some(), &self.spectrum)
    }

    fn is_finished(&self) -> bool {
        // `empty` also holds before a preview is ever appended, so the format
        // gate keeps a fresh player from reporting a finished preview.
        self.format.get().is_some() && self.player.empty()
    }

    fn set_equalizer(&self, equalizer: EqSnapshot) {
        self.equalizer.store(equalizer);
    }

    fn append(&self, preview: Self::Preview) {
        // Read the format and length from the decoded source before moving it
        // into the adapter chain; the encoded length comes from the download
        // so the format can report the bitrate. A fresh player per track sets
        // each cell once.
        let DecodedPreview {
            source,
            encoded_bytes,
        } = preview;
        self.append_source(source, encoded_bytes);
    }

    fn pause(&self) {
        self.player.pause();
    }

    fn play(&self) {
        self.player.play();
    }

    fn stop(&self) {
        self.player.stop();
    }
}

/// The production [`AudioOutput`], backed by `rodio`.
///
/// The default output device and the current `rodio` player live on a worker
/// thread; this type only holds the channel that reaches it, so it is
/// `Send + Sync` and cheap to clone behind an `Arc`.
pub struct RodioOutput {
    /// The worker's command channel; wrapped in a `Mutex` because the seam
    /// requires `Sync` and the standard sender is only `Send`.
    pub(super) commands: Mutex<Sender<Command>>,
}

impl RodioOutput {
    /// Opens the default output device on a dedicated worker thread.
    ///
    /// The worker reports whether the device opened back through a channel, so
    /// a machine with no output device gets `Err` rather than a panic. The wait
    /// for that report is bounded by [`DEVICE_OPEN_TIMEOUT`], so a platform
    /// audio daemon that blocks the open cannot stall the caller. The worker
    /// then runs until this value is dropped, when its command sender closes.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the worker thread cannot be spawned,
    /// the default output device cannot be opened, or the open does not report
    /// within [`DEVICE_OPEN_TIMEOUT`].
    pub fn new() -> Result<Self, AppleMusicError> {
        let (commands, receiver) = mpsc::channel();
        let (ready_sender, ready_receiver) = mpsc::channel();
        crate::threads::spawn_named("nostalgia-audio", move || {
            run_worker(receiver, ready_sender)
        })
        .map_err(|error| {
            AppleMusicError::new(format!("spawning the audio thread failed: {error}"))
        })?;

        // Dropping `commands` on the error path closes the worker's channel, so
        // a worker still blocked in `open_default_sink` exits as soon as that
        // call returns instead of lingering with a live command queue.
        await_device_ready(ready_receiver, DEVICE_OPEN_TIMEOUT)?;
        Ok(Self {
            commands: Mutex::new(commands),
        })
    }

    /// Sends `command` to the worker, mapping a closed channel to a seam error.
    fn send(&self, command: Command) -> Result<(), AppleMusicError> {
        self.commands
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send(command)
            .map_err(|_| AppleMusicError::new("the audio thread is gone"))
    }

    /// Issues a query command to the worker and waits up to
    /// [`WORKER_QUERY_TIMEOUT`] for its reply.
    ///
    /// Returns `None` when the worker is gone or misses the bound, matching
    /// the five backend queries [`AudioOutput::position`],
    /// [`AudioOutput::duration`], [`AudioOutput::playback_format`],
    /// [`AudioOutput::spectrum`], and [`AudioOutput::is_finished`], which all
    /// call this helper so a stalled worker cannot block the caller.
    fn query<T>(&self, command: impl FnOnce(Sender<Option<T>>) -> Command) -> Option<T> {
        let (reply, replies) = mpsc::channel();
        if self.send(command(reply)).is_err() {
            return None;
        }
        replies.recv_timeout(WORKER_QUERY_TIMEOUT).ok().flatten()
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

    fn set_balance(&self, balance: f32) -> Result<(), AppleMusicError> {
        self.send(Command::SetBalance(balance))
    }

    fn seek_by(&self, offset_secs: i64) -> Result<(), AppleMusicError> {
        self.send(Command::SeekBy(offset_secs))
    }

    fn seek_to(&self, position: Duration) -> Result<(), AppleMusicError> {
        self.send(Command::SeekTo(position))
    }

    fn position(&self) -> Option<Duration> {
        self.query(Command::Position)
    }

    fn duration(&self) -> Option<Duration> {
        self.query(Command::Duration)
    }

    fn playback_format(&self) -> Option<PlaybackFormat> {
        self.query(Command::PlaybackFormat)
    }

    fn spectrum(&self) -> Option<Spectrum> {
        self.query(Command::Spectrum)
    }

    fn is_finished(&self) -> bool {
        self.query(Command::Finished).unwrap_or(false)
    }

    fn set_equalizer(&self, equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
        self.send(Command::SetEqualizer(equalizer))
    }
}

/// Waits up to `timeout` for the worker to report whether the default device
/// opened, classifying the three outcomes the caller must tell apart.
///
/// A worker that reports success or a device-open failure yields that result
/// unchanged. A worker that has not reported within `timeout` is reported as a
/// timed-out open, so [`super::audio_with_fallback`] can stand in with
/// [`super::SilentOutput`] instead of the caller hanging. A worker whose thread ended
/// without reporting (it panicked before sending) is reported distinctly, as
/// before. Kept separate from [`RodioOutput::new`] so the bound is testable
/// without a real, hanging audio device.
pub(super) fn await_device_ready(
    ready: Receiver<Result<(), AppleMusicError>>,
    timeout: Duration,
) -> Result<(), AppleMusicError> {
    match ready.recv_timeout(timeout) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error),
        Err(RecvTimeoutError::Timeout) => Err(AppleMusicError::new(format!(
            "opening the default audio device timed out after {timeout:?}"
        ))),
        Err(RecvTimeoutError::Disconnected) => Err(AppleMusicError::new(
            "the audio thread ended before opening a device",
        )),
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

    serve_commands(
        receiver,
        || RodioPlayer::new(rodio::Player::connect_new(device.mixer())),
        |url| download_and_decode(preview_agent(), url),
    );
}

/// The gain, stereo balance, and equalizer curve a player starts at. The
/// worker keeps one copy across players, so a setting changed while stopped
/// still applies to the next `Play`, and a new player starts at it rather than
/// the backend's defaults.
#[derive(Debug, Clone, Copy)]
struct PlaybackSettings {
    /// The output gain, where `1.0` is full volume.
    volume: f32,
    /// The stereo balance, where `-1.0` is hard left and `1.0` is hard right.
    balance: f32,
    /// The equalizer curve.
    equalizer: EqSnapshot,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            volume: 1.0,
            balance: 0.0,
            equalizer: EqSnapshot::default(),
        }
    }
}

/// Serves [`Command`]s until the sender closes, driving players created by
/// `create_player` and previews decoded by `fetch`.
///
/// `create_player` and `fetch` are injected — rather than the loop opening a
/// device and calling [`download_and_decode`] directly — so its state
/// transitions are testable without an output device (see [`WorkerPlayer`]);
/// production passes `rodio::Player::connect_new` over the opened device and
/// [`download_and_decode`] over [`preview_agent`].
///
/// A failed `Play` download stops and drops the previous player before
/// logging. By the time the worker downloads a selection, `play_track` has
/// already committed it as the current track, so leaving the previous player
/// running would keep the old preview audible under the new track's name. The
/// seam's `play` returned `Ok` when it queued this command, so the caller
/// cannot observe the asynchronous failure and roll the state back; silencing
/// the old player is what keeps the audio consistent with the committed
/// state.
///
/// A download runs on this thread, so any command the caller issues meanwhile
/// waits in the channel. Once a download finishes, [`handle_play`] drains those
/// commands before installing the preview: if a later `Play` or a `Stop` is
/// among them, the just-downloaded selection is already superseded, and
/// installing it would play the old audio under the newer committed track's
/// name. The stale preview is dropped and the queued commands are handled
/// instead.
pub(super) fn serve_commands<P, D>(
    receiver: Receiver<Command>,
    create_player: impl Fn() -> P,
    fetch: impl Fn(&str) -> Result<D, AppleMusicError>,
) where
    P: WorkerPlayer<Preview = D>,
{
    let mut player: Option<P> = None;
    let mut settings = PlaybackSettings::default();
    // Commands that arrived while a download was in flight, handled in order
    // once it finishes. A later `Play` or `Stop` among them supersedes the
    // preview the download produced.
    let mut queued: VecDeque<Command> = VecDeque::new();
    loop {
        let command = match queued.pop_front() {
            Some(command) => command,
            None => match receiver.recv() {
                Ok(command) => command,
                Err(_) => break,
            },
        };
        match command {
            Command::Play(url) => handle_play(
                &url,
                &receiver,
                &mut queued,
                &mut player,
                settings,
                &create_player,
                &fetch,
            ),
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
                settings.volume = new_volume;
                if let Some(player) = &player {
                    player.set_volume(settings.volume);
                }
            }
            Command::SetBalance(new_balance) => {
                settings.balance = new_balance;
                if let Some(player) = &player {
                    player.set_balance(settings.balance);
                }
            }
            Command::SeekBy(offset_secs) => {
                if let Some(player) = &player {
                    player.seek_by(offset_secs);
                }
            }
            Command::SeekTo(position) => {
                if let Some(player) = &player {
                    player.seek_to(position);
                }
            }
            Command::Position(reply) => {
                let _ = reply.send(player.as_ref().map(WorkerPlayer::position));
            }
            Command::Duration(reply) => {
                let _ = reply.send(player.as_ref().and_then(WorkerPlayer::duration));
            }
            Command::PlaybackFormat(reply) => {
                let _ = reply.send(player.as_ref().and_then(WorkerPlayer::playback_format));
            }
            Command::Spectrum(reply) => {
                let _ = reply.send(player.as_ref().and_then(WorkerPlayer::spectrum));
            }
            Command::Finished(reply) => {
                let _ = reply.send(player.as_ref().map(WorkerPlayer::is_finished));
            }
            Command::SetEqualizer(new_equalizer) => {
                settings.equalizer = new_equalizer;
                if let Some(player) = &player {
                    player.set_equalizer(settings.equalizer);
                }
            }
        }
    }
}

/// Handles one [`Command::Play`]: fetch `url`, then install a player for the
/// decoded preview unless a command that arrived during the download
/// superseded the selection.
///
/// The receiver is drained into `queued` first, so the outer loop handles
/// those commands in order; a later `Play` or `Stop` among them means the UI
/// has already committed a newer state, so the just-downloaded preview is
/// dropped rather than played under the newer track's name. A failed fetch
/// stops and drops the previous player (as `Command::Stop` does), so the old
/// preview is not left audible under the new track's name.
fn handle_play<P, D>(
    url: &str,
    receiver: &Receiver<Command>,
    queued: &mut VecDeque<Command>,
    player: &mut Option<P>,
    settings: PlaybackSettings,
    create_player: &impl Fn() -> P,
    fetch: &impl Fn(&str) -> Result<D, AppleMusicError>,
) where
    P: WorkerPlayer<Preview = D>,
{
    let preview = match fetch(url) {
        Ok(preview) => preview,
        Err(error) => {
            // Stop and drop the previous player (as `Command::Stop` does) so a
            // failed new selection is not misreported by the old preview still
            // playing.
            if let Some(player) = player.as_ref() {
                player.stop();
            }
            *player = None;
            eprintln!("audio playback failed for {url:?}: {error}");
            return;
        }
    };

    // Collect every command that arrived while the download ran. If one
    // supersedes this selection, drop the preview and let the queued command
    // be handled by the outer loop; the UI has already committed the newer
    // state, so playing this one would be the wrong audio under the right
    // title.
    while let Ok(command) = receiver.try_recv() {
        queued.push_back(command);
    }
    if queued
        .iter()
        .any(|command| matches!(command, Command::Play(_) | Command::Stop))
    {
        return;
    }

    // A fresh player per track replaces the previous one, so a new selection
    // does not queue behind the old track. It starts at the remembered gain
    // rather than rodio's full-volume default.
    let next = create_player();
    next.set_volume(settings.volume);
    next.set_balance(settings.balance);
    next.set_equalizer(settings.equalizer);
    next.append(preview);
    *player = Some(next);
}
