//! Seek arithmetic shared by the production player and the worker tests:
//! relative target computation, clamping to the preview's length, and the
//! small [`SeekablePlayer`] seam that makes the composition testable.

use std::time::Duration;

use rodio::source::SeekError;

/// Returns `position` moved by `offset_secs`: backward for a negative offset
/// (saturating at zero) and forward for a positive one (saturating at
/// [`Duration::MAX`]).
pub(in crate::audio) fn seek_target(position: Duration, offset_secs: i64) -> Duration {
    let offset = Duration::from_secs(offset_secs.unsigned_abs());
    if offset_secs < 0 {
        position.saturating_sub(offset)
    } else {
        position.saturating_add(offset)
    }
}

/// Clamps `position` to the loaded preview's `duration`, or passes it through
/// when the backend reports no length.
pub(in crate::audio) fn clamp_seek(position: Duration, duration: Option<Duration>) -> Duration {
    match duration {
        Some(duration) => position.min(duration),
        None => position,
    }
}

/// The two player operations a relative seek needs, split from
/// [`RodioPlayer`](super::rodio_player::RodioPlayer) so the composition of
/// reading the position and seeking to [`seek_target`] from it is testable
/// without an output device.
pub(in crate::audio) trait SeekablePlayer {
    fn position(&self) -> Duration;
    fn seek_to(&self, position: Duration) -> Result<(), SeekError>;
}

impl SeekablePlayer for rodio::Player {
    fn position(&self) -> Duration {
        self.get_pos()
    }

    fn seek_to(&self, position: Duration) -> Result<(), SeekError> {
        self.try_seek(position)
    }
}

/// Seeks `player` by `offset_secs` from its current position, logging a
/// rejected seek without changing any state.
pub(in crate::audio) fn seek_player(player: &impl SeekablePlayer, offset_secs: i64) {
    let position = player.position();
    if let Err(error) = player.seek_to(seek_target(position, offset_secs)) {
        eprintln!("seeking the preview failed: {error}");
    }
}
