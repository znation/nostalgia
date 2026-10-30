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
pub struct AppleMusicError {
    message: String,
    kind: AppleMusicErrorKind,
}

/// What category of failure an [`AppleMusicError`] reports. Carried by the
/// error so a consumer that knows what it can do about a failure — the UI's
/// browse failure report, say — can read it without parsing the message.
/// New constructors default to [`AppleMusicErrorKind::Other`]; only the REST
/// transport's auth-rejection branch builds [`AppleMusicErrorKind::Auth`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppleMusicErrorKind {
    /// Apple Music rejected the request's authorization — a stale or revoked
    /// `MusicKit` user token, an expired developer token. The recovery is
    /// signing in again.
    Auth,
    /// Every other failure: a transport error, an unreadable or oversized
    /// response body, a non-success status that is not an auth rejection, an
    /// invalid id, a failed sign-in flow.
    Other,
}

impl std::fmt::Display for AppleMusicError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
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
        Self {
            message: message.into(),
            kind: AppleMusicErrorKind::Other,
        }
    }

    /// Builds a failure whose [`Display`](std::fmt::Display) output is
    /// `message` and whose [`kind`](AppleMusicError::kind) is
    /// [`AppleMusicErrorKind::Auth`] — the constructor the REST transport
    /// uses for a 401/403 reply, where Apple Music rejected the request's
    /// authorization. The message text is unchanged from what a plain `new`
    /// would carry; only the kind differs, so `Display`-based reports read
    /// the same and kind-aware consumers can react.
    #[must_use]
    pub fn new_auth(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: AppleMusicErrorKind::Auth,
        }
    }

    /// The failure's category. `Other` for every failure but a transport
    /// 401/403 reply, which is [`AppleMusicErrorKind::Auth`].
    #[must_use]
    pub fn kind(&self) -> AppleMusicErrorKind {
        self.kind.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::AppleMusicError;
    use super::AppleMusicErrorKind;

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

    // The kind is what the browse failure report reads to decide whether to
    // hint at signing in again, so the two constructors' kinds are pinned:
    // `new_auth` carries the auth kind a 401/403 reply is built with, and
    // `new` — the constructor every other failure path uses — stays `Other`.
    #[test]
    fn new_auth_carries_the_auth_kind_and_new_the_other_kind() {
        assert_eq!(
            AppleMusicError::new_auth("x").kind(),
            AppleMusicErrorKind::Auth
        );
        assert_eq!(AppleMusicError::new("x").kind(), AppleMusicErrorKind::Other);
    }

    // The kind must survive the clone and equality the seam's callers rely on
    // (stub replies are compared for equality in the transport tests), or an
    // auth classification would be lost when an error travels through the
    // seam.
    #[test]
    fn the_kind_survives_clone_and_equality() {
        let auth = AppleMusicError::new_auth("msg");
        assert_eq!(auth, auth.clone());
        assert_eq!(auth.clone().kind(), AppleMusicErrorKind::Auth);
        assert_eq!(
            AppleMusicError::new_auth("m"),
            AppleMusicError::new_auth("m")
        );
        let other = AppleMusicError::new("msg");
        assert_ne!(auth, other, "equal messages with different kinds differ");
        assert_ne!(auth.kind(), other.kind());
    }

    // The kind is metadata, not message text: an auth-classified error's
    // `Display` output is the bare cause, unchanged from what a plain `new`
    // with the same message renders, so `Display`-based reports stay stable.
    #[test]
    fn an_auth_errors_display_is_its_bare_message() {
        assert_eq!(
            AppleMusicError::new_auth("HTTP 401").to_string(),
            "HTTP 401"
        );
    }
}
