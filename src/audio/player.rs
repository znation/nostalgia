//! The `rodio`-backed output seam: the [`AudioOutput`] facade and its bounded
//! worker queries.
//!
//! The default output device and the current player live on a worker thread;
//! [`RodioOutput`] only holds the channel that reaches it, so it is
//! `Send + Sync` and cheap to clone behind an `Arc`. [`worker`] owns that
//! thread: its command protocol, startup readiness wait, and the loop that
//! drives one player. [`rodio_player`] holds the production
//! [`WorkerPlayer`](worker::WorkerPlayer), [`mod@format`] reads a decoded
//! source's format and length, and [`mod@seek`] holds the seek arithmetic
//! those paths share.

use std::sync::Mutex;
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

use crate::music_error::AppleMusicError;

use super::equalizer::EqSnapshot;
use super::{AudioOutput, PlaybackFormat, Spectrum};

pub(super) mod format;
pub(super) mod rodio_player;
pub(super) mod seek;
pub(super) mod worker;

use worker::{Command, await_device_ready, run_worker};

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
