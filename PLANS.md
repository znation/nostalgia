# Plans

## Planned

### Extract root seek-wheel routing tests from the mixer module (planned 2026-11-17 by the plan role)

**Goal.** Give root seek-wheel event routing a focused test module, reducing
the responsibilities of `src/ui/tests/mixer_tests.rs` without changing behavior.
Checked 2026-11-17: the two tests below remain in that module.

**Approach and files.** Move these tests, including their comments, attributes,
inputs, and assertions, into `src/ui/tests/seek_wheel_tests.rs` with
`use super::*;`:

- `wheel_over_the_root_seek_slider_seeks_without_nudging_the_volume`
- `wheel_over_a_blank_seek_strip_still_nudges_the_volume`

Register the sibling in `src/ui/tests/mod.rs`, remove the old definitions,
and correct their file attribution in `docs/invariants.md`'s `Seek wheel`
section. Existing `test_player`, `captured_root_view_container`, and
`root_wheel_messages` fixtures in `src/ui/tests/support.rs` suffice. Keep
fixtures and production code in place. The view-level tests in
`src/ui/views/tests/seek_wheel.rs` remain there. This extraction is independent
of the waiting equalizer backend relocation. No visible or control behavior
changes are intended; usage and screenshots need no update.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::seek_wheel_tests`, with no copy under `ui::tests::mixer_tests`.
- With position 45 seconds and duration 180 seconds, a positive or negative
  vertical roll over the drawn root seek slider produces exactly one seek
  message in the corresponding direction and no volume-wheel message.
- With neither position nor duration, the root seek slider is absent; a
  positive roll over the blank strip produces exactly one volume-wheel
  message and no forward/backward seek message.
- The diff is limited to test relocation, module registration, and invariant
  file-attribution corrections, preserving the existing assertions and inputs;
  `make check` passes.

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

## Done
