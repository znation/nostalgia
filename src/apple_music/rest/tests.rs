use super::*;

use crate::test_support::{StubTransport, sample_album, sample_artist, sample_song};

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
            sample_artist(),
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
            artist_id: "artist-9".to_string(),
            ..sample_album()
        }]
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/artists/artist-9/albums",
    );
}

#[test]
fn songs_from_album_map_library_json_and_set_the_album_id() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","durationInMillis":210000}}]}"#,
    );
    let library = library_over(&stub);

    let songs = library.get_songs_from_album(&session(), "album-9").unwrap();

    assert_eq!(
        songs,
        vec![Song {
            album_id: "album-9".to_string(),
            ..sample_song()
        }]
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/albums/album-9/tracks",
    );
}

// A track the API returns without `attributes.durationInMillis` (an older or
// partial resource) must map to the model's `0` "no duration supplied"
// sentinel rather than failing the whole response, so the Now Playing bar
// shows `--:--` for it instead of dropping the song.
#[test]
fn songs_from_album_maps_a_missing_duration_to_zero() {
    assert_eq!(
        song_from_attributes(r#"{"name":"Opening"}"#),
        Song {
            id: "song-1".to_string(),
            title: "Opening".to_string(),
            artist: String::new(),
            album_id: "album-9".to_string(),
            duration_ms: 0,
            preview_url: None,
        }
    );
}

// A track the API returns without `attributes.artistName` (or with a blank
// one) must map to the model's `""` "no artist supplied" sentinel rather than
// failing the whole response, so the Now Playing bar falls back to the title
// alone instead of dropping the song or rendering a blank artist.
#[test]
fn songs_from_album_maps_a_missing_or_blank_artist_to_empty() {
    for attributes in [
        r#"{"name":"Opening"}"#,
        r#"{"name":"Opening","artistName":""}"#,
        r#"{"name":"Opening","artistName":"   "}"#,
    ] {
        assert_eq!(song_from_attributes(attributes).artist, "");
    }
}

/// The [`Song`] `get_songs_from_album` maps from one resource carrying
/// `attributes` — the id (`song-1`) and album (`album-9`) are fixed, so a
/// caller varies only the attribute subset it probes. The duration, artist,
/// and preview tests each assert one mapped field of the same resource, so
/// this builder holds the JSON envelope and the fetch in one place.
fn song_from_attributes(attributes: &str) -> Song {
    let stub = StubTransport::returning(&format!(
        r#"{{"data":[{{"id":"song-1","attributes":{attributes}}}]}}"#
    ));
    let library = library_over(&stub);
    library
        .get_songs_from_album(&session(), "album-9")
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
}

// The audio-output decision (QUESTIONS.md `## Answered`, 2026-10-08) scopes
// playback to the Apple Music preview asset, so the browse client must carry
// the preview's URL onto `Song`. The cases below differ only in the resource's
// `attributes`, so they build it through `song_from_attributes` and read the
// mapped `preview_url` here.
fn preview_url_from(attributes: &str) -> Option<String> {
    song_from_attributes(attributes).preview_url
}

#[test]
fn songs_from_album_maps_the_first_preview_url() {
    assert_eq!(
        preview_url_from(
            r#"{"name":"Opening","previews":[{"url":"https://example.test/preview.m4a"}]}"#
        ),
        Some("https://example.test/preview.m4a".to_string())
    );
}

#[test]
fn songs_from_album_maps_no_preview_when_previews_is_absent() {
    assert_eq!(preview_url_from(r#"{"name":"Opening"}"#), None);
}

#[test]
fn songs_from_album_maps_no_preview_when_previews_is_empty() {
    assert_eq!(
        preview_url_from(r#"{"name":"Opening","previews":[]}"#),
        None
    );
}

#[test]
fn songs_from_album_maps_no_preview_when_every_url_is_blank() {
    // A preview with no `url` key and one with a whitespace-only url are both
    // "blank": neither can play, so the resource maps to `None`.
    assert_eq!(
        preview_url_from(r#"{"name":"Opening","previews":[{"url":""},{}]}"#),
        None
    );
}

#[test]
fn songs_from_album_skips_a_blank_preview_url_for_the_next() {
    assert_eq!(
        preview_url_from(
            r#"{"name":"Opening","previews":[{"url":"   "},{"url":"https://example.test/second.m4a"}]}"#
        ),
        Some("https://example.test/second.m4a".to_string())
    );
}

#[test]
fn songs_from_album_keeps_a_padded_preview_url_verbatim() {
    // `Resource::preview_url` compares each candidate trimmed, so a padded
    // URL counts as non-blank and is selected rather than skipped, but it is
    // returned exactly as the resource carried it. This is the reachable twin
    // of the whitespace-only skip above: whitespace-only is skipped, padded is
    // taken as-is (the padding is not normalized away).
    assert_eq!(
        preview_url_from(
            r#"{"name":"Opening","previews":[{"url":"  https://example.test/padded.m4a\t"}]}"#
        ),
        Some("  https://example.test/padded.m4a\t".to_string())
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

// A response can be valid JSON and still not be the `{ "data": [ ... ] }`
// envelope the client expects (an unexpected shape, or an `errors` envelope on
// a 2xx the status check never inspected). `serde_json` classifies that as a
// `Data` error, distinct from a syntax error; the report must say the envelope
// does not match rather than mislabeling valid JSON as invalid. Pin the
// distinct wording, and that it still carries `serde_json`'s own detail.
#[test]
fn valid_json_that_is_not_the_collection_envelope_names_the_envelope() {
    let stub = StubTransport::returning(r#"{"artists":[]}"#);
    let library = library_over(&stub);

    let error = library
        .get_albums_by_artist(&session(), "artist-9")
        .unwrap_err()
        .to_string();

    assert!(
        error.starts_with("response did not match the Apple Music collection envelope:"),
        "{error}"
    );
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

// A resource with a blank id and a valid name is the id twin of the
// `nameless_*` cases: an id is how the model links rows together and how the
// seam later resolves a browse, so a blank one can name nothing and must fail
// the response rather than become a row that errors when pressed. Each of the
// three maps calls `required_id`, so each query gets its own probe.
#[test]
fn blank_artist_id_is_an_error() {
    let stub = StubTransport::returning(r#"{"data":[{"id":"   ","attributes":{"name":"Ghost"}}]}"#);
    let library = library_over(&stub);

    let error = library
        .get_favorite_artists(&session())
        .unwrap_err()
        .to_string();

    assert_eq!(
        error,
        "response carried artist with a blank id (got \"   \")"
    );
}

#[test]
fn blank_album_id_is_an_error() {
    let stub = StubTransport::returning(r#"{"data":[{"id":"","attributes":{"name":"Ghost"}}]}"#);
    let library = library_over(&stub);

    let error = library
        .get_albums_by_artist(&session(), "artist-9")
        .unwrap_err()
        .to_string();

    assert_eq!(error, "response carried album with a blank id (got \"\")");
}

#[test]
fn blank_song_id_is_an_error() {
    let stub = StubTransport::returning(r#"{"data":[{"id":"","attributes":{"name":"Ghost"}}]}"#);
    let library = library_over(&stub);

    let error = library
        .get_songs_from_album(&session(), "album-9")
        .unwrap_err()
        .to_string();

    assert_eq!(error, "response carried song with a blank id (got \"\")");
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

/// Formats a complete HTTP/1.1 response around `body`: the `status_line`
/// (e.g. `"200 OK"`), the `Content-Type` header, and the exact
/// `Content-Length`, closed with `Connection: close`. The loopback tests
/// either hand the result to [`serve_one_response`] or write it from a
/// request-capturing thread, so the framing lives here rather than being
/// rebuilt at each site.
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
    let error = UreqTransport::new().get(&url, &session()).unwrap_err();
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
    let error = UreqTransport::new().get(&url, &session()).unwrap_err();
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
    let got = UreqTransport::new().get(&url, &session()).unwrap();

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
    let (listener, address) = loopback_listener();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        use std::io::Write;
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_some_request(&mut stream);
        let body = r#"{"data":[]}"#;
        let response = http_response("200 OK", "application/json", body);
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
        let _ = sender.send(request);
    });

    let url = format!("http://{address}/me/library/artists");
    UreqTransport::new()
        .get(&url, &session())
        .expect("a 200 with an empty collection succeeds");

    let request = receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the server captured the request");
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

// The page-bound and unfollowable-link notices are pure, like the other
// report formatters, so the wording and the escaping of server-controlled
// text are testable without capturing stderr. Pin each notice's exact wording,
// including the production `MAX_PAGES` in the bound message.
#[test]
fn page_bound_notice_names_the_unread_page_and_the_bound() {
    assert_eq!(
        page_bound_notice("/v1/me/library/artists?offset=1000"),
        "Apple Music returned a next page \"/v1/me/library/artists?offset=1000\" after 10 pages; stopping at the page limit"
    );
}

#[test]
fn unfollowable_next_notice_names_the_unfollowed_page() {
    assert_eq!(
        unfollowable_next_notice("https://evil.example/steal"),
        "Apple Music returned a next page \"https://evil.example/steal\" that is not a same-origin path; not following it"
    );
}

// `next` is server-controlled and reaches the terminal, so a control character
// or a Unicode format character must be escaped rather than emitted raw, as
// `api_error_cause` escapes an API error detail. Pin both notices: `\u{1b}` (a
// control character) and `\u{202e}` (the right-to-left override, which
// `char::is_control` does not classify as a control).
#[test]
fn pagination_notices_escape_control_and_format_characters() {
    for notice in [
        page_bound_notice("/v1/evil\u{1b}\u{202e}"),
        unfollowable_next_notice("/v1/evil\u{1b}\u{202e}"),
    ] {
        assert!(!notice.contains('\u{1b}'), "{notice:?}");
        assert!(!notice.contains('\u{202e}'), "{notice:?}");
        assert!(notice.contains("\\u{1b}"), "{notice:?}");
        assert!(notice.contains("\\u{202e}"), "{notice:?}");
    }
}

// `absolute_next_url` is the credential-isolation rule: only a same-origin
// path may be followed, so a hostile `next` cannot redirect the session's
// tokens to another host. A relative path is resolved against the API origin;
// an absolute URL, a scheme-relative `//host` target, and a bare relative
// target all yield `None`.
#[test]
fn absolute_next_url_follows_only_a_same_origin_path() {
    assert_eq!(
        absolute_next_url("/v1/me/library/artists?offset=100"),
        Some("https://api.music.apple.com/v1/me/library/artists?offset=100".to_string())
    );
    for next in [
        "https://evil.example/steal",
        "//evil.example/steal",
        "me/library/artists?offset=100",
        "",
    ] {
        assert_eq!(absolute_next_url(next), None, "{next:?}");
    }
}

// `API_BASE` must stay the origin plus the version path, or a followed `next`
// and a first-page query would disagree about where the API lives.
#[test]
fn api_base_is_the_origin_plus_the_version_path() {
    assert_eq!(API_BASE, format!("{API_ORIGIN}/v1"));
}

// A collection over one page links the next; the client follows it and returns
// both pages' rows in order, so a library larger than 100 items is read in
// full. Two recorded calls, the second to the resolved next URL, prove the
// link was followed exactly once.
#[test]
fn a_next_page_is_followed_and_both_pages_are_returned_in_order() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}],"next":"/v1/me/library/artists?offset=1"}"#,
        r#"{"data":[{"id":"artist-2","attributes":{"name":"Echo Chamber"}}]}"#,
    ]);
    let library = library_over(&stub);

    let artists = library.get_favorite_artists(&session()).unwrap();

    assert_eq!(
        artists,
        vec![
            sample_artist(),
            Artist {
                id: "artist-2".to_string(),
                name: "Echo Chamber".to_string(),
            },
        ]
    );
    let calls = stub.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[0].0,
        "https://api.music.apple.com/v1/me/library/artists"
    );
    assert_eq!(
        calls[1].0,
        "https://api.music.apple.com/v1/me/library/artists?offset=1"
    );
}

// Pagination exists so a large library is read in full, so a transport
// failure on a *later* page must surface as an error rather than be swallowed
// to return the earlier pages' rows: silently returning a prefix would
// reintroduce the truncation the paging loop replaced. Page 1 succeeds and
// links page 2; the second request fails. The page-named cause and the two
// recorded calls prove `fetch` followed the link once, stopped at the failure,
// and returned no partial rows.
#[test]
fn a_transport_error_on_a_later_page_surfaces_instead_of_truncating() {
    let stub = StubTransport::returning_results(&[
        Ok(r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}],"next":"/v1/me/library/artists?offset=1"}"#.to_string()),
        Err(AppleMusicError::new("connection reset")),
    ]);
    let library = library_over(&stub);

    let error = library
        .get_favorite_artists(&session())
        .unwrap_err()
        .to_string();

    // The UI labels the query ("loading favorite artists") but cannot know the
    // failure was on page 2, so the transport cause carries the page suffix —
    // the transport-path twin of `a_failure_following_a_next_page_names_the_page`.
    assert_eq!(error, "connection reset (page 2)");
    let calls = stub.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[1].0,
        "https://api.music.apple.com/v1/me/library/artists?offset=1"
    );
}

// An endless `next` chain — a server that always links another page, or one
// that links back to a page already read — must stop at `MAX_PAGES` rather
// than loop forever. The stub repeats its one body, so the call count is the
// page bound.
#[test]
fn an_endless_next_chain_stops_at_the_page_bound() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}],"next":"/v1/me/library/artists?offset=1"}"#,
    );
    let library = library_over(&stub);

    let artists = library.get_favorite_artists(&session()).unwrap();

    assert_eq!(artists.len(), MAX_PAGES);
    assert_eq!(stub.calls().len(), MAX_PAGES);
}

