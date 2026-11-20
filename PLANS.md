# Plans

## Planned

_None yet._

### Extract the file-info and current-track tests into a focused module (planned 2026-11-20 by the plan role)

**Goal.** Give the file-info track resolution and the launch-time
current-track restore their own test home, continuing the shrink of
`src/ui/tests/readouts_tests.rs` (1161 lines after the seek-preview and
save-panel extractions). Checked 2026-11-20: the five tests below are still
defined there as one contiguous block at the end of the file;
`window_tests.rs` and `screen_tests.rs` hold the file-info panel's visibility
and rendering tests but define none of these resolution tests; no finished or
pending plan covers this extraction (PLANS-DONE.md greps clean for
`file_info_tests` and this test group), and it shares no file with the
pending spectrum extraction beyond `readouts_tests.rs`, `mod.rs`, and
`docs/invariants.md` — different regions of each, so the two merge
independently in either order.

**Approach and files.** Move these complete tests, preserving their
comments, inputs, and assertions, into a new
`src/ui/tests/file_info_tests.rs` with `use super::*;` (the shape of the
pending `spectrum_tests.rs` plan and the landed
`save_preset_panel_tests.rs`):

- `file_info_track_resolves_the_selected_song_row`
- `file_info_track_falls_back_to_the_current_track_without_a_selection`
- `current_track_resolved_records_the_restored_track`
- `current_track_resolved_skips_a_stale_or_failed_resolution`
- `resolve_current_track_skips_the_work_an_already_known_track_needs`

Register the sibling in `src/ui/tests/mod.rs` beside `readouts_tests` and
remove the old definitions. No helper move is needed: every helper the moved
tests use already reaches a sibling module through the existing chains —
`test_player`, `known_track`, `seed_current_title`, and `assert_no_task` are
`pub(super)` in `support.rs`; `stepping_songs` comes from
`crate::test_support` via `mod.rs`; `update`, `Message`, `PlaybackFormat`,
`CurrentView`, and `views::format_track_time` reach the new module through
the `use super::*;` glob the same way they reach `readouts_tests.rs`; and the
last test's direct `super::handlers::playback::resolve::resolve_current_track`
call resolves identically from any child of `src/ui/tests`
(`readouts_tests.rs` already proves the path). Update
`docs/invariants.md`'s `## File info panel` Pinned-by list: its trailing
parenthetical `(`src/ui/tests/readouts_tests.rs`)` currently covers both the
six `known_tracks_record_the_played_songs_*` lines (which stay in
`readouts_tests.rs`) and the two `file_info_track_*` lines; after the move,
per the L5 lesson, the last kept line (`known_tracks_record_the_played_songs_content_rating`)
gains an explicit `(`src/ui/tests/readouts_tests.rs`)` so it does not inherit
the moved tests' new attribution, and the two `file_info_track_*` lines gain
`(`src/ui/tests/file_info_tests.rs`)`. No other invariant section pins these
five tests — the mention of `resolve_current_track` and `current_track_resolved`
in `## Settings persistence` is body text naming the handler and message,
not a Pinned-by entry, and needs no edit. This is one cohesive test
relocation; it changes no player behavior and requires no usage or
screenshot update.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::file_info_tests`, with no copy under
  `ui::tests::readouts_tests`.
- With the sample library loaded, `WinampPlayer::file_info_track` resolves
  the selected song row ahead of the current track — a selected row with a
  year set shows that year and an empty `number`, and a selection still wins
  when a current track with a `known_tracks` entry is also loaded; without a
  song-row selection (a group-list selection, or none at all) it falls back
  to the current track's `known_tracks` entry with the loaded
  `PlaybackFormat` attached.
- A `Message::CurrentTrackResolved` reply records the restored song's title,
  artist, album, year, genre, and formatted time with a blank `number`, and
  leaves the player stopped with the `current_track` id unchanged; a reply
  carrying no song records nothing; a reply for an id the state no longer
  points at is dropped, leaving the new selection's play-path entry and its
  `"1. "` number prefix standing.
- Resolving a current track whose id is already in `known_tracks`, and
  resolving with no current track at all, schedules no follow-up work
  (no task).
- The diff contains only test relocation, module registration, and
  invariant-attribution updates; no moved test's inputs or assertions
  change.

### Extract the position-tick tests into a focused module (planned 2026-11-20 by the plan role)

**Goal.** Give the position tick's query scheduling, lazy format/duration
fetch, finished-state query, and paused-blink phase their own test home,
continuing the shrink of `src/ui/tests/readouts_tests.rs` (1030 lines after
the spectrum merge). Checked 2026-11-20: the seven `position_tick_*` tests
plus the helpers `tick_replies` and `assert_position_queried` form one
contiguous block (`fn tick_replies` through
`position_tick_queries_only_the_missing_value`'s closing brace), and the
helpers have no callers outside that block. Every helper the block calls
(`test_player`, `player_with_recording`, `player_with_audio`,
`task_outputs`) is `pub(super)` in `src/ui/tests/support.rs` or reachable
via `super::*`/`crate::test_support`, so nothing else moves. Grep on
PLANS-DONE.md and PLANS.md finds no finished or pending plan covering a
`position_tick_tests` module. It shares `readouts_tests.rs`, `mod.rs`, and
docs/invariants.md with the waiting file-info extraction, but touches a
disjoint region of each (its block ends before the file-info tests; the
invariant edits land in the `## Now Playing label` and finished-state
sections, not `## File info panel`), so the two merge independently in
either order.

