//! The `MusicKit` loopback authorization flow.
//!
//! Apple's `MusicKit` JS can mint a *user token* for the Apple Music API, but it
//! only runs in a browser. This module runs that flow without a webview: it
//! starts a one-shot HTTP server on a loopback port, opens the system browser
//! at a page that loads `MusicKit` JS and calls `authorize()`, and captures the
//! token Apple's page posts back. The developer token stays owner-side and is
//! read from the environment by the sibling wiring plan.
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
//! `state` nonce. Every request is read into a buffer capped at
//! `MAX_REQUEST_BYTES` (defined in the `http` submodule): the cap is checked
//! while reading headers *and* before the declared body is read, so a hostile
//! client cannot make the server allocate without bound or panic it with an
//! overflowing `Content-Length`. Per-connection failures — a client that closes
//! early, stalls, or declares an oversized body — are ignored and the server
//! keeps waiting for the real callback; only a well-formed callback with the
//! wrong `state` or an invalid user token ends the flow with an error. Each
//! connection's reads are capped by the time left before [`AUTH_TIMEOUT`], so a
//! client that dribbles bytes cannot hold the flow past its deadline.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io;
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crate::apple_music::AppleMusicError;

mod http;
mod page;

use http::{read_http_request, write_response};
use page::{SUCCESS_PAGE, render_auth_page};

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
/// `open_url` is called with the loopback page URL; production passes
/// [`open_in_browser`], and tests pass a fake opener that drives the callback.
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
/// browser cannot be opened, when a callback carries the wrong `state` or an
/// invalid user token, or when no callback arrives before [`AUTH_TIMEOUT`].
// `AppleMusicService::authenticate` is this function's caller; it is public so
// the wiring in `apple_music` can obtain a session at startup.
pub fn authorize(
    developer_token: &str,
    open_url: &dyn Fn(&str) -> io::Result<()>,
) -> Result<MusicKitSession, AppleMusicError> {
    authorize_with_timeout(developer_token, open_url, AUTH_TIMEOUT)
}

/// [`authorize`] with an injectable deadline, so tests need not wait
/// [`AUTH_TIMEOUT`].
fn authorize_with_timeout(
    developer_token: &str,
    open_url: &dyn Fn(&str) -> io::Result<()>,
    timeout: Duration,
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
    open_url(&url).map_err(|error| {
        AppleMusicError::new(format!("could not open the sign-in page: {error}"))
    })?;
    listener.set_nonblocking(true).map_err(|error| {
        AppleMusicError::new(format!("could not poll the sign-in server: {error}"))
    })?;

    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() >= deadline {
            // Name the bound that expired, as the fetch and play timeout
            // reports do (`ui::loading`), so a user who waited can tell how
            // long the flow waited before giving up.
            return Err(AppleMusicError::new(format!(
                "timed out waiting for the Apple Music sign-in callback after {timeout:?}"
            )));
        }
        match listener.accept() {
            Ok((mut stream, _address)) => {
                match serve_connection(&mut stream, developer_token, &nonce, deadline) {
                    Connection::Authorized(session) => return Ok(session),
                    Connection::Continue => {}
                    Connection::Rejected(error) => return Err(error),
                }
            }
            Err(ref error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(ACCEPT_POLL_INTERVAL);
            }
            Err(error) => {
                return Err(AppleMusicError::new(format!(
                    "the sign-in server failed: {error}"
                )));
            }
        }
    }
}

/// Opens `url` in the system's default browser.
///
/// The opener is a short-lived launcher, so it is reaped on a detached thread
/// (see [`spawn_opener`]) rather than left as a zombie for the rest of the
/// app's life.
///
/// # Errors
///
/// Returns the [`std::io::Error`] from spawning the platform's opener command;
/// the message names the command (e.g. `xdg-open`), so a missing opener is
/// diagnosable rather than a bare "No such file or directory".
// `AppleMusicService::authenticate` passes this to [`authorize`]; it is public
// so the wiring in `apple_music` can supply the real opener.
pub fn open_in_browser(url: &str) -> io::Result<()> {
    if cfg!(target_os = "macos") {
        spawn_opener("open", &[url])
    } else if cfg!(target_os = "windows") {
        spawn_opener("cmd", &["/C", "start", "", url])
    } else {
        spawn_opener("xdg-open", &[url])
    }
}

/// Spawns `program` with `args`, naming it in any spawn error so a missing
/// opener reads as "`xdg-open`: No such file or directory" rather than a bare
/// OS error.
///
/// The opener exits as soon as it has handed the URL to the browser, but a
/// [`std::process::Child`] dropped without a wait leaves a zombie for the rest
/// of the app's life, one per sign-in. Waiting here would block the sign-in
/// flow before it starts accepting callbacks, so [`reap_in_background`] waits
/// on its own thread and the flow continues immediately.
fn spawn_opener(program: &str, args: &[&str]) -> io::Result<()> {
    let child = Command::new(program)
        .args(args)
        .spawn()
        .map_err(|error| io::Error::new(error.kind(), format!("{program}: {error}")))?;
    reap_in_background(child);
    Ok(())
}

