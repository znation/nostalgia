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

### Match genre and composer names in the offline library search, like Winamp's quick search (planned 2026-10-29 by the plan role)

**Goal.** Winamp's Media Library quick search matches across a track's metadata fields, but with
no Apple Music session `sample_songs_matching` (`src/apple_music/sample.rs`) only checks a song's
title, artist, and album title. So typing a genre or composer that the Genres and Composers lists
show (`Rock`, `Sam Ample`) finds nothing. Also keep a sample song when `Song::genre` or
`Song::composer` contains the query, ignoring case. Results stay in library order. The signed-in
REST search path does not change.

**Approach.**
- `src/apple_music/sample.rs` `sample_songs_matching`: add `|| song.genre.to_lowercase().contains(&needle)`
  and `|| song.composer.to_lowercase().contains(&needle)` to the filter. An empty genre or
  composer can never match, because `search_songs` rejects a blank query before this runs. Change
  the doc comment to list title, artist, album title, genre, or composer.
- `src/apple_music/browse.rs`: the `AppleMusicService::search_songs` doc comment opens with
  "every song whose title, artist, or album title contains `query`". Change that list to "title,
  artist, album title, genre, or composer". No logic change.
- `docs/usage.md`: in the `Search library` sentence, the parenthetical "matching the query
  case-insensitively against each song's title, artist, and album title" becomes "… title,
  artist, album title, genre, and composer".
- `docs/invariants.md` `## Library search matching`: add genre and composer to the
  **Guarantee** field list and to the `sample_songs_matching` line under **Enforced by**. Add
  the new test under **Pinned by** and a sentence about the change under **History**.

**Tests.** In `src/apple_music/tests/browse_sample.rs`, add
`search_songs_matches_the_sample_library_by_genre_or_composer` next to
`search_songs_matches_the_sample_library_by_album_title`, using `test_service()` and `assert_ids`.
The sample data comes from `src/sample_library.rs`: song-1..3 are `Rock` and composed by
`Sam Ample`, song-4 is `Alternative`, song-5 is `Electronic`, and song-4/5 have no composer.
- `"  ROCK  "` returns `song-1`, `song-2`, `song-3` in that order.
- `"electronic"` returns exactly `song-5`.
- `"sam ample"` returns `song-1`, `song-2`, `song-3`.
- `"alternative"` returns exactly `song-4`.

Before adding them, re-check that none of these queries also matches a sample title, artist, or
album title. Change the comment above
`search_songs_matches_the_sample_library_by_title_or_artist` so it says the genre/composer match
has its own test. Its assertions stay as they are. Its `"no such track"` query, and the UI
tests' `"echo"`/`"test"` queries in `src/ui/tests/search_tests.rs`, match no sample genre or
composer, so their results do not change.

**Files touched.** `src/apple_music/sample.rs`, `src/apple_music/browse.rs` (doc only),
`src/apple_music/tests/browse_sample.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- With no session, `search_songs` keeps a sample song when the trimmed query appears, ignoring
  case, in its title, artist, album title, genre, or composer. Results stay in library order.
- The four queries above return the listed ids. Title, artist, and album-title matches, the
  empty result for a non-matching query, and the blank-query error all behave as before.
- There is no new message, state, dependency, or UI change, and the REST search branch is
  untouched.
- `docs/usage.md` and `docs/invariants.md` name genre and composer, the new test passes, and
  `make check` passes.

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
