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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::apple_music::AppleMusicError;
use crate::apple_music::rest::HttpTransport;
use crate::audio::{AudioOutput, EqSnapshot, PlaybackFormat, Spectrum};
use crate::equalizer::{BAND_COUNT, PRESETS, Preset};
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
            album: "First Record".to_string(),
            year: None,
            album_id: "album-1".to_string(),
            duration_ms: 210_000,
            preview_url: None,
        },
        Song {
            id: "song-2".to_string(),
            title: "Two".to_string(),
            artist: "The Sample Band".to_string(),
            album: "First Record".to_string(),
            year: None,
            album_id: "album-1".to_string(),
            duration_ms: 125_000,
            preview_url: None,
        },
        Song {
            id: "song-3".to_string(),
            title: "Three".to_string(),
            artist: "The Sample Band".to_string(),
            album: "First Record".to_string(),
            year: None,
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
        album: "First Record".to_string(),
        year: None,
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
        album: "Second Record".to_string(),
        year: None,
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
        album_count: 0,
    }
}

/// The representative album fixture; [`sample_song`] belongs to it.
pub(crate) fn sample_album() -> Album {
    Album {
        id: "album-1".to_string(),
        title: "First Record".to_string(),
        artist_id: "artist-1".to_string(),
        artist: "The Sample Band".to_string(),
        duration_ms: 0,
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
        album: "First Record".to_string(),
        year: None,
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
/// The browse-query tests in the `apple_music`, `apple_music::rest` client,
/// and `ui` suites each fetch a list and pin its exact contents by mapping the
/// list to its ids and comparing; only the id projection and the expected ids
/// differ, so the map-to-ids-then-compare chain lives here once and each call
/// site only names its list and expectation. Comparing the ids *in order* is what
/// catches a dropped or reordered element that a bare non-empty assertion
/// would miss. The `sample_library` suite's `assert_id_label_pairs` is a
/// distinct helper — it pins each item's secondary label alongside its id,
/// not the id alone — so it stays local to that suite.
pub(crate) fn assert_ids<T>(items: &[T], id: impl Fn(&T) -> &str, expected: &[&str]) {
    let ids: Vec<&str> = items.iter().map(id).collect();
    assert_eq!(ids, expected);
}

/// A transport stub shared by the `apple_music` service tests, the
/// `apple_music::rest` client tests, and the `ui` player-construction helpers:
/// it records every `(url, session)` it is handed and answers with its canned
/// result. Cloning it shares the recording and the queued responses, so a test
/// keeps a handle after the service or
/// [`RestLibrary`](crate::apple_music::rest::RestLibrary) boxes the clone.
/// All three suites need this stub; defining it independently would put the
/// recording contract in more than one place, so it lives here once and is
/// compiled only for tests.
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
    /// `seek_to` was called with this absolute position.
    SeekTo(Duration),
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

/// The fixed spectrum [`RecordingAudio`] reports: one distinctive nonzero
/// band, so a test can pin that the UI draws what the backend's loaded preview
/// reports rather than an all-zero fallback.
pub(crate) fn recording_spectrum() -> Spectrum {
    let mut bands = [0.0; BAND_COUNT];
    bands[2] = 0.5;
    Spectrum { bands }
}

/// Appends `call` to a fake backend's recorded calls and reports success. The
/// recording [`AudioOutput`] fakes in this module and the `apple_music` tests
/// all share this lock-and-push body, so a change to how a command is logged
/// happens once.
pub(crate) fn record_call(
    calls: &Mutex<Vec<AudioCall>>,
    call: AudioCall,
) -> Result<(), AppleMusicError> {
    calls.lock().unwrap().push(call);
    Ok(())
}

/// A recording [`AudioOutput`] so a test observes the commands issued to the
/// backend without touching an audio device. The `apple_music`, `ui`, and
/// `audio` suites all need one, so the fake lives here once.
#[derive(Debug, Default)]
pub(crate) struct RecordingAudio {
    calls: Mutex<Vec<AudioCall>>,
    finished: AtomicBool,
}

impl RecordingAudio {
    /// A snapshot of the recorded calls, cloned out from under the mutex.
    pub(crate) fn calls(&self) -> Vec<AudioCall> {
        self.calls.lock().unwrap().clone()
    }

    /// Appends `call` to the recorded calls and reports success, the body every
    /// accepted [`AudioOutput`] command shares.
    fn record(&self, call: AudioCall) -> Result<(), AppleMusicError> {
        record_call(&self.calls, call)
    }

    /// Sets the finished state [`AudioOutput::is_finished`] reports, so a test
    /// can drive both values of a preview's completion without a real player.
    pub(crate) fn set_finished(&self, finished: bool) {
        self.finished.store(finished, Ordering::Relaxed);
    }
}

impl AudioOutput for RecordingAudio {
    fn play(&self, url: &str) -> Result<(), AppleMusicError> {
        self.record(AudioCall::Play(url.to_string()))
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        self.record(AudioCall::Pause)
    }

    fn resume(&self) -> Result<(), AppleMusicError> {
        self.record(AudioCall::Resume)
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        self.record(AudioCall::Stop)
    }

    fn set_volume(&self, volume: f32) -> Result<(), AppleMusicError> {
        self.record(AudioCall::SetVolume(volume))
    }

    fn set_balance(&self, balance: f32) -> Result<(), AppleMusicError> {
        self.record(AudioCall::SetBalance(balance))
    }

    fn seek_by(&self, offset_secs: i64) -> Result<(), AppleMusicError> {
        self.record(AudioCall::SeekBy(offset_secs))
    }

    fn seek_to(&self, position: Duration) -> Result<(), AppleMusicError> {
        self.record(AudioCall::SeekTo(position))
    }

    /// Reports a fixed stub position; the recording fake models no real
    /// playback clock, so tests that need a known elapsed value use this one.
    fn position(&self) -> Option<Duration> {
        Some(Duration::from_secs(7))
    }

    /// Reports a fixed stub format, so a test can pin that the UI displays
    /// what the backend's loaded preview reports.
    fn playback_format(&self) -> Option<PlaybackFormat> {
        Some(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: Some(256),
        })
    }

    /// Reports a fixed stub spectrum, so the LCD visualizer has known bands
    /// and a test can pin what the backend's loaded preview reports.
    fn spectrum(&self) -> Option<Spectrum> {
        Some(recording_spectrum())
    }

    /// Reports a fixed stub preview length, so the seek bar has a scale and a
    /// test can pin what the backend's loaded preview reports.
    fn duration(&self) -> Option<Duration> {
        Some(Duration::from_secs(30))
    }

    /// Reports the settable finished state, so a test can drive a preview's
    /// natural end without a real player.
    fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }

    fn set_equalizer(&self, equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
        self.record(AudioCall::SetEqualizer(equalizer))
    }
}

