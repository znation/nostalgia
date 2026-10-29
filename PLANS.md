# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the usage reference: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. The README's detailed Usage reference moved to
> `docs/usage.md` on 2026-10-26, so plans that change usage prose edit that file, not the README.
> Record any new drift here rather than appending another note.

## Planned

_None yet._

### Show each Genres, Years, and Composers row's total playing time beside its song count, like the song-list footer (planned 2026-10-29 by the plan role)

**Goal.** A Genres, Years, or Composers row's secondary label reads only `1 song` / `N songs`
(`song_count_label` in `src/ui/song_groups.rs`). Album rows already show their running time, and
opening a group shows a song-list footer of `N songs - M:SS`. Winamp's Media Library shows the
same per-row totals. Make each grouped row's label read the way that footer does:
`<count> song(s) - <total>`, for example `2 songs - 5:05`. As in the footer, a total that
includes a song of unknown length (`duration_ms == 0`) gets a trailing `+`, for example
`2 songs - 3:30+` or `1 song - 0:00+`. Grouping, row order, and row names stay the same.

**Approach.**
- `src/ui/song_groups.rs`: rename `song_count_label` to `group_row_label` and rewrite its doc
  comment. Sum `group.songs`' `duration_ms` with `u64::saturating_add`, format the sum with
  `crate::ui::views::format_minutes_seconds` (already `pub(in crate::ui)` in
  `src/ui/views/mod.rs`), and append `+` when any song's `duration_ms` is `0`. The result is
  `format!("{count} {noun} - {total}")`, where `noun` is `song` for one song and `songs` for
  any other count. The arithmetic and wording follow `song_list_summary` in
  `src/ui/views/browse/rows/text.rs`, using its unselected form. Do not change that function's
  visibility or call it: it is `pub(in crate::ui::views)`, and the grouped label takes only a few
  lines.
- `src/ui/handlers/browse/fetch.rs`: update the `use` and the three `store_with_labels(…,
  song_count_label)` calls (genres, years, composers) to `group_row_label`. Then change the doc
  comments that say "song-count label" to "song-count and playing-time label".
- `src/ui/views/browse.rs`: in the doc comments on `view_genres`, `view_years`, and
  `view_composers`, change "song-count label" to "song-count and playing-time label".
  This is a doc-only change.
- `docs/usage.md`: the `Genres`, `Years`, and `Composers` sentences each say "each with its
  song count" (the Composers one wraps as "song\ncount"). Change each to "each with its song
  count and total playing time".

**Tests.**
- `src/ui/song_groups.rs` unit tests: rename `count_label_pluralizes` to
  `row_label_pluralizes_and_totals_the_playing_time`. Its fixture `song` builds on
  `sample_song()`, which is 210 000 ms, so the assertions become `"1 song - 3:30"` and
  `"2 songs - 7:00"`. Add `row_label_marks_a_total_with_an_unknown_length_with_a_plus`:
  a group of one 0 ms song reads `"1 song - 0:00+"`, and a group of one 0 ms song plus one
  210 000 ms song reads `"2 songs - 3:30+"`.
- Keep the names of the UI tests `genres_loaded_stores_rows_with_song_count_labels`
  (`src/ui/tests/flat_lists_tests.rs`), `years_loaded_stores_rows_with_song_count_labels`
  (`src/ui/tests/years_tests.rs`), and `composers_loaded_stores_rows_with_song_count_labels`
  (`src/ui/tests/composers_tests.rs`), because `docs/invariants.md` `## Browse hierarchy`
  pins them by name. Update their label assertions from `["2 songs", "1 song"]` to
  `["2 songs - 5:05", "1 song - 2:05"]`. Each fixture builds on `stepping_songs()`
  (`src/test_support/fixtures.rs`: 210 000, 125 000, 95 000 ms), with songs 0 and 2 in the
  named group (305 000 ms) and song 1 in `Unknown` (125 000 ms).
- Before finishing, grep `src` for `song_count_label` and for `"2 songs"` / `"1 song"`. Make
  sure no other test asserts the old grouped label.

**Files touched.** `src/ui/song_groups.rs`, `src/ui/handlers/browse/fetch.rs`,
`src/ui/views/browse.rs` (doc only), `src/ui/tests/flat_lists_tests.rs`,
`src/ui/tests/years_tests.rs`, `src/ui/tests/composers_tests.rs`, `docs/usage.md`.
`docs/invariants.md` needs no edit, because no pinned test is renamed and no guarantee there
names the label text.

**Acceptance criteria.**
- Each Genres, Years, and Composers row's secondary label reads `<count> song(s) - <m:ss>`,
  with `song` for exactly one song. The total is the saturating sum of the group's
  `duration_ms`, formatted by `format_minutes_seconds`, plus a trailing `+` when any song in the
  group has a `duration_ms` of `0`.
- Row names, group order, the `Unknown` group, opening a group, and the song-list footer all
  behave as before. There is no new message, state, setting, or dependency.
- `docs/usage.md` says each of the three lists shows its song count and total playing time.
- The renamed and new unit tests and the three updated UI tests pass, and `make check` passes.

## Done
