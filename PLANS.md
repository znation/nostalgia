# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; updated 2026-10-07).** The initial prompt
> names two commitments — a "very close if not pixel-perfect" classic Winamp UI
> and Apple Music as the library. The first is now moving: the base-skin
> palette landed ("Add a Winamp 2.x base-skin palette and apply it as the app
> theme", done 2026-10-06), and the reusable bevel layer followed ("Add a
> Winamp two-tone bevel layer and frame the Now Playing and equalizer panels",
> done 2026-10-06), framing those two panels through `bevel::lcd_well` and
> `bevel::raised_panel` in `src/ui/bevel.rs`. `## Planned` is no longer empty:
> it holds "Style the transport buttons as raised Winamp chrome", which
> restyles the five transport buttons and the browse Back button — all built
> through `labeled_button`, still a default iced `Button`. Still unplanned is
> the rest of widget-level fidelity: slider chrome (the bevel entry and the
> transport plan both defer it), a custom title bar, and playlist chrome. The
> library side is still all stub: `AppleMusicService` answers every browse
> query (`get_favorite_artists`, `get_albums_by_artist`,
> `get_songs_from_album`) from `sample_library()`, `play_track` only mutates
> shared state, and the token field is unset — and no PLANS.md entry plans the
> real integration. Risk: the longer Apple Music stays stubbed the more UI
> grows around the sample library's shape, while the remaining chrome work is
> easier to sequence now that the bevel layer exists. Recommend the plan loop
> keep scheduling the remaining fidelity plans (slider chrome, title bar,
> playlist chrome) and put the Apple Music integration plan on the near
> horizon.

## Planned

### Add the Apple Music REST browse client (found 2026-10-08)

The auth seam landed (`AppleMusicService` stores a `MusicKitSession`), but the
browse queries still answer from `sample_library()`. This entry adds the client
that turns a session plus the Apple Music REST API into the model's
`Artist`/`Album`/`Song` lists, behind a transport trait so its tests run
without a network. It changes no app behavior on its own; its sibling "Answer
the browse queries from the Apple Music REST API when signed in" wires it into
`AppleMusicService`.

**Depends on.** Nothing. The sibling wiring entry depends on this one.

**Goal.** A new `apple_music::rest` module with a synchronous `RestLibrary`
that issues the three library browse queries over the Apple Music API and maps
each JSON response onto `Vec<Artist>`/`Vec<Album>`/`Vec<Song>`, tested against
an in-memory stub transport with no network.

**Approach.**

- `Cargo.toml`: add `ureq = "3.4"` to `[dependencies]` with a comment naming
  it the blocking HTTP client for this integration (the choice recorded in
  `QUESTIONS.md` `## Answered`, 2026-10-08), leaving its default features
  (rustls TLS). Move `serde_json = "1.0.146"` from `[dev-dependencies]` to
  `[dependencies]` (the REST client parses responses in production now) and
  update its now-stale "Test-only" comment.
- A new `rest` submodule of `apple_music`, declared `pub mod rest;` in
  `src/apple_music.rs` beside its existing `#[cfg(test)] mod tests;`:
  - `pub trait HttpTransport: Send + Sync` with
    `fn get(&self, url: &str, session: &MusicKitSession) -> Result<String, AppleMusicError>`
    — the narrow seam the tests stub.
  - `pub struct UreqTransport;` implementing it: `ureq::get(url)` with an
    `Authorization: Bearer <developer_token>` header and a
    `Music-User-Token: <user_token>` header, `.call()`, map any `ureq::Error`
    to `AppleMusicError::new(...)`, then `response.body_mut().read_to_string()`.
    ureq 3's default `http_status_as_error` makes a 4xx/5xx an `Err` here, so a
    rejected request never parses as data. No error message includes a token.
  - `pub struct RestLibrary { transport: Box<dyn HttpTransport> }` with
    `pub fn new(transport: Box<dyn HttpTransport>) -> Self` and three
    synchronous methods, each taking `&MusicKitSession`:
    - `get_favorite_artists` → `GET {API_BASE}/me/library/artists`; map each
      resource's `id` and `attributes.name` to `Artist { id, name }`.
    - `get_albums_by_artist(session, artist_id)` →
      `GET {API_BASE}/me/library/artists/{artist_id}/albums`; map
      `attributes.name` to `Album::title` and set `Album::artist_id` to the
      `artist_id` argument (the query already scopes the results to it).
    - `get_songs_from_album(session, album_id)` →
      `GET {API_BASE}/me/library/albums/{album_id}/tracks`; map
      `attributes.name` to `Song::title` and set `Song::album_id` to the
      `album_id` argument.
  - `const API_BASE: &str = "https://api.music.apple.com/v1";` — the library
    endpoints need no storefront, so none is fetched.
  - Private serde DTOs for the `{ "data": [ ... ] }` envelope: a generic
    `Envelope<T> { data: Vec<T> }`, a `Resource { id: String, attributes: Option<Attributes> }`,
    and `Attributes { name: Option<String> }`. A resource whose `attributes.name`
    is absent or blank is an `AppleMusicError` naming its `id` rather than a
    blank row.
  - Each method runs `transport.get(...)`, then
    `serde_json::from_str::<Envelope<Resource>>(...)`, then maps; every failure
    (transport, JSON, a nameless resource) is an `AppleMusicError` whose
    message names the query.
  - Only the first page is read: the API's `next` link is ignored, documented
    as a known limit (Apple caps a page at 100 items).
  - The id is interpolated into the URL directly. The service layer (the
    sibling entry) runs the existing `ensure_id_is_valid` guard before calling
    here, and the ids are Apple-generated or from the sample library.
- Tests in the `rest` module's own `#[cfg(test)] mod tests;` child (mirroring
  how `apple_music` holds its tests): a `StubTransport` that records
  each `(url, session)` and returns a canned body or an `Err`, plus tests that
  each query maps canned library JSON to the expected model values, that the
  stub observes the expected URL and both session tokens, that a transport
  `Err`, malformed JSON, and a resource missing `attributes.name` each return
  an `AppleMusicError`, and that unknown extra attributes are tolerated.

**Files touched.** `Cargo.toml`, `src/apple_music.rs` (one `pub mod rest;`
line), and the new `rest` module and its tests submodule beside it.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary` maps canned Apple Music library JSON to the model for all
  three queries, and the stub observes the request URL and both session tokens.
- A transport error, malformed JSON, or a resource without a name returns an
  `AppleMusicError`; no test touches the network.
- `AppleMusicService` and the app are unchanged by this entry: the existing
  sample-library browse tests still pass.

### Answer the browse queries from the Apple Music REST API when signed in (found 2026-10-08)

The sibling entry "Add the Apple Music REST browse client" produces
`RestLibrary`; this entry makes `AppleMusicService` use it whenever a
`MusicKitSession` is stored, and keep the sample library otherwise. It lands
independently once that module's public API exists.

**Depends on.** "Add the Apple Music REST browse client" (`RestLibrary`,
`HttpTransport`, `UreqTransport`).

**Goal.** The three browse queries answer from the signed-in user's Apple
Music library when a session is stored and from `sample_library()` when not,
and the blocking HTTP call runs off iced's executor so it cannot freeze the UI.

**Approach.**

- `src/apple_music.rs`:
  - Add `rest: Arc<rest::RestLibrary>` to `AppleMusicService`.
  - `pub fn new(state)` builds it with `rest::UreqTransport`; add
    `pub fn with_transport(state, transport: Box<dyn rest::HttpTransport>) -> Self`
    and have `new` delegate to it, so tests inject a stub transport.
  - Each browse method: after the existing `ensure_id_is_valid`, match
    `self.session()`. `Some(session)` clones the `Arc<RestLibrary>` and the id,
    then runs `off_thread(move || rest.get_...(&session, &id)).await`; `None`
    runs the existing sample-library lookup. A REST failure propagates as an
    `AppleMusicError` (no silent fallback to the sample library), so the UI's
    existing `*LoadFailed`/Retry path reports it.
  - Add a private
    `async fn off_thread<T: Send + 'static>(work: impl FnOnce() -> Result<T, AppleMusicError> + Send + 'static) -> Result<T, AppleMusicError>`
    that spawns a `std::thread`, sends the result through a
    `tokio::sync::oneshot`, and awaits the receiver; a dropped sender (the
    thread panicked) becomes an `AppleMusicError`. This mirrors
    `init_service`'s sign-in thread and avoids `tokio::task::spawn_blocking`,
    which needs a tokio runtime iced's executor does not provide (see
    `ui::loading::with_timeout`).
  - Remove the now-live `#[allow(dead_code)]` from `session()` and update its
    doc, and update the module doc to say the browse queries use the session
    when one is stored.
- Tests in `src/apple_music/tests.rs`: keep `test_service()` on
  `AppleMusicService::new` (no session), so the existing sample-library tests
  pass unchanged. Add a helper that builds a service with a stub transport via
  `with_transport`, stores a session with `authenticate_with`, and then asserts
  each browse query returns the stub's mapped rows rather than the sample
  library and that a stub `Err` propagates; add a test that a session-less
  service still returns sample data even with a transport installed.

**Files touched.** `src/apple_music.rs`, `src/apple_music/tests.rs`.

**Acceptance criteria.**

- `make check` passes.
- With no session, all three browse queries return the sample library (the
  existing tests pass unchanged).
- With a stored session and a stub transport, each query returns the stub's
  parsed rows, the stub observes the URL and both tokens, and a transport `Err`
  propagates as an `AppleMusicError`.
- `session()` carries no `#[allow(dead_code)]`.
- Manual check (`cargo run` with a real `APPLE_MUSIC_DEVELOPER_TOKEN` and a
  completed sign-in): the artist → album → song browser shows the signed-in
  user's library; without the variable it still shows the sample library.

## Done

### Wire the MusicKit session into `AppleMusicService` (found 2026-10-08, done 2026-10-08)

The sibling entry "Add the MusicKit loopback authorization module" produces a
`MusicKitSession`; this entry stores it on the service and lets the app obtain
one at startup. It lands independently once that module's public API exists.

**Depends on.** "Add the MusicKit loopback authorization module"
(`MusicKitSession`, `authorize`, `open_in_browser`, `validate_developer_token`).

**Goal.** `AppleMusicService` holds the authenticated MusicKit session and can
obtain one from `APPLE_MUSIC_DEVELOPER_TOKEN` off the UI thread; the
sample-library queries keep answering until a later REST-integration plan
consumes the session.

**Approach.**