**Approach.**
- Create `src/ui/tests/position_tick_tests.rs` starting with `use super::*;`
  and move into it, unchanged, the block above: helpers `tick_replies` and
  `assert_position_queried`, and tests `position_tick_schedules_a_query`,
  `position_tick_queries_position_and_format`,
  `position_tick_queries_position_and_duration`,
  `position_tick_queries_preview_finished`,
  `position_tick_blinks_the_elapsed_readout_only_while_paused`,
  `position_tick_skips_the_format_and_length_once_both_are_known`, and
  `position_tick_queries_only_the_missing_value`.
- Register `mod position_tick_tests;` in `src/ui/tests/mod.rs` between
  `panels_tests` and `randomize_tests`.
- docs/invariants.md, `## Now Playing label`: the three moved tests that
  inherit the section default (`position_tick_schedules_a_query`,
  `position_tick_skips_the_format_and_length_once_both_are_known`,
  `position_tick_queries_only_the_missing_value`) each get an explicit
  `(`src/ui/tests/position_tick_tests.rs`)`, and the two that already carry
  explicit readouts paths (`position_tick_queries_position_and_duration`,
  `position_tick_queries_position_and_format`) are retagged. In the same
  run, five kept lines whose tests actually live in
  `src/ui/tests/subscriptions_tests.rs` but inherit the readouts default
  today (`position_tick_stream_emits_position_ticks`,
  `a_refused_position_thread_ends_the_tick_stream_instead_of_panicking`,
  `a_refused_position_thread_reports_the_cause_and_the_loss`,
  `the_position_thread_is_named_nostalgia_position_poll`,
  `the_position_timer_runs_only_while_there_is_a_position_to_refresh`)
  each get an explicit `(`src/ui/tests/subscriptions_tests.rs`)`, and the
  doubled `((...))` parentheses on `a_paused_preview_needs_the_position_poll_before_its_first_reply`
  are normalized to single. Per the plan-role lesson on parenthetical runs,
  check every kept neighbor around each edited line still resolves to its
  defining file.
- docs/invariants.md, finished-state section: retag
  `position_tick_queries_preview_finished` to the new path; and correct the
  `## Now Playing label` prose that places
  `position_tick_blinks_the_elapsed_readout_only_while_paused` "in
  `src/state.rs`" — the test is defined in the readouts test module today
  and in the new module after this change.
- The poll-side tests (`position_poll_waits_for_a_stop_to_reach_the_backend`,
  `position_poll_waits_for_a_selected_tracks_play_to_settle`,
  `an_unrelated_transport_completion_keeps_the_poll_waiting`,
  `position_replies_store_and_stale_ones_are_dropped`,
  `a_paused_preview_needs_the_position_poll_before_its_first_reply`) stay
  in `readouts_tests.rs`; they are a separate follow-up extraction.
- No production code changes.

**Files touched.** `src/ui/tests/readouts_tests.rs`,
`src/ui/tests/position_tick_tests.rs` (new), `src/ui/tests/mod.rs`,
docs/invariants.md.

**Acceptance criteria.**
- `make test-one TEST=position_tick` runs each of the seven moved tests,
  each listed once under `ui::tests::position_tick_tests`, with no copy
  under `ui::tests::readouts_tests`; grep finds `fn tick_replies` and
  `fn assert_position_queried` only in `position_tick_tests.rs`.
- `readouts_tests.rs` defines no `position_tick_*` test, and its kept
  neighbors around the removed block (`playback_duration_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`,
  `now_playing_artist_shows_the_played_tracks_artist_after_browsing_to_another_album`)
  still compile and pass, showing the removal left no dangling helper
  references.
- Every Pinned-by line in docs/invariants.md naming one of the seven moved
  tests carries `(`src/ui/tests/position_tick_tests.rs`)`; no Pinned-by
  line naming a moved test still names `readouts_tests.rs`; the five
  subscriptions-side tests in the same run carry explicit
  `(`src/ui/tests/subscriptions_tests.rs`)` parentheticals; and the
  `## Now Playing label` prose names the new module for the blink test
  instead of `src/state.rs`.
- The diff contains only test relocation, module registration, and
  invariant-attribution updates; no moved test's inputs or assertions
  change.

## Done

## Done
