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

### Match each word of an offline library search against any field, like Winamp's Media Library search (planned 2026-10-29 by the plan role)

**Goal.** With no Apple Music session, `sample_songs_matching` in `src/apple_music/sample.rs`
matches the whole trimmed query as one substring, so a query that mixes fields — `sample 2003`
(artist + year), `rock ending` (genre + title), `debut echo` (album + artist) — finds nothing,
although Winamp's Media Library search, and this app's own Jump to file
(`views::jump_file::jump_file_matches`), match each space-separated word on its own. Make the
offline search split the query on whitespace and return the songs for which **every** word appears
case-insensitively in **at least one** of the six fields it already searches (title, artist, album
title, genre, composer, release year), each word free to match a different field, in any order.
Results stay in library order. The signed-in REST path is unchanged (Apple's search service does
its own matching).

**Approach.** Decided; no open questions. Standard library only.

1. In `sample_songs_matching`, replace the single `needle` with
   `let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();` and filter
   with `words.iter().all(|word| <the existing six-field disjunction, with `word` in place of
   `&needle`>)`. Keep `sample_album_titles_by_id` and the library-order walk as they are. Moving the
   disjunction into a small private helper (e.g. `fn song_matches_word(song: &Song, album_titles:
   &HashMap<&str, String>, word: &str) -> bool`) is fine. Do not share code with
   `src/ui/views/jump_file.rs` (different layer; the duplicated `split_whitespace` is one line).
2. Update its doc comment: the query is split on whitespace, and a song matches when each word is
   contained case-insensitively in its title, artist, album title, genre, composer, or release
   year. `AppleMusicService::search_songs` (`src/apple_music/browse.rs`) already rejects a blank
   query before this runs, so the no-words case is unreachable there; say so rather than adding a
   second guard.
3. Add a test `search_songs_matches_every_word_across_fields` to
   `src/apple_music/tests/browse_sample.rs` beside the other `search_songs_matches_…` tests, using
   the sample library (`src/sample_library.rs`) and the file's `test_service()` / `assert_ids`:
   - `"sample 2003"` → `["song-4"]` (artist + year);
   - `"  ROCK   ending "` → `["song-3"]` (genre + title, mixed case, extra whitespace);
   - `"debut echo"` → `["song-5"]` (album title + artist, order differs from the fields);
   - `"echo 2001"` → `[]` (each word matches some song, but no single song has both);
   - `"opening zzz"` → `[]` (one word matches nothing).
   The existing `search_songs_matches_…` and `search_songs_rejects_a_blank_query` tests keep passing
   unchanged (every current multi-word query, e.g. `"sample band"`, `"sam ample"`, `"first record"`,
   `"no such track"`, gives the same result under word matching — verify by running them).
4. `docs/usage.md`: in the Browse paragraph, the parenthetical "(with no Apple Music session,
   matching the query case-insensitively against each song's title, artist, album title, genre,
   composer, and release year)" becomes one saying each whitespace-separated word of the query must
   appear case-insensitively in one of those fields, in any order (so `rock 2001` finds the 2001
   rock songs).
5. `docs/invariants.md` `## Library search matching`: reword the Guarantee and Enforced-by
   sentences for per-word matching, add the new test to Pinned by, and add a History sentence
   ("Matched each word separately so a query mixing fields, like Winamp's Media Library search,
   finds its songs offline").

**Files touched.** `src/apple_music/sample.rs`, `src/apple_music/tests/browse_sample.rs`,
`docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- The five queries above give exactly the listed ids through `AppleMusicService::search_songs` with
  no session.
- The existing sample search tests and the REST search tests in
  `src/apple_music/tests/browse_rest.rs` pass unmodified.
- No new dependency, message, state field, or setting; the REST search term is still the trimmed
  query, unsplit.
- `docs/usage.md` and `docs/invariants.md` describe per-word matching, and `make check` passes.

## Done
