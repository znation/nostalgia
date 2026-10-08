//! Pure transport sequencing: which song the Previous/Next buttons land on.
//!
//! The transport buttons are a player concern — they step through the
//! currently loaded album's songs — so the index logic lives here as pure
//! functions, mirroring `views.rs`, and knows nothing about `WinampPlayer` or
//! the update loop. `AppleMusicService` stays the library API seam and keeps
//! its `next_track`/`previous_track` stubs untouched.

use crate::library::Song;

/// The id of the song one step from `current` in `songs`, in the given
/// direction. With `repeat` (Repeat on) the step wraps around the ends:
/// forward for Next (starting at the first song when there is no current
/// one), backward for Previous (starting at the last). With `repeat` off
/// (the Winamp default), a step that would cross the boundary stays on the
/// edge song instead of wrapping, so the button re-lands on the current
/// track. `None` when `songs` is empty. Both direction lookups share the
/// empty guard, the current-song position scan, and the boundary handling,
/// so the stepping logic lives here once and the two public functions below
/// only name the direction.
fn stepped_track_id(
    songs: &[Song],
    current: Option<&str>,
    forward: bool,
    repeat: bool,
) -> Option<String> {
    if songs.is_empty() {
        return None;
    }
    let index = current.and_then(|current_id| songs.iter().position(|song| song.id == current_id));
    let stepped = match index {
        // No current song (or one not in the list): land on the edge the step
        // moves toward — the first song going forward, the last going backward.
        // With no current there is no boundary to respect, so this holds with
        // Repeat either on or off.
        None if forward => 0,
        None => songs.len() - 1,
        // One step in the direction. With Repeat on, wrap at the boundary
        // (backward is `len - 1` forward steps mod `len`); with Repeat off,
        // stay on the edge song rather than crossing it.
        Some(i) if forward => {
            if repeat {
                (i + 1) % songs.len()
            } else {
                (i + 1).min(songs.len() - 1)
            }
        }
        Some(i) => {
            if repeat {
                (i + songs.len() - 1) % songs.len()
            } else {
                i.saturating_sub(1)
            }
        }
    };
    Some(songs[stepped].id.clone())
}

/// The id of the song to play when Next is pressed: one past `current` in
/// `songs`. With `repeat`, wraps from the end back to the start; without it
/// (the default), stays on the last song. `None` when there is nothing to
/// step through (empty `songs`); with no `current` (or an unknown one), the
/// first song regardless of `repeat`.
pub fn next_track_id(songs: &[Song], current: Option<&str>, repeat: bool) -> Option<String> {
    stepped_track_id(songs, current, true, repeat)
}

