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


### Show each album row's release year beside its running time, like Winamp's Media Library (planned 2026-10-29 by the plan role)

**Goal.** Winamp's Media Library album view has a Year column. Nostalgia's album rows (per-artist
Albums, All Albums, Recently Added) show `artist - title` and, at the right, only the running
time (`10:30`). Show the album's release year before the running time, `2001  10:30` (two
spaces between them). An album with no known year still shows only `10:30` (or `--:--`).

**Approach.**
- `src/library.rs`: add `pub year: Option<u16>` to `Album`, marked `#[serde(default)]` and with a
  doc comment matching `Song::year`'s (`None` means the source supplied no usable release date).
  Add `year: None,` to every `Album { .. }` struct literal (`grep -rn 'Album {' src` lists them,
  in about 13 files, most of them tests). The exceptions are the sample albums below and
  `into_album`.
- `src/apple_music/rest/wire/mapping.rs` `Resource::into_album`: read `attributes.release_date`
  next to `name`/`artist_name` and set `year: release_date.as_deref().and_then(release_year)`,
  which reuses the existing `release_year` helper that `into_song` uses. `Attributes::release_date`
  already exists in `src/apple_music/rest/wire.rs`, so the wire struct does not change.
- `src/sample_library.rs`: give the sample albums the years their songs carry: `First Record`
  `Some(2001)`, `Second Record` `Some(2003)`, `Debut` `Some(2005)`.
- `src/ui/views/mod.rs`: next to `pub fn format_track_time`, add
  `pub fn album_row_label(album: &Album) -> String`. It returns
  `format!("{year}  {}", format_track_time(album.duration_ms))` when `album.year` is `Some`, else
  `format_track_time(album.duration_ms)`. Make it the fallback label in `album_row`
  (`src/ui/views/browse/rows/text.rs`). Then `store_albums` in `src/ui/handlers/browse/fetch.rs` passes
  `views::album_row_label` instead of `views::format_track_time(album.duration_ms)`, so the cached
  label and the fallback match.
- Sorting, Jump to File and selection read `display_title`, so they do not change.

