use super::*;
use crate::audio::AudioOutput;
use crate::test_support::{
    AudioCall, FailingAudio, PREVIEW_URL, RecordingAudio, StubTransport, assert_ids,
    assert_single_call, rest_session, sample_album, sample_artist, sample_song, sign_in_session,
};

fn test_service() -> AppleMusicService {
    // Over a stub transport and the silent audio backend, so a test never
    // touches the network or an audio device; the sample library answers
    // because no session is stored.
    AppleMusicService::with_transport(
        Arc::new(Mutex::new(AppState::default())),
        Box::new(StubTransport::returning("")),
    )
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
/// pausing, rejecting a blank id, a control-character id, or a blank
/// preview URL mid-playback, and the transport stubs — all need that same
/// starting point, so the ordinary play (guard reporting "not superseded")
/// lives here once and each test names only the state it drives to.
async fn service_with_song_1_playing() -> (AppleMusicService, Arc<Mutex<AppState>>) {
    let (service, state) = test_service_with_state();
    service.play_track("song-1", None, || true).await.unwrap();
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

/// A service over `audio` and a stub transport, plus its shared state, so a
/// playback test drives the injected backend. The transport never answers
/// because no session is stored, matching the sample-library path.
fn service_with_audio(audio: Arc<dyn AudioOutput>) -> (AppleMusicService, Arc<Mutex<AppState>>) {
    let state = Arc::new(Mutex::new(AppState::default()));
    let service =
        AppleMusicService::with_audio(state.clone(), Box::new(StubTransport::returning("")), audio);
    (service, state)
}

/// A service over `audio` and its shared state with `song-1`'s preview already
/// playing, so the backend holds a loaded player. The transport tests whose
/// subject is a *later* transition — resume, stop, resume-after-stop, and the
/// two toggle directions — all need that starting point, so the `Some`-preview
/// play lives here once and each test names only the transition it drives.
async fn service_with_preview_playing(
    audio: Arc<dyn AudioOutput>,
) -> (AppleMusicService, Arc<Mutex<AppState>>) {
    let (service, state) = service_with_audio(audio);
    service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap();
    (service, state)
}

/// A service over a recording backend, the shared state it mutates, and the
/// recording that captured its `AudioCall`s.
type RecordingService = (AppleMusicService, Arc<Mutex<AppState>>, Arc<RecordingAudio>);

/// A fresh service over a recording backend, plus its shared state and the
/// recording itself, so a transport test can drive the service and then assert
/// the exact `AudioCall` sequence it sent. The four tests that start from a
/// stopped transport build this same `Arc::new`/`Arc::clone` pair, so it lives
/// here once.
fn recording_service() -> RecordingService {
    let recording = Arc::new(RecordingAudio::default());
    let (service, state) = service_with_audio(Arc::clone(&recording) as Arc<dyn AudioOutput>);
    (service, state, recording)
}

/// [`recording_service`] with `song-1`'s preview already playing, so the
/// backend holds a loaded player for the five tests whose subject is a later
/// transition. They build the same pair, so it lives here once.
async fn recording_service_with_preview() -> RecordingService {
    let recording = Arc::new(RecordingAudio::default());
    let (service, state) =
        service_with_preview_playing(Arc::clone(&recording) as Arc<dyn AudioOutput>).await;
    (service, state, recording)
}

/// A recording backend whose `pause` blocks until released, so a test can hold
/// one transport transition inside the backend and prove a second one waits on
/// the service's transport lock rather than entering the backend alongside it.
struct GatedAudio {
    calls: std::sync::Mutex<Vec<AudioCall>>,
    entered: std::sync::mpsc::Sender<()>,
    release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
}

impl GatedAudio {
    fn calls(&self) -> Vec<AudioCall> {
        self.calls.lock().unwrap().clone()
    }
}

impl AudioOutput for GatedAudio {
    fn play(&self, url: &str) -> Result<(), AppleMusicError> {
        self.calls
            .lock()
            .unwrap()
            .push(AudioCall::Play(url.to_string()));
        Ok(())
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        self.calls.lock().unwrap().push(AudioCall::Pause);
        let _ = self.entered.send(());
        let _ = self.release.lock().unwrap().recv();
        Ok(())
    }

    fn resume(&self) -> Result<(), AppleMusicError> {
        self.calls.lock().unwrap().push(AudioCall::Resume);
        Ok(())
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        self.calls.lock().unwrap().push(AudioCall::Stop);
        Ok(())
    }

    fn set_volume(&self, volume: f32) -> Result<(), AppleMusicError> {
        self.calls
            .lock()
            .unwrap()
            .push(AudioCall::SetVolume(volume));
        Ok(())
    }
}

/// The one backend command a [`RejectingAudio`] backend refuses.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RejectedCommand {
    Pause,
    Resume,
    Stop,
    SetVolume,
}

/// An audio backend that records every command and refuses one chosen command,
/// so a test can reach a live transport and then pin that the service reports
/// the backend's failure *before* shared state changes. Every other command
/// succeeds, so the service arrives at the state the rejected command must
/// leave untouched, and the recorded calls let a test prove a command was
/// *not* issued — for example, that a rejected `set_volume` keeps `play_track`
/// from starting the preview at all.
struct RejectingAudio {
    rejected: RejectedCommand,
    calls: std::sync::Mutex<Vec<AudioCall>>,
}

impl RejectingAudio {
    fn new(rejected: RejectedCommand) -> Self {
        Self {
            rejected,
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<AudioCall> {
        self.calls.lock().unwrap().clone()
    }

    fn check(&self, command: RejectedCommand) -> Result<(), AppleMusicError> {
        if self.rejected == command {
            Err(AppleMusicError::new("the audio device is gone"))
        } else {
            Ok(())
        }
    }
}

impl AudioOutput for RejectingAudio {
    fn play(&self, url: &str) -> Result<(), AppleMusicError> {
        self.calls
            .lock()
            .unwrap()
            .push(AudioCall::Play(url.to_string()));
        Ok(())
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        self.calls.lock().unwrap().push(AudioCall::Pause);
        self.check(RejectedCommand::Pause)
    }

    fn resume(&self) -> Result<(), AppleMusicError> {
        self.calls.lock().unwrap().push(AudioCall::Resume);
        self.check(RejectedCommand::Resume)
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        self.calls.lock().unwrap().push(AudioCall::Stop);
        self.check(RejectedCommand::Stop)
    }

    fn set_volume(&self, volume: f32) -> Result<(), AppleMusicError> {
        self.calls
            .lock()
            .unwrap()
            .push(AudioCall::SetVolume(volume));
        self.check(RejectedCommand::SetVolume)
    }
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

// The library search is the UI search box's backend: it answers with the
// sample songs whose title or artist contains the query, case-insensitively,
// so a user can find a track by either field without matching the exact case.
#[tokio::test]
async fn search_songs_matches_the_sample_library_by_title_or_artist() {
    let service = test_service();

    // A title-only match, case-insensitively, with surrounding whitespace
    // trimmed off the query.
    let by_title = service.search_songs("  OPENING  ").await.unwrap();
    assert_ids(&by_title, |song| song.id.as_str(), &["song-1"]);

    // An artist-only match: "song-5" is the only Echo Chamber track.
    let by_artist = service.search_songs("echo").await.unwrap();
    assert_ids(&by_artist, |song| song.id.as_str(), &["song-5"]);

    // A match spanning two albums of one artist comes back in library order,
    // not the `songs_by_album` map's unspecified iteration order.
    let band = service.search_songs("sample band").await.unwrap();
    assert_ids(
        &band,
        |song| song.id.as_str(),
        &["song-1", "song-2", "song-3", "song-4"],
    );

    // A query matching nothing is an empty list, not an error.
    let none = service.search_songs("no such track").await.unwrap();
    assert!(none.is_empty());
}

// A blank search has nothing to match; returning the whole library (or an
// empty list) would report the caller's empty input as an ordinary result.
// The error names the offending query, whitespace included.
#[tokio::test]
async fn search_songs_rejects_a_blank_query() {
    let service = test_service();

    for query in ["", "   "] {
        let error = service.search_songs(query).await.unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("search query must not be blank"),
            "unexpected message: {message}"
        );
        assert!(
            message.contains(&format!("{query:?}")),
            "the error must name the query: {message}"
        );
    }
}

#[tokio::test]
async fn play_track_sets_current_track_and_starts_playing() {
    let (service, state) = test_service_with_state();

    service.play_track("song-1", None, || true).await.unwrap();

    assert_playback_state(&state, Some("song-1"), true).await;
}

// The service hands a track's preview URL to the injected audio backend and
// commits the shared-state transition. The recording fake pins the exact URL,
// so a regression that dropped it (or reordered the arguments) fails here.
#[tokio::test]
async fn play_track_starts_the_preview_through_the_audio_backend() {
    let (service, state, recording) = recording_service();

    service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap();

    assert_eq!(
        recording.calls(),
        vec![
            AudioCall::SetVolume(0.5),
            AudioCall::Play(PREVIEW_URL.to_string())
        ]
    );
    assert_playback_state(&state, Some("song-1"), true).await;
}

// `set_output_volume` forwards to the injected backend without touching the
// shared state: the UI stores the clamped value itself, so this method is only
// the seam to the output gain.
#[tokio::test]
async fn set_output_volume_forwards_to_the_audio_backend() {
    let recording = Arc::new(RecordingAudio::default());
    let (service, state) = service_with_audio(Arc::clone(&recording) as Arc<dyn AudioOutput>);

    service.set_output_volume(0.3).unwrap();

    assert_eq!(recording.calls(), vec![AudioCall::SetVolume(0.3)]);
    assert_eq!(state.lock().await.volume(), 0.5);
}

// A song the library supplied without a preview URL still commits the state
// transition — the Now Playing bar must name it — but starts no audio. The
// recording fake pins that no `Play` reaches the backend.
#[tokio::test]
async fn play_track_without_a_preview_url_commits_state_without_audio() {
    let (service, state, recording) = recording_service();

    service.play_track("song-1", None, || true).await.unwrap();

    assert!(recording.calls().is_empty());
    assert_playback_state(&state, Some("song-1"), true).await;
}

// A `Some` preview URL that is blank can play nothing, so the seam rejects it
// before committing state — the same guard a blank track id gets. The REST
// layer never produces one (`Resource::preview_url` maps a blank preview to
// `None`), so this pins the public seam's contract for a caller that hands it
// one directly. Both an empty and a whitespace-only URL are blank, and the
// error quotes the value with `{url:?}` so the whitespace case is visible.
// State must survive untouched, so the already-playing song keeps playing.
#[tokio::test]
async fn play_track_rejects_a_blank_preview_url_without_touching_state() {
    let (service, state) = service_with_song_1_playing().await;

    let error = service
        .play_track("song-2", Some(""), || true)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "preview URL must not be blank (got \"\")"
    );

    let error = service
        .play_track("song-2", Some("   "), || true)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "preview URL must not be blank (got \"   \")"
    );

    assert_playback_state(&state, Some("song-1"), true).await;
}

// A preview URL comes from the Apple Music API response and is handed to a
// network fetch by the audio backend, so the seam must reject anything that is
// not an https URL with an acceptable host: a cleartext URL, a
// loopback/private/link-local address literal (an SSRF target), a non-http
// scheme, a numeric host form that resolves to an address, and a value
// carrying a control character. Each must fail before shared state is
// committed and before the backend records a play, so a hostile or compromised
// API reply cannot reach the fetch through a cleartext, address-literal, or
// numeric-host URL.
#[tokio::test]
async fn play_track_rejects_a_preview_url_that_is_not_a_public_https_url() {
    let recording = Arc::new(RecordingAudio::default());
    let (service, state) = service_with_audio(Arc::clone(&recording) as Arc<dyn AudioOutput>);

    for url in [
        "http://127.0.0.1:8080/preview.m4a",
        "http://169.254.169.254/latest/meta-data/",
        "https://localhost/preview.m4a",
        "https://[::1]/preview.m4a",
        "https://10.0.0.5/preview.m4a",
        "https://192.168.1.1/preview.m4a",
        "https://2130706433/preview.m4a",
        "https://127.1/preview.m4a",
        "https://0x7f000001/preview.m4a",
        "https://0x7f.0.0.1/preview.m4a",
        "https://0177.0.0.1/preview.m4a",
        "file:///etc/passwd",
        "https://example.test/\u{1b}[2Jpreview.m4a",
    ] {
        let error = service
            .play_track("song-1", Some(url), || true)
            .await
            .unwrap_err();
        assert!(
            error.to_string().starts_with("preview URL "),
            "unexpected error for {url:?}: {error}"
        );
    }

    assert!(recording.calls().is_empty());
    assert_playback_state(&state, None, false).await;
}

// The guard must not reject a legitimate preview URL, so Apple's CDN host, a
// plain public host, and a public address literal all still reach the backend.
// `play_track` first forwards the state's volume (the default 0.5) so the
// preview starts at the slider's value, then starts the play.
#[tokio::test]
async fn play_track_accepts_a_public_https_preview_url() {
    for url in [
        "https://audio-ssl.itunes.apple.com/itunes-assets/AudioPreview/x.m4a",
        PREVIEW_URL,
        "https://93.184.216.34/preview.m4a",
    ] {
        let recording = Arc::new(RecordingAudio::default());
        let (service, _state) = service_with_audio(Arc::clone(&recording) as Arc<dyn AudioOutput>);
        service
            .play_track("song-1", Some(url), || true)
            .await
            .unwrap();
        assert_eq!(
            recording.calls(),
            vec![AudioCall::SetVolume(0.5), AudioCall::Play(url.to_string())]
        );
    }
}

// A backend that cannot start the preview reports it through the seam, so the
// UI's play path can log the failure instead of silently claiming success.
// The preview never started, so shared state must not keep claiming it is
// playing: `current_track` records the selection but `is_playing` is rolled
// back.
#[tokio::test]
async fn play_track_propagates_an_audio_backend_failure() {
    let (service, state) = service_with_audio(Arc::new(FailingAudio));

    let error = service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-1"), false).await;
}

// A play can fail after a newer one has already committed: the user clicks
// song-1, then song-2, and song-1's backend call fails after song-2 is
// playing. The failure rollback must not clear the newer play's flag, or the
// UI would show song-2 stopped while it plays. The guard reports "current" at
// commit and "superseded" at rollback — the window in which a newer play can
// land — and the rollback then leaves the flag set.
#[tokio::test]
async fn play_track_rollback_leaves_a_newer_plays_flag_alone() {
    let (service, state) = service_with_audio(Arc::new(FailingAudio));

    // First call (the commit guard) reports current; the second (the rollback
    // guard) reports superseded.
    let calls = std::cell::Cell::new(0u32);
    let is_current = || {
        let call = calls.get();
        calls.set(call + 1);
        call == 0
    };

    let error = service
        .play_track("song-1", Some(PREVIEW_URL), is_current)
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-1"), true).await;
    // Two calls: the commit guard and the rollback guard. A missing rollback
    // would consult the guard once and leave the assertion above green.
    assert_eq!(calls.get(), 2);
}

// `play_track` starts the preview only after the backend accepts the starting
// volume: `set_volume` and `play` are chained, so a backend that cannot set the
// gain must not start the preview at rodio's full-volume default. Pin both the
// surfaced error and the exact command sequence — a regression that issued
// `play` anyway (or set the volume best-effort) would sound the preview while
// reporting failure, and only the recorded calls can catch that.
#[tokio::test]
async fn play_track_does_not_start_the_preview_when_the_volume_cannot_be_set() {
    let audio = Arc::new(RejectingAudio::new(RejectedCommand::SetVolume));
    let (service, state) = service_with_audio(Arc::clone(&audio) as Arc<dyn AudioOutput>);

    let error = service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-1"), false).await;
    assert_eq!(audio.calls(), vec![AudioCall::SetVolume(0.5)]);
}

// `pause` pauses the injected backend as well as clearing the shared playing
// flag; the test pins both halves of that contract.
#[tokio::test]
async fn pause_pauses_the_audio_backend() {
    let (service, state, recording) = recording_service();

    service.play_track("song-1", None, || true).await.unwrap();
    service.pause().await.unwrap();

    assert_eq!(recording.calls(), vec![AudioCall::Pause]);
    assert_playback_state(&state, Some("song-1"), false).await;
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

    service.play_track("song-2", None, || true).await.unwrap();

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
    service.play_track("song-2", None, || true).await.unwrap();
    service.play_track("song-1", None, || false).await.unwrap();

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
        service.play_track("", None, || true),
        service.play_track("   ", None, || true),
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

    let error = service.play_track("", None, || true).await.unwrap_err();

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
    assert_control_character_id_rejected(service.play_track(id, None, || true), IdKind::Track, id)
        .await;

    assert_playback_state(&state, Some("song-1"), true).await;
}

#[tokio::test]
async fn pause_stops_playing_but_keeps_current_track() {
    let (service, state) = service_with_song_1_playing().await;

    service.pause().await.unwrap();

    assert_playback_state(&state, Some("song-1"), false).await;
}

// `resume` drives the injected backend and sets the shared playing flag, the
// other half of `pause`. A `Some` preview is loaded first so the backend has a
// paused player to resume (the service only resumes a loaded preview, or
// restarts a stopped one).
#[tokio::test]
async fn resume_resumes_the_audio_backend() {
    let (service, state, recording) = recording_service_with_preview().await;

    service.pause().await.unwrap();
    service.resume().await.unwrap();

    assert_eq!(
        recording.calls(),
        vec![
            AudioCall::SetVolume(0.5),
            AudioCall::Play(PREVIEW_URL.to_string()),
            AudioCall::Pause,
            AudioCall::Resume
        ]
    );
    assert_playback_state(&state, Some("song-1"), true).await;
}

// `stop` drives the injected backend and clears the shared playing flag while
// keeping `current_track`, exactly as `AppState::stop` does.
#[tokio::test]
async fn stop_stops_the_audio_backend() {
    let (service, state, recording) = recording_service_with_preview().await;

    service.stop().await.unwrap();

    assert_eq!(
        recording.calls(),
        vec![
            AudioCall::SetVolume(0.5),
            AudioCall::Play(PREVIEW_URL.to_string()),
            AudioCall::Stop
        ]
    );
    assert_playback_state(&state, Some("song-1"), false).await;
}

// `pause` documents a backend-first order: a backend that cannot accept the
// pause reports it before shared state claims playback stopped. A regression
// that cleared the shared flag first would leave the Now Playing bar reading
// "paused" while the preview kept sounding, and only a failing backend can
// catch it.
#[tokio::test]
async fn pause_reports_a_backend_failure_without_clearing_playing() {
    let (service, state) =
        service_with_preview_playing(Arc::new(RejectingAudio::new(RejectedCommand::Pause))).await;

    let error = service.pause().await.unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-1"), true).await;
}

// `stop`'s backend-first order, the twin of the pause contract above: a
// rejected stop must not clear the playing flag, or the UI would report a
// stopped transport while the preview still sounds.
#[tokio::test]
async fn stop_reports_a_backend_failure_without_clearing_playing() {
    let (service, state) =
        service_with_preview_playing(Arc::new(RejectingAudio::new(RejectedCommand::Stop))).await;

    let error = service.stop().await.unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-1"), true).await;
}

// `resume_locked` calls the backend before it sets the playing flag, so a
// backend that rejects the resume leaves the transport paused rather than
// claiming a preview that never restarted. The starting point is a loaded but
// paused preview: play it, then pause it successfully.
#[tokio::test]
async fn resume_reports_a_backend_failure_without_claiming_playback() {
    let (service, state) =
        service_with_preview_playing(Arc::new(RejectingAudio::new(RejectedCommand::Resume))).await;
    service.pause().await.unwrap();

    let error = service.resume().await.unwrap_err();

    assert_eq!(error.to_string(), "the audio device is gone");
    assert_playback_state(&state, Some("song-1"), false).await;
}

// The Play/Pause toggle pauses while playing.
#[tokio::test]
async fn toggle_play_pause_pauses_while_playing() {
    let (service, state, recording) = recording_service_with_preview().await;

    service.toggle_play_pause().await.unwrap();

    assert_eq!(
        recording.calls(),
        vec![
            AudioCall::SetVolume(0.5),
            AudioCall::Play(PREVIEW_URL.to_string()),
            AudioCall::Pause
        ]
    );
    assert_playback_state(&state, Some("song-1"), false).await;
}

// The Play/Pause toggle resumes while paused.
#[tokio::test]
async fn toggle_play_pause_resumes_while_paused() {
    let (service, state, recording) = recording_service_with_preview().await;

    service.pause().await.unwrap();
    service.toggle_play_pause().await.unwrap();

    assert_eq!(
        recording.calls(),
        vec![
            AudioCall::SetVolume(0.5),
            AudioCall::Play(PREVIEW_URL.to_string()),
            AudioCall::Pause,
            AudioCall::Resume
        ]
    );
    assert_playback_state(&state, Some("song-1"), true).await;
}

// Stop discards the backend's player, so `resume` cannot resume it; instead
// the service replays the remembered preview URL, restarting it from the
// beginning rather than claiming playback with nothing loaded. Without this,
// Play after Stop would set `is_playing` while the backend held no player —
// the desync this wiring exists to prevent.
#[tokio::test]
async fn resume_after_stop_restarts_the_preview() {
    let (service, state, recording) = recording_service_with_preview().await;

    service.stop().await.unwrap();
    service.resume().await.unwrap();

    assert_eq!(
        recording.calls(),
        vec![
            AudioCall::SetVolume(0.5),
            AudioCall::Play(PREVIEW_URL.to_string()),
            AudioCall::Stop,
            AudioCall::Play(PREVIEW_URL.to_string())
        ]
    );
    assert_playback_state(&state, Some("song-1"), true).await;
}

// With no preview ever played there is nothing to resume or restart, so Play
// leaves the transport stopped rather than claiming playback the backend
// cannot produce.
#[tokio::test]
async fn resume_without_a_preview_leaves_the_transport_stopped() {
    let (service, state, recording) = recording_service();

    service.resume().await.unwrap();

    assert!(recording.calls().is_empty());
    assert_playback_state(&state, None, false).await;
}

// Two Play/Pause toggles can arrive close together (a double-click), and each
// runs as its own task. The service holds the transport lock across the flag
// read, the backend call, and the flag write, so the second toggle observes
// the first's write instead of both reading "playing" and both pausing. This
// test holds the first toggle inside the backend's `pause`, starts the second,
// and proves it has not entered the backend: were the lock dropped before the
// backend call, the second pause would enter concurrently, the gate would fire
// twice, and the call list would end with two `Pause`s and the wrong flag.
#[test]
fn concurrent_toggles_serialize_on_the_transport_lock() {
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let audio = Arc::new(GatedAudio {
        calls: std::sync::Mutex::new(Vec::new()),
        entered: entered_tx,
        release: std::sync::Mutex::new(release_rx),
    });
    let (service, state) = service_with_audio(Arc::clone(&audio) as Arc<dyn AudioOutput>);
    futures::executor::block_on(service.play_track("song-1", Some(PREVIEW_URL), || true)).unwrap();

    let first = service.clone();
    let first = std::thread::spawn(move || {
        futures::executor::block_on(first.toggle_play_pause()).unwrap();
    });
    entered_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the first toggle must reach the backend");

    let second = service.clone();
    let second = std::thread::spawn(move || {
        futures::executor::block_on(second.toggle_play_pause()).unwrap();
    });

    // A second entry while the first is held means the toggles overlapped in
    // the backend. `recv_timeout` returning `Ok` is that failure.
    assert!(
        entered_rx
            .recv_timeout(std::time::Duration::from_millis(200))
            .is_err(),
        "a second toggle entered the backend while the first was still in it"
    );

    release_tx.send(()).unwrap();
    first.join().unwrap();
    second.join().unwrap();

    assert_eq!(
        audio.calls(),
        vec![
            AudioCall::SetVolume(0.5),
            AudioCall::Play(PREVIEW_URL.to_string()),
            AudioCall::Pause,
            AudioCall::Resume
        ]
    );
    assert!(futures::executor::block_on(state.lock()).is_playing);
}

// `pause`'s test above pins its state change; its two sibling stubs,
// `next_track` and `previous_track`, have no production caller at all.
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
    let expected = sign_in_session();

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
    let expected = sign_in_session();

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

// The session-ready signal: `wait_for_session` resolves once a session is
// stored, whether it was already stored before the wait began or arrives
// afterwards. The UI's boot task awaits it to re-issue the artists fetch after
// the background sign-in completes, so both orderings must resolve.

#[tokio::test]
async fn wait_for_session_returns_immediately_when_a_session_is_already_stored() {
    let service = test_service();
    service
        .authenticate_with("dev-token", &|_| Ok(sign_in_session()))
        .unwrap();

    service.wait_for_session().await;
}

#[test]
fn wait_for_session_resolves_when_a_session_is_stored_after_the_wait_starts() {
    use std::task::{Context, Poll};

    let service = test_service();
    let future = service.wait_for_session();
    futures::pin_mut!(future);

    // Poll once so the waiter registers on the notification before the session
    // is stored; the already-stored fast path therefore cannot be what
    // resolves it.
    let waker = futures::task::noop_waker();
    let mut context = Context::from_waker(&waker);
    assert!(future.as_mut().poll(&mut context).is_pending());

    service
        .authenticate_with("dev-token", &|_| Ok(sign_in_session()))
        .unwrap();

    assert!(matches!(
        future.as_mut().poll(&mut context),
        Poll::Ready(())
    ));
}

#[test]
fn a_failed_authentication_leaves_a_stored_session_unchanged() {
    let service = test_service();
    let stored = sign_in_session();
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

#[tokio::test]
async fn get_favorite_artists_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}]}"#,
    );
    let service = signed_in_service(&stub);

    let artists = service.get_favorite_artists().await.unwrap();

    assert_eq!(artists, vec![sample_artist()]);
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/artists");
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
            artist_id: "artist-9".to_string(),
            ..sample_album()
        }]
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/artists/artist-9/albums",
    );
}

