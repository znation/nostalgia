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

### Show the selected album's running time before the total in the album-list footer, like the song-list footer (planned 2026-10-29 by the plan role)

**Goal.** While a song row is selected, the song-list footer shows that song's length before the
total (`2 songs - 3:30/4:15`, `song_list_summary` in `src/ui/views/browse/rows/text.rs`), like
Winamp's playlist status line. Album rows can be selected too (`BrowseList::select_row`, painted
by `scrollable_list`), but the album-list footer (`album_list_summary`) ignores the selection and
always reads `3 albums - 45:30`. Make it read `3 albums - 10:30/45:30` while an album is selected,
in the Albums, All Albums, and Recently Added views (all three build through `rows::album_list`).

**Approach.**
1. `src/ui/views/browse/rows/text.rs`: change `album_list_summary(albums: &[Album])` to
   `album_list_summary(albums: &[Album], selected: Option<usize>) -> String`, following
   `song_list_summary`'s shape. The total and its `+` rule stay as they are. When
   `selected.and_then(|i| albums.get(i))` names an album, the output is
   `"<count> <noun> - <selected>/<total>"`. `<selected>` is that album's `duration_ms` from
   `format_minutes_seconds`, with a trailing `+` when the album's `duration_ms == 0`. An index past
   the list falls back to the plain form. Update the doc comment: drop "Selecting a row does not
   change it" and describe the selected form.
2. `src/ui/views/browse/rows.rs` `album_list`: pass its existing `selected` argument, so the
   footer becomes `album_list_summary(albums, selected)`. Update its doc comment to say the footer
   includes the selected album's running time.
3. Tests in `src/ui/views/tests/browse.rs`. Change the calls in
   `album_list_summary_counts_albums_and_sums_their_running_time` to pass `None`, keeping their
   expected strings. Then add:
   - `album_list_summary_puts_the_selected_running_time_before_the_total`:
     `[summary_album(630_000), summary_album(210_000)]` with `Some(1)` gives
     `"2 albums - 3:30/14:00"`.
   - `album_list_summary_marks_an_unknown_selected_running_time`:
     `[summary_album(0), summary_album(210_000)]` with `Some(0)` gives `"2 albums - 0:00+/3:30+"`.
   - `album_list_summary_ignores_a_selection_past_the_list`: `Some(5)` on one album gives the
     plain form.
4. Rendered test in `src/ui/tests/view_tests.rs` (next to
   `root_view_artists_footer_totals_the_album_counts`):
   `root_view_album_footer_shows_the_selected_running_time`. Set `player.all_albums.items` to two
   albums with known `duration_ms` and `loading = false`, and set
   `current_view = CurrentView::AllAlbums`. Select row 1 with
   `player.all_albums.select_row(player.all_albums.epoch, 1)`, then assert through
   `captured_root_view_text_bounds` that the expected `"2 albums - <a>/<total>"` text is drawn.
5. `docs/usage.md`: in the album-lists clause of the browse paragraph (the one with
   `3 albums - 45:30`), add that while an album row is selected, the footer shows that album's
   running time before the total (`3 albums - 10:30/45:30`), with the same trailing `+` rule.
6. `docs/invariants.md`: add a `## Album-list footer` section right after `## Song-list footer`,
   in the same Guarantee / Enforced by / Pinned by / History form. It names `album_list_summary`,
   `album_list`, and the tests above.

**Files touched.** `src/ui/views/browse/rows/text.rs`, `src/ui/views/browse/rows.rs`,
`src/ui/views/tests/browse.rs`, `src/ui/tests/view_tests.rs`, `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**
- With no selection, a loaded album list's footer is unchanged (`3 albums - 45:30[+]`), and the
  existing `album_list_summary` expectations pass with `None`.
- While an album row is selected, the footer reads `<count> album(s) - <selected>/<total>`, and
  an unknown-length selection is marked `+`. The three new unit tests and the rendered test pin it.
- An empty album list still draws no footer: `an_empty_browse_level_draws_no_count_footer` passes
  unchanged.
- No new dependency, message, state field, cache, or setting.
- `docs/usage.md` and `docs/invariants.md` describe the selected form, and `make check` passes.

## Done