**Files touched.** `src/library.rs`, `src/apple_music/rest/wire/mapping.rs`,
`src/sample_library.rs`, `src/ui/views/mod.rs`, `src/ui/views/browse/rows/text.rs`,
`src/ui/handlers/browse/fetch.rs`, the struct-literal sites, `src/ui/views/tests/browse.rs`,
the REST mapping tests (`src/apple_music/rest/tests/library.rs`), `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**
- New test in `src/ui/views/tests/browse.rs`: `album_row` for an album with `year: Some(2001)` and
  `duration_ms` 630000 returns the label `2001  10:30`. The existing
  `album_row_uses_title_and_duration` (year `None`) still returns just the time. A `--:--` album
  with a year reads `2001  --:--`.
- New REST test in `src/apple_music/rest/tests/library.rs`: a library-albums response whose album
  carries `"releaseDate": "2003-05-01"` maps to `year: Some(2003)`, and one with no `releaseDate`
  maps to `None`.
- A UI test (in an existing album-list test file such as `src/ui/tests/browse_tests.rs`) loads
  the signed-out sample All Albums list and checks that the stored row labels read
  `2001  10:30`, `2003  1:35` and `2005  3:20`, which confirms the `store_albums` cache uses the
  new label.
- `docs/usage.md`: the album-row sentence says the row shows the release year (when known)
  before the running time, e.g. `2001  10:30`. `docs/invariants.md` "Browse album running times"
  gets the year rule, names `album_row_label` under Enforced by, and lists the new tests under
  Pinned by.
- `make check` passes.
- Independent of "Share one song-grouping module between the Genres and Years lists": that plan
  touches `src/ui/genres.rs`, `src/ui/years.rs` and the grouped-list views, not album rows or the
  `Album` model.

### Browse the library by composer from a Composers list, like Winamp's Media Library (part 1/2) (planned 2026-10-29 by the plan role)

**Goal.** Winamp's Media Library can filter the audio library by Composer. Every `Song` already
carries `composer: String` (`src/library.rs`, shown by File info's `Composer:` line, empty when the
source supplied none). Add a `Composers` button to the Artists view. It opens a Composers list
with one row per composer and its song count. Opening a row shows that composer's songs in the
Songs view, and Back returns to Composers. Part 2/2 ("Reopen the library at the Composers list
across launches") makes the list persist across launches.

**Approach.** Mirror every Years touchpoint (`grep -rn "Year" src/ui` lists them; the finished
Years plan in PLANS-DONE.md names each one).
- New file `src/ui/composers.rs` (declared in `src/ui/mod.rs` beside `years`), shaped like
  `src/ui/genres.rs`: `pub(in crate::ui) type Composer = SongGroup;` and
  `pub(in crate::ui) fn group_by_composer(songs: Vec<Song>) -> Vec<Composer>` calling
  `song_groups::group_songs` with the genre key's rule applied to `song.composer`: trim it, a
  blank composer is `None` (the `Unknown` group, last), otherwise
  `Some((trimmed.to_lowercase(), trimmed.to_string()))`. Signed out, the sample library
  (`src/sample_library.rs`, pinned by `songs_carry_the_documented_composers`) shows
  `Sam Ample` (3 songs) then `Unknown` (2 songs). Row labels use the existing
  `song_groups::song_count_label`.
- `src/ui/player.rs`: add `CurrentView::Composers`, `SongsOrigin::Composer`, and a field
  `composers: BrowseList<Composer>` (started like `years`). In `library_view`, map both to
  `LibraryView::Artists` for now; part 2/2 replaces that.
- `src/ui/message.rs`: add `ShowComposers`, `ComposersLoaded(Vec<Composer>)`,
  `ComposersLoadFailed(String)`, and `ComposerSelected { epoch: u64, index: usize }`, documented
  like their Years twins, and route them in `src/ui/mod.rs`'s `update`.
- `src/ui/handlers/browse.rs`: `show_composers` (a copy of `show_years` mapping `get_all_songs`
  through `group_by_composer`, logging `"loading library composers"`) and `composer_selected`
  (a copy of `year_selected` setting `SongsOrigin::Composer`). Add a `CurrentView::Composers`
  arm wherever `CurrentView::Years` has one (`select_row`, `open_selected`, `open_row`, both
  matches in `back` — `Composers → Artists`, `SongsOrigin::Composer → Composers` with
  `player.composers.restore_library_order()` — `sort_browse` by name, `reverse_browse`,
  `randomize_browse`, `move_selected_row`).
- `src/ui/handlers/browse/fetch.rs`: `composers_loaded` (`store_with_labels(composers,
  song_count_label)` as `years_loaded` does), `composers_load_failed`, and a
  `ComposersLoaded | ComposersLoadFailed` arm in `browse_reply` checking
  `player.composers.is_current`.
- Views: in `src/ui/views/browse.rs`, `view_composers` (a copy of `view_years`, count nouns
  `composer`/`composers`) and `view_composers_button` (`labeled_button("Composers",
  Message::ShowComposers)`), and add `Composers` to that file's list of every `CurrentView`. In
  `src/ui/views/browse/rows/text.rs`, a `"No composers"` arm in `empty_list_label`. In
  `src/ui/view/library.rs`, render the view beside the `CurrentView::Years` arm and push
  `view_composers_button()` right after `view_years_button()` in the Artists-only button
  column; update that block's comment, which names the buttons (it currently omits Genres and
  Years too — list all eight).
- Fix any other non-exhaustive match the compiler flags (for example the `CurrentView` arrays in
  `src/ui/tests/browse_tests.rs`) in the Years style.
- Docs: in `docs/usage.md`, name `Composers` in the Artists-list buttons sentence after `Years`
  (renumbering the ordinals that follow), say what its rows show (composer, song count, by name
  ignoring case, `Unknown` last for songs without one), that opening a row lists its songs, and
  that Back returns to Composers; add Composers wherever the doc lists the flat lists Back
  returns to. In `docs/invariants.md`, add `Composers → Artists` and `Songs → Composers` to
  `## Browse hierarchy`, add Composers to the `## UI layout` sentence on the Artists-only
  buttons, and list the new tests under Pinned by.

