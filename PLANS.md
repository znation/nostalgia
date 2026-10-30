# Plans

## Planned

_None yet._


### Show the songs you played this run in the signed-out Recently Played list (planned 2026-10-29 by the plan role)

**Goal.** While no MusicKit session is stored, the Recently Played button shows the service
seam's stand-in — the sample library's songs in reversed library order
(`sample_recently_played_songs`, `src/apple_music/sample.rs`) — and keeps showing that fixed list
however much you play: play five songs signed out, reopen Recently Played, and you still see the
same stand-in instead of anything you chose. While a session is stored the service's own
list answers, so the wart is only the signed-out one and it only needs data the app already
records: every committed play lands in `PlayHistory` (`src/ui/transport/history.rs`). Make the
signed-out list show the songs you actually played this run, newest first, falling back to the
sample stand-in while nothing has played yet. A relaunch starts empty again (the list is not
persisted — that is the separate offline Recently-Played-persistence idea, not this plan), so
this entry is independent of the waitlisted saved-sign-in plan above it: it touches playback
recording and the browse handler, not settings, auth, or the save path, and merges in either
order.

**Approach.** A second recorder beside the play history, and one branch in the Recently Played
open path.

- `src/ui/transport/history.rs`, in the same module as `PlayHistory` (so it reuses the private
  `HISTORY_LIMIT`): add `pub(in crate::ui) struct PlayedSongs { items: Vec<Song> }` with
  `record(&mut self, song: Song)` — drop an earlier row with the same id and push the new one at
  the back, mirroring `PlayHistory::record`'s move-to-back dedup, and let rows fall off the front
  beyond `HISTORY_LIMIT` — `newest_first(&self) -> Vec<Song>` (a reversed clone, rows most recent
  first), and `is_empty(&self) -> bool`. Unit tests beside the existing `PlayHistory` ones
  pin dedup (recording the same song again keeps one row, now newest), the cap (more plays than
  `HISTORY_LIMIT` keeps exactly that many rows, the oldest falling off the front), and
  newest_first's order.
- `src/ui/player.rs`: `WinampPlayer` gains a `played_songs: PlayedSongs` field, initialized
  empty beside `play_history` in `WinampPlayer::new`.
- `src/ui/handlers/playback.rs` `track_played`, inside the committed arm where
  `play_history.record(current)` runs: from the same `match player.current_view` items slice the
  block already uses for `shuffle_round`, find the `Song` whose id equals the committed id and
  `record` a clone of it. A committed play whose track is absent from the showing list (say the
  resolved-but-listed-nowhere restored track) feeds the play history only — exactly as the
  committed arm does today — and adds no played-list row.
- `src/ui/handlers/browse.rs` `show_recently_played`: keep `switch_view` and the fetch, but when
  `player.apple_music_service.session().is_none()` and the played list is non-empty, replace the
  `fetch_at_top(fetch_level(...))` call with storing the played rows: clear-then-
  `store_with_row_text(newest_first(), display_title, length)` on `player.recently_played`, the
  same transition a fetched reply uses (a fresh click clears the stale list and bumps its epoch,
  so an in-flight media press cannot play into an in-flight list). The signed-in branch — and the
  session-arrives refetch through `session_ready` → `restore_view`, which replaces the played
  rows with the service's list — stays today's unchanged path.
- Document the change: `docs/usage.md`'s Recently Played sentence says the signed-out list shows
  the songs you played this run newest first (the sample stand-in while nothing has played yet,
  and the same sample list again after a relaunch), and the list refreshes on every reopen like
  the other flat lists; `docs/invariants.md` gains a short `## Signed-out Recently Played`
  section pinning the guarantee (recorded plays, stand-in fallback, signed-in refetch) to the new
  tests.

**Files touched.** `src/ui/transport/history.rs`, `src/ui/player.rs`,
`src/ui/handlers/playback.rs`, `src/ui/handlers/browse.rs`, `src/ui/tests/
recently_played_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- Signed out (the `test_player` service stores no session): open Recently Played and play two
  sample songs, then reopen it — the list holds exactly those two songs, the first press's song
  newest first, numbered from 1 like the other lists, playing like any other song row, with the
  footer's count and playing time over its rows.
- With nothing played this run, the signed-out list is the sample stand-in as today — the
  existing reversed-sample-songs test keeps passing.
- Playing a song a second time keeps one row for it, moved to the newest position; the list cap
  is `HISTORY_LIMIT` rows, matching the play history's cap.
- Only a committed play completion arriving through `track_played`'s committed arm adds a
  played-list row — a failed or superseded completion records none, matching the play-history
  rule that arm already enforces.
- With a stored session (the UI test stores one through `authenticate_with` with a stub
  `authorize`, the way the session-seam tests do) reopening Recently Played ignores the played
  list and fetches the service's list as today; after sign-in the showing list is refetched and
  the played rows drop.
- The signed-in code path — `fetch_at_top`, `RecentlyPlayedLoaded`/`LoadFailed` handling,
  selection, Sort/Ctrl+R/Shuffle-row reorders over the list — is unchanged.
- `make check` passes.

## Done
