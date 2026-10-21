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
