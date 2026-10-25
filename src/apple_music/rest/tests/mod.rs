use std::time::{Duration, Instant};

use super::paging::*;
use super::transport::{MAX_RESPONSE_BYTES, shared_agent};
use super::wire::api_error_cause;
use super::*;

use crate::test_support::{
    PREVIEW_URL, StubTransport, assert_ids, assert_single_call, gzip_empty_stored_blocks,
    gzip_zeros, loopback_listener, record_first_request, rest_session, sample_album, sample_artist,
    sample_playlist, sample_song, serve_one_response, serve_one_response_bytes,
    serve_one_response_capturing_request,
};

/// A [`RestLibrary`] over `stub`, plus the handle to inspect its calls.
fn library_over(stub: &StubTransport) -> RestLibrary {
    RestLibrary::new(Box::new(stub.clone()))
}

/// The error message `query` reports when the transport answers with `body`.
///
/// The malformed-body and rejected-resource tests all build the same
/// single-page stub over `body` and reduce their query's error to its `Display`
/// string; this runs that scaffold once. `query` names the browse method under
/// test, so each test states only the response body and the expected report.
fn error_over<T>(
    body: &str,
    query: impl FnOnce(&RestLibrary) -> Result<T, AppleMusicError>,
) -> String {
    let stub = StubTransport::returning(body);
    let library = library_over(&stub);
    query(&library).map(|_| ()).unwrap_err().to_string()
}

/// The error message `query` reports when the transport itself fails with
/// "network down".
///
/// The bare-cause tests all build the same failing stub and reduce their
/// query's error to its `Display` string; this runs that scaffold once.
fn transport_error_over<T>(
    query: impl FnOnce(&RestLibrary) -> Result<T, AppleMusicError>,
) -> String {
    let stub = StubTransport::failing("network down");
    let library = library_over(&stub);
    query(&library).map(|_| ()).unwrap_err().to_string()
}

mod library;
mod paging;
mod transport;
