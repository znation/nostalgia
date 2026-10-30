# Plans

## Planned

_None yet._

### Move `src/settings.rs`'s in-file test module to `src/settings/tests.rs` (planned 2026-10-30 by the plan role)

**Goal.** `src/settings.rs` is the one source file no longer readable in one sitting: `wc -l`
measures it at 1202, and almost all of that is its in-file `#[cfg(test)] mod tests` block, the
file's only test home. PRINCIPLES.md pins "Keep each file focused on one responsibility, and
small enough to read in one sitting," and the codebase already settled the sibling-file pattern
for subsystem tests (`src/audio/tests/`, `src/apple_music/tests/`, `src/ui/tests/`, and
`src/ui/views/tests/`), while `src/settings/` already holds the source's own submodules.
This entry moves the test module out to `src/settings/tests.rs` so the `Settings` source reads
in one sitting; nothing else changes.

**Ground truth verified this run (2026-10-30).**

- The in-file module `#[cfg(test)] mod tests` carries the whole suite (helpers like
  `state_with_every_field_set` and tests like `to_text_round_trips_every_field` and the
  restore/`apply_to` cases) and its only preamble is `use super::*;`. Moving it to a child
  module file needs no import edits: the moved file stays a child of `crate::settings`, so
  `use super::*;` keeps working there, private items (`mod text`) stay accessible to it, and
  every test name and count stays the same.
- `src/settings/` today holds the source submodules `atomic_write.rs`, `file_store.rs`,
  `library_view.rs`, `seam.rs`, the private `text.rs`, and `writer.rs`, and no
  `tests.rs` sibling yet.
- `docs/invariants.md` pins four moved tests with the path `src/settings.rs`:
  `a_line_broken_played_id_writes_one_whole_settings_line` (its anchor names two tests "both
  in `src/settings.rs`"), `autoscroll_round_trips_and_ignores_malformed_values`,
  `eq_custom_preset_lines_round_trip_and_skip_the_malformed_ones`, and
  `a_line_broken_playlist_song_id_writes_one_whole_settings_line`. Those path anchors change
  to `src/settings/tests.rs` in this change; no guarantee sentence is touched.
- `docs/usage.md` names no settings test path; nothing there changes.
- No other source file carries an in-file test module of comparable size (`state.rs` 919,
  `library.rs` 655, `sample_library.rs` 637, `equalizer.rs` 492 lines), so this is a single
  focused change, not the start of a series.
- A grep of PLANS.md, PLANS-DONE.md, and BUGS.md for a settings test-module split shows no
  prior entry covers it; the waiting clamp plan touches `src/settings.rs` only in prose.

**Approach.**

1. Create `src/settings/tests.rs` holding the moved block verbatim: the `use super::*;`
   preamble plus every helper and test, unchanged.
2. `src/settings.rs`: replace the in-file block with a `#[cfg(test)] mod tests;`
   declaration beside the module declarations at the top of the file, carrying a one-line
   comment naming the sibling-file pattern (like `src/audio/tests/`). The source keeps only
   the module doc, the submodule declarations, the `Settings` and `SavedPlaylist` types,
   `from_state`/`apply_to`, and the accessors.
3. `docs/invariants.md`: change the four `src/settings.rs` path anchors named in ground
   truth to `src/settings/tests.rs`. No other edits.

Nothing visible to the player changes, so no `winamp-screenshots/` comparison and no
`docs/usage.md` change applies. No new test ships with it — this changes no behavior; every
moved test runs unchanged.

**Acceptance criteria.**

- `src/settings.rs` keeps every symbol the rest of the code reaches (`Settings`,
  `SavedPlaylist`, `from_state`, `apply_to`, the accessors) at the same paths, so no other
  module's imports change and the diff on non-test code is only the `mod tests;` declaration.
- `src/settings/tests.rs` compiles with `use super::*;` as its preamble and the moved items,
  and the moved suite runs under the same names under `make check`.
- `make check` passes with the moved tests enabled.
- `docs/invariants.md`'s four moved-test anchors name `src/settings/tests.rs`.

**Size.** One new file (a verbatim move of the in-file block), one declaration plus a
comment line in `src/settings.rs`, and four one-word path edits in `docs/invariants.md`.
No behavior change, so no new tests; out of scope: any further split of the `Settings`
struct, accessors, or `text` module, and any change to state.rs, library.rs, or
sample_library.rs.

## Done
