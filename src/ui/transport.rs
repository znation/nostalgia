//! Pure transport sequencing: which song the Previous/Next buttons land on.
//!
//! The transport buttons are a player concern — they step through the
//! currently loaded album's songs — so the index logic lives here as pure
//! functions, mirroring `views.rs`, and knows nothing about `WinampPlayer` or
//! the update loop. `AppleMusicService` stays the library API seam and keeps
//! its `next_track`/`previous_track` stubs untouched.

use crate::library::Song;

/// The id of the song one step from `current` in `songs`, in the given
/// direction, wrapping around the ends: forward for Next (starting at the
/// first song when there is no current one), backward for Previous (starting
/// at the last). `None` when `songs` is empty. Both direction lookups share
/// the empty guard, the current-song position scan, and the wrap-around, so
/// the stepping logic lives here once and the two public functions below only
/// name the direction.
fn stepped_track_id(songs: &[Song], current: Option<&str>, forward: bool) -> Option<String> {
    if songs.is_empty() {
        return None;
    }
    let index = current.and_then(|current_id| songs.iter().position(|song| song.id == current_id));
    let stepped = match index {
        // No current song (or one not in the list): land on the edge the step
        // moves toward — the first song going forward, the last going backward.
        None if forward => 0,
        None => songs.len() - 1,
        // One step in the direction, wrapping at the boundary. Backward is
        // `len - 1` forward steps mod `len`.
        Some(i) if forward => (i + 1) % songs.len(),
        Some(i) => (i + songs.len() - 1) % songs.len(),
    };
    Some(songs[stepped].id.clone())
}

/// The id of the song to play when Next is pressed: one past `current` in
/// `songs`, wrapping from the end back to the start. `None` when there is
/// nothing to step through (empty `songs`); with no `current` (or an unknown
/// one), the first song.
pub fn next_track_id(songs: &[Song], current: Option<&str>) -> Option<String> {
    stepped_track_id(songs, current, true)
}

/// The id of the song to play when Previous is pressed: one before `current`
/// in `songs`, wrapping from the start back to the end. `None` when there is
/// nothing to step through (empty `songs`); with no `current` (or an unknown
/// one), the last song.
pub fn previous_track_id(songs: &[Song], current: Option<&str>) -> Option<String> {
    stepped_track_id(songs, current, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::stepping_songs;

    #[test]
    fn empty_list_is_a_noop_for_both_directions() {
        assert_eq!(next_track_id(&[], Some("song-1")), None);
        assert_eq!(previous_track_id(&[], Some("song-1")), None);
    }

    #[test]
    fn next_without_current_starts_at_first() {
        assert_eq!(
            next_track_id(&stepping_songs(), None),
            Some("song-1".to_string())
        );
    }

    #[test]
    fn previous_without_current_starts_at_last() {
        assert_eq!(
            previous_track_id(&stepping_songs(), None),
            Some("song-3".to_string())
        );
    }

    #[test]
    fn next_with_unknown_current_starts_at_first() {
        assert_eq!(
            next_track_id(&stepping_songs(), Some("nope")),
            Some("song-1".to_string())
        );
    }

    #[test]
    fn previous_with_unknown_current_starts_at_last() {
        assert_eq!(
            previous_track_id(&stepping_songs(), Some("nope")),
            Some("song-3".to_string())
        );
    }

    #[test]
    fn next_advances_through_the_list() {
        assert_eq!(
            next_track_id(&stepping_songs(), Some("song-1")),
            Some("song-2".to_string())
        );
        assert_eq!(
            next_track_id(&stepping_songs(), Some("song-2")),
            Some("song-3".to_string())
        );
    }

    #[test]
    fn previous_reverses_through_the_list() {
        assert_eq!(
            previous_track_id(&stepping_songs(), Some("song-2")),
            Some("song-1".to_string())
        );
    }

    #[test]
    fn next_wraps_from_last_to_first() {
        assert_eq!(
            next_track_id(&stepping_songs(), Some("song-3")),
            Some("song-1".to_string())
        );
    }

    #[test]
    fn previous_wraps_from_first_to_last() {
        assert_eq!(
            previous_track_id(&stepping_songs(), Some("song-1")),
            Some("song-3".to_string())
        );
    }

    #[test]
    fn single_song_steps_to_itself_in_both_directions() {
        let songs = vec![Song {
            id: "song-1".to_string(),
            title: "Only".to_string(),
            album_id: "album-1".to_string(),
        }];
        assert_eq!(
            next_track_id(&songs, Some("song-1")),
            Some("song-1".to_string())
        );
        assert_eq!(
            previous_track_id(&songs, Some("song-1")),
            Some("song-1".to_string())
        );
    }
}