/// The id of the song to play when Previous is pressed: one before `current`
/// in `songs`. With `repeat`, wraps from the start back to the end; without
/// it (the default), stays on the first song. `None` when there is nothing to
/// step through (empty `songs`); with no `current` (or an unknown one), the
/// last song regardless of `repeat`.
pub fn previous_track_id(songs: &[Song], current: Option<&str>, repeat: bool) -> Option<String> {
    stepped_track_id(songs, current, false, repeat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{single_song_album, stepping_songs};

    /// Asserts Next from `current` — with Repeat `repeat` — lands on
    /// `expected` in the shared three-song fixture. The three-song tests all
    /// drive the same `next_track_id(&stepping_songs(), ..)` comparison, so
    /// the fixture and the `Some(..to_string())` wrapping live here once and
    /// each test names only its input and expected id. The empty-list and
    /// single-song tests use different fixtures and call the function
    /// directly.
    fn assert_next(current: Option<&str>, repeat: bool, expected: &str) {
        assert_eq!(
            next_track_id(&stepping_songs(), current, repeat),
            Some(expected.to_string())
        );
    }

    /// The Previous twin of [`assert_next`], over the same three-song fixture.
    fn assert_previous(current: Option<&str>, repeat: bool, expected: &str) {
        assert_eq!(
            previous_track_id(&stepping_songs(), current, repeat),
            Some(expected.to_string())
        );
    }

    /// Asserts both directions on the one-song fixture step to its only song
    /// from `current`. The two single-song tests differ only in the starting
    /// current — the song itself, or none at all — so the fixture and the
    /// two-direction assertion live here once and each test names only the
    /// branch it pins.
    fn assert_single_song_steps_to_itself(current: Option<&str>) {
        let songs = single_song_album();
        assert_eq!(
            next_track_id(&songs, current, false),
            Some("song-1".to_string())
        );
        assert_eq!(
            previous_track_id(&songs, current, false),
            Some("song-1".to_string())
        );
    }

    #[test]
    fn empty_list_is_a_noop_for_both_directions() {
        assert_eq!(next_track_id(&[], Some("song-1"), false), None);
        assert_eq!(previous_track_id(&[], Some("song-1"), false), None);
    }

    #[test]
    fn next_without_current_starts_at_first() {
        assert_next(None, false, "song-1");
    }

    #[test]
    fn previous_without_current_starts_at_last() {
        assert_previous(None, false, "song-3");
    }

    #[test]
    fn next_with_unknown_current_starts_at_first() {
        assert_next(Some("nope"), false, "song-1");
    }

    #[test]
    fn previous_with_unknown_current_starts_at_last() {
        assert_previous(Some("nope"), false, "song-3");
    }

    #[test]
    fn next_advances_through_the_list() {
        assert_next(Some("song-1"), false, "song-2");
        assert_next(Some("song-2"), false, "song-3");
    }

    #[test]
    fn previous_reverses_through_the_list() {
        assert_previous(Some("song-2"), false, "song-1");
    }

    #[test]
    fn next_wraps_from_last_to_first() {
        assert_next(Some("song-3"), true, "song-1");
    }

    #[test]
    fn previous_wraps_from_first_to_last() {
        assert_previous(Some("song-1"), true, "song-3");
    }

    // With Repeat off (the Winamp default) a step that would cross the
    // album's edge stays on the edge song instead of wrapping: Next from the
    // last song re-lands on the last, Previous from the first on the first.
    // The wrap tests above pass `true`; these pin the off half of the
    // boundary at the unit level, while the arm-level repeat tests in
    // `tests.rs` reach it only through `update`.
    #[test]
    fn next_stays_on_last_without_repeat() {
        assert_next(Some("song-3"), false, "song-3");
    }

    #[test]
    fn previous_stays_on_first_without_repeat() {
        assert_previous(Some("song-1"), false, "song-1");
    }

    #[test]
    fn single_song_steps_to_itself_in_both_directions() {
        assert_single_song_steps_to_itself(Some("song-1"));
    }

    // The no-current branch on a one-song list is the one intersection the
    // other stepping tests don't pin: `next_without_current_starts_at_first`
    // and `previous_without_current_starts_at_last` step the three-song
    // album, and `single_song_steps_to_itself_in_both_directions` sets a
    // current track. With no current on a one-song album, Next must land on
    // the forward edge (the first song) and Previous on the backward edge
    // (the last) — the same song here — not `None`. A regression that
    // misread "no current" on a one-element list as "nothing to step to"
    // (e.g. special-casing a short list to return `None`) would clear every
    // other test and only fail here.
    #[test]
    fn single_song_with_no_current_steps_to_itself_in_both_directions() {
        assert_single_song_steps_to_itself(None);
    }

    // The documented no-current contract is direction-only — the first song
    // going forward, the last going backward — with the shared Repeat flag
    // irrelevant because there is no boundary to wrap. Every no-current test
    // above passes Repeat off, so a regression that made the no-current
    // branch consult Repeat (e.g. returning the last song going forward when
    // Repeat is on) would clear them all. Pin the on half too, for both a
    // missing and an unknown current.
    #[test]
    fn no_current_step_ignores_repeat() {
        for current in [None, Some("nope")] {
            assert_next(current, true, "song-1");
            assert_previous(current, true, "song-3");
        }
    }
}
