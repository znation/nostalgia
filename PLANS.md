# Plans

## Planned

_None yet._

### Reuse the saved MusicKit sign-in at the next launch instead of re-running the browser flow (planned 2026-10-29 by the plan role)

**Goal.** With `APPLE_MUSIC_DEVELOPER_TOKEN` set, every launch opens the browser-based MusicKit
sign-in again (`init_service` → `start_sign_in` → `authorize`, even when the previous run succeeded
seconds ago): the relaunch costs a browser detour, and everything signed-in — browse, search,
Recently Added/Played, and the resolved restored-track name the entry above resolves — waits for the
session to land while the sample library answers. The session struct holds only the
developer token (env-owned, `startup_token`) and the user token MusicKit JS returned
(`MusicKitSession.user_token`, persisted nowhere today: `grep`-testable absence — no token field
exists in `Settings`), so the user token is exactly the piece that can make a relaunch feel like a
reboot instead of a re-login. Persist the successful sign-in's user token in the settings file, and
at the next launch reseed a session from it — the same `MusicKitSession::session()` the browser path
stores — so the player boots straight into the signed-in library with no browser and no waiting.

A settled design choice, so no implementer decision is left open: the token is a bearer value, and
it lands next to volume, balance, and the last-played id in the user's own XDG config file,
"`music_kit_user_token=…`" — stdlib-first, matching every other persisted knob here; a keyring
dependency is deliberately added for nothing. The developer token is never saved — it keeps coming
from the environment, so a seeded relaunch refuses when the variable is unset. The stale-token rule
(no expiry parsing, no revocation detection): a saved token Apple Music stops answering degrades
exactly like today's failed signed-in query — the browse handler folds the `Err` and the sample
library answers, no error shown, and the stale line stays in the file until the next successful
browser sign-in overwrites it; a user who wants a fresh flow anyway deletes the
`music_kit_user_token=` line (with the variable still set) and relaunches.

**Approach.** A validated reseed via `seed_session`, a getter the player's save path reads, the
`init_service` decision between reseed and browser thread.

- `src/music_kit_auth.rs`, beside `validate_developer_token`: add `pub fn validate_user_token(token:
  &str) -> Result<(), AppleMusicError>`, sharing the private `jwt_shape_problem` — a non-empty,
  three-segment base64url JWT (MusicKit JS user tokens are JWT-shaped, and the accepted alphabet is
  the same; a hand-edited settings line that breaks it is the common case to be rejected cleanly).
  Its error names the defect only — never the token — holding `validate_developer_token`'s
  redaction promise (a token must not reach a log), and the session's redacting `Debug` stays the
  last word on any panic path.
