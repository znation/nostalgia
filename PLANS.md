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

### Show each album row's release year beside its running time, like Winamp's Media Library (planned 2026-10-29 by the plan role)

**Goal.** Winamp's Media Library album view has a Year column. Nostalgia's album rows (per-artist
Albums, All Albums, Recently Added) show `artist - title` and, at the right, only the running
time (`10:30`). Show the album's release year before the running time, `2001  10:30` (two
spaces between them). An album with no known year still shows only `10:30` (or `--:--`).

**Approach.**
- `src/library.rs`: add `pub year: Option<u16>` to `Album`, marked `#[serde(default)]` and with a
  doc comment matching `Song::year`'s (`None` means the source supplied no usable release date).
  Add `year: None,` to every `Album { .. }` struct literal (`grep -rn 'Album {' src` lists them,
  in about 13 files, most of them tests). The exceptions are the sample albums below and
  `into_album`.
- `src/apple_music/rest/wire/mapping.rs` `Resource::into_album`: read `attributes.release_date`
  next to `name`/`artist_name` and set `year: release_date.as_deref().and_then(release_year)`,
  which reuses the existing `release_year` helper that `into_song` uses. `Attributes::release_date`
  already exists in `src/apple_music/rest/wire.rs`, so the wire struct does not change.
- `src/sample_library.rs`: give the sample albums the years their songs carry: `First Record`
  `Some(2001)`, `Second Record` `Some(2003)`, `Debut` `Some(2005)`.
- `src/ui/views/mod.rs`: next to `pub fn format_track_time`, add
  `pub fn album_row_label(album: &Album) -> String`. It returns
  `format!("{year}  {}", format_track_time(album.duration_ms))` when `album.year` is `Some`, else
  `format_track_time(album.duration_ms)`. Make it the fallback label in `album_row`
  (`src/ui/views/browse/rows/text.rs`). Then `store_albums` in `src/ui/handlers/browse/fetch.rs` passes
  `views::album_row_label` instead of `views::format_track_time(album.duration_ms)`, so the cached
  label and the fallback match.
- Sorting, Jump to File and selection read `display_title`, so they do not change.

**Files touched.** `src/library.rs`, `src/apple_music/rest/wire/mapping.rs`,
`src/sample_library.rs`, `src/ui/views/mod.rs`, `src/ui/views/browse/rows/text.rs`,
`src/ui/handlers/browse/fetch.rs`, the struct-literal sites, `src/ui/views/tests/browse.rs`,
the REST mapping tests (`src/apple_music/rest/tests/library.rs`), `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**
- New test in `src/ui/views/tests/browse.rs`: `album_row` for an album with `year: Some(2001)` and
  `duration_ms` 630000 returns the label `2001  10:30`. The existing
  `album_row_uses_title_and_duration` (year `None`) still returns just the time. A `--:--` album
  with a year reads `2001  --:--`.
- New REST test in `src/apple_music/rest/tests/library.rs`: a library-albums response whose album
  carries `"releaseDate": "2003-05-01"` maps to `year: Some(2003)`, and one with no `releaseDate`
  maps to `None`.
- A UI test (in an existing album-list test file such as `src/ui/tests/browse_tests.rs`) loads
  the signed-out sample All Albums list and checks that the stored row labels read
  `2001  10:30`, `2003  1:35` and `2005  3:20`, which confirms the `store_albums` cache uses the
  new label.
- `docs/usage.md`: the album-row sentence says the row shows the release year (when known)
  before the running time, e.g. `2001  10:30`. `docs/invariants.md` "Browse album running times"
  gets the year rule, names `album_row_label` under Enforced by, and lists the new tests under
  Pinned by.
- `make check` passes.
- Independent of "Share one song-grouping module between the Genres and Years lists": that plan
  touches `src/ui/genres.rs`, `src/ui/years.rs` and the grouped-list views, not album rows or the
  `Album` model.

## Done
