//! Test-only fixtures and assertions shared across the crate's unit tests.
//!
//! These are the sample `Artist`/`Album`/`Song`/`Playlist` values and stepping
//! fixtures,
//! the shared preview URL, the sign-in session fixtures, the `"Rock"` preset,
//! the serde-contract assertions, the transport stub, the audio fakes, and the
//! loopback HTTP server fixtures
//! that the `library`, `apple_music`, `ui`, `state`, `equalizer`, and `audio`
//! test suites share.
//! They live in one named module
//! rather than inside `library` so the data model module stays only the model;
//! every item here is compiled only for tests.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::apple_music::AppleMusicError;
use crate::apple_music::rest::HttpTransport;
use crate::audio::{AudioOutput, EqSnapshot};
use crate::equalizer::{PRESETS, Preset};
use crate::library::{Album, Artist, Playlist, Song};
use crate::music_kit_auth::MusicKitSession;

/// The preview asset URL the transport and audio tests load and assert on, and
/// the browse tests map a resource to. It lives here once so a change to the
/// placeholder reaches every suite that names it instead of drifting between
/// per-file copies.
pub(crate) const PREVIEW_URL: &str = "https://example.test/preview.m4a";

/// The three songs on `album-1` that the Previous/Next stepping tests step
/// through, shared by the `ui::transport` and `ui` test suites. Both suites
/// pin the same stepping behavior over the same three-song album; building
/// this list independently in each suite would let a rename or reorder in one
/// fixture silently diverge from the other, so it lives here once, next to the
/// other shared fixtures, and is compiled only for tests.
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
/// Previous on a one-element list). Both need this same list; building it
/// independently in each suite would let a rename in one fixture silently
/// diverge from the other, so it lives here once, next to [`stepping_songs`],
/// and is compiled only for tests.
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
/// need this list, as do
/// `album_selected_clears_the_previous_albums_songs`,
/// `selection_messages_with_a_stale_epoch_or_index_do_nothing`,
/// `track_selected_stale_press_leaves_known_tracks_untouched`, and the three
/// bar tests through `play_song_1_then_browse_to_album_2`; a retitle or id
/// change in one copy would silently diverge from the others, so it lives
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
/// suites. Each suite needs these same objects; building them independently
/// would let a retitle or id change in one fixture silently diverge from the
/// others, so they live here once, next to [`stepping_songs`], and each test
/// only names which one it wants. The browse-mapping tests reuse these,
/// setting the artist or album id from the query where the browse supplies
/// one.
pub(crate) fn sample_artist() -> Artist {
    Artist {
        id: "artist-1".to_string(),
        name: "The Sample Band".to_string(),
    }
}

/// The representative album fixture; [`sample_song`] belongs to it.
pub(crate) fn sample_album() -> Album {
    Album {
        id: "album-1".to_string(),
        title: "First Record".to_string(),
        artist_id: "artist-1".to_string(),
    }
}

/// The representative playlist fixture.
pub(crate) fn sample_playlist() -> Playlist {
    Playlist {
        id: "playlist-1".to_string(),
        name: "Morning Drive".to_string(),
    }
}

/// The representative song fixture; it belongs to [`sample_album`] by
/// [`sample_artist`].
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

/// The session the browser sign-in flow returns in the tests, with
/// recognizable tokens so a test can assert the service stored exactly what
/// the flow produced. The `apple_music` sign-in tests and the `ui` startup
/// test all build this same session, so the token pair lives here once.
pub(crate) fn sign_in_session() -> MusicKitSession {
    MusicKitSession {
        developer_token: "dev-token".to_string(),
        user_token: "user-token".to_string(),
    }
}

/// The session a signed-in service stores, with recognizable tokens so a test
/// can assert the transport saw exactly these credentials. The `apple_music`
/// service tests and the `apple_music::rest` client tests both build it, so
/// the token pair lives here once.
pub(crate) fn rest_session() -> MusicKitSession {
    MusicKitSession {
        developer_token: "developer-token".to_string(),
        user_token: "user-token".to_string(),
    }
}

