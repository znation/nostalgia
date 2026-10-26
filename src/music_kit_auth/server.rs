//! The loopback server that serves the sign-in page and captures the
//! `POST /token` callback.
//!
//! Each accepted connection is read without waiting, so a stalled client holds
//! only its own slot, and only until its budget is spent. When all slots are
//! taken, a newer connection (the browser's callback included) waits in the
//! kernel accept queue until a slot frees (see [`serve_pending`] and
//! [`accept_pending`]).
//!
//! Request routing and response construction live in `route`; the
//! form-urlencoded parsing, state-nonce comparison, and loopback `Host` check
//! live in `query`.

use std::io;
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use crate::music_error::AppleMusicError;

use super::MusicKitSession;
use super::http::RequestReader;

pub(super) mod query;
mod route;

use route::route_request;

/// How many accepted connections the server reads at once.
///
/// The connections are read one after another but never *waited* for, so this
/// only bounds the server's share of the process's file descriptors. When the
/// bound is reached the accept loop stops taking on connections until a
/// pending one is served or its budget is spent; an accepted connection is
/// never displaced by a newer one.
pub(super) const MAX_PENDING_CONNECTIONS: usize = 64;

/// How many connections one pass of the accept loop takes on.
///
/// The bound keeps a flood of connections from filling the pending list in one
/// pass. While the pending list has room, a callback queued behind N
/// connections is taken on within `ceil((N + 1) / MAX_ACCEPTS_PER_PASS)`
/// passes and served in the pass after that. When the pending list is full
/// ([`MAX_PENDING_CONNECTIONS`]) nothing is accepted, so the callback waits in
/// the kernel queue until a pending connection is served or its budget is
/// spent.
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
/// not arrived yet is left in the kernel's queue for the next pass. When the
/// pending list is full this accepts nothing, leaving the waiting connections
/// in the kernel's accept queue until a pending one is served or its budget is
/// spent, so a flood cannot displace a connection the server has already
/// accepted. Each accepted connection is bounded by the per-connection budget
/// from the moment it is accepted, with the flow's deadline as the outer bound.
///
/// A transient `accept` failure — a signal interrupting the call, or a peer
/// that reset after being queued but before it was accepted — is retried
/// within the same pass, up to `MAX_ACCEPTS_PER_PASS`, instead of ending the
/// flow, so one local client cannot abort a sign-in the user is still
/// completing. A failure of the listener itself is still returned.
pub(super) fn accept_pending(
    listener: &TcpListener,
    pending: &mut Vec<PendingConnection>,
    deadline: Instant,
    connection_budget: Duration,
) -> io::Result<()> {
    accept_pending_with(|| listener.accept(), pending, deadline, connection_budget)
}

/// [`accept_pending`] with the `accept` call injected, so a test can drive
/// transient failures without relying on the OS to produce one.
pub(super) fn accept_pending_with(
    mut accept: impl FnMut() -> io::Result<(TcpStream, std::net::SocketAddr)>,
    pending: &mut Vec<PendingConnection>,
    deadline: Instant,
    connection_budget: Duration,
) -> io::Result<()> {
    for _ in 0..MAX_ACCEPTS_PER_PASS {
        // The list is full: leave the rest in the kernel's queue and try again
        // next pass rather than evicting a connection already accepted (which
        // could be the browser's callback between its header and body reads).
        if pending.len() >= MAX_PENDING_CONNECTIONS {
            return Ok(());
        }
        let (stream, _address) = match accept() {
            Ok(accepted) => accepted,
            Err(ref error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
            // An interruption or a reset of a queued connection says nothing
            // about the listener, so retry it (bounded by
            // `MAX_ACCEPTS_PER_PASS`) instead of failing the sign-in.
            Err(ref error) if accept_error_is_transient(error) => continue,
            Err(error) => return Err(error),
        };
        let reader = match RequestReader::new(stream) {
            Ok(reader) => reader,
            // A connection the server cannot read is dropped; the flow keeps
            // waiting for the real callback.
            Err(_) => continue,
        };
        let connection_deadline = deadline.min(Instant::now() + connection_budget);
        pending.push(PendingConnection {
            reader,
            deadline: connection_deadline,
        });
    }
    Ok(())
}

/// Whether an `accept` failure is a transient interruption the loop will
/// retry, rather than a failure of the listener.
///
/// `Interrupted` means a signal arrived before a connection could be accepted,
/// while `ConnectionAborted` and `ConnectionReset` both mean a peer queued for
/// acceptance reset its connection first (the platform chooses which kind to
/// report). None says anything about the listener, so treating any as fatal
/// would let one local client (or one signal) end a sign-in the user is still
/// completing.
fn accept_error_is_transient(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::Interrupted
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
    )
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
