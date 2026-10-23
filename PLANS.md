# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; updated 2026-10-20).** The initial prompt
> names two commitments — a "very close if not pixel-perfect" classic Winamp UI
> and Apple Music as the library. The first is now moving: the base-skin
> palette landed ("Add a Winamp 2.x base-skin palette and apply it as the app
> theme", done 2026-10-06), and the reusable bevel layer followed ("Add a
> Winamp two-tone bevel layer and frame the Now Playing and equalizer panels",
> done 2026-10-06), framing those two panels through `bevel::lcd_well` and
> `bevel::raised_panel` in `src/ui/bevel.rs`. The transport-button chrome plan
> landed ("Style the transport buttons as raised Winamp chrome", done
> 2026-10-07), and both transport toggles are reachable: the Repeat toggle
> landed 2026-10-06 and the Shuffle toggle 2026-10-08, and their R and S
> keyboard bindings followed ("Add the R and S keyboard bindings for Repeat
> and Shuffle", done 2026-10-20). The rest of widget-level fidelity has
> landed too: slider chrome ("Style the volume and
> equalizer sliders as sunken Winamp grooves with raised chrome thumbs", done
> 2026-10-07), a custom title bar ("Add a Winamp custom title bar and drop the
> OS window frame", done 2026-10-07), and playlist chrome ("Frame the browse
> list as a sunken Winamp playlist editor", done 2026-10-07). The library side
> is real:
> `AppleMusicService` answers its browse and search queries through the REST
> client once a `MusicKit` session is stored, falling back to
> `sample_library()` otherwise — playlist queries included — and `play_track`
> plays the selected preview through the audio seam. Risk: the Winamp chrome
> arrived piecemeal, so later changes still need one consistent vocabulary.
> The equalizer-audio and seek plans this note pointed at have since landed
> (both 2026-10-20); the 2026-10-22 note below records drift that has since
> been resolved.

> **Steward drift note (2026-10-22 by the steward role; resolved 2026-10-23
> by the steward role).** That note recorded the built Now Playing bar and
> browse rows drifting from the brief's mockups — no elapsed readout, no
> `O A I D V` strip, no status row, album rows without running time, and a
> queued playback-position plan that put an `<elapsed>/<total>` readout at the
> LCD's right edge. Those drifts have since landed: "Add the classic elapsed
> readout to the Now Playing LCD and restore the plain total at its right
> edge", "Draw the classic O A I D V strip at the LCD's left edge", "Show the
> loaded preview's bitrate in the LCD status row", "Show each album row's
> running time", and "Show each artist row's album count" are done, and the
> playback-position plan is no longer queued. No current brief-vs-build drift
> is known. The LCD `MONO`/`STEREO` pair has since landed (done 2026-10-23),
> which brought the status row to `docs/screenshots/player.png` and
> `docs/screenshots/transport.png`, and the volume and balance slider fill
> has since landed (done 2026-10-23) with the orange and green grooves those
> mockups draw, and the playlist-number title prefix landed too (done
> 2026-10-23), matching `docs/screenshots/player.png`, and the transport
> glyph row landed on the same day. No planned entry remains.

## Planned

_None yet._

### Adjust the volume with the mouse wheel (planned 2026-10-23 by the plan role)

**Goal.** Classic Winamp changes the volume when you roll the mouse wheel over the main window,
and Nostalgia currently has no wheel behavior outside the browse list's own scrollbar. Make a roll
over the player panels raise or lower the volume by one slider step, using the same
`AppState::nudge_volume` path and backend forward the volume slider and up/down arrow keys already
use, so the thumb and the playing preview follow immediately. A roll over the browse list must keep
scrolling the list and leave the volume alone; a horizontal-only roll must do nothing. This touches
only the view tree, one message, and its handler; it is independent of the queued settings plan.

**Approach.**

- `src/ui/views/mod.rs`: add `pub(super) fn volume_wheel_message(delta: iced::mouse::ScrollDelta)
  -> Message`. For both `ScrollDelta::Lines` and `ScrollDelta::Pixels`, return
  `Message::VolumeWheel(1.0)` when the vertical delta is positive, `Message::VolumeWheel(-1.0)`
  when it is negative, and `Message::VolumeWheel(0.0)` for a horizontal-only roll (`y == 0.0`).
  Document that iced passes winit's positive-y wheel direction (the direction that scrolls up) and
  that the sign, not the magnitude, is the step, so one event moves the slider once regardless of
  the backend's delta units. One event is one step, like one arrow press.
- `src/ui/message.rs`: add `VolumeWheel(f32)` with a doc comment naming it the mouse wheel's
  vertical direction over the player panels (positive raises, negative lowers, zero is a no-op),
  and saying the browse list is deliberately outside the area that publishes it.
- `src/ui/mod.rs`: route `Message::VolumeWheel(direction)` in the `update` match to
  `handlers::mixer::volume_wheel`.
- `src/ui/handlers/mixer.rs`: add `pub(in crate::ui) fn volume_wheel(player: &mut WinampPlayer,
  direction: f32) -> Task<Message>`. Return `Task::none()` for a zero or non-finite direction
  (the `direction > 0.0` / `direction < 0.0` test leaves NaN on the no-op path), otherwise call the
  existing `volume_up` or `volume_down` handler unchanged. That reuses `views::VOLUME_STEP`, the
  `nudge_volume` clamp, and the audio-backend forward, so the wheel cannot drift from the arrows.
- `src/ui/view.rs`: in the normal branch, move the title bar, Now Playing bar, seek strip,
  transport row, equalizer title band, and equalizer panel into one `Column`, wrap that column in
  `iced::widget::MouseArea::new(...).on_scroll(views::volume_wheel_message)`, and push the wrapped
  area where those panels were. Leave the library title band, Back/Retry, the All Songs/All
  Albums/Playlists buttons, the Top/Sort row, the search box, and the browse list outside the
  `MouseArea`, so the list's own `Scrollable` still takes the wheel. In the shade branch, wrap the
  title-bar-plus-shade-strip `Column` the same way, since the strip carries a volume slider. The
  `MouseArea` forwards events to its content first, so the title bar's drag/double-click area,
  the LCD's clickable readout, the sliders, and the buttons keep their existing behavior.
- `src/ui/views/tests.rs`: add `volume_wheel_message_maps_a_vertical_roll_to_a_direction`, covering
  positive and negative `Lines` and `Pixels` deltas, and
  `volume_wheel_message_zeroes_a_horizontal_only_roll`, covering `Lines { x: 1.0, y: 0.0 }` and its
  `Pixels` twin.
- `src/ui/tests.rs`: add `volume_wheel_nudges_by_one_slider_step_and_ignores_zero`, driving
  `Message::VolumeWheel(1.0)`, `Message::VolumeWheel(-1.0)`, and `Message::VolumeWheel(0.0)`
  through `assert_message_schedules_no_work` and asserting the shared volume moves by exactly
  `views::VOLUME_STEP` for the first two and is unchanged for zero. Add
  `volume_wheel_clamps_at_the_ends` beside `volume_keys_clamp_at_the_ends`.
- `README.md`: after the Usage sentence saying the volume slider and up/down arrows set the playback
  level, add that rolling the mouse wheel over the player panels nudges the volume by one slider
  step in that direction, while over the browse list the wheel keeps scrolling the list. Do not
  touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Volume wheel` section after `## Level slider fill` stating that a
  wheel roll over the player panels moves the volume by exactly one `VOLUME_STEP` in the roll's
  vertical direction, through `AppState::nudge_volume` and the same backend forward as the arrows;
  that the browse list is outside that area, so its `Scrollable` keeps the wheel and the volume is
  unchanged; and that a horizontal-only roll or a zero direction is a no-op. Name
  `views::volume_wheel_message`, `Message::VolumeWheel`, `handlers::mixer::volume_wheel`, and the
  two `MouseArea::on_scroll` wrappers in `ui::view` as enforcers, and the new test names as pins.

**Files touched.** `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/message.rs`,
`src/ui/mod.rs`, `src/ui/handlers/mixer.rs`, `src/ui/view.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- A wheel roll over the title bar, Now Playing bar, seek strip, transport row, or equalizer panel
  raises the volume by one `views::VOLUME_STEP` when the vertical delta is positive and lowers it by
  one step when it is negative, for both line and pixel deltas, with the shared state clamping at
  the ends and forwarding the level to the audio backend exactly as the arrow keys do.
- A wheel roll over the browse list still scrolls the list and changes no volume; a horizontal-only
  roll and a zero direction change no volume and schedule no work.
- The wheel uses the existing `VolumeUp`/`VolumeDown` behavior through `handlers::mixer::volume_wheel`,
  so no second clamping rule, step size, backend path, timer, or subscription is added; no dependency
  is added.
- The mapping tests cover both delta kinds and the horizontal-only case, the handler tests cover
  both directions, zero, and the clamps, and `make check` passes.
- The README and the new invariant section describe the same behavior, and the browse list's
  scrolling, the title bar's drag/shade, the LCD click, and the sliders keep their current behavior.

## Done
