# Plans

## Planned

### Surface a stale Apple Music sign-in in the browse failure reports (classify HTTP 401/403 at the service seam)

(planned 2026-10-29 by the plan role, as option 1 of the open
QUESTIONS.md planner-directions question and after ground-truthing it)

**Goal.** A signed-in library fetch that Apple Music answers with HTTP 401 or 403 — a stale or
revoked MusicKit user token, an expired developer token — reaches the browse panel and stderr
today as `music-library fetch failed (…): HTTP 401 Unauthorized: <detail>`, indistinguishable
from every other REST failure and with no hint that the recovery is signing in again.
`UreqTransport::request` (`src/apple_music/rest/transport.rs`, the non-success branch at
`status.is_success()`) builds that bare-string `AppleMusicError`, and
`fetch_failure_report` (`src/ui/loading.rs`) formats it verbatim; neither carries a kind the
UI could read. This plan classifies auth rejections at the seam and lets the browse failure
report name them, without changing the fallback or the failure path.

**Ground truth checked this run.** A stored stale session does *not* fall back to the sample
library: `AppleMusicService::browse` (`src/apple_music/browse.rs`, the `Some(session) =>
off_thread(...)` arm) routes every stored-session query to REST and the error propagates to
the `*LoadFailed` report; the sample library answers only when no session is stored. But
`seed_session` (`src/apple_music.rs`, beside `validate_developer_token`/`validate_user_token`)
validates a saved token by format only, so a stale-but-well-formed token seeds a session whose
every fetch then 401s. `docs/usage.md`'s sign-in paragraph instead says a stale one's "queries
are answered by the sample library" — inaccurate for the stored-session path; this plan fixes
that paragraph in the same run.

**Approach.**

1. `src/music_error.rs` — the seam's error type carries the kind: add a private `kind` field
   to `AppleMusicError`, back it with a new `pub enum AppleMusicErrorKind { Auth, Other }`
   (`#[derive(Debug, Clone, PartialEq, Eq)]`; `AppleMusicError` already derives both, so
   equality keeps working off the derived kind). `new` keeps producing `Other`; add
   `pub fn new_auth(message: impl Into<String>) -> Self` — the constructor the transport uses
   for auth rejections — and `pub fn kind(&self) -> AppleMusicErrorKind`. `Display` stays the
   bare message; only the accessors are new. Update the module's four existing tests
   (`new_accepts_string_and_str`, `equality_and_clone`, …) for the added field.
2. `src/apple_music/rest/transport.rs` — in `UreqTransport::request`'s non-success branch
   (the `status.is_success()` check that currently builds `"HTTP {status}: {cause}"` or
   `"HTTP {status}"`), build the error with `AppleMusicError::new_auth(...)` when
   `status.as_u16()` is 401 or 403 and with `AppleMusicError::new(...)` otherwise. The message
   text stays byte-identical to today so the pinned transport tests
   (`a_non_success_status_surfaces_the_api_error_detail`,
   `a_non_success_status_with_a_blank_api_error_detail_reports_just_the_status`,
   `a_non_success_status_without_an_api_error_body_reports_just_the_status`, all in
   `src/apple_music/rest/tests/transport.rs`) keep passing.
3. `src/ui/loading.rs` — make the report auth-aware. Verify first: `fetch_into`'s callers
   (`fetch_level` and the `loading_tests` call sites) all map service browse queries whose
   errors are already `AppleMusicError`, so narrow `fetch_into`'s (`fetch_timeout`-sibling)
   `E: std::fmt::Display + Send + 'static` bound to the concrete `AppleMusicError`.
   `fetch_failure_report` then takes `&AppleMusicError`: when `error.kind()` is
   `AppleMusicErrorKind::Auth` it appends one hint line naming the recovery, held in a named
   constant (exact wording, e.g. `"; Apple Music rejected the signed-in request — sign in
   again and restart the player"`) so the test pins it; non-auth reports stay byte-identical
   to today's pinned format. `fetch_failure_report`'s report string is both the stderr line
   and the text `*LoadFailed` shows in the browse panel (see `browse_failure_report`'s doc
   comment in `loading.rs`), so both surfaces get the hint with no other change. Leave
   `play_failure_report`, `replay_failure_report`, `transport_failure_report`, and both
   timeout reports generic and untouched — the preview download (`src/audio/preview.rs`) does
   not carry Music-User-Token, so the play path has no auth failure to surface yet.
4. Docs, same run. `docs/usage.md`: replace the inaccurate stale-token sentence in the sign-in
   paragraph (around lines where it says a stale saved token's "queries are answered by the
   sample library") with the real behavior — a stale stored token's queries fail with the
   browse panel's failure report naming the rejected sign-in — and name the new hint. Also
   append a short section to `docs/invariants.md` next to `## REST transport` (e.g. `## REST
   auth failures`): Guarantee = 401/403 classified as `AppleMusicErrorKind::Auth` and the
   browse report carries the recovery hint; Enforced by = `UreqTransport::request`'s
   401/403 branch and `fetch_failure_report`; Pinned by = the new tests named below.

**Acceptance criteria.**

- `src/apple_music/rest/tests/transport.rs`, beside
  `a_non_success_status_surfaces_the_api_error_detail`: a 401 reply and a 403 reply produce
  errors whose `kind()` is `AppleMusicErrorKind::Auth` and whose messages equal today's
  strings; a 404 (the existing test) or a 500 stub reply produces `AppleMusicErrorKind::Other`.
- `src/music_error.rs` tests: `AppleMusicError::new_auth("x")` reports
  `AppleMusicErrorKind::Auth`, `AppleMusicError::new("x")` reports `Other`, and the kind
  survives `Clone` and `PartialEq` comparison against a like-built value.
- `src/ui/loading.rs` test module: an auth-kind error reports the pinned full format
  (`fetch_failure_report` output = today's format plus the exact hint constant); a
  `AppleMusicError::new("boom")` reports byte-identically to the existing
  `fetch_failure_report_names_the_fetch_and_includes_the_error` pin (update that test's
  construction from `&"boom"` to `AppleMusicError::new("boom")`); the stderr line and the
  `*LoadFailed` payload remain the same string.
- Nothing else changes: `browse`'s sample fallback (`None => Ok(sample())`), the
  `*LoadFailed`/Retry flow, and all UI chrome are untouched — no screenshot comparison is
  needed.
- `make check` passes.

**Size.** Four source files plus the two docs pages; new code and tests together well under
two hundred lines. No dependency added. Out of scope: surfacing the hint on the play, replay,
or transport-action failure reports; prompting a re-auth from the UI; verifying a saved token
against the network at seed time.

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
