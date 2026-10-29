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

### Browse the library by genre from a Genres list, like Winamp's Media Library (part 1/2) (planned 2026-10-29 by the plan role)

**Goal.** Winamp's Media Library lets you filter the audio library by Genre. Nostalgia shows each
song's primary genre in File info and uses it for AUTO (`Song::genre`, filled by
`primary_genre` in `src/apple_music/rest/wire/mapping.rs`), but you can't browse by it. Add a
`Genres` button to the Artists view. It opens a Genres list with one row per genre and the
genre's song count. Opening a row shows that genre's songs in the Songs view, and Back returns to
Genres. Part 2/2 ("Reopen the library at the Genres list across launches") makes the list
persist across launches.

**Approach.**
- New file `src/ui/genres.rs` (declared in `src/ui/mod.rs`): `pub(in crate::ui) struct Genre
  { name: String, songs: Vec<Song> }` (derive `Debug, Clone, PartialEq`) and a pure
  `pub(in crate::ui) fn group_by_genre(songs: Vec<Song>) -> Vec<Genre>`. Group by
  `song.genre.trim()`, comparing names case-insensitively, and name the group after the first
  spelling seen. Order groups by lowercase name. Songs with a blank genre go in a group named
  `Unknown`, always last; the sample library sets no genres, so it shows that one row. Songs
  inside a group keep their input (library) order.
- `src/ui/player.rs`: add `CurrentView::Genres`, `SongsOrigin::Genre`, and a field
  `genres: BrowseList<Genre>` (start it with `BrowseList::new(false)` in the constructor, like
  `playlists`). In `library_view`, map both `CurrentView::Genres` and `SongsOrigin::Genre` to
  `LibraryView::Artists` for now; part 2/2 replaces that.
- `src/ui/message.rs`: add `ShowGenres`, `GenresLoaded(Vec<Genre>)`, `GenresLoadFailed(String)`,
  and `GenreSelected { epoch: u64, index: usize }`, documented like their Playlists twins. Route
  them in `src/ui/mod.rs`'s `update`.
- `src/ui/handlers/browse.rs`:
  - `show_genres`: a copy of `show_playlists` that calls
    `fetch_level(&mut player.genres, …, "loading library genres", |service| async move {
    service.get_all_songs().await.map(group_by_genre) }, Message::GenresLoaded,
    Message::GenresLoadFailed)`.
  - `genre_selected`: resolve the row with `player.genres.select(epoch, index)` and clone its
    songs. Call `player.songs.begin_fetch()` so an in-flight album or playlist `SongsLoaded`
    reply goes stale. Store the songs with `fetch::store_songs` (widen it to `pub(super)`), set
    `songs_origin = SongsOrigin::Genre`, `switch_view(player, CurrentView::Songs)`, and return
    `scroll::scroll_browse_to_top()`. No fetch is needed.
  - Add a `CurrentView::Genres` arm to `select_row`, `open_selected`, `open_row` (sends
    `GenreSelected`), both `match`es in `back` (`Genres → Artists`, `SongsOrigin::Genre →
    Genres`, and `player.genres.restore_library_order()`), `sort_browse` (by name),
    `reverse_browse`, `randomize_browse`, and `move_selected_row`.
- `src/ui/handlers/browse/fetch.rs`: add `genres_loaded` (`player.genres.store_with_labels(genres,
  count label)`; the label is `1 song` or `N songs`) and `genres_load_failed`. Add a
  `GenresLoaded | GenresLoadFailed` arm to `browse_reply` checking `player.genres.is_current`.
- Views: in `src/ui/views/browse.rs`, add `view_genres` (a copy of `view_playlists` that uses
  the cached labels, as the album lists do, with count nouns `genre`/`genres`) and
  `view_genres_button` (`labeled_button("Genres", Message::ShowGenres)`). Add `Genres` to that
  file's list of every `CurrentView`. In `src/ui/views/browse/rows/text.rs`, add a
  `"No genres"` arm to `empty_list_label`. In `src/ui/view/library.rs`, render the view and push
  the button after `view_playlists_button()` on the Artists view.
