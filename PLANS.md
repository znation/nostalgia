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

### Close the transient panels with Escape, like a Winamp dialog (planned 2026-10-26 by the plan role)

**Goal.** Each transient panel — About, File info, Jump to time, and Jump to File — closes from its
own `Close` button, a backdrop click, or its toggle key, but not from the keyboard's universal
dismiss key. Add one shared route: a plain or Shift-held Escape press closes whichever of those
panels is showing, including while its filter or time box has focus. Escape with Control, Alt, or
Logo held, and every other key, stay as they are; with no panel open Escape stays unbound as it is
today, so a focused `Search library` box keeps it for clearing its own focus. The new route only
clears panel-visibility flags: no query, selection, playback, transport, mixer, browse, or settings
state changes, no work is scheduled, and nothing is persisted.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) fn transient_panel_open(&self) -> bool` beside
  `close_transient_panels`, returning `self.about_visible || self.file_info_visible ||
  self.jump_time_visible || self.jump_file_visible`. Document it beside `close_transient_panels`
  as the read side of the same four-flag set, so the subscription gate and the closer cannot
  drift; the set stays named in this one impl rather than in each caller.
- `src/ui/message.rs`: add a payload-free `EscapePressed` after `JumpFileRowSelected`, documented
  as closing the showing transient panel (About, File info, Jump to time, or Jump to File) by
  clearing only those four visibility flags, changing no other state, and scheduling no work.
- `src/ui/handlers/window.rs`: add `pub(in crate::ui) fn escape_pressed(player: &mut WinampPlayer)
  -> Task<Message>` beside `toggle_about`: call `player.close_transient_panels()` and return
  `Task::none()`. Document that the subscription only delivers it while a panel is open, and that
  with none open it is still a no-op. It is not a toggle, so it never opens a panel.
- `src/ui/timers.rs`:
  - Add `pub(super) fn escape_key_message(event: iced::Event, _status: iced::event::Status,
    _window: iced::window::Id) -> Option<Message>` beside `mixer_key_message`. Match
    `iced::Event::Keyboard(keyboard::Event::KeyPressed { key: keyboard::Key::Named(
    keyboard::key::Named::Escape), modifiers, .. })` and return `Some(Message::EscapePressed)`
    unless `modifiers.control() || modifiers.alt() || modifiers.logo()` holds; every other event,
    and every Escape release, returns `None`. Ignore the status argument deliberately: this
    listener exists so the press is seen even when a focused `TextInput` captures Escape and
    clears its own focus, which `keyboard::listen`'s ignored-only feed would drop.
  - Give `subscriptions` a fifth `transient_panel_open: bool` argument after
    `mixer_readout_showing`, and push `iced::event::listen_with(escape_key_message)` only when it
    holds. Document that the Escape listener joins only while a panel is open, so Escape outside
    a panel rebuilds no screen, and that the event-with-status form is required because the panel
    boxes hold focus.
- `src/ui/mod.rs`: pass `player.transient_panel_open()` as the fifth `subscriptions` argument,
  and route `Message::EscapePressed => handlers::window::escape_pressed(player)` beside the
  `ToggleAbout`/`ToggleFileInfo` arms.
- `src/ui/tests/timers_tests.rs`: append `false` to every existing `subscriptions(...)` call. Add
  `the_escape_listener_runs_only_while_a_transient_panel_is_open`, asserting the baseline
  `subscriptions(false, false, false, false, false)` and
  `subscriptions(false, false, false, false, true)` differ by exactly the Escape listener (3 vs
  2). Add `escape_key_message_maps_escape_but_not_its_chords_or_other_keys`, building a
  `KeyPressed` event as `a_key_or_wheel_event_ends_a_mixer_readout` does
  (`src/ui/tests/mixer_tests.rs`), asserting `Some(Message::EscapePressed)` for a plain and a
  Shift-held press with `Status::Captured` (proving the captured-event seam), and `None` for
  Ctrl, Alt, and Logo chords, an Escape release, and another named key.
- `src/ui/tests/panels_tests.rs`: add
  `escape_closes_each_transient_panel_without_touching_anything_else`. From one `test_player()`,
  drive `Message::SongsLoaded(stepping_songs())`, set `current_view = CurrentView::Songs`, call
  `make_current_track_known`, and set `playback_duration = Some(...)` so all four panels can
  open. Assert `Message::EscapePressed` with no panel open schedules no work and leaves
  `transient_panel_open()` false. Then loop over the four toggles
  (`ToggleAbout`, `ToggleFileInfo`, `ToggleJumpTime`, `ToggleJumpFile`): open it, assert
  `transient_panel_open()`, drive `Message::EscapePressed` with
  `assert_message_schedules_no_work`, and assert `!transient_panel_open()`; snapshot transport
  mode, current track, volume, balance, repeat, shuffle, current view, and the songs row
  selection before the loop and assert them unchanged after it.
- `docs/usage.md`: in the transient-panel sentence, say a plain or Shift-held Escape press closes
  whichever panel is open, like its `Close` button or the backdrop, and that with no panel open
  Escape keeps its existing behavior.
- `docs/invariants.md`: add a `## Transient panel dismissal` section after `## Jump to File panel`
  stating the guarantee above, enforced by `WinampPlayer::transient_panel_open`,
  `timers::escape_key_message`, `Message::EscapePressed`,
  `handlers::window::escape_pressed`, and `WinampPlayer::close_transient_panels`, pinned by the
  three new tests; name `handlers::window::escape_pressed` and `ui::timers` in that section. Add
  one line to each of `## About panel`, `## File info panel`, `## Jump to time panel`, and
  `## Jump to File panel` saying a plain or Shift-held Escape also closes it, cross-referencing
  `## Transient panel dismissal`, and add the new tests to those sections' **Pinned by** lists.

**Files touched.** `src/ui/player.rs`, `src/ui/message.rs`, `src/ui/handlers/window.rs`,
`src/ui/timers.rs`, `src/ui/mod.rs`, `src/ui/tests/timers_tests.rs`,
`src/ui/tests/panels_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- With any one of About, File info, Jump to time, or Jump to File open, a plain or Shift-held
  Escape press closes exactly that panel: all four `*_visible` flags are false afterwards, no
  task is scheduled, and no other state changes.
- The Escape press is delivered even while the panel's own text box has focus, because the
  listener uses `iced::event::listen_with` and ignores the event status; it is subscribed only
  while `WinampPlayer::transient_panel_open()` holds, so Escape outside a panel sends nothing.
- Escape with Control, Alt, or Logo held and Escape key releases map to nothing, and
  `shortcuts::message_for` still returns `None` for a plain Escape press, so every existing
  keyboard and media-key binding is unchanged.
- With no panel open, `Message::EscapePressed` is a no-op, and Escape keeps its current behavior
  (including a focused `Search library` box clearing its own focus).
- Queries, browse selection, playback, transport, mixer, settings, and persisted state are
  untouched; nothing new is saved.
- `docs/usage.md` and `docs/invariants.md` describe the dismissal route, and `make check` passes.

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
