# Plans

## Planned

### Extract balance and equalizer wheel-message tests from the mixer module (planned 2026-11-17 by the plan role)

**Goal.** Give slider-wheel message handling a focused test module, reducing
the responsibilities of `src/ui/tests/mixer_tests.rs` without changing behavior.
Checked 2026-11-17: the tests below remain there, and
`src/ui/tests/mod.rs` has no `slider_wheel_tests` sibling registration.

**Approach and files.** Move these tests, including their attached comments
and attributes, into `src/ui/tests/slider_wheel_tests.rs` with `use super::*;`:

- `balance_wheel_nudges_by_one_step_and_ignores_zero_and_nan`
- `eq_preamp_wheel_nudges_by_one_db_and_ignores_zero_and_nan`
- `eq_band_wheel_nudges_only_its_band_and_ignores_zero_nan_and_an_out_of_range_index`
- `slider_wheels_forward_their_new_values_to_the_audio_backend`

Register the sibling in `src/ui/tests/mod.rs`, remove the moved definitions,
and update the moved tests' file attribution in `docs/invariants.md`'s
`Slider wheel` section. Keep its event-routing test attributions accurate:
those tests remain in `mixer_tests.rs`. Existing shared fixtures in
`src/ui/tests/support.rs` suffice. Leave fixtures, production code, volume-wheel
tests, event-routing tests, and drag/readout tests in place. This extraction is
independent of the waiting volume/balance backend-test extraction. No visible
or control behavior changes are intended; usage and screenshots need no update.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::slider_wheel_tests`, with no copy under `ui::tests::mixer_tests`.
- The relocated tests preserve their inputs and assertions: positive and
  negative rolls nudge the selected value, zero and NaN leave it unchanged,
  repeated rolls clamp at the tested limits, invalid band indices leave the
  curve unchanged, and the recording backend sees the expected new values.
- The diff is limited to test relocation, module registration, and invariant
  file-attribution corrections, with no assertion or shared-fixture changes.
- `make check` passes after extraction.
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
