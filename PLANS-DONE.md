# Done plans

Finished plans, newest first, moved here by tumwater from PLANS.md's `## Done`. This file
only grows: search it with grep, or read one entry by its line range. Never read it whole.

## Done

### Show the transport state on the Now Playing LCD (Transport glyph, part 2/2: planned 2026-10-23 by the plan role, requires part 1/2 landed) — done 2026-10-23 by the feature role

**Goal.** Finish the paused state from `docs/screenshots/transport.png`: while the player is paused, the large elapsed readout blinks instead of sitting steady, while the position it shows stays frozen. Reuse the existing one-second position poll for the blink phase rather than adding a timer, and hide the readout by drawing it transparent rather than blanking it, so the row keeps its bounds and the title does not reflow.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) blink_visible: bool` to `WinampPlayer`, initialized `true`, documented as the paused readout's blink phase: `position_tick` toggles it while paused and holds it true whenever the transport is not paused. The view uses it only in the paused mode.
- `src/ui/handlers/playback.rs`: in `position_tick`, after the `pending_position_transitions` early return and before scheduling the queries, read `player.state.blocking_lock().transport_mode()` into a local and set `player.blink_visible = if mode == TransportMode::Paused { !player.blink_visible } else { true };` so the paused readout alternates once per position tick and a resumed or stopped player is always visible.
- `src/ui/views/now_playing.rs`: add `pub(super) fn elapsed_readout_color(mode: TransportMode, visible: bool) -> Color`, returning `Color::from_rgba(0.0, 0.0, 0.0, 0.0)` for `Paused` with `visible == false` and `theme::LCD_GREEN` otherwise. Add a trailing `elapsed_visible: bool` parameter to `now_playing_row`, `now_playing_content`, and `view_now_playing`; build the elapsed text with `lcd_elapsed_text(elapsed).color(elapsed_readout_color(mode, elapsed_visible))` so the text still lays out at the same width while transparent.
- `src/ui/view.rs`: pass `player.blink_visible` into `views::view_now_playing`.
- `src/ui/tests.rs`: add `position_tick_blinks_the_elapsed_readout_only_while_paused`, building a player with a loaded preview, setting `paused` through the state, and driving `position_tick`: the first tick hides the readout, the second shows it again, and a tick after `play` (or `stop`) leaves it visible.
- `src/ui/views/tests.rs`: add `elapsed_readout_color_blinks_only_while_paused` (`Paused`/hidden is transparent; `Paused`/visible, `Playing`, and `Stopped` in either phase are `LCD_GREEN`) and `now_playing_row_keeps_the_elapsed_bounds_when_the_blink_hides_it` (the paused visible and hidden captures draw the same elapsed string with the same bounds). Update the existing row/content/view calls for the new argument.
- `README.md`: extend the transport-glyph sentence with the paused blink. `docs/invariants.md`: extend the `## Now Playing label` glyph bullet with the blink guarantee and name `position_tick`'s phase flag and `elapsed_readout_color` as the enforcers and the new tests as the pins.

**Files touched.** `src/ui/player.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `src/ui/view.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- While `transport_mode()` is `Paused`, each `position_tick` flips `blink_visible`; while it is `Playing` or `Stopped`, a tick sets `blink_visible` to true, so playing and stopped always show the elapsed readout.
- `elapsed_readout_color(TransportMode::Paused, false)` is fully transparent, and every other mode/phase returns `theme::LCD_GREEN`.
- The elapsed readout keeps the same text and bounds in the paused visible and hidden states, so blinking does not move the title, total, or status row.
- The position poll keeps running while paused as it does today, and the blink adds no new subscription or timer.
- `README.md` and `docs/invariants.md` describe the paused blink, and `make check` passes.

### Show the transport state on the Now Playing LCD (Transport glyph, part 1/2: planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match `docs/screenshots/transport.png`'s three LCD states: a green `▶` beside the elapsed readout while playing, a green `II` while paused, and a red `■` while stopped. The app draws no such glyph today, and `AppState` cannot tell Pause from Stop — both leave `is_playing == false` — so the view has no way to choose between `II` and `■`. Add that distinction to shared state and draw the glyph in a fixed-width slot before the elapsed readout. The paused blink is deliberately left to part 2/2.

**Approach.**

