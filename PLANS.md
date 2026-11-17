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

### Move equalizer backend tests into the focused mixer backend module (planned 2026-11-17 by the plan role)

**Goal.** Consolidate mixer backend-forwarding and failure tests in the
existing `src/ui/tests/mixer_backend_tests.rs`, reducing the responsibilities
of `src/ui/tests/mixer_tests.rs`. Checked 2026-11-17: the two tests below
remain in the latter file; the backend module currently covers volume and
balance.

**Approach and files.** Move these definitions, including their attached
comments, attributes, inputs, assertions, and nested `snapshot` helper, from
`src/ui/tests/mixer_tests.rs` to `src/ui/tests/mixer_backend_tests.rs`:

- `equalizer_messages_forward_to_the_audio_backend`
- `an_equalizer_backend_failure_still_commits_the_state`

The destination already imports `super::*` and is registered in
`src/ui/tests/mod.rs`. Its shared `player_with_recording`,
`player_with_failing_audio`, and `assert_message_schedules_no_work` fixtures
in `src/ui/tests/support.rs` suffice. Leave fixtures and production code in
place. This relocation is independent of the waiting volume-input extraction.
It changes no control behavior or subsystem guarantee, so usage, screenshots,
and invariant documentation need no update.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::mixer_backend_tests`, with no copy under `ui::tests::mixer_tests`.
- The relocated forwarding test still observes equalizer snapshots matching
  stored state after toggle, preamp, band, and built-in preset messages,
  including the existing maximum-gain clamp and NaN-to-flat case; these
  messages schedule no work.
- The relocated failure test still observes band zero stored at 4.0 and one
  attempted equalizer command when the injected backend rejects that command;
  the message schedules no work.
- The diff contains test relocation without assertion, fixture, module
  registration, or production-code changes, and `make check` passes.

## Done