/// The classic "Rock" preset from [`PRESETS`], shared by the `equalizer`,
/// `state`, and `ui` test suites. Each suite needs the same
/// `.find(|preset| preset.name == "Rock").expect(...)` lookup; without this
/// helper a rename in the table would fail at each call site, each repeating
/// the panic message, so the lookup lives here once and each test only names
/// the curve it wants.
pub(crate) fn rock_preset() -> Preset {
    *PRESETS
        .iter()
        .find(|preset| preset.name == "Rock")
        .expect("the preset table must contain Rock")
}

/// Asserts that `T` requires every field `payload` declares: for each key,
/// removing that one key must fail deserialization, because a payload missing
/// a required field must error rather than silently yield a half-populated
/// value the UI would render as blank data. The four model-type tests
/// (`Artist`, `Album`, `Song`, and `Playlist`) each hand in a full, valid
/// payload, so the walk-every-key loop lives here once. Probing every key
/// matters: a single named field per type leaves the others unpinned, and a
/// `#[serde(default)]` added to a field no probe omitted would clear the
/// suite while blanking that field.
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
/// out-and-back trip through `serde_json` unchanged. The four model-type
/// tests (`Artist`, `Album`, `Song`, and `Playlist`) each pin the same two
/// halves — the exact serialized field names (a `#[serde(rename)]` would pass
/// a round-trip alone but fails this pin), then the round trip via
/// [`assert_round_trips`] — so the serialize-then-compare-then-round-trip
/// sequence lives here once and no caller can pin one half without the
/// other.
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
/// field must be ignored rather than failing the whole parse. The four
/// model-type tests (`Artist`, `Album`, `Song`, and `Playlist`) each probe
/// this contract, so the `from_value`-then-`assert_eq` chain lives here once.
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
/// Both suites need this stub; defining it independently would put the
/// recording contract in two places, so it lives here once and is compiled
/// only for tests.
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

/// Asserts the stub saw exactly one request, to `expected_url` with the
/// [`rest_session`] the browse tests store. The `apple_music` service tests
/// and the `apple_music::rest` client tests both need this same
/// len-and-two-fields check, so it lives here once.
pub(crate) fn assert_single_call(stub: &StubTransport, expected_url: &str) {
    let calls = stub.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, expected_url);
    assert_eq!(calls[0].1, rest_session());
}

/// The commands a [`RecordingAudio`] recorded, in order, so a test can pin
/// exactly what the service handed the audio backend. The derive drops `Eq`
/// because [`AudioCall::SetVolume`], [`AudioCall::SetBalance`], and
/// [`AudioCall::SetEqualizer`] carry an `f32`, which is only `PartialEq`; the
/// only uses compare whole `Vec<AudioCall>`s with `assert_eq!`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AudioCall {
    /// `play` was called with this URL.
    Play(String),
    /// `pause` was called.
    Pause,
    /// `resume` was called.
    Resume,
    /// `stop` was called.
    Stop,
    /// `set_volume` was called with this gain.
    SetVolume(f32),
    /// `set_balance` was called with this balance.
    SetBalance(f32),
    /// `seek_by` was called with this signed offset in seconds.
    SeekBy(i64),
    /// `set_equalizer` was called with this curve.
    SetEqualizer(EqSnapshot),
}

