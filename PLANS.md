# Plans

## Planned

_None yet._

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

## Done