/// The commands an [`AudioOutput`] fake refuses, so a test can name the
/// backend failure it drives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FailingCommand {
    /// `play`.
    Play,
    /// `pause`, `resume`, and `stop`. The three transport commands share one
    /// counter, because a test that drives them wants one total.
    Transport,
    /// `set_volume`.
    Volume,
    /// `set_balance`.
    Balance,
    /// `seek_by`.
    Seek,
    /// `set_equalizer`.
    Equalizer,
}

/// An [`AudioOutput`] that refuses the commands a test names and accepts the
/// rest, so a test can pin that the service propagates the backend's error.
///
/// The `apple_music` and `ui` suites each need a backend that fails one
/// command — a play, a seek, the volume and balance forwards, the equalizer
/// forward, and the transport commands — and each of those fakes implemented
/// every [`AudioOutput`] method to accept the commands it did not fail.
/// One fake serves all of them here: a constructor names the commands it
/// refuses, and the per-command counters let a test prove its arm reached the
/// failing backend instead of skipping the forward, and that the other arm
/// stayed untouched.
#[derive(Debug)]
pub(crate) struct FailingAudio {
    /// The commands this fake refuses; every other command succeeds.
    failing: Vec<FailingCommand>,
    play_attempts: AtomicUsize,
    transport_attempts: AtomicUsize,
    volume_attempts: AtomicUsize,
    balance_attempts: AtomicUsize,
    seek_attempts: AtomicUsize,
    equalizer_attempts: AtomicUsize,
}

