//! The HTTP seam between the REST client and the blocking `ureq` client.
//!
//! [`HttpTransport`] is the narrow seam the REST client's tests replace with an
//! in-memory stub; [`UreqTransport`] is the production implementation.

use std::io::Read;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use super::super::AppleMusicError;
use super::paging::REQUEST_TIMEOUT;
use super::wire::api_error_cause;
use crate::http::agent_with_timeout;
use crate::music_kit_auth::MusicKitSession;

/// The largest response body, in both wire and decoded bytes, the REST
/// transport accepts.
///
/// A REST reply is server-controlled data, and the shared agent deliberately
/// honors the environment proxy, so a hostile or compromised endpoint or proxy
/// can answer a query. `ureq`'s reader limit is set one byte above this so an
/// exactly-at-cap body reaches EOF rather than tripping the limit on the probe
/// past it; that limit counts wire bytes and sits under the gzip decoder, so
/// the body is also read through a decoded-byte `take`, exactly as the preview
/// downloader does. Eight mebibytes is far above one 100-item Apple Music page
/// and keeps `MAX_PAGES` pages from accumulating into an unbounded buffer.
pub(super) const MAX_RESPONSE_BYTES: u64 = 8 * 1024 * 1024;

/// The narrow seam over the blocking HTTP client, so [`RestLibrary`](super::RestLibrary)'s tests
/// run without a network.
///
/// [`UreqTransport`] is the production implementation; a test installs a stub
/// that returns a canned body and records the request it saw.
pub trait HttpTransport: Send + Sync {
    /// Fetches `url` with `session`'s developer and user tokens and returns the
    /// response body as text.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails — a transport
    /// error, a non-success status, or a body that cannot be read. The message
    /// is the bare cause (the HTTP status and Apple's detail, or the underlying
    /// error); [`RestLibrary`](super::RestLibrary) passes it through unchanged and leaves the query
    /// context to the UI, so the user-facing report names the query once.
    fn get(&self, url: &str, session: &MusicKitSession) -> Result<String, AppleMusicError>;

    /// Fetches `url` like [`get`](Self::get), but must not run past `deadline`.
    ///
    /// `RestLibrary::fetch` calls this with one deadline for the whole
    /// paginated fetch, so a slow multi-page collection shares a single budget
    /// instead of letting each page start a fresh one. The default ignores
    /// `deadline` and delegates to [`get`](Self::get), which is safe for a test
    /// stub that answers instantly; the production [`UreqTransport`] overrides
    /// it to bound the request.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails or when `deadline`
    /// has already passed.
    fn get_within(
        &self,
        url: &str,
        session: &MusicKitSession,
        deadline: Instant,
    ) -> Result<String, AppleMusicError> {
        let _ = deadline;
        self.get(url, session)
    }
}

/// The production [`HttpTransport`], backed by the blocking `ureq` client.
///
/// Every request runs on one process-wide [`ureq::Agent`]: a fresh agent per request (`ureq::get`)
/// would open a new TCP connection and TLS session each time, while the shared agent reuses the
/// pooled connection across the library browse and search requests. The shared agent carries
/// [`REQUEST_TIMEOUT`] as its global timeout, so a stalled server cannot block a request forever,
/// and follows no redirects, so the `Music-User-Token` credential is only ever sent to the URL this
/// client built (see [`crate::http::agent_with_timeout`]).
///
/// The agent also disables `ureq`'s status-as-error shortcut (`http_status_as_error(false)`), so a
/// 4xx/5xx response reaches [`UreqTransport::get`] with its body intact and the Apple Music error
/// detail can be surfaced instead of a bare status code.
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl UreqTransport {
    /// Builds the transport over the process-wide pooled [`ureq::Agent`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            agent: shared_agent().clone(),
        }
    }

    /// [`new`](Self::new) with an explicit global timeout, so a test can use a
    /// short bound against a stalled loopback server instead of waiting out
    /// the production 30 seconds.
    #[cfg(test)]
    pub(super) fn with_timeout(timeout: Duration) -> Self {
        Self {
            agent: agent_with_timeout(timeout),
        }
    }
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// The shared [`ureq::Agent`], built once on first use. `ureq` keeps idle connections per host for
/// 15 seconds by default, so consecutive browse queries reuse the same TLS connection. Its global
/// timeout is [`REQUEST_TIMEOUT`], so every request through it is bounded end to end.
pub(super) fn shared_agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| agent_with_timeout(REQUEST_TIMEOUT))
}

