# Plans

## Planned

### Give a stdout-only service failure a test-visible report path (planned 2026-10-29 by the steward role)

**Goal.** Three Fixed entries in BUGS-FIXED.md carry the same validation gap — `no-observability`,
a failure that left no trace any check could see — and three carriers is the promotion threshold
for retiring it with infrastructure (current BUGS-FIXED.md validation-gap tally: 20 `none`,
12 `unclear-invariant`, 3 `no-observability`, 2 `no-repro`, 2 `real-run-needed`, 1 `no-fake`):

- "Play after a relaunch with a restored last-played track does nothing" — the broken Play
  reported only a stdout line and no invariant pinned the docs' promise, so closing the gap
  meant pinning the whole replay path afterwards rather than reproducing an observed failure.
- "A stale `PLANS.md.bak` copy of PLANS.md ships in the repository root" — the stray file broke
  no build or test and surfaced only through the `repo_hygiene.rs` scan written for it.
- "A balance-drag doc comment starts mid-sentence after its first line was dropped" — no check
  reads private doc prose, so the truncation left no failing trace. Out of scope below: no
  automatable prose checker this plan could add stays honest (any text-matching heuristic would
  nag every rewording), so this carrier is cited, not addressed.

Out of scope regardless: no behavior change at the surfaces themselves — the play path, settings
file, and doc comments the three fixes already landed are untouched.

**Approach.** Make a reported failure visible where the check can read it, at the point of
reporting, so the next failure of this class is confirmable like a `none`-tagged one:

- `src/apple_music.rs`, `src/apple_music/threads.rs`, `src/apple_music/playback.rs`: the
  failure-path `println!`/`eprintln!` reporting (sign-in failures and skips, transport
  "Nothing to resume"-style dead ends) routes through one test-visible seam — an injectable
  reporter like the service's other injectables, whose default keeps the operator's stdout and
  whose stub records what it was given. The feature confirms the exact shape (a `Reporter`
  trait, a recorded field on the service, or recorded entries on shared state) against the
  current code before wiring and keeps the seam narrow.
- Tests: a stub-recorded failure round-trip per route (sign-in refusals through `threads.rs`
  and the session store in `apple_music.rs`, the playback dead ends), asserting each route's
  stub records what its default would only print; each new reporter route replaces a named
  `stdout`/`eprintln!` call still present at wiring time.
- `docs/invariants.md` gets a section pinning "a service failure is reported through the
  recorded seam, not only stdout", with its enforcing test named, so the next `no-observability`
  bug in this area reconstructs nothing.

## Done
