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
## Done
