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

### Browse the library by release year from a Years list, like Winamp's Media Library (part 1/2) (planned 2026-10-29 by the plan role)

**Goal.** Winamp's Media Library can filter the audio library by Year. Nostalgia already carries
each song's release year (`Song::year: Option<u16>` in `src/library.rs`, shown by File info) and
has a Genres list built the same way (`src/ui/genres.rs`). Add a `Years` button to the Artists
view. It opens a Years list with one row per release year and the year's song count. Opening a
row shows that year's songs in the Songs view, and Back returns to Years. Part 2/2 ("Reopen the
library at the Years list across launches") makes the list persist across launches.

**Approach.** Mirror every Genres touchpoint (`grep -rn Genre src/ui` lists them).
- New file `src/ui/years.rs` (declared in `src/ui/mod.rs` beside `genres`):
  `pub(in crate::ui) struct Year { name: String, songs: Vec<Song> }` (derive `Debug, Clone,
  PartialEq`) and a pure `pub(in crate::ui) fn group_by_year(songs: Vec<Song>) -> Vec<Year>`.
  Group by `song.year`; name each group with the year's decimal text (`2001`). Order groups by
  year, oldest first. Songs whose `year` is `None` go in a group named `Unknown`, always last.
  Songs inside a group keep their input (library) order. Signed out, the sample library shows
  `2001` (3 songs), `2003`, and `2005`.
- Share the count label: in `src/ui/genres.rs`, move the `1 song`/`N songs` text into
  `pub(in crate::ui) fn songs_label(count: usize) -> String`, have `song_count_label` call it,
  and add `years::song_count_label(year: &Year) -> String` calling it too.
- `src/ui/player.rs`: add `CurrentView::Years`, `SongsOrigin::Year`, and a field
  `years: BrowseList<Year>` (started with `BrowseList::new(false)` like `genres`). In
  `library_view`, map both to `LibraryView::Artists` for now; part 2/2 replaces that.
- `src/ui/message.rs`: add `ShowYears`, `YearsLoaded(Vec<Year>)`, `YearsLoadFailed(String)`, and
  `YearSelected { epoch: u64, index: usize }`, documented like their Genres twins, and route them
  in `src/ui/mod.rs`'s `update`.
- `src/ui/handlers/browse.rs`: `show_years` (a copy of `show_genres` mapping
  `get_all_songs` through `group_by_year`, logging `"loading library years"`) and
  `year_selected` (a copy of `genre_selected` setting `SongsOrigin::Year`). Add a
  `CurrentView::Years` arm wherever `CurrentView::Genres` has one: `select_row`,
  `open_selected`, `open_row` (sends `YearSelected`), both matches in `back` (`Years →
  Artists`, `SongsOrigin::Year → Years`, `player.years.restore_library_order()`), `sort_browse`
  (by name), `reverse_browse`, `randomize_browse`, and `move_selected_row`.
- `src/ui/handlers/browse/fetch.rs`: `years_loaded` (`store_with_labels(years,
  years::song_count_label)`), `years_load_failed`, and a `YearsLoaded | YearsLoadFailed` arm in
  `browse_reply` checking `player.years.is_current`.
- Views: in `src/ui/views/browse.rs`, `view_years` (a copy of `view_genres`, count nouns
  `year`/`years`) and `view_years_button` (`labeled_button("Years", Message::ShowYears)`), and
  add `Years` to that file's list of every `CurrentView`. In
  `src/ui/views/browse/rows/text.rs`, a `"No years"` arm in `empty_list_label`. In
  `src/ui/view/library.rs`, render the view and push the button right after
  `view_genres_button()`, and add Years to the comment naming those buttons.
- Playback, jump-to-file, and overlay matches already treat everything but the named lists
  through `_ => player.songs`, so a year's songs play, step, and jump unchanged. Fix any other
  non-exhaustive match the compiler flags in the Genres style.
- Docs: in `docs/usage.md`, name the `Years` button in the Artists-list buttons sentence (after
  `Genres`, renumbering the ordinals that follow), say what its rows show (year, song count,
  oldest first, `Unknown` last), that opening a row lists its songs, and that Back returns to
  Years. In `docs/invariants.md`, add `Years → Artists` and `Songs → Years` to
  `## Browse hierarchy`, add Years to the `## UI layout` sentence on the Artists-only buttons,
  and list the new tests under Pinned by.

**Tests.**
- Unit tests in `src/ui/years.rs`: groups ordered oldest first with songs in library order;
  `None` years form a last `Unknown` group; `song_count_label` pluralizes. Keep the existing
  `genres.rs` `count_label_pluralizes` passing.
- New `src/ui/tests/years_tests.rs` (registered in `src/ui/tests/mod.rs`), following the Genres
  cases in `flat_lists_tests.rs`: `ShowYears` then `YearsLoaded` stores rows labelled like
  `3 songs`; `show_years` clears an active search; Back from Years returns to Artists;
  `YearSelected` shows exactly that year's songs in `CurrentView::Songs` and Back returns to
  Years; a stale-epoch `YearSelected` does nothing; a superseded `YearsLoaded` is dropped.
- A `sort_reorders_years_by_name` case in `sort_tests.rs`, beside `sort_reorders_genres_by_name`.

**Acceptance criteria.**
- The Artists view shows a `Years` button that lists the library's release years with song
  counts, oldest first, with `Unknown` last when any song lacks a year.
- Opening a year shows only its songs, which play, step, and jump like any Songs list. Back
  returns to Years, and Back again to Artists.
- Sort, reverse, randomize, and Alt+Up/Down work on the Years list like on Genres.
- The new tests pass, docs/usage.md and docs/invariants.md describe the view, and `make check`
  passes.

### Reopen the library at the Years list across launches (part 2/2) ("requires part 1/2 landed") (planned 2026-10-29 by the plan role)

**Goal.** Part 1/2 ("Browse the library by release year from a Years list") saves the Years list
and a year's Songs as `LibraryView::Artists`. Save and restore Years like every other top-level
list (`LibraryView` in `src/settings/library_view.rs`, saved as `library_view`).

**Approach.**
- `src/settings/library_view.rs`: add `LibraryView::Years` with key `"years"`, and add it to the
  `from_key` array and the `keys_round_trip_for_every_variant` test.
- `src/ui/player.rs` `library_view`: map `CurrentView::Years` and `SongsOrigin::Year` to
  `LibraryView::Years`.
- `src/ui/handlers/browse.rs`: add `LibraryView::Years => show_years(player)` to
  `restore_view`, and add `CurrentView::Years` to the flat-list arm of `session_ready` so sign-in
  refetches it.
- Docs: in `docs/usage.md`, add Years to the settings paragraph's list of remembered lists. In
  `docs/invariants.md` `## Settings persistence`, add the key and name the new tests.

**Tests.**
- In `src/ui/tests/settings_tests.rs`, following the Playlists cases: showing Years, and a year's
  Songs, saves `library_view = years`; booting with `years` saved opens the Years list and issues
  its fetch.
- In the test file holding the `session_ready` refetch cases: a session landing while Years
  shows refetches it.

**Acceptance criteria.**
- Quitting on the Years list, or on a year's songs, reopens the library at Years on the next
  launch. Every other saved list behaves as before, and an unknown stored word still falls back
  to Artists.
- The new tests pass, the docs name Years, and `make check` passes.

## Done
