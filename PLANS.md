# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the usage reference: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. The README's detailed Usage reference moved to
> `docs/usage.md` on 2026-10-26, so plans that change usage prose edit that file, not the README.
> Record any new drift here rather than appending another note.

## Planned

_None yet._

### Mark a song-list footer total that skips unknown lengths with `+`, like Winamp (planned 2026-10-27 by the plan role)

**Goal.** Winamp's playlist editor adds a `+` after a time total when one or more of the
entries it covers has no known length (`3:30+`), so the total does not look exact when it
isn't. Today `song_list_summary` sums `duration_ms` and treats `0` ("the source supplied no
duration", per `Song::duration_ms`) as zero time without saying so. `[210_000, 0]` therefore
reads `2 songs - 3:30`, and a selected unknown-length row reads `0:00` as if it really were zero.

**Approach.** Make the change in `song_list_summary` (`src/ui/views/browse/rows/text.rs`) only.
It is a pure helper, and `views::browse::rows::song_list` already passes its result to the footer.
- Total: append `+` to `format_minutes_seconds(total_ms)` exactly when some song in `songs`
  has `duration_ms == 0` (`songs.iter().any(|song| song.duration_ms == 0)`). The song count
  is unchanged, so every entry still counts.
- Selected part: when the selected song's `duration_ms == 0`, it reads `0:00+` (Winamp's
  selection total is marked the same way for the same reason). A known selected length is
  unchanged.
- An empty list has no unknown entries and still reads `0 songs - 0:00`, so
  `an_empty_browse_level_draws_no_count_footer` (`src/ui/tests/view_tests.rs`) is
  unaffected. Do not touch `format_minutes_seconds`, `format_track_time`, album row totals
  (`--:--`), or the compact transport readouts.
- Update the helper's doc comment, which currently says "a zero-length selection reads
  `0:00`".

**Files touched.** `src/ui/views/browse/rows/text.rs`, `src/ui/views/tests/browse.rs`,
`docs/usage.md` (the footer sentence in the browse-list paragraph, after the
`2 songs - 3:30/4:15` example), `docs/invariants.md` (the `## Song-list footer` section's
Guarantee and Pinned-by list).

**Acceptance criteria.**
- In `src/ui/views/tests/browse.rs`, rename `song_list_summary_counts_unknown_durations_without_adding_time` to
  `song_list_summary_marks_a_total_that_skips_unknown_durations`, asserting
  `song_list_summary(&[summary_song(210_000), summary_song(0)], None) == "2 songs - 3:30+"`
  and an all-unknown `[summary_song(0)]` reads `"1 song - 0:00+"`.
- Rename `song_list_summary_reads_a_zero_length_selection_as_zero` to
  `song_list_summary_marks_an_unknown_selected_length`, asserting
  `[210_000, 0]` with `Some(1)` reads `"2 songs - 0:00+/3:30+"` and with `Some(0)` reads
  `"2 songs - 3:30/3:30+"`.
- The other existing `song_list_summary_*` tests (all-known durations, empty list, selection
  past the list, saturation, sub-second truncation) pass unchanged.
- docs/usage.md and the invariants section describe the `+` marker and name the renamed
  tests. `make check` passes.

### Let Shuffle play every song once before repeating any, like Winamp (planned 2026-10-27 by the plan role)

**Goal.** Winamp's Shuffle plays the playlist in a shuffled order, so every song plays once
before any song plays again. Today a shuffled forward step (`stepping::forward_track_id`, used by
Next in `step_track` and by the preview-end advance in `handlers/playback/preview.rs`) calls
`transport::shuffled_track_id`, which picks any song except the current one. In a three-song list
it can go song-1, song-2, song-1 and never reach song-3. Fix this by tracking a shuffle *round*:
a shuffled forward step picks only from the showing list's songs not yet played this round.
Once every song in the list has played, a new round starts.

