# Plans

## Planned

### Move the window-position and window-level tests from settings_tests.rs into window_tests.rs (planned 2026-11-20 by the plan role)

**Needs review 2026-11-21 by feature: plan defect — the deliverable is already on main in commit `903cdb02`: all eight named tests are defined in `src/ui/tests/window_tests.rs`, absent from `settings_tests.rs`, and their invariant pins name the new file.**

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

### Move the autoscroll persistence tests from settings_tests.rs into toggle_persist_tests.rs (planned 2026-11-21 by the plan role)

**Needs review 2026-11-21 by feature: plan defect — the deliverable is already on main in commit `6fe774be`: both named tests are defined in `src/ui/tests/toggle_persist_tests.rs`, absent from `settings_tests.rs`, and their invariant pins name the new file.**

**Goal.** `src/ui/tests/settings_tests.rs` (848 lines) still holds the two
autoscroll persistence tests in the middle of window-position and launch-clamp
tests. The focused `src/ui/tests/toggle_persist_tests.rs` module (196 lines)
already owns exactly this topic — one toggle/save pair and one restore pair per
persisted UI choice (`toggle_time_display_reaches_the_saved_snapshot` /
`saved_time_display_mode_is_restored_on_boot`, the same shape for the
visualizer, the panels, always-on-top, and shade) — so the autoscroll pair
belongs beside them. This follows the pending window-position move into
`window_tests.rs` (which shrinks the same file from the other end) and the
merged toggle extraction that created the receiving module.

**Approach and files.** Move these two complete tests, with their bodies and
messages unchanged, from `src/ui/tests/settings_tests.rs` into the existing
`src/ui/tests/toggle_persist_tests.rs`:

- `saved_autoscroll_choice_is_restored_on_boot`
- `toggle_autoscroll_reaches_the_saved_snapshot`

The pair is contiguous in `settings_tests.rs` (between
`saved_window_position_is_restored_on_boot` and
`launch_position_clamps_the_saved_point_or_uses_the_default`) and every other
`settings_tests.rs` test stays. Do not register a new module in
`src/ui/tests/mod.rs` — `toggle_persist_tests` is already listed. No helper
moves: the pair calls only `test_player`, `update`, `Message::ToggleAutoscroll`,
`MemorySettingsStore`, and `Settings`, all already used by the receiving
module's existing tests through its `use super::*;` preamble. Keep
`settings_tests.rs`'s preamble unchanged.

In `docs/invariants.md` `## Settings persistence`, retag the autoscroll
sentence's two test paths —
`saved_autoscroll_choice_is_restored_on_boot` and
`toggle_autoscroll_reaches_the_saved_snapshot` — from
`(src/ui/tests/settings_tests.rs)` to `(src/ui/tests/toggle_persist_tests.rs)`,
leaving `autoscroll_round_trips_and_ignores_malformed_values`
(`src/settings/tests.rs`) untouched. This plan and the pending window-position
plan retag different named tests in the same section; each implementation
greps the named tests' real home at merge time and retags only its own, so the
changes merge in either order.

**Dedupe and scope.** Greps of PLANS.md, PLANS-DONE.md, and the pending plans
found the toggle extraction that created `toggle_persist_tests.rs` and the
`toggle_autoscroll` handler plan it already shipped, but no move of this pair.
The two tests currently live only in `settings_tests.rs`. This is one
test-only move of roughly 30 lines across three files; production behavior,
assertions, helper signatures, `docs/usage.md`, and screenshots are unchanged.

**Acceptance criteria.**

- `src/ui/tests/toggle_persist_tests.rs` defines the two moved tests with
  their original bodies, and `src/ui/tests/settings_tests.rs` no longer
  defines them; its remaining tests are unchanged.
- Test discovery lists each moved test once under
  `ui::tests::toggle_persist_tests` with its original leaf name, and
  `make test-one TEST=autoscroll_choice_is_restored` and
  `make test-one TEST=toggle_autoscroll_reaches` each run the moved test
  from the new home.
- The autoscroll pinning sentence in `## Settings persistence` resolves both
  named tests to `src/ui/tests/toggle_persist_tests.rs`, and no Pinned-by line
  naming a test that still lives in `settings_tests.rs` is retagged.
- `make check` passes, and no production code or test assertion changes.

## Done
