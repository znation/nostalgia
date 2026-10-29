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

### Show the selected artist's album count before the total in the Artists footer, like the song-list footer (planned 2026-10-29 by the plan role)

**Goal.** The Artists footer (`artist_list_summary` in `src/ui/views/browse/rows/text.rs`) always
reads `3 artists - 5 albums`, even while a row is selected. The song-list and album-list footers
put the selection before the total (`2 songs - 3:30/4:15`, `3 albums - 10:30/45:30`). Make the
Artists footer read `3 artists - 2/5 albums` while an artist with a counted album is selected.
This is independent of the grouped-list selected-footer plan above: it touches
`artist_list_summary` and `view_artists`, not `song_groups.rs` or `grouped_list` (the two share
`src/ui/views/browse.rs` and the doc files, in different functions and clauses).

**Approach.**
1. `src/ui/views/browse/rows/text.rs`: change `artist_list_summary(artists)` to
   `artist_list_summary(artists: &[Artist], selected: Option<usize>) -> String`. The total is the
   same saturating sum of `album_count`. Rules, in order:
   - total `0`: `"<count> artist(s)"` (unchanged).
   - `selected.and_then(|i| artists.get(i))` names an artist whose `album_count` is non-zero:
     `"<count> artist(s) - <selected>/<total> <noun>"`, where `<noun>` is `album` when the total
     is 1 and `albums` otherwise (so `1 artist - 1/1 album`, `3 artists - 2/5 albums`).
   - otherwise (no selection, an index past the list, or a selected artist whose `album_count`
     is `0`): the plain form `"<count> artist(s) - <album_count_label(total)>"`. A `0` count can
     mean the signed-in name matched no album (see the existing doc comment), so `0/5 albums`
     would mislead, the same reason the row shows no label for `0`.
   Update the doc comment to describe the selected form and these fallbacks.
2. `src/ui/views/browse.rs` `view_artists`: pass its existing `selected` to
   `artist_list_summary(artists, selected)`, and say in its doc comment that the footer puts the
   selected artist's album count before the total.
3. Unit tests in `src/ui/views/tests/browse.rs`. Change the calls in
   `artist_list_summary_counts_artists_and_totals_their_albums` to pass `None`, keeping their
   expected strings. Then add, building artists the same way (`sample_artist()` with
   `album_count` set; lift the `with_albums` closure into a small fn helper if shared):
   - `artist_list_summary_puts_the_selected_album_count_before_the_total`: counts `[2, 0, 3]`
     with `Some(0)` gives `"3 artists - 2/5 albums"`; `[1]` with `Some(0)` gives
     `"1 artist - 1/1 album"`.
   - `artist_list_summary_keeps_the_plain_form_for_a_selected_artist_without_albums`: `[2, 0, 3]`
     with `Some(1)` gives `"3 artists - 5 albums"`; `[0, 0]` with `Some(0)` gives `"2 artists"`.
   - `artist_list_summary_ignores_a_selection_past_the_list`: `[2, 3]` with `Some(5)` gives
     `"2 artists - 5 albums"`.
4. Rendered test in `src/ui/tests/view_tests.rs`, next to
   `root_view_artists_footer_totals_the_album_counts`:
   `root_view_artists_footer_shows_the_selected_album_count`. Two `sample_artist()`s with
   `album_count` 2 and 3, `player.artists.loading = false`,
   `player.current_view = CurrentView::Artists`, then
   `let epoch = player.artists.epoch; player.artists.select_row(epoch, 1);` and assert through
   `captured_root_view_text_bounds` that `"2 artists - 3/5 albums"` is drawn.
5. `docs/usage.md`: in the browse paragraph's Artists clause (`3 artists - 5 albums`, or just
   `3 artists` …), add that while an artist with albums is selected the footer shows its album
   count before the total (`3 artists - 2/5 albums`).
6. `docs/invariants.md`: add a `## Artists footer` section right before `## Library footer` (after
   `## Album-list footer`, and after `## Grouped-list footer` if that plan has landed), in the same
   Guarantee / Enforced by / Pinned by / History form, naming `artist_list_summary`,
   `view_artists`, and the tests above (including the existing
   `artist_list_summary_counts_artists_and_totals_their_albums` and
   `root_view_artists_footer_totals_the_album_counts`).

**Files touched.** `src/ui/views/browse/rows/text.rs`, `src/ui/views/browse.rs`,
`src/ui/views/tests/browse.rs`, `src/ui/tests/view_tests.rs`, `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**
- With no selection the Artists footer is unchanged (`3 artists - 5 albums`, `3 artists` for a
  zero total), and the existing expectations pass with `None`.
- While an artist with a non-zero `album_count` is selected, the footer reads
  `<count> artist(s) - <selected>/<total> album(s)`, the noun following the total; a selected
  artist with `0` albums, a zero total, or an out-of-range index gives the plain form. The three
  new unit tests and the rendered test pin it.
- An empty Artists list still draws no footer (`view_artists` skips `with_footer` when empty).
- No new dependency, message, state field, cache, or setting.
- `docs/usage.md` and `docs/invariants.md` describe the selected form, and `make check` passes.

## Done