impl UreqTransport {
    /// Runs one request with `timeout` as its bound, or the agent's own global
    /// bound when `timeout` is `None`. Both [`get`](Self::get) and the trait's
    /// [`get_within`](HttpTransport::get_within) share this body so the
    /// response handling lives here once.
    fn request(
        &self,
        url: &str,
        session: &MusicKitSession,
        timeout: Option<Duration>,
    ) -> Result<String, AppleMusicError> {
        let authorization = format!("Bearer {}", session.developer_token);
        let request = self.agent.get(url);
        // A per-request bound wins over the agent's global one, so a later page
        // of a paginated fetch cannot outlive the fetch's shared deadline.
        let request = match timeout {
            Some(timeout) => request.config().timeout_global(Some(timeout)).build(),
            None => request,
        };
        let mut response = request
            .header("Authorization", authorization.as_str())
            .header("Music-User-Token", session.user_token.as_str())
            .call()
            .map_err(|error| AppleMusicError::new(error.to_string()))?;

        let status = response.status();
        let mut bytes = Vec::new();
        response
            .body_mut()
            .with_config()
            .limit(MAX_RESPONSE_BYTES + 1)
            .lossy_utf8(true)
            .reader()
            .take(MAX_RESPONSE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| {
                AppleMusicError::new(format!("reading the response body failed: {error}"))
            })?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_RESPONSE_BYTES {
            return Err(AppleMusicError::new(format!(
                "the response body is larger than the {} MiB cap",
                MAX_RESPONSE_BYTES / (1024 * 1024)
            )));
        }
        let body = String::from_utf8(bytes).map_err(|error| {
            AppleMusicError::new(format!("reading the response body failed: {error}"))
        })?;

        // `http_status_as_error(false)` leaves a non-2xx response as `Ok`, so
        // the status is checked here where the body is still available:
        // Apple's error body names the cause ("Invalid developer token",
        // say), which `ureq`'s bare `Error::StatusCode` would otherwise
        // discard.
        if status.is_success() {
            Ok(body)
        } else {
            // A 401 or 403 is Apple Music rejecting this request's
            // authorization — a stale or revoked MusicKit user token, an
            // expired developer token. It is classified as `Auth` (while the
            // message stays byte-identical to a plain failure's) so the UI's
            // report can hint at the recovery — signing in again — without
            // parsing the status text.
            //
            // A 429 is Apple Music counting too many requests from this
            // token and asking the client to back off. It is classified as
            // `Throttled` so the UI's report can point at the transient
            // recovery — wait and retry — instead of leaving a throttle
            // looking like a hard failure. The `Retry-After` header, when it
            // names a whole number of seconds, is echoed into the message (`
            // (retry after N s)`); an absent or unparseable one is ignored —
            // nothing is invented from a value this client does not
            // understand.
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.trim().parse::<u64>().ok());
            let mut message = match api_error_cause(&body) {
                Some(cause) => format!("HTTP {status}: {cause}"),
                None => format!("HTTP {status}"),
            };
            if let Some(seconds) = retry_after {
                message.push_str(&format!(" (retry after {seconds} s)"));
            }
            Err(match status.as_u16() {
                401 | 403 => AppleMusicError::new_auth(message),
                429 => AppleMusicError::new_throttled(message),
                _ => AppleMusicError::new(message),
            })
        }
    }
}

impl HttpTransport for UreqTransport {
    fn get(&self, url: &str, session: &MusicKitSession) -> Result<String, AppleMusicError> {
        self.request(url, session, None)
    }

    fn get_within(
        &self,
        url: &str,
        session: &MusicKitSession,
        deadline: Instant,
    ) -> Result<String, AppleMusicError> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(AppleMusicError::new(
                "the request deadline had already passed",
            ));
        }
        self.request(url, session, Some(remaining))
    }
}
