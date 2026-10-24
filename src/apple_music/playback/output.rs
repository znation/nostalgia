//! The output controls and backend queries for [`AppleMusicService`]: the
//! volume, balance, seek, and equalizer commands, and the loaded preview's
//! position, length, format, spectrum, and finished state.
//!
//! Every method is a synchronous forward to the injected
//! [`crate::audio::AudioOutput`]; the transport transitions that serialize with
//! these controls live in [`super`].

use std::time::Duration;

use crate::audio;

use super::super::{AppleMusicError, AppleMusicService};

impl AppleMusicService {
    /// Forwards `volume` to the audio backend, so the volume slider reaches
    /// the output gain. Synchronous, and touches no shared state: the UI
    /// already stored the clamped value under its own lock, and
    /// [`crate::audio::AudioOutput::set_volume`] is a non-blocking channel
    /// send, so an async transition here would let a slider drag's messages
    /// race the state lock.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the audio backend cannot accept
    /// the request.
    pub(crate) fn set_output_volume(&self, volume: f32) -> Result<(), AppleMusicError> {
        self.audio.set_volume(volume)
    }

    /// Forwards `balance` to the audio backend, so the balance slider reaches
    /// the output's stereo image. Synchronous, and touches no shared state:
    /// the UI already stored the clamped value under its own lock, and
    /// [`crate::audio::AudioOutput::set_balance`] is a non-blocking channel
    /// send, so an async transition here would let a slider drag's messages
    /// race the state lock.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the audio backend cannot accept
    /// the request.
    pub(crate) fn set_output_balance(&self, balance: f32) -> Result<(), AppleMusicError> {
        self.audio.set_balance(balance)
    }

    /// Forwards a relative seek to the audio backend, so the left and right
    /// arrow keys move the playing preview without changing the UI's own
    /// state. Positive offsets seek forward and negative offsets backward.
    /// Synchronous, and touches no shared state: the UI owns the position and
    /// [`crate::audio::AudioOutput::seek_by`] is a non-blocking channel send.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the audio backend cannot accept
    /// the request.
    pub(crate) fn seek_output(&self, offset_secs: i64) -> Result<(), AppleMusicError> {
        self.audio.seek_by(offset_secs)
    }

    /// Reports whether the audio backend still holds a preview that can be
    /// paused, resumed, or seeked — true for a playing or paused preview, and
    /// false after [`stop`](Self::stop) discarded it or before the first play.
    ///
    /// The UI's subscription builder calls this on its main thread, so it never
    /// waits: it takes the transport lock only if that lock is free. While a
    /// transport transition holds the lock it reports `true`, keeping the
    /// position poll subscribed across the transition rather than dropping it
    /// from a stale read; the transition's completion re-evaluates it.
    pub(crate) fn has_loaded_preview(&self) -> bool {
        self.transport
            .try_lock()
            .map_or(true, |transport| transport.loaded)
    }

    /// Reports the loaded preview's playback position, or `None` when no
    /// preview is loaded or the audio backend cannot answer.
    ///
    /// Synchronous and touches no shared state. The backend bounds the wait
    /// (see [`crate::audio::AudioOutput::position`]), so callers on the UI
    /// thread should still run it as a task rather than blocking `update`.
    /// The Now Playing readout polls it once per second.
    pub(crate) fn playback_position(&self) -> Option<Duration> {
        self.audio.position()
    }

    /// Forwards an absolute seek to the audio backend, so releasing the seek
    /// bar moves the playing preview to the dragged position. Synchronous, and
    /// touches no shared state: the UI owns the position and
    /// [`crate::audio::AudioOutput::seek_to`] is a non-blocking channel send.
    /// The backend clamps the target to the loaded preview's length.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the audio backend cannot accept
    /// the request.
    pub(crate) fn seek_output_to(&self, position: Duration) -> Result<(), AppleMusicError> {
        self.audio.seek_to(position)
    }

    /// Reports the loaded preview's own length, or `None` when no preview is
    /// loaded or the audio backend cannot answer.
    ///
    /// Synchronous and touches no shared state. The backend bounds the wait
    /// (see [`crate::audio::AudioOutput::duration`]), so callers on the UI
    /// thread should still run it as a task rather than blocking `update`.
    /// The Now Playing seek bar polls it beside the position.
    pub(crate) fn playback_duration(&self) -> Option<Duration> {
        self.audio.duration()
    }

    /// Reports the loaded preview's sample rate and channel mode in the
    /// backend's own format, or `None` when no preview is loaded or the
    /// audio backend cannot answer.
    ///
    /// Synchronous and touches no shared state. The backend bounds the wait
    /// (see [`crate::audio::AudioOutput::playback_format`]), so callers on
    /// the UI thread should still run it as a task rather than blocking
    /// `update`. The Now Playing LCD polls it beside the position.
    pub(crate) fn playback_format(&self) -> Option<audio::PlaybackFormat> {
        self.audio.playback_format()
    }

    /// Reports the loaded preview's live ten-band spectrum, or `None` when no
    /// preview is loaded or the audio backend cannot answer.
    ///
    /// Synchronous and touches no shared state. The backend bounds the wait
    /// (see [`crate::audio::AudioOutput::spectrum`]), so callers on the UI
    /// thread should still run it as a task rather than blocking `update`.
    /// The Now Playing LCD polls it while playback runs.
    pub(crate) fn playback_spectrum(&self) -> Option<audio::Spectrum> {
        self.audio.spectrum()
    }

    /// Reports whether the loaded preview has played to its end.
    ///
    /// Synchronous and touches no shared state: it forwards the backend's own
    /// finished state (see [`crate::audio::AudioOutput::is_finished`]). The
    /// backend bounds the wait, so callers on the UI thread should run it as a
    /// task rather than blocking `update`. The position poll consumes it to
    /// advance the transport when a preview reaches its end.
    pub(crate) fn preview_finished(&self) -> bool {
        self.audio.is_finished()
    }

    /// Forwards `equalizer` to the audio backend, so a preview starts at the
    /// curve the equalizer panel shows. Synchronous, and touches no shared
    /// state: the UI already stored the clamped curve under its own lock, and
    /// [`crate::audio::AudioOutput::set_equalizer`] is a non-blocking channel
    /// send, so an async transition here would let a slider drag's messages
    /// race the state lock.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the audio backend cannot accept
    /// the request.
    pub(crate) fn set_output_equalizer(
        &self,
        equalizer: audio::EqSnapshot,
    ) -> Result<(), AppleMusicError> {
        self.audio.set_equalizer(equalizer)
    }
}
