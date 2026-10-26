//! The audio worker thread: its command protocol, device-open readiness, and
//! the loop that drives one player.
//!
//! [`Command`] is the seam between [`RodioOutput`](super::RodioOutput) and the
//! thread that owns the device. [`serve_commands`] is generic over the
//! [`WorkerPlayer`] it drives and the previews it fetches, so its state
//! transitions are testable without an output device; [`run_worker`] supplies
//! the production `RodioPlayer` and the guarded preview download.

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use crate::music_error::AppleMusicError;

use super::super::preview::{PreviewCache, download_and_decode_cached, preview_agent};
use super::super::{EqSnapshot, PlaybackFormat, Spectrum};
use super::rodio_player::RodioPlayer;

/// How many `Play` downloads in a row may fail before a finished query stops
/// reporting the missing player as ended.
const MAX_CONSECUTIVE_FETCH_FAILURES: u32 = 5;

/// A command sent from [`RodioOutput`](super::RodioOutput) to its worker thread.
pub(in crate::audio) enum Command {
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
pub(in crate::audio) trait WorkerPlayer {
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

/// Waits up to `timeout` for the worker to report whether the default device
/// opened, classifying the three outcomes the caller must tell apart.
///
/// A worker that reports success or a device-open failure yields that result
/// unchanged. A worker that has not reported within `timeout` is reported as a
/// timed-out open, so [`super::super::audio_with_fallback`] can stand in with
/// [`super::super::SilentOutput`] instead of the caller hanging. A worker whose
/// thread ended without reporting (it panicked before sending) is reported
/// distinctly, as before. Kept separate from
/// [`RodioOutput::new`](super::RodioOutput::new) so the bound is testable
/// without a real, hanging audio device.
pub(in crate::audio) fn await_device_ready(
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
/// [`RodioOutput::new`](super::RodioOutput::new) can report a device failure to
/// its caller. Every later failure — a download, a decode — is logged here,
/// because the caller is no longer waiting on this thread.
pub(in crate::audio) fn run_worker(
    receiver: Receiver<Command>,
    ready: Sender<Result<(), AppleMusicError>>,
) {
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

    let cache = PreviewCache::default();
    serve_commands(
        receiver,
        || RodioPlayer::new(rodio::Player::connect_new(device.mixer())),
        |url| download_and_decode_cached(&cache, preview_agent(), url),
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

/// Runs `act` against the installed player, if one exists.
///
/// Commands that only change the live player are no-ops before the first
/// preview has loaded, since no player has been created yet.
fn with_player<P>(player: &Option<P>, act: impl FnOnce(&P)) {
    if let Some(player) = player {
        act(player);
    }
}

/// Serves [`Command`]s until the sender closes, driving players created by
/// `create_player` and previews decoded by `fetch`.
///
/// `create_player` and `fetch` are injected — rather than the loop opening a
/// device and calling the guarded download directly — so its state
/// transitions are testable without an output device (see [`WorkerPlayer`]);
/// production passes `rodio::Player::connect_new` over the opened device and
/// [`download_and_decode_cached`] over [`preview_agent`].
///
/// A failed `Play` download stops and drops the previous player before
/// logging. By the time the worker downloads a selection, `play_track` has
/// already committed it as the current track, so leaving the previous player
/// running would keep the old preview audible under the new track's name. The
/// seam's `play` returned `Ok` when it queued this command, so the caller
/// cannot observe the asynchronous failure and roll the state back; silencing
/// the old player is what keeps the audio consistent with the committed
/// state. The failure is still observable afterwards: until the next `Play`
/// or `Stop`, a finished query answers `Some(true)` with no player installed,
/// so the UI's end-of-preview poll moves on. After
/// [`MAX_CONSECUTIVE_FETCH_FAILURES`] failures in a row it answers `None`
/// again, so Repeat cannot keep re-downloading unreachable previews.
///
/// A download runs on this thread, so any command the caller issues meanwhile
/// waits in the channel. Once a download finishes, [`handle_play`] drains those
/// commands before installing the preview: if a later `Play` or a `Stop` is
/// among them, the just-downloaded selection is already superseded, and
/// installing it would play the old audio under the newer committed track's
/// name. The stale preview is dropped and the queued commands are handled
/// instead.
pub(in crate::audio) fn serve_commands<P, D>(
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
    // Consecutive `Play` downloads that failed, each leaving no player for a
    // selection the UI already shows as playing. Reset by a `Play` that does
    // not fail or a `Stop`.
    let mut failed_fetches: u32 = 0;
    loop {
        let command = match queued.pop_front() {
            Some(command) => command,
            None => match receiver.recv() {
                Ok(command) => command,
                Err(_) => break,
            },
        };
        match command {
            Command::Play(url) => {
                let failed = handle_play(
                    &url,
                    &receiver,
                    &mut queued,
                    &mut player,
                    settings,
                    &create_player,
                    &fetch,
                );
                failed_fetches = if failed { failed_fetches + 1 } else { 0 };
            }
            Command::Pause => with_player(&player, WorkerPlayer::pause),
            Command::Resume => with_player(&player, WorkerPlayer::play),
            Command::Stop => {
                with_player(&player, WorkerPlayer::stop);
                player = None;
                failed_fetches = 0;
            }
            Command::SetVolume(new_volume) => {
                settings.volume = new_volume;
                with_player(&player, |player| player.set_volume(settings.volume));
            }
            Command::SetBalance(new_balance) => {
                settings.balance = new_balance;
                with_player(&player, |player| player.set_balance(settings.balance));
            }
            Command::SeekBy(offset_secs) => {
                with_player(&player, |player| player.seek_by(offset_secs));
            }
            Command::SeekTo(position) => {
                with_player(&player, |player| player.seek_to(position));
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
                // A selection whose download failed has no player, but the UI
                // committed it as playing; report it as ended so the UI moves
                // on instead of showing a track that will never advance. Past
                // MAX_CONSECUTIVE_FETCH_FAILURES it stops, so Repeat cannot
                // cycle a list of unreachable previews forever.
                let finished = match player.as_ref() {
                    Some(player) => Some(player.is_finished()),
                    None => (1..=MAX_CONSECUTIVE_FETCH_FAILURES)
                        .contains(&failed_fetches)
                        .then_some(true),
                };
                let _ = reply.send(finished);
            }
            Command::SetEqualizer(new_equalizer) => {
                settings.equalizer = new_equalizer;
                with_player(&player, |player| player.set_equalizer(settings.equalizer));
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
/// preview is not left audible under the new track's name. Returns whether the
/// fetch failed.
fn handle_play<P, D>(
    url: &str,
    receiver: &Receiver<Command>,
    queued: &mut VecDeque<Command>,
    player: &mut Option<P>,
    settings: PlaybackSettings,
    create_player: &impl Fn() -> P,
    fetch: &impl Fn(&str) -> Result<D, AppleMusicError>,
) -> bool
where
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
            return true;
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
        return false;
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
    false
}
