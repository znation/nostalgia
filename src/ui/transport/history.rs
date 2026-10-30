//! Shuffle's play history: the tracks actually played, for Previous to retrace.

use crate::library::Song;

/// The maximum number of played tracks [`PlayHistory`] keeps. Older entries
/// fall off the front, so a long session still retraces its recent plays
/// without growing without bound.
const HISTORY_LIMIT: usize = 100;

/// The track a history-backed Previous jumped to but whose play has not
/// committed yet. `target` indexes [`PlayHistory`]'s entries; `from` is the
/// track that was current when the jump was issued, so repeated Previous
/// presses before the jump lands can keep walking back from the same place.
/// `retry` marks a play that failed after `play_track` had already named the
/// target as current, so the next Previous offers that same target again.
#[derive(Debug)]
struct PendingJump {
    target: usize,
    from: Option<String>,
    retry: bool,
}

/// The tracks actually played, in play order, for Shuffle's Previous.
///
/// [`Self::record`] appends each committed play and [`Self::previous`] walks
/// back from the cursor to the nearest recorded track the visible list still
/// shows. Between a Previous press and its play committing there is a
/// [`PendingJump`], so repeated presses keep walking back even though
/// `current` has not changed yet; a new track selected before the jump lands
/// supersedes it.
#[derive(Debug, Default)]
pub struct PlayHistory {
    entries: Vec<String>,
    cursor: Option<usize>,
    pending: Option<PendingJump>,
}

impl PlayHistory {
    /// Records `track_id` as the track whose playback committed.
    ///
    /// A repeated current track is a no-op. A pending jump whose target is
    /// `track_id` has landed: the cursor moves to that entry without pushing
    /// or truncating, so the tracks after it stay available until a new play
    /// truncates them. Any other play ends a pending jump, drops the forward
    /// entries past the cursor, and appends the new track; entries beyond
    /// [`HISTORY_LIMIT`] fall off the front with the cursor following them.
    pub fn record(&mut self, track_id: &str) {
        if let Some(target) = self.pending.as_ref().map(|jump| jump.target)
            && self.entries.get(target).map(String::as_str) == Some(track_id)
        {
            self.cursor = Some(target);
            self.pending = None;
            return;
        }
        self.pending = None;
        let cursor_entry = self
            .cursor
            .and_then(|cursor| self.entries.get(cursor))
            .map(String::as_str);
        if cursor_entry == Some(track_id) {
            return;
        }
        self.entries
            .truncate(self.cursor.map_or(0, |cursor| cursor + 1));
        self.entries.push(track_id.to_string());
        self.cursor = Some(self.entries.len() - 1);
        let excess = self.entries.len().saturating_sub(HISTORY_LIMIT);
        if excess > 0 {
            self.entries.drain(..excess);
            self.cursor = self.cursor.map(|cursor| cursor - excess);
        }
    }

    /// Keeps a pending jump whose play failed, so the next [`Self::previous`]
    /// returns the same target once more instead of walking past it. Shared
    /// state names the failed target (the backend rolls `is_playing` back but
    /// leaves `current_track` set), so a retry is the track the user just
    /// selected. A failed play with no pending jump is a no-op.
    pub fn retry_pending(&mut self) {
        if let Some(jump) = self.pending.as_mut() {
            jump.retry = true;
        }
    }

