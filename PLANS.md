# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

## Planned

_None yet._

## Done

### Show the song's title in the Now Playing bar, not its raw id (done 2026-10-05)

Found by plan 2026-10-05.

**Goal.** `AppState.current_track` stores a track *id*, and `WinampPlayer::view`
renders it verbatim — after playing "Opening" the bar reads
"Now Playing: song-1", not the title a Winamp-style player should show.
Resolve the id against the songs currently loaded in `WinampPlayer.songs`
(the same list the songs view renders and the Previous/Next plan sequences)
and display the matching `Song.title`, falling back to the id when the track
isn't in the loaded songs and to "Nothing" when nothing is current. Keep the
resolution pure so it is testable without the UI.

**Approach.**

- `src/ui/mod.rs`: add a private
  `fn now_playing_label(songs: &[Song], current_track: Option<&str>) -> String`
  beside `WinampPlayer::view` (the formatting is a view-time concern, so the
  helper stays with the player UI rather than in `views.rs`, whose stated
  contract is data → `Element`). Semantics: `current_track` is `None` →
  `"Nothing"`; `current_track` is `Some(id)` whose id matches a `Song` in
  `songs` → that song's `title` (cloned); `Some(id)` with no match → the raw
  `id` itself (a track is current, so "Nothing" would lie). Add a
  `#[cfg(test)] mod tests` covering: no current track → "Nothing"; current id
  found in `songs` → its title; current id not in `songs` → the id; empty
  `songs` with a current id → the id. The function is pure data → `String`, so
  it is testable without the UI.
- In `WinampPlayer::view`, replace the existing
  ```rust
  let now_playing = state
      .current_track
      .clone()
      .unwrap_or_else(|| "Nothing".to_string());
  ```
  with a call that reads the id while the `blocking_lock` guard is held and
  delegates the formatting to the helper:
  ```rust
  let now_playing = now_playing_label(&player.songs, state.current_track.as_deref());
  ```
  No `Message` change and no `AppState` change: `current_track` stays the
  id (the Previous/Next plan reads it as an id via
  `current_track.as_deref()`), and the title lookup is a view-time concern.
  A track played from album A stays resolvable after navigating to an album
  B's songs only if its id is still in `player.songs` (which isn't cleared on
  navigation) — the id fallback covers any miss; sourcing titles more
  robustly is a later plan.

**Files touched.** `src/ui/mod.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `now_playing_label` unit tests (no
  current track, title lookup, unknown-id fallback, empty songs list).
- `cargo fmt --check` passes.
- `cargo run`: after playing a song from the songs view, the Now Playing bar
  shows that song's title (e.g. "Opening") instead of its id ("song-1"); at
  boot it still shows "Nothing" (manual check — build + tests are the primary
  gate).

### Make Previous/Next step through the songs of the current album (done 2026-10-05)

Found by plan 2026-10-04.

**Goal.** The transport row renders Previous and Next buttons, but in
`WinampPlayer::update` the `Message::NextTrack | Message::PreviousTrack` arm
maps to `Task::none()` — pressing them does nothing. Land functional transport:
Previous/Next advance through the songs of the currently loaded album (the
`WinampPlayer.songs` list), reusing the existing `play_track` path so the Now
Playing bar updates. Sequencing is a player concern, so the index logic is a
pure, tested function beside the player, not in `AppleMusicService` (whose
`next_track`/`previous_track` private stubs stay unused and untouched — the
service models the library API, not the current playlist).

**Approach.**

- New file `src/ui/transport.rs`, a pure module mirroring `views.rs`:
  - `pub fn next_track_id(songs: &[Song], current: Option<&str>) -> Option<String>`
    and `pub fn previous_track_id(...) -> Option<String>`.
  - Semantics: empty `songs` → `None` (no-op). `current` is `None` or not
    found in `songs` → first song for next, last song for previous. `current`
    found at index `i` → `songs[(i + 1) % len].id` for next,
    `songs[(i + len - 1) % len].id` for previous (wrap in both directions;
    repeat/shuffle semantics are a later plan).
  - A `#[cfg(test)] mod tests` covering: empty list → `None`; `current` `None`
    → first/last; unknown `current` id → first/last; mid-list advance and
    reverse; wrap from last → first (next) and from first → last (previous).
