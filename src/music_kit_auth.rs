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
//! `POST /token` callback. Every request is read into a buffer capped at
//! [`MAX_REQUEST_BYTES`]: the cap is checked while reading headers *and* before
//! the declared body is read, so a hostile client cannot make the server
//! allocate without bound or panic it with an overflowing `Content-Length`.
//! Per-connection failures — a client that closes early, stalls, or declares an
//! oversized body — are ignored and the server keeps waiting for the real
//! callback; only a well-formed callback with the wrong `state` or an invalid
//! user token ends the flow with an error.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crate::apple_music::AppleMusicError;

/// How long [`authorize`] waits for the browser callback before giving up.
const AUTH_TIMEOUT: Duration = Duration::from_secs(300);

/// The largest HTTP request, headers and body combined, the loopback server
/// will buffer. A request larger than this is dropped without being read.
const MAX_REQUEST_BYTES: usize = 16 * 1024;

/// How long the non-blocking accept loop sleeps between polls.
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// How long a single connection may take to send its request before it is
/// dropped as a stalled client.
const CONNECTION_READ_TIMEOUT: Duration = Duration::from_secs(5);

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
/// dot-separated segments of base64url characters.
// The wiring plan in PLANS.md is this function's caller; it is public so that
// plan can validate the developer token before storing it.
#[allow(dead_code)]
pub fn validate_developer_token(token: &str) -> Result<(), AppleMusicError> {
    if is_jwt_shaped(token) {
        Ok(())
    } else {
        Err(AppleMusicError::new(
            "the developer token must be a three-segment base64url JWT",
        ))
    }
}

/// Runs the `MusicKit` loopback sign-in flow and returns the resulting session.
///
/// `open_url` is called with the loopback page URL; production passes
/// [`open_in_browser`], and tests pass a fake opener that drives the callback.
///
/// # Errors
///
/// Returns an [`AppleMusicError`] when `developer_token` is malformed, when the
/// browser cannot be opened, when a callback carries the wrong `state` or an
/// invalid user token, or when no callback arrives before [`AUTH_TIMEOUT`].
// The wiring plan in PLANS.md is this function's caller; it is public so that
// plan can obtain a session at startup.
#[allow(dead_code)]
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
            return Err(AppleMusicError::new(
                "timed out waiting for the Apple Music sign-in callback",
            ));
        }
        match listener.accept() {
            Ok((mut stream, _address)) => {
                match serve_connection(&mut stream, developer_token, &nonce) {
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
/// # Errors
///
/// Returns the [`std::io::Error`] from spawning the platform's opener command.
// The wiring plan in PLANS.md is this function's caller; it is public so that
// plan can pass it to [`authorize`].
#[allow(dead_code)]
pub fn open_in_browser(url: &str) -> io::Result<()> {
    if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn().map(|_| ())
    } else if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map(|_| ())
    } else {
        Command::new("xdg-open").arg(url).spawn().map(|_| ())
    }
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
fn serve_connection(stream: &mut TcpStream, developer_token: &str, nonce: &str) -> Connection {
    // The accepted socket is blocking on the platforms this runs on, but be
    // explicit: the reader relies on a blocking read with a timeout.
    if stream.set_nonblocking(false).is_err() {
        return Connection::Continue;
    }
    let _ = stream.set_read_timeout(Some(CONNECTION_READ_TIMEOUT));

    let request = match read_http_request(stream) {
        Ok(Some(request)) => request,
        // A connection that closed early, stalled, or declared an oversized
        // request is not the callback; keep waiting for the real one.
        Ok(None) | Err(_) => return Connection::Continue,
    };

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

/// A parsed HTTP request: the method, the path (query string stripped), and the
/// body bytes.
struct HttpRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

/// Reads one HTTP request into memory, bounded by [`MAX_REQUEST_BYTES`].
///
/// Returns `Ok(None)` when the request is unusable — the peer closed before a
/// full request, the connection timed out, or the declared body is larger than
/// the cap (including a `Content-Length` that would overflow `usize`) — so the
/// caller can ignore it and keep waiting.
fn read_http_request(stream: &mut TcpStream) -> io::Result<Option<HttpRequest>> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 1024];

    // Read until the end of the headers, never buffering past the cap.
    let header_end = loop {
        if let Some(end) = find_header_end(&buffer) {
            break end;
        }
        if buffer.len() >= MAX_REQUEST_BYTES {
            return Ok(None);
        }
        let read = match stream.read(&mut chunk) {
            Ok(0) => return Ok(None),
            Ok(read) => read,
            Err(error) if is_timeout(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        let room = MAX_REQUEST_BYTES - buffer.len();
        buffer.extend_from_slice(&chunk[..read.min(room)]);
    };

    let header = std::str::from_utf8(&buffer[..header_end])
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "request headers are not UTF-8"))?;
    let mut lines = header.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing request line"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing method"))?
        .to_string();
    let target = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing target"))?;
    let path = target.split('?').next().unwrap_or(target).to_string();

    let mut content_length = 0usize;
    for line in lines {
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            // A malformed length is treated as no body; the callback then
            // fails its own validation rather than being read unbounded.
            content_length = value.trim().parse().unwrap_or(0);
        }
    }

    // Refuse an oversized declared body *before* reading it: `checked_add`
    // rejects a `Content-Length` that would overflow `usize`, and the cap check
    // rejects one larger than the buffer. Neither path allocates.
    let body_start = header_end;
    let body_end = match body_start.checked_add(content_length) {
        Some(end) if end <= MAX_REQUEST_BYTES => end,
        _ => return Ok(None),
    };

    while buffer.len() < body_end {
        let room = body_end - buffer.len();
        let limit = room.min(chunk.len());
        let read = match stream.read(&mut chunk[..limit]) {
            Ok(0) => return Ok(None),
            Ok(read) => read,
            Err(error) if is_timeout(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        buffer.extend_from_slice(&chunk[..read]);
    }

    Ok(Some(HttpRequest {
        method,
        path,
        body: buffer[body_start..body_end].to_vec(),
    }))
}

/// Writes a minimal HTTP/1.1 response and closes the connection.
fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &str,
) -> io::Result<()> {
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

/// Returns the index just past the first `\r\n\r\n` in `buffer`, if any.
fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
}

