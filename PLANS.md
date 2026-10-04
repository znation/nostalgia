# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

## Planned

### Make the app build on iced 0.14 and render a browsable sample library

Found by plan 2026-10-04.

**Goal.** The crate does not compile. `src/ui/mod.rs` implements iced's
`Application` trait against the iced ≤0.13 API — `iced_native::Command`,
`async fn update`, `Length::Units`, `WinampPlayer::run(Settings::default())` —
while `Cargo.toml` pins iced 0.14, whose `Program` requires a synchronous
`update` returning `iced::Task` and is booted via the `iced::application(boot,
update, view)` builder. Independently, the UI calls
`AppleMusicService::get_favorite_artists`, `get_albums_by_artist`,
`get_songs_from_album` and imports `Artist`, `Album`, `Song`, none of which
`src/apple_music/mod.rs` defines, and `src/main.rs` awaits the synchronous
`ui::init_ui`. Land the artist → album → song library-browse flow end to end:
add the missing model and service methods (backed by an in-memory sample
library, matching the existing stub data in `get_library`; real Apple Music
auth/API is a later, larger plan), port the UI to iced 0.14, and get
`cargo build` + `cargo test` green.

**Approach.**

- `src/apple_music/mod.rs`:
  - Add `pub struct Artist { pub id: String, pub name: String }`,
    `pub struct Album { pub id: String, pub title: String, pub artist_id: String }`,
    `pub struct Song { pub id: String, pub title: String, pub album_id: String }`,
    each deriving `Clone, Debug, PartialEq, Serialize, Deserialize`.
  - Add a private builder, e.g. `fn sample_library() -> Vec<Artist>`, that
    returns a small in-memory library (2–3 artists, each with 1–2 albums,
    each with a few songs).
  - Add three `pub async fn(&self, ...) -> Result<Vec<…>, reqwest::Error>`
    methods on `AppleMusicService`: `get_favorite_artists` (all artists),
    `get_albums_by_artist(&self, artist_id: &str)` (albums whose
    `artist_id` matches), `get_songs_from_album(&self, album_id: &str)`
    (songs whose `album_id` matches); unknown id → empty `Vec`.
  - Derive `Clone` on `AppleMusicService` and `AppleMusicToken` so the UI
    can move a cloned service into a `'static` task future.
  - Add a `#[cfg(test)] mod tests` covering: sample library is non-empty;
    `get_albums_by_artist` returns only matching albums and is empty for an
    unknown id; `get_songs_from_album` likewise.
- `src/ui/mod.rs`:
  - Drop the `iced_native` and `executor` imports and the `Application`
    trait impl; boot with `iced::application(boot, update, view).run()` per
    iced 0.14's own example (or the equivalent `application!` macro), where
    `update` is synchronous and returns `Task<Message>` and `view` returns
    `Element<Message>`.
  - Replace the three `Command::perform(...)` load sites (`LoadArtists`,
    `ArtistSelected`, `AlbumSelected`) with `Task::perform`, moving a
    `service.clone()` and the id `String`s into `async move { ... }` blocks
    so the future satisfies `Task::perform`'s `'static + Send` bound.
  - Replace `Length::Units(20)` with `Length::Pixels(20)` (renamed in 0.14).
  - Keep the `Message` enum and `view_artists` / `view_albums` /
    `view_songs` helpers; adjust `Text::new(...).size(..)` to iced 0.14's
    text widget API only as the compiler requires.
- `src/main.rs`: drop the `.await` on the synchronous `ui::init_ui(...)`
  call so `main` compiles.

**Files touched.** `src/apple_music/mod.rs`, `src/ui/mod.rs`, `src/main.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new sample-library tests (non-empty
  library; correct filtering by `artist_id` / `album_id`; empty result for
  unknown ids).
- `cargo run` starts without panicking and, with a display, shows the sample
  artists; selecting an artist lists that artist's albums, and selecting an
  album lists its songs (manual check — the build + tests are the primary
  gate).

## Done

_None yet._