#[tokio::test]
async fn get_songs_from_album_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","durationInMillis":210000}}]}"#,
    );
    let service = signed_in_service(&stub);

    let songs = service.get_songs_from_album("album-9").await.unwrap();

    assert_eq!(
        songs,
        vec![Song {
            album_id: "album-9".to_string(),
            ..sample_song()
        }]
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/albums/album-9/tracks",
    );
}

// The search box's backend routes through the REST client when a session is
// stored, so a signed-in user searches the real library rather than the
// sample one. The stub returns a song whose id is not a sample id, so the
// result can only have come from the REST search envelope; the URL pins the
// search endpoint, the term, and the documented page-size maximum.
#[tokio::test]
async fn search_songs_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning(
        r#"{"results":{"library-songs":{"data":[{"id":"rest-song","attributes":{"name":"Remote Result","artistName":"Remote Band"}}]}}}"#,
    );
    let service = signed_in_service(&stub);

    let songs = service.search_songs("remote").await.unwrap();

    assert_eq!(
        songs,
        vec![Song {
            id: "rest-song".to_string(),
            title: "Remote Result".to_string(),
            artist: "Remote Band".to_string(),
            album_id: String::new(),
            duration_ms: 0,
            preview_url: None,
        }]
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/search?term=remote&types=library-songs&limit=25",
    );
}

