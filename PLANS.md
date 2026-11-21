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

### Move AppState's inline tests into state/tests.rs (planned 2026-11-20 by the organize role)

**Goal.** Give the shared state module a focused production file and test module. `src/state.rs`
contains about 412 lines of production definitions followed by a roughly 500-line inline test
module; the tests account for more than half of the file and obscure the state API when reading
its implementation.

**Approach and files.** Move the complete `#[cfg(test)] mod tests` from `src/state.rs` to
`src/state/tests.rs`, with `#[cfg(test)] mod tests;` at the end of `state.rs`. Preserve the test
bodies and helper organization. Update the five `Pinned-by` test citations in `docs/invariants.md`
that name tests in `src/state.rs` to name `src/state/tests.rs`; keep production-function
citations to `src/state.rs` in place. The path attribute/module should follow the existing
`src/ui/shortcuts.rs` and `src/ui/shortcuts/tests.rs` pattern.

**Cost removed.** Readers currently pass through hundreds of state behavior tests to reach
other production code or the end of the module; a sibling test file makes the implementation
and its contract tests independently navigable.

**Acceptance criteria.**

- `src/state.rs` contains the unchanged production implementation and a test module declaration;
  `src/state/tests.rs` contains the moved module contents and compiles under the existing test
  configuration.
- The five state-test pins in `docs/invariants.md` resolve to `src/state/tests.rs`, while
  production pins remain on `src/state.rs`.
- Grep finds no stale moved test path in source or markdown, and `make check` passes.
- No behavior, assertion, or usage documentation changes.


## Done
