# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

## Planned

### Add a working volume slider to the transport controls

Found by plan 2026-10-04.

**Goal.** The transport row renders Play/Pause, Previous, and Next, and
`WinampPlayer::update` already handles `Message::VolumeChange(f32)` (clamps
and stores into `state.volume`) — but no widget ever emits it, so the volume
control is dead code and playback volume cannot actually be changed. Land the
classic Winamp-style volume slider: add iced's `Slider` widget to the transport
row in `WinampPlayer::view`, wired to `Message::VolumeChange`, and move the
clamping logic into a pure, tested helper. Assumes the iced 0.14 port above has
landed first — this touches the same `src/ui/mod.rs` and the crate must compile
for any of this to run.

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
    Message::VolumeChange)` with a fixed width (`Length::Pixels(100)` or
    similar) to the transport `Row` that currently holds the Play/Pause,
    Previous, and Next buttons.
  - In the `Message::VolumeChange(volume)` arm of `WinampPlayer::update`,
    replace the inline `volume.clamp(0.0, 1.0)` with
    `state::clamp_volume(volume)`.
  - Use iced 0.14's `Length::Pixels` spelling near the slider, per the port
    plan above.

**Files touched.** `src/state.rs`, `src/ui/mod.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `clamp_volume` tests (clamp below 0,
  above 1, pass through in-range).
- `cargo run` shows a volume slider in the transport row; dragging it changes
  the volume (manual check — build + tests are the primary gate).
- `Message::VolumeChange` is no longer dead: a `grep -n 'VolumeChange' src`
  shows both the emitter (`Slider` `on_changed` in `view`) and the handler in
  `update`.

### Make Previous/Next step through the songs of the current album

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

## Done

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
`src/apple_music/mod.rs` defines. Land the artist → album → song library-browse
flow end to end: add the missing model and service methods (backed by an
in-memory sample library, matching the existing stub data in `get_library`; real
Apple Music auth/API is a later, larger plan), port the UI to iced 0.14, and get
`cargo build` + `cargo test` green.

**Approach.**

- `src/apple_music/mod.rs`:
  - Add `pub struct Artist { pub id: String, pub name: String }`,
    `pub struct Album { pub id: String, pub title: String, pub artist_id: String }`,
    `pub struct Song { pub id: String, pub title: String, pub album_id: String }`,
    each deriving `Clone, Debug, PartialEq, Serialize, Deserialize`.
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

**Files touched.** `Cargo.toml`, `Cargo.lock`, `src/apple_music/mod.rs`,
`src/ui/mod.rs`, `src/ui/views.rs`, `src/main.rs`.

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
