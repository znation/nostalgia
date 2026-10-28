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

### Read the user's recently played songs through the service seam (Recently Played, part 1/2: planned 2026-10-27 by the plan role)

**Goal.** Winamp's playlist is the songs you have been listening to, and Apple Music keeps that
history server-side, but Nostalgia cannot read it. This part adds the data path,
`AppleMusicService::get_recently_played_songs`, over Apple's
`GET /v1/me/recent/played/tracks` with a sample-library fallback. A later part 2/2 (not yet
planned; it would show the list as a flat song view beside `All Songs`) depends on it. This part
changes nothing the player shows, so `docs/usage.md` is untouched. It touches none of the UI
files that "Show a Recently Added list on the Artists view" edits, so the two can run side by side.

**Approach.**
- `src/apple_music/rest/wire/mapping.rs`: beside `Resource::is_library_album`, add
  `Resource::is_song(&self) -> bool`, true when `kind` is `Some("songs")` or
  `Some("library-songs")`. Its doc comment says these are the two types
  `RestLibrary::get_recently_played_songs` maps. A catalog `songs` resource carries the same
  attributes `Resource::into_song` reads (`name`, `artistName`, `albumName`, `durationInMillis`,
  `previews`, …), so no mapping change is needed.
- `src/apple_music/rest.rs`: after `get_recently_added_albums`, add
  `RestLibrary::get_recently_played_songs(&self, session) -> Result<Vec<Song>, AppleMusicError>`.
  It requests `{API_BASE}/me/recent/played/tracks` with no query string, the same way
  `get_recently_added_albums` sends no `limit`, and follows `next` through `self.fetch`. It maps
  each resource where `is_song()` holds with `into_song("")`, the empty album id that
  `get_all_songs` also gives, and drops every other resource, such as `music-videos` or an
  untyped one. As in `get_recently_added_albums`, use `Ok(None)` and then `flatten`. Keep
  Apple's order, which is most recent first. The doc comment follows `get_recently_added_albums`.
- `src/apple_music/sample.rs`: add `pub(super) fn sample_recently_played_songs() -> Vec<Song>`
  returning `sample_all_songs()` reversed. Its doc comment says the sample library keeps no play
  history, so this stands in, just as `sample_recently_added_albums` does.
- `src/apple_music/browse.rs`: after `get_recently_added_albums`, add
  `pub async fn get_recently_played_songs(&self) -> Result<Vec<Song>, AppleMusicError>` through
  `self.browse(|rest, session| rest.get_recently_played_songs(session),
  sample_recently_played_songs)`. Import the new sample function where the other `sample_*`
  imports sit. Its doc comment follows `get_all_songs`'s, including the `# Errors` section.

**Tests.**
- New file `src/apple_music/rest/tests/recently_played.rs`, registered in
  `src/apple_music/rest/tests/mod.rs` next to `mod recently_added;` and modelled on
  `recently_added.rs` (`library_over`, `StubTransport`, `assert_single_call`, `assert_ids`).
  Its `URL` constant is `https://api.music.apple.com/v1/me/recent/played/tracks`.
  - `get_recently_played_songs_keeps_catalog_and_library_songs`: a body with one `songs`, one
    `library-songs`, and one `music-videos` resource maps the two songs in order, with full
    `Song` equality on the catalog one (title, artist, album, and its `previews[0].url` as
    `preview_url`, using `PREVIEW_URL` from `test_support`, and an empty `album_id`). It also
    asserts a single call to `URL`.
  - `get_recently_played_songs_drops_a_resource_without_a_type`: an untyped resource gives an
    empty list.
  - `get_recently_played_songs_keeps_apples_order_and_follows_a_next_page`: two pages, the first
    ending in `"next":"/v1/me/recent/played/tracks?offset=10"`. Assert the ids in order and that
    the second call's URL is the absolute next URL, the same way the recently-added paging test
    asserts it.
- `src/apple_music/tests/browse_rest.rs`:
  `get_recently_played_songs_uses_the_rest_library_when_signed_in` uses `signed_in_service` and
  `assert_call_urls` to show one call to the tracks URL and the mapped song.
- `src/apple_music/tests/browse_sample.rs`:
  `get_recently_played_songs_returns_the_sample_songs_reversed` compares against
  `get_all_songs()` reversed, the same way the Recently Added sample test does.

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/wire/mapping.rs`,
`src/apple_music/sample.rs`, `src/apple_music/browse.rs`, new
`src/apple_music/rest/tests/recently_played.rs`, `src/apple_music/rest/tests/mod.rs`,
`src/apple_music/tests/browse_rest.rs`, `src/apple_music/tests/browse_sample.rs`.

**Acceptance criteria.**
- When signed in, `AppleMusicService::get_recently_played_songs` issues
  `GET https://api.music.apple.com/v1/me/recent/played/tracks` (plus any `next` pages). It returns
  the `songs` and `library-songs` resources as `Song`s in Apple's order and drops every other
  resource type and untyped resources.
- When signed out, it returns the sample library's songs in reverse library order, and it never
  fails.
- No UI, `docs/usage.md`, or `docs/invariants.md` change. The tests above pass under
  `make check`.

## Done