**Tests.**
- Unit tests in `src/ui/composers.rs`: names compare case-insensitively and trim (`Jane Doe`
  and ` jane doe ` share one group named `Jane Doe`); blank composers form a last `Unknown`
  group; groups order by lowercase name, not first appearance.
- New `src/ui/tests/composers_tests.rs` (registered in `src/ui/tests/mod.rs`), mirroring
  `years_tests.rs` case for case: `ComposersLoaded` stores rows labelled like `3 songs`;
  `show_composers` clears an active search; Back from Composers returns to Artists;
  `ComposerSelected` shows exactly that composer's songs in `CurrentView::Songs` and Back returns
  to Composers; a stale-epoch `ComposerSelected` does nothing; a superseded `ComposersLoaded` is
  dropped.
- `sort_reorders_composers_by_name` in `src/ui/tests/sort_tests.rs`, beside
  `sort_reorders_years_by_name`, and a Composers case beside the Years one in
  `src/ui/tests/loading_tests.rs` if that file enumerates every loaded/failed message pair.

**Acceptance criteria.**
- The Artists view shows a `Composers` button that lists the library's composers with song
  counts, ordered by name ignoring case, with `Unknown` last when any song has no composer.
- Opening a composer shows only its songs, which play, step, and jump like any Songs list. Back
  returns to Composers, and Back again to Artists.
- Sort, reverse, randomize, and Alt+Up/Down work on the Composers list like on Years.
- The new tests pass, docs/usage.md and docs/invariants.md describe the view, and `make check`
  passes.
- Independent of "Show each album row's release year beside its running time": that plan
  touches album rows and the `Album` model, not the grouped lists.

### Reopen the library at the Composers list across launches (part 2/2) ("requires part 1/2 landed") (planned 2026-10-29 by the plan role)

**Goal.** Part 1/2 ("Browse the library by composer from a Composers list") saves the Composers
list and a composer's Songs as `LibraryView::Artists`. Save and restore Composers like every
other top-level list (`LibraryView` in `src/settings/library_view.rs`, saved as `library_view`).

**Approach.**
- `src/settings/library_view.rs`: add `LibraryView::Composers` with key `"composers"`, and add
  it to the `from_key` array and the `keys_round_trip_for_every_variant` test.
- `src/ui/player.rs` `library_view`: map `CurrentView::Composers` and `SongsOrigin::Composer` to
  `LibraryView::Composers`.
- `src/ui/handlers/browse.rs`: add `LibraryView::Composers => show_composers(player)` to
  `restore_view`, and add `CurrentView::Composers` to the flat-list arm of `session_ready` so
  sign-in refetches it.
- Docs: in `docs/usage.md`, add Composers to the settings paragraph's list of remembered lists.
  In `docs/invariants.md` `## Settings persistence`, add the key (beside `years`) and name the
  new tests.

**Tests.**
- In `src/ui/tests/settings_tests.rs`, following the Years cases: showing Composers, and a
  composer's Songs, saves `library_view = composers`; booting with `composers` saved opens the
  Composers list and issues its fetch.
- In the test file holding the `session_ready` refetch cases: a session landing while Composers
  shows refetches it.

**Acceptance criteria.**
- Quitting on the Composers list, or on a composer's songs, reopens the library at Composers on
  the next launch. Every other saved list behaves as before, and an unknown stored word still
  falls back to Artists.
- The new tests pass, the docs name Composers, and `make check` passes.

## Done
