//! Test-only fixtures and assertions shared across the crate's unit tests.
//!
//! One submodule per kind of helper, re-exported here so callers keep naming
//! `crate::test_support::<item>`:
//!
//! - `fixtures`: the sample `Artist`/`Album`/`Song`/`Playlist` values, stepping
//!   fixtures, the shared preview URL, the sign-in session fixtures, and the
//!   `"Rock"` preset.
//! - `assertions`: the serde-contract assertions and the id-list assertion.
//! - `transport`: the recording `HttpTransport` stub.
//! - `audio`: the recording and failing `AudioOutput` fakes.
//! - `loopback`: the loopback HTTP server fixtures and gzip builders.
//!
//! They live in one named module rather than inside `library` so the data
//! model module stays only the model; every item here is compiled only for
//! tests.

mod assertions;
mod audio;
mod fixtures;
mod loopback;
mod transport;

pub(crate) use assertions::*;
pub(crate) use audio::*;
pub(crate) use fixtures::*;
pub(crate) use loopback::*;
pub(crate) use transport::*;
