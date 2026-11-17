# Plans

## Planned

### Extract volume and balance backend tests from the mixer test module (planned 2026-11-17 by the plan role)

**Goal.** Give volume and balance's backend-forwarding contract a focused test
module, reducing the responsibilities of `src/ui/tests/mixer_tests.rs`.
Checked 2026-11-17: the four tests below remain in that file; no sibling
`mixer_backend_tests` module is registered in `src/ui/tests/mod.rs`.

**Approach and files.** Move these tests, preserving their attached comments,
attributes, inputs, and assertions, into a new
`src/ui/tests/mixer_backend_tests.rs` with `use super::*;`:

- `volume_messages_forward_to_the_audio_backend`
- `balance_messages_forward_to_the_audio_backend`
- `a_volume_backend_failure_still_commits_the_state`
- `a_balance_backend_failure_still_commits_the_state`

Register the sibling in `src/ui/tests/mod.rs` and remove the moved definitions
from `src/ui/tests/mixer_tests.rs`. The existing shared fixtures in
`src/ui/tests/support.rs` suffice; leave them and the wheel, equalizer, and
preset tests in place. This is independent of the queued preset-deletion
extraction. Correct a qualified reference in `docs/invariants.md` if it names
one of the moved tests' old paths. No runtime or visible behavior changes are
intended, so usage and screenshots need no update.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::mixer_backend_tests`, with no copy under `ui::tests::mixer_tests`.
- The relocated tests retain their existing checks: volume changes and arrow
  steps produce the expected recorded gain values; balance changes produce
  the expected recorded balance values, including the existing clamp and NaN
  cases; rejected volume and balance commands leave the chosen shared-state
  value committed and record the expected attempted command.
- The diff contains the relocation and module registration, plus any stale
  qualified invariant-reference correction, without changes to test assertions,
  fixtures, or production code.
- `make check` passes after extraction.

## Done