- `src/ui/mod.rs`:
  - Add `mod transport;` beside `mod views;`.
  - In `update`, split the combined arm: `Message::NextTrack` reads
    `current_track` via `player.state.blocking_lock()`, calls
    `transport::next_track_id(&player.songs, current.as_deref())`, and returns
    `Task::done(Message::TrackSelected(id))` when `Some`, else `Task::none()`.
    `Message::PreviousTrack` does the same with `previous_track_id`.
  - Dispatching `Message::TrackSelected` (rather than calling `play_track`
    directly) reuses the one existing play path — it performs `play_track`,
    which sets `state.current_track`/`is_playing`, and re-renders via
    `TrackPlayed`; the same mechanism `boot` already uses with
    `Task::done(Message::LoadArtists)` feeds the message back into `update`.

**Files touched.** `src/ui/transport.rs` (new), `src/ui/mod.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `next_track_id` / `previous_track_id`
  unit tests (empty list, no current, unknown current, mid-list advance/reverse,
  wrap at both ends).
- `cargo fmt --check` passes.
- `Message::NextTrack` / `Message::PreviousTrack` are no longer dead: the
  `update` arms no longer map to `Task::none()`.
- `cargo run`: after loading an album's songs and playing one, Previous/Next
  move the Now Playing track through that album's songs, wrapping at the ends;
  at the artist/album list (no songs loaded) they do nothing (manual check —
  build + tests are the primary gate).

### Add a working volume slider to the transport controls (done 2026-10-05)

Found by plan 2026-10-04.

**Goal.** The transport row renders Play/Pause, Previous, and Next, and
`AppState` holds a `volume: f32` (default 0.5) that nothing currently reads or
changes — the iced 0.14 port dropped the `VolumeChange` message entirely, so
the volume control must be built from scratch (message variant, `update` arm,
and widget), not re-wired to a handler that no longer exists. Land the classic
Winamp-style volume slider: add iced's `Slider` widget to the transport row in
`WinampPlayer::view` wired to a new `Message::VolumeChange(f32)` variant and
`update` arm that stores the clamped value into `state.volume`, keeping the
clamping logic in a pure, tested helper. The iced 0.14 port has landed (done
2026-10-04), so the crate compiles on iced 0.14 as-is and this plan can build.

**Approach.**

- `src/state.rs`: add `pub fn clamp_volume(volume: f32) -> f32` returning
  `volume.clamp(0.0, 1.0)`, plus a `#[cfg(test)] mod tests` covering the three
  cases: below 0 → 0.0, above 1 → 1.0, in-range value unchanged. Volume is
  shared state, so the helper lives beside `AppState` rather than in the UI
  module.
- `src/ui/mod.rs`:
  - Extend the `iced::widget` import (`Button, Column, Row, Space, Text`) to
    include `Slider`.
  - In `WinampPlayer::view`, append `Slider::new(0.0..=1.0, state.volume,
    Message::VolumeChange)` with a fixed width (`Length::Fixed(100.0)` or
    similar) to the transport `Row` that currently holds the Play/Pause,
    Previous, and Next buttons.
  - In `WinampPlayer::update`, add a `Message::VolumeChange(volume)` arm — the
    port dropped this message, so no such arm exists today — that stores
    `state::clamp_volume(volume)` into `state.volume`.
  - Use iced 0.14's `Length::Fixed` spelling near the slider (e.g.
    `Length::Fixed(100.0)`): `Length::Pixels` does not exist in iced 0.14
    (the port's Done note records the rename from `Length::Units` to
    `Length::Fixed`).
  - In the `#[cfg(test)] mod tests`, add a `volume_change_clamps_value_before_storing`
    test driving the new `Message::VolumeChange` arm: out-of-range values are
    clamped before landing in `state.volume`, an in-range value is stored as-is
    (every other `update` arm in this module carries such a test).

