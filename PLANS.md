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

### Read the library's recently added albums through the service seam (Recently Added, part 1/2: planned 2026-10-27 by the plan role)

**Goal.** Apple Music's own app opens on a Recently Added shelf, but Nostalgia has no way to
read it: `AppleMusicService` offers artists, all albums, all songs, playlists, and search only.
This part adds the data path, `AppleMusicService::get_recently_added_albums`, over Apple's
`GET /v1/me/library/recently-added` with a sample-library fallback, so part 2/2 ("Show a Recently
Added list on the Artists view") can show it. No UI changes here.

**Approach.**
- `src/apple_music/rest/wire.rs`: give `Resource` an optional resource type,
  `#[serde(rename = "type")] kind: Option<String>`, documented as Apple's resource type
  (`library-albums`, `library-playlists`, …). Update every `let Resource { id, attributes } = self;`
  destructure in `src/apple_music/rest/wire/mapping.rs` to ignore it (`..`).
- `src/apple_music/rest.rs`: add `RestLibrary::get_recently_added_albums(&self, session)`
  requesting `{API_BASE}/me/library/recently-added` with no query string (the endpoint's page size
  is smaller than the 100-item collections, so pass no `limit`, which could be out of contract;
  `fetch` follows `next` up to `MAX_PAGES` and returns the rows read when it stops). The endpoint
  mixes resource types, so map with a closure returning `Result<Option<Album>, _>`: a resource whose
  `kind` is exactly `Some("library-albums")` maps through `Resource::into_album("")` (like
  `get_all_albums`, the collection carries no artist id); any other or missing kind (library
  playlists, catalog `albums` whose ids `get_songs_from_album`'s library-albums tracks request
  cannot open, stations) yields `None`. Flatten the `None`s out. A nameless library album still
  fails the whole query through `into_album`, as in `get_all_albums`. Keep Apple's order (newest
  first). Add an accessor on `Resource` in `wire/mapping.rs` (`is_library_album(&self) -> bool`)
  rather than making the field public.
- `src/apple_music/sample.rs`: add `sample_recently_added_albums()` returning
  `sample_all_albums()` reversed. The sample library carries no added dates, so its doc comment
  states that the reversed library order stands in for newest first.
- `src/apple_music/browse.rs`: add `AppleMusicService::get_recently_added_albums` beside
  `get_all_albums`, through `self.browse`: signed in, it reads `rest.get_recently_added_albums`
  and fills running times with `with_album_durations` (same as `get_all_albums`); signed out, it
  returns `sample_recently_added_albums`. Its doc comment follows `get_all_albums`'s, errors
  section included.

**Files touched.** `src/apple_music/rest/wire.rs`, `src/apple_music/rest/wire/mapping.rs`,
`src/apple_music/rest.rs`, `src/apple_music/sample.rs`, `src/apple_music/browse.rs`, a new test
file `src/apple_music/rest/tests/recently_added.rs` (registered in
`src/apple_music/rest/tests/mod.rs`; `library.rs` there already runs past 1000 lines), and
`src/apple_music/tests/browse_rest.rs` / `browse_sample.rs`.

**Acceptance criteria.**
- REST tests (new file, using `StubTransport`, `library_over`, `rest_session`,
  `assert_single_call`, `assert_follows_next_page`, `transport_error_over` as `library.rs` does):
  a response mixing a `library-albums`, a `library-playlists`, and a catalog `albums` resource
  maps to only the library album, with `artist_id` empty and the artist name read, and the single
  call goes to `https://api.music.apple.com/v1/me/library/recently-added`; a resource with no
  `type` is dropped; the order of two library albums is kept; a `next` page is followed; a
  transport error is reported bare; a nameless `library-albums` resource fails the query.
- Service tests: signed out, `get_recently_added_albums` returns `sample_all_albums()` reversed;
  signed in, it returns the REST albums with durations filled from the whole-library songs (mirror
  `get_all_albums_uses_the_rest_library_when_signed_in`), and a failed durations request fails it
  (mirror `get_all_albums_fails_when_the_album_durations_request_fails`).
- Every existing REST and browse test passes unchanged. No `docs/usage.md` change (nothing the
  player shows changes). `make check` passes.

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
