//! Loopback HTTP server fixtures and gzip builders shared by the REST and audio tests.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

/// Binds a loopback listener, returning it with the address it bound, so a
/// test can stand up a loopback server on an ephemeral port without repeating
/// the bind. [`serve_one_response`], the `apple_music::rest` redirect and
/// request-capture tests, and the `audio` preview-download tests call it.
pub(crate) fn loopback_listener() -> (TcpListener, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    (listener, address)
}

/// Reads whatever one request has sent within a one-second bound and returns
/// it lossily as text, so a test can look for a header without a full parse.
pub(crate) fn read_some_request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let mut buffer = [0u8; 4096];
    let read = stream.read(&mut buffer).unwrap_or(0);
    String::from_utf8_lossy(&buffer[..read]).into_owned()
}

/// The first request recorded by [`record_first_request`], together with the
/// name of the thread that recorded it so a test can pin the helper's thread
/// name instead of leaving it unobservable.
pub(crate) struct RecordedFirstRequest {
    /// The request text, or `None` when no request arrived within the bound.
    pub(crate) request: Option<String>,
    /// The recording thread's name, read on that thread.
    pub(crate) thread_name: Option<String>,
}

/// Spawns a thread that accepts the first request to `listener` and returns
/// its text, giving up after one second so "no request arrived" is a bounded
/// result rather than a hang. The listener is switched to nonblocking and
/// polled; a `WouldBlock` before the deadline sleeps, and the deadline returns
/// a record with no request.
///
/// The `apple_music::rest` and `audio` suites' redirect tests both stand up a
/// "foreign" listener this way to prove a redirect was not followed: joining
/// the handle and finding no request means no request reached the redirect
/// target.
pub(crate) fn record_first_request(
    listener: TcpListener,
) -> std::thread::JoinHandle<RecordedFirstRequest> {
    listener.set_nonblocking(true).unwrap();
    std::thread::Builder::new()
        .name("nostalgia-test-listener".to_string())
        .spawn(move || {
            let thread_name = std::thread::current().name().map(str::to_string);
            let deadline = Instant::now() + Duration::from_secs(1);
            loop {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        return RecordedFirstRequest {
                            request: Some(read_some_request(&mut stream)),
                            thread_name,
                        };
                    }
                    Err(ref error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() >= deadline =>
                    {
                        return RecordedFirstRequest {
                            request: None,
                            thread_name,
                        };
                    }
                    Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("foreign listener failed: {error}"),
                }
            }
        })
        .unwrap()
}

/// Serves exactly one HTTP response on a fresh loopback listener: binds,
/// accepts a single connection, reads whatever request arrived, writes
/// `response` verbatim, and closes. Returns the bound address so a test can
/// point an HTTP client at it without rebuilding the accept/read/write
/// scaffolding. Shared by the `apple_music::rest` and `audio` suites.
///
/// The recording-free variant of [`serve_one_response_capturing_request`]:
/// the request it read is dropped, so a test that needs those bytes calls
/// that helper instead.
pub(crate) fn serve_one_response(response: &str) -> SocketAddr {
    let (address, _request) = serve_one_response_capturing_request(response);
    address
}

/// [`serve_one_response`] with a binary response, so a test can serve a body
/// that is not valid UTF-8 (a gzip stream, say). Otherwise identical: binds,
/// accepts one connection, reads whatever request arrived, writes `response`
/// verbatim, and closes.
pub(crate) fn serve_one_response_bytes(response: &[u8]) -> SocketAddr {
    spawn_one_shot_server(response.to_vec()).0
}

/// The request served by [`serve_one_response_capturing_request`], together
/// with the name of the thread that served it so a test can pin the helper's
/// thread name instead of leaving it unobservable.
pub(crate) struct CapturedRequest {
    /// The request text the server read.
    pub(crate) request: String,
    /// The serving thread's name, read on that thread.
    pub(crate) thread_name: Option<String>,
}

/// Serves exactly one HTTP response on a fresh loopback listener and returns
/// the bound address together with a receiver for the request text it read, so
/// a test can assert on the bytes the client sent as well as the response. The
/// accept/read/write scaffolding lives here once; [`serve_one_response`] is
/// the recording-free variant that drops the receiver. The `apple_music::rest`
/// request-capture test calls it.
pub(crate) fn serve_one_response_capturing_request(
    response: &str,
) -> (SocketAddr, std::sync::mpsc::Receiver<CapturedRequest>) {
    spawn_one_shot_server(response.as_bytes().to_vec())
}

/// The shared one-shot server behind the `serve_one_response*` helpers: a
/// named thread accepts one connection, reads the request, writes `response`
/// verbatim, and reports the request on the returned receiver (which callers
/// that do not need it simply drop).
fn spawn_one_shot_server(
    response: Vec<u8>,
) -> (SocketAddr, std::sync::mpsc::Receiver<CapturedRequest>) {
    let (listener, address) = loopback_listener();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("nostalgia-test-server".to_string())
        .spawn(move || {
            let thread_name = std::thread::current().name().map(str::to_string);
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_some_request(&mut stream);
            let _ = stream.write_all(&response);
            let _ = stream.flush();
            let _ = sender.send(CapturedRequest {
                request,
                thread_name,
            });
        })
        .unwrap();
    (address, receiver)
}

