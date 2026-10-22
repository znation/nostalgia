//! The loopback server's HTTP/1.1 mechanics: reading one bounded request and
//! writing one minimal response.
//!
//! These helpers know nothing about `MusicKit` — they parse a request into its
//! method, path, query, host, and body, and write a `Connection: close` response — so the
//! sign-in flow's request routing (in the parent module) reads as application
//! logic over a small protocol layer. The request buffer is capped at
//! [`MAX_REQUEST_BYTES`]: the cap is checked
//! while reading headers *and* before the declared body is read, so a hostile
//! client cannot make the server allocate without bound or panic it with an
//! overflowing `Content-Length`. A request is read *incrementally*:
//! [`RequestReader::poll`] reports what the bytes received so far say and
//! never waits for a peer that has nothing to say yet, so one stalled
//! connection cannot hold the sign-in flow while the browser's callback waits
//! behind it. Each response write is bounded by [`CONNECTION_WRITE_TIMEOUT`],
//! so a client that stops reading cannot block the server indefinitely.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// The largest HTTP request, headers and body combined, the loopback server
/// will buffer. A request larger than this is dropped without being read.
pub(super) const MAX_REQUEST_BYTES: usize = 16 * 1024;

/// How long a single connection may take to send its request before it is
/// dropped as a stalled client. The bound covers the whole request, measured
/// from the moment the connection is accepted, not each read.
pub(super) const CONNECTION_READ_TIMEOUT: Duration = Duration::from_secs(5);

/// How long a single response write may take before it is abandoned as a
/// stalled client. A client that stops reading — or a response larger than
/// the socket send buffer, which the caller controls through the developer
/// token it embeds in the page — would otherwise block `write_all`
/// indefinitely. The bound is fixed rather than tied to the flow deadline
/// because a response is written in one call, not a loop of reads, so one
/// timeout is enough to keep the flow from hanging.
pub(super) const CONNECTION_WRITE_TIMEOUT: Duration = Duration::from_secs(5);

/// A parsed HTTP request: the method, the path (query string stripped), the
/// raw query string (if the target carried one), the `Host` header value (if
/// any), and the body bytes.
pub(super) struct HttpRequest {
    /// The request method as sent (for example, `GET`).
    pub(super) method: String,
    /// The request target with its query string stripped.
    pub(super) path: String,
    /// Everything after the first `?` in the request target, undecoded.
    pub(super) query: Option<String>,
    /// The `Host` header's value, if the request carried one.
    pub(super) host: Option<String>,
    /// The request body bytes; empty when the request declared no body.
    pub(super) body: Vec<u8>,
}

/// Reads one HTTP request from a connection without blocking on the peer.
///
/// The reader owns the socket and keeps it non-blocking, so the sign-in
/// server can hold several connections at once and read each of them in its
/// own turn: a peer that opens a connection and never finishes its request
/// cannot hold the browser's callback behind it.
pub(super) struct RequestReader {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl RequestReader {
    /// Wraps the server side of `stream`.
    ///
    /// The socket is switched to non-blocking mode, so [`Self::poll`] reports
    /// "nothing yet" instead of waiting for a peer that may never speak.
    pub(super) fn new(stream: TcpStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            buffer: Vec::new(),
        })
    }

    /// The socket the request arrived on, for the response write.
    pub(super) fn stream(&mut self) -> &mut TcpStream {
        &mut self.stream
    }

    /// Reads the bytes that are available now and reports what they say about
    /// the request.
    ///
    /// Never blocks: the socket is non-blocking, so a peer that has nothing
    /// to say yields `Ok(None)` at once and the caller can go on serving the
    /// other connections it holds. `Ok(Some(request))` carries the whole
    /// request; `Ok(None)` means more bytes are needed; and an error means
    /// the connection can never be served — the peer hung up, or the request
    /// is too large or malformed. The error names the defect.
    pub(super) fn poll(&mut self) -> io::Result<Option<HttpRequest>> {
        let mut chunk = [0u8; 1024];
        loop {
            let room = MAX_REQUEST_BYTES.saturating_sub(self.buffer.len());
            if room == 0 {
                // The cap is reached and the request is still incomplete, so
                // it can never be served.
                return Err(refused("the request is larger than the cap"));
            }
            let limit = room.min(chunk.len());
            match self.stream.read(&mut chunk[..limit]) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "the peer closed the connection",
                    ));
                }
                Ok(read) => self.buffer.extend_from_slice(&chunk[..read]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    // Nothing more is available right now: report what the
                    // bytes received so far say.
                    return parse_http_request(&self.buffer);
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
            // The request is not whole yet: read on.
            if let Some(request) = parse_http_request(&self.buffer)? {
                return Ok(Some(request));
            }
        }
    }
}

