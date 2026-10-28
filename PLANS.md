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

### Recently Played, part 1/2: read the user's recently played songs through the service seam (planned 2026-10-27 by the plan role)

**Goal.** Winamp's playlist is the songs you have been listening to, and Apple Music keeps that
history server-side, but Nostalgia cannot read it. This part adds the data path,
`AppleMusicService::get_recently_played_songs`, over Apple's
`GET /v1/me/recent/played/tracks` with a sample-library fallback. A later part 2/2 ("Recently Played, part 2/2: show a Recently Played list on the Artists view", which
shows the list as a flat song view beside `All Songs`) depends on it. This part
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

### Recently Played, part 2/2: show a Recently Played list on the Artists view (planned 2026-10-27 by the plan role; "requires part 1/2 landed")

**Goal.** Add a `Recently Played` button to the Artists view, after `Recently Added`. It opens a
flat song list read from `AppleMusicService::get_recently_played_songs` (part 1/2, "Recently Played,
part 1/2: read the user's recently played songs through the service seam"), most recent first. When signed out it
shows the sample library's songs in reverse order. It behaves exactly like All Songs:
- its rows play when pressed;
- Next/Previous, the preview-end advance, Home/End jumps and Jump to File all use it while it
  is showing;
- Back returns to Artists.

**Approach.** Mirror All Songs at each touchpoint. Grep `AllSongs` and `all_songs` under
`src/ui` (excluding tests) and add a sibling for each hit:
- `src/ui/player.rs`: add `CurrentView::RecentlyPlayed` and a `recently_played:
  BrowseList<Song>` field, initialised with `BrowseList::new(false)` the way `all_songs` is.
  Add no `SongsOrigin` variant, because the list is flat and its rows play.
- `src/ui/message.rs`: add `ShowRecentlyPlayed`, `RecentlyPlayedLoaded(Vec<Song>)` and
  `RecentlyPlayedLoadFailed(String)`, with the same types as the `AllSongs*` variants. Route
  them in `src/ui/mod.rs`.
- `src/ui/handlers/browse.rs`:
  - Add `show_recently_played` beside `show_all_songs`. It calls `service.get_recently_played_songs()`
    with the fetch label `"loading recently played songs"`.
  - Wherever these matches have a `CurrentView::AllSongs` arm, give `RecentlyPlayed` the same
    behaviour over `recently_played`: row selection, the selected-index lookup, `open_row` (a
    `TrackSelected`), the Back match (to `CurrentView::Artists`), `restore_library_order` (no-op
    with the song lists), `sort_browse` (by `display_title`), reverse, shuffle, and the
    Alt+Up/Down move.
- `src/ui/handlers/browse/fetch.rs`: add `recently_played_loaded` and
  `recently_played_load_failed`, going through `store_songs` and `fail_list`. Add the freshness
  arm using `recently_played.is_current`.
- The song-list resolvers currently pick `all_songs` when `current_view == CurrentView::AllSongs`
  and `songs` otherwise. Turn each into a match that also maps `CurrentView::RecentlyPlayed` to
  `recently_played`. The resolvers are:
  - `showing_songs` and the inline list choice in `step_track` in
    `src/ui/handlers/playback/stepping.rs`. Keep `step_track` inline, because its doc says
    `play_history` is borrowed there.
  - `track_selected`, and the `shuffle_round.record` list in the preview-end commit handler, in
    `src/ui/handlers/playback.rs`.
  - `song_list` in `src/ui/handlers/jump.rs`.
  - The Jump to File overlay match in `src/ui/view/overlay.rs`.
  - `scroll_browse_to_playing_row` in `src/ui/handlers/browse/scroll.rs`.
  
  Update the comments that name "the flat All Songs view" so they cover both flat song lists.
- `src/ui/views/browse.rs`:
  - Add `view_recently_played`, the same as `view_all_songs` but over
    `CurrentView::RecentlyPlayed`.
  - Add `view_recently_played_button`, which is
    `labeled_button("Recently Played", Message::ShowRecentlyPlayed)`.
  - Add `CurrentView::RecentlyPlayed` to `BROWSE_VIEWS` after `RecentlyAdded`, making it length 8.
  
  Export both new functions from `src/ui/views/mod.rs`. In `src/ui/views/browse/rows/text.rs`
  the empty-list text is `"No recently played songs"`.
- `src/ui/view/library.rs`: render the list for `CurrentView::RecentlyPlayed` the way the
  `AllSongs` arm does. Push `view_recently_played_button()` after `view_recently_added_button()`
  in the Artists-only block, and name it in that block's comment. The buttons stack vertically
  in the library column, so a fifth button needs no layout change.
- Any other match the compiler flags for the new variant gets the All Songs behaviour.

**Tests.** Add a new file `src/ui/tests/recently_played_tests.rs`, registered in
`src/ui/tests/mod.rs` and modelled on `recently_added_tests.rs` and the All Songs tests in
`flat_lists_tests.rs` and `stepping_tests.rs`. Extend the existing button test in
`src/ui/tests/view_tests.rs` (`flat_list_buttons`), and the per-view lists in
`browse_tests.rs`, `loading_tests.rs` and `sort_tests.rs` where the Recently Added change
extended them.

**Files touched.** All the files named above, plus `docs/usage.md` and `docs/invariants.md`.

**Acceptance criteria.**
- With the sample library:
  - `ShowRecentlyPlayed` from Artists switches to `CurrentView::RecentlyPlayed`.
  - The loaded list's titles equal `sample_all_songs()` reversed.
  - `TrackSelected` with that list's epoch and index 0 plays the first song of the reversed
    list, which is the library's last song.
  - Back goes to Artists.
- While Recently Played is showing:
  - `NextTrack` and the preview-end advance step through `recently_played`, not `songs`. The
    test is modelled on `next_track_in_all_songs_steps_the_flat_list` and
    `preview_end_in_all_songs_steps_the_flat_list`.
  - Jump to File submits against its titles, modelled on
    `submit_uses_the_all_songs_list_when_it_is_showing`.
- A stale `RecentlyPlayedLoaded` with an older epoch is ignored. A `RecentlyPlayedLoadFailed`
  shows its report.
- Ctrl+R reverses the list and Ctrl+Shift+1 sorts it by title.
- The `Recently Played` button renders only on the Artists view. Every per-view test that walks
  `BROWSE_VIEWS` passes with the new entry.
- `docs/usage.md`: the sentence that lists the Artists list's flat-list buttons also names
  `Recently Played`. It says that:
  - the list shows the songs you played most recently first (the sample library's songs in
    reverse order when signed out);
  - its rows play like any other song row;
  - Next/Previous step through it while it is showing.
- `docs/invariants.md`:
  - `## Browse hierarchy` gains "Recently Played → Artists".
  - `## UI layout`'s flat-list button line names Recently Played.
- `make check` passes.

## Done
