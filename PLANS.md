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

### Show the Artists list's total album count beside its artist count in the footer, like Winamp's "All (N albums)" row (planned 2026-10-29 by the plan role)

**Goal.** In Winamp's Media Library, the artist pane's top row counts the artists and the album
pane's top row counts their albums. Our Artists list puts each artist's album count on its row
(`album_count_label`: `2 albums`, `1 album`, nothing for `0`), but its footer only reads
`3 artists`. Make the Artists footer read `<count> artist(s) - <albums>`, where `<albums>` is
`album_count_label` of the `usize::saturating_add` sum of the listed artists' `Artist::album_count`.
Examples are `3 artists - 5 albums` and `1 artist - 1 album`. When that sum is `0`, the footer stays
`<count> artist(s)` with no album part, matching the row rule that shows no count for `0`. A
signed-in artist whose name matched no album (`get_favorite_artists` in
`src/apple_music/browse.rs`) has a count of `0`, so this rule avoids a misleading `0 albums`. An
empty Artists list still draws only its placeholder. The footer has no selected-row form. The
album, playlist, song, and Genres/Years/Composers footers and every row label stay as they are.
This plan is independent of the waiting album-footer plan above: it adds a different function and
edits `view_artists`, not `album_list`.

**Approach.**
- `src/ui/views/browse/rows/text.rs`: add
  `pub(in crate::ui::views) fn artist_list_summary(artists: &[Artist]) -> String` directly after
  `album_count_label`. Build the artist part with `list_count_summary(artists.len(), "artist",
  "artists")`. Sum `album_count` with `saturating_add`. When `album_count_label(total)` is empty,
  return the artist part alone; otherwise return `format!("{artists} - {albums}")`. Give it a doc
  comment in the style of `song_list_summary`'s, naming the zero rule and why it exists. Fix
  `list_count_summary`'s doc comment so it no longer claims the artist list. That list then names
  the playlist and grouped lists, plus album lists if the album-footer plan has not merged yet.
- `src/ui/views/browse/rows.rs`: make `with_footer` `pub(in crate::ui::views)`. Add
  `[`super::view_artists`]` to its doc's list of footer layouts.
- `src/ui/views/browse.rs`, `view_artists`: keep the `scrollable_list` call. Return it alone when
  `artists.is_empty()`, otherwise `with_footer(list, artist_list_summary(artists))`. Import
  `with_footer` from `rows` and `artist_list_summary` from `rows::text`. Update the doc comment's
  last sentence so it names the new footer instead of `list_with_count`. Summing per frame is a
  plain `usize` walk with no formatting per row, so no cache is needed.
- `src/ui/views/tests/mod.rs`: add `artist_list_summary` to the `rows::text` import list.
- `src/ui/views/tests/browse.rs`: add
  `artist_list_summary_counts_artists_and_totals_their_albums`, using `sample_artist()` copies
  (`src/test_support/fixtures.rs`) with `album_count` set:
  - one artist at 1 → `"1 artist - 1 album"`
  - counts 2, 0, 3 → `"3 artists - 5 albums"`
  - one at 0 → `"1 artist"`
  - two at 0 → `"2 artists"`
  - one at `usize::MAX` beside one at 1 → no panic, and the result starts with `"2 artists - "`
- `src/ui/tests/view_tests.rs`: the fixture artist has `album_count: 0`, so
  `root_view_draws_the_current_browse_levels_own_rows` (`"1 artist"`) and
  `an_empty_browse_level_draws_no_count_footer` (`"0 artists"`) should pass unchanged. Run them
  and confirm. Add one view-level case: set the root view's artist to `album_count = 2` and
  assert the captured text includes `"1 artist - 2 albums"`. Either extend the existing Artists
  case or add a small sibling test built the same way.
- `docs/usage.md`: in the browse-list paragraph, change the closing clause "and artist, album,
  and playlist lists showing their item count beneath theirs" to "the artists list showing its
  artist count and the total of its rows' album counts beneath it (`3 artists - 5 albums`, or
  just `3 artists` when no artist has a counted album), and album and playlist lists showing their
  item count beneath theirs". If the album-footer plan has merged first, keep its wording for
  album lists and change only the artist part.

**Files touched.** `src/ui/views/browse/rows/text.rs`, `src/ui/views/browse/rows.rs`,
`src/ui/views/browse.rs`, `src/ui/views/tests/mod.rs`, `src/ui/views/tests/browse.rs`,
`src/ui/tests/view_tests.rs`, `docs/usage.md`.

**Acceptance criteria.**
- A non-empty Artists list draws `<count> artist(s) - <album_count_label(sum)>`, or only
  `<count> artist(s)` when the sum is 0. The new unit test covers each case in the Approach, and
  the view-level assertion sees `1 artist - 2 albums`.
- An empty Artists list still draws only its placeholder (`an_empty_browse_level_draws_no_count_footer`
  passes).
- Artist row labels (`album_count_label_singular_plural_and_none`,
  `artist_row_uses_name_and_album_count`) and the playlist footer
  (`list_count_summary_uses_the_singular_and_plural`) pass unchanged.
- No new dependency, message, state field, cache, or setting.
- `docs/usage.md` describes the new Artists footer, and `make check` passes.

## Done
