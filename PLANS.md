# Plans

## Planned

### Extract the browse row-pitch tests from screen_tests.rs into a focused row_pitch_tests.rs module (planned 2026-11-20 by the plan role) — done 2026-11-20 by the feature role

**Needs review 2026-11-21 by feature: plan defect — already implemented in merged commit 9cedae34; `src/ui/tests/row_pitch_tests.rs` contains the helpers and four tests, and the invariant pins target the new module.**

**Goal.** `src/ui/tests/screen_tests.rs` (1350 lines) still holds the four
browse row-pitch tests and their two local helpers in the middle of general
root-view layout tests. They pin one concern — the browse list's actual row
pitch equals `BROWSE_ROW_HEIGHT`, and title line breaks do not change it —
so give them a focused module and keep shrinking `screen_tests.rs`. This is
the next cohesive block after the file-info rendering move and is independent
of the in-flight toggle extraction in `settings_tests.rs`.

**Approach and files.** Move this contiguous block verbatim, including its
leading comments, from `src/ui/tests/screen_tests.rs` into a new
`src/ui/tests/row_pitch_tests.rs` that opens with `use super::*;`:

- the private helpers `capture_songs_list_with_first_title` and
  `assert_first_song_row_pitch`
- `browse_row_height_matches_the_laid_out_row_pitch`
- `a_hard_line_break_in_a_title_does_not_change_the_row_pitch`
- `a_carriage_return_in_a_title_does_not_change_the_row_pitch`
- `a_crlf_in_a_title_does_not_change_the_row_pitch`

The block sits between `browse_open_button_sits_above_the_browse_list` and
`transport_row_keeps_play_and_pause_glyphs_fixed_across_playback_states`;
every other `screen_tests.rs` test stays. Register `mod row_pitch_tests;`
alphabetically in `src/ui/tests/mod.rs` (after `mod reverse_tests;`). No
support move: the helpers call only `player_stepping_from`,
`captured_root_view_text_bounds`, `layout_capture`, `CurrentView`,
`display_title`, and `views::BROWSE_ROW_HEIGHT`, all already reachable from
any child of `src/ui/tests`.

In `docs/invariants.md`, retag the six Pinned-by lines that name moved tests
under `## Browse playing-row visibility` and `## Browse row caching` — the
three entries for `browse_row_height_matches_the_laid_out_row_pitch`,
`a_hard_line_break_in_a_title_does_not_change_the_row_pitch`, and
`a_carriage_return_in_a_title_does_not_change_the_row_pitch` — from
`src/ui/tests/screen_tests.rs` to `src/ui/tests/row_pitch_tests.rs`.
`a_crlf_in_a_title_does_not_change_the_row_pitch`
is not pinned. No other Pinned-by entry changes; the queued file-info move
edits different sections of the same file, and the in-flight toggle
extraction edits the settings and shade sections, so the changes merge in
any order.

**Dedupe and scope.** `src/ui/tests/row_pitch_tests.rs` does not exist, and all
four named tests plus both helpers currently live in `screen_tests.rs`.
Searches of PLANS.md, PLANS-DONE.md, and the pending plans found earlier
extractions from `screen_tests.rs` (count footer, footer time, and the queued
File info move) but none for this block. This is one test-only move of
roughly 100 lines across four files; production behavior, fixtures,
assertions, helper signatures, `docs/usage.md`, and screenshots are unchanged.

**Acceptance criteria.**

- `src/ui/tests/row_pitch_tests.rs` defines the two helpers and the four
  named tests with their original comments, inputs, and assertions, and
  `src/ui/tests/screen_tests.rs` no longer defines any of them; its remaining
  tests retain their bodies.
- Test discovery lists each moved test once under
  `ui::tests::row_pitch_tests` with its original leaf name, and
  `make test-one TEST=row_pitch` runs all four from the new home; the
  long-title case still asserts the title stays left of the `3:30` label, the
  `\n`, `\r`, and `\r\n` cases still assert one-space or two-space flattening
  as before, and every case asserts the first two rows one
  `BROWSE_ROW_HEIGHT` apart.
- Each of the six Pinned-by entries naming a moved test in
  `docs/invariants.md` resolves to `src/ui/tests/row_pitch_tests.rs`, and the
  invariants' guarantee wording is unchanged.
- `make check` passes, and no production code, support helper, or test
  assertion changes.

### Move the window-position and window-level tests from settings_tests.rs into window_tests.rs (planned 2026-11-20 by the plan role)

**Goal.** `src/ui/tests/settings_tests.rs` (848 lines) still holds eight tests
of one topic — the window's saved position, its launch clamping, and the
saved always-on-top level applied when the window id resolves — mixed in with
the sign-in-token, song-list, library-view, autoscroll, and equalizer-setting
tests. `src/ui/tests/window_tests.rs` (411 lines) already owns window
behavior: id resolution, title-bar actions, always-on-top, and the window
title. Moving these tests gives the window tests one home and continues
shrinking `settings_tests.rs`, independent of the queued `screen_tests.rs`
extractions and the pending AppState-test extraction.

