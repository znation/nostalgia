# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the README's Usage section: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. Record any new drift here rather than appending
> another note.

## Planned

_None yet._

### Reload the library from a button above the browse list (planned 2026-10-23 by the plan role)

**Goal.** The artists list is fetched at boot and again only when the startup sign-in stores a
session (or when Retry re-runs it after a failure), while the other browse levels refetch only
when they are opened, so a library that changed on another device, or a stale sample-library
list, can only be replaced by restarting the app. Add a `Reload` button
beside `Top` and `Sort` that refetches the top-level artists list, clears an active search, and
returns the browse view to Artists. It reuses the existing fetch path and request generations,
adds no dependency, and leaves playback and the other browse buffers alone (the flat lists
refetch when next opened).

**Approach.**

- `src/ui/message.rs`: add a `Message::ReloadLibrary` variant beside the browse-navigation
  messages (`ScrollBrowseToTop`, `SortBrowse`), documented as refetching the top-level artists
  list and returning to Artists, clearing an active search but not the search box's text, and
  leaving playback and the other lists alone. Do not widen `Message::LoadArtists`: boot and the
  post-sign-in reload both send it and must not reset the user's current browse level.
- `src/ui/handlers/browse.rs`: add `pub(in crate::ui) fn reload_library(player: &mut
  WinampPlayer) -> Task<Message>`. It sets `player.search_active = false`, sets
  `player.current_view = CurrentView::Artists`, and returns `fetch_at_top(fetch::load_artists(player))`
  so the normal clear-before-fetch, loading flag, error clearing, request generation, artists
  store, and snap-to-top all come from the existing paths. It must not touch
  `player.songs_origin`, the other `BrowseList` buffers, or `AppState`.
- `src/ui/views/browse.rs`: add `pub fn view_reload_button() -> Element<'static, Message>` built
  with the existing `labeled_button("Reload", Message::ReloadLibrary)`, and push it after
  `view_sort_button()` in `view_browse_nav_buttons` so Top, Sort, and Reload share one row. Update
  the module and nav-row docs to name Reload and its return-to-Artists behavior.
- `src/ui/views/mod.rs`: re-export `view_reload_button` with the other browse buttons.
- `src/ui/view.rs`: update the nav-row comment from "Top and Sort" to "Top, Sort, and Reload"; the
  existing unconditional `view_browse_nav_buttons()` push makes the button available on every
  browse level without a new root branch.
- `src/ui/mod.rs`: route `Message::ReloadLibrary` to `handlers::browse::reload_library` beside the
  other browse-navigation arms.
- Tests. `src/ui/tests.rs`: add `reload_library_returns_to_artists_and_refetches_them` using
  `test_player`, seeding `CurrentView::AllSongs`, an active search with `search_query` set, an
  existing `all_songs` row, and an artists error plus rows. Drive `Message::ReloadLibrary`; assert
  the view is Artists, `search_active` is false, `search_query` is unchanged, the artists buffer
  is empty and loading with no error, the `all_songs` row and the shared state's current track /
  playing flag are untouched, then use `drive_fetch_and_assert_loaded` to assert the sample
  artists (`artist-1`, `artist-2`, `artist-3`) are stored. Add
  `reload_button_is_shown_on_every_browse_level` laying out the root view for each `BROWSE_VIEWS`
  entry and finding the `Reload` label, and add `browse_nav_buttons_place_reload_beside_sort`
  asserting Reload shares Top's and Sort's row and starts at or after Sort's right edge, the same
  shape as `browse_nav_buttons_place_sort_beside_top`. `src/ui/views/tests.rs`: build
  `view_reload_button()` in `now_playing_bar_and_browse_action_buttons_construct` and import it.
- `README.md`: in the browse-controls sentence that documents Top and Sort, add the Reload button,
  saying it refetches the library's artists and returns to the Artists list, clearing any active
  search while leaving the typed query in the box. Do not touch the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Browse hierarchy`, state that every level also offers Reload, which
  clears an active search, returns to Artists, refetches the artists list through the same fetch
  Retry uses, snaps the list to its first row, and leaves playback, the search text, and the other
  browse buffers alone; note that `Message::LoadArtists` keeps its boot, sign-in, and Retry
  behavior. Name `browse::reload_library`, `fetch::load_artists`, `fetch_at_top`,
  `views::view_reload_button`, and `Message::ReloadLibrary` as the enforcers, add the three new
  tests to **Pinned by**, and record the addition in **History**.

**Files touched.** `src/ui/message.rs`, `src/ui/mod.rs`, `src/ui/handlers/browse.rs`,
`src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/view.rs`, `src/ui/tests.rs`,
`src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Pressing `Reload` on any browse level clears an active search, switches the view to Artists,
  clears any previous artists rows and error, shows the loading placeholder, snaps the browse
  list to its first row, and stores the refetched artists when the reply lands. The typed search
  text, playback state, and the songs/albums/flat lists are unchanged.
- The Reload button renders on every browse level in the same row as Top and Sort, after Sort;
  the Artists Retry button still appears only after a failed artists fetch, and boot and the
  post-sign-in artists reload still do not change the current browse level.
- `make check` passes, and README and `docs/invariants.md` describe the action and its bounds.

## Done
