//! The failure type shared by the Apple Music service seam and the `MusicKit`
//! authorization flow.
//!
//! It lives in its own module so the two can share one error without depending
//! on each other: [`crate::music_kit_auth`] reports a failed sign-in with it,
//! [`crate::apple_music`] reports a failed browse, playback, or sign-in with
//! it, and only `apple_music` depends on `music_kit_auth`, never the reverse.
//! The type stays reachable as `apple_music::AppleMusicError` through the
//! re-export in [`crate::apple_music`].

/// A failed music-library, playback, or sign-in operation.
///
/// The sample-library stub produces one when handed an invalid id:
/// `play_track` rejects a blank or control-character track id, and
/// `get_albums_by_artist` / `get_songs_from_album` reject a blank or
/// control-character artist/album id (an id is blank when it is empty or only
/// whitespace). [`crate::apple_music::AppleMusicService::authenticate`] reports
/// a malformed developer token, a browser that will not open, a rejected
/// callback, or a timeout through this type as well. Every other in-memory
/// query and playback transition succeeds.
/// The seam carries this type so a real Apple Music
/// backend can report a failure without tying the seam's public API to a
/// specific HTTP client. The message is the human-readable cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleMusicError(String);

impl std::fmt::Display for AppleMusicError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AppleMusicError {}

// The constructor is reached by `play_track` and the two browse queries through
// `ensure_id_is_valid`, and stays public for a real Apple Music backend that
// will report failures from another module, so it is live and needs no
// `dead_code` allowance. The transport stubs in `apple_music` still carry
// theirs; a *newly* dead private item elsewhere still triggers the compiler's
// `dead_code` warning. (A `pub` item in a `pub mod` is reachable from the
// crate root and so is never reported as dead.)
impl AppleMusicError {
    /// Builds a failure whose [`Display`](std::fmt::Display) output is
    /// `message` — the human-readable cause. The wrapped message is private,
    /// so this constructor (rather than a struct literal) is how a real Apple
    /// Music backend outside this module reports a failure through the seam.
    /// `impl Into<String>` accepts both `&str` and `String`.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
