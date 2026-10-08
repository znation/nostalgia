//! The loopback server's HTTP/1.1 mechanics: reading one bounded request and
//! writing one minimal response.
//!
//! These helpers know nothing about `MusicKit` — they parse a request into its
//! method, path, query, host, and body, and write a `Connection: close` response — so the
//! sign-in flow's request routing (in the parent module) reads as application
//! logic over a small protocol layer. The request buffer is capped at
//! [`MAX_REQUEST_BYTES`] while reading headers *and* before the declared body
//! is read, so a hostile client cannot make the server allocate without bound
//! or panic it with an overflowing `Content-Length`. Each response write is
//! bounded by [`CONNECTION_WRITE_TIMEOUT`], so a client that stops reading
//! cannot block the server indefinitely.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

/// The largest HTTP request, headers and body combined, the loopback server
/// will buffer. A request larger than this is dropped without being read.
pub(super) const MAX_REQUEST_BYTES: usize = 16 * 1024;

/// How long a single connection may take to send its request before it is
/// dropped as a stalled client. Each read is capped further by the time left
/// before the flow's deadline.
pub(super) const CONNECTION_READ_TIMEOUT: Duration = Duration::from_secs(5);

/// How long a single response write may take before it is abandoned as a
/// stalled client. A client that stops reading — or a response larger than the
/// socket send buffer, which the caller controls through the developer token it
/// embeds in the page — would otherwise block `write_all` indefinitely. The
/// bound is fixed rather than tied to the flow deadline because a response is
/// written in one call, not a loop of reads, so one timeout is enough to keep
/// the flow from hanging.
pub(super) const CONNECTION_WRITE_TIMEOUT: Duration = Duration::from_secs(5);

/// The smallest read timeout to arm. A zero (or sub-granularity) timeout means
/// "block forever", so a deadline closer than this is treated as already
/// arrived rather than armed.
const MIN_READ_TIMEOUT: Duration = Duration::from_millis(1);

/// A parsed HTTP request: the method, the path (query string stripped), the
/// raw query string (if the target carried one), the `Host` header value (if
/// any), and the body bytes.
pub(super) struct HttpRequest {
    pub(super) method: String,
    pub(super) path: String,
    /// Everything after the first `?` in the request target, undecoded.
    pub(super) query: Option<String>,
    pub(super) host: Option<String>,
    pub(super) body: Vec<u8>,
}

/// Reads one HTTP request into memory, bounded by [`MAX_REQUEST_BYTES`].
///
/// Returns `Ok(None)` when the request is unusable — the peer closed before a
/// full request, the connection timed out, `deadline` arrived, or the declared
/// body is larger than the cap (including a `Content-Length` that would
/// overflow `usize`) — so the caller can ignore it and keep waiting.
pub(super) fn read_http_request(
    stream: &mut TcpStream,
    deadline: Instant,
) -> io::Result<Option<HttpRequest>> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 1024];

    // Read until the end of the headers, never buffering past the cap.
    let header_end = loop {
        if let Some(end) = find_header_end(&buffer) {
            break end;
        }
        if buffer.len() >= MAX_REQUEST_BYTES {
            return Ok(None);
        }
        if !arm_read_timeout(stream, deadline) {
            return Ok(None);
        }
        let read = match stream.read(&mut chunk) {
            Ok(0) => return Ok(None),
            Ok(read) => read,
            Err(error) if is_timeout(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        let room = MAX_REQUEST_BYTES - buffer.len();
        buffer.extend_from_slice(&chunk[..read.min(room)]);
    };

    let header = std::str::from_utf8(&buffer[..header_end])
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "request headers are not UTF-8"))?;
    let mut lines = header.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing request line"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing method"))?
        .to_string();
    let target = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing target"))?;
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
        _ => return Ok(None),
    };

    while buffer.len() < body_end {
        if !arm_read_timeout(stream, deadline) {
            return Ok(None);
        }
        let room = body_end - buffer.len();
        let limit = room.min(chunk.len());
        let read = match stream.read(&mut chunk[..limit]) {
            Ok(0) => return Ok(None),
            Ok(read) => read,
            Err(error) if is_timeout(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        buffer.extend_from_slice(&chunk[..read]);
    }

    Ok(Some(HttpRequest {
        method,
        path,
        query,
        host,
        body: buffer[body_start..body_end].to_vec(),
    }))
}

/// Writes a minimal HTTP/1.1 response and closes the connection.
///
/// Arms [`CONNECTION_WRITE_TIMEOUT`] first, so a client that stops reading
/// cannot block `write_all` — and with it the sign-in flow — indefinitely. A
/// small response to a live browser is unaffected.
pub(super) fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &str,
) -> io::Result<()> {
    stream.set_write_timeout(Some(CONNECTION_WRITE_TIMEOUT))?;
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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

/// Arms the read timeout for the next read, never letting it outlive the
/// overall `deadline`. Returns `false` when the deadline has arrived (or is
/// closer than the timer's granularity), so the caller gives up without
/// blocking instead of waiting out [`CONNECTION_READ_TIMEOUT`].
fn arm_read_timeout(stream: &TcpStream, deadline: Instant) -> bool {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining < MIN_READ_TIMEOUT {
        return false;
    }
    // `remaining` is at least `MIN_READ_TIMEOUT`, so this is never the zero
    // timeout that would mean "block forever".
    let _ = stream.set_read_timeout(Some(remaining.min(CONNECTION_READ_TIMEOUT)));
    true
}

/// Whether an I/O error is a read timeout rather than a real failure.
fn is_timeout(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}
