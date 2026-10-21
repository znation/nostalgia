# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; updated 2026-10-20).** The initial prompt
> names two commitments — a "very close if not pixel-perfect" classic Winamp UI
> and Apple Music as the library. The first is now moving: the base-skin
> palette landed ("Add a Winamp 2.x base-skin palette and apply it as the app
> theme", done 2026-10-06), and the reusable bevel layer followed ("Add a
> Winamp two-tone bevel layer and frame the Now Playing and equalizer panels",
> done 2026-10-06), framing those two panels through `bevel::lcd_well` and
> `bevel::raised_panel` in `src/ui/bevel.rs`. The transport-button chrome plan
> landed ("Style the transport buttons as raised Winamp chrome", done
> 2026-10-07), and both transport toggles are reachable: the Repeat toggle
> landed 2026-10-06 and the Shuffle toggle 2026-10-08, and their R and S
> keyboard bindings followed ("Add the R and S keyboard bindings for Repeat
> and Shuffle", done 2026-10-20). The rest of widget-level fidelity has
> landed too: slider chrome ("Style the volume and
> equalizer sliders as sunken Winamp grooves with raised chrome thumbs", done
> 2026-10-07), a custom title bar ("Add a Winamp custom title bar and drop the
> OS window frame", done 2026-10-07), and playlist chrome ("Frame the browse
> list as a sunken Winamp playlist editor", done 2026-10-07). The library side
> is real:
> `AppleMusicService` answers its browse and search queries through the REST
> client once a `MusicKit` session is stored, falling back to
> `sample_library()` otherwise — playlist queries included — and `play_track`
> plays the selected preview through the audio seam. Risk: the Winamp chrome
> arrived piecemeal, so later changes still need one consistent vocabulary.
> Recommend the plan loop keep scheduling the open equalizer and seek plans.

## Planned

### Make Previous retrace the play history when Shuffle is on (planned 2026-10-20 by the plan role)

**Goal.** With Shuffle on, pressing Previous returns to the track actually played before the current one, retracing the play order like classic Winamp, instead of stepping one row backward in the visible list. With Shuffle off, Previous keeps its sequential and Repeat-aware behavior. Today `step_track` in `src/ui/mod.rs` sends only a forward step through `transport::shuffled_track_id`; every backward step goes through `transport::previous_track_id`. The `step_track` doc comment and `previous_with_shuffle_still_steps_backward` in `src/ui/tests.rs` pin that Previous ignores Shuffle because no play history exists.

**Approach.**

- `src/ui/transport.rs`: beside `shuffled_track_id`, add `pub(super) const HISTORY_LIMIT: usize = 100` and `#[derive(Debug, Default)] pub struct PlayHistory` holding `entries: Vec<String>`, `cursor: Option<usize>`, and `pending: Option<PendingJump>`, plus a private `#[derive(Debug)] struct PendingJump { target: usize, from: Option<String> }`.
  - `pub fn record(&mut self, track_id: &str)`: when `pending` names a target entry equal to `track_id`, the history jump landed, so set `cursor` to that target and clear `pending` without pushing, truncating, or trimming — the jump neither duplicates the entry nor drops the forward entries, and a later new track truncates them from the landed cursor. Otherwise clear `pending` first; when the cursor already names `track_id` do nothing, and otherwise truncate entries after the cursor, push `track_id`, point the cursor at the new last entry, then drop the oldest entries past `HISTORY_LIMIT` while moving the cursor down with them.
  - `pub fn previous(&mut self, songs: &[Song], current: Option<&str>) -> Option<String>`:
    - A pending jump stays in effect while `current` is `None`, names the jump's target entry (its play committed but `record` has not run), or names the jump's `from` track (its play has not committed yet). When `current` names anything else, the user selected another track before the jump landed, so clear `pending` and continue from the cursor, which still names the last track actually recorded.
    - With a jump in effect, scan strictly before its target index, so repeated Previous presses before that play commits keep walking back. Without a jump, take `self.cursor?` and scan strictly before it when its entry is `current`, otherwise up to and including it (`cursor + 1`) when `current` is a newer, unrecorded selection or a failed play.
    - Find the highest index below the start whose id `songs` contains; on a hit set `pending` to that index (keeping an in-effect jump's original `from`), return its id, and leave the cursor unchanged until the play records. On a miss return `None` leaving the cursor and any in-effect jump unchanged, so `step_track` falls back to `transport::previous_track_id`. Skip entries absent from `songs` so a browse to another list selects a row the visible list can show. Do not add the rejected draft's `at_start` helper: an exhausted history uses the same sequential fallback.
- `src/ui/mod.rs`: add `play_history: transport::PlayHistory` to `WinampPlayer` and initialize it with `Default::default()` in `WinampPlayer::new`. Change `step_track` to take `&mut WinampPlayer`: when `!forward && state.shuffle`, first try `player.play_history.previous(items, state.current_track.as_deref())`, use its id when present, and otherwise fall through to `transport::previous_track_id` as today; the forward Shuffle branch and both sequential branches stay as they are. In the `Message::TrackPlayed` arm, inside the existing current-generation guard, record `state.current_track` only when `state.is_playing` is true, so a failed play never enters the history and a superseded completion is skipped by the generation check; a history jump's completion records the target `pending` already names, which `record` handles without duplicating or truncating. Update the `step_track` doc comment to describe the history-backed Previous behavior instead of "Previous ignores Shuffle".
- `src/ui/tests.rs`: replace `previous_with_shuffle_still_steps_backward` with behavior tests, and drive `Message::TrackPlayed` to exercise the recording path. Cover a shuffled Previous that retraces two recorded plays; two presses before the first jump commits (same `current`) that keep walking back; a pending jump superseded by a new committed `current` that then steps from the last actually played track; a shuffled Previous with no usable history that falls back to the sequential row; a history entry absent from the visible list being skipped; and two guard tests. `track_played_ignores_a_failed_completion` and `track_played_ignores_a_superseded_completion` each seed `["song-1", "song-2"]` and report `song-3` without recording it, then assert `previous(&songs, None)` returns `"song-2"` — with a wrongly recorded `song-3` it would return `"song-3"`, so the assertion actually pins the guard instead of passing through `previous`'s current-track branch.
- `src/ui/transport.rs` tests: cover recording order and the repeated-current no-op; recording after a back step dropping the forward entries; the `HISTORY_LIMIT` bound and cursor shift; `previous` returning `None` with no history; walking backward on repeated calls and across a pending jump; a jump target's `record` landing on its entry without duplicating or truncating; a superseded jump stepping from the last recorded play rather than skipping it; including the last recorded entry when `current` is a newer selection; skipping ids absent from the supplied `songs`; and returning `None` at the oldest entry without moving the cursor.
- `README.md`: in the Usage paragraph, say that with Shuffle on Previous retraces the tracks played before the current one. Keep the hunk to that sentence.
- `docs/invariants.md` has no transport-stepping section (its headings cover the browse hierarchy, browse reply freshness, and the Now Playing label), so this plan leaves it unchanged.
- When the feature run moves this entry to `PLANS-DONE.md`, write the done entry's approach from the shipped code — including `PendingJump` and `previous(songs, current)` — so it does not repeat the stale draft entry that described a different API.

**Files touched.** `src/ui/transport.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `PlayHistory::record` keeps chronological play order, treats a repeated current id as a no-op, drops forward entries after a back step when a new track is recorded, keeps the forward entries when a pending jump's own target records, and keeps at most `HISTORY_LIMIT` entries with the cursor following the oldest dropped entry.
- `PlayHistory::previous` returns the nearest earlier entry present in `songs`, walks further back on repeated calls including before a jump's play commits, skips entries from a list the player no longer shows, includes the last recorded entry when `current` is a newer unrecorded selection, and returns `None` without moving when no earlier usable entry exists.
- A jump whose play never lands is abandoned when `current` names a different track: from history `["song-1", "song-2", "song-3"]`, a Previous from `song-3` then a Previous from a new `song-4` selects `song-3`, not `song-1` or `song-2`.
- With Shuffle on, Previous selects the recorded previous track rather than the row before the current one; with no usable history it falls back to `previous_track_id`. With Shuffle off, Previous still follows `previous_track_id` and the shared Repeat flag.
- A current-generation `TrackPlayed` with `is_playing` true records the committed track; one with `is_playing` false, or a superseded generation, records nothing, and the two guard tests distinguish a recorded `song-3` (`previous(.., None)` would return `"song-3"`) from the unrecorded expected `"song-2"`.
- The new tests named in the approach pass alongside the existing Next/Previous wiring tests, and the README documents the Shuffle-on Previous behavior.
- The implementation diff changes only the four files listed above for the feature itself (the `PLANS.md`/`PLANS-DONE.md` move aside), and the README hunk is limited to the Shuffle-on Previous sentence.

### Show each song list's count and total playing time (planned 2026-10-21 by the plan role)

**Goal.** Every loaded song list shows, beneath its rows, how many songs it holds and their combined playing time — the classic Winamp playlist editor's status line. Today `song_list` in `src/ui/views/browse.rs` builds only the scrollable rows: each row shows its own formatted length (`song_row` via `format_track_time`), but the list never aggregates them. This is independent of the planned Shuffle-On Previous history: it reads only the `[Song]` slice the view already receives.

**Approach.**

- `src/ui/views/browse.rs`: add `pub(super) fn song_list_summary(songs: &[Song]) -> String` returning `"<count> song(s) - <total>"`, with singular `song` for a count of one and plural `songs` otherwise. Count every entry, including one whose `duration_ms` is `0`, and sum the durations with `u64::saturating_add` so a large list cannot overflow the total.
- Add a private `fn format_list_time(total_ms: u64) -> String` beside it: `0` formats as `"0:00"`, otherwise zero-padded `m:ss` with unbounded minutes (`3_665_000` formats as `"61:05"`), mirroring `views::format_track_time`'s arithmetic but not its `0` → `"--:--"` placeholder. Do not change `format_track_time`: `0` there means one track's duration is unknown, while `0` here is a valid list total when every duration is unknown.
- In `fn song_list`, keep the current `scrollable_list(...)` result for the rows; when `songs` is non-empty, return a `Column::new().push(list).push(Text::new(song_list_summary(songs)).size(14))` with `Length::Fill` width and the scrollable's current `Length::FillPortion(3)` height, so the browse panel keeps its share of the window and the summary sits left-aligned below the rows. When `songs` is empty, return the scrollable unchanged so the loading, error, and empty placeholders stay alone instead of gaining a `0 songs` footer. Compute the summary once per view refresh, not once per row, and update `song_list`'s doc comment to say it appends the count/time footer.
- No call-site changes are needed: `view_songs` (album and playlist songs and search results) and `view_all_songs` both build through `song_list`, so every loaded song list gains the footer. Artist, album, and playlist lists keep their current bodies.
- `src/ui/views/tests.rs`: add direct tests for the pure helper — `song_list_summary_counts_songs_and_sums_durations` (210_000 + 45_000 → `"2 songs - 4:15"`), `song_list_summary_uses_the_singular_for_one_song` (210_000 → `"1 song - 3:30"`), `song_list_summary_counts_unknown_durations_without_adding_time` (`[210_000, 0]` → `"2 songs - 3:30"`), and `song_list_summary_keeps_counting_minutes_past_an_hour` (3_665_000 → `"1 song - 61:05"`). The existing `browse_views_construct_over_the_loaded_library` and `browse_views_construct_over_an_empty_list` already exercise both footer branches through the view builders.
- `README.md`: add one sentence to the Usage paragraph saying each song list shows its song count and total playing time beneath the rows. Keep the hunk to that sentence.
- `docs/invariants.md` records browse navigation, reply freshness, and the Now Playing label; this display-only footer changes none of those guarantees, so leave it unchanged.

**Files touched.** `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `song_list_summary` reports the exact count and sum: `[]` → `"0 songs - 0:00"`, one 210_000 ms song → `"1 song - 3:30"`, 210_000 + 45_000 ms → `"2 songs - 4:15"`, and 3_665_000 ms → `"1 song - 61:05"`.
- A song with `duration_ms == 0` still counts toward the song total but contributes no time, so `[210_000, 0]` reads `"2 songs - 3:30"`.
- A non-empty song list built by `view_songs` or `view_all_songs` includes the footer, while an empty list shows only its existing placeholder; the construct tests cover both cases without panicking.
- The implementation diff changes only the three files listed above for the feature itself (the `PLANS.md`/`PLANS-DONE.md` move aside), and the README hunk is limited to the new summary sentence.

## Done
