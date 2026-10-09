//! Test-only fixtures and assertions shared across the crate's unit tests.
//!
//! These are the sample `Artist`/`Album`/`Song` values and stepping fixtures,
//! the `"Rock"` preset, the serde-contract assertions, the transport stub, the
//! audio fakes, and the loopback HTTP server fixtures that the `library`,
//! `apple_music`, `ui`, `state`, `equalizer`, and `audio` test suites share.
//! They live in one named module
//! rather than inside `library` so the data model module stays only the model;
//! every item here is compiled only for tests.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::apple_music::AppleMusicError;
use crate::apple_music::rest::HttpTransport;
use crate::audio::AudioOutput;
use crate::equalizer::{PRESETS, Preset};
use crate::library::{Album, Artist, Song};
use crate::music_kit_auth::MusicKitSession;

/// The three songs on `album-1` that the Previous/Next stepping tests step
/// through, shared by the `ui::transport` and `ui` test suites. Both suites
/// pin the same stepping behavior over the same three-song album and used to
/// build this list independently — a rename or reorder in one fixture would
/// silently diverge from the other — so it lives here once, next to the other
/// shared fixtures, and is compiled only for tests.
pub(crate) fn stepping_songs() -> Vec<Song> {
    vec![
        Song {
            id: "song-1".to_string(),
            title: "One".to_string(),
            artist: "The Sample Band".to_string(),
            album_id: "album-1".to_string(),
            duration_ms: 210_000,
            preview_url: None,
        },
        Song {
            id: "song-2".to_string(),
            title: "Two".to_string(),
            artist: "The Sample Band".to_string(),
            album_id: "album-1".to_string(),
            duration_ms: 125_000,
            preview_url: None,
        },
        Song {
            id: "song-3".to_string(),
            title: "Three".to_string(),
            artist: "The Sample Band".to_string(),
            album_id: "album-1".to_string(),
            duration_ms: 95_000,
            preview_url: None,
        },
    ]
}

/// The one-song album the single-song stepping tests step through, shared by
/// the two `ui::transport` tests that pin the single-song edge cases (Next and
/// Previous on a one-element list). Both used to build this same list
/// independently — a rename in one fixture would silently diverge from the
/// other — so it lives here once, next to [`stepping_songs`], and is compiled
/// only for tests.
pub(crate) fn single_song_album() -> Vec<Song> {
    vec![Song {
        id: "song-1".to_string(),
        title: "Only".to_string(),
        artist: "The Sample Band".to_string(),
        album_id: "album-1".to_string(),
        duration_ms: 180_000,
        preview_url: None,
    }]
}

/// The one-song album the browse-away tests land on after leaving `album-1`:
/// album-2's "song-4" / "B-side".
/// `songs_loaded_replaces_the_previous_albums_songs` and
/// `now_playing_label_keeps_the_track_name_after_browsing_to_another_album`
/// first built this same list inline, and it now also backs
/// `album_selected_clears_the_previous_albums_songs`,
/// `selection_messages_with_a_stale_epoch_or_index_do_nothing`,
/// `track_selected_stale_press_leaves_known_tracks_untouched`, and the three
/// bar tests through `play_song_1_then_browse_to_album_2` — a retitle or id
/// change in one copy would silently diverge from the others — so it lives
/// here once, next to [`single_song_album`], and is compiled only for tests.
pub(crate) fn second_album_songs() -> Vec<Song> {
    vec![Song {
        id: "song-4".to_string(),
        title: "B-side".to_string(),
        artist: "The Sample Band".to_string(),
        album_id: "album-2".to_string(),
        duration_ms: 95_000,
        preview_url: None,
    }]
}

/// A single representative artist, album, and song, shared by the `library`,
/// `apple_music` (including its `rest` client), `ui::views`, and `ui` test
/// suites. Each suite used to build these same objects independently — a
/// retitle or id change in one fixture would silently diverge from the
/// others — so they live here once, next to [`stepping_songs`], and each test
/// only names which one it wants. The browse-mapping tests reuse these,
/// setting the artist or album id from the query where the browse supplies
/// one.
pub(crate) fn sample_artist() -> Artist {
    Artist {
        id: "artist-1".to_string(),
        name: "The Sample Band".to_string(),
    }
}

pub(crate) fn sample_album() -> Album {
    Album {
        id: "album-1".to_string(),
        title: "First Record".to_string(),
        artist_id: "artist-1".to_string(),
    }
}

pub(crate) fn sample_song() -> Song {
    Song {
        id: "song-1".to_string(),
        title: "Opening".to_string(),
        artist: "The Sample Band".to_string(),
        album_id: "album-1".to_string(),
        duration_ms: 210_000,
        preview_url: None,
    }
}

/// The classic "Rock" preset from [`PRESETS`], shared by the `equalizer`,
/// `state`, and `ui` test suites. Each suite used to look it up inline with
/// the same `.find(|preset| preset.name == "Rock").expect(...)` — a rename in
/// the table would then fail in five places, each repeating the panic
/// message — so the lookup lives here once and each test only names the curve
/// it wants.
pub(crate) fn rock_preset() -> Preset {
    *PRESETS
        .iter()
        .find(|preset| preset.name == "Rock")
        .expect("the preset table must contain Rock")
}