#[tokio::test]
async fn a_transport_error_propagates_from_a_signed_in_browse_query() {
    let service = signed_in_service(&StubTransport::failing("network down"));

    let error = service.get_favorite_artists().await.unwrap_err();

    assert_eq!(error.to_string(), "network down");
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

// `off_thread` reports the thread's result through a `oneshot` channel, so a
// panic in the blocking work drops the sender without a value. That must
// surface as an error rather than leaving the browse query's `await` pending
// forever; this test drives exactly that branch. The panic prints to stderr on
// the spawned thread, which is expected here.
#[tokio::test]
async fn off_thread_reports_an_error_when_its_work_thread_panics() {
    let result: Result<(), AppleMusicError> = off_thread(|| panic!("work thread panicked")).await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "the library request thread ended without a result"
    );
}

// The REST worker runs on a named thread (like the audio worker), so a panic
// there is attributed to `nostalgia-rest` rather than `<unnamed>`. Pin the
// name: a regression back to a bare `std::thread::spawn` would leave the panic
// test above green (it checks only the error) while the log lost the
// attribution.
#[tokio::test]
async fn off_thread_names_its_work_thread() {
    let name =
        off_thread(|| Ok::<_, AppleMusicError>(std::thread::current().name().map(str::to_string)))
            .await
            .unwrap();

    assert_eq!(name.as_deref(), Some(REST_THREAD_NAME));
}

