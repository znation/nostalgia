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

### Interpolate the equalizer's response curve smoothly between bands (planned 2026-10-26 by the plan role)

**Goal.** The mockup's traced response line curves smoothly between the ten bands, while
`response_curve_offsets` in `src/ui/views/equalizer.rs` joins adjacent band gains with straight
line segments. Ease each adjacent-band span so the trace meets every band at a zero slope and
still passes exactly through that band's gain, matching the mockup's rounded curve instead of the
visible corners a piecewise-linear trace leaves at the band positions. The queued guide plan
draws its 0 dB line and band markers behind the trace and leaves the trace's shape alone, so this
change touches only the offsets and leaves `response_curve_row` and the guide layers untouched.

**Approach.**

- `src/ui/views/equalizer.rs`: in `response_curve_offsets`, replace the linear `fraction` in the
  adjacent-band blend with a cubic smoothstep, `fraction * fraction * (3.0 - 2.0 * fraction)`.
  Keep the existing column-to-band mapping, the `clamp_gain` normalization, the `columns == 0`
  and `columns == 1` guards, and the exact endpoint behavior: column 0 and the last column still
  return the first and last bands' normalized gains, and every band position still returns that
  band's own offset. Because smoothstep is monotone over `0.0..=1.0` and the endpoint offsets are
  already clamped, every returned offset stays in `0.0..=1.0` without a second clamp. Update the
  function's doc comment and the module's trace description to say the line is eased between
  bands rather than straight, and note that a flat curve, all-minimum curve, and all-maximum
  curve are unchanged.
- `src/ui/views/tests/equalizer.rs`: extend
  `response_curve_offsets_follow_the_band_gains` with the eased-shape checks and keep its flat,
  all-minimum, all-maximum, endpoint, and zero/one-column assertions. Use a curve with
  `bands[4] = GAIN_MIN_DB`, `bands[5] = GAIN_MAX_DB`, and every other band `0.0`, laid out with
  `(BAND_COUNT - 1) * 4 + 1` columns so each band lands on exactly column `index * 4`. Assert the
  band columns still equal that band's gain (the curve passes through every band), and assert the
  three columns between those two bands have offsets `0.15625`, `0.5`, and `0.84375` (the
  smoothstep values at `0.25`, `0.5`, and `0.75`) rather than the linear `0.25`, `0.5`, `0.75`,
  proving the span is eased. Assert every offset in that curve lies in `0.0..=1.0`. Leave
  `response_curve_row_keeps_its_height_and_draws_one_segment_per_column` as it is: the row's
  geometry, height, and per-column segment count do not change.
- `docs/usage.md`: in the equalizer-panel description, say the response curve is eased smoothly
  between the band gains rather than drawn as straight segments.
- `docs/invariants.md` `## UI layout`: extend the response-curve bullet with the eased
  interpolation, name `views::equalizer::response_curve_offsets` in **Enforced by**, add the
  updated `response_curve_offsets_follow_the_band_gains` to **Pinned by**, and add a one-line
  History note.

**Files touched.** `src/ui/views/equalizer.rs`, `src/ui/views/tests/equalizer.rs`,
`docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `response_curve_offsets` still returns exactly the requested column count, `0.0` for an
  all-minimum curve, `0.5` for a flat curve, and `1.0` for an all-maximum curve, with column 0
  and the last column equal to the first and last bands' normalized gains.
- The trace passes through every band's normalized gain at that band's column, and each span
  between adjacent bands is eased with the cubic smoothstep rather than interpolated linearly
  (the quarter-span offsets are `0.15625` and `0.84375` for a full-span rise).
- Every offset stays within `0.0..=1.0`, including a curve whose gains sit at both ends of the
  range, so no eased overshoot can leave the plot.
- `response_curve_row` keeps its fixed height, one segment per column, and the guide layers the
  queued plan adds; no gain, preset, guide, message, or audio behavior changes.
- The project's check passes.

## Done
