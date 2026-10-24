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

### Open an About panel from the title bar's M button (planned 2026-10-24 by the plan role)

**Goal.** `docs/screenshots/hero.png` shows the app bar's leftmost button as `M`, the classic
Winamp main-menu slot, but `views::view_title_bar` draws an `A` there (always-on-top, which the
LCD strip's `A` already does) and the app has no About UI. Replace the title bar's `A` with `M`,
make it toggle a modal About panel that names the app, shows the package version from
`CARGO_PKG_VERSION`, and carries the mockup's one-line description, and keep the LCD strip's `A`
as the always-on-top control. The panel is transient and not persisted; opening it while shaded
restores the window first, and it closes from M, its `Close` button, or a click on the backdrop.
No new dependency, shortcut, or playback behavior.

**Approach.**

- `src/ui/views/title_bar.rs`: rename `CLUTTER_BUTTON_WIDTH` to `TITLE_BAR_LEFT_BUTTON_WIDTH`
  (keep 18 px) and change `view_title_bar`'s argument to `about_visible: bool`. Draw `M` in the
  left slot with `Message::ToggleAbout` and `style::toggle_button_style(about_visible, status)`;
  update the module and function docs, the width assertion, and the comments that call the slot
  the A toggle.
- `src/ui/message.rs`: add `ToggleAbout` beside the window actions, documented as the title bar's
  M button; it only flips the panel flag (after unshading when needed).
- `src/ui/player.rs`: add `about_visible: bool` beside `library_panel_visible`, initialized
  `false`, documented as transient and not persisted.
- `src/ui/handlers/window.rs`: add `toggle_about(player) -> Task<Message>`: flip
  `about_visible`; when opening while `player.shaded` holds, call the existing
  `toggle_window_shade(player)` first and return its task so the window restores; otherwise
  `Task::none()`. Document that the panel never coexists with shade mode.
- `src/ui/mod.rs`: route `Message::ToggleAbout` beside the other window actions.
- New `src/ui/views/about.rs`, declared and re-exported by `src/ui/views/mod.rs`: a private
  `const ABOUT_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));` and
  `view_about_overlay(base) -> Element` that stacks `base` under a full-window `MouseArea`
  backdrop (a translucent `container::Style` built in the module) publishing
  `Message::ToggleAbout`, and above it a centered `bevel::raised_panel` holding
  `ABOUT NOSTALGIA`, `ABOUT_VERSION`, the mockup line `The classic Winamp look, rebuilt in Rust —
  with Apple Music as the library.`, and a `Close` button publishing `Message::ToggleAbout`. Wrap
  the panel itself in a `MouseArea` that swallows presses with `Message::Ignored`, so only the
  backdrop dismisses. Reuse the existing chrome and style helpers; add no theme constant outside
  the module.
- `src/ui/view.rs`: build the shade or normal base tree into one `Element` (replacing the shade
  branch's early return) and, while `player.about_visible && !player.shaded` holds, pass it
  through `views::view_about_overlay`; otherwise return it unchanged. Pass `player.about_visible`
  (not `always_on_top`) to `views::view_title_bar` in both branches.
- `src/ui/views/tests.rs`: update `title_bar_centers_its_name_between_its_edge_controls` for the
  renamed M slot; add `title_bar_m_button_publishes_toggle_about` (headless press) and
  `about_overlay_draws_the_name_version_and_tagline` (layout capture), plus a press on the panel's
  `Close` and on a backdrop point outside the panel each publishing exactly one `ToggleAbout`.
- `src/ui/tests.rs`: replace `title_bar_constructs_for_both_always_on_top_states` with an
  About-state construction test; add
  `toggle_about_shows_and_hides_the_panel_without_touching_playback` (drive `Message::ToggleAbout`,
  assert the flag flips, no work is scheduled, and transport, mixer, and browse state are
  unchanged) and `toggle_about_while_shaded_restores_the_window_first` (set `shaded`, give a
  window id, drive it, assert `!shaded`, `about_visible`, and scheduled resize work); render with
  the panel open in both `view_constructs_over_the_apps_full_input_space` and
  `view_constructs_when_the_window_is_shaded`.
- `README.md`: replace the title bar's `A` claim with the M/About description (name, version,
  tagline, and the three ways to close it), keeping the LCD strip's `A` as the always-on-top
  control. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: update `## UI layout`'s app-bar bullet (M and shade/minimize/close, not
  A); add `## About panel` after `## Window shade` stating that the panel is transient and
  unpersisted, draws only while `about_visible` holds outside shade mode, names `ABOUT_VERSION`
  from the package, and is enforced by `views::view_about_overlay`,
  `handlers::window::toggle_about`, and `Message::ToggleAbout`, with the new test names; note in
  `## Window shade` that opening About restores the window (the shade tree stays title bar plus
  strip).

**Files touched.** `src/ui/views/title_bar.rs`, `src/ui/views/about.rs`, `src/ui/views/mod.rs`,
`src/ui/views/tests.rs`, `src/ui/message.rs`, `src/ui/player.rs`,
`src/ui/handlers/window.rs`, `src/ui/mod.rs`, `src/ui/view.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- The title bar's left slot draws `M` and publishes exactly one `Message::ToggleAbout` on a press;
  the LCD strip's `A` still publishes `Message::ToggleAlwaysOnTop` exactly as before.
- While `about_visible` holds in normal mode, the root view draws the About panel over the
  otherwise unchanged base, showing `ABOUT NOSTALGIA`, the `v`-prefixed `CARGO_PKG_VERSION`, the
  tagline, and `Close`.
- Pressing M, `Close`, or the backdrop flips `about_visible` exactly once; pressing the panel's
  non-button chrome publishes `Message::Ignored` and leaves it open.
- Opening from shade mode schedules the existing unshade resize and leaves `shaded` false with
  `about_visible` true; the view never draws the panel in the shade branch.
- The title bar no longer draws an always-on-top toggle; the strip's `A` and the saved
  always-on-top behavior are unchanged.
- No new dependency, shortcut, persisted setting, or playback change is added; the new tests and
  `make check` pass.

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
