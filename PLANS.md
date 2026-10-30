# Plans

## Planned

_None yet._

### Surface Apple Music's HTTP 429 in the browse failure reports (classify the rate limit at the service seam) (planned 2026-10-30 by the plan role)

**Goal.** When Apple Music throttles a signed-in request the REST transport answers
HTTP 429, and today it lands in the browse panel's failure report indistinguishably from a
hard failure: `src/apple_music/rest/transport.rs`'s non-success branch classifies only
401/403 (the landed `AppleMusicErrorKind::Auth` classification) and every other status
takes the plain-`AppleMusicError` arm, so `fetch_failure_report` (`src/ui/loading.rs`)
renders a bare `HTTP 429` with no hint that the failure is transient — the panel already
has a Retry button, the user just is not told the retry is the right recovery. This entry
gives the rate limit the same treatment the 401/403 classification got: a transport-level
kind, a report hint, and tests, with every wording and status choice pinned here so
nothing is left open.

**Ground truth verified this run (2026-10-30).**
`src/apple_music/rest/transport.rs::UreqTransport::request`'s non-success branch maps
`401 | 403 => AppleMusicError::new_auth(message)` and `_ => AppleMusicError::new(message)`;
`src/music_error.rs`'s enum holds only `AppleMusicErrorKind::Auth`, its `new_auth`
constructor, and the Display impl that renders the message bare (pinned by the
`AppleMusicError::new_auth("HTTP 401").to_string()` assertion in its test module); the
hint lives in `src/ui/loading.rs`'s `fetch_failure_report` as `AUTH_RECOVERY_HINT`, appended
when `err.kind() == AppleMusicErrorKind::Auth`; the transport's classification tests spin a
real tiny HTTP server (`http_response`/`serve_one_response` in
`src/apple_music/rest/tests/transport.rs`, e.g.
`an_http_401_reply_is_classified_as_an_auth_error_with_todays_message`), so a 429 test
follows that same pattern; the status line is already in hand before the body is read, so
reading the response's `Retry-After` header costs nothing. A grep of `PLANS-DONE.md` and
`BUGS-FIXED.md` for `429`, `rate limit`, and `retry`/`backoff` shows no entry covers this
(the browse panel's Retry *button* is already landed — and is what the new hint points
to).

**Approach.**

1. `src/music_error.rs`: add `AppleMusicErrorKind::Throttled` beside `Auth`, with
   `pub fn new_throttled(message: impl Into<String>) -> AppleMusicError` mirroring
   `new_auth`'s doc-comment style (what a 429 means: Apple Music counted too many requests
   from this token and asks the client to back off), `kind()` returning it in the `match`,
   and no change to `Display` — the message still renders bare.
2. `src/apple_music/rest/transport.rs`: in `UreqTransport::request`'s non-success branch,
   before building `message` per the existing `api_error_cause` shape, read the response's
   `Retry-After` header; when it parses to a non-negative `u64` of seconds, append
   ` (retry after {n} s)` to the message, and map `429` to
   `AppleMusicError::new_throttled(message)` in its own arm. Every other status (including
   the 500 that `an_http_500_reply_classifies_as_other` pins) stays on the plain arm, and
   nothing echoes an unparsed `Retry-After` value.
3. `src/ui/loading.rs`: beside `AUTH_RECOVERY_HINT`, add
   `const THROTTLED_RECOVERY_HINT`, and append it in `fetch_failure_report` when
   `err.kind() == AppleMusicErrorKind::Throttled`, keeping the non-throttled report
   byte-identical to today's. Do not touch `play_failure_report`/
   `transport_failure_report`: the Auth precedent appends its hint in the browse report
   only.
4. `docs/usage.md`: extend the sign-in paragraph's failure sentence (the "Apple Music
   rejected the signed-in request — sign in again and restart the player" one, in the
   "Once a stale token does seed a session…" flow) with the rate-limit twin: a throttled
   query's report is named so the control reference stays current.
5. `docs/invariants.md`: open a sibling `## REST rate limiting` section beside
   `## REST auth failures` with the 429 → `AppleMusicErrorKind::Throttled` classification
   and the hint, enforced by the same two code points, pinned by the tests below.

**Acceptance criteria.**

- A test in `src/apple_music/rest/tests/transport.rs` (beside the 401/403 ones, through
  `http_response("429 Too Many Requests", …)` plus a `Retry-After: 5` header, served by
  `serve_one_response`) proves the classification: a fetch through the real transport fails
  with `kind() == AppleMusicErrorKind::Throttled` and message carrying `HTTP 429` and
  `(retry after 5 s)`; a second test pins the no-header message without the parenthetical;
  a third pins that a malformed `Retry-After` renders without the parenthetical.
- `src/music_error.rs`'s test module: `new_throttled` carries the Throttled kind and
  renders bare through `Display` (the twin of
  `new_auth_carries_the_auth_kind_and_new_the_other_kind`), and the kind survives `clone`
  and `==` alongside the Auth one (the twin of
  `the_kind_survives_clone_and_equality`).
- `src/ui/loading.rs`'s test module: a throttled report appends `THROTTLED_RECOVERY_HINT`
  (the twin of `an_auth_error_report_appends_the_sign_in_hint`), and the existing
  plain-error test pins that a non-throttled failure still renders byte-identically with
  no hint.
- `make check` passes.

**Size.** One enum variant with its constructor, one transport arm and one header read,
one hint constant with its append, the twins of five already-landed tests, and two doc
anchors — roughly a hundred lines including tests, mirroring the landed 401/403
classification's shape. Out of scope: any automatic retry or backoff (the panel's Retry
button is the recovery), any classification change for other statuses (5xx stays plain,
as today's test pins), the playback and transport failure reports, and anything in the UI
chrome.

## Done
