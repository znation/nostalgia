//! Playback and transport transitions for [`AppleMusicService`].

use std::time::Duration;

use super::validation::{IdKind, ensure_id_is_valid, ensure_preview_url_is_valid};
use super::{AppleMusicError, AppleMusicService};
use crate::audio;

/// The service's view of the audio backend: whether a preview is loaded and
/// paused (ready for [`crate::audio::AudioOutput::resume`]), and the last
/// preview URL handed to [`crate::audio::AudioOutput::play`] so a stopped
/// preview can be restarted. `last_url` is kept after a stop; `loaded` is
/// cleared because the backend discards the player then.
pub(super) struct TransportState {
    /// A preview is loaded and ready to resume.
    pub(super) loaded: bool,
    /// The most recent preview URL, or `None` when no preview has played.
    pub(super) last_url: Option<String>,
}

/// The transport transitions the UI drives. Each is `pub(crate)` so the `ui`
/// Play/Pause, Play, Pause, and Stop arms call one method each, and each
/// reuses the matching [`crate::state::AppState`] mutator so none of those goes dead.
impl AppleMusicService {
    /// Pauses the audio and clears the shared playing flag, keeping
    /// `current_track` set so the Now Playing bar still names the track.
    ///
    /// The backend is called first: one that cannot accept the request reports
    /// it before shared state claims playback stopped.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the audio backend cannot accept the
    /// request.
    pub(crate) async fn pause(&self) -> Result<(), AppleMusicError> {
        let _transport = self.transport.lock().await;
        self.audio.pause()?;
        self.state.lock().await.pause();
        println!("Paused playback");
        Ok(())
    }

    /// Resumes a paused preview, or restarts a stopped one from the last
    /// preview URL, and sets the shared playing flag. Does nothing when no
    /// preview has ever played, so Play cannot claim playback with nothing to
    /// sound.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the audio backend cannot accept the
    /// request.
    pub(crate) async fn resume(&self) -> Result<(), AppleMusicError> {
        let mut transport = self.transport.lock().await;
        self.resume_locked(&mut transport).await
    }

    /// Stops and discards the audio and clears the shared playing flag,
    /// keeping `current_track` set so the Now Playing bar still names the
    /// track. The last preview URL is kept so Play can restart it.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the audio backend cannot accept the
    /// request.
    pub(crate) async fn stop(&self) -> Result<(), AppleMusicError> {
        let mut transport = self.transport.lock().await;
        self.audio.stop()?;
        transport.loaded = false;
        self.state.lock().await.stop();
        println!("Stopped playback");
        Ok(())
    }

    /// Flips the transport: pauses when playing, and resumes (or restarts)
    /// when not. The flag read, the backend call, and the flag write all
    /// happen under the transport lock, so two toggles that arrive together
    /// cannot both read the same flag and both pause.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the audio backend cannot accept the
    /// request.
    pub(crate) async fn toggle_play_pause(&self) -> Result<(), AppleMusicError> {
        let mut transport = self.transport.lock().await;
        let playing = self.state.lock().await.is_playing;
        if playing {
            self.audio.pause()?;
            self.state.lock().await.toggle_playing();
            println!("Paused playback");
            Ok(())
        } else {
            self.resume_locked(&mut transport).await
        }
    }

    /// The shared body of [`resume`](Self::resume) and the resume half of
    /// [`toggle_play_pause`](Self::toggle_play_pause), run with the transport
    /// lock already held. Resumes a paused player; when the backend has
    /// nothing loaded but a preview URL is remembered, replays it at the
    /// current equalizer curve (a stop discards the player, so Play after Stop
    /// restarts rather than resumes); when neither is true, leaves the flag
    /// alone.
    async fn resume_locked(&self, transport: &mut TransportState) -> Result<(), AppleMusicError> {
        if transport.loaded {
            self.audio.resume()?;
        } else if let Some(url) = transport.last_url.clone() {
            // The restart builds a fresh player from the backend's remembered
            // curve, so forward the panel's current curve first rather than
            // replaying at whatever was set when the previous preview began.
            let equalizer = {
                let state = self.state.lock().await;
                state.eq_snapshot()
            };
            self.set_output_equalizer(equalizer)?;
            self.audio.play(&url)?;
            transport.loaded = true;
        } else {
            // No preview has ever played, so there is nothing to sound; Play
            // leaves the transport stopped rather than claiming playback.
            println!("Nothing to resume");
            return Ok(());
        }
        self.state.lock().await.play();
        println!("Resumed playback");
        Ok(())
    }

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

/// Transport stubs kept as the seam a real Apple Music implementation will
/// fill: `next_track` and `previous_track` have no production caller — the
/// Previous/Next buttons pick their song through
/// `transport::next_track_id`/`previous_track_id` — so `dead_code` is allowed
/// on exactly this block: a *newly* dead private field or method elsewhere
/// still triggers the compiler's `dead_code` warning.
#[allow(dead_code)]
impl AppleMusicService {
    pub(super) async fn next_track(&self) -> Result<(), AppleMusicError> {
        // In a real implementation, this would:
        // 1. Get current track position
        // 2. Play next track in library

        println!("Playing next track");
        Ok(())
    }

