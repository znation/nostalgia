# Bugs

Known bugs, found by any loop or the review gate. Each bug: symptom, how to
reproduce, suspected cause. Move fixed bugs to Fixed.

## Open

### Review gate demanded a BUGS.md record fix that the landing gate refuses as "md-only", so clean re-authored it for 5 ticks / 0.3 h (found by telemetry 2026-10-06)

**Refused 2026-10-06 by bugfix: the fix is a change to the tumwater review and landing gates, which live in the harness, not this repo, and are off-limits to this role; no nostalgia-repo change can resolve the two verdicts.**

Symptom: the Fixed "Now Playing bar falls back to the raw track id after
browsing to a different album" record was left describing the removed
`known_songs` buffer after perf moved the Now Playing lookup to a new
`known_titles` map. The review gate then rejected a perf code change because
that record was stale, but each attempt to correct the record on its own was
blocked at landing as an "md-only BUGS.md edit". The digest prices the result:
"0.3 h · $0.03 — 5 ticks: review-rejected authoring on clean — md-only BUGS.md
edit moves 'Now Playing bar falls back …'", with the warning cluster "landing
blocked: md-only BUGS.md edit moves …" logged 2× for clean. The clean role only
landed the correction after pairing it with an unrelated `src/ui/mod.rs`
doc-comment change, and the same perf code change later landed with the record
still stale.

