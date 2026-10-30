# Plans

## Planned

_None yet._

### Persist the signed-out Recently Played list across relaunches via the settings file (planned 2026-10-29 by the plan role)

**Goal.** Play songs signed out, quit, and the Recently Played view starts over from the sample
stand-in: the run's rows live only in `WinampPlayer::played_songs` (`src/ui/transport/history.rs`),
and the `## Signed-out Recently Played` invariant in docs/invariants.md even documents "always
after a relaunch, the list is not persisted". Save that list in the settings file like every
other remembered choice and restore it at the next launch, so the songs you played survive the
window closing. This is the offline-persistence sibling of the merged signed-out-list run plan
(PLANS-DONE.md explicitly defers it); it needs no new persistence machinery — the saved-playlists
`user_playlist=` codec and the read-only `AppleMusicService::resolve_song` already cover every
piece. Out of scope: the signed-in list keeps fetching the service's own Recently Played on every
open, with a stored session nothing restores or changes; ids the launch cannot answer are dropped
at restore, exactly like a saved playlist's stale ids; any position-within-track memory; any
change to `PlayHistory` (Shuffle's Previous) or `last_played` (`current_track=`).

**Encoding.** One line, newest first, ids only:
`recently_played=<id>,<id>,…,<id>` — the first id is the most recently played song, matching
`PlayedSongs::newest_first` so the round trip is order-identical. Only ids are stored (a catalog
id or a `song-N` sample id, never a name with text-unsafe characters), written when non-empty.
Parse mirror: blank value or all-empty, whitespace-trimmed tokens are skipped like
`parse_user_playlist`'s malformed forms; the cap stays `PlayedSongs`'s business (`HISTORY_LIMIT`),
so the settings codec stays value-blind exactly like `user_playlist`.

**Approach.**

- `src/settings.rs`: add the `recently_played: Vec<String>` field (empty by default), a
  `#[must_use] pub fn recently_played(&self) -> &[String]` getter, and
  `pub fn set_recently_played(&mut self, ids: Vec<String>)` beside the other snapshot setters;
  doc the field as the signed-out list's ids in play order, newest first. In `to_text`
  (src/settings/text.rs, right after the `user_playlist=` block, the settings file's other
  id list) emit the line only when the list is non-empty.
- `src/settings/text.rs`: route the token in the parse match beside `"user_playlist"` and add
  `fn parse_recently_played(&mut self, value: &str)` beside `parse_user_playlist`: no `:` split here
  (there is no name), comma-split, `str::trim`, drop empty tokens, keep the order verbatim, store
  in `recently_played` only when at least one id survives; a malformed line leaves the field alone
  and the other lines still load. Field docs state: ids arrived with by the player (an id the
  library no longer answers is dropped at restore time) are kept verbatim here.
- `src/ui/transport/history.rs`: extend `PlayedSongs` with
  `pub(in crate::ui) fn newest_first_ids(&self) -> Vec<String>` (the ids in `newest_first`'s order,
  what the snapshot stores) and `pub(in crate::ui) fn restore(&mut self, newest_first: Vec<Option<Song>>)`
  that walks the reply oldest first and funnels every `Some(song)` through `record`, so replays
  dedupe, the cap is enforced, and a play that committed while the restore was in flight keeps its
  row and its newest position. Update the type's doc comment, which currently promises "the list
  lives only for this run: it is not persisted".
- `src/ui/message.rs`: add `Message::RecentlyPlayedResolved { songs: Vec<Option<Song>> }` beside
  `CurrentTrackResolved`, with a doc comment noting it carries the boot restore's reply and is
  applied only while no session is stored.
- `src/ui/mod.rs`: in `boot`'s task batch add `Task::done(Message::ResolveRecentlyPlayed)` right
  after `Task::done(Message::ResolveCurrentTrack)` (both run after the init closure's
  `adopt_saved_settings` seeded the player, so tasks see the parsed ids), and route
  `Message::RecentlyPlayedResolved` to `handlers::playback::recently_played_resolved` in the
  `Message::ResolveCurrentTrack` arm's neighborhood of `update`.
- `src/ui/handlers/playback.rs`: two functions beside the `resolve_current_track` pair, mirroring
  their shape:
  - `pub(in crate::ui) fn resolve_recently_played(player) -> Task<Message>` — a no-op when a
    session is stored (`AppleMusicService::session().is_some()`) or when
    `player.recently_played_seeded` is empty; otherwise issues `resolve_song` per seeded id
    sequentially in one `Task::perform` (sample-library ids answer immediately with no session and
    no network; a signed-in Apple id is never restored here), replying `RecentlyPlayedResolved`. No re-issue at `session_ready`: with a session stored the
    view fetches the service's list anyway and the rows are dropped, so a persistence sweep that
    never fires is harmless — document that decision in the function's doc comment.
  - `pub(in crate::ui) fn recently_played_resolved(player, songs) -> Task<Message>` — skip when a
    session is now stored (sign-in raced the restore); otherwise `restore(songs)` and nothing
    else: no fetch, no view change, no transport state. The list stays put until the user opens
    Recently Played, whose signed-out arm shows `newest_first`.
- `src/ui/player.rs`: add `pub(in crate::ui) recently_played_seeded: Vec<String>` — the ids the
  launch parsed, adopted in `adopt_saved_settings` from `saved.recently_played()`. This field is
  the anti-erase guard: without it, the shared `persist_settings_if_changed` sweep would write the
  empty pre-restore snapshot over the saved line the moment any message fires before the async
  restore lands — the same class of bug as the `music_kit_user_token=` erase (BUGS-FIXED.md,
  fixed 2026-10-29). In `persist_settings_if_changed`, feed the setter from the rows when they are
  non-empty and from the seed when they are not:
  `snapshot.set_recently_played(if self.played_songs.is_empty() { self.recently_played_seeded.clone() }
  else { self.played_songs.newest_first_ids() })`, doc'd so: the seed holds parity with the saved
  snapshot until the restore answers, a fully-resolved restore then matches the seed again (no
  save), and a restore that dropped unnameable ids shortens the list — the documented drop.

**Files touched.** `src/settings.rs`, `src/settings/text.rs`, `src/ui/transport/history.rs`,
`src/ui/message.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/player.rs`, tests in
`src/settings.rs`'s `mod tests`, `src/ui/transport/history.rs`'s `mod tests`,
`src/ui/tests/recently_played_tests.rs`, `src/ui/tests/settings_tests.rs`, `docs/usage.md`,
`docs/invariants.md`, plus the `PLANS.md` move.

**Tests.**

- `src/ui/transport/history.rs`: `played_songs_newest_first_ids_lists_rows_newest_first`,
  `played_songs_restore_records_oldest_first_and_dedupes_replays`,
  `played_songs_restore_keeps_a_play_committed_mid_restore_newest`, and restore honoring
  `HISTORY_LIMIT`, beside the existing `played_songs_*` tests.
- `src/settings.rs` + `src/settings/text.rs` codec tests: the line is written newest first and
  only when non-empty, sits after the `user_playlist=` lines, survives the parse round trip in
  order, and a blank, whitespace-only, or empty-token value skips without touching other lines.
- `src/ui/tests/recently_played_tests.rs`: `resolve_recently_played_is_a_no_op_with_a_session_or_nothing_restored`,
  `resolve_recently_played_resolves_each_seeded_id_in_order` (stub service, ids newest first),
  `recently_played_resolved_stores_the_answered_rows_in_play_order`,
  `recently_played_resolved_drops_unanswerable_ids`,
  `recently_played_resolved_skips_once_a_session_is_stored`, and a boot test asserting the batch
  gains `Message::ResolveRecentlyPlayed` after `Message::ResolveCurrentTrack` (the pattern of the
  boot-batch tests that pin the current three).
- `src/ui/tests/settings_tests.rs`: the seeded launch's snapshot carries the saved ids bit for bit
  — a boot with a saved `recently_played=` line saves nothing back (snapshot parity, mirroring the
  `music_kit_user_token=`-erase regression test's guard), and after resolve stores rows the next
  snapshot writes the same list back.

**Docs.**

- `docs/usage.md`: extend `## Settings persistence`'s saved-values list with the signed-out
  Recently Played songs (the `recently_played=` line, ids newest first) — the list reopens after a
  relaunch instead of the sample stand-in; ids the library no longer answers are dropped at
  restore, as for a saved playlist's ids; with a stored session the view keeps fetching the
  service's own list and nothing is restored. The existing "always after a relaunch, the list is
  not persisted" claim is now false — remove it in the same change.
- `docs/invariants.md`: rewrite `## Signed-out Recently Played`'s guarantee for the persisted
  life, and extend it with the restore rules (the no-session and non-empty-seed guards, what
  `record` dedupes during a merge, what the seed parity protects, and that a raced sign-in drops
  the restored rows); update the **Pinned by** test list in the same change.

**Acceptance criteria.**

- `make check` passes.
- Signed out: play some songs, close, relaunch, and opening Recently Played shows those songs
  again, most recent first; the sample stand-in only appears when the saved list is empty or every
  id went unanswerable.
- The snapshot fires no save at launch with a saved list and no resolving needed; an id the
  library rejects shortens the list on the next save, like a stale-playlist id disappears.
- With a session stored at launch (saved sign-in), nothing restores and the view keeps fetching
  the service's own list; a sign-in racing the restore drops the restored rows and keeps fetch.
- `PlayHistory` (Shuffle's Previous), `current_track=` (last-played restore), the
  `user_playlist=` codec, and the signed-in list behavior are untouched.
- `docs/usage.md` and `docs/invariants.md` carry the new behavior in the same change.

## Done