// A `next` link that names another host must not be followed: the session's
// developer and user tokens only ever go to the API origin (the same rule as
// `max_redirects(0)`). One recorded call and the first page's row prove the
// link was dropped.
#[test]
fn a_next_link_to_another_host_is_not_followed() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}],"next":"https://evil.example/steal"}"#,
    );
    let library = library_over(&stub);

    let artists = library.get_favorite_artists(&session()).unwrap();

    assert_eq!(artists.len(), 1);
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/artists");
}

// A failure while following a `next` link must say which page it happened on.
// The UI labels the query ("loading favorite artists"), but only the client
// knows the failure was on page 2 rather than the first page, so without the
// page number the report is ambiguous. The first page's row is returned by the
// stub's first body; the second body is not JSON, so the parse fails on page 2.
#[test]
fn a_failure_following_a_next_page_names_the_page() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}],"next":"/v1/me/library/artists?offset=1"}"#,
        "not json",
    ]);
    let library = library_over(&stub);

    let error = library
        .get_favorite_artists(&session())
        .unwrap_err()
        .to_string();

    assert!(
        error.starts_with("response was not valid JSON:") && error.ends_with("(page 2)"),
        "{error}"
    );
}

// A resource `map` rejects on a later page must name the page, just as a
// transport or parse failure does: the UI labels the query but cannot know
// which page carried the bad resource, so a multi-page browse would otherwise
// report an ambiguous cause. Page 1 maps a valid artist and links page 2;
// page 2's artist has no name.
#[test]
fn a_nameless_resource_on_a_later_page_names_the_page() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}],"next":"/v1/me/library/artists?offset=1"}"#,
        r#"{"data":[{"id":"artist-2","attributes":{}}]}"#,
    ]);
    let library = library_over(&stub);

    let error = library
        .get_favorite_artists(&session())
        .unwrap_err()
        .to_string();

    assert_eq!(
        error,
        "response carried artist \"artist-2\" without a name (page 2)"
    );
}

