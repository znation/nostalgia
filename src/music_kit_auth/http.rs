//! The loopback server's HTTP/1.1 mechanics: reading one bounded request and
//! writing one minimal response.
//!
//! These helpers know nothing about `MusicKit` — they parse a request into its
//! method, path, and body, and write a `Connection: close` response — so the
//! sign-in flow's request routing (in the parent module) reads as application
//! logic over a small protocol layer. The request buffer is capped at
//! [`MAX_REQUEST_BYTES`] while reading headers *and* before the declared body
//! is read, so a hostile client cannot make the server allocate without bound
//! or panic it with an overflowing `Content-Length`.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// The largest HTTP request, headers and body combined, the loopback server
/// will buffer. A request larger than this is dropped without being read.
const MAX_REQUEST_BYTES: usize = 16 * 1024;

/// How long a single connection may take to send its request before it is
/// dropped as a stalled client.
pub(super) const CONNECTION_READ_TIMEOUT: Duration = Duration::from_secs(5);

/// A parsed HTTP request: the method, the path (query string stripped), and the
/// body bytes.
pub(super) struct HttpRequest {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) body: Vec<u8>,
}

/// Reads one HTTP request into memory, bounded by [`MAX_REQUEST_BYTES`].
///
/// Returns `Ok(None)` when the request is unusable — the peer closed before a
/// full request, the connection timed out, or the declared body is larger than
/// the cap (including a `Content-Length` that would overflow `usize`) — so the
/// caller can ignore it and keep waiting.
pub(super) fn read_http_request(stream: &mut TcpStream) -> io::Result<Option<HttpRequest>> {
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
    let path = target.split('?').next().unwrap_or(target).to_string();

    let mut content_length = 0usize;
    for line in lines {
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            // A malformed length is treated as no body; the callback then
            // fails its own validation rather than being read unbounded.
            content_length = value.trim().parse().unwrap_or(0);
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
        body: buffer[body_start..body_end].to_vec(),
    }))
}

/// Writes a minimal HTTP/1.1 response and closes the connection.
pub(super) fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &str,
) -> io::Result<()> {
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

/// Whether an I/O error is a read timeout rather than a real failure.
fn is_timeout(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}
