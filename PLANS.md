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

## Done
