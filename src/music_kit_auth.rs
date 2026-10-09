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
//! embeds. Every request is read into a buffer capped at
//! `MAX_REQUEST_BYTES` (defined in the `http` submodule): the cap is checked
//! while reading headers *and* before the declared body is read, so a hostile
//! client cannot make the server allocate without bound or panic it with an
//! overflowing `Content-Length`. Per-connection failures — a client that closes
//! early, stalls, or declares an oversized body — are ignored and the server
//! keeps waiting for the real callback; only a well-formed callback carrying an
//! invalid user token ends the flow with an error, while a callback with the
//! wrong `state` is answered and ignored so a local client cannot abort the
//! sign-in by forging one. Connections are served one at a time, so each is
//! bounded by [`CONNECTION_READ_TIMEOUT`] from the moment it is accepted — a
//! local client that opens a connection and dribbles a partial request can
//! hold the accept loop for that budget, not for the whole [`AUTH_TIMEOUT`],
//! so it cannot starve the browser's callback until the flow gives up. Each
//! response write carries its own bounded timeout, so a client that stops
//! reading cannot hold the connection there either.

use std::collections::hash_map::RandomState;
use std::env;
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::io;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crate::music_error::AppleMusicError;

mod http;
mod page;

use http::{CONNECTION_READ_TIMEOUT, read_http_request, write_response};
use page::{SUCCESS_PAGE, render_auth_page, render_bootstrap_page};

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
                // Connections are served sequentially, so cap the whole
                // connection, not just each read: a client that opens one and
                // dribbles a partial request would otherwise hold the accept
                // loop until the flow deadline and starve the browser's
                // callback. A legitimate request arrives in milliseconds, so
                // this budget is generous. `min` keeps the flow deadline the
                // outer bound.
                let connection_deadline = deadline.min(Instant::now() + connection_budget);
                match serve_connection(&mut stream, developer_token, &nonce, connection_deadline) {
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

/// Opens `path` in the system's default browser.
///
/// `path` is the owner-only bootstrap page [`BootstrapPage`] wrote, not the
/// sign-in URL: the URL carries the `state` nonce, and a command-line argument
/// is world-readable on Unix, so only the local file path may reach the opener.
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
pub fn open_in_browser(path: &str) -> io::Result<()> {
    if cfg!(target_os = "macos") {
        spawn_opener("open", &[path])
    } else if cfg!(target_os = "windows") {
        spawn_opener("cmd", &["/C", "start", "", path])
    } else {
        spawn_opener("xdg-open", &[path])
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

/// A private, single-use bootstrap page that redirects the browser to the
/// sign-in URL.
///
/// The sign-in URL carries the per-flow `state` nonce. Handing it to the
/// browser as a command-line argument would expose it to every local user
/// through `/proc/<pid>/cmdline` on Linux (and `ps` on macOS). Writing the URL
/// into an owner-only file and opening *that* file keeps the nonce out of the
/// only world-readable channel to the browser: the spawned opener and browser
/// see this file's path, never the URL. The file and its directory are removed
/// when the guard is dropped, after the flow has finished with them.
struct BootstrapPage {
    directory: PathBuf,
    file: PathBuf,
}

impl BootstrapPage {
    /// Writes a redirect page carrying `url` into a fresh owner-only temp
    /// directory and returns the guard that owns it.
    fn write(url: &str) -> io::Result<Self> {
        // Retry on the vanishingly unlikely name collision so a stale entry
        // cannot wedge the flow; `create_dir` refuses an existing path, so a
        // pre-planted file or symlink is never followed.
        for _ in 0..8 {
            let directory = env::temp_dir().join(format!("nostalgia-auth-{}", random_nonce()));
            match create_private_dir(&directory) {
                Ok(()) => {
                    let file = directory.join("sign-in.html");
                    write_private_file(&file, &render_bootstrap_page(url))?;
                    return Ok(Self { directory, file });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not create a private temp directory for the sign-in page",
        ))
    }

    /// The local file path handed to the browser opener.
    fn path_string(&self) -> String {
        self.file.to_string_lossy().into_owned()
    }
}

impl Drop for BootstrapPage {
    /// Removes the page and its directory. A failure is ignored: the flow has
    /// already finished, so there is nothing left to retry or report.
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.file);
        let _ = fs::remove_dir(&self.directory);
    }
}

/// Creates `directory` with owner-only permissions. The mode is set at
/// creation (via `mkdir`), so there is no window where another user can enter
/// the directory.
#[cfg(unix)]
fn create_private_dir(directory: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(0o700).create(directory)
}

/// Creates `directory`. The world-readable `/proc` argument channel this guard
/// defends against is Unix-specific, so the non-Unix fallback needs no mode.
#[cfg(not(unix))]
fn create_private_dir(directory: &Path) -> io::Result<()> {
    fs::DirBuilder::new().create(directory)
}

/// Writes `contents` to `file`, creating it with owner-only permissions and
/// refusing to overwrite an existing path.
#[cfg(unix)]
fn write_private_file(file: &Path, contents: &str) -> io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut handle = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(file)?;
    handle.write_all(contents.as_bytes())
}

/// Writes `contents` to `file`, refusing to overwrite an existing path. The
/// world-readable `/proc` argument channel this guard defends against is
/// Unix-specific, so the non-Unix fallback needs no mode.
#[cfg(not(unix))]
fn write_private_file(file: &Path, contents: &str) -> io::Result<()> {
    let mut handle = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file)?;
    handle.write_all(contents.as_bytes())
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
        // The page embeds the owner-side developer token, and the browser was
        // opened at `/?state=<nonce>`, so the legitimate page request already
        // presents the per-flow nonce. Refuse any request that does not: a
        // local client that can reach the loopback port but does not know the
        // nonce would otherwise read the developer token and the nonce, then
        // forge a callback.
        if !page_query_carries_nonce(request.query.as_deref(), nonce) {
            let _ = write_response(
                stream,
                403,
                "Forbidden",
                "text/plain; charset=utf-8",
                "Forbidden",
            );
            return Connection::Continue;
        }
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

/// Whether the page request's query carries this flow's `state` nonce.
///
/// The browser is opened at `/?state=<nonce>`, so the legitimate page request
/// already presents the per-flow secret. Requiring it here means a local
/// client that reaches the loopback port without knowing the nonce is refused
/// the sign-in page and the developer token it embeds.
fn page_query_carries_nonce(query: Option<&str>, nonce: &str) -> bool {
    query.is_some_and(|query| {
        query.split('&').any(|pair| {
            pair.split_once('=')
                .is_some_and(|(key, value)| key == "state" && percent_decode(value) == nonce)
        })
    })
}

/// Handles the `POST /token` callback: validates `state` and `userToken` and
/// either returns the session, ignores a wrong-state request, or rejects a
/// missing or malformed user token, naming which it was and — for a malformed
/// one — the shape defect.
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
        // A wrong state is ignored, not fatal: any local client can reach
        // this route without knowing the nonce, so aborting on one would let
        // it end a sign-in the user is completing. The real callback (with the
        // right state) still arrives on a later connection.
        return Connection::Continue;
    }

    match user_token {
        Some(token) => match jwt_shape_problem(&token) {
            None => {
                let _ = write_response(stream, 200, "OK", "text/html; charset=utf-8", SUCCESS_PAGE);
                Connection::Authorized(MusicKitSession {
                    developer_token: developer_token.to_string(),
                    user_token: token,
                })
            }
            // Name the defect rather than a generic "invalid user token",
            // mirroring `validate_developer_token`: a broken callback is then
            // diagnosable from the logged error. The phrase is fixed (see
            // `jwt_shape_problem`), so the token itself stays out of the log.
            Some(problem) => reject_token(
                stream,
                &format!(
                    "the sign-in callback's user token must be a three-segment base64url JWT, but {problem}"
                ),
            ),
        },
        None => reject_token(stream, "the sign-in callback did not carry a user token"),
    }
}

/// Answers an invalid `POST /token` callback with a `400` and ends the flow
/// with `message`. Shared by the two rejection cases — a missing user token
/// and a malformed one — so the response and return shape live here once.
/// `message` never echoes the token: the malformed case is named by the fixed
/// phrase [`jwt_shape_problem`] returns.
fn reject_token(stream: &mut TcpStream, message: &str) -> Connection {
    let _ = write_response(
        stream,
        400,
        "Bad Request",
        "text/plain; charset=utf-8",
        message,
    );
    Connection::Rejected(AppleMusicError::new(message))
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

/// Why `token` is not a three-segment base64url JWT, or `None` when it is.
///
/// The reason is a fixed phrase — never the token itself, which is a secret
/// and must not reach a log — so [`validate_developer_token`] and
/// [`handle_token`] can name the defect without echoing the value. The checks
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