/// Asserts that `T` requires every field `payload` declares: for each key,
/// removing that one key must fail deserialization, because a payload missing
/// a required field must error rather than silently yield a half-populated
/// value the UI would render as blank data. The three model-type tests
/// (`Artist`, `Album`, and `Song`) each hand in a full, valid payload, so the
/// walk-every-key loop lives here once. Probing every key matters: a single
/// named field per type leaves the others unpinned, and a `#[serde(default)]`
/// added to a field no probe omitted would clear the suite while blanking
/// that field.
pub(crate) fn assert_every_field_required<T>(payload: serde_json::Value)
where
    T: serde::de::DeserializeOwned,
{
    let object = payload
        .as_object()
        .expect("a full payload must be a JSON object");
    assert!(
        !object.is_empty(),
        "a full payload must declare at least one field"
    );
    for key in object.keys() {
        let mut without = object.clone();
        without.remove(key);
        assert!(
            serde_json::from_value::<T>(serde_json::Value::Object(without)).is_err(),
            "a payload missing required field {key:?} must be rejected"
        );
    }
}

/// Asserts that `value` serializes to exactly `expected` and then survives an
/// out-and-back trip through `serde_json` unchanged. The three model-type
/// tests (`Artist`, `Album`, and `Song`) each pin the same two halves — the
/// exact serialized field names (a `#[serde(rename)]` would pass a round-trip
/// alone but fails this pin), then the round trip via [`assert_round_trips`] —
/// so the serialize-then-compare-then-round-trip sequence lives here once and
/// no caller can pin one half without the other.
pub(crate) fn assert_serializes_as<T>(value: T, expected: serde_json::Value)
where
    T: PartialEq + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned,
{
    assert_eq!(serde_json::to_value(&value).unwrap(), expected);
    assert_round_trips(value);
}

/// Asserts that `value` survives an out-and-back trip through `serde_json`
/// unchanged: serialize it, deserialize the result, and compare. The
/// round-trip half of [`assert_serializes_as`], kept separate so the chain has
/// one named implementation. Private to this module: [`assert_serializes_as`]
/// is the only entry point, so no caller can pin the round trip alone.
fn assert_round_trips<T>(value: T)
where
    T: PartialEq + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned,
{
    let back: T = serde_json::from_value(serde_json::to_value(&value).unwrap()).unwrap();
    assert_eq!(back, value);
}

/// Asserts that `payload` deserializes as `T` to exactly `expected`, proving
/// serde's default tolerance of fields beyond `T`'s declared set: a real
/// Apple Music payload carries more than the model's fields, so an unknown
/// field must be ignored rather than failing the whole parse. The three
/// model-type tests (`Artist`, `Album`, and `Song`) each probe this
/// contract, so the `from_value`-then-`assert_eq` chain lives here once.
pub(crate) fn assert_unknown_fields_tolerated<T>(payload: serde_json::Value, expected: T)
where
    T: PartialEq + std::fmt::Debug + serde::de::DeserializeOwned,
{
    let parsed: T = serde_json::from_value(payload).expect("unknown fields must be tolerated");
    assert_eq!(parsed, expected);
}

/// Asserts that `items` yield exactly the expected ids, in order.
///
/// The browse-query tests in the `apple_music` and `ui` suites each fetch a
/// list and pin its exact contents by mapping the list to its ids and
/// comparing; only the id projection and the expected ids differ, so the
/// map-to-ids-then-compare chain lives here once and each call site only
/// names its list and expectation. Comparing the ids *in order* is what
/// catches a dropped or reordered element that a bare non-empty assertion
/// would miss. The `sample_library` suite's `assert_id_label_pairs` is a
/// distinct helper — it pins each item's secondary label alongside its id,
/// not the id alone — so it stays local to that suite.
pub(crate) fn assert_ids<T>(items: &[T], id: impl Fn(&T) -> &str, expected: &[&str]) {
    let ids: Vec<&str> = items.iter().map(id).collect();
    assert_eq!(ids, expected);
}

/// A transport stub shared by the `apple_music` service tests and the
/// `apple_music::rest` client tests: it records every `(url, session)` it is
/// handed and answers with its canned result. Cloning it shares the recording
/// and the queued responses, so a test keeps a handle after the service or
/// [`RestLibrary`](crate::apple_music::rest::RestLibrary) boxes the clone.
/// Both suites used to define this stub independently, so the recording
/// contract lived in two places; it lives here once and is compiled only for
/// tests.
#[derive(Clone)]
pub(crate) struct StubTransport {
    /// The response repeated once [`Self::queue`] is exhausted.
    result: Result<String, AppleMusicError>,
    /// The responses served in order, front first, before `result` takes over.
    /// Empty for the single-response stubs, so they answer every call with
    /// `result`; `returning_bodies` fills it for a paginated fetch.
    queue: Arc<Mutex<VecDeque<Result<String, AppleMusicError>>>>,
    calls: Arc<Mutex<Vec<(String, MusicKitSession)>>>,
}

