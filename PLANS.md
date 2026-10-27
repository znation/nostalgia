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

### Sink toggles while on, part 1/2: Repeat and Shuffle through a shared toggle helper (planned 2026-10-26 by the plan role; split 2026-10-27 by the plan role)

**Previously held 1**

Part 1 of 2; part 2/2 ("Sink toggles while on, part 2/2") reuses the
helpers this part adds.

**Goal.** The title bar's `M` and the transport row's `EQ` and `PL` already draw sunken chrome
(`style::toggle_button_style`) while on. The LCD strip's `A` is not a button: it lights through
`views::now_playing::indicator::indicator_label_color` and keeps that behavior. The transport's
Repeat and Shuffle buttons still draw ordinary raised chrome while on, so their state reads only
from the `On`/`Off` label. Make them sink while on and stay ordinary while off, through one
shared toggle constructor. Labels, widths, messages, and click behavior stay as they are; no
playback or settings state changes.

**Approach.**

- `src/ui/style.rs`: correct the comment above the test `toggle_button_style_sinks_while_on`,
  which names "The A, EQ, and PL toggles": use `M`, `EQ`, and `PL` (and, after this change,
  Repeat and Shuffle) as the sunken-button examples, and do not mention `A`.
- `src/ui/views/mod.rs`: beside `fixed_width_button`, add
  `pub(super) fn fixed_width_toggle_button(label: &'static str, width: f32, message: Message, on: bool) -> Button<'static, Message>`,
  building on `fixed_width_button` and applying
  `.style(move |_theme, status| style::toggle_button_style(on, status))`. Document it as the place
  the sunken-on rule is applied. (The unfixed-width `toggle_button` is added by part 2/2, its
  first caller, so no helper lands without a caller.)
- `src/ui/views/transport.rs`: move `REPEAT_WIDTH` and `SHUFFLE_WIDTH` from inside
  `transport_buttons` to module scope (same values, 104.0 and 112.0). Add
  `pub(super) fn repeat_toggle(repeat: bool) -> Button<'static, Message>` and
  `pub(super) fn shuffle_toggle(shuffle: bool) -> Button<'static, Message>`, built with
  `fixed_width_toggle_button` from `repeat_label`/`shuffle_label`, the widths, and
  `Message::ToggleRepeat`/`Message::ToggleShuffle`; use them in `transport_buttons` at the same
  positions. Change `panel_toggle_button` to call `fixed_width_toggle_button`, and rewrite its doc
  comment: it shares the title bar's `M` rule, and the LCD strip's `A` is a lit letter, not
  sunken chrome.
- Tests: in `src/ui/views/tests/mod.rs` add
  `fn region_has_color(region: &[u8], color: Color) -> bool`, scanning RGBA bytes from
  `PixelCapture::region` for the color converted to 8-bit channels. In
  `src/ui/views/tests/transport.rs` add `repeat_and_shuffle_toggles_sink_while_on`: for each of
  `repeat_toggle` and `shuffle_toggle`, render on and off through
  `layout_capture::capture_text_and_pixels` (with `layout_capture::headless_renderer()`), take the
  region `Rectangle::with_size(text.size())`, and assert the on region has
  `theme::BUTTON_FACE_PRESSED` and not `theme::BUTTON_FACE`, and the off region the reverse.
- Docs: `docs/usage.md` — say Repeat and Shuffle sink while on, beside the sentence naming their
  labels. `docs/invariants.md` `## UI layout` — add the sunken-on rule to the transport
  guarantee, name `fixed_width_toggle_button` under **Enforced by**, add
  `repeat_and_shuffle_toggles_sink_while_on` under **Pinned by**, and a dated History line.

