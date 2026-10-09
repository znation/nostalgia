use super::*;

use crate::test_support::StubTransport;

/// A session whose tokens are recognizable, so a test can assert the transport
/// saw exactly these credentials.
fn session() -> MusicKitSession {
    MusicKitSession {
        developer_token: "developer-token".to_string(),
        user_token: "user-token".to_string(),
    }
}

/// A [`RestLibrary`] over `stub`, plus the handle to inspect its calls.
fn library_over(stub: &StubTransport) -> RestLibrary {
    RestLibrary::new(Box::new(stub.clone()))
}

/// Asserts the stub saw exactly one request, to `expected_url` with the
/// expected session.
fn assert_single_call(stub: &StubTransport, expected_url: &str) {
    let calls = stub.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, expected_url);
    assert_eq!(calls[0].1, session());
}

#[test]
fn favorite_artists_map_library_json() {
    // The extra `genres` attribute is real-payload noise: serde must tolerate
    // it rather than fail the whole parse.
    let stub = StubTransport::returning(
        r#"{"data":[
            {"id":"artist-1","attributes":{"name":"The Sample Band","genres":["rock"]}},
            {"id":"artist-2","attributes":{"name":"Second Act"}}
        ]}"#,
    );
    let library = library_over(&stub);

    let artists = library.get_favorite_artists(&session()).unwrap();

    assert_eq!(
        artists,
        vec![
            Artist {
                id: "artist-1".to_string(),
                name: "The Sample Band".to_string(),
            },
            Artist {
                id: "artist-2".to_string(),
                name: "Second Act".to_string(),
            },
        ]
    );
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/artists");
}

#[test]
fn albums_by_artist_map_library_json_and_set_the_artist_id() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"album-1","attributes":{"name":"First Record"}}]}"#,
    );
    let library = library_over(&stub);

    let albums = library
        .get_albums_by_artist(&session(), "artist-9")
        .unwrap();

    assert_eq!(
        albums,
        vec![Album {
            id: "album-1".to_string(),
            title: "First Record".to_string(),
            artist_id: "artist-9".to_string(),
        }]
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/artists/artist-9/albums",
    );
}

