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

### Light the LCD indicator letters for the modes that are on, like Winamp (planned 2026-10-26 by the plan role)

**Goal.** The LCD's left-edge `O A I D V` strip draws all five letters in the same bright
`theme::LCD_GREEN`, so it cannot tell you that Always on top is engaged, the File info panel is
open, or the visualizer is showing, and its decorative `O` and `D` read as live as the rest.
Make each live letter report its mode: `A` lit while always-on-top is on, `I` lit while the File
info panel is open, and `V` lit while the LCD visualizer is shown; draw the inactive letters,
including the decorative `O` and `D`, in `theme::LCD_GREEN_DIM`. The strip then reports its three
live modes at a glance instead of drawing every letter the same. The letters keep their order,
fixed width, and press messages; this is presentation plus the state the view already holds, with
no new toggle or playback behavior. The plan is independent of the waiting Jump to File paging
plan and touches none of its files.

**Approach.**

- `src/ui/views/now_playing/indicator.rs`: add `#[derive(Debug, Clone, Copy, Default, PartialEq,
  Eq)] pub struct IndicatorModes { pub always_on_top: bool, pub file_info_visible: bool, pub
  visualizer_visible: bool }`, the three flags the strip lights. Add the pure `pub fn
  indicator_label_color(label: &str, modes: IndicatorModes) -> Color`: `A` is
  `theme::LCD_GREEN` when `always_on_top`, `I` when `file_info_visible`, and `V` when
  `visualizer_visible`; every other case, including `O` and `D`, is `theme::LCD_GREEN_DIM`. Change
  `lcd_indicator_strip` to take `modes: IndicatorModes` and color each letter through the helper,
  keeping the same `MouseArea`/message wiring. Update the module and function docs to say the live
  letters light only while their mode holds and `O`/`D` stay dim.
- `src/ui/views/now_playing.rs`: re-export `IndicatorModes` and `lcd_indicator_strip` from the
  `indicator` module, add a `modes: IndicatorModes` parameter to `view_now_playing` and
  `now_playing_content`, and pass it to `lcd_indicator_strip`. Update both docs to describe the
  modes parameter beside the already-resolved readouts; the function keeps its
  `#![allow(clippy::too_many_arguments)]`.
- `src/ui/views/mod.rs`: add `IndicatorModes` to the `pub use now_playing::{...}` re-export list.
- `src/ui/view.rs`: pass `views::IndicatorModes { always_on_top: player.always_on_top,
  file_info_visible: player.file_info_visible, visualizer_visible: player.visualizer_visible }`
  to `view_now_playing`.
- Tests: update the existing `view_now_playing`/`now_playing_content` call sites with an
  `IndicatorModes` argument: `captured_now_playing_texts` in `src/ui/views/tests/mod.rs`,
  `captured_content_with_spectrum` and `now_playing_content_draws_the_indicator_strip_beside_the_title`
  in `src/ui/views/tests/lcd_display.rs`/`src/ui/views/tests/now_playing.rs`, and
  `now_playing_bar_and_browse_action_buttons_construct` in `src/ui/views/tests/browse.rs` pass
  `IndicatorModes::default()`. In `src/ui/views/tests/now_playing.rs`, add
  `indicator_label_color_lights_only_the_active_mode` pinning all five labels under each single
  mode plus all-off and all-on (and that `O`/`D` never light), and
  `lcd_indicator_strip_lights_the_active_letters_and_dims_the_rest`, which lays the strip out with
  `layout_capture::capture_text_and_pixels` for all-off and all-on and asserts the lit letters'
  `max_green_in` clear the dim ones' by more than 100, mirroring
  `now_playing_status_row_lights_the_reported_channel_mode`. Keep the existing order, width, and
  press tests; let `captured_indicator_texts` pass `IndicatorModes::default()`.
- `docs/usage.md`: in the LCD strip sentence, add that each live letter is lit while its mode is on
  (`A` for always-on-top, `I` while File info is open, `V` while the visualizer is shown) and dim
  otherwise, and that `O` and `D` stay dim.
- `docs/invariants.md` `## Now Playing label`: extend the indicator-strip bullet with the
  lit/dim rule and its `LCD_GREEN`/`LCD_GREEN_DIM` colors, name
  `views::now_playing::indicator::indicator_label_color` and `IndicatorModes` in **Enforced by**,
  and add the two new tests to **Pinned by**.

