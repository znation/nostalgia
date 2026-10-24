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

mod authorize;
mod bootstrap;
mod http_request;
mod opener;
mod server;
mod tokens;
