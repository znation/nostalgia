//! The `MusicKit` loopback authorization flow.
//!
//! Apple's `MusicKit` JS can mint a *user token* for the Apple Music API, but it
//! only runs in a browser. This module runs that flow without a webview: it
//! starts a one-shot HTTP server on a loopback port, opens the system browser
//! at a page that loads `MusicKit` JS and calls `authorize()`, and captures the
//! token Apple's page posts back. The developer token stays owner-side and is
//! read from the environment by [`crate::apple_music::init_service`].
//!
//! The flow is synchronous and blocking by design — the caller runs it off the
//! UI thread — and uses only the standard library, matching the project's
//! preference for no new dependency.
//!
//! # Server safety
//!
//! The server is bound to `127.0.0.1` on an ephemeral port and accepts a single
//! `POST /token` callback. A request whose `Host` header does not name
//! `127.0.0.1` is refused, so a DNS-rebinding page — which reaches the socket
//! under its own hostname — cannot read the sign-in page's developer token or
//! `state` nonce. The page route itself is served only to a request that
//! presents this flow's `state` nonce, and the browser reaches that URL through
//! an owner-only bootstrap file rather than a command-line argument — process
//! arguments are world-readable on Unix, so a URL carrying the nonce would leak
//! it to every local user. A local client that can reach the loopback port but
//! does not know the nonce therefore cannot read the developer token the page
//! embeds. The nonce stays owner-side in the browser too: the page is served
//! at a URL that carries it and loads Apple's `MusicKit` script from a
//! third-party origin, so every response sets `Referrer-Policy: no-referrer`
//! and the browser cannot be talked into sending that URL — and with it the
//! nonce — to the third party. Every request is read into a buffer capped at
//! `MAX_REQUEST_BYTES` (defined in the `http` submodule): the cap is checked
//! while reading headers *and* before the declared body is read, so a hostile
//! client cannot make the server allocate without bound or panic it with an
//! overflowing `Content-Length`. Per-connection failures — a client that closes
//! early, stalls, or declares an oversized body — are ignored and the server
//! keeps waiting for the real callback; only a well-formed callback carrying an
//! invalid user token ends the flow with an error, while a callback with the
//! wrong `state` is answered and ignored so a local client cannot abort the
//! sign-in by forging one. The connections the server has accepted are read
//! together, not one after another: a local client that opens a connection and
//! dribbles a partial request is bounded by `CONNECTION_READ_TIMEOUT` from
//! the moment it is accepted — it is dropped when that budget is spent — but
//! it never holds the accept loop while it waits, so the browser's callback is
//! served as soon as the accept loop has worked through the connections queued
//! ahead of it. Each response write carries its own bounded timeout, so a
//! client that stops reading cannot hold the connection there either.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io;
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use crate::music_error::AppleMusicError;

mod bootstrap;
mod http;
mod opener;
mod page;
mod server;

pub use opener::open_in_browser;

use bootstrap::BootstrapPage;
use http::CONNECTION_READ_TIMEOUT;
use server::{Connection, PendingConnection, accept_pending, serve_pending};

/// How long [`authorize`] waits for the browser callback before giving up:
/// five minutes, enough for a sign-in the user completes by hand.
const AUTH_TIMEOUT: Duration = Duration::from_secs(300);

/// How long the non-blocking accept loop sleeps between polls.
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// An authenticated `MusicKit` session: the owner-side developer token and the
/// user token the sign-in flow obtained.
#[derive(Clone, PartialEq, Eq)]
pub struct MusicKitSession {
    /// The owner-side developer token (an ES256 JWT) supplied by the caller.
    pub developer_token: String,
    /// The user token `MusicKit` JS returned for this sign-in.
    pub user_token: String,
}

impl std::fmt::Debug for MusicKitSession {
    /// Redacts both tokens so a session never leaks into a log or a failed
    /// assertion's message.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MusicKitSession")
            .field("developer_token", &"<redacted>")
            .field("user_token", &"<redacted>")
            .finish()
    }
}

/// Checks that `token` is a three-segment, non-empty, base64url-shaped JWT.
///
/// This rejects a blank or malformed token and, because the accepted alphabet
/// is `[A-Za-z0-9_-]`, guarantees the value is safe to interpolate into the
/// page's JavaScript string literal.
///
/// # Errors
///
/// Returns an [`AppleMusicError`] when `token` is not exactly three non-empty
/// dot-separated segments of base64url characters. The message names the
/// defect — the wrong number of segments, an empty segment, or a character
/// outside the base64url alphabet — without echoing the token, which is a
/// secret, so the caller learns what to fix without the value reaching a log.
// `authorize` below is this function's caller; it is public so the wiring in
// `apple_music` can validate the developer token before storing it.
pub fn validate_developer_token(token: &str) -> Result<(), AppleMusicError> {
    match jwt_shape_problem(token) {
        None => Ok(()),
        Some(problem) => Err(AppleMusicError::new(format!(
            "the developer token must be a three-segment base64url JWT, but {problem}"
        ))),
    }
}

