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

### Add Winamp's options menu from the LCD's `O` cell and the transport row (planned 2026-10-26 by the plan role)

**Goal.** The mockups draw the LCD's classic `O` indicator cell and a lightning menu button at the
end of the transport row, but `lcd_indicator_message` leaves `O` inert and the transport row has no
menu button. Make both open a Winamp-style options menu: a transient, unpersisted panel listing the
app's existing on/off choices plus an About row. Picking a row applies that choice and closes the
menu, like a Winamp menu item; `Close`, the backdrop, or Escape dismisses it. `D` stays decorative
(double-size is not implemented). No setting is added or persisted, and playback, browse, search,
equalizer audio, and every existing control's non-menu behavior are otherwise unchanged.

**Approach.**

- `src/ui/message.rs`: add `pub(in crate::ui) enum OptionsChoice` with `#[derive(Debug, Clone,
  Copy, PartialEq, Eq)]` and variants `AlwaysOnTop`, `TimeDisplay`, `Visualizer`,
  `EqualizerPanel`, and `LibraryPanel`, each documented with the handler it reuses. Add
  `Message::ToggleOptions` (the `O` cell and lightning button: flip `options_visible`, unshading
  first) and `Message::OptionsChoice(OptionsChoice)` (a menu row: apply the choice, then close the
  menu). Document both beside `ToggleAbout`, and update `EscapePressed`'s comment from four panels
  to five.
- `src/ui/player.rs`: add `pub(in crate::ui) options_visible: bool`, defaulting false in
  `WinampPlayer::new` and documented like `about_visible` (transient, not persisted, never drawn in
  shade mode). Add it to `close_transient_panels` and `transient_panel_open`, extending both docs
  from four panels to five, and add it to the `At most one of …` lists on the existing
  `about_visible`, `file_info_visible`, `jump_time_visible`, and `jump_file_visible` fields.
- `src/ui/handlers/window.rs`: add `pub(in crate::ui) fn toggle_options(player) -> Task<Message>`
  mirroring `toggle_about` (flip; on open call `close_transient_panels` first, set the flag, and
  unshade through `toggle_window_shade`). Add `pub(in crate::ui) fn options_choice(player,
  choice: OptionsChoice) -> Task<Message>` that clears `options_visible` first, then dispatches to
  the existing `toggle_always_on_top`, `handlers::playback::toggle_time_display`,
  `handlers::playback::toggle_visualizer`, `handlers::mixer::toggle_equalizer_panel`, or
  `handlers::mixer::toggle_library_panel` and returns its `Task`. Update the module and
  `escape_pressed` docs.
- `src/ui/mod.rs`: dispatch `ToggleOptions` and `OptionsChoice`.
- `src/ui/views/options.rs` (new, modeled on `views/about.rs`): pure `pub(super)` label helpers
  `always_on_top_label(bool)`, `time_label(bool)`, `visualizer_label(bool)`,
  `equalizer_label(bool)`, and `library_label(bool)` returning the exact strings below, plus
  `pub fn view_options_overlay(base, always_on_top, time_remaining, visualizer_visible,
  eq_panel_visible, library_panel_visible) -> Element` stacking a `modal_overlay` panel titled
  `OPTIONS` with one `labeled_button` per row:
  - `Always on Top: On/Off` → `Message::OptionsChoice(OptionsChoice::AlwaysOnTop)`
  - `Time: Remaining/Elapsed` → `OptionsChoice::TimeDisplay`
  - `Visualizer: Shown/Hidden` → `OptionsChoice::Visualizer`
  - `Equalizer: Shown/Hidden` → `OptionsChoice::EqualizerPanel`
  - `Library: Shown/Hidden` → `OptionsChoice::LibraryPanel`
  - `About Nostalgia` → `Message::ToggleAbout` (the existing handler closes the menu through
    `close_transient_panels`)
  - `Close` → `Message::ToggleOptions`
  Give the module its own unit tests for the five label pairs.
- `src/ui/views/mod.rs`: `mod options;` and `pub use options::view_options_overlay;`.
- `src/ui/views/transport.rs`: add `pub(super) fn options_button() -> Element<'static, Message>`
  returning a fixed-width `⚡` button publishing `Message::ToggleOptions`, with a documented
  `OPTIONS_BUTTON_WIDTH`, and push it after the `PL` toggle in `view_transport_controls`, separated
  by the existing `spacer`. Update the module, `view_transport_controls`, and button docs so the row
  reads Previous, Play, Pause, Stop, Next, Repeat, Shuffle, volume, balance, EQ, PL, menu.
- `src/ui/view.rs`: in the normal-mode overlay chain, draw `views::view_options_overlay` while
  `player.options_visible && !player.shaded`, passing `player.always_on_top`,
  `player.time_display == views::TimeDisplay::Remaining`, `player.visualizer_visible`,
  `player.eq_panel_visible`, and `player.library_panel_visible`; add
  `options_visible: player.options_visible` to the `IndicatorModes` it builds. Update the
  root-view doc.
- `src/ui/views/now_playing/indicator.rs`: `lcd_indicator_message("O")` returns
  `Some(Message::ToggleOptions)`; add `options_visible: bool` to `IndicatorModes`; light `O` through
  `indicator_label_color` while that flag holds; update the docs so `O` is live and only `D` stays
  decorative.
