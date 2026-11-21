# Plans

## Planned

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
