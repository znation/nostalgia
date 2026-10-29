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

### Reopen the library at the Years list across launches (part 2/2) ("requires part 1/2 landed") (planned 2026-10-29 by the plan role)

**Goal.** Part 1/2 ("Browse the library by release year from a Years list") saves the Years list
and a year's Songs as `LibraryView::Artists`. Save and restore Years like every other top-level
list (`LibraryView` in `src/settings/library_view.rs`, saved as `library_view`).

**Approach.**
- `src/settings/library_view.rs`: add `LibraryView::Years` with key `"years"`, and add it to the
  `from_key` array and the `keys_round_trip_for_every_variant` test.
- `src/ui/player.rs` `library_view`: map `CurrentView::Years` and `SongsOrigin::Year` to
  `LibraryView::Years`.
- `src/ui/handlers/browse.rs`: add `LibraryView::Years => show_years(player)` to
  `restore_view`, and add `CurrentView::Years` to the flat-list arm of `session_ready` so sign-in
  refetches it.
- Docs: in `docs/usage.md`, add Years to the settings paragraph's list of remembered lists. In
  `docs/invariants.md` `## Settings persistence`, add the key and name the new tests.

**Tests.**
- In `src/ui/tests/settings_tests.rs`, following the Playlists cases: showing Years, and a year's
  Songs, saves `library_view = years`; booting with `years` saved opens the Years list and issues
  its fetch.
- In the test file holding the `session_ready` refetch cases: a session landing while Years
  shows refetches it.

**Acceptance criteria.**
- Quitting on the Years list, or on a year's songs, reopens the library at Years on the next
  launch. Every other saved list behaves as before, and an unknown stored word still falls back
  to Artists.
- The new tests pass, the docs name Years, and `make check` passes.

### Share one song-grouping module between the Genres and Years lists (planned 2026-10-29 by the plan role)

**Goal.** `src/ui/genres.rs` (`Genre`, `group_by_genre`) and `src/ui/years.rs` (`Year`,
`group_by_year`) are near-line-for-line copies: the same `{ name, songs }` row struct, the same
find-or-push grouping loop, the same trailing `Unknown` group and the same `song_count_label`.
Extract the shared part into one module, so a further grouped list (Composers, say) becomes a
key function instead of a third copy. This is a refactor: nothing the player shows changes.

**Approach.**
- New `src/ui/song_groups.rs` (declare `mod song_groups;` in `src/ui/mod.rs`), holding:
  - `pub(in crate::ui) struct SongGroup { pub(in crate::ui) name: String, pub(in crate::ui) songs: Vec<Song> }`
    with the derives `Genre` has today (`Debug, Clone, PartialEq`).
  - `pub(in crate::ui) fn group_songs<K: Ord>(songs: Vec<Song>, key: impl Fn(&Song) -> Option<(K, String)>) -> Vec<SongGroup>`:
    `key` returns `None` for a song with no value (it goes in the `Unknown` group) or
    `Some((sort_key, display_name))`. Songs with equal sort keys share a group named after the
    first display name seen; groups are ordered by sort key, songs keep input order inside a
    group, and an `Unknown` group (the `"Unknown"` constant moves here) is appended last only
    when non-empty. Same algorithm as today's two loops.
  - `songs_label` (moved from `genres.rs`) and `song_count_label(group: &SongGroup)`.
- `src/ui/genres.rs`: `pub(in crate::ui) type Genre = SongGroup;`, and `group_by_genre` becomes
  `group_songs(songs, |song| …)` returning `Some((trimmed.to_lowercase(), trimmed.to_string()))`
  for a non-blank trimmed genre, else `None`. Re-export `song_count_label` (`pub(in crate::ui) use`)
  or update its one caller in `src/ui/handlers/browse/fetch.rs` `genres_loaded` to import it from
  `song_groups`. Its existing tests stay and must pass unchanged.
- `src/ui/years.rs`: `pub(in crate::ui) type Year = SongGroup;`, and `group_by_year` becomes
  `group_songs(songs, |song| song.year.map(|year| (year, year.to_string())))`. Drop its
  `song_count_label` and its `use super::genres::songs_label`; point `years_loaded` in
  `fetch.rs` at `song_groups::song_count_label`. Its existing tests stay and must pass unchanged.
- Callers that name `Genre`/`Year` (`src/ui/player.rs`, `src/ui/message.rs`,
  `src/ui/views/browse.rs`, `src/ui/tests/mod.rs`, `src/ui/tests/sort_tests.rs` struct literals)
  keep compiling through the aliases; no edit is needed there beyond what the compiler demands.
- No `docs/usage.md` change (nothing visible changes). No `docs/invariants.md` change: its
  browse sections name behaviors and test names, none of which move.

**Tests.** In `src/ui/song_groups.rs`'s own `#[cfg(test)]` module, with a key over `Song::id`
or `Song::genre` built from `crate::test_support::sample_song`:
- groups are ordered by sort key, not by display name or first appearance;
- two songs with equal keys but different display names share one group named after the first;
- `None`-keyed songs form a last `Unknown` group, and no `Unknown` group appears when every song
  has a key;
- `song_count_label` reads `1 song` / `2 songs`.
Move `genres.rs`'s `count_label_pluralizes` test here, or keep it there if
`song_count_label` stays re-exported from `genres`.

**Acceptance criteria.**
- `genres.rs` and `years.rs` each contain only their alias, their key function wrapper and their
  existing behavior tests; neither has its own grouping loop or `Unknown` constant.
- The pre-existing tests in `genres.rs`, `years.rs`, `src/ui/tests/flat_lists_tests.rs`,
  `src/ui/tests/years_tests.rs` and `src/ui/tests/sort_tests.rs` pass without edits to their
  assertions; the new `song_groups` tests pass; `make check` passes.
- Independent of "Reopen the library at the Years list across launches (part 2/2)": that plan
  touches settings, `library_view` and `restore_view`, not the grouping modules.

## Done
