use super::bootstrap::*;
use super::http::write_response;
use super::opener::*;
use super::server::*;
use super::*;
use std::env;
use std::io;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

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

/// The recording-free variant of [`background_capturing`]: wraps `flow` as an
/// opener that runs it on a background thread and returns the handles a test
/// joins once `authorize` has returned. The calls [`background_capturing`]
/// records are dropped.
fn background<F>(flow: F) -> (impl Fn(&str) -> io::Result<()>, OpenerHandles)
where
    F: FnOnce(u16, String) + Send + 'static,
{
    let (opener, handles, _calls) = background_capturing(flow);
    (opener, handles)
}

/// Wraps `flow` as an opener that runs it on a background thread, so the
/// blocking `authorize` call can accept the connection, and records each
/// `(bootstrap path, sign-in URL)` pair the opener received so a test can
/// inspect what reached the opener. The returned handles let a test join the
/// thread (and surface any panic in it) once `authorize` has returned.
///
/// The opener receives the path of the private bootstrap page, not the
/// sign-in URL, so it reads the page to recover the port and nonce the way a
/// browser would follow the redirect. [`background`] is the recording-free
/// variant and delegates here, so the opener contract lives once.
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
/// `/?state=` page query, a wrong-state `POST /token`, an unknown path, a
/// foreign `Host`, a page request without a `Host`, and the loopback
/// port-bearing `Host` — share this scaffold, so each keeps only its own
/// assertion on the returned response. The probe receives the loopback port
/// so a request can build the port-bearing `Host` the browser sends.
fn response_to_probe<F>(probe: F) -> String
where
    F: Fn(u16, &str) -> String + Send + 'static,
{
    let observed = Arc::new(Mutex::new(String::new()));
    let observed_for_flow = Arc::clone(&observed);
    authorize_with_flow(move |port, state| {
        *observed_for_flow.lock().expect("observed lock") = request(port, &probe(port, &state));
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

// Dropping a bootstrap page whose directory holds a leftover file (for
// example one left behind by an interrupted atomic write) must still remove
// the whole directory, not fail on `remove_dir` because it is non-empty.
#[test]
fn bootstrap_page_drop_removes_directory_with_leftover_files() {
    let url = "http://127.0.0.1:4321/?state=deadbeefdeadbeef";
    let page = BootstrapPage::write(url).expect("write the bootstrap page");
    let path = page.path_string();
    let directory = std::path::Path::new(&path)
        .parent()
        .expect("the bootstrap page has a directory")
        .to_path_buf();
    // Simulate a leftover temp file from an interrupted write.
    std::fs::write(directory.join(".sign-in-leftover.tmp"), b"partial").expect("plant leftover");

    drop(page);
    assert!(
        !directory.exists(),
        "dropping the bootstrap page removes a directory containing leftover files, got: {}",
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

// A directory left behind by a process that died mid-sign-in has no guard to
// remove it, so a later sign-in sweeps it once it is older than twice the flow
// deadline. Only the aged bootstrap directories go: a fresh one may belong to
// a flow still running in another process, and an entry with another name or a
// non-directory entry is not ours to remove.
#[cfg(unix)]
#[test]
fn the_bootstrap_sweep_removes_only_old_bootstrap_directories() {
    let root = std::env::temp_dir().join(format!("nostalgia-sweep-{}", random_nonce()));
    std::fs::create_dir(&root).expect("create the sweep scratch root");
    let old = root.join(format!("{BOOTSTRAP_DIR_PREFIX}old"));
    let fresh = root.join(format!("{BOOTSTRAP_DIR_PREFIX}fresh"));
    let unrelated = root.join("other");
    let file = root.join(format!("{BOOTSTRAP_DIR_PREFIX}file"));
    std::fs::create_dir(&old).expect("create the old bootstrap directory");
    std::fs::create_dir(&fresh).expect("create the fresh bootstrap directory");
    std::fs::create_dir(&unrelated).expect("create an unrelated directory");
    std::fs::write(&file, b"not a directory").expect("create an unrelated file");
    std::fs::write(old.join("sign-in.html"), b"stale").expect("plant a stale page");

    let now = std::time::SystemTime::now();
    std::fs::File::open(&old)
        .expect("open the old directory")
        .set_modified(now - Duration::from_secs(3600))
        .expect("age the old directory");

    sweep_stale_bootstrap_dirs(&root, now, BOOTSTRAP_STALE_AFTER);

    assert!(
        !old.exists(),
        "an aged bootstrap directory must be swept, got: {}",
        old.display()
    );
    assert!(
        fresh.exists(),
        "a fresh bootstrap directory must be left alone"
    );
    assert!(
        unrelated.exists(),
        "an unrelated directory must be left alone"
    );
    assert!(file.exists(), "a non-directory entry must be left alone");

    std::fs::remove_dir_all(&root).expect("remove the sweep scratch root");
}

// A symlink carrying the bootstrap prefix must be skipped by its own type, not
// followed to its target's metadata: an aged target would otherwise make the
// sweep treat the link as a stale bootstrap directory and remove it.
#[cfg(unix)]
#[test]
fn the_bootstrap_sweep_skips_a_symlink_instead_of_following_it() {
    let root = std::env::temp_dir().join(format!("nostalgia-sweep-link-{}", random_nonce()));
    std::fs::create_dir(&root).expect("create the sweep scratch root");
    let target = root.join("victim");
    std::fs::create_dir(&target).expect("create the symlink target");
    std::fs::write(target.join("keep.txt"), b"keep").expect("plant the target's contents");
    let link = root.join(format!("{BOOTSTRAP_DIR_PREFIX}link"));
    std::os::unix::fs::symlink(&target, &link).expect("plant a symlink");

    let now = std::time::SystemTime::now();
    std::fs::File::open(&target)
        .expect("open the link target")
        .set_modified(now - Duration::from_secs(3600))
        .expect("age the link target");

    sweep_stale_bootstrap_dirs(&root, now, BOOTSTRAP_STALE_AFTER);

    assert!(
        std::fs::symlink_metadata(&link).is_ok(),
        "a symlink must be skipped, not removed: {}",
        link.display()
    );
    assert!(
        target.join("keep.txt").exists(),
        "the sweep must not follow the symlink into its target: {}",
        target.display()
    );

    std::fs::remove_dir_all(&root).expect("remove the sweep scratch root");
}

// The guard cannot run when the process dies, so the sweep must actually be
// wired into the next bootstrap write rather than only existing as a helper.
#[cfg(unix)]
#[test]
fn a_new_bootstrap_write_sweeps_a_stale_directory_from_a_crashed_flow() {
    let stale = std::env::temp_dir().join(format!("{BOOTSTRAP_DIR_PREFIX}{}", random_nonce()));
    std::fs::create_dir(&stale).expect("create the stale bootstrap directory");
    std::fs::write(stale.join("sign-in.html"), b"stale").expect("plant a stale page");
    std::fs::File::open(&stale)
        .expect("open the stale directory")
        .set_modified(std::time::SystemTime::now() - Duration::from_secs(3600))
        .expect("age the stale directory");

    let page = BootstrapPage::write("http://127.0.0.1:1/?state=deadbeefdeadbeef")
        .expect("write a new bootstrap page");
    assert!(
        !stale.exists(),
        "a new sign-in must sweep the crashed flow's directory, got: {}",
        stale.display()
    );
    drop(page);
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
// loopback address. The flow tests below cover a missing `Host` and a
// port-bearing loopback `Host`, but never a foreign host carrying a port, so
// this pins that branch directly.
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

// Both nonce checks use `nonce_matches`, whose constant-time comparison is
// what keeps a guess's reply time from depending on how many leading bytes it
// got right. A prefix of the nonce, a superstring, an empty value, and a value
// with one byte changed must all be rejected; only the exact nonce matches.
// The timing property itself is not observable from a unit test, so this pins
// the equivalence the constant-time fold must preserve.
#[test]
fn nonce_matches_accepts_only_the_exact_nonce() {
    let nonce = "0123456789abcdef";

    assert!(nonce_matches(nonce, nonce));
    assert!(!nonce_matches("", nonce));
    assert!(!nonce_matches(&nonce[..nonce.len() - 1], nonce));
    assert!(!nonce_matches(&format!("{nonce}0"), nonce));
    assert!(!nonce_matches("0123456789abcdee", nonce));
    for index in 0..nonce.len() {
        let mut guess = nonce.as_bytes().to_vec();
        guess[index] ^= 1;
        let guess = String::from_utf8(guess).expect("flipping one ASCII byte keeps UTF-8");
        assert!(
            !nonce_matches(&guess, nonce),
            "a nonce differing at byte {index} must not match"
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
    let page = response_to_probe(|_port, state| {
        format!("GET /?state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
    });
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(page.contains(SAMPLE_DEVELOPER_TOKEN));
}

#[test]
fn authorize_marks_the_sign_in_page_no_store() {
    // The page carries the owner-side developer token in its HTML. A 200 GET
    // response without an explicit caching policy may be written to the
    // browser's disk cache, leaving that secret behind after the flow. Mark
    // every response no-store so the token-bearing page is never cached.
    let page = response_to_probe(|_port, state| {
        format!("GET /?state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
    });
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(
        page.contains("\r\nCache-Control: no-store\r\n"),
        "the token-bearing sign-in page must not be cacheable, got: {page}"
    );
}

#[test]
fn authorize_marks_the_sign_in_page_no_referrer() {
    // The page is served at a URL that carries the per-flow `state` nonce, and
    // the page loads Apple's MusicKit script from a third-party origin — so the
    // browser is about to send that third party a request whose `Referer` may
    // carry the nonce. Whether it does is the browser's policy to decide
    // unless the server decides it: a browser configured with `unsafe-url`, or
    // one old enough to default to `no-referrer-when-downgrade` (which sends
    // the full URL on an http -> https request), would hand the nonce over,
    // and a holder of the nonce can fetch the page and read the developer
    // token. The server must set the policy itself.
    let page = response_to_probe(|_port, state| {
        format!("GET /?state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
    });
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(
        page.contains("\r\nReferrer-Policy: no-referrer\r\n"),
        "the sign-in page must not let the browser's referrer policy leak the nonce, got: {page}"
    );
}

// A response larger than the socket send buffer, written to a client that stops
// reading, blocks `write_all` with no timeout — holding the sign-in flow past
// `AUTH_TIMEOUT`. `write_response` must arm a bounded write timeout first (a
// `None` timeout means "block forever"); the read path's deadline cap has its
// own tests below, and this pins the write side.
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
    let wrong = response_to_probe(|_port, _state| token_request("wrong-state", SAMPLE_USER_TOKEN));
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
    let missing = response_to_probe(|_port, _state| {
        "GET /nope HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n".to_string()
    });
    assert!(missing.starts_with("HTTP/1.1 404"));
}

#[test]
fn authorize_rejects_a_foreign_host_header() {
    // A DNS-rebinding page reaches the loopback socket under its own
    // hostname, so its `Host` header names that hostname, not 127.0.0.1.
    // The response must not carry the sign-in page (and its developer
    // token), and the real callback must still complete the flow.
    let rebound = response_to_probe(|_port, _state| {
        "GET / HTTP/1.1\r\nHost: evil.example\r\n\r\n".to_string()
    });
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
    let page = response_to_probe(|_port, state| format!("GET /?state={state} HTTP/1.1\r\n\r\n"));
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
    let page = response_to_probe(|port, state| {
        format!("GET /?state={state} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n")
    });
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

/// A connected loopback pair: the server side handed to the request reader
/// and the client side used to send raw bytes.
fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind a loopback listener");
    let port = listener.local_addr().expect("listener address").port();
    let client = TcpStream::connect(("127.0.0.1", port)).expect("connect to the listener");
    let (server, _) = listener.accept().expect("accept the client");
    (server, client)
}

// The request reader promises a hostile client cannot make the server
// allocate without bound. The flow test above covers a `Content-Length` that
// overflows `usize`; these cover the large-but-parseable declared body and
// the never-terminated header, which the `MAX_REQUEST_BYTES` guards reject.
#[test]
fn parse_http_request_refuses_a_declared_body_past_the_cap() {
    // The whole declared body is present: a server that refused only after
    // reading it would return the request instead of refusing it.
    let raw = format!(
        "POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n{}",
        http::MAX_REQUEST_BYTES + 1,
        "x".repeat(http::MAX_REQUEST_BYTES + 1)
    );

    let error = http::parse_http_request(raw.as_bytes())
        .err()
        .expect("a body past the cap must be refused");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(
        error.to_string(),
        "the declared body is larger than the cap"
    );
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
fn parse_http_request_accepts_a_request_filling_the_cap_exactly() {
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
    let request = http::parse_http_request(raw.as_bytes())
        .expect("an exact-cap request is not an I/O error")
        .expect("a request filling the cap exactly is accepted");
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/token");
    assert_eq!(
        request.body,
        vec![b'x'; body_len],
        "the exact-cap body must be read whole"
    );
}

#[test]
fn parse_http_request_refuses_headers_past_the_cap() {
    // A header line longer than the cap, with no `\r\n\r\n` terminator: the
    // server must give up as soon as it has buffered the cap instead of
    // waiting out the deadline.
    let raw = format!(
        "GET / HTTP/1.1\r\nX-Fill: {}\r\n",
        "x".repeat(http::MAX_REQUEST_BYTES)
    );

    let error = http::parse_http_request(raw.as_bytes())
        .err()
        .expect("headers past the cap must be refused");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(
        error.to_string(),
        "the request headers are larger than the cap"
    );
}

// A parse failure is an `io::ErrorKind::InvalidData` error, which the sign-in
// server treats like a closed connection: the malformed request is ignored and
// the flow keeps waiting. Each test names the branch that produced it, so a
// parse change that silently accepts a hostile line is caught.

/// Asserts that `raw` is refused as a malformed request: an
/// `io::ErrorKind::InvalidData` error whose message is `expected`. The three
/// malformed-request tests below differ only in the bytes and the message, so
/// the assert scaffold lives here once and each test names the branch it
/// pins.
fn assert_request_parse_error(raw: &[u8], expected: &str) {
    let error = http::parse_http_request(raw)
        .err()
        .expect("the malformed request is a parse error");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(error.to_string(), expected);
}

#[test]
fn parse_http_request_rejects_non_utf8_headers() {
    assert_request_parse_error(
        b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Bad: \xff\xfe\r\n\r\n",
        "request headers are not UTF-8",
    );
}

#[test]
fn parse_http_request_rejects_a_request_line_without_a_method() {
    assert_request_parse_error(b"\r\n\r\n", "missing method");
}

#[test]
fn parse_http_request_rejects_a_request_line_without_a_target() {
    assert_request_parse_error(b"GET\r\n\r\n", "missing target");
}

// A `Content-Length` that does not parse as a number is treated as no body
// (the `parse().unwrap_or(0)` branch), so the request is still returned with
// an empty body rather than aborting the flow or reading an unbounded body.
// Pin that: propagating the parse error instead would reject the request, and
// reading to the deadline would hang it, so either regression fails here.
#[test]
fn parse_http_request_treats_a_malformed_content_length_as_no_body() {
    let raw =
        b"POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: not-a-number\r\n\r\nbody";

    let request = http::parse_http_request(raw)
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

// A request that has not arrived whole is *incomplete*, not refused: the
// reader reports it as such so the sign-in server can keep the connection and
// read the rest of it later. This is what lets one stalled client sit in the
// server's pending list without holding the browser's callback behind it, so
// the branch is pinned here: an error or a served request here would drop the
// connection instead of waiting for the rest of it.
#[test]
fn the_reader_reports_a_partial_request_as_incomplete() {
    let (server, mut client) = connected_pair();
    let mut reader = http::RequestReader::new(server).expect("wrap the server side");

    client
        .write_all(b"POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\n")
        .expect("write a partial request");

    assert!(
        reader
            .poll()
            .expect("a partial request is not an I/O error")
            .is_none(),
        "a request that has not arrived whole must be reported as incomplete"
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
fn the_reader_drops_a_body_that_closes_before_its_declared_length() {
    let (server, mut client) = connected_pair();
    // Declare 100 body bytes, send five, then close the write half so the
    // reader sees EOF while it still wants 95 more.
    client
        .write_all(b"POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 100\r\n\r\nshort")
        .expect("write a truncated body");
    client
        .shutdown(std::net::Shutdown::Write)
        .expect("close the write half");
    let mut reader = http::RequestReader::new(server).expect("wrap the server side");

    let error = reader
        .poll()
        .err()
        .expect("a body that closes before its declared length must be refused");
    assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
    assert_eq!(error.to_string(), "the peer closed the connection");
}

// A declared body larger than one 1024-byte read chunk arrives over several
// socket reads, so the reader's buffer runs more than once. The small
// callback body usually arrives in the same read as the headers, so it never
// enters this loop at all; a real MusicKit user token is a multi-kilobyte
// JWT, so the server must reassemble the body byte-for-byte instead of
// truncating or misaligning it at a chunk boundary.
#[test]
fn the_reader_reads_a_body_across_multiple_reads() {
    let (server, mut client) = connected_pair();
    // 4000 bytes is more than one read can deliver even after the header read
    // already buffered part of the body, so the reader runs at least twice.
    let body: String = (0..4000)
        .map(|i| char::from(b'a' + (i % 26) as u8))
        .collect();
    let raw = format!(
        "POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let writer = std::thread::spawn(move || {
        let _ = client.write_all(raw.as_bytes());
        let _ = client.shutdown(std::net::Shutdown::Write);
    });
    let mut reader = http::RequestReader::new(server).expect("wrap the server side");

    // The reader never blocks, so it is polled until the whole request has
    // arrived.
    let request = loop {
        match reader
            .poll()
            .expect("a multi-chunk body is not an I/O error")
        {
            Some(request) => break request,
            None => thread::sleep(Duration::from_millis(1)),
        }
    };

    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/token");
    assert_eq!(
        request.body,
        body.as_bytes(),
        "the body must be reassembled byte-for-byte across reads"
    );
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

#[test]
fn authorize_serves_the_callback_after_a_stalled_connection_is_dropped() {
    // A local client can reach the loopback port without the nonce and open a
    // connection that never completes its request. The accepted connections
    // are read together, so that stalled connection does not hold the accept
    // loop for the whole flow: the per-connection budget drops it, and the
    // real browser callback on a later connection is then served. Before the
    // budget, the stalled connection held the loop past the flow deadline and
    // the callback was starved, so this test failed with a timeout.
    let (opener, handles) = background(|port, state| {
        let mut stalled = TcpStream::connect(("127.0.0.1", port)).expect("connect the stall");
        // A header that never completes: no `\r\n\r\n`.
        stalled
            .write_all(b"POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\n")
            .expect("write a partial request");
        // Let the server accept the stalled connection before the callback
        // arrives, so the test exercises the accept loop with a stalled
        // connection pending.
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

#[test]
fn a_stalled_connection_is_dropped_when_its_budget_is_spent() {
    // Each connection is bounded by the per-connection budget from the moment
    // it is accepted, so a client that opens a connection and never finishes
    // its request is dropped when that budget is spent — long before the
    // flow's own deadline. The elapsed time is what pins the budget: without
    // the deadline check the connection is held until the flow gives up, and
    // the stalled client then sees it close at the flow's deadline instead.
    const BUDGET: Duration = Duration::from_millis(100);
    const FLOW: Duration = Duration::from_millis(600);
    let (opener, handles) = background(|port, _state| {
        let mut stalled = TcpStream::connect(("127.0.0.1", port)).expect("connect the stall");
        stalled
            .write_all(b"POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\n")
            .expect("write a partial request");
        // The read ends when the server drops the connection, which it does
        // when the connection's budget is spent.
        let started = Instant::now();
        let mut sink = [0u8; 64];
        let read = stalled.read(&mut sink);
        let elapsed = started.elapsed();
        assert!(
            matches!(read, Ok(0) | Err(_)),
            "the stalled connection should be closed, read {read:?}"
        );
        assert!(
            elapsed >= BUDGET / 2,
            "the stalled connection was dropped after {elapsed:?}, before its {BUDGET:?} budget was spent"
        );
        assert!(
            elapsed < FLOW - Duration::from_millis(150),
            "the stalled connection was held for {elapsed:?}, past its {BUDGET:?} budget"
        );
    });

    let error = authorize_with_bounds(SAMPLE_DEVELOPER_TOKEN, &opener, FLOW, BUDGET)
        .expect_err("no callback arrives");
    join_all(&handles);

    assert!(error.to_string().contains("timed out"));
}

#[test]
fn authorize_serves_the_callback_after_repeated_stalled_connections() {
    // One stalled connection is bounded by the per-connection budget, but the
    // cost of each stall used to be *serialized* behind the previous one:
    // connections were served one at a time, so ten stalled connections cost
    // ten budgets and the browser's callback, queued behind them, was never
    // served. The pending connections are read together now, so the callback
    // is served as soon as it arrives however many stalls are held open.
    const STALLS: usize = 10;
    let (opener, handles) = background(|port, state| {
        let mut stalled = Vec::new();
        for _ in 0..STALLS {
            let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect the stall");
            // A header that never completes: no `\r\n\r\n`.
            stream
                .write_all(b"POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\n")
                .expect("write a partial request");
            stalled.push(stream);
        }
        // Let the server accept the stalled connections before the callback
        // arrives, so the callback is queued behind them.
        thread::sleep(Duration::from_millis(50));
        let response = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
        assert!(
            response.starts_with("HTTP/1.1 200"),
            "the callback should be served after the stalls, got: {response}"
        );
        drop(stalled);
    });

    let started = Instant::now();
    let session = authorize_with_bounds(
        SAMPLE_DEVELOPER_TOKEN,
        &opener,
        Duration::from_millis(600),
        Duration::from_millis(100),
    )
    .expect("repeated stalled connections do not starve the callback");
    let elapsed = started.elapsed();
    join_all(&handles);

    assert_eq!(session.user_token, SAMPLE_USER_TOKEN);
    // Ten stalls at a hundred milliseconds each are a second of serialized
    // waiting, past the six-hundred-millisecond flow deadline.
    assert!(
        elapsed < Duration::from_millis(500),
        "the flow waited {elapsed:?} for the callback behind the stalls"
    );
}

// The server reads a bounded number of connections at once, so a flood of
// connections cannot grow the server's share of the process's file descriptors
// without bound. The cap is behavior, not a comment: the pending list never
// grows past `MAX_PENDING_CONNECTIONS`, and a connection already accepted is
// never displaced by a newer one — the extra connections wait in the kernel's
// accept queue until a pending connection is served or its budget is spent.
#[test]
fn the_pending_list_is_capped_without_displacing_an_accepted_connection() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind the listener");
    let port = listener
        .local_addr()
        .expect("the listener has an address")
        .port();
    listener.set_nonblocking(true).expect("poll the listener");
    // One client more than the pending list can hold.
    let mut clients = Vec::new();
    for _ in 0..=MAX_PENDING_CONNECTIONS {
        clients.push(TcpStream::connect(("127.0.0.1", port)).expect("connect a client"));
    }
    // The client's ephemeral port tells the connections apart.
    let ports: Vec<u16> = clients
        .iter()
        .map(|client| {
            client
                .local_addr()
                .expect("the client has an address")
                .port()
        })
        .collect();

    let mut pending = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    // Take on every client that fits; each pass takes on at most
    // `MAX_ACCEPTS_PER_PASS`.
    while pending.len() < MAX_PENDING_CONNECTIONS {
        accept_pending(&listener, &mut pending, deadline, Duration::from_secs(5))
            .expect("accept the waiting connections");
    }
    // The list is full, so this pass takes on nothing and the extra client
    // stays in the kernel's accept queue.
    accept_pending(&listener, &mut pending, deadline, Duration::from_secs(5))
        .expect("a full pending list accepts no more connections");

    assert_eq!(
        pending.len(),
        MAX_PENDING_CONNECTIONS,
        "the pending list must not grow past its cap"
    );
    // The oldest connection was not displaced: the first pending connection
    // is still the first client.
    let oldest_kept = pending[0]
        .reader
        .stream()
        .peer_addr()
        .expect("the pending connection has a peer")
        .port();
    assert_eq!(
        oldest_kept, ports[0],
        "an accepted connection must not be displaced by a newer one"
    );

    // Free one slot the way serving that connection does, then the extra
    // client that was waiting in the kernel is taken on.
    pending.remove(0);
    accept_pending(&listener, &mut pending, deadline, Duration::from_secs(5))
        .expect("accept the queued connection once a slot is free");
    let pending_ports: Vec<u16> = pending
        .iter_mut()
        .map(|connection| {
            connection
                .reader
                .stream()
                .peer_addr()
                .expect("the pending connection has a peer")
                .port()
        })
        .collect();
    assert_eq!(pending.len(), MAX_PENDING_CONNECTIONS);
    assert!(
        pending_ports.contains(&ports[MAX_PENDING_CONNECTIONS]),
        "the extra connection must be accepted after a slot frees"
    );
}

// One pass of the accept loop takes on a bounded number of connections, so a
// flood cannot fill the pending list in one pass: the browser's callback is
// read on a later pass.
#[test]
fn one_pass_takes_on_a_bounded_number_of_connections() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind the listener");
    let port = listener
        .local_addr()
        .expect("the listener has an address")
        .port();
    listener.set_nonblocking(true).expect("poll the listener");
    let mut clients = Vec::new();
    for _ in 0..2 * MAX_ACCEPTS_PER_PASS {
        clients.push(TcpStream::connect(("127.0.0.1", port)).expect("connect a client"));
    }

    let mut pending = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    accept_pending(&listener, &mut pending, deadline, Duration::from_secs(5))
        .expect("accept the waiting connections");

    assert_eq!(
        pending.len(),
        MAX_ACCEPTS_PER_PASS,
        "one pass must take on at most {MAX_ACCEPTS_PER_PASS} connections"
    );
    accept_pending(&listener, &mut pending, deadline, Duration::from_secs(5))
        .expect("accept the waiting connections");
    assert_eq!(
        pending.len(),
        2 * MAX_ACCEPTS_PER_PASS,
        "the next pass must take on the connections the first one left"
    );
}

/// Waits up to five seconds for `pid` to leave `/proc`, which only happens
/// once the child has been reaped. Panics naming `context` when it remains.
#[cfg(target_os = "linux")]
fn assert_child_is_reaped(pid: u32, context: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::path::Path::new(&format!("/proc/{pid}")).exists() {
        assert!(
            Instant::now() < deadline,
            "child {pid} was still present five seconds after {context}"
        );
        thread::sleep(Duration::from_millis(10));
    }
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
    assert_child_is_reaped(pid, "reap_in_background");
}

/// Spawns a shell that exits with `exit_code`, reaps it through `reap_with`,
/// and returns the receiver of the one report it emits (or none, for a zero
/// exit). The two tests below share this scaffolding and differ only in the
/// status they inject and the report they expect.
#[cfg(unix)]
fn reap_exit_code(exit_code: i32) -> std::sync::mpsc::Receiver<String> {
    let child = Command::new("sh")
        .arg("-c")
        .arg(format!("exit {exit_code}"))
        .spawn()
        .expect("spawn a short-lived shell");
    let (sender, receiver) = std::sync::mpsc::channel();
    reap_with(child, move |report| {
        let _ = sender.send(report);
    });
    receiver
}

// The opener reaps on a detached thread, so its exit status is the only signal
// the flow has about whether the browser actually opened. A nonzero exit (a
// broken or missing browser) used to be discarded: the sign-in then sat for
// the full `AUTH_TIMEOUT` and reported only a generic timeout, hiding the
// cause. `reap_with`'s injectable reporter makes the status observable, and a
// real nonzero-exiting process is the fault injected.
#[cfg(unix)]
#[test]
fn a_nonzero_opener_exit_is_reported() {
    let report = reap_exit_code(3)
        .recv_timeout(Duration::from_secs(10))
        .expect("a nonzero opener exit must be reported");
    assert!(
        report.contains("opener"),
        "the report must name the opener, got: {report}"
    );
    assert!(
        report.contains('3'),
        "the report must name the exit status, got: {report}"
    );
}

// The twin of the test above: a successful opener exit must stay quiet, so the
// new report cannot fire on every sign-in and turn a working browser into a
// logged failure. The child exits zero and the reporter must never receive a
// line.
#[cfg(unix)]
#[test]
fn a_successful_opener_exit_is_not_reported() {
    assert!(
        reap_exit_code(0)
            .recv_timeout(Duration::from_millis(500))
            .is_err(),
        "a successful opener exit must not be reported"
    );
}

// An opener that never exits is the fault a bare `Child::wait` cannot bound:
// the reaper thread blocks forever and the child process outlives the sign-in.
// The injected fault is a real long-running child plus a short deadline; the
// report must name the timeout and the pid must be reaped (a killed child still
// needs a `wait`, and `/proc` only disappears once it has one).
#[cfg(target_os = "linux")]
#[test]
fn a_hung_opener_is_killed_and_reported() {
    let child = Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("spawn a long-running child");
    let pid = child.id();
    let (sender, receiver) = std::sync::mpsc::channel();
    reap_with_timeout(child, Duration::from_millis(200), move |report| {
        let _ = sender.send(report);
    });
    let report = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("a hung opener must be reported");
    assert!(
        report.contains("did not exit"),
        "the report must name the hang, got: {report}"
    );
    assert_child_is_reaped(pid, "the reap timeout");
}

// If the reaper thread cannot be started, the child it was meant to wait on
// would otherwise be dropped without a wait and stay a zombie, and the old
// plain `thread::spawn` would panic the sign-in thread instead. The injected
// fault is a failing spawner; the failure must be reported and the real child
// must be killed and reaped synchronously.
#[cfg(target_os = "linux")]
#[test]
fn a_reaper_thread_spawn_failure_kills_the_opener_and_is_reported() {
    let child = Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("spawn a long-running child");
    let pid = child.id();
    let (sender, receiver) = std::sync::mpsc::channel();
    reap_with_timeout_and_spawn(
        child,
        Duration::from_secs(30),
        move |report| {
            let _ = sender.send(report);
        },
        |_work| Err(io::Error::other("the OS refused a thread")),
    );
    let report = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("a failed reaper spawn must be reported");
    assert!(
        report.contains("reaper thread"),
        "the report must name the reaper thread, got: {report}"
    );
    assert!(
        report.contains("the OS refused a thread"),
        "the report must carry the spawn cause, got: {report}"
    );
    assert_child_is_reaped(pid, "the reaper spawn failed");
}

// A failed `try_wait` does not prove the child exited: the reaper used to log
// the wait error and return, leaving a still-running opener (and its pid)
// behind for the rest of the app's life. The injected fault is a poll that
// always errors against a real long-running child; the fix must kill and reap
// it before reporting, and the report must still carry the poll's cause.
#[cfg(target_os = "linux")]
#[test]
fn a_failed_reap_poll_kills_the_opener_and_reports_the_cause() {
    let mut child = Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("spawn a long-running child");
    let pid = child.id();
    let (sender, receiver) = std::sync::mpsc::channel();
    reap_child_with(
        &mut child,
        Duration::from_secs(30),
        move |report| {
            let _ = sender.send(report);
        },
        |_child| Err(io::Error::other("the wait failed")),
    );
    let report = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("a failed reap poll must be reported");
    assert!(
        report.contains("could not reap"),
        "the report must name the failed reap, got: {report}"
    );
    assert!(
        report.contains("the wait failed"),
        "the report must carry the poll's cause, got: {report}"
    );
    assert!(
        report.contains("killed and reaped it"),
        "the failed poll must still kill and reap the child, got: {report}"
    );
    assert_child_is_reaped(pid, "the failed reap poll");
}

// A kill that fails must not hide the reason behind an unbounded wait: the
// reaper used to discard the kill error and then call `wait`, so a child that
// could not be signaled kept the thread — and the report — blocked until the
// child happened to exit. The injected fault is a refused kill against a real
// child that outlives the assertion; the report must arrive while that child
// is still running, then the follow-up wait reaps it once it exits on its own.
#[cfg(target_os = "linux")]
#[test]
fn a_failed_opener_kill_is_reported_before_waiting_for_the_child() {
    let mut child = Command::new("sh")
        .args(["-c", "sleep 2"])
        .spawn()
        .expect("spawn a child that outlives the report");
    let (sender, receiver) = std::sync::mpsc::channel();
    let reaper = thread::spawn(move || {
        kill_and_reap(
            &mut child,
            |_child| Err(io::Error::other("the kill was refused")),
            "the browser opener did not exit within 50ms",
            move |report| {
                let _ = sender.send(report);
            },
        );
    });
    let report = receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("a failed kill must be reported without waiting for the child");
    assert!(
        report.contains("did not exit"),
        "the report must name the timeout, got: {report}"
    );
    assert!(
        report.contains("the kill was refused"),
        "the report must carry the kill failure, got: {report}"
    );
    reaper.join().expect("the reaper thread finished");
}

// The reaper runs on a named thread, so a panic in it is attributed to
// `nostalgia-opener-reaper` rather than `<unnamed>`. Pin the name on the
// production spawner, since the failure test above injects its own spawner
// and would not notice a rename. The name is spelled out here rather than
// read back from `REAPER_THREAD_NAME`, so a rename of the constant fails
// this test instead of renaming the thread and its assertion together.
#[test]
fn spawn_reaper_names_its_work_thread() {
    let (sender, receiver) = std::sync::mpsc::channel();
    spawn_reaper(Box::new(move || {
        let _ = sender.send(std::thread::current().name().map(str::to_string));
    }))
    .expect("start the reaper thread");

    assert_eq!(
        receiver
            .recv()
            .expect("the reaper reports its name")
            .as_deref(),
        Some("nostalgia-opener-reaper")
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
