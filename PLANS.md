# Plans

## Planned

### Extract the save-panel and save-commit tests from the mixer module (planned 2026-11-20 by the plan role)

**Goal.** Give the Save preset panel's open and save-commit coverage a focused
home, reducing the responsibilities of `src/ui/tests/mixer_tests.rs` further.
Checked 2026-11-20: the five tests below are still defined there; the
finished extractions (`mixer_drag_readout_tests`, `mixer_readout_input_tests`,
`auto_preset_tests`, `save_preset_validation_tests`, and the balance and
wheel-routing modules) cover different tests, and `save_preset_validation_tests.rs`
holds only the submit-refusal tests.

**Approach and files.** Move these complete tests, preserving comments,
attributes, inputs, and assertions, into a new
`src/ui/tests/save_preset_panel_tests.rs` with `use super::*;`:

- `a_custom_choice_applies_the_whole_curve_and_keeps_the_selection_none`
- `the_save_row_opens_the_panel_and_closes_the_other_panels`
- `a_closed_save_panel_ignores_its_submit`
- `save_preset_submitted_saves_the_applied_curve_under_the_typed_name`
- `saving_over_an_existing_custom_replaces_only_its_curve`

These are exactly the five tests `docs/invariants.md`'s `## Save preset panel`
section still attributes to `src/ui/tests/mixer_tests.rs`; nothing else moves.
Register the sibling in `src/ui/tests/mod.rs`, remove the old definitions, and
update that section's Pinned-by list to name the five tests at the new module.
Keep existing shared fixtures and production code unchanged.
`assert_curve_applied` is used both by a moving test
(`the_save_row_opens_the_panel_and_closes_the_other_panels`) and by a staying
one (`eq_preset_selected_applies_the_curve_and_selection`), so move it into
`src/ui/tests/support.rs` as a `pub(super)` helper in the same change (the
existing shared-helpers pattern, like `assert_toggles_shared_state`) and update
both modules' call sites. This is one cohesive test relocation; it changes no
player behavior and requires no usage or screenshot update.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::save_preset_panel_tests`, with no copy under
  `ui::tests::mixer_tests`.
- With a built-in `rock_preset` applied, choosing `PresetChoice::Save` opens
  the Save preset panel with an empty box and closes Jump to time, exactly as
  before; a closed panel ignores `SavePresetSubmitted`.
- Saving under the trimmed name `Very Fuzzed` keeps the first save's stored
  name when re-saved as `very fuzzed` and replaces only its curve, and the
  shared state keeps `eq_preset() == Some(rock_preset())` after saving.
- The diff contains only test relocation, module registration, and
  invariant-path updates; no moved test's inputs or assertions change.

_None yet._
