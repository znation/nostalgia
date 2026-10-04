# Bugs

Known bugs, recorded by any loop and fixed by the bugfix loop.
Each bug: symptom, how to reproduce, suspected cause if known. Move fixed bugs to Fixed, with the
required `**Validation gap:** <tag> — <one sentence>` line recording what made the bug hard to
confirm (tag one of: none, no-repro, no-fake, real-run-needed, no-observability, slow-check,
unclear-invariant).

## Open

### Crate does not compile — `cargo check` fails with 4 errors; no test can run (found by readme and coverage 2026-10-04)

Symptom: `cargo check` fails with `E0432`/`E0433` — `src/ui/mod.rs` uses
`apple_music::{Artist, Album, Song}` and `get_favorite_artists`/`get_albums_by_artist`/
`get_songs_from_album`, none of which exist in `src/apple_music/mod.rs`; `E0425` —
`apple_music::init_service` is a method on `AppleMusicService`, not a free function, so the call
in `main.rs` doesn't resolve; `E0404` — iced 0.14's `Application` is used as a trait in
`src/ui/mod.rs`; `E0433` — `#[tokio::main]` needs tokio features `macros` + `rt`, but Cargo.toml
enables only `sync`. Any `cargo test` dies at compile time before a test executes, so the project
currently has no runnable test suite and coverage cannot be measured at all.

How to reproduce: from the worktree, run `cargo check`.

Suspected cause: `src/ui/mod.rs` and `src/main.rs` reference APIs that do not exist as written:
- `src/ui/mod.rs:13` — unresolved import `crate::apple_music::{Artist, Album, Song}` (E0432): those
types are not defined anywhere, and `ui` calls `AppleMusicService` methods (`get_favorite_artists`,
`get_albums_by_artist`, `get_songs_from_album`) that are not implemented.
- `src/ui/mod.rs:51` — `impl Application for WinampPlayer` fails with E0404: `iced::Application` is a
struct, not a trait, in iced 0.14; the UI targets an older iced API.
- `src/main.rs:13` — `#[tokio::main]` fails with E0433: the `tokio` dependency enables only the `sync`
feature, so the `macros`/`rt` features are unavailable.
- `src/main.rs:25` — `apple_music::init_service` is not found (E0425): it exists only as the associated
function `AppleMusicService::init_service`, not as a module-level function.

The Apple Music module ships only stub methods (`authenticate`, `get_library`, `play_track`,
`pause`, `next_track`, `previous_track`) and no `Artist`/`Album`/`Song` types, while the iced UI
depends on types and methods that were never defined; the tokio/iced feature and API usage also
predates the versions in Cargo.toml.

**Validation gap:** none — reproduced deterministically with `cargo check` (exit 101).

## Fixed

_None yet._