#[test]
fn songs_from_album_map_library_json_and_set_the_album_id() {
    let stub =
        StubTransport::returning(r#"{"data":[{"id":"song-1","attributes":{"name":"Opening"}}]}"#);
    let library = library_over(&stub);

    let songs = library.get_songs_from_album(&session(), "album-9").unwrap();

    assert_eq!(
        songs,
        vec![Song {
            id: "song-1".to_string(),
            title: "Opening".to_string(),
            album_id: "album-9".to_string(),
        }]
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/albums/album-9/tracks",
    );
}

#[test]
fn albums_by_artist_percent_encodes_the_id() {
    let stub = StubTransport::returning(r#"{"data":[]}"#);
    let library = library_over(&stub);

    library
        .get_albums_by_artist(&session(), "a/b c?d#e")
        .unwrap();

    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/artists/a%2Fb%20c%3Fd%23e/albums",
    );
}

#[test]
fn songs_from_album_percent_encodes_the_id() {
    let stub = StubTransport::returning(r#"{"data":[]}"#);
    let library = library_over(&stub);

    library
        .get_songs_from_album(&session(), "caf\u{e9}")
        .unwrap();

    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/albums/caf%C3%A9/tracks",
    );
}

#[test]
fn encode_path_segment_leaves_unreserved_bytes_and_encodes_the_rest() {
    assert_eq!(encode_path_segment("i.abc-123_~"), "i.abc-123_~");
    assert_eq!(encode_path_segment("a b/c?d#e"), "a%20b%2Fc%3Fd%23e");
    assert_eq!(encode_path_segment("caf\u{e9}"), "caf%C3%A9");
}

#[test]
fn transport_error_propagates_the_bare_cause() {
    let stub = StubTransport::failing("network down");
    let library = library_over(&stub);

    let error = library
        .get_favorite_artists(&session())
        .unwrap_err()
        .to_string();

    // The cause is bare: the UI supplies the context naming the query, so
    // this layer does not repeat it.
    assert_eq!(error, "network down");
}

#[test]
fn albums_by_artist_transport_error_propagates_the_bare_cause() {
    let stub = StubTransport::failing("network down");
    let library = library_over(&stub);

    let error = library
        .get_albums_by_artist(&session(), "artist-9")
        .unwrap_err()
        .to_string();

    assert_eq!(error, "network down");
}

#[test]
fn songs_from_album_transport_error_propagates_the_bare_cause() {
    let stub = StubTransport::failing("network down");
    let library = library_over(&stub);

    let error = library
        .get_songs_from_album(&session(), "album-9")
        .unwrap_err()
        .to_string();

    assert_eq!(error, "network down");
}

#[test]
fn malformed_json_surfaces_the_parse_error() {
    let stub = StubTransport::returning("not json");
    let library = library_over(&stub);

    let error = library
        .get_albums_by_artist(&session(), "artist-9")
        .unwrap_err()
        .to_string();

    assert!(error.starts_with("response was not valid JSON:"), "{error}");
}

#[test]
fn nameless_artist_is_an_error_naming_its_id() {
    let stub = StubTransport::returning(r#"{"data":[{"id":"artist-1","attributes":{}}]}"#);
    let library = library_over(&stub);

    let error = library
        .get_favorite_artists(&session())
        .unwrap_err()
        .to_string();

    assert_eq!(error, "response carried artist \"artist-1\" without a name");
}

#[test]
fn blank_album_name_is_an_error() {
    let stub =
        StubTransport::returning(r#"{"data":[{"id":"album-1","attributes":{"name":"   "}}]}"#);
    let library = library_over(&stub);

    let error = library
        .get_albums_by_artist(&session(), "artist-9")
        .unwrap_err()
        .to_string();

    assert_eq!(error, "response carried album \"album-1\" without a name");
}

#[test]
fn nameless_song_is_an_error() {
    let stub = StubTransport::returning(r#"{"data":[{"id":"song-1"}]}"#);
    let library = library_over(&stub);

    let error = library
        .get_songs_from_album(&session(), "album-9")
        .unwrap_err()
        .to_string();

    assert_eq!(error, "response carried song \"song-1\" without a name");
}

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
        let _ = sender.send(transport.get(&url, &session()));
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

/// Binds a loopback listener, returning it with the address it bound.
fn loopback_listener() -> (std::net::TcpListener, std::net::SocketAddr) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    (listener, address)
}

/// Reads whatever one request has sent within a one-second bound and returns
/// it lossily as text, so a test can look for a header without a full parse.
fn read_some_request(stream: &mut std::net::TcpStream) -> String {
    use std::io::Read;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(1)))
        .unwrap();
    let mut buffer = [0u8; 4096];
    let read = stream.read(&mut buffer).unwrap_or(0);
    String::from_utf8_lossy(&buffer[..read]).into_owned()
}

/// Serves exactly one HTTP response on a fresh loopback listener: binds,
/// accepts a single connection, reads whatever request arrived, writes
/// `response` verbatim, and closes. Returns the bound address for
/// [`UreqTransport`], so a test can pin a response-dependent failure mode
/// without rebuilding the accept/read/write scaffolding.
fn serve_one_response(response: &str) -> std::net::SocketAddr {
    let (listener, address) = loopback_listener();
    let response = response.to_string();
    std::thread::spawn(move || {
        use std::io::Write;
        let (mut stream, _) = listener.accept().unwrap();
        let _ = read_some_request(&mut stream);
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });
    address
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
    let (redirector, redirector_addr) = loopback_listener();

    // The redirecting server answers the one request with a 302 pointing at
    // the "foreign" server, exactly what a hostile or compromised API reply
    // could return.
    let redirect_thread = std::thread::spawn(move || {
        let (mut stream, _) = redirector.accept().unwrap();
        let _ = read_some_request(&mut stream);
        use std::io::Write;
        let response = format!(
            "HTTP/1.1 302 Found\r\nLocation: http://{foreign_addr}/leak\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });

    // The foreign server records whether any redirected request arrived. A
    // one-second poll makes "no request" a bounded, observable result rather
    // than a hang.
    let foreign_thread = std::thread::spawn(move || {
        foreign.set_nonblocking(true).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        loop {
            match foreign.accept() {
                Ok((mut stream, _)) => return Some(read_some_request(&mut stream)),
                Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        return None;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(error) => panic!("foreign listener failed: {error}"),
            }
        }
    });

    let transport = UreqTransport::with_timeout(std::time::Duration::from_secs(2));
    let url = format!("http://{redirector_addr}/redirect");
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(transport.get(&url, &session()));
    });
    let _ = receiver.recv_timeout(std::time::Duration::from_secs(5));

    redirect_thread.join().unwrap();
    let leaked = foreign_thread.join().unwrap();
    assert!(
        leaked.is_none(),
        "the user token reached the redirect target: {leaked:?}"
    );
}

// The production transport is the one that carries real credentials, and it
// uses the process-wide `shared_agent` rather than the test-only
// `with_timeout` path, so pin the shared agent's redirect policy directly.
#[test]
fn the_shared_agent_does_not_follow_redirects() {
    assert_eq!(shared_agent().config().max_redirects(), 0);
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
    let addr = serve_one_response(&format!(
        "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ));

    let url = format!("http://{addr}/me/library/artists");
    let error = UreqTransport::new().get(&url, &session()).unwrap_err();
    assert_eq!(
        error.to_string(),
        "HTTP 401 Unauthorized: Invalid developer token"
    );
}

// The `None` arm of the same status check: a non-2xx response whose body is
// not the documented error envelope has no cause to add, so the message names
// only the status. Pinned so a regression cannot drop the status itself or
// invent a cause from an unrelated body.
#[test]
fn a_non_success_status_without_an_api_error_body_reports_just_the_status() {
    let body = "Not found";
    let addr = serve_one_response(&format!(
        "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ));

    let url = format!("http://{addr}/me/library/artists");
    let error = UreqTransport::new().get(&url, &session()).unwrap_err();
    assert_eq!(error.to_string(), "HTTP 404 Not Found");
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

    let error = transport.get(&url, &session()).unwrap_err().to_string();

    assert!(
        error.starts_with("reading the response body failed:"),
        "{error}"
    );
}
