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
