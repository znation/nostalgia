use super::*;
use crate::test_support::assert_ids;

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

/// A fresh service plus its shared state with `song-1` already playing.
/// The tests whose subject is a *later* operation — replacing the track,
/// pausing, rejecting a blank or control-character id mid-playback, and
/// the transport stubs — all need that same starting point, so the
/// ordinary play (guard reporting "not superseded") lives here once and
/// each test names only the state it drives to.
async fn service_with_song_1_playing() -> (AppleMusicService, Arc<Mutex<AppState>>) {
    let (service, state) = test_service_with_state();
    service.play_track("song-1", || true).await.unwrap();
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

/// Asserts that both blank forms of an id — empty and whitespace-only —
/// are rejected by the query, each with the seam's blank-id error naming
/// `kind`. `ensure_id_is_valid` rejects the two forms through the same
/// branch, and a query that dropped the guard would answer with its
/// ordinary empty list instead; pinning both keeps the whitespace case
/// from silently reading as an unknown id. The three public id-taking
/// queries each repeat this two-case assertion, so it lives here once and
/// each test names its query and kind.
async fn assert_blank_id_rejected<T: std::fmt::Debug>(
    empty: impl std::future::Future<Output = Result<T, AppleMusicError>>,
    whitespace: impl std::future::Future<Output = Result<T, AppleMusicError>>,
    kind: IdKind,
) {
    let error = empty.await.unwrap_err().to_string();
    assert_eq!(error, format!("{kind} id must not be blank (got \"\")"));

    let error = whitespace.await.unwrap_err().to_string();
    assert_eq!(error, format!("{kind} id must not be blank (got \"   \")"));
}

/// Asserts that a query rejects an id carrying a terminal control
/// character with the seam's control-character error naming `kind` and
/// quoting `id`. Each public id-taking query has a control-character twin
/// of its blank-id test, so the assertion shape lives here once.
async fn assert_control_character_id_rejected<T: std::fmt::Debug>(
    result: impl std::future::Future<Output = Result<T, AppleMusicError>>,
    kind: IdKind,
    id: &str,
) {
    let error = result.await.unwrap_err().to_string();
    assert_eq!(
        error,
        format!("{kind} id must not contain control characters (got {id:?})")
    );
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
        ensure_id_is_valid("", IdKind::Track)
            .unwrap_err()
            .to_string(),
        "track id must not be blank (got \"\")"
    );
    assert_eq!(
        ensure_id_is_valid("\t", IdKind::Artist)
            .unwrap_err()
            .to_string(),
        "artist id must not be blank (got \"\\t\")"
    );
    // A nameable id passes through untouched, including one with an
    // internal space; only a wholly blank id is rejected.
    assert!(ensure_id_is_valid("song-1", IdKind::Track).is_ok());
    assert!(ensure_id_is_valid("a b", IdKind::Album).is_ok());
}

#[test]
fn ensure_id_is_valid_rejects_terminal_control_characters() {
    // `play_track` writes the track id to the terminal, so an id carrying
    // an escape sequence (`\u{1b}`) or other control byte would let a
    // hostile library entry manipulate the log stream. The guard rejects
    // it and quotes the value with `{id:?}`, so the error message itself
    // cannot carry the raw control bytes.
    let escape = "evil\u{1b}]0;pwnd\u{7}";
    let error = ensure_id_is_valid(escape, IdKind::Track)
        .unwrap_err()
        .to_string();
    assert_eq!(
        error,
        "track id must not contain control characters (got \"evil\\u{1b}]0;pwnd\\u{7}\")"
    );
    assert!(!error.contains('\u{1b}'));
    assert!(!error.contains('\u{7}'));
    // A newline is a control character too, so an id cannot forge a
    // second log line; a plain id still passes.
    assert!(ensure_id_is_valid("song\n1", IdKind::Track).is_err());
    assert!(ensure_id_is_valid("song-1", IdKind::Track).is_ok());
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
    let service = test_service();
    assert_blank_id_rejected(
        service.get_albums_by_artist(""),
        service.get_albums_by_artist("   "),
        IdKind::Artist,
    )
    .await;
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
    let service = test_service();
    let id = "artist\u{1b}1";
    assert_control_character_id_rejected(service.get_albums_by_artist(id), IdKind::Artist, id)
        .await;
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
    let service = test_service();
    assert_blank_id_rejected(
        service.get_songs_from_album(""),
        service.get_songs_from_album("   "),
        IdKind::Album,
    )
    .await;
}

