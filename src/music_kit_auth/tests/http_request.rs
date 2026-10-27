use super::*;

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

// A request line is `method SP target SP HTTP-version`; a line missing the
// version, naming one this HTTP/1.1 server does not speak, or carrying a
// fourth token is malformed rather than an older or newer dialect. Refusing it
// keeps the parser from accepting a line whose fields it only guessed at.
#[test]
fn parse_http_request_rejects_a_request_line_without_an_http_version() {
    assert_request_parse_error(b"GET /\r\n\r\n", "missing HTTP version");
}

#[test]
fn parse_http_request_rejects_an_unsupported_http_version() {
    assert_request_parse_error(b"GET / HTTP/2.0\r\n\r\n", "unsupported HTTP version");
}

#[test]
fn parse_http_request_rejects_text_after_the_http_version() {
    assert_request_parse_error(
        b"GET / HTTP/1.1 extra\r\n\r\n",
        "unexpected text after the HTTP version",
    );
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

#[test]
fn the_reader_preserves_a_partial_body_between_polls() {
    let (server, mut client) = connected_pair();
    client
        .write_all(b"POST /token?nonce=test HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 11\r\n\r\nhello")
        .expect("write headers and the first body fragment");
    server
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("bound the readiness check");
    // Ensure the first poll has bytes to consume, rather than merely polling
    // an empty socket before both fragments arrive together.
    assert_eq!(
        server.peek(&mut [0u8; 1]).expect("first fragment arrived"),
        1
    );
    let mut reader = http::RequestReader::new(server).expect("wrap the server side");
    assert!(reader.poll().expect("read the first fragment").is_none());
    assert!(
        reader
            .poll()
            .expect("poll the stalled body again")
            .is_none()
    );

    client.write_all(b" world").expect("finish the body");
    let deadline = Instant::now() + Duration::from_secs(2);
    let request = loop {
        if let Some(request) = reader.poll().expect("resume the partial request") {
            break request;
        }
        assert!(Instant::now() < deadline, "the completed request was lost");
        thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/token");
    assert_eq!(request.query.as_deref(), Some("nonce=test"));
    assert_eq!(request.host.as_deref(), Some("127.0.0.1"));
    assert_eq!(request.body, b"hello world");
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