- `src/apple_music.rs`:
  - Lift the storing block of `authenticate_with` (the mutex write plus the
    `session_ready.notify_one()`) into `pub(crate) fn store_session(&self, session: MusicKitSession)`;
    `authenticate_with` becomes `authorize → store_session → Ok(())` with no behavior change, so
    the two session-arrival paths share one insert site.
  - On top of it, `pub fn seed_session(&self, developer_token: &str, user_token: &str) ->
    Result<(), AppleMusicError>`: trim the developer token (the same trim `authorize_with_bounds`
    applies at its boundary), run `validate_developer_token` and `validate_user_token`, and wrap
    them into a `MusicKitSession` — the same struct shape the browser flow returns — passed to
    `store_session`. An `Err` stores and notifies nothing.
  - `pub fn session_user_token(&self) -> Option<String>`, beside `session()` — clone the user token
    out from under the mutex; the player reads this at save time, and nothing else ever prints it.
  - `init_service` gains a second parameter: `pub fn init_service(state: Arc<Mutex<AppState>>,
    saved_user_token: Option<&str>) -> AppleMusicService`. After `startup_token` returns a
    developer token, try `service.seed_session(&developer_token, saved)` first: on `Ok`, log
    `Apple Music sign-in skipped: the saved sign-in was reused` and return without starting the
    sign-in thread; on `Err` (a blank, malformed, or absent saved value) fall through to
    `start_sign_in(service.clone(), developer_token)` exactly as today — the saved value is left
    alone and the browser flow may overwrite it. `session_ready` is notified inside
    `store_session`, and because the seed lands before `init_ui`, `wait_for_session` resolves
    immediately: the boot fetch is the first signed-in one, with no sample-library flash. Update
    the doc comments (the init one's skip-case list) and the call site in `src/ui/tests/
    threads_and_session.rs` (`init_service(state.clone(), None)`), and `src/main.rs` passes
    `saved.music_kit_user_token().as_deref()` — the settings snapshot `main` already loads before
    this line exists.
- `src/settings.rs`: a `music_kit_user_token: Option<String>` field following `window_position`'s
  pattern exactly — `Default` `None`, `from_state` leaves it `None` (the field list doc comment
  above the struct gets a new word: this is another value the player owns, not the state), and
  accessors `music_kit_user_token()` / `set_music_kit_user_token(Option<String>)` with the
  player-owns-it doc comments the panel choices carry.
- `src/settings/text.rs`: `to_text` writes `music_kit_user_token=<token>\n` only when `Some`
  (the `window_position` conditional, not `current_track`'s always-line — an absent token's absence
  is itself the clear-the-line documentation); `from_text` maps the key, a blank or missing value
  to `None`, any other text stored as-is — the shape check is `seed_session`'s job at boot, so the
  codec stays value-blind (the same rule as every other string the file carries).
- `src/ui/player.rs`, in `persist_settings_if_changed` after `set_window_position`:
  `snapshot.set_music_kit_user_token(self.apple_music_service.session_user_token());` — one line.
  Consequences worth the doc-comment sentence: no token stored → `None` equals the default and
  nothing changes; the first persist after a browser sign-in sees `Some` and writes the line (the
  next poll message carries it — no dedicated save call); a seeded launch's snapshot equals
  `saved_settings`, so it rewrites nothing. The token reaching the file needs no other wiring than
  the player holding the same service clone it already holds.
- Tests:
  - `src/ui/tests/settings_tests.rs`, beside `saved_current_track_survives_the_persist_round_trip`:
    `the_stored_sessions_token_reaches_the_saved_snapshot` (`test_player()`, then seed the player's
    service through `seed_session("dev-token", valid)`, one `update` with `Message::Ignored`, and
    the snapshots last value holds `Some(valid)` — the token change alone triggers the write) and
    `an_unchanged_token_writes_nothing` (seed, persist once, `Message::Ignored` again, snapshot
    count unchanged).
  - Settings codec round trip beside `current_track_round_trips_and_defaults_to_none`:
    `music_kit_user_token_round_trips_and_defaults_to_none` (Some round trips through `to_text` →
    `from_text`; a blank value, a missing line, and garbage text each land on `None` in
    `from_text`, and `to_text` on `None` writes no token line).
  - `src/apple_music/tests/browse_rest.rs` or the session tests in
    `src/apple_music/tests/threads_and_session.rs`: `seed_session_stores_a_valid_pair`
    (`test_service()`, `seed_session(" dev-token ", user)` → `session()` holds the trimmed
    developer token and the user token), `seed_session_rejects_a_malformed_user_token_without
    _storing` (blank, unwrapped text, a two-segment string → `Err` and `session().is_none()`),
    and the service-side accessor `session_user_token_answers_the_stored_token` (None before, Some
    after).
  - `src/music_kit_auth.rs` tests, beside `validate_developer_token`'s: `validate_user_token
    _accepts_a_jwt_shape_and_rejects_malformed_values` — accept `h.h.h`-style segments only; reject
    blank, one segment, two segments, whitespace in a segment, and a non-base64url character, each
    message naming the defect and holding no token text (assert on a substring that cannot include
    the token, per the existing rejection-message style).
- Docs:
  - `docs/usage.md`, sign-in paragraph: after the `APPLE_MUSIC_DEVELOPER_TOKEN` sentence, say the
    last successful sign-in's user token is saved in the settings file, that a saved token plus the
    variable relaunches straight into the library with no browser, that a blank, malformed, or
    stale saved token is ignored or fails over like today (the sample library answers a stale one's
    queries, delete the `music_kit_user_token=` line and relaunch to force a fresh flow), and that
    the developer token itself is never saved and the token is stored unencrypted in the settings
    file, like the rest of its content; adjust the settings-file list sentence ("the volume,
    balance, … are saved … and restored") to include the saved sign-in token.
  - `docs/invariants.md`, `## Settings persistence`: extend the persisted-fields sentence with the
    token and state the reimplementer's rule — a non-blank valid saved token seeds the session at
    boot (`init_service`'s reseed arm through `seed_session` → `store_session`), an invalid one
    falls through to the browser flow, and the browser path's first save records the new token —
    and note the plaintext placement as the settled design. `## Sign-in server`: add that the
    seeded relaunch never starts the loopback listener — the browser flow is the only caller.

No visible surface exists: the window, its chrome, the menus, and the bar render exactly as today
(see `docs/screenshots/player.png` and `winamp-screenshots/winamp-2.05-main.jpg`); the only sign a
user sees is stdout's skip line and a first fetch that is already signed-in. This entry is
independent of the entry above it — different files (settings, auth, `main`, `player.rs`'s save
path) — and can merge in either order.

**Files touched.** `src/music_kit_auth.rs`, `src/apple_music.rs`, `src/apple_music/tests/
threads_and_session.rs`, `src/main.rs`, `src/settings.rs`, `src/settings/text.rs`,
`src/ui/player.rs`, `src/ui/tests/settings_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- After one successful browser sign-in, the settings file gains a `music_kit_user_token=` line with
  the token the session holds, and the next launch with `APPLE_MUSIC_DEVELOPER_TOKEN` set reuses it:
  no browser flow, the `nostalgia-signin` thread never starts, `wait_for_session` resolves
  immediately, and the first artists fetch is already the signed-in one — a seeded relaunch reads
  the fetched library and not the sample rows, with no sample-library flash.
- A fresh install (no token line, variable set) signs in through the browser exactly as today, and
  the first save afterwards records the token — no dedicated save action is needed.
- A blank or malformed saved token matches today exactly: the browser flow runs, no error names the
  token, and a later success overwrites the line.
- With `APPLE_MUSIC_DEVELOPER_TOKEN` unset, the saved token is inert: no session, no REST calls,
  the sample library, and the existing skip line names the reason as today.
- A token Apple Music stops answering degrades to today's failed-query behavior — the handler folds
  the error and the sample library answers — and the stale line stays until a fresh sign-in or a
  hand-edited delete; no stdout/stderr line, panic message, or settings-file echo ever carries the
  token, and the developer token never reaches the settings file at all.
- `make check` passes.

## Done
