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

### Sink the Repeat, Shuffle, EQ, and AUTO toggles while on, like Winamp (planned 2026-10-26 by the plan role)

**Previously held 1**

**Replanned 2026-10-26 by the plan role:** distinguish lit LCD letters from sunken
chrome buttons, including the existing comments identified by review.

**Routing 2026-10-26 by the director role: user requests breakdown and refinement by the plan role.**
Keep this implementation hold until the plan role replaces the revised draft below with focused,
independently mergeable sub-plans sized for one implementation run each. Evaluate a shared
helper/transport part and an equalizer part; put any prerequisite in the dependent heading as
"requires part 1/2 landed" if using two parts. Each part must name its files and symbols,
behavior tests, usage/invariant documentation updates, and acceptance criteria. Carry forward
the revised draft's distinction between lit LCD indicators and sunken chrome, its requirement
to correct misleading comments, and preservation of the LCD `A` indicator's existing behavior.
The revised draft below is input for replanning, not authorization to resume implementation as written.

**Goal.** The title bar's `M` and the transport row's `EQ` and `PL` already use shared sunken
chrome (`style::toggle_button_style`) while on. The LCD strip's `A` instead lights through
`views::now_playing::indicator::indicator_label_color` and keeps that behavior.
The four state toggles whose labels carry an `On`/`Off` value — the transport's
Repeat and Shuffle buttons and the equalizer's EQ on/off and AUTO buttons — still draw ordinary
raised chrome while on, so their state reads only from the label. Make those four buttons follow
the same on/off rule: sunken chrome while on, ordinary chrome while off. Their labels, widths,
messages, and click behavior stay exactly as they are, and no playback, equalizer, or settings
state changes.

**Approach.**

- Correct the comment above `toggle_button_style_sinks_while_on` in `src/ui/style.rs`:
  use `M`, `EQ`, and `PL` as the existing sunken-button examples, not `A`. Correct the
  `panel_toggle_button` doc comment in `src/ui/views/transport.rs` as well: compare it
  with the title bar's `M`, and explicitly distinguish the LCD's lit `A` letter.
  These comment corrections are part of this change's acceptance, not optional cleanup.
- `src/ui/views/mod.rs`: add `pub(super) fn toggle_button(label, message, on) -> Button` and
  `pub(super) fn fixed_width_toggle_button(label, width, message, on) -> Button`. They build on the
  existing `labeled_button` and `fixed_width_button` and apply
  `style::toggle_button_style(on, status)`, so the sunken-on rule lives in one place. Document them
  beside those helpers.
- `src/ui/views/transport.rs`: extract the Repeat and Shuffle buttons into
  `pub(super) fn repeat_toggle(repeat: bool) -> Button` and
  `pub(super) fn shuffle_toggle(shuffle: bool) -> Button`, built with
  `fixed_width_toggle_button` and the existing `REPEAT_WIDTH`, `SHUFFLE_WIDTH`,
  `repeat_label`, and `shuffle_label`. Use them in `transport_buttons` in the same positions.
  Move `REPEAT_WIDTH` and `SHUFFLE_WIDTH` from inside `transport_buttons` to module scope
  so the extracted constructors can reuse their existing values.
  Change `panel_toggle_button` to call `fixed_width_toggle_button` so the EQ and PL toggles, the
  Repeat and Shuffle toggles, and the new helpers all share one construction.
- `src/ui/views/equalizer.rs`: extract `pub(super) fn eq_toggle(enabled: bool) -> Button` (the
  existing `eq_enabled_label` plus `Message::ToggleEqualizer`, keeping its unfixed width) and
  `pub(super) fn auto_toggle(auto: bool) -> Button` (the existing `auto_label`,
  `AUTO_BUTTON_WIDTH`, and `Message::ToggleEqAuto`). Build both with the new shared helpers and use
  them in `view_equalizer`'s header row.
- Tests: add a shared test helper in `src/ui/views/tests/mod.rs`,
  `region_has_color(region: &[u8], color: Color) -> bool`, that scans a captured RGBA region for a
  color. In `src/ui/views/tests/transport.rs` add
  `repeat_and_shuffle_toggles_sink_while_on`: for each of `repeat_toggle` and `shuffle_toggle`,
  render the on and off buttons through `layout_capture::capture_text_and_pixels`, take each
  button's whole root region from `TextCapture::size`, and assert the on region contains
  `theme::BUTTON_FACE_PRESSED` (and not `theme::BUTTON_FACE`) while the off region contains
  `theme::BUTTON_FACE` (and not the pressed face). Repeat the same test for `eq_toggle` and
  `auto_toggle` as `eq_and_auto_toggles_sink_while_on` in `src/ui/views/tests/equalizer.rs`.
  Leave `repeat_label_mirrors_repeat_state`, `shuffle_label_mirrors_shuffle_state`,
  `eq_enabled_label_mirrors_enabled_state`, `auto_button_draws_its_state_and_publishes_toggle_eq_auto`,
  and `transport_buttons_keep_their_measured_face_widths` in place; they already pin the labels,
  messages, and widths.
- Docs: in `docs/usage.md`, say the Repeat and Shuffle toggles and the equalizer's EQ and AUTO
  buttons sink while on, alongside the existing sentence naming their labels. In
  `docs/invariants.md` `## UI layout`, extend the transport and equalizer guarantees with the
  sunken-on rule, name the new helpers under **Enforced by**, add the two new test names under
  **Pinned by**, and add a History line dated 2026-10-26. No settings section changes because no
  persisted value changes.

**Files touched.** `src/ui/style.rs`, `src/ui/views/mod.rs`, `src/ui/views/transport.rs`,
`src/ui/views/equalizer.rs`, `src/ui/views/tests/mod.rs`,
`src/ui/views/tests/transport.rs`, `src/ui/views/tests/equalizer.rs`, `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- The Repeat, Shuffle, EQ on/off, and AUTO buttons draw `theme::BUTTON_FACE_PRESSED` chrome in the
  sunken bevel while on, and ordinary `theme::BUTTON_FACE` chrome while off, at rest.
- Their labels (`Repeat: On/Off`, `Shuffle: On/Off`, `EQ: On/Off`, `AUTO: On/Off`), fixed widths,
  and published messages are unchanged, and pressing each still dispatches exactly its existing
  toggle message.
- The title bar's `M` and the transport row's `EQ` and `PL` toggles keep their
  existing sunken-on behavior and their geometry, and the transport row's fixed widths still match
  `transport_buttons_keep_their_measured_face_widths`.
- The LCD strip's `A` keeps its lit/dim text behavior from `indicator_label_color`;
  no sunken chrome is added to the strip. The comments above `toggle_button_style_sinks_while_on`
  and `panel_toggle_button` no longer describe `A` as sinking. Retain
  `indicator_label_color_lights_only_the_active_mode` as coverage for the distinct LCD rule.
- No playback, equalizer, browse, search, selection, window, or settings state changes, and no
  setting is added.
- The usage reference and `## UI layout` describe the four sunken toggles, and the project's check
  passes.


## Done