    /// Returns the id of the nearest recorded track before `current` that
    /// `songs` still contains, leaving the cursor on the last recorded track
    /// until the returned track's play commits, or `None` when no earlier
    /// usable entry exists.
    ///
    /// A pending jump stays in effect while `current` is absent, names the
    /// jump's target (its play committed but `record` has not run), or names
    /// the jump's `from` track (its play has not committed yet); in that case
    /// the scan starts strictly before the jump's target, so repeated presses
    /// keep walking back. Any other `current` supersedes the jump, and the
    /// scan resumes from the cursor. A jump marked by [`Self::retry_pending`]
    /// returns its target once more when `current` still names it, rather than
    /// walking past a track whose play failed. Entries absent from `songs` are
    /// skipped so a browse to another list lands on a row it can show. On a
    /// miss the cursor and any in-effect jump stay unchanged, letting the
    /// caller fall back to sequential Previous.
    pub fn previous(&mut self, songs: &[Song], current: Option<&str>) -> Option<String> {
        let retry_target = self.pending.as_ref().and_then(|jump| {
            let entry = self.entries.get(jump.target)?;
            (jump.retry
                && current == Some(entry.as_str())
                && songs.iter().any(|song| song.id == *entry))
            .then_some(jump.target)
        });
        let jump_in_effect = self.pending.as_ref().is_some_and(|jump| {
            let target = self.entries.get(jump.target).map(String::as_str);
            current.is_none() || current == target || current == jump.from.as_deref()
        });
        if self.pending.is_some() && !jump_in_effect {
            self.pending = None;
        }
        if retry_target.is_some()
            && let Some(jump) = self.pending.as_mut()
        {
            jump.retry = false;
            return Some(self.entries[jump.target].clone());
        }
        let (with_jump, start) = match &self.pending {
            Some(jump) if jump_in_effect => (true, jump.target),
            _ => {
                let cursor = self.cursor?;
                let entry = self.entries.get(cursor)?;
                let start = if Some(entry.as_str()) == current {
                    cursor
                } else {
                    cursor + 1
                };
                (false, start)
            }
        };
        let found = (0..start)
            .rev()
            .find(|&index| songs.iter().any(|song| song.id == self.entries[index]))?;
        let from = if with_jump {
            self.pending.as_ref().and_then(|jump| jump.from.clone())
        } else {
            current.map(str::to_string)
        };
        self.pending = Some(PendingJump {
            target: found,
            from,
            retry: false,
        });
        Some(self.entries[found].clone())
    }
}

/// The songs whose plays committed this run, in play order, for the
/// signed-out Recently Played list.
///
/// [`Self::record`] moves a row whose id is already listed to the back, so a
/// replay never duplicates a song, and drops the oldest rows past
/// [`HISTORY_LIMIT`], the same cap the play history keeps.
/// [`Self::newest_first`] returns the rows most recent first, the order the
/// Recently Played view shows. The list lives only for this run: it is not
/// persisted, so a relaunch starts empty and the view falls back to the
/// sample stand-in until a play commits.
#[derive(Debug, Default)]
pub(in crate::ui) struct PlayedSongs {
    items: Vec<Song>,
}

impl PlayedSongs {
    /// Records `song` as the song whose playback just committed: drops an
    /// earlier row with the same id and appends the new one at the back, so
    /// the oldest row falls off the front beyond [`HISTORY_LIMIT`].
    pub fn record(&mut self, song: Song) {
        self.items.retain(|row| row.id != song.id);
        self.items.push(song);
        let excess = self.items.len().saturating_sub(HISTORY_LIMIT);
        if excess > 0 {
            self.items.drain(..excess);
        }
    }

    /// Returns every recorded song, most recent first.
    pub fn newest_first(&self) -> Vec<Song> {
        self.items.iter().rev().cloned().collect()
    }

    /// Returns whether nothing has played (or been recorded) this run, so a
    /// signed-out Recently Played open still shows the sample stand-in.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::stepping_songs;

    /// A history with `ids` recorded in order, the starting shape most of the
    /// `PlayHistory` tests share.
    fn recorded(ids: &[&str]) -> PlayHistory {
        let mut history = PlayHistory::default();
        for id in ids {
            history.record(id);
        }
        history
    }

    #[test]
    fn record_keeps_play_order_and_ignores_a_repeated_current() {
        let mut history = recorded(&["song-1", "song-2"]);

        history.record("song-2");

        assert_eq!(
            history.entries,
            vec!["song-1".to_string(), "song-2".to_string()]
        );
        assert_eq!(history.cursor, Some(1));
    }

    // A jump target's own record lands on its entry without pushing a
    // duplicate or truncating the forward entries; the next new track then
    // drops those forward entries from the landed cursor.
    #[test]
    fn recording_a_jump_target_keeps_the_forward_entries_until_a_new_play() {
        let songs = stepping_songs();
        let mut history = recorded(&["song-1", "song-2", "song-3"]);

        assert_eq!(
            history.previous(&songs, Some("song-3")),
            Some("song-2".to_string())
        );
        history.record("song-2");
        assert_eq!(
            history.entries,
            vec![
                "song-1".to_string(),
                "song-2".to_string(),
                "song-3".to_string()
            ]
        );
        assert_eq!(history.cursor, Some(1));

        history.record("song-4");
        assert_eq!(
            history.entries,
            vec![
                "song-1".to_string(),
                "song-2".to_string(),
                "song-4".to_string()
            ]
        );
        assert_eq!(history.cursor, Some(2));
    }

