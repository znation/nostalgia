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