// The album-query twin of the control-character artist-id rejection above.
#[tokio::test]
async fn get_songs_from_album_rejects_a_control_character_album_id() {
    let service = test_service();
    let id = "album\u{1b}1";
    assert_control_character_id_rejected(service.get_songs_from_album(id), IdKind::Album, id).await;
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
    let (service, state) = service_with_song_1_playing().await;

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
// error path the UI's `play_into` reports is pinned here, and
// shared state must be left untouched (the rejection happens before the
// state lock). Both an empty and a whitespace-only id are blank.
#[tokio::test]
async fn play_track_rejects_a_blank_track_id_without_touching_state() {
    let (service, state) = test_service_with_state();

    assert_blank_id_rejected(
        service.play_track("", || true),
        service.play_track("   ", || true),
        IdKind::Track,
    )
    .await;

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
    let (service, state) = service_with_song_1_playing().await;

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
    let (service, state) = service_with_song_1_playing().await;

    let id = "evil\u{1b}]0;pwnd\u{7}";
    assert_control_character_id_rejected(service.play_track(id, || true), IdKind::Track, id).await;

    assert_playback_state(&state, Some("song-1"), true).await;
}

#[tokio::test]
async fn pause_stops_playing_but_keeps_current_track() {
    let (service, state) = service_with_song_1_playing().await;

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
    let (service, state) = service_with_song_1_playing().await;

    service.next_track().await.unwrap();
    service.previous_track().await.unwrap();

    assert_playback_state(&state, Some("song-1"), true).await;
}

// `startup_token` decides whether startup sign-in runs and, when it does not,
// which of the three skip cases the message should name. The three cases are
// otherwise indistinguishable through `init_service` (which reads the real,
// process-global environment), so the pure classifier is pinned directly. A
// present, non-blank token is returned exactly as read — trimming happens
// later, at the `authorize` boundary — so a token file's trailing newline
// still signs in rather than being dropped here.
#[test]
fn startup_token_names_the_missing_blank_and_non_utf8_cases() {
    assert_eq!(
        startup_token(Ok("dev-token".to_string())),
        Ok("dev-token".to_string())
    );
    assert_eq!(
        startup_token(Ok("  dev-token  ".to_string())),
        Ok("  dev-token  ".to_string())
    );
    assert_eq!(
        startup_token(Ok("   ".to_string())),
        Err("APPLE_MUSIC_DEVELOPER_TOKEN is set but blank")
    );
    assert_eq!(
        startup_token(Err(std::env::VarError::NotPresent)),
        Err("APPLE_MUSIC_DEVELOPER_TOKEN is not set")
    );
    // `OsString` only holds non-UTF-8 bytes on Unix; on other platforms the
    // `NotUnicode` variant cannot be constructed portably, so that one arm is
    // covered where it exists.
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let invalid = std::ffi::OsString::from_vec(vec![0xff, 0xfe]);
        assert_eq!(
            startup_token(Err(std::env::VarError::NotUnicode(invalid))),
            Err("APPLE_MUSIC_DEVELOPER_TOKEN is not valid UTF-8")
        );
    }
}

// `init_service` is the startup seam `main` calls before the UI boots.
// When `APPLE_MUSIC_DEVELOPER_TOKEN` is unset it skips sign-in and returns a
// service with no session; when it is set it spawns a sign-in thread on a
// clone that only touches the service's own session, never the shared playback
// state. Pin the contract both paths keep — the shared state is exactly as
// `init_service` found it and the returned service is the one the UI shares —
// so a regression that wired the startup hook into `AppState` (or reused
// `pause`'s clear-the-flag logic) can't ship silently. `blocking_lock` panics
// inside an async runtime, so this stays a plain test. CI leaves
// `APPLE_MUSIC_DEVELOPER_TOKEN` unset, so this takes the skip path.
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

    let service = init_service(state.clone());

    assert!(service.session().is_none());
    let state = state.blocking_lock();
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(state.is_playing);
    assert!(state.repeat);
    assert_eq!(state.volume(), 0.7);
}

