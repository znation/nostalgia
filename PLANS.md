# Plans

## Planned

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