/// Waits on `child` on a detached thread so it is reaped without blocking the
/// caller. A failed wait is reported: the child could not be reaped, which is
/// worth a line on stderr rather than silence.
fn reap_in_background(mut child: std::process::Child) {
    thread::spawn(move || {
        if let Err(error) = child.wait() {
            eprintln!("could not reap the browser opener process: {error}");
        }
    });
}

/// What serving one connection decided for the overall flow.
enum Connection {
    /// The callback completed and produced a session.
    Authorized(MusicKitSession),
    /// The request was served or ignored; keep accepting.
    Continue,
    /// A well-formed callback was rejected; stop with this error.
    Rejected(AppleMusicError),
}

/// Serves one accepted connection, turning transport-level failures into
/// [`Connection::Continue`] so a bad connection never aborts the sign-in.
fn serve_connection(
    stream: &mut TcpStream,
    developer_token: &str,
    nonce: &str,
    deadline: Instant,
) -> Connection {
    // The accepted socket is blocking on the platforms this runs on, but be
    // explicit: the reader relies on a blocking read with a timeout.
    if stream.set_nonblocking(false).is_err() {
        return Connection::Continue;
    }

    let request = match read_http_request(stream, deadline) {
        Ok(Some(request)) => request,
        // A connection that closed early, stalled, or declared an oversized
        // request is not the callback; keep waiting for the real one.
        Ok(None) | Err(_) => return Connection::Continue,
    };

    // Browsers set `Host` from the URL and forbid scripts from overriding it,
    // so requiring the loopback address stops a DNS-rebinding page (which
    // reaches this socket under its own hostname) from reading the sign-in
    // page's developer token and `state` nonce. A hostile request is ignored,
    // not fatal, so it cannot abort a sign-in the user is completing.
    if !is_loopback_host(request.host.as_deref()) {
        let _ = write_response(
            stream,
            403,
            "Forbidden",
            "text/plain; charset=utf-8",
            "Forbidden",
        );
        return Connection::Continue;
    }

    if request.method == "GET" && request.path == "/" {
        let page = render_auth_page(developer_token, nonce);
        let _ = write_response(stream, 200, "OK", "text/html; charset=utf-8", &page);
        return Connection::Continue;
    }

    if request.method == "POST" && request.path == "/token" {
        return handle_token(stream, developer_token, nonce, &request.body);
    }

    let _ = write_response(
        stream,
        404,
        "Not Found",
        "text/plain; charset=utf-8",
        "Not found",
    );
    Connection::Continue
}

/// Handles the `POST /token` callback: validates `state` and `userToken` and
/// either returns the session or a rejection.
fn handle_token(
    stream: &mut TcpStream,
    developer_token: &str,
    nonce: &str,
    body: &[u8],
) -> Connection {
    let form = String::from_utf8_lossy(body);
    let mut state = None;
    let mut user_token = None;
    for pair in form.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            match key {
                "state" => state = Some(percent_decode(value)),
                "userToken" => user_token = Some(percent_decode(value)),
                _ => {}
            }
        }
    }

    if state.as_deref() != Some(nonce) {
        let _ = write_response(
            stream,
            400,
            "Bad Request",
            "text/plain; charset=utf-8",
            "The sign-in state did not match.",
        );
        return Connection::Rejected(AppleMusicError::new(
            "the sign-in callback carried an unexpected state",
        ));
    }

    match user_token {
        Some(token) if is_jwt_shaped(&token) => {
            let _ = write_response(stream, 200, "OK", "text/html; charset=utf-8", SUCCESS_PAGE);
            Connection::Authorized(MusicKitSession {
                developer_token: developer_token.to_string(),
                user_token: token,
            })
        }
        _ => {
            let _ = write_response(
                stream,
                400,
                "Bad Request",
                "text/plain; charset=utf-8",
                "The sign-in callback did not carry a valid user token.",
            );
            Connection::Rejected(AppleMusicError::new(
                "the sign-in callback did not carry a valid user token",
            ))
        }
    }
}

/// Builds the `state` nonce: a hex-formatted hash drawn from a fresh
/// [`RandomState`], which seeds itself from the process's random keys.
fn random_nonce() -> String {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u8(0);
    format!("{:016x}", hasher.finish())
}

/// Whether the request's `Host` header names the loopback address the server
/// is bound to, with or without a port.
fn is_loopback_host(host: Option<&str>) -> bool {
    let Some(host) = host else {
        return false;
    };
    let name = host.rsplit_once(':').map_or(host, |(name, _port)| name);
    name == "127.0.0.1"
}

/// Whether `token` is exactly three non-empty dot-separated segments of
/// `[A-Za-z0-9_-]`.
fn is_jwt_shaped(token: &str) -> bool {
    jwt_shape_problem(token).is_none()
}

/// Why `token` is not a three-segment base64url JWT, or `None` when it is.
///
/// The reason is a fixed phrase — never the token itself, which is a secret
/// and must not reach a log — so [`validate_developer_token`] can name the
/// defect without echoing the value. The checks run from the most structural
/// to the most specific: the segment count, then emptiness, then the alphabet.
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

/// Decodes a form-urlencoded value (`+` for space, `%XX` escapes).
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                if let (Some(high), Some(low)) =
                    (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
                {
                    decoded.push(high * 16 + low);
                    index += 3;
                } else {
                    decoded.push(bytes[index]);
                    index += 1;
                }
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// The numeric value of a single hex digit, or `None` for a non-hex byte.
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
