//! Selecting a track: [`AppleMusicService::play_track`] and the escaped
//! playback log line it writes.
//!
//! The transport transitions that this play serializes with live in [`super`].

use super::super::validation::{IdKind, ensure_id_is_valid, ensure_preview_url_is_valid};
use super::super::{AppleMusicError, AppleMusicService};

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
    /// [`preview_url_problem`](super::super::validation::preview_url_problem)); the
    /// audio backend hands it to a network fetch, so a cleartext URL, an
    /// internal address literal, a numeric host, or a value carrying a control
    /// character is rejected before shared state is touched.
    ///
    /// When the audio backend rejects the preview, shared state must not keep
    /// claiming playback: [`crate::state::AppState::stop`] runs before the
    /// error is returned, clearing the playing and paused flags and leaving
    /// `current_track` set as that method leaves it, so the Now Playing bar
    /// still names the track the user selected. The rollback is itself guarded
    /// by `is_current` (see below), so a play a newer one has superseded never
    /// clears the newer play's flag.
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
        state.play();
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
                state.stop();
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