**Files touched.** `src/ui/views/now_playing/indicator.rs`, `src/ui/views/now_playing.rs`,
`src/ui/views/mod.rs`, `src/ui/view.rs`, `src/ui/views/tests/mod.rs`,
`src/ui/views/tests/lcd_display.rs`, `src/ui/views/tests/now_playing.rs`,
`src/ui/views/tests/browse.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- In every playback state the strip draws `O`, `A`, `I`, `D`, `V` in that order at its fixed
  width, and presses still publish `ToggleAlwaysOnTop` for `A`, `ToggleFileInfo` for `I`,
  `ToggleVisualizer` for `V`, and nothing for `O` and `D`.
- `A` draws `theme::LCD_GREEN` exactly while `always_on_top` holds, `I` exactly while
  `file_info_visible` holds, and `V` exactly while `visualizer_visible` holds; every other letter
  and every inactive live letter draws `theme::LCD_GREEN_DIM`.
- The rendered strip shows the lit letters brighter than the dim ones, while the text order and
  bounds are unchanged.
- The project's check passes.

### Draw the equalizer's 0 dB guide and band markers behind the response curve (planned 2026-10-26 by the plan role)

**Goal.** `docs/screenshots/equalizer.png` draws the response plot as a reference grid: a dotted
horizontal line at 0 dB and a vertical line at each of the ten bands, with the response trace over
them. `response_curve_row` in `src/ui/views/equalizer.rs` draws only the green trace on
`theme::LCD_BACKGROUND`, so a shaped curve gives no flat reference and its columns cannot be lined
up with the band sliders. Add the same reference grid in `theme::LCD_GREEN_DIM` behind the trace,
using `iced::widget::stack` (already available; no new dependency or feature). The trace keeps its
color, columns, and height, and no gain, preset, curve-shape, message, or audio behavior changes.

**Approach.**

- `src/ui/views/equalizer.rs`: add `pub(super) const RESPONSE_CURVE_GUIDE_THICKNESS: f32 = 1.0;`
  for the dot size and marker width, documented as distinct from
  `RESPONSE_CURVE_LINE_HEIGHT` so the trace's container filter still counts only trace segments.
- In `response_curve_row`, build the guide layers inline so the existing generic
  theme/renderer bounds stay in one function. The band-marker layer is a `Row` filling the width
  and height: for `0..BAND_COUNT`, push a `Container` of width
  `RESPONSE_CURVE_GUIDE_THICKNESS`, height `Length::Fill`, and the dim guide style, with a
  `Space::new().width(Length::Fill)` between consecutive markers (none after the last), so the
  ten markers sit at the bands' `i / (BAND_COUNT - 1)` positions from the plot's left edge to its
  right edge. The 0 dB layer is a `Column` of height `RESPONSE_CURVE_HEIGHT`: a top `Space` of
  `(RESPONSE_CURVE_HEIGHT - RESPONSE_CURVE_GUIDE_THICKNESS) / 2.0`, a `Row` of
  `RESPONSE_CURVE_COLUMNS` equal-width cells each centering a `RESPONSE_CURVE_GUIDE_THICKNESS`
  square in the dim guide style (`Container::new(dot).center_x(Length::Fill)`), and the matching
  bottom `Space`. Use one inline closure for both layers' style, painting
  `theme::LCD_GREEN_DIM`.
- Keep the existing 64-column trace `Row` exactly as it is, but move its background from its own
  `Container` to the outer one: combine the layers with
  `iced::widget::stack([markers, zero_guide, trace])`, markers and guide first and the trace last
  so it draws on top, size the stack `Length::Fill` by
  `Length::Fixed(RESPONSE_CURVE_HEIGHT)`, and wrap it in the existing
  `theme::LCD_BACKGROUND` `Container` at that fixed height. Update the module and
  `response_curve_row` docs to describe the dim guide, its 0 dB and band positions, and that the
  trace draws over it.
- `src/ui/views/tests/equalizer.rs`: add
  `response_curve_guide_draws_the_zero_db_line_and_band_markers_behind_the_trace`. Lay out
  `response_curve_row` for a shaped curve (first half `GAIN_MIN_DB`, second half `GAIN_MAX_DB`)
  with the null renderer and `CapturedContainers`: assert the root height is
  `RESPONSE_CURVE_HEIGHT`; count `RESPONSE_CURVE_COLUMNS` containers of
  `RESPONSE_CURVE_GUIDE_THICKNESS` square (the dots) and `BAND_COUNT` containers
  `RESPONSE_CURVE_GUIDE_THICKNESS` wide by `RESPONSE_CURVE_HEIGHT` tall (the markers); assert the
  markers' `x` values run from the plot's left edge to its right edge in equal steps; and assert
  every dot's center is within 1 px of the plot's vertical midline. Then render the same row with
  `layout_capture::capture_text_and_pixels` and the headless renderer: assert a marker's top area
  is brighter than the same-height background between markers, and a bottom-left trace segment
  is brighter than the guide dots there, using `max_green_in`, so the dim guide is drawn and the
  bright trace stays on top. Render a flat curve too and assert its midline is
  `theme::LCD_GREEN` rather than the dim guide, proving the trace covers the guide where they
  overlap. Leave `response_curve_offsets_follow_the_band_gains` and
  `response_curve_row_keeps_its_height_and_draws_one_segment_per_column` unchanged; the guide
  uses `RESPONSE_CURVE_GUIDE_THICKNESS`/`RESPONSE_CURVE_HEIGHT`, so the existing height-2 filter
  still sees exactly the 64 trace segments.
- `docs/usage.md`: in the equalizer-panel sentence, add that the plot marks the 0 dB midline and
  the ten bands with dim guides behind the green trace.
- `docs/invariants.md` `## UI layout`: extend the response-curve bullet with the dotted 0 dB line
  and the band markers in `theme::LCD_GREEN_DIM`, behind the `theme::LCD_GREEN` trace; name
  `views::equalizer::response_curve_row` in **Enforced by** and the new test in **Pinned by**; add
  a one-line History note.

**Files touched.** `src/ui/views/equalizer.rs`, `src/ui/views/tests/equalizer.rs`,
`docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The plot keeps its fixed height and draws exactly one 2 px `theme::LCD_GREEN` segment per
  column at the same offsets as before.
- Behind the trace it draws a `RESPONSE_CURVE_COLUMNS`-dot dotted line centered on the 0 dB
  midline and `BAND_COUNT` full-height 1 px markers evenly spaced from the plot's left edge to its
  right edge, all in `theme::LCD_GREEN_DIM`.
- The trace draws over the guide: rendered trace pixels keep `theme::LCD_GREEN`, guide pixels stay
  dimmer, and a flat curve's midline is the bright trace rather than the guide.
- The guide adds no messages and changes no gain, preset, curve, or audio behavior; the existing
  curve tests pass unchanged.
- The project's check passes.

## Done
