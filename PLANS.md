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

### Open and navigate the options menu from the keyboard (planned 2026-10-26 by the plan role)

**Goal.** The transient options menu is the only panel with no keyboard route: it opens only from
the LCD's `O` cell or the transport row's lightning button, and its rows only accept mouse clicks.
Give it the treatment the other panels already have: `Alt+O` opens and closes the menu like `Alt+3`
opens File info and `Ctrl+F1` opens About, and while the menu is open Up and Down move a highlighted
row and Enter invokes it, mirroring the Jump to File panel's arrow-and-Enter navigation. Mouse
clicks, labels, widths, and every existing toggle's behavior stay as they are; the menu and its
highlight remain layout-only and unpersisted.

**Approach.**

- `src/ui/message.rs`: add `Message::OptionsSelectionUp`, `Message::OptionsSelectionDown`, and
  `Message::OptionsActivate`, documented beside `ToggleOptions`/`OptionsChoice`. Add
  `pub(in crate::ui) enum OptionsRow { AlwaysOnTop, TimeDisplay, Visualizer, EqualizerPanel,
  LibraryPanel, About, Close }` with `ALL: [Self; 7]` in draw order and `message(self) -> Message`
  returning exactly the message each row's button publishes (`OptionsChoice(..)` for the five
  toggles, `ToggleAbout`, `ToggleOptions`). This keeps the row order in one place shared by the
  view and the keyboard handler. Unit-test that `ALL` maps to those messages in order.
- `src/ui/player.rs`: add `pub(in crate::ui) options_selected: usize` (default `0`), documented as
  an index into `OptionsRow::ALL`, reset whenever the menu opens, layout-only and not persisted.
- `src/ui/handlers/window.rs`: in `toggle_options`, reset `options_selected = 0` on the opening
  branch. Add `options_selection_up`/`options_selection_down`, which are no-ops unless
  `options_visible` holds and otherwise clamp at the first and last rows. Add `options_activate`,
  which is a no-op unless `options_visible` holds, takes `OptionsRow::ALL.get(options_selected)`,
  and dispatches the message from `OptionsRow::message` by calling the existing `options_choice`,
  `toggle_about`, or `toggle_options` for that message, so a keyboard activation follows exactly
  the mouse row's path.
- `src/ui/shortcuts.rs`: add `options_shortcut(physical_key, modifiers)` matching the physical
  `KeyO` with Alt held and Control, Shift, and Logo released to `Message::ToggleOptions`, and wire
  it into `message_for` before the plain-letter bindings. Add `message_for_options`, used while the
  menu is open: it maps Up and Down (Control, Alt, and Logo released; Shift allowed) to the
  selection messages and Enter to `Message::OptionsActivate`, and falls through to `message_for`
  for every other event so `Alt+O` still closes the menu and the rest of the key table keeps its
  existing behavior, exactly like `message_for_jump_file`.
- `src/ui/subscriptions/mod.rs`: add an `options_open: bool` parameter to `subscriptions` and choose
  the keyboard mapper as Jump to File, then Options, then the normal `message_for`. Update the
  function doc and `src/ui/mod.rs`'s subscription call to pass `player.options_visible`.
- `src/ui/mod.rs`: dispatch the three new messages to the new handlers and re-export `OptionsRow`
  beside `Message` and `OptionsChoice`.
- `src/ui/views/options.rs`: change `view_options_overlay` to take a final `selected: usize`, draw
  its rows by iterating `OptionsRow::ALL` through a new `options_row_label(row, always_on_top,
  time_remaining, visualizer_visible, eq_panel_visible, library_panel_visible)` that delegates to
  the existing five label helpers and returns `"About Nostalgia"`/`"Close"`, and build each row
  with `labeled_button` overridden by `options_row_style(index == selected, status)`. That new pure
  helper keeps `style::chrome_button_style(status)` for an unselected row and, for the selected
  row, paints the blue `theme::ROW_HIGHLIGHT` bar with white `theme::PLAYING_ROW_TEXT` in every
  button status; unit-test both branches. `src/ui/view.rs` passes `player.options_selected`.
- Tests: in `src/ui/shortcuts.rs`, add `options_shortcut_maps_alt_o_and_rejects_other_chords` and
  `message_for_options_maps_up_down_enter_and_falls_back`. In `src/ui/tests/panels_tests.rs`, add
  `options_keyboard_selection_clamps_and_activates_each_row`: opening highlights row 0, Up at the
  first row and Down at the last row stay put, Down walks the rows, and `OptionsActivate` on each
  of the seven rows applies exactly that row's effect and closes the menu (About opens About; Close
  only closes; each toggle flips only its flag, reusing the expectations in
  `options_choice_applies_each_toggle_and_closes_the_panel`). Add a no-op case with the menu
  closed. Update every `subscriptions(...)` call in `src/ui/tests/subscriptions_tests.rs` for the
  new parameter, and update `options_overlay_element` and its tests in
  `src/ui/views/tests/chrome.rs` for the new `selected` argument (row 0 selected).
- Docs: in `docs/usage.md`, add `Alt+O` to the keyboard list as the options menu's open/close
  chord and extend the options-menu paragraph with the highlighted row, Up/Down, and Enter (mouse
  behavior unchanged). In `docs/invariants.md` `## Options panel`, record the `Alt+O` chord, the
  highlighted-row state and its reset, the Up/Down/Enter mapper, the new handlers, the
  `OptionsRow` vocabulary, and the `options_open` subscription choice under **Enforced by** and
  **Pinned by**, with a History line dated 2026-10-26. No settings section changes.

