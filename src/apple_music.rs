//! The seam to the Apple Music API.
//!
//! This is the narrow integration point a real Apple Music backend will
//! replace. Until it lands, `AppleMusicService` answers every browse query
//! from the shared in-memory sample library
//! ([`crate::sample_library::sample_library`]) and stubs playback as
//! shared-state transitions: `play_track` records the selected track and
//! marks it playing, `pause` clears the flag. The still-unimplemented stubs
//! (`next_track`, `previous_track`, and the `token` field with its
//! `AppleMusicToken` type) stay so a real implementation has a surface to
//! land on.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::library::{Album, Artist, Song};
use crate::sample_library::sample_library;
use crate::state::AppState;

/// A failed music-library or playback operation.
///
/// The stub produces one only when handed an invalid id: `play_track` rejects
/// a blank or control-character track id, and `get_albums_by_artist` /
/// `get_songs_from_album` reject a blank or control-character artist/album id
/// (an id is blank when it is empty or only whitespace). Every other in-memory
/// query and playback transition succeeds.
/// The seam carries this type so a real Apple Music
/// backend can report a failure without tying the seam's public API to a
/// specific HTTP client. The message is the human-readable cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleMusicError(String);

impl std::fmt::Display for AppleMusicError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AppleMusicError {}

// The seam's error constructor is reached by `play_track` and the two browse
// queries through `ensure_id_is_valid`, and stays public for a real Apple
// Music backend that will report failures from another module, so it is live
// and needs no `dead_code` allowance. The transport stubs and token field
// below still carry theirs; a *newly* dead private item elsewhere still
// triggers the compiler's `dead_code` warning. (A `pub` item in a `pub mod` is
// reachable from the crate root and so is never reported as dead.)
impl AppleMusicError {
    /// Builds a failure whose [`Display`](std::fmt::Display) output is
    /// `message` — the human-readable cause. The wrapped message is private,
    /// so this constructor (rather than a struct literal) is how a real Apple
    /// Music backend outside this module reports a failure through the seam.
    /// `impl Into<String>` accepts both `&str` and `String`.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct AppleMusicToken {
    access_token: String,
    expires_in: u64,
    refresh_token: String,
}

/// The music-library service. Until the real Apple Music API lands, every
/// browse query is answered from the shared [`crate::sample_library::sample_library`],
/// so the UI and its tests agree on the same stub data.
#[derive(Clone)]
pub struct AppleMusicService {
    // Unused until a real Apple Music API sets it during authentication.
    #[allow(dead_code)]
    token: Option<AppleMusicToken>,
    state: Arc<Mutex<AppState>>,
}

/// Transport stubs kept as the seam a real Apple Music implementation will
/// fill: pause, next, and previous are not yet wired to the UI (the
/// transport.rs stepping helpers drive those buttons), so `dead_code` is
/// allowed on exactly this block and the `token` field — a *newly* dead
/// private field or method elsewhere still triggers the compiler's
/// `dead_code` warning.
#[allow(dead_code)]
impl AppleMusicService {
    async fn pause(&self) -> Result<(), AppleMusicError> {
        // As with `play_track`, the stub only owns the shared-state
        // transition — clear the playing flag. A real implementation
        // would add the API call that pauses audio.

        let mut state = self.state.lock().await;
        state.is_playing = false;

        println!("Paused playback");
        Ok(())
    }

    async fn next_track(&self) -> Result<(), AppleMusicError> {
        // In a real implementation, this would:
        // 1. Get current track position
        // 2. Play next track in library

        println!("Playing next track");
        Ok(())
    }

    async fn previous_track(&self) -> Result<(), AppleMusicError> {
        // In a real implementation, this would:
        // 1. Get current track position
        // 2. Play previous track in library

        println!("Playing previous track");
        Ok(())
    }
}

/// Initializes the service.
///
/// In a real implementation, this would:
/// 1. Authenticate with Apple Music
/// 2. Get user's library
/// 3. Set up event listeners
///
/// The stub only records that the service is ready; `_state` is kept as the
/// handle a real implementation will use.
pub fn init_service(_state: Arc<Mutex<AppState>>) {
    println!("Apple Music service initialized");
}

impl AppleMusicService {
    /// Wraps the shared [`AppState`] in a new service; the token field starts
    /// unset (authentication is stubbed until a real Apple Music API lands).
    pub fn new(state: Arc<Mutex<AppState>>) -> Self {
        Self { token: None, state }
    }

