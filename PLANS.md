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
