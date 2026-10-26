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

### Show the current track's year in the File info panel, like Winamp's ID3 panel (planned 2026-10-26 by the plan role)

**Goal.** Winamp's File info dialog lists a track's year beside its title, artist, and album.
Nostalgia's File info panel already names the title, artist, album, length, and loaded preview
format, but `Song` carries no release year, so no year can reach the panel. Carry the source's
release year through `Song` and the `KnownTrack` cache and draw a `Year: <yyyy>` line between
`Album:` and `Length:` while the source supplies one. No new state, handlers, settings, or
playback behavior.

**Approach.**

- `src/library.rs`: add `#[serde(default)] pub year: Option<u16>` to `Song`, documented as the
  release year parsed from the source's release date (`None` when absent or malformed). Add the
  `"year": null` key to the test `song_payload`, and add a round-trip test pinning that a
  `Some(2001)` year serializes and deserializes while a payload without `year` defaults to `None`.
- `src/apple_music/rest/wire.rs`: add `#[serde(rename = "releaseDate")] release_date:
  Option<String>` to the private `Attributes`, carry it through `Resource::into_song`'s destructure,
  and set `Song::year` from a new private `release_year(date: &str) -> Option<u16>` helper. The
  helper trims the date and returns `Some` only when its first four bytes are ASCII digits (so
  `2001-06-05`, `2001`, and `2001-06` map to `Some(2001)`; missing, blank, short, or non-numeric
  dates map to `None`).
- `src/sample_library.rs`: give the five sample songs explicit years (for example `2001` for the
  three First Record tracks, `2003` for B-side, and `2005` for Headliner), so the signed-out File
  info panel shows a year too.
- `src/ui/views/now_playing/text.rs`: add `pub year: Option<u16>` to `KnownTrack`, documented as
  copied from `Song::year`. `src/ui/handlers/playback.rs` records `year: song.year` in the
  `KnownTrack` it inserts.
- `src/ui/views/file_info.rs`: add `const YEAR_LABEL: &str = "Year: ";` and push a
  `Year: <year>` line after the album line and before the length line only while `track.year` is
  `Some`; no line otherwise. Update the view's doc comment to name the year.
- Tests: add `release_date_maps_to_the_song_year` and
  `release_date_absent_or_malformed_maps_to_no_year` to
  `src/apple_music/rest/tests/library.rs` (through the existing `song_from_attributes` helper), a
  focused `known_tracks_record_the_played_songs_year` in `src/ui/tests/readouts_tests.rs` that
  plays a song carrying `Some(year)` and asserts the recorded `KnownTrack`, and
  `file_info_overlay_draws_the_tracks_year` plus
  `file_info_overlay_omits_the_year_line_without_one` in `src/ui/views/tests/chrome.rs`. Add
  `year: None` to every test-built `KnownTrack` literal and `year` to every `Song`/`KnownTrack`
  fixture the compiler reports.
- `docs/usage.md`: in the File info sentence, name the year line and say the album and year lines
  are left out when the source supplies none. `docs/invariants.md` `## File info panel`: add the
  `Year: …` line (after `Album: …`, omitted when `KnownTrack::year` is `None`, sourced from
  `Song::year`, itself parsed from `releaseDate` by `Resource::into_song`), name the new tests in
  `Pinned by`, and leave every other guarantee unchanged.

**Files touched.** `src/library.rs`, `src/apple_music/rest/wire.rs`,
`src/apple_music/rest/tests/library.rs`, `src/sample_library.rs`, `src/test_support.rs`,
`src/ui/views/now_playing/text.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/file_info.rs`,
`src/ui/views/tests/chrome.rs`, `src/ui/tests/readouts_tests.rs` (plus `year: None` in the other
`KnownTrack` test literals), `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A REST song resource carrying `attributes.releaseDate` `2001-06-05`, `2001`, or `2001-06` maps
  to `Song::year == Some(2001)`; an absent, blank, short, or non-numeric release date maps to
  `None`.
- Playing a song whose `Song::year` is `Some(2001)` records `KnownTrack::year == Some(2001)`.
- The File info panel draws `Year: 2001` between `Album:` and `Length:` when the year is present
  and draws no Year line when it is absent; the title, artist, album, length, format, Close, and
  backdrop behavior stay as they are.
- The sample library's songs carry years, so the signed-out panel shows one.
- `make check` passes.

## Done