- `src/apple_music.rs`:
  - Remove `struct AppleMusicToken` and the `use serde::{Deserialize, Serialize};`
    import (the struct was its only serde user), and replace the
    `token: Option<AppleMusicToken>` field with
    `session: Arc<std::sync::Mutex<Option<crate::music_kit_auth::MusicKitSession>>>`
    so the blocking flow runs on a `std::thread` without a tokio runtime. Drop
    the old field's `#[allow(dead_code)]`.
  - `AppleMusicService::new` starts `session` as `None`.
  - `pub fn authenticate(&self, developer_token: &str) -> Result<(), AppleMusicError>`
    delegates to a private
    `authenticate_with(&self, developer_token, authorize: &dyn Fn(&str) -> Result<MusicKitSession, AppleMusicError>)`
    that stores the returned session under the mutex and leaves any previous
    session in place on error. The public method passes `browser_sign_in`, a
    private free function that calls
    `crate::music_kit_auth::authorize(dt, &crate::music_kit_auth::open_in_browser)`,
    so `authenticate` and `init_service` share one definition of the real flow.
    Both are synchronous and blocking by design; callers run them off the UI
    thread.
  - `pub fn session(&self) -> Option<MusicKitSession>` clones the stored session;
    it carries an `#[allow(dead_code)]` comment naming the future REST plan as
    its caller, matching the existing seam style.
  - `init_service(state) -> AppleMusicService`: build the one service, read
    `APPLE_MUSIC_DEVELOPER_TOKEN`; when it is set and non-blank, spawn a
    `std::thread` that runs the blocking sign-in on a *clone* via a private
    `sign_in` (which logs success or the error); otherwise log that sign-in is
    skipped and the sample library stays in use. The UI thread never blocks.
    Return the service so `main` hands that same instance to the UI — a clone
    shares the session handle, so the session stored at startup is the session
    the UI reads (the earlier version built a throwaway service inside the
    thread and dropped the session with it). `sign_in` is synchronous and
    separate from the spawn so a test can drive the startup path with a stub
    flow.
  - `src/main.rs`: capture the service `init_service` returns and pass it to
    `ui::init_ui`.
  - `src/ui/mod.rs`: `init_ui`, `boot`, and `WinampPlayer::new` take the
    service instead of each building one, so the UI shares the startup
    instance; `src/ui/tests.rs`'s `test_player` builds its own service for the
    UI tests.
  - Update the module doc and `AppleMusicError` docs to describe the session
    seam instead of the token stub.
- `Cargo.toml`: the `serde` `derive` comment names `apple_music` as a user;
  after this change only `library` uses the derive, so trim the comment.
- Tests in `src/apple_music/tests.rs`: delete the three `AppleMusicToken` serde tests
  (`apple_music_token_round_trips_through_json`,
  `apple_music_token_deserialization_rejects_missing_required_fields`,
  `apple_music_token_deserialization_ignores_unknown_fields`), plus their now-
  unused `sample_token`, `token_payload`, `use serde_json::json;`, and
  `assert_every_field_required` / `assert_serializes_as` /
  `assert_unknown_fields_tolerated` imports (those test-support helpers stay —
  `src/library.rs` still uses them); keep `assert_ids`. Add session tests.

**Files touched.** `src/apple_music.rs`, `src/apple_music/tests.rs`, `Cargo.toml`,
`src/music_kit_auth.rs` (removed the three now-stale `#[allow(dead_code)]`
markers and their wiring-plan comments, since the flow is live now),
`src/main.rs`, `src/ui/mod.rs`, `src/ui/tests.rs` (thread the one service
instance to the UI so its clone shares the startup session).

**Acceptance criteria.**

- `make check` passes.
- `AppleMusicService::new` has `session()` `None`.
- `authenticate_with` stores the `MusicKitSession` returned by a stub flow, so
  `session()` returns it; a stub flow returning `Err` propagates the
  `AppleMusicError` and leaves an already-stored session unchanged.
- The startup sign-in (`sign_in`, the body `init_service` runs on its thread)
  stores the session on the shared service: a clone survives the signing
  clone's drop and still reports the session, and `init_service` returns the
  service the UI is handed.
- The existing browse and playback tests still pass unchanged (the
  sample-library seam is untouched).
- Manual check (`cargo run` with a real developer token): the browser opens, a
  completed sign-in logs that the session was stored, and the browse UI behaves
  as before.

### Add the MusicKit loopback authorization module (found 2026-10-08, done 2026-10-08)

Answer to "How should Nostalgia authenticate to Apple Music?" chose the
MusicKit authorization flow (see QUESTIONS.md `## Answered`, decision
2026-10-08). The mechanism decided here is MusicKit JS served from a loopback
page opened in the system browser, not an embedded webview: the project's
principles prefer the standard library over a new dependency, native MusicKit
is macOS-only and cannot build on the Linux landing gate, and an embedded
webview would add a platform-specific dependency and have to be driven on
iced's winit event loop. The developer token stays owner-side
(`APPLE_MUSIC_DEVELOPER_TOKEN`, wired by the sibling plan) because signing the
ES256 JWT is a secret-keeping concern. This entry delivers the flow; the
sibling entry "Wire the MusicKit session into `AppleMusicService`" stores its
result.

**Depends on.** Nothing; the sibling wiring plan depends on this one.

**Goal.** A synchronous, standard-library-only module obtains a MusicKit *user
token* by running MusicKit JS `authorize()` in the user's browser and capturing
the token Apple's page posts back to a one-shot loopback HTTP server, returning
a `MusicKitSession` (developer token + user token).

**Approach.**

- A new `music_kit_auth` module beside `src/apple_music.rs`, declared
  `mod music_kit_auth;` in `src/main.rs` beside the other module declarations.
- `pub struct MusicKitSession { pub developer_token: String, pub user_token: String }`
  derives `Clone, PartialEq, Eq` and implements `Debug` by hand, printing
  `"<redacted>"` for both token fields so a session never leaks into a log or a
  failed-assertion message.
- `pub fn validate_developer_token(token: &str) -> Result<(), AppleMusicError>`:
  accepts exactly three non-empty dot-separated segments of `[A-Za-z0-9_-]`
  (a JWT shape). This rejects a blank or malformed token and guarantees the
  value is safe to interpolate into the page's JavaScript string.
- `pub fn authorize(developer_token: &str, open_url: &dyn Fn(&str) -> std::io::Result<()>) -> Result<MusicKitSession, AppleMusicError>`
  delegates to a private `authorize_with_timeout(developer_token, open_url, AUTH_TIMEOUT)`
  where `const AUTH_TIMEOUT: Duration = Duration::from_secs(300)`, so a test can
  pass a short deadline.
  - `validate_developer_token` first.
  - Binds `std::net::TcpListener` to `127.0.0.1:0`, reads the assigned port,
    and builds a `state` nonce by hex-formatting
    `std::collections::hash_map::RandomState::new().build_hasher().finish()`
    (std-only randomness).
  - Calls `open_url(&format!("http://127.0.0.1:{port}/?state={nonce}"))`; an
    `Err` becomes an `AppleMusicError`.
  - Sets the listener non-blocking and loops `accept()` with a short
    `thread::sleep` until `POST /token` arrives or the deadline passes (a
    deadline error otherwise).
  - `GET /` answers `200` with `AUTH_PAGE`, a `const &str` whose
    `{{DEVELOPER_TOKEN}}` and `{{STATE}}` placeholders are replaced by the real
    values. The page loads
    `https://js-cdn.music.apple.com/musickit/v3/musickit.js`, on `musickitloaded`
    calls `MusicKit.configure({ developerToken, app: { name: "Nostalgia", build: env!("CARGO_PKG_VERSION") } })`
    then `MusicKit.getInstance().authorize()`, and POSTs
    `state=<nonce>&userToken=<token>` (form-urlencoded) to `/token`. The
    loopback origin is a browser secure context, so no TLS is needed; if Apple
    rejects the origin, the page's error is surfaced and the token's `origin`
    claim is the first thing to check.
  - `POST /token` parses the request line, headers, and `Content-Length` body by
    hand (std-only; `serde_json` is test-only), reads `state` and `userToken`,
    and returns the session only when `state` equals the nonce and `userToken`
    is a non-empty JWT by `validate_developer_token`'s segment rule. Any other
    request gets `404` and the loop continues; a bad callback gets `400`. The
    success body tells the user the browser tab can be closed.
- `pub fn open_in_browser(url: &str) -> std::io::Result<()>` dispatches on
  `cfg!(target_os = "macos")` → `open`, `cfg!(target_os = "windows")` →
  `cmd /C start "" <url>`, otherwise `xdg-open`, returning the spawn result.
- `authorize`, `open_in_browser`, and `validate_developer_token` carry an
  item-level `#[allow(dead_code)]` with a comment naming the sibling wiring plan
  as the caller, matching the existing seam allowances in `src/apple_music.rs`;
  the wiring plan removes it.
- Module tests live in the `music_kit_auth` module and drive the callback with
  a fake `open_url` closure that spawns a `std::thread`.

**Files touched.** the new `music_kit_auth` module, `src/main.rs`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo doc` with rustdoc warnings denied, `cargo test`).
- `validate_developer_token` accepts a three-segment base64url JWT and rejects a
  blank value, a one-segment value, and a value containing `"` or `\`.
- `MusicKitSession`'s `Debug` output contains neither token.
- With a fake opener that GETs `/` and then POSTs the matching `state` and a
  sample JWT `userToken`, `authorize` returns a session carrying both tokens,
  and the served page contains the developer token and the nonce.
- `authorize` rejects a blank or malformed developer token; rejects a callback
  whose `state` differs from the nonce and one whose `userToken` is empty; and a
  fake opener that never calls back makes `authorize_with_timeout` return a
  timeout error within a short test deadline instead of hanging.
- Manual check (`cargo run` once the sibling wiring plan lands): with a real
  `APPLE_MUSIC_DEVELOPER_TOKEN`, the system browser opens the sign-in page, and
  completing sign-in logs that a session was stored.

### Add classic Winamp main-window keyboard shortcuts for transport and volume (found 2026-10-08, done 2026-10-08)

Winamp is driven from the keyboard as much as the mouse: Z previous, X play,
C pause, V stop, B next, ArrowUp/ArrowDown volume. Nostalgia's transport and
volume are mouse-only today — `src/ui/mod.rs`'s `init_ui` installs no
subscription and nothing in `src/` reads `iced::keyboard` — so this adds the
classic bindings on top of the existing transport messages.

**Goal.** With the player window focused, the classic Winamp keys perform the
same actions as the transport buttons and volume slider: Z steps back, B
steps forward, X plays, C pauses, V stops, and ArrowUp/ArrowDown raise/lower
the volume by `views::VOLUME_STEP`, clamped to `[0.0, 1.0]` by the existing
`AppState::set_volume`.

**Approach.**

