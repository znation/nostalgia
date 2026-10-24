//! The loopback server that serves the sign-in page and captures the
//! `POST /token` callback.
//!
//! Each accepted connection is read without waiting, so a client that stalls
//! cannot hold the browser's callback behind it (see [`serve_pending`] and
//! [`accept_pending`]).

use std::io;
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use crate::music_error::AppleMusicError;

use super::http::{HttpRequest, RequestReader, write_response};
use super::page::{SUCCESS_PAGE, render_auth_page};
use super::{MusicKitSession, jwt_shape_problem};

/// How many accepted connections the server reads at once.
///
/// The connections are read one after another but never *waited* for, so this
/// only bounds the server's share of the process's file descriptors; the
/// oldest pending connection makes way for the newest when the bound is
/// reached.
pub(super) const MAX_PENDING_CONNECTIONS: usize = 64;

/// How many connections one pass of the accept loop takes on.
///
/// The bound keeps a flood of connections from filling the pending list in one
/// pass, and it bounds the wait of a connection that is queued behind others:
/// a callback queued behind N connections is taken on within
/// `ceil((N + 1) / MAX_ACCEPTS_PER_PASS)` passes and served in the pass after
/// that, so a callback queued behind eight other connections waits one pass
/// per eight queued connections.
pub(super) const MAX_ACCEPTS_PER_PASS: usize = 8;

/// A connection the server has accepted but whose request has not arrived yet.
pub(super) struct PendingConnection {
    /// The reader that accumulates the connection's request.
    pub(super) reader: RequestReader,
    /// When this connection's budget is spent: the flow's deadline, or the
    /// moment it was accepted plus the per-connection budget, whichever comes
    /// first.
    deadline: Instant,
}

/// Serves the pending connections whose request has arrived.
///
/// Each connection is read without waiting: one that has not sent a whole
/// request yet stays pending and is read again on the next pass, one that has
/// is served at once, and one that closed or sent a request that can never be
/// served is dropped. A connection whose budget is spent is dropped too, so a
/// stalled client cannot hold the flow.
///
/// Returns [`Connection::Authorized`] when the browser's callback completed
/// the flow and [`Connection::Rejected`] when a well-formed callback was
/// rejected; otherwise the flow continues.
pub(super) fn serve_pending(
    pending: &mut Vec<PendingConnection>,
    developer_token: &str,
    nonce: &str,
) -> Connection {
    let mut index = 0;
    while index < pending.len() {
        if Instant::now() >= pending[index].deadline {
            // The connection's budget is spent: drop it, so a stalled client
            // cannot hold the flow.
            pending.remove(index);
            continue;
        }
        let outcome = match pending[index].reader.poll() {
            Ok(Some(request)) => {
                let outcome = route_request(
                    pending[index].reader.stream(),
                    &request,
                    developer_token,
                    nonce,
                );
                pending.remove(index);
                outcome
            }
            // The request is not whole yet: keep the connection and read it
            // again on the next pass.
            Ok(None) => {
                index += 1;
                continue;
            }
            // A connection that closed or sent a request that can never be
            // served is dropped; the flow keeps waiting for the real callback.
            Err(_) => {
                pending.remove(index);
                continue;
            }
        };
        match outcome {
            Connection::Continue => {}
            Connection::Authorized(session) => return Connection::Authorized(session),
            Connection::Rejected(error) => return Connection::Rejected(error),
        }
    }
    Connection::Continue
}