// `fetch` maps each page's resources as that page is read, before it follows
// the response's `next` link, so a resource that fails validation on page 1 is
// reported immediately and the linked page is never fetched. Pinning the
// single recorded call proves the link was not followed; a regression that
// accumulated every page before mapping (the shape this refactor replaced)
// would request page 2 first, and its transport error would surface instead of
// the page-1 validation failure. The body deliberately links a page the stub
// would answer with a transport error, so the two failures are distinguishable.
#[test]
fn a_nameless_resource_on_the_first_page_stops_before_the_next_page() {
    let stub = StubTransport::returning_results(&[
        Ok(r#"{"data":[{"id":"artist-1","attributes":{}}],"next":"/v1/me/library/artists?offset=1"}"#.to_string()),
        Err(AppleMusicError::new("connection reset")),
    ]);
    let library = library_over(&stub);

    let error = library
        .get_favorite_artists(&session())
        .unwrap_err()
        .to_string();

    assert_eq!(error, "response carried artist \"artist-1\" without a name");
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/artists");
}

// The page suffix is only appended when it adds information: the first page is
// the query the UI already names, so its cause stays bare (the transport and
// parse tests above pin that end to end). A unit test pins the boundary
// directly, so a refactor cannot start appending "(page 1)" to every failure.
#[test]
fn page_context_leaves_the_first_page_message_bare() {
    assert_eq!(page_context("network down", 1), "network down");
    assert_eq!(page_context("network down", 2), "network down (page 2)");
}