- `src/ui/mod.rs`:
  - Add the pure key→message mapping beside `with_window_id`/`mutate_state`,
    where the module already keeps its message-wiring helpers:
    - `fn message_for(event: iced::keyboard::Event) -> Option<Message>`:
      matches `Event::KeyPressed { key, modifiers, .. }` and returns
      `shortcut(&key, modifiers)`; every other event (`KeyReleased`,
      `ModifiersChanged`) returns `None`.
    - `fn shortcut(key: &iced::keyboard::Key, modifiers: iced::keyboard::Modifiers) -> Option<Message>`:
      returns `None` when `modifiers.control() || modifiers.alt() ||
      modifiers.logo()`, so OS/browser chords are left alone; Shift is allowed
      and the character match uses `eq_ignore_ascii_case`, so Shift+Z works.
      - `Key::Character(c)`: `"z"` → `PreviousTrack`, `"x"` → `Play`, `"c"` →
        `Pause`, `"v"` → `Stop`, `"b"` → `NextTrack`.
      - `Key::Named(key::Named::ArrowUp)` → `VolumeUp`,
        `Key::Named(key::Named::ArrowDown)` → `VolumeDown`.
  - Add `Play`, `Pause`, `VolumeUp`, `VolumeDown` to `Message`.
  - `update` arms, all through the existing `mutate_state` helper so each
    schedules no follow-up work: `Play` → `AppState::play`, `Pause` →
    `AppState::pause`, and `VolumeUp`/`VolumeDown` → a closure that reads
    `state.volume()`, adds/subtracts `views::VOLUME_STEP`, and passes the
    result to `state.set_volume(..)` (the setter clamps).
  - In `init_ui`, add
    `.subscription(|_player| iced::keyboard::listen().filter_map(message_for))`.
    `message_for` is a plain fn item, which is the non-capturing, zero-sized
    mapper `Subscription::filter_map` requires.
- `src/ui/views.rs`: change `const VOLUME_STEP` to `pub(super) const
  VOLUME_STEP` so the update loop reuses the slider's step instead of a second
  copy.
- `src/state.rs`: add `pub fn play(&mut self)` (sets `is_playing = true`) and
  `pub fn pause(&mut self)` (sets `is_playing = false`) beside
  `toggle_playing`/`stop`, with doc comments naming the keyboard's X and C as
  the explicit callers and noting that Pause and Stop are the same flag change
  today (the real-playback seam the `Stop` doc already names).
- `src/ui/tests.rs`: the mapping and update-arm tests (below).
- `README.md`: name the keys in Usage.

**Files touched.** `src/ui/mod.rs`, `src/ui/views.rs`, `src/state.rs`,
`src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo doc` with rustdoc warnings denied, `cargo test`).
- `src/ui/tests.rs` mapping tests: each bound key maps to its message
  (`matches!` on the returned `Option<Message>`); the uppercase forms map the
  same; a `Modifiers::CTRL`, `ALT`, or `LOGO` modifier yields `None`; an
  unbound key, a `KeyReleased`, and a `ModifiersChanged` event yield `None`;
  and one full `Event::KeyPressed` drives `message_for` end to end.
- `src/ui/tests.rs`: driving `Message::Play` sets `state.is_playing` and
  `Message::Pause` clears it; `Message::VolumeUp`/`VolumeDown` move
  `state.volume()` by `views::VOLUME_STEP` and clamp at `1.0`/`0.0`; all four
  arms schedule no follow-up work via `assert_message_schedules_no_work`.
- Manual check (`cargo run`): with the window focused, Z/B step tracks, X/C/V
  play/pause/stop, and the arrow keys move the volume slider; typing a letter
  in no other widget is affected because there are no text inputs yet.

### Add Winamp window-shade ("roll up") mode (found 2026-10-08, done 2026-10-08)

Found by plan 2026-10-08. The README's Screenshots note still says
"window-shade mode is not built yet", and it is the last widget-level fidelity
item the steward drift note and the title-bar Done entry leave unplanned. The
custom title bar already owns the drag/minimize/close actions; this plan adds
the classic double-click roll-up: the window shrinks to just the title bar and
its content, and double-clicking again restores it.

**Goal.** Double-clicking the title bar toggles shade mode. Shaded, the window
renders only the title bar and is resized to `TITLE_BAR_HEIGHT`; the pre-shade
inner size is remembered and restored on unshade.

**Approach.**

- `src/ui/views.rs`:
  - Make `TITLE_BAR_HEIGHT` `pub(super)` so the update loop can size the shaded
    window to it; keep its value `24.0`.
  - On the title bar's `drag_region` `MouseArea`, add
    `.on_double_click(Message::ToggleWindowShade)` beside the existing
    `.on_press(Message::WindowDragged)`, and extend `view_title_bar`'s doc
    comment to name the roll-up gesture.
- `src/ui/mod.rs`:
  - Add two `WinampPlayer` fields: `shaded: bool` (the window is rolled up)
    and `unshaded_size: Option<iced::Size>` (the inner size captured just
    before the first shade, so unshade can restore it); initialize both in
    `WinampPlayer::new` (`false`, `None`).
  - Add `ToggleWindowShade` and `WindowShadeMeasured(iced::Size)` to
    `Message`.
  - `update` arms:
    - `ToggleWindowShade`: when `shaded`, clear it and, if `unshaded_size` is
      `Some(size)`, schedule `iced::window::resize(id, size)` through
      `with_window_id`; otherwise set `shaded` and schedule
      `iced::window::size(id).map(Message::WindowShadeMeasured)` through
      `with_window_id`. Without a resolved window id the flag still flips and
      no resize is scheduled — the same guard the other title-bar actions
      use.
    - `WindowShadeMeasured(size)`: store `Some(size)` and schedule
      `iced::window::resize(id, iced::Size::new(size.width,
      views::TITLE_BAR_HEIGHT))` through `with_window_id`.
  - In `view`, return `views::view_title_bar()` immediately when
    `player.shaded`, before the state lock and the content column, so shade
    mode builds only the title bar.
- `src/ui/tests.rs`:
  - `toggle_window_shade_flips_the_flag_and_measures_the_window`: with a
    window id, drive `ToggleWindowShade`, assert `player.shaded` is true and
    the task schedules work (`iced_runtime::task::into_stream(..).is_some()`);
    drive it again, assert `shaded` is false and work is still scheduled once
    `unshaded_size` is set.
  - `window_shade_measured_stores_the_size_and_resizes`: with a window id,
    drive `WindowShadeMeasured(iced::Size::new(800.0, 600.0))`, assert
    `player.unshaded_size == Some(iced::Size::new(800.0, 600.0))` and work is
    scheduled.
  - `toggle_window_shade_without_a_window_id_still_flips_the_flag`: with no
    window id, drive `ToggleWindowShade`, assert `shaded` is true and no work
    is scheduled (the `assert_message_schedules_no_work` probe).
  - `view_constructs_when_the_window_is_shaded`: set `player.shaded = true`
    and build `view` for each `BROWSE_VIEWS` entry, empty and seeded — the
    same no-panic contract as
    `view_constructs_over_the_apps_full_input_space`.
- `README.md`: drop "window-shade mode is not built yet" from the Screenshots
  note, add window-shade mode to the Status sentence, and name it in Usage;
  leave the `tumwater:prompt` block untouched.

**Files touched.** `src/ui/views.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`,
`README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo doc` with rustdoc warnings denied, `cargo test`).
- The four new tests above pass, and
  `view_constructs_over_the_apps_full_input_space` still passes unchanged.
- No `dead_code`/unused warnings: the new fields, messages, and `pub(super)`
  constant are all read by the update loop or its tests.
- Manual check (`cargo run`): double-clicking the title bar rolls the window
  up to a title-bar-height strip showing only the title bar; double-clicking
  again restores it; dragging the title bar still moves the window. The resize
  is best-effort: `iced` maps `window::resize` to winit's
  `request_inner_size`, which some windowing systems ignore for a
  non-resizable window; where it is ignored the content still switches to the
  title-bar-only view, and unshading needs no restore.

### Add Winamp equalizer preset curves and a preset pick list (found 2026-10-07, done 2026-10-07)

Found by plan 2026-10-07. The equalizer panel Done entry ("Add the Winamp
equalizer panel: on/off, preamp, and ten band sliders") deferred "preset
curves" to a later plan, and the README's Screenshots note still says preset
curves are not built. `src/equalizer.rs` holds only the band labels, the gain
range, and `clamp_gain`; `AppState` stores a flat curve with no notion of a
named preset. This plan lands the curves and the control that applies them,
and leaves audio processing (still stubbed) alone.

**Goal.** Give the equalizer the Winamp 2.x preset curves: a `Preset` table in
`src/equalizer.rs` (name, preamp, ten band gains), an `AppState` selection that
applies a preset's whole curve and remembers which preset is selected, and a
`PickList` in the equalizer panel that shows the selected preset's name (or
`"(none)"`) and applies one on selection. Moving any preamp or band slider by
hand makes the curve custom and clears the selection.

**Approach.**

- `src/equalizer.rs`:
  - Add `#[derive(Debug, Clone, Copy, PartialEq)] pub struct Preset { pub
    name: &'static str, pub preamp: f32, pub bands: [f32; BAND_COUNT] }` and
    `impl std::fmt::Display for Preset` returning `self.name` (so
    `iced::widget::PickList` can render it).
  - Add `pub const PRESETS: [Preset; 19]` — the Winamp 2.x curves, in
    base-skin order: `Flat`, `Classical`, `Club`, `Dance`, `Full Bass`, `Full
    Bass & Treble`, `Full Treble`, `Headphones`, `Large Hall`, `Live`,
    `Party`, `Pop`, `Reggae`, `Rock`, `Ska`, `Soft`, `Soft Rock`, `Techno`,
    `Vocal`. Every `preamp` is `0.0`; the band arrays, in `BAND_FREQUENCIES`
    order, are: Flat `[0; 10]`; Classical `[0,0,0,0,0,0,-4.4,-4.4,-4.4,-5.6]`;
    Club `[0,0,8,5.6,5.6,5.6,3.1,0,0,0]`; Dance `[5.6,7.5,4.4,0,0,-5.6,-7.5,-7.5,0,0]`;
    Full Bass `[8.7,8.7,8.7,5.6,1.9,-4.4,-7.5,-8.1,-8.7,-8.7]`; Full Bass &
    Treble `[7.5,5.6,0,-7.5,-4.4,1.9,8.7,8.7,8.7,8.7]`; Full Treble
    `[-9.4,-9.4,-9.4,-4.4,1.9,8.7,9.4,9.4,9.4,10.0]`; Headphones
    `[4.4,7.5,5.6,0,-3.1,-1.9,1.9,5.6,8.1,8.7]`; Large Hall
    `[8.7,8.7,5.6,5.6,0,-5.6,-5.6,-5.6,0,0]`; Live
    `[-4.4,-1.9,0,3.1,5.6,5.6,5.6,3.1,1.9,1.9]`; Party
    `[7.5,7.5,0,0,0,0,0,0,7.5,7.5]`; Pop
    `[-1.9,1.9,4.4,5.6,5.6,0,-1.9,-1.9,-1.9,-1.9]`; Reggae
    `[0,0,0,-4.4,0,4.4,4.4,0,0,0]`; Rock
    `[8.1,4.4,-5.6,-7.5,-3.1,4.4,8.7,11.2,11.2,11.2]`; Ska
    `[-2.5,-4.4,-2.5,0,0,0,0,-2.5,-4.4,-5.6]`; Soft
    `[4.4,1.9,0,-1.9,-1.9,0,4.4,8.1,8.1,8.1]`; Soft Rock
    `[4.4,4.4,1.9,-1.9,-3.1,-1.9,1.9,5.6,8.1,8.1]`; Techno
    `[4.4,1.9,0,-1.9,-4.4,-1.9,4.4,8.1,8.1,8.1]`; Vocal
    `[-1.9,-3.1,-3.1,1.9,5.6,5.6,5.6,3.1,0,0]`.
  - Tests: `presets_have_unique_non_empty_names`;
    `preset_gains_stay_within_the_clamp_range` (every `preamp` and band equals
    `clamp_gain` of itself, so a preset can never store out of range);
    `flat_preset_is_all_zero`; `rock_preset_pins_the_classic_curve` (name and
    exact array).
