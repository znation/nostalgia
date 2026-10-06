# Bugs

Known bugs, found by any loop or the review gate. Each bug: symptom, how to
reproduce, suspected cause. Move fixed bugs to Fixed.

## Open

_None yet._

## Fixed

### Now Playing bar shows the track id instead of the song title (fixed 2026-10-05)

Found by bugfix 2026-10-05.

Playing a song sets `state.current_track` to the track **id** (`"song-1"`), and
`WinampPlayer::view` rendered that raw id directly
(`state.current_track.clone().unwrap_or_else(|| "Nothing".to_string())`), so
clicking "Opening" in the Songs view made the Now Playing bar read
"Now Playing: song-1". The id is an internal key; the bar is meant to name the
song for the user.

Fixed by resolving `current_track` against the loaded `player.songs` in a pure
`now_playing_label(&[Song], Option<&str>)` helper that `view` calls: a known id
maps to the song's title, an id not in `songs` falls back to the id itself, and
no current track yields "Nothing". The helper and its four cases (stopped,
known track, unknown id, no songs loaded) are unit-tested in
`src/ui/views.rs` (the helper, its tests, and the Now Playing bar and
transport row construction moved there from `src/ui/mod.rs` on 2026-10-06);
`cargo build`, `cargo test`, and `cargo fmt --check` all pass.

**Validation gap:** unclear-invariant — the Now Playing bar's contract (show a
human-readable song title, not the internal id) was never specified or tested,
so that invariant had to be reconstructed before the fix.

### Crate does not compile — `cargo check` fails with 5 errors (4 root causes); no test can run (fixed 2026-10-04)

Fixed by the iced 0.14 port (PLANS.md "Make the app build on iced 0.14 and
render a browsable sample library", done 2026-10-04). The four root causes:
1. `Artist`/`Album`/`Song` types now live in `src/library.rs`, and the
   `get_favorite_artists` / `get_albums_by_artist` / `get_songs_from_album`
   methods exist on `AppleMusicService` in `src/apple_music.rs`, backed by
   an in-memory sample library.
2. The UI was ported from the pre-0.14 `Application` trait to the
   `iced::application(boot, update, view)` builder with a synchronous
   `update` returning `Task`; `iced_native` was dropped from Cargo.toml.
3. `#[tokio::main]` was removed: `main` is a plain synchronous
   `fn main() -> iced::Result`, so no tokio `macros`/`rt` features are needed
   at runtime (they are enabled only for the `#[tokio::test]` suite).
4. `apple_music::init_service` is a module-level free function again, so the
   call in `main.rs` resolves.

`cargo build`, `cargo test` (the new sample-library tests included), and
`cargo fmt --check` all pass.