**Files touched.** `src/ui/style.rs`, `src/ui/views/mod.rs`, `src/ui/views/transport.rs`,
`src/ui/views/tests/mod.rs`, `src/ui/views/tests/transport.rs`, `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- Repeat and Shuffle draw `theme::BUTTON_FACE_PRESSED` sunken chrome while on and
  `theme::BUTTON_FACE` while off, at rest, pinned by `repeat_and_shuffle_toggles_sink_while_on`.
- Labels, fixed widths, and messages are unchanged: `repeat_label_mirrors_repeat_state`,
  `shuffle_label_mirrors_shuffle_state`, and `transport_buttons_keep_their_measured_face_widths`
  pass unmodified.
- `EQ`, `PL`, and the title bar's `M` keep their sunken-on behavior and geometry;
  `panel_toggle_button` builds through `fixed_width_toggle_button`.
- The LCD `A` keeps its lit/dim behavior; `indicator_label_color_lights_only_the_active_mode`
  passes unmodified, and neither corrected comment describes `A` as sinking.
- The usage reference and `## UI layout` describe the change, and `make check` passes.

### Sink toggles while on, part 2/2: the equalizer's EQ and AUTO toggles (planned 2026-10-26 by the plan role; split 2026-10-27 by the plan role; "requires part 1/2 landed")

**Previously held 1**

Part 2 of 2; builds on `fixed_width_toggle_button` and the test helper
`region_has_color` from part 1/2 ("Sink toggles while on, part 1/2").

**Goal.** The equalizer header's EQ on/off and AUTO buttons draw raised chrome while on, so their
state reads only from the `EQ: On/Off` and `AUTO: On/Off` labels. Make them sink while on, like
Repeat, Shuffle, `EQ`/`PL`, and `M`. Labels, widths, messages, and click behavior stay as they
are; no equalizer or settings state changes.

**Approach.**

- `src/ui/views/equalizer.rs`: add `pub(super) fn eq_toggle(enabled: bool) -> Button<'static, Message>`
  (`toggle_button` with `eq_enabled_label(enabled)` and `Message::ToggleEqualizer`, keeping its
  current unfixed width) and `pub(super) fn auto_toggle(auto: bool) -> Button<'static, Message>`
  (`fixed_width_toggle_button` with `auto_label(auto)`, `AUTO_BUTTON_WIDTH`, and
  `Message::ToggleEqAuto`). Replace the two inline button constructions in `view_equalizer`'s
  header row with them.
- `src/ui/views/mod.rs`: beside `labeled_button` and part 1/2's `fixed_width_toggle_button`, add
  `pub(super) fn toggle_button(label: &'static str, message: Message, on: bool) -> Button<'static, Message>`,
  building on `labeled_button` and applying `style::toggle_button_style(on, status)` the same way.
- Tests: in `src/ui/views/tests/equalizer.rs` add `eq_and_auto_toggles_sink_while_on`, the same
  shape as part 1/2's `repeat_and_shuffle_toggles_sink_while_on`: on regions have
  `theme::BUTTON_FACE_PRESSED` and not `theme::BUTTON_FACE`, off regions the reverse, using
  `region_has_color`.
- Docs: `docs/usage.md` — say the equalizer's EQ and AUTO buttons sink while on.
  `docs/invariants.md` `## UI layout` — extend the equalizer guarantee with the sunken-on rule,
  name `toggle_button`, `eq_toggle`, and `auto_toggle` under **Enforced by**, add
  `eq_and_auto_toggles_sink_while_on` under **Pinned by**, and a dated History line.

**Files touched.** `src/ui/views/equalizer.rs`, `src/ui/views/mod.rs`,
`src/ui/views/tests/equalizer.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- EQ on/off and AUTO draw `theme::BUTTON_FACE_PRESSED` sunken chrome while on and
  `theme::BUTTON_FACE` while off, at rest, pinned by `eq_and_auto_toggles_sink_while_on`.
- `eq_enabled_label_mirrors_enabled_state` and
  `auto_button_draws_its_state_and_publishes_toggle_eq_auto` pass unmodified; AUTO keeps
  `AUTO_BUTTON_WIDTH`, and each button still dispatches exactly its existing message.
- No equalizer, playback, or settings state changes, and no setting is added.
- The usage reference and `## UI layout` describe the change, and `make check` passes.

## Done