/// Whether an I/O error is a read timeout rather than a real failure.
fn is_timeout(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

/// Builds the `state` nonce: a hex-formatted hash drawn from a fresh
/// [`RandomState`], which seeds itself from the process's random keys.
fn random_nonce() -> String {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u8(0);
    format!("{:016x}", hasher.finish())
}

/// Whether `token` is exactly three non-empty dot-separated segments of
/// `[A-Za-z0-9_-]`.
fn is_jwt_shaped(token: &str) -> bool {
    let valid = |segment: &str| {
        !segment.is_empty()
            && segment.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
            })
    };
    let mut segments = token.split('.');
    matches!(
        (
            segments.next(),
            segments.next(),
            segments.next(),
            segments.next(),
        ),
        (Some(first), Some(second), Some(third), None)
            if valid(first) && valid(second) && valid(third)
    )
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

/// Builds the sign-in page, substituting the developer token, nonce, and build
/// version.
fn render_auth_page(developer_token: &str, nonce: &str) -> String {
    AUTH_PAGE
        .replace("{{DEVELOPER_TOKEN}}", developer_token)
        .replace("{{STATE}}", nonce)
        .replace("{{VERSION}}", env!("CARGO_PKG_VERSION"))
}

/// The page opened in the browser: it loads `MusicKit` JS, configures it, calls
/// `authorize()`, and posts the resulting user token back to `/token`.
const AUTH_PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Nostalgia - Apple Music sign-in</title>
</head>
<body>
<h1>Nostalgia - Apple Music sign-in</h1>
<p id="status">Loading MusicKit...</p>
<script src="https://js-cdn.music.apple.com/musickit/v3/musickit.js"></script>
<script>
var developerToken = "{{DEVELOPER_TOKEN}}";
var state = "{{STATE}}";
document.addEventListener("musickitloaded", function () {
  MusicKit.configure({
    developerToken: developerToken,
    app: { name: "Nostalgia", build: "{{VERSION}}" }
  });
  MusicKit.getInstance().authorize().then(function (userToken) {
    var body = "state=" + encodeURIComponent(state) +
      "&userToken=" + encodeURIComponent(userToken);
    return fetch("/token", {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: body
    });
  }).then(function () {
    document.getElementById("status").textContent =
      "Sign-in complete. You can close this tab.";
  }).catch(function (error) {
    document.getElementById("status").textContent = "Sign-in failed: " + error;
  });
});
</script>
</body>
</html>
"#;

