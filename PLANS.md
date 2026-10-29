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

### Show the Genres, Years, and Composers lists' total playing time beside their count in the footer, like the album-list footer (planned 2026-10-29 by the plan role)

**Goal.** Each grouped row already reads like the song-list footer (`group_row_label`:
`2 songs - 5:05`). The album lists' footer reads `3 albums - 45:30` (`album_list_summary`). But
the Genres, Years, and Composers footers still use the bare item count from `list_with_count`
(`2 genres`). Make each grouped footer read `<count> <noun> - <total>`: `2 genres - 7:10`,
`1 year - 3:30`, `2 composers - 7:10`. `<total>` is the `u64::saturating_add` sum of
`duration_ms` over every song in every listed group, formatted by `format_minutes_seconds`. It
gets a trailing `+` when any of those songs has `duration_ms == 0`, the same unknown-length rule
as `group_row_label` and `album_list_summary`. Every song is in exactly one group (`group_songs`
in `src/ui/song_groups.rs` puts keyless songs in the last `Unknown` group), so the total is the
playing time of the whole grouped library. An empty grouped list still draws only its
placeholder. The footer has no selected-row form. The row labels and the Artists, playlist,
album, and song footers stay as they are.

This plan is independent of the waiting Artists-footer plan above. Both make `with_footer` in
`src/ui/views/browse/rows.rs` `pub(in crate::ui::views)`. Whichever plan merges second finds
that change already made and keeps it, adding its own view to `with_footer`'s doc list.

**Approach.**
- `src/ui/song_groups.rs`: move `group_row_label`'s total-and-mark code into a private
  `fn songs_total<'a>(songs: impl IntoIterator<Item = &'a Song>) -> String`. It returns
  `format_minutes_seconds(sum)` followed by `+` when any song has `duration_ms == 0`; for no songs
  it returns `0:00`. `group_row_label` keeps its output and calls `songs_total(&group.songs)`.
  Add `pub(in crate::ui) fn group_list_summary(groups: &[SongGroup], singular: &str, plural: &str) -> String`
  directly after `group_row_label`, returning `format!("{count} {noun} - {total}")`. `count` is
  `groups.len()`, `noun` is `singular` exactly when `count == 1`, and `total` is
  `songs_total(groups.iter().flat_map(|group| &group.songs))`. Doc comment in the style of
  `group_row_label`'s. Update the module doc's first line so it names the footer too.
- `src/ui/views/browse/rows.rs`: make `with_footer` `pub(in crate::ui::views)` if it is not
  already. Name the Genres, Years, and Composers lists in its doc's list of footer layouts. In
  `list_with_count`'s doc, say it now serves only the artist and playlist lists (or only the
  playlist list if the Artists-footer plan has merged).
- `src/ui/views/browse.rs`, `grouped_list`: change the first parameter from
  `names: impl ExactSizeIterator<Item = &'a str>` to `groups: &'a [SongGroup]`, building each row
  from `group.name.as_str()`. Return the `scrollable_list` alone when `groups.is_empty()`;
  otherwise return `with_footer(list, group_list_summary(groups, singular, plural))`. `view_genres`,
  `view_years`, and `view_composers` pass their slices (`Genre`, `Year`, and `Composer` are aliases
  of `SongGroup`). Import `SongGroup` and `group_list_summary` from `crate::ui::song_groups` and
  `with_footer` from `rows`. Rewrite `grouped_list`'s doc and the "item-count footer" sentence in
  `view_genres`'s doc to name the count-and-time footer. Summing per frame is a plain `u64` walk
  with one format call, so no cache is needed.
- `src/ui/song_groups.rs` tests: add
  `list_summary_counts_groups_and_totals_every_song`, using `by_genre` and `song`:
  - three songs in two groups gives `2 genres - 10:30`;
  - one group gives the singular `1 genre - 3:30`;
  - a group holding a `duration_ms: 0` song gives a trailing `+`;
  - `u64::MAX` plus another duration does not panic and starts with `2 genres - `.
  The existing `row_label_*` tests must pass unchanged.
- Rendered tests, using `captured_root_view_text_bounds` (`src/ui/tests/mod.rs`) and asserting
  the captured texts contain the footer:
  - `src/ui/tests/flat_lists_tests.rs`: `genres_view_footer_counts_genres_and_totals_their_songs`
    with `player_with_genres()`, expecting `2 genres - 7:10`;
  - `src/ui/tests/years_tests.rs`: the `player_with_years()` twin, expecting `2 years - 7:10`;
  - `src/ui/tests/composers_tests.rs`: the `player_with_composers()` twin, expecting
    `2 composers - 7:10`.
  Each fixture's row labels are `2 songs - 5:05` and `1 song - 2:05`, which add up to 7:10.
  Confirm each helper leaves `current_view` on its list, and set it if not.
- `docs/usage.md`: in the sentence listing the footers ("album lists showing their album count
  and total playing time beneath theirs …"), add that the Genres, Years, and Composers lists show
  their count and total playing time beneath theirs (`2 genres - 7:10`, with the same trailing
  `+`), and keep artist and playlist lists on their item count.

**Files touched.** `src/ui/song_groups.rs`, `src/ui/views/browse.rs`,
`src/ui/views/browse/rows.rs`, `src/ui/tests/flat_lists_tests.rs`, `src/ui/tests/years_tests.rs`,
`src/ui/tests/composers_tests.rs`, `docs/usage.md`.

**Acceptance criteria.**
- A loaded Genres, Years, or Composers list draws `<count> <noun> - <total>[+]` beneath its rows.
  The three rendered tests above pin it.
- `group_list_summary` pluralizes, sums with `saturating_add`, and marks unknown lengths with `+`.
  `list_summary_counts_groups_and_totals_every_song` pins all three.
- An empty grouped list draws no footer, only its placeholder.
- The `row_label_*` tests, `genres_loaded_stores_rows_with_song_count_labels` and its Years and
  Composers twins, `list_count_summary_uses_the_singular_and_plural`, and
  `an_empty_browse_level_draws_no_count_footer` pass unchanged.
- No new dependency, message, state field, cache, or setting.
- `docs/usage.md` describes the new grouped footer, and `make check` passes.

## Done
