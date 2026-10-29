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

### Match genre and composer names in the offline library search, like Winamp's quick search (planned 2026-10-29 by the plan role)

**Goal.** Winamp's Media Library quick search matches across a track's metadata fields, but with
no Apple Music session `sample_songs_matching` (`src/apple_music/sample.rs`) only checks a song's
title, artist, and album title. So typing a genre or composer that the Genres and Composers lists
show (`Rock`, `Sam Ample`) finds nothing. Also keep a sample song when `Song::genre` or
`Song::composer` contains the query, ignoring case. Results stay in library order. The signed-in
REST search path does not change.

**Approach.**
- `src/apple_music/sample.rs` `sample_songs_matching`: add `|| song.genre.to_lowercase().contains(&needle)`
  and `|| song.composer.to_lowercase().contains(&needle)` to the filter. An empty genre or
  composer can never match, because `search_songs` rejects a blank query before this runs. Change
  the doc comment to list title, artist, album title, genre, or composer.
- `src/apple_music/browse.rs`: the `AppleMusicService::search_songs` doc comment opens with
  "every song whose title, artist, or album title contains `query`". Change that list to "title,
  artist, album title, genre, or composer". No logic change.
- `docs/usage.md`: in the `Search library` sentence, the parenthetical "matching the query
  case-insensitively against each song's title, artist, and album title" becomes "… title,
  artist, album title, genre, and composer".
- `docs/invariants.md` `## Library search matching`: add genre and composer to the
  **Guarantee** field list and to the `sample_songs_matching` line under **Enforced by**. Add
  the new test under **Pinned by** and a sentence about the change under **History**.

**Tests.** In `src/apple_music/tests/browse_sample.rs`, add
`search_songs_matches_the_sample_library_by_genre_or_composer` next to
`search_songs_matches_the_sample_library_by_album_title`, using `test_service()` and `assert_ids`.
The sample data comes from `src/sample_library.rs`: song-1..3 are `Rock` and composed by
`Sam Ample`, song-4 is `Alternative`, song-5 is `Electronic`, and song-4/5 have no composer.
- `"  ROCK  "` returns `song-1`, `song-2`, `song-3` in that order.
- `"electronic"` returns exactly `song-5`.
- `"sam ample"` returns `song-1`, `song-2`, `song-3`.
- `"alternative"` returns exactly `song-4`.

Before adding them, re-check that none of these queries also matches a sample title, artist, or
album title. Change the comment above
`search_songs_matches_the_sample_library_by_title_or_artist` so it says the genre/composer match
has its own test. Its assertions stay as they are. Its `"no such track"` query, and the UI
tests' `"echo"`/`"test"` queries in `src/ui/tests/search_tests.rs`, match no sample genre or
composer, so their results do not change.

**Files touched.** `src/apple_music/sample.rs`, `src/apple_music/browse.rs` (doc only),
`src/apple_music/tests/browse_sample.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- With no session, `search_songs` keeps a sample song when the trimmed query appears, ignoring
  case, in its title, artist, album title, genre, or composer. Results stay in library order.
- The four queries above return the listed ids. Title, artist, and album-title matches, the
  empty result for a non-matching query, and the blank-query error all behave as before.
- There is no new message, state, dependency, or UI change, and the REST search branch is
  untouched.
- `docs/usage.md` and `docs/invariants.md` name genre and composer, the new test passes, and
  `make check` passes.

## Done