/// The page the browser lands on after a successful callback.
const SUCCESS_PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Nostalgia - signed in</title>
</head>
<body>
<h1>Signed in to Apple Music</h1>
<p>You can close this tab and return to Nostalgia.</p>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    const SAMPLE_DEVELOPER_TOKEN: &str = "eyJhbGciOiJFUzI1NiJ9.eyJpc3MiOiJ0ZWFtIn0.c2ln";
    const SAMPLE_USER_TOKEN: &str = "eyJhbGciOiJFUzI1NiJ9.eyJzdWIiOiJ1c2VyIn0.c2ln";

    /// The background opener threads a test started, so it can join them.
    type OpenerHandles = Arc<Mutex<Vec<thread::JoinHandle<()>>>>;

    /// The port embedded in the opener URL.
    fn port_of(url: &str) -> u16 {
        let authority = url
            .strip_prefix("http://")
            .and_then(|rest| rest.split('/').next())
            .expect("opener URL has an authority");
        authority
            .rsplit(':')
            .next()
            .expect("authority has a port")
            .parse()
            .expect("port is a number")
    }

    /// The `state` query value embedded in the opener URL.
    fn state_of(url: &str) -> String {
        url.split_once("?state=")
            .expect("opener URL carries a state")
            .1
            .to_string()
    }

    /// Sends one raw HTTP request to `port` and returns the whole response.
    fn request(port: u16, raw: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to the server");
        stream.write_all(raw.as_bytes()).expect("write the request");
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("read the response");
        response
    }

    /// Wraps `flow` as an opener that runs it on a background thread, so the
    /// blocking `authorize` call can accept the connection. The returned
    /// handles let a test join the thread (and surface any panic in it) once
    /// `authorize` has returned.
    fn background<F>(flow: F) -> (impl Fn(&str) -> io::Result<()>, OpenerHandles)
    where
        F: FnOnce(u16, String) + Send + 'static,
    {
        let slot = Arc::new(Mutex::new(Some(flow)));
        let handles = Arc::new(Mutex::new(Vec::new()));
        let handles_for_opener = Arc::clone(&handles);
        let opener = move |url: &str| {
            let port = port_of(url);
            let state = state_of(url);
            let flow = slot.lock().expect("opener slot lock").take();
            if let Some(flow) = flow {
                let handle = thread::spawn(move || flow(port, state));
                handles_for_opener
                    .lock()
                    .expect("handles lock")
                    .push(handle);
            }
            Ok(())
        };
        (opener, handles)
    }

    /// Joins every opener thread, propagating a panic from one to the test.
    fn join_all(handles: &OpenerHandles) {
        let drained: Vec<_> = std::mem::take(&mut *handles.lock().expect("handles lock"));
        for handle in drained {
            handle.join().expect("the opener thread finished");
        }
    }

    /// A `POST /token` request carrying `state` and `userToken`.
    fn token_request(state: &str, user_token: &str) -> String {
        let body = format!("state={state}&userToken={user_token}");
        format!(
            "POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
    }

    #[test]
    fn validate_developer_token_accepts_a_three_segment_jwt() {
        assert!(validate_developer_token(SAMPLE_DEVELOPER_TOKEN).is_ok());
    }

    #[test]
    fn validate_developer_token_rejects_blank_and_malformed_values() {
        for token in ["", " ", "a", "a.b", "a.b.c.d", "a..c", "a.b.c\"", "a.b.\\c"] {
            assert!(
                validate_developer_token(token).is_err(),
                "expected {token:?} to be rejected"
            );
        }
    }

    #[test]
    fn debug_redacts_both_tokens() {
        let session = MusicKitSession {
            developer_token: SAMPLE_DEVELOPER_TOKEN.to_string(),
            user_token: SAMPLE_USER_TOKEN.to_string(),
        };
        let debug = format!("{session:?}");
        assert!(!debug.contains(SAMPLE_DEVELOPER_TOKEN));
        assert!(!debug.contains(SAMPLE_USER_TOKEN));
        assert!(debug.contains("<redacted>"));
    }

    #[test]
    fn authorize_returns_a_session_on_a_matching_callback() {
        let observed = Arc::new(Mutex::new(Vec::<String>::new()));
        let observed_for_flow = Arc::clone(&observed);
        let (opener, handles) = background(move |port, state| {
            let page = request(port, "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
            let response = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
            let mut observed = observed_for_flow.lock().expect("observed lock");
            observed.push(state);
            observed.push(page);
            observed.push(response);
        });

        let session =
            authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
                .expect("authorize succeeds");
        join_all(&handles);

        assert_eq!(session.developer_token, SAMPLE_DEVELOPER_TOKEN);
        assert_eq!(session.user_token, SAMPLE_USER_TOKEN);

        let observed = observed.lock().expect("observed lock");
        let state = &observed[0];
        let page = &observed[1];
        assert!(page.contains(SAMPLE_DEVELOPER_TOKEN));
        assert!(page.contains(state));
        assert!(observed[2].starts_with("HTTP/1.1 200"));
    }

    #[test]
    fn authorize_rejects_a_malformed_developer_token_without_opening() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_opener = Arc::clone(&calls);
        let opener = move |_url: &str| {
            calls_for_opener.fetch_add(1, Ordering::SeqCst);
            Ok(())
        };

        let error = authorize_with_timeout("not-a-jwt", &opener, Duration::from_millis(50))
            .expect_err("a malformed developer token is rejected");

        assert!(error.to_string().contains("developer token"));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn authorize_rejects_a_callback_with_the_wrong_state() {
        let (opener, handles) = background(|port, _state| {
            let _ = request(port, &token_request("wrong-state", SAMPLE_USER_TOKEN));
        });

        let error = authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
            .expect_err("a wrong state is rejected");
        join_all(&handles);
        assert!(error.to_string().contains("state"));
    }

    #[test]
    fn authorize_rejects_a_callback_with_an_empty_user_token() {
        let (opener, handles) = background(|port, state| {
            let _ = request(port, &token_request(&state, ""));
        });

        let error = authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
            .expect_err("an empty user token is rejected");
        join_all(&handles);
        assert!(error.to_string().contains("user token"));
    }

    #[test]
    fn authorize_ignores_an_unknown_request_before_the_callback() {
        let observed = Arc::new(Mutex::new(String::new()));
        let observed_for_flow = Arc::clone(&observed);
        let (opener, handles) = background(move |port, state| {
            let missing = request(port, "GET /nope HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
            let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
            *observed_for_flow.lock().expect("observed lock") = missing;
        });

        authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
            .expect("the real callback still succeeds");
        join_all(&handles);

        assert!(
            observed
                .lock()
                .expect("observed lock")
                .starts_with("HTTP/1.1 404")
        );
    }

    #[test]
    fn authorize_survives_a_connection_that_closes_early() {
        let (opener, handles) = background(|port, state| {
            // A client that connects and closes without a complete request.
            drop(TcpStream::connect(("127.0.0.1", port)).expect("connect"));
            // Then the real callback.
            let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
        });

        authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
            .expect("an aborted connection does not abort the flow");
        join_all(&handles);
    }

    #[test]
    fn authorize_survives_an_oversized_content_length() {
        let (opener, handles) = background(|port, state| {
            // `Content-Length: usize::MAX` overflows a naive `body_start + len`.
            let oversized = "POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 18446744073709551615\r\n\r\n";
            let _ = request(port, oversized);
            // Then the real callback.
            let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
        });

        authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
            .expect("an oversized Content-Length does not abort the flow");
        join_all(&handles);
    }

    #[test]
    fn authorize_times_out_without_a_callback() {
        let opener = |_url: &str| Ok(());

        let error =
            authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_millis(120))
                .expect_err("no callback times out");
        assert!(error.to_string().contains("timed out"));
    }
}
