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