**Approach and files.** Move these tests verbatim, assertions and inputs
unchanged, from `src/ui/tests/settings_tests.rs` to the end of
`src/ui/tests/window_tests.rs`, in three non-contiguous blocks:

- `window_id_resolved_applies_a_saved_always_on_top_level`, between
  `a_sessionless_run_saves_no_token_line` and the
  `a_fresh_artists_reply_drops_the_saved_sort_permutation` comment;
- `window_moved_reaches_the_saved_snapshot` and
  `saved_window_position_is_restored_on_boot`, contiguous after
  `a_fresh_artists_reply_drops_the_saved_sort_permutation` and before
  `saved_autoscroll_choice_is_restored_on_boot`;
- `launch_position_clamps_the_saved_point_or_uses_the_default` through
  `clamp_into_monitor_pins_a_monitor_smaller_than_the_window_at_zero` (five
  contiguous tests), after `toggle_autoscroll_reaches_the_saved_snapshot` and
  before the `saved_library_view` helper.

Leave `saved_autoscroll_choice_is_restored_on_boot`,
`toggle_autoscroll_reaches_the_saved_snapshot`, the library-view tests, and
every other `settings_tests.rs` test in place. No module registration changes:
`src/ui/tests/mod.rs` already lists `mod window_tests;`, and both files open
with `use super::*;`. No helper moves: the moved tests use only `test_player`,
`player_with_window_id`, `assert_message_schedules_work`,
`assert_message_schedules_no_work`, `MemorySettingsStore`, `Settings`, and
`update`, all reachable from any child of `src/ui/tests`.

In `docs/invariants.md`, retag the eight Pinned-by entries in the
`## Settings persistence` list that name moved tests from
`src/ui/tests/settings_tests.rs` to `src/ui/tests/window_tests.rs`: the seven
window-position entries (`window_moved_reaches_the_saved_snapshot`,
`saved_window_position_is_restored_on_boot`,
`launch_position_clamps_the_saved_point_or_uses_the_default`, and the four
`clamp_into_monitor_*` tests) plus
`window_id_resolved_applies_a_saved_always_on_top_level`. A pending
invariants rewrite may reshape the same list, so resolve each moved name's
real home by grepping its `fn` at merge time rather than relying on its
position; entries naming tests that stay in `settings_tests.rs` keep their
path. The guarantee wording and every other invariant section are unchanged.

**Dedupe and scope.** `window_tests.rs` defines none of these eight names,
and all eight currently live in `settings_tests.rs`. No PLANS.md entry, no
`PLANS-DONE.md` entry, and no pending plan covers moving the window-position
or always-on-top tests; the landed toggle extraction lists "window position
and clamping" as out of scope. The pending plans touch `src/state/tests.rs`,
`src/ui/tests/screen_tests.rs`, and `src/ui/tests/transport_tests.rs`, so this
move shares no file or section with them. Roughly 130 test lines across three
files; production code, fixtures, assertions, `docs/usage.md`, and
screenshots are unchanged.

**Acceptance criteria.**

- `src/ui/tests/window_tests.rs` defines all eight named tests with their
  original bodies, and `src/ui/tests/settings_tests.rs` no longer defines any
  of them; every remaining test in that file keeps its body.
- Test discovery lists each moved test once under `ui::tests::window_tests`
  with its original leaf name, and `make test-one TEST=window_moved`,
  `make test-one TEST=saved_window_position`,
  `make test-one TEST=launch_position_clamps`,
  `make test-one TEST=clamp_into_monitor`, and
  `make test-one TEST=window_id_resolved_applies` each run the moved test
  from the new home.
- Behavior is unchanged: a `WindowMoved(300, 200)` writes one snapshot whose
  position is `Some((300.0, 200.0))` and a repeated move writes no second
  snapshot; a saved `(10.0, -20.0)` is restored and a missing position restores
  `None`; `launch_position(Some(..))` yields
  `iced::window::Position::SpecificWith` and `launch_position(None)` yields
  `Position::Default`; the clamp keeps `(40, 60)` in bounds, snaps
  `(2000, 60)` to `(1080, 60)` and `(40, 2000)` to `(40, 620)` for a 1280×720
  monitor and 200×100 window, snaps `(-50, -1)` to the origin, and pins a
  1280×720 window on an 800×600 monitor at the origin; and resolving a window
  id schedules work while always-on-top is set and none while it is clear,
  storing the id in both cases.
- Every Pinned-by entry naming a moved test in `docs/invariants.md` resolves
  to `src/ui/tests/window_tests.rs`; no other entry's path or the section's
  guarantee wording changes.
- `make check` passes, and no production code, support helper, assertion, or
  helper signature changes.

## Done
