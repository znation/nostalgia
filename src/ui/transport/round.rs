//! Shuffle's round: the songs already played, so a shuffled forward step
//! plays every song of the showing list once before any repeats.

use std::collections::HashSet;

use crate::library::Song;

use super::shuffled_track_id;

/// The ids of the songs played in the current Shuffle round.
///
/// [`Self::record`] adds each committed play and [`Self::pick`] chooses among
/// the showing list's songs not yet played. Once every song has played, the
/// round restarts from the song now playing.
#[derive(Debug, Default)]
pub struct ShuffleRound {
    played: HashSet<String>,
}

impl ShuffleRound {
    /// Records `track_id` as played this round.
    ///
    /// Ids that `songs` no longer contains are dropped, so the set stays
    /// bounded by the showing list. When every song of a non-empty `songs`
    /// has now played, the round is complete: it restarts holding only
    /// `track_id`.
    pub fn record(&mut self, track_id: &str, songs: &[Song]) {
        self.played.insert(track_id.to_owned());
        self.played
            .retain(|id| songs.iter().any(|song| &song.id == id));
        if !songs.is_empty() && songs.iter().all(|song| self.played.contains(&song.id)) {
            self.played.clear();
            self.played.insert(track_id.to_owned());
        }
    }

    /// The song a shuffled forward step selects: `roll` picks among the songs
    /// of `songs` that are neither `current` nor played this round. With none
    /// left it falls back to [`shuffled_track_id`].
    #[must_use]
    pub fn pick(&self, songs: &[Song], current: Option<&str>, roll: u64) -> Option<String> {
        let candidates: Vec<&Song> = songs
            .iter()
            .filter(|song| Some(song.id.as_str()) != current && !self.played.contains(&song.id))
            .collect();
        if candidates.is_empty() {
            return shuffled_track_id(songs, current, roll);
        }
        let len = u64::try_from(candidates.len()).ok()?;
        let index = usize::try_from(roll % len).ok()?;
        candidates.get(index).map(|song| song.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::stepping_songs;

    #[test]
    fn pick_skips_songs_played_this_round() {
        let songs = stepping_songs();
        let mut round = ShuffleRound::default();
        round.record("song-1", &songs);
        round.record("song-2", &songs);
        for roll in 0..10 {
            assert_eq!(
                round.pick(&songs, Some("song-2"), roll).as_deref(),
                Some("song-3")
            );
        }
    }

    #[test]
    fn a_completed_round_starts_over_from_the_current_song() {
        let songs = stepping_songs();
        let mut round = ShuffleRound::default();
        for id in ["song-1", "song-2", "song-3"] {
            round.record(id, &songs);
        }
        let picks: HashSet<String> = (0..10)
            .filter_map(|roll| round.pick(&songs, Some("song-3"), roll))
            .collect();
        assert!(picks.contains("song-1"));
        assert!(picks.contains("song-2"));
        assert!(!picks.contains("song-3"));
    }

    #[test]
    fn pick_falls_back_when_no_unplayed_song_remains() {
        let songs: Vec<Song> = stepping_songs().into_iter().take(1).collect();
        let mut round = ShuffleRound::default();
        round.record("song-1", &songs);
        assert_eq!(
            round.pick(&songs, Some("song-1"), 3),
            shuffled_track_id(&songs, Some("song-1"), 3)
        );
    }

    #[test]
    fn record_forgets_songs_the_list_no_longer_shows() {
        let songs = stepping_songs();
        let other: Vec<Song> = songs.iter().skip(1).cloned().collect();
        let mut round = ShuffleRound::default();
        round.record("song-1", &songs);
        // song-1 is not in `other`, so it is forgotten and cannot complete
        // the round there.
        round.record("song-2", &other);
        assert!(!round.played.contains("song-1"));
        assert_eq!(round.played.len(), 1);
        assert_eq!(
            round.pick(&songs, Some("song-2"), 0).as_deref(),
            Some("song-1")
        );
    }
}
