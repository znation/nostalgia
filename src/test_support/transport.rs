//! The recording `HttpTransport` stub shared by the Apple Music and `ui` tests.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::apple_music::AppleMusicError;
use crate::apple_music::rest::HttpTransport;
use crate::music_kit_auth::MusicKitSession;

use super::rest_session;

/// A REST response body with an empty `data` list, the shape of a library or
/// catalog request that matched nothing.
pub(crate) const EMPTY_DATA_BODY: &str = r#"{"data":[]}"#;

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

/// Asserts the stub saw exactly the requests to `expected_urls`, in order.
/// The two-request browse loads (artists then albums, albums then songs)
/// pin both endpoints this way; the session is not checked.
pub(crate) fn assert_call_urls(stub: &StubTransport, expected_urls: &[&str]) {
    let urls: Vec<String> = stub.calls().into_iter().map(|call| call.0).collect();
    assert_eq!(urls, expected_urls);
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
