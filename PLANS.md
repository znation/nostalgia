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

### Match release years in the offline library search, like Winamp's quick search (planned 2026-10-29 by the plan role)

**Goal.** With no Apple Music session, the `Search library` box matches a song's title, artist,
album title, genre, and composer (`sample_songs_matching` in `src/apple_music/sample.rs`), but
not its release year, though the Years list and the File info panel both show it. Winamp's
Media Library quick search matches every column, the year included. Make the offline search also
match a song whose `Song::year`, written as a plain decimal number (`2001`), contains the trimmed
query, so typing `2003` returns the songs from 2003. A song with `year: None` never matches by
year. The signed-in REST search, the blank-query rejection, and result order do not change.
This plan is independent of the grouped-row playing-time plan above: it touches none of its files.

**Approach.**
- `src/apple_music/sample.rs`, `sample_songs_matching`: add one more `||` arm to the filter,
  `song.year.is_some_and(|year| year.to_string().contains(&needle))`. Plain substring, the same
  as the other fields, so `200` matches every song from 2000–2009. Update the doc comment's field
  list to "title, artist, album title, genre, composer, or release year".
- `src/apple_music/browse.rs`, `AppleMusicService::search_songs` doc comment: add release year to
  the signed-out field list.
- `src/apple_music/tests/browse_sample.rs`: add
  `search_songs_matches_the_sample_library_by_release_year`, using the sample library's years
  (song-1..song-3 are 2001, song-4 is 2003, song-5 is 2005):
  - `"  2001  "` returns `song-1`, `song-2`, `song-3` in that order (whitespace trimmed);
  - `"2003"` returns only `song-4`; `"2005"` returns only `song-5`;
  - `"200"` returns all five songs in library order (substring match);
  - `"1999"` returns an empty list.
  Before adding the test, confirm no sample title, artist, album title, genre, or composer
  contains these digit strings (none does today), so the year arm is what makes them match.
- `docs/invariants.md`, `## Library search matching`: add release year to the Guarantee's and
  Enforced-by's field lists, and add the new test to Pinned by.
- `docs/usage.md`: in the `Search library` sentence, extend "title, artist, album title, genre,
  and composer" to "title, artist, album title, genre, composer, and release year".

**Files touched.** `src/apple_music/sample.rs`, `src/apple_music/browse.rs` (doc comment only),
`src/apple_music/tests/browse_sample.rs`, `docs/invariants.md`, `docs/usage.md`.

**Acceptance criteria.**
- Signed out, searching `2003` lists only `B-side` (song-4); searching `2001` lists the three
  First Record songs in library order; `200` lists all five sample songs in library order;
  `1999` lists none.
- Existing search tests (`..._by_title_or_artist`, `..._by_album_title`,
  `..._by_genre_or_composer`, `search_songs_rejects_a_blank_query`,
  `search_songs_uses_the_rest_library_when_signed_in`) pass unchanged.
- No new dependency, message, state, or setting; the REST path is untouched.
- `docs/usage.md` and `docs/invariants.md` name release year as a searched field, and
  `make check` passes.

## Done