    /// Plays the given track by id, recording it as the current track and
    /// marking it playing. A blank `track_id` — empty or only whitespace — or
    /// one carrying a terminal control character is rejected with an
    /// [`AppleMusicError`] and leaves shared state untouched: a blank id can
    /// never name a track, and a control-character id must not reach shared
    /// state. The playback log below escapes the id with `Debug` (see
    /// [`play_log_line`]), so even a non-control Unicode format character the
    /// guard does not reject cannot reach the terminal raw. The stub owns only
    /// this shared-state transition — a real implementation would add the API
    /// call that starts audio.
    ///
    /// `is_current` guards the commit: it is evaluated while the state lock is
    /// held, and when it returns `false` the track is left uncommitted because
    /// a newer play has superseded this one. A slow backend can complete two
    /// plays out of order, and without the guard the older reply would
    /// overwrite the newer track in shared state; the UI passes the guard from
    /// its playback-request counter, exactly as the browse path does.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when `track_id` is blank (empty or only
    /// whitespace) or carries a control character.
    pub async fn play_track(
        &self,
        track_id: &str,
        is_current: impl FnOnce() -> bool,
    ) -> Result<(), AppleMusicError> {
        ensure_id_is_valid(track_id, IdKind::Track)?;

        let mut state = self.state.lock().await;
        if !is_current() {
            return Ok(());
        }
        state.current_track = Some(track_id.to_string());
        state.is_playing = true;

        println!("{}", play_log_line(track_id));
        Ok(())
    }

    /// All favorite artists (every artist in the sample library).
    ///
    /// # Errors
    ///
    /// The stub never fails; the `Result` is the seam a real Apple Music
    /// backend reports a failed request through.
    pub async fn get_favorite_artists(&self) -> Result<Vec<Artist>, AppleMusicError> {
        Ok(sample_library().artists.clone())
    }

    /// Albums by the given artist; unknown artists yield an empty list.
    ///
    /// A blank `artist_id` — empty or only whitespace — or one carrying a
    /// terminal control character is rejected with an [`AppleMusicError`]
    /// instead of looked up: no artist has a blank id, so a blank id is a
    /// caller bug, and returning the empty list would report it as the
    /// ordinary "no albums" case. The same id guard
    /// [`AppleMusicService::play_track`] applies to its track id.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when `artist_id` is blank (empty or only
    /// whitespace) or carries a control character.
    pub async fn get_albums_by_artist(
        &self,
        artist_id: &str,
    ) -> Result<Vec<Album>, AppleMusicError> {
        ensure_id_is_valid(artist_id, IdKind::Artist)?;
        Ok(lookup(&sample_library().albums_by_artist, artist_id))
    }

    /// Songs on the given album; unknown albums yield an empty list.
    ///
    /// A blank `album_id` — empty or only whitespace — or one carrying a
    /// terminal control character is rejected with an [`AppleMusicError`], the
    /// album-query twin of [`AppleMusicService::get_albums_by_artist`].
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when `album_id` is blank (empty or only
    /// whitespace) or carries a control character.
    pub async fn get_songs_from_album(&self, album_id: &str) -> Result<Vec<Song>, AppleMusicError> {
        ensure_id_is_valid(album_id, IdKind::Album)?;
        Ok(lookup(&sample_library().songs_by_album, album_id))
    }
}

/// The resource an id names, so the seam's validation error can label it.
///
/// [`ensure_id_is_valid`] serves the three id-taking queries; naming the id
/// kinds as a closed enum (rather than passing a `&str`) makes a misspelled
/// label impossible to compile and keeps the three kinds the only ones the
/// guard accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdKind {
    Track,
    Artist,
    Album,
}

impl std::fmt::Display for IdKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Track => "track",
            Self::Artist => "artist",
            Self::Album => "album",
        })
    }
}

/// Validates an id at the seam: `Ok` when `id` names a resource, `Err` naming
/// the offending value when it is blank or carries a terminal control
/// character.
///
/// Every id in the library names a real Apple Music resource, so a blank id
/// is a caller bug rather than the ordinary "unknown id" case the browse
/// queries answer with an empty list. `kind` names the id in the message
/// (see [`IdKind`]). A whitespace-only id is rejected too:
/// like an empty one it can name nothing, and letting it through would record
/// a blank-looking Now Playing entry or report the bug as an ordinary empty
/// result. A control character (`\u{1b}`, `\n`, and the rest) is rejected
/// because `play_track` writes the id to the terminal log; without this check
/// a hostile id from the library could inject escape sequences there. The
/// message quotes the value with `{id:?}`, so both a whitespace-only id and a
/// control-character id read as escaped text rather than as invisible bytes.
fn ensure_id_is_valid(id: &str, kind: IdKind) -> Result<(), AppleMusicError> {
    if id.trim().is_empty() {
        return Err(AppleMusicError::new(format!(
            "{kind} id must not be blank (got {id:?})"
        )));
    }
    if id.chars().any(char::is_control) {
        return Err(AppleMusicError::new(format!(
            "{kind} id must not contain control characters (got {id:?})"
        )));
    }
    Ok(())
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
fn play_log_line(track_id: &str) -> String {
    format!("Playing track: {track_id:?}")
}

/// The group stored under `id` in `index`, or an empty list when the id is
/// unknown. The read-side twin of `index_by`: the browse queries both look
/// up their matches in the prebuilt tables, so this get-then-clone-then-
/// default chain lives here once instead of in each query method.
fn lookup<T: Clone>(index: &HashMap<String, Vec<T>>, id: &str) -> Vec<T> {
    index.get(id).cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests;
