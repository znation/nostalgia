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

## Done
