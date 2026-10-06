//! Pure transport sequencing: which song the Previous/Next buttons land on.
//!
//! The transport buttons are a player concern — they step through the
//! currently loaded album's songs — so the index logic lives here as pure
//! functions, mirroring `views.rs`, and knows nothing about `WinampPlayer` or
//! the update loop. `AppleMusicService` stays the library API seam and keeps
//! its `next_track`/`previous_track` stubs untouched.

use crate::library::Song;

/// The id of the song to play when Next is pressed: one past `current` in
/// `songs`, wrapping from the end back to the start. `None` when there is
/// nothing to step through (empty `songs`); with no `current` (or an unknown
/// one), the first song.
pub fn next_track_id(songs: &[Song], current: Option<&str>) -> Option<String> {
    let Some(first) = songs.first() else {
        return None;
    };
    let index = current.and_then(|current_id| songs.iter().position(|song| song.id == current_id));
    match index {
        // No current song (or one not in the list): start the album over.
        None => Some(first.id.clone()),
        // One past the current song, wrapping from the end back to the start.
        Some(i) => Some(songs[(i + 1) % songs.len()].id.clone()),
    }
}

/// The id of the song to play when Previous is pressed: one before `current`
/// in `songs`, wrapping from the start back to the end. `None` when there is
/// nothing to step through (empty `songs`); with no `current` (or an unknown
/// one), the last song.
pub fn previous_track_id(songs: &[Song], current: Option<&str>) -> Option<String> {
    let Some(last) = songs.last() else {
        return None;
    };
    let index = current.and_then(|current_id| songs.iter().position(|song| song.id == current_id));
    match index {
        // No current song (or one not in the list): land on the last song.
        None => Some(last.id.clone()),
        // One before the current song, wrapping from the start back to the end.
        Some(i) => Some(songs[(i + songs.len() - 1) % songs.len()].id.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_songs() -> Vec<Song> {
        vec![
            Song {
                id: "song-1".to_string(),
                title: "One".to_string(),
                album_id: "album-1".to_string(),
            },
            Song {
                id: "song-2".to_string(),
                title: "Two".to_string(),
                album_id: "album-1".to_string(),
            },
            Song {
                id: "song-3".to_string(),
                title: "Three".to_string(),
                album_id: "album-1".to_string(),
            },
        ]
    }

    #[test]
    fn empty_list_is_a_noop_for_both_directions() {
        assert_eq!(next_track_id(&[], Some("song-1")), None);
        assert_eq!(previous_track_id(&[], Some("song-1")), None);
    }

    #[test]
    fn next_without_current_starts_at_first() {
        assert_eq!(
            next_track_id(&sample_songs(), None),
            Some("song-1".to_string())
        );
    }

    #[test]
    fn previous_without_current_starts_at_last() {
        assert_eq!(
            previous_track_id(&sample_songs(), None),
            Some("song-3".to_string())
        );
    }

    #[test]
    fn next_with_unknown_current_starts_at_first() {
        assert_eq!(
            next_track_id(&sample_songs(), Some("nope")),
            Some("song-1".to_string())
        );
    }

    #[test]
    fn previous_with_unknown_current_starts_at_last() {
        assert_eq!(
            previous_track_id(&sample_songs(), Some("nope")),
            Some("song-3".to_string())
        );
    }

    #[test]
    fn next_advances_through_the_list() {
        assert_eq!(
            next_track_id(&sample_songs(), Some("song-1")),
            Some("song-2".to_string())
        );
        assert_eq!(
            next_track_id(&sample_songs(), Some("song-2")),
            Some("song-3".to_string())
        );
    }

    #[test]
    fn previous_reverses_through_the_list() {
        assert_eq!(
            previous_track_id(&sample_songs(), Some("song-2")),
            Some("song-1".to_string())
        );
    }

    #[test]
    fn next_wraps_from_last_to_first() {
        assert_eq!(
            next_track_id(&sample_songs(), Some("song-3")),
            Some("song-1".to_string())
        );
    }

    #[test]
    fn previous_wraps_from_first_to_last() {
        assert_eq!(
            previous_track_id(&sample_songs(), Some("song-1")),
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
