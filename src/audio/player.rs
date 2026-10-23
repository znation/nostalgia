//! The `rodio`-backed output: its worker thread, command channel, and the
//! player the worker drives.

use std::collections::VecDeque;
use std::io::Cursor;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use rodio::source::SeekError;

use crate::music_error::AppleMusicError;

use super::balance::BalanceSource;
use super::equalizer::{EqSettings, EqSnapshot, EqSource};
use super::preview::{download_and_decode, preview_agent};
use super::{AudioOutput, PlaybackFormat};

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

/// The longest [`RodioOutput::position`] and [`RodioOutput::playback_format`]
/// wait for the worker to answer.
///
/// The worker handles commands in channel order, so a query issued while the
/// worker is downloading a preview is answered only after that download
/// installs the new player. A download can hold the worker for up to
/// [`super::preview::PREVIEW_TIMEOUT`], far longer than a UI poll should
/// block, so this bound is short: a query that misses it reports `None`
/// rather than stalling the caller, and the next poll asks again. It bounds
/// both queued backend queries — the position and the format — because the
/// same download sits in front of either one.
pub(super) const POSITION_QUERY_TIMEOUT: Duration = Duration::from_millis(100);

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
    /// Report the current player's position on `reply` (`None` when none is
    /// loaded).
    Position(Sender<Option<Duration>>),
    /// Report the loaded preview's sample rate and channel mode on `reply`
    /// (`None` when none is loaded).
    PlaybackFormat(Sender<Option<PlaybackFormat>>),
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
    /// Reports the player's current position.
    fn position(&self) -> Duration;
    /// Reports the loaded preview's format, when the player has one. The
    /// default answers `None`, so a fake without a format compiles.
    fn playback_format(&self) -> Option<PlaybackFormat> {
        None
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
/// the format the Now Playing LCD's status row shows.
pub(super) fn format_for_source(source: &impl rodio::Source) -> PlaybackFormat {
    PlaybackFormat {
        sample_rate_hz: source.sample_rate().get(),
        channels: source.channels().get(),
    }
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
struct RodioPlayer {
    player: rodio::Player,
    balance: Arc<AtomicU32>,
    equalizer: Arc<EqSettings>,
    /// The format of the preview appended to this player, set once because
    /// each played track builds a fresh player.
    format: OnceLock<PlaybackFormat>,
}

impl RodioPlayer {
    fn new(player: rodio::Player) -> Self {
        Self {
            player,
            balance: Arc::new(AtomicU32::new(0.0_f32.to_bits())),
            equalizer: Arc::new(EqSettings::default()),
            format: OnceLock::new(),
        }
    }
}

impl WorkerPlayer for RodioPlayer {
    type Preview = rodio::Decoder<Cursor<Vec<u8>>>;

    fn set_volume(&self, volume: f32) {
        self.player.set_volume(volume);
    }

    fn set_balance(&self, balance: f32) {
        self.balance.store(balance.to_bits(), Ordering::Relaxed);
    }

    fn seek_by(&self, offset_secs: i64) {
        seek_player(&self.player, offset_secs);
    }

    fn position(&self) -> Duration {
        self.player.get_pos()
    }

    fn playback_format(&self) -> Option<PlaybackFormat> {
        self.format.get().copied()
    }

    fn set_equalizer(&self, equalizer: EqSnapshot) {
        self.equalizer.store(equalizer);
    }

    fn append(&self, preview: Self::Preview) {
        // Read the format from the decoded source before moving it into the
        // adapter chain; a fresh player per track sets the cell once.
        let _ = self.format.set(format_for_source(&preview));
        let equalized = EqSource::new(preview, Arc::clone(&self.equalizer));
        self.player
            .append(BalanceSource::new(equalized, Arc::clone(&self.balance)));
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

    fn position(&self) -> Option<Duration> {
        let (reply, replies) = mpsc::channel();
        if self.send(Command::Position(reply)).is_err() {
            return None;
        }
        replies.recv_timeout(POSITION_QUERY_TIMEOUT).ok().flatten()
    }

    fn playback_format(&self) -> Option<PlaybackFormat> {
        let (reply, replies) = mpsc::channel();
        if self.send(Command::PlaybackFormat(reply)).is_err() {
            return None;
        }
        replies.recv_timeout(POSITION_QUERY_TIMEOUT).ok().flatten()
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
            Command::Position(reply) => {
                let _ = reply.send(player.as_ref().map(WorkerPlayer::position));
            }
            Command::PlaybackFormat(reply) => {
                let _ = reply.send(player.as_ref().and_then(WorkerPlayer::playback_format));
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
