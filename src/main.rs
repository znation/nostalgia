//! The nostalgia binary: it builds the shared `AppState`, wires it into the
//! stub Apple Music service, and hands both to the iced application.
//!
//! `unsafe` is forbidden crate-wide: the player is safe Rust, so an `unsafe`
//! block added here is a mistake worth failing the build for rather than
//! catching in review.
#![forbid(unsafe_code)]
// Doc comments name code items (`Message` variants, types, functions); the
// `doc_markdown` lint keeps those identifiers backticked so rustdoc renders
// them as code. It is allow-by-default, so the landing gate's `-D warnings`
// would not enable it; denying it here makes an unbackticked identifier fail
// `make check` like any other warning.
#![deny(clippy::doc_markdown)]
// Every public item carries a doc comment (the seam and the model document
// their contracts), and rustdoc renders those comments as the crate's API
// reference; the `missing_docs` lint keeps a new public item from landing
// undocumented. It is allow-by-default, so the landing gate's `-D warnings`
// would not enable it; denying it here makes an undocumented public item fail
// `make check` like any other warning. The lint sees only items reachable from
// the crate root, so the production modules below are declared `pub mod`: a
// `pub` item inside a private module is unreachable and would go unchecked.
#![deny(missing_docs)]
// A public function returning a `Result` documents when it returns `Err`, so
// a caller of the Apple Music seam can read the failure contract without
// tracing the body. The `missing_errors_doc` lint keeps a new fallible public
// function from landing without its `# Errors` section; it is
// allow-by-default (a pedantic lint), so the landing gate's `-D warnings`
// would not enable it, and denying it here makes the omission fail
// `make check` like any other warning.
#![deny(clippy::missing_errors_doc)]
// A `dbg!` left in the tree prints to stderr and is never intended to ship;
// the crate's intentional output goes through `println!`/`eprintln!` at named
// sites, so a stray debug print is a mistake worth failing the build for. The
// `dbg_macro` lint is allow-by-default (a restriction lint), so the landing
// gate's `-D warnings` would not enable it; denying it here makes a `dbg!`
// fail `make check` like any other warning. Unlike the panic lints below it is
// denied in tests too: a debug print is not a test's assertion mechanism, and
// a temporary `dbg!` left behind is exactly what this guard catches.
#![deny(clippy::dbg_macro)]
// Production code must not abort the player on an unexpected value: an
// `unwrap`/`expect`/`panic`/`todo`/`unimplemented` crashes the process instead
// of reporting the failure through the `AppleMusicError` seam or the clamps in
// `state`/`equalizer`. These restriction lints are allow-by-default, so the
// landing gate's `-D warnings` would not enable them; denying them here makes
// a production panic fail `make check`. They are denied only for the non-test
// build: the `#[cfg(test)]` modules use `unwrap`/`expect`/`panic!` to assert,
// which is what a test is for.
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented
    )
)]

use std::sync::Arc;
use tokio::sync::Mutex;

pub mod apple_music;
pub mod clamp;
pub mod equalizer;
pub mod library;
pub mod music_kit_auth;
pub mod sample_library;
pub mod state;
#[cfg(test)]
mod test_support;
pub mod ui;

use crate::state::AppState;

fn main() -> iced::Result {
    let state = Arc::new(Mutex::new(AppState::default()));

    // Initialize the (stub) Apple Music service. The same service instance is
    // handed to the UI, so the UI's clone shares the session the startup
    // sign-in stores.
    let service = apple_music::init_service(state.clone());

    println!("Winamp-style Apple Music Player started!");

    // Run the UI; blocks until the window is closed.
    ui::init_ui(state, service)
}
