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
/// The service's id guard produces one when handed an invalid id:
/// `play_track` rejects a blank or control-character track id, and
/// `get_albums_by_artist`, `get_songs_from_album`, and
/// `get_songs_from_playlist` reject a blank or control-character
/// artist/album/playlist id (an id is blank when it is empty or only
/// whitespace). The REST-backed browse queries report transport, response,
/// and mapping failures through this type as well, while a sample-library
/// lookup with a valid id succeeds.
/// [`crate::apple_music::AppleMusicService::authenticate`] reports a
/// malformed developer token, a browser that will not open, a rejected
/// callback, or a timeout through this type as well.
/// The seam carries this type so the REST client and the sign-in flow can
/// report a failure without tying the seam's public API to a specific HTTP
/// client. The message is the human-readable cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleMusicError(String);

impl std::fmt::Display for AppleMusicError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AppleMusicError {}

// The constructor is reached by the four id-taking queries (including
// `play_track`) through `ensure_id_is_valid`, and is public so the REST client
// and the sign-in flow can report failures from other modules, so it is live and
// needs no `dead_code` allowance. The transport stubs in `apple_music` still carry
// theirs; a *newly* dead private item elsewhere still triggers the compiler's
// `dead_code` warning. (A `pub` item in a `pub mod` is reachable from the
// crate root and so is never reported as dead.)
impl AppleMusicError {
    /// Builds a failure whose [`Display`](std::fmt::Display) output is
    /// `message` — the human-readable cause. The wrapped message is private,
    /// so this constructor (rather than a struct literal) is how the REST
    /// client reports a failure from another module through the seam.
    /// `impl Into<String>` accepts both `&str` and `String`.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::AppleMusicError;

    #[test]
    fn display_echoes_the_message() {
        let err = AppleMusicError::new("boom");
        assert_eq!(err.to_string(), "boom");
    }

    #[test]
    fn new_accepts_string_and_str() {
        let from_str = AppleMusicError::new("x");
        let from_string = AppleMusicError::new(String::from("x"));
        assert_eq!(from_str, from_string);
    }

    #[test]
    fn equality_and_clone() {
        let a = AppleMusicError::new("msg");
        let b = a.clone();
        assert_eq!(a, b);
        assert_ne!(a, AppleMusicError::new("other"));
    }

    #[test]
    fn implements_std_error() {
        let err: &dyn std::error::Error = &AppleMusicError::new("e");
        assert_eq!(err.to_string(), "e");
    }
}