- `src/state.rs`:
  - Add a private `eq_preset: Option<equalizer::Preset>` field to `AppState`,
    documented as the applied preset or `None` for a custom curve; initialize
    it `None` in the manual `Default` and extend the "Initial state" comment.
  - Add `pub fn apply_eq_preset(&mut self, preset: equalizer::Preset)` — store
    `preset.preamp` and `preset.bands` through the same clamps
    (`equalizer::clamp_gain` and `preset.bands.map(equalizer::clamp_gain)`)
    and set `eq_preset = Some(preset)`.
  - Make `set_eq_preamp` and `set_eq_band` clear `eq_preset` to `None` after
    storing (a hand-moved slider is a custom curve), and extend their doc
    comments.
  - Add `#[must_use] pub fn eq_preset(&self) -> Option<equalizer::Preset>`.
  - Tests: default `eq_preset()` is `None`; `apply_eq_preset` stores the whole
    curve and the selection and keeps track/volume (reuse
    `assert_keeps_track_and_volume`);
    `moving_a_slider_clears_the_preset_selection` (apply a preset, then call
    `set_eq_preamp` and `set_eq_band`, assert `None`).
- `src/ui/style.rs`:
  - Add `pick_list` and `overlay::menu` to the existing `iced::widget`
    import.
  - Add `pub fn chrome_pick_list_style(status: pick_list::Status) ->
    pick_list::Style` reusing `chrome_face` (opened/pressed sinks the face):
    `text_color: theme::TEXT`, `placeholder_color: theme::PANEL_EDGE_LIGHT`,
    `handle_color: theme::TEXT`, `background:
    Background::Color(chrome_face(false, opened))` (the closed control keeps
    the resting face; only the open menu sinks it), `border:
    square_border(theme::PANEL_EDGE_DARK)`.
  - Add `pub fn preset_menu_style() -> menu::Style` — `background:
    theme::BUTTON_FACE.into()`, `border:
    square_border(theme::PANEL_EDGE_DARK)`, `text_color: theme::TEXT`,
    `selected_text_color: theme::TEXT`, `selected_background:
    theme::TITLE_BLUE.into()`, `shadow: Shadow::default()`.
  - Tests: `chrome_pick_list_style_sinks_the_face_when_opened` (`Active` and
    `Hovered` use `BUTTON_FACE`, `Opened { is_hovered: false }` uses
    `BUTTON_FACE_PRESSED`, every status uses `TEXT` text);
    `preset_menu_style_uses_the_base_skin_face_and_selection_blue` (face,
    dark border, `TITLE_BLUE` selection).
- `src/ui/views.rs`:
  - Import `PickList` from `iced::widget` and `PRESETS`/`Preset` from
    `crate::equalizer`.
  - Change `view_equalizer` to take a fourth parameter `preset:
    Option<Preset>`; the panel previously pushed the EQ on/off button
    straight into its column, so wrap the button and the pick list in a
    header `Row` and push
    `PickList::new(PRESETS.as_slice(), preset, Message::EqPresetSelected)
    .placeholder("(none)").text_size(12).style(|_theme, status|
    style::chrome_pick_list_style(status)).menu_style(|_theme|
    style::preset_menu_style())`.
  - Update `equalizer_panel_constructs_for_both_states_and_gain_endpoints` to
    pass `None` and `Some(PRESETS[0])` across its loops.
- `src/ui/mod.rs`:
  - Add `EqPresetSelected(crate::equalizer::Preset)` to `Message`.
  - Add the `update` arm `Message::EqPresetSelected(preset) =>
    mutate_state(player, move |state| state.apply_eq_preset(preset))`.
  - In `view`, read `state.eq_preset()` into the existing state-lock tuple and
    pass it as `view_equalizer`'s fourth argument.
- `src/ui/tests.rs`: add `eq_preset_selected_applies_the_curve_and_selection` —
  drive `Message::EqPresetSelected(PRESETS[i])` through `update` and assert the
  shared `eq_preamp()`, `eq_bands()`, and `eq_preset()` match that preset.
- `README.md`: drop "preset curves" from the Screenshots "not built yet" note
  (leaving window-shade mode) and add the preset pick list to the Status
  sentence; leave the `tumwater:prompt` block untouched.

**Files touched.** `src/equalizer.rs`, `src/state.rs`, `src/ui/style.rs`,
`src/ui/views.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo doc` with rustdoc warnings denied, `cargo test`).
- The new `equalizer`, `state`, `style`, `views`, and `ui` tests listed above
  pass, including the `rock_preset_pins_the_classic_curve` exact-array pin and
  the `moving_a_slider_clears_the_preset_selection` behavior.
- No `dead_code`/unused warnings: every new constant, field, method, and style
  function is read by the UI or its tests.
- The existing `view_constructs_over_the_apps_full_input_space` still passes:
  it builds `view` for every browse/playback shape, so it exercises the pick
  list in the equalizer panel.
- `cargo run`: the equalizer header shows the EQ on/off button and a preset
  pick list reading "(none)"; selecting "Rock" moves the ten band sliders to
  the Rock curve and the pick list reads "Rock"; dragging any band slider
  afterwards makes the pick list read "(none)" again (manual check — build +
  tests are the primary gate).

### Add a Winamp custom title bar and drop the OS window frame (found 2026-10-07, done 2026-10-07)

Found by plan 2026-10-07, taking the "custom title bar" item the steward drift
note schedules and the base-skin Done entry lists among its later fidelity
plans. The bevel layer, buttons, sliders, and playlist chrome are done, but the
window still wears the OS title bar: `init_ui` in `src/ui/mod.rs` never sets
window settings, so the top of the app does not read as Winamp. `theme.rs`
already names `TITLE_BLUE` for the title bar and `bevel::raised_panel` already
supplies the raised bevel it needs.

**Goal.** Replace the OS window frame with a Winamp-style title bar: a
full-width raised `TITLE_BLUE` bar at the top of the window showing the app
name, draggable to move the window, with chrome minimize and close buttons on
the right. The window opens undecorated and fixed-size. No new theme colours
and no change to the existing view builders.

**Approach.**

- `src/ui/style.rs`: add `pub fn title_bar_style() -> container::Style`
  returning `container::Style { background: Some(Background::Color(
  theme::TITLE_BLUE)), text_color: Some(theme::TEXT),
  ..container::Style::default() }` — the flat base-skin title-bar fill and its
  light text, reusing the existing constants exactly as the other style
  functions do. Add a `#[cfg(test)]` test
  `title_bar_style_is_title_blue_with_light_text` pinning both fields.
- `src/ui/views.rs`: add module-level `const TITLE_BAR_TEXT: &str =
  "NOSTALGIA";` and `const TITLE_BAR_HEIGHT: f32 = 24.0;`, plus
  `pub fn view_title_bar() -> Element<'static, Message>`. It builds a `Row`
  of:
  - the drag region: `MouseArea::new(Container::new(Text::new(TITLE_BAR_TEXT)
    .size(14).color(theme::TEXT)).width(Length::Fill).height(Length::Fill)
    .align_y(iced::alignment::Vertical::Center).padding([0, 6]))
    .on_press(Message::WindowDragged)` — a `MouseArea` so a press anywhere on
    the bar (but not on the buttons) begins a window drag;
  - two `fixed_width_button`s ("–" → `Message::MinimizeWindow`, "✕" →
    `Message::CloseWindow`), each chrome-styled and width-pinned so the glyph
    cannot resize them.
  Wrap the `Row` in a `Container` with `.width(Length::Fill)` and
  `.height(Length::Fixed(TITLE_BAR_HEIGHT))` styled with
  `|_theme| style::title_bar_style()`, then in `bevel::raised_panel(..)` so the
  bar carries the same raised bevel as the other chrome. Add `MouseArea` to
  the `iced::widget::{..}` import.
- `src/ui/mod.rs`:
  - Add `window_id: Option<iced::window::Id>` to `WinampPlayer` and
    initialize it `None` in `WinampPlayer::new`.
  - Add `Message` variants `WindowIdResolved(Option<iced::window::Id>)`,
    `WindowDragged`, `MinimizeWindow`, and `CloseWindow`.
  - `boot`: batch the existing `Task::done(Message::LoadArtists)` with
    `iced::window::latest().map(Message::WindowIdResolved)`. iced runs the boot
    task only after the window opens (`iced_winit`'s `run` chains it onto
    `runtime::window::open`), so `latest()` resolves to the app window.
  - `update` arms: `WindowIdResolved(id)` stores `player.window_id = id` and
    returns `Task::none()`; the three window actions match on
    `player.window_id` and return `iced::window::drag(id)` /
    `iced::window::minimize(id, true)` / `iced::window::close(id)` when it is
    set, and `Task::none()` when it is not.
  - In `view`, push `views::view_title_bar()` as the first item of the
    `column`, above `view_now_playing`.
  - In `init_ui`, chain `.decorations(false)` and `.resizable(false)` onto the
    `iced::application(..)` builder so the OS frame is gone and the fixed size
    leaves no resize affordance the undecorated window cannot honour.
