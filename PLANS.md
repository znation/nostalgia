# Plans

## Planned

### Extract balance center-detent tests from the mixer module (planned 2026-11-17 by the plan role)

**Goal.** Give balance center-detent coverage a focused home and reduce the
responsibilities of `src/ui/tests/mixer_tests.rs`. Checked 2026-11-17: the
three tests below remain there; the finished center-detent behavior plan
does not cover this relocation.

**Approach and files.** Move these complete tests, including their explanatory
comments, attributes, inputs, and assertions, into
`src/ui/tests/balance_detent_tests.rs` with `use super::*;`:

- `dragging_balance_into_the_middle_snaps_to_center`
- `a_continuous_inward_balance_drag_stays_snapped`
- `a_balance_step_after_release_leaves_center`

Register the sibling module in `src/ui/tests/mod.rs`, remove the old definitions,
and update the test attribution in `docs/invariants.md`'s `Balance center detent`
section. Use the existing `test_player`, `seed_current_title`, and
`root_view_draws` fixtures without changing them. Keep the neighboring balance
readout test and production code in place. This is independent of the queued
seek-drag extraction: it moves different tests into a different module. No
visible behavior changes are intended, so usage and screenshots need no update.

**Acceptance criteria.**

- Test discovery lists each named test once under `ui::tests::balance_detent_tests`,
  with no copy under `ui::tests::mixer_tests`.
- Given balance requests of -0.4 then -0.05, stored balance is exactly 0.0
  and the root view shows `Balance: Center`.
- After an initial -0.4 request, successive requests of -0.05, -0.04,
  -0.02, and 0.01 keep stored balance at 0.0; the next request of 0.06
  stores 0.06.
- After -0.4 then -0.05 and a mixer release, a 0.01 request stores 0.01.
- The diff contains test relocation, module registration, and invariant
  attribution changes; the existing test inputs and assertions are preserved,
  and `make check` passes.

## Done
