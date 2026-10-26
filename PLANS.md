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

_None yet._

### Point the mouse wheel at the balance and equalizer sliders (planned 2026-10-26 by the plan role)

**Goal.** A wheel roll over the player panels already nudges the volume, and a roll over the
drawn seek slider seeks instead, but a roll over the balance slider still changes the volume,
and a roll over the equalizer's preamp or a band slider also changes the volume. Make a plain
wheel roll over the balance slider nudge the balance, and over the preamp or any band nudge
that gain, by one slider step in the roll's vertical direction, forwarded live to the audio
backend like a drag. Everywhere else the wheel keeps nudging the volume, and the drawn seek
slider keeps its seek exception. The new wheel paths change no drag readout and schedule no
work. This is independent of the waiting Escape plan: both add a `Message` variant and a
dispatch arm, so keep both when they meet in `src/ui/message.rs` and `src/ui/mod.rs`.

**Approach.**

- `src/ui/views/mod.rs`: extract the existing sign mapping in `volume_wheel_message` into a
  private `fn wheel_direction(delta: ScrollDelta) -> f32` returning `1.0`, `-1.0`, or `0.0`
  (zero also covers `NaN` and a horizontal-only roll). Keep `volume_wheel_message`'s output
  unchanged. Add `pub(super) fn balance_wheel_message(delta) -> Message`,
  `pub(super) fn eq_preamp_wheel_message(delta) -> Message`, and
  `pub(super) fn eq_band_wheel_message(index: usize, delta) -> Message`, each wrapping
  `wheel_direction` in `Message::BalanceWheel`, `Message::EqPreampWheel`, and
  `Message::EqBandWheel(index, direction)`. Change `BALANCE_STEP` and `EQ_STEP` from private to
  `pub(super)` so the handlers use the sliders' own granularity, as `VOLUME_STEP` already does.
- `src/ui/message.rs`: add `BalanceWheel(f32)`, `EqPreampWheel(f32)`, and
  `EqBandWheel(usize, f32)` beside `VolumeWheel`, documenting that each carries a signed wheel
  direction and moves only that shared value by one step without setting `mixer_drag`.
- `src/ui/handlers/mixer.rs`: add `balance_wheel`, `eq_preamp_wheel`, and `eq_band_wheel` beside
  `volume_wheel`. Each returns `Task::none()` for a zero or `NaN` direction. The balance
  handler calls `state.set_balance(state.balance() + direction.signum() * views::BALANCE_STEP)`
  through `mutate_balance`; the preamp and band handlers do the same through `mutate_equalizer`
  with `views::EQ_STEP`, `state.eq_preamp()`, and `state.eq_bands()[band]`. An out-of-range
  `band` is a no-op before the lock. Never set `player.mixer_drag`.
- `src/ui/mod.rs`: dispatch the three new variants to those handlers beside `Message::VolumeWheel`.
- `src/ui/views/transport.rs`: wrap only the balance `chrome_slider` in
  `MouseArea::new(..).on_scroll(super::balance_wheel_message)`. iced runs the slider's own update
  first and the wrapper only when the slider did not capture the event, so the slider's existing
  Ctrl+wheel step stays as it is. Add a `#[cfg(test)]` `BALANCE_SLIDER_ID` on a `Container`
  around that wrapper, like `seek_bar`'s ids.
- `src/ui/views/equalizer.rs`: give `eq_slider_column` an `on_scroll: impl Fn(ScrollDelta) ->
  Message` parameter and wrap its `VerticalSlider` in `MouseArea::new(slider).on_scroll(on_scroll)`
  before pushing the label. `preamp_column` passes `super::eq_preamp_wheel_message`; the band loop
  passes a closure capturing `index` and calling `super::eq_band_wheel_message(index, delta)`. Add
  a `#[cfg(test)]` `EQ_PREAMP_SLIDER_ID` and a `#[cfg(test)] pub(super) fn
  eq_band_slider_id(index) -> String`, and wrap the preamp and each band column in a test-only
  `Container` with those ids.