// The startup wiring: `init_service` builds one service, returns it for the
// UI, and runs the blocking sign-in on a clone. The session that sign-in
// stores must be visible through the returned service and must outlive the
// signing clone — the regression test for an earlier version that built a
// throwaway service inside the sign-in thread and dropped the session with it.
// `sign_in` is the synchronous body that thread runs; driving it directly
// exercises the startup path without the real browser flow. `init_service`
// itself reads the environment and spawns the thread, so only that thin wrapper
// is not driven here.
#[test]
fn startup_sign_in_stores_the_session_on_the_shared_service() {
    let service = test_service();
    let sign_in_service = service.clone();
    let expected = MusicKitSession {
        developer_token: "dev-token".to_string(),
        user_token: "user-token".to_string(),
    };

    sign_in_service.sign_in("dev-token", &|token| {
        assert_eq!(token, "dev-token");
        Ok(expected.clone())
    });
    drop(sign_in_service);

    assert_eq!(service.session(), Some(expected));
}

// The session seam: `new` starts with no session, `authenticate_with`
// stores the session its flow returns, and a failed flow propagates the
// error without disturbing an already-stored session. The real
// `authenticate` wraps the browser flow and is covered by the manual check
// in PLANS.md; these tests drive `authenticate_with` with a stub.

#[test]
fn a_new_service_has_no_session() {
    assert!(test_service().session().is_none());
}

#[test]
fn authenticate_with_stores_the_session_its_flow_returns() {
    let service = test_service();
    let expected = MusicKitSession {
        developer_token: "dev-token".to_string(),
        user_token: "user-token".to_string(),
    };

    service
        .authenticate_with("dev-token", &|token| {
            assert_eq!(token, "dev-token");
            Ok(expected.clone())
        })
        .unwrap();

    // `session` clones, so a second read returns the same value rather
    // than consuming the stored session.
    assert_eq!(service.session(), Some(expected.clone()));
    assert_eq!(service.session(), Some(expected));
}

#[test]
fn a_failed_authentication_leaves_a_stored_session_unchanged() {
    let service = test_service();
    let stored = MusicKitSession {
        developer_token: "dev-token".to_string(),
        user_token: "user-token".to_string(),
    };
    service
        .authenticate_with("dev-token", &|_| Ok(stored.clone()))
        .unwrap();

    let error = service
        .authenticate_with("dev-token", &|_| {
            Err(AppleMusicError::new("sign-in failed"))
        })
        .unwrap_err();

    assert_eq!(error.to_string(), "sign-in failed");
    assert_eq!(service.session(), Some(stored));
}

// The REST browse wiring: with a session stored, each browse query runs
// through the injected `rest::HttpTransport` and returns its mapped rows;
// without a session, the same service still answers from the sample library
// and never calls the transport. The stub records each request, so the URL
// and both tokens are pinned alongside the mapped rows.

use std::sync::Mutex as StdMutex;

/// A transport stub for the service tests: records every `(url, session)` it
/// is handed and returns the same canned result to each call. Cloning it
/// shares the recording, so a test keeps a handle after `with_transport`
/// boxes the clone.
#[derive(Clone)]
struct StubTransport {
    result: Result<String, AppleMusicError>,
    calls: Arc<StdMutex<Vec<(String, MusicKitSession)>>>,
}

impl StubTransport {
    /// A stub that answers every request with `body`.
    fn returning(body: &str) -> Self {
        Self {
            result: Ok(body.to_string()),
            calls: Arc::new(StdMutex::new(Vec::new())),
        }
    }

    /// A stub that fails every request with `message`.
    fn failing(message: &str) -> Self {
        Self {
            result: Err(AppleMusicError::new(message)),
            calls: Arc::new(StdMutex::new(Vec::new())),
        }
    }

