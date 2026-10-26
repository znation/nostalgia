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

### Shade the LCD spectrum bars green through orange by height, like the mockup's analyzer (planned 2026-10-26 by the plan role)

**Goal.** The mockup's LCD analyzer paints each bar with a vertical color ramp — dark green at
the rail, then green, yellow-green, yellow, and orange at the tallest peaks
(`docs/screenshots/hero.png`) — while the app paints every bar flat `theme::LCD_GREEN`. Repaint
the existing ten-bar visualizer from that ramp, keeping the bar widths, gaps, fixed row height,
hidden/visible behavior, and the spectrum data untouched. No state, settings, audio, handler, or
playback change.

**Approach.**

- `src/ui/views/now_playing/visualizer.rs`: add a private ramp for the strip,
  `const LCD_VISUALIZER_RAMP_STEP: f32 = 2.0` and `const LCD_VISUALIZER_RAMP: [Color; 12]`, bottom
  to top, using the 8-bit colors sampled from the mockup's 24 px analyzer:
  `(24, 132, 8)`, `(41, 148, 0)`, `(49, 156, 8)`, `(57, 181, 16)`, `(50, 190, 16)`,
  `(41, 206, 16)`, `(148, 222, 33)`, `(189, 222, 41)`, `(214, 181, 33)`, `(222, 165, 24)`,
  `(198, 123, 8)`, `(214, 115, 0)`. Document that the steps are absolute rows above the rail, so a
  bar's color follows its height rather than being stretched over the bar.
- Add `pub fn visualizer_bar_segments(height: f32) -> Vec<(f32, Color)>`: clamp `height` into
  `0.0..=LCD_VISUALIZER_HEIGHT` (NaN → 0), then, for each ramp step bottom first, push
  `(shown, color)` where `shown = (height - index as f32 * LCD_VISUALIZER_RAMP_STEP)
  .clamp(0.0, LCD_VISUALIZER_RAMP_STEP)` and skip zeros. The returned widths sum to the clamped
  height, so every bar keeps its measured height.
- In `now_playing_visualizer_row`, replace each band's single plain-green `Container` with a
  `Column` of fixed width `LCD_VISUALIZER_BAR_WIDTH` and height `LCD_VISUALIZER_HEIGHT`: a top
  `Space` of `LCD_VISUALIZER_HEIGHT - bar_height` (skipped at zero) followed by one full-width
  segment `Container` per slice, iterated from the top slice down (the reverse of
  `visualizer_bar_segments`' bottom-to-top order) and painted that slice's color. A silent band
  draws only its top spacer, so it keeps its fixed-width slot. Keep the row's height, spacing,
  `MouseArea`/`ToggleVisualizer` press, and empty-spectrum behavior exactly as they are, and do not
  change `visualizer_bar_heights`, `Spectrum`, `WinampPlayer`, or any handler.
- `src/ui/views/tests/lcd_display.rs` (and the import list in `src/ui/views/tests/mod.rs`):
  - Add `visualizer_bar_segments_ramp_from_dark_green_to_orange`: `24.0` yields twelve 2 px
    segments in ramp order summing to 24; `3.0` yields a 2 px step plus a 1 px step in the next
    color; `0.0`, a negative, and NaN yield none; and a fractional height such as `12.5` sums to
    12.5 with no segment over 2 px and its partial top in the correct step's color.
  - Update `now_playing_visualizer_row_draws_one_fixed_width_bar_per_band` to use all ten non-zero
    bands and group the width-8 containers by x: expect ten x positions, each group's segment
    heights summing to `band * 24.0` (within 0.01), every segment at most 2 px tall, and every
    color one of the ramp's. Assert the laid-out row width is
    `BAND_COUNT * 8.0 + (BAND_COUNT - 1) * 3.0` for a spectrum with silent bands, so a silent band
    still reserves its slot.
  - Update `now_playing_content_keeps_the_visualizer_height_with_and_without_a_spectrum` to group
    its width-8 containers by x instead of expecting exactly `BAND_COUNT`; keep its "bars sit
    between the title and status row" and equal-height assertions.
  - Add `now_playing_visualizer_row_paints_its_ramp_from_green_to_orange`: lay the row out with
    one band at `1.0` and the rest `0.0`, draw it with `layout_capture::headless_renderer`, and
    assert the full band's top 2 px strip has a much higher maximum red channel than its bottom
    2 px strip. Add `max_red_in` beside `max_green_in` in `src/ui/layout_capture.rs` (same bounds
    walk over the red byte) so the test can read it; the pure test pins the exact ramp colors, so
    the pixel test only proves the drawing uses them.
  - Update the `CapturedContainers` doc comment in `src/ui/views/tests/mod.rs`, which currently
    says counting containers pins the bar count, to say the bar segments are the containers and
    tests group them by their x position.
- `docs/usage.md`: in the visualizer sentence, replace "small green ten-bar spectrum visualizer"
  with the ten bars shaded from dark green at the rail through yellow to orange at their peaks.
- `docs/invariants.md`: in `## Now Playing label`, change the "exactly `BAND_COUNT` green bars"
  bullet to describe up to twelve 2 px ramp steps per bar, bottom to top, from
  `visualizer_bar_segments`; in `## Preview spectrum`, add
  `views::now_playing::visualizer::visualizer_bar_segments` to the enforcer list and the new tests
  to `Pinned by`, and state that the color follows the bar's height above the rail while the bar
  heights, widths, gaps, row height, and visible/hidden behavior stay unchanged.

**Files touched.** `src/ui/views/now_playing/visualizer.rs`, `src/ui/views/tests/lcd_display.rs`,
`src/ui/views/tests/mod.rs`, `src/ui/layout_capture.rs` (the `max_red_in` test helper),
`docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Each non-empty bar is built from consecutive 2 px steps of the ramp starting at dark green at
  the rail; a 24 px bar shows all twelve steps through orange at its top, and a shorter bar shows
  only the steps its height reaches.
- Segment heights for a bar sum to its `visualizer_bar_heights` height (within 0.01), no segment
  exceeds 2 px, zero and NaN heights draw no segments, and every segment uses a ramp color.
- The visualizer row keeps its 24 px height and one fixed-width 8 px slot per band (ten slots,
  3 px gaps), with silent bands still reserving their slots; a press still publishes
  `Message::ToggleVisualizer`, and hiding, Pause, Stop, and a new selection behave exactly as
  before.
- A rendered full-height bar is orange at its top and dark green at its bottom.
- `make check` passes.

## Done
