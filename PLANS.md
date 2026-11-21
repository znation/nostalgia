# Plans

## Planned

### Extract the toggle save/restore round-trip tests from settings_tests.rs into a focused toggle_persist_tests.rs module (planned 2026-11-20 by the plan role)

**Goal.** `src/ui/tests/settings_tests.rs` still holds 11 contiguous tests of
one topic — the save/restore round-trip of each visual-surface toggle (time
display, always-on-top, visualizer, panel visibility, window shade, and the
shade-timeout guard) — interleaved with the sign-in-token, song-list, and
window-position persistence tests. Extracting them gives settings persistence
the same focused-module shape the position-tick, position-poll, and mixer
extractions gave the other areas.

**Approach.**
- Move the contiguous run
  `toggle_time_display_reaches_the_saved_snapshot` →
  `shaded_eq_toggle_saves_nothing` (the 11 tests from
  `toggle_time_display_reaches_the_saved_snapshot` through
  `saved_shade_is_restored_on_boot` plus
  `shaded_eq_toggle_saves_nothing`, ~194 lines) to a new
  `src/ui/tests/toggle_persist_tests.rs`, moved verbatim — the block carries
  no doc comments and defines no helpers of its own.
- No support move: the block calls only `test_player`,
  `MemorySettingsStore`, `Settings::default`,
  `update(&mut player, …)`, `store.snapshots()`,
  `player.adopt_saved_settings`, and asserts on `player.saved_settings` —
  `test_player` and `MemorySettingsStore` are `pub(super)` in
  `src/ui/tests/support.rs` and stay there.
- Register the new module (`mod toggle_persist_tests;`) alphabetically in
  `src/ui/tests/mod.rs`, matching the existing registration list.
- Retag every Pinned-by mention of the 11 moved names in
  `docs/invariants.md` — the persistence section's group list (the run from
  `toggle_time_display_reaches_the_saved_snapshot` through
  `saved_shade_is_restored_on_boot`) and the window-shade section's explicit
  pair (`toggle_window_shade_reaches_the_saved_snapshot` and
  `saved_shade_is_restored_on_boot` (`src/ui/tests/settings_tests.rs`)) —
  giving each moved name an explicit
  `(`src/ui/tests/toggle_persist_tests.rs`)` and leaving every kept entry's
  resolution untouched. Because the steward's invariants-normalization plan
  may reshape the same lists before this lands, resolve each entry at merge
  time by grepping the test's `fn` for its real home and word the edit so its
  order in the list does not matter.
- Out of scope: the remaining settings tests (sign-in token, song lists,
  library view, window position and clamping), any support.rs change, and
  any player behavior — a pure test move requires no `docs/usage.md` or
  screenshot update.

**Acceptance criteria.**
- `src/ui/tests/toggle_persist_tests.rs` holds the 11 tests moved verbatim,
  `src/ui/tests/settings_tests.rs` no longer defines them, and the crate
  builds with no other edit to either file's body.
- `make test-one TEST=toggle_time_display` and
  `make test-one TEST=shaded_eq_toggle` still run the moved tests, now from
  the new module.
- Every moved name in `docs/invariants.md` resolves to
  `src/ui/tests/toggle_persist_tests.rs`, and every kept name in the same
  lists still resolves to its real home.
- No test's assertions, names, or expected values change.

### Move the File info panel rendering tests into file_info_tests.rs (planned 2026-11-20 by the plan role)

**Goal.** Give the File info panel one test home. `src/ui/tests/` already has
`file_info_tests.rs` for the panel's track resolution, but its three rendering
tests still sit in the general `src/ui/tests/screen_tests.rs` (1350 lines),
where they are the only tests of that panel. Moving them completes the
focused module and continues shrinking `screen_tests.rs` without touching the
in-flight toggle-extraction region of `settings_tests.rs`.

**Approach and files.** Move these three complete tests, with their comments,
inputs, and assertions unchanged, from `src/ui/tests/screen_tests.rs` into the
existing `src/ui/tests/file_info_tests.rs`:

- `file_info_panel_draws_the_loaded_previews_format`
- `file_info_panel_draws_each_metadata_line_only_when_the_track_has_it`
- `file_info_overlay_names_the_selected_rows_song`

They form one contiguous block between `view_constructs_when_the_window_is_shaded`
and `title_bar_constructs_for_both_about_states`; every other
`screen_tests.rs` test stays where it is. Keep the receiving file's
`use super::*;` preamble and its five resolution tests in place; do not
re-register a module in `src/ui/tests/mod.rs` (the sibling is already listed).
No helper moves: the block calls only `test_player`, `seed_current_title`,
`seed_browse_lists`, `captured_root_view_text_bounds`, and glob imports such
as `PlaybackFormat`, `CurrentView`, and `known_track`, all already reachable
from any child of `src/ui/tests` through `support.rs` and `mod.rs`.

In `docs/invariants.md`, retag the two `## File info panel` Pinned-by entries
that name moved tests — `file_info_overlay_names_the_selected_rows_song` and
`file_info_panel_draws_the_loaded_previews_format` — from
`src/ui/tests/screen_tests.rs` to `src/ui/tests/file_info_tests.rs`. The
metadata-line test is not in that list. The in-flight toggle extraction also
edits `docs/invariants.md`, but only the settings-persistence and window-shade
sections, so the two changes merge in either order.

**Acceptance criteria.**

- `src/ui/tests/file_info_tests.rs` defines the three moved tests with their
  original bodies, and `src/ui/tests/screen_tests.rs` no longer defines them;
  its remaining tests and the receiving module's existing resolution tests
  are unchanged.
- Test discovery lists each moved test once under
  `ui::tests::file_info_tests` with its original leaf name, and
  `make test-one TEST=file_info_panel_draws_the_loaded_previews_format`,
  `make test-one TEST=file_info_panel_draws_each_metadata_line`,
  and `make test-one TEST=file_info_overlay_names_the_selected_rows_song`
  each run the moved test from the new home.
- A loaded preview with a reported format draws
  `Format: 256 KBPS  44 KHZ STEREO`, and stopping the preview removes the
  `Format:` line while keeping the title; a bare track draws only `Title:` and
  `Length:`, a fully filled track draws its artist, album, year, genre, track,
  disc, composer, and rating lines, and `Explicit` becomes `Clean`; and a
  selected song row names that row's song ahead of the loaded track without a
  `Format:` line.
- Every Pinned-by entry naming a moved test in `docs/invariants.md` resolves to
  `src/ui/tests/file_info_tests.rs`.
- `make check` passes; no behavior, fixture, assertion, or helper signature
  changes.

## Done