- `src/ui/views/mod.rs`: re-export the three test-only slider ids behind `#[cfg(test)]` beside
  `SEEK_BAR_ID`, and the three new wheel message functions are already in `views`.
- `src/ui/views/tests/transport.rs`: add
  `balance_wheel_message_maps_a_vertical_roll_to_a_direction` covering `Lines` and `Pixels`, both
  signs, and zero/horizontal/`NaN` as `0.0`.
- `src/ui/views/tests/equalizer.rs`: add
  `equalizer_wheel_messages_map_the_roll_direction_and_band` asserting `EqPreampWheel` and
  `EqBandWheel(index, ..)` for both signs and a zero roll.
- `src/ui/tests/mixer_tests.rs`: add `balance_wheel_nudges_by_one_step_and_ignores_zero_and_nan`,
  `eq_preamp_wheel_nudges_by_one_db_and_ignores_zero_and_nan`, and
  `eq_band_wheel_nudges_only_its_band_and_ignores_zero_nan_and_an_out_of_range_index`, each using
  `assert_message_schedules_no_work` and asserting the clamp at an end plus no `mixer_drag`. Add
  `wheel_over_the_balance_slider_nudges_the_balance_not_the_volume`: capture the root view's
  `views::BALANCE_SLIDER_ID` bounds, roll `1.0` and `-1.0`, and assert exactly one `BalanceWheel`
  with the matching sign and no `VolumeWheel`. Add
  `wheel_over_the_preamp_and_every_band_nudges_that_slider_not_the_volume`: ensure
  `eq_panel_visible`, then for `views::EQ_PREAMP_SLIDER_ID` and every
  `views::eq_band_slider_id(index)` roll once and assert exactly the matching `EqPreampWheel` or
  `EqBandWheel(index, ..)` and no `VolumeWheel`.
- `docs/usage.md`: in the wheel sentence, add that a roll over the balance slider, the equalizer
  preamp, or an equalizer band moves that slider by one step, while the drawn seek slider still
  seeks and every other player panel still nudges the volume.
- `docs/invariants.md`: amend `## Volume wheel` to name the balance, preamp, and band sliders as
  exceptions beside the drawn seek slider, and add a `## Slider wheel` section after
  `## Seek wheel` stating the one-step rule, the sign mapping, the zero/`NaN` no-op, the live
  backend forward, the unchanged volume outside them, and that no readout is shown. Name
  `views::balance_wheel_message`, `views::eq_preamp_wheel_message`,
  `views::eq_band_wheel_message`, the three `handlers::mixer` wheel handlers, and the two slider
  wrappers as enforcers, with the new tests in **Pinned by**.

**Files touched.** `src/ui/views/mod.rs`, `src/ui/views/transport.rs`,
`src/ui/views/equalizer.rs`, `src/ui/message.rs`, `src/ui/handlers/mixer.rs`, `src/ui/mod.rs`,
`src/ui/views/tests/transport.rs`, `src/ui/views/tests/equalizer.rs`,
`src/ui/tests/mixer_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A plain wheel roll over the balance slider changes only the balance by one `BALANCE_STEP` in
  the roll's vertical direction, clamped to its range, and forwards the new balance to the audio
  backend; a roll over the preamp or any band changes only that gain by one `EQ_STEP`, clamped,
  forwards the curve, and clears the preset selection exactly as a drag does.
- None of those rolls also nudges the volume, and none sets `mixer_drag`, so the LCD keeps its
  title rather than showing a readout.
- Every other player panel still nudges the volume, the drawn seek slider still seeks, and a
  horizontal-only, zero, `NaN`, or out-of-range-band roll is a no-op that schedules no work.
- The wheel mapping tests, the handler tests, and the root-view wheel tests pass, and
  `make check` passes.

## Done