/// Asserts `calls` recorded the four [`AudioCall`]s that starting a preview
/// always emits — the play-time volume set to the default `0.5`, the balance
/// set to the default `0.0`, the default flat/disabled equalizer, then
/// [`PREVIEW_URL`] — followed by `expected`, the calls under test. The
/// `apple_music` service tests and the `ui` transport tests both need that
/// prefix, so it lives here once.
pub(crate) fn assert_preview_calls(calls: &[AudioCall], expected: &[AudioCall]) {
    let mut wanted = vec![
        AudioCall::SetVolume(0.5),
        AudioCall::SetBalance(0.0),
        AudioCall::SetEqualizer(EqSnapshot::default()),
        AudioCall::Play(PREVIEW_URL.to_string()),
    ];
    wanted.extend_from_slice(expected);
    assert_eq!(calls, wanted);
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

    fn set_balance(&self, balance: f32) -> Result<(), AppleMusicError> {
        self.calls
            .lock()
            .unwrap()
            .push(AudioCall::SetBalance(balance));
        Ok(())
    }

    fn seek_by(&self, offset_secs: i64) -> Result<(), AppleMusicError> {
        self.calls
            .lock()
            .unwrap()
            .push(AudioCall::SeekBy(offset_secs));
        Ok(())
    }

    /// Reports a fixed stub position; the recording fake models no real
    /// playback clock, so tests that need a known elapsed value use this one.
    fn position(&self) -> Option<Duration> {
        Some(Duration::from_secs(7))
    }

    fn set_equalizer(&self, equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
        self.calls
            .lock()
            .unwrap()
            .push(AudioCall::SetEqualizer(equalizer));
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

    fn resume(&self) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn set_volume(&self, _volume: f32) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn set_balance(&self, _balance: f32) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn seek_by(&self, _offset_secs: i64) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn set_equalizer(&self, _equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
        Ok(())
    }
}

/// An [`AudioOutput`] whose `seek_by` fails, so a test can pin that the
/// service propagates the backend's error and the UI logs it without
/// scheduling work. `attempts` counts the rejected calls, letting a test prove
/// the message reached the failing backend instead of skipping the seek.
#[derive(Debug, Default)]
pub(crate) struct FailingSeekAudio {
    attempts: AtomicUsize,
}

impl FailingSeekAudio {
    /// How many seek commands the backend rejected.
    pub(crate) fn attempts(&self) -> usize {
        self.attempts.load(Ordering::SeqCst)
    }
}

impl AudioOutput for FailingSeekAudio {
    fn play(&self, _url: &str) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn resume(&self) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn set_volume(&self, _volume: f32) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn set_balance(&self, _balance: f32) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn seek_by(&self, _offset_secs: i64) -> Result<(), AppleMusicError> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        Err(AppleMusicError::new("the audio device is gone"))
    }

    fn set_equalizer(&self, _equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
        Ok(())
    }
}

/// An [`AudioOutput`] whose volume and balance commands fail, so a UI test can
/// pin that each update arm logs the backend's error and still commits its
/// state mutation. The per-command counters let a test prove its arm reached
/// the failing backend instead of skipping the forward, and that the other arm
/// stayed untouched.
#[derive(Debug, Default)]
pub(crate) struct FailingVolumeBalanceAudio {
    volume_attempts: AtomicUsize,
    balance_attempts: AtomicUsize,
}

impl FailingVolumeBalanceAudio {
    /// How many volume commands the backend rejected.
    pub(crate) fn volume_attempts(&self) -> usize {
        self.volume_attempts.load(Ordering::SeqCst)
    }

    /// How many balance commands the backend rejected.
    pub(crate) fn balance_attempts(&self) -> usize {
        self.balance_attempts.load(Ordering::SeqCst)
    }
}

impl AudioOutput for FailingVolumeBalanceAudio {
    fn play(&self, _url: &str) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn resume(&self) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn set_volume(&self, _volume: f32) -> Result<(), AppleMusicError> {
        self.volume_attempts.fetch_add(1, Ordering::SeqCst);
        Err(AppleMusicError::new("the audio device is gone"))
    }

    fn set_balance(&self, _balance: f32) -> Result<(), AppleMusicError> {
        self.balance_attempts.fetch_add(1, Ordering::SeqCst);
        Err(AppleMusicError::new("the audio device is gone"))
    }

    fn seek_by(&self, _offset_secs: i64) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn set_equalizer(&self, _equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
        Ok(())
    }
}

/// An [`AudioOutput`] whose equalizer command fails, so a UI test can pin that
/// the equalizer update arm logs the backend's error and still commits its
/// state mutation. `attempts` counts the rejected calls, letting the test
/// prove the message reached the failing backend instead of skipping it.
#[derive(Debug, Default)]
pub(crate) struct FailingEqualizerAudio {
    attempts: AtomicUsize,
}

