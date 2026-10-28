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

### Show a Recently Added list on the Artists view (Recently Added, part 2/2: planned 2026-10-27 by the plan role, requires part 1/2 landed)

**Goal.** Add a `Recently Added` button beside `All Songs`, `All Albums`, and `Playlists` on the
Artists view. It opens a flat album list read from `AppleMusicService::get_recently_added_albums`
(part 1/2, "Read the library's recently added albums through the service seam"), newest first;
opening an album shows its songs, and Back from those songs returns to Recently Added, then to
Artists. It behaves exactly like All Albums in every other way.

**Approach.** Mirror All Albums at each touchpoint; grep `AllAlbums` and `all_albums` under
`src/ui` (excluding tests) and add a sibling everywhere:
- `src/ui/player.rs`: `CurrentView::RecentlyAdded`, `SongsOrigin::RecentlyAdded` (Back returns
  to `CurrentView::RecentlyAdded`), and a `recently_added: BrowseList<Album>` field initialised
  `BrowseList::new(false)` like `all_albums`.
- `src/ui/message.rs`: `ShowRecentlyAdded`, `RecentlyAddedLoaded(Vec<Album>)`,
  `RecentlyAddedLoadFailed(String)`, typed like the `AllAlbums*` variants; route them in
  `src/ui/mod.rs`.
- `src/ui/handlers/browse.rs`: `show_recently_added` beside `show_all_albums` (fetch label
  `"loading recently added albums"`); the album-open path (the `from_all_albums` branch) treats
  `CurrentView::RecentlyAdded` the same way, selecting from `recently_added` and setting
  `SongsOrigin::RecentlyAdded`; add arms in the Back match, `restore_library_order`, row
  selection, the selected-index lookup, sort (by title, as for albums), reverse, shuffle, and
  Alt+Up/Down move matches.
- `src/ui/handlers/browse/fetch.rs`: `recently_added_loaded` / `recently_added_load_failed`
  through `store_albums` / `fail_list`, and the freshness arm with `recently_added.is_current`.
- `src/ui/views/browse.rs`: `view_recently_added` (as `view_all_albums`, over
  `CurrentView::RecentlyAdded`) and `view_recently_added_button` (`labeled_button("Recently Added",
  Message::ShowRecentlyAdded)`); add the view to `BROWSE_VIEWS` (length 7). Export them from
  `src/ui/views/mod.rs`. `src/ui/views/browse/rows/text.rs`: the empty text is
  `"No recently added albums"`.
- `src/ui/view/library.rs`: render the list for `CurrentView::RecentlyAdded`, and push the button
  after `view_playlists_button` in the Artists-only block (update its comment).
- Any match the compiler flags for the new variants gets the All Albums behaviour.

**Files touched.** The files above, plus tests in `src/ui/tests/flat_lists_tests.rs` (or a new
`src/ui/tests/recently_added_tests.rs` if that file is near 1000 lines), `docs/usage.md`, and
`docs/invariants.md`.

**Acceptance criteria.**
- With the sample library: `ShowRecentlyAdded` from Artists switches to
  `CurrentView::RecentlyAdded`, and the loaded list's titles equal `sample_all_albums()`'s
  reversed; opening its first row shows that album's songs with
  `songs_origin == SongsOrigin::RecentlyAdded`; Back goes to Recently Added, then to Artists.
- A stale `RecentlyAddedLoaded` (an older epoch) is ignored and a `RecentlyAddedLoadFailed`
  shows its report, as the All Albums tests check for their list.
- Ctrl+R reverses and Ctrl+Shift+1 sorts the Recently Added list, and Back from its songs restores
  the newest-first order.
- The button renders only on the Artists view (extend the test that checks the All Songs / All
  Albums / Playlists buttons). Every per-view test that walks `BROWSE_VIEWS` passes with the new
  entry.
- `docs/usage.md`: the sentence listing the Artists list's `All Songs`, `All Albums`, and
  `Playlists` buttons names `Recently Added` too, says it lists the library's recently added
  albums newest first (the sample library's albums in reverse order when signed out), and that
  Back from its songs returns to it. `docs/invariants.md` `## Browse hierarchy` gains
  "Songs → Recently Added when the songs were opened from the Recently Added list" and
  "Recently Added → Artists", and `## UI layout`'s button line names it. `make check` passes.

## Done