- `src/ui/tests.rs`: add `window_id_resolved_stores_the_window_id` (update with
  `Some(iced::window::Id::unique())`, assert `player.window_id` equals it, then
  with `None` and assert it clears) and
  `title_bar_window_actions_are_noops_without_a_window_id` (for each of the
  three action messages, assert `iced_runtime::task::into_stream(update(&mut
  player, message)).is_none()`), plus
  `title_bar_window_actions_schedule_work_with_a_window_id` (set the id, then
  assert each action's task stream is `Some`).
- `README.md`: add the custom title bar to the Status sentence; leave the
  `tumwater:prompt` block untouched.

**Files touched.** `src/ui/style.rs`, `src/ui/views.rs`, `src/ui/mod.rs`,
`src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo doc` with rustdoc warnings denied, `cargo test`).
- `title_bar_style_is_title_blue_with_light_text` passes, pinning the
  `TITLE_BLUE` background and `TEXT` text colour.
- `window_id_resolved_stores_the_window_id`,
  `title_bar_window_actions_are_noops_without_a_window_id`, and
  `title_bar_window_actions_schedule_work_with_a_window_id` pass.
- The existing `view_constructs_over_the_apps_full_input_space` still passes:
  it builds `view` for every browse/playback shape, so it exercises the new
  title bar in the column.
- `cargo run`: the window opens with no OS frame, a blue raised title bar at
  the top reading "NOSTALGIA" with minimize and close buttons, and dragging the
  bar moves the window while minimize and close work; manual check — the build
  and tests are the primary gate.

### Frame the browse list as a sunken Winamp playlist editor (found 2026-10-07, done 2026-10-07)

Found by plan 2026-10-07, taking the "playlist chrome" half of the widget-level
fidelity the steward drift note schedules after slider chrome. The bevel layer
(`src/ui/bevel.rs`) and the widget chrome styles (`src/ui/style.rs`) now dress
the panels, buttons, and sliders, but the browse
list — the app's playlist-editor stand-in — is still a plain `Scrollable` on the
window face: `scrollable_list` in `src/ui/views.rs` builds default-iced `Button`
rows (their hover/press is the theme's `TITLE_BLUE` primary) on no background,
and `Scrollable::new(column)` uses iced's default scrollbar rails. The base
skin's playlist editor is a near-black sunken panel with light rows and a
selection bar.

**Goal.** Give the artist/album/song browse list the playlist editor's chrome:
one pure `playlist_scrollable_style()` in `src/ui/style.rs` frames the list in a
sunken near-black well and restyles the scrollbar rails, and one pure
`playlist_row_style(status)` dresses the rows as flat playlist entries (light
text, chrome-face hover/press) while the currently playing row keeps its
selection highlight. No new theme colours and no signature changes.

**Approach.**

- `src/ui/style.rs`: add `scrollable` to the existing `iced::widget::{...}`
  import (the `container`, `Border`, `Shadow`, `Vector`, and `Background` types
  are already imported). Add two pure functions, both reusing the base-skin
  constants exactly as `chrome_button_style` and `chrome_slider_style` do:
  - `pub fn playlist_scrollable_style() -> scrollable::Style` — builds the whole
    struct (no `Default` impl exists for `scrollable::Style`, so all five fields
    are set):
    - `container`: `container::Style { text_color: Some(theme::TEXT),
      background: Some(theme::LCD_BACKGROUND.into()), border: Border { color:
      theme::PANEL_EDGE_DARK, width: 1.0, radius: 0.0.into() }, shadow: Shadow {
      color: theme::PANEL_EDGE_LIGHT, offset: Vector::new(1.0, 1.0), blur_radius:
      0.0 }, snap: false }` — the near-black recess with the same dark-border +
      light-offset-shadow sunken edge approximation `chrome_button_style` uses
      for its pressed state.
    - `vertical_rail` and `horizontal_rail`: `scrollable::Rail { background:
      Some(Background::Color(theme::WINDOW_BACKGROUND)), border: Border { color:
      theme::PANEL_EDGE_DARK, width: 1.0, radius: 0.0.into() }, scroller:
      scrollable::Scroller { background: Background::Color(theme::BUTTON_FACE),
      border: Border { color: theme::PANEL_EDGE_LIGHT, width: 1.0, radius:
      0.0.into() } } }` — a dark track with a raised chrome scroller, so the
      scrollbar reads on the well.
    - `gap: None`.
    - `auto_scroll`: `scrollable::AutoScroll { background:
      Background::Color(theme::LCD_BACKGROUND), border: Border { color:
      theme::PANEL_EDGE_LIGHT, width: 1.0, radius: 0.0.into() }, shadow: Shadow
      { color: theme::PANEL_EDGE_DARK, offset: Vector::new(1.0, 1.0),
      blur_radius: 0.0 }, icon: theme::TEXT }` (only drawn during touch
      auto-scroll, but the struct is total). Pure and status-independent:
      classic Winamp scrollbars give no hover/drag feedback, so the closure
      ignores the `scrollable::Status` iced passes.
  - `pub fn playlist_row_style(status: button::Status) -> button::Style` —
    `background: None` for `Active`/`Disabled`, `Some(Background::Color(
    theme::BUTTON_FACE))` for `Hovered`, `Some(Background::Color(
    theme::BUTTON_FACE_PRESSED))` for `Pressed`; `text_color: theme::TEXT`;
    everything else from `button::Style::default()` (no border or shadow —
    playlist rows are flat). The transparent Active face lets the well's
    near-black show through; hover/press lift and sink the row using the same
    faces `chrome_face` names.
  - `#[cfg(test)] mod tests` additions: a `playlist_row_style` ladder test
    (`Active` background `None`, `Hovered` `BUTTON_FACE`, `Pressed`
    `BUTTON_FACE_PRESSED`, every status `text_color == TEXT`), and a
    `playlist_scrollable_style` test pinning the container's `LCD_BACKGROUND`
    background, `PANEL_EDGE_DARK` border and `PANEL_EDGE_LIGHT` offset shadow,
    plus the rails' `BUTTON_FACE` scroller.
- `src/ui/views.rs`: in `scrollable_list`, replace the default-styled `Button`
  rows and the `iced::widget::button::background(theme, status)` current-row
  override with the new styles: every row gets `.style(|_theme, status|
  style::playlist_row_style(status))`, and the current row overrides only
  `background` to `Background::Color(super::theme::PLAYING_ROW_HIGHLIGHT)` on
  top of `playlist_row_style(status)` (keeping its light text and hover/press
  face, exactly as before). Add `.style(|_theme, _status|
  style::playlist_scrollable_style())` to the `Scrollable`, whose `width` /
  `height` stay `Fill` / `FillPortion(3)`. `scrollable_list` is the single
  builder behind `view_artists`, `view_albums`, and `view_songs`, so all three
  browse levels pick up the chrome with no signature change.
- `README.md`: extend the Status sentence to say the browse list is framed as a
  sunken Winamp playlist well with chrome rows; leave the `tumwater:prompt`
  block untouched.

**Files touched.** `src/ui/style.rs`, `src/ui/views.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo doc` with rustdoc warnings denied, `cargo test`).
- The new `playlist_row_style` and `playlist_scrollable_style` tests pass,
  pinning the row background ladder, the well's background/border/shadow
  colours, and the rails' scroller face.
- The existing browse tests still pass unchanged —
  `browse_views_construct_over_the_loaded_library`,
  `browse_views_construct_over_an_empty_list`,
  `empty_list_label_names_the_empty_browse_level`, and
  `browse_placeholder_distinguishes_loading_from_an_empty_list` build the same
  widget trees through the restyled `scrollable_list`.
- `cargo run`: the browse list sits in a near-black sunken well with a dark
  track scrollbar and a chrome scroller, its rows show light text that lifts on
  hover and sinks on press, and the playing row keeps its blue selection bar;
  manual check — the build and tests are the primary gate.

### Style the volume and equalizer sliders as sunken Winamp grooves with raised chrome thumbs (found 2026-10-07, done 2026-10-07)

Found by plan 2026-10-07, taking the slider half of the "transport button and
slider chrome" follow-up the bevel Done entry defers and the transport-button
Done entry leaves open ("Slider chrome is a separate, later plan"). The
bevel layer (`src/ui/bevel.rs`: `bevel_edges`, `lcd_well`, `raised_panel`)
and `style::chrome_button_style` now dress the panels and buttons, but every
`Slider` / `VerticalSlider` in `src/ui/views.rs` — the volume slider, the
preamp slider, and the ten EQ band sliders — still renders with iced's
default theme slider, a thin rail and a round handle, not the base skin's
dark sunken groove with a blocky raised chrome thumb.

**Goal.** Add one pure `chrome_slider_style(status)` to `src/ui/style.rs` that
maps iced's `slider::Status` to the base skin's groove and thumb, and route
every slider through it, so the volume, preamp, and ten band sliders read as
Winamp chrome. No new theme colours and no signature changes: the function
reuses the existing base-skin constants, exactly as `chrome_button_style`
does.

**Approach.**

- `src/ui/style.rs`: add `slider` to the existing `iced::widget::{...}`
  import (`Slider`/`VerticalSlider` share one `slider::Style`, `slider::Rail`,
  `slider::Handle`, and `slider::HandleShape` — `iced_widget`'s
  `vertical_slider` re-exports the slider types, so both widgets take the same
  style function). Add:
  - `pub fn chrome_slider_style(status: slider::Status) -> slider::Style` —
    pure, needs no theme, the testable heart (like `chrome_button_style`).
    Build:
    - `rail`: `slider::Rail { backgrounds: (Background::Color(theme::LCD_BACKGROUND),
      Background::Color(theme::LCD_BACKGROUND)), width: 4.0, border: Border
      { color: theme::PANEL_EDGE_DARK, width: 1.0, radius: 0.0.into() } }`.
      iced draws `backgrounds.0` as the segment left of / below the handle and
      `backgrounds.1` as the remainder (`iced_widget-0.14.2/src/slider.rs`);
      both are the near-black `LCD_BACKGROUND`, so the rail is one uniform
      sunken groove and the thumb — not a fill colour — is the value
      indicator, as in the base skin. `Rail` carries a single-colour `Border`
      and no shadow, so the sunken edge is approximated by the dark 1px rail
      outline.
    - `handle`: `slider::Handle { shape: slider::HandleShape::Rectangle {
      width: 8, border_radius: 0.0.into() }, background:
      Background::Color(face), border_width: 1.0, border_color:
      theme::PANEL_EDGE_LIGHT }`. The 8px rectangle gives a blocky thumb
      (full slider width, 8px thick on the long axis) and the light 1px
      border approximates the raised chrome edge — the same light-border
      trick `chrome_button_style` uses within iced's single-colour `Border`.
      `face` matches `status`, reusing the button face shades: `Active` →
      `BUTTON_FACE`, `Hovered` → `BUTTON_FACE_HOVERED`, `Dragged` →
      `BUTTON_FACE_PRESSED` (the match is exhaustive — `slider::Status` has
      no `Disabled`).
  - `#[cfg(test)] mod tests` additions: `chrome_slider_style(Active)` has
    `rail.backgrounds == (Background::Color(LCD_BACKGROUND),
    Background::Color(LCD_BACKGROUND))`, `rail.width == 4.0`,
    `rail.border.color == PANEL_EDGE_DARK`, `rail.border.width == 1.0`,
    `handle.shape == HandleShape::Rectangle { width: 8, border_radius:
    0.0.into() }`, `handle.background == Background::Color(BUTTON_FACE)`,
    `handle.border_width == 1.0`, `handle.border_color == PANEL_EDGE_LIGHT`;
    `Hovered` and `Dragged` keep the rail unchanged and swap only the handle
    face to `BUTTON_FACE_HOVERED` / `BUTTON_FACE_PRESSED`.
