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

### Clamp the restored window position to the monitor the window opens on, so a saved off-screen point cannot hide the player (planned 2026-10-30 by the plan role)

**Goal.** `docs/usage.md`'s settings paragraph tells a user whose window opened off-screen
that they must "delete the `window_position` line to reset an off-screen window": the saved
screen position is restored verbatim with no bound (`launch_position` maps the saved point to
`iced::window::Position::Specific`), and `docs/invariants.md`'s Settings persistence section
pins that as "a set value is passed to `.position(...)` through `launch_position` without
clamping to monitors (iced cannot query them before the window opens)". iced 0.14 can clamp
at open time after all: `iced::window::Position::SpecificWith(fn(Size, Size) -> Point)` hands
the opening window its own logical size and its monitor's logical resolution, and the window
system open path applies the returned point relative to that monitor's origin — so an
unbounded saved point can be brought back on screen before the window exists. This entry
clamps the restored point by that rule, with the clamp's exact arithmetic, its fallbacks,
and the doc anchors pinned here so nothing is left open.

**Ground truth verified this run (2026-10-30).** `src/ui/handlers/window.rs::launch_position`
maps `Some((x, y))` to `iced::window::Position::Specific(iced::Point::new(x, y))` and `None`
to `Position::Default`; its doc comment still claims iced cannot query monitors before open,
which is stale ground truth now — but the API is a `fn` pointer, so the saved coordinates
cannot ride in a closure and must reach the function another way (a one-write-once
`std::sync::OnceLock` set in `launch_position` before the pointer is handed over). The window
system applies `settings.position` in `iced_winit-0.14.1/src/conversion.rs::position` against
the primary monitor: `Position::SpecificWith(to_position)` calls `to_position(window_size,
monitor_logical_resolution)` and treats the returned `Point` as an offset from
`monitor.position()` (falling back to the window system's own default when no monitor is
reported — the same fallback `Position::Default` gets). The saved position itself is logical
(`window_moved` stores the `iced::Point` the `Moved` event delivers, written by the
persistence machinery as `window_position=<x>,<y>` in `src/settings.rs`), so all three sizes
the clamp sees are logical and share one coordinate space. The restore path's own tests sit
in `src/ui/tests/settings_tests.rs` (`launch_position_opens_at_the_saved_point_or_the_default`),
and the persistence guarantee's doc home is the `## Settings persistence` section of
`docs/invariants.md` (the sentence naming `launch_position`). A grep of `PLANS-DONE.md` and
`BUGS-FIXED.md` for `clamp`, `monitor`, and `off-screen` shows the landed window-position
feature ("Remember the window's screen position across launches") has no clamp coverage —
its entry and the invariant both record the unclamped behavior as a limitation. Wayland is
unaffected by design there: it reports no window moves, writes no `window_position` line,
and keeps opening at the window system's default.

**Approach.**

1. `src/ui/handlers/window.rs`: add a module-private `static SAVED_LAUNCH_POSITION:
   std::sync::OnceLock<(f32, f32)>` and a pure helper `fn clamp_into_monitor(point:
   iced::Point, window: iced::Size, monitor: iced::Size) -> iced::Point`, with the rule
   pinned exactly: clamp each axis independently — `point.x.clamp(0.0, (monitor.width -
   window.width).max(0.0))`, the same for `y` — so the window opens fully inside the monitor
   it opens on, and a monitor smaller than the window leaves the point at `0` on that axis.
   Rewrite `launch_position`'s saved arm to store the point in the `OnceLock` (`set`; the
   first write wins, and `init_ui` calls this once at boot) and return
   `iced::window::Position::SpecificWith(clamped_saved_position)`, where
   `clamped_saved_position(window, monitor)` reads the `OnceLock` through
   `clamp_into_monitor` (an unset lock — only a test's default path — clamps
   `Point::ORIGIN`). Keep the `None` arm as `Position::Default`.
2. `src/ui/handlers/window.rs` doc comment: replace the "the position is not clamped to the
   monitors, since iced cannot query them before the window opens" sentence with the new
   truth — the clamp happens inside the window system's open path (`Position::SpecificWith`
   consults the monitor at creation), anchored to the monitor the window opens on, and the
   launch monitor's origin is not queried, which is the pinned trade-off below.
3. `docs/usage.md` settings paragraph: replace "The window reopens at its saved position
   without checking that a monitor is still there; delete the `window_position` line to
   reset an off-screen window." with the clamped behavior: a saved position now opens
   clamped inside the monitor the window opens on, so a point saved for a monitor that is
   no longer there (or for a monitor smaller than the player) reopens on screen at the
   nearest edge instead of hiding the window; a point carried over from another monitor is
   re-anchored to the monitor the window opens on. Keep the Wayland sentence unchanged.
4. `docs/invariants.md` `## Settings persistence`: replace the "a set value is passed to
   `.position(...)` through `launch_position` without clamping to monitors" sentence with
   the new guarantee (clamped at open time by `clamp_into_monitor`, per the rule in
   approach item 1), enforced by `launch_position` and pinned by the tests below.

Nothing in the window's chrome, layout, size, or rendering changes — this only bounds the
point handed to the window system at open, so no `winamp-screenshots/` comparison applies.

**Acceptance criteria.**

- `src/ui/tests/settings_tests.rs`: `launch_position(Some((10.0, 20.0)))` now matches
  `iced::window::Position::SpecificWith(_)` and `launch_position(None)` still matches
  `iced::window::Position::Default` (the renamed twin of
  `launch_position_opens_at_the_saved_point_or_the_default`).
- New tests beside it pin `clamp_into_monitor`'s rule: an in-bounds point returns
  unchanged; a point beyond the monitor's right or bottom edge is clamped to `monitor
  extent - window extent` while its other axis passes through independently; a negative
  coordinate clamps to `0`; a monitor smaller than the window clamps both axes to `0`.
- `make check` passes.

**Size.** One `OnceLock`, one pure clamp helper (its rule is four `clamp` expressions),
one enum-arm swap and one doc comment in `src/ui/handlers/window.rs`, two doc anchors —
roughly sixty lines including the five tests. Out of scope: any monitoring of unplugged
monitors after the launch, any per-monitor origin correction (iced 0.14 has no monitor
list with origins — `iced_runtime-0.14.0/src/window.rs` exposes only `monitor_size`),
any change to what `window_moved` stores or what Wayland does, and anything in the UI
chrome.

## Done