impl FailingEqualizerAudio {
    /// How many equalizer commands the backend rejected.
    pub(crate) fn attempts(&self) -> usize {
        self.attempts.load(Ordering::SeqCst)
    }
}

impl AudioOutput for FailingEqualizerAudio {
    fn play(&self, _url: &str) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn resume(&self) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn set_volume(&self, _volume: f32) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn set_balance(&self, _balance: f32) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn seek_by(&self, _offset_secs: i64) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn set_equalizer(&self, _equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        Err(AppleMusicError::new("the audio device is gone"))
    }
}

/// An [`AudioOutput`] whose `play`, volume, and balance commands succeed but
/// whose pause, resume, and stop commands fail, so a UI test can reach a
/// loaded preview and then drive each transport arm through
/// `loading::transport_into`'s error branch. `attempts` counts the rejected
/// calls, letting a test prove the arm actually reached the failing backend
/// instead of skipping it.
#[derive(Debug, Default)]
pub(crate) struct FailingTransportAudio {
    attempts: AtomicUsize,
}

impl FailingTransportAudio {
    /// How many pause/resume/stop commands the backend rejected.
    pub(crate) fn attempts(&self) -> usize {
        self.attempts.load(Ordering::SeqCst)
    }

    fn reject(&self) -> Result<(), AppleMusicError> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        Err(AppleMusicError::new("the audio device is gone"))
    }
}

impl AudioOutput for FailingTransportAudio {
    fn play(&self, _url: &str) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        self.reject()
    }

    fn resume(&self) -> Result<(), AppleMusicError> {
        self.reject()
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        self.reject()
    }

    fn set_volume(&self, _volume: f32) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn set_balance(&self, _balance: f32) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn seek_by(&self, _offset_secs: i64) -> Result<(), AppleMusicError> {
        Ok(())
    }

    fn set_equalizer(&self, _equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
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

/// Spawns a thread that accepts the first request to `listener` and returns
/// its text, giving up after one second so "no request arrived" is a bounded
/// result rather than a hang. The listener is switched to nonblocking and
/// polled; a `WouldBlock` before the deadline sleeps, and the deadline returns
/// `None`.
///
/// The `apple_music::rest` and `audio` suites' redirect tests both stand up a
/// "foreign" listener this way to prove a redirect was not followed: joining
/// the handle and finding `None` means no request reached the redirect target.
pub(crate) fn record_first_request(
    listener: TcpListener,
) -> std::thread::JoinHandle<Option<String>> {
    listener.set_nonblocking(true).unwrap();
    std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => return Some(read_some_request(&mut stream)),
                Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return None;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("foreign listener failed: {error}"),
            }
        }
    })
}

/// Serves exactly one HTTP response on a fresh loopback listener: binds,
/// accepts a single connection, reads whatever request arrived, writes
/// `response` verbatim, and closes. Returns the bound address so a test can
/// point an HTTP client at it without rebuilding the accept/read/write
/// scaffolding. Shared by the `apple_music::rest` and `audio` suites.
///
/// The recording-free variant of [`serve_one_response_capturing_request`]:
/// the request it read is dropped, so a test that needs those bytes calls
/// that helper instead.
pub(crate) fn serve_one_response(response: &str) -> SocketAddr {
    let (address, _request) = serve_one_response_capturing_request(response);
    address
}

/// Serves exactly one HTTP response on a fresh loopback listener and returns
/// the bound address together with a receiver for the request text it read, so
/// a test can assert on the bytes the client sent as well as the response. The
/// accept/read/write scaffolding lives here once; [`serve_one_response`] is
/// the recording-free variant that drops the receiver. The `apple_music::rest`
/// request-capture test calls it.
pub(crate) fn serve_one_response_capturing_request(
    response: &str,
) -> (SocketAddr, std::sync::mpsc::Receiver<String>) {
    let (listener, address) = loopback_listener();
    let response = response.to_string();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_some_request(&mut stream);
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
        let _ = sender.send(request);
    });
    (address, receiver)
}
