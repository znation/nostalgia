# Plans

## Planned

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

### Move the File info root-view wiring test from screen_tests.rs into file_info_tests.rs (planned 2026-11-21 by the plan role)

**Goal.** `src/ui/tests/screen_tests.rs` (1036 lines) still holds the File
info panel's root-view wiring test,
`root_view_draws_the_file_info_panel_for_a_known_current_track`, between the
transport title-bar tests and the shade layout tests. The focused
`src/ui/tests/file_info_tests.rs` module (311 lines) already owns the File
info panel — its two panel-rendering tests and its overlay and resolution
tests were moved there by the landed File info extraction — so this wiring
test, which pins that the root `view` branch actually draws the panel and
drops it for an unresolved track, completes that topic in one home. This is
independent of the in-flight autoscroll move in `settings_tests.rs` and of
the pending row-pitch retire, which touch different files.

**Approach and files.** Move this one test, with its five leading comment
lines ("The File info panel is wired through the root `view` …"), verbatim
from `src/ui/tests/screen_tests.rs` — where it sits between
`root_view_draws_the_file_info_panel_for_a_known_current_track`'s neighbors
`title_bar_constructs_for_both_about_states` and the moved block that ended
before it — into `src/ui/tests/file_info_tests.rs`, inserted immediately
after `file_info_panel_draws_each_metadata_line_only_when_the_track_has_it`
and before `file_info_overlay_names_the_selected_rows_song`, so the three
root-view panel-rendering tests sit together. Every other `screen_tests.rs`
test stays. Do not register a new module in `src/ui/tests/mod.rs` —
`file_info_tests` is already listed. No helper moves: the test calls only
`test_player`, `captured_root_view_text_bounds`, `player.known_tracks`,
`player.file_info_visible`, `views::KnownTrack`, and
`state.blocking_lock()`, all already used by the receiving module's existing
tests through its `use super::*;` preamble (`views` and the support helpers
are re-exported by `src/ui/tests/mod.rs`). Keep `screen_tests.rs`'s preamble
unchanged.

`docs/invariants.md` needs no change: grepping it for the moved test name
finds no Pinned-by entry, and no other pin names a test this move touches.

**Dedupe and scope.** `file_info_tests.rs` does not define
`root_view_draws_the_file_info_panel_for_a_known_current_track`, and the
test currently lives only in `screen_tests.rs`. Greps of PLANS.md,
PLANS-DONE.md, and the pending plans found the landed File info extraction,
which moved the two panel-rendering tests and left this one in place, but no
plan covers moving it. This is one test-only move of roughly 55 lines across
two files; production behavior, fixtures, assertions, `docs/usage.md`, and
screenshots are unchanged.

**Acceptance criteria.**

- `src/ui/tests/file_info_tests.rs` defines
  `root_view_draws_the_file_info_panel_for_a_known_current_track` with its
  original comment and body, inserted after
  `file_info_panel_draws_each_metadata_line_only_when_the_track_has_it`, and
  `src/ui/tests/screen_tests.rs` no longer defines it; every remaining test
  in that file keeps its body.
- Test discovery lists the moved test once under
  `ui::tests::file_info_tests` with its original leaf name, and
  `make test-one TEST=file_info_panel_for_a_known` runs it from the new
  home; the known-track case still asserts the texts contain `FILE INFO`,
  `Title: Opening`, `Artist: The Sample Band`, and `Length: 3:30`, and the
  unresolved `missing`-track case still asserts no text equals `FILE INFO`.
- `make check` passes, and no production code, support helper, invariant
  entry, or test assertion changes.

## Done
