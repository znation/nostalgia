//! Playback and transport transitions for [`AppleMusicService`].
//!
//! The transport transitions (Pause, Play, Stop, and their shared resume) live
//! in this module; the output controls and backend queries live in [`output`],
//! and selecting a track and its playback log line live in [`play`].

mod output;
pub(super) mod play;

use std::time::Duration;

use super::{AppleMusicError, AppleMusicService};

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
    /// Pauses the audio, or resumes it when the player is already paused with a preview loaded
    /// (Winamp's Pause toggles). Pausing clears the shared playing flag, and sets the shared paused
    /// flag when a preview is loaded, keeping `current_track` set so the Now Playing bar still
    /// names the track. The paused flag is what tells this state apart from a Stop, which clears
    /// both flags.
    ///
    /// The backend is called first: one that cannot accept the request reports
    /// it before shared state claims playback stopped. A pause with no loaded
    /// preview stops the shared transport instead: the backend has nothing to
    /// pause, and claiming `Paused` would report a state Play could never
    /// resume.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the audio backend cannot accept the
    /// request.
    pub(crate) async fn pause(&self) -> Result<(), AppleMusicError> {
        let mut transport = self.transport.lock().await;
        let paused = self.state.lock().await.paused;
        if paused && transport.loaded {
            return self.resume_locked(&mut transport).await;
        }
        self.audio.pause()?;
        let mut state = self.state.lock().await;
        if transport.loaded {
            state.pause();
        } else {
            state.stop();
        }
        drop(state);
        println!("Paused playback");
        Ok(())
    }

    /// Restarts a playing preview from its start (Winamp's Play while
    /// playing), resumes a paused one, or restarts a stopped one from the last
    /// preview URL, sets the shared playing flag, and clears the shared paused
    /// flag. The playing restart is one absolute seek to zero and leaves the
    /// shared state alone. Does nothing when no preview has ever played, so
    /// Play cannot claim playback with nothing to sound.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the audio backend cannot accept the
    /// request.
    pub(crate) async fn resume(&self) -> Result<(), AppleMusicError> {
        let mut transport = self.transport.lock().await;
        let is_playing = self.state.lock().await.is_playing;
        if transport.loaded && is_playing {
            self.seek_output_to(Duration::ZERO)?;
            println!("Restarted playback");
            return Ok(());
        }
        self.resume_locked(&mut transport).await
    }

    /// Stops and discards the audio, clears the shared playing and paused
    /// flags, and keeps `current_track` set so the Now Playing bar still names
    /// the track. The last preview URL is kept so Play can restart it.
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
    /// when not. A track marked playing with no loaded preview (a source that
    /// supplied no playable asset) has nothing to pause, so the toggle stops it
    /// rather than claiming `Paused` for a preview Play could not resume. The
    /// flag read, the backend call, and the flag write all happen under the
    /// transport lock, so two toggles that arrive together cannot both read the
    /// same flag and both pause.
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
            let mut state = self.state.lock().await;
            if transport.loaded {
                state.toggle_playing();
            } else {
                state.stop();
            }
            drop(state);
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
}

/// Transport stubs kept as the seam a real Apple Music implementation will
/// fill: `next_track` and `previous_track` have no production caller — the
/// Previous/Next buttons pick their song through
/// `transport::next_track_id`/`previous_track_id` — so `dead_code` is allowed
/// on exactly this block: a *newly* dead private field or method elsewhere
/// still triggers the compiler's `dead_code` warning.
#[allow(dead_code)]
impl AppleMusicService {
    /// A stub for advancing to the next library track: it prints and leaves
    /// the transport state untouched. Kept as the seam a real Apple Music
    /// implementation will fill.
    pub(super) async fn next_track(&self) -> Result<(), AppleMusicError> {
        // In a real implementation, this would:
        // 1. Get current track position
        // 2. Play next track in library

        println!("Playing next track");
        Ok(())
    }

    /// A stub for stepping back to the previous library track: it prints and
    /// leaves the transport state untouched. Kept as the seam a real Apple
    /// Music implementation will fill.
    pub(super) async fn previous_track(&self) -> Result<(), AppleMusicError> {
        // In a real implementation, this would:
        // 1. Get current track position
        // 2. Play previous track in library

        println!("Playing previous track");
        Ok(())
    }
}