- `src/ui/views.rs`: chain `.style(|_theme, status|
  style::chrome_slider_style(status))` onto the volume `Slider` in
  `view_transport_controls`, the preamp `Slider`, and the `VerticalSlider` in
  `view_equalizer`'s band loop. These are the only `Slider` /
  `VerticalSlider` call sites in the crate, so all three slider kinds pick up
  the chrome with no builder or signature change; the browse row buttons in
  `scrollable_list` keep their own highlight style and are untouched.
- `README.md`: in the Status block, say the volume slider and the equalizer's
  preamp and ten band sliders are drawn as sunken grooves with raised chrome
  thumbs; leave the `tumwater:prompt` block untouched.

**Files touched.** `src/ui/style.rs`, `src/ui/views.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo doc` with rustdoc warnings denied, `cargo test`).
- The new `chrome_slider_style` tests pass, pinning the uniform dark rail, its
  4px width and dark 1px border, the 8px rectangular handle with its light
  border, and the three status-driven handle faces.
- The existing `views` tests still pass unchanged —
  `transport_controls_construct_for_both_play_states_volume_endpoints_and_repeat_states`
  and `equalizer_panel_constructs_for_both_states_and_gain_endpoints` build
  the same widget trees through the restyled sliders.
- `cargo run`: the volume, preamp, and EQ band sliders render as dark sunken
  grooves with a blocky grey thumb that lightens on hover and darkens while
  dragging; manual check — the build and tests are the primary gate.

### Style the transport buttons as raised Winamp chrome (found 2026-10-07, done 2026-10-07)

Found by plan 2026-10-07, taking the "transport button and slider chrome"
follow-up the bevel Done entry defers. The bevel layer (`src/ui/bevel.rs`:
`bevel_edges`, `lcd_well`, `raised_panel`) frames the panels, but
`labeled_button` in `src/ui/views.rs` still builds a default-styled `Button`,
so the Play/Pause, Stop, Previous, Next, and Repeat controls and the browse
Back button render with iced's theme button (the palette's `TITLE_BLUE`
primary) instead of the base skin's raised chrome. iced 0.14's `button::Style`
carries a single-colour `Border` and a `Shadow` (`iced_core/src/border.rs`:
`color`, `width`, `radius`; `iced_core/src/shadow.rs`: `color`, `offset`,
`blur_radius`), so — as with the panel bevel — the two-tone edge is composed
from a light border plus a dark offset shadow.

**Goal.** Add one pure `chrome_button_style(status)` to `src/ui/style.rs` that
maps iced's `button::Status` to the base skin's raised (Active/Hovered) and
sunken (Pressed) chrome, and route every `labeled_button` through it, so the
five transport buttons and the Back button read as Winamp chrome. Slider
chrome is a separate, later plan (as the bevel entry notes).

**Approach.**

- `src/ui/theme.rs`: add three public `Color` constants beside the existing
  base-skin colours, each `Color::from_rgb`, in the module's existing
  doc-comment style:
  - `BUTTON_FACE` — the raised chrome button face,
    `Color::from_rgb(0.30, 0.30, 0.30)` (lighter than `WINDOW_BACKGROUND` so
    the button reads as raised).
  - `BUTTON_FACE_HOVERED` — the hover face,
    `Color::from_rgb(0.38, 0.38, 0.38)`.
  - `BUTTON_FACE_PRESSED` — the pressed (sunken) face,
    `Color::from_rgb(0.22, 0.22, 0.22)`.
- `src/ui/style.rs`: import `button`, `Background`, `Border`, `Shadow`, and
  `Vector` from `iced` (all re-exported at the crate root). Add:
  - `pub fn chrome_button_style(status: button::Status) -> button::Style` —
    pure, needs no theme, the testable heart (like `bevel_edges`). Match
    `status`:
    - `Active` → edges `(PANEL_EDGE_LIGHT, PANEL_EDGE_DARK)`, face
      `BUTTON_FACE`.
    - `Hovered` → the same edges, face `BUTTON_FACE_HOVERED`.
    - `Pressed` → reversed edges `(PANEL_EDGE_DARK, PANEL_EDGE_LIGHT)`, face
      `BUTTON_FACE_PRESSED`.
    - `Disabled` → the `Active` chrome with `theme::TEXT.scale_alpha(0.5)` as
      `text_color` (no button in this app is disabled, but the match must be
      exhaustive).

    Build
    `button::Style { background: Some(Background::Color(face)), text_color:
    theme::TEXT, border: Border { color: top_left, width: 1.0, radius:
    0.0.into() }, shadow: Shadow { color: bottom_right, offset:
    Vector::new(1.0, 1.0), blur_radius: 0.0 }, ..button::Style::default() }`.
    The 1px light border shows on the top/left and the dark (or, when pressed,
    light) no-blur shadow offset down-right shows on the bottom/right — the
    same two-tone edge the panel bevel draws, approximated within `Border`'s
    single colour.
  - `#[cfg(test)] mod tests` additions: `chrome_button_style(Active)` has
    `background == Some(Background::Color(BUTTON_FACE))`, `text_color ==
    TEXT`, `border.color == PANEL_EDGE_LIGHT`, `border.width == 1.0`,
    `shadow.color == PANEL_EDGE_DARK`, `shadow.offset == Vector::new(1.0,
    1.0)`, `shadow.blur_radius == 0.0`; `Hovered` differs from `Active` in
    `background == BUTTON_FACE_HOVERED`; `Pressed` reverses to `border.color ==
    PANEL_EDGE_DARK` / `shadow.color == PANEL_EDGE_LIGHT` with `background ==
    BUTTON_FACE_PRESSED`; `Disabled` keeps the Active edges and sets
    `text_color.a < TEXT.a`.
- `src/ui/views.rs`: in `labeled_button`, chain `.style(|_theme, status|
  style::chrome_button_style(status))` onto the button. `labeled_button` is the
  single builder behind all five transport buttons (`view_transport_controls`)
  and the Back button (`view_back_button`), so both pick up the chrome with no
  signature change; the browse row buttons in `scrollable_list` keep their own
  highlight style and are untouched.
- `README.md`: refresh the Status sentence to say the transport controls and
  Back button are drawn as raised Winamp chrome buttons; leave the
  `tumwater:prompt` block untouched.

**Files touched.** `src/ui/style.rs`, `src/ui/theme.rs`, `src/ui/views.rs`,
`README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo doc` with rustdoc warnings denied, `cargo test`).
- The new `chrome_button_style` tests pass, pinning the raised/sunken edge
  colours, the three face shades, and the disabled text dimming.
- The existing `views` tests still pass unchanged —
  `transport_controls_construct_for_both_play_states_volume_endpoints_and_repeat_states`
  and `now_playing_bar_and_back_button_construct` build the same widget trees
  through the restyled `labeled_button`.
- `cargo run`: the transport buttons and Back button render as raised gray
  chrome with a light top/left and dark bottom/right edge, darkening on hover
  and reading sunken while pressed; manual check — the build and tests are the
  primary gate.

### Add a Winamp two-tone bevel layer and frame the Now Playing and equalizer panels (found 2026-10-06, done 2026-10-06)

**Update (2026-10-08, organize).** The bevel composition primitives this entry
created (`bevel_edges`, `styled_edge`, `bevel_overlay`, `beveled`, `lcd_well`,
`raised_panel`) and their tests moved to a new `src/ui/bevel.rs`;
`src/ui/style.rs` now holds only the widget chrome styles that consume them
(the title bar, buttons, pick list/menu, sliders, and playlist rows/well). The
split gives the reusable bevel layer its own module and keeps `style.rs` to
per-widget `Status` → `Style` builders.

Found by plan 2026-10-06, following the steward drift note's call for custom
widget styling. The palette landed (previous Done entry), but every panel is
still flat: iced 0.14's `Border` is a single colour of uniform width
(`iced_core/src/border.rs`: `color`, `width`, `radius` only), so no view draws
the base skin's defining 3D edge — light top/left, dark bottom/right on raised
chrome, reversed in a sunken LCD well.

**Goal.** Add a small `src/ui/style.rs` that composes the bevel from 1px `Rule`
edges and wrap the two panels with intrinsic height: the Now Playing bar as a
sunken dark LCD well and the equalizer panel as a raised chrome panel. This is
the reusable styling layer the drift note asks for; transport button and slider
chrome is a separate, later plan that styles against these same edge colours.

**Approach.**

- `src/ui/theme.rs`: add three public `Color` constants beside the existing
  base-skin colours, each `Color::from_rgb`, in the module's existing
  doc-comment style:
  - `PANEL_EDGE_LIGHT` — the light top/left bevel edge,
    `Color::from_rgb(0.55, 0.55, 0.55)`.
  - `PANEL_EDGE_DARK` — the dark bottom/right bevel edge,
    `Color::from_rgb(0.05, 0.05, 0.05)`.
  - `LCD_BACKGROUND` — the near-black LCD recess behind the green title,
    `Color::from_rgb(0.05, 0.05, 0.05)`.
