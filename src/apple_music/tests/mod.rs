use super::address::{ip_is_internal, is_numeric_host_form};
use super::playback::play::play_log_line;
use super::threads::{
    off_thread, off_thread_with, spawn_sign_in_thread, start_sign_in_with, startup_token,
};
use super::validation::{IdKind, ensure_id_is_valid, preview_url_problem};
use super::*;
use crate::audio::{AudioOutput, EqSnapshot};
use crate::library::{Album, Artist, Playlist, Song};
use crate::sample_library::sample_library;
use crate::state::TransportMode;
use crate::test_support::{
    AudioCall, EMPTY_DATA_BODY, FailingAudio, FailingCommand, PREVIEW_URL, RecordingAudio,
    StubTransport, assert_call_urls, assert_ids, assert_preview_calls, assert_single_call,
    record_call, rest_session, sample_album, sample_artist, sample_song, sign_in_session,
};
use std::time::Duration;

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
/// `AppState`. The tests that need both share this setup.
fn test_service_with_state() -> (AppleMusicService, Arc<Mutex<AppState>>) {
    let service = test_service();
    let state = service.state.clone();
    (service, state)
}

/// A fresh service plus its shared state with `song-1`'s preview already
/// playing. The tests whose subject is a *later* operation — replacing the
/// track, pausing, rejecting a blank id, a control-character id, or a blank
/// preview URL mid-playback, and the transport stubs — all need that same
/// starting point, so the ordinary play (guard reporting "not superseded")
/// lives here once and each test names only the state it drives to.
async fn service_with_song_1_playing() -> (AppleMusicService, Arc<Mutex<AppState>>) {
    let (service, state) = test_service_with_state();
    service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap();
    (service, state)
}

/// Locks the shared playback state and asserts it holds `expected_track`
/// with `expected_playing`. The lock-and-compare sequence lives here once, so
/// each calling test only names the state it drove to.
async fn assert_playback_state(
    state: &Arc<Mutex<AppState>>,
    expected_track: Option<&str>,
    expected_playing: bool,
) {
    let state = state.lock().await;
    assert_eq!(state.current_track.as_deref(), expected_track);
    assert_eq!(state.is_playing, expected_playing);
}

/// Locks the shared state and asserts its resolved [`TransportMode`], so a
/// transport test can tell Pause from Stop even though both clear the playing
/// flag.
async fn assert_transport_mode(state: &Arc<Mutex<AppState>>, expected: TransportMode) {
    assert_eq!(state.lock().await.transport_mode(), expected);
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
/// the exact `AudioCall` sequence it sent. The tests that drive a stopped
/// transport through the recording backend build this same
/// `Arc::new`/`Arc::clone` pair, so it lives here once.
fn recording_service() -> RecordingService {
    let recording = Arc::new(RecordingAudio::default());
    let (service, state) = service_with_audio(Arc::clone(&recording) as Arc<dyn AudioOutput>);
    (service, state, recording)
}

/// [`recording_service`] with `song-1`'s preview already playing, so the
/// backend holds a loaded player for the tests whose subject is a later
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
        record_call(&self.calls, AudioCall::Play(url.to_string()))
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::Pause)?;
        let _ = self.entered.send(());
        let _ = self.release.lock().unwrap().recv();
        Ok(())
    }

    fn resume(&self) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::Resume)
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::Stop)
    }

    fn set_volume(&self, volume: f32) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::SetVolume(volume))
    }

    fn set_balance(&self, balance: f32) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::SetBalance(balance))
    }

    fn seek_by(&self, offset_secs: i64) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::SeekBy(offset_secs))
    }

    fn seek_to(&self, position: Duration) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::SeekTo(position))
    }

    fn set_equalizer(&self, equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::SetEqualizer(equalizer))
    }
}

/// The one backend command a [`RejectingAudio`] backend refuses.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RejectedCommand {
    Pause,
    Resume,
    Stop,
    SetVolume,
    SetBalance,
    SetEqualizer,
    /// Every `play` after the first, so a test can load the first preview and
    /// then refuse the restart a Play-after-Stop issues.
    PlayAfterFirst,
    /// Every `set_equalizer` after the first, so a test can load the first
    /// preview and then refuse the curve Play-after-Stop forwards before it
    /// replays.
    SetEqualizerAfterFirst,
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
    /// Counts `play` calls so [`RejectedCommand::PlayAfterFirst`] lets the
    /// initial preview load before refusing its restart.
    plays: std::sync::Mutex<usize>,
    /// Counts `set_equalizer` calls so
    /// [`RejectedCommand::SetEqualizerAfterFirst`] lets the initial preview
    /// load before refusing the curve a restart forwards.
    equalizers: std::sync::Mutex<usize>,
}

