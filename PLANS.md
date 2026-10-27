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

### Hold and drop peak markers above the LCD spectrum bars, like Winamp's analyzer (planned 2026-10-27 by the plan role)

**Goal.** Winamp's spectrum analyzer draws a thin gray peak marker above each bar. The marker
jumps up with the bar, holds when the bar falls, and then drops back slowly. Nostalgia's LCD
visualizer (`views::now_playing::visualizer::now_playing_visualizer_row`) draws only the bars, so
it jumps straight from one poll's levels to the next. Add the markers. They are presentation state
only, held beside the stored spectrum, and they follow the bars' lifecycle: Pause freezes them,
Stop and a new selection clear them, and hiding the visualizer hides them too. This work does not
depend on the Stop-after-current series and touches none of its files.

**Approach.**

- `src/ui/views/now_playing/visualizer.rs`:
  - Add `const LCD_VISUALIZER_PEAK_FALL: f32 = 0.125;`. This is how far a held peak drops on each
    spectrum reply, as a fraction of full height: 3 px of the 24 px strip, so a peak falls the
    whole strip in about two seconds at the poll's four replies a second.
  - Add `const LCD_VISUALIZER_PEAK_HEIGHT: f32 = 1.0;` (the marker's height in px) and
    `const LCD_VISUALIZER_PEAK_COLOR: Color = Color::from_rgb8(150, 150, 150);` (the base skin's
    `viscolor.txt` peak-dot gray). Every ramp color has blue at or below 41, so this is the only
    visualizer color with strong blue.
  - Add `pub fn next_spectrum_peaks(previous: Option<Spectrum>, spectrum: Option<Spectrum>) ->
    Option<Spectrum>`. A `None` spectrum returns `None`. Otherwise each band becomes
    `max(level, previous_band - LCD_VISUALIZER_PEAK_FALL)` floored at `0.0`. `level` is the
    reported band run through `clamp_with_nan_fallback(band, 0.0, 1.0, 0.0)`, and a missing
    `previous` counts as all zeros. The function is pure so the hold-and-fall rule is testable.
  - Add `pub fn visualizer_peak_marker_bottom(height: f32, peak_height: f32) -> Option<f32>`. It
    returns the marker's bottom edge in px above the rail, which is `max(peak_height, height)`
    clamped to `0.0..=LCD_VISUALIZER_HEIGHT - LCD_VISUALIZER_PEAK_HEIGHT`. It returns `None` when
    `peak_height` is not above `0.0`, NaN included. The marker therefore never sits inside its
    bar, and it always stays inside the strip.
  - `now_playing_visualizer_row` gains a second parameter, `peaks: Option<Spectrum>`. It scales
    the peaks with the existing `visualizer_bar_heights` and zips them with the bar heights.
  - For a band with a marker, the column is built top-down. First a spacer of
    `LCD_VISUALIZER_HEIGHT - bottom - LCD_VISUALIZER_PEAK_HEIGHT`. Then the 1 px marker, a
    `Container` styled with `LCD_VISUALIZER_PEAK_COLOR`. Then a spacer of `bottom - drawn`, then
    the ramp segments of `drawn = height.min(bottom)`.
  - Zero-height spacers are skipped, as the existing spacer already is. The column's fixed
    width and height stay the same.
  - A bar at full height gives its top pixel to the marker. A band without a marker draws
    exactly as today.
  - Update the doc comments of `now_playing_visualizer_row` and the module to describe the
    markers.
- `src/ui/views/now_playing.rs`: re-export `next_spectrum_peaks` beside `visible_spectrum`. Add
  `peaks: Option<Spectrum>` after `spectrum` to both `view_now_playing` and
  `now_playing_content` (both already carry `#[allow(clippy::too_many_arguments)]`), and pass it
  to `now_playing_visualizer_row`.
- `src/ui/views/mod.rs`: add `next_spectrum_peaks` to the `pub use now_playing::{…}` list.
- `src/ui/player.rs`: add `pub(in crate::ui) spectrum_peaks: Option<Spectrum>` after
  `spectrum`. Its doc says it holds the peak markers' levels, follows `spectrum`, is not
  persisted, and starts as `None` in `WinampPlayer::new`.
- `src/ui/handlers/playback/preview.rs`, `spectrum`: inside the existing `reply_is_current`
  guard, set `player.spectrum_peaks = views::next_spectrum_peaks(player.spectrum_peaks,
  spectrum)` before storing `player.spectrum`. A dropped reply therefore moves neither of them.
  Update the doc comment.
- `src/ui/handlers/playback.rs`, `reset_position_readout`: set `player.spectrum_peaks = None`
  beside `player.spectrum = None`. Its doc already says Pause does not call it, so frozen
  markers stay on screen through a pause.
- `src/ui/view/player_panels.rs`: pass `views::visible_spectrum(player.spectrum_peaks,
  player.visualizer_visible)` as the new `view_now_playing` argument, after the spectrum one.
- Existing call sites pass `None` for the new parameter and keep their assertions:
  - `view_now_playing` in `src/ui/views/tests/browse.rs`.
  - `now_playing_content` in `captured_content_with_spectrum` (`src/ui/views/tests/lcd_display.rs`)
    and in `src/ui/views/tests/now_playing.rs`.
  - Every `now_playing_visualizer_row(…)` call in those two test files.
- `src/ui/layout_capture.rs`: add `pub(crate) fn max_blue_in(&self, bounds)`, mirroring
  `max_red_in` (channel 2), plus a unit test like
  `max_red_in_reads_only_the_red_of_pixels_inside_the_bounds`.

**Tests.**

- `src/ui/views/tests/now_playing.rs` (beside
  `visible_spectrum_hides_the_stored_bars_while_the_visualizer_is_off`):
  - `next_spectrum_peaks_follows_a_rising_band_and_falls_slowly_after_it`: a missing previous
    gives the clamped bands. A lower reply drops the held band by `0.125`, never below the new
    band and never below `0.0`. A higher reply raises it to the band. An out-of-range or NaN band
    counts as clamped, NaN as `0.0`. A `None` spectrum returns `None`.
  - `visualizer_peak_marker_bottom_sits_on_or_above_its_bar_inside_the_strip`: `None` for a zero
    or NaN peak. The bar's height when the peak is below it. The peak's height otherwise. `23.0`
    for a full-height peak.
- `src/ui/views/tests/lcd_display.rs`:
  - `now_playing_visualizer_row_draws_a_gray_peak_marker_above_a_fallen_bar`: zero bars, with
    peaks band 0 at `1.0`. Through `layout_capture::capture_text_and_pixels`, slot 0's top 1 px
    row has `max_blue_in` above 100, and slot 0 below it has `max_blue_in` below 100.
  - Extend the existing captured-row height check so a row drawn with peaks keeps
    `LCD_VISUALIZER_HEIGHT`.
- `src/ui/tests/readouts_tests.rs`:
  - Extend `spectrum_reply_stores_the_loaded_previews_spectrum`. After a `recording_spectrum()`
    reply, `spectrum_peaks` equals it. A following reply with band 2 at `0.0` leaves band 2's
    peak at `0.375`. A `None` reply clears both.
  - Extend `stop_clears_the_spectrum_while_pause_keeps_it` so Pause keeps `spectrum_peaks` and
    Stop clears it.
  - Add `a_stale_spectrum_reply_leaves_the_peaks_alone`: stored peaks are unchanged after a
    reply whose epoch is out of date.
- `src/ui/tests/panels_tests.rs`: extend
  `toggle_visualizer_hides_and_shows_the_bars_without_clearing_the_spectrum` so the toggle also
  keeps `spectrum_peaks`.

**Docs.**

- `docs/usage.md`: in the visualizer sentence, after "updating about four times a second while
  playback runs", add that a thin gray peak marker above each bar jumps up with the bar and then
  drops back slowly. Change "Pause freezes the bars at their last levels, Stop and a new
  selection clear them" so it covers the markers too. The `V` sentence says hiding hides the
  markers with the bars.
- `docs/invariants.md`:
  - `## UI layout`: the visualizer bullet gets the marker guarantee. The marker draws only for a
    positive peak, never inside its bar, and always inside the strip. Peaks follow the
    hold-and-fall rule and share the spectrum's reply guards, Pause freeze, Stop and selection
    clear, and visibility flag.
  - The section's code paragraph names `next_spectrum_peaks`, `visualizer_peak_marker_bottom`,
    and `WinampPlayer::spectrum_peaks`, and pins the new tests.

**Acceptance criteria.**

- While a preview plays, each bar with a positive peak shows a 1 px gray marker at its highest
  recent level. The marker drops 3 px for each spectrum reply until the bar reaches it again.
- Pause freezes the markers. Stop, a new selection, or a `None` spectrum reply clears them. A
  stale or in-transition reply changes neither the bars nor the markers. Hiding the visualizer
  hides the markers and keeps their stored levels.
- Without peaks, the bars, ramp colors, row height, slot widths, and the
  `Message::ToggleVisualizer` press are unchanged. `make check` passes.

## Done
