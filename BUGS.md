# Bugs

Known bugs, recorded by any loop and fixed by the bugfix loop.
Each bug: symptom, how to reproduce, suspected cause if known. Move fixed bugs to Fixed, with the
required `**Validation gap:** <tag> — <one sentence>` line recording what made the bug hard to
confirm (tag one of: none, no-repro, no-fake, real-run-needed, no-observability, slow-check,
unclear-invariant).

## Open

### Crate does not compile — `cargo check` fails with 5 errors (4 root causes); no test can run (found by readme, coverage, and bugfix 2026-10-04)

Symptom: `cargo check` fails with `E0432`/`E0433` — `src/ui/mod.rs` and
`src/ui/views.rs` import `apple_music::{Artist, Album, Song}`, and `ui` calls
`get_favorite_artists`/`get_albums_by_artist`/
`get_songs_from_album`, none of which exist in `src/apple_music/mod.rs`; `E0425` —
`apple_music::init_service` is a method on `AppleMusicService`, not a free function, so the call
in `main.rs` doesn't resolve; `E0404` — iced 0.14's `Application` is used as a trait in
`src/ui/mod.rs`; `E0433` — `#[tokio::main]` needs tokio features `macros` + `rt`, but Cargo.toml
enables only `sync`. Nothing builds or runs: any `cargo test` dies at compile time before a test
executes, so the project currently has no runnable test suite and coverage cannot be measured at all.

How to reproduce: from the worktree, run `cargo check`.

Suspected cause — four coupled breaks, all of which must land together before the crate compiles
(the landing check runs the project build, so no sub-part can land or be verified on its own):
1. `src/ui/mod.rs:11` and `src/ui/views.rs:13` — unresolved import `crate::apple_music::{Artist, Album, Song}` (E0432): those
types are not defined anywhere, and `ui` calls `AppleMusicService` methods (`get_favorite_artists`,
`get_albums_by_artist`, `get_songs_from_album`) that are not implemented.
2. `src/ui/mod.rs:49` — `impl Application for WinampPlayer` fails with E0404: the UI is written
against the pre-0.14 iced trait API (`Application` as a trait, `Command` from `iced_native` 0.10.3,
async `update`, `Length::Units`, `Element<'a, Message>`), but the project depends on iced 0.14.0
where `Application` is a builder struct and the model is `iced::application(boot, update, view)` /
`Program` with a synchronous `update` returning `Task`. A migration must also decide how
`AppleMusicService` (currently `Arc<Mutex<AppState>>`-backed and async) interacts with iced's
synchronous, UI-owned state.
3. `src/main.rs:10` — `#[tokio::main]` fails with E0433: the `tokio` dependency enables only the
`sync` feature (Cargo.toml) — needs `macros`/`rt` features or a hand-rolled runtime.
4. `src/main.rs:22` — `apple_music::init_service` is not found (E0425): it exists only as the
associated function `AppleMusicService::init_service`, not as a module-level function.

The Apple Music module ships only stub methods (`authenticate`, `get_library`, `play_track`,
`pause`, `next_track`, `previous_track`) and no `Artist`/`Album`/`Song` types, while the iced UI
depends on types and methods that were never defined; the tokio/iced feature and API usage also
predates the versions in Cargo.toml.

**Validation gap:** none — reproduced deterministically with `cargo check` (exit 101).

## Fixed

_None yet._
