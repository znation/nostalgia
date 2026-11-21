# Plans

## Planned

### Extract the browse row-pitch tests from screen_tests.rs into a focused row_pitch_tests.rs module (planned 2026-11-20 by the plan role)

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

### Move the transport glyph rendering tests into transport_tests.rs (planned 2026-11-20 by the plan role)

**Goal.** Give the transport row's rendering tests a home in the focused
`src/ui/tests/transport_tests.rs` module, which already owns the transport
messages, instead of the 1350-line general `screen_tests.rs`. This is
independent of the in-flight file-info move and the pending row-pitch move:
the three tests sit together below the row-pitch block and above the title-band
tests, and neither of those changes touches them.

**Approach and files.** Move these three complete tests, with their comments
and assertions unchanged, from `src/ui/tests/screen_tests.rs` into the existing
`src/ui/tests/transport_tests.rs`:

- `transport_row_keeps_play_and_pause_glyphs_fixed_across_playback_states`
- `root_view_draws_the_transport_glyph_row`
- `root_view_draws_the_glyph_for_the_shared_transport_mode`

They form one contiguous block; every other `screen_tests.rs` test stays where
it is. Keep `transport_tests.rs`'s `use super::*;` preamble and its existing
transport-message tests in place; do not re-register a module in
`src/ui/tests/mod.rs` (the receiving module is already listed). No helper
moves: the block calls only `test_player` and
`captured_root_view_text_bounds`, both reachable from any child of
`src/ui/tests` through `support.rs` and `mod.rs`.

In `docs/invariants.md`, retag the two `## UI layout` Pinned-by entries that
name moved tests —
`transport_row_keeps_play_and_pause_glyphs_fixed_across_playback_states` and
`root_view_draws_the_transport_glyph_row` — from
`src/ui/tests/screen_tests.rs` to `src/ui/tests/transport_tests.rs`. The
shared-mode glyph test is not in that list. The pending row-pitch plan edits
the browse visibility and caching sections, not `## UI layout`, so the two
changes merge in either order.

**Acceptance criteria.**

- `src/ui/tests/transport_tests.rs` defines the three moved tests with their
  original bodies, and `src/ui/tests/screen_tests.rs` no longer defines them;
  its remaining tests and the receiving module's existing tests are unchanged.
- Test discovery lists each moved test once under
  `ui::tests::transport_tests` with its original leaf name, and
  `make test-one TEST=transport_row_keeps_play_and_pause`,
  `make test-one TEST=root_view_draws_the_transport_glyph_row`, and
  `make test-one TEST=root_view_draws_the_glyph_for_the_shared_transport_mode`
  each run the moved test from the new home.
- Laid out in the root view, Play and Pause glyphs keep the same bounds when
  playback starts, and Previous and Next share one row with Next to the right;
  the LCD glyph reads `■` stopped, `▶` playing, `II` paused, and `■` again
  after Stop.
- Every Pinned-by entry naming a moved test in `docs/invariants.md` resolves to
  `src/ui/tests/transport_tests.rs`.
- `make check` passes; no behavior, fixture, assertion, or helper signature
  changes.

## Done
