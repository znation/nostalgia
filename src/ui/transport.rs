//! Pure transport sequencing: which song the Previous/Next buttons land on.
//!
//! The transport buttons are a player concern — they step through the
//! currently loaded album's songs — so the index logic lives here as pure
//! functions, mirroring `views`, and knows nothing about `WinampPlayer` or
//! the update loop. `AppleMusicService` stays the library API seam and keeps
//! its `next_track`/`previous_track` stubs untouched.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

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
///
/// `#[must_use]`: the returned id is the call's entire purpose, so a caller
/// that drops it has silently done nothing — the Next button would not step.
/// The attribute turns that mistake into a compile-time warning.
#[must_use]
pub fn next_track_id(songs: &[Song], current: Option<&str>, repeat: bool) -> Option<String> {
    stepped_track_id(songs, current, true, repeat)
}

/// The id of the song to play when Previous is pressed: one before `current`
/// in `songs`. With `repeat`, wraps from the start back to the end; without
/// it (the default), stays on the first song. `None` when there is nothing to
/// step through (empty `songs`); with no `current` (or an unknown one), the
/// last song regardless of `repeat`.
///
/// `#[must_use]`: the Previous twin of [`next_track_id`], guarded for the same
/// reason — a discarded id means the button silently did nothing.
#[must_use]
pub fn previous_track_id(songs: &[Song], current: Option<&str>, repeat: bool) -> Option<String> {
    stepped_track_id(songs, current, false, repeat)
}

/// The id of the song to play when Next is pressed with Shuffle on: a pick
/// from `songs` that is not `current`, so Next never re-lands on the song
/// already playing. `roll` selects among the candidates (taken modulo their
/// count), which keeps the function pure and testable — production passes a
/// fresh [`shuffle_roll`]. `None` when `songs` is empty; when every song is
/// the current one (a one-song album already current) there is no other song
/// to pick, so the current song's id is returned.
///
/// `#[must_use]`: the returned id is the call's entire purpose, so a caller
/// that drops it has silently done nothing — the Next button would not step.
#[must_use]
pub fn shuffled_track_id(songs: &[Song], current: Option<&str>, roll: u64) -> Option<String> {
    let candidates: Vec<&Song> = songs
        .iter()
        .filter(|song| Some(song.id.as_str()) != current)
        .collect();
    match candidates.len() {
        0 => songs.first().map(|song| song.id.clone()),
        count => Some(candidates[(roll % count as u64) as usize].id.clone()),
    }
}

/// The maximum number of played tracks [`PlayHistory`] keeps. Older entries
/// fall off the front, so a long session still retraces its recent plays
/// without growing without bound.
pub(super) const HISTORY_LIMIT: usize = 100;

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

/// A fresh random value for [`shuffled_track_id`]'s `roll`, drawn from
/// `std::collections::hash_map::RandomState` — std's per-thread random hash
/// seed, so no new dependency is needed. Called once per Shuffled Next press;
/// the candidate modulo in `shuffled_track_id` spreads the value over the
/// album.
///
/// `#[must_use]`: a dropped roll is a call that produced no randomness for
/// its caller, which is always a mistake.
#[must_use]
pub fn shuffle_roll() -> u64 {
    RandomState::new().build_hasher().finish()
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

    // Shuffle's core promise: Next never re-lands on the song already playing.
    // The sweep covers every residue the modulo can produce on the two
    // candidates (`song-1` and `song-3`), so a regression that picked from the
    // whole list rather than the non-current candidates — or off-by-one in the
    // modulo — would surface here.
    #[test]
    fn shuffled_pick_never_returns_the_current_song() {
        let songs = stepping_songs();
        for roll in 0..4 {
            let picked = shuffled_track_id(&songs, Some("song-2"), roll);
            assert!(
                matches!(picked.as_deref(), Some("song-1") | Some("song-3")),
                "roll {roll} re-landed on the current song: {picked:?}"
            );
        }
    }

    // Both candidates are reachable, one per roll: roll 0 lands on the first
    // non-current song and roll 1 on the second. This pins that the modulo
    // spans the candidate set rather than collapsing onto one song.
    #[test]
    fn shuffled_pick_visits_every_candidate() {
        let songs = stepping_songs();
        let picks: Vec<Option<String>> = (0..2)
            .map(|roll| shuffled_track_id(&songs, Some("song-2"), roll))
            .collect();
        assert_eq!(
            picks,
            vec![Some("song-1".to_string()), Some("song-3".to_string())]
        );
    }

    #[test]
    fn shuffled_pick_on_empty_list_is_none() {
        assert_eq!(shuffled_track_id(&[], Some("song-1"), 0), None);
    }

    // A one-song album already playing has no other song to pick, so Shuffle
    // returns that song rather than `None` — the same edge the sequential
    // stepping functions document for a one-element list.
    #[test]
    fn shuffled_pick_on_a_one_song_album_returns_that_song() {
        let songs = single_song_album();
        assert_eq!(
            shuffled_track_id(&songs, Some("song-1"), 7),
            Some("song-1".to_string())
        );
    }

    // With no current song every song is a candidate, so the sweep reaches all
    // three. This pins that the `current` filter does not accidentally drop a
    // song when there is nothing to exclude.
    #[test]
    fn shuffled_pick_without_current_can_reach_every_song() {
        let songs = stepping_songs();
        let mut reached: Vec<String> = (0..3)
            .filter_map(|roll| shuffled_track_id(&songs, None, roll))
            .collect();
        reached.sort();
        reached.dedup();
        assert_eq!(reached, vec!["song-1", "song-2", "song-3"]);
    }

    // Shuffle's pick is only as varied as the roll production hands it: every
    // test above injects its own `roll`, so `shuffle_roll` itself is never
    // called and a regression that returned a constant (or the same value on
    // every call) would leave them all green while Next re-picked the same
    // non-current song forever. Draw a handful of rolls and require more than
    // one distinct value; a constant fails here.
    #[test]
    fn shuffle_roll_varies_across_calls() {
        let first = shuffle_roll();
        let varied = (0..15).any(|_| shuffle_roll() != first);
        assert!(
            varied,
            "shuffle_roll returned {first} for sixteen calls, so Shuffle could not vary its pick"
        );
    }

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
}
