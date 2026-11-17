# Plans

## Planned

### Extract volume key and wheel message tests from the mixer module (planned 2026-11-17 by the plan role)

**Goal.** Give volume-step message handling a focused test module and reduce
the responsibilities of `src/ui/tests/mixer_tests.rs`. Checked 2026-11-17:
the tests below remain there, and `src/ui/tests/mod.rs` has no
`volume_input_tests` sibling registration.

**Approach and files.** Move these tests, preserving attached comments,
attributes, inputs, and assertions, into `src/ui/tests/volume_input_tests.rs`
with `use super::*;`:

- `volume_up_and_down_nudge_by_the_slider_step`
- `volume_keys_clamp_at_the_ends`
- `volume_wheel_nudges_by_one_slider_step_and_ignores_zero`
- `volume_wheel_clamps_at_the_ends`

Register the sibling in `src/ui/tests/mod.rs`, remove the moved definitions
from `src/ui/tests/mixer_tests.rs`, and correct their file attribution in
`docs/invariants.md`'s `Volume wheel` section. Existing `test_player` and
`assert_message_schedules_no_work` fixtures in `src/ui/tests/support.rs`
suffice; leave fixtures, backend-forwarding tests, flash tests, and production
code in place. This is independent of the queued balance/equalizer wheel
extraction. No visible or control behavior changes are intended, so usage
and screenshots need no update.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::volume_input_tests`, with no copy under `ui::tests::mixer_tests`.
- The relocated tests retain their existing observations: Up and positive
  wheel messages raise volume by `views::VOLUME_STEP`; Down and negative
  wheel messages restore the starting volume; a zero wheel message leaves
  volume unchanged; repeated key or wheel messages clamp at 1.0 and 0.0.
  Each exercised message continues to schedule no work.
- The diff contains test relocation, module registration, and invariant
  file-attribution corrections without assertion, fixture, or production
  code changes.
- `make check` passes after extraction.

## Done