- `src/ui/style.rs` (new file, declared `mod style;` beside `mod theme;` in
  `src/ui/mod.rs`), with `//!` module docs and a doc comment on every public
  item (the `docs` stage denies rustdoc warnings):
  - `pub fn bevel_edges(raised: bool) -> (Color, Color)` — pure, returns the
    `(top_left, bottom_right)` edge colours: `raised` → `(PANEL_EDGE_LIGHT,
    PANEL_EDGE_DARK)`, sunken → `(PANEL_EDGE_DARK, PANEL_EDGE_LIGHT)`. This is
    the testable heart of the bevel; the widget composition only consumes it.
  - `fn horizontal_edge<'a, R>(color: Color) -> Element<'a, Message, Theme, R>`
    and `fn vertical_edge<'a, R>(color: Color) -> Element<'a, Message, Theme,
    R>` (`R: iced::advanced::Renderer`) — a 1px `Rule` (`Rule::horizontal(1.0)`
    / `Rule::vertical(1.0)`) styled with `rule::Style { color, radius:
    0.0.into(), fill_mode: rule::FillMode::Full, snap: true }`. A `Rule` fills
    whatever length it is laid out against; the [`Stack`] composition in
    `beveled` is what constrains that to the panel's resolved size. The
    renderer is generic so the layout test below can drive the composition
    with iced's null `()` renderer.
  - `fn bevel_overlay<'a, R>(top_left: Color, bottom_right: Color) ->
    Element<'a, Message, Theme, R>` — a `Length::Fill` `Column` of top edge, a
    `Length::Fill` `Row` of left edge + `Space::new().width(Length::Fill)` +
    right edge, and bottom edge, coloured from `bevel_edges`; the spacer puts
    the right edge on the panel's outer edge.
  - `fn beveled<'a, R>(content: Element<'a, Message, Theme, R>, raised: bool)
    -> Element<'a, Message, Theme, R>` — a `Stack` whose base layer is
    `content` and whose overlay is `bevel_overlay`; the stack is
    `width(Length::Fill)` and `height(Length::Shrink)`, so the panel fills the
    available width but takes the content's intrinsic height. The `Stack`
    sizes itself to its base layer and lays the overlay out against that
    resolved size, so the bevel's vertical edges span the panel exactly
    instead of the window's leftover height.
  - `pub fn lcd_well<'a>(content: impl Into<Element<'a, Message>>) ->
    Element<'a, Message>` — wraps `content` in a full-width `Container` with
    `background: LCD_BACKGROUND` and a few px of padding, then
    `beveled(.., false)`. No `text_color` is set, so the `"Now Playing: "`
    caption keeps the theme text colour and only the title's explicit
    `LCD_GREEN` stays green.
  - `pub fn raised_panel<'a>(content: impl Into<Element<'a, Message>>) ->
    Element<'a, Message>` — `beveled(.., true)` around content on the window
    face (no extra background, so it composes with the existing theme).
  - `#[cfg(test)] mod tests`: `bevel_edges(true)` equals
    `(theme::PANEL_EDGE_LIGHT, theme::PANEL_EDGE_DARK)` and `bevel_edges(false)`
    equals the reverse; `lcd_well(Text::new("x"))` and
    `raised_panel(Text::new("x"))` build and request a `Length::Fill` width
    with a `Length::Shrink` (content-driven) height; and
    `beveled_tracks_content_height_and_fills_the_available_width` lays the
    composition out with iced's null `()` renderer under a 500×400 limit and
    asserts the resolved node is 500×7 for a 20×7 content — the height
    regression guard the builder-only tests cannot be.
- `src/ui/views.rs`:
  - In `view_now_playing`, wrap the existing `Row` in
    `super::bevel::lcd_well(..)`.
  - In `view_equalizer`, wrap the existing `Column` in
    `super::bevel::raised_panel(..)`.
  - Add `use super::style;`; no signature changes, so the existing view
    construction tests compile and pass unchanged.
- `README.md`: refresh the Status sentence to say the Now Playing bar and
  equalizer are framed with Winamp's raised/sunken bevels; leave the
  `tumwater:prompt` block untouched.

**Files touched.** `src/ui/style.rs` (new), `src/ui/theme.rs`, `src/ui/mod.rs`,
`src/ui/views.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo doc` with rustdoc
  warnings denied, `cargo test`).
- The new `style` unit tests pass; the existing `views` construction tests
  (`now_playing_bar_and_back_button_construct`,
  `equalizer_panel_constructs_for_both_states_and_gain_endpoints`) still pass
  with the wrapped panels.
- `cargo run`: the Now Playing bar reads as a dark sunken LCD well (light
  top/left, dark bottom/right edges) and the equalizer as a raised panel (the
  reverse); manual check — the build and tests are the primary gate.

### Add a Winamp 2.x base-skin palette and apply it as the app theme (done 2026-10-06)

Found by plan 2026-10-06, following the steward drift note above.

**Goal.** Every widget still renders in iced 0.14's default theme: no custom
`Theme`/`Palette` is built anywhere (`grep -rn 'Theme\|palette' src` matches
only the one highlight-colour button style in `views.rs`), so the window does
not yet look like Winamp. Land the fidelity foundation the steward note asks
for: a `src/ui/theme.rs` module naming the Winamp 2.x base-skin colours and a
custom `iced::Theme` built from them, applied app-wide in `init_ui`, plus the
Now Playing title in Winamp's LCD green. Later fidelity plans (title bar,
panel bevels, playlist chrome) style against these names; this plan only
introduces the palette and the app-wide application, so it stays one run and
leaves no widget unstyled by accident.

**Approach.**

- New file `src/ui/theme.rs` (a sibling of `views.rs`/`transport.rs`, declared
  `mod theme;` in `src/ui/mod.rs`), with module `//!` docs and a doc comment on
  every public item (the `docs` stage denies rustdoc warnings):
  - Public `Color` constants for the Winamp 2.x base skin, each a `const` via
    `Color::from_rgb`, exactly as the existing `views::PLAYING_ROW_HIGHLIGHT`
    is built:
    - `WINDOW_BACKGROUND` — the dark gray window face,
      `Color::from_rgb(0.18, 0.18, 0.18)`.
    - `TEXT` — light chrome text, `Color::from_rgb(0.87, 0.87, 0.87)`.
    - `TITLE_BLUE` — Winamp's title-bar/selection blue,
      `Color::from_rgb(0.0, 0.0, 0.55)`.
    - `LCD_GREEN` — the playlist/LCD green, `Color::from_rgb(0.0, 1.0, 0.0)`.
    - `PLAYING_ROW_HIGHLIGHT` — moved here from `views.rs` with the same value
      `Color::from_rgb(0.25, 0.5, 1.0)`, so all UI colours live in one module.
  - `pub fn palette() -> iced::theme::Palette` filling all six fields:
    `background: WINDOW_BACKGROUND`, `text: TEXT`, `primary: TITLE_BLUE`,
    `success: LCD_GREEN`, and `warning`/`danger` set to
    `Color::from_rgb(1.0, 0.65, 0.0)` / `Color::from_rgb(1.0, 0.2, 0.2)`.
  - `pub fn winamp_theme() -> iced::Theme` returning
    `iced::Theme::custom("Winamp", palette())`.
  - `#[cfg(test)] mod tests`: `winamp_theme().palette().background` and
    `.text` equal the named constants; `winamp_theme().extended_palette().is_dark`
    is true; `winamp_theme().to_string() == "Winamp"`.
- `src/ui/mod.rs`:
  - Add `mod theme;` beside `mod transport;` / `mod views;`.
  - Chain `.theme(|_: &WinampPlayer| theme::winamp_theme())` onto the
    `iced::application(..)` builder in `init_ui`, before `.run()`.
- `src/ui/views.rs`:
  - Delete the local `PLAYING_ROW_HIGHLIGHT` const and use
    `super::theme::PLAYING_ROW_HIGHLIGHT` in `scrollable_list` instead; drop
    `Color` from the `iced` import list (it is no longer used once the const
    moves).
  - In `view_now_playing`, colour the current-track `Text` with
    `super::theme::LCD_GREEN` via `Text::color` so the Now Playing bar reads as
    Winamp's green LCD; the `"Now Playing: "` caption keeps the theme text
    colour.
- `README.md`: refresh the Status sentence to say the window uses a Winamp
  base-skin colour theme; leave the `tumwater:prompt` block untouched.

**Files touched.** `src/ui/theme.rs` (new), `src/ui/mod.rs`,
`src/ui/views.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo doc` with rustdoc
  warnings denied, `cargo test`).
- The new `theme` unit tests pass; the existing `views` construction tests
  (`now_playing_bar_and_back_button_construct`,
  `browse_views_construct_over_the_loaded_library`) still pass with the moved
  constant, and no unused-import or `dead_code` warning remains.
- `cargo run`: the window renders on the dark Winamp face with light text and
  a green Now Playing title, while the Songs view's playing-row highlight is
  unchanged (manual check — build + tests are the primary gate).

### Add the Winamp equalizer panel: on/off, preamp, and ten band sliders (done 2026-10-06)

Found by plan 2026-10-06.

**Goal.** The equalizer is the largest classic Winamp UI element the app still
lacks — the README's own mockup (`docs/screenshots/equalizer.png`) shows it
docked under the main window, but no equalizer code exists (`grep -ri equaliz
src` finds nothing). Land the equalizer's core controls as a panel in the
existing single window: an EQ on/off toggle, a preamp slider, and ten vertical
band sliders (60 Hz–16 kHz, ±12 dB) whose values live in the shared `AppState`,
mirroring how `volume` and `repeat` already work. Out of scope (later plans):
preset curves, a separate/docked OS window, window-shade mode, and any real
audio processing (the Apple Music stub has no audio pipeline yet).

**Approach.**

- New file `src/equalizer.rs`, the equalizer data model — a sibling of
  `library.rs` and `state.rs` so both the shared state and the UI can depend on
  it without a `state` ← `ui` cycle:
  - `pub const BAND_FREQUENCIES: [&str; 10] = ["60", "170", "310", "600", "1K", "3K", "6K", "12K", "14K", "16K"];`
  - `pub const BAND_COUNT: usize = BAND_FREQUENCIES.len();`
  - `pub const GAIN_MIN_DB: f32 = -12.0;` and `pub const GAIN_MAX_DB: f32 = 12.0;`
  - `#[must_use] pub fn clamp_gain(gain: f32) -> f32` — `clamp` to the range,
    mapping NaN to `0.0`, exactly as `state::clamp_volume` does for volume.
  - A `#[cfg(test)] mod tests` pinning `clamp_gain` at both bounds, in range,
    and NaN, plus `BAND_FREQUENCIES.len() == BAND_COUNT`.
- `src/main.rs`: add `mod equalizer;` beside the other `mod` declarations.
- `src/state.rs`:
  - Add to `AppState`: `pub eq_enabled: bool`, `pub eq_preamp: f32`,
    `pub eq_bands: [f32; equalizer::BAND_COUNT]`.
  - Extend the manual `Default` impl: `eq_enabled: false`, `eq_preamp: 0.0`,
    `eq_bands: [0.0; equalizer::BAND_COUNT]` (flat), and update the
    "Initial state" doc comment.
  - Add `toggle_equalizer(&mut self)` (flips `eq_enabled`, like
    `toggle_repeat`), `set_eq_preamp(&mut self, gain: f32)` (stores
    `equalizer::clamp_gain(gain)`), and `set_eq_band(&mut self, band: usize,
    gain: f32)` (clamps and stores into `eq_bands[band]`, ignoring an
    out-of-range `band` via `get_mut`).
  - Tests: default eq is off/flat; `toggle_equalizer` flips only `eq_enabled`;
    both setters clamp an out-of-range value and `set_eq_band` ignores an
    out-of-range band index; none of the eq mutators change
    `current_track`/`volume` (reuse the existing
    `assert_keeps_track_and_volume` helper).