- Tests: in `src/ui/views/tests/chrome.rs` add
  `options_overlay_draws_each_row_with_its_state` (all-off/elapsed/hidden and
  all-on/remaining/shown layouts, asserting the exact row strings, `About Nostalgia`, and
  `Close`), `options_overlay_publishes_each_choice_and_closes_from_close_and_backdrop` (click every
  row, asserting its `Message::OptionsChoice(..)` or `ToggleAbout`; `Close` and the backdrop publish
  `ToggleOptions`; panel chrome publishes only `Ignored`), and
  `transport_options_button_publishes_toggle_options` (lay out `view_transport_controls`, assert
  `⚡` sits after `PL`, and click it). In `src/ui/views/tests/now_playing.rs`, extend
  `indicator_label_color_lights_only_the_active_mode`,
  `lcd_indicator_strip_lights_the_active_letters_and_dims_the_rest`, and the indicator press test
  for `options_visible`/`O`, renaming
  `lcd_indicator_message_arms_a_i_and_v_and_leaves_o_d_inert` to
  `lcd_indicator_message_arms_o_a_i_and_v_and_leaves_d_inert` and extending the strip press test's
  name to include `options_on_o`. In `src/ui/tests/panels_tests.rs` add
  `options_choice_applies_each_toggle_and_closes_the_panel` (every `OptionsChoice` flips exactly
  its target flag, clears `options_visible`, and leaves the other flags and playback state alone)
  and `toggle_options_opens_and_closes_and_excludes_each_other_panel` (`ToggleOptions` while
  unshaded schedules no work; opening About/File info/Jump to time/Jump to File closes it and vice
  versa), and add `Message::ToggleOptions` to the toggle list in
  `escape_closes_each_transient_panel_without_touching_anything_else`. In
  `src/ui/tests/view_tests.rs` add `options_panel_draws_over_the_player_and_drops_out_in_shade_mode`
  (a row string appears with `options_visible`, and disappears once `shaded` holds).
- Docs: `docs/usage.md`'s Now Playing sentence makes `O` open the options menu and light while it
  is open (only `D` stays decorative), and adds the menu's rows, the close-on-choice rule, the
  shade restore, and the transport row's lightning button. `docs/invariants.md`: update
  `## Now Playing label` (`O` live and lighting, `D` decorative), `## UI layout` (the menu button
  after `PL`), and `## Transient panel dismissal` (five panels); add `## Options panel` after
  `## About panel` with the guarantee, **Enforced by** the symbols above, **Pinned by** the new test
  names, and a History line. No settings section changes because the menu is not persisted.

**Files touched.** `src/ui/message.rs`, `src/ui/player.rs`, `src/ui/handlers/window.rs`,
`src/ui/mod.rs`, `src/ui/views/options.rs`, `src/ui/views/mod.rs`, `src/ui/views/transport.rs`,
`src/ui/view.rs`, `src/ui/views/now_playing/indicator.rs`, `src/ui/views/tests/chrome.rs`,
`src/ui/views/tests/now_playing.rs`, `src/ui/tests/panels_tests.rs`, `src/ui/tests/view_tests.rs`,
`docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Pressing the LCD's `O` or the transport row's `⚡` opens an `OPTIONS` panel over the normal-mode
  screen; while unshaded, opening or closing it schedules no work. Opening it closes any other
  transient panel, and opening it while shaded restores the window first.
- The panel draws exactly the rows above with labels reflecting the current state on every
  rebuild; pressing a row applies that one choice and closes the panel, `About Nostalgia` opens the
  About panel, and `Close`, the backdrop, or Escape closes it.
- The `O` letter lights `theme::LCD_GREEN` while the menu is open and dims otherwise; `A`, `I`, and
  `V` keep their existing behavior, and `D` stays decorative and publishes nothing.
- The menu is not persisted, adds no setting, and leaves playback, transport, browse, search,
  equalizer audio, volume/balance, and every existing toggle's non-menu behavior unchanged.
- Every existing About route still works: the `M` button, Ctrl+F1, `Close`, the backdrop, Escape,
  and the library-source line.
- The usage reference and the four invariant sections describe the menu and the live `O`, and the
  project's check passes.

### Sink the Repeat, Shuffle, EQ, and AUTO toggles while on, like Winamp (planned 2026-10-26 by the plan role)

**Goal.** The app already draws every panel toggle that is on with the shared sunken chrome
(`style::toggle_button_style`): the LCD strip's `A`, the title bar's `M`, and the transport row's
`EQ` and `PL`. The four state toggles whose labels carry an `On`/`Off` value — the transport's
Repeat and Shuffle buttons and the equalizer's EQ on/off and AUTO buttons — still draw ordinary
raised chrome while on, so their state reads only from the label. Make those four buttons follow
the same on/off rule: sunken chrome while on, ordinary chrome while off. Their labels, widths,
messages, and click behavior stay exactly as they are, and no playback, equalizer, or settings
state changes.

**Approach.**

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

**Files touched.** `src/ui/views/mod.rs`, `src/ui/views/transport.rs`,
`src/ui/views/equalizer.rs`, `src/ui/views/tests/mod.rs`,
`src/ui/views/tests/transport.rs`, `src/ui/views/tests/equalizer.rs`, `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- The Repeat, Shuffle, EQ on/off, and AUTO buttons draw `theme::BUTTON_FACE_PRESSED` chrome in the
  sunken bevel while on, and ordinary `theme::BUTTON_FACE` chrome while off, at rest.
- Their labels (`Repeat: On/Off`, `Shuffle: On/Off`, `EQ: On/Off`, `AUTO: On/Off`), fixed widths,
  and published messages are unchanged, and pressing each still dispatches exactly its existing
  toggle message.
- The LCD strip's `A`, the title bar's `M`, and the transport row's `EQ` and `PL` toggles keep their
  existing sunken-on behavior and their geometry, and the transport row's fixed widths still match
  `transport_buttons_keep_their_measured_face_widths`.
- No playback, equalizer, browse, search, selection, window, or settings state changes, and no
  setting is added.
- The usage reference and `## UI layout` describe the four sunken toggles, and the project's check
  passes.

## Done
