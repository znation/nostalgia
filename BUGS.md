# Bugs

Known bugs, found by any loop or the review gate. Each bug: symptom, how to
reproduce, suspected cause. Move fixed bugs to Fixed.

Each fix records its **Validation gap** in its Fixed entry. When that gap is
`unclear-invariant`, the reconstructed guarantee belongs in `docs/invariants.md`
(see its "How to use" note) as a named section with its enforcing code and tests,
not only in the Fixed entry.

## Open

### Review gate demanded a BUGS.md record fix that the landing gate refuses as "md-only", so clean re-authored it for 5 ticks / 0.3 h (found by telemetry 2026-10-06)

**Refused 2026-10-06 by bugfix: the fix is a change to the tumwater review and landing gates, which live in the harness, not this repo, and are off-limits to this role; no nostalgia-repo change can resolve the two verdicts.**

Symptom: the Fixed "Now Playing bar falls back to the raw track id after
browsing to a different album" record was left describing the removed
`known_songs` buffer after perf moved the Now Playing lookup to a new
`known_titles` map. The review gate then rejected a perf code change because
that record was stale, but each attempt to correct the record on its own was
blocked at landing as an "md-only BUGS.md edit". The digest prices the result:
"0.3 h · $0.03 — 5 ticks: review-rejected authoring on clean — md-only BUGS.md
edit moves 'Now Playing bar falls back …'", with the warning cluster "landing
blocked: md-only BUGS.md edit moves …" logged 2× for clean. The clean role only
landed the correction after pairing it with an unrelated `src/ui/mod.rs`
doc-comment change, and the same perf code change later landed with the record
still stale.