/// Builds a gzip stream with `blocks` non-final empty stored blocks and a
/// final empty one, decoding to no bytes while occupying five wire bytes per
/// block. The wire-only expansion that a decoded-byte cap misses. Shared by
/// the preview and REST body-cap tests.
pub(crate) fn gzip_empty_stored_blocks(blocks: usize) -> Vec<u8> {
    // gzip header: magic, deflate, no flags, no mtime, OS = unknown.
    let mut bytes = vec![0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, 0, 0xff];
    // BFINAL = 0, BTYPE = 00, padded to a byte, then LEN = 0, NLEN = 0xffff.
    for _ in 0..blocks {
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0xff, 0xff]);
    }
    // The final block sets BFINAL = 1; empty output has CRC 0 and size 0.
    bytes.extend_from_slice(&[0x01, 0x00, 0x00, 0xff, 0xff]);
    bytes.extend_from_slice(&[0; 8]);
    bytes
}

/// Builds a gzip stream that decodes to `1 + 258 * matches` zero bytes: one
/// literal zero, then `matches` DEFLATE fixed-Huffman length-258/distance-1
/// matches. A body much smaller than its decoded output is the shape that
/// defeats a cap applied to the wire bytes; the suite has no compression
/// crate, and fixed Huffman is the least code that writes such a stream.
/// Shared by the preview and REST body-cap tests.
pub(crate) fn gzip_zeros(matches: usize) -> Vec<u8> {
    struct BitWriter {
        bytes: Vec<u8>,
        current: u8,
        used: u8,
    }
    impl BitWriter {
        fn new() -> Self {
            // gzip header: magic, deflate, no flags, no mtime, OS = unknown.
            Self {
                bytes: vec![0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, 0, 0xff],
                current: 0,
                used: 0,
            }
        }
        fn bit(&mut self, bit: u8) {
            self.current |= (bit & 1) << self.used;
            self.used += 1;
            if self.used == 8 {
                self.bytes.push(self.current);
                self.current = 0;
                self.used = 0;
            }
        }
        fn code(&mut self, code: u32, bits: u8) {
            for shift in (0..bits).rev() {
                self.bit(((code >> shift) & 1) as u8);
            }
        }
        fn finish(mut self) -> Vec<u8> {
            if self.used > 0 {
                self.bytes.push(self.current);
            }
            self.bytes
        }
    }

    let mut writer = BitWriter::new();
    writer.bit(1); // BFINAL
    writer.bit(1); // BTYPE = 01 (fixed), least-significant bit first
    writer.bit(0);
    writer.code(0x30, 8); // literal 0x00
    for _ in 0..matches {
        writer.code(0xC5, 8); // length 258, no extra bits
        writer.code(0, 5); // distance 1
    }
    writer.code(0, 7); // end of block
    let decoded = 1 + 258 * matches;
    let mut bytes = writer.finish();
    bytes.extend_from_slice(&crc32_of_zeros(decoded).to_le_bytes());
    bytes.extend_from_slice(&(decoded as u32).to_le_bytes());
    bytes
}

/// CRC-32 (IEEE, reflected) of `count` zero bytes, the gzip trailer's check
/// value for [`gzip_zeros`]. Built from a runtime table so the test is fast.
fn crc32_of_zeros(count: usize) -> u32 {
    let mut table = [0u32; 256];
    for (index, slot) in table.iter_mut().enumerate() {
        let mut value = index as u32;
        for _ in 0..8 {
            value = if value & 1 == 0 {
                value >> 1
            } else {
                0xedb8_8320 ^ (value >> 1)
            };
        }
        *slot = value;
    }
    let mut crc = 0xffff_ffffu32;
    for _ in 0..count {
        crc = table[(crc & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffff_ffff
}

#[cfg(test)]
mod loopback_thread_names {
    use std::io::Write;
    use std::net::TcpStream;
    use std::time::Duration;

    use super::{loopback_listener, record_first_request, serve_one_response_capturing_request};

    // Both one-shot loopback helpers run on named threads so a panic there is
    // attributed to `nostalgia-test-*` rather than `<unnamed>`. Pin each name
    // on the helper's own thread; the name is spelled out here rather than
    // read back from the helper, so a rename in either place fails the test.
    #[test]
    fn record_first_request_names_its_thread() {
        let (listener, address) = loopback_listener();
        let handle = record_first_request(listener);
        let mut client = TcpStream::connect(address).unwrap();
        client.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();

        let recorded = handle.join().unwrap();
        assert!(recorded.request.is_some());
        assert_eq!(
            recorded.thread_name.as_deref(),
            Some("nostalgia-test-listener")
        );
    }

    #[test]
    fn serve_one_response_capturing_request_names_its_thread() {
        let (address, receiver) = serve_one_response_capturing_request(
            "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        let mut client = TcpStream::connect(address).unwrap();
        client.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();

        let captured = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(captured.request.starts_with("GET / HTTP/1.1"));
        assert_eq!(
            captured.thread_name.as_deref(),
            Some("nostalgia-test-server")
        );
    }
}