/// Parses the request buffered so far.
///
/// `Ok(Some(request))` carries the whole request, `Ok(None)` means more bytes
/// are needed, and an error means the request can never be served — it is too
/// large or malformed, and the error names the defect.
pub(super) fn parse_http_request(buffer: &[u8]) -> io::Result<Option<HttpRequest>> {
    let Some(header_end) = find_header_end(buffer) else {
        return if buffer.len() >= MAX_REQUEST_BYTES {
            Err(refused("the request headers are larger than the cap"))
        } else {
            Ok(None)
        };
    };

    let header = match std::str::from_utf8(&buffer[..header_end]) {
        Ok(header) => header,
        Err(_) => return Err(refused("request headers are not UTF-8")),
    };
    let mut lines = header.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let Some(method) = parts.next() else {
        return Err(refused("missing method"));
    };
    let method = method.to_string();
    let Some(target) = parts.next() else {
        return Err(refused("missing target"));
    };
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path.to_string(), Some(query.to_string())),
        None => (target.to_string(), None),
    };

    let mut content_length = 0usize;
    let mut host = None;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                // A malformed length is treated as no body; the callback then
                // fails its own validation rather than being read unbounded.
                content_length = value.trim().parse().unwrap_or(0);
            } else if name.eq_ignore_ascii_case("host") {
                host = Some(value.trim().to_string());
            }
        }
    }

    // Refuse an oversized declared body *before* reading it: `checked_add`
    // rejects a `Content-Length` that would overflow `usize`, and the cap check
    // rejects one larger than the buffer. Neither path allocates.
    let body_start = header_end;
    let body_end = match body_start.checked_add(content_length) {
        Some(end) if end <= MAX_REQUEST_BYTES => end,
        _ => return Err(refused("the declared body is larger than the cap")),
    };

    if buffer.len() < body_end {
        return Ok(None);
    }

    Ok(Some(HttpRequest {
        method,
        path,
        query,
        host,
        body: buffer[body_start..body_end].to_vec(),
    }))
}

/// The error for a request the server will not serve: it is too large or
/// malformed, and the message names the defect.
fn refused(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

/// Writes a minimal HTTP/1.1 response and closes the connection.
///
/// The socket is switched back to blocking mode first: the reader keeps it
/// non-blocking so a silent peer cannot hold the accept loop, and the browser
/// must receive the whole response. The write is bounded by
/// [`CONNECTION_WRITE_TIMEOUT`], armed below, so a client that stops reading
/// cannot block `write_all` — and with it the sign-in flow — indefinitely.
/// Only the sign-in page is large enough to fill the socket's send buffer, and
/// it is served only to a request that carries this flow's `state` nonce, so a
/// client that does not know the nonce cannot make the write wait.
///
/// Every response carries `Cache-Control: no-store`: the sign-in page embeds
/// the owner-side developer token, so the browser must never write it to its
/// disk cache, and the other responses share one policy rather than each
/// opting in.
///
/// Every response also carries `Referrer-Policy: no-referrer`. The sign-in
/// page is served at a URL that carries the per-flow `state` nonce — the only
/// secret that gates the developer token the page embeds — and the page loads
/// Apple's `MusicKit` script from a third-party origin, so the browser is
/// about to send a request to that origin with a `Referer`. Whether the
/// nonce-bearing URL travels with it is the *browser's* policy to decide
/// unless the server decides it: a browser configured with `unsafe-url`, or
/// one old enough to default to `no-referrer-when-downgrade` (which sends the
/// full URL on an `http` to `https` request), hands the nonce to that third
/// party, and a holder of the nonce can fetch the page and read the
/// developer token. The server sets the policy itself so the nonce stays
/// owner-side whatever the browser would have done.
pub(super) fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &str,
) -> io::Result<()> {
    let _ = stream.set_nonblocking(false);
    stream.set_write_timeout(Some(CONNECTION_WRITE_TIMEOUT))?;
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

/// Returns the index just past the first `\r\n\r\n` in `buffer`, if any.
fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
}
