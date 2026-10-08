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

// The seam's error constructor is called by `play_track` and the two browse
// queries to reject a blank id, and stays public for a real Apple Music
// backend that will report failures from another module, so it is live and
// needs no `dead_code` allowance. The transport stubs and token field below
// still carry theirs; a *newly* dead item elsewhere still triggers the
// warning the clean loop relies on to find removable code.
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
/// field or method elsewhere still triggers the warning the clean loop
/// relies on to find removable code.
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
    pub async fn play_track(
        &self,
        track_id: &str,
        is_current: impl FnOnce() -> bool,
    ) -> Result<(), AppleMusicError> {
        ensure_id_is_valid(track_id, "track")?;

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
    pub async fn get_albums_by_artist(
        &self,
        artist_id: &str,
    ) -> Result<Vec<Album>, AppleMusicError> {
        ensure_id_is_valid(artist_id, "artist")?;
        Ok(lookup(&sample_library().albums_by_artist, artist_id))
    }

    /// Songs on the given album; unknown albums yield an empty list.
    ///
    /// A blank `album_id` — empty or only whitespace — or one carrying a
    /// terminal control character is rejected with an [`AppleMusicError`], the
    /// album-query twin of [`AppleMusicService::get_albums_by_artist`].
    pub async fn get_songs_from_album(&self, album_id: &str) -> Result<Vec<Song>, AppleMusicError> {
        ensure_id_is_valid(album_id, "album")?;
        Ok(lookup(&sample_library().songs_by_album, album_id))
    }
}

