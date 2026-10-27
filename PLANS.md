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

### Stop after current, part 1/2: Ctrl+V stops after the current song, like Winamp (planned 2026-10-27 by the plan role)

**Goal.** Winamp's Ctrl+V arms "Stop after current": the playing song plays to its end, and then
playback stops instead of advancing; the choice is used up when it fires. Nostalgia binds plain V
to Stop (`shortcut` in `src/ui/shortcuts.rs`) but leaves Ctrl+V unbound, and
`handlers::playback::preview::playback_finished` always advances when a next song exists. Add the
chord and the one-shot stop. Part 2/2 ("Stop after current, part 2/2: show it in the options menu") adds the
visible menu row; this part has no on-screen indicator, like Winamp's main window.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) stop_after_current: bool` to `WinampPlayer`, doc'd as
  the transient one-shot flag (false at launch, not persisted, cleared when a natural preview end
  consumes it), and initialise it to `false` in `WinampPlayer::new`.
- `src/ui/message.rs`: add `Message::ToggleStopAfterCurrent`, documented beside `Message::Stop`.
- `src/ui/shortcuts.rs`: in `shortcut`'s Control block (the no-Alt, no-Logo match that already holds
  Ctrl+Z/B/R/W/J/A/T), map `v`/`V` with or without Shift to `Message::ToggleStopAfterCurrent`,
  and add `v` to that block's leading comment. Plain V stays `Message::Stop`.
- `src/ui/handlers/playback.rs`: add `pub(in crate::ui) fn toggle_stop_after_current(player)` that
  flips the flag and returns `Task::none()` — no transport, playback, or window change. Dispatch it
  from `update` in `src/ui/mod.rs`.
- `src/ui/handlers/playback/preview.rs`, `playback_finished`: after the existing early returns
  (not finished, stale epoch / pending transition, not playing) and before picking the next song,
  when `player.stop_after_current` holds, set it to `false` and return the existing `stop(player)`
  task. Extend the function's doc comment with this rule. A dropped reply leaves the flag armed,
  and manual Play, Pause, Stop, Next, and Previous neither read nor clear it.

**Tests.**

- `src/ui/shortcuts.rs` tests: a `message_for_maps_ctrl_v_to_stop_after_current` test covering
  `v`/`V` under `Modifiers::CTRL` and `CTRL | SHIFT`, and `Modifiers::CTRL | ALT` and
  `CTRL | LOGO` mapping to none; `assert_letter_bindings`' plain `v` → `Message::Stop` still holds.
- `src/ui/tests/stepping_tests.rs`, reusing `player_playing_stepping_song` and
  `drive_playback_finished` (which yields `Message::TransportSettled` on the stop path, as
  `preview_end_at_the_last_song_stops_when_repeat_is_off` asserts, and `TrackSelected` on an
  advance):
  - `preview_end_with_stop_after_current_stops_and_clears_it`: playing `song-1` of the stepping
    list with a next song available, flag armed → after the finished reply, `is_playing` is false,
    `current_track` is still `song-1`, and `stop_after_current` is false.
  - `preview_end_after_stop_after_current_fired_advances_again`: a second song played after the
    flag fired advances normally on its natural end.
  - `stop_after_current_survives_a_dropped_finished_reply`: a stale-epoch reply leaves the flag
    armed.
  - `toggle_stop_after_current_flips_only_the_flag`: two `Message::ToggleStopAfterCurrent` updates
    go true then false, and the returned task produces no message; playing state is unchanged.

**Docs.** `docs/usage.md`'s keyboard paragraph (beside "X plays, C pauses, V stops") gains Ctrl+V;
`docs/invariants.md` `## Preview end` records the one-shot rule and pins the new stepping tests.

**Acceptance criteria.**

- Ctrl+V (and Ctrl+Shift+V) toggles `stop_after_current`; Ctrl+Alt+V and Ctrl+Logo+V do nothing;
  plain V still stops.
- With it armed, a natural preview end stops playback through the existing `stop` path, keeps the
  song current for Play, and disarms; with it off, natural-end behavior is unchanged (the existing
  `preview_end_*` tests pass untouched).
- No setting is added or persisted, and `make check` passes.

### Stop after current, part 2/2: show it in the options menu (planned 2026-10-27 by the plan role; "requires part 1/2 landed")

**Goal.** Part 1/2 ("Stop after current, part 1/2: Ctrl+V stops after the current song") arms
`WinampPlayer::stop_after_current` with no on-screen state. Winamp shows that choice as a checked
menu item, so add a `Stop after current: On/Off` row to the options menu, after `Library` and
before `About Nostalgia`, which toggles the flag and closes the menu like the other rows.

**Approach.**

- `src/ui/message.rs`: add `OptionsChoice::StopAfterCurrent` (doc: reuses
  `Message::ToggleStopAfterCurrent`) and `OptionsRow::StopAfterCurrent`, placed in
  `OptionsRow::ALL` after `LibraryPanel` (the array grows to 8), mapped in `OptionsRow::message`;
  update `options_rows_map_to_mouse_messages_in_draw_order`.
- `src/ui/handlers/window.rs`, `options_choice`: route the new choice to
  `super::playback::toggle_stop_after_current`. Keyboard selection already clamps against
  `OptionsRow::ALL.len()`, so Up/Down/Enter reach the row unchanged.
- `src/ui/views/options.rs`: adding a seventh bool would push `view_options_overlay` to eight
  parameters and trip clippy's `too_many_arguments` under `make check`. So replace its five bools
  with one `#[derive(Debug, Clone, Copy)] pub struct OptionsState { always_on_top, time_remaining,
  visualizer_visible, eq_panel_visible, library_panel_visible, stop_after_current }` (all `bool`,
  documented). `options_row_label(row, state: OptionsState)` and
  `view_options_overlay(base, state: OptionsState, selected)` take it. Add
  `stop_after_current_label(bool)` → `"Stop after current: On"` / `"Stop after current: Off"`.
  Re-export it beside the existing `pub use options::view_options_overlay;` in
  `src/ui/views/mod.rs` so `src/ui/view/overlay.rs` can name it.
- `src/ui/view/overlay.rs`, `apply_overlays`: build the `OptionsState` from the player fields it
  passes today plus `player.stop_after_current`.

**Tests.**

- `src/ui/views/options.rs`: `stop_after_current_label_mirrors_the_flag`.
- `src/ui/views/tests/chrome.rs`: `options_overlay_element` builds an `OptionsState` with every
  field `all_on`, and `options_overlay_draws_each_row_with_its_state` expects the new row in both
  cases, in draw order.
- `src/ui/tests/panels_tests.rs`: choosing `OptionsChoice::StopAfterCurrent` arms the flag and
  closes the menu (`options_visible` false), and choosing it again disarms it.

**Docs.** `docs/usage.md`'s options-menu sentence lists `Stop after current: On/Off` between
`Library: Shown/Hidden` and `About Nostalgia`; `docs/invariants.md` `## Options panel` names the
row and pins the new tests.

**Acceptance criteria.**

- The options menu shows `Stop after current: On` while the flag is armed and `Off` otherwise,
  between the Library and About rows; clicking it or Enter on it toggles the flag and closes the
  menu.
- Every other row's label, order, and message are unchanged, and `make check` passes.

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
