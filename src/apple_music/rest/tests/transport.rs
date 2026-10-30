use super::*;
use crate::music_error::AppleMusicErrorKind;

// `ureq` defaults every network timeout to `None`, so a server that accepts
// the connection and never sends a response used to block `UreqTransport::get`
// forever — the UI's own fetch timeout only abandons the detached `off_thread`
// thread, it does not stop it. The bound is asserted from a worker thread with
// `recv_timeout`, so an unbounded transport fails this test at 5s instead of
// hanging the suite.
#[test]
fn a_stalled_server_is_bounded_by_the_request_timeout() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let transport = UreqTransport::with_timeout(std::time::Duration::from_millis(200));
    let url = format!("http://{addr}/stalled");
    let (sender, receiver) = std::sync::mpsc::channel();
    let started = std::time::Instant::now();
    std::thread::spawn(move || {
        let _ = sender.send(transport.get(&url, &rest_session()));
    });
    let result = receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the transport must return within its bound");
    assert!(result.is_err(), "{result:?}");
    // A connect failure would also return `Err`, instantly; require that the
    // call actually waited for the bound, so this test only passes because the
    // stalled response was timed out.
    assert!(
        started.elapsed() >= std::time::Duration::from_millis(100),
        "returned before the 200ms bound: {:?}",
        started.elapsed()
    );
}

// The agent's global bound is not the only one that matters: a paginated fetch
// shares one deadline across its pages, so a single request must also honor a
// deadline shorter than the agent's. The shared agent carries 30 seconds, so
// without the per-request override this call would block for the full agent
// bound; the 200ms deadline is what makes it return.
#[test]
fn a_request_is_bounded_by_its_deadline_not_only_the_agent_timeout() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let transport = UreqTransport::new();
    let url = format!("http://{addr}/stalled");
    let deadline = Instant::now() + std::time::Duration::from_millis(200);
    let (sender, receiver) = std::sync::mpsc::channel();
    let started = Instant::now();
    std::thread::spawn(move || {
        let _ = sender.send(transport.get_within(&url, &rest_session(), deadline));
    });
    let result = receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the transport must return within its deadline");
    assert!(result.is_err(), "{result:?}");
    assert!(
        started.elapsed() >= std::time::Duration::from_millis(100),
        "returned before the 200ms deadline: {:?}",
        started.elapsed()
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "the 30s agent bound leaked through instead of the 200ms deadline: {:?}",
        started.elapsed()
    );
}

// A fetch whose shared deadline has already passed must fail immediately
// without opening a connection, so a later page cannot start a fresh request
// after the UI has stopped waiting.
#[test]
fn an_expired_deadline_fails_without_opening_a_connection() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let transport = UreqTransport::new();
    let url = format!("http://{addr}/never");

    let error = transport
        .get_within(&url, &rest_session(), Instant::now())
        .unwrap_err()
        .to_string();

    assert!(error.contains("deadline"), "{error}");
    assert!(
        matches!(
            listener.accept(),
            Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock
        ),
        "the transport opened a connection after its deadline had passed"
    );
}