impl FailingAudio {
    /// A fake whose `play` fails, so a test can pin that the service
    /// propagates the backend's error.
    pub(crate) fn failing_play() -> Self {
        Self::refusing(&[FailingCommand::Play])
    }

    /// A fake whose `seek_by` fails, so a test can pin that the service
    /// propagates the backend's error and the UI logs it without scheduling
    /// work.
    pub(crate) fn failing_seek() -> Self {
        Self::refusing(&[FailingCommand::Seek])
    }

    /// A fake whose volume and balance commands fail, so a UI test can pin
    /// that each update arm logs the backend's error and still commits its
    /// state mutation.
    pub(crate) fn failing_volume_and_balance() -> Self {
        Self::refusing(&[FailingCommand::Volume, FailingCommand::Balance])
    }

    /// A fake whose equalizer command fails, so a UI test can pin that the
    /// equalizer update arm logs the backend's error and still commits its
    /// state mutation.
    pub(crate) fn failing_equalizer() -> Self {
        Self::refusing(&[FailingCommand::Equalizer])
    }

    /// A fake whose pause, resume, and stop commands fail, so a UI test can
    /// reach a loaded preview and then drive each transport arm through
    /// `loading::transport_into`'s error branch.
    pub(crate) fn failing_transport() -> Self {
        Self::refusing(&[FailingCommand::Transport])
    }

    /// A fake that refuses exactly the commands in `commands`.
    fn refusing(commands: &[FailingCommand]) -> Self {
        Self {
            failing: commands.to_vec(),
            play_attempts: AtomicUsize::new(0),
            transport_attempts: AtomicUsize::new(0),
            volume_attempts: AtomicUsize::new(0),
            balance_attempts: AtomicUsize::new(0),
            seek_attempts: AtomicUsize::new(0),
            equalizer_attempts: AtomicUsize::new(0),
        }
    }

    /// How many times the backend refused `command`. Only refusals are
    /// counted, so a command this fake accepts always reads zero.
    pub(crate) fn attempts(&self, command: FailingCommand) -> usize {
        self.counter(command).load(Ordering::SeqCst)
    }

    /// The counter that records the refusals of `command`.
    fn counter(&self, command: FailingCommand) -> &AtomicUsize {
        match command {
            FailingCommand::Play => &self.play_attempts,
            FailingCommand::Transport => &self.transport_attempts,
            FailingCommand::Volume => &self.volume_attempts,
            FailingCommand::Balance => &self.balance_attempts,
            FailingCommand::Seek => &self.seek_attempts,
            FailingCommand::Equalizer => &self.equalizer_attempts,
        }
    }

    /// Refuses `command` when this fake fails it, counting the refusal; the
    /// commands it does not fail succeed.
    fn attempt(&self, command: FailingCommand) -> Result<(), AppleMusicError> {
        if !self.failing.contains(&command) {
            return Ok(());
        }
        self.counter(command).fetch_add(1, Ordering::SeqCst);
        Err(AppleMusicError::new("the audio device is gone"))
    }
}

impl AudioOutput for FailingAudio {
    fn play(&self, _url: &str) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Play)
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Transport)
    }

    fn resume(&self) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Transport)
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Transport)
    }

    fn set_volume(&self, _volume: f32) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Volume)
    }

    fn set_balance(&self, _balance: f32) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Balance)
    }

    fn seek_by(&self, _offset_secs: i64) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Seek)
    }

    fn seek_to(&self, _position: Duration) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Seek)
    }

    fn set_equalizer(&self, _equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
        self.attempt(FailingCommand::Equalizer)
    }
}

/// Binds a loopback listener, returning it with the address it bound, so a
/// test can stand up a loopback server on an ephemeral port without repeating
/// the bind. [`serve_one_response`], the `apple_music::rest` redirect and
/// request-capture tests, and the `audio` preview-download tests call it.
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

/// The first request recorded by [`record_first_request`], together with the
/// name of the thread that recorded it so a test can pin the helper's thread
/// name instead of leaving it unobservable.
pub(crate) struct RecordedFirstRequest {
    /// The request text, or `None` when no request arrived within the bound.
    pub(crate) request: Option<String>,
    /// The recording thread's name, read on that thread.
    pub(crate) thread_name: Option<String>,
}