**Files touched.** `src/state.rs`, `src/ui/mod.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `clamp_volume` tests (clamp below 0,
  above 1, pass through in-range) and the `volume_change_clamps_value_before_storing`
  update-arm test in `src/ui/mod.rs`.
- `cargo run` shows a volume slider in the transport row; dragging it changes
  the volume (manual check — build + tests are the primary gate).
- The new `Message::VolumeChange` is wired on both ends: a `grep -n
  'VolumeChange' src` shows both the emitter (`Slider` `on_changed` in
  `view`) and the handler in `update`.

### Make the app build on iced 0.14 and render a browsable sample library (done 2026-10-04)

Found by plan 2026-10-04.

**Goal.** The crate does not compile. `src/ui/mod.rs` implements iced's
`Application` trait against the iced ≤0.13 API — `iced_native::Command`,
`async fn update`, `Length::Units`, `WinampPlayer::run(Settings::default())` —
while `Cargo.toml` pins iced 0.14, whose `Program` requires a synchronous
`update` returning `iced::Task` and is booted via the `iced::application(boot,
update, view)` builder. Independently, the UI calls
`AppleMusicService::get_favorite_artists`, `get_albums_by_artist`,
`get_songs_from_album` and imports `Artist`, `Album`, `Song`, none of which
`src/library.rs` defines. Land the artist → album → song library-browse
flow end to end: add the missing model and service methods (backed by an
in-memory sample library; real Apple Music auth/API is a later, larger plan),
port the UI to iced 0.14, and get
`cargo build` + `cargo test` green.

**Approach.**

- `src/library.rs`:
  - Add `pub struct Artist { pub id: String, pub name: String }`,
    `pub struct Album { pub id: String, pub title: String, pub artist_id: String }`,
    `pub struct Song { pub id: String, pub title: String, pub album_id: String }`,
    each deriving `Clone, Debug, PartialEq, Serialize, Deserialize`.
- `src/apple_music.rs`:
  - Add a private builder `fn sample_library() -> SampleLibrary` returning a
    small in-memory library (3 artists, 3 albums, 5 songs) in one struct so
    the browse queries and the tests share the same data.
  - Add three `pub async fn(&self, ...) -> Result<Vec<…>, reqwest::Error>`
    methods on `AppleMusicService`: `get_favorite_artists` (all artists),
    `get_albums_by_artist(&self, artist_id: &str)` (albums whose
    `artist_id` matches), `get_songs_from_album(&self, album_id: &str)`
    (songs whose `album_id` matches); unknown id → empty `Vec`. `play_track`
    is made `pub` so the UI can call it.
  - Derive `Clone` on `AppleMusicService` and `AppleMusicToken` so the UI
    can move a cloned service into a `'static` task future.
  - Make `init_service` a synchronous module-level free function (it never
    awaited anything) and add a `#[cfg(test)] mod tests` covering: sample
    library is non-empty; `get_albums_by_artist` returns only matching
    albums and is empty for an unknown id; `get_songs_from_album` likewise.
- `src/ui/mod.rs`:
  - Drop the `iced_native`, `executor`, and `Settings` imports and the
    `Application` trait impl; boot with
    `iced::application(boot, update, view).title("nostalgia").run()`, where
    `update` is synchronous and returns `Task<Message>` and `view` returns
    `Element<'_, Message>`.
  - Replace the three `Command::perform(...)` load sites (`LoadArtists`,
    `ArtistSelected`, `AlbumSelected`) with `Task::perform`, moving a
    `service.clone()` and the id `String`s into `async move { ... }` blocks
    so the future satisfies `Task::perform`'s `'static + Send` bound.
    `TrackSelected` does the same around `play_track`, emitting a no-op
    `TrackPlayed` message so the Now Playing bar re-renders.
  - Replace `Length::Units(20)` with `Length::Fixed(20.0)` and
    `Space::with_width(...)` with `Space::new().width(...)` (both renamed in
    iced 0.14; `Length::Pixels` does not exist).
  - `view` copies the current track and play label out of the shared state
    into owned values (the `Text` widgets own their strings, so no borrow of
    the temporary `MutexGuard` escapes `view`); drop the `VolumeChange`
    message (no widget sends it).
  - Keep the `Message` enum and the `view_artists` / `view_albums` /
    `view_songs` helpers, which live in `src/ui/views.rs` as free functions
    taking `&[Artist]` / `&[Album]` / `&[Song]`.
- `src/main.rs`: `main` is now a plain synchronous `fn main() -> iced::Result`
  (no `#[tokio::main]`): iced drives its event loop synchronously on the
  calling thread, and `update`/`view` use `blocking_lock` on the shared state,
  which panics inside a tokio runtime. `ui::init_ui(state)` is no longer
  awaited.
- `Cargo.toml`: drop the `iced_native` dependency; enable tokio `macros` and
  `rt` features for the `#[tokio::test]` runtime (only `sync` was enabled).

**Files touched.** `Cargo.toml`, `Cargo.lock`, `src/library.rs`,
`src/apple_music.rs`, `src/ui/mod.rs`, `src/ui/views.rs`, `src/main.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new sample-library tests (non-empty
  library; correct filtering by `artist_id` / `album_id`; empty result for
  unknown ids).
- `cargo fmt --check` passes.
- `cargo run` starts without panicking and, with a display, shows the sample
  artists; selecting an artist lists that artist's albums, and selecting an
  album lists its songs (manual check — the build + tests are the primary
  gate).
