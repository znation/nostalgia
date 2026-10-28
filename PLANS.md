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

### Show the current track's disc number in the File info panel, like Winamp's ID3 panel (planned 2026-10-28 by the plan role)

**Goal.** Winamp's File info dialog lists a track's position in a multi-disc set beside its track
number. Nostalgia's File info panel shows the title, artist, album, year, genre, track number,
composer, length, and loaded preview format. `Song` carries no disc number even though Apple
Music reports one as `attributes.discNumber`. This change carries that attribute through `Song`
and the `KnownTrack` cache. The panel then draws a `Disc #: <n>` line between `Track #:` and
`Composer:` while the source supplies a positive number. It adds no new state, handlers,
settings, or playback behavior, and it leaves search and sorting alone. It mirrors the finished
"Show the current track's composer in the File info panel" and "Show the current track's track
number in the File info panel" plans in PLANS-DONE.md. It does not depend on the Recently Played
series. Both edit `src/apple_music/rest/wire/mapping.rs`, but this plan touches only
`Resource::into_song` (plus a new private helper) there, and Recently Played 1/2 adds
`Resource::is_song` beside `Resource::is_library_album`.

**Approach.**

- `src/library.rs`: add `#[serde(default)] pub disc_number: Option<u32>` to `Song`, after
  `track_number`. Its doc comment follows `Self::track_number`'s: the track's 1-based disc in its
  album's set; `None` when the source supplied none or reported a non-positive number; the File
  info panel draws a `Disc #:` line only while it is `Some`. In the test module:
  - add the `"disc_number": null` key to `song_payload`;
  - add `song_round_trips_its_disc_number` and
    `song_disc_number_defaults_to_none_when_the_payload_omits_it` (built from
    `song_required_fields_payload`) beside `song_round_trips_its_track_number`.
- `src/apple_music/rest/wire.rs`: add `#[serde(rename = "discNumber")] disc_number:
  Option<serde_json::Number>` to the private `Attributes`, after `track_number`. Its doc comment
  matches `track_number`'s (parsed as a JSON number so an out-of-`u32` value maps to `None`
  instead of failing the whole response).
- `src/apple_music/rest/wire/mapping.rs`: move the `track_number` chain in `Resource::into_song`
  (`as_u64`, `> 0`, `u32::try_from`) into a private
  `fn positive_position(number: Option<serde_json::Number>) -> Option<u32>` beside
  `primary_genre`, carrying the existing comment as its doc comment. Map both
  `track_number` and `disc_number` through it. Carry `disc_number` through the attributes
  destructure (add one more `None` to the fallback tuple). Name the disc number in
  `into_song`'s doc comment among the fields it falls back for.
- `src/sample_library.rs`: give Opening, Middle, and Ending (First Record) `disc_number:
  Some(1)`, and give B-side and Headliner `None`. Signed out, the panel then shows both the line
  and its omission. Add `songs_carry_the_documented_disc_numbers` beside
  `songs_carry_the_documented_composers`, pinning every song's id and disc number in library
  order.
- `src/ui/views/now_playing/text.rs`: add `pub disc_number: Option<u32>` to `KnownTrack`, after
  `track_number`. Its doc comment says it is copied from `Song::disc_number`, and that `None`
  means the panel leaves the line out. In `src/ui/handlers/playback.rs`, the `KnownTrack` literal
  that records `track_number: song.track_number` also records `disc_number: song.disc_number`.
- `src/ui/views/file_info.rs`: add `const DISC_NUMBER_LABEL: &str = "Disc #: ";` between
  `TRACK_NUMBER_LABEL` and `COMPOSER_LABEL`. In `view_file_info_overlay`, push a
  `Disc #: <n>` line after the conditional track-number line and before the conditional composer
  line, only while `track.disc_number` is `Some`. Name the disc number in the module and
  `view_file_info_overlay` doc comments.
- Tests:
  - In `src/apple_music/rest/tests/library.rs`, use the existing `song_from_attributes` helper.
    `songs_from_album_maps_the_disc_number` checks that `"discNumber":2` maps to `Some(2)`.
    `songs_from_album_maps_a_missing_zero_negative_or_oversized_disc_number_to_none` checks that
    an absent, `null`, `0`, `-1`, and `4294967296` disc number each map to `None`. The existing
    track-number tests stay and pin that the helper extraction left that mapping unchanged.
  - In `src/ui/tests/readouts_tests.rs`, add `known_tracks_record_the_played_songs_disc_number`,
    modelled on `known_tracks_record_the_played_songs_track_number`.
  - In `src/ui/views/tests/chrome.rs`, add `file_info_overlay_draws_the_tracks_disc_number` (with
    a track number, disc number, and composer all set, the `Disc #: 2` line sits after
    `Track #:` and before `Composer:`) and
    `file_info_overlay_omits_the_disc_number_line_when_the_source_supplied_none`, modelled on the
    two composer tests there.
  - Add `disc_number: None` to `sample_known_track` in `src/ui/views/tests/chrome.rs`, and to every
    other `Song` and `KnownTrack` literal the compiler reports (the composer commit touched
    `src/test_support/fixtures.rs`, `src/apple_music/tests/mod.rs`,
    `src/apple_music/rest/tests/library.rs`, `src/ui/views/tests/chrome.rs`, and several
    `src/ui/tests/*.rs` files).
- `docs/usage.md`: in the Alt+3 File info sentence, add the disc number after the track number in
  the list of lines, and add it to the parenthetical of lines left out when the source supplies
  none.
- `docs/invariants.md` `## File info panel`: add `Disc #: …` (left out when
  `KnownTrack::disc_number` is `None`) after `Track #: …` and before `Composer: …`. Add a
  sentence that it is copied from `Song::disc_number`, which `Resource::into_song` takes from
  `attributes.discNumber` with the same absent/`null`/non-positive/above-`u32::MAX` → `None`
  rule as the track number. Name the new tests in its `Pinned by`, and leave every other
  guarantee unchanged.

**Files touched.** `src/library.rs`, `src/apple_music/rest/wire.rs`,
`src/apple_music/rest/wire/mapping.rs`, `src/apple_music/rest/tests/library.rs`,
`src/sample_library.rs`, `src/ui/views/now_playing/text.rs`, `src/ui/handlers/playback.rs`,
`src/ui/views/file_info.rs`, `src/ui/views/tests/chrome.rs`,
`src/ui/tests/readouts_tests.rs`, plus the one-line `disc_number` field in the other test-built
`Song`/`KnownTrack` literals, `docs/usage.md`, and `docs/invariants.md`.

**Acceptance criteria.**

- A REST song resource whose `attributes.discNumber` is `2` maps to `Song::disc_number ==
  Some(2)`. An absent, `null`, zero, negative, or above-`u32::MAX` disc number maps to `None`,
  and a resource without the attribute still parses.
- The track-number mapping behaves as before (its existing tests pass unchanged).
- A serialized `Song` round-trips its disc number, and a payload without the `disc_number` key
  parses with `None`.
- The sample library's First Record songs carry `Some(1)`, and B-side and Headliner carry `None`.
- Playing a song records its disc number in the `KnownTrack` the File info panel reads.
- With `Some(n)`, the File info panel draws `Disc #: n` after the `Track #:` line (or after
  whichever earlier line is present) and before `Composer:` (or `Length:` when there is no
  composer). With `None` it draws no `Disc #:` line, and every other line is unchanged.
- `docs/usage.md` and `docs/invariants.md` name the new line as above.
- `make check` passes.

## Done
