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

### Match album titles in the library search (planned 2026-10-23 by the plan role)

**Goal.** The built-in sample library's search currently matches a song only when the query
appears in its title or artist, so typing an album title that the browse lists show (for example
`Second Record`) returns nothing. Extend `sample_songs_matching` to also match the title of the
album a song belongs to, case-insensitively, while keeping the existing title and artist matches
and the library-order result. This makes the offline search find an album's songs by the name the
user sees; the signed-in Apple Music path keeps returning whatever the REST search service
decides.

**Approach.**

- `src/apple_music/sample.rs`: add a small private helper
  `sample_album_titles_by_id() -> std::collections::HashMap<&'static str, String>` that walks
  `sample_albums_in_library_order()` and maps each `Album::id` to its case-folded
  `Album::title`. In `sample_songs_matching`, build the map once per call and keep a song when
  its title, its artist, or its album's folded title contains the folded query; keep the
  existing `sample_songs_in_library_order()` walk and clone, so the result order does not change.
  Update the helper's doc comment to name title, artist, and album title.
- `src/apple_music/browse.rs`: update `AppleMusicService::search_songs`'s doc comment (and the
  `sample_songs_matching` description beside it) to say the sample fallback matches title,
  artist, or album title. No logic changes: the blank-query guard, trimming, and the signed-in
  REST branch stay as they are.
- `src/apple_music/tests.rs`: add
  `search_songs_matches_the_sample_library_by_album_title`, using `test_service()` and
  `assert_ids`. Assert `"  SECOND RECORD  "` returns exactly `song-4` (the only song on
  `Second Record`), and `"first record"` returns `song-1`, `song-2`, and `song-3` in library
  order. Update the comment above
  `search_songs_matches_the_sample_library_by_title_or_artist` so it says this test pins the
  title and artist matches while the new test pins the album-title match; keep its assertions
  unchanged.
- `README.md`: in the search-box sentence, add that the sample fallback matches the query
  case-insensitively against a song's title, artist, and album title. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a short `## Library search matching` section after `## Search
  focus`. State that a sample-library search returns every song whose title, artist, or album
  title contains the trimmed query case-insensitively, in library order; that a blank query is
  rejected; and that the signed-in path returns the REST service's results. Name
  `sample_songs_matching` and `sample_album_titles_by_id` in `src/apple_music/sample.rs` and
  `AppleMusicService::search_songs` in `src/apple_music/browse.rs` as the enforcers, list the
  title/artist/album tests and the blank-query test under **Pinned by**, and record the addition
  in **History**.

**Files touched.** `src/apple_music/sample.rs`, `src/apple_music/browse.rs`,
`src/apple_music/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- With no session, `search_songs` matches a sample song when the query appears case-insensitively
  in its title, its artist, or the title of the album named by its `album_id`.
- `"Second Record"`, `"second record"`, and the trimmed query all return exactly `song-4`;
  `"first record"` returns `song-1`, `song-2`, and `song-3` in library order; title and artist
  matches are unchanged.
- A query matching no title, artist, or album title still returns an empty list, and a blank
  query is still rejected with the existing error.
- No new message, handler, state, dependency, or UI change is introduced; the signed-in REST
  search path is untouched.
- The new test passes, `make check` passes, and README and `docs/invariants.md` describe the
  album-title match.

## Done