impl StubTransport {
    /// A stub that answers every request with `body`.
    pub(crate) fn returning(body: &str) -> Self {
        Self::with_result(Ok(body.to_string()))
    }

    /// A stub that fails every request with `message`.
    pub(crate) fn failing(message: &str) -> Self {
        Self::with_result(Err(AppleMusicError::new(message)))
    }

    /// A stub that answers the first calls with `bodies` in order and then
    /// repeats the last body for every later call. `returning` is the
    /// single-body case; this is the multi-response case a paginated fetch
    /// needs.
    pub(crate) fn returning_bodies(bodies: &[&str]) -> Self {
        let responses: Vec<Result<String, AppleMusicError>> =
            bodies.iter().map(|body| Ok((*body).to_string())).collect();
        Self::returning_results(&responses)
    }

    /// A stub that answers the first calls with `responses` in order and then
    /// repeats the last for every later call. [`Self::returning_bodies`] is
    /// the all-success shorthand; this is the mixed case a test needs when an
    /// earlier page of a paginated fetch succeeds and a later one fails, so
    /// the queue can carry an error rather than only bodies.
    pub(crate) fn returning_results(responses: &[Result<String, AppleMusicError>]) -> Self {
        let responses: VecDeque<Result<String, AppleMusicError>> =
            responses.iter().cloned().collect();
        let last = responses
            .back()
            .cloned()
            .unwrap_or_else(|| Ok(String::new()));
        Self {
            result: last,
            queue: Arc::new(Mutex::new(responses)),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// A stub with an empty queue, so `result` answers every call.
    fn with_result(result: Result<String, AppleMusicError>) -> Self {
        Self {
            result,
            queue: Arc::new(Mutex::new(VecDeque::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// The requests recorded so far, oldest first.
    pub(crate) fn calls(&self) -> Vec<(String, MusicKitSession)> {
        self.calls.lock().unwrap().clone()
    }
}

impl HttpTransport for StubTransport {
    fn get(&self, url: &str, session: &MusicKitSession) -> Result<String, AppleMusicError> {
        self.calls
            .lock()
            .unwrap()
            .push((url.to_string(), session.clone()));
        self.queue
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| self.result.clone())
    }
}

/// The commands a [`RecordingAudio`] recorded, in order, so a test can pin
/// exactly what the service handed the audio backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AudioCall {
    /// `play` was called with this URL.
    Play(String),
    /// `pause` was called.
    Pause,
    /// `stop` was called.
    Stop,
}

/// A recording [`AudioOutput`] so a test observes the commands issued to the
/// backend without touching an audio device. The `apple_music`, `ui`, and
/// `audio` suites all need one, so the fake lives here once.
#[derive(Debug, Default)]
pub(crate) struct RecordingAudio {
    calls: Mutex<Vec<AudioCall>>,
}

impl RecordingAudio {
    /// A snapshot of the recorded calls, cloned out from under the mutex.
    pub(crate) fn calls(&self) -> Vec<AudioCall> {
        self.calls.lock().unwrap().clone()
    }
}

impl AudioOutput for RecordingAudio {
    fn play(&self, url: &str) -> Result<(), AppleMusicError> {
        self.calls
            .lock()
            .unwrap()
            .push(AudioCall::Play(url.to_string()));
        Ok(())
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        self.calls.lock().unwrap().push(AudioCall::Pause);
        Ok(())
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        self.calls.lock().unwrap().push(AudioCall::Stop);
        Ok(())
    }
}

/// An [`AudioOutput`] whose `play` fails, so a test can pin that the service
/// propagates the backend's error.
#[derive(Debug, Default)]
pub(crate) struct FailingAudio;

impl AudioOutput for FailingAudio {
    fn play(&self, _url: &str) -> Result<(), AppleMusicError> {
        Err(AppleMusicError::new("the audio device is gone"))
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        Ok(())
    }
}

/// Binds a loopback listener, returning it with the address it bound, so a
/// test can stand up a loopback server on an ephemeral port without repeating
/// the bind. [`serve_one_response`] and the `apple_music::rest` suite's
/// redirect and request-capture tests call it.
pub(crate) fn loopback_listener() -> (TcpListener, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    (listener, address)
}

/// Reads whatever one request has sent within a one-second bound and returns
/// it lossily as text, so a test can look for a header without a full parse.
pub(crate) fn read_some_request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let mut buffer = [0u8; 4096];
    let read = stream.read(&mut buffer).unwrap_or(0);
    String::from_utf8_lossy(&buffer[..read]).into_owned()
}

/// Serves exactly one HTTP response on a fresh loopback listener: binds,
/// accepts a single connection, reads whatever request arrived, writes
/// `response` verbatim, and closes. Returns the bound address so a test can
/// point an HTTP client at it without rebuilding the accept/read/write
/// scaffolding. Shared by the `apple_music::rest` and `audio` suites.
pub(crate) fn serve_one_response(response: &str) -> SocketAddr {
    let (listener, address) = loopback_listener();
    let response = response.to_string();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = read_some_request(&mut stream);
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });
    address
}
