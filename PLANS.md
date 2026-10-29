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

### Match release years in the offline library search, like Winamp's quick search (planned 2026-10-29 by the plan role)

**Goal.** With no Apple Music session, the `Search library` box matches a song's title, artist,
album title, genre, and composer (`sample_songs_matching` in `src/apple_music/sample.rs`), but
not its release year, though the Years list and the File info panel both show it. Winamp's
Media Library quick search matches every column, the year included. Make the offline search also
match a song whose `Song::year`, written as a plain decimal number (`2001`), contains the trimmed
query, so typing `2003` returns the songs from 2003. A song with `year: None` never matches by
year. The signed-in REST search, the blank-query rejection, and result order do not change.
This plan is independent of the grouped-row playing-time plan above: it touches none of its files.

**Approach.**
- `src/apple_music/sample.rs`, `sample_songs_matching`: add one more `||` arm to the filter,
  `song.year.is_some_and(|year| year.to_string().contains(&needle))`. Plain substring, the same
  as the other fields, so `200` matches every song from 2000–2009. Update the doc comment's field
  list to "title, artist, album title, genre, composer, or release year".
- `src/apple_music/browse.rs`, `AppleMusicService::search_songs` doc comment: add release year to
  the signed-out field list.
- `src/apple_music/tests/browse_sample.rs`: add
  `search_songs_matches_the_sample_library_by_release_year`, using the sample library's years
  (song-1..song-3 are 2001, song-4 is 2003, song-5 is 2005):
  - `"  2001  "` returns `song-1`, `song-2`, `song-3` in that order (whitespace trimmed);
  - `"2003"` returns only `song-4`; `"2005"` returns only `song-5`;
  - `"200"` returns all five songs in library order (substring match);
  - `"1999"` returns an empty list.
  Before adding the test, confirm no sample title, artist, album title, genre, or composer
  contains these digit strings (none does today), so the year arm is what makes them match.
- `docs/invariants.md`, `## Library search matching`: add release year to the Guarantee's and
  Enforced-by's field lists, and add the new test to Pinned by.
- `docs/usage.md`: in the `Search library` sentence, extend "title, artist, album title, genre,
  and composer" to "title, artist, album title, genre, composer, and release year".

**Files touched.** `src/apple_music/sample.rs`, `src/apple_music/browse.rs` (doc comment only),
`src/apple_music/tests/browse_sample.rs`, `docs/invariants.md`, `docs/usage.md`.

**Acceptance criteria.**
- Signed out, searching `2003` lists only `B-side` (song-4); searching `2001` lists the three
  First Record songs in library order; `200` lists all five sample songs in library order;
  `1999` lists none.
- Existing search tests (`..._by_title_or_artist`, `..._by_album_title`,
  `..._by_genre_or_composer`, `search_songs_rejects_a_blank_query`,
  `search_songs_uses_the_rest_library_when_signed_in`) pass unchanged.
- No new dependency, message, state, or setting; the REST path is untouched.
- `docs/usage.md` and `docs/invariants.md` name release year as a searched field, and
  `make check` passes.

## Done