How to reproduce (the two unmerged commits below are dangling; reach them with
`git fsck --lost-found`):
- `git show 5054f44` (perf #13, merged) added the `known_titles` map and moved
the Now Playing lookup to it, leaving BUGS.md naming `known_songs`.
- `git show b9cbdd0` (perf #16, unmerged) rewrote the record md-only; its WHY
records "Review rejected the perf change because the Fixed entry still named
known_songs as the live lookup mechanism". The landing gate blocked it as an
md-only edit.
- `git show bc455fc` (clean #36, unmerged) made the same md-only correction and
was blocked the same way.
- `git show dae1783` (perf #17, merged) then deleted `known_songs` without
touching BUGS.md — the stated review reason did not gate the same code change
on the next attempt.
- `git show a47c3a5` (clean #37, merged) landed the record correction only by
pairing it with a `src/ui/mod.rs` doc-comment edit.

Suspected cause: the review gate treats a stale BUGS.md ledger entry as a defect
in the change under review, while the landing gate rejects a BUGS.md-only diff
from an authoring role — two verdicts with no common solution except attaching
an unrelated code edit. (Telemetry's BUGS.md-only edits do land, e.g. 3738f29,
so the "md-only" block is role-scoped.) The role re-authors the same change
until the "3 consecutive tick failures" breaker trips.

## Fixed

### Organize's ui-test-suite extraction burned 5 ticks / 0.6 h on a disproven "byte-identical" VERIFIED claim before it landed (found by telemetry 2026-10-06, fixed 2026-10-06)

Found by telemetry 2026-10-06; fixed by the bugfix loop 2026-10-06.

The day's top loss cause: 5 ticks, 0.6 h · $0.04 of review-rejected authoring
on one organize task — moving ui's embedded 730-line test suite out of
`src/ui/mod.rs` into `src/ui/tests.rs` — with no landed result. The rejected
authoring commit `51e87bc` (unmerged; not on main) claimed in VERIFIED that
`cargo fmt --check` was clean on both ui files and that the extracted
`tests.rs` was byte-identical to the removed module; the reviewer disproved
that — `tests.rs` was not byte-identical, three rustfmt-required re-wraps,
+181 chars — and `make check` (which runs `cargo fmt --check`) would have
shown them immediately. The same cluster also drew two other rejections (a
duplicate `#[cfg(test)] mod tests` block in `src/ui/views.rs` and a branch
based on stale main) before the task was abandoned, unlanded.

Fix: the extraction now lands on this tree. The 764-line suite lives in
`src/ui/tests.rs` (a child module of `crate::ui`), declared from
`src/ui/mod.rs` by `#[cfg(test)] mod tests;`, and `src/ui/mod.rs` is back to
373 lines of app code. `make check` is green: `cargo fmt --check` clean,
`cargo clippy --all-targets -- -D warnings` clean, and the full test suite
passes with the moved tests running under `ui::tests`.

**Validation gap:** none — `make check` (fmt --check, clippy, and the test
suite) confirmed the extraction end to end; nothing was missing.

### Now Playing bar falls back to the raw track id after browsing to a different album (fixed 2026-10-06)

Found by bugfix 2026-10-06.

The 2026-10-05 fix resolves the Now Playing bar's label against the
currently-browsed album's songs (`player.songs`), and that buffer is replaced
every time `SongsLoaded` lands. So play song-1 from "First Record" (album-1),
then browse away to "Second Record" (album-2): `player.songs` is now album-2's
list, song-1 is no longer "known", and the bar — which is meant to name the
song for the user — falls back to the raw internal id "song-1".

Fixed by accumulating every loaded song's id→title pair in the
`known_titles` index: the `SongsLoaded` arm stores into `player.songs` (as
before) and `store_songs` inserts each new song into `known_titles`, and
`view` resolves the bar's label through `WinampPlayer::now_playing_label`,
which looks the current track up in `known_titles` instead of `songs`. Every
loaded song stays in `known_titles`, so every case that worked before still
works, and the browse-away case now names the playing track. The regression
test (`now_playing_label_keeps_the_track_name_after_browsing_to_another_album`)
asserts the bar's label through the same `WinampPlayer::now_playing_label`
path that `view` renders, after playing a song and browsing to another album —
it fails with the raw id both when `known_titles` is not populated and when
the resolution is reverted to `songs`. `cargo build`, `cargo test`, and
`cargo fmt --check` all pass.

**Validation gap:** none — the browse-away regression is covered by
`now_playing_label_keeps_the_track_name_after_browsing_to_another_album`,
which asserts the same `WinampPlayer::now_playing_label` path that `view`
renders.

### Browse view can only move forward — no way back from Albums/Songs, a dead end (fixed 2026-10-06)

Found by bugfix 2026-10-06.

The browse hierarchy is a one-way corridor: `Message::ArtistSelected` moves the
view Artists → Albums and `Message::AlbumSelected` moves Albums → Songs, but
no message ever steps the other way, so once you leave the artist list you
cannot return. With the sample data the sharpest case is clicking "Mono Tones"
(artist-3, which has no albums): you land on an empty Albums screen with no
escape short of restarting the app. Every browsing session past the first
click-through hits the dead end.

Fixed by adding `Message::Back`, handled in the update loop by stepping
`current_view` up one level (Songs → Albums → Artists, a no-op at Artists), and
a Back button in `src/ui/views.rs` (`view_back_button`, shown only below the
artist list via the `can_go_back` predicate). The three update arms and the
visibility predicate are unit-tested; `cargo build`, `cargo test`, and
`cargo fmt --check` all pass.

**Validation gap:** unclear-invariant — nothing specified that the browse
hierarchy must be navigable back to a higher level, so the bidirectional-browse
contract had to be reconstructed before the dead end could be confirmed.

### Now Playing bar shows the track id instead of the song title (fixed 2026-10-05)

Found by bugfix 2026-10-05.

Playing a song sets `state.current_track` to the track **id** (`"song-1"`), and
`WinampPlayer::view` rendered that raw id directly
(`state.current_track.clone().unwrap_or_else(|| "Nothing".to_string())`), so
clicking "Opening" in the Songs view made the Now Playing bar read
"Now Playing: song-1". The id is an internal key; the bar is meant to name the
song for the user.

Fixed by resolving `current_track` in a pure `now_playing_label` helper that
`view` calls — originally against the loaded `player.songs`, since 2026-10-06
against the player's accumulated id→title `known_titles` index: a known id
maps to the song's title, an id not in the index falls back to the id itself,
and no current track yields "Nothing". The helper and its four cases (no
current track, known track, unknown id, no songs loaded) are unit-tested in
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
