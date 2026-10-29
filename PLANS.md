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

### Show the selected group's playing time before the total in the Genres, Years, and Composers footers, like the song-list footer (planned 2026-10-29 by the plan role)

**Goal.** The grouped lists' footer (`group_list_summary` in `src/ui/song_groups.rs`) always reads
`2 genres - 7:10`, even while a row is selected (`BrowseList::select_row`, painted by
`scrollable_list`). The song-list footer shows the selected song's length before the total
(`2 songs - 3:30/4:15`, `song_list_summary`). Make the grouped footer read
`2 genres - 3:30/10:30` while a group is selected, in the Genres, Years, and Composers views (all
three build through `grouped_list` in `src/ui/views/browse.rs`). This is independent of the
album-list selected-footer plan above: it touches `song_groups.rs` and `browse.rs`, not
`rows/text.rs` or `rows.rs` (the two only share the doc files).

**Approach.**
1. `src/ui/song_groups.rs`: change `group_list_summary(groups, singular, plural)` to
   `group_list_summary(groups: &[SongGroup], selected: Option<usize>, singular: &str, plural: &str)
   -> String`. The total stays `songs_total` of every song in every group. When
   `selected.and_then(|i| groups.get(i))` names a group, the output is
   `"<count> <noun> - <selected>/<total>"`, where `<selected>` is `songs_total(&group.songs)` (so
   it carries its own trailing `+` when one of that group's songs has `duration_ms == 0`). An index
   past the list falls back to the plain form. Update the doc comment to describe the selected form.
2. `src/ui/views/browse.rs` `grouped_list`: pass its existing `selected` to
   `group_list_summary(groups, selected, singular, plural)`, and say in its doc comment that the
   footer includes the selected group's playing time.
3. Tests in `song_groups.rs`'s `mod tests`. Change the calls in
   `list_summary_counts_groups_and_totals_every_song` to pass `None`, keeping their expected
   strings. Then add (using the same `timed`/`by_genre` helpers):
   - `list_summary_puts_the_selected_group_time_before_the_total`: songs `("1","a",180_000)`,
     `("2","a",120_000)`, `("3","b",330_000)` with `Some(1)` gives `"2 genres - 5:30/10:30"`.
   - `list_summary_marks_an_unknown_selected_group_time`: `("1","a",210_000)`, `("2","b",0)` with
     `Some(1)` gives `"2 genres - 0:00+/3:30+"`.
   - `list_summary_ignores_a_selection_past_the_list`: `Some(5)` on one group gives the plain
     form `"1 genre - 3:30"`.
4. Rendered test in `src/ui/tests/view_tests.rs`, next to
   `root_view_artists_footer_totals_the_album_counts`:
   `root_view_genres_footer_shows_the_selected_group_time`. Set `player.genres.items` to two
   `SongGroup`s (built with `sample_song()` and known `duration_ms`, e.g. 210_000 and 120_000),
   `player.genres.loading = false`, `player.current_view = CurrentView::Genres`, then
   `player.genres.select_row(player.genres.epoch, 0)`, and assert through
   `captured_root_view_text_bounds` that `"2 genres - 3:30/5:30"` is drawn. (`grouped_list`
   tolerates missing `labels`, so the test need not fill them.)
5. `docs/usage.md`: in the browse paragraph's grouped-lists clause (`2 genres - 7:10`), add that
   while a row is selected the footer shows that group's playing time before the total
   (`2 genres - 3:30/7:10`), with the same trailing `+` rule.
6. `docs/invariants.md`: add a `## Grouped-list footer` section right before `## Library footer`
   (after `## Song-list footer`, and after `## Album-list footer` if that plan has landed), in the
   same Guarantee / Enforced by / Pinned by / History form, naming `group_list_summary`,
   `grouped_list`, and the tests above.

**Files touched.** `src/ui/song_groups.rs`, `src/ui/views/browse.rs`,
`src/ui/tests/view_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- With no selection, a loaded Genres, Years, or Composers footer is unchanged
  (`2 genres - 7:10[+]`), and the existing `group_list_summary` expectations pass with `None`.
- While a group row is selected, the footer reads `<count> <noun> - <selected>/<total>`, an
  unknown-length song in the selected group marks `<selected>` with `+`, and an out-of-range
  index gives the plain form; the three new unit tests and the rendered test pin it.
- An empty grouped list still draws no footer (`grouped_list` returns early when `groups` is empty).
- No new dependency, message, state field, cache, or setting.
- `docs/usage.md` and `docs/invariants.md` describe the selected form, and `make check` passes.

## Done
