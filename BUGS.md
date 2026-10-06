# Bugs

Known bugs, found by any loop or the review gate. Each bug: symptom, how to
reproduce, suspected cause. Move fixed bugs to Fixed.

## Open

### Organize's ui-test-suite extraction burned 5 ticks / 0.6 h on a disproven "byte-identical" VERIFIED claim and never landed (found by telemetry 2026-10-06)

Found by telemetry 2026-10-06.

The day's top loss cause: 5 ticks, 0.6 h · $0.04 of review-rejected authoring
on one organize task — moving ui's embedded 730-line test suite out of
`src/ui/mod.rs` into `src/ui/tests.rs` — with no landed result. The rejected
authoring commit `51e87bc` ("Move ui's embedded 730-line test suite into
src/ui/tests.rs", unmerged; parent `a2d6176` on main) claims in VERIFIED that
`cargo fmt --check` is clean on both ui files and that a diff of the old
embedded module versus the new `tests.rs` leaves the file byte-identical.
The reviewer disproved that: `tests.rs` is not byte-identical to the removed
module — three rustfmt-required re-wraps, +181 chars. The branch's own stat
bears this out: it removes 730 lines from `src/ui/mod.rs` but adds only 722 in
`src/ui/tests.rs`, so the extraction is not byte-equal, and `make check`
(which runs `cargo fmt --check`) on the branch would have shown the three
re-wraps immediately. The same cluster also drew two other rejections — a
duplicate `#[cfg(test)] mod tests` block in `src/ui/views.rs` (two blocks,
one starting at line 88) and a branch based on stale main `64ca8c9` (main 15
commits ahead) — each flagged as "3 consecutive tick failures" before the
task was abandoned, unlanded; current main still embeds the suite in
`src/ui/mod.rs` and has no `src/ui/tests.rs`.

**Wrong harness response:** after the first disproven VERIFIED claim, the
re-authoring loop kept re-rolling the same task instead of running the
project's own gate (`make check` → `cargo fmt --check`) to reconcile the
claim, and escalated only after the third consecutive failure. The root cause
is an authoring/claim defect: a universal word ("byte-identical") was asserted
without the byte-level check that would have disproven it — the loop backed it
with a whitespace-insensitive diff, which cannot establish byte-identity and
never runs rustfmt.

Repro: at `51e87bc`, `cargo fmt --check` on `src/ui/tests.rs` reports the
three re-wraps; alternatively diff the 730 lines removed from `src/ui/mod.rs`
against the 722 added in `src/ui/tests.rs` (byte diff is non-empty). The
branch is the unmerged head of that organize run — there is no correlated
merged commit because the work never landed.

Suspected cause: the organize loop reported a VERIFIED "fmt-clean /
byte-identical" outcome it had not actually confirmed with `cargo fmt
--check`, and the harness's rejection loop did not force the branch through
`make check` or escalate after the first disproven claim, so five ticks
burned with nothing landing.

## Fixed

### Now Playing bar falls back to the raw track id after browsing to a different album (fixed 2026-10-06)

Found by bugfix 2026-10-06.

The 2026-10-05 fix resolves the Now Playing bar's label against the
currently-browsed album's songs (`player.songs`), and that buffer is replaced
every time `SongsLoaded` lands. So play song-1 from "First Record" (album-1),
then browse away to "Second Record" (album-2): `player.songs` is now album-2's
list, song-1 is no longer "known", and the bar — which is meant to name the
song for the user — falls back to the raw internal id "song-1".

Fixed by accumulating every loaded song in a new `known_songs` buffer: the
`SongsLoaded` arm stores into `player.songs` (as before) and folds each new
song into `known_songs`, and `view` resolves the bar's label through
`WinampPlayer::now_playing_label`, which looks the current track up in
`known_songs` instead of `songs`. `player.songs ⊆ known_songs` always, so
every case that worked before still works, and the browse-away case now names
the playing track. The regression test
(`now_playing_label_keeps_the_track_name_after_browsing_to_another_album`)
asserts the bar's label through the same `WinampPlayer::now_playing_label`
path that `view` renders, after playing a song and browsing to another album —
it fails with the raw id both when `known_songs` is not accumulated and when
the resolution is reverted to `songs`. `cargo build`, `cargo test`, and
`cargo fmt --check` all pass.

**Validation gap:** no-observability — the failing label is computed inside
`view`'s iced `Element`, which no test path inspected, so the browse-away
regression left no trace in the suite.

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