/// Takes on the connections that are waiting to be served.
///
/// The listener is non-blocking, so this never waits: a connection that has
/// not arrived yet is left in the kernel's queue for the next pass. Each
/// accepted connection is bounded by the per-connection budget from the moment
/// it is accepted, with the flow's deadline as the outer bound.
pub(super) fn accept_pending(
    listener: &TcpListener,
    pending: &mut Vec<PendingConnection>,
    deadline: Instant,
    connection_budget: Duration,
) -> io::Result<()> {
    for _ in 0..MAX_ACCEPTS_PER_PASS {
        let (stream, _address) = match listener.accept() {
            Ok(accepted) => accepted,
            Err(ref error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
            Err(error) => return Err(error),
        };
        let reader = match RequestReader::new(stream) {
            Ok(reader) => reader,
            // A connection the server cannot read is dropped; the flow keeps
            // waiting for the real callback.
            Err(_) => continue,
        };
        // The oldest pending connection makes way for the newest, so the
        // server reads a bounded number of connections at once.
        if pending.len() >= MAX_PENDING_CONNECTIONS {
            pending.remove(0);
        }
        let connection_deadline = deadline.min(Instant::now() + connection_budget);
        pending.push(PendingConnection {
            reader,
            deadline: connection_deadline,
        });
    }
    Ok(())
}

/// What serving the connections decided for the overall flow.
pub(super) enum Connection {
    /// The callback completed and produced a session.
    Authorized(MusicKitSession),
    /// The request was served or ignored; keep accepting.
    Continue,
    /// A well-formed callback was rejected; stop with this error.
    Rejected(AppleMusicError),
}

/// Routes one parsed request, turning transport-level failures into
/// [`Connection::Continue`] so a bad request never aborts the sign-in.
///
/// The request has already been read; this decides what it asks for and
/// answers it.
fn route_request(
    stream: &mut TcpStream,
    request: &HttpRequest,
    developer_token: &str,
    nonce: &str,
) -> Connection {
    // Browsers set `Host` from the URL and forbid scripts from overriding it,
    // so requiring the loopback address stops a DNS-rebinding page (which
    // reaches this socket under its own hostname) from reading the sign-in
    // page's developer token and `state` nonce. A hostile request is ignored,
    // not fatal, so it cannot abort a sign-in the user is completing.
    if !is_loopback_host(request.host.as_deref()) {
        return forbid(stream);
    }

    if request.method == "GET" && request.path == "/" {
        // The page embeds the owner-side developer token, and the browser was
        // opened at `/?state=<nonce>`, so the legitimate page request already
        // presents the per-flow nonce. Refuse any request that does not: a
        // local client that can reach the loopback port but does not know the
        // nonce would otherwise read the developer token and the nonce, then
        // forge a callback.
        if !page_query_carries_nonce(request.query.as_deref(), nonce) {
            return forbid(stream);
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

/// Answers a request that fails a security guard with a `403 Forbidden`
/// plain-text body and keeps the sign-in flow alive. Both the non-loopback
/// `Host` and the page request without the state nonce are refused this way,
/// so the response and the `Continue` return live here once.
fn forbid(stream: &mut TcpStream) -> Connection {
    let _ = write_response(
        stream,
        403,
        "Forbidden",
        "text/plain; charset=utf-8",
        "Forbidden",
    );
    Connection::Continue
}

/// Splits `application/x-www-form-urlencoded` text into its `name=value`
/// pairs.
///
/// Both the GET page request's query string and the POST callback's body use
/// this syntax, so the `&`-separated pair scan lives here once; the caller
/// decodes the value it wants with [`percent_decode`] and applies its own
/// duplicate-field rule. A pair without `=` names no field and is skipped.
fn form_pairs(form: &str) -> impl Iterator<Item = (&str, &str)> {
    form.split('&').filter_map(|pair| pair.split_once('='))
}

/// Whether the page request's query carries this flow's `state` nonce.
///
/// The browser is opened at `/?state=<nonce>`, so the legitimate page request
/// already presents the per-flow secret. Requiring it here means a local
/// client that reaches the loopback port without knowing the nonce is refused
/// the sign-in page and the developer token it embeds.
fn page_query_carries_nonce(query: Option<&str>, nonce: &str) -> bool {
    query.is_some_and(|query| {
        form_pairs(query)
            .any(|(key, value)| key == "state" && nonce_matches(&percent_decode(value), nonce))
    })
}

/// Whether `candidate` equals this flow's `state` nonce, compared in constant
/// time.
///
/// Both checks of the nonce — the page query and the callback body — compare a
/// value any local client can choose against the one secret that gates the
/// developer token. Ordinary string equality stops at the first differing
/// byte, so a client that times the server's replies could learn how many
/// leading bytes of a guess matched and recover the nonce one byte at a time.
/// Here every byte of both values is folded into the same accumulator, and a
/// length difference is folded into it too rather than returning early, so a
/// reply's timing does not depend on where a guess first differs. The nonce's
/// own length is fixed and public (see `random_nonce`), so the loop length
/// depends only on values the client already knows.
pub(super) fn nonce_matches(candidate: &str, nonce: &str) -> bool {
    let candidate = candidate.as_bytes();
    let nonce = nonce.as_bytes();
    let mut difference = u8::from(candidate.len() != nonce.len());
    for index in 0..candidate.len().max(nonce.len()) {
        let left = candidate.get(index).copied().unwrap_or(0);
        let right = nonce.get(index).copied().unwrap_or(0);
        difference |= left ^ right;
    }
    difference == 0
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
    for (key, value) in form_pairs(&form) {
        match key {
            "state" => state = Some(percent_decode(value)),
            "userToken" => user_token = Some(percent_decode(value)),
            _ => {}
        }
    }

    let matches = state
        .as_deref()
        .is_some_and(|state| nonce_matches(state, nonce));
    if !matches {
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

    let Some(token) = user_token else {
        return reject_token(stream, "the sign-in callback did not carry a user token");
    };
    let Some(problem) = jwt_shape_problem(&token) else {
        let _ = write_response(stream, 200, "OK", "text/html; charset=utf-8", SUCCESS_PAGE);
        return Connection::Authorized(MusicKitSession {
            developer_token: developer_token.to_string(),
            user_token: token,
        });
    };
    // Name the defect rather than a generic "invalid user token", mirroring
    // `validate_developer_token`: a broken callback is then diagnosable from
    // the logged error. The phrase is fixed (see `jwt_shape_problem`), so the
    // token itself stays out of the log.
    reject_token(
        stream,
        &format!(
            "the sign-in callback's user token must be a three-segment base64url JWT, but {problem}"
        ),
    )
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

/// Whether the request's `Host` header names the loopback address the server
/// is bound to, with or without a port.
pub(super) fn is_loopback_host(host: Option<&str>) -> bool {
    let Some(host) = host else {
        return false;
    };
    let name = host.rsplit_once(':').map_or(host, |(name, _port)| name);
    name == "127.0.0.1"
}

/// Decodes a form-urlencoded value (`+` for space, `%XX` escapes).
pub(super) fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'+' {
            decoded.push(b' ');
            index += 1;
            continue;
        }
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
        {
            decoded.push(high * 16 + low);
            index += 3;
            continue;
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
