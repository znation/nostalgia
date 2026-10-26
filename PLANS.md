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

_None yet._

### Show the current track's album in the File info panel, like Winamp (planned 2026-10-26 by the plan role)

**Goal.** Winamp's File info names the current file's album; Nostalgia's panel names only the
title, artist, and length. The REST resource already parses `attributes.albumName` for
`Resource::into_album_duration`, but `Resource::into_song` drops it and `Song` keeps only
`album_id`, so the title is gone by the time a track plays. Carry the album title onto `Song`,
cache it on `KnownTrack` when a track plays, and draw an `Album: …` line in
`view_file_info_overlay` only when the source supplied one. No other view, playback, browse,
search, or persistence behavior changes.

**Approach.**

- `src/library.rs`:
  - Add `pub album: String` to `Song` after `artist`, documented as the album title with `""`
    meaning the source supplied none. Mark it `#[serde(default)]` so an older or partial payload
    without it still deserializes, as `preview_url` does.
  - Add `"album": "First Record"` to `song_payload`. Leave album out of
    `song_required_fields_payload` (it is optional) and add
    `song_deserialization_defaults_a_missing_album_to_empty` parsing that payload.
  - Add `"album": "First Record"` to the payload
    `song_deserialization_ignores_unknown_fields` parses, so it still equals `sample_song()`.
- `src/apple_music/rest/wire.rs` `Resource::into_song`: take `attributes.album_name` from
  `Attributes`, map a missing or whitespace-only name to `""` (the same sentinel as
  `artist_or_empty`), and set it on the returned `Song`. Update the function's doc comment to
  list the album among the mapped fields.
- `src/sample_library.rs`: give song-1, song-2, and song-3 `album: "First Record"`, song-4
  `"Second Record"`, and song-5 `"Debut"`.
- `src/test_support.rs`: `sample_song`, `stepping_songs`, and `single_song_album` carry
  `"First Record"`; `second_album_songs` carries `"Second Record"`. Every `Song` literal the
  compiler flags gains the fixture's album title, or `String::new()` where the test has no album
  context; those literals are in `src/apple_music/rest/tests/library.rs`,
  `src/apple_music/tests/mod.rs`, `src/ui/tests/browse_tests.rs`, and
  `src/ui/tests/transport_tests.rs`. The REST mapping expectations in
  `src/apple_music/rest/tests/library.rs` and `src/apple_music/tests/browse_rest.rs` whose stubs
  omit `albumName` must override `album: String::new()`, since the `..sample_song()` update would
  otherwise supply `"First Record"`.
- `src/ui/views/now_playing/text.rs` `KnownTrack`: add `pub album: String` after `artist`,
  documented as the album title or `""`.
- `src/ui/handlers/playback.rs` `track_selected`: fill `album: song.album.clone()` in the
  `known_tracks` insert, and extend the nearby comment to say title, artist, album, number, and
  length.
- `src/ui/views/file_info.rs`: add `const ALBUM_LABEL: &str = "Album: ";` and push
  `Album: <album>` between the artist and length lines only when `track.album` is non-empty.
  Update the module and function docs.
- Every `KnownTrack` literal the compiler flags gains `album`: the fixtures in
  `src/ui/tests/window_tests.rs`, `src/ui/tests/mod.rs`, `src/ui/tests/timers_tests.rs`,
  `src/ui/tests/view_tests.rs`, `src/ui/views/tests/mod.rs`, and
  `src/ui/views/tests/chrome.rs`. Use `"First Record"` where the fixture already describes
  song-1 and `String::new()` in generic title/marquee fixtures.
- `src/ui/tests/readouts_tests.rs`: extend
  `track_selected_records_the_played_tracks_title_and_time` to assert `track.album ==
  "First Record"` after playing row 1 of `stepping_songs`.
- `src/ui/views/tests/chrome.rs`: give `sample_known_track` an album. Rename
  `file_info_overlay_draws_the_title_artist_and_length` to
  `file_info_overlay_draws_the_title_artist_album_and_length` and assert the `Album: First
  Record` line too. Add `file_info_overlay_omits_the_album_line_when_the_source_supplied_none`,
  using an empty album and asserting no captured text starts with `"Album:"`.
- `src/apple_music/rest/tests/library.rs`: add `songs_from_album_maps_the_album_name` through
  `song_from_attributes` (asserting the mapped `album`) and
  `songs_from_album_maps_a_missing_or_blank_album_name_to_empty` covering a missing `albumName`,
  `""`, and whitespace-only. Update existing expected `Song`s whose stubs omit `albumName` to
  expect `album: String::new()`.
- `README.md` (Usage): the Alt+3 sentence says the panel names the current track's "title,
  artist, album, and length" and notes the album line is left out when the source supplies none.
- `docs/invariants.md` `## File info panel`: add the album line (left out for an empty album) to
  the Guarantee, and add the new/renamed chrome and REST tests to **Pinned by**.

**Files touched.** `src/library.rs`, `src/apple_music/rest/wire.rs`,
`src/apple_music/rest/tests/library.rs`, `src/apple_music/tests/browse_rest.rs`,
`src/apple_music/tests/mod.rs`, `src/sample_library.rs`, `src/test_support.rs`,
`src/ui/views/now_playing/text.rs`, `src/ui/handlers/playback.rs`,
`src/ui/views/file_info.rs`, `src/ui/tests/readouts_tests.rs`, the `KnownTrack` fixtures under
`src/ui/tests/` and `src/ui/views/tests/`, `README.md`, and `docs/invariants.md`.

**Acceptance criteria.**

- `Song` deserializes a missing `album` to `""`, serializes an `album` key, and maps
  `attributes.albumName` from REST, with a blank name mapped to `""`.
- After `TrackSelected` on row 1 of `stepping_songs`, the `known_tracks` entry for `song-1`
  carries album `"First Record"` alongside its existing title, number, and time.
- `view_file_info_overlay` draws `Album: First Record` for a track with an album and no
  `Album:` text for one with `""`; the title, the artist's empty-value omission, the length, and
  the `Close`/backdrop behavior are unchanged.
- No playback, browse, selection, search, settings, or persistence behavior changes; the
  existing File info, window, and library tests pass.
- `make check` passes, including `cargo doc` with no broken intra-doc link.

## Done