/// Formats a complete HTTP/1.1 response around `body`: the `status_line` (e.g. `"200 OK"`), the
/// `Content-Type` header, and the exact `Content-Length`, closed with `Connection: close`. The
/// loopback tests either hand the result to [`serve_one_response`] or write it from a
/// request-capturing thread, so the framing lives here rather than being rebuilt at each site.
fn http_response(status_line: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

// The API requests carry the user token in the custom `Music-User-Token`
// header, and `ureq` follows redirects by default while stripping only
// `Authorization`, `Cookie`, and `Content-Length` from the redirected request
// — a custom header survives. A response whose `Location` names another host
// would therefore re-send the user token there. Pin the guard at the
// transport: a redirect is not followed, so the token never leaves the URL the
// client built.
#[test]
fn a_redirect_is_not_followed_so_the_user_token_cannot_leak() {
    let (foreign, foreign_addr) = loopback_listener();

    // The redirecting server answers the one request with a 302 pointing at
    // the "foreign" server, exactly what a hostile or compromised API reply
    // could return. `serve_one_response` binds, accepts, reads, and writes the
    // reply, so the redirecting server is the same one-shot loopback server
    // the response tests use.
    let redirector_addr = serve_one_response(&format!(
        "HTTP/1.1 302 Found\r\nLocation: http://{foreign_addr}/leak\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    ));

    // The foreign server records whether any redirected request arrived; the
    // shared recorder polls for one second, so "no request" is a bounded,
    // observable result rather than a hang.
    let foreign_thread = record_first_request(foreign);

    let transport = UreqTransport::with_timeout(std::time::Duration::from_secs(2));
    let url = format!("http://{redirector_addr}/redirect");
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(transport.get(&url, &rest_session()));
    });
    let _ = receiver.recv_timeout(std::time::Duration::from_secs(5));

    let leaked = foreign_thread.join().unwrap().request;
    assert!(
        leaked.is_none(),
        "the user token reached the redirect target: {leaked:?}"
    );
}

// The production transport is the one that carries real credentials, and it
// uses the process-wide `shared_agent` rather than the test-only
// `with_timeout` path. The stalled-server and redirect tests construct their
// transport with `with_timeout`, so they pin `agent_with_timeout` but would
// stay green if `shared_agent` stopped calling it; the non-success-status
// tests go through `UreqTransport::new()` and so cover the status policy
// behaviorally. Pin the shared agent's three transport choices directly: a
// regression rebuilding it from `ureq`'s defaults — no global timeout,
// redirects followed, status-as-error on — could otherwise block the detached
// browse thread forever, send the `Music-User-Token` past a redirect, or
// discard Apple's error body.
#[test]
fn the_shared_agent_carries_the_documented_transport_settings() {
    assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(30));
    let config = shared_agent().config();
    assert_eq!(config.timeouts().global, Some(REQUEST_TIMEOUT));
    assert_eq!(config.max_redirects(), 0);
    assert!(!config.http_status_as_error());
}

// The `api_error_cause` unit tests pin the parsing `UreqTransport::get`
// relies on, without a network: the documented envelope's `detail`, the
// `title` fallback, and the bodies that must not be mistaken for a cause.

#[test]
fn api_error_cause_prefers_the_detail() {
    let body = r#"{"errors":[{"title":"Unauthorized","detail":"Invalid developer token"}]}"#;
    assert_eq!(
        api_error_cause(body),
        Some("Invalid developer token".to_string())
    );
}

#[test]
fn api_error_cause_falls_back_to_the_title() {
    let body = r#"{"errors":[{"title":"Unauthorized"}]}"#;
    assert_eq!(api_error_cause(body), Some("Unauthorized".to_string()));
}

// A field the API supplies but leaves blank names no cause, so it must not
// win over a present `title` — otherwise the status message would end in a
// dangling ": " instead of naming the cause the title carries.
#[test]
fn api_error_cause_skips_a_blank_detail_for_the_title() {
    let body = r#"{"errors":[{"title":"Unauthorized","detail":""}]}"#;
    assert_eq!(api_error_cause(body), Some("Unauthorized".to_string()));
}

// Apple may return several `errors` entries, and the first may name no cause
// (an entry carrying only a `status`, say). The search must continue to a later
// entry rather than returning `None` and dropping a cause the body carries, so
// the status message names it instead of reporting the bare status.
#[test]
fn api_error_cause_falls_through_a_causeless_entry_to_a_later_cause() {
    let body = r#"{"errors":[{"status":"401"},{"title":"Unauthorized","detail":"Invalid developer token"}]}"#;
    assert_eq!(
        api_error_cause(body),
        Some("Invalid developer token".to_string())
    );
}

// Entries are searched in order, and the `detail`-before-`title` preference
// holds only *within* an entry: an earlier entry's non-blank `title` is the
// cause even when a later entry carries a non-blank `detail`. Apple lists its
// errors most-significant-first, so a refactor that pooled every entry's
// `detail` ahead of every `title` (a two-pass search) would report a secondary
// error's detail instead of the primary error's title. The causeless-entry
// test above never reaches this case, because its first entry has no cause at
// all; pin the cross-entry ordering.
#[test]
fn api_error_cause_prefers_an_earlier_entrys_title_to_a_later_detail() {
    let body = r#"{"errors":[{"title":"Unauthorized"},{"detail":"Invalid developer token"}]}"#;
    assert_eq!(api_error_cause(body), Some("Unauthorized".to_string()));
}

