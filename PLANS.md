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

### Open the File info panel from the LCD strip's `I`, like Winamp (planned 2026-10-26 by the plan role)

**Goal.** Winamp's main-window indicator strip has a live `I` cell that opens File info for the
current track. Nostalgia draws the same `O A I D V` strip and already has the File info panel via
Alt+3, but `lcd_indicator_message` leaves `I` inert along with `O` and `D`. Publish the existing
`Message::ToggleFileInfo` from the strip's `I` cell, so the current-track gate, panel closing,
About exclusivity, and shade restore all come from the existing handler. No new message, state,
shortcut, or persistence behavior.

**Approach.**

- `src/ui/views/now_playing.rs`:
  - In `lcd_indicator_message`, add `"I" => Some(Message::ToggleFileInfo)` beside the existing
    `"A"` and `"V"` arms, leaving `O` and `D` to the `_ => None` arm.
  - Update the doc comments on `LCD_INDICATOR_LABELS`, `lcd_indicator_message`, and
    `lcd_indicator_strip`: `A`, `I`, and `V` are live controls, `O` and `D` stay decorative, and
    `I` opens the same File info panel as Alt+3.
- `src/ui/views/tests/now_playing.rs`:
  - Rename `lcd_indicator_message_arms_a_and_v_and_leaves_o_i_d_inert` to
    `lcd_indicator_message_arms_a_i_and_v_and_leaves_o_d_inert`. Assert `A` maps to
    `Message::ToggleAlwaysOnTop`, `V` to `Message::ToggleVisualizer`, and `I` to
    `Message::ToggleFileInfo`; keep `O` and `D` inert in the loop.
  - Rename `lcd_indicator_strip_publishes_always_on_top_on_a_and_visualizer_on_v` to
    `lcd_indicator_strip_publishes_always_on_top_on_a_visualizer_on_v_and_file_info_on_i`.
    Drive a real left press at `A`, `I`, `V`, and `O` as it does now: `I` must publish
    `Message::ToggleFileInfo`, `O` must publish nothing, and the `A`/`V` assertions stay
    unchanged.
  - Leave `lcd_indicator_strip_draws_each_label_in_order_at_a_fixed_width` unchanged; the five
    letters and the fixed width do not change.
- `src/ui/handlers/window.rs`: the `toggle_file_info` doc comment starts "Alt+3: flip …"; say
  Alt+3 or the LCD strip's `I`, and leave the handler body alone.
- `src/ui/views/file_info.rs`: the module doc says the panel names the track without naming a
  route, so no change is needed; this plan adds no album or format line.
- `docs/invariants.md`:
  - In `## Now Playing label`, change the indicator-strip bullet and the `Enforced by` sentence
    that say `A` and `V` are live while `O`, `I`, and `D` are decorative: `A`, `I`, and `V`
    publish messages through `lcd_indicator_message`, `I` publishes `Message::ToggleFileInfo`,
    and only `O` and `D` stay decorative.
  - Replace the old test names in that section's `Pinned by` list with the two renamed tests.
  - In `## File info panel`, add a bullet to the Guarantee: pressing the LCD strip's `I`
    publishes the same `Message::ToggleFileInfo` as Alt+3 and goes through the same
    `handlers::window::toggle_file_info` gate, so it does not open without a known current
    track. Add `views::now_playing::lcd_indicator_message` to `Enforced by` and the renamed
    strip-press test to `Pinned by`.
- `README.md` (Usage, outside the brief markers): in the opening sentence about the
  `O A I D V` strip, say `A` toggles always-on-top, `I` opens the File info panel, and `V` hides
  and restores the visualizer, while only `O` and `D` stay decorative. Leave the later Alt+3
  sentence for the waiting album plan, which already edits it.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests/now_playing.rs`,
`src/ui/handlers/window.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**

- `lcd_indicator_message("I")` returns `Some(Message::ToggleFileInfo)`; `"A"` and `"V"` keep
  their existing messages, and `"O"` and `"D"` still return `None`.
- A left press on the laid-out strip's `I` publishes `Message::ToggleFileInfo`; the `A` and `V`
  presses publish their existing messages, and `O` publishes nothing.
- The panel still opens only while `current_known_track()` resolves: the existing
  `toggle_file_info_needs_a_known_current_track`, About-exclusivity, shade-restore, and
  overlay tests pass unchanged.
- No other strip letter, LCD label, playback, settings, or persistence behavior changes.
- `make check` passes, including `cargo doc` with no broken intra-doc link.

## Done
