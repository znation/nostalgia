# Plans

## Planned

### Extract seek-drag readout tests from the mixer module (planned 2026-11-17 by the plan role)

**Goal.** Give seek-drag title and marquee coverage a focused home, reducing
the responsibilities of `src/ui/tests/mixer_tests.rs` without changing behavior.
Checked 2026-11-17: these three tests remain in the mixer module; the original
finished seek-readout plan delivered the behavior, not this extraction.

**Approach and files.** Move these complete tests, preserving their attributes,
inputs, and assertions, to `src/ui/tests/seek_drag_tests.rs` with `use super::*;`:

- `dragging_the_seek_slider_shows_the_target_until_release`
- `a_seek_drag_without_a_duration_shows_no_seek_text`
- `the_marquee_pauses_during_a_seek_drag`

Register the sibling in `src/ui/tests/mod.rs`, remove the old definitions, and
correct their attribution in `docs/invariants.md`'s `Mixer drag readout` section.
Use the existing `test_player`, `seed_current_title`, and `root_view_draws`
fixtures from `src/ui/tests/support.rs` without changing them. Keep production
code and neighboring mixer tests in place. This is independent of the queued
root seek-wheel routing extraction, which moves different tests to a different
module. No visible behavior changes are intended; usage and screenshots need
no update.

**Acceptance criteria.**

- Test discovery lists each named test once under `ui::tests::seek_drag_tests`,
  with no copy under `ui::tests::mixer_tests`.
- Given title `Tune`, duration 30 seconds, and seek target 12.5 seconds, the
  root view shows `Seek to: 00:12/00:30 (41%)` instead of the title during the
  drag; release restores the title and removes that seek text.
- With no duration and a stored 12-second target, the title remains visible
  and the tested zero-duration seek text is absent.
- For the existing long-title fixture, marquee eligibility is true before
  the seek drag, false during it, and true after release.
- The diff contains test relocation, module registration, and invariant
  attribution changes; existing test inputs and assertions are preserved,
  and `make check` passes.

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