- The playback, jump-to-file, and overlay matches (`handlers/playback.rs`, `handlers/jump.rs`,
  `view/overlay.rs`) already treat `CurrentView::Songs` as `player.songs` and everything else
  through `_`, so a genre's songs play, step, and jump with no change there. Fix any other
  non-exhaustive match the compiler flags in the same style.
- Docs: in `docs/usage.md`, name the `Genres` button in the Artists-list buttons sentence. Say
  what its rows show (name, song count, `Unknown` last), that opening a row lists its songs, and
  that Back returns to Genres. In `docs/invariants.md`, add `Genres → Artists` and
  `Songs → Genres` to `## Browse hierarchy`, add Genres to the `## UI layout` sentence on the
  Artists-only buttons, and list the new tests under Pinned by.

**Tests.**
- Unit tests in `src/ui/genres.rs`:
  - grouping and ordering;
  - case-insensitive merging that keeps the first spelling;
  - blank and whitespace-only genres go to a last `Unknown` group;
  - songs keep their library order inside a group.
- In `src/ui/tests/flat_lists_tests.rs`, following the Playlists tests:
  - `ShowGenres` then `GenresLoaded` fills `player.genres` with labels like `2 songs`;
  - `GenreSelected` shows exactly that genre's songs in `CurrentView::Songs`;
  - Back returns to Genres and then to Artists;
  - a stale-epoch `GenreSelected` does nothing;
  - a `TrackSelected` on a genre's song plays it;
  - a superseded `GenresLoaded` is dropped.
- Add a `sort_reorders_genres_by_name` case in `sort_tests.rs`.

**Acceptance criteria.**
- The Artists view shows a `Genres` button that lists the library's genres with song counts,
  sorted by name, with `Unknown` last when any song lacks a genre.
- Opening a genre shows only its songs, which play, step with Next/Previous, and jump like any
  Songs list. Back returns to Genres, and Back again returns to Artists.
- Sort, reverse, randomize, and Alt+Up/Down work on the Genres list like on Playlists.
- The new tests pass, docs/usage.md and docs/invariants.md describe the view, and `make check`
  passes.

### Reopen the library at the Genres list across launches (part 2/2) ("requires part 1/2 landed") (planned 2026-10-29 by the plan role)

**Goal.** The library reopens at the last top-level list (`LibraryView` in
`src/settings/library_view.rs`, saved as `library_view`), but part 1/2 ("Browse the library by
genre from a Genres list") saves the Genres list and a genre's Songs as `Artists`. Save and
restore Genres like every other top-level list.

**Approach.**
- `src/settings/library_view.rs`: add `LibraryView::Genres` with key `"genres"`, and add it to
  the `from_key` array and the `keys_round_trip_for_every_variant` test.
- `src/ui/player.rs` `library_view`: map `CurrentView::Genres` and `SongsOrigin::Genre` to
  `LibraryView::Genres`.
- `src/ui/handlers/browse.rs`: add `LibraryView::Genres => show_genres(player)` to
  `restore_view`, and add `CurrentView::Genres` to the flat-list arm of `session_ready` so
  sign-in refetches it.
- Docs: in `docs/usage.md`, add Genres to the settings paragraph's list of remembered lists.
  In `docs/invariants.md` `## Settings persistence`, add the key and name the new tests.

**Tests.**
- In `src/ui/tests/settings_tests.rs`, following the Playlists cases:
  - showing Genres, and a genre's Songs, saves `library_view = genres`;
  - booting with `genres` saved opens the Genres list and issues its fetch.
- In `loading_tests.rs` or the matching `session_ready` test file: a session landing while
  Genres shows refetches it.

**Acceptance criteria.**
- Quitting on the Genres list, or on a genre's songs, reopens the library at Genres on the next
  launch. Every other saved list behaves as before.
- An unknown stored word still falls back to Artists.
- The new tests pass, the docs name Genres, and `make check` passes.

## Done