- `src/ui/views.rs`:
  - Import `VerticalSlider` from `iced::widget` and `BAND_COUNT` /
    `BAND_FREQUENCIES` / `GAIN_MIN_DB` / `GAIN_MAX_DB` from `crate::equalizer`.
  - `fn eq_enabled_label(enabled: bool) -> &'static str` returning `"EQ: On"`
    / `"EQ: Off"`, mirroring `repeat_label`.
  - `pub fn view_equalizer(enabled: bool, preamp: f32, bands: &[f32; BAND_COUNT]) -> Element<'static, Message>`
    — a `Column` of:
    - a header `Row` with `labeled_button(eq_enabled_label(enabled), Message::ToggleEqualizer)`;
    - a preamp `Row` of `Text::new("Preamp")` and
      `Slider::new(GAIN_MIN_DB..=GAIN_MAX_DB, preamp, Message::EqPreampChange).step(1.0).width(Length::Fixed(150.0))`;
    - a bands `Row` of `BAND_COUNT` `Column`s, each a
      `VerticalSlider::new(GAIN_MIN_DB..=GAIN_MAX_DB, bands[i], move |gain| Message::EqBandChange(i, gain)).step(1.0).height(Length::Fixed(100.0))`
      above `Text::new(BAND_FREQUENCIES[i]).size(12)`.
  - Tests: `eq_enabled_label` mirrors the flag; `view_equalizer` constructs for
    enabled/disabled and both gain endpoints (mirroring the existing
    transport-controls construction test).
- `src/ui/mod.rs`:
  - Add `Message::{ToggleEqualizer, EqPreampChange(f32), EqBandChange(usize, f32)}`.
  - `update` arms: `ToggleEqualizer => mutate_state(player, AppState::toggle_equalizer)`;
    `EqPreampChange(gain) => mutate_state(player, |s| s.set_eq_preamp(gain))`;
    `EqBandChange(band, gain) => mutate_state(player, |s| s.set_eq_band(band, gain))`.
  - In `view`, read `eq_enabled`/`eq_preamp`/`eq_bands` in the existing
    state-lock block and push
    `views::view_equalizer(eq_enabled, eq_preamp, &eq_bands)` after
    `views::view_transport_controls(..)`.
- `src/ui/tests.rs`: add update tests — `Message::ToggleEqualizer` flips the
  shared flag; `EqPreampChange`/`EqBandChange` store clamped values (e.g.
  `EqBandChange(0, 99.0)` stores `12.0`).
- `README.md`: refresh the Status and Usage lines to describe the equalizer
  panel; leave the `tumwater:prompt` block untouched.

**Files touched.** `src/equalizer.rs` (new), `src/main.rs`, `src/state.rs`,
`src/ui/views.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo doc` with rustdoc
  warnings denied, `cargo test`).
- The new `equalizer`, `state`, and `views` unit tests and the `ui` update
  tests listed above pass.
- No `dead_code`/unused warnings: every new constant, field, and method is read
  by the UI or its tests.
- `cargo run`: below the transport row the window shows an "EQ: Off" button, a
  Preamp slider, and ten band sliders labelled `60`…`16K`; pressing the button
  changes its label to "EQ: On", and dragging any slider moves it (manual check
  — build + tests are the primary gate).


### Add a Repeat toggle to the transport controls (done 2026-10-06)

Found by plan 2026-10-06.

**Goal.** Match Winamp's Repeat control: a Repeat button in the transport row that switches Previous/Next through the current album's songs between wrap-around (Repeat on) and stop-at-the-edge (Repeat off) stepping. Today `transport::next_track_id` and `transport::previous_track_id` always wrap at the album's ends, so the current default is indistinguishable from Repeat-on; this feature makes wrapping explicitly opt-in and starts Repeat off, as Winamp does.

**Approach.**
- `src/state.rs`: add `pub repeat: bool` to `AppState` (initialised `false` in `impl Default`), plus a `pub fn toggle_repeat(&mut self)` mirroring `toggle_playing`. Update `default_state_is_stopped_at_half_volume` to assert `!state.repeat`; add `toggle_repeat_flips_only_the_repeat_flag` (flips both ways, leaves `current_track`, `is_playing`, `volume` untouched). The `stop_clears_playing_flag_and_keeps_current_track` test builds `AppState` with `..Default::default()`, so it needs no change.
- `src/ui/transport.rs`: add a `repeat: bool` parameter to `stepped_track_id`, `next_track_id`, and `previous_track_id`. With `repeat == false`, a step that would cross the boundary stays on the edge instead of wrapping — Next from the last song returns the last song's id, Previous from the first returns the first's — so the button re-lands on the current track rather than moving or doing nothing. With `repeat == true`, and in every no-current / empty-list / single-song case, behavior is unchanged: the no-current branch still lands on the forward/backward edge. Update the two wrap tests (`next_wraps_from_last_to_first`, `previous_wraps_from_first_to_last`) to pass `true`; give the other existing transport tests the new argument (any value where the expectation does not depend on repeat); add `next_stays_on_last_without_repeat` and `previous_stays_on_first_without_repeat`.
- `src/ui/views.rs`: `view_transport_controls` gains a `repeat: bool` parameter and pushes a Repeat button (via the existing `labeled_button`) after the Next button; add `fn repeat_label(repeat: bool) -> &'static str` beside `play_pause_label`, returning "Repeat: Off"/"Repeat: On", and pin it in `repeat_label_mirrors_repeat_state`. Extend `transport_controls_construct_for_both_play_states_and_volume_endpoints` to also loop over `[false, true]` for repeat.
- `src/ui/mod.rs`: add `Message::ToggleRepeat`; its update arm routes to `mutate_state(player, AppState::toggle_repeat)`. `step_track`'s `step` parameter becomes `fn(&[Song], Option<&str>, bool) -> Option<String>` and the Next/Previous arms pass `state.repeat` (read under the existing lock), so the arm wiring keeps the shared flag in step. `view` reads `repeat` in its lock block and passes it to `views::view_transport_controls`. Add wiring tests: `toggle_repeat_flips_shared_state` (drive `Message::ToggleRepeat` through `update`, like `play_pause_toggles_is_playing`) and a stepping-wiring test driving `Message::NextTrack` from the last song — wraps when `state.repeat` is true, stays on the last song when false.

**Files touched.** `src/state.rs`, `src/ui/transport.rs`, `src/ui/views.rs`, `src/ui/mod.rs` (each including its `#[cfg(test)]` module).

**Acceptance criteria.**
- `AppState::default().repeat` is `false`; `toggle_repeat` flips it and nothing else.
- `next_track_id`/`previous_track_id` with `repeat = false` stop at the album's edge (return the current edge song's id); with `repeat = true`, and in every no-current/empty/single-song case, they behave exactly as before the change.
- The transport row renders a Repeat button whose label tracks the shared `repeat` flag, and pressing it flips that flag in shared state.
- `make check` is green: `cargo fmt --check`, `cargo clippy --all-targets`, and the full `cargo test` suite pass.


### Highlight the currently playing song in the Songs browse view (done 2026-10-06)

Found by plan 2026-10-06.

**Goal.** Winamp's playlist editor draws a highlighted bar on the track that is
currently playing, so you can see where in the list the music is and watch it
move as you step. This player has no such cue: the Songs view renders every row
identically, so after clicking a song (or pressing Next/Previous) there is no
way to tell which song in the album is current. Mark the row whose id equals the
shared `current_track` — a `▶` prefix on its title and a highlighted button
background — so the song list reads as a playlist. The marker is pure data
derived state (current id vs. the displayed songs), so it stays in the view
layer like the rest of `views.rs` and never touches the service seam.

**Approach.** All changes are in the view layer; `AppleMusicService`, `state`,
`library`, and the transport stepping arithmetic are untouched.

- `src/ui/views.rs`:
  - Extend the browse-row tuple from `(String, &'static str, Message)` to
    `(String, &'static str, Message, bool)` — the new bool is whether this row
    is the currently playing track. `scrollable_list` reads it: when true, it
    prefixes the title with `"▶ "` and gives the button a highlighted
    background (a `Button::style` closure returning a
    `iced::widget::button::Style` with a `Background::Color`); when false, the
    plain button as today. `artist_row` and `album_row` return `false` — the
    marker only ever applies inside an album's song list.
  - `song_row(song: &Song, current_track: Option<&str>)` computes the flag as
    `Some(&song.id) == current_track`.
  - `view_songs(songs: &[Song], current_track: Option<&str>)` takes the current
    track id and passes it to `song_row`. `view_artists`/`view_albums`
    signatures are unchanged.
  - Tests: update `song_row_uses_title_and_selects_the_song` (and the
    `artist_row`/`album_row` tests) for the 4-tuple, and add a
    `song_row_marks_the_current_track` test pinning the flag for `Some(id)`
    matching the song, a different `Some`, and `None`. Update the two
    `browse_views_construct_*` construction tests for the new `view_songs`
    signature and add a construction case passing a current track, following
    the existing pattern (iced `Element`s are not introspectable, so the
    row-tuple flag is the testable contract and construction tests cover
    rendering without panicking).
- `src/ui/mod.rs` `view()`: the current-track id must reach `view_songs`. The
    Songs arm acquires the shared-state lock briefly and passes
    `state.current_track.as_deref()` — a second short `blocking_lock` rather
    than a per-frame `Option<String>` clone, consistent with the earlier
    commit that removed the per-frame `current_track` clone (4d7a585). The
    label tuple block above stays as is.

**Files touched.** `src/ui/views.rs`, `src/ui/mod.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `song_row_marks_the_current_track`
  row-flag test and the updated construction tests.
- `cargo fmt --check` and `cargo clippy --all-targets` are clean.
- `cargo run`: in a Songs view, clicking a song marks it (`▶` + highlighted
  background); Next/Previous move the marker through the list; browsing to
  another album shows no marker (its songs do not contain the current track);
  stepping works with the marker following along. Build + tests are the
  primary gate; the visual check confirms the marker renders.

Note: the acceptance criteria require `cargo fmt --check` clean, and the only
line failing it was a pre-existing over-long `assert_ids` call in
`src/apple_music.rs` (added by c9cb1cb) — this change reformats that one call
so the gate passes; no other line in the integration seam is touched.

- Add a Stop button to the transport controls (done 2026-10-06; commit 8e91e1e)
- Show the song's title in the Now Playing bar, not its raw id (done 2026-10-05; commit 7d29183)
- Make Previous/Next step through the songs of the current album (done 2026-10-05; commit 4c8e223)
- Add a working volume slider to the transport controls (done 2026-10-05; commit 7029804)
- Make the app build on iced 0.14 and render a browsable sample library (done 2026-10-04; commit 5af3f08)