    /// The requests recorded so far, oldest first.
    fn calls(&self) -> Vec<(String, MusicKitSession)> {
        self.calls.lock().unwrap().clone()
    }
}

impl rest::HttpTransport for StubTransport {
    fn get(&self, url: &str, session: &MusicKitSession) -> Result<String, AppleMusicError> {
        self.calls
            .lock()
            .unwrap()
            .push((url.to_string(), session.clone()));
        self.result.clone()
    }
}

/// The session a signed-in service stores, with recognizable tokens so a test
/// can assert the transport saw exactly these credentials.
fn rest_session() -> MusicKitSession {
    MusicKitSession {
        developer_token: "developer-token".to_string(),
        user_token: "user-token".to_string(),
    }
}

/// A service over `transport` with a session stored, so its browse queries
/// route through the REST client.
fn signed_in_service(transport: &StubTransport) -> AppleMusicService {
    let service = AppleMusicService::with_transport(
        Arc::new(Mutex::new(AppState::default())),
        Box::new(transport.clone()),
    );
    service
        .authenticate_with("developer-token", &|_| Ok(rest_session()))
        .unwrap();
    service
}

/// Asserts the stub saw exactly one request, to `expected_url` with the
/// stored session.
fn assert_single_rest_call(stub: &StubTransport, expected_url: &str) {
    let calls = stub.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, expected_url);
    assert_eq!(calls[0].1, rest_session());
}

#[tokio::test]
async fn get_favorite_artists_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}]}"#,
    );
    let service = signed_in_service(&stub);

    let artists = service.get_favorite_artists().await.unwrap();

    assert_eq!(
        artists,
        vec![Artist {
            id: "artist-1".to_string(),
            name: "The Sample Band".to_string(),
        }]
    );
    assert_single_rest_call(&stub, "https://api.music.apple.com/v1/me/library/artists");
}

#[tokio::test]
async fn get_albums_by_artist_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"album-1","attributes":{"name":"First Record"}}]}"#,
    );
    let service = signed_in_service(&stub);

    let albums = service.get_albums_by_artist("artist-9").await.unwrap();

    assert_eq!(
        albums,
        vec![Album {
            id: "album-1".to_string(),
            title: "First Record".to_string(),
            artist_id: "artist-9".to_string(),
        }]
    );
    assert_single_rest_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/artists/artist-9/albums",
    );
}

#[tokio::test]
async fn get_songs_from_album_uses_the_rest_library_when_signed_in() {
    let stub =
        StubTransport::returning(r#"{"data":[{"id":"song-1","attributes":{"name":"Opening"}}]}"#);
    let service = signed_in_service(&stub);

    let songs = service.get_songs_from_album("album-9").await.unwrap();

    assert_eq!(
        songs,
        vec![Song {
            id: "song-1".to_string(),
            title: "Opening".to_string(),
            album_id: "album-9".to_string(),
        }]
    );
    assert_single_rest_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/albums/album-9/tracks",
    );
}

#[tokio::test]
async fn a_transport_error_propagates_from_a_signed_in_browse_query() {
    let service = signed_in_service(&StubTransport::failing("network down"));

    let error = service.get_favorite_artists().await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "favorite artists request failed: network down"
    );
}

#[tokio::test]
async fn a_signed_out_service_ignores_the_transport_and_returns_sample_data() {
    let stub = StubTransport::failing("must not be called");
    let service = AppleMusicService::with_transport(
        Arc::new(Mutex::new(AppState::default())),
        Box::new(stub.clone()),
    );

    assert_eq!(
        service.get_favorite_artists().await.unwrap(),
        sample_library().artists.clone()
    );
    assert_eq!(
        service.get_albums_by_artist("artist-1").await.unwrap(),
        sample_library().albums_by_artist["artist-1"].clone()
    );
    assert_eq!(
        service.get_songs_from_album("album-1").await.unwrap(),
        sample_library().songs_by_album["album-1"].clone()
    );
    assert!(stub.calls().is_empty());
}