/// Spawns a thread that accepts the first request to `listener` and returns
/// its text, giving up after one second so "no request arrived" is a bounded
/// result rather than a hang. The listener is switched to nonblocking and
/// polled; a `WouldBlock` before the deadline sleeps, and the deadline returns
/// a record with no request.
///
/// The `apple_music::rest` and `audio` suites' redirect tests both stand up a
/// "foreign" listener this way to prove a redirect was not followed: joining
/// the handle and finding no request means no request reached the redirect
/// target.
pub(crate) fn record_first_request(
    listener: TcpListener,
) -> std::thread::JoinHandle<RecordedFirstRequest> {
    listener.set_nonblocking(true).unwrap();
    std::thread::Builder::new()
        .name("nostalgia-test-listener".to_string())
        .spawn(move || {
            let thread_name = std::thread::current().name().map(str::to_string);
            let deadline = Instant::now() + Duration::from_secs(1);
            loop {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        return RecordedFirstRequest {
                            request: Some(read_some_request(&mut stream)),
                            thread_name,
                        };
                    }
                    Err(ref error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() >= deadline =>
                    {
                        return RecordedFirstRequest {
                            request: None,
                            thread_name,
                        };
                    }
                    Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("foreign listener failed: {error}"),
                }
            }
        })
        .unwrap()
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

/// [`serve_one_response`] with a binary response, so a test can serve a body
/// that is not valid UTF-8 (a gzip stream, say). Otherwise identical: binds,
/// accepts one connection, reads whatever request arrived, writes `response`
/// verbatim, and closes.
pub(crate) fn serve_one_response_bytes(response: &[u8]) -> SocketAddr {
    spawn_one_shot_server(response.to_vec()).0
}

/// The request served by [`serve_one_response_capturing_request`], together
/// with the name of the thread that served it so a test can pin the helper's
/// thread name instead of leaving it unobservable.
pub(crate) struct CapturedRequest {
    /// The request text the server read.
    pub(crate) request: String,
    /// The serving thread's name, read on that thread.
    pub(crate) thread_name: Option<String>,
}

/// Serves exactly one HTTP response on a fresh loopback listener and returns
/// the bound address together with a receiver for the request text it read, so
/// a test can assert on the bytes the client sent as well as the response. The
/// accept/read/write scaffolding lives here once; [`serve_one_response`] is
/// the recording-free variant that drops the receiver. The `apple_music::rest`
/// request-capture test calls it.
pub(crate) fn serve_one_response_capturing_request(
    response: &str,
) -> (SocketAddr, std::sync::mpsc::Receiver<CapturedRequest>) {
    spawn_one_shot_server(response.as_bytes().to_vec())
}

/// The shared one-shot server behind the `serve_one_response*` helpers: a
/// named thread accepts one connection, reads the request, writes `response`
/// verbatim, and reports the request on the returned receiver (which callers
/// that do not need it simply drop).
fn spawn_one_shot_server(
    response: Vec<u8>,
) -> (SocketAddr, std::sync::mpsc::Receiver<CapturedRequest>) {
    let (listener, address) = loopback_listener();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("nostalgia-test-server".to_string())
        .spawn(move || {
            let thread_name = std::thread::current().name().map(str::to_string);
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_some_request(&mut stream);
            let _ = stream.write_all(&response);
            let _ = stream.flush();
            let _ = sender.send(CapturedRequest {
                request,
                thread_name,
            });
        })
        .unwrap();
    (address, receiver)
}

/// Builds a gzip stream with `blocks` non-final empty stored blocks and a
/// final empty one, decoding to no bytes while occupying five wire bytes per
/// block. The wire-only expansion that a decoded-byte cap misses. Shared by
/// the preview and REST body-cap tests.
pub(crate) fn gzip_empty_stored_blocks(blocks: usize) -> Vec<u8> {
    // gzip header: magic, deflate, no flags, no mtime, OS = unknown.
    let mut bytes = vec![0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, 0, 0xff];
    // BFINAL = 0, BTYPE = 00, padded to a byte, then LEN = 0, NLEN = 0xffff.
    for _ in 0..blocks {
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0xff, 0xff]);
    }
    // The final block sets BFINAL = 1; empty output has CRC 0 and size 0.
    bytes.extend_from_slice(&[0x01, 0x00, 0x00, 0xff, 0xff]);
    bytes.extend_from_slice(&[0; 8]);
    bytes
}

