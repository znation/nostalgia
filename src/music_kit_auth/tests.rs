use super::*;
use std::io::{Read, Write};
use std::net::TcpStream;
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

    let session = authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
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