- `src/state.rs`: add `pub paused: bool` beside `is_playing`, documented as true only while the loaded preview is paused and false while playing or stopped; its `Default` value is `false`. Set it in the existing mutators: `toggle_playing` flips `is_playing` and then sets `paused` to the new `!is_playing` (so the pause half pauses and the resume half resumes), `stop` and `play` clear it, and `pause` sets it. Update the `Default` doc's starting-state sentence. Add a public `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum TransportMode { Playing, Paused, Stopped }` and `pub fn transport_mode(&self) -> TransportMode`, returning `Playing` when `is_playing`, `Paused` when `paused`, and `Stopped` otherwise.
- `src/apple_music/playback.rs`: in `play_track`, replace the direct `state.is_playing = true` commit with `state.play()`, and the direct `state.is_playing = false` backend-failure rollback with `state.stop()`; both keep `current_track` as the surrounding code expects, and routing through the setters keeps `paused` from going stale. Update the `pause`/`resume`/`stop` doc comments to name the paused flag.
- `src/ui/theme.rs`: add `pub const LCD_RED: Color = Color::from_rgb(1.0, 0.1, 0.1);` beside `LCD_GREEN`, documented as the stopped-transport glyph color.
- `src/ui/views/now_playing.rs`: import `crate::state::TransportMode` and add `LCD_TRANSPORT_SIZE: f32 = 22.0` and `LCD_TRANSPORT_WIDTH: f32 = 16.0`. Add pure `pub(super) fn transport_glyph(mode: TransportMode) -> &'static str` returning `▶`, `II`, and `■`, and `pub(super) fn transport_glyph_color(mode: TransportMode) -> Color` returning `theme::LCD_GREEN` for `Playing`/`Paused` and `theme::LCD_RED` for `Stopped`. In `now_playing_row`, prepend a `Container` of fixed `LCD_TRANSPORT_WIDTH` holding the glyph as a `Text` at `LCD_TRANSPORT_SIZE` in that color, before the elapsed `MouseArea`. The fixed slot keeps the elapsed readout at one x position as the glyph changes. Add a trailing `mode: TransportMode` parameter to `now_playing_row`, `now_playing_content`, and `view_now_playing`, threading it through unchanged.
- `src/ui/view.rs`: resolve `state.transport_mode()` in the normal-mode state destructure and pass it to `views::view_now_playing`. Shade mode needs no change: its Play/Pause glyph keys off `is_playing`, so paused already shows `▶`, which is correct for a control that resumes.
- `src/ui/views/tests.rs`: update every `now_playing_row`/`now_playing_content`/`view_now_playing` call for the new argument. Change `captured_now_playing_texts` to take a `TransportMode` and assert the glyph is the first drawn fragment, the elapsed readout the second. Add `transport_glyph_names_and_colors_each_mode` (exact strings and exact `LCD_GREEN`/`LCD_RED` colors), `now_playing_row_keeps_the_elapsed_readout_in_place_across_transport_modes` (the elapsed fragment's x is equal in all three modes while each glyph draws), and extend `now_playing_content_draws_the_indicator_strip_beside_the_title` to assert the glyph follows the `O A I D V` strip.
- `src/state.rs` tests: extend `default_state_is_stopped_at_half_volume` with `!state.paused` and `TransportMode::Stopped`, rewrite `toggle_playing_flips_only_the_playback_flag` as `toggle_playing_switches_between_playing_and_paused` (playing → paused → playing), and add `pause_sets_the_paused_mode_while_play_and_stop_clear_it` driving `pause`, `play`, and `stop` and checking `transport_mode()`.
- `src/apple_music/tests.rs`: assert the mode in `pause_stops_playing_but_keeps_current_track` (Paused), `resume_resumes_the_audio_backend` and `resume_after_stop_restarts_the_preview` (Playing), `stop_stops_the_audio_backend` (Stopped), and `toggle_play_pause_pauses_while_playing` (Paused).
- `README.md`: in the Usage paragraph that describes the large elapsed readout, add one sentence naming the green `▶`, green `II`, and red `■` states. Do not touch the equalizer sentence the queued tint plan edits.
- `docs/invariants.md`: in `## Now Playing label`, add a bullet stating the glyph rule, that Pause sets the paused mode while Play and Stop clear it, and that the glyph's fixed-width slot leaves the elapsed readout in place; name `state::TransportMode`/`AppState::transport_mode`, `views::now_playing::transport_glyph`/`transport_glyph_color`, and `now_playing_row` as the enforcers and the new tests as the pins.

**Files touched.** `src/state.rs`, `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `src/ui/theme.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/view.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `AppState::pause` leaves `is_playing == false` and sets `paused == true`; `play` and `stop` clear both; `toggle_playing` alternates `Playing` and `Paused`; `transport_mode()` reports `Stopped` for the default state.
- `transport_glyph` returns exactly `▶` for `Playing`, `II` for `Paused`, and `■` for `Stopped`; `transport_glyph_color` returns `theme::LCD_GREEN` for the first two and `theme::LCD_RED` for `Stopped`.
- The LCD row draws the glyph before the elapsed readout in all three modes, and the elapsed readout keeps the same x across the three glyphs because the slot is fixed-width.
- Pause and Stop remain distinguishable after a real Play/Pause/Stop sequence through `AppleMusicService`: Pause reports `Paused` and keeps the current track, while Stop reports `Stopped` and keeps it too.
- `README.md` and `docs/invariants.md` describe the glyph, and `make check` passes with no new dependency.

### Tint each equalizer band slider by its gain, green through yellow to red (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Close the remaining visible gap in `docs/screenshots/equalizer.png`'s panel: the mockup paints each band slider's whole groove according to that band's stored gain (green at a cut, yellow at `0 dB`, red at a boost), while `view_equalizer` (`src/ui/views/equalizer.rs`) dresses every band in the same gray `chrome_slider_style`. Paint the existing sliders from their gain, with no custom widget, image asset, or dependency.

**Approach.**

- `src/ui/theme.rs`: add three public gain-stop colors beside the existing palette constants: `EQ_GAIN_LOW` (equal to `LCD_GREEN`), `EQ_GAIN_MID` (a yellow, e.g. `Color::from_rgb(0.85, 0.85, 0.0)`), and `EQ_GAIN_HIGH` (a red-orange, e.g. `Color::from_rgb(1.0, 0.2, 0.1)`), each with a doc comment naming the gain it stands for.
- `src/ui/style.rs`: add `pub fn band_gain_color(gain: f32) -> Color`. Run `gain` through `crate::equalizer::clamp_gain` first, so an out-of-range or NaN value cannot leave the range (NaN maps to the flat `0.0`, matching `clamp_gain`'s own fallback), then interpolate the RGB components linearly between `EQ_GAIN_LOW` and `EQ_GAIN_MID` across `GAIN_MIN_DB..=0.0` and between `EQ_GAIN_MID` and `EQ_GAIN_HIGH` across `0.0..=GAIN_MAX_DB`, keeping alpha `1.0`. The three named gains must return the stop colors exactly so the panel's flat preset reads yellow and its extremes read green/red.
- `src/ui/style.rs`: add `pub fn band_slider_style(gain: f32, status: slider::Status) -> slider::Style`. Start from `chrome_slider_style(status)` and replace both halves of `rail.backgrounds` with `Background::Color(band_gain_color(gain))`, leaving `rail.width`, `rail.border`, and the whole `handle` untouched. Both halves matter: iced's slider draws `backgrounds.0` from the range start to the thumb and `backgrounds.1` from the thumb to the range end, so painting only one half would leave a gray segment whose extent moves with the gain. The pure function keeps the color rule testable without building a widget, as the neighboring chrome styles are.
- `src/ui/views/equalizer.rs`: in `view_equalizer`'s band loop, copy `let gain = bands[index];` before constructing the `VerticalSlider` and change its style to `.style(move |_theme, status| style::band_slider_style(gain, status))`. Capture the gain by value rather than the index so the closure owns only a `f32`. Leave the preamp's `chrome_slider` call as it is: the mockup leaves the preamp groove uniform.
- `src/ui/style.rs` test module: import `crate::equalizer::{GAIN_MAX_DB, GAIN_MIN_DB}` and add `band_gain_color_runs_green_through_yellow_to_red` (the three named gains return `EQ_GAIN_LOW`, `EQ_GAIN_MID`, `EQ_GAIN_HIGH` exactly; the midpoint of each half sits between its two stops component-wise), `band_gain_color_clamps_and_maps_nan_to_flat` (far below `GAIN_MIN_DB` is the low stop, far above `GAIN_MAX_DB` the high stop, and `f32::NAN` the mid stop), and `band_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb` (both `rail.backgrounds` equal `Background::Color(band_gain_color(gain))` for a nonzero gain; `rail.width`, `rail.border`, and `handle` equal `chrome_slider_style(status)`'s for `Active`, `Hovered`, and `Dragged`).
- `README.md`: after the Usage sentence listing the equalizer panel's widgets, add one sentence: each band slider's groove is tinted by its own gain, from green at `-12 dB` through yellow at `0 dB` to red at `+12 dB`. Do not rewrite the widget-list sentence itself, because the queued response-curve plan also edits it.
- `docs/invariants.md`: add a short `## Equalizer band tint` section after `## UI layout`: the guarantee above, enforced by `style::band_gain_color` and `style::band_slider_style` and used by `views::view_equalizer`, pinned by the three style tests, with a one-line History noting the tint is presentation-only and does not change the curve the audio filter applies.

**Files touched.** `src/ui/theme.rs`, `src/ui/style.rs`, `src/ui/views/equalizer.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `band_gain_color(GAIN_MIN_DB)`, `band_gain_color(0.0)`, and `band_gain_color(GAIN_MAX_DB)` return `theme::EQ_GAIN_LOW`, `theme::EQ_GAIN_MID`, and `theme::EQ_GAIN_HIGH` exactly, and intermediate gains interpolate between the neighboring stops.
- `band_gain_color` clamps values outside `GAIN_MIN_DB..=GAIN_MAX_DB` to the end stops and maps `f32::NAN` to the flat mid stop instead of producing an invalid color.
- `band_slider_style` paints both rail halves with `band_gain_color(gain)` for every slider status and leaves the chrome rail width, border, and handle identical to `chrome_slider_style(status)`.
- The ten band sliders in `view_equalizer` use `band_slider_style` with their own stored gain, so the flat preset renders all-yellow grooves and the Rock preset renders green cuts and red boosts; the existing `equalizer_panel_constructs_for_both_states_and_gain_endpoints` and `equalizer_band_labels_leave_a_gap_between_frequencies` still pass.
- `README.md` describes the gain tint, and `make check` passes with no new dependency.

### Draw the equalizer's response curve above the band sliders (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Close a visible gap in `docs/screenshots/equalizer.png`'s active panel: the mockup draws a green response line above the ten band sliders (flat for the Flat preset, a traced curve for Rock), while `view_equalizer` (`src/ui/views/equalizer.rs`) renders only the header, preamp slider, and band sliders. Draw the line from the stored band gains without adding a dependency, using a column approximation instead of `iced`'s `canvas` feature (which would pull `lyon_path`).

**Approach.**

- In `src/ui/views/equalizer.rs`, add the plot constants `pub(super) const RESPONSE_CURVE_COLUMNS: usize = 64`, `pub(super) const RESPONSE_CURVE_HEIGHT: f32 = 24.0`, and `RESPONSE_CURVE_LINE_HEIGHT: f32 = 2.0`, sized so the extra row fits the panel without changing the slider rows.
- Add the pure `pub(super) fn response_curve_offsets(bands: &[f32; BAND_COUNT], columns: usize) -> Vec<f32>`: returns exactly `columns` offsets, each a height from the plot's bottom (`0.0` at `GAIN_MIN_DB`, `1.0` at `GAIN_MAX_DB`). Column 0 sits on the first band and the last column on the last band; each intermediate column linearly interpolates between the two adjacent band gains, so the line follows the same gains the sliders and `BandFilter` use. Run each endpoint gain through `clamp_gain` before normalizing, so a NaN or an out-of-range value cannot leave the plot. `columns == 0` returns an empty vector; `columns == 1` returns the first band's offset rather than dividing by zero.
- Add `pub(super) fn response_curve_row<'a, Theme, Renderer>(bands: &[f32; BAND_COUNT]) -> Element<'a, Message, Theme, Renderer>`, generic over the theme and renderer like `now_playing_visualizer_row` so the tests can lay it out with iced's null renderer: a `Container` filling the width at `RESPONSE_CURVE_HEIGHT`, with `theme::LCD_BACKGROUND`, holding a `Row` of `RESPONSE_CURVE_COLUMNS` equal-width columns. Each column is a `Column` of a top `Space` of `(1.0 - offset) * (RESPONSE_CURVE_HEIGHT - RESPONSE_CURVE_LINE_HEIGHT)`, a `RESPONSE_CURVE_LINE_HEIGHT` bar in `theme::LCD_GREEN`, and a bottom `Space` of the remainder; adjacent bars touch so the dots read as one line. The preamp is deliberately not plotted: it is a uniform gain offset, not a bend in the curve.
- Push `response_curve_row(bands)` into `view_equalizer` between the preamp row and `band_row`, so the plot spans the ten sliders and reads at the same place as the mockup. It draws whether the EQ is on or off, tracing the stored gains in both states.
- `src/ui/views/tests.rs`: import the new helpers and add `response_curve_offsets_follow_the_band_gains` (a flat `0.0` curve is `0.5` at every column; all-`GAIN_MIN_DB` is `0.0`; all-`GAIN_MAX_DB` is `1.0`; the first and last offsets equal the first and last gain's normalized values; a two-band `-12.0, 12.0` curve with three columns yields `0.0, 0.5, 1.0`; `0` columns yields none and `1` column does not panic). Add `response_curve_row_keeps_its_height_and_draws_one_segment_per_column`, laying the row out with the null renderer (the `now_playing_visualizer_row_keeps_its_height_without_a_spectrum` pattern), asserting the root height is `RESPONSE_CURVE_HEIGHT` and `CapturedContainers` holds `RESPONSE_CURVE_COLUMNS` containers at `RESPONSE_CURVE_LINE_HEIGHT` for a shaped curve including zero-offset columns, with the rising curve's first segment on the bottom edge and its last on the top, then doing the same for a flat curve and requiring every segment on the midline.
- `README.md`: add the response curve to the Usage sentence that lists the equalizer panel's widgets (`EQ on/off button, preset pick list, preamp slider, and a response curve above ten band sliders`).

**Files touched.** `src/ui/views/equalizer.rs`, `src/ui/views/tests.rs`, `README.md`.

**Acceptance criteria.**

- `response_curve_offsets` returns exactly the requested column count, `0.0`/`0.5`/`1.0` for all-min, flat, and all-max curves, endpoints equal to the first and last band gains' normalized positions, and linear interpolation between adjacent bands (`-12.0, 12.0` across three columns is `0.0, 0.5, 1.0`).
- The equalizer panel draws the curve row between the preamp and band rows at the fixed `RESPONSE_CURVE_HEIGHT`, with one line segment per `RESPONSE_CURVE_COLUMNS`; the existing `equalizer_panel_constructs_for_both_states_and_gain_endpoints` and `equalizer_band_labels_leave_a_gap_between_frequencies` still pass.
- The mockup's flat and Rock shapes are both traced: a flat stored curve draws a centered line, and a curved preset draws a line whose high columns sit above its low columns.
- `README.md` lists the curve among the equalizer panel's widgets.
- `make check` passes (fmt, clippy, rustdoc, tests), and the change adds no dependency or `iced` feature.

### Add a shared headless layout-capture harness so UI geometry bugs confirm offline (planned 2026-10-23 by the steward role as a gap promotion for `real-run-needed`) — done 2026-10-23 by the feature role

**Goal.** Retire the `real-run-needed` validation gap for UI geometry: make the laid-out bounds of the real widget tree readable from a unit test, so a layout or label-reflow bug is confirmed by `make test` instead of a hand-run window measurement. Four Fixed entries carry the `real-run-needed` gap. The two non-layout ones now have offline coverage — the sign-in race through the injectable session seam and `AppleMusicService::wait_for_session`, and the nonce leak through the capturing opener in `authorize_keeps_the_state_nonce_out_of_the_opener_argument` — while the two layout ones, "Sort renders below Top instead of beside it, contradicting the documented layout" and "Transport buttons resize to their label text, so pressing Play/Pause reflows the whole control row", were confirmed only by running the GUI and measuring a captured window. The ad hoc `CapturedText` operation in `src/ui/views/tests.rs` and `CapturedTextBounds` in `src/ui/tests.rs` prove the offline path works; this plan promotes them into one shared harness so the next layout bug does not reconstruct it.

**Approach.**

- Add a test-only `src/ui/layout_capture.rs` (declared `#[cfg(test)] pub(crate) mod layout_capture;` in `src/ui/mod.rs`), holding one `CapturedText` widget `Operation` and a small API: a capture function that lays a prepared `Element` out under a fixed `Limits`, runs the operation, and returns each drawn `(text, Rectangle)` with its absolute bounds plus the root `Size`; a `find(label)` lookup that panics with the captured fragments when a label is missing; and `headless_renderer()`, which builds iced's tiny-skia `Headless` renderer with `futures::executor::block_on` (the pattern `browse_nav_buttons_place_sort_beside_top` already uses), so trees containing the fallback-renderer `bevel` widgets lay out too.
- Move the duplicate capture structs out of `src/ui/views/tests.rs` (`CapturedText`) and `src/ui/tests.rs` (`CapturedTextBounds`) and point both suites at the shared module, keeping the existing helpers' assertions (the Now Playing fragment order, the indicator strip's order and width) intact.
- Rewrite `browse_nav_buttons_place_sort_beside_top` on the harness without weakening it: it must still lay out the whole root `view` (not the nav row alone) and assert Sort shares Top's row and starts at or after Top's right edge.
- Add `transport_row_does_not_reflow_when_the_play_label_changes`: lay the root `view` out with the player stopped and then playing, and assert the `Stop` and `Repeat` labels keep the same absolute x, which fails if a label-driven width change reaches the row as it did before the fixed-width fix. Keep the builder-level `transport_buttons_keep_a_fixed_width_across_label_changes` and the measured-width pins unchanged.
- Add a harness self-test that lays out a small known column and asserts the captured fragments and root size, so a broken capture fails directly rather than as a confusing geometry assertion.
- `docs/invariants.md`: add a `## UI layout` section after `## Browse row order` stating that Sort lays out beside Top in the root view and that the transport row's faces keep their bounds across the Play/Pause, Repeat, and Shuffle label changes; name `views::view_browse_nav_buttons`, `views::view_transport_controls`, and `views::transport::transport_buttons` as the enforcers and cite the two layout tests and the two Fixed entries. Add a line to the "How to use" note pointing layout bugs at the harness.

**Files touched.** `src/ui/layout_capture.rs` (new), `src/ui/mod.rs`, `src/ui/tests.rs`, `src/ui/views/tests.rs`, `docs/invariants.md`.

**Acceptance criteria.**

- One shared test module returns every drawn text fragment with its absolute `Rectangle` and the root `Size` for both a null-renderer view and the tiny-skia root view; both existing capture helpers are deleted and their tests run through it.
- `browse_nav_buttons_place_sort_beside_top` still lays out the root `view` and fails if Sort is stacked below Top or drawn before Top's right edge.
- `transport_row_does_not_reflow_when_the_play_label_changes` lays the root view out in both playback states and asserts Stop and Repeat keep their x, so the historical reflow is confirmed offline.
- The new `## UI layout` invariant states both guarantees, names their enforcing code, and lists the tests that pin them; the harness is discoverable from `docs/invariants.md`'s "How to use" note.
- `make check` passes (fmt, clippy, rustdoc, tests), and the change adds no dependency.

**Cited entries.** `BUGS-FIXED.md`: the four `real-run-needed` entries — "Sort renders below Top instead of beside it, contradicting the documented layout", "Transport buttons resize to their label text, so pressing Play/Pause reflows the whole control row", "The boot-time artists fetch races the async sign-in, so a signed-in user sees the sample library", and "The sign-in nonce travels in the browser opener's command line, so another local user can read it and fetch the page's developer token".

### Show the mini transport in window-shade mode (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match the mockup's rolled-up window (`docs/screenshots/equalizer.png`, the "WINDOW SHADE" panel): shading the window shows, below the title bar, a compact strip with Previous, Play/Pause, Stop, Next, the current elapsed readout, and a volume slider, so playback stays controllable while rolled up. Normal mode is unchanged; the strip only reads existing state and emits existing messages, adding no new playback behavior.

**Approach.**

- Add `src/ui/views/shade.rs` (declare `mod shade;` and re-export it from `src/ui/views/mod.rs` beside `TITLE_BAR_HEIGHT`):
  - `pub(crate) const SHADE_STRIP_HEIGHT: f32 = 28.0;` and `pub(crate) const SHADED_HEIGHT: f32 = TITLE_BAR_HEIGHT + SHADE_STRIP_HEIGHT;`, so the strip height and the whole rolled-up height have one owner each (crate visibility lets `resize_to_shade` read `SHADED_HEIGHT` through `views`).
  - `pub(super) fn play_pause_glyph(is_playing: bool) -> &'static str`: `"II"` while playing and `"▶"` otherwise, the compact twin of `transport::play_pause_label`.
  - `pub(super) fn shade_controls(is_playing: bool) -> [(&'static str, f32, Message); 4]`: the four `(label, fixed width, message)` specs in row order — `"◀◀"`/`Message::PreviousTrack`, [`play_pause_glyph`]/`Message::PlayPause`, `"■"`/`Message::Stop`, `"▶▶"`/`Message::NextTrack` — each with a fixed width (34, 30, 30, 34 px) so the glyph swap cannot reflow the strip. Pure, so labels, messages, and widths are testable without an iced renderer.
  - `pub fn view_shade_strip<'a>(is_playing: bool, volume: f32, elapsed: Cow<'a, str>) -> Element<'a, Message>`: one full-width row built from `fixed_width_button` per `shade_controls` entry with `spacer(6.0)` between them, the elapsed as `Text::new(elapsed).size(14).color(theme::LCD_GREEN)`, and `chrome_slider(VOLUME_MIN, VOLUME_MAX, volume, VOLUME_STEP, 80.0, Message::VolumeChange)` at the right. Paint the row with `style::title_bar_style()` inside a container fixed at `SHADE_STRIP_HEIGHT` (wrapped in `bevel::raised_panel` like the title bar) so the strip shares the title bar's blue chrome. No new message or handler is added.
- `src/ui/mod.rs` `view`: replace the shaded early return's bare title bar with a short state lock for `state.is_playing`, `state.volume()`, and `player.now_playing_elapsed(state.current_track.as_deref())`, then a `Column` of `views::view_title_bar(...)` and `views::view_shade_strip(...)`. Passing the same `now_playing_elapsed` string the LCD uses means the elapsed/remaining toggle carries into shade mode. Update the function's doc comment and inline comment to say the rolled-up tree is the title bar plus the strip. Leave the unshaded assembly untouched.
- `src/ui/handlers/window.rs`: `resize_to_shade` resizes to `views::SHADED_HEIGHT` instead of `views::TITLE_BAR_HEIGHT`, and its doc comment names the strip as part of the rolled-up window. The measurement and restore paths in `toggle_window_shade` and `window_shade_measured` are unchanged.
- `src/ui/views/tests.rs`: add `shade_play_pause_glyph_switches_with_playback_state`, `shade_controls_map_to_transport_messages_and_keep_fixed_widths`, `shade_height_pins_the_rolled_up_window` (`SHADE_STRIP_HEIGHT` pinned to 28.0 and `SHADED_HEIGHT` to their sum), and `view_shade_strip_keeps_its_fixed_height_for_both_playback_states` (lay the widget out with the headless-renderer `Limits` pattern already used in this file and assert the resolved height is `SHADE_STRIP_HEIGHT`).
- `src/ui/tests.rs`: update `title_bar_height_pins_the_shade_strip`'s comment to point at `SHADED_HEIGHT`; add `shaded_root_view_is_the_title_bar_plus_the_strip` (lay out the root `view` with `player.shaded = true` using the headless tiny-skia pattern from `browse_nav_buttons_place_sort_beside_top`, and assert the resolved height is `views::SHADED_HEIGHT`) and `shade_strip_draws_only_while_shaded` (capture text bounds and assert the `"◀◀"` control appears in the shaded tree and not in the normal-mode tree). Keep the existing shade measurement/restore tests green.
- `README.md`: in the Usage paragraph, say shade mode rolls the window up to the title bar plus a compact transport strip with Previous, Play/Pause, Stop, Next, the current time readout, and a volume slider. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Window shade` section after `## Now Playing label`: shade mode builds exactly the title bar plus the fixed-height strip and resizes the window to `SHADED_HEIGHT`; the strip's five controls emit the same messages as their full transport-row twins, its Play/Pause glyph follows the playback state, its time label follows the elapsed/remaining choice, and its volume slider writes `Message::VolumeChange`; normal mode builds no strip. Name `views::shade::view_shade_strip`/`shade_controls`/`play_pause_glyph`/`SHADED_HEIGHT` and `handlers::window::resize_to_shade` as the enforcers, and list the new test names.

**Files touched.** `src/ui/views/shade.rs` (new), `src/ui/views/mod.rs`, `src/ui/mod.rs`, `src/ui/handlers/window.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- In shade mode the rendered tree is the title bar over a strip exactly `SHADE_STRIP_HEIGHT` tall, and the window resizes to `SHADED_HEIGHT`; the normal-mode tree contains no strip.
- The strip shows Previous, Play/Pause, Stop, Next, the current elapsed/remaining readout, and a volume slider, each emitting the same message as its full transport-row twin (`Message::PreviousTrack`, `Message::PlayPause`, `Message::Stop`, `Message::NextTrack`, `Message::VolumeChange`).
- The Play/Pause glyph is `"▶"` while stopped and `"II"` while playing, and every compact button keeps its fixed width across that swap.
- Laying out the strip and the shaded root view with a headless renderer resolves to the fixed heights above, so the view and `resize_to_shade` cannot drift apart.
- Normal-mode layout, the existing shade measurement/restore tests, and playback behavior are unchanged.
- `make check` passes (fmt, clippy, rustdoc, tests).

### Add the LCD spectrum visualizer (Spectrum visualizer, part 2/2: planned 2026-10-23 by the plan role, requires part 1/2 landed) — done 2026-10-23 by the feature role

**Goal.** Complete the mockup's LCD (`docs/screenshots/player.png`): draw a fixed-height row of ten green bars beneath the Now Playing title, whose heights follow the loaded preview's live spectrum from part 1, updating about four times a second while playback runs. The strip is blank (but keeps its height) when no spectrum is reported, so the silent fallback and a mid-download selection do not jump the layout. Pause freezes the bars at their last levels; Stop and a new selection clear them through the same epoch/reset path the sample-rate status row uses. No new dependency and no change to playback or the browse list.

**Approach.**

- `src/apple_music/playback.rs`: add `pub(crate) fn playback_spectrum(&self) -> Option<audio::Spectrum>` forwarding to `self.audio.spectrum()`, documented like `playback_format`. Add `playback_spectrum_forwards_the_audio_output_spectrum` to `src/apple_music/tests.rs`, driving the injected `RecordingAudio`.
- `src/test_support.rs`: give `RecordingAudio` a fixed stub `spectrum` (a `Spectrum` with one distinctive nonzero band) so UI tests can pin what the backend reports, matching its stub position, format, and duration.
- `src/ui/mod.rs`: add `spectrum: Option<Spectrum>` to `WinampPlayer`, initialized `None`, and import `Spectrum` beside `PlaybackFormat`. Add `Message::SpectrumTick` (the timer's tick) and `Message::Spectrum { epoch: u64, spectrum: Option<Spectrum> }` (the query's reply). Add the `update` arms forwarding to the handlers below, add `needs_spectrum_poll` beside `needs_position_poll` (true only while `state.is_playing`), and pass `player.spectrum` to `views::view_now_playing`.
- `src/ui/timers.rs`: add `pub(super) const SPECTRUM_TICK: Duration = Duration::from_millis(250)` and mirror the position timer's plumbing: `spectrum_ticks`, `spectrum_tick_stream`/`spectrum_tick_stream_with`, `spawn_spectrum_thread`, `SPECTRUM_THREAD_NAME = "nostalgia-spectrum-poll"`, and a failure report naming the visualizer. Add a `spectrum_needed` argument to `subscriptions`, and pass `player.needs_spectrum_poll()` from `init_ui`, so the timer runs only while playback is running and is dropped on Pause/Stop.
- `src/ui/handlers/playback.rs`: add `spectrum_tick(player) -> Task<Message>`: when no transport transition is pending, capture `position_epoch` and schedule `apple_music_service.playback_spectrum()`, mapping the reply to `Message::Spectrum { epoch, spectrum }`; otherwise return `Task::none()`. Add `spectrum(player, epoch, spectrum) -> Task<Message>` storing the value only when the epoch is current and no transition is pending, so stale replies cannot restore an old preview's bars. Clear `player.spectrum` in `reset_position_readout` beside the existing position, format, and duration clears, so Stop and a new selection blank the strip while Pause leaves it frozen.
- `src/ui/views/now_playing.rs`: add the visualizer constants (`LCD_VISUALIZER_HEIGHT`, `LCD_VISUALIZER_BAR_WIDTH`, `LCD_VISUALIZER_BAR_GAP`) and `pub fn visualizer_bar_heights(spectrum: Option<Spectrum>) -> [f32; BAND_COUNT]`: `None` yields all-zero heights, and each band is clamped to `0.0..=1.0` and multiplied by the maximum bar height. Add `now_playing_visualizer_row` drawing exactly `BAND_COUNT` bottom-aligned `Container` bars left to right in `theme::LCD_GREEN` inside a row of fixed `LCD_VISUALIZER_HEIGHT`; an empty spectrum draws the same row with zero-height bars. Push that row between `now_playing_row` and `now_playing_status_row` in `now_playing_content`, and thread the `spectrum` argument through `view_now_playing` and `now_playing_content`.
- `src/ui/views/tests.rs`: add `visualizer_bar_heights_scale_and_clamp_the_reported_bands` (`None` all zero, bands `0.0`/`0.5`/`1.0`/`2.0` map to zero/half/full/full) and a `view_now_playing` construction test with `Some` and `None` spectra asserting the title and status text still draw and the visualizer row keeps its height in both cases.
- `src/ui/tests.rs`: add `spectrum_reply_stores_the_loaded_previews_spectrum`, `spectrum_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`, `stop_clears_the_spectrum_while_pause_keeps_it`, `spectrum_tick_queries_spectrum_and_carries_the_epoch`, and `the_spectrum_timer_runs_only_while_playback_runs`; update `position_tick_queries_position_and_format`-style timer tests for the new `subscriptions` argument where the signature changed.
- `README.md`: in the Usage paragraph describing the LCD, say a loaded preview drives a small green ten-bar spectrum beneath the title, that Pause freezes it, and that the strip stays blank (with its height) when no preview reports audio. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Preview spectrum`, add the UI half: the LCD draws exactly `BAND_COUNT` bars from the stored spectrum, heights clamp to the fixed row, Pause freezes them, Stop and a new selection clear them, stale or pre-transition replies are dropped, and the poll runs only while playing. Extend `## Now Playing label` with the visualizer bullet and add `views::now_playing::visualizer_bar_heights`/`now_playing_visualizer_row`, `WinampPlayer::needs_spectrum_poll`, and the new handler and timer test names to the enforcer and Pinned-by lists.

**Files touched.** `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/mod.rs`, `src/ui/timers.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `visualizer_bar_heights` returns exactly ten heights in `BAND_COUNT` order, all zero for `None` and for zero bands, half the maximum for `0.5`, and the maximum for values at or above `1.0`.
- The LCD draws the fixed-height visualizer row beneath the title and above the status row in both the reporting and blank states, so loading a preview or the silent fallback does not move the title or status text.
- The spectrum poll is subscribed only while playback runs, queries `AudioOutput::spectrum` through the service, and drops a reply whose epoch is stale or whose transport transition is still pending; Pause keeps the last bars, Stop and a new selection clear them.
- The silent fallback and a backend without a spectrum leave every bar at zero while keeping the row's height.
- `make check` passes (fmt, clippy, rustdoc, tests).

### Add the LCD spectrum visualizer (Spectrum visualizer, part 1/2: planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Add a narrow backend query that reports the loaded preview's live ten-band amplitude spectrum, computed from the samples the audio worker actually pulls. This is the deferred half of the mockup's LCD visualizer (`docs/screenshots/player.png` draws green bars beneath the elapsed time): it gives part 2 a real signal to draw instead of an animation, with no new dependency and no UI change in this part. The meter is observational only — it must not alter the samples reaching the equalizer, balance, or output. When no preview is loaded, or the backend cannot answer (the silent fallback, a test fake, a closed worker), the query reports nothing, so part 2 can keep its strip blank.

**Approach.**

- `src/audio/meter.rs` (new module declared in `src/audio.rs`): add `pub const SPECTRUM_WINDOW: usize = 2048`, `pub struct Spectrum { pub bands: [f32; crate::equalizer::BAND_COUNT] }`, and re-export `Spectrum` from `src/audio.rs`. A band value is a normalized `0.0..=1.0` amplitude in `crate::equalizer::BAND_HZ` order, so the visualizer and equalizer name the same ten frequencies.
- In the same module, add `pub(super) fn spectrum_bands(samples: &[f32], sample_rate: u32) -> [f32; crate::equalizer::BAND_COUNT]`: run a Goertzel magnitude at each `BAND_HZ` over the supplied mono window, normalize a full-scale sine to about `1.0` (`2 * magnitude / window_len`), clamp each band into `0.0..=1.0`, and report `0.0` for a band at or above Nyquist or for silence. Keep it pure so the frequency mapping is testable without an audio device.
- Add `SpectrumMeter` as the shared state: a `std::sync::Mutex<SpectrumWindow>` holding a fixed `[f32; SPECTRUM_WINDOW]` ring plus a downmix accumulator, and an `AtomicU32` sample rate. `push(sample, channels)` downmixes interleaved samples to mono and overwrites the oldest slot once the ring fills; `take()` copies the ring in chronological order under the lock and computes `spectrum_bands` after releasing it, so the audio thread never waits on the Goertzel work; `clear()` empties the ring; poisoned locks are recovered rather than panicking (the crate forbids production `unwrap`/`panic`).
- Add `pub(super) struct MeterSource<S> { inner: S, meter: Arc<SpectrumMeter> }`: `new` records `inner.sample_rate()`, `next()` pushes the sample and returns it unchanged, and the `Source` impl delegates `current_span_len`, `channels`, `sample_rate`, `total_duration`, and `size_hint` to the inner source. `try_seek` delegates and then clears the window, so a seek does not splice old audio into the new position's spectrum.
- `src/audio.rs`: add `fn spectrum(&self) -> Option<Spectrum> { None }` to `AudioOutput`, documented like `playback_format` (only while a preview is loaded; `None` for the silent fallback, a fake, or an unanswered query). The default keeps every existing implementor compiling.
- `src/audio/player.rs`: `RodioPlayer` gains `spectrum: Arc<SpectrumMeter>`, initialized in `new`. `WorkerPlayer` gains `fn spectrum(&self) -> Option<Spectrum> { None }`. In `append`, after reading the format and duration and before building `EqSource`, wrap the decoded preview with `MeterSource::new(preview, Arc::clone(&self.spectrum))`; leave the rest of the chain and the returned samples unchanged. `RodioPlayer::spectrum` reports `Some(self.spectrum.take())` only while the player holds a preview (`self.format.get().is_some()`), otherwise `None`. Add `Command::Spectrum(Sender<Option<Spectrum>>)`, handle it in `serve_commands` beside `PlaybackFormat`, and add `RodioOutput::spectrum` mirroring `RodioOutput::playback_format`, including the existing query timeout.
- `src/audio/tests.rs`: add `spectrum_bands_peak_at_the_matching_band_for_a_sine` (a 1 kHz stereo-free sine wins the `1_000.0` band and silence gives all zeros), `spectrum_bands_zero_a_band_at_or_above_nyquist` (at 22_050 Hz the `16_000.0` band is zero while 1 kHz remains), `meter_source_passes_samples_through_and_records_their_spectrum` (collect a sine `SamplesBuffer` through `MeterSource`, assert the output samples are unchanged and `take()` peaks at the sine's band), `spectrum_query_returns_the_loaded_players_spectrum` and `spectrum_query_reports_none_when_nothing_is_loaded` (give `RecordingPlayer` an overridable `spectrum`), `silent_output_reports_no_spectrum`, and extend `rodio_output_reports_a_closed_worker_for_every_command` with a `None` spectrum answer. Add a `MeterSource` delegation assertion for sample rate, channels, and total duration.
- `docs/invariants.md`: add a `## Preview spectrum` section after `## Preview download`: the backend reports ten normalized band amplitudes in `BAND_HZ` order only while a preview is loaded, computed from unchanged samples pulled through `MeterSource`; silence and bands at or above Nyquist read zero; Stop, a new selection, and a seek clear the window; the silent fallback and a closed worker report `None`. Name `audio::meter::Spectrum`/`spectrum_bands`/`MeterSource`, `audio::AudioOutput::spectrum`, and `audio::player::RodioPlayer::spectrum` as the enforcers, and list the new test names.

**Files touched.** `src/audio/meter.rs`, `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `docs/invariants.md`.

**Acceptance criteria.**

- `spectrum_bands` maps a 1 kHz sine to the `1K` band with the other bands lower, maps silence to all zeros, and reports zero for a band at or above Nyquist.
- `MeterSource` returns every sample it is given unchanged, delegates the source's sample rate, channels, length, and span, and records the same samples into its `SpectrumMeter`; a seek clears the recorded window.
- `AudioOutput::spectrum` answers `Some` for a loaded preview through `RodioOutput`, and `None` for `SilentOutput`, a recording fake without an override, and a closed worker; Stop and a new preview cannot report the previous preview's spectrum.
- No UI code reads the spectrum yet; the equalizer, balance, volume, and output samples are unchanged by the meter.
- `make check` passes (fmt, clippy, rustdoc, tests).

### Show the loaded preview's bitrate in the LCD status row (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Complete the mockup's LCD status line (`docs/screenshots/player.png`, `256 KBPS 44 KHZ`) by showing the loaded preview's average bitrate beside the sample rate and channel mode: `256 KBPS  44 KHZ  STEREO`. When the backend cannot compute it, the row falls back to today's `44 KHZ  STEREO`. The value is presentation only, derived from the downloaded encoded preview body and the decoded source's own total duration; it follows the loaded preview as the sample rate does (Pause keeps it, Stop and a new selection clear it). The landed status-row plan left bitrate out because `preview::download_and_decode` discards the body length, so this plan carries that length through to `PlaybackFormat`. It is independent of the queued `O A I D V` strip: that adds a column inside the well, while this changes the format value and the existing status text.

**Approach.**

- `src/audio/preview.rs`: add `pub(super) struct DecodedPreview { pub source: rodio::Decoder<Cursor<Vec<u8>>>, pub encoded_bytes: u64 }`. In `download_and_decode`, record `bytes.len()` as `encoded_bytes` before the bytes move into `Cursor`, and return the struct after `rodio::Decoder::new_mp4`. Keep the existing status check, wire/decoded caps, and error wording unchanged.
- `src/audio.rs`: add `pub bitrate_kbps: Option<u32>` to `PlaybackFormat`, documented as the preview's average encoded bitrate in kilobits per second and `None` when the backend cannot report the decoded source's length.
- `src/audio/player.rs`: change `RodioPlayer`'s `WorkerPlayer::Preview` to `DecodedPreview`. Add the pure helper `pub(super) fn preview_bitrate_kbps(encoded_bytes: u64, duration: Duration) -> Option<u32>`: multiply bytes by 8, divide by `duration.as_millis()` (one bit per millisecond is one kilobit per second), round to nearest, and return `None` for zero duration or a result that does not fit `u32`. Change `format_for_source` to take the encoded byte count, fill `bitrate_kbps` by calling `preview_bitrate_kbps` with `source.total_duration()` and passing through `None` when the source reports no length, and keep reading sample rate and channels as it does today. In `append`, destructure `DecodedPreview`, pass `encoded_bytes` to `format_for_source`, and read the same source's duration for `duration_for_source` as today.
- `src/ui/views/now_playing.rs`: change `format_playback_status` to prefix `"{kbps} KBPS  "` when `bitrate_kbps` is `Some`; keep `"{khz} KHZ  {mode}"` when it is `None`. Remove the doc sentence that says bitrate is deliberately absent, and note the fallback. `now_playing_status_row` and its fixed height stay as they are.
- `src/audio/tests.rs`: add `preview_bitrate_kbps_computes_encoded_bits_over_the_preview_length` (`960_000` bytes over 30 seconds is `Some(256)`; zero duration is `None`). Update `format_for_source_reads_sample_rate_and_channels` for the new argument and assert the computed bitrate for a buffer whose total duration is known, and add `bitrate_kbps: None` to every other `PlaybackFormat` literal in the file unless the test is about bitrate.
- `src/ui/views/tests.rs`: extend `format_playback_status_formats_khz_and_channel_mode` with `Some(256)` reading `"256 KBPS  44 KHZ  STEREO"` and a `None` case reading `"44 KHZ  STEREO"`; extend `now_playing_status_row_draws_the_format_or_nothing` to capture the bitrate text; add `bitrate_kbps` to the remaining literals.
- `src/ui/tests.rs`, `src/apple_music/tests.rs`, and `src/test_support.rs`: add `bitrate_kbps: None` (or a value where the fake is meant to represent a loaded preview with a known rate) to every `PlaybackFormat` literal so the suite compiles.
- `README.md`: in the Usage sentence describing the status row, say it shows the preview's bitrate, sample rate, and channel mode (`256 KBPS  44 KHZ  STEREO`) when the backend reports them, and falls back to the sample rate and mode alone when it cannot compute the bitrate. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, replace the status-row bullet's "Bitrate is not shown" sentence with the guarantee that a loaded preview's average bitrate is shown when the download length and decoded duration are both known, is omitted otherwise, and follows the same Pause/Stop/epoch rules as the sample rate. Name `audio::preview::DecodedPreview`, `audio::player::preview_bitrate_kbps`/`format_for_source`, and `views::now_playing::format_playback_status` as the enforcers, and add the new test names to the Pinned-by list.

**Files touched.** `src/audio/preview.rs`, `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `preview_bitrate_kbps(960_000, Duration::from_secs(30)) == Some(256)`, a zero-length duration returns `None`, and `format_for_source` fills `bitrate_kbps` from the encoded byte count and the source's own total duration.
- `download_and_decode` returns a `DecodedPreview` whose `encoded_bytes` is the length of the downloaded body and whose `source` is the decoded preview; the existing download-failure and cap errors are unchanged.
- `format_playback_status` renders `256 KBPS  44 KHZ  STEREO` for `Some(256)`, 44_100 Hz, two channels, and keeps `44 KHZ  STEREO` when the bitrate is `None`; the status row draws that text and keeps its fixed height and blank state.
- Bitrate follows the existing preview lifecycle with no new state: Pause keeps it, Stop and a new selection clear it through the existing `PlaybackFormat` reset and epoch guards, and the silent fallback or a backend without a duration reports no bitrate.
- `make check` passes (fmt, clippy, rustdoc, tests).

### Draw the classic O A I D V strip at the LCD's left edge (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Draw the mockup's still-missing static LCD element (`docs/screenshots/player.png`): a vertical column of the classic `O`, `A`, `I`, `D`, `V` letters down the well's left edge, beside the large elapsed readout and the title/status rows. The strip is decorative — it emits no messages and toggles nothing, because the app already exposes always-on-top through the title bar's `A` button — and it is drawn in every Now Playing state, including the empty `--:--` bar, so the LCD matches the mockup without duplicating controls. It has a fixed width so the content beside it keeps its bounds: the title starts one strip-width further right, and the right-edge total stays at the edge because the content column fills the remaining width. This is independent of the queued seek slider: that plan adds a strip below the LCD well, while this one adds content inside it.

**Approach.**

- `src/ui/views/now_playing.rs`: add `pub(super) const LCD_INDICATOR_LABELS: [&str; 5] = ["O", "A", "I", "D", "V"]` (the mockup's order; the steward's 2026-10-22 note spells the strip `O C I D V`, which the screenshot shows is a typo), a fixed-width constant (`LCD_INDICATOR_WIDTH = 12.0`), and a small text-size constant (`LCD_INDICATOR_SIZE = 10.0`). Add `pub(super) fn lcd_indicator_strip<'a, Theme, Renderer>() -> Column<'a, Message, Theme, Renderer>` with the same text-catalog and renderer bounds as `lcd_text`, building one small [`theme::LCD_GREEN`] `Text` per label in a fixed-width column with tight spacing, top-aligned and vertically arranged. Change `view_now_playing` to put the strip and the existing title/status `Column` in a `Row` before wrapping that row in `bevel::lcd_well`, so the letters sit inside the same recessed well; keep `now_playing_row`, `now_playing_status_row`, and all readout logic unchanged. Add a `pub(super) fn now_playing_content` generic builder holding that `Row`, because `bevel::lcd_well` returns an element tied to iced's fallback renderer and so cannot be laid out with the null renderer the tests use; `view_now_playing` becomes a one-line wrap of it. Update the function's doc comment to mention the decorative strip.
- `src/ui/views/tests.rs`: import `lcd_indicator_strip` and `now_playing_content` (not the constants). Extend `CapturedText` to record each fragment's absolute bounds alongside its text, and add a `captured_indicator_texts` helper that lays out `lcd_indicator_strip()` with iced's null renderer through that operation, returning the drawn text and the laid-out width. Add `lcd_indicator_strip_draws_each_label_in_order_at_a_fixed_width`, asserting the captured text equals the literal `["O", "A", "I", "D", "V"]` and the width equals the literal `12.0`, so changing either production constant fails the test. Add `now_playing_content_draws_the_indicator_strip_beside_the_title`, which lays out `now_playing_content` (a fitting track, e.g. `"00:53"` / `"Opening"` / `"Band"` / `"3:30"` with no format) through the same operation, asserts the first five drawn fragments are the five literal letters, asserts each letter's `x` sits left of the elapsed value, the title, and the total, and asserts the total's right edge reaches the laid-out content width, so a future refactor cannot drop the strip or displace the right-edge total while leaving its builder tested.
- `README.md`: in the Usage sentence that describes the Now Playing bar, say a small vertical `O A I D V` strip runs down the LCD's left edge as in the mockup and is decorative. Keep the hunk to that sentence; do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the `## Now Playing label` section, add a bullet stating that the LCD's left edge always draws the five classic indicator letters `O`, `A`, `I`, `D`, `V` in that order at a fixed width, in every playback state, and that the strip is decorative (it emits no message and changes nothing); name `views::now_playing::LCD_INDICATOR_LABELS` and `lcd_indicator_strip` as the enforcers and add the two new test names to the Pinned-by list.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `LCD_INDICATOR_LABELS` is exactly `["O", "A", "I", "D", "V"]`, and `lcd_indicator_strip` draws them in that order, one letter per line, in the LCD green at a fixed `LCD_INDICATOR_WIDTH` width.
- `view_now_playing` lays the strip out at the left of the existing elapsed/title/status content inside `bevel::lcd_well` (through `now_playing_content`, which the null-renderer test lays out); laying out that content captures all five letters and the unchanged elapsed, title, and total text.
- The strip is present in the empty and loaded states, has no `Message` and no event handler, and does not alter the large readout, the marquee, the status row, or the right-edge total.
- The README and `docs/invariants.md` describe the strip as decorative, and `make check` passes (fmt, clippy, rustdoc, tests).

### Add a draggable seek slider beneath the Now Playing LCD (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Fill the horizontal groove `docs/screenshots/player.png` draws between the LCD well and the transport row: a chrome slider whose thumb tracks the loaded preview's playback position, and which seeks when the user drags and releases it. The bar spans the loaded preview's own backend-reported length rather than the catalog track's length, because a preview asset can be shorter than the song (the right-edge LCD total keeps showing the catalog length). It is drawn only while the backend reports both a position and a preview length — the production `rodio` backend does; the silent fallback does not — and otherwise renders as an empty fixed-height strip so the layout does not jump. Dragging moves only the thumb; releasing sends exactly one absolute seek. This is independent of the queued sample-rate/channel status row: that plan adds a row inside the LCD well and a format query, while this one adds a sibling strip below the well plus a duration query and an absolute-seek seam. Both plans edit `position_tick` and `reset_position_readout`, so whichever lands second extends the existing batch and clear rather than replacing them.

**Approach.**

- `src/audio.rs`: add `fn seek_to(&self, position: Duration) -> Result<(), AppleMusicError>` and `fn duration(&self) -> Option<Duration> { None }` to `AudioOutput`. Document `seek_to` as an absolute seek of the current player, clamped by the backend to the loaded preview's length and doing nothing while stopped; document `duration` as the loaded preview's own length, answering `None` when no player is loaded or the backend cannot answer, with the default keeping existing fakes compiling. `SilentOutput::seek_to` logs and returns `Ok`; `duration` inherits `None`.
- `src/audio/player.rs`:
  - Add `Command::SeekTo(Duration)` and `Command::Duration(Sender<Option<Duration>>)`.
  - Add `fn seek_to(&self, position: Duration)` and `fn duration(&self) -> Option<Duration>` to `WorkerPlayer`.
  - Add `pub(super) fn duration_for_source(source: &impl rodio::Source) -> Option<Duration>` beside `seek_target`, returning `source.total_duration()`. Give `RodioPlayer` a `duration: OnceLock<Option<Duration>>`; in `append`, set it from `duration_for_source(&preview)` before the preview is wrapped and moved. `RodioPlayer::duration` returns the stored value, and `RodioPlayer::seek_to` clamps the target to that value (a `clamp_seek(position, duration)` free function beside `seek_target`, with `None` meaning no clamp) before calling `self.player.try_seek`, logging a rejected seek without changing state.
  - In `serve_commands`, handle `Command::SeekTo(position)` by calling `player.seek_to(position)` only when a player is installed, and `Command::Duration(reply)` by sending `player.as_ref().and_then(WorkerPlayer::duration)`.
  - Implement `AudioOutput::seek_to` on `RodioOutput` by sending `Command::SeekTo`, and `AudioOutput::duration` by sending `Command::Duration` and waiting on the reply with the existing `POSITION_QUERY_TIMEOUT` (`None` when the send fails or the wait times out). Extend that constant's doc to say it bounds the position and duration queries, without renaming it.
- `src/apple_music/playback.rs`: add `pub(crate) fn seek_output_to(&self, position: Duration) -> Result<(), AppleMusicError>` forwarding to `self.audio.seek_to(position)`, and `pub(crate) fn playback_duration(&self) -> Option<Duration>` forwarding to `self.audio.duration()`. Document both as synchronous and bounded by the seam, like `playback_position`.
- `src/ui/views/seek_bar.rs` (new small module): add `pub fn seek_bar_value(position: Option<Duration>, duration: Option<Duration>) -> Option<(f32, f32)>`, returning `(thumb seconds, length seconds)` with the thumb clamped into `0..=length` only when both a position and a positive duration are reported, or `None` when either is absent (or the duration is zero), so the bar appears only once the backend reports a position to place its thumb. Add `pub fn view_seek_bar(position: Option<Duration>, duration: Option<Duration>) -> Element<'static, Message>`: when `seek_bar_value` answers, build a `Slider::new(0.0..=length, thumb, Message::SeekPreviewChanged)` with `.step(1.0)`, `.on_release(Message::SeekPreviewReleased)`, `Length::Fill` width, and `style::chrome_slider_style`; when it does not, build an empty `Space` of the same fixed height. Wrap either in a fixed-height `Container` (`SEEK_BAR_HEIGHT = 16.0`) so the strip never changes height. Construct the `Slider` here rather than widening the fixed-width `views::chrome_slider` helper, because this slider fills the row.
- `src/ui/views/mod.rs`: declare `mod seek_bar;` and re-export `seek_bar_value`/`view_seek_bar` beside the other view builders.
- `src/ui/mod.rs`: add `playback_duration: Option<Duration>` and `seek_preview: Option<Duration>` to `WinampPlayer`, both initialized `None` and documented as the backend-reported preview length and the in-progress drag. Add `Message::PlaybackDuration { epoch: u64, duration: Option<Duration> }`, `Message::SeekPreviewChanged(f32)`, and `Message::SeekPreviewReleased`, routed from the `update` table to `handlers::playback`. In `view`, push `views::view_seek_bar(player.seek_preview.or(player.playback_position), player.playback_duration)` after `view_now_playing` and before the transport row; both fields are plain player state, so no state lock is needed.
- `src/ui/handlers/playback.rs`:
  - `position_tick` batches the existing position query with a second query of a cloned service's `playback_duration()` mapped to `Message::PlaybackDuration`, and returns `Task::batch`; both are skipped while `pending_position_transitions` is nonzero.
  - `playback_duration` stores the reply only when `epoch == player.position_epoch` and `pending_position_transitions == 0`, mirroring `playback_position`.
  - `seek_preview_changed` stores `Duration::from_secs_f32(value.max(0.0))` in `seek_preview` and returns no task, so dragging never touches the backend.
  - `seek_preview_released` takes the stored drag: when it is `Some(position)`, call `player.apple_music_service.seek_output_to(position)`; on success bump `position_epoch` (dropping any position or duration reply already in flight), set `playback_position = Some(position)`, and clear `seek_preview`; on failure log the backend error and clear `seek_preview` so the next poll restores the real position. When no drag is stored it only clears the field.
  - `reset_position_readout` also clears `playback_duration` and `seek_preview`, so Stop and a new selection remove the bar's scale and any drag with the position.
- Tests. `src/audio/tests.rs`: extend `every_command` with `seek_to` (and the closed-worker and command-mapping tests with it); add `silent_output_reports_no_playback_duration` asserting `duration().is_none()`; add `duration_for_source_reads_a_samples_buffers_length`, `clamp_seek_clamps_to_the_preview_length_and_passes_through_without_one`, `rodio_output_asks_the_worker_for_its_duration`, `rodio_output_reports_none_when_the_duration_reply_times_out`, `seek_to_command_reaches_the_loaded_player`, `duration_query_returns_the_loaded_players_duration`, and `duration_query_reports_none_when_nothing_is_loaded`; give `RecordingPlayer` a settable absolute position and duration, make `seek_to` set the position, and keep its no-seek position at `42s` for the existing position tests. `src/test_support.rs`: add `AudioCall::SeekTo(Duration)`, record it in `RecordingAudio::seek_to`, return a fixed `Some(Duration::from_secs(30))` from `RecordingAudio::duration`, and implement `FailingAudio::seek_to` through the existing `FailingCommand::Seek` with `duration` answering `None`. `src/apple_music/tests.rs`: add `seek_output_to_forwards_an_absolute_position`, `seek_output_to_propagates_an_audio_backend_failure`, and `playback_duration_forwards_the_audio_output_duration`. `src/ui/views/tests.rs`: add `seek_bar_value_clamps_the_thumb_and_hides_without_a_length` (position 45s/length 30s → `(30.0, 30.0)`, `None` position → `None`, absent/zero length → `None`) and call `view_seek_bar` with a known and an unknown length to pin that both construct without panicking. `src/ui/tests.rs`: add `playback_duration_reply_stores_the_loaded_previews_length`, `playback_duration_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`, `stop_and_track_selection_clear_the_playback_duration`, `seek_preview_changed_tracks_the_drag_without_touching_the_backend`, `seek_preview_released_seeks_once_to_the_dragged_position` (asserting one `AudioCall::SeekTo`, the optimistic `playback_position`, and a bumped epoch), `seek_preview_released_without_a_drag_does_nothing`, `seek_preview_released_invalidates_an_in_flight_position_reply`, `seek_preview_released_backend_failure_leaves_the_position_alone`, and `position_tick_queries_position_and_duration` (drive the batched task's stream to its end and assert both replies arrive).
- `README.md`: in the Usage paragraph that describes the Now Playing bar, say a slider beneath it tracks the loaded preview and drags to seek, spans the preview's own length, and stays blank when the backend reports no length. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the `## Now Playing label` section, add the seek-slider guarantee (the bar measures the backend-reported preview length, the thumb follows the stored position or the drag in progress, only a release sends `AudioOutput::seek_to`, the seek is clamped to that length, a successful seek records the position optimistically and invalidates in-flight replies, a failed seek lets the next poll correct the thumb, and Stop/new selection clear the length and drag) and name `seek_bar_value`, `view_seek_bar`, the `SeekPreviewChanged`/`SeekPreviewReleased`/`PlaybackDuration` handlers, `seek_output_to`, and the duration query as the enforcers. Add the new test names to the Pinned-by list.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/views/seek_bar.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `seek_bar_value` returns `(30.0, 30.0)` for a 45-second position on a 30-second preview, and `None` for a missing position or a missing or zero preview length; the thumb never exceeds the bar's length.
- The slider is drawn beneath the LCD only while both a position and a positive preview length are reported; with no preview, the silent backend, Stop, or a new selection the strip is blank and keeps its fixed height.
- Dragging the slider stores the drag value and sends no backend command; releasing sends exactly one absolute `seek_to` with the dragged position, which the backend clamps to the preview length, and the bar shows that position until the next backend reply.
- Stop and a new selection clear the stored preview length and any drag immediately; a `PlaybackDuration` reply stamped with an older epoch or landing while a transport transition is in flight is dropped.
- `position_tick` asks the backend for both the position and the duration, and asks for neither while a transport transition is in flight.
- The right-edge LCD total continues to show the catalog track's length, not the preview length, and the README and `docs/invariants.md` describe the slider that way; `make check` passes (fmt, clippy, rustdoc, tests).

### Show the loaded preview's sample rate and channel mode in the Now Playing LCD (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Fill the real, queryable part of the mockup's LCD status line (`docs/screenshots/player.png`): while a preview is loaded, show its sample rate and mono/stereo mode in a small green row beneath the Now Playing title (`44 KHZ  STEREO`), and leave that row blank when no preview or backend reports a format. The value is presentation only and follows the loaded preview: Pause keeps it, Stop and a new selection clear it. Bitrate stays out of scope because `download_and_decode` discards the downloaded body length, so `256 KBPS` cannot be computed without a separate preview-metadata change. This is a separate LCD concern from the queued elapsed/remaining toggle — that plan changes the large left readout and `KnownTrack`, while this one adds an audio query and the status row — and the two are additive edits at distinct insertion points in the same files.

**Approach.**

- `src/audio.rs`: add `pub struct PlaybackFormat { pub sample_rate_hz: u32, pub channels: u16 }` (derive `Debug, Clone, Copy, PartialEq, Eq`, with doc comments on the type and both fields). Add `fn playback_format(&self) -> Option<PlaybackFormat> { None }` to `AudioOutput`, documented as reporting the loaded preview's format and answering `None` when no player is loaded or the backend cannot answer; the default keeps `SilentOutput` and every existing test fake compiling, and `SilentOutput` inherits the `None` answer.
- `src/audio/player.rs`:
  - Import `rodio::Source` and add `pub(super) fn format_for_source(source: &impl rodio::Source) -> PlaybackFormat` beside `seek_target`, reading `source.sample_rate().get()` and `source.channels().get()`. Give `RodioPlayer` a `format: OnceLock<PlaybackFormat>` initialized in `new`; in `append`, call `format_for_source(&preview)` before moving the preview and set the cell (each track builds a fresh player, so it is set once). Add `fn playback_format(&self) -> Option<PlaybackFormat>` to `WorkerPlayer` (default `None`, so the recording fake compiles) and have `RodioPlayer` return the stored copy.
  - Add `Command::PlaybackFormat(Sender<Option<PlaybackFormat>>)`. In `serve_commands`, answer it with `player.as_ref().and_then(WorkerPlayer::playback_format)`, so a stopped or failed player has no format.
  - Implement `AudioOutput::playback_format` on `RodioOutput` by sending the command and waiting on the reply with the existing `POSITION_QUERY_TIMEOUT` (`None` when the send fails or the wait times out). Extend that constant's doc to say it bounds both queued backend queries, since a download can hold the worker past it.
- `src/apple_music/playback.rs`: add `pub(crate) fn playback_format(&self) -> Option<audio::PlaybackFormat>` forwarding to `self.audio.playback_format()`, documented as synchronous and bounded by the seam (like `playback_position`).
- `src/ui/views/now_playing.rs`:
  - Add `pub fn format_playback_status(format: PlaybackFormat) -> String`, returning `"{khz} KHZ  {mode}"` where `khz = sample_rate_hz / 1000` and `mode` is `"MONO"` for one channel and `"STEREO"` for any other count. Document the integer-kilohertz truncation and that previews are mono or stereo.
  - Add `pub(super) fn now_playing_status_row<'a, Theme, Renderer>(format: Option<PlaybackFormat>) -> Row<'a, Message, Theme, Renderer>` with the same text-catalog and renderer bounds as `now_playing_row`. It is a fixed-height row (16px) with a `Length::Fill` spacer then a small (12px) `theme::LCD_GREEN` status text right-aligned; with `None` it draws no text but keeps the height, so the bar does not jump when a preview loads.
  - Change `view_now_playing` to take a sixth `format: Option<PlaybackFormat>` argument and wrap `Column::new().push(now_playing_row(...)).push(now_playing_status_row(format))` in `bevel::lcd_well`, so the status line sits beneath the title/total inside the same LCD well. Keep `now_playing_row` itself unchanged.
- `src/ui/mod.rs`: add `playback_format: Option<PlaybackFormat>` to `WinampPlayer`, initialized `None` and documented as the loaded preview's backend-reported format (Pause keeps it; Stop and a new selection clear it). Add `Message::PlaybackFormat { epoch: u64, format: Option<PlaybackFormat> }` with a doc comment naming the epoch guard, route it from the `update` table to `handlers::playback::playback_format`, and add the field to the tuple `view` resolves under the state lock before passing it to `views::view_now_playing`.
- `src/ui/handlers/playback.rs`: in `position_tick`, after the pending-transition guard, build the existing position query and a second query of a cloned service's `playback_format()` mapped to `Message::PlaybackFormat`, and return `Task::batch([position, format])`; both are therefore skipped while a transition is in flight. Add `pub(in crate::ui) fn playback_format(player, epoch, format) -> Task<Message>` that stores `format` only when `epoch == player.position_epoch` and `pending_position_transitions == 0`. In `reset_position_readout`, also set `player.playback_format = None`, so Stop and a new selection clear the row immediately; Pause deliberately leaves the existing reset path alone.
- Tests. `src/audio/tests.rs`: add `format_for_source_reads_sample_rate_and_channels` over a mono and a stereo `SamplesBuffer`; give `RecordingPlayer` a `playback_format` returning a fixed format and add `playback_format_query_returns_the_loaded_players_format` and `playback_format_query_reports_none_when_nothing_is_loaded`; add `rodio_output_asks_the_worker_for_its_format`; extend `silent_output_reports_no_playback_position` (or a sibling) to assert `playback_format().is_none()`, and extend `rodio_output_reports_a_closed_worker_for_every_command` with a `None` format answer. `src/test_support.rs`: have `RecordingAudio::playback_format` return `Some(PlaybackFormat { sample_rate_hz: 44_100, channels: 2 })`; `src/apple_music/tests.rs`: add `playback_format_forwards_the_audio_output_format`. `src/ui/views/tests.rs`: import the new helpers and cover `format_playback_status_formats_khz_and_channel_mode` (`44_100`/2 → `44 KHZ  STEREO`, `44_100`/1 → `44 KHZ  MONO`, `48_000`/2 → `48 KHZ  STEREO`) and `now_playing_status_row_draws_the_format_or_nothing` by laying out the row with the existing null renderer and reading its text; update the three `view_now_playing` calls in `now_playing_bar_and_browse_action_buttons_construct` with the new argument. `src/ui/tests.rs`: add `playback_format_reply_stores_the_loaded_previews_format`, `playback_format_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`, `stop_clears_the_playback_format_while_pause_keeps_it`, and `position_tick_queries_position_and_format` (drive the batched task's stream to its end and assert both reply messages arrive).
- `README.md`: in the Usage sentence that describes the Now Playing bar, say a loaded preview's sample rate and mono/stereo mode appear in a small status row beneath the title when the backend reports them. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the `## Now Playing label` section, add the status-row bullet (format shown only while a preview is loaded, `44 KHZ  STEREO`/`MONO` from the backend's own source, blank when absent, kept across Pause, cleared by Stop/new selection, stale/pre-transition replies dropped, bitrate not shown) and name `PlaybackFormat`, `format_for_source`, `views::format_playback_status`/`now_playing_status_row`, and the `PlaybackFormat` handler plus `reset_position_readout` as the enforcers. Add the new test names to the Pinned-by list.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/views/now_playing.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `format_playback_status` renders `44 KHZ  STEREO` for a 44.1 kHz stereo format, `44 KHZ  MONO` for one channel, and `48 KHZ  STEREO` for 48 kHz.
- The LCD draws that text in a small green row beneath the title only while the backend reports a format; with no reported format the row is blank and keeps the bar's height.
- Stop and a new selection clear the status text immediately; Pause keeps it; a reply stamped with an older epoch or landing while a transport transition is in flight is dropped.
- `position_tick` asks the backend for both the position and the format, and asks for neither while a transport transition is in flight.
- No bitrate is displayed; the README and `docs/invariants.md` say the row carries sample rate and channel mode only, and `make check` passes (fmt, clippy, rustdoc, tests).

### Toggle the Now Playing elapsed readout between elapsed and remaining time (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Make the large left-hand LCD readout behave like Winamp's: clicking it switches from the elapsed time (`00:53`) to the time remaining (`-02:37`), and clicking again returns to elapsed. The mode is presentation-only, survives Pause, Stop, Next/Previous, and a new selection until clicked again, and changes nothing about playback, the right-edge total, or the marquee. This is independent of the queued Sort plan: it touches the Now Playing machinery, not the browse lists.

**Approach.**

- `src/ui/views/now_playing.rs`:
  - Add `pub enum TimeDisplay { Elapsed, Remaining }` (derive `Debug, Clone, Copy, PartialEq, Eq, Default`, with `Elapsed` as `#[default]`) and a `pub fn toggled(self) -> Self` beside it (public so `handlers::playback` can flip it; `pub(super)` would be confined to `views`). Re-export `TimeDisplay` from `src/ui/views/mod.rs` like `KnownTrack`.
  - Add `duration_ms: u64` to `KnownTrack` and document it as the raw length the remaining readout subtracts from (the existing `time` field stays the formatted total for the right edge). `src/ui/handlers/playback.rs`'s `track_selected` records `duration_ms: song.duration_ms` in the same entry it already builds.
  - Add `pub fn format_lcd_remaining(remaining_ms: u64) -> String`, formatting the shared core of `format_lcd_elapsed` with a leading `-`: `0` → `"-00:00"`, `157_000` → `"-02:37"`, `3_600_000` → `"-60:00"`.
  - Change `now_playing_elapsed` to take the display mode as its last argument. Unknown/absent track still returns borrowed `"--:--"`. In `TimeDisplay::Elapsed` keep today's behavior exactly (`00:00` with no reported position). In `TimeDisplay::Remaining`, return borrowed `"--:--"` when the known track's `duration_ms` is `0` (no length to count down from), otherwise `Cow::Owned(format_lcd_remaining(duration_ms.saturating_sub(position_ms)))`, treating a missing position as zero (so a stopped track shows its full length remaining). Position milliseconds come from the existing `duration_millis` helper. Document the mode's persistence and the saturating subtraction.
  - Wrap the elapsed fragment in `now_playing_row` in `iced::widget::MouseArea::new(lcd_elapsed_text(elapsed)).on_press(Message::ToggleTimeDisplay).interaction(iced::mouse::Interaction::Pointer)`, keeping it the row's first child so the existing captured-text tests still see it first. The helper's existing `text::Renderer` and theme bounds already satisfy `MouseArea`, and `view_now_playing` keeps its signature.
- `src/ui/mod.rs`:
  - Add `Message::ToggleTimeDisplay` next to `PlaybackPosition` with a doc comment saying it only flips the LCD's time mode, and dispatch it to `handlers::playback::toggle_time_display` beside the position arms (not among the browse arms, so the Sort plan's message edits stay separate).
  - Add `time_display: TimeDisplay` to `WinampPlayer`, initialized `TimeDisplay::Elapsed` in `new` (or via `Default`), documented as presentation-only and preserved across tracks. Pass it to `views::now_playing_elapsed` in `WinampPlayer::now_playing_elapsed`; the accessor keeps its one-argument signature.
- `src/ui/handlers/playback.rs`: add `pub(in crate::ui) fn toggle_time_display(player: &mut WinampPlayer) -> Task<Message>` that assigns `player.time_display = player.time_display.toggled()` and returns `Task::none()`. It touches no playback state and clears no position.
- Tests. `src/ui/views/tests.rs`: give the `known_tracks` helper `duration_ms: 0`; update its existing `now_playing_elapsed` calls with `TimeDisplay::Elapsed`; add `format_lcd_remaining_prefixes_a_minus_and_pads` and `now_playing_elapsed_remaining_counts_down_from_the_total` (known 210_000 ms track with a 53 s position → `-02:37`, no position → `-03:30`, `duration_ms` 0 → `--:--`, unknown id → `--:--`, a position past the total → `-00:00`). `src/ui/tests.rs`: add `duration_ms` to the two hand-built `KnownTrack` literals, then add `toggle_time_display_switches_to_remaining_and_persists` beside the elapsed tests: play song 1 (210 s), set `playback_position` to 53 s, send `Message::ToggleTimeDisplay`, assert `-02:37` and an unchanged right-edge total; send `Message::Pause` and assert the remaining value survives; select song 2 and assert the new track counts down from its own total (`-02:05`); toggle back and assert elapsed mode again (`00:00` after the selection). Add `toggle_time_display_with_no_track_keeps_the_placeholder` (a fresh player toggles and still reads `--:--`).
- `README.md`: in the Usage paragraph describing the Now Playing bar, say clicking the large elapsed readout switches it to the track's remaining time (`-02:37`) and back, and that the choice stays until clicked again. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: extend the `## Now Playing label` split-readout bullet with the toggle: the left readout shows elapsed by default, clicking it shows `-MM:SS` remaining (saturating at `-00:00`, `--:--` when the track's length is unknown), the right edge keeps the plain total in both modes, and the mode persists across Pause, Stop, Next/Previous, and a new selection. Name `TimeDisplay`, `views::now_playing_elapsed`, and `format_lcd_remaining` as the enforcers, and add the new tests to the Pinned-by list.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/mod.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The large left readout still shows elapsed `MM:SS` by default, unchanged for every existing position case.
- Pressing the readout flips it to `-MM:SS` remaining and pressing it again flips back; no other control, playback call, track selection, marquee offset, or right-edge total changes.
- Remaining is `total - position`, saturating at `-00:00`; a known track with `duration_ms == 0` and any unknown/absent track read `--:--`.
- The chosen mode survives Pause, Stop, Next/Previous, and selecting another track, and applies to that track's own length.
- The README and `docs/invariants.md` describe the toggle in the same change, and `make check` passes (fmt, clippy, rustdoc, tests).

### Sort the current browse list alphabetically from a Sort button (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Add the library window's `SORT LIB` button from `docs/screenshots/player.png` as a title-case `Sort` button above every browse level, beside `Top`. Pressing it reorders the list currently showing (Artists, Albums, Songs, All Songs, All Albums, or Playlists) alphabetically by each row's rendered text, case-insensitively, leaving every other list and the player state alone. The sort is one-shot like the classic button: it changes only the loaded list in place, so reopening the level fetches its library order again. No persistent sort mode or new dependency is needed.

**Approach.**

- `src/ui/browse.rs`: add `pub(super) fn sort_rows_by(&mut self, key: impl Fn(&T) -> String)` on `BrowseList<T>`, requiring `T: Clone` (the library models derive it). It builds an index permutation (`0..items.len()`) and stable-sorts it by each row's key lowercased with `to_lowercase()`; Rust's stable sort keeps library order for keys that compare equal. Rebuild `items` and, when they are non-empty, `titles` and `labels` by the same permutation so the cached song text stays index-aligned with the reordered songs (the artist/album/playlist lists leave both caches empty). After reordering, bump `epoch` with the same `wrapping_add(1)` `replace_rows` uses, since the row indices a press carries have changed; do not touch `loading`, `error`, or the request generation. A list with fewer than two rows is a no-op (no reorder, no epoch bump), so pressing Sort while a level is loading, empty, or failed changes nothing.
- `src/ui/handlers/browse.rs`: add `pub(in crate::ui) fn sort_browse(player: &mut WinampPlayer) -> Task<Message>`, matching `player.current_view` and calling `sort_rows_by` on that view's list: artists and playlists by `name`, albums (both the per-artist and All Albums lists) by `Album::display_title` (so `artist - title` is the sort key), and songs (both the Songs list, including search results, and All Songs) by `Song::display_title`. Return `Task::none()`; the sort changes no other state, and a reload after navigation restores the fetch's order.
- `src/ui/mod.rs`: add `SortBrowse` to `Message` with a doc comment naming the one-shot reorder and its epoch bump, route it in the `update` dispatch table to `handlers::browse::sort_browse`, and push `views::view_sort_button()` immediately after `views::view_top_button()` so it is present above every browse level.
- `src/ui/views/browse.rs`: add `pub fn view_sort_button() -> Element<'static, Message>` using `labeled_button("Sort", Message::SortBrowse)` beside `view_top_button`, with a doc comment saying it reorders the visible list on press. In `src/ui/views/mod.rs`, export `view_sort_button` and add it to the `labeled_button` doc comment's list of browse buttons.
- Tests in `src/ui/tests.rs` (the browse state and handler tests live there, not in `src/ui/browse.rs`):
  - `sort_reorders_artists_case_insensitively`: drive `Message::ArtistsLoaded` with local `Artist` rows named `"banana"`, `"Apple"`, `"cherry"`, press `Message::SortBrowse`, and assert the names read `Apple`, `banana`, `cherry` — the lowercase key proves the comparison ignores case rather than sorting uppercase first.
  - `sort_reorders_albums_by_their_display_title` and `sort_reorders_playlists_by_name`: load local rows out of order, sort, and assert the rendered display order.
  - `sort_reorders_songs_and_keeps_their_cached_text_aligned`: drive `Message::SongsLoaded(stepping_songs())` (library order One, Two, Three), sort, and assert the ids order One, Three, Two while `songs.titles` and `songs.labels` reorder with them (`"The Sample Band - One"`/`"3:30"`, `"The Sample Band - Three"`/`"1:35"`, `"The Sample Band - Two"`/`"2:05"` in the new order), so no row shows another track's length. Repeat the same assertion for `Message::AllSongsLoaded` over the `all_songs` list.
  - `sort_reorders_all_albums_by_their_display_title`: load local `Album` rows into `all_albums`, sort, and assert their `artist - title` order. This covers the sixth `CurrentView` list, so every browse level has a sort test.
  - `sort_bumps_the_epoch_so_a_pre_sort_press_is_rejected`: capture the artists list's epoch before sorting, sort, then drive `Message::ArtistSelected { epoch: old_epoch, index: 0 }` and assert `player.current_view` is still `CurrentView::Artists`; drive the same press with the new epoch and index and assert it switches to `CurrentView::Albums`. This pins that row indices and the epoch move together.
  - `sort_with_an_empty_or_loading_list_is_a_noop`: press `Message::SortBrowse` on each view before any rows load and assert no panic, no epoch change, and the view stays where it was.
- `README.md`: add to the Usage paragraph that describes the browse lists that a `Sort` button above every list reorders the showing list alphabetically by its row text, case-insensitively, and that reopening the list restores library order. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Browse row order` section after `## Browse reply freshness` stating that Sort reorders only the visible list by its rendered text, keeps equal keys in library order, reorders the cached song titles and labels with the songs, bumps the epoch so a pre-sort press is rejected, and leaves the other lists and playback state unchanged; name `BrowseList::sort_rows_by` and `browse::sort_browse` as the enforcers and cite the new tests.

**Files touched.** `src/ui/browse.rs`, `src/ui/handlers/browse.rs`, `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A `Sort` button is drawn above every browse level beside `Top`; pressing it leaves the current view, current track, playback, search state, and every other list unchanged.
- The showing list is reordered ascending by its rows' rendered text, compared case-insensitively, for Artists, Albums, Songs, All Songs, All Albums, and Playlists; equal keys keep their library order.
- Song lists reorder `items`, `titles`, and `labels` by the same permutation, so each row still shows its own title and duration after a sort.
- A sort bumps the current list's epoch: a `TrackSelected`/`ArtistSelected`/`AlbumSelected`/`PlaylistSelected` press carrying the pre-sort epoch is ignored, while the post-sort indices resolve to the sorted rows.
- Pressing Sort on an empty, loading, or failed list is a no-op that neither reorders rows nor bumps the epoch.
- Navigating away and back reloads library order; no sort flag persists between fetches.
- `make check` passes (fmt, clippy, rustdoc, tests), and the README Usage text and the new `docs/invariants.md` section describe the sort in the same change.

### Add the classic elapsed readout to the Now Playing LCD and restore the plain total at its right edge (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Split the LCD well the way `docs/screenshots/player.png` draws it: a large left-hand elapsed-time readout (`00:53`, zero-padded minutes and seconds) and the plain total length at the right edge. The landed playback-position series already supplies the position, but it currently renders `<elapsed>/<total>` in the right-edge slot — a value the mockup shows nowhere. Move that elapsed value into the big left readout and put the right edge back to the plain total, settling the drift recorded in the steward's 2026-10-22 note. The `O C I D V` strip, the visualizer, and the `256 KBPS 44 KHZ` / `MONO STEREO` status row stay out of scope, as do artist-row album counts.

**Approach.**

- `src/ui/views/now_playing.rs`:
  - Replace `format_position_time` and `now_playing_progress` with `format_lcd_elapsed(position_ms: u64) -> String`, which zero-pads both fields: `0` → `"00:00"`, `53_000` → `"00:53"`, `192_000` → `"03:12"`. Minutes stay unbounded, so an hour reads `"60:00"`. Delete the two replaced helpers and their tests rather than leaving dead public functions.
  - Add `now_playing_elapsed<'a>(tracks, current_track, position: Option<Duration>) -> Cow<'a, str>` beside [`now_playing_time`]: `"--:--"` when `known_track_for` finds nothing for `current_track`; `"00:00"` when the track is known but the backend reports no position (no preview, a failed start, or the silent backend); `Cow::Owned(format_lcd_elapsed(duration_millis(position)))` when a position is reported.
  - Change `now_playing_time` to drop its `position` parameter and always return the cached total (`KnownTrack::time`, borrowed) or `"--:--"` for an unknown/absent track. Update its doc comment to say the right edge is the plain total.
  - Add `LCD_ELAPSED_SIZE` (34) and an `lcd_elapsed_text` helper beside `lcd_text`, using the same `theme::LCD_GREEN`. Change `now_playing_row` to take `elapsed: Cow<'a, str>` as its first argument and push `lcd_elapsed_text(elapsed)` followed by a fixed-width `Space` (12 px) before the existing title line. Reduce `MARQUEE_WIDTH` from 28 to 22 and update its doc comment so the scrolling title line still fits the well beside the new readout; the `display_len`/`needs_marquee` tests key off the constant, so adjust only the comments that hard-code 28.
- `src/ui/mod.rs`: add `WinampPlayer::now_playing_elapsed` borrowing from `views::now_playing_elapsed`; change `WinampPlayer::now_playing_time` to call the two-argument view helper. In `view`, add `now_playing_elapsed` to the state-lock destructuring tuple and pass it as the new first argument to `views::view_now_playing`; update the `now_playing_time` doc comment there.
- Tests. `src/ui/views/tests.rs`: update the `captured_now_playing_texts` helper to take an elapsed string and assert the readout's position in the captured fragments; add `format_lcd_elapsed_pads_minutes_and_seconds` and `now_playing_elapsed_reads_placeholder_zero_or_position` (unknown id → `"--:--"`, known track with `None` → `"00:00"`, `Some(53s)` → `"00:53"`); add `now_playing_time_returns_the_plain_total_even_with_a_position` (the helper no longer takes a position). `src/ui/tests.rs`: drop the position argument from every `player.now_playing_time(...)` call; repoint the elapsed assertions at `player.now_playing_elapsed(...)` (known position → `"00:53"`-style, pause keeps the frozen value, Stop and a new selection read `"00:00"`, stale-epoch replies are still dropped); keep the epoch/poll tests unchanged. Update `now_playing_bar_and_browse_action_buttons_construct` for the new `view_now_playing` arity.
- `README.md`: in the Usage paragraph that describes the Now Playing bar, say the big readout at the left shows the loaded preview's elapsed time and the length at the right is the plain total; remove the elapsed/total sentence. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, replace the `<elapsed>/<total>` guarantee with the split one (left readout `MM:SS`, plain total at the right, `00:00` for a known track without a position, `--:--` with no current track), name `views::now_playing_elapsed`/`format_lcd_elapsed`/`now_playing_time` and the `PlaybackPosition` handler as the enforcers, and cite the new tests plus the existing position-epoch tests.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/mod.rs`, `src/ui/mod.rs`, `src/ui/timers.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`. (`src/ui/views/mod.rs` re-exports the new helper; `src/ui/timers.rs` had a comment calling the old combined readout elapsed/total.)

**Acceptance criteria.**

- The LCD row's first drawn text is the elapsed readout: `00:53` for a 53-second position, `03:12` for 192 seconds, `00:00` for a known track with no reported position, and `--:--` when no track or an unknown id is current.
- The right edge shows the cached plain total (`3:30`) for a known track regardless of any reported position, and `--:--` for no/unknown track; no code path renders `<elapsed>/<total>` and `now_playing_progress`/`format_position_time` no longer exist.
- Pausing keeps the frozen elapsed readout, Stop and a new selection reset it to `00:00`, and a `PlaybackPosition` reply stamped with an older epoch is still dropped (the existing position tests pass unchanged apart from the elapsed accessor).
- `make check` passes (fmt, clippy, rustdoc, tests).
- The README Usage paragraph and the `docs/invariants.md` Now Playing guarantee describe the split readout in the same change.

### Retire the `unclear-invariant` validation gap by pinning subsystem contracts in docs/invariants.md (planned 2026-10-23 by the steward role) — done 2026-10-23 by the feature role

**Goal.** Three Fixed entries (the stale browse reply store-time invariant, the bidirectional browse contract, and the Now Playing title contract) each reconstructed a guarantee that nothing had written down, recorded as `gap: unclear-invariant`. Promote that recurring tag into infrastructure: make `docs/invariants.md` the single, discoverable home for every subsystem contract that a bug has had to reconstruct, so the next bug in those areas finds the invariant instead of rebuilding it, and so the tag stops recurring.

**Approach.**

- Audit `docs/invariants.md` against the three Fixed entries cited below and ensure each reconstructed guarantee is stated as a named, test-cited section (Browse hierarchy, Now Playing label, stale reply store-time window), adding any that are missing.
- Add a short "How to use" note at the top of `docs/invariants.md` pointing readers (and the bugfix role) at it before reconstructing an invariant, and cross-reference it from the BUGS.md header's Validation gap description.
- No code change; this is a docs-only plan whose acceptance is `make check` plus the three cited guarantees each having a section.

**Cited entries.** `BUGS-FIXED.md`: "A browse reply is stored after a newer request was issued …" (unclear-invariant, store-time invariant), "Now Playing bar shows the track id instead of the song title" (unclear-invariant, Now Playing contract), and the bidirectional-browse entry (unclear-invariant, browse hierarchy).

**Files touched.** `docs/invariants.md`, `BUGS.md`.

**Acceptance criteria.**

- `make check` passes.
- Every `unclear-invariant` Fixed entry cited above has a matching section in `docs/invariants.md` naming the enforcing code and the tests that fail if it is removed.
- The BUGS.md header cross-references `docs/invariants.md` for the `unclear-invariant` tag.

### Show each album row's artist beside its title (planned 2026-10-21 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Every album row reads `<artist> - <title>` when the album carries an artist and just the title when it does not, so the per-artist Albums view and the flat All Albums view match the library mockup's album rows (`docs/screenshots/library.png`). The secondary `View Songs` hint, row numbering, the playing-row marker, selection messages, and ordering stay exactly as they are. This is independent of the playback-position series: it touches the album model, the REST album mapping, and browse rows, not the audio seam or the Now Playing bar. Album row running times are out of scope because an album query does not supply its songs' durations.

**Approach.**

- `src/library.rs`: add `pub artist: String` to `Album`, with `#[serde(default)]` and a doc comment saying `""` means the source supplied no artist (the same sentinel as `Song::artist`). Add `pub fn display_title(&self) -> Cow<'_, str>` mirroring `Song::display_title`: borrow the title when the artist is empty, otherwise return an owned `"<artist> - <title>"`. In the tests, add `"artist": "The Sample Band"` to `album_payload` and to the payload in `album_deserialization_ignores_unknown_fields` (which compares against `sample_album`); add an `album_required_fields_payload` that omits the optional artist and point `album_deserialization_rejects_missing_required_fields` at it; add `album_deserialization_defaults_a_missing_artist_to_empty`; and add `album_display_title_joins_the_artist_and_title` plus `album_display_title_borrows_the_title_when_the_artist_is_empty`, mirroring the two `display_title` song tests.
- `src/test_support.rs`: give `sample_album`'s new field `artist: "The Sample Band"`.
- `src/sample_library.rs`: fill the three sample `Album` literals with their owner's name — `"The Sample Band"` for `album-1` and `album-2`, `"Echo Chamber"` for `album-3`.
- `src/apple_music/rest.rs`: in `Resource::into_album`, take `attributes.artist_name` and store it in `Album.artist`, filtering out a blank or whitespace-only name to `""` exactly as `Resource::into_song` does; document that a resource without an artist name maps to the empty sentinel. Both `get_albums_by_artist` and `get_all_albums` already map through this function, so no query changes are needed.
- `src/apple_music/rest/tests.rs`: add `"artistName":"The Sample Band"` to the album stubs in `albums_by_artist_map_library_json_and_set_the_artist_id` and `get_all_albums_maps_the_library_albums_collection`, and update their expected `Album` values (including the one spelled out in `get_all_albums_maps_the_library_albums_collection`) with the mapped artist. Add `get_all_albums_leaves_a_missing_album_artist_empty`, using a stub whose album attributes omit `artistName`, and assert `artist` is `""`.
- `src/apple_music/tests.rs`: update the two signed-in browse expectations that construct an `Album` in `get_albums_by_artist_uses_the_rest_library_when_signed_in` and `get_all_albums_uses_the_rest_library_when_signed_in`; supply `artistName` in their stubs and assert the mapped names so the service-level path is pinned too.
- `src/ui/tests.rs`: add the new field to the `flat_album` literal in `album_selected_in_all_albums_loads_its_songs_and_back_returns_to_all_albums` and the `album-3` literal in `album_selected_fetches_the_albums_songs_into_the_player` (use the sample library's `"Echo Chamber"`); other `Album` values come from `sample_album`.
- `src/ui/views/browse.rs`: change `album_row`'s title from `Cow::Borrowed(album.title.as_str())` to `album.display_title()`, and update its doc to say an album with an artist reads `<artist> - <title>` while an artist-less one reads the title alone.
- `src/ui/views/tests.rs`: update `album_row_uses_title_and_selects_the_album_by_index` to expect `"The Sample Band - First Record"`; add `album_row_shows_the_title_alone_when_the_artist_is_empty`, blanking `sample_album().artist` and asserting the title is the bare `"First Record"` and borrows.
- `README.md`: extend the Usage sentence that describes browse rows so it also says album rows read `<artist> - <title>`, falling back to the title when the source supplies no artist. Keep the diff to the Usage paragraph; do not touch the `<!-- tumwater:status:start -->` block.
- Completion: move this entry from `PLANS.md` `## Planned` to `PLANS-DONE.md` `## Done` with a `done <landing date>` stamp.

**Files touched.** `src/library.rs`, `src/test_support.rs`, `src/sample_library.rs`, `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music/tests.rs`, `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, plus the `PLANS.md`/`PLANS-DONE.md` move on completion.

**Acceptance criteria.**

- `make check` passes.
- `Album::display_title` returns `"The Sample Band - First Record"` for the sample album, borrows the title alone for an album whose artist is empty, and never emits a dangling `" - "` prefix.
- `album_row` returns the combined artist/title for both the per-artist Albums list and the All Albums list, since both build through `album_row`, and returns the bare title when the artist is empty; the secondary label and selection message are unchanged.
- `Resource::into_album` maps a non-blank `artistName` into `Album.artist`, and a missing or blank one leaves `""`; the two signed-in browse tests and the two REST mapping tests show the name reaching the album returned to the UI.
- `Album` round-trips with the new field, still tolerates unknown fields, and its missing-required-field probe still rejects a payload without an id, title, or artist id while a missing artist defaults to `""`.
- The README change is confined to the Usage paragraph, and the status block is byte-for-byte unchanged.
- The implementation changes only the ten listed files plus the `PLANS.md`/`PLANS-DONE.md` move, and the commit body names them consistently with `git diff --stat`.

### Add the playback-position readout to the Now Playing bar (Playback position, part 3/3: planned 2026-10-21 by the plan role, requires part 2/3 landed) — done 2026-10-22 by the feature role

**Goal.** The Now Playing bar's right-hand readout shows the current track's elapsed and total time as `"<elapsed>/<total>"` (for example `0:45/3:12`) whenever the audio backend reports a loaded preview's position, and falls back to the existing plain total (`"--:--"` when no track is current) when it does not. Pause leaves the stored value and the paused player's position in place, a seek is reflected by the next position reply, Stop and a new selection clear the readout, and a play whose preview fails to install leaves the plain total rather than a frozen `0:00`.

This is the second half of the former part 2/2 and consumes part 2/3's authoritative `AudioOutput::position`: the backend answers `None` when no preview is loaded, so the UI keeps no playback guard and a selected track with no preview URL never turns one on.

**Approach.**

- `src/apple_music/playback.rs`: update `playback_position`'s doc to say it reports the loaded preview's position and `None` when no preview is loaded or the backend cannot answer, and remove its `#[allow(dead_code)]` now that the UI calls it. `play_track` also stops the backend's previously loaded preview when the selected song has no preview URL, so the old track neither keeps sounding nor keeps answering the position query under the new track's name; `transport.loaded` then records that no preview started.
- `src/apple_music/tests.rs`: replace `play_track_without_a_preview_url_commits_state_without_audio` with `play_track_without_a_preview_url_stops_the_previous_preview`, which plays a preview first and then selects a song without one, and pins the backend call sequence (`Stop`) and the committed state; `pause_pauses_the_audio_backend` now expects that `Stop` before its `Pause`.
- `src/ui/views/now_playing.rs`: add `pub fn format_position_time(position_ms: u64) -> String`, calling the existing `format_minutes_seconds` (which already yields `"0:00"` for zero and keeps minutes unbounded). Add `pub fn now_playing_progress(position_ms: u64, total: &str) -> String` returning `format!("{}/{}", format_position_time(position_ms), total)`. Change `now_playing_time` to take `position: Option<Duration>`: when `current_track` is in `tracks` and `position` is `Some`, return `Cow::Owned(now_playing_progress(...))` with the duration converted saturating to milliseconds; otherwise keep today's borrowed `KnownTrack::time` or `"--:--"` fallback. Update its doc comment to say the combined case allocates.
- `src/ui/mod.rs`: add `playback_position: Option<Duration>` and `position_epoch: u64` to `WinampPlayer`, initialized `None`/`0` in `new`, and define `POSITION_POLL_INTERVAL` (`Duration::from_secs(1)`) with a doc note that it bounds the readout refresh, not the audio. Add `Message::PositionTick` and `Message::PlaybackPosition { epoch: u64, position: Option<Duration> }`, and route both from the `update` dispatch table to the two handlers in `src/ui/handlers/playback.rs` below. This app does not enable iced's timed-subscription backend (see the `marquee_ticks` note), so add `position_ticks`/`position_tick_stream` beside it, feeding a one-slot channel from a std thread once per `POSITION_POLL_INTERVAL`, and batch that subscription in `init_ui` with `keyboard::listen().filter_map(message_for)` and `marquee_ticks()`. `WinampPlayer::now_playing_time` keeps its signature but passes `self.playback_position` to `views::now_playing_time`; the `view` call site is unchanged, though its comment that all three readouts always borrow must note the combined elapsed case allocates.
- `src/ui/handlers/playback.rs`: the two handlers the table routes to — `position_tick` and `playback_position` — plus the readout reset and the transition counter. `position_tick` captures `player.position_epoch`, clones the service, and returns `Task::perform(async move { service.playback_position() }, move |position| Message::PlaybackPosition { epoch, position })`; `playback_position` stores `position` (including `None`) only when `epoch == player.position_epoch` and no transport transition is outstanding, dropping a reply issued before a transition. Add `fn reset_position_readout(player: &mut WinampPlayer)` that bumps `position_epoch` and sets `playback_position = None`; call it after the song resolves in `track_selected` and at the start of `stop` (which becomes a block returning the existing `transport_into` call). `pending_position_transitions: u64` on `WinampPlayer` counts the transitions in flight: every handler that schedules a backend transport (Play, Pause, Play/Play-Pause, Stop, and a selected track's play) begins one, and the completion that reports that same transition (`TransportSettled` for the transports, `TrackPlayed` for the play) ends it, so the count is nonzero exactly while some transition is in flight. While the count is nonzero the poll issues no query and no reply is stored, so a poll cannot query the previous player under the new epoch and a completion belonging to another transition cannot close that window early. Pause leaves the field alone, so the next poll keeps showing the paused player's frozen position.
- `src/ui/views/tests.rs`: update the `now_playing_time_resolves_known_track_to_cached_label` call sites to pass `None` as the position; extend it (or add `now_playing_time_shows_elapsed_over_total_when_a_position_is_known`) so `now_playing_time(&tracks, Some("song-1"), Some(Duration::from_secs(192)))` reads `"3:12/3:30"` and is `Cow::Owned`; add `format_position_time_formats_elapsed_and_zero` (`0`→`"0:00"`, `192_000`→`"3:12"`, `65_000`→`"1:05"`) and `now_playing_progress_joins_elapsed_and_total` (`(192_000, "12:38")`→`"3:12/12:38"`).
- `src/ui/tests.rs`: keep `now_playing_time_shows_the_played_tracks_duration_after_browsing_to_another_album` asserting the plain total (its calls now use the unchanged `WinampPlayer::now_playing_time` signature); add `now_playing_time_shows_elapsed_over_total_while_a_position_is_known` (reuse `play_song_1_then_browse_to_album_2`, set `player.playback_position = Some(Duration::from_secs(45))`, assert `"0:45/3:30"`), `pause_keeps_the_stored_elapsed_readout` (set a known position, drive `Message::Pause`, then assert the field still holds that position and the bar still reads `"0:45/3:30"`; together with part 2/3's `a_pause_keeps_the_loaded_player_reporting_its_position` this pins that pause freezes rather than clears), `a_position_reply_after_a_seek_updates_the_readout` (set `Some(45)`, drive `Message::SeekForward`, feed `Message::PlaybackPosition` with `Some(50)`, assert `"0:50/3:30"`; the existing `seek_messages_drive_the_audio_backend` pins the backend call), `track_selected_clears_the_elapsed_readout`, `stop_clears_the_elapsed_readout`, `position_replies_store_and_stale_ones_are_dropped` (store a reply at the current epoch, settle the Stop's transition, then drive a reply stamped with the pre-`Stop` epoch and assert it does not overwrite), `position_poll_waits_for_a_stop_to_reach_the_backend` and `position_poll_waits_for_a_selected_tracks_play_to_settle` (the poll issues no query while the reset's transition is outstanding and resumes once its completion lands), `an_unrelated_transport_completion_keeps_the_poll_waiting` (a Pause that settles while a Stop is outstanding keeps the poll waiting), and `position_tick_schedules_a_query` (`assert_message_schedules_work`); the shared `play_song_1_then_browse_to_album_2` fixture now delivers the selection's `TrackPlayed` completion so the transition counter settles.
- `README.md`: extend only the Usage sentence that describes the Now Playing bar's artist and title so it says the length reads as elapsed/total while a preview is loaded and as the plain length otherwise. Do not edit the `<!-- tumwater:status:start -->` block or add archive narrative anywhere; that block is state-only (see `762af8c`), and the README diff must be confined to the Usage paragraph.
- `docs/invariants.md`: in the Now Playing label section, add the time-readout guarantee (elapsed/total only while a loaded preview position is known, the cached total otherwise, `--:--` with no current track), its enforcement by `views::now_playing_time`/`now_playing_progress` and the `PlaybackPosition` handler, and cite the new tests plus part 2/3's audio-seam `position_query_reports_none_when_nothing_is_loaded`, `a_seek_is_reflected_by_the_next_position_query`, and `a_pause_keeps_the_loaded_player_reporting_its_position`.

**Files touched.** `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes.
- `format_position_time` returns `"0:00"`, `"3:12"`, and `"1:05"` for zero, 192000 ms, and 65000 ms; `now_playing_progress(192_000, "12:38")` returns `"3:12/12:38"`.
- With a known current track and a `Some` position, the bar renders `elapsed/total`; with `None`, the plain total; with no current track or an unknown id, `"--:--"`.
- Pause does not clear the stored position and the next poll still reads the paused player, so the readout keeps the frozen value; `a_position_reply_after_a_seek_updates_the_readout` shows a seek's new position reaching the bar on the next reply; Stop and a new selection clear the readout immediately through `reset_position_readout`. A failed preview start leaves `None`, so the bar falls back to the plain total.
- A `PlaybackPosition` reply stamped with an older `position_epoch` is dropped, so a poll in flight across a Stop or selection cannot restore the previous elapsed value.
- The poll issues no query while a backend transport transition is in flight, and only the completion that reports that transition releases it, so a poll cannot query the previous player under the new epoch.
- A selection whose song has no preview URL stops the backend's previous preview, so the position query answers `None` for the new track and the bar falls back to the plain total.
- The poll is an iced subscription emitting `PositionTick` once per `POSITION_POLL_INTERVAL` (via the app's std-thread tick source, since `iced::time::every` is unavailable), and the position query runs in a task, not in `update`.
- The README diff is confined to the Usage sentence; the status block is byte-for-byte unchanged.
- The change touches only the nine listed files plus the `PLANS.md` move.

### Split the `update` loop into one handler module per concern (landed 2026-10-22; no plan-role entry — rescued manually from a polluted main)

**Goal.** `src/ui/mod.rs` had grown to 1319 lines, with `update` holding every message's behaviour inline and eight helpers shared across arms. Each concern's behaviour now lives in its own module under `src/ui/handlers/` — `playback` (the transport buttons and keys, the track stepping, the play request and its completion, the marquee tick), `mixer` (the volume, the balance, the seek, and the equalizer), `browse` (the hierarchy, the library search, and the fetch results), and `window` (the resolved window id and the title bar's actions) — and `update` is a dispatch table that routes each `Message` variant to its handler and carries no behaviour of its own.

**Approach.**

- `src/ui/mod.rs`: declare `mod handlers;`, drop the imports the moved code took with it (`EqSnapshot`, the `loading::{...}` items, `Ordering`), delete the inline arms and the helpers they shared (`step_track`, `mutate_state`, `mutate_volume`, `mutate_balance`, `seek_preview`, `mutate_equalizer`, `scroll_browse_to_top`, `fetch_at_top`, `with_window_id`, `resize_to_shade`), and rewrite `update` as one delegation per variant.
- `src/ui/handlers/`: one module per concern, each holding its handlers, its shared helpers, and the rationale the inline comments carried, so a change to one concern never reaches into another's code.
- `src/ui/tests.rs`: take `RequestGeneration` and `play_into` from `loading` directly, since `update` no longer re-exports them.
- `docs/invariants.md`: the Browse hierarchy and Browse reply freshness sections now name the handlers in `src/ui/handlers/browse.rs` rather than arms in `src/ui/mod.rs`.

**Files touched.** `src/ui/mod.rs`, `src/ui/handlers/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/handlers/mixer.rs`, `src/ui/handlers/browse.rs`, `src/ui/handlers/window.rs`, `src/ui/tests.rs`, `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes: formatting, clippy with `-D warnings`, rustdoc, and the test suite.
- Every `Message` variant has exactly one dispatch row, and no row holds behaviour.
- The moved logic has one definition: no helper or arm body survives in `src/ui/mod.rs`.
- The existing UI tests pass unchanged apart from the two `loading` imports `src/ui/tests.rs` now takes directly.

### Add the playback-position readout to the Now Playing bar (Playback position, part 2/3: planned 2026-10-21 by the plan role, requires part 1/2 landed, done 2026-10-21)

**Goal.** The audio seam answers a position only while a preview is loaded: `AudioOutput::position` returns `Some(position)` when a player is installed (including `Some(Duration::ZERO)` at the start of a preview) and `None` when no player is loaded or the backend cannot answer. This is the first half of the former part 2/2, split so the seam contract and its Pause/seek behavior are reviewed apart from the Now Playing readout that consumes them. The silent backend keeps the trait's default `None`, so a machine with no audio device shows no elapsed value rather than a frozen `0:00`.

This revision leaves the seam approach as it stands, since no recorded review objection names the code path, and fixes the scope wording that produced a commit body contradicting its own diff: finishing a plan also moves its entry from `PLANS.md` to `PLANS-DONE.md`, so that move is part of this change and must be reported as such rather than denied in RISK or VERIFIED.

**Approach.**

- `src/audio.rs`: revise `AudioOutput::position`'s contract and doc to "`Some` while a player is loaded (including `Duration::ZERO` at its start), `None` when no player is loaded or the backend cannot answer"; the default stays `None`. `SilentOutput` needs no method because the default already answers `None`.
- `src/audio/player.rs`: change `Command::Position` to carry `Sender<Option<Duration>>` and update its doc to say `None` when no player is loaded. In `serve_commands`, reply with `player.as_ref().map(WorkerPlayer::position)`. In `RodioOutput::position`, return `replies.recv_timeout(POSITION_QUERY_TIMEOUT).ok().flatten()`, so a closed worker or timed-out reply is `None`.
- `src/audio/tests.rs`: update `rodio_output_asks_the_worker_for_its_position` to send `Some(Duration::from_secs(3))`; make `position_query_returns_the_loaded_players_position` expect `Some(Duration::from_secs(42))`; rename `position_query_returns_zero_when_nothing_is_loaded` to `position_query_reports_none_when_nothing_is_loaded`, expect `None`, and update its comment from "answers zero" to "reports `None`". The timeout and closed-worker tests already expect `None`.
- `src/audio/tests.rs`: make `RecordingPlayer::seek_by` record the offset and make its `position` account for the recorded offsets via `seek_target(Duration::from_secs(42), ...)`, so the fake models a player whose clock moves on a seek while its no-seek position stays `42s` for the existing test. Add `a_seek_is_reflected_by_the_next_position_query` (Play, `SeekBy(5)`, `Position` replies `Some(47)`) and `a_pause_keeps_the_loaded_player_reporting_its_position` (Play, `Pause`, `Position` still answers `Some(42)`, the player was paused, and it was not stopped). The pause test pins the worker's half of the guarantee: Pause reaches the player and the worker keeps it installed, so the next poll reads the player's own frozen clock; the fake has no advancing clock, so it cannot itself prove rodio's clock stops.
- `src/audio/tests.rs`: add `a_failed_preview_leaves_no_position_to_report`. Its `create_player` factory, built like `recording_players` with a `Mutex<Option<...>>` so it queues only on its first call and stays `Fn`, sends `Command::Play("second")` and then `Command::Position(reply)` through the test's sender; its `fetch` succeeds for `"first"` and fails for `"second"`. The factory drops its sender clone and the test drops its own before `serve_commands`, so the loop ends after the query. The failed second `Play` drops the player, so `replies.recv()` is `None`.
- Completion: move this entry from `PLANS.md` `## Planned` to `PLANS-DONE.md` `## Done` under its part 2/3 heading with a `done <landing date>` stamp, leaving part 3/3 in `## Planned`.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, plus `PLANS.md` and `PLANS-DONE.md` (this entry moved to Done).

**Acceptance criteria.**

- `make check` passes.
- A loaded player makes `Command::Position` answer `Some(player.position())`; no player answers `None`; a closed worker and a timed-out reply answer `None`; the silent backend answers `None`.
- `a_seek_is_reflected_by_the_next_position_query` shows the fake player's reported position moving by the seek (`42s` to `47s`), and `a_pause_keeps_the_loaded_player_reporting_its_position` shows the paused player still installed and reporting its position.
- A failed asynchronous preview start leaves no player, so the following position query answers `None`.
- The implementation changes exactly `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `PLANS.md`, and `PLANS-DONE.md` (this entry moved to `## Done`), and no other file. The commit body's SUMMARY, RISK, and VERIFIED name all five files and agree with `git diff --stat`; none claims `PLANS-DONE.md` is untouched.

### Scroll an overlong Now Playing title across the LCD (planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** The brief's mockup says "The song title scrolls across the display": when the Now Playing bar's combined `artist - title` line is wider than the LCD, it scrolls one character per tick and wraps through a short gap, so the full track name is readable instead of running under the time readout. A line that fits stays still and keeps the existing zero-allocation rendering. This is independent of the playback-position series: it reads only the label and artist the bar already resolves, and touches neither the audio seam nor the time readout.

**Approach.**

- `src/ui/views/now_playing.rs`: add `pub(super) const MARQUEE_WIDTH: usize = 28` — a documented character budget sized to keep the line inside the roughly 550px-wide window beside the `"Now Playing: "` caption and the time — and `const MARQUEE_GAP: usize = 3` (the spaces between repeats). Add `pub(super) fn marquee_text(text: &str, offset: usize, width: usize) -> String`: count characters (not bytes), return `text.to_string()` unchanged when it fits `width`, otherwise walk a cycle of `text` plus `MARQUEE_GAP` spaces for `width` characters starting at `offset % cycle`, substituting a space past the text's end. The pure helper is called only by the overlong branch, so its allocation belongs to the scrolling path.
- `src/ui/views/now_playing.rs`: change `view_now_playing` to take `marquee_offset: usize` as a fourth argument. Compute the display length as `label.chars().count()` plus `artist.chars().count() + 3` when the artist is non-empty. When it fits `MARQUEE_WIDTH`, keep the current structure — the separate artist, `" - "`, and label widgets, all borrowed, plus the `Space` fill and time. When it does not fit, build `artist - label` (or the label alone when the artist is empty) and render it as one `Text` of `marquee_text(&display, marquee_offset, MARQUEE_WIDTH)` in `theme::LCD_GREEN`, keeping the `Space` fill and time at the right edge. Update the doc comment to describe the scrolling branch and the character-budget approximation.
- `src/ui/mod.rs`: add `marquee_offset: usize` to `WinampPlayer`, initialized `0` in `WinampPlayer::new`. Add `Message::MarqueeTick` with a doc comment saying it advances the title window; its `update` arm sets `player.marquee_offset = player.marquee_offset.wrapping_add(1)` and returns `Task::none()`. In the `TrackSelected` arm, set `player.marquee_offset = 0` beside the `known_tracks` entry, so a new song starts at the beginning of its title. Add `const MARQUEE_TICK: std::time::Duration = std::time::Duration::from_millis(250)` near `init_ui` (import `Duration` from `std::time`). Change `init_ui`'s subscription to `iced::Subscription::batch([...])` of the existing `keyboard::listen().filter_map(message_for)` and `marquee_ticks()`, a helper that feeds ticks from one std thread through the futures channel iced re-exports (iced's `time::every` needs a `smol`/`tokio` backend feature the app does not enable, and its default thread-pool backend's `time` module is empty); `marquee_ticks` is a thin wrapper over `marquee_tick_stream()`, whose one-slot channel drops a tick when the UI is behind but stops the thread only when the receiver is gone, so a slow poll cannot end the scrolling. The timer runs unconditionally, and the view ignores the offset whenever the line fits. Pass `player.marquee_offset` to `views::view_now_playing` in `view`.
- `src/ui/views/tests.rs`: add `marquee_text_keeps_a_line_that_fits` (`marquee_text("Opening", 9, 28) == "Opening"`), `marquee_text_windows_from_the_offset` (`"The Sample Band - Opening"`, width 10: offset 0 is `"The Sample"`, offset 2 is `"e Sample B"`), `marquee_text_wraps_through_the_gap_to_the_start` (offset 25 at width 10 is `"   The Sam"`), and `marquee_text_counts_characters_not_bytes` (`marquee_text("Café — Ok", 0, 5) == "Café "`). Add `overlong_now_playing_line_renders_a_new_window_as_the_offset_advances`: lay the row out with iced's null renderer, capture the text widgets through a widget `Operation`, and assert the overlong line contains the window for offsets 0 and 1 (which differ) while a fitting line captures the same separate artist and title at every offset. Extend `now_playing_bar_and_browse_action_buttons_construct` to pass a `marquee_offset` to both `view_now_playing` calls and add one overlong-label construction that exercises the scrolling branch.
- `src/ui/tests.rs`: add `marquee_tick_advances_the_offset` (two `Message::MarqueeTick` updates take `marquee_offset` from 0 to 2), `marquee_ticks_keep_ticking_when_the_receiver_falls_behind` (queue the two messages the channel can hold, sleep through several more tick periods so the sender attempts a send against the full channel, then read a third tick, proving a full channel drops a tick instead of stopping the timer), and `track_selected_resets_the_marquee_offset` (load `stepping_songs()`, set the offset to 7, drive `TrackSelected { epoch: 1, index: 0 }`, assert 0).
- `README.md`: extend only the Usage sentence that describes the Now Playing bar's artist and title so it also says the line scrolls when it is longer than the display. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the Now Playing label section, add the scrolling guarantee: a combined artist/title longer than `MARQUEE_WIDTH` characters is windowed by `views::now_playing::marquee_text`, advanced one character per `Message::MarqueeTick`, reset to the start by `TrackSelected`, and shown whole-and-borrowed when it fits. Cite the marquee tests in `src/ui/views/tests.rs` and the tick/reset tests in `src/ui/tests.rs`.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`, plus the `PLANS.md`/`PLANS-DONE.md` move.

**Acceptance criteria.**

- `make check` passes.
- `marquee_text` returns the input unchanged for a line that fits; for `"The Sample Band - Opening"` at width 10 it returns `"The Sample"` at offset 0, `"e Sample B"` at offset 2, and `"   The Sam"` at offset 25; `marquee_text("Café — Ok", 0, 5)` is `"Café "`, proving character-count indexing.
- A label plus artist within `MARQUEE_WIDTH` renders through the existing separate artist/title widgets and stays still at every offset; a longer one renders the single windowed `Text` and advances with the offset, while the time stays at the right edge. The row-rendering test captures those text widgets and asserts the different windows.
- `Message::MarqueeTick` advances `marquee_offset` by one with no task, `marquee_ticks` emits one per `MARQUEE_TICK` alongside the unchanged key bindings and keeps ticking after a delayed receiver, and a `TrackSelected` resets the offset to 0 so a new song starts at character 0.
- The README diff is confined to the Usage sentence; the status block is byte-for-byte unchanged.
- The change touches only the six listed source/docs files plus the `PLANS.md`/`PLANS-DONE.md` move, and adds no dependency.

### Show each song row's artist and title in every browse list (planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** Every song row in the Songs view, All Songs, and search results reads `<artist> - <title>` when the song carries an artist (for example `The Sample Band - Opening`), as the library mockup's playlist rows do, and reads just the title when the artist is empty. Row numbering, the playing-row marker, the length label, selection messages, and ordering stay exactly as they are. This is independent of the pending playback-position plan: it touches the browse rows, not the Now Playing bar or the audio seam.

**Approach.**

- `src/library.rs`: add an inherent `Song::display_title(&self) -> Cow<'_, str>` (import `std::borrow::Cow`). It returns `Cow::Borrowed(&self.title)` when `self.artist.is_empty()` and `Cow::Owned(format!("{} - {}", self.artist, self.title))` otherwise. This is presentation-only; the serialized model is unchanged. Add a test asserting `sample_song().display_title()` is `"The Sample Band - Opening"`, that a clone with `artist = String::new()` yields the borrowed `"Opening"`, and that the joined case is owned.
- `src/ui/browse.rs`: add a `titles: Vec<String>` cache to `BrowseList<T>`, documented beside `labels` as the per-row primary title cached when the list is stored so a view refresh does not re-join every artist and title. Rename `store_with_labels` to `store_with_row_text(items, title, label)`, taking two closures: the title closure fills `titles` and the label closure fills `labels`, both in row order. `store` clears both caches; `clear` and `fail` drop both with the rows; `replace_rows` leaves both to the caller. Update the module doc and every existing `store_with_labels` reference in docs/comments to the new name.
- `src/ui/mod.rs`: in the `SongsLoaded` and `AllSongsLoaded` arms call `store_with_row_text` with `|song| song.display_title().into_owned()` for the title and the existing `|song| views::format_track_time(song.duration_ms)` for the length. Pass `&player.songs.titles` / `&player.all_songs.titles` to `views::view_songs` / `views::view_all_songs` in `view`.
- `src/ui/views/browse.rs`: change `scrollable_list`'s item title from `&'a str` to `Cow<'a, str>` so a row can borrow a cached title; wrap the artist, album, and playlist row titles in `Cow::Borrowed`. Change `song_row` to take a `cached_title: Option<&'a str>` beside the existing cached length and return the row title as a `Cow`: `Cow::Borrowed(cached_title)` when the cache has one, otherwise `song.display_title()`. Thread a `titles: &'a [String]` slice through `song_list`, `view_songs`, and `view_all_songs`, pairing each song with `titles.get(index).map(String::as_str)` exactly as the length labels are paired now. Update the affected doc comments, including why the cache exists and that an index without a cached title falls back to composing it.
- `src/ui/views/tests.rs`: update every `song_row` call for the new `cached_title` argument and `view_songs`/`view_all_songs` calls for the `titles` slice. Change `song_row_uses_title_and_selects_the_song_by_index` to expect `"The Sample Band - Opening"` from the fallback, add `song_row_prefers_the_lists_cached_title` (a cached value that differs from the song fields must win), and add `song_row_uses_the_title_alone_when_the_artist_is_empty`. Keep the existing length-cache, unknown-duration, current-marker, and construct-every-view tests passing with the new argument order.
- `src/ui/tests.rs`: extend the song-list cache tests to assert `player.songs.titles` and `player.all_songs.titles` are built in row order from `display_title` (for example `["The Sample Band - One", "The Sample Band - Two", "The Sample Band - Three"]`), and that a navigation's `clear` drops the titles together with the length labels. Rename the cache test so its comment names both caches if needed.
- `README.md`: extend the Usage sentence that says song rows show each track's length so it also says song rows read `artist - title` (or the title alone when the source supplies no artist).
- `docs/invariants.md`: update the browse reply-freshness guarantee's "Enforced by" reference from `store_with_labels` to the renamed `store_with_row_text`; the guarantees themselves do not change.

**Files touched.** `src/library.rs`, `src/ui/browse.rs`, `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes.
- `Song::display_title` returns `"The Sample Band - Opening"` for the sample song and the borrowed title `"Opening"` when the artist is empty.
- `SongsLoaded` and `AllSongsLoaded` fill both `titles` and `labels` in row order; `store`, `clear`, and `fail` drop both caches, so a refresh cannot pair a new row with a stale title or length.
- Every song row renders the cached display title when present and `Song::display_title` otherwise; a cached title wins over the song's own fields. The length label, message payload, epoch, numbering, playing marker, and list order are unchanged.
- Songs, All Songs, and search results all render through the same `song_list`, so each shows `<artist> - <title>` and an artist-less song shows only its title.
- The change touches only the eight listed files plus this backlog move.

### Add the library window's Top button and open every browse list at the top (planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** The browse list gains the mockup's `TOP` control: a `Top` button above the list returns the current list to its first row, and opening a browse level — drill-down, flat-list button, search, or Back — starts that newly shown list at the top instead of leaving it at whatever offset the previous list had. This is independent of the playback-position series: it touches no audio code and no Now Playing state.

**Approach.**

- `src/ui/views/browse.rs`: define `pub const BROWSE_LIST_ID: &str = "nostalgia-browse-list";` with a doc comment naming it as the one scrollable's id so the update loop's scroll operation can reach it. In `scrollable_list`, add `.id(BROWSE_LIST_ID)` to the `Scrollable` before its style, leaving the rows, placeholder, and height unchanged. Add `pub fn view_top_button() -> Element<'static, Message>` returning `labeled_button("Top", Message::ScrollBrowseToTop).into()`.
- `src/ui/views/mod.rs`: re-export `BROWSE_LIST_ID` and `view_top_button` from `browse` beside `view_back_button` and `view_retry_button`.
- `src/ui/mod.rs`: add `Message::ScrollBrowseToTop`. Add a private helper `fn scroll_browse_to_top() -> Task<Message>` returning `iced::widget::operation::snap_to(views::BROWSE_LIST_ID, iced::widget::scrollable::RelativeOffset::START)`. Handle the new message by returning that helper. Add a private helper `fn fetch_at_top(task: Task<Message>) -> Task<Message>` returning `Task::batch([task, scroll_browse_to_top()])`, and wrap the `fetch_level(...)` result in the `ArtistSelected`, `AlbumSelected`, `PlaylistSelected`, `ShowAllSongs`, `ShowAllAlbums`, `ShowPlaylists`, and `SearchSubmitted` arms. In `Message::Back`, return `scroll_browse_to_top()` after the existing view change so stepping up also starts the restored list at the top.
- `src/ui/mod.rs` `view`: push `views::view_top_button()` above the browse content for every `CurrentView` — after the Back/Retry buttons and before the Artists-only flat-list buttons and the search box — so the mockup's `TOP` control is available at each browse level.
- `src/ui/views/tests.rs`: extend `now_playing_bar_and_browse_action_buttons_construct` with `let _top = view_top_button();`.
- `src/ui/tests.rs`: add `scroll_browse_to_top_schedules_a_scroll_task`, driving `Message::ScrollBrowseToTop` through `assert_message_schedules_work` and asserting `assert_view` and the shared state are unchanged; add `back_schedules_the_restored_list_to_the_top`, starting in Albums and asserting `Message::Back` both returns to Artists and schedules follow-up work (the scroll task).
- `README.md`: extend the Usage paragraph that describes the browse buttons with one sentence: a `Top` button above every browse list returns it to the first row, and opening a list starts it at the top.

**Files touched.** `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `Message::ScrollBrowseToTop` schedules an iced `snap_to` task for `views::BROWSE_LIST_ID` at `RelativeOffset::START`, and driving it through `update` changes neither `current_view` nor shared state.
- The `Top` button is built into the browse area for all six `CurrentView` variants and is absent in shade mode, which returns before the browse content is assembled.
- Every navigation arm that replaces the visible list batches the snap-to-top task with its fetch, and `Message::Back` schedules it after stepping up, so a list opened after a navigation starts at its first row.
- Scrolling a list manually still works between navigations; the id is stable while the same list is displayed.
- The change touches only the six listed files plus this `PLANS.md` edit; no audio, `apple_music`, or `docs/invariants.md` changes, because the browse hierarchy guarantees are unchanged.

### Add the playback-position readout to the Now Playing bar (Playback position, part 1/2: planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** The audio seam can report where the loaded preview is, so the Now Playing bar can later show elapsed against total. `AudioOutput::position()` answers `Option<Duration>`: `Some` with the loaded player's position (including `Some(Duration::ZERO)` when the worker has no player), or `None` when the backend cannot answer (the silent fallback, a default test stub, or a query the worker cannot reach within the seam's bound). `AppleMusicService::playback_position()` forwards that value. This part changes no UI; it gives part 2/2 a narrow, testable source for the elapsed readout.

**Approach.**

- `src/audio.rs`: import `std::time::Duration` and add a provided trait method after `seek_by`: `fn position(&self) -> Option<Duration> { None }`. Its doc says it returns the loaded preview's position, `Some(Duration::ZERO)` when the backend has no loaded player, and `None` when the backend cannot answer; the default keeps `SilentOutput` and every existing test fake compiling without a position of their own.
- `src/audio/player.rs`: add `Command::Position(std::sync::mpsc::Sender<Duration>)`. Add `fn position(&self) -> Duration` to `WorkerPlayer`, with `RodioPlayer` returning `self.player.get_pos()` (the same source `seek_player` already reads). In `serve_commands`, handle `Command::Position(reply)` by sending `player.as_ref().map_or(Duration::ZERO, WorkerPlayer::position)`; because the loop handles the command in channel order, a position query sent during a preview download is answered after that download installs the new player. In `impl AudioOutput for RodioOutput`, add `position`: send `Command::Position` through the existing `send`, then wait on the reply with `recv_timeout`, returning `None` when the send fails or the wait times out. Define `POSITION_QUERY_TIMEOUT` next to `DEVICE_OPEN_TIMEOUT` (100 ms) and document that a download can hold the worker for up to `preview::PREVIEW_TIMEOUT`, so the query must not block the caller indefinitely.
- `src/apple_music.rs`: add `pub(crate) fn playback_position(&self) -> Option<Duration> { self.audio.position() }` beside `seek_output`, documented as synchronous, touching no shared state, and bounded by the seam's query timeout.
- `src/audio/tests.rs`: give `RecordingPlayer` a `position` implementation returning a fixed non-zero `Duration`, then add `position_query_returns_the_loaded_players_position` (send `Command::Position` after a `Play`, assert the reply equals the fake's position) and `position_query_returns_zero_when_nothing_is_loaded` (send `Command::Position` with no player). Add `rodio_output_asks_the_worker_for_its_position` (a helper thread reads `Command::Position` from the command channel and replies, while the test calls `position()`) and extend `rodio_output_reports_a_closed_worker_for_every_command` with `position() == None`.
- `src/test_support.rs`: override `position` for `RecordingAudio` to return `Some(Duration::from_secs(7))`, documented as a fixed stub position; no other call recording changes.
- `src/apple_music/tests.rs`: add `playback_position_forwards_the_audio_output_position`, building the service with `Arc::new(RecordingAudio::default())` through `with_audio` and asserting the service returns `Some(Duration::from_secs(7))`.
- `docs/invariants.md`: unchanged; this part adds a seam, not a user-visible guarantee.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`.

**Acceptance criteria.**

- `make check` passes.
- With a player loaded, `serve_commands` answers `Command::Position` with that player's `position()`; with no player it answers `Duration::ZERO`.
- `RodioOutput::position` returns the reply the worker sends, and returns `None` when the command channel is closed or no reply arrives within `POSITION_QUERY_TIMEOUT`.
- `AppleMusicService::playback_position` returns exactly the injected backend's `position()` value.
- `SilentOutput` and all existing `AudioOutput` test fakes keep compiling without adding a method, because the trait method has a `None` default.
- The change touches only the six listed files plus this `PLANS.md` edit; no UI, README, or invariant text changes.

### Show the item count beneath artist, album, and playlist lists (planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** Every loaded non-song browse list shows how many rows it holds beneath the rows — `3 artists`, `2 albums`, `1 playlist` — as the library mockup's status area does (`docs/screenshots/library.png`). The song lists keep their existing count-and-time footer and gain nothing new. This is independent of the pending row numbering: it reads only the slice length each view already receives.

**Approach.**

- `src/ui/views/browse.rs`: add a pure `pub(super) fn list_count_summary(count: usize, singular: &str, plural: &str) -> String` returning `"<count> <noun>"`, with `singular` chosen exactly when `count == 1` and `plural` otherwise. It formats any count, including `0`, so the wording is testable without an iced renderer (`format_minutes_seconds` is the shared formatter for the song footer).
- Add a private `fn list_with_count<'a>(list: Element<'a, Message>, count: usize, singular: &str, plural: &str) -> Element<'a, Message>`: when `count` is non-zero, wrap `list` in a `Column::new().push(list).push(Text::new(list_count_summary(...)).size(14))` with `Length::Fill` width and `Length::FillPortion(3)` height, mirroring the song footer in `song_list`; when `count` is zero, return `list` unchanged so a loading, error, or empty placeholder gets no `0 artists` footer.
- Wrap the three non-song list builders at their row-construction sites: `view_artists` passes `artists.len()` with `"artist"`/`"artists"`, `album_list` passes `albums.len()` with `"album"`/`"albums"` (so both `view_albums` and `view_all_albums` gain the footer once), and `view_playlists` passes `playlists.len()` with `"playlist"`/`"playlists"`. Do not wrap inside `scrollable_list` itself: `song_list` already appends its own count-and-time footer, so wrapping there would give every song list a second footer.
- Update the doc comments of `view_artists`, `album_list` (which `view_albums` and `view_all_albums` both delegate to), and `view_playlists` to say a non-empty list appends its item-count footer, and note in `list_with_count` why it skips an empty list.
- `src/ui/views/tests.rs`: import `list_count_summary` and add `list_count_summary_uses_the_singular_and_plural`, asserting `"1 artist"` for a count of one, `"3 artists"` for a count of three, `"1 playlist"`, and `"0 albums"`. The existing `browse_views_construct_over_the_loaded_library` and `browse_views_construct_over_an_empty_list` build the populated and empty branches of every wrapped builder through `construct_every_browse_view`.
- `README.md`: extend the existing Usage sentence — which already says each song list shows its song count and total playing time — to also say artist, album, and playlist lists show their item count beneath the rows. Keep the hunk to that sentence.
- `docs/invariants.md`: the browse-hierarchy and reply-freshness guarantees do not change, and the document makes no claim about footers, so leave it unchanged.

**Files touched.** `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `list_count_summary` returns `"1 artist"` for `(1, "artist", "artists")`, `"3 artists"` for `(3, "artist", "artists")`, `"1 playlist"` for `(1, "playlist", "playlists")`, and `"0 albums"` for `(0, "album", "albums")`.
- A loaded Artists, Albums, All Albums, or Playlists view renders its rows followed by exactly one footer whose count equals the number of rows passed in; an empty, loading, or failed one still renders only its placeholder, with no `0 …` footer.
- Song lists keep the existing `song_list_summary` footer and call no new footer helper, so no song list shows two footers.
- Row order, row labels, selection messages, epochs, and placeholder wording are unchanged, and both construct tests build every browse view without panicking.
- The change touches only the three files listed above (plus the `PLANS.md` edit and backlog moves), and the README hunk is limited to the item-count clause.

### Number every browse-list row (planned 2026-10-21 by the plan role; done 2026-10-21)

**Goal.** Each row in every browse list opens with its 1-based position and a period (`1. The Sample Band`, `2. First Record`), as the classic Winamp playlist editor does; the currently playing row keeps its `▶` marker before the number (`▶ 3. Opening`). This is independent of the planned count/time footer and of the browse data: numbering is derived from each list's row order, so it needs no service or model change.

**Approach.**

- `src/ui/views/browse.rs`: add a pure `pub(super) fn row_prefix(position: usize, is_current: bool) -> String` that returns `"{n}. "` for zero-based `position`, with `"▶ "` before the number when `is_current` is true (so `row_prefix(0, false)` is `"1. "` and `row_prefix(0, true)` is `"▶ 1. "`). It is the one place the marker and number order are composed, so a marker moved after the number cannot slip in untested.
- In `fn scrollable_list`, enumerate the incoming items (`for (position, (title, label, message, is_current)) in items.into_iter().enumerate()`) and build the row's leading label from `row_prefix(position, is_current)` instead of the current conditional `▶ ` row. Keep the title and secondary label as they are, and keep the existing per-row style selection (`current_row_style` for the playing row, `playlist_row_style` otherwise) and the empty-placeholder branch unchanged. Every browse list — Artists, Albums, Songs, All Songs, All Albums, Playlists — renders through `scrollable_list`, so all six gain numbering with no row-builder or call-site change.
- Update `scrollable_list`'s doc comment: it now says each row is prefixed with its 1-based position (and the playing marker), and that the prefix is one owned `String` per row per refresh while the title and secondary label still borrow. Do not cache prefixes in `BrowseList`: the position comes from the iterator and a cached list would need invalidation on every store for no correctness gain.
- `src/ui/views/tests.rs`: add a direct test for the helper, `row_prefix_numbers_rows_and_puts_the_play_marker_first`, asserting `row_prefix(0, false) == "1. "`, `row_prefix(9, false) == "10. "`, `row_prefix(99, false) == "100. "`, and `row_prefix(0, true) == "▶ 1. "`. The existing `browse_views_construct_over_the_loaded_library` and `browse_views_construct_over_an_empty_list` continue to build every numbered view through the shared constructor.
- `README.md`: add one sentence to the Usage paragraph saying every browse list numbers its rows from 1 like a Winamp playlist. Keep the hunk to that sentence.
- `docs/invariants.md` records browse navigation, reply freshness, and the Now Playing label; row numbering changes none of those guarantees, so leave it unchanged.

**Files touched.** `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `row_prefix` returns `"1. "`, `"10. "`, and `"100. "` for positions 0, 9, and 99 with `is_current == false`, and `"▶ 1. "` for position 0 with `is_current == true`.
- Every row built by `scrollable_list` gets the prefix before its title in both the playing and non-playing branches; the playing-row background and style are unchanged, and an empty list still shows only its placeholder with no number.
- Row selection messages, cached length labels, list order, and empty/loading/error wording are unchanged, and the construct tests build every browse view over both the loaded and empty library without panicking.
- The feature itself changes only the three files listed above (the `PLANS.md`/`PLANS-DONE.md` move aside), and the README hunk is limited to the numbering sentence.

### Show each song list's count and total playing time (planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** Every loaded song list shows, beneath its rows, how many songs it holds and their combined playing time — the classic Winamp playlist editor's status line. Today `song_list` in `src/ui/views/browse.rs` builds only the scrollable rows: each row shows its own formatted length (`song_row` via `format_track_time`), but the list never aggregates them. This is independent of the planned Shuffle-On Previous history: it reads only the `[Song]` slice the view already receives.

**Approach.**

- `src/ui/views/browse.rs`: add `pub(super) fn song_list_summary(songs: &[Song]) -> String` returning `"<count> song(s) - <total>"`, with singular `song` for a count of one and plural `songs` otherwise. Count every entry, including one whose `duration_ms` is `0`, and sum the durations with `u64::saturating_add` so a large list cannot overflow the total.
- Add a shared `fn format_minutes_seconds(total_ms: u64) -> String` in `now_playing.rs`: `0` formats as `"0:00"`, otherwise zero-padded `m:ss` with unbounded minutes (`3_665_000` formats as `"61:05"`), mirroring `views::format_track_time`'s arithmetic but not its `0` → `"--:--"` placeholder. Do not change `format_track_time`: `0` there means one track's duration is unknown, while `0` here is a valid list total when every duration is unknown.
- In `fn song_list`, keep the current `scrollable_list(...)` result for the rows; when `songs` is non-empty, return a `Column::new().push(list).push(Text::new(song_list_summary(songs)).size(14))` with `Length::Fill` width and the scrollable's current `Length::FillPortion(3)` height, so the browse panel keeps its share of the window and the summary sits left-aligned below the rows. When `songs` is empty, return the scrollable unchanged so the loading, error, and empty placeholders stay alone instead of gaining a `0 songs` footer. Compute the summary once per view refresh, not once per row, and update `song_list`'s doc comment to say it appends the count/time footer.
- No call-site changes are needed: `view_songs` (album and playlist songs and search results) and `view_all_songs` both build through `song_list`, so every loaded song list gains the footer. Artist, album, and playlist lists keep their current bodies.
- `src/ui/views/tests.rs`: add direct tests for the pure helper — `song_list_summary_counts_songs_and_sums_durations` (210_000 + 45_000 → `"2 songs - 4:15"`), `song_list_summary_uses_the_singular_for_one_song` (210_000 → `"1 song - 3:30"`), `song_list_summary_counts_unknown_durations_without_adding_time` (`[210_000, 0]` → `"2 songs - 3:30"`), and `song_list_summary_keeps_counting_minutes_past_an_hour` (3_665_000 → `"1 song - 61:05"`). The existing `browse_views_construct_over_the_loaded_library` and `browse_views_construct_over_an_empty_list` already exercise both footer branches through the view builders.
- `README.md`: add one sentence to the Usage paragraph saying each song list shows its song count and total playing time beneath the rows. Keep the hunk to that sentence.
- `docs/invariants.md` records browse navigation, reply freshness, and the Now Playing label; this display-only footer changes none of those guarantees, so leave it unchanged.

**Files touched.** `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `song_list_summary` reports the exact count and sum: `[]` → `"0 songs - 0:00"`, one 210_000 ms song → `"1 song - 3:30"`, 210_000 + 45_000 ms → `"2 songs - 4:15"`, and 3_665_000 ms → `"1 song - 61:05"`.
- A song with `duration_ms == 0` still counts toward the song total but contributes no time, so `[210_000, 0]` reads `"2 songs - 3:30"`.
- A non-empty song list built by `view_songs` or `view_all_songs` includes the footer, while an empty list shows only its existing placeholder; the construct tests cover both cases without panicking.
- The implementation diff changes only the three files listed above for the feature itself (the `PLANS.md`/`PLANS-DONE.md` move aside), and the README hunk is limited to the new summary sentence.

### Make Previous retrace the play history when Shuffle is on (planned 2026-10-20 by the plan role, done 2026-10-21)

**Goal.** With Shuffle on, pressing Previous returns to the track actually played before the current one, retracing the play order like classic Winamp, instead of stepping one row backward in the visible list. With Shuffle off, Previous keeps its sequential and Repeat-aware behavior. Today `step_track` in `src/ui/mod.rs` sends only a forward step through `transport::shuffled_track_id`; every backward step goes through `transport::previous_track_id`. The `step_track` doc comment and `previous_with_shuffle_still_steps_backward` in `src/ui/tests.rs` pin that Previous ignores Shuffle because no play history exists.

**Approach.**

- `src/ui/transport.rs`: beside `shuffled_track_id`, add `pub(super) const HISTORY_LIMIT: usize = 100` and `#[derive(Debug, Default)] pub struct PlayHistory` holding `entries: Vec<String>`, `cursor: Option<usize>`, and `pending: Option<PendingJump>`, plus a private `#[derive(Debug)] struct PendingJump { target: usize, from: Option<String>, retry: bool }`.
  - `pub fn record(&mut self, track_id: &str)`: when `pending` names a target entry equal to `track_id`, the history jump landed, so set `cursor` to that target and clear `pending` without pushing, truncating, or trimming — the jump neither duplicates the entry nor drops the forward entries, and a later new track truncates them from the landed cursor. Otherwise clear `pending` first; when the cursor already names `track_id` do nothing, and otherwise truncate entries after the cursor, push `track_id`, point the cursor at the new last entry, then drop the oldest entries past `HISTORY_LIMIT` while moving the cursor down with them.
  - `pub fn previous(&mut self, songs: &[Song], current: Option<&str>) -> Option<String>`:
    - A pending jump stays in effect while `current` is `None`, names the jump's target entry (its play committed but `record` has not run), or names the jump's `from` track (its play has not committed yet). When `current` names anything else, the user selected another track before the jump landed, so clear `pending` and continue from the cursor, which still names the last track actually recorded.
    - A failed play calls `retry_pending`, which keeps the pending jump and marks it for retry: `play_track` leaves `current_track` naming the target when the backend rejects the preview, so the next `previous` returns that same target once more (clearing the retry flag) and only a later press walks before it. A failed play with no pending jump is a no-op.
    - With a jump in effect, scan strictly before its target index, so repeated Previous presses before that play commits keep walking back. Without a jump, take `self.cursor?` and scan strictly before it when its entry is `current`, otherwise up to and including it (`cursor + 1`) when `current` is a newer, unrecorded selection.
    - Find the highest index below the start whose id `songs` contains; on a hit set `pending` to that index (keeping an in-effect jump's original `from`), return its id, and leave the cursor unchanged until the play records. On a miss return `None` leaving the cursor and any in-effect jump unchanged, so `step_track` falls back to `transport::previous_track_id`. Skip entries absent from `songs` so a browse to another list selects a row the visible list can show. Do not add the rejected draft's `at_start` helper: an exhausted history uses the same sequential fallback.
- `src/ui/mod.rs`: add `play_history: transport::PlayHistory` to `WinampPlayer` and initialize it with `Default::default()` in `WinampPlayer::new`. Change `step_track` to take `&mut WinampPlayer`: when `!forward && state.shuffle`, first try `player.play_history.previous(items, state.current_track.as_deref())`, use its id when present, and otherwise fall through to `transport::previous_track_id` as today; the forward Shuffle branch and both sequential branches stay as they are. In the `Message::TrackPlayed` arm, inside the existing current-generation guard, record `state.current_track` only when `state.is_playing` is true, so a failed play never enters the history and a superseded completion is skipped by the generation check; a history jump's completion records the target `pending` already names, which `record` handles without duplicating or truncating. A current-generation completion with `is_playing` false calls `play_history.retry_pending()`: the pending jump stays in place (the backend left `current_track` naming its target), so the next Previous retries that target rather than walking past it. Update the `step_track` doc comment to describe the history-backed Previous behavior instead of "Previous ignores Shuffle".
- `src/ui/tests.rs`: replace `previous_with_shuffle_still_steps_backward` with behavior tests, and drive `Message::TrackPlayed` to exercise the recording path. Cover a shuffled Previous that retraces two recorded plays; two presses before the first jump commits (same `current`) that keep walking back; a failed jump whose target the backend left in `current_track` and whose next Previous retries it; a pending jump superseded by a new committed `current` that then steps from the last actually played track; a shuffled Previous with no usable history that falls back to the sequential row; a history entry absent from the visible list being skipped; and two guard tests. `track_played_ignores_a_failed_completion` and `track_played_ignores_a_superseded_completion` each seed `["song-1", "song-2"]` and report `song-3` without recording it, then assert `previous(&songs, None)` returns `"song-2"` — with a wrongly recorded `song-3` it would return `"song-3"`, so the assertion actually pins the guard instead of passing through `previous`'s current-track branch.
- `src/ui/transport.rs` tests: cover recording order and the repeated-current no-op; recording after a back step dropping the forward entries; the `HISTORY_LIMIT` bound and cursor shift; `previous` returning `None` with no history; walking backward on repeated calls and across a pending jump; a jump target's `record` landing on its entry without duplicating or truncating; a failed jump retrying its target once before walking further back; a superseded jump stepping from the last recorded play rather than skipping it; including the last recorded entry when `current` is a newer selection; skipping ids absent from the supplied `songs`; and returning `None` at the oldest entry without moving the cursor.
- `README.md`: in the Usage paragraph, say that with Shuffle on Previous retraces the tracks played before the current one. Keep the hunk to that sentence.
- `docs/invariants.md` has no transport-stepping section (its headings cover the browse hierarchy, browse reply freshness, and the Now Playing label), so this plan leaves it unchanged.
- When the feature run moves this entry to `PLANS-DONE.md`, write the done entry's approach from the shipped code — including `PendingJump` and `previous(songs, current)` — so it does not repeat the stale draft entry that described a different API.

**Files touched.** `src/ui/transport.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `PlayHistory::record` keeps chronological play order, treats a repeated current id as a no-op, drops forward entries after a back step when a new track is recorded, keeps the forward entries when a pending jump's own target records, and keeps at most `HISTORY_LIMIT` entries with the cursor following the oldest dropped entry.
- `PlayHistory::previous` returns the nearest earlier entry present in `songs`, walks further back on repeated calls including before a jump's play commits, skips entries from a list the player no longer shows, includes the last recorded entry when `current` is a newer unrecorded selection, and returns `None` without moving when no earlier usable entry exists.
- A jump whose play never lands is retried when the backend leaves its target in `current_track`: from history `["song-1", "song-2", "song-3"]`, a Previous from `song-3` selects `song-2`; a failed completion names `song-2` and the next Previous selects `song-2` again before walking on to `song-1`. A jump superseded by a different current track is abandoned: a Previous from `song-3` then a Previous from a new `song-4` selects `song-3`, not `song-1` or `song-2`.
- With Shuffle on, Previous selects the recorded previous track rather than the row before the current one; with no usable history it falls back to `previous_track_id`. With Shuffle off, Previous still follows `previous_track_id` and the shared Repeat flag.
- A current-generation `TrackPlayed` with `is_playing` true records the committed track; one with `is_playing` false marks the pending jump for retry and records nothing, while a superseded generation records nothing. The two guard tests distinguish a recorded `song-3` (`previous(.., None)` would return `"song-3"`) from the unrecorded expected `"song-2"`.
- The new tests named in the approach pass alongside the existing Next/Previous wiring tests, and the README documents the Shuffle-on Previous behavior.
- The implementation diff changes only the four files listed above for the feature itself (the `PLANS.md`/`PLANS-DONE.md` move aside), and the README hunk is limited to the Shuffle-on Previous sentence.

### Seek the playing preview five seconds with the left and right arrow keys (planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** Pressing Left seeks the playing preview back five seconds and pressing Right seeks it forward five seconds, matching classic Winamp's keyboard bindings. Today `shortcuts::shortcut` maps only ArrowUp/ArrowDown (to `Message::VolumeUp`/`VolumeDown`), `Message` has no seek variant, and `AudioOutput` exposes only play/pause/resume/stop/`set_volume`/`set_balance`. `rodio` 0.22's `Player` already provides `get_pos` and `try_seek`, but nothing in `src/audio.rs` calls them, so the left/right keys currently do nothing.

**Approach.**

- **Seek-time channel phase.** rodio 0.22.2's `Source::try_seek` implementations leave the next sample on the same channel as before the seek: `SamplesBuffer::try_seek` offsets the target by the current channel and `SymphoniaDecoder::try_seek` re-advances to the active channel. The wrappers' cursors must stay in that phase. `BalanceSource::try_seek` already only delegates, so leave it that way and say in its doc comment why it must not reset `channel`; `EqSource::try_seek` currently assigns `self.channel = 0` after seeking, so remove that assignment but keep `rebuild_filters()` so a seek still clears the filter ringing.
- `src/audio.rs`: add `fn seek_by(&self, offset_secs: i64) -> Result<(), AppleMusicError>` to `AudioOutput`, documented as a relative move (positive seeks forward, negative backward), a no-op when no player is loaded, and a non-blocking channel send like `set_volume`. Implement it for `SilentOutput` (log and return `Ok`), add `Command::SeekBy(i64)`, and add `fn seek_by(&self, offset_secs: i64)` to `WorkerPlayer`. Add a private pure `fn seek_target(position: Duration, offset_secs: i64) -> Duration` that saturating-adds or saturating-subtracts `Duration::from_secs(offset_secs.unsigned_abs())`, so a backward seek past the start lands at zero. `RodioPlayer::seek_by` reads `self.player.get_pos()`, calls `self.player.try_seek(seek_target(position, offset_secs))`, and logs a `try_seek` error with `eprintln!` without changing any state. `serve_commands` handles `Command::SeekBy` by calling `player.seek_by(offset_secs)` only when a player is installed; the offset is not remembered across players, so a seek while stopped does nothing.
- `src/test_support.rs`: add `AudioCall::SeekBy(i64)`, record it in `RecordingAudio::seek_by`, and return `Ok(())` from `FailingAudio::seek_by`.
- `src/apple_music.rs`: add `pub(crate) fn seek_output(&self, offset_secs: i64) -> Result<(), AppleMusicError>` beside `set_output_volume` and `set_output_balance`, forwarding to `self.audio.seek_by(offset_secs)` and touching no shared state.
- `src/apple_music/tests.rs`: implement `seek_by` for `GatedAudio` and `RejectingAudio`, recording `AudioCall::SeekBy` and succeeding (like their other non-rejected commands). Add `seek_output_forwards_to_the_audio_backend`, asserting one `AudioCall::SeekBy(-5)` and that `AppState` is unchanged.
- `src/ui/mod.rs`: add `SeekBackward` and `SeekForward` variants beside `VolumeUp`/`VolumeDown`, and add a private `seek_preview(player, offset_secs)` helper that calls `player.apple_music_service.seek_output(offset_secs)`, logs a backend error like `mutate_volume`, and returns `Task::none()`. Route the two new arms through it with `-views::SEEK_STEP_SECS` and `views::SEEK_STEP_SECS`.
- `src/ui/views/mod.rs`: add `pub(super) const SEEK_STEP_SECS: i64 = 5;` beside `VOLUME_STEP`, documenting Winamp's five-second seek step.
- `src/ui/shortcuts.rs`: map `Key::Named(key::Named::ArrowLeft)` to `Message::SeekBackward` and ArrowRight to `Message::SeekForward`, updating the `message_for`/`shortcut` doc lists. Extend `message_for_maps_the_arrow_keys` and add both arrows to `shortcut_ignores_chord_modifiers`.
- `src/ui/tests.rs`: add `seek_messages_drive_the_audio_backend`, driving both messages through `update` over a `RecordingAudio` and asserting one `AudioCall::SeekBy(-SEEK_STEP_SECS)` and one `SeekBy(SEEK_STEP_SECS)` with no scheduled task.
- `src/audio.rs` tests: extend `silent_output_reports_success_for_every_command`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` with `seek_by`; add `seek_target_moves_forward_and_backward` (10s +5 → 15s, 10s -5 → 5s) and `seek_target_saturates_at_the_start` (3s -5 → 0); extend the recording `WorkerPlayer` with `seek_by` and `transport_volume_and_balance_commands_drive_the_installed_player` with a `Command::SeekBy(-5)` before its `Stop`, asserting the installed player received it. Change the `SeekableSource` test double's `try_seek` to land on the target frame plus its current channel instead of resetting to a frame boundary, so it models rodio's contract. Rewrite `eq_source_seek_clears_filter_state` to seek from a frame boundary and assert the channel is unchanged, and add `eq_source_seek_keeps_the_channel_cursor` and `balance_source_seek_keeps_the_channel_cursor`, which seek from mid-frame and assert the next sample is still treated as the same channel.
- `README.md`: change the arrow-key sentence to say the up/down arrows nudge the volume and the left/right arrows seek five seconds back or forward.

**Files touched.** `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views/mod.rs`, `src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `seek_target` moves forward and backward by the requested seconds and clamps a backward seek past the start at zero.
- `RodioOutput::seek_by` sends `Command::SeekBy` carrying the same signed offset; `SilentOutput::seek_by` returns `Ok`; a closed worker reports "the audio thread is gone".
- `serve_commands` applies `SeekBy` only to an installed player; the recording player receives `-5` from the extended transport test.
- `AppleMusicService::seek_output` records exactly one `AudioCall::SeekBy` and leaves `AppState` unchanged.
- `message_for` maps ArrowLeft to `Message::SeekBackward` and ArrowRight to `Message::SeekForward`; Control, Alt, and Logo chords map neither.
- Driving both seek messages through `update` records exactly one `AudioCall::SeekBy(-5)` and one `AudioCall::SeekBy(5)` and schedules no task.
- The README documents the left/right five-second seek keys.
- `EqSource::try_seek` clears filter state without changing `channel`, and `BalanceSource::try_seek` leaves `channel` unchanged; a mid-frame seek through either wrapper keeps the next sample on the same channel, and the updated `SeekableSource` models that contract.
- The implementation diff against the pre-change commit changes only the nine files listed above for the feature itself (the move of this entry to `PLANS-DONE.md` aside), and the README hunk is limited to the arrow-key sentence.

### Apply the equalizer curve to the audio output (Equalizer audio, parts 1/2 and 2/2: planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** A preview starts through the equalizer's ten band filters at the curve the panel currently shows. Today `AppState` stores `eq_enabled`, `eq_preamp`, and `eq_bands`, and the equalizer update arms only write them; `AudioOutput` has no equalizer method, so every preview plays flat. This part adds the filter chain and applies the stored curve when a preview starts; part 2 forwards changes made while a preview is already playing.

**Approach.**

- `src/equalizer.rs`: add `pub const BAND_HZ: [f32; BAND_COUNT]` holding the numeric centres matching `BAND_FREQUENCIES` (`60.0` through `16_000.0`, with `1K` as `1_000.0`) and `pub struct BandFilter` with `new(sample_rate)`, `set_curve(enabled, preamp_db, bands_db)`, and `process(sample)`. Build each band as an RBJ peaking biquad at its `BAND_HZ` centre with a fixed `Q = 1.41` (roughly one octave), rebuild the coefficients in `set_curve`, and return the sample unchanged when disabled. When enabled, multiply by the preamp gain `10^(preamp_db / 20)` and run the sample through all ten biquads. Keep `Biquad` private to the module; `BandFilter` is the public type the audio source uses.
- `src/audio.rs`: add `pub struct EqSnapshot { pub enabled: bool, pub preamp_db: f32, pub bands_db: [f32; BAND_COUNT] }` (derive `Debug, Clone, Copy, PartialEq`) and private `EqSettings` holding those values in `AtomicBool`/`AtomicU32`s plus an `AtomicU64` version. Add `EqSource<S>` holding `inner`, `Arc<EqSettings>`, the last seen version, one `BandFilter` per channel from `inner.channels()`, and a channel cursor; on each `next`, rebuild every channel filter when the version changed, then process the sample through the current channel's filter. Delegate `current_span_len`, `channels`, `sample_rate`, `total_duration`, and `try_seek` like `BalanceSource`, resetting the filters on a successful seek. Add `set_equalizer(&self, equalizer: EqSnapshot)` to `AudioOutput`, `Command::SetEqualizer(EqSnapshot)`, and `WorkerPlayer::set_equalizer`; `SilentOutput` logs and returns `Ok`, `RodioPlayer` stores the snapshot in its `Arc<EqSettings>`, and `append` wraps each preview as `BalanceSource::new(EqSource::new(preview, eq), balance)` so equalization runs before the stereo balance. `serve_commands` remembers the snapshot across players and applies it to every newly created player, exactly as it remembers volume and balance.
- `src/test_support.rs`: add `AudioCall::SetEqualizer(EqSnapshot)`, record it in `RecordingAudio::set_equalizer`, and return `Ok(())` from `FailingAudio::set_equalizer`. Extend `assert_preview_calls` so a preview start expects `SetVolume(0.5)`, `SetBalance(0.0)`, `SetEqualizer(default)`, then `Play`.
- `src/apple_music.rs`: add `pub(crate) fn set_output_equalizer(&self, equalizer: EqSnapshot) -> Result<(), AppleMusicError>` forwarding to `self.audio.set_equalizer(equalizer)` and touching no shared state, like `set_output_volume`. In `play_track`, read `state.eq_enabled`, `state.eq_preamp()`, and `state.eq_bands()` under the state lock, then chain `set_volume(volume)`, `set_balance(balance)`, `set_equalizer(snapshot)`, and `play(url)` so the first preview starts at the stored curve rather than flat. Keep the existing rollback on a backend error.
- `src/audio.rs` tests: update `RecordingPlayer` to implement `set_equalizer`; add `a_new_preview_starts_at_the_remembered_equalizer` beside `a_new_preview_starts_at_the_remembered_balance`; extend `silent_output_reports_success_for_every_command`, `audio_with_fallback_falls_back_to_silence_when_open_fails`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` with the new method. Add `eq_source_disabled_passes_samples_through`, `eq_source_enabled_flat_curve_passes_samples_through`, `eq_source_preamp_scales_by_the_decibel_gain`, `eq_source_filters_each_channel_with_its_own_state`, and `eq_source_follows_the_curve_changed_while_playing`.
- `src/equalizer.rs` tests: add `band_frequencies_have_matching_numeric_centres`, `band_filter_boost_and_cut_at_the_center_frequency` (a generated 1 kHz sine through a `+12 dB` and a `-12 dB` 1 kHz band, comparing settled RMS), and `band_filter_preamp_is_a_flat_gain`.
- `src/apple_music/tests.rs`: add `set_output_equalizer_forwards_to_the_audio_backend` and update `play_track_starts_the_preview_through_the_audio_backend` and `assert_preview_calls` callers to expect the equalizer snapshot before `Play`.
- `README.md`: state in the usage paragraph that the equalizer panel's curve is applied to preview playback.

**Files touched.** `src/equalizer.rs`, `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- A disabled `BandFilter` returns each input sample unchanged; an enabled flat curve (zero preamp, zero bands) also passes samples through within floating-point error.
- A `+12 dB` 1 kHz band raises a settled 1 kHz sine's RMS, a `-12 dB` band lowers it, and a `+6.0206 dB` preamp produces approximately twice the input amplitude; each channel of a stereo source keeps its own filter state.
- `EqSource` reports its inner source's channel count, sample rate, span, duration, and seek result, and follows a curve changed through its shared `EqSettings` between samples.
- `RodioOutput::set_equalizer` sends `Command::SetEqualizer` with the same snapshot; `SilentOutput::set_equalizer` returns `Ok`; a closed worker reports "the audio thread is gone".
- `AppleMusicService::set_output_equalizer` records one `AudioCall::SetEqualizer` and leaves `AppState` unchanged; `play_track` records `SetVolume`, `SetBalance`, `SetEqualizer` carrying the state's current curve, then `Play`.

**Part 2/2 (done 2026-10-20).** Toggling the equalizer, dragging the preamp or a band slider, or picking a preset changes the sound of the preview already playing, without restarting it. Part 1 applies the stored curve when a preview starts; before this part the four equalizer update arms still called `mutate_state`, so a change made mid-track never reached the audio backend.

- `src/ui/mod.rs`: added `fn mutate_equalizer(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message>` beside `mutate_volume` and `mutate_balance`: it locks the state, applies the mutation, reads `eq_enabled`, `eq_preamp()`, and `eq_bands()`, builds an `EqSnapshot`, and calls `player.apple_music_service.set_output_equalizer(snapshot)`, logging a backend error with the same `eprintln!` shape as the volume and balance helpers. `ToggleEqualizer`, `EqPreampChange`, `EqBandChange`, and `EqPresetSelected` now route through it instead of `mutate_state`. Because `EqSource` reads its shared `EqSettings` between samples, the running preview picks up the new curve without a new `Play`.
- `src/ui/tests.rs`: added `equalizer_messages_forward_to_the_audio_backend`, driving `ToggleEqualizer`, `EqPreampChange(3.0)`, `EqBandChange(0, 4.0)`, out-of-range and `f32::NAN` band gains, and `EqPresetSelected(rock_preset())` through `update` over a `player_with_audio` built with a `RecordingAudio` fake, asserting the recorded `SetEqualizer` snapshots match the clamped `AppState` values and that no arm schedules a task. Also added `an_equalizer_backend_failure_still_commits_the_state`, which drives `EqBandChange` over a `FailingEqualizerAudio` that rejects `set_equalizer`, asserting the clamped band still lands in `AppState` and the backend saw the attempt.
- `src/test_support.rs`: added `FailingEqualizerAudio`, an `AudioOutput` whose `set_equalizer` records the attempt and returns an error, so the equalizer arm's backend-error branch is reachable from the UI suite.
- `README.md`: extended the equalizer sentence to say the toggles, sliders, and presets change the playing preview's curve live.

**Files touched.** `src/ui/mod.rs`, `src/ui/tests.rs`, `src/test_support.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- Each equalizer message records exactly one `SetEqualizer` call whose enabled flag, preamp, and ten band gains equal the clamped values now in `AppState`; no call restarts the preview.
- A backend error from `set_output_equalizer` is logged and the state mutation still commits, as with volume and balance.
- The README says equalizer changes apply to the playing preview.

### Add the R and S keyboard bindings for Repeat and Shuffle (planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** Pressing R toggles Repeat and pressing S toggles Shuffle from the keyboard, matching classic Winamp, so both transport toggles are reachable without the mouse. Today `shortcuts::shortcut` maps Z/X/C/V/B and the arrow keys only: R and S fall through to `None`, even though `Message::ToggleRepeat`, `Message::ToggleShuffle`, and their `update` arms already exist.

**Approach.**

- `src/ui/shortcuts.rs`: add two arms to `shortcut` beside the existing letter arms — `r` returns `Message::ToggleRepeat` and `s` returns `Message::ToggleShuffle` — using the same `eq_ignore_ascii_case` guard as the other letters, so uppercase and Shift work and the existing Control/Alt/Logo rejection applies. Update the doc comments on `message_for` and `shortcut` to list the two new bindings.
- `README.md`: extend the usage paragraph's classic-key sentence after the V entry with "R toggles Repeat and S toggles Shuffle".
- Tests in `src/ui/shortcuts.rs`: extend the `assert_letter_bindings` helper so the existing `message_for_maps_each_bound_letter` and `shortcut_matches_letters_case_insensitively` tests also assert that `r`/`R` map to `Message::ToggleRepeat` and `s`/`S` map to `Message::ToggleShuffle`; add both keys to the key list in `shortcut_ignores_chord_modifiers`, so a Control, Alt, or Logo chord still maps neither.

**Files touched.** `src/ui/shortcuts.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `message_for` maps a pressed `Key::Character("r")` to `Message::ToggleRepeat` and `Key::Character("s")` to `Message::ToggleShuffle`, with or without Shift, and maps their uppercase forms the same way.
- Control, Alt, and Logo chords over R or S still map to `None`, and the existing unbound-key and key-release behavior is unchanged.
- The README's classic-key sentence lists R and S beside the existing transport and volume keys.

### Back the playlists browse with the Apple Music REST endpoints (Playlists browse, part 2/2: planned 2026-10-20 by the plan role, requires part 1/2 landed, done 2026-10-20)

**Goal.** Point the part-1 playlist queries at the signed-in user's Apple Music library, so a stored session shows that user's real playlists and each playlist's real tracks instead of the sample data, while a session-less service keeps its sample fallback. This is the second half of the playlist browse, the twin of "Back the library search with the Apple Music REST endpoint" (done); land it only once the Playlists view from part 1/2 is the running build.

**Approach.**

- `src/apple_music/rest.rs`: add `Playlist` to the `use crate::library::{Album, Artist, Song}` import and the module doc's model list, then add `pub fn get_all_playlists(&self, session: &MusicKitSession) -> Result<Vec<Playlist>, AppleMusicError>` requesting `{API_BASE}/me/library/playlists` through `self.fetch` and mapping each resource with `Resource::into_playlist`; add `pub fn get_songs_from_playlist(&self, session: &MusicKitSession, playlist_id: &str) -> Result<Vec<Song>, AppleMusicError>` requesting `{API_BASE}/me/library/playlists/{}/tracks` with `encode_path_segment(playlist_id)` and mapping with `Resource::into_song("")` — a library playlist track carries no album id, the same shape `get_all_songs` gives its songs. Add `Resource::into_playlist`, which moves the resource's `id` and `attributes.name` into the model through `required_id(id, "playlist")` and `required_name(…, &id, "playlist")`, exactly as `into_album` does for albums. Document the endpoint, the empty `album_id`, and the id encoding on both methods.
- `src/apple_music.rs`: add `Playlist` to the `use crate::library::{Album, Artist, Song}` import, rewrite `get_playlists` as `self.browse(|rest, session| rest.get_all_playlists(session), sample_all_playlists)` and `get_songs_from_playlist` as `self.browse(move |rest, session| rest.get_songs_from_playlist(session, &id), || lookup(&sample_library().songs_by_playlist, playlist_id))` after the existing `ensure_id_is_valid` guard, keeping the guard's error contract. Add `fn sample_all_playlists() -> Vec<Playlist>` cloning `sample_library().playlists`, and remove the part-1 doc caveat that the queries ignore a stored session.
- `README.md`: remove the "playlist data comes from the sample library until the REST endpoints land" caveat, leaving playlists described alongside the other signed-in browse queries.
- Tests: `src/apple_music/rest/tests.rs` gains `get_all_playlists_maps_the_library_playlists_collection` (pin `/v1/me/library/playlists` and the mapped id/name), `get_all_playlists_follows_a_next_page`, `get_all_playlists_reports_a_transport_error_bare`, `get_all_playlists_rejects_a_nameless_playlist`, `get_songs_from_playlist_maps_the_playlist_tracks_collection` (pin the id-scoped path and empty `album_id`), `get_songs_from_playlist_percent_encodes_the_playlist_id`, and `get_songs_from_playlist_reports_a_transport_error_bare`. `src/apple_music/tests.rs` adds `get_playlists_uses_the_rest_library_when_signed_in` and `get_songs_from_playlist_uses_the_rest_library_when_signed_in` over the stub transport, and keeps the existing no-session sample tests green (updating only the two part-1 tests that pinned the sample-only signed-in behavior).

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/sample_library.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary::get_all_playlists` requests `/v1/me/library/playlists`, maps each resource's id and name, and follows a same-origin `next` page; a nameless playlist is an error naming its id and a transport failure propagates its cause unchanged.
- `RestLibrary::get_songs_from_playlist` requests `/v1/me/library/playlists/{encoded-id}/tracks`, maps songs with an empty `album_id`, percent-encodes a reserved character in the id, and propagates a transport failure.
- With a stored session, `AppleMusicService::get_playlists` and `get_songs_from_playlist` return the REST results; with no session they return the sample playlists and playlist songs in the part-1 order.
- A blank, whitespace-only, or control-character playlist id is still rejected by the service before any REST call is issued.

### Add a Playlists browse level backed by the sample library (Playlists browse, part 1/2: planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** A `Playlists` button on the Artists view opens a flat list of the user's library playlists; pressing a playlist opens its songs in the existing Songs view, and those rows play exactly like any other song row. Back from the songs returns to Playlists, and Back from Playlists returns to Artists. The Playlists level is a new top-level browse sibling of the All Songs and All Albums flat lists. This first half answers the two new service queries from the sample library, the same sample-first split "Add a library search box to the playlist editor" (done) used before its REST sibling landed; the sibling "Back the playlists browse with the Apple Music REST endpoints" (Playlists browse, part 2/2) points the same two methods at the signed-in library. Until that sibling lands, a signed-in service also answers playlists from the sample library — documented in the method docs, not left implicit.

**Approach.**

- `src/library.rs`: add `pub struct Playlist { pub id: String, pub name: String }`, deriving `Debug, Clone, PartialEq, Serialize, Deserialize` like `Artist`, and extend the module doc's model list. Add tests mirroring the `Artist` pins: a hand-written `playlist_payload()` (`{"id":"playlist-1","name":"Morning Drive"}`), serialized-field-name/round-trip, unknown-field tolerance, and `assert_every_field_required`.
- `src/test_support.rs`: add `sample_playlist()` beside `sample_artist`/`sample_album`/`sample_song`, carrying id `playlist-1` and name `Morning Drive`, for the `library` and `apple_music` suites.
- `src/sample_library.rs`: add `pub playlists: Vec<Playlist>` and `pub songs_by_playlist: HashMap<String, Vec<Song>>` to `SampleLibrary`. Build three playlists in render order — `playlist-1` "Morning Drive", `playlist-2` "Focus", `playlist-3` "Sample Favorites" — and index their songs from the existing sample songs, preserving each playlist's named order (`playlist-1` → `song-1`, `song-4`; `playlist-2` → `song-2`, `song-5`; `playlist-3` → `song-1`, `song-2`, `song-3`). Add tests mirroring `albums_are_indexed_by_artist_in_library_order_with_no_empty_groups`: playlists in order, three non-empty groups, and each group's song ids in the documented order. Update the module doc's browse-query list.
- `src/apple_music.rs`: add `pub async fn get_playlists(&self) -> Result<Vec<Playlist>, AppleMusicError>` returning `sample_library().playlists.clone()`, and `pub async fn get_songs_from_playlist(&self, playlist_id: &str) -> Result<Vec<Song>, AppleMusicError>` calling `ensure_id_is_valid(playlist_id, IdKind::Playlist)` then `lookup(&sample_library().songs_by_playlist, playlist_id)`. Add `IdKind::Playlist` and its `"playlist"` `Display` arm. Document on both methods that they answer from the sample library even when a session is stored, and that the REST sibling rewires them through `browse`; the async signatures and id guard already match that end state.
- `src/ui/mod.rs`: add `Playlist` to the `use crate::library::{Album, Artist, Song}` import, `CurrentView::Playlists`, `Message::ShowPlaylists`, `Message::PlaylistsLoaded(Vec<Playlist>)`, `Message::PlaylistsLoadFailed(String)`, and `Message::PlaylistSelected { epoch, index }`; add `playlists: BrowseList<Playlist>` to `WinampPlayer`, built with `BrowseList::new(false)` in `WinampPlayer::new`; add `SongsOrigin::Playlist`. The `ShowPlaylists` arm clears `search_active`, sets `CurrentView::Playlists`, and calls `fetch_level` on `player.playlists` with report `"loading library playlists"` and `|service| async move { service.get_playlists().await }`. The `PlaylistSelected` arm mirrors `AlbumSelected`: resolve through `player.playlists.select(epoch, index)`, clone the id, set `songs_origin = SongsOrigin::Playlist`, switch to `CurrentView::Songs`, clear any active search, and `fetch_level` the songs with `"loading songs from playlist {playlist_id:?}"` and `get_songs_from_playlist`. The `Back` arm's `CurrentView::Songs` match adds `SongsOrigin::Playlist => CurrentView::Playlists`, and `CurrentView::Playlists => CurrentView::Artists`. Store/fail through `player.playlists` in the loaded/failed arms, and map the two new reply messages to `player.playlists.is_current(issued)` in the `BrowseReply` arm.
- `src/ui/views/browse.rs`: add `playlist_row` (name, `"View Songs"`, `Message::PlaylistSelected`, never current) and `view_playlists` built through `scrollable_list` with the `CurrentView::Playlists` placeholder from `browse_placeholder`; add `view_playlists_button` returning `labeled_button("Playlists", Message::ShowPlaylists)`; add `CurrentView::Playlists => "No playlists"` to `empty_list_label` and `Playlists` to `BROWSE_VIEWS` (six entries).
- `src/ui/views/mod.rs`: re-export `view_playlists` and `view_playlists_button`.
- `src/ui/mod.rs`'s `view`: add the `CurrentView::Playlists` browse arm rendering `views::view_playlists(&player.playlists.items, player.playlists.epoch, player.playlists.loading, player.playlists.error.as_deref())`, and push `views::view_playlists_button()` beside the All Songs and All Albums buttons on the Artists view.
- `docs/invariants.md`: add `Playlists → Artists` and `Songs → Playlists when the songs were opened from a playlist` to the Browse hierarchy guarantee, and name `back_from_playlists_returns_to_the_artists_list` and `playlist_selected_in_playlists_loads_its_songs_and_back_returns_to_playlists` under "Pinned by".
- `README.md`: in the usage paragraph, describe the `Playlists` button and its Back path, and state that playlist data comes from the sample library until the REST endpoints land (the sibling plan removes that caveat).
- Tests: `src/apple_music/tests.rs` gains `get_playlists_returns_the_sample_playlists_in_order`, `get_songs_from_playlist_returns_its_sample_songs_in_order`, `get_songs_from_playlist_rejects_a_blank_id` (empty, whitespace-only, control character), and `get_songs_from_playlist_returns_empty_for_an_unknown_playlist`; the session variants pin the documented sample-only behavior so the sibling's change is deliberate. `src/ui/tests.rs` gains `show_playlists_fetches_and_switches_view`, `playlists_loaded_stores_the_list`, `playlist_selected_in_playlists_loads_its_songs_and_back_returns_to_playlists`, `back_from_playlists_returns_to_the_artists_list`, and `show_playlists_clears_an_active_search`, and extends `seed_browse_lists`, `assert_browse_lists_empty`, `each_browse_failure_stores_its_report_in_its_own_list`, and `view_constructs_over_the_apps_full_input_space` to cover `playlists`. `src/ui/views/tests.rs` pins `playlist_row`, `view_playlists_button`, the `empty_list_label` arm, and the extended `BROWSE_VIEWS` loops.

**Files touched.** `src/library.rs`, `src/test_support.rs`, `src/sample_library.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `Playlist` serializes as exactly `{"id", "name"}`, round-trips, tolerates an unknown field, and rejects a payload missing either field.
- The sample library exposes `playlist-1`/`playlist-2`/`playlist-3` in that order with the documented song ids in each `songs_by_playlist` group; no group is empty.
- `AppleMusicService::get_playlists` returns those three playlists in order with no session and with a session; `get_songs_from_playlist("playlist-1")` returns `song-1` then `song-4`; an unknown id returns an empty list; a blank, whitespace-only, or control-character id returns an error naming `playlist`.
- Pressing `Playlists` clears an active search, switches to `CurrentView::Playlists`, and fetches into `playlists`; a loaded reply stores the rows and a failed reply records the report on `playlists` alone.
- Pressing a playlist row fetches that playlist's songs into `songs` and shows `CurrentView::Songs`; Back returns to `CurrentView::Playlists`, while Back from playlist songs reached through any other origin still returns to Albums or All Albums as before.
- A stale playlist press (wrong epoch or out-of-range index) issues no fetch and leaves the view unchanged.
- The Artists view shows the `Playlists` button, and all six browse levels assemble, including empty and failed `playlists`, without panicking.

### Apply the balance slider to the audio output (planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** Moving the balance slider changes the stereo image of the playing preview, and a preview starts at the slider's balance. Today `Message::BalanceChange` only writes `AppState::balance`: `AudioOutput` has no balance method, so `mutate_state` never forwards the value and every preview plays centered. This is the balance twin of "Drive the volume slider through the audio backend" (done), whose approach deliberately left `BalanceChange` alone because rodio's `Player` has no panner. A small `Source` wrapper supplies the missing panner without a new dependency.

**Approach.**

- `src/audio.rs`:
  - Add `set_balance(&self, balance: f32) -> Result<(), AppleMusicError>` to `AudioOutput` (doc: `-1.0` is hard left, `0.0` centered, `1.0` hard right; it affects the current player and is remembered for the next `play`). `SilentOutput::set_balance` logs the value and returns `Ok`.
  - Add `Command::SetBalance(f32)`; `RodioOutput::set_balance` sends it.
  - Add a private `BalanceSource<S: Source>` adapter that wraps the decoded preview and holds the balance as an `Arc<AtomicU32>` of `f32` bits, read on every sample so a slider drag changes the current preview without restarting it. Delegate `current_span_len`, `channels`, `sample_rate`, `total_duration`, and `try_seek` to the inner source. Gain per channel: for a mono source `1.0 - balance.abs()`; otherwise even channel indices use `1.0 - balance.max(0.0)` (left) and odd indices use `1.0 + balance.min(0.0)` (right), so `-1.0` silences the right channel, `+1.0` the left, and `0.0` is unity. Do not use rodio's `ChannelVolume`/`Spatial`: both downmix the input to mono, which would collapse the preview's stereo image.
  - Add a private `RodioPlayer` struct holding the `rodio::Player` and its balance `Arc<AtomicU32>`; implement `WorkerPlayer` for it with `Preview = rodio::Decoder<Cursor<Vec<u8>>>`, appending the preview wrapped in `BalanceSource`, and add `set_balance` to the `WorkerPlayer` trait (the recording fake records it). `run_worker` builds a `RodioPlayer` instead of a bare `rodio::Player`.
  - In `serve_commands`, remember the balance (initial `0.0`) across players, map `Command::SetBalance` to storing it and calling `set_balance` on the current player when one exists, and call `set_balance(balance)` on a newly created player before `append`, as `set_volume` already does. Update the worker tests: the recording fake records `set_balance`, and a test pins that a balance set while stopped applies to the next player and one set while playing reaches the current player.
  - Add unit tests for `BalanceSource` using a two-channel `SamplesBuffer` with distinct left/right constants: `-1.0` keeps left and zeroes right, `+1.0` the reverse, `0.0` is unchanged, and changing the shared atomic between `next()` calls changes the following samples (the live-update contract).
  - Extend `silent_output_reports_success_for_every_command`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` to cover `set_balance`.
- `src/test_support.rs`: add `AudioCall::SetBalance(f32)` and record it in `RecordingAudio::set_balance`; `FailingAudio::set_balance` returns `Ok(())` like its volume twin. Extend `assert_preview_calls` (and its callers) so a preview start expects `SetVolume`, then `SetBalance`, then `Play`.
- `src/apple_music.rs`: add `pub(crate) fn set_output_balance(&self, balance: f32) -> Result<(), AppleMusicError>` forwarding to `self.audio.set_balance(balance)` and touching no shared state, exactly as `set_output_volume` does. In `play_track`, capture `state.balance()` before dropping the lock and apply it with the volume before `play`, so the first preview starts at the slider's balance.
- `src/ui/mod.rs`: add `mutate_balance(player, mutation) -> Task<Message>` beside `mutate_volume`: apply the mutation under one synchronous `blocking_lock`, read back the clamped `state.balance()`, forward it to `player.apple_music_service.set_output_balance(...)`, and log a backend error. Replace the `Message::BalanceChange` arm's `mutate_state` call with it and drop `BalanceChange` from `mutate_state`'s doc list.
- `README.md`: state in the usage paragraph that the balance slider sets the audio backend's stereo balance, beside the existing volume sentence.
- Tests: `src/apple_music/tests.rs` gains `set_output_balance_forwards_to_the_audio_backend` (one `AudioCall::SetBalance`, `AppState::balance` unchanged) and updates `play_track_starts_the_preview_through_the_audio_backend` to expect the balance call. `src/ui/tests.rs` gains `balance_messages_forward_to_the_audio_backend`, driving `Message::BalanceChange` through `update` over a `player_with_audio(RecordingAudio)` and asserting the recorded balance and the clamped `AppState::balance`, including an out-of-range value and `f32::NAN` (which stores and forwards `0.0`).

**Files touched.** `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `BalanceSource` at `-1.0` leaves the left channel unchanged and zeroes the right; at `+1.0` the reverse; at `0.0` both unchanged; a mono source is attenuated by `1.0 - balance.abs()`; mutating the shared balance between `next()` calls changes later samples without rebuilding the source.
- `RodioOutput::set_balance(0.4)` sends `Command::SetBalance(0.4)`; `SilentOutput::set_balance` returns `Ok`; a closed worker reports "the audio thread is gone" from `set_balance`.
- `serve_commands` applies a remembered balance to the player a later `Play` creates and to the current player on `SetBalance`.
- `AppleMusicService::set_output_balance(-0.4)` records one `AudioCall::SetBalance(-0.4)` and leaves `AppState::balance` unchanged; `play_track` records `SetVolume`, then `SetBalance` carrying the state's current balance, then `Play`.
- Driving `Message::BalanceChange(-0.4)` stores `-0.4` in `AppState::balance` and records `SetBalance(-0.4)`; an out-of-range value stores and forwards the clamped end, and `NaN` stores and forwards `0.0`.

### Add docs/invariants.md pinning browse and Now Playing guarantees to their tests (planned 2026-10-20 by the steward role as a gap promotion for `unclear-invariant`, done 2026-10-20)

**Goal.** Create `docs/invariants.md`, the record the validation-gap policy tells every role to read and update but which the repository does not have. Three Fixed entries carry the `unclear-invariant` tag because the guarantee had to be reconstructed before the bug could be confirmed: "Browse view can only move forward — no way back from Albums/Songs, a dead end", "A browse reply is stored after a newer request was issued, so a navigation shows the previous level's rows", and "Now Playing bar shows the track id instead of the song title". The document states each guarantee, names the code that enforces it, and names the tests that pin it, so a later bug in these areas reads the invariant instead of reconstructing it. No behavior changes.

**Approach.**

- Create `docs/invariants.md` with a short header explaining the file: one section per subsystem, each stating the guarantee, the enforcing code, and the tests that fail if it is removed; a change to a guarantee updates its section in the same commit.
- `## Browse hierarchy`: every browse level above Artists has a Back action that steps one level up, Artists is the top level, and Back from an active search leaves the search for the Artists list. Enforced by the `Message::Back` arm in `src/ui/mod.rs` and `views::can_go_back` in `src/ui/views/browse.rs`. Pinned by `back_from_albums_returns_to_artists`, `back_from_songs_returns_to_albums`, `back_from_artists_is_a_noop`, `back_from_all_songs_returns_to_the_artists_list`, `back_from_all_albums_returns_to_the_artists_list`, `back_leaves_an_active_search_for_the_artists_list`, and `back_button_is_available_only_below_the_artist_list`.
- `## Browse reply freshness`: a fetched reply is stored only when no newer request was issued for that list. `RequestGeneration::is_current` drops it on iced's executor thread and `BrowseList::is_current` re-checks at the store on the UI thread; a selection also carries its list's epoch so a press that outlived its list is ignored. Enforced by `RequestGeneration` in `src/ui/loading.rs` and `BrowseList::is_current`, `select`, and `store` in `src/ui/browse.rs`. Pinned by `a_slow_stale_browse_reply_does_not_overwrite_the_newer_list`, `a_slow_stale_browse_failure_does_not_overwrite_the_newer_list_or_error`, `a_reply_superseded_between_the_mapper_and_update_is_not_stored`, `a_stale_songs_reply_superseded_before_update_is_not_stored`, `a_stale_albums_reply_superseded_before_update_is_not_stored`, and `selection_messages_with_a_stale_epoch_or_index_do_nothing`.
- `## Now Playing label`: the bar shows the known current track's human-readable title and artist, never the internal id when the id is in the known-track index; an unknown id falls back to the id, no current track shows "Nothing", and Stop keeps the interrupted track's title. Enforced by `now_playing_label` and `now_playing_artist` in `src/ui/views/now_playing.rs`. Pinned by `now_playing_label_resolves_known_track_to_title`, `now_playing_label_falls_back_to_id_when_track_not_in_tracks`, `now_playing_label_shows_nothing_with_no_current_track`, `now_playing_artist_resolves_known_track_to_artist`, `now_playing_label_keeps_the_track_name_after_browsing_to_another_album`, `now_playing_label_borrows_the_title_and_nothing_literal`, and `now_playing_artist_borrows_the_artist`.
- Each section cites the Fixed entry it came from (title and `BUGS-FIXED.md`) so the reconstruction history stays traceable.

**Files touched.** `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes; the change is documentation only and touches no source.
- `docs/invariants.md` exists with the three sections above, each naming its enforcing code and at least one test.
- Every cited test name is a real `fn` in the named file, checked with `make test-one TEST=<name>` (or by `grep` before running it).
- The browse section records a Back path from every level above Artists; the freshness section records both the executor-thread check and the store-time re-check; the Now Playing section records the unknown-id fallback and the "Nothing" contract.

### Add an All Albums view listing the whole library flat (planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** A new top-level `All Albums` browse view lists every album in the user's library in one flat list, in library order. Pressing an album opens its songs exactly as the per-artist Albums view does, and Back from those songs returns to All Albums rather than to the artist list. It is reached by an `All Albums` button on the Artists view, beside `All Songs`, and Back from All Albums returns to Artists. Today albums are reachable only by drilling Artists → Albums, so there is no whole-library album view. The REST half reads `/v1/me/library/albums` through the same paginated collection helper as the other browse queries; the sample half walks the sample library's artists → albums in library order. Like `get_all_songs`, an all-albums resource has no artist id available in the collection mapping, so `Album::artist_id` is left empty; nothing that opens an all-albums row reads it (`get_songs_from_album` uses only the album id). Independent of the All Songs and search work, so it can land on its own.

**Approach.**

- `src/apple_music/rest.rs`:
  - Add `pub fn get_all_albums(&self, session: &MusicKitSession) -> Result<Vec<Album>, AppleMusicError>` requesting `{API_BASE}/me/library/albums` through `self.fetch`. Map each resource with `Resource::into_album("")` (the same consuming mapping `get_albums_by_artist` uses, which validates the album's id and name and leaves `artist_id` empty); document why the artist id is empty, as `get_all_songs` documents its empty `album_id`.
- `src/apple_music.rs`:
  - Add `pub async fn get_all_albums(&self) -> Result<Vec<Album>, AppleMusicError>` through the existing `browse` helper: `self.browse(|rest, session| rest.get_all_albums(session), sample_all_albums)`.
  - Add a private `fn sample_albums_in_library_order() -> impl Iterator<Item = &'static Album>` walking `sample_library().artists` then `albums_by_artist`, and make `sample_songs_in_library_order` start from that same album walk so the two orders cannot drift. Add `fn sample_all_albums() -> Vec<Album>` from the album walk.
- `src/ui/mod.rs`:
  - Add `CurrentView::AllAlbums`, `Message::ShowAllAlbums`, `Message::AllAlbumsLoaded(Vec<Album>)`, and `Message::AllAlbumsLoadFailed(String)`.
  - Add `all_albums: BrowseList<Album>` to `WinampPlayer`, initialised `BrowseList::new(false)` in `WinampPlayer::new`.
  - Add a `SongsOrigin { Album, AllAlbums }` enum and a `songs_origin: SongsOrigin` field initialised to `SongsOrigin::Album`. The `AlbumSelected` arm must resolve the pressed row against `player.all_albums` when `player.current_view == CurrentView::AllAlbums` and against `player.albums` otherwise, clone the selected album id, then set `songs_origin` from that same view before moving to `CurrentView::Songs`. The `Back` arm's `CurrentView::Songs` match uses `songs_origin` to return to `AllAlbums` or `Albums`; `AllAlbums` on Back returns to `Artists`.
  - `Message::ShowAllAlbums` arm: clear `search_active`, set `current_view = CurrentView::AllAlbums`, and call `fetch_level` on `player.all_albums` with the report `"loading all library albums"`, `|service| async move { service.get_all_albums().await }`, and the two new messages.
  - `AllAlbumsLoaded`/`AllAlbumsLoadFailed` store/fail through `player.all_albums`; the `BrowseReply` arm re-checks `player.all_albums.is_current(issued)` beside the existing lists.
  - `view`'s browse match gains a `CurrentView::AllAlbums` arm rendering `views::view_all_albums(&player.all_albums.items, …)` (no current-track argument, like the Albums arm), and pushes `views::view_all_albums_button()` on the Artists view beside `view_all_songs_button`.
- `src/ui/views/browse.rs`:
  - Add `pub fn view_all_albums(albums, epoch, loading, error)` built by `album_list` from `album_row` with the `CurrentView::AllAlbums` placeholder, and `pub fn view_all_albums_button()` returning `labeled_button("All Albums", Message::ShowAllAlbums)`.
  - Add `CurrentView::AllAlbums => "No albums"` to `empty_list_label` and `CurrentView::AllAlbums` to `BROWSE_VIEWS`.
- Tests:
  - `src/apple_music/rest/tests.rs`: `get_all_albums_maps_the_library_albums_collection` (pin `/v1/me/library/albums`, the mapped id/title, and the empty `artist_id`); `get_all_albums_follows_a_next_page`; `get_all_albums_reports_a_transport_error_bare`; `get_all_albums_rejects_a_nameless_album`.
  - `src/apple_music/tests.rs`: `get_all_albums_returns_every_sample_album_in_library_order` (album-1, album-2, album-3); `get_all_albums_uses_the_rest_library_when_signed_in`.
  - `src/ui/tests.rs`: `show_all_albums_fetches_the_flat_list_and_switches_view`; `all_albums_loaded_stores_the_flat_list`; `back_from_all_albums_returns_to_the_artists_list`; `show_all_albums_clears_an_active_search`; `album_selected_in_all_albums_loads_its_songs_and_back_returns_to_all_albums` (proves the press resolves against `all_albums` and the Back origin). Extend `seed_browse_lists`, `assert_browse_lists_empty`, `each_browse_failure_stores_its_report_in_its_own_list`, and `view_constructs_over_the_apps_full_input_space`'s failure shape to cover `all_albums`.
  - `src/ui/views/tests.rs`'s suite: a `view_all_albums_button` construction check, the `empty_list_label` arm, and the extended `BROWSE_VIEWS` loops.

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary::get_all_albums` requests `/v1/me/library/albums`, maps each resource's id and title, leaves `artist_id` empty, and follows a same-origin `next` page; a transport failure propagates its cause and a nameless album is an error naming its id.
- With no session, `AppleMusicService::get_all_albums` returns every sample-library album in library order (album-1, album-2, album-3); with a session it returns the REST collection.
- Pressing `All Albums` on the Artists view clears the buffer, clears an active search, switches to `CurrentView::AllAlbums`, and fetches; the loaded reply stores rows and a failed reply records the report on `all_albums` alone.
- Back from `CurrentView::AllAlbums` returns to `CurrentView::Artists`.
- Pressing an album row in `CurrentView::AllAlbums` fetches that album's songs into `songs` and shows `CurrentView::Songs`; Back then returns to `CurrentView::AllAlbums`, while Back from songs opened through the per-artist Albums view still returns to `CurrentView::Albums`.
- The Artists view shows the `All Albums` button and the view assembles every browse level, including an empty/failed `all_albums`, without panicking.

### Add an All Songs view listing the whole library flat (found 2026-10-09, done 2026-10-09)

**Goal.** A new top-level `All Songs` browse view lists every song in the user's Apple Music library in one flat list, in library order, and each row plays exactly like a browsed song. It is reached by an `All Songs` button on the Artists view, and Back returns there. Today the only way to reach a song is to drill Artists → Albums → Songs, and the search box returns only query matches, so there is no way to see the library's songs as a whole. The REST half reads the library-songs collection (`/v1/me/library/songs`, which follows `next` pages like the sibling collections) and the sample half lists the whole sample library, through the same signed-in/sample split `browse` already applies. It is independent of the existing search box and can land on its own.

**Approach.**

- `src/apple_music/rest.rs`:
  - Add `pub fn get_all_songs(&self, session: &MusicKitSession) -> Result<Vec<Song>, AppleMusicError>`. Build `{API_BASE}/me/library/songs` and call `self.fetch` — the same paginated collection helper `get_favorite_artists` and `get_albums_by_artist` use, so a library larger than one page is read in full under one `REQUEST_TIMEOUT`. Map each `Resource` exactly as `search_library` does (`Resource::into_song("")`, which moves the id, title, artist, and preview URL out of the parsed resource and validates its id and name) with `album_id: String::new()`, because a library-songs resource carries no album id and nothing that plays a song reads it.
- `src/apple_music.rs`:
  - Add `pub async fn get_all_songs(&self) -> Result<Vec<Song>, AppleMusicError>` answering through the existing `browse` helper: `self.browse(move |rest, session| rest.get_all_songs(session), sample_all_songs)`.
  - Extract the ordered library walk `sample_songs_matching` already spells out into a private `fn sample_songs_in_library_order() -> impl Iterator<Item = &'static Song>` (artists → `albums_by_artist` → `songs_by_album`, in library order), so the new `fn sample_all_songs() -> Vec<Song>` (`sample_songs_in_library_order().cloned().collect()`) and `sample_songs_matching` (the same walk, filtered) cannot drift apart.
- `src/ui/mod.rs`:
  - `Message::TrackSelected` resolves against `all_songs` when `current_view` is `AllSongs` and against `songs` otherwise, and `step_track` steps the same view's list, so a flat row plays — and Next/Previous move through — the flat list rather than the stale browsed songs.
  - Add `CurrentView::AllSongs`, `Message::ShowAllSongs`, `Message::AllSongsLoaded(Vec<Song>)`, and `Message::AllSongsLoadFailed(String)`.
  - Add `all_songs: BrowseList<Song>` to `WinampPlayer`, initialised `BrowseList::new(false)` in `WinampPlayer::new`.
  - `Message::ShowAllSongs` arm: clear `search_active`, set `current_view = CurrentView::AllSongs`, and issue `fetch_level(&mut player.all_songs, …, "loading all library songs".to_string(), |service| async move { service.get_all_songs().await }, Message::AllSongsLoaded, Message::AllSongsLoadFailed)`.
  - `AllSongsLoaded` stores via `player.all_songs.store`; `AllSongsLoadFailed` records via `player.all_songs.fail`; the `BrowseReply` arm re-checks `player.all_songs.is_current(issued)` for both, beside the existing three lists.
  - `Back` arm: `CurrentView::AllSongs => CurrentView::Artists`.
  - `view`'s browse match: `CurrentView::AllSongs` builds `views::view_all_songs` from `player.all_songs`, mirroring the `Songs` arm (including the second lock that hands the current track id to the view).
  - Push `views::view_all_songs_button()` in `view` when `current_view` is `CurrentView::Artists`, beside the Back/Retry buttons.
- `src/ui/views.rs`:
  - Add `pub fn view_all_songs(songs, epoch, loading, error, current_track)` that calls `song_list` from `song_row` with the `CurrentView::AllSongs` placeholder, like `view_songs`.
  - Add `pub fn view_all_songs_button()` returning `labeled_button("All Songs", Message::ShowAllSongs)`.
  - `empty_list_label`: add `CurrentView::AllSongs => "No songs"`.
  - Add `CurrentView::AllSongs` to `BROWSE_VIEWS` so the existing per-view construction and placeholder loops exercise it.
- Tests:
  - `src/apple_music/rest/tests.rs`: `get_all_songs_maps_the_library_songs_collection` (pin the requested URL, the mapped songs, and the empty `album_id`); `get_all_songs_follows_a_next_page`; `get_all_songs_reports_a_transport_error_bare`; `get_all_songs_rejects_a_nameless_song`.
  - `src/apple_music/tests.rs`: `get_all_songs_returns_every_sample_song_in_library_order` (song-1 through song-5); `get_all_songs_reads_the_rest_library_when_signed_in` (store a session over a stub transport and assert the REST songs, not the sample list).
  - `src/ui/tests.rs`: `show_all_songs_fetches_the_flat_list_and_switches_view`; `all_songs_loaded_stores_the_flat_list`; `back_from_all_songs_returns_to_the_artists_list`; `show_all_songs_clears_an_active_search`. Extend `seed_browse_lists` to populate `all_songs` and `assert_browse_lists_empty` to check it, so the shared full-input and failure-shape tests cover the new buffer.
  - `src/ui/views.rs`'s suite: a `view_all_songs_button` construction check and the `empty_list_label` arm, plus the extended `BROWSE_VIEWS` loops.

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views.rs`, `src/ui/tests.rs`.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary::get_all_songs` requests `/v1/me/library/songs`, maps every resource's id, title, artist, duration, and first non-blank preview URL, leaves `album_id` empty, and follows a same-origin `next` page.
- A transport failure propagates as the transport's own cause; a resource without a name is an error naming its id.
- With no session, `AppleMusicService::get_all_songs` returns every sample-library song in library order (song-1, song-2, song-3, song-4, song-5); with a session, it returns the REST collection.
- Pressing `All Songs` on the Artists view clears the buffer, switches to `CurrentView::AllSongs`, and fetches; the loaded reply stores the rows and a failed reply records the report.
- Back from `CurrentView::AllSongs` returns to `CurrentView::Artists`.
- Selecting a row in `CurrentView::AllSongs` plays the flat list's song, and Next/Previous step the flat list rather than the browsed `songs` buffer.

### Back the library search with the Apple Music REST endpoint (found 2026-10-09, done 2026-10-09)

**Goal.** `AppleMusicService::search_songs`, added by the sibling plan "Add a library search box to the playlist editor" (found 2026-10-09), queries the signed-in user's Apple Music library through the REST client and falls back to the sample-library filter when no session is stored — the same signed-in/sample split every browse query already uses. Land this only once the search box is the running build.

**Approach.**

- `src/apple_music/rest.rs`:
  - Add `pub fn search_library(&self, session: &MusicKitSession, query: &str) -> Result<Vec<Song>, AppleMusicError>`. Build `{API_BASE}/me/library/search?term={encode_path_segment(query)}&types=library-songs&limit=25` — Apple documents the library-search `limit` maximum as 25 (default 5), so the request asks for the documented maximum rather than the out-of-contract 100 this entry first named — and parse the search envelope `{"results":{"library-songs":{"data":[...],"next":...}}}` with new private `SearchEnvelope` / `SearchResults` / `SearchCollection` structs (the `library-songs` key renamed in serde). Apple omits `results.library-songs` for a search that matched nothing, so the field is an `Option` and an absent key maps to an empty list rather than a parse error. Map each `Resource` exactly as `get_songs_from_album` does, but with `album_id: String::new()`: a library-songs search result carries no album id, and nothing that plays a song reads `album_id` (only `get_songs_from_album` and the sample-library index use it). Follow the collection's `next` link with the same same-origin, `MAX_PAGES`, `page_context`, and page-notice rules `fetch` applies, so a query matching more than one page is read in full. To share that loop, split `fetch` into a thin wrapper over a new private `fetch_parsed` that takes the page parser as a closure, so the top-level and nested envelopes reuse the same pagination, deadline, and page-notice rules.
- `src/apple_music.rs`: rewrite `search_songs` to answer through the existing `browse` helper — `self.browse(move |rest, session| rest.search_library(session, &query), || sample_songs_matching(&query))` — keeping the blank-query guard from the sibling plan. A signed-in user now searches the real library; a signed-out one keeps the sample filter.
- `README.md`: extend the status and usage paragraphs to say the search box queries the signed-in Apple Music library.
- Tests:
  - `src/apple_music/rest/tests.rs`: `search_library_maps_the_search_envelope_and_percent_encodes_the_term` (assert the requested URL and the mapped songs, with the empty `album_id`); `search_library_returns_no_songs_when_the_results_carry_no_library_songs` (an absent and an empty `library-songs` both return an empty list); `search_library_follows_a_next_page`; `search_library_reports_a_transport_error_bare`; `search_library_rejects_a_nameless_song`.
  - `src/apple_music/tests.rs`: `search_songs_uses_the_rest_library_when_signed_in` — store a session through `authenticate_with` over a stub transport and assert the returned songs come from the REST search envelope, not the sample filter.

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary::search_library` requests `/v1/me/library/search?term=<percent-encoded query>&types=library-songs&limit=25` (a term with a space and a reserved character is encoded), maps the envelope's songs with their id, title, artist, duration, and first non-blank preview URL, and leaves `album_id` empty.
- A no-match search response that omits `results.library-songs` (or carries an empty `data`) returns an empty list rather than a parse error.
- A search response whose `library-songs.next` names a same-origin path is followed, and the songs of both pages are returned.
- A transport failure propagates as the transport's own cause; a resource without a name is an error naming its id.
- With a session stored, `AppleMusicService::search_songs` returns the REST results; with none, it returns the sample-library matches.

### Add a library search box to the playlist editor (found 2026-10-09, done 2026-10-09)

**Goal.** A search box sits above the browse list on every browse screen; typing a query and pressing Enter replaces the playlist-editor list with the matching songs, which play exactly like a browsed song. This is the first half of library search — the UI plus the service seam over the in-memory sample library. The sibling plan "Back the library search with the Apple Music REST endpoint" (found 2026-10-09) points the same `AppleMusicService::search_songs` method at the signed-in Apple Music library; land that one second.

**Approach.**

- `src/apple_music.rs`: add `pub async fn search_songs(&self, query: &str) -> Result<Vec<Song>, AppleMusicError>`. Trim the query and reject a blank one with an `AppleMusicError` (the query guard the id lookups use: a blank search has nothing to match and must not read as an empty library). Answer from the sample library with a new private pure `sample_songs_matching(query)` that keeps every sample-library song whose title or artist contains the query case-insensitively, in library order. (The sample library indexes songs by album in a `HashMap`, so the filter walks the ordered `artists` → `albums_by_artist` → `songs_by_album` chain rather than the map's unspecified iteration order, which would shuffle the matches between calls.)
- `src/ui/mod.rs`:
  - Add `search_query: String` (the box's text, `String::new()`) and `search_active: bool` (`false`) to `WinampPlayer`, initialised in `WinampPlayer::new`.
  - Add `Message::SearchChanged(String)` (stores the text) and `Message::SearchSubmitted(String)`.
  - `SearchSubmitted` trims the query; a blank one is `Task::none()`. Otherwise set `search_query`, set `search_active = true`, set `current_view = CurrentView::Songs`, call `player.songs.clear()`, and issue `fetch_into` with `player.songs.begin_fetch()`, context `format!("searching the library for {query:?}")`, fetch `|service| async move { service.search_songs(&query).await }`, and the existing `Message::SongsLoaded` / `Message::SongsLoadFailed`. Reusing the songs list keeps the `BrowseReply` generation guard, the `SongsLoaded`/`SongsLoadFailed` arms, and `view_songs` unchanged.
  - Clear `search_active` in the `ArtistSelected` and `AlbumSelected` arms. In `Back`, when `search_active` is set, clear it and set `current_view = CurrentView::Artists` (leave the search) instead of the Songs → Albums step, so Back never opens the empty or stale album list the search was issued from.
  - Push `views::view_search_box(&player.search_query)` above `main_content` (after the Back/Retry buttons) so the box is on every browse screen.
- `src/ui/views.rs`: add `pub fn view_search_box(query: &str) -> Element<'_, Message>` — an iced `TextInput` with the `"Search library"` placeholder, `on_input(Message::SearchChanged)`, `on_submit(Message::SearchSubmitted(query.to_string()))` (iced's submit callback carries a value, so the view captures the current box text), and the new `style::chrome_text_input_style`.
- `src/ui/style.rs`: add `chrome_text_input_style(status) -> text_input::Style` using the base-skin face, the light top-left bevel edge (iced's text-input style carries no shadow, so only the top-left half is drawn), and light text, mirroring `chrome_button_style`; focus sinks the face and reverses the edge.
- Tests:
  - `src/apple_music/tests.rs`: `search_songs_matches_the_sample_library_by_title_or_artist` (a title-only match, an artist-only match, case-insensitive, a match spanning two albums in library order, and a non-match absent); `search_songs_rejects_a_blank_query`.
  - `src/ui/tests.rs`: `search_submitted_replaces_the_songs_list_with_the_matches` (drive `Message::SearchSubmitted` through `update` and `drive_task`, then assert `player.songs.items` holds the matches and `current_view == CurrentView::Songs`); `search_submitted_with_a_blank_query_is_a_no_op`; `search_changed_stores_the_box_text`; `back_leaves_an_active_search_for_the_artists_list`; `entering_a_browse_level_clears_the_active_search`.
  - `src/ui/views.rs`'s suite: a `view_search_box` construction test like the other widget-construction tests; `src/ui/style.rs`'s suite: pin the text-input style's face, text, and bevel edges like `chrome_button_style_active_is_raised_chrome`.

**Files touched.** `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views.rs`, `src/ui/style.rs`, `src/ui/tests.rs`.

**Acceptance criteria.**

- `make check` passes.
- `AppleMusicService::search_songs("...")` returns the sample-library songs whose title or artist contains the query case-insensitively; a blank query returns an error naming the query.
- Driving `Message::SearchSubmitted(query)` through `update` and the returned task leaves `current_view == CurrentView::Songs`, stores the matches in `player.songs.items`, and marks the search active; a blank query changes nothing.
- Driving `Message::Back` while a search is active returns to `CurrentView::Artists` and clears the active flag; `Message::ArtistSelected` and `Message::AlbumSelected` also clear it.
- `views::view_search_box` builds an element with the `Search library` placeholder whose input callback is `SearchChanged` and whose submit callback is `SearchSubmitted` carrying the box text.

### Drive the volume slider through the audio backend (found 2026-10-09, done 2026-10-09)

**Goal.** Moving the volume slider, or pressing the up/down arrow keys, changes the loudness of the playing preview, and a preview starts at the slider's volume. Today `Message::VolumeChange`, `VolumeUp`, and `VolumeDown` only write `AppState::volume`; `AudioOutput` has no volume method, so every preview plays at rodio's full-volume default no matter where the slider sits. This is the volume twin of the just-planned transport wiring and the same seam gap: only `play_track` reaches the backend.

**Approach.**

- `src/audio.rs`:
  - Add `set_volume(&self, volume: f32) -> Result<(), AppleMusicError>` to `AudioOutput` (doc: sets the output gain, `1.0` is full volume; it applies to the current player and to the next `play`, and does nothing when nothing is loaded).
  - `SilentOutput::set_volume` logs and returns `Ok`.
  - Add `Command::SetVolume(f32)`; `RodioOutput::set_volume` sends it.
  - In `run_worker`, hold the current gain in a `volume: f32` (initial `1.0`), map `Command::SetVolume(volume)` to storing it and calling `set_volume` on the current `rodio::Player` when one exists, and call `set_volume(volume)` on the player a `Command::Play` creates, so a gain set while stopped still applies to the next track.
  - Extend `silent_output_reports_success_for_every_command`, `audio_with_fallback_falls_back_to_silence_when_open_fails`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` to cover `set_volume`.
- `src/test_support.rs`: add `AudioCall::SetVolume(f32)`; `RecordingAudio::set_volume` records it; `FailingAudio::set_volume` returns `Ok(())` like its `pause`/`stop`, so a `play` failure still surfaces `play`'s own error. Because `f32` is not `Eq`, drop `Eq` from `AudioCall`'s derive (keep `Debug, Clone, PartialEq`); nothing requires `Eq` (the only uses are `assert_eq!` on a `Vec<AudioCall>`).
- `src/apple_music.rs`:
  - Add a synchronous `pub(crate) fn set_output_volume(&self, volume: f32) -> Result<(), AppleMusicError>` that forwards to `self.audio.set_volume(volume)` and touches no shared state. It is synchronous because the UI already stores the clamped value under its own lock (see `ui::mutate_volume`), and `AudioOutput::set_volume` is a non-blocking channel send; an async transition here would let a drag's messages race the state lock.
  - In `play_track`, capture the state's current volume before dropping the lock and apply it before the preview starts: `self.audio.set_volume(volume).and_then(|()| self.audio.play(url))`, keeping the existing `Err` rollback. This makes the first play use the slider's value (the `AppState` default `0.5`) rather than rodio's full-volume default.
- `src/ui/mod.rs`:
  - Add `mutate_volume(player, mutation) -> Task<Message>` beside `mutate_state`: apply `mutation` to `AppState` under one synchronous `blocking_lock`, read back the clamped `state.volume()`, forward that value to `player.apple_music_service.set_output_volume(...)`, and log a backend error. The state write stays synchronous so a drag remains responsive and ordered; the backend forward is a non-blocking channel send, so it runs on the UI thread.
  - Replace the `VolumeChange`, `VolumeUp`, and `VolumeDown` arms with `mutate_volume` calls (still `AppState::set_volume` and `AppState::nudge_volume`), and drop those three names from `mutate_state`'s doc list. `BalanceChange` is unchanged — rodio's `Player` has no panner.
- Tests:
  - `src/apple_music/tests.rs`: add `set_output_volume_forwards_to_the_audio_backend` (one `AudioCall::SetVolume`, `AppState::volume` unchanged). Update `play_track_starts_the_preview_through_the_audio_backend` to expect `[SetVolume(0.5), Play(url)]`.
  - `src/ui/tests.rs`: update `track_selected_plays_the_songs_preview_url` to expect `[SetVolume(0.5), Play(url)]`; add `volume_messages_forward_to_the_audio_backend`, which drives `VolumeChange`, `VolumeUp`, and `VolumeDown` through `update` over a `player_with_audio(RecordingAudio)` and asserts the recorded `SetVolume` values and `AppState::volume` (including a `f32::NAN` `VolumeChange` storing and forwarding the clamped `0.0`).

**Files touched.** `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`.

**Acceptance criteria.**

- `make check` passes.
- `RodioOutput::set_volume(0.4)` sends `Command::SetVolume(0.4)`; `SilentOutput::set_volume` returns `Ok`; a closed worker reports "the audio thread is gone" from `set_volume`.
- `AppleMusicService::set_output_volume(0.3)` records one `AudioCall::SetVolume(0.3)` and leaves `AppState::volume` unchanged.
- `play_track` with a preview records `SetVolume` carrying the state's current volume immediately before `Play`.
- Driving `Message::VolumeChange(0.2)` leaves `AppState::volume` at `0.2` and records `SetVolume(0.2)`; `Message::VolumeUp`/`VolumeDown` record the nudged, clamped value; an out-of-range or `NaN` `VolumeChange` stores and forwards the clamped value.

### Drive the transport's Play/Pause/Stop through the audio backend (found 2026-10-08, done 2026-10-09)

**Goal.** Pressing the transport's Play/Pause, Play, Pause, and Stop controls — and their X, C, and V keys — drives the injected `audio::AudioOutput` as well as `AppState`, so pausing or stopping silences the playing preview and Play resumes a paused preview or restarts a stopped one. Before this, only `AppleMusicService::play_track` reached the backend: the service's `pause` was dead code and there was no `resume` or `stop`, so the preview kept sounding after Stop and Pause. This is the wiring the audio-seam plan (PLANS.md `## Done`, 2026-10-08) explicitly deferred.

**Approach.**

- `src/audio.rs`: add `resume(&self) -> Result<(), AppleMusicError>` to `AudioOutput` (doc: resumes a paused player; no effect when nothing is loaded — rodio's `Player::play` resumes, it does not restart). Implement it for `SilentOutput` (log and `Ok`); add `Command::Resume`; `RodioOutput::resume` sends it; the `run_worker` loop maps `Command::Resume` to `player.play()` on the current `rodio::Player`. Extend `silent_output_reports_success_for_every_command`, `audio_with_fallback_falls_back_to_silence_when_open_fails`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` to cover `resume`.
- `src/test_support.rs`: add `AudioCall::Resume`; `RecordingAudio::resume` records it; `FailingAudio::resume` returns `Ok(())` like its `pause`/`stop`.
- `src/apple_music.rs`: made the service the single owner of each transport transition, reusing the existing `AppState` mutators so none goes dead:
  - Moved `pause` out of the `#[allow(dead_code)]` block, narrowed that attribute to `next_track`/`previous_track`, and made the four transport methods `pub(crate)` so `ui` can call them.
  - Added a `transport: Arc<tokio::sync::Mutex<TransportState>>` field, where `TransportState { loaded, last_url }` records whether the backend holds a paused preview and the most recent preview URL. Every transport method and `play_track` holds this lock across the flag read, the backend call, and the flag write, so two overlapping toggles serialize instead of both reading the same flag.
  - `pause` calls `self.audio.pause()?`, then `self.state.lock().await.pause()`.
  - `resume` resumes a loaded player; when the backend discarded it (after a `stop`) it replays `last_url`, so Play after Stop restarts rather than claiming playback with nothing loaded; when no preview has ever played it logs and leaves the flag alone. On success it calls `self.state.lock().await.play()`.
  - `stop` calls `self.audio.stop()?`, clears `loaded`, then `self.state.lock().await.stop()`.
  - `toggle_play_pause` reads `is_playing` under the transport lock, calls `audio.pause()` (then `AppState::toggle_playing`) or `resume`, all under that one lock.
  - `play_track` also takes the transport lock and records `loaded`/`last_url`, clearing both when the backend rejects the preview.
- `src/ui/loading.rs`: add `transport_into(service, context, action) -> Task<Message>`: clone the service, await `action(service)`, report a backend error through a new pure `transport_failure_report(context, err)`, and map completion to `Message::TransportSettled`. Unlike `play_into` it carries no track or generation — a pause/stop cannot be superseded by a newer play — applies no timeout (the service's transport methods only lock and send a command to the audio worker), and does not prune the title index.
- `src/ui/mod.rs`: add the `Message::TransportSettled` variant with a no-op arm (its purpose is to make iced re-render after the async transition). Replace the four synchronous arms `PlayPause`, `Play`, `Pause`, and `Stop` with `transport_into(...)` calls to `toggle_play_pause`/`resume`/`pause`/`stop`; drop those four from `mutate_state`'s doc list and import `transport_into`.
- Tests:
  - `src/apple_music/tests.rs`: kept the two existing `pause` tests; added `resume_resumes_the_audio_backend`, `stop_stops_the_audio_backend`, `toggle_play_pause_pauses_while_playing`, `toggle_play_pause_resumes_while_paused`, `resume_after_stop_restarts_the_preview`, `resume_without_a_preview_leaves_the_transport_stopped`, and `concurrent_toggles_serialize_on_the_transport_lock` (a gated backend proves the second of two overlapping toggles waits on the transport lock; the test fails when the lock is dropped before the backend call).
  - `src/ui/tests.rs`: converted `play_pause_toggles_is_playing`, `stop_clears_is_playing`, and `play_and_pause_set_the_playback_flag_explicitly` into `#[tokio::test]`s that drive the returned task with `drive_task` before reading state, and updated their stale `blocking_lock` comments. Added `transport_messages_drive_the_audio_backend`, which drives each of the four messages through `update` and `drive_task` over a `player_with_audio(RecordingAudio)` and asserts the recorded calls (`Stop`, `Resume`, `Pause`, and a `Pause` from the toggle).
  - `src/ui/loading.rs`'s suite: added a pure-function test for `transport_failure_report`, like the existing `play_failure_report` test.
- `README.md`: updated the status and usage paragraphs to describe Pause/Stop driving the backend and Play resuming a paused preview or restarting a stopped one.

**Files touched.** `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/loading.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- With a `RecordingAudio`, `AppleMusicService::stop` records one `AudioCall::Stop` and clears `AppState::is_playing` while keeping `current_track`; `resume` after a paused preview records one `AudioCall::Resume` and sets `is_playing`; `toggle_play_pause` records `Pause` from playing and `Resume` from paused.
- `RodioOutput::resume` sends `Command::Resume`, and `SilentOutput::resume` returns `Ok`.
- `resume` after `stop` replays the remembered preview URL and sets `is_playing`; `resume` with no preview ever played records no call and leaves `is_playing` false.
- Two overlapping `toggle_play_pause` calls serialize on the transport lock: the second observes the first's flag write, so the pair ends playing with one `Pause` then one `Resume`.
- Driving `Message::Stop`, `Message::Pause`, `Message::Play`, and `Message::PlayPause` through `update` and the returned task leaves `AppState::is_playing` matching the control (false after Stop/Pause, true after Play when a preview is loaded, toggled after PlayPause when one is) and records the matching `AudioCall`.
- `transport_failure_report` names the context and includes the backend error.

### Document `make test-one` and report a build failure distinctly from a mistyped name (found 2026-10-08, done 2026-10-08)

`make test-one TEST=<name>` (added 2026-10-08) runs only the tests whose
names contain `<name>`, but the README's Development section never mentioned
it, and the target's no-match guard reported a build failure and a mistyped
name through one hedged message: `no test name contains 'X' (or the test build
failed)`.

**Goal.** The README documents the target, and `make test-one` reports a build
failure and a mistyped name distinctly.

**Approach.**

- `Makefile`: capture the `cargo test --list` output and check its exit status
  separately, so a failed build reprints the compiler diagnostics under a
  `make test-one: the test build failed` line, while a successful build with
  no match reports `no test name contains '<name>'`. Update the target's
  comment block to match. The cargo command is the `CARGO` variable so
  `test-one-guard-test` can substitute a stub.
- `README.md`: add `make test-one TEST=<name>` to the individual stages in the
  Development section.
- `Makefile`: `test-one-guard-test` invokes the recipe through `$(MAKE)` with
  an empty `TEST=` (the usage guard), `CARGO=false` (a failed build), and
  `CARGO=true` (a successful build that names no test), asserting each cause
  is reported distinctly, so removing any of the three guards fails
  `make check`.

**Acceptance criteria.**

- `make check` passes.
- `make test-one TEST=zzz_no_such_test` reports the no-match cause rather
  than the build-failure one.
- `make test-one-guard-test` exercises the empty-`TEST` usage guard and both
  the build-failure and no-match branches of the recipe and fails if any
  message regresses.

**Files touched.** `Makefile`, `README.md`.

### Reject a blank preview URL at the playback seam (found 2026-10-08, done 2026-10-08)

`AppleMusicService::play_track` validates its track id — a blank or
control-character id is rejected with a named error before shared state is
touched — but passed `preview_url` straight to the audio backend. A caller
handing `Some("")` (or only whitespace) made the backend attempt a request to
an empty URL, so the user saw a transport error that did not name the actual
defect. The REST layer already maps a blank preview to `None`, but the seam is
public and documented for a real backend to fill, so the guard belongs at the
seam too.

**Goal.** `play_track` rejects a present-but-blank `preview_url` with a named
`AppleMusicError` before committing state, while `None` keeps its documented
"no playable asset" behavior (state committed, no audio).

**Approach.**

- `src/apple_music.rs`: add `ensure_preview_url_is_valid(Option<&str>)`, which
  returns `Ok` for `None` or a non-blank URL and names the offending value
  (quoted with `{url:?}`) for a blank one; call it in `play_track` after
  `ensure_id_is_valid`, before the state lock. Update `play_track`'s doc comment
  and its `# Errors` list.

**Acceptance criteria.**

- `make check` passes.
- A test drives `play_track` with `Some("")` and `Some("   ")` mid-playback and
  sees the named error with shared state untouched.

**Files touched.** `src/apple_music.rs`, `src/apple_music/tests.rs`.

### Show the playing track's artist in the Now Playing bar (found 2026-10-08, done 2026-10-08)

Classic Winamp's main-window marquee reads "Artist - Title", and the Apple
Music browse response already carries each track's `attributes.artistName`.
Nostalgia's `Song` drops it — `rest::Attributes` reads only `name`,
`durationInMillis`, and `previews` — so the Now Playing bar can only name the
title. This carries the artist onto the model and shows it beside the title.
The playlist editor's rows keep their `title + length` layout, and the artist
never reaches the transport, browse navigation, volume, balance, or
equalizer.

**Goal.** `Song` carries `artist: String` — the performing artist's display
name, `""` when the source supplied none. The REST browse client parses
`attributes.artistName` into it; the sample library and test fixtures fill
it. The Now Playing bar renders `<artist> - <title>` when the artist is known
and falls back to the title alone when it is empty. No new dependency.

**Approach.**

- `src/library.rs`: add `pub artist: String` to `Song`, documented as the
  performing artist's display name with `""` meaning the source supplied
  none. Add `"artist": "The Sample Band"` to `song_payload()` and to
  `song_required_fields_payload()`, so `assert_every_field_required` still
  covers every required key and the round-trip test pins the field name.
- `src/apple_music/rest.rs`: add `#[serde(rename = "artistName")]
  artist_name: Option<String>` to `Attributes`, and set `artist:
  resource.artist_name.clone().filter(|name| !name.trim().is_empty())
  .unwrap_or_default()` in the `get_songs_from_album` mapping — an absent or
  blank name maps to `""`, the model's "no artist" sentinel, matching the
  `duration_ms`/`preview_url` treatment of missing source data.
- `src/sample_library.rs`: every `Song` literal gains `artist` — "The Sample
  Band" for the `album-1`/`album-2` songs, "Echo Chamber" for the `album-3`
  song — matching the album's `artist_id`. Add a `songs_carry_the_documented_artists`
  test mirroring `songs_carry_the_documented_durations`, pinning every song's
  artist in library order.
- `src/test_support.rs`: `sample_song`, `stepping_songs`,
  `single_song_album`, and `second_album_songs` gain
  `artist: "The Sample Band".to_string()`.
- The remaining `Song { … }` literals (`src/apple_music/tests.rs`,
  `src/apple_music/rest/tests.rs`, `src/ui/tests.rs`) gain the field.
- `src/apple_music/rest/tests.rs`: the `songs_from_album` fixture carries
  `"artistName": "The Sample Band"` and its expected `Song` carries that
  artist; a companion test pins an absent `artistName` to `""`.
- `src/ui/views.rs`:
  - Add `pub artist: String` to `KnownTrack`, documented beside `title` and
    `duration_ms`.
  - Add `pub fn now_playing_artist<'a>(tracks: &'a HashMap<String,
    KnownTrack>, current_track: Option<&str>) -> Cow<'a, str>`: the known
    track's artist borrowed from the map, or the `""` literal when the track
    is unknown or none is current. It mirrors `now_playing_label`, so the
    artist adds no per-frame `String` allocation.
  - Change `view_now_playing(label: Cow<'_, str>, artist: Cow<'_, str>, time:
    String)` to push the artist and a `" - "` separator before the title only
    when `artist` is non-empty, so an artist-less track still reads `Now
    Playing: <title>`.
  - In the `#[cfg(test)] mod tests`: the `known_tracks` helper takes
    `(id, artist, title)` triples; add value and borrow tests for
    `now_playing_artist`; pass the artist to the `view_now_playing` construct
    call.
- `src/ui/mod.rs`:
  - Add a `WinampPlayer::now_playing_artist` method mirroring
    `now_playing_label`.
  - The `TrackSelected` arm records `artist: song.artist.clone()` in the
    `KnownTrack` entry.
  - `view` resolves the artist in the same state-lock block as the label and
    passes it to `view_now_playing`.

**Files touched.** `src/library.rs`, `src/apple_music/rest.rs`,
`src/apple_music/rest/tests.rs`, `src/sample_library.rs`,
`src/test_support.rs`, plus the `Song` literals and the `KnownTrack` fixture
in `src/apple_music/tests.rs`, `src/ui/tests.rs`, and `src/ui/views.rs`, and
the Now Playing bar wiring in `src/ui/mod.rs`.

**Acceptance criteria.**

- `make check` passes.
- A `get_songs_from_album` stub response carrying `"artistName": "The Sample
  Band"` maps that string to `Song::artist`; a response omitting it (or
  carrying a blank one) maps to `""`.
- Every `sample_library()` song carries its album's artist name, pinned by
  `songs_carry_the_documented_artists`.
- `now_playing_artist` returns the known track's artist, `""` for an unknown
  id, and `""` when no track is current; the known case borrows from the map
  (`Cow::Borrowed`).
- `view_now_playing` builds with a non-empty artist and with an empty one.
- The `now_playing_label` value and borrow tests still pass unchanged, and
  the browse-away regression
  (`now_playing_label_keeps_the_track_name_after_browsing_to_another_album`)
  still passes.

### Bound the audio worker's preview download with a timeout (found 2026-10-08, done 2026-10-08)

The audio seam's worker downloaded the preview with `ureq::get(url)`, whose
default agent leaves every network timeout `None`. A server that accepted the
connection and then stalled blocked the worker thread forever, and because the
worker serves commands one at a time, every later play, pause, and stop queued
behind it. The REST client already bounded its requests; this applies the same
bound to the app's other network call.

**Goal.** A preview download that stalls fails with a logged error within a
fixed bound, so the worker keeps serving later commands, and the production
agent still reuses its pooled connection.

**Approach.**

- `src/audio.rs`: add `PREVIEW_TIMEOUT: Duration = Duration::from_secs(30)`,
  matching the REST client's `REQUEST_TIMEOUT`.
- Add `preview_agent() -> &'static ureq::Agent` (a `OnceLock`, built by
  `agent_with_timeout`) so consecutive plays reuse one pooled agent, and
  `agent_with_timeout(timeout)`, split out so a test can bound a download
  against a stalled loopback server without waiting out the production 30s.
- `download_and_decode(agent, url)` takes the agent and calls `agent.get(url)`;
  the worker passes `preview_agent()`. The download error already logs on the
  worker, so a timeout reads like any other download failure.

**Acceptance criteria.**

- `make check` passes.
- A stalled loopback preview server makes `download_and_decode` return `Err`
  within a short injected bound, asserted from a worker thread so an unbounded
  download fails the test instead of hanging the suite.

**Files touched.** `src/audio.rs`.

### Add the audio-output seam and a rodio backend (found 2026-10-08, done 2026-10-08)

The audio-output decision (QUESTIONS.md `## Answered`, 2026-10-08) scopes
playback to the Apple Music **preview**; this lands the playback half behind an
injectable seam, with `rodio` as the production backend. Rodio 0.22's API
differs from the names first sketched here — the device is a `MixerDeviceSink`
from `DeviceSinkBuilder::open_default_sink()`, playback goes through
`rodio::Player`, and the preview decodes with `Decoder::new_mp4` — so the
approach below names the API that landed. The transport's Pause and Stop
buttons still update only UI state; wiring them to the service's `pause`/`stop`
seam is left to a later plan.

**Goal.** `AppleMusicService` plays real audio through an injectable
`AudioOutput` seam; the production implementation uses `rodio`, `play_track`
starts the selected song's preview, `pause` pauses it, and a recording stub
stands in under test. Depends on "Carry the Apple Music preview URL on
`Song`" (the URL must be on `Song` first).

**Approach.**

- `Cargo.toml`: `rodio = { version = "0.22.2", default-features = false,
  features = ["playback", "mp4"] }`. The `mp4` feature already pulls
  `symphonia-isomp4` and `symphonia-aac` (Apple's preview is AAC in M4A), and
  dropping the default features leaves out recording and the unused decoders.
- A new `audio` module (the `AudioOutput` trait, `SilentOutput`, `RodioOutput`, and `audio_with_fallback` symbols), registered by `pub mod audio;` in `src/main.rs`:
  - `pub trait AudioOutput: Send + Sync` with `fn play(&self, url: &str) ->
    Result<(), AppleMusicError>`, `fn pause(&self) -> Result<(),
    AppleMusicError>`, and `fn stop(&self) -> Result<(), AppleMusicError>`.
  - `pub struct SilentOutput`, whose methods log and return `Ok(())`: the
    fallback when no device opens, and the production stand-in for the browse
    tests.
  - `pub struct RodioOutput`, owning a `Mutex<std::sync::mpsc::Sender<Command>>`
    to a worker `std::thread`. Rodio 0.22 opens the device through
    `DeviceSinkBuilder::open_default_sink()` (a `MixerDeviceSink`) and plays
    through `rodio::Player`; keeping both on the worker makes the seam
    `Send + Sync` regardless of the platform handle's thread bounds. The
    sender sits behind a `Mutex` because the standard sender is `Send` but not
    `Sync`. `Command` is `Play(String)`, `Pause`, `Stop`. The worker downloads
    the URL with `ureq`, buffers it, and decodes it with
    `rodio::Decoder::new_mp4(Cursor<Vec<u8>>)`, appending to a fresh `Player`;
    a download/decode failure is logged with `eprintln!` (the trait call
    itself fails only when the worker is gone). `RodioOutput::new() ->
    Result<Self, AppleMusicError>` opens the default device on the worker and
    reports success/failure back through a `std::sync::mpsc` channel, so a
    device-less machine gets `Err` rather than a panic.
  - `pub(crate) fn audio_with_fallback(open: impl FnOnce() ->
    Result<Arc<dyn AudioOutput>, AppleMusicError>) -> Arc<dyn AudioOutput>`:
    returns the opened output, or a `SilentOutput` when `open` fails. Pure and
    injectable so the fallback is testable without a device.
- `src/apple_music.rs`:
  - Add `audio: Arc<dyn audio::AudioOutput>` to `AppleMusicService`.
  - `with_transport` builds `SilentOutput`; add
    `with_audio(state, transport, audio)` for the playback tests; `new(state)`
    builds `RodioOutput` through `audio_with_fallback`, then calls
    `with_audio`.
  - `play_track(&self, track_id: &str, preview_url: Option<&str>, is_current:
    …)`: keep the id guard and the `is_current`-guarded state commit, then call
    `self.audio.play(url)` when `preview_url` is `Some` and propagate an audio
    error. `pause` calls `self.audio.pause()` before clearing `is_playing`.
  - Update the doc comments that call playback a shared-state-only stub.
- `src/ui/mod.rs`: the `TrackSelected` arm passes `song.preview_url.as_deref()`
  to `play_track`.
- Tests: `AudioCall`/`RecordingAudio`/`FailingAudio` live in
  `src/test_support.rs`, shared by the `apple_music` and `ui` suites. The
  `apple_music` suite asserts `play_track(…, Some(url), …)` records one
  `Play(url)` and commits state, `play_track(…, None, …)` records none and
  still commits, a failing backend's error propagates, and `pause` records
  `Pause`. The `ui` suite drives `TrackSelected` end to end and asserts the
  backend saw the selected `Song::preview_url`. The service constructors in
  both suites use `with_transport` (silent audio), so no test opens an audio
  device.

**Files touched.** `Cargo.toml`, `README.md`, `src/main.rs`, the new
`src/audio.rs` (the `AudioOutput`, `SilentOutput`, `RodioOutput`, and
`audio_with_fallback` symbols), `src/apple_music.rs`,
`src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/mod.rs`,
`src/ui/tests.rs`.

**Acceptance criteria.**

- `make check` passes, including on a machine with no audio device (the
  `RodioOutput` tests cover `new` returning `Err` without panicking; they do
  not assert a device is present).
- `audio_with_fallback` with a failing opener returns a `SilentOutput` whose
  `play`/`pause`/`stop` return `Ok`.
- With a recording `AudioOutput`, `play_track("song-1",
  Some("https://example.test/preview.m4a"), || true)` records one `Play` with
  that URL and sets `AppState::current_track`/`is_playing`.
- `play_track(…, None, …)` records no `Play` but still commits the state
  transition.
- `pause` records a `Pause`.
- The UI passes the selected `Song::preview_url` to `play_track`.

### Reject a `make test-one TEST=<name>` that matches no test name (found 2026-10-08, done 2026-10-08)

`make test-one` forwarded its filter to libtest, which exits `0` when the
filter matches zero tests — so a typo (`TEST=clamp_with_nan_fallback`, a
function name rather than a behavior name) printed "running 0 tests" and a
green result, reading as a pass.

**Goal.** A `TEST` that matches no test name fails the target with a message,
while a real name still runs only the matching tests.

**Approach.**

- `Makefile`: `test-one` lists the tests the filter would select
  (`cargo test --locked -- --list "$(TEST)"`) and requires at least one line
  naming a test before the filtered run. The match is anchored to libtest's
  test-name lines (`: test$`) so the trailing summary (`N tests, 0
  benchmarks`) cannot satisfy the guard. The list command's output is
  captured together with its stderr and its exit status checked separately,
  so a build failure is reported as such rather than as a mistyped name. The
  match pattern lives in the `TEST_LIST_HAS_TEST` variable.
- `Makefile`: `test-one-guard-test` feeds that pattern a synthetic list with
  a test line and a summary-only list, asserting the first matches and the
  second does not; it is added to `check`, so a guard that reintroduces the
  summary-line false pass fails the gate.

**Acceptance criteria.**

- `make check` passes.
- `make test-one TEST=<name>` with a real test name runs only the matching
  tests and exits zero.
- `make test-one` with no `TEST` exits non-zero with a usage line.
- `make test-one TEST=benchmarks` (a value that appears only in libtest's
  summary line) and `make test-one TEST=no_such_test` each exit non-zero with
  a "no test name contains" message and run no tests.
- `make test-one-guard-test` passes.

- Carry the Apple Music preview URL on `Song` (found 2026-10-08, done 2026-10-08; commit 8e9faf3)
- Name the page when a paginated Apple Music browse fails (found 2026-10-08, done 2026-10-08; commit 71853ef)
- Show each track's length in the playlist editor (found 2026-10-08, done 2026-10-08; commit f695290)
- Read a later Apple Music error entry when the first names no cause (found 2026-10-08, done 2026-10-08; commit bb1696d)
- Follow a browse collection's next page so a large library is read in full (found 2026-10-08, done 2026-10-08; commit a330d17)
- Show the current track's duration in the Now Playing bar (found 2026-10-08, done 2026-10-08; commit 98aa4b3)
- Log a notice when a browse response has an unread next page (found 2026-10-08, done 2026-10-08; commit 210deb5)
- Add a `make test-one` target for running one focused test (found 2026-10-08, done 2026-10-08; commit 277625f)
- Name the user-token shape defect when the sign-in callback is rejected (found 2026-10-08, done 2026-10-08; commit db297ed)
- Name the envelope, not "invalid JSON", when a valid-JSON body does not match (found 2026-10-08, done 2026-10-08; commit ad94f37)
- Report just the HTTP status when an Apple Music error body's cause is blank (found 2026-10-08, done 2026-10-08; commit 1723afd)
- Add a Winamp Shuffle toggle that randomizes Next (found 2026-10-08, done 2026-10-08; commit 7c299a7)
- Reject a blank resource id from an Apple Music browse response (found 2026-10-08, done 2026-10-08; commit e19867b)
- De-duplicate the query label in a failed browse report (found 2026-10-08, done 2026-10-08; commit 196e68a)
- Add a Winamp balance slider beside the volume slider (found 2026-10-08, done 2026-10-08; commit 8ee7986)
- Add the Winamp title-bar clutter bar and shade button (found 2026-10-08, done 2026-10-08; commit 7bcb23f)
- Answer the browse queries from the Apple Music REST API when signed in (found 2026-10-08, done 2026-10-08; commit 4a4f2e6)
- Add the Apple Music REST browse client (found 2026-10-08, done 2026-10-08; commit c63808d)
- Wire the MusicKit session into `AppleMusicService` (found 2026-10-08, done 2026-10-08; commit cf514ca)
- Add the MusicKit loopback authorization module (found 2026-10-08, done 2026-10-08; commit c15f6b6)
- Add classic Winamp main-window keyboard shortcuts for transport and volume (found 2026-10-08, done 2026-10-08; commit 97f6658)
- Add Winamp window-shade ("roll up") mode (found 2026-10-08, done 2026-10-08; commit 1e46f04)
- Add Winamp equalizer preset curves and a preset pick list (found 2026-10-07, done 2026-10-07; commit f7647ca)
- Add a Winamp custom title bar and drop the OS window frame (found 2026-10-07, done 2026-10-07; commit 81eb991)
- Frame the browse list as a sunken Winamp playlist editor (found 2026-10-07, done 2026-10-07; commit 74bbe47)
- Style the volume and equalizer sliders as sunken Winamp grooves with raised chrome thumbs (found 2026-10-07, done 2026-10-07; commit 4858b11)
- Style the transport buttons as raised Winamp chrome (found 2026-10-07, done 2026-10-07; commit dd0a5c5)

- Add a Winamp two-tone bevel layer and frame the Now Playing and equalizer panels (found 2026-10-06, done 2026-10-06; commit 8984e90)
- Add a Winamp 2.x base-skin palette and apply it as the app theme (done 2026-10-06; commit 9d74276)
- Add the Winamp equalizer panel: on/off, preamp, and ten band sliders (done 2026-10-06; commit 82b9757)
- Add a Repeat toggle to the transport controls (done 2026-10-06; commit d2f72bc)
- Highlight the currently playing song in the Songs browse view (done 2026-10-06; commit 7e6d83b)
- Add a Stop button to the transport controls (done 2026-10-06; commit 8e91e1e)
- Show the song's title in the Now Playing bar, not its raw id (done 2026-10-05; commit 7d29183)
- Make Previous/Next step through the songs of the current album (done 2026-10-05; commit 4c8e223)
- Add a working volume slider to the transport controls (done 2026-10-05; commit 7029804)
- Make the app build on iced 0.14 and render a browsable sample library (done 2026-10-04; commit 5af3f08)
