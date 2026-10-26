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
//! the worker (and stops the previous player, so a failed new selection does
//! not leave the old track audible) rather than surfacing as a caller error.
//! A selection a newer `Play` or a `Stop` superseded while it was downloading
//! is dropped rather than installed, so the worker cannot start the old
//! preview after the UI has committed a newer track. Each download is bounded
//! by [`preview::PREVIEW_TIMEOUT`], so a stalled server cannot block the worker — and
//! with it every later play, pause, stop, and volume command — forever. Opening
//! the device is bounded by [`player::DEVICE_OPEN_TIMEOUT`],
//! so a wedged platform audio daemon cannot stall the caller that builds the
//! service at startup.

use std::sync::Arc;
use std::time::Duration;

use crate::music_error::AppleMusicError;

mod balance;
mod delegate;
mod equalizer;
mod meter;
mod player;
mod preview;

pub use equalizer::EqSnapshot;
pub use meter::Spectrum;
pub use player::RodioOutput;

/// The format of the loaded preview's audio, as the backend's own source
/// reports it.
///
/// This is presentation data for the Now Playing LCD's status row: it names
/// the sample rate, channel layout, and average bitrate the decoded preview
/// carries, and says nothing about playback state or the output device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackFormat {
    /// The preview's sample rate in hertz, as the source reports it (for
    /// example `44_100`).
    pub sample_rate_hz: u32,
    /// The number of audio channels in the preview: `1` is mono, `2` is
    /// stereo.
    pub channels: u16,
    /// The preview's average encoded bitrate in kilobits per second, derived
    /// from the downloaded body and the decoded source's own length. `None`
    /// when either is unknown, so the status row omits it.
    pub bitrate_kbps: Option<u32>,
}

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

    /// Sets the stereo balance, where `-1.0` is hard left, `0.0` is centered,
    /// and `1.0` is hard right.
    ///
    /// Applies to the current player, if one is loaded, and is remembered so
    /// the next [`play`](Self::play) starts at the same balance. This is how
    /// the balance slider reaches a preview, and how a preview starts at the
    /// slider's value rather than centered.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the backend cannot accept the
    /// request (see [`AudioOutput::play`]).
    fn set_balance(&self, balance: f32) -> Result<(), AppleMusicError>;

    /// Seeks the current player by `offset_secs` relative to its position.
    ///
    /// Positive offsets move forward and negative offsets move backward; a
    /// backward seek past the start lands at zero. Applies only to the
    /// current player, if one is loaded: a seek while stopped does nothing,
    /// and the offset is not remembered for the next [`play`](Self::play).
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the backend cannot accept the
    /// request (see [`AudioOutput::play`]).
    fn seek_by(&self, offset_secs: i64) -> Result<(), AppleMusicError>;

    /// Seeks the current player to `position`.
    ///
    /// An absolute seek, unlike [`seek_by`](Self::seek_by): the backend clamps
    /// the target to the loaded preview's own length, so a position past the
    /// end lands at the end. Applies only to the current player, if one is
    /// loaded: a seek while stopped does nothing, and the position is not
    /// remembered for the next [`play`](Self::play).
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the backend cannot accept the
    /// request (see [`AudioOutput::play`]).
    fn seek_to(&self, position: Duration) -> Result<(), AppleMusicError>;

    /// Reports the loaded preview's own length.
    ///
    /// This is the preview asset's length, not the catalog track's: a preview
    /// can be shorter than the song, so the seek bar measures this value while
    /// the Now Playing LCD's right edge keeps the catalog total. Returns
    /// `None` when no player is loaded or the backend cannot answer (the
    /// silent fallback, a test fake, or a query that waits past the backend's
    /// bound). The default keeps every existing implementor compiling without
    /// a length of its own.
    fn duration(&self) -> Option<Duration> {
        None
    }

    /// Reports the loaded preview's playback position.
    ///
    /// Returns `Some` while a player is loaded — including [`Duration::ZERO`]
    /// at the start of its preview — and `None` when no player is loaded or
    /// the backend cannot answer (the silent fallback, a test fake, or a query
    /// that waits past the backend's bound). The default keeps every existing
    /// implementor compiling without a position of its own.
    fn position(&self) -> Option<Duration> {
        None
    }

    /// Reports the loaded preview's sample rate and channel mode.
    ///
    /// Returns `Some` while a player is loaded and its source reports a
    /// format, and `None` when no player is loaded (including after a Stop)
    /// or the backend cannot answer (the silent fallback, a test fake without
    /// a format, or a query that waits past the backend's bound). The default
    /// keeps every existing implementor compiling without a format of its
    /// own.
    fn playback_format(&self) -> Option<PlaybackFormat> {
        None
    }

    /// Reports the loaded preview's live ten-band spectrum.
    ///
    /// Returns `Some` while a player is loaded, with one normalized amplitude
    /// per [`crate::equalizer::BAND_HZ`] band computed from the samples the
    /// worker pulled most recently, and `None` when no player is loaded
    /// (including after a Stop) or the backend cannot answer (the silent
    /// fallback, a test fake without a spectrum, or a query that waits past
    /// the backend's bound). The default keeps every existing implementor
    /// compiling without a spectrum of its own.
    fn spectrum(&self) -> Option<Spectrum> {
        None
    }

    /// Reports whether the loaded preview has played to its end.
    ///
    /// Returns true when a loaded preview's source has played to the end, and
    /// also (in the worker) for a selection whose download just failed, so the
    /// transport moves on; that stops after a few failures in a row. Returns
    /// false when no player is loaded (including after a Stop), while a paused preview is still mid-asset, for the silent
    /// fallback, and when the backend cannot answer within its bound. The
    /// default keeps every existing implementor compiling without a finished
    /// state of its own.
    fn is_finished(&self) -> bool {
        false
    }

    /// Sets the equalizer curve applied to the output.
    ///
    /// Applies to the current player, if one is loaded, and is remembered so
    /// the next [`play`](Self::play) starts at the same curve. A disabled
    /// snapshot passes samples through unchanged.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the backend cannot accept the
    /// request (see [`AudioOutput::play`]).
    fn set_equalizer(&self, equalizer: EqSnapshot) -> Result<(), AppleMusicError>;
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

    fn set_balance(&self, balance: f32) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; balance set to {balance}");
        Ok(())
    }

    fn seek_by(&self, offset_secs: i64) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; nothing to seek by {offset_secs}s");
        Ok(())
    }

    fn seek_to(&self, position: Duration) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; nothing to seek to {position:?}");
        Ok(())
    }

    fn set_equalizer(&self, equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; equalizer set to {equalizer:?}");
        Ok(())
    }
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
mod tests;
