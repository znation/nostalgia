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

### Blink the shade strip's and library footer's paused time readout, like Winamp (planned 2026-10-27 by the plan role)

**Goal.** Winamp blinks the paused time in the window-shade bar as well as in the main LCD. Our
LCD's large elapsed readout already blinks while paused: `handlers::playback::preview::position_tick`
flips `WinampPlayer::blink_visible` on each one-second position poll, and
`views::now_playing::elapsed_readout_color` draws the hidden phase transparent. The compact
readout (`00:53/3:30`) is the only time shown in shade mode. It is built once by
`mini_transport_row_with_optional_seek` in `src/ui/views/mini_transport.rs` and shared by the
shade strip and the library footer, and it stays solid green while paused. That makes it
impossible to tell a paused rolled-up window from a playing one at a glance. Make the compact
readout blink in step with the LCD.

**Approach.** Decided, so the implementer has no open questions:
- The whole compact readout text blinks, both elapsed and `/total`. Winamp's shade bar shows only
  the time, so the whole readout is the blinking part. Keeping it one `Text` also leaves the
  layout untouched.
- The hidden phase draws the text fully transparent and does not remove it, so its bounds and
  the seek and volume sliders do not move (the same technique `elapsed_readout_color` uses).
- The playing and stopped colors are unchanged: `theme::LCD_GREEN`, as today. The readout's click
  behavior is also unchanged: `elapsed_readout_clickable` still gates it, so a paused readout in
  its hidden phase still toggles the time display.
- No new timer or subscription. `needs_position_poll` already holds while paused with a known
  position, so the existing poll keeps the phase moving in shade mode.

Changes:
- `src/ui/views/mini_transport.rs`: add a pure
  `pub(super) fn compact_readout_color(mode: TransportMode, visible: bool) -> Color`. It returns
  `Color::from_rgba(0.0, 0.0, 0.0, 0.0)` for `(TransportMode::Paused, false)` and
  `theme::LCD_GREEN` otherwise. In `mini_transport_row_with_optional_seek`, replace
  `.color(theme::LCD_GREEN)` on the readout with `.color(compact_readout_color(mode, elapsed_visible))`.
  Add an `elapsed_visible: bool` parameter right after `mode` to
  `mini_transport_row_with_optional_seek`, `mini_transport_row`, `mini_transport_row_with_seek`,
  `mini_transport_strip`, `mini_transport_strip_with_seek` and `view_library_strip`, and thread it
  through. `mini_transport_strip_with_seek` grows to 8 arguments, past clippy's
  `too_many_arguments` limit of 7, so give it `#[allow(clippy::too_many_arguments)]` with a
  one-line comment, as `now_playing_row` in `src/ui/views/now_playing.rs` does. Update the module doc and `view_library_strip`'s doc to say the paused readout blinks.
- `src/ui/views/shade.rs`: add `elapsed_visible: bool` after `mode` to `view_shade_strip`, pass it
  on, and mention the blink in its doc.
- `src/ui/view/shade.rs` (`view_shade`) and `src/ui/view/player_panels.rs` (the
  `views::view_library_strip` call): pass `player.blink_visible`.
- Tests: every existing call of `view_shade_strip` / `view_library_strip` (in
  `src/ui/views/tests/shade.rs` and `src/ui/views/tests/seek_wheel.rs`) passes `true`. Add these
  to `src/ui/views/tests/shade.rs`, importing `compact_readout_color` through
  `src/ui/views/tests/mod.rs` the way it imports `mini_transport_readout`:
  - `compact_readout_color_blinks_only_while_paused`: Paused+false is transparent (alpha 0).
    Paused+true, Playing with either phase, and Stopped with either phase are `theme::LCD_GREEN`.
  - `compact_readout_keeps_its_bounds_when_the_blink_hides_it`: lay out
    `view_shade_strip(TransportMode::Paused, false, …)` and `(Paused, true, …)` with
    `captured_mini_strip`. Both capture the `00:53/3:30` text, and the volume-slider rectangle is
    equal in the two.
- `docs/usage.md`: in the paragraph with "While paused, the large elapsed readout blinks once per
  second", add that the compact readouts in the library footer and the shade strip blink with it.
- `docs/invariants.md`: in the paused-blink guarantee (the bullet naming
  `WinampPlayer::blink_visible` and `elapsed_readout_color`), add that
  `views::mini_transport::compact_readout_color` draws the compact readout transparent in the
  hidden phase without removing it, and name the two new tests.

**Files touched.** `src/ui/views/mini_transport.rs`, `src/ui/views/shade.rs`,
`src/ui/view/shade.rs`, `src/ui/view/player_panels.rs`, `src/ui/views/tests/shade.rs`,
`src/ui/views/tests/seek_wheel.rs`, `src/ui/views/tests/mod.rs`, `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**
- While paused, the shade strip's and the library footer's `elapsed/total` readout disappears and
  reappears on alternate one-second polls, in phase with the LCD's large readout.
- While playing or stopped, the compact readout looks as it does today.
- The hidden phase keeps the readout's bounds, so no strip control moves. Clicking the paused
  readout in either phase still toggles elapsed/remaining.
- No new subscription or timer is added.
- The two new tests pass, the docs describe the blink, and `make check` passes.

## Done