// With every field present but blank there is no cause to add, so the
// transport reports the status alone rather than "HTTP 401: ".
#[test]
fn api_error_cause_is_none_when_every_field_is_blank() {
    let body = r#"{"errors":[{"title":"  ","detail":"\t"}]}"#;
    assert_eq!(api_error_cause(body), None);
}

#[test]
fn api_error_cause_is_none_without_an_envelope_or_cause() {
    assert_eq!(api_error_cause("not json"), None);
    assert_eq!(api_error_cause(r#"{"errors":[]}"#), None);
    assert_eq!(api_error_cause(r#"{"errors":[{"status":"401"}]}"#), None);
    assert_eq!(api_error_cause(r#"{"data":[]}"#), None);
}

// The cause is a third-party reply, not this program's text: it reaches the
// terminal through the browse failure report, so a raw escape sequence in the
// detail must be rendered as a visible escape rather than driving the
// terminal. Covers both a control character (`\u{1b}`) and a Unicode format
// character (`\u{202e}`), which `char::is_control` does not classify as one.
#[test]
fn api_error_cause_escapes_terminal_control_characters() {
    let body = r#"{"errors":[{"detail":"bad \u001b[31mtoken\u202e"}]}"#;
    let cause = api_error_cause(body).unwrap();
    assert!(!cause.contains('\u{1b}'), "{cause:?}");
    assert!(!cause.contains('\u{202e}'), "{cause:?}");
    assert!(cause.contains("\\u{1b}"), "{cause:?}");
    assert!(cause.contains("\\u{202e}"), "{cause:?}");
}

// `ureq` turns a 4xx/5xx into a bare `Error::StatusCode` unless
// `http_status_as_error(false)` is set, which discards Apple's error body —
// the part that names the cause. `UreqTransport` now checks the status where
// the body is available and surfaces the envelope's `detail`, so a rejected
// token reads as "Invalid developer token" rather than just "401".
#[test]
fn a_non_success_status_surfaces_the_api_error_detail() {
    let body = r#"{"errors":[{"title":"Unauthorized","detail":"Invalid developer token"}]}"#;
    let addr = serve_one_response(&http_response("401 Unauthorized", "application/json", body));

    let url = format!("http://{addr}/me/library/artists");
    let error = UreqTransport::new().get(&url, &rest_session()).unwrap_err();
    assert_eq!(
        error.to_string(),
        "HTTP 401 Unauthorized: Invalid developer token"
    );
}

// A 401 whose error envelope carries only blank fields is the `None` arm at
// the transport: there is nothing to append, so the message is the bare
// status rather than a status with a dangling colon.
#[test]
fn a_non_success_status_with_a_blank_api_error_detail_reports_just_the_status() {
    let body = r#"{"errors":[{"title":"","detail":"  "}]}"#;
    let addr = serve_one_response(&http_response("401 Unauthorized", "application/json", body));

    let url = format!("http://{addr}/me/library/artists");
    let error = UreqTransport::new().get(&url, &rest_session()).unwrap_err();
    assert_eq!(error.to_string(), "HTTP 401 Unauthorized");
}

// The `None` arm of the same status check: a non-2xx response whose body is
// not the documented error envelope has no cause to add, so the message names
// only the status. Pinned so a regression cannot drop the status itself or
// invent a cause from an unrelated body.
#[test]
fn a_non_success_status_without_an_api_error_body_reports_just_the_status() {
    let body = "Not found";
    let addr = serve_one_response(&http_response("404 Not Found", "text/plain", body));

    let url = format!("http://{addr}/me/library/artists");
    let error = UreqTransport::new().get(&url, &rest_session()).unwrap_err();
    assert_eq!(error.to_string(), "HTTP 404 Not Found");
    assert_eq!(error.kind(), AppleMusicErrorKind::Other);
}

// A 401 reply is Apple Music rejecting the request's authorization — a stale
// or revoked MusicKit user token, an expired developer token — so the error
// is classified `Auth` while the message stays byte-identical to today's
// `HTTP <status>: <detail>` pin (the report above keeps it readable).
#[test]
fn an_http_401_reply_is_classified_as_an_auth_error_with_todays_message() {
    let body = r#"{"errors":[{"title":"Unauthorized","detail":"Invalid developer token"}]}"#;
    let addr = serve_one_response(&http_response("401 Unauthorized", "application/json", body));

    let url = format!("http://{addr}/me/library/artists");
    let error = UreqTransport::new().get(&url, &rest_session()).unwrap_err();
    assert_eq!(error.kind(), AppleMusicErrorKind::Auth);
    assert_eq!(
        error.to_string(),
        "HTTP 401 Unauthorized: Invalid developer token"
    );
}

// The 403 twin: a revoked token is the same recovery (sign in again), so it
// classifies `Auth` too, message unchanged.
#[test]
fn an_http_403_reply_is_classified_as_an_auth_error_with_todays_message() {
    let body = r#"{"errors":[{"title":"Forbidden","detail":"User token was revoked"}]}"#;
    let addr = serve_one_response(&http_response("403 Forbidden", "application/json", body));

    let url = format!("http://{addr}/me/library/artists");
    let error = UreqTransport::new().get(&url, &rest_session()).unwrap_err();
    assert_eq!(error.kind(), AppleMusicErrorKind::Auth);
    assert_eq!(
        error.to_string(),
        "HTTP 403 Forbidden: User token was revoked"
    );
}

// A 500 is not an auth rejection — the recovery is not signing in again — so
// it stays `Other` even though it is a non-success status.
#[test]
fn an_http_500_reply_classifies_as_other() {
    let body = r#"{"errors":[{"title":"Internal","detail":"try later"}]}"#;
    let addr = serve_one_response(&http_response(
        "500 Internal Server Error",
        "application/json",
        body,
    ));

    let url = format!("http://{addr}/me/library/artists");
    let error = UreqTransport::new().get(&url, &rest_session()).unwrap_err();
    assert_eq!(error.kind(), AppleMusicErrorKind::Other);
    assert_eq!(
        error.to_string(),
        "HTTP 500 Internal Server Error: try later"
    );
}

// [`http_response`] with a `Retry-After` header spliced in before the
// `Content-Length`, for the 429 pins the rate-limit classification needs;
// `None` omits the header entirely, pinning the no-header message.
fn http_response_with_retry_after(retry_after: Option<&str>, body: &str) -> String {
    let header = retry_after
        .map(|value| format!("Retry-After: {value}\r\n"))
        .unwrap_or_default();
    format!(
        "HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\n{header}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

// A 429 is Apple Music counting too many requests from this token — a
// transient failure the user can clear by waiting and retrying, not by
// signing in again. It is classified `Throttled` so the report can point at
// the panel's Retry button, and a whole-number-of-seconds `Retry-After` is
// echoed into the message so the report names how long to wait.
#[test]
fn an_http_429_reply_is_classified_as_throttled_with_the_retry_after() {
    let body = r#"{"errors":[{"title":"Too Many Requests","detail":"Rate limit exceeded"}]}"#;
    let addr = serve_one_response(&http_response_with_retry_after(Some("5"), body));

    let url = format!("http://{addr}/me/library/artists");
    let error = UreqTransport::new().get(&url, &rest_session()).unwrap_err();
    assert_eq!(error.kind(), AppleMusicErrorKind::Throttled);
    assert_eq!(
        error.to_string(),
        "HTTP 429 Too Many Requests: Rate limit exceeded (retry after 5 s)"
    );
}

// A 429 whose reply names no `Retry-After` still classifies `Throttled`,
// with the message byte-identical to what a plain failure would carry — the
// parenthetical appears only when the header parsed.
#[test]
fn an_http_429_without_a_retry_after_classifies_as_throttled_without_the_parenthetical() {
    let body = r#"{"errors":[{"title":"Too Many Requests","detail":"Rate limit exceeded"}]}"#;
    let addr = serve_one_response(&http_response_with_retry_after(None, body));

    let url = format!("http://{addr}/me/library/artists");
    let error = UreqTransport::new().get(&url, &rest_session()).unwrap_err();
    assert_eq!(error.kind(), AppleMusicErrorKind::Throttled);
    assert_eq!(
        error.to_string(),
        "HTTP 429 Too Many Requests: Rate limit exceeded"
    );
}

// A `Retry-After` this client cannot parse as a whole number of seconds —
// HTTP dates are the other documented form — is ignored rather than echoed,
// so the message never carries a value the transport itself cannot
// interpret.
#[test]
fn a_malformed_retry_after_is_not_echoed_into_the_message() {
    let body = r#"{"errors":[{"title":"Too Many Requests","detail":"Rate limit exceeded"}]}"#;
    let addr = serve_one_response(&http_response_with_retry_after(
        Some("Fri, 31 Dec 2100 23:59:59 GMT"),
        body,
    ));

    let url = format!("http://{addr}/me/library/artists");
    let error = UreqTransport::new().get(&url, &rest_session()).unwrap_err();
    assert_eq!(error.kind(), AppleMusicErrorKind::Throttled);
    assert_eq!(
        error.to_string(),
        "HTTP 429 Too Many Requests: Rate limit exceeded"
    );
}

// `UreqTransport::get` has a second failure path the stalled-server test does
// not reach: a response whose headers arrive but whose body cannot be read
// whole. A server that declares a `Content-Length` and then closes the
// connection before sending that many bytes makes `read_to_string` fail; this
// pins that the read error is reported as a body-read failure rather than
// surfacing as a truncated body or a panic.
#[test]
fn a_truncated_response_body_reports_a_read_error() {
    // Declare 100 bytes but send only five, then close the connection.
    let addr = serve_one_response(
        "HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\nshort",
    );
    let transport = UreqTransport::with_timeout(std::time::Duration::from_secs(5));
    let url = format!("http://{addr}/truncated");

    let error = transport
        .get(&url, &rest_session())
        .unwrap_err()
        .to_string();

    assert!(
        error.starts_with("reading the response body failed:"),
        "{error}"
    );
}

// A REST reply is server-controlled data, and the REST agent deliberately
// honors the environment proxy, so a hostile or compromised endpoint or proxy
// can answer a query. `ureq`'s reader limit sits under its gzip decoder, so a
// small `Content-Encoding: gzip` body can expand past the wire cap. This body
// is a fixed-Huffman stream that decodes to just over `MAX_RESPONSE_BYTES`; a
// wire-only cap reads the small compressed body whole and yields a decoded
// body over the cap, while the decoded-byte take rejects it before it becomes
// a `String`.
#[test]
fn a_compressed_response_larger_than_the_cap_is_rejected() {
    let cap = usize::try_from(MAX_RESPONSE_BYTES).expect("the cap fits a usize");
    let body = gzip_zeros(cap / 258 + 1);
    assert!(
        body.len() < cap,
        "the compressed body must stay under the wire cap, got {}",
        body.len()
    );
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(&body);
    let address = serve_one_response_bytes(&response);
    let transport = UreqTransport::with_timeout(Duration::from_secs(10));
    let url = format!("http://{address}/me/library/artists");

    let error = transport
        .get(&url, &rest_session())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("larger than the 8 MiB cap"),
        "unexpected error: {error}"
    );
}

// The decoded-byte cap alone would leave the transport willing to read an
// arbitrarily large compressed body that decodes to little: an empty stored
// block costs five wire bytes and produces no output. This body puts more than
// the cap on the wire while decoding to zero bytes, so only `ureq`'s wire-byte
// limit can reject it; without that limit the read succeeds and the transport
// returns an empty body for a truncated download.
#[test]
fn a_response_with_more_wire_bytes_than_the_cap_is_rejected() {
    let cap = usize::try_from(MAX_RESPONSE_BYTES).expect("the cap fits a usize");
    let body = gzip_empty_stored_blocks(cap / 5 + 1);
    assert!(
        body.len() > cap,
        "the compressed body must exceed the wire cap, got {}",
        body.len()
    );
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(&body);
    let address = serve_one_response_bytes(&response);
    let transport = UreqTransport::with_timeout(Duration::from_secs(10));
    let url = format!("http://{address}/me/library/artists");

    let error = transport
        .get(&url, &rest_session())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("reading the response body failed"),
        "unexpected error: {error}"
    );
}

// A body of exactly the cap is legal: the reader limit sits one byte above the
// cap precisely so the read past an exactly-at-cap body reaches EOF instead of
// `ureq`'s limit error. A limit of `MAX_RESPONSE_BYTES` would reject this body
// even though nothing exceeds the cap.
#[test]
fn a_response_of_exactly_the_cap_is_accepted() {
    let cap = usize::try_from(MAX_RESPONSE_BYTES).expect("the cap fits a usize");
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {cap}\r\nConnection: close\r\n\r\n"
    )
    .into_bytes();
    response.resize(response.len() + cap, b' ');
    let address = serve_one_response_bytes(&response);
    let transport = UreqTransport::with_timeout(Duration::from_secs(10));
    let url = format!("http://{address}/me/library/artists");

    let body = transport
        .get(&url, &rest_session())
        .expect("a body of exactly the cap must be accepted");
    assert_eq!(body.len(), cap);
}

// The old `Body::read_to_string()` configured ureq's lossy UTF-8 replacement
// for `text/*` responses; the transport reads through `.reader()` instead, so
// it must keep that setting. Without `lossy_utf8(true)`, this lone invalid
// byte fails `String::from_utf8` and turns the reply into a read error.
#[test]
fn a_text_response_with_invalid_utf8_keeps_the_lossy_replacement() {
    let mut response =
        b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 1\r\nConnection: close\r\n\r\n"
            .to_vec();
    response.push(0xff);
    let address = serve_one_response_bytes(&response);
    let transport = UreqTransport::with_timeout(Duration::from_secs(10));
    let url = format!("http://{address}/me/library/artists");

    let body = transport
        .get(&url, &rest_session())
        .expect("a text body with invalid UTF-8 must keep the lossy replacement");
    assert_eq!(body, "?");
}

// The transport's success arm. Every other `UreqTransport::get` test pins a
// failure — a non-2xx status, a body that cannot be read whole, a stalled
// server, or a redirect — and the truncated-body test returns 200 but fails
// while reading the body, so it never reaches the status check. This pins that
// a 2xx whose body reads whole is returned verbatim rather than being treated
// as an error: the one path the browse methods depend on.
#[test]
fn a_successful_response_returns_its_body() {
    let body = r#"{"data":[]}"#;
    let addr = serve_one_response(&http_response("200 OK", "application/json", body));

    let url = format!("http://{addr}/me/library/artists");
    let got = UreqTransport::new().get(&url, &rest_session()).unwrap();

    assert_eq!(got, body);
}

// The transport authenticates every browse request with two headers: the
// developer token as `Authorization: Bearer <token>` and the MusicKit user
// token as `Music-User-Token: <token>`. Apple Music rejects a request without
// them, but the response-driven tests above assert only the reply, so a
// refactor that dropped either `.header(..)` call would leave every browse
// query unauthenticated with the suite still green. Capture the request the
// real transport sends and pin both credentials (and the URL path).
#[test]
fn a_request_carries_the_developer_and_user_tokens() {
    let body = r#"{"data":[]}"#;
    let (address, receiver) =
        serve_one_response_capturing_request(&http_response("200 OK", "application/json", body));

    let url = format!("http://{address}/me/library/artists");
    UreqTransport::new()
        .get(&url, &rest_session())
        .expect("a 200 with an empty collection succeeds");

    let request = receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the server captured the request")
        .request;
    // Header names are case-insensitive, so match on a lowercased copy while
    // reporting the captured request verbatim in any failure.
    let lower = request.to_ascii_lowercase();

    assert!(
        lower.starts_with("get /me/library/artists http/1.1\r\n"),
        "{request:?}"
    );
    assert!(
        lower.contains("authorization: bearer developer-token\r\n"),
        "{request:?}"
    );
    assert!(
        lower.contains("music-user-token: user-token\r\n"),
        "{request:?}"
    );
}
