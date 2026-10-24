# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the README's Usage section: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. Record any new drift here rather than appending
> another note.

## Planned

### Seek with the mouse wheel over a drawn seek slider (planned 2026-10-24 by the plan role)

**Goal.** The existing player-panel wheel area sends every roll to the volume, including a roll
over the Now Playing seek slider and the shade strip's seek slider. Make a roll over a *drawn*
seek slider seek five seconds forward for a positive `y` or back for a negative `y`, like the
classic position bar, while every other player panel keeps its volume nudge and a blank seek
strip stays part of the volume area. Reuse `Message::SeekForward`, `Message::SeekBackward`, and
`views::SEEK_STEP_SECS`; add no message, player field, persisted setting, or backend path.

**Approach.**

- `src/ui/views/seek_bar.rs`: add `pub(super) const SEEK_STRIP_ID: &str = "now-playing-seek-strip"`
  for the strip's outer container and `pub(super) const SEEK_BAR_ID: &str =
  "now-playing-seek"` for its drawn slider. Add `pub(super) fn seek_wheel_message(delta:
  ScrollDelta) -> Message`: a positive `y` (iced's wheel-up direction, as in
  `views::volume_wheel_message`) publishes `Message::SeekForward`, a negative `y` publishes
  `Message::SeekBackward`, and a horizontal-only, zero, or non-finite roll publishes
  `Message::Ignored`. In `view_seek_bar`, keep the outer container tagged `SEEK_STRIP_ID`; when
  `seek_slider` returns a slider, wrap it in a fill-width inner container tagged `SEEK_BAR_ID`
  around `MouseArea::new(slider).on_scroll(seek_wheel_message)`. Leave the blank `Space` case
  unwrapped, so only a drawn slider captures the wheel.
- `src/ui/views/mini_transport.rs`: in `mini_transport_row_with_seek`, wrap the `Slider` inside
  the existing `MINI_SEEK_ID` container in `MouseArea::new(..).on_scroll(seek_wheel_message)`, so
  the shade strip's drawn slider seeks too. The library footer builds `mini_transport_row`
  without a seek slider and stays unchanged.
- `src/ui/views/mod.rs`: re-export `seek_wheel_message`, `SEEK_STRIP_ID`, and `SEEK_BAR_ID` at
  `pub(super)`/`pub(crate)` visibility for the views and root-view tests, following the existing
  `seek_bar_value` and `MINI_SEEK_ID` exports.
- `src/ui/views/tests.rs`: add `seek_wheel_message_maps_a_vertical_roll_to_a_direction` (both
  `Lines` and `Pixels`, both signs) and `seek_wheel_message_ignores_a_horizontal_only_roll`;
  add `wheel_over_the_shade_seek_slider_seeks` using the existing `MINI_SEEK_ID` bounds capture
  and a `mouse::Event::WheelScrolled`, asserting exactly one `SeekForward`/`SeekBackward` and no
  `VolumeWheel`.
- `src/ui/tests.rs`: with a player whose `playback_position` and `playback_duration` are set,
  use `captured_root_view_container(player, SEEK_BAR_ID)` and a `WheelScrolled` at its center to
  assert one `SeekForward`/`SeekBackward` and no `VolumeWheel`; with both cleared, use
  `SEEK_STRIP_ID` to assert the blank strip publishes one `VolumeWheel` and no seek. Name them
  `wheel_over_the_root_seek_slider_seeks_without_nudging_the_volume` and
  `wheel_over_a_blank_seek_strip_still_nudges_the_volume`.
- `README.md`: in the wheel sentence in `## Usage`, note that a roll over the drawn seek slider
  seeks five seconds forward or back instead of changing the volume, in both the Now Playing bar
  and the shade strip; leave the browse list's wheel behavior and the rest of the volume rule as
  they are.
- `docs/invariants.md`: amend `## Volume wheel` to name the drawn seek slider as the exception,
  and add `## Seek wheel` after it stating the sign mapping, the `Message::Ignored` no-op, the
  drawn-only capture, and that a blank strip and every other panel keep the volume nudge, with
  the enforcing code and the new test names.

**Files touched.** `src/ui/views/seek_bar.rs`, `src/ui/views/mini_transport.rs`,
`src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- A vertical wheel roll over the drawn Now Playing seek slider publishes exactly one
  `Message::SeekForward` for a positive `y` and one `Message::SeekBackward` for a negative `y`,
  and no `Message::VolumeWheel`; a horizontal-only roll publishes no seek.
- A vertical wheel roll over the shade strip's drawn seek slider does the same.
- A roll over the blank Now Playing seek strip, with no position or preview length, publishes
  the panel's one `Message::VolumeWheel` and no seek, so the exception needs a drawn slider.
- Every other player panel and the browse list keep their current wheel behavior, and the
  existing seek, volume, and shade-strip tests still pass.
- No new `Message` variant, `WinampPlayer` field, persisted setting, or backend call is added;
  `make check` passes.

## Done