/// Runs the `MusicKit` loopback sign-in flow and returns the resulting session.
///
/// `open_page` is called with the path of an owner-only bootstrap page that
/// redirects the browser to the loopback URL; production passes
/// [`open_in_browser`], and tests pass a fake opener that reads the page and
/// drives the callback.
///
/// Surrounding whitespace on `developer_token` — a trailing newline read from
/// a token file, say — is trimmed before validation and before the token is
/// used or stored: whitespace can never be part of a base64url JWT, and an
/// environment variable commonly carries it, so rejecting it would fail a
/// valid token for a reason the caller cannot see.
///
/// # Errors
///
/// Returns an [`AppleMusicError`] when `developer_token` is malformed, when the
/// browser cannot be opened, when a callback carries an invalid user token, or
/// when no callback arrives before [`AUTH_TIMEOUT`]. A callback with the wrong
/// `state` is answered with a `400` and ignored rather than aborting the flow,
/// so a local client that can reach the loopback port but does not know the
/// nonce cannot end a sign-in the user is completing.
// `AppleMusicService::authenticate` is this function's caller; it is public so
// the wiring in `apple_music` can obtain a session at startup.
pub fn authorize(
    developer_token: &str,
    open_page: &dyn Fn(&str) -> io::Result<()>,
) -> Result<MusicKitSession, AppleMusicError> {
    authorize_with_timeout(developer_token, open_page, AUTH_TIMEOUT)
}

/// [`authorize`] with an injectable deadline, so tests need not wait
/// [`AUTH_TIMEOUT`].
fn authorize_with_timeout(
    developer_token: &str,
    open_page: &dyn Fn(&str) -> io::Result<()>,
    timeout: Duration,
) -> Result<MusicKitSession, AppleMusicError> {
    authorize_with_bounds(developer_token, open_page, timeout, CONNECTION_READ_TIMEOUT)
}

/// [`authorize_with_timeout`] with an injectable per-connection budget, so a
/// test can prove one stalled connection cannot hold the accept loop for the
/// whole flow without waiting out the production five-second budget.
fn authorize_with_bounds(
    developer_token: &str,
    open_page: &dyn Fn(&str) -> io::Result<()>,
    timeout: Duration,
    connection_budget: Duration,
) -> Result<MusicKitSession, AppleMusicError> {
    // Normalize at the boundary before validating or using the token: the
    // production caller reads it from `APPLE_MUSIC_DEVELOPER_TOKEN`, where a
    // trailing newline or a stray space is easy to introduce and impossible
    // to see in the failure message. Whitespace is never a base64url JWT
    // character, so trimming cannot turn a malformed token into a valid one.
    let developer_token = developer_token.trim();
    validate_developer_token(developer_token)?;

    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| {
        AppleMusicError::new(format!("could not start the sign-in server: {error}"))
    })?;
    let port = listener
        .local_addr()
        .map_err(|error| {
            AppleMusicError::new(format!("could not read the sign-in server port: {error}"))
        })?
        .port();
    let nonce = random_nonce();
    let url = format!("http://127.0.0.1:{port}/?state={nonce}");
    // Keep the URL — and with it the `state` nonce — out of the opener's
    // command line, which is world-readable on Unix. The browser opens this
    // owner-only file, which redirects it to the loopback page. `bootstrap`
    // stays alive until the flow returns so the browser can read it.
    let bootstrap = BootstrapPage::write(&url).map_err(|error| {
        AppleMusicError::new(format!(
            "could not write the sign-in bootstrap page: {error}"
        ))
    })?;
    open_page(&bootstrap.path_string()).map_err(|error| {
        AppleMusicError::new(format!("could not open the sign-in page: {error}"))
    })?;
    listener.set_nonblocking(true).map_err(|error| {
        AppleMusicError::new(format!("could not poll the sign-in server: {error}"))
    })?;

    let deadline = Instant::now() + timeout;
    let mut pending: Vec<PendingConnection> = Vec::new();
    loop {
        if Instant::now() >= deadline {
            // Name the bound that expired, as the fetch and play timeout
            // reports do (`ui::loading`), so a user who waited can tell how
            // long the flow waited before giving up.
            return Err(AppleMusicError::new(format!(
                "timed out waiting for the Apple Music sign-in callback after {timeout:?}"
            )));
        }
        // Serve the connections whose request has arrived. Every pending
        // connection is read in its own turn and none of them is waited for,
        // so a client that opens a connection and never finishes its request
        // cannot hold the browser's callback behind it.
        match serve_pending(&mut pending, developer_token, &nonce) {
            Connection::Continue => {}
            Connection::Authorized(session) => return Ok(session),
            Connection::Rejected(error) => return Err(error),
        }
        // Take on the connections that are waiting, then wait for the next
        // pass.
        accept_pending(&listener, &mut pending, deadline, connection_budget)
            .map_err(|error| AppleMusicError::new(format!("the sign-in server failed: {error}")))?;
        thread::sleep(ACCEPT_POLL_INTERVAL);
    }
}

/// Builds the `state` nonce: a hex-formatted hash drawn from a fresh
/// [`RandomState`], which seeds itself from the process's random keys.
fn random_nonce() -> String {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u8(0);
    format!("{:016x}", hasher.finish())
}

/// Why `token` is not a three-segment base64url JWT, or `None` when it is.
///
/// The reason is a fixed phrase — never the token itself, which is a secret
/// and must not reach a log — so [`validate_developer_token`] and
/// `handle_token` can name the defect without echoing the value. The checks
/// run from the most structural to the most specific: the segment count, then
/// emptiness, then the alphabet.
fn jwt_shape_problem(token: &str) -> Option<&'static str> {
    let mut parts = token.split('.');
    let (Some(first), Some(second), Some(third), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Some("it does not have exactly three dot-separated segments");
    };

    let segments = [first, second, third];
    if segments.iter().any(|segment| segment.is_empty()) {
        return Some("one of its segments is empty");
    }
    if segments.iter().any(|segment| {
        !segment
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    }) {
        return Some("it contains a character outside the base64url alphabet");
    }
    None
}

#[cfg(test)]
mod tests;