**Approach.**
- Add `src/ui/transport/round.rs` with `pub struct ShuffleRound { played: HashSet<String> }`
  (`#[derive(Debug, Default)]`), declared beside `mod history;` in `src/ui/transport.rs` and
  re-exported like `PlayHistory`. It has two methods:
  - `record(&mut self, track_id: &str, songs: &[Song])` inserts `track_id`, then drops `played`
    ids that `songs` no longer contains (so the set stays bounded by the showing list). If every
    song in a non-empty `songs` is now in `played`, the round is complete: clear `played` and
    re-insert just `track_id`, so the new round starts with the song now playing.
  - `pick(&self, songs: &[Song], current: Option<&str>, roll: u64) -> Option<String>` (`#[must_use]`)
    takes candidates in list order: songs that are neither `current` nor in `played`. If any exist,
    return `candidates[roll % len]`. Otherwise fall back to
    `transport::shuffled_track_id(songs, current, roll)`, so behavior with no unplayed song left
    (for example a one-song list, or a list whose plays were never recorded) is unchanged.
- Add `pub(in crate::ui) shuffle_round: transport::ShuffleRound` to `WinampPlayer` in
  `src/ui/player.rs`, defaulted where `play_history` is.
- `handlers::playback::track_played` (src/ui/handlers/playback.rs): in the same committed branch
  that calls `player.play_history.record(current)`, also call `player.shuffle_round.record(current,
  items)`. Here `items` is the list the current view shows (`all_songs.items` for
  `CurrentView::AllSongs`, otherwise `songs.items`). Pick it through the disjoint fields as
  `step_track` does, not through `showing_songs(player)`, so that the mutable borrow of
  `shuffle_round` does not conflict. Record every committed play, whether or not Shuffle is on: a
  song played in order before Shuffle was switched on has still been heard this round. Failed and
  superseded completions record nothing, as for the history.
- `stepping::forward_track_id` gains a `round: &transport::ShuffleRound` parameter. Its shuffle
  branch calls `round.pick(items, current, transport::shuffle_roll())`. Update its doc comment and
  both callers (`step_track` passes `&player.shuffle_round`, and the preview-end advance in
  `preview.rs` does the same next to `showing_songs(player)`; both borrows are shared). The
  preview-end advance's existing same-song filter stays as it is.
- Out of scope: Shuffle's Previous (`PlayHistory`) is unchanged, and so is Shuffle with Repeat
  off, which still keeps going into a new round rather than stopping (the documented "a shuffled
  song when another exists").

**Files touched.** `src/ui/transport/round.rs` (new, with its in-file `#[cfg(test)] mod tests`
like `history.rs`), `src/ui/transport.rs`, `src/ui/player.rs`, `src/ui/handlers/playback.rs`,
`src/ui/handlers/playback/stepping.rs`, `src/ui/handlers/playback/preview.rs`,
`src/ui/tests/stepping_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- Unit tests in `round.rs` over `test_support::stepping_songs()` (song-1..song-3):
  - `pick_skips_songs_played_this_round`: after `record("song-1")` and `record("song-2")`,
    `pick(.., Some("song-2"), roll)` is `Some("song-3")` for every roll in `0..10`.
  - `a_completed_round_starts_over_from_the_current_song`: after recording all three songs ending
    with song-3, `pick(.., Some("song-3"), roll)` over `0..10` returns both song-1 and song-2 and
    never song-3.
  - `pick_falls_back_when_no_unplayed_song_remains`: a one-song list with that song recorded and
    current returns that song's id, matching `shuffled_track_id`.
  - `record_forgets_songs_the_list_no_longer_shows`: a song recorded from a different list does
    not keep a song of the current list from being picked, and does not count toward completing
    its round.
- UI test `next_with_shuffle_plays_the_unplayed_song` in `src/ui/tests/stepping_tests.rs`: build
  the player with `player_stepping_from(None)`, turn Shuffle on, and `record_a_play` song-1 then
  song-2. `Message::NextTrack` must then select index 2, checked with `assert_track_selected`.
- A preview-end test alongside the existing Shuffle preview-end tests in `stepping_tests.rs`. It
  records song-1 and song-2 as plays, then a finished reply while song-2 plays with Shuffle on.
  The advance must land on song-3 (index 2).
- `next_with_shuffle_never_reselects_the_current_track` and the existing Shuffle/Previous
  tests pass unchanged.
- docs/usage.md's Shuffle sentences (the preview-end advance paragraph and Next's description)
  say Shuffle plays each song of the showing list once before any repeats, like Winamp. A new
  `## Shuffle rounds` section in docs/invariants.md states the guarantee, names `ShuffleRound`
  and `track_played` as the enforcement, and lists the new tests. `make check` passes.

## Done
