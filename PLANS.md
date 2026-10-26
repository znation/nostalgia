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

### Split the transport's Play/Pause button into separate Play and Pause buttons, like Winamp (planned 2026-10-26 by the plan role)

**Goal.** `docs/screenshots/transport.png` and `hero.png` draw distinct Play (`▶`) and Pause (`II`)
buttons between Previous and Stop, but `transport_glyphs` in `src/ui/views/transport.rs` builds one
combined Play/Pause button whose glyph swaps with `is_playing`. Split it into two fixed buttons that
dispatch the existing explicit `Message::Play` (the X key's play/resume) and `Message::Pause` (the C
key's pause), in the main transport row, the shade strip, and the library footer. This is
presentation and message wiring only: no new `Message`, handler, state field, setting, or audio
behavior, and the LCD's own transport glyph, the X/C keys, and the system media keys stay as they
are. The mockup's sixth glyph (eject) stays out of scope because the app has no open-a-file action.

**Approach.**

- `src/ui/views/transport.rs`: make `transport_glyphs()` a five-entry, playback-independent spec in
  row order: Previous `("◀◀", "Previous", 34.0, Message::PreviousTrack)`, Play
  `("▶", "Play", 30.0, Message::Play)`, Pause `("II", "Pause", 30.0, Message::Pause)`, Stop
  `("■", "Stop", 30.0, Message::Stop)`, Next `("▶▶", "Next", 34.0, Message::NextTrack)`. Delete
  `play_pause_glyph` and `play_pause_word`, and drop `is_playing` from `transport_glyphs`,
  `transport_buttons`, and `view_transport_controls`. `transport_buttons` returns seven elements
  (the five tooltip-wrapped glyphs, then the Repeat and Shuffle toggles). Update the module and
  function docs from four glyphs to five. Also update the `Message::Play`/`Message::Pause` variant
  comments in `src/ui/message.rs` and the `play`/`pause` doc comments in
  `src/ui/handlers/playback.rs` to name the new buttons alongside X and C; the handlers themselves
  do not change.
- `src/ui/views/mini_transport.rs`: `mini_transport_controls()` loses its `is_playing` argument and
  returns five tuples; update its "four buttons" docs and the row's leading-comment wording. The
  loop in `mini_transport_row_with_optional_seek` and the seek/footer geometry stay put.
- `src/ui/view.rs`: drop the now-unused `is_playing` argument at the `view_transport_controls`
  call; no other root-view arm changes.
- Tests: in `src/ui/views/tests/transport.rs`, rewrite
  `transport_glyphs_match_the_mockup_order_and_widths` for the five glyphs, words, widths
  `[34.0, 30.0, 30.0, 30.0, 34.0]`, and messages (`PreviousTrack`, `Play`, `Pause`, `Stop`,
  `NextTrack`); extend `transport_buttons_keep_their_measured_face_widths` to the seven widths;
  delete `play_pause_glyph_and_word_mirror_playing_state`; drop the removed imports and
  `is_playing` arguments elsewhere in the file. In `src/ui/views/tests/shade.rs`, update
  `main_transport_and_shade_strip_share_glyphs_order_and_widths` and
  `shade_controls_map_to_transport_messages_and_keep_fixed_widths` to five entries and the new
  messages, replace `shade_play_pause_glyph_switches_with_playback_state` with a check that both
  `▶` and `II` are drawn in every playback state, and rename
  `library_strip_draws_the_four_controls_time_and_right_edge_volume` to its five-control twin,
  expecting `["◀◀", "▶", "II", "■", "▶▶"]` while the readout and right-edge volume checks stay.
  In `src/ui/tests/view_tests.rs`, replace
  `transport_row_does_not_reflow_when_the_play_glyph_changes` with a test that lays out the root
  view in stopped and playing states and asserts the Play and Pause glyphs keep the same bounds
  (the row no longer depends on playback). Remove the deleted helpers from
  `src/ui/views/tests/mod.rs`.
- Docs: `docs/usage.md`'s main-row, library-strip, and shade-strip sentences name Previous, Play,
  Pause, Stop, and Next, and say Play starts or resumes while Pause pauses (stopping a track whose
  source supplied no preview); X and C keep their current descriptions. `docs/invariants.md`
  `## UI layout`'s transport bullet names the five glyphs and their messages, replaces the
  "Play/Pause glyph swap cannot reflow" rationale with the static-row one (the fixed widths still
  pin every button), updates **Enforced by** and the **Pinned by** test names to the tests above,
  and adds a History line.

**Files touched.** `src/ui/views/transport.rs`, `src/ui/views/mini_transport.rs`,
`src/ui/view.rs`, `src/ui/message.rs`, `src/ui/handlers/playback.rs`,
`src/ui/views/tests/mod.rs`, `src/ui/views/tests/transport.rs`,
`src/ui/views/tests/shade.rs`, `src/ui/tests/view_tests.rs`, `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- The root view, the shade strip, and the library footer each draw five glyphs in the order
  Previous, Play, Pause, Stop, Next; Play dispatches `Message::Play` and Pause `Message::Pause`,
  while no drawn glyph dispatches `Message::PlayPause` (X/C and the system media Play/Pause keys
  keep their existing messages).
- Each row's laid-out content is identical in the stopped, playing, and paused states: the five
  glyphs, Repeat, Shuffle, volume, balance, EQ, and PL keep the same positions, and the compact
  strips still fit their five glyphs, time readout, and right-edge volume slider.
- Play resumes or restarts and Pause pauses through the same service calls as X and C; no LCD
  transport glyph, seek, volume, balance, equalizer, browse, or settings behavior changes.
- No new `Message`, handler, state field, setting, or dependency is added, and
  `play_pause_glyph` / `play_pause_word` no longer exist.
- The usage reference and the UI-layout invariant describe the five-glyph row, and the project's
  check passes.

## Done
