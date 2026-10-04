# Bugs

Known bugs, recorded by any loop and fixed by the bugfix loop.
Each bug: symptom, how to reproduce, suspected cause if known. Move fixed bugs to Fixed, with the
required `**Validation gap:** <tag> — <one sentence>` line recording what made the bug hard to
confirm (tag one of: none, no-repro, no-fake, real-run-needed, no-observability, slow-check,
unclear-invariant).

## Open

- **The crate does not compile (four errors)** — (found by readme 2026-10-04).
  Symptom: `cargo check` fails with `E0432`/`E0433` — `src/ui/mod.rs` uses
  `apple_music::{Artist, Album, Song}` and `get_favorite_artists`/`get_albums_by_artist`/
  `get_songs_from_album`, none of which exist in `src/apple_music/mod.rs`; `E0425` —
  `apple_music::init_service` is a method on `AppleMusicService`, not a free function, so the call
  in `main.rs` doesn't resolve; `E0404` — iced 0.14's `Application` is used as a trait in
  `src/ui/mod.rs`; `E0433` — `#[tokio::main]` needs tokio features `macros` + `rt`, but Cargo.toml
  enables only `sync`.
  Reproduce: `cargo check` from a clean checkout.
  Suspected cause: the Apple Music module ships only stub methods (`authenticate`, `get_library`,
  `play_track`, `pause`, `next_track`, `previous_track`) and no `Artist`/`Album`/`Song` types,
  while the iced UI depends on types and methods that were never defined; the tokio/iced feature
  and API usage also predates the versions in Cargo.toml.
  **Validation gap:** none — reproduced deterministically with `cargo check` (exit 101).

## Fixed

_None yet._