/// Validates an id at the seam: `Ok` when `id` names a resource, `Err` naming
/// the offending value when it is blank or carries a terminal control
/// character.
///
/// Every id in the library names a real Apple Music resource, so a blank id
/// is a caller bug rather than the ordinary "unknown id" case the browse
/// queries answer with an empty list. `kind` names the id in the message
/// ("track", "artist", or "album"). A whitespace-only id is rejected too:
/// like an empty one it can name nothing, and letting it through would record
/// a blank-looking Now Playing entry or report the bug as an ordinary empty
/// result. A control character (`\u{1b}`, `\n`, and the rest) is rejected
/// because `play_track` writes the id to the terminal log; without this check
/// a hostile id from the library could inject escape sequences there. The
/// message quotes the value with `{id:?}`, so both a whitespace-only id and a
/// control-character id read as escaped text rather than as invisible bytes.
fn ensure_id_is_valid(id: &str, kind: &str) -> Result<(), AppleMusicError> {
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
mod tests {
    use super::*;
    use crate::test_support::{
        assert_every_field_required, assert_ids, assert_serializes_as,
        assert_unknown_fields_tolerated,
    };
    use serde_json::json;

    fn test_service() -> AppleMusicService {
        AppleMusicService::new(Arc::new(Mutex::new(AppState::default())))
    }

    /// A fresh service plus a handle to the shared state it mutates, so a
    /// playback test can drive the service and then inspect the resulting
    /// `AppState`. The playback tests below all start with this same
    /// service-and-state pair, so it lives here once.
    fn test_service_with_state() -> (AppleMusicService, Arc<Mutex<AppState>>) {
        let service = test_service();
        let state = service.state.clone();
        (service, state)
    }

    /// Locks the shared playback state and asserts it holds `expected_track`
    /// with `expected_playing`. The playback tests below all end on that
    /// same pair — the recorded track id and the playing flag — so the
    /// lock-and-compare sequence lives here once and each test only names the
    /// state it drove to.
    async fn assert_playback_state(
        state: &Arc<Mutex<AppState>>,
        expected_track: Option<&str>,
        expected_playing: bool,
    ) {
        let state = state.lock().await;
        assert_eq!(state.current_track.as_deref(), expected_track);
        assert_eq!(state.is_playing, expected_playing);
    }

    /// The representative Apple Music token the three token tests share: a full,
    /// valid payload for the round-trip, missing-field, and unknown-field
    /// contracts. Each test used to spell out the same three field values — twice
    /// as a struct literal and three times as the equivalent JSON object — so the
    /// fixture lives here once and each test only names the contract it pins.
    fn sample_token() -> AppleMusicToken {
        AppleMusicToken {
            access_token: "abc123".to_string(),
            expires_in: 3600,
            refresh_token: "refresh-me".to_string(),
        }
    }

    /// The token's JSON wire shape, the payload the token tests deserialize. Kept
    /// a hand-written literal rather than derived from `sample_token`, so
    /// `apple_music_token_round_trips_through_json` still pins the serialized
    /// field names — a `#[serde(rename)]` would change `to_value(sample_token())`
    /// but not this object. The missing- and unknown-field probes hand it in as
    /// the full, valid payload.
    fn token_payload() -> serde_json::Value {
        json!({ "access_token": "abc123", "expires_in": 3600, "refresh_token": "refresh-me" })
    }

    #[test]
    fn apple_music_error_displays_its_message() {
        // The seam's error type formats as the cause itself, so a reported
        // failure reads as the message, not as a struct dump.
        let error = AppleMusicError::new("track not found");
        assert_eq!(error.to_string(), "track not found");
    }

    #[test]
    fn apple_music_error_new_accepts_an_owned_message() {
        // `new` takes `impl Into<String>`, so a backend can pass a formatted
        // `String` as readily as a `&str`; both must format identically.
        let error = AppleMusicError::new(format!("album {} not found", "album-1"));
        assert_eq!(error.to_string(), "album album-1 not found");
    }

    #[test]
    fn ensure_id_is_valid_rejects_empty_and_whitespace_ids() {
        // An id names a real Apple Music resource, so a blank one is a caller
        // bug. Both an empty id and a whitespace-only id are blank; the
        // message quotes the value so the whitespace case is visible rather
        // than reading as an empty string.
        assert_eq!(
            ensure_id_is_valid("", "track").unwrap_err().to_string(),
            "track id must not be blank (got \"\")"
        );
        assert_eq!(
            ensure_id_is_valid("\t", "artist").unwrap_err().to_string(),
            "artist id must not be blank (got \"\\t\")"
        );
        // A nameable id passes through untouched, including one with an
        // internal space; only a wholly blank id is rejected.
        assert!(ensure_id_is_valid("song-1", "track").is_ok());
        assert!(ensure_id_is_valid("a b", "album").is_ok());
    }

    #[test]
    fn ensure_id_is_valid_rejects_terminal_control_characters() {
        // `play_track` writes the track id to the terminal, so an id carrying
        // an escape sequence (`\u{1b}`) or other control byte would let a
        // hostile library entry manipulate the log stream. The guard rejects
        // it and quotes the value with `{id:?}`, so the error message itself
        // cannot carry the raw control bytes.
        let escape = "evil\u{1b}]0;pwnd\u{7}";
        let error = ensure_id_is_valid(escape, "track").unwrap_err().to_string();
        assert_eq!(
            error,
            "track id must not contain control characters (got \"evil\\u{1b}]0;pwnd\\u{7}\")"
        );
        assert!(!error.contains('\u{1b}'));
        assert!(!error.contains('\u{7}'));
        // A newline is a control character too, so an id cannot forge a
        // second log line; a plain id still passes.
        assert!(ensure_id_is_valid("song\n1", "track").is_err());
        assert!(ensure_id_is_valid("song-1", "track").is_ok());
    }

    // The success log line is the second terminal sink for a track id, and the
    // guard does not close it: `char::is_control` is true only for the Cc
    // category, so a Unicode format character such as the right-to-left
    // override (`\u{202e}`, category Cf) passes `ensure_id_is_valid`. The sink
    // must therefore escape the id itself; `play_log_line` formats with
    // `Debug`, which renders both the control and the format character as
    // `\u{...}`. The helper is pure so this contract is testable without
    // capturing stdout.
    #[test]
    fn play_log_line_escapes_control_and_format_characters() {
        let line = play_log_line("evil\u{1b}]0;pwnd\u{7}");
        assert_eq!(line, "Playing track: \"evil\\u{1b}]0;pwnd\\u{7}\"");
        assert!(!line.contains('\u{1b}'));
        assert!(!line.contains('\u{7}'));

        let line = play_log_line("a\u{202e}b");
        assert_eq!(line, "Playing track: \"a\\u{202e}b\"");
        assert!(!line.contains('\u{202e}'));
    }

    // "All favorite artists" is the service's content contract, not just a
    // non-empty list: every artist, in library order. The albums and songs
    // queries each pin their exact group in order, but the artists query only
    // asserted non-empty — so a regression that dropped an artist (e.g. a
    // `.take(1)`) or reversed the list would pass every existing test while
    // the browse view silently lost or reordered an artist. `assert_ids` is
    // the same helper the other browse queries use.
    #[tokio::test]
    async fn get_favorite_artists_returns_all_artists_in_library_order() {
        let artists = test_service().get_favorite_artists().await.unwrap();
        assert_ids(
            &artists,
            |artist| artist.id.as_str(),
            &["artist-1", "artist-2", "artist-3"],
        );
    }

    #[tokio::test]
    async fn get_albums_by_artist_returns_only_matching_albums() {
        let service = test_service();
        let albums = service.get_albums_by_artist("artist-1").await.unwrap();
        assert!(!albums.is_empty());
        assert!(albums.iter().all(|album| album.artist_id == "artist-1"));
        assert!(albums.iter().all(|album| album.artist_id != "artist-2"));
    }

    #[tokio::test]
    async fn get_albums_by_artist_returns_all_matching_albums_in_library_order() {
        // The lookup table must keep every album of a multi-album artist and
        // preserve the library's original order: a regression that dropped
        // duplicates or reversed a group would still pass the all()-style
        // checks above, so pin the exact group here.
        let albums = test_service()
            .get_albums_by_artist("artist-1")
            .await
            .unwrap();
        assert_ids(&albums, |album| album.id.as_str(), &["album-1", "album-2"]);

        // An existing artist with no albums yields an empty list, distinct
        // from an unknown id (same lookup path, but worth pinning the sample).
        assert!(
            test_service()
                .get_albums_by_artist("artist-3")
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn get_albums_by_artist_unknown_id_is_empty() {
        let albums = test_service()
            .get_albums_by_artist("no-such-artist")
            .await
            .unwrap();
        assert!(albums.is_empty());
    }

    // A blank id is a caller bug, not an unknown artist: the seam rejects it
    // with an error rather than the empty list an unknown id yields, so a
    // future caller that drops the id gets a report naming the cause instead
    // of a silent "no albums". The whitespace-only case goes through the same
    // guard as the empty one.
    #[tokio::test]
    async fn get_albums_by_artist_rejects_a_blank_artist_id() {
        let error = test_service().get_albums_by_artist("").await.unwrap_err();
        assert_eq!(error.to_string(), "artist id must not be blank (got \"\")");

        let error = test_service()
            .get_albums_by_artist("   ")
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "artist id must not be blank (got \"   \")"
        );
    }

    // The security twin of the blank-id rejection above: an artist id carrying
    // a terminal control character — the shape a hostile Apple Music reply
    // could supply — must be rejected rather than looked up. The guard's own
    // test pins the check with kind "track"; this pins that the public browse
    // query reaches it for artist ids, so a regression that dropped the
    // control-character check from this path (or fell back to a blank-only
    // guard) cannot ship silently.
    #[tokio::test]
    async fn get_albums_by_artist_rejects_a_control_character_artist_id() {
        let error = test_service()
            .get_albums_by_artist("artist\u{1b}1")
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "artist id must not contain control characters (got \"artist\\u{1b}1\")"
        );
    }

    #[tokio::test]
    async fn get_albums_by_artist_returns_single_album_artists_album() {
        // The empty (artist-3) and multi-album (artist-1) groups are pinned
        // above; artist-2's single-album group is the third branch of the
        // `index_by` grouping — a regression that dropped a one-element group
        // (e.g. only pushing after a second element arrives) would clear the
        // other two tests and silently lose this artist's album.
        let albums = test_service()
            .get_albums_by_artist("artist-2")
            .await
            .unwrap();
        assert_ids(&albums, |album| album.id.as_str(), &["album-3"]);
    }

    #[tokio::test]
    async fn get_songs_from_album_returns_only_matching_songs() {
        let service = test_service();
        let songs = service.get_songs_from_album("album-1").await.unwrap();
        assert!(!songs.is_empty());
        assert!(songs.iter().all(|song| song.album_id == "album-1"));
    }

    #[tokio::test]
    async fn get_songs_from_album_returns_all_songs_in_library_order() {
        // As with albums: a multi-song album must come back whole and in the
        // sample library's order, not a subset or a reversed group.
        let songs = test_service()
            .get_songs_from_album("album-1")
            .await
            .unwrap();
        assert_ids(
            &songs,
            |song| song.id.as_str(),
            &["song-1", "song-2", "song-3"],
        );
    }

    #[tokio::test]
    async fn get_songs_from_album_unknown_id_is_empty() {
        let songs = test_service()
            .get_songs_from_album("no-such-album")
            .await
            .unwrap();
        assert!(songs.is_empty());
    }

    // The album-query twin of the blank-artist-id rejection above, whitespace
    // case included.
    #[tokio::test]
    async fn get_songs_from_album_rejects_a_blank_album_id() {
        let error = test_service().get_songs_from_album("").await.unwrap_err();
        assert_eq!(error.to_string(), "album id must not be blank (got \"\")");

        let error = test_service()
            .get_songs_from_album("   ")
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "album id must not be blank (got \"   \")"
        );
    }

    // The album-query twin of the control-character artist-id rejection above.
    #[tokio::test]
    async fn get_songs_from_album_rejects_a_control_character_album_id() {
        let error = test_service()
            .get_songs_from_album("album\u{1b}1")
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "album id must not contain control characters (got \"album\\u{1b}1\")"
        );
    }

    #[tokio::test]
    async fn get_songs_from_album_returns_single_song_albums_song() {
        // album-1 (multi-song) and the unknown-id case are pinned above;
        // album-2's single-song group is the third branch — if the grouping
        // dropped one-element groups, this song would silently vanish.
        let songs = test_service()
            .get_songs_from_album("album-2")
            .await
            .unwrap();
        assert_ids(&songs, |song| song.id.as_str(), &["song-4"]);
    }

    #[tokio::test]
    async fn play_track_sets_current_track_and_starts_playing() {
        let (service, state) = test_service_with_state();

        service.play_track("song-1", || true).await.unwrap();

        assert_playback_state(&state, Some("song-1"), true).await;
    }

    // The test above plays once from the default (no track), so it only
    // proves the first selection is recorded. The Songs view lets the user
    // click any row, so every later selection must replace `current_track`;
    // a regression that only set it when none was playing (or that made
    // `play_track` idempotent on the id) would clear that test while the
    // Now Playing bar silently kept naming the first song. Pin the overwrite,
    // and that playback stays on across it.
    #[tokio::test]
    async fn play_track_replaces_the_current_track_when_another_song_is_played() {
        let (service, state) = test_service_with_state();

        service.play_track("song-1", || true).await.unwrap();
        service.play_track("song-2", || true).await.unwrap();

        assert_playback_state(&state, Some("song-2"), true).await;
    }

    // A play reply can complete out of order: the user clicks song-1, then
    // song-2, and song-1's slower play lands last. Committing that older
    // reply would replace the newer track in shared state. The guard is
    // evaluated under the state lock, so a superseded play leaves the newer
    // track untouched — the seam half of the UI's out-of-order-play guard.
    #[tokio::test]
    async fn a_superseded_play_leaves_the_newer_track_in_place() {
        let (service, state) = test_service_with_state();

        // The newer play commits, then the older one completes late with a
        // guard reporting it has been superseded.
        service.play_track("song-2", || true).await.unwrap();
        service.play_track("song-1", || false).await.unwrap();

        assert_playback_state(&state, Some("song-2"), true).await;
    }

    // A blank id can never name a track, so the seam rejects it instead of
    // recording a blank `current_track` and marking nothing as playing. The
    // error path the UI's `played_or_reported` reports is pinned here, and
    // shared state must be left untouched (the rejection happens before the
    // state lock). Both an empty and a whitespace-only id are blank.
    #[tokio::test]
    async fn play_track_rejects_a_blank_track_id_without_touching_state() {
        let (service, state) = test_service_with_state();

        let error = service.play_track("", || true).await.unwrap_err();
        assert_eq!(error.to_string(), "track id must not be blank (got \"\")");

        let error = service.play_track("   ", || true).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "track id must not be blank (got \"   \")"
        );

        assert_playback_state(&state, None, false).await;
    }

    // The rejection test above starts from the default (no track, not
    // playing), so a regression that cleared the shared state *while*
    // rejecting the blank id would still leave it at `None`/`false` and pass
    // unnoticed. The Songs view can be clicked mid-playback, so pin the
    // rejection against a live track: the already-playing song must survive
    // untouched rather than being cleared to "nothing is playing".
    #[tokio::test]
    async fn play_track_rejects_a_blank_track_id_while_a_song_is_playing() {
        let (service, state) = test_service_with_state();

        service.play_track("song-1", || true).await.unwrap();
        let error = service.play_track("", || true).await.unwrap_err();

        assert_eq!(error.to_string(), "track id must not be blank (got \"\")");
        assert_playback_state(&state, Some("song-1"), true).await;
    }

    // A track id carrying a terminal control character would inject an escape
    // sequence into the `Playing track: {id}` log line (and into the
    // `current_track` the Now Playing bar later renders). The seam rejects it
    // exactly as it rejects a blank id, leaving shared state untouched; the
    // error message escapes the value with `{id:?}` so the report itself is
    // safe to print.
    #[tokio::test]
    async fn play_track_rejects_a_control_character_track_id() {
        let (service, state) = test_service_with_state();

        service.play_track("song-1", || true).await.unwrap();
        let error = service
            .play_track("evil\u{1b}]0;pwnd\u{7}", || true)
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "track id must not contain control characters (got \"evil\\u{1b}]0;pwnd\\u{7}\")"
        );
        assert_playback_state(&state, Some("song-1"), true).await;
    }

    #[tokio::test]
    async fn pause_stops_playing_but_keeps_current_track() {
        let (service, state) = test_service_with_state();

        service.play_track("song-1", || true).await.unwrap();
        service.pause().await.unwrap();

        assert_playback_state(&state, Some("song-1"), false).await;
    }

    // `pause`'s test above pins its state change; its two sibling stubs,
    // `next_track` and `previous_track`, have no test reaching them at all.
    // They are deliberately unimplemented no-ops — the transport buttons step
    // through `transport::next_track_id`/`previous_track_id` instead — so pin
    // the one contract they do carry: they report success and, unlike
    // `pause`, leave the shared playback state untouched. A regression that
    // wired either stub into `AppState` (or made it fail) would otherwise
    // ship silently, since no caller reaches them today.
    #[tokio::test]
    async fn next_and_previous_track_stubs_succeed_without_touching_state() {
        let (service, state) = test_service_with_state();

        service.play_track("song-1", || true).await.unwrap();

        service.next_track().await.unwrap();
        service.previous_track().await.unwrap();

        assert_playback_state(&state, Some("song-1"), true).await;
    }

    // `init_service` is the startup seam `main` calls before the UI boots,
    // and the hook a real Apple Music backend will authenticate and load the
    // library through. Today's stub only prints that it is ready, and no test
    // reaches it. Pin the one contract the stub carries — it leaves the
    // shared playback state exactly as it found it — so a regression that
    // wired the startup hook into `AppState` (or reused `pause`'s
    // clear-the-flag logic) can't ship silently. The stub takes the handle a
    // real implementation will use, so the untouched state is the observable
    // contract, as with `next_track`/`previous_track` above. `blocking_lock`
    // panics inside an async runtime, so this stays a plain test.
    #[test]
    fn init_service_leaves_the_shared_state_untouched() {
        let state = Arc::new(Mutex::new(AppState::default()));
        {
            let mut state = state.blocking_lock();
            state.current_track = Some("song-1".to_string());
            state.is_playing = true;
            state.repeat = true;
            state.set_volume(0.7);
        }

        init_service(state.clone());

        let state = state.blocking_lock();
        assert_eq!(state.current_track.as_deref(), Some("song-1"));
        assert!(state.is_playing);
        assert!(state.repeat);
        assert_eq!(state.volume(), 0.7);
    }

    // `AppleMusicToken` is the auth payload the real Apple Music API will
    // hand back, so its wire contract is pinned the same way the model types
    // in `library.rs` are: field names serialize as-is, the value round-trips
    // through `serde_json`, unknown fields are tolerated, and a payload
    // missing a required field is rejected.

    // The token's JSON shape is the contract: field names serialize as-is and
    // the value survives an out-and-back trip through `serde_json` unchanged.
    #[test]
    fn apple_music_token_round_trips_through_json() {
        assert_serializes_as(sample_token(), token_payload());
    }

    // As with the model types: a payload missing any required field must
    // error, not silently yield a half-populated token.
    #[test]
    fn apple_music_token_deserialization_rejects_missing_required_fields() {
        assert_every_field_required::<AppleMusicToken>(token_payload());
    }

    // The unknown-field half of the token's deserialization contract: a real
    // token payload carries fields beyond the declared three (the OAuth 2.0
    // token response includes a `token_type`, for one), and serde's default
    // must tolerate the extras rather than failing the whole parse. No other
    // test deserializes the token with an extra field, so a
    // `#[serde(deny_unknown_fields)]` added here would clear every existing
    // test while breaking real authentication.
    #[test]
    fn apple_music_token_deserialization_ignores_unknown_fields() {
        let mut payload = token_payload();
        payload
            .as_object_mut()
            .expect("the token payload is a JSON object")
            .insert("token_type".to_string(), json!("Bearer"));
        assert_unknown_fields_tolerated::<AppleMusicToken>(payload, sample_token());
    }
}