/// Builds a gzip stream that decodes to `1 + 258 * matches` zero bytes: one
/// literal zero, then `matches` DEFLATE fixed-Huffman length-258/distance-1
/// matches. A body much smaller than its decoded output is the shape that
/// defeats a cap applied to the wire bytes; the suite has no compression
/// crate, and fixed Huffman is the least code that writes such a stream.
/// Shared by the preview and REST body-cap tests.
pub(crate) fn gzip_zeros(matches: usize) -> Vec<u8> {
    struct BitWriter {
        bytes: Vec<u8>,
        current: u8,
        used: u8,
    }
    impl BitWriter {
        fn new() -> Self {
            // gzip header: magic, deflate, no flags, no mtime, OS = unknown.
            Self {
                bytes: vec![0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, 0, 0xff],
                current: 0,
                used: 0,
            }
        }
        fn bit(&mut self, bit: u8) {
            self.current |= (bit & 1) << self.used;
            self.used += 1;
            if self.used == 8 {
                self.bytes.push(self.current);
                self.current = 0;
                self.used = 0;
            }
        }
        fn code(&mut self, code: u32, bits: u8) {
            for shift in (0..bits).rev() {
                self.bit(((code >> shift) & 1) as u8);
            }
        }
        fn finish(mut self) -> Vec<u8> {
            if self.used > 0 {
                self.bytes.push(self.current);
            }
            self.bytes
        }
    }

    let mut writer = BitWriter::new();
    writer.bit(1); // BFINAL
    writer.bit(1); // BTYPE = 01 (fixed), least-significant bit first
    writer.bit(0);
    writer.code(0x30, 8); // literal 0x00
    for _ in 0..matches {
        writer.code(0xC5, 8); // length 258, no extra bits
        writer.code(0, 5); // distance 1
    }
    writer.code(0, 7); // end of block
    let decoded = 1 + 258 * matches;
    let mut bytes = writer.finish();
    bytes.extend_from_slice(&crc32_of_zeros(decoded).to_le_bytes());
    bytes.extend_from_slice(&(decoded as u32).to_le_bytes());
    bytes
}

/// CRC-32 (IEEE, reflected) of `count` zero bytes, the gzip trailer's check
/// value for [`gzip_zeros`]. Built from a runtime table so the test is fast.
fn crc32_of_zeros(count: usize) -> u32 {
    let mut table = [0u32; 256];
    for (index, slot) in table.iter_mut().enumerate() {
        let mut value = index as u32;
        for _ in 0..8 {
            value = if value & 1 == 0 {
                value >> 1
            } else {
                0xedb8_8320 ^ (value >> 1)
            };
        }
        *slot = value;
    }
    let mut crc = 0xffff_ffffu32;
    for _ in 0..count {
        crc = table[(crc & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffff_ffff
}

#[cfg(test)]
mod loopback_thread_names {
    use std::io::Write;
    use std::net::TcpStream;
    use std::time::Duration;

    use super::{loopback_listener, record_first_request, serve_one_response_capturing_request};

    // Both one-shot loopback helpers run on named threads so a panic there is
    // attributed to `nostalgia-test-*` rather than `<unnamed>`. Pin each name
    // on the helper's own thread; the name is spelled out here rather than
    // read back from the helper, so a rename in either place fails the test.
    #[test]
    fn record_first_request_names_its_thread() {
        let (listener, address) = loopback_listener();
        let handle = record_first_request(listener);
        let mut client = TcpStream::connect(address).unwrap();
        client.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();

        let recorded = handle.join().unwrap();
        assert!(recorded.request.is_some());
        assert_eq!(
            recorded.thread_name.as_deref(),
            Some("nostalgia-test-listener")
        );
    }

    #[test]
    fn serve_one_response_capturing_request_names_its_thread() {
        let (address, receiver) = serve_one_response_capturing_request(
            "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        let mut client = TcpStream::connect(address).unwrap();
        client.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();

        let captured = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(captured.request.starts_with("GET / HTTP/1.1"));
        assert_eq!(
            captured.thread_name.as_deref(),
            Some("nostalgia-test-server")
        );
    }
}
