//! Routing one parsed HTTP request for the sign-in server: serve the page,
//! take the `POST /token` callback, or answer a refused or unknown request,
//! and build each response.

use std::net::TcpStream;

use crate::music_error::AppleMusicError;

use super::super::http::{HttpRequest, write_response};
use super::super::page::{SUCCESS_PAGE, render_auth_page};
use super::super::{MusicKitSession, jwt_shape_problem};
use super::Connection;
use super::query::{
    form_pairs, is_loopback_host, nonce_matches, page_query_carries_nonce, percent_decode,
};

/// Routes one parsed request, turning transport-level failures into
/// [`Connection::Continue`] so a bad request never aborts the sign-in.
///
/// The request has already been read; this decides what it asks for and
/// answers it.
pub(super) fn route_request(
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
        write_flow_response(
            stream,
            "the sign-in page",
            200,
            "OK",
            "text/html; charset=utf-8",
            &page,
        );
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
        write_flow_response(
            stream,
            "the sign-in success page",
            200,
            "OK",
            "text/html; charset=utf-8",
            SUCCESS_PAGE,
        );
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

/// Writes a response the sign-in flow depends on and reports a failed write.
///
/// Only the sign-in page and the callback's success page go through this: the
/// browser must receive them for the flow to make sense, so a write failure is
/// logged with its OS cause instead of surfacing only as the generic sign-in
/// timeout. The refusal and not-found responses keep the best-effort
/// `let _ = write_response(..)` form, so a hostile local client that hangs up
/// early cannot flood the log.
fn write_flow_response(
    stream: &mut TcpStream,
    what: &str,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &str,
) {
    write_flow_response_with(
        what,
        |body| write_response(stream, status, reason, content_type, body),
        |report| eprintln!("{report}"),
        body,
    );
}

/// [`write_flow_response`] with the write and the reporter injected, so a test
/// can drive the failure branch without a socket that refuses a write.
fn write_flow_response_with(
    what: &str,
    write: impl FnOnce(&str) -> std::io::Result<()>,
    report: impl FnOnce(String),
    body: &str,
) {
    if let Err(error) = write(body) {
        report(response_write_failure_report(what, &error));
    }
}

/// The report for a response the sign-in flow could not deliver: `what` names
/// the response (`"the sign-in page"`, say) and `error` carries the OS cause,
/// so a browser that never received its page is diagnosable from the log.
fn response_write_failure_report(what: &str, error: &impl std::fmt::Display) -> String {
    format!("could not send {what}: {error}")
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

#[cfg(test)]
mod tests {
    use super::write_flow_response_with;

    #[test]
    fn a_failed_write_reports_the_response_and_its_cause() {
        let mut reported = None;
        write_flow_response_with(
            "the sign-in page",
            |_| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "the pipe is gone",
                ))
            },
            |report| reported = Some(report),
            "body",
        );

        assert_eq!(
            reported.as_deref(),
            Some("could not send the sign-in page: the pipe is gone")
        );
    }

    #[test]
    fn a_successful_write_reports_nothing() {
        let mut reports = 0;
        write_flow_response_with("the sign-in page", |_| Ok(()), |_| reports += 1, "body");

        assert_eq!(reports, 0);
    }
}
