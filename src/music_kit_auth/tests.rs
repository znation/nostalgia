use super::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
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

/// Drives one `authorize_with_timeout` run against `flow` as the browser
/// opener: starts `flow` in the background with [`background`], calls the real
/// `authorize_with_timeout` with the sample developer token and the standard
/// five-second deadline, joins the opener threads, and returns the result.
///
/// Tests that inject their own token or deadline call `authorize_with_timeout`
/// directly instead.
fn authorize_with_flow<F>(flow: F) -> Result<MusicKitSession, AppleMusicError>
where
    F: FnOnce(u16, String) + Send + 'static,
{
    let (opener, handles) = background(flow);
    let result = authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5));
    join_all(&handles);
    result
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
fn validate_developer_token_names_the_specific_defect() {
    // Every malformed token used to get the same "must be a three-segment
    // base64url JWT" message, which does not say which part is wrong. Each
    // defect now gets its own phrase, so a user with, say, a pasted payload
    // can tell the segment count is the problem rather than the alphabet.
    let cases = [
        ("a.b", "exactly three dot-separated segments"),
        ("a.b.c.d", "exactly three dot-separated segments"),
        ("a..c", "one of its segments is empty"),
        ("a.b.c\"", "outside the base64url alphabet"),
        ("a.b.\\c", "outside the base64url alphabet"),
    ];
    for (token, expected) in cases {
        let error = validate_developer_token(token).expect_err("a malformed token is rejected");
        assert!(
            error.to_string().contains(expected),
            "expected {token:?} to report {expected:?}, got {error}"
        );
    }
}

#[test]
fn validate_developer_token_does_not_echo_the_token() {
    // The developer token is a secret, so the defect phrase must never quote
    // the offending value into the message that `sign_in` logs to stderr.
    let secret = "topsecret.payload.sig!";
    let error = validate_developer_token(secret).expect_err("a malformed token is rejected");
    assert!(!error.to_string().contains("topsecret"), "got {error}");
}