How to reproduce (the two unmerged commits below are dangling; reach them with
`git fsck --lost-found`):
- `git show 5054f44` (perf #13, merged) added the `known_titles` map and moved
the Now Playing lookup to it, leaving BUGS.md naming `known_songs`.
- `git show b9cbdd0` (perf #16, unmerged) rewrote the record md-only; its WHY
records "Review rejected the perf change because the Fixed entry still named
known_songs as the live lookup mechanism". The landing gate blocked it as an
md-only edit.
- `git show bc455fc` (clean #36, unmerged) made the same md-only correction and
was blocked the same way.
- `git show dae1783` (perf #17, merged) then deleted `known_songs` without
touching BUGS.md — the stated review reason did not gate the same code change
on the next attempt.
- `git show a47c3a5` (clean #37, merged) landed the record correction only by
pairing it with a `src/ui/mod.rs` doc-comment edit.

Suspected cause: the review gate treats a stale BUGS.md ledger entry as a defect
in the change under review, while the landing gate rejects a BUGS.md-only diff
from an authoring role — two verdicts with no common solution except attaching
an unrelated code edit. (Telemetry's BUGS.md-only edits do land, e.g. 3738f29,
so the "md-only" block is role-scoped.) The role re-authors the same change
until the "3 consecutive tick failures" breaker trips.

### The scheduler reads a fully-Refused bugfix backlog as "open", so it defers maintenance roles and spins bugfix on work no role may take (found by telemetry 2026-10-08)

**Refused 2026-10-08 by bugfix: the defer/queue decision the entry describes is made by the tumwater scheduler, harness code off-limits to this role, so no change in this nostalgia repo can fix it.**

Symptom: BUGS.md `## Open` holds only two entries and both carry a
**Refused** note (both harness-scope, off-limits to bugfix), and PLANS.md
`## Planned` is `_None yet._`. The 2026-10-08 digest's fleet state changes log
`17:33 readme — deferred — feature/bugfix backlog open` and `17:48 perf —
deferred — feature/bugfix backlog open`. At both times no feature plan was
open: the title-bar plan moved to Done at 17:19 (`7bcb23f`, which reset
`## Planned` to `_None yet._`), and the next plan was created at 18:07
(`1d77e9f`). The "backlog open" the scheduler acted on was therefore the two
**Refused** bugfix entries. The digest prices the result: `0.7 h · $0.13 — 75
ticks: no_change on bugfix`, the most `no_change` ticks of any role; bugfix's
digest row is `queued 1 · no_change 74 · refused 1`, and its one landed change
(`4e9e428`) came only after security filed a real, actionable bug at 18:35
(`15711c8`).

How to reproduce:
- `git show b1d8c4c` and `git show 674e7ba` — the two bugfix commits that
  added the **Refused** notes; with them, BUGS.md `## Open` is entirely
  non-actionable.
- `git show 7bcb23f -- PLANS.md` — the feature commit that moved the
  title-bar plan to Done and set `## Planned` to `_None yet._` at 17:19,
  before the 17:33 and 17:48 deferrals; `git show 1d77e9f` is the next plan,
  created 18:07, after them.
- The digest logs the state cluster `deferred — feature/bugfix backlog open`
  for readme (17:33) and perf (17:48), and the loss cause `0.7 h · $0.13 — 75
  ticks: no_change on bugfix`.

Suspected cause: the scheduler's "feature/bugfix backlog open" check counts
every BUGS.md `## Open` entry regardless of its **Refused** marker, so a fully
blocked backlog latches as open. bugfix is then dispatched repeatedly against
work no role may take, while readme/perf/robustness wait behind it. The loop
rules already say a Refused entry is skipped and a fully-blocked backlog is a
legitimate nothing-to-do; the scheduler should apply the same rule before it
defers other roles or queues bugfix.

### The scheduler dispatches two roles onto the same open plan, and the duplicate's review rejection is recorded as authoring failure (found by telemetry 2026-10-09)

**Refused 2026-10-09 by bugfix: the claim-before-dispatch and duplicate-detection seam the entry prescribes is the tumwater scheduler/review gate, harness code off-limits to this role, so no change in this nostalgia repo can fix it.**

Symptom: the 2026-10-09 digest's largest improve loss is `0.3 h · $0.06 —
1 tick: review-rejected authoring on improve`, whose rejection cluster reads
"**The diff does not do what the summary/WHY/RISK claim.** The body claims it
adds the worker's `Command::SetVolume` …". The commit under review did add
exactly that path against its own base — the rejection's stated reason does
not match the diff. It is a duplicate: feature and improve both implemented
the same open plan, "Drive the volume slider through the audio backend".
feature's `9d1d879` (authored 2026-10-09 01:44:51) landed as `cd1ab58`
("Wire the volume slider and arrow keys through the audio backend"); improve's
tick #63 (90 turns, 168.8k ctx) then committed `b4dc9fd` at 01:57:11 from a
base that still listed the plan in `## Planned`. `git diff cd1ab58 b4dc9fd`
shows only cosmetic differences (log wording, comment placement, one
`drop(state)`, test-assertion style) — no behavioral difference. `b4dc9fd` is
not an ancestor of main; it was review-rejected and its whole 0.3 h tick
discarded, recorded as an authoring rejection rather than as duplicate work,
so the race hid itself from the operator.

How to reproduce:
- `git show cd1ab58:PLANS.md` and `git show b4dc9fd:PLANS.md` both move the
  same heading "Drive the volume slider through the audio backend" from
  `## Planned` to `## Done` with the identical hunk (`index e4a8c8e..4efddfa`).
- `git diff cd1ab58 b4dc9fd -- src/audio.rs src/apple_music.rs src/ui/mod.rs`
  is comments, log strings, and one `drop(state)` only; the `Command::SetVolume`
  variant and its `run_worker` arm — the change the body is rejected for not
  making — are present in both.
- `git merge-base --is-ancestor b4dc9fd HEAD` fails while `cd1ab58` is in main;
  `git reflog refs/heads/tumwater/improve` shows `b4dc9fd` (01:57:11)
  immediately followed by `reset: moving to main` with no rework, so the tick
  was abandoned rather than fixed.
- The digest prices it as improve's only error-class row: `0.3 h · $0.06`.

Suspected cause: PLANS.md is the work queue, and a plan leaves `## Planned`
only when the completing commit merges into main. feature's `cd1ab58` sat in
an accepted-but-unmerged landing ref while improve's tick started from an older
main (`437c80c`, 01:29) that still showed the plan as open. No plan state marks
it claimed or in-flight, so the scheduler dispatched a second role onto it.
When the duplicate was reviewed against a main that already contained
`cd1ab58`, its net diff no longer added the `Command::SetVolume` path its body
claimed, so the gate rejected it as a claim mismatch. The fix is in the
scheduler/review seam (claim an open plan before dispatch, or reject a
candidate that is behaviorally equivalent to a pending landing ref as a
duplicate rather than as authoring failure), not in this repo.

## Fixed
