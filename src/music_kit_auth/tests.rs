use super::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const SAMPLE_DEVELOPER_TOKEN: &str = "eyJhbGciOiJFUzI1NiJ9.eyJpc3MiOiJ0ZWFtIn0.c2ln";
const SAMPLE_USER_TOKEN: &str = "eyJhbGciOiJFUzI1NiJ9.eyJzdWIiOiJ1c2VyIn0.c2ln";

/// The background opener threads a test started, so it can join them.
type OpenerHandles = Arc<Mutex<Vec<thread::JoinHandle<()>>>>;

/// Each `(bootstrap path, sign-in URL)` pair a capturing opener received.
type OpenerCalls = Arc<Mutex<Vec<(String, String)>>>;

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

/// The sign-in URL embedded in a bootstrap page.
fn url_of_bootstrap(html: &str) -> String {
    let start = html.find("url=").expect("the bootstrap page carries a url") + 4;
    let rest = &html[start..];
    let end = rest.find('"').expect("the bootstrap url is quoted");
    rest[..end].to_string()
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
///
/// The opener receives the path of the private bootstrap page, not the
/// sign-in URL, so it reads the page to recover the port and nonce the way a
/// browser would follow the redirect.
fn background<F>(flow: F) -> (impl Fn(&str) -> io::Result<()>, OpenerHandles)
where
    F: FnOnce(u16, String) + Send + 'static,
{
    let slot = Arc::new(Mutex::new(Some(flow)));
    let handles = Arc::new(Mutex::new(Vec::new()));
    let handles_for_opener = Arc::clone(&handles);
    let opener = move |page_path: &str| {
        let html = std::fs::read_to_string(page_path).expect("the bootstrap page is readable");
        let url = url_of_bootstrap(&html);
        let port = port_of(&url);
        let state = state_of(&url);
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

/// Like [`background`], but also records each `(bootstrap path, sign-in URL)`
/// pair the opener received, so a test can inspect what reached the opener.
fn background_capturing<F>(flow: F) -> (impl Fn(&str) -> io::Result<()>, OpenerHandles, OpenerCalls)
where
    F: FnOnce(u16, String) + Send + 'static,
{
    let slot = Arc::new(Mutex::new(Some(flow)));
    let handles = Arc::new(Mutex::new(Vec::new()));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let handles_for_opener = Arc::clone(&handles);
    let calls_for_opener = Arc::clone(&calls);
    let opener = move |page_path: &str| {
        let html = std::fs::read_to_string(page_path).expect("the bootstrap page is readable");
        let url = url_of_bootstrap(&html);
        calls_for_opener
            .lock()
            .expect("calls lock")
            .push((page_path.to_string(), url.clone()));
        let port = port_of(&url);
        let state = state_of(&url);
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
    (opener, handles, calls)
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

/// Runs the flow with `probe` sent as the first request and returns the raw
/// HTTP response it received; the real callback then completes the flow.
///
/// The tests that inspect the response to an unusual request — the browser's
/// `/?state=` page query, a wrong-state `POST /token`, an unknown path, and a
/// foreign `Host` — share this scaffold, so each keeps only its own assertion
/// on the returned response.
fn response_to_probe<F>(probe: F) -> String
where
    F: Fn(&str) -> String + Send + 'static,
{
    let observed = Arc::new(Mutex::new(String::new()));
    let observed_for_flow = Arc::clone(&observed);
    authorize_with_flow(move |port, state| {
        *observed_for_flow.lock().expect("observed lock") = request(port, &probe(&state));
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
    })
    .expect("the real callback still succeeds");
    observed.lock().expect("observed lock").clone()
}

#[test]
fn bootstrap_page_is_private_and_carries_the_sign_in_url() {
    let url = "http://127.0.0.1:4321/?state=deadbeefdeadbeef";
    let page = BootstrapPage::write(url).expect("write the bootstrap page");
    let path = page.path_string();
    let directory = std::path::Path::new(&path)
        .parent()
        .expect("the bootstrap page has a directory")
        .to_path_buf();
    let html = std::fs::read_to_string(&path).expect("read the bootstrap page");
    assert!(
        html.contains(url),
        "the bootstrap page must redirect to the sign-in URL, got: {html}"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let file_mode = std::fs::metadata(&path)
            .expect("bootstrap metadata")
            .permissions()
            .mode();
        assert_eq!(
            file_mode & 0o077,
            0,
            "the bootstrap page must be owner-only, mode {file_mode:o}"
        );
        let dir_mode = std::fs::metadata(&directory)
            .expect("bootstrap directory metadata")
            .permissions()
            .mode();
        assert_eq!(
            dir_mode & 0o077,
            0,
            "the bootstrap directory must be owner-only, mode {dir_mode:o}"
        );
    }

    drop(page);
    assert!(
        !std::path::Path::new(&path).exists(),
        "dropping the bootstrap page removes it"
    );
    assert!(
        !directory.exists(),
        "dropping the bootstrap page removes its private directory, got: {}",
        directory.display()
    );
}

// A failed page write must not leak the private directory the guard created:
// the guard owns the directory from the moment `create_private_dir` succeeds,
// so a write error (a full disk, say) drops it and removes the directory
// rather than leaving a `nostalgia-auth-*` entry and a partial page behind.
// `write_private_file` cannot be made to fail deterministically in a test, so
// `write_with` injects the failure; the production `write` calls the same
// helper with the real writer.
#[test]
fn a_failed_bootstrap_write_removes_the_private_directory() {
    let written_to = Mutex::new(None);
    let result = BootstrapPage::write_with(
        "http://127.0.0.1:1/?state=deadbeefdeadbeef",
        &|file, _contents| {
            *written_to.lock().expect("record lock") = Some(file.to_path_buf());
            Err(io::Error::other("simulated write failure"))
        },
    );

    assert!(result.is_err(), "the write failure must propagate");
    let file = written_to
        .lock()
        .expect("record lock")
        .clone()
        .expect("the page writer was called");
    assert!(
        !file.exists(),
        "the partial bootstrap page must be removed, got: {}",
        file.display()
    );
    let directory = file.parent().expect("the page has a parent directory");
    assert!(
        !directory.exists(),
        "the private directory must be removed when the page write fails, got: {}",
        directory.display()
    );
}

// `render_auth_page` fills every template placeholder. The flow tests assert
// that the served page carries the developer token and the per-flow nonce, but
// nothing pins the build version the page hands MusicKit's `configure` as
// `app.build`: dropping the `{{VERSION}}` substitution, or adding a fourth
// placeholder without a matching `replace`, would serve a literal `{{...}}`
// into the browser's MusicKit config with the whole suite still green. Render
// the template directly and pin all three substitutions plus the absence of
// any surviving placeholder.
#[test]
fn render_auth_page_substitutes_every_template_placeholder() {
    let page = super::page::render_auth_page(SAMPLE_DEVELOPER_TOKEN, "deadbeefdeadbeef");

    assert!(
        page.contains(SAMPLE_DEVELOPER_TOKEN),
        "the page must embed the developer token"
    );
    assert!(
        page.contains("deadbeefdeadbeef"),
        "the page must embed the per-flow state nonce"
    );
    assert!(
        page.contains(&format!("build: \"{}\"", env!("CARGO_PKG_VERSION"))),
        "the page must hand MusicKit the crate's build version"
    );
    assert!(
        !page.contains("{{"),
        "no template placeholder may survive rendering, got: {page}"
    );
}

// The bootstrap guard's security claim is that it creates its directory and
// file with `create`/`create_new`, so an entry already at the path — a file an
// attacker pre-planted, or a symlink to a victim — is refused rather than
// adopted, followed, or overwritten. `BootstrapPage::write` names its
// directory with a fresh 64-bit nonce, so a collision cannot be forced through
// it; exercise the two guards directly, since they are what the claim rests on.
#[test]
fn bootstrap_guard_refuses_a_pre_planted_path() {
    let base = std::env::temp_dir().join(format!("nostalgia-guard-{}", random_nonce()));
    std::fs::create_dir(&base).expect("create the scratch directory");

    // `create_private_dir` must refuse a path that already exists — the
    // `AlreadyExists` signal `BootstrapPage::write` retries on — and must
    // leave the existing entry alone rather than adopt or replace it.
    let planted_dir = base.join("private");
    std::fs::write(&planted_dir, b"pre-planted").expect("plant a file at the directory path");
    let error = create_private_dir(&planted_dir).expect_err("an existing path is refused");
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(
        std::fs::read(&planted_dir).expect("read the planted file"),
        b"pre-planted",
        "the guard must not replace the entry already at the path"
    );

    // `write_private_file` opens with `create_new`, so a pre-planted symlink
    // is refused rather than followed: the link's target must stay untouched.
    #[cfg(unix)]
    {
        let victim = base.join("victim");
        std::fs::write(&victim, b"original").expect("write the symlink target");
        let planted_file = base.join("sign-in.html");
        std::os::unix::fs::symlink(&victim, &planted_file).expect("plant a symlink");
        let error = write_private_file(&planted_file, "attacker-controlled")
            .expect_err("an existing symlink is refused");
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(
            std::fs::read(&victim).expect("read the symlink target"),
            b"original",
            "the guard must not follow a pre-planted symlink"
        );
    }

    std::fs::remove_dir_all(&base).expect("remove the scratch directory");
}

#[test]
fn authorize_keeps_the_state_nonce_out_of_the_opener_argument() {
    // `/proc/<pid>/cmdline` (and `ps`) is world-readable on Unix, so the
    // sign-in URL — which carries the `state` nonce — must never be a
    // command-line argument. The flow writes the URL into an owner-only
    // bootstrap file and hands the opener only that file's path.
    let (opener, handles, calls) = background_capturing(|port, state| {
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
    });
    let session = authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
        .expect("the callback completes the flow");
    join_all(&handles);
    assert_eq!(session.user_token, SAMPLE_USER_TOKEN);

    let calls = calls.lock().expect("calls lock");
    let (path, url) = calls.first().expect("the opener was called");
    let nonce = state_of(url);
    assert!(
        !path.contains(&nonce),
        "the state nonce must not travel in the opener argument: {path}"
    );
    assert!(
        !path.contains("state=") && !path.starts_with("http"),
        "the opener must receive a local path, not the sign-in URL: {path}"
    );
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

// The `Host` header is the DNS-rebinding guard: the server is bound to
// `127.0.0.1`, so only that address — with or without the port the browser
// appends — may read the sign-in page, which embeds the developer token.
// A missing `Host` (a hand-written or HTTP/1.0 request) must be refused
// exactly like a foreign one, and the port must be stripped before the
// comparison so a foreign host carrying a port is not mistaken for the
// loopback address. The flow tests below only ever send a `Host`, so this
// pins the `None` and port-bearing-foreign branches they never reach.
#[test]
fn is_loopback_host_accepts_only_the_bound_loopback_address() {
    assert!(is_loopback_host(Some("127.0.0.1")));
    assert!(is_loopback_host(Some("127.0.0.1:51234")));
    assert!(!is_loopback_host(Some("evil.example")));
    assert!(!is_loopback_host(Some("evil.example:51234")));
    assert!(!is_loopback_host(None));
}

// The nonce is the only thing a local process must guess to fetch the page
// that embeds the developer token: the loopback `Host` check passes for any
// local client, so the page route refuses a request whose `state` is not this
// flow's nonce. `random_nonce` is therefore a security boundary, and no flow
// test pins it — they all read the nonce back out of the opener URL and
// compare it to itself, so a constant (or otherwise trivially guessable)
// nonce would clear the whole suite while exposing the token to any local
// process. Pin that each call is a fresh 16-hex-digit value.
#[test]
fn random_nonce_is_a_fresh_sixteen_hex_digit_value() {
    let first = random_nonce();
    let second = random_nonce();

    assert_eq!(
        first.len(),
        16,
        "the nonce should be 16 hex digits, got {first:?}"
    );
    assert!(
        first.chars().all(|character| character.is_ascii_hexdigit()),
        "the nonce should be hex, got {first:?}"
    );
    assert_ne!(
        first, second,
        "each flow's nonce must be fresh, not a constant"
    );
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
        let page = request(
            port,
            &format!("GET /?state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
        );
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
    // The browser opens `/?state=<nonce>`, so the page route must both match
    // `/` (the reader strips the query from the path) and carry the nonce. A
    // regression in either would refuse the page and the sign-in would never
    // start.
    let page = response_to_probe(|state| {
        format!("GET /?state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
    });
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(page.contains(SAMPLE_DEVELOPER_TOKEN));
}

// A response larger than the socket send buffer, written to a client that stops
// reading, blocks `write_all` with no timeout — holding the sign-in flow past
// `AUTH_TIMEOUT`. `write_response` must arm a bounded write timeout first (a
// `None` timeout means "block forever"); the read path's deadline cap has its
// own test above, and this pins the write side.
#[test]
fn write_response_arms_a_bounded_write_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a loopback listener");
    let address = listener.local_addr().expect("read the listener address");
    let _client = TcpStream::connect(address).expect("connect a client");
    let (mut server, _) = listener.accept().expect("accept the client");

    write_response(&mut server, 200, "OK", "text/plain; charset=utf-8", "hello")
        .expect("write the response");

    assert!(
        server
            .write_timeout()
            .expect("read the stream's write timeout")
            .is_some(),
        "write_response must arm a bounded write timeout, not leave it unbounded"
    );
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
fn authorize_reports_a_failed_opener_with_its_cause() {
    // A missing opener — `xdg-open` absent on a headless Linux box, say — is
    // a real startup failure, so the flow must surface it at once rather than
    // block until the callback deadline. The opener is called once with the
    // bootstrap path, and its own `io::Error` is named in the returned message.
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_opener = Arc::clone(&calls);
    let opener = move |_page_path: &str| {
        calls_for_opener.fetch_add(1, Ordering::SeqCst);
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "xdg-open: not found",
        ))
    };

    let error = authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
        .expect_err("a failed opener aborts the flow");

    assert!(
        error
            .to_string()
            .contains("could not open the sign-in page"),
        "the message must name the opener step, got: {error}"
    );
    assert!(
        error.to_string().contains("xdg-open: not found"),
        "the message must carry the opener's own error, got: {error}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn authorize_refuses_the_page_without_the_state_nonce() {
    let observed = Arc::new(Mutex::new(Vec::<String>::new()));
    let observed_for_flow = Arc::clone(&observed);
    let (opener, handles) = background(move |port, state| {
        // A local client that does not present the per-flow nonce — here with
        // a guess and with no query at all — must not be served the page,
        // which embeds the developer token. The real browser request carries
        // the nonce, so the flow still completes.
        let guessed = request(
            port,
            "GET /?state=not-the-nonce HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        );
        let absent = request(port, "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        let mut observed = observed_for_flow.lock().expect("observed lock");
        observed.push(guessed);
        observed.push(absent);
        drop(observed);
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
    });

    authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
        .expect("a page request without the nonce does not abort the flow");
    join_all(&handles);

    let observed = observed.lock().expect("observed lock");
    for response in observed.iter() {
        assert!(
            response.starts_with("HTTP/1.1 403"),
            "a page request without the state nonce should be forbidden, got: {response}"
        );
        assert!(
            !response.contains(SAMPLE_DEVELOPER_TOKEN),
            "the sign-in page must not leak the developer token without the nonce"
        );
    }
}

#[test]
fn authorize_ignores_a_callback_with_the_wrong_state() {
    // Any local client can reach `POST /token` without knowing the nonce, so a
    // wrong-state request must not abort the sign-in: it is answered 400 and
    // the real callback still completes on a later connection.
    let wrong = response_to_probe(|_state| token_request("wrong-state", SAMPLE_USER_TOKEN));
    assert!(
        wrong.starts_with("HTTP/1.1 400"),
        "a wrong state should be rejected with 400"
    );
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
fn authorize_rejects_a_malformed_user_token_naming_the_defect() {
    // A present-but-malformed token gets the same shape-defect wording the
    // developer-token validator uses, so a broken callback is diagnosable
    // rather than reported as a generic "invalid user token". The message
    // must name the defect without echoing the token, which is a secret.
    let error = authorize_with_flow(|port, state| {
        let _ = request(port, &token_request(&state, "aaa.bbb.cc!"));
    })
    .expect_err("a malformed user token is rejected");
    let message = error.to_string();
    assert!(
        message.contains("character outside the base64url alphabet"),
        "the message must name the defect, got: {message}"
    );
    assert!(
        !message.contains("aaa.bbb.cc!"),
        "the message must not echo the user token, got: {message}"
    );
}

#[test]
fn a_malformed_user_token_is_named_in_the_400_body_without_echoing_it() {
    // The 400 body is what the browser displays, so it must name the shape
    // defect the returned error names and must not echo the token, which is a
    // secret. A rejection is terminal, so `response_to_probe` — which expects
    // the flow to finish with a valid callback — cannot observe it; capture
    // the response the browser actually receives instead.
    let observed = Arc::new(Mutex::new(String::new()));
    let observed_for_flow = Arc::clone(&observed);
    let (opener, handles) = background(move |port, state| {
        *observed_for_flow.lock().expect("observed lock") =
            request(port, &token_request(&state, "aaa.bbb.cc!"));
    });
    authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
        .expect_err("a malformed user token is rejected");
    join_all(&handles);

    let response = observed.lock().expect("observed lock").clone();
    assert!(
        response.starts_with("HTTP/1.1 400"),
        "the callback should be answered 400, got: {response}"
    );
    assert!(
        response.contains("character outside the base64url alphabet"),
        "the body must name the defect, got: {response}"
    );
    assert!(
        !response.contains("aaa.bbb.cc!"),
        "the body must not echo the user token, got: {response}"
    );
}

#[test]
fn authorize_rejects_a_callback_without_a_user_token_field() {
    // A callback missing `userToken` altogether is rejected with a message
    // distinct from a malformed one, so the two causes are not conflated.
    let error = authorize_with_flow(|port, state| {
        let body = format!("state={state}");
        let raw = format!(
            "POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let _ = request(port, &raw);
    })
    .expect_err("a callback without a user token is rejected");
    assert!(
        error.to_string().contains("did not carry a user token"),
        "got: {error}"
    );
}

#[test]
fn authorize_ignores_an_unknown_request_before_the_callback() {
    let missing =
        response_to_probe(|_state| "GET /nope HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n".to_string());
    assert!(missing.starts_with("HTTP/1.1 404"));
}

#[test]
fn authorize_rejects_a_foreign_host_header() {
    // A DNS-rebinding page reaches the loopback socket under its own
    // hostname, so its `Host` header names that hostname, not 127.0.0.1.
    // The response must not carry the sign-in page (and its developer
    // token), and the real callback must still complete the flow.
    let rebound =
        response_to_probe(|_state| "GET / HTTP/1.1\r\nHost: evil.example\r\n\r\n".to_string());
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
fn authorize_refuses_a_page_request_without_a_host_header() {
    // A hand-written or HTTP/1.0 request can omit `Host` entirely. The probe
    // carries the correct nonce, so only the loopback guard stands between it
    // and the page that embeds the developer token: a guard that treated a
    // missing host as loopback would serve the page here.
    let observed = Arc::new(Mutex::new(String::new()));
    let observed_for_flow = Arc::clone(&observed);
    authorize_with_flow(move |port, state| {
        let page = request(port, &format!("GET /?state={state} HTTP/1.1\r\n\r\n"));
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
        *observed_for_flow.lock().expect("observed lock") = page;
    })
    .expect("the real callback still succeeds");

    let page = observed.lock().expect("observed lock");
    assert!(
        page.starts_with("HTTP/1.1 403"),
        "a page request without a Host should be forbidden, got: {page}"
    );
    assert!(
        !page.contains(SAMPLE_DEVELOPER_TOKEN),
        "the sign-in page must not leak the developer token without a Host"
    );
}

#[test]
fn authorize_serves_the_page_for_a_loopback_host_carrying_the_port() {
    // The browser opens `http://127.0.0.1:{port}/?state={state}`, so its `Host`
    // header is the port-bearing form `127.0.0.1:{port}` — not the bare address
    // the other tests send — and the page request also carries the per-flow
    // nonce. The host check must strip the port before comparing, or the
    // legitimate page request is refused as a foreign host and sign-in never
    // starts.
    let observed = Arc::new(Mutex::new(String::new()));
    let observed_for_flow = Arc::clone(&observed);
    authorize_with_flow(move |port, state| {
        let page = request(
            port,
            &format!("GET /?state={state} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"),
        );
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
        *observed_for_flow.lock().expect("observed lock") = page;
    })
    .expect("the real callback still succeeds");

    let page = observed.lock().expect("observed lock");
    assert!(page.starts_with("HTTP/1.1 200"), "got: {page}");
    assert!(page.contains(SAMPLE_DEVELOPER_TOKEN));
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

// The cap's accept side: a request whose headers and body together fill
// `MAX_REQUEST_BYTES` exactly is still served, not refused. The test above and
// the header one below pin only the reject side (`MAX + 1` body, over-cap
// headers), so the `<=` in the cap check (`end <= MAX_REQUEST_BYTES`) has its
// boundary unpinned: a regression to `<` would reject only an exact-fill
// request while every smaller one and every over-cap one still behaved as
// tested. Build the request so its total length is exactly the cap and assert
// the declared body comes back whole.
#[test]
fn read_http_request_accepts_a_request_filling_the_cap_exactly() {
    let (mut server, mut client) = connected_pair();

    // Solve for the body length that makes header + body exactly the cap. The
    // header embeds the length, and its digit count is the only thing that
    // changes as the length changes, so a few iterations converge.
    let header = |length: usize| {
        format!("POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {length}\r\n\r\n")
    };
    let mut body_len = http::MAX_REQUEST_BYTES;
    for _ in 0..4 {
        body_len = http::MAX_REQUEST_BYTES - header(body_len).len();
    }
    assert_eq!(
        header(body_len).len() + body_len,
        http::MAX_REQUEST_BYTES,
        "the fixture must fill the cap exactly"
    );

    let raw = format!("{}{}", header(body_len), "x".repeat(body_len));
    let writer = std::thread::spawn(move || {
        let _ = client.write_all(raw.as_bytes());
    });

    let request = read_http_request(&mut server, Instant::now() + Duration::from_secs(5))
        .expect("an exact-cap request is not an I/O error")
        .expect("a request filling the cap exactly is accepted");
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/token");
    assert_eq!(
        request.body,
        vec![b'x'; body_len],
        "the exact-cap body must be read whole"
    );

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

// A parse failure is `io::ErrorKind::InvalidData`, which `serve_connection`
// treats like a closed connection: the malformed request is ignored and the
// flow keeps waiting. Each test names the branch that produced it, so a parse
// change that silently accepts a hostile line is caught.

/// Writes `raw` to a fresh connection and asserts `read_http_request` rejects
/// it as a malformed request: an `io::ErrorKind::InvalidData` parse error whose
/// message is `expected`. The three malformed-request tests below differ only
/// in the bytes and the message, so the write-read-assert scaffold lives here
/// once and each test names the branch it pins.
fn assert_request_parse_error(raw: &[u8], expected: &str) {
    let (mut server, mut client) = connected_pair();
    client.write_all(raw).expect("write the malformed request");

    let error = read_http_request(&mut server, Instant::now() + Duration::from_secs(5))
        .err()
        .expect("the malformed request is a parse error");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(error.to_string(), expected);
}

#[test]
fn read_http_request_rejects_non_utf8_headers() {
    assert_request_parse_error(
        b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Bad: \xff\xfe\r\n\r\n",
        "request headers are not UTF-8",
    );
}

#[test]
fn read_http_request_rejects_a_request_line_without_a_method() {
    assert_request_parse_error(b"\r\n\r\n", "missing method");
}

#[test]
fn read_http_request_rejects_a_request_line_without_a_target() {
    assert_request_parse_error(b"GET\r\n\r\n", "missing target");
}

// A `Content-Length` that does not parse as a number is treated as no body
// (the `parse().unwrap_or(0)` branch), so the request is still returned with
// an empty body rather than aborting the flow or reading an unbounded body.
// Pin that: propagating the parse error instead would reject the request, and
// reading to the deadline would hang it, so either regression fails here.
#[test]
fn read_http_request_treats_a_malformed_content_length_as_no_body() {
    let (mut server, mut client) = connected_pair();
    client
        .write_all(
            b"POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: not-a-number\r\n\r\nbody",
        )
        .expect("write a request with a malformed Content-Length");

    let request = read_http_request(&mut server, Instant::now() + Duration::from_secs(5))
        .expect("a malformed Content-Length is not an I/O error")
        .expect("a malformed Content-Length is treated as no body, not refused");
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/token");
    assert!(
        request.body.is_empty(),
        "the malformed length must not read a body, got {:?}",
        request.body
    );
}

// A request whose headers declare a `Content-Length` but whose body closes
// before that many bytes arrive must be ignored, not returned with the bytes
// that did arrive. `authorize_survives_a_connection_that_closes_early` pins
// the same EOF on the header loop; this is the body loop's own read, which
// only a client that sends headers, sends part of the declared body, and then
// hangs up reaches. Returning the partial request would hand the callback
// parser a truncated body it would read as the fields the attacker did send.
#[test]
fn read_http_request_refuses_a_body_that_closes_before_its_declared_length() {
    let (mut server, mut client) = connected_pair();
    // Declare 100 body bytes, send five, then close the write half so the
    // server's body-loop read sees EOF while it still wants 95 more. Without
    // the shutdown the server would instead wait out the deadline, making the
    // test slow and the branch it pins less clear.
    client
        .write_all(b"POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 100\r\n\r\nshort")
        .expect("write a truncated body");
    client
        .shutdown(std::net::Shutdown::Write)
        .expect("close the write half");

    let request = read_http_request(&mut server, Instant::now() + Duration::from_secs(5))
        .expect("a truncated body is not an I/O error");

    assert!(
        request.is_none(),
        "a body that closes before its declared length must be refused, not returned partially"
    );
}

// A declared body larger than one 1024-byte read chunk arrives over several
// socket reads, so the body loop's `limit = room.min(chunk.len())` and its
// extend run more than once. The small callback body usually arrives in the
// same read as the headers, so it never enters this loop at all; a real
// MusicKit user token is a multi-kilobyte JWT, so the server must reassemble
// the body byte-for-byte instead of truncating or misaligning it at a chunk
// boundary.
#[test]
fn read_http_request_reads_a_body_across_multiple_reads() {
    let (mut server, mut client) = connected_pair();
    // 4000 bytes is more than one read can deliver even after the header read
    // already buffered part of the body, so the loop runs at least twice.
    let body: String = (0..4000)
        .map(|i| char::from(b'a' + (i % 26) as u8))
        .collect();
    let raw = format!(
        "POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    client.write_all(raw.as_bytes()).expect("write the request");
    client
        .shutdown(std::net::Shutdown::Write)
        .expect("close the write half so the body loop sees a complete body");

    let request = read_http_request(&mut server, Instant::now() + Duration::from_secs(5))
        .expect("a multi-chunk body is not an I/O error")
        .expect("a complete multi-chunk body is accepted");

    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/token");
    assert_eq!(
        request.body,
        body.as_bytes(),
        "the body must be reassembled byte-for-byte across reads"
    );
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

#[test]
fn authorize_serves_the_callback_after_a_stalled_connection_is_dropped() {
    // A local client can reach the loopback port without the nonce and open a
    // connection that never completes its request. Connections are served one
    // at a time, so that stalled connection must not hold the accept loop for
    // the whole flow: the per-connection budget drops it, and the real browser
    // callback on a later connection is then served. Before the budget, the
    // stalled connection held the loop past the flow deadline and the callback
    // was starved, so this test failed with a timeout.
    let (opener, handles) = background(|port, state| {
        let mut stalled = TcpStream::connect(("127.0.0.1", port)).expect("connect the stall");
        // A header that never completes: no `\r\n\r\n`.
        stalled
            .write_all(b"POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\n")
            .expect("write a partial request");
        // Let the server accept and block on the stalled connection before the
        // callback arrives, so the test exercises the sequential accept loop.
        thread::sleep(Duration::from_millis(50));
        let response = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
        assert!(
            response.starts_with("HTTP/1.1 200"),
            "the callback should be served once the stalled connection is dropped, got: {response}"
        );
        drop(stalled);
    });

    let session = authorize_with_bounds(
        SAMPLE_DEVELOPER_TOKEN,
        &opener,
        Duration::from_millis(500),
        Duration::from_millis(100),
    )
    .expect("a stalled connection does not starve the callback");
    join_all(&handles);

    assert_eq!(session.user_token, SAMPLE_USER_TOKEN);
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