#[test]
fn percent_decode_decodes_form_escapes_and_leaves_bad_ones_literal() {
    // The callback body is form-urlencoded by the browser's `fetch`, so a
    // `+` is a space and `%XX` is the decoded byte (upper- or lowercase hex).
    assert_eq!(percent_decode("a+b"), "a b");
    assert_eq!(percent_decode("a%20b"), "a b");
    assert_eq!(percent_decode("%2E"), ".");
    assert_eq!(percent_decode("%2e"), ".");
    // Multi-byte UTF-8 arrives as several escapes and decodes as one char.
    assert_eq!(percent_decode("%C3%A9"), "\u{e9}");
    // A non-hex, incomplete, or trailing escape stays literal rather than
    // being dropped or panicking.
    assert_eq!(percent_decode("%zz"), "%zz");
    assert_eq!(percent_decode("100%"), "100%");
    assert_eq!(percent_decode("%2"), "%2");
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
    let session = authorize_with_flow(move |port, state| {
        let page = request(port, "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        let response = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
        let mut observed = observed_for_flow.lock().expect("observed lock");
        observed.push(state);
        observed.push(page);
        observed.push(response);
    })
    .expect("authorize succeeds");

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
fn authorize_serves_the_page_for_the_browsers_query_request() {
    // The browser opens `/?state=<nonce>`, so the page route matches `/`
    // only because the reader strips the query string. A regression there
    // would 404 the page and the sign-in would never start.
    let observed = Arc::new(Mutex::new(String::new()));
    let observed_for_flow = Arc::clone(&observed);
    authorize_with_flow(move |port, state| {
        let page = request(
            port,
            &format!("GET /?state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
        );
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
        *observed_for_flow.lock().expect("observed lock") = page;
    })
    .expect("the real callback still succeeds");

    let page = observed.lock().expect("observed lock");
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(page.contains(SAMPLE_DEVELOPER_TOKEN));
}

#[test]
fn authorize_trims_surrounding_whitespace_from_the_developer_token() {
    // `APPLE_MUSIC_DEVELOPER_TOKEN` commonly arrives with a trailing newline
    // (read from a file) or a stray space; the strict validator would reject
    // it as "not base64url", which does not say why. The flow trims it, so
    // the padded token both succeeds and is stored trimmed.
    let (opener, handles) = background(|port, state| {
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
    });

    let padded = format!("  {SAMPLE_DEVELOPER_TOKEN}\n");
    let session = authorize_with_timeout(&padded, &opener, Duration::from_secs(5))
        .expect("a token with surrounding whitespace is accepted");
    join_all(&handles);

    assert_eq!(session.developer_token, SAMPLE_DEVELOPER_TOKEN);
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
    let error = authorize_with_flow(|port, _state| {
        let _ = request(port, &token_request("wrong-state", SAMPLE_USER_TOKEN));
    })
    .expect_err("a wrong state is rejected");
    assert!(error.to_string().contains("state"));
}

#[test]
fn authorize_rejects_a_callback_with_an_empty_user_token() {
    let error = authorize_with_flow(|port, state| {
        let _ = request(port, &token_request(&state, ""));
    })
    .expect_err("an empty user token is rejected");
    assert!(error.to_string().contains("user token"));
}

#[test]
fn authorize_ignores_an_unknown_request_before_the_callback() {
    let observed = Arc::new(Mutex::new(String::new()));
    let observed_for_flow = Arc::clone(&observed);
    authorize_with_flow(move |port, state| {
        let missing = request(port, "GET /nope HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
        *observed_for_flow.lock().expect("observed lock") = missing;
    })
    .expect("the real callback still succeeds");

    assert!(
        observed
            .lock()
            .expect("observed lock")
            .starts_with("HTTP/1.1 404")
    );
}

#[test]
fn authorize_rejects_a_foreign_host_header() {
    let observed = Arc::new(Mutex::new(String::new()));
    let observed_for_flow = Arc::clone(&observed);
    authorize_with_flow(move |port, state| {
        // A DNS-rebinding page reaches the loopback socket under its own
        // hostname, so its `Host` header names that hostname, not 127.0.0.1.
        // The response must not carry the sign-in page (and its developer
        // token), and the real callback must still complete the flow.
        let rebound = request(port, "GET / HTTP/1.1\r\nHost: evil.example\r\n\r\n");
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
        *observed_for_flow.lock().expect("observed lock") = rebound;
    })
    .expect("a foreign host does not abort the flow");

    let rebound = observed.lock().expect("observed lock");
    assert!(
        rebound.starts_with("HTTP/1.1 403"),
        "a foreign Host should be forbidden, got: {rebound}"
    );
    assert!(
        !rebound.contains(SAMPLE_DEVELOPER_TOKEN),
        "the sign-in page must not leak the developer token to a foreign Host"
    );
}

#[test]
fn authorize_survives_a_connection_that_closes_early() {
    authorize_with_flow(|port, state| {
        // A client that connects and closes without a complete request.
        drop(TcpStream::connect(("127.0.0.1", port)).expect("connect"));
        // Then the real callback.
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
    })
    .expect("an aborted connection does not abort the flow");
}

#[test]
fn authorize_survives_an_oversized_content_length() {
    authorize_with_flow(|port, state| {
        // `Content-Length: usize::MAX` overflows a naive `body_start + len`.
        let oversized = "POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 18446744073709551615\r\n\r\n";
        let _ = request(port, oversized);
        // Then the real callback.
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
    })
    .expect("an oversized Content-Length does not abort the flow");
}

/// A connected loopback pair: the server side handed to `read_http_request`
/// and the client side used to send raw bytes.
fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind a loopback listener");
    let port = listener.local_addr().expect("listener address").port();
    let client = TcpStream::connect(("127.0.0.1", port)).expect("connect to the listener");
    let (server, _) = listener.accept().expect("accept the client");
    (server, client)
}

// `read_http_request` promises a hostile client cannot make the server
// allocate without bound. The flow test above covers a `Content-Length` that
// overflows `usize`; these cover the large-but-parseable declared body and
// the never-terminated header, which the `MAX_REQUEST_BYTES` guards reject.
#[test]
fn read_http_request_refuses_a_declared_body_past_the_cap() {
    let (mut server, mut client) = connected_pair();
    // Send the full declared body: a server that refused only after reading it
    // would return the request instead of `None`.
    let raw = format!(
        "POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n{}",
        http::MAX_REQUEST_BYTES + 1,
        "x".repeat(http::MAX_REQUEST_BYTES + 1)
    );
    let writer = std::thread::spawn(move || {
        let _ = client.write_all(raw.as_bytes());
    });

    let request = read_http_request(&mut server, Instant::now() + Duration::from_secs(5))
        .expect("a large declared body is not an I/O error");
    assert!(request.is_none(), "a body past the cap must be refused");

    drop(server);
    let _ = writer.join();
}

#[test]
fn read_http_request_refuses_headers_past_the_cap() {
    let (mut server, mut client) = connected_pair();
    // A header line longer than the cap, with no `\r\n\r\n` terminator, and a
    // client that stays connected: the server must give up as soon as it has
    // buffered the cap instead of waiting out the deadline.
    let raw = format!(
        "GET / HTTP/1.1\r\nX-Fill: {}\r\n",
        "x".repeat(http::MAX_REQUEST_BYTES)
    );
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let writer = std::thread::spawn(move || {
        let _ = client.write_all(raw.as_bytes());
        let _ = release_rx.recv();
    });

    let started = Instant::now();
    let request = read_http_request(&mut server, Instant::now() + Duration::from_secs(5))
        .expect("oversized headers are not an I/O error");
    let elapsed = started.elapsed();
    assert!(request.is_none(), "headers past the cap must be refused");
    assert!(
        elapsed < Duration::from_secs(1),
        "the header cap must be refused without waiting out the deadline (took {elapsed:?})"
    );

    drop(server);
    let _ = release_tx.send(());
    let _ = writer.join();
}

#[test]
fn authorize_times_out_without_a_callback_and_names_the_bound() {
    let opener = |_url: &str| Ok(());

    let error = authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_millis(120))
        .expect_err("no callback times out");
    // The report names the bound that expired, like the fetch and play
    // timeout reports in `ui::loading`, so the message says how long the
    // flow waited, not just that it gave up.
    assert_eq!(
        error.to_string(),
        "timed out waiting for the Apple Music sign-in callback after 120ms"
    );
}

#[test]
fn authorize_deadline_bounds_a_stalled_connection() {
    // A client that connects and sends a partial request, then stalls,
    // must not outlive the flow's deadline: `AUTH_TIMEOUT` bounds the
    // whole flow, not just the accept loop. Before the read timeout was
    // capped by the deadline, this waited out `CONNECTION_READ_TIMEOUT`
    // (five seconds) despite the much shorter injected deadline.
    let (opener, handles) = background(|port, _state| {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        // A header that never completes: no `\r\n\r\n`, then hold.
        stream
            .write_all(b"POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\n")
            .expect("write a partial request");
        // Block until the server gives up and closes the connection.
        let mut sink = [0u8; 64];
        let _ = stream.read(&mut sink);
    });

    let start = Instant::now();
    let error = authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_millis(150))
        .expect_err("a stalled connection still times out");
    let elapsed = start.elapsed();
    join_all(&handles);

    assert!(error.to_string().contains("timed out"));
    assert!(
        elapsed < Duration::from_secs(1),
        "the flow waited {elapsed:?} for a stalled connection"
    );
}

// The browser opener is a short-lived launcher. Dropping its `Child` without
// waiting leaves a zombie for the rest of the app's life, one per sign-in;
// `reap_in_background` waits on a detached thread so the child is reaped
// without blocking the caller. A real child plus `/proc` is the only way to
// observe reaping: before the wait was added, the pid stayed a zombie and this
// loop timed out.
#[cfg(target_os = "linux")]
#[test]
fn reap_in_background_reaps_the_child() {
    let child = Command::new("sh")
        .args(["-c", "exit 0"])
        .spawn()
        .expect("spawn a short-lived child");
    let pid = child.id();
    reap_in_background(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::path::Path::new(&format!("/proc/{pid}")).exists() {
        assert!(
            Instant::now() < deadline,
            "child {pid} was still present five seconds after reap_in_background"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

// `AUTH_TIMEOUT` is production behavior — how long `authorize` waits for
// the browser callback — but `authorize` is its only reader and the
// timeout tests above inject their own deadline, so a changed constant
// would clear the suite. Pin the documented five minutes.
#[test]
fn auth_timeout_is_the_documented_five_minutes() {
    assert_eq!(AUTH_TIMEOUT, Duration::from_secs(300));
}

// A missing opener is the most likely spawn failure on a minimal Linux
// box, and a bare "No such file or directory" does not say which command
// is missing. `spawn_opener` names the program in the error, so the
// `authorize` message ("could not open the sign-in page: ...") says what
// to install. The probe program cannot exist, so this fails without
// starting a process on every platform.
#[test]
fn spawn_opener_names_the_missing_program_in_its_error() {
    let error = spawn_opener("nostalgia-no-such-opener", &[]).unwrap_err();
    assert!(
        error.to_string().contains("nostalgia-no-such-opener"),
        "the error should name the program, got {error:?}"
    );
}
