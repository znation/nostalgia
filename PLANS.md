# Plans

## Planned

### Extract preset-deletion tests from the mixer test module (planned 2026-11-17 by the plan role)

**Goal.** Give the Save preset panel's deletion contract a focused test module.
Checked 2026-11-17: `src/ui/tests/mixer_tests.rs` contains the deletion group
after the save/replace tests; a separate deletion test module is not registered.

**Approach and files.** Move these tests, with their attached comments,
attributes, and bodies preserved, into a new
`src/ui/tests/delete_preset_tests.rs` using `use super::*;`:

- `delete_preset_submitted_removes_the_named_custom_preset`
- `delete_preset_submitted_refuses_empty_builtin_colon_and_unknown_names`
- `delete_preset_submitted_is_a_no_op_with_the_panel_closed`
- `deleting_the_applied_custom_leaves_the_applied_curve_untouched`

Register the sibling in `src/ui/tests/mod.rs` and remove the moved definitions
from `src/ui/tests/mixer_tests.rs`. Leave the save/replace and custom-choice
tests in place. This is one cohesive extraction, with no production or shared
helper changes. Check the Save preset panel section of `docs/invariants.md`;
update a qualified test path there if the move makes it stale. Usage and visual
behavior do not change, so no usage-reference or screenshot update is needed.

**Acceptance criteria.**

- Test discovery lists each named test once under `ui::tests::delete_preset_tests`
  and no longer under `ui::tests::mixer_tests`.
- The extracted tests retain their existing inputs and assertions: a matching
  name removes its saved curve, invalid names preserve the saved list, a closed
  panel ignores deletion, and deleting the applied custom preserves its curve.
- The diff contains the test relocation and module registration, plus any stale
  qualified invariant-test reference correction, without assertion or runtime
  behavior changes.
- `make check` passes after extraction.

## Done