// `std::thread::spawn` panics when the OS refuses a thread, which would abort
// the player on a browse query. `off_thread` spawns through the fallible
// `Builder::spawn` and reports the refusal through the seam instead; this test
// injects a failing spawner to drive that branch without exhausting the OS.
#[tokio::test]
async fn off_thread_reports_a_thread_spawn_failure_as_a_seam_error() {
    let result = off_thread_with(
        || Ok::<_, AppleMusicError>(()),
        |_work| {
            Err(AppleMusicError::new(
                "spawning the library request thread failed: no more threads",
            ))
        },
    )
    .await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "spawning the library request thread failed: no more threads"
    );
}

// `ip_is_internal` is the shared SSRF guard: `preview_url_problem` applies it
// to an address literal a preview URL spells, and `audio::PublicAddressResolver`
// applies it to every address a preview hostname resolves to. The URL-level
// tests above only reach the branches their sample literals happen to hit, so
// a dropped range check (broadcast, documentation, CGNAT, a v6 form) would be
// invisible. This table pins each branch on both sides: every internal form
// must report `true`, and public v4/v6 addresses — including the addresses
// bordering `100.64.0.0/10` and a public v4-mapped v6 address — must report
// `false`, so the guard cannot pass by rejecting everything.
#[test]
fn ip_is_internal_refuses_every_internal_range_and_allows_public_addresses() {
    for (text, expected) in [
        // IPv4 internal: private, loopback, link-local, unspecified,
        // broadcast, documentation, 0.0.0.0/8, and 100.64.0.0/10.
        ("10.0.0.1", true),
        ("172.16.0.1", true),
        ("192.168.0.1", true),
        ("127.0.0.1", true),
        ("169.254.169.254", true),
        ("0.0.0.0", true),
        ("255.255.255.255", true),
        ("192.0.2.1", true),
        ("198.51.100.1", true),
        ("203.0.113.1", true),
        ("0.1.2.3", true),
        ("100.64.0.1", true),
        ("100.127.255.255", true),
        // IPv4 public: the addresses bordering the CGNAT range and ordinary
        // public hosts.
        ("100.63.255.255", false),
        ("100.128.0.0", false),
        ("8.8.8.8", false),
        ("93.184.216.34", false),
        // IPv6 internal: loopback, unspecified, multicast, unique-local
        // (fc00::/7), link-local (fe80::/10), and v4-mapped internal
        // addresses.
        ("::1", true),
        ("::", true),
        ("ff02::1", true),
        ("fc00::1", true),
        ("fd12:3456::1", true),
        ("fe80::1", true),
        ("::ffff:127.0.0.1", true),
        ("::ffff:10.0.0.1", true),
        // IPv6 public: a public v4-mapped address and ordinary public v6.
        ("::ffff:8.8.8.8", false),
        ("2001:4860:4860::8888", false),
        ("2606:4700:4700::1111", false),
    ] {
        let ip: std::net::IpAddr = text
            .parse()
            .unwrap_or_else(|_| panic!("bad test address {text}"));
        assert_eq!(
            ip_is_internal(ip),
            expected,
            "ip_is_internal({text}) should be {expected}"
        );
    }
}
