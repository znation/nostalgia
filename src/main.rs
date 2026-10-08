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

mod apple_music;
mod clamp;
mod equalizer;
mod library;
mod sample_library;
mod state;
#[cfg(test)]
mod test_support;
mod ui;

use crate::state::AppState;

fn main() -> iced::Result {
    let state = Arc::new(Mutex::new(AppState::default()));

    // Initialize the (stub) Apple Music service.
    apple_music::init_service(state.clone());

    println!("Winamp-style Apple Music Player started!");

    // Run the UI; blocks until the window is closed.
    ui::init_ui(state)
}