impl RejectingAudio {
    fn new(rejected: RejectedCommand) -> Self {
        Self {
            rejected,
            calls: std::sync::Mutex::new(Vec::new()),
            plays: std::sync::Mutex::new(0),
            equalizers: std::sync::Mutex::new(0),
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
        record_call(&self.calls, AudioCall::Play(url.to_string()))?;
        if self.rejected == RejectedCommand::PlayAfterFirst {
            let mut plays = self.plays.lock().unwrap();
            *plays += 1;
            if *plays > 1 {
                return Err(AppleMusicError::new("the audio device is gone"));
            }
        }
        Ok(())
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::Pause)?;
        self.check(RejectedCommand::Pause)
    }

    fn resume(&self) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::Resume)?;
        self.check(RejectedCommand::Resume)
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::Stop)?;
        self.check(RejectedCommand::Stop)
    }

    fn set_volume(&self, volume: f32) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::SetVolume(volume))?;
        self.check(RejectedCommand::SetVolume)
    }

    fn set_balance(&self, balance: f32) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::SetBalance(balance))?;
        self.check(RejectedCommand::SetBalance)
    }

    fn seek_by(&self, offset_secs: i64) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::SeekBy(offset_secs))
    }

    fn seek_to(&self, position: Duration) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::SeekTo(position))
    }

    fn set_equalizer(&self, equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
        record_call(&self.calls, AudioCall::SetEqualizer(equalizer))?;
        if self.rejected == RejectedCommand::SetEqualizerAfterFirst {
            let mut equalizers = self.equalizers.lock().unwrap();
            *equalizers += 1;
            if *equalizers > 1 {
                return Err(AppleMusicError::new("the audio device is gone"));
            }
        }
        self.check(RejectedCommand::SetEqualizer)
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

// The REST browse wiring: with a session stored, each browse query runs
// through the injected `rest::HttpTransport` and returns its mapped rows;
// without a session, the same service still answers from the sample library
// and never calls the transport. The stub records each request, so the URL
// and both tokens are pinned alongside the mapped rows.

// The non-sample song the signed-in browse tests carry: its id, title, and
// artist feed both the expected `Song` and the stub envelopes, so a change to
// the mapped values shows up in one place. The search, All Songs, and
// playlist-tracks tests share it; the two outer envelope shapes are built by
// [`rest_song_data`] and [`rest_search_song_data`].
const REST_SONG_ID: &str = "rest-song";
const REST_SONG_TITLE: &str = "Remote Result";
const REST_SONG_ARTIST: &str = "Remote Band";

/// The song the signed-in browse tests' song envelopes carry: a non-sample
/// id with only the attributes those envelopes include, so a result can only
/// have come from REST.
fn rest_song() -> Song {
    Song {
        id: REST_SONG_ID.to_string(),
        title: REST_SONG_TITLE.to_string(),
        artist: REST_SONG_ARTIST.to_string(),
        album: String::new(),
        year: None,
        genre: String::new(),
        track_number: None,
        disc_number: None,
        composer: String::new(),
        album_id: String::new(),
        duration_ms: 0,
        preview_url: None,
    }
}

/// The JSON song object the signed-in browse envelopes carry, built from the
/// same constants as [`rest_song`].
fn rest_song_json() -> String {
    format!(
        r#"{{"id":"{}","attributes":{{"name":"{}","artistName":"{}"}}}}"#,
        REST_SONG_ID, REST_SONG_TITLE, REST_SONG_ARTIST
    )
}

/// A stub body for the collection endpoints, whose envelope is `{"data": [...]}`.
fn rest_song_data() -> String {
    format!(r#"{{"data":[{}]}}"#, rest_song_json())
}

/// A stub body for the search endpoint, which nests the data under
/// `results.library-songs`.
fn rest_search_song_data() -> String {
    format!(
        r#"{{"results":{{"library-songs":{{"data":[{}]}}}}}}"#,
        rest_song_json()
    )
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

mod validation;

mod browse_sample;

mod browse_rest;

mod play_track;

mod audio_forwarding;

mod transport;

mod threads_and_session;
