# Plans

## Planned

### Extract AUTO preset tests from the mixer module (planned 2026-11-17 by the plan role)

**Goal.** Give genre-driven preset loading a focused test home, reducing the
responsibilities of `src/ui/tests/mixer_tests.rs` without changing behavior.
Checked 2026-11-17: the tests below remain in that module; the archived AUTO
feature plan implements the behavior rather than this test organization.

**Approach and files.** Move these complete tests, preserving comments,
attributes, inputs, and assertions, into a new
`src/ui/tests/auto_preset_tests.rs` with `use super::*;`:

- `toggle_eq_auto_flips_only_the_auto_flag`
- `auto_on_applies_a_genre_matched_preset_on_track_selection`

Give the new module a private copy of the small `assert_curve_applied` helper;
retain the original for the preset-selection and Save panel tests that still
use it. Register the module in `src/ui/tests/mod.rs`, remove the old test
definitions, and update the `Auto-load presets` test attribution in
`docs/invariants.md`. Keep fixtures and production code, including
`handlers::mixer::{toggle_eq_auto, apply_auto_preset}` and
`handlers::playback::track_selected`, as they stand. This is one independent
test relocation; no usage or screenshot update is needed.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::auto_preset_tests`, with no copy under `ui::tests::mixer_tests`.
- Starting from the Rock curve with EQ disabled, toggling AUTO on preserves
  the curve, selected preset, and disabled EQ state and schedules no work;
  toggling it off preserves the Rock selection and band gains and schedules
  no work.
- With AUTO enabled and Vocal initially selected, selecting the song whose
  genre is `  rOcK  ` selects the Rock preset, stores its preamp and bands,
  and records one additional equalizer backend call carrying that curve.
  Selecting `Alternative`, then selecting `Vocal` with AUTO disabled, retains
  the Rock selection and adds no backend calls.
- The diff contains only test relocation, the local assertion helper,
  module registration, and invariant attribution changes; moved test inputs
  and assertions are preserved, and `make check` passes.

## Done

### Extract preset-name validation tests from the mixer module (planned 2026-11-17 by the plan role)

**Goal.** Give Save preset name-refusal coverage a focused home, reducing the
responsibilities of `src/ui/tests/mixer_tests.rs`. Checked 2026-11-17: the two
tests below remain there, and the archived preset-deletion extraction covers
different tests.

**Approach and files.** Move these complete tests, preserving their comments,
attributes, inputs, and assertions, into a new
`src/ui/tests/save_preset_validation_tests.rs` with `use super::*;`:

- `save_preset_submitted_refuses_empty_builtin_and_colon_names`
- `save_preset_submitted_refuses_a_name_with_a_line_break`

Register the module in `src/ui/tests/mod.rs` and remove the old definitions.
Update `docs/invariants.md`'s `Save preset panel` test attribution to the new
module, including the existing line-break test that the section currently
does not list. Use the existing `test_player` fixture without changing it.
Leave save/replace, custom-choice, deletion, and production code in place.
This extraction is independent of the queued balance-detent extraction.
No visible behavior changes are intended; usage and screenshots need no update.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::save_preset_validation_tests`, with no copy under
  `ui::tests::mixer_tests`.
- With the Save preset panel open, submitting an empty name, `rock`, or
  `A:name` keeps it open, saves no custom preset, and shows the corresponding
  existing empty-name, built-in-name, or colon-name note.
- Submitting `Fuzz\nBass` or `Fuzz\rBass` keeps the panel open, saves no
  custom preset, and shows the existing line-break note; editing the name
  between those submissions clears the previous note.
- The diff preserves the moved test inputs and assertions and contains only
  test relocation, module registration, and invariant attribution changes;
  `make check` passes.