**Files touched.** `src/ui/message.rs`, `src/ui/player.rs`, `src/ui/handlers/window.rs`,
`src/ui/shortcuts.rs`, `src/ui/subscriptions/mod.rs`, `src/ui/mod.rs`, `src/ui/views/options.rs`,
`src/ui/view.rs`, `src/ui/tests/panels_tests.rs`, `src/ui/tests/subscriptions_tests.rs`,
`src/ui/views/tests/chrome.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `Alt+O` toggles the options menu open and closed, opening it with the same unshade-first and
  close-other-panels behavior as the `O` cell and lightning button; it does not fire with Control,
  Shift, or Logo held. Opening highlights the first row, Up and Down move the highlight and clamp at
  the first and last rows, and Enter applies the highlighted row and closes the menu exactly like a
  click on it.
- The seven rows keep their current labels, order, and published messages; the selected row
  draws the blue highlight bar with white text in every button status, and unselected rows keep
  ordinary chrome. Mouse row presses, `Close`, the backdrop, and Escape behave as before.
- With the menu closed, Up and Down still nudge the volume, Enter still opens the selected browse
  row, and every other existing binding is unchanged; the options mapper is used only while
  `options_visible` holds.
- No playback, equalizer, browse, search, selection, window, or settings state changes beyond the
  row's existing toggle, the highlight index is layout-only and resets whenever the menu opens, and
  no setting is added or persisted.
- The usage reference and `## Options panel` describe the keyboard access, and the project's check
  passes.

### Split the root screen assembly into one module per branch (planned 2026-10-26 by the organize role)

**Goal.** `src/ui/view.rs` is the root assembly: a single 326-line `view()` function holding three
unrelated jobs — the shaded tree, the normal-mode panel stack plus browse column, and the transient
overlay selection. A change to the shaded strip has to read past the normal and overlay branches,
and the normal branch resolves one 17-field state snapshot whose fields only some of its panels
use. Split `view` into a `src/ui/view/` directory with one module per assembly, so each module owns
only the state and widgets its branch draws. The public entry point stays `ui::view::view`, and the
assembled tree, overlay precedence, and published messages stay exactly what `view()` builds today.

**Target structure.**

- `src/ui/view/mod.rs`: the module doc and `pub(in crate::ui) fn view(player)`, which builds
  `shade::view_shade(player)` when `player.shaded` holds, otherwise
  `normal::view_normal(player)`, then hands that base to `overlay::apply_overlays`.
- `src/ui/view/shade.rs`: `view_shade`, owning the shade branch's state snapshot
  (`transport_mode`, `volume`, elapsed, total, position, duration), the title-bar and shade-strip
  column, and the `MouseArea::on_scroll(views::volume_wheel_message)` around it.
- `src/ui/view/normal.rs`: `view_normal`, owning the `library_panel_visible` branch, the
  `MouseArea::on_scroll(views::volume_wheel_message)` around the panel stack, and the column that
  appends `library::view_library(player)` while the library panel is visible.
- `src/ui/view/player_panels.rs`: `view_player_panels`, owning the normal branch's current-track,
  playback-format, spectrum, transport, volume, balance, Repeat, Shuffle, and equalizer snapshot
  and the title bar, Now Playing bar, seek bar, transport row, and optional equalizer section it
  feeds.
- `src/ui/view/library.rs`: `view_library`, owning the `CurrentView` match and its per-level
  arguments, the current-track lock for the Songs and All Songs playing-row markers, the Back,
  Retry, Open, browse-navigation, and flat-list buttons, the search row, the current list, and the
  library strip.
- `src/ui/view/overlay.rs`: `apply_overlays(base, player)`, owning the about → options → file info →
  jump time → jump file chain and its `!player.shaded` guards.

No handler or `views` module changes; every item in the new directory is `pub(super)` except the
`view` entry point.

**Approach.** Move each branch whole, keeping its explanatory comments with the widgets they
describe. Resolve each state lock inside the module that reads it, so `view()` no longer holds the
17-field snapshot and each module locks only for its own fields. Keep the two deliberate second
locks for the Songs and All Songs playing-row markers in `library`. Replace `src/ui/view.rs` with
the directory rather than keeping a compatibility shim. Update `docs/invariants.md`'s current-path
mentions of `src/ui/view.rs` in the Library search clearing, UI layout, Volume wheel, About panel,
and Options panel sections to the owning file; leave historical `PLANS-DONE.md` and `BUGS-FIXED.md`
paths alone. No behavior
test is added: the existing root-view, layout, panel, browse, and shortcut tests already exercise
`ui::view::view`.

**Files touched.** `src/ui/view.rs` (deleted); new
`src/ui/view/{mod,shade,normal,player_panels,library,overlay}.rs`; `docs/invariants.md`.

**Acceptance criteria.**

- `ui::view::view` remains the only root-assembly entry point, and `src/ui/mod.rs`'s re-export and
  `init_ui` wiring are unchanged.
- The widget calls, their arguments, the overlay precedence, and every published message match the
  current `view()`; the existing root-view, layout, panel, browse, and shortcut tests pass without
  changes.
- `src/ui/view.rs` no longer exists, and a grep for `src/ui/view.rs` over `src/`, `docs/`, and the
  open backlog finds no stale path (the historical archives keep theirs).
- `make check` passes.

## Done