    // The signed-out Recently Played list shares PlayHistory's cap: more
    // plays than HISTORY_LIMIT keeps exactly HISTORY_LIMIT rows, the oldest
    // falling off the front.
    #[test]
    fn played_songs_drops_the_oldest_past_the_limit() {
        let songs = stepping_songs();
        let mut played = PlayedSongs::default();
        for index in 0..HISTORY_LIMIT + 5 {
            let mut song = songs[0].clone();
            song.id = format!("song-{index}");
            played.record(song);
        }

        assert_eq!(played.items.len(), HISTORY_LIMIT);
        assert_eq!(played.items.first().unwrap().id, "song-5");
        assert_eq!(
            played.items.last().unwrap().id,
            format!("song-{}", HISTORY_LIMIT + 4)
        );
    }

    // Recording the same song again keeps one row for it, moved to the
    // newest position, so a replay never duplicates a row.
    #[test]
    fn played_songs_recording_a_song_again_moves_its_row_newest() {
        let songs = stepping_songs();
        let mut played = PlayedSongs::default();
        played.record(songs[0].clone());
        played.record(songs[1].clone());

        played.record(songs[0].clone());

        assert_eq!(
            played.items.iter().map(|song| &song.id).collect::<Vec<_>>(),
            vec![&"song-2".to_string(), &"song-1".to_string()]
        );
    }

    #[test]
    fn played_songs_newest_first_lists_rows_most_recent_first() {
        let songs = stepping_songs();
        let mut played = PlayedSongs::default();
        assert!(played.is_empty());
        assert!(played.newest_first().is_empty());

        played.record(songs[0].clone());
        played.record(songs[1].clone());
        played.record(songs[2].clone());

        assert!(!played.is_empty());
        assert_eq!(
            played
                .newest_first()
                .iter()
                .map(|song| song.id.as_str())
                .collect::<Vec<_>>(),
            vec!["song-3", "song-2", "song-1"]
        );
        // The recorded rows stay in play order behind the newest-first view.
        assert_eq!(
            played
                .items
                .iter()
                .map(|song| song.id.as_str())
                .collect::<Vec<_>>(),
            vec!["song-1", "song-2", "song-3"]
        );
    }

    #[test]
    fn history_drops_the_oldest_entries_past_the_limit_and_moves_the_cursor() {
        let mut history = PlayHistory::default();
        for index in 0..HISTORY_LIMIT + 5 {
            history.record(&format!("song-{index}"));
        }

        assert_eq!(history.entries.len(), HISTORY_LIMIT);
        assert_eq!(history.entries.first(), Some(&"song-5".to_string()));
        assert_eq!(
            history.entries.last(),
            Some(&format!("song-{}", HISTORY_LIMIT + 4))
        );
        assert_eq!(history.cursor, Some(HISTORY_LIMIT - 1));
    }

    #[test]
    fn previous_without_history_is_none_and_leaves_no_cursor() {
        let songs = stepping_songs();
        let mut history = PlayHistory::default();

        assert_eq!(history.previous(&songs, Some("song-1")), None);
        assert_eq!(history.cursor, None);
        assert!(history.pending.is_none());
    }

    // Repeated presses before the jump's play commits keep walking back from
    // the jump target. Recording the landed track moves the cursor to it
    // without disturbing the entries, and the oldest entry then returns
    // `None` without moving the cursor.
    #[test]
    fn previous_walks_back_on_repeated_calls_before_the_jump_commits() {
        let songs = stepping_songs();
        let mut history = recorded(&["song-1", "song-2", "song-3"]);

        assert_eq!(
            history.previous(&songs, Some("song-3")),
            Some("song-2".to_string())
        );
        assert_eq!(history.cursor, Some(2));
        assert_eq!(
            history.previous(&songs, Some("song-3")),
            Some("song-1".to_string())
        );

        history.record("song-1");
        assert_eq!(
            history.entries,
            vec![
                "song-1".to_string(),
                "song-2".to_string(),
                "song-3".to_string()
            ]
        );
        assert_eq!(history.cursor, Some(0));
        assert_eq!(history.previous(&songs, Some("song-1")), None);
        assert_eq!(history.cursor, Some(0));
        assert!(history.pending.is_none());
    }