    pub(super) async fn previous_track(&self) -> Result<(), AppleMusicError> {
        // In a real implementation, this would:
        // 1. Get current track position
        // 2. Play previous track in library

        println!("Playing previous track");
        Ok(())
    }
}

impl AppleMusicService {
    /// Plays the given track by id, recording it as the current track and
    /// marking it playing, then applies the state's current volume and balance
    /// to the injected [`crate::audio::AudioOutput`] and starts `preview_url`
    /// through it, so the first play uses the sliders' values rather than
    /// rodio's full-volume and centered defaults. A blank `track_id` — empty or only
    /// whitespace — or one carrying a terminal control character is rejected
    /// with an [`AppleMusicError`] and leaves shared state untouched: a blank
    /// id can never name a track, and a control-character id must not reach
    /// shared state. The playback log below escapes the id with `Debug` (see
    /// [`play_log_line`]), so even a non-control Unicode format character the
    /// guard does not reject cannot reach the terminal raw. A `None`
    /// `preview_url` is a track with no playable asset: the state transition
    /// is committed, any previously loaded preview is stopped so it neither
    /// sounds nor keeps reporting its position under the new track, and no new
    /// audio starts. A `preview_url` that is `Some` but
    /// blank is a caller bug — no asset has a blank URL — and is rejected the
    /// same way a blank id is, before shared state is touched. A non-blank
    /// `preview_url` must be an `https` URL whose host is not an internal
    /// address literal or a legacy numeric address form (see
    /// [`preview_url_problem`](super::validation::preview_url_problem)); the audio backend hands it to a network
    /// fetch, so a cleartext URL, an internal address literal, a numeric
    /// host, or a value carrying a control character is rejected before
    /// shared state is touched.
    ///
    /// When the audio backend rejects the preview, shared state must not keep
    /// claiming playback: `is_playing` is cleared before the error is
    /// returned, leaving `current_track` set as [`crate::state::AppState::stop`]
    /// leaves it, so the Now Playing bar still names the track the user
    /// selected. The rollback is itself guarded by `is_current` (see below),
    /// so a play a newer one has superseded never clears the newer play's
    /// flag.
    ///
    /// `is_current` guards the commit: it is evaluated while the state lock is
    /// held, and when it returns `false` the track is left uncommitted — and
    /// no audio starts — because a newer play has superseded this one. A slow
    /// backend can complete two plays out of order, and without the guard the
    /// older reply would overwrite the newer track in shared state; the UI
    /// passes the guard from its playback-request counter, exactly as the
    /// browse path does. The same guard is consulted again under the lock
    /// before an audio failure rolls the flag back, because a newer play may
    /// have committed in the window between the commit and the backend call.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when `track_id` is blank (empty or only
    /// whitespace), carries a control character, when `preview_url` is present
    /// but blank or is not an `https` URL with an acceptable host, or the audio
    /// backend cannot start the preview or stop the previous one.
    pub async fn play_track(
        &self,
        track_id: &str,
        preview_url: Option<&str>,
        is_current: impl Fn() -> bool,
    ) -> Result<(), AppleMusicError> {
        ensure_id_is_valid(track_id, IdKind::Track)?;
        ensure_preview_url_is_valid(preview_url)?;

        // Serialize with the transport transitions, so a Stop cannot land
        // between this play's state commit and its backend call.
        let mut transport = self.transport.lock().await;

        let mut state = self.state.lock().await;
        if !is_current() {
            return Ok(());
        }
        state.current_track = Some(track_id.to_string());
        state.is_playing = true;
        // The gain, balance, and equalizer curve to start this preview at,
        // read before the lock drops. A preview starts at the sliders' values
        // and the panel's curve rather than rodio's defaults.
        let volume = state.volume();
        let balance = state.balance();
        let equalizer = state.eq_snapshot();
        drop(state);

        println!("{}", play_log_line(track_id));

        // Remember the URL so a later Play can restart this preview after a
        // Stop, and record whether the backend now holds it. A selection with
        // no preview stops the previous player: otherwise the old preview
        // would keep sounding and its position clock would keep reporting
        // under the new track's name.
        transport.last_url = preview_url.map(str::to_string);
        transport.loaded = false;
        let started = match preview_url {
            Some(url) => self
                .audio
                .set_volume(volume)
                .and_then(|()| self.audio.set_balance(balance))
                .and_then(|()| self.set_output_equalizer(equalizer))
                .and_then(|()| self.audio.play(url)),
            None => self.audio.stop(),
        };
        if let Err(error) = started {
            // The backend refused the preview, so shared state must not keep
            // claiming playback. Roll the flag back only when no newer play
            // has replaced this one: a newer play owns the state now, and
            // clearing it here would make that track look stopped.
            // `current_track` is left in place, as `stop` leaves it, so the
            // bar still names the track the user selected.
            let mut state = self.state.lock().await;
            if is_current() {
                state.is_playing = false;
            }
            transport.last_url = None;
            return Err(error);
        }
        transport.loaded = preview_url.is_some();
        Ok(())
    }
}

/// The playback log line for `track_id`, escaped so a hostile library id
/// cannot inject terminal output.
///
/// `play_track` writes this line to stdout. The id is formatted with `Debug`,
/// not `Display`: `Display` passes every byte through raw, so an id carrying a
/// control character (`\u{1b}`) or a Unicode format character — the
/// right-to-left override `\u{202e}`, say, which `char::is_control` does not
/// classify as a control — would reach the terminal verbatim, while `Debug`
/// renders both as `\u{...}` escapes. This matches the failure report, which
/// already quotes the id with `{track_id:?}`. Kept pure so the escaping
/// contract is testable without capturing stdout.
pub(crate) fn play_log_line(track_id: &str) -> String {
    format!("Playing track: {track_id:?}")
}