    // A jump whose play fails must be retried: `play_track` has already
    // written the jump target to `current_track` when the backend rejects the
    // preview, so the next Previous returns that same target again instead of
    // stepping past it from the cursor.
    #[test]
    fn retrying_a_failed_jump_returns_its_target_again() {
        let songs = stepping_songs();
        let mut history = recorded(&["song-1", "song-2", "song-3"]);

        assert_eq!(
            history.previous(&songs, Some("song-3")),
            Some("song-2".to_string())
        );
        history.retry_pending();
        assert_eq!(
            history.previous(&songs, Some("song-2")),
            Some("song-2".to_string())
        );
        assert_eq!(history.cursor, Some(2));
        assert_eq!(
            history.previous(&songs, Some("song-2")),
            Some("song-1".to_string())
        );
    }

    // A jump whose play never lands is abandoned when `current` names another
    // track: the scan includes the last recorded entry rather than walking
    // past it from the jump's target.
    #[test]
    fn previous_supersedes_a_jump_when_current_names_a_new_unrecorded_track() {
        let songs = stepping_songs();
        let mut history = recorded(&["song-1", "song-2", "song-3"]);

        assert_eq!(
            history.previous(&songs, Some("song-3")),
            Some("song-2".to_string())
        );
        assert_eq!(
            history.previous(&songs, Some("song-4")),
            Some("song-3".to_string())
        );
        assert_eq!(history.cursor, Some(2));
    }

    // A new committed play also supersedes the pending jump; the next
    // Previous resumes from that new current rather than the abandoned jump.
    #[test]
    fn previous_after_a_recorded_new_track_resumes_from_that_track() {
        let songs = stepping_songs();
        let mut history = recorded(&["song-1", "song-2", "song-3"]);

        assert_eq!(
            history.previous(&songs, Some("song-3")),
            Some("song-2".to_string())
        );
        history.record("song-4");
        assert_eq!(
            history.previous(&songs, Some("song-4")),
            Some("song-3".to_string())
        );
    }

    // A recorded track the visible list no longer shows is skipped, so the
    // browse lands on a row the player can render.
    #[test]
    fn previous_skips_entries_absent_from_the_visible_list() {
        let visible: Vec<Song> = stepping_songs()
            .into_iter()
            .filter(|song| song.id != "song-2")
            .collect();
        let mut history = recorded(&["song-1", "song-2", "song-3"]);

        assert_eq!(
            history.previous(&visible, Some("song-3")),
            Some("song-1".to_string())
        );
        assert_eq!(history.cursor, Some(2));
    }

    // Walking back past the oldest entry finds nothing; the jump to that
    // oldest entry stays in effect, so repeated presses keep missing and its
    // play still lands on the entry when it commits.
    #[test]
    fn previous_at_the_oldest_entry_misses_and_keeps_the_jump() {
        let songs = stepping_songs();
        let mut history = recorded(&["song-1", "song-2"]);

        assert_eq!(
            history.previous(&songs, Some("song-2")),
            Some("song-1".to_string())
        );
        assert_eq!(history.previous(&songs, Some("song-2")), None);
        assert_eq!(history.previous(&songs, Some("song-1")), None);

        history.record("song-1");
        assert_eq!(history.cursor, Some(0));
        assert_eq!(history.entries.len(), 2);
    }

    // A failed play with no jump pending has nothing to retry: Previous walks
    // back from the cursor as usual.
    #[test]
    fn retry_pending_without_a_jump_changes_nothing() {
        let songs = stepping_songs();
        let mut history = recorded(&["song-1", "song-2", "song-3"]);

        history.retry_pending();

        assert_eq!(
            history.previous(&songs, Some("song-3")),
            Some("song-2".to_string())
        );
    }

    // A retry only re-offers a target the visible list still shows; one that
    // has since left the list is walked past to the next usable entry.
    #[test]
    fn retrying_a_jump_whose_target_left_the_list_walks_past_it() {
        let songs = stepping_songs();
        let visible: Vec<Song> = songs
            .iter()
            .filter(|song| song.id != "song-2")
            .cloned()
            .collect();
        let mut history = recorded(&["song-1", "song-2", "song-3"]);

        assert_eq!(
            history.previous(&songs, Some("song-3")),
            Some("song-2".to_string())
        );
        history.retry_pending();

        assert_eq!(
            history.previous(&visible, Some("song-2")),
            Some("song-1".to_string())
        );
    }
}
