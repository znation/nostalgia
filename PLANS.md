# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; updated 2026-10-07).** The initial prompt
> names two commitments — a "very close if not pixel-perfect" classic Winamp UI
> and Apple Music as the library. The first is now moving: the base-skin
> palette landed ("Add a Winamp 2.x base-skin palette and apply it as the app
> theme", done 2026-10-06), and the reusable bevel layer followed ("Add a
> Winamp two-tone bevel layer and frame the Now Playing and equalizer panels",
> done 2026-10-06), framing those two panels through `style::lcd_well` and
> `style::raised_panel` in `src/ui/style.rs`. `## Planned` is no longer empty:
> it holds "Style the transport buttons as raised Winamp chrome", which
> restyles the five transport buttons and the browse Back button — all built
> through `labeled_button`, still a default iced `Button`. Still unplanned is
> the rest of widget-level fidelity: slider chrome (the bevel entry and the
> transport plan both defer it), a custom title bar, and playlist chrome. The
> library side is still all stub: `AppleMusicService` answers every browse
> query (`get_favorite_artists`, `get_albums_by_artist`,
> `get_songs_from_album`) from `sample_library()`, `play_track` only mutates
> shared state, and the token field is unset — and no PLANS.md entry plans the
> real integration. Risk: the longer Apple Music stays stubbed the more UI
> grows around the sample library's shape, while the remaining chrome work is
> easier to sequence now that the bevel layer exists. Recommend the plan loop
> keep scheduling the remaining fidelity plans (slider chrome, title bar,
> playlist chrome) and put the Apple Music integration plan on the near
> horizon.

## Planned

### Add a Winamp custom title bar and drop the OS window frame (found 2026-10-07)

Found by plan 2026-10-07, taking the "custom title bar" item the steward drift
note schedules and the base-skin Done entry lists among its later fidelity
plans. The bevel layer, buttons, sliders, and playlist chrome are done, but the
window still wears the OS title bar: `init_ui` in `src/ui/mod.rs` never sets
window settings, so the top of the app does not read as Winamp. `theme.rs`
already names `TITLE_BLUE` for the title bar and `style::raised_panel` already
supplies the raised bevel it needs.

**Goal.** Replace the OS window frame with a Winamp-style title bar: a
full-width raised `TITLE_BLUE` bar at the top of the window showing the app
name, draggable to move the window, with chrome minimize and close buttons on
the right. The window opens undecorated and fixed-size. No new theme colours
and no change to the existing view builders.

**Approach.**

- `src/ui/style.rs`: add `pub fn title_bar_style() -> container::Style`
  returning `container::Style { background: Some(Background::Color(
  theme::TITLE_BLUE)), text_color: Some(theme::TEXT),
  ..container::Style::default() }` — the flat base-skin title-bar fill and its
  light text, reusing the existing constants exactly as the other style
  functions do. Add a `#[cfg(test)]` test
  `title_bar_style_is_title_blue_with_light_text` pinning both fields.
- `src/ui/views.rs`: add module-level `const TITLE_BAR_TEXT: &str =
  "NOSTALGIA";` and `const TITLE_BAR_HEIGHT: f32 = 24.0;`, plus
  `pub fn view_title_bar() -> Element<'static, Message>`. It builds a `Row`
  of:
  - the drag region: `MouseArea::new(Container::new(Text::new(TITLE_BAR_TEXT)
    .size(14).color(theme::TEXT)).width(Length::Fill).height(Length::Fill)
    .align_y(iced::alignment::Vertical::Center).padding([0, 6]))
    .on_press(Message::WindowDragged)` — a `MouseArea` so a press anywhere on
    the bar (but not on the buttons) begins a window drag;
  - two `fixed_width_button`s ("–" → `Message::MinimizeWindow`, "✕" →
    `Message::CloseWindow`), each chrome-styled and width-pinned so the glyph
    cannot resize them.
  Wrap the `Row` in a `Container` with `.width(Length::Fill)` and
  `.height(Length::Fixed(TITLE_BAR_HEIGHT))` styled with
  `|_theme| style::title_bar_style()`, then in `style::raised_panel(..)` so the
  bar carries the same raised bevel as the other chrome. Add `MouseArea` to
  the `iced::widget::{..}` import.
- `src/ui/mod.rs`:
  - Add `window_id: Option<iced::window::Id>` to `WinampPlayer` and
    initialize it `None` in `WinampPlayer::new`.
  - Add `Message` variants `WindowIdResolved(Option<iced::window::Id>)`,
    `WindowDragged`, `MinimizeWindow`, and `CloseWindow`.
  - `boot`: batch the existing `Task::done(Message::LoadArtists)` with
    `iced::window::latest().map(Message::WindowIdResolved)`. iced runs the boot
    task only after the window opens (`iced_winit`'s `run` chains it onto
    `runtime::window::open`), so `latest()` resolves to the app window.
  - `update` arms: `WindowIdResolved(id)` stores `player.window_id = id` and
    returns `Task::none()`; the three window actions match on
    `player.window_id` and return `iced::window::drag(id)` /
    `iced::window::minimize(id, true)` / `iced::window::close(id)` when it is
    set, and `Task::none()` when it is not.
  - In `view`, push `views::view_title_bar()` as the first item of the
    `column`, above `view_now_playing`.
  - In `init_ui`, chain `.decorations(false)` and `.resizable(false)` onto the
    `iced::application(..)` builder so the OS frame is gone and the fixed size
    leaves no resize affordance the undecorated window cannot honour.
- `src/ui/tests.rs`: add `window_id_resolved_stores_the_window_id` (update with
  `Some(iced::window::Id::unique())`, assert `player.window_id` equals it, then
  with `None` and assert it clears) and
  `title_bar_window_actions_are_noops_without_a_window_id` (for each of the
  three action messages, assert `iced_runtime::task::into_stream(update(&mut
  player, message)).is_none()`), plus
  `title_bar_window_actions_schedule_work_with_a_window_id` (set the id, then
  assert each action's task stream is `Some`).
- `README.md`: add the custom title bar to the Status sentence; leave the
  `tumwater:prompt` block untouched.

**Files touched.** `src/ui/style.rs`, `src/ui/views.rs`, `src/ui/mod.rs`,
`src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo doc` with rustdoc warnings denied, `cargo test`).
- `title_bar_style_is_title_blue_with_light_text` passes, pinning the
  `TITLE_BLUE` background and `TEXT` text colour.
- `window_id_resolved_stores_the_window_id`,
  `title_bar_window_actions_are_noops_without_a_window_id`, and
  `title_bar_window_actions_schedule_work_with_a_window_id` pass.
- The existing `view_constructs_over_the_apps_full_input_space` still passes:
  it builds `view` for every browse/playback shape, so it exercises the new
  title bar in the column.
- `cargo run`: the window opens with no OS frame, a blue raised title bar at
  the top reading "NOSTALGIA" with minimize and close buttons, and dragging the
  bar moves the window while minimize and close work; manual check — the build
  and tests are the primary gate.

## Done

### Frame the browse list as a sunken Winamp playlist editor (found 2026-10-07, done 2026-10-07)

Found by plan 2026-10-07, taking the "playlist chrome" half of the widget-level
fidelity the steward drift note schedules after slider chrome. The bevel layer
(`src/ui/style.rs`) now dresses the panels, buttons, and sliders, but the browse
list — the app's playlist-editor stand-in — is still a plain `Scrollable` on the
window face: `scrollable_list` in `src/ui/views.rs` builds default-iced `Button`
rows (their hover/press is the theme's `TITLE_BLUE` primary) on no background,
and `Scrollable::new(column)` uses iced's default scrollbar rails. The base
skin's playlist editor is a near-black sunken panel with light rows and a
selection bar.

**Goal.** Give the artist/album/song browse list the playlist editor's chrome:
one pure `playlist_scrollable_style()` in `src/ui/style.rs` frames the list in a
sunken near-black well and restyles the scrollbar rails, and one pure
`playlist_row_style(status)` dresses the rows as flat playlist entries (light
text, chrome-face hover/press) while the currently playing row keeps its
selection highlight. No new theme colours and no signature changes.

**Approach.**

- `src/ui/style.rs`: add `scrollable` to the existing `iced::widget::{...}`
  import (the `container`, `Border`, `Shadow`, `Vector`, and `Background` types
  are already imported). Add two pure functions, both reusing the base-skin
  constants exactly as `chrome_button_style` and `chrome_slider_style` do:
  - `pub fn playlist_scrollable_style() -> scrollable::Style` — builds the whole
    struct (no `Default` impl exists for `scrollable::Style`, so all five fields
    are set):
    - `container`: `container::Style { text_color: Some(theme::TEXT),
      background: Some(theme::LCD_BACKGROUND.into()), border: Border { color:
      theme::PANEL_EDGE_DARK, width: 1.0, radius: 0.0.into() }, shadow: Shadow {
      color: theme::PANEL_EDGE_LIGHT, offset: Vector::new(1.0, 1.0), blur_radius:
      0.0 }, snap: false }` — the near-black recess with the same dark-border +
      light-offset-shadow sunken edge approximation `chrome_button_style` uses
      for its pressed state.
    - `vertical_rail` and `horizontal_rail`: `scrollable::Rail { background:
      Some(Background::Color(theme::WINDOW_BACKGROUND)), border: Border { color:
      theme::PANEL_EDGE_DARK, width: 1.0, radius: 0.0.into() }, scroller:
      scrollable::Scroller { background: Background::Color(theme::BUTTON_FACE),
      border: Border { color: theme::PANEL_EDGE_LIGHT, width: 1.0, radius:
      0.0.into() } } }` — a dark track with a raised chrome scroller, so the
      scrollbar reads on the well.
    - `gap: None`.
    - `auto_scroll`: `scrollable::AutoScroll { background:
      Background::Color(theme::LCD_BACKGROUND), border: Border { color:
      theme::PANEL_EDGE_LIGHT, width: 1.0, radius: 0.0.into() }, shadow: Shadow
      { color: theme::PANEL_EDGE_DARK, offset: Vector::new(1.0, 1.0),
      blur_radius: 0.0 }, icon: theme::TEXT }` (only drawn during touch
      auto-scroll, but the struct is total). Pure and status-independent:
      classic Winamp scrollbars give no hover/drag feedback, so the closure
      ignores the `scrollable::Status` iced passes.
  - `pub fn playlist_row_style(status: button::Status) -> button::Style` —
    `background: None` for `Active`/`Disabled`, `Some(Background::Color(
    theme::BUTTON_FACE))` for `Hovered`, `Some(Background::Color(
    theme::BUTTON_FACE_PRESSED))` for `Pressed`; `text_color: theme::TEXT`;
    everything else from `button::Style::default()` (no border or shadow —
    playlist rows are flat). The transparent Active face lets the well's
    near-black show through; hover/press lift and sink the row using the same
    faces `chrome_face` names.
  - `#[cfg(test)] mod tests` additions: a `playlist_row_style` ladder test
    (`Active` background `None`, `Hovered` `BUTTON_FACE`, `Pressed`
    `BUTTON_FACE_PRESSED`, every status `text_color == TEXT`), and a
    `playlist_scrollable_style` test pinning the container's `LCD_BACKGROUND`
    background, `PANEL_EDGE_DARK` border and `PANEL_EDGE_LIGHT` offset shadow,
    plus the rails' `BUTTON_FACE` scroller.
- `src/ui/views.rs`: in `scrollable_list`, replace the default-styled `Button`
  rows and the `iced::widget::button::background(theme, status)` current-row
  override with the new styles: every row gets `.style(|_theme, status|
  style::playlist_row_style(status))`, and the current row overrides only
  `background` to `Background::Color(super::theme::PLAYING_ROW_HIGHLIGHT)` on
  top of `playlist_row_style(status)` (keeping its light text and hover/press
  face, exactly as before). Add `.style(|_theme, _status|
  style::playlist_scrollable_style())` to the `Scrollable`, whose `width` /
  `height` stay `Fill` / `FillPortion(3)`. `scrollable_list` is the single
  builder behind `view_artists`, `view_albums`, and `view_songs`, so all three
  browse levels pick up the chrome with no signature change.
- `README.md`: extend the Status sentence to say the browse list is framed as a
  sunken Winamp playlist well with chrome rows; leave the `tumwater:prompt`
  block untouched.

**Files touched.** `src/ui/style.rs`, `src/ui/views.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo doc` with rustdoc warnings denied, `cargo test`).
- The new `playlist_row_style` and `playlist_scrollable_style` tests pass,
  pinning the row background ladder, the well's background/border/shadow
  colours, and the rails' scroller face.
- The existing browse tests still pass unchanged —
  `browse_views_construct_over_the_loaded_library`,
  `browse_views_construct_over_an_empty_list`,
  `empty_list_label_names_the_empty_browse_level`, and
  `browse_placeholder_distinguishes_loading_from_an_empty_list` build the same
  widget trees through the restyled `scrollable_list`.
- `cargo run`: the browse list sits in a near-black sunken well with a dark
  track scrollbar and a chrome scroller, its rows show light text that lifts on
  hover and sinks on press, and the playing row keeps its blue selection bar;
  manual check — the build and tests are the primary gate.

### Style the volume and equalizer sliders as sunken Winamp grooves with raised chrome thumbs (found 2026-10-07, done 2026-10-07)

Found by plan 2026-10-07, taking the slider half of the "transport button and
slider chrome" follow-up the bevel Done entry defers and the transport-button
Done entry leaves open ("Slider chrome is a separate, later plan"). The
bevel layer (`src/ui/style.rs`: `bevel_edges`, `lcd_well`, `raised_panel`)
and `style::chrome_button_style` now dress the panels and buttons, but every
`Slider` / `VerticalSlider` in `src/ui/views.rs` — the volume slider, the
preamp slider, and the ten EQ band sliders — still renders with iced's
default theme slider, a thin rail and a round handle, not the base skin's
dark sunken groove with a blocky raised chrome thumb.

**Goal.** Add one pure `chrome_slider_style(status)` to `src/ui/style.rs` that
maps iced's `slider::Status` to the base skin's groove and thumb, and route
every slider through it, so the volume, preamp, and ten band sliders read as
Winamp chrome. No new theme colours and no signature changes: the function
reuses the existing base-skin constants, exactly as `chrome_button_style`
does.

**Approach.**

- `src/ui/style.rs`: add `slider` to the existing `iced::widget::{...}`
  import (`Slider`/`VerticalSlider` share one `slider::Style`, `slider::Rail`,
  `slider::Handle`, and `slider::HandleShape` — `iced_widget`'s
  `vertical_slider` re-exports the slider types, so both widgets take the same
  style function). Add:
  - `pub fn chrome_slider_style(status: slider::Status) -> slider::Style` —
    pure, needs no theme, the testable heart (like `chrome_button_style`).
    Build:
    - `rail`: `slider::Rail { backgrounds: (Background::Color(theme::LCD_BACKGROUND),
      Background::Color(theme::LCD_BACKGROUND)), width: 4.0, border: Border
      { color: theme::PANEL_EDGE_DARK, width: 1.0, radius: 0.0.into() } }`.
      iced draws `backgrounds.0` as the segment left of / below the handle and
      `backgrounds.1` as the remainder (`iced_widget-0.14.2/src/slider.rs`);
      both are the near-black `LCD_BACKGROUND`, so the rail is one uniform
      sunken groove and the thumb — not a fill colour — is the value
      indicator, as in the base skin. `Rail` carries a single-colour `Border`
      and no shadow, so the sunken edge is approximated by the dark 1px rail
      outline.
    - `handle`: `slider::Handle { shape: slider::HandleShape::Rectangle {
      width: 8, border_radius: 0.0.into() }, background:
      Background::Color(face), border_width: 1.0, border_color:
      theme::PANEL_EDGE_LIGHT }`. The 8px rectangle gives a blocky thumb
      (full slider width, 8px thick on the long axis) and the light 1px
      border approximates the raised chrome edge — the same light-border
      trick `chrome_button_style` uses within iced's single-colour `Border`.
      `face` matches `status`, reusing the button face shades: `Active` →
      `BUTTON_FACE`, `Hovered` → `BUTTON_FACE_HOVERED`, `Dragged` →
      `BUTTON_FACE_PRESSED` (the match is exhaustive — `slider::Status` has
      no `Disabled`).
  - `#[cfg(test)] mod tests` additions: `chrome_slider_style(Active)` has
    `rail.backgrounds == (Background::Color(LCD_BACKGROUND),
    Background::Color(LCD_BACKGROUND))`, `rail.width == 4.0`,
    `rail.border.color == PANEL_EDGE_DARK`, `rail.border.width == 1.0`,
    `handle.shape == HandleShape::Rectangle { width: 8, border_radius:
    0.0.into() }`, `handle.background == Background::Color(BUTTON_FACE)`,
    `handle.border_width == 1.0`, `handle.border_color == PANEL_EDGE_LIGHT`;
    `Hovered` and `Dragged` keep the rail unchanged and swap only the handle
    face to `BUTTON_FACE_HOVERED` / `BUTTON_FACE_PRESSED`.
- `src/ui/views.rs`: chain `.style(|_theme, status|
  style::chrome_slider_style(status))` onto the volume `Slider` in
  `view_transport_controls`, the preamp `Slider`, and the `VerticalSlider` in
  `view_equalizer`'s band loop. These are the only `Slider` /
  `VerticalSlider` call sites in the crate, so all three slider kinds pick up
  the chrome with no builder or signature change; the browse row buttons in
  `scrollable_list` keep their own highlight style and are untouched.
- `README.md`: in the Status block, say the volume slider and the equalizer's
  preamp and ten band sliders are drawn as sunken grooves with raised chrome
  thumbs; leave the `tumwater:prompt` block untouched.

**Files touched.** `src/ui/style.rs`, `src/ui/views.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo doc` with rustdoc warnings denied, `cargo test`).
- The new `chrome_slider_style` tests pass, pinning the uniform dark rail, its
  4px width and dark 1px border, the 8px rectangular handle with its light
  border, and the three status-driven handle faces.
- The existing `views` tests still pass unchanged —
  `transport_controls_construct_for_both_play_states_volume_endpoints_and_repeat_states`
  and `equalizer_panel_constructs_for_both_states_and_gain_endpoints` build
  the same widget trees through the restyled sliders.
- `cargo run`: the volume, preamp, and EQ band sliders render as dark sunken
  grooves with a blocky grey thumb that lightens on hover and darkens while
  dragging; manual check — the build and tests are the primary gate.

### Style the transport buttons as raised Winamp chrome (found 2026-10-07, done 2026-10-07)

Found by plan 2026-10-07, taking the "transport button and slider chrome"
follow-up the bevel Done entry defers. The bevel layer (`src/ui/style.rs`:
`bevel_edges`, `lcd_well`, `raised_panel`) frames the panels, but
`labeled_button` in `src/ui/views.rs` still builds a default-styled `Button`,
so the Play/Pause, Stop, Previous, Next, and Repeat controls and the browse
Back button render with iced's theme button (the palette's `TITLE_BLUE`
primary) instead of the base skin's raised chrome. iced 0.14's `button::Style`
carries a single-colour `Border` and a `Shadow` (`iced_core/src/border.rs`:
`color`, `width`, `radius`; `iced_core/src/shadow.rs`: `color`, `offset`,
`blur_radius`), so — as with the panel bevel — the two-tone edge is composed
from a light border plus a dark offset shadow.

**Goal.** Add one pure `chrome_button_style(status)` to `src/ui/style.rs` that
maps iced's `button::Status` to the base skin's raised (Active/Hovered) and
sunken (Pressed) chrome, and route every `labeled_button` through it, so the
five transport buttons and the Back button read as Winamp chrome. Slider
chrome is a separate, later plan (as the bevel entry notes).

**Approach.**

- `src/ui/theme.rs`: add three public `Color` constants beside the existing
  base-skin colours, each `Color::from_rgb`, in the module's existing
  doc-comment style:
  - `BUTTON_FACE` — the raised chrome button face,
    `Color::from_rgb(0.30, 0.30, 0.30)` (lighter than `WINDOW_BACKGROUND` so
    the button reads as raised).
  - `BUTTON_FACE_HOVERED` — the hover face,
    `Color::from_rgb(0.38, 0.38, 0.38)`.
  - `BUTTON_FACE_PRESSED` — the pressed (sunken) face,
    `Color::from_rgb(0.22, 0.22, 0.22)`.
- `src/ui/style.rs`: import `button`, `Background`, `Border`, `Shadow`, and
  `Vector` from `iced` (all re-exported at the crate root). Add:
  - `pub fn chrome_button_style(status: button::Status) -> button::Style` —
    pure, needs no theme, the testable heart (like `bevel_edges`). Match
    `status`:
    - `Active` → edges `(PANEL_EDGE_LIGHT, PANEL_EDGE_DARK)`, face
      `BUTTON_FACE`.
    - `Hovered` → the same edges, face `BUTTON_FACE_HOVERED`.
    - `Pressed` → reversed edges `(PANEL_EDGE_DARK, PANEL_EDGE_LIGHT)`, face
      `BUTTON_FACE_PRESSED`.
    - `Disabled` → the `Active` chrome with `theme::TEXT.scale_alpha(0.5)` as
      `text_color` (no button in this app is disabled, but the match must be
      exhaustive).

    Build
    `button::Style { background: Some(Background::Color(face)), text_color:
    theme::TEXT, border: Border { color: top_left, width: 1.0, radius:
    0.0.into() }, shadow: Shadow { color: bottom_right, offset:
    Vector::new(1.0, 1.0), blur_radius: 0.0 }, ..button::Style::default() }`.
    The 1px light border shows on the top/left and the dark (or, when pressed,
    light) no-blur shadow offset down-right shows on the bottom/right — the
    same two-tone edge the panel bevel draws, approximated within `Border`'s
    single colour.
  - `#[cfg(test)] mod tests` additions: `chrome_button_style(Active)` has
    `background == Some(Background::Color(BUTTON_FACE))`, `text_color ==
    TEXT`, `border.color == PANEL_EDGE_LIGHT`, `border.width == 1.0`,
    `shadow.color == PANEL_EDGE_DARK`, `shadow.offset == Vector::new(1.0,
    1.0)`, `shadow.blur_radius == 0.0`; `Hovered` differs from `Active` in
    `background == BUTTON_FACE_HOVERED`; `Pressed` reverses to `border.color ==
    PANEL_EDGE_DARK` / `shadow.color == PANEL_EDGE_LIGHT` with `background ==
    BUTTON_FACE_PRESSED`; `Disabled` keeps the Active edges and sets
    `text_color.a < TEXT.a`.
- `src/ui/views.rs`: in `labeled_button`, chain `.style(|_theme, status|
  style::chrome_button_style(status))` onto the button. `labeled_button` is the
  single builder behind all five transport buttons (`view_transport_controls`)
  and the Back button (`view_back_button`), so both pick up the chrome with no
  signature change; the browse row buttons in `scrollable_list` keep their own
  highlight style and are untouched.
- `README.md`: refresh the Status sentence to say the transport controls and
  Back button are drawn as raised Winamp chrome buttons; leave the
  `tumwater:prompt` block untouched.

**Files touched.** `src/ui/style.rs`, `src/ui/theme.rs`, `src/ui/views.rs`,
`README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo doc` with rustdoc warnings denied, `cargo test`).
- The new `chrome_button_style` tests pass, pinning the raised/sunken edge
  colours, the three face shades, and the disabled text dimming.
- The existing `views` tests still pass unchanged —
  `transport_controls_construct_for_both_play_states_volume_endpoints_and_repeat_states`
  and `now_playing_bar_and_back_button_construct` build the same widget trees
  through the restyled `labeled_button`.
- `cargo run`: the transport buttons and Back button render as raised gray
  chrome with a light top/left and dark bottom/right edge, darkening on hover
  and reading sunken while pressed; manual check — the build and tests are the
  primary gate.

### Add a Winamp two-tone bevel layer and frame the Now Playing and equalizer panels (found 2026-10-06, done 2026-10-06)

Found by plan 2026-10-06, following the steward drift note's call for custom
widget styling. The palette landed (previous Done entry), but every panel is
still flat: iced 0.14's `Border` is a single colour of uniform width
(`iced_core/src/border.rs`: `color`, `width`, `radius` only), so no view draws
the base skin's defining 3D edge — light top/left, dark bottom/right on raised
chrome, reversed in a sunken LCD well.

**Goal.** Add a small `src/ui/style.rs` that composes the bevel from 1px `Rule`
edges and wrap the two panels with intrinsic height: the Now Playing bar as a
sunken dark LCD well and the equalizer panel as a raised chrome panel. This is
the reusable styling layer the drift note asks for; transport button and slider
chrome is a separate, later plan that styles against these same edge colours.

**Approach.**

- `src/ui/theme.rs`: add three public `Color` constants beside the existing
  base-skin colours, each `Color::from_rgb`, in the module's existing
  doc-comment style:
  - `PANEL_EDGE_LIGHT` — the light top/left bevel edge,
    `Color::from_rgb(0.55, 0.55, 0.55)`.
  - `PANEL_EDGE_DARK` — the dark bottom/right bevel edge,
    `Color::from_rgb(0.05, 0.05, 0.05)`.
  - `LCD_BACKGROUND` — the near-black LCD recess behind the green title,
    `Color::from_rgb(0.05, 0.05, 0.05)`.
- `src/ui/style.rs` (new file, declared `mod style;` beside `mod theme;` in
  `src/ui/mod.rs`), with `//!` module docs and a doc comment on every public
  item (the `docs` stage denies rustdoc warnings):
  - `pub fn bevel_edges(raised: bool) -> (Color, Color)` — pure, returns the
    `(top_left, bottom_right)` edge colours: `raised` → `(PANEL_EDGE_LIGHT,
    PANEL_EDGE_DARK)`, sunken → `(PANEL_EDGE_DARK, PANEL_EDGE_LIGHT)`. This is
    the testable heart of the bevel; the widget composition only consumes it.
  - `fn horizontal_edge<'a, R>(color: Color) -> Element<'a, Message, Theme, R>`
    and `fn vertical_edge<'a, R>(color: Color) -> Element<'a, Message, Theme,
    R>` (`R: iced::advanced::Renderer`) — a 1px `Rule` (`Rule::horizontal(1.0)`
    / `Rule::vertical(1.0)`) styled with `rule::Style { color, radius:
    0.0.into(), fill_mode: rule::FillMode::Full, snap: true }`. A `Rule` fills
    whatever length it is laid out against; the [`Stack`] composition in
    `beveled` is what constrains that to the panel's resolved size. The
    renderer is generic so the layout test below can drive the composition
    with iced's null `()` renderer.
  - `fn bevel_overlay<'a, R>(top_left: Color, bottom_right: Color) ->
    Element<'a, Message, Theme, R>` — a `Length::Fill` `Column` of top edge, a
    `Length::Fill` `Row` of left edge + `Space::new().width(Length::Fill)` +
    right edge, and bottom edge, coloured from `bevel_edges`; the spacer puts
    the right edge on the panel's outer edge.
  - `fn beveled<'a, R>(content: Element<'a, Message, Theme, R>, raised: bool)
    -> Element<'a, Message, Theme, R>` — a `Stack` whose base layer is
    `content` and whose overlay is `bevel_overlay`; the stack is
    `width(Length::Fill)` and `height(Length::Shrink)`, so the panel fills the
    available width but takes the content's intrinsic height. The `Stack`
    sizes itself to its base layer and lays the overlay out against that
    resolved size, so the bevel's vertical edges span the panel exactly
    instead of the window's leftover height.
  - `pub fn lcd_well<'a>(content: impl Into<Element<'a, Message>>) ->
    Element<'a, Message>` — wraps `content` in a full-width `Container` with
    `background: LCD_BACKGROUND` and a few px of padding, then
    `beveled(.., false)`. No `text_color` is set, so the `"Now Playing: "`
    caption keeps the theme text colour and only the title's explicit
    `LCD_GREEN` stays green.
  - `pub fn raised_panel<'a>(content: impl Into<Element<'a, Message>>) ->
    Element<'a, Message>` — `beveled(.., true)` around content on the window
    face (no extra background, so it composes with the existing theme).
  - `#[cfg(test)] mod tests`: `bevel_edges(true)` equals
    `(theme::PANEL_EDGE_LIGHT, theme::PANEL_EDGE_DARK)` and `bevel_edges(false)`
    equals the reverse; `lcd_well(Text::new("x"))` and
    `raised_panel(Text::new("x"))` build and request a `Length::Fill` width
    with a `Length::Shrink` (content-driven) height; and
    `beveled_tracks_content_height_and_fills_the_available_width` lays the
    composition out with iced's null `()` renderer under a 500×400 limit and
    asserts the resolved node is 500×7 for a 20×7 content — the height
    regression guard the builder-only tests cannot be.
- `src/ui/views.rs`:
  - In `view_now_playing`, wrap the existing `Row` in
    `super::style::lcd_well(..)`.
  - In `view_equalizer`, wrap the existing `Column` in
    `super::style::raised_panel(..)`.
  - Add `use super::style;`; no signature changes, so the existing view
    construction tests compile and pass unchanged.
- `README.md`: refresh the Status sentence to say the Now Playing bar and
  equalizer are framed with Winamp's raised/sunken bevels; leave the
  `tumwater:prompt` block untouched.

**Files touched.** `src/ui/style.rs` (new), `src/ui/theme.rs`, `src/ui/mod.rs`,
`src/ui/views.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo doc` with rustdoc
  warnings denied, `cargo test`).
- The new `style` unit tests pass; the existing `views` construction tests
  (`now_playing_bar_and_back_button_construct`,
  `equalizer_panel_constructs_for_both_states_and_gain_endpoints`) still pass
  with the wrapped panels.
- `cargo run`: the Now Playing bar reads as a dark sunken LCD well (light
  top/left, dark bottom/right edges) and the equalizer as a raised panel (the
  reverse); manual check — the build and tests are the primary gate.

### Add a Winamp 2.x base-skin palette and apply it as the app theme (done 2026-10-06)

Found by plan 2026-10-06, following the steward drift note above.

**Goal.** Every widget still renders in iced 0.14's default theme: no custom
`Theme`/`Palette` is built anywhere (`grep -rn 'Theme\|palette' src` matches
only the one highlight-colour button style in `views.rs`), so the window does
not yet look like Winamp. Land the fidelity foundation the steward note asks
for: a `src/ui/theme.rs` module naming the Winamp 2.x base-skin colours and a
custom `iced::Theme` built from them, applied app-wide in `init_ui`, plus the
Now Playing title in Winamp's LCD green. Later fidelity plans (title bar,
panel bevels, playlist chrome) style against these names; this plan only
introduces the palette and the app-wide application, so it stays one run and
leaves no widget unstyled by accident.

**Approach.**

- New file `src/ui/theme.rs` (a sibling of `views.rs`/`transport.rs`, declared
  `mod theme;` in `src/ui/mod.rs`), with module `//!` docs and a doc comment on
  every public item (the `docs` stage denies rustdoc warnings):
  - Public `Color` constants for the Winamp 2.x base skin, each a `const` via
    `Color::from_rgb`, exactly as the existing `views::PLAYING_ROW_HIGHLIGHT`
    is built:
    - `WINDOW_BACKGROUND` — the dark gray window face,
      `Color::from_rgb(0.18, 0.18, 0.18)`.
    - `TEXT` — light chrome text, `Color::from_rgb(0.87, 0.87, 0.87)`.
    - `TITLE_BLUE` — Winamp's title-bar/selection blue,
      `Color::from_rgb(0.0, 0.0, 0.55)`.
    - `LCD_GREEN` — the playlist/LCD green, `Color::from_rgb(0.0, 1.0, 0.0)`.
    - `PLAYING_ROW_HIGHLIGHT` — moved here from `views.rs` with the same value
      `Color::from_rgb(0.25, 0.5, 1.0)`, so all UI colours live in one module.
  - `pub fn palette() -> iced::theme::Palette` filling all six fields:
    `background: WINDOW_BACKGROUND`, `text: TEXT`, `primary: TITLE_BLUE`,
    `success: LCD_GREEN`, and `warning`/`danger` set to
    `Color::from_rgb(1.0, 0.65, 0.0)` / `Color::from_rgb(1.0, 0.2, 0.2)`.
  - `pub fn winamp_theme() -> iced::Theme` returning
    `iced::Theme::custom("Winamp", palette())`.
  - `#[cfg(test)] mod tests`: `winamp_theme().palette().background` and
    `.text` equal the named constants; `winamp_theme().extended_palette().is_dark`
    is true; `winamp_theme().to_string() == "Winamp"`.
- `src/ui/mod.rs`:
  - Add `mod theme;` beside `mod transport;` / `mod views;`.
  - Chain `.theme(|_: &WinampPlayer| theme::winamp_theme())` onto the
    `iced::application(..)` builder in `init_ui`, before `.run()`.
- `src/ui/views.rs`:
  - Delete the local `PLAYING_ROW_HIGHLIGHT` const and use
    `super::theme::PLAYING_ROW_HIGHLIGHT` in `scrollable_list` instead; drop
    `Color` from the `iced` import list (it is no longer used once the const
    moves).
  - In `view_now_playing`, colour the current-track `Text` with
    `super::theme::LCD_GREEN` via `Text::color` so the Now Playing bar reads as
    Winamp's green LCD; the `"Now Playing: "` caption keeps the theme text
    colour.
- `README.md`: refresh the Status sentence to say the window uses a Winamp
  base-skin colour theme; leave the `tumwater:prompt` block untouched.

**Files touched.** `src/ui/theme.rs` (new), `src/ui/mod.rs`,
`src/ui/views.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo doc` with rustdoc
  warnings denied, `cargo test`).
- The new `theme` unit tests pass; the existing `views` construction tests
  (`now_playing_bar_and_back_button_construct`,
  `browse_views_construct_over_the_loaded_library`) still pass with the moved
  constant, and no unused-import or `dead_code` warning remains.
- `cargo run`: the window renders on the dark Winamp face with light text and
  a green Now Playing title, while the Songs view's playing-row highlight is
  unchanged (manual check — build + tests are the primary gate).

### Add the Winamp equalizer panel: on/off, preamp, and ten band sliders (done 2026-10-06)

Found by plan 2026-10-06.

**Goal.** The equalizer is the largest classic Winamp UI element the app still
lacks — the README's own mockup (`docs/screenshots/equalizer.png`) shows it
docked under the main window, but no equalizer code exists (`grep -ri equaliz
src` finds nothing). Land the equalizer's core controls as a panel in the
existing single window: an EQ on/off toggle, a preamp slider, and ten vertical
band sliders (60 Hz–16 kHz, ±12 dB) whose values live in the shared `AppState`,
mirroring how `volume` and `repeat` already work. Out of scope (later plans):
preset curves, a separate/docked OS window, window-shade mode, and any real
audio processing (the Apple Music stub has no audio pipeline yet).

**Approach.**

- New file `src/equalizer.rs`, the equalizer data model — a sibling of
  `library.rs` and `state.rs` so both the shared state and the UI can depend on
  it without a `state` ← `ui` cycle:
  - `pub const BAND_FREQUENCIES: [&str; 10] = ["60", "170", "310", "600", "1K", "3K", "6K", "12K", "14K", "16K"];`
  - `pub const BAND_COUNT: usize = BAND_FREQUENCIES.len();`
  - `pub const GAIN_MIN_DB: f32 = -12.0;` and `pub const GAIN_MAX_DB: f32 = 12.0;`
  - `#[must_use] pub fn clamp_gain(gain: f32) -> f32` — `clamp` to the range,
    mapping NaN to `0.0`, exactly as `state::clamp_volume` does for volume.
  - A `#[cfg(test)] mod tests` pinning `clamp_gain` at both bounds, in range,
    and NaN, plus `BAND_FREQUENCIES.len() == BAND_COUNT`.
- `src/main.rs`: add `mod equalizer;` beside the other `mod` declarations.
- `src/state.rs`:
  - Add to `AppState`: `pub eq_enabled: bool`, `pub eq_preamp: f32`,
    `pub eq_bands: [f32; equalizer::BAND_COUNT]`.
  - Extend the manual `Default` impl: `eq_enabled: false`, `eq_preamp: 0.0`,
    `eq_bands: [0.0; equalizer::BAND_COUNT]` (flat), and update the
    "Initial state" doc comment.
  - Add `toggle_equalizer(&mut self)` (flips `eq_enabled`, like
    `toggle_repeat`), `set_eq_preamp(&mut self, gain: f32)` (stores
    `equalizer::clamp_gain(gain)`), and `set_eq_band(&mut self, band: usize,
    gain: f32)` (clamps and stores into `eq_bands[band]`, ignoring an
    out-of-range `band` via `get_mut`).
  - Tests: default eq is off/flat; `toggle_equalizer` flips only `eq_enabled`;
    both setters clamp an out-of-range value and `set_eq_band` ignores an
    out-of-range band index; none of the eq mutators change
    `current_track`/`volume` (reuse the existing
    `assert_keeps_track_and_volume` helper).
- `src/ui/views.rs`:
  - Import `VerticalSlider` from `iced::widget` and `BAND_COUNT` /
    `BAND_FREQUENCIES` / `GAIN_MIN_DB` / `GAIN_MAX_DB` from `crate::equalizer`.
  - `fn eq_enabled_label(enabled: bool) -> &'static str` returning `"EQ: On"`
    / `"EQ: Off"`, mirroring `repeat_label`.
  - `pub fn view_equalizer(enabled: bool, preamp: f32, bands: &[f32; BAND_COUNT]) -> Element<'static, Message>`
    — a `Column` of:
    - a header `Row` with `labeled_button(eq_enabled_label(enabled), Message::ToggleEqualizer)`;
    - a preamp `Row` of `Text::new("Preamp")` and
      `Slider::new(GAIN_MIN_DB..=GAIN_MAX_DB, preamp, Message::EqPreampChange).step(1.0).width(Length::Fixed(150.0))`;
    - a bands `Row` of `BAND_COUNT` `Column`s, each a
      `VerticalSlider::new(GAIN_MIN_DB..=GAIN_MAX_DB, bands[i], move |gain| Message::EqBandChange(i, gain)).step(1.0).height(Length::Fixed(100.0))`
      above `Text::new(BAND_FREQUENCIES[i]).size(12)`.
  - Tests: `eq_enabled_label` mirrors the flag; `view_equalizer` constructs for
    enabled/disabled and both gain endpoints (mirroring the existing
    transport-controls construction test).
- `src/ui/mod.rs`:
  - Add `Message::{ToggleEqualizer, EqPreampChange(f32), EqBandChange(usize, f32)}`.
  - `update` arms: `ToggleEqualizer => mutate_state(player, AppState::toggle_equalizer)`;
    `EqPreampChange(gain) => mutate_state(player, |s| s.set_eq_preamp(gain))`;
    `EqBandChange(band, gain) => mutate_state(player, |s| s.set_eq_band(band, gain))`.
  - In `view`, read `eq_enabled`/`eq_preamp`/`eq_bands` in the existing
    state-lock block and push
    `views::view_equalizer(eq_enabled, eq_preamp, &eq_bands)` after
    `views::view_transport_controls(..)`.
- `src/ui/tests.rs`: add update tests — `Message::ToggleEqualizer` flips the
  shared flag; `EqPreampChange`/`EqBandChange` store clamped values (e.g.
  `EqBandChange(0, 99.0)` stores `12.0`).
- `README.md`: refresh the Status and Usage lines to describe the equalizer
  panel; leave the `tumwater:prompt` block untouched.

**Files touched.** `src/equalizer.rs` (new), `src/main.rs`, `src/state.rs`,
`src/ui/views.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo doc` with rustdoc
  warnings denied, `cargo test`).
- The new `equalizer`, `state`, and `views` unit tests and the `ui` update
  tests listed above pass.
- No `dead_code`/unused warnings: every new constant, field, and method is read
  by the UI or its tests.
- `cargo run`: below the transport row the window shows an "EQ: Off" button, a
  Preamp slider, and ten band sliders labelled `60`…`16K`; pressing the button
  changes its label to "EQ: On", and dragging any slider moves it (manual check
  — build + tests are the primary gate).


### Add a Repeat toggle to the transport controls (done 2026-10-06)

Found by plan 2026-10-06.

**Goal.** Match Winamp's Repeat control: a Repeat button in the transport row that switches Previous/Next through the current album's songs between wrap-around (Repeat on) and stop-at-the-edge (Repeat off) stepping. Today `transport::next_track_id` and `transport::previous_track_id` always wrap at the album's ends, so the current default is indistinguishable from Repeat-on; this feature makes wrapping explicitly opt-in and starts Repeat off, as Winamp does.

**Approach.**
- `src/state.rs`: add `pub repeat: bool` to `AppState` (initialised `false` in `impl Default`), plus a `pub fn toggle_repeat(&mut self)` mirroring `toggle_playing`. Update `default_state_is_stopped_at_half_volume` to assert `!state.repeat`; add `toggle_repeat_flips_only_the_repeat_flag` (flips both ways, leaves `current_track`, `is_playing`, `volume` untouched). The `stop_clears_playing_flag_and_keeps_current_track` test builds `AppState` with `..Default::default()`, so it needs no change.
- `src/ui/transport.rs`: add a `repeat: bool` parameter to `stepped_track_id`, `next_track_id`, and `previous_track_id`. With `repeat == false`, a step that would cross the boundary stays on the edge instead of wrapping — Next from the last song returns the last song's id, Previous from the first returns the first's — so the button re-lands on the current track rather than moving or doing nothing. With `repeat == true`, and in every no-current / empty-list / single-song case, behavior is unchanged: the no-current branch still lands on the forward/backward edge. Update the two wrap tests (`next_wraps_from_last_to_first`, `previous_wraps_from_first_to_last`) to pass `true`; give the other existing transport tests the new argument (any value where the expectation does not depend on repeat); add `next_stays_on_last_without_repeat` and `previous_stays_on_first_without_repeat`.
- `src/ui/views.rs`: `view_transport_controls` gains a `repeat: bool` parameter and pushes a Repeat button (via the existing `labeled_button`) after the Next button; add `fn repeat_label(repeat: bool) -> &'static str` beside `play_pause_label`, returning "Repeat: Off"/"Repeat: On", and pin it in `repeat_label_mirrors_repeat_state`. Extend `transport_controls_construct_for_both_play_states_and_volume_endpoints` to also loop over `[false, true]` for repeat.
- `src/ui/mod.rs`: add `Message::ToggleRepeat`; its update arm routes to `mutate_state(player, AppState::toggle_repeat)`. `step_track`'s `step` parameter becomes `fn(&[Song], Option<&str>, bool) -> Option<String>` and the Next/Previous arms pass `state.repeat` (read under the existing lock), so the arm wiring keeps the shared flag in step. `view` reads `repeat` in its lock block and passes it to `views::view_transport_controls`. Add wiring tests: `toggle_repeat_flips_shared_state` (drive `Message::ToggleRepeat` through `update`, like `play_pause_toggles_is_playing`) and a stepping-wiring test driving `Message::NextTrack` from the last song — wraps when `state.repeat` is true, stays on the last song when false.

**Files touched.** `src/state.rs`, `src/ui/transport.rs`, `src/ui/views.rs`, `src/ui/mod.rs` (each including its `#[cfg(test)]` module).

**Acceptance criteria.**
- `AppState::default().repeat` is `false`; `toggle_repeat` flips it and nothing else.
- `next_track_id`/`previous_track_id` with `repeat = false` stop at the album's edge (return the current edge song's id); with `repeat = true`, and in every no-current/empty/single-song case, they behave exactly as before the change.
- The transport row renders a Repeat button whose label tracks the shared `repeat` flag, and pressing it flips that flag in shared state.
- `make check` is green: `cargo fmt --check`, `cargo clippy --all-targets`, and the full `cargo test` suite pass.


### Highlight the currently playing song in the Songs browse view (done 2026-10-06)

Found by plan 2026-10-06.

**Goal.** Winamp's playlist editor draws a highlighted bar on the track that is
currently playing, so you can see where in the list the music is and watch it
move as you step. This player has no such cue: the Songs view renders every row
identically, so after clicking a song (or pressing Next/Previous) there is no
way to tell which song in the album is current. Mark the row whose id equals the
shared `current_track` — a `▶` prefix on its title and a highlighted button
background — so the song list reads as a playlist. The marker is pure data
derived state (current id vs. the displayed songs), so it stays in the view
layer like the rest of `views.rs` and never touches the service seam.

**Approach.** All changes are in the view layer; `AppleMusicService`, `state`,
`library`, and the transport stepping arithmetic are untouched.

- `src/ui/views.rs`:
  - Extend the browse-row tuple from `(String, &'static str, Message)` to
    `(String, &'static str, Message, bool)` — the new bool is whether this row
    is the currently playing track. `scrollable_list` reads it: when true, it
    prefixes the title with `"▶ "` and gives the button a highlighted
    background (a `Button::style` closure returning a
    `iced::widget::button::Style` with a `Background::Color`); when false, the
    plain button as today. `artist_row` and `album_row` return `false` — the
    marker only ever applies inside an album's song list.
  - `song_row(song: &Song, current_track: Option<&str>)` computes the flag as
    `Some(&song.id) == current_track`.
  - `view_songs(songs: &[Song], current_track: Option<&str>)` takes the current
    track id and passes it to `song_row`. `view_artists`/`view_albums`
    signatures are unchanged.
  - Tests: update `song_row_uses_title_and_selects_the_song` (and the
    `artist_row`/`album_row` tests) for the 4-tuple, and add a
    `song_row_marks_the_current_track` test pinning the flag for `Some(id)`
    matching the song, a different `Some`, and `None`. Update the two
    `browse_views_construct_*` construction tests for the new `view_songs`
    signature and add a construction case passing a current track, following
    the existing pattern (iced `Element`s are not introspectable, so the
    row-tuple flag is the testable contract and construction tests cover
    rendering without panicking).
- `src/ui/mod.rs` `view()`: the current-track id must reach `view_songs`. The
    Songs arm acquires the shared-state lock briefly and passes
    `state.current_track.as_deref()` — a second short `blocking_lock` rather
    than a per-frame `Option<String>` clone, consistent with the earlier
    commit that removed the per-frame `current_track` clone (4d7a585). The
    label tuple block above stays as is.

**Files touched.** `src/ui/views.rs`, `src/ui/mod.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `song_row_marks_the_current_track`
  row-flag test and the updated construction tests.
- `cargo fmt --check` and `cargo clippy --all-targets` are clean.
- `cargo run`: in a Songs view, clicking a song marks it (`▶` + highlighted
  background); Next/Previous move the marker through the list; browsing to
  another album shows no marker (its songs do not contain the current track);
  stepping works with the marker following along. Build + tests are the
  primary gate; the visual check confirms the marker renders.

Note: the acceptance criteria require `cargo fmt --check` clean, and the only
line failing it was a pre-existing over-long `assert_ids` call in
`src/apple_music.rs` (added by c9cb1cb) — this change reformats that one call
so the gate passes; no other line in the integration seam is touched.

### Add a Stop button to the transport controls (done 2026-10-06)

Found by plan 2026-10-06.

**Goal.** The classic Winamp transport is Play, Pause, Stop, Previous, Next; this
player renders Play/Pause, Previous, and Next (plus the volume slider) but no
Stop — a Winamp-clone transport without Stop is missing one of its five core
buttons. Land a Stop button that halts playback: it clears `is_playing` while
leaving `current_track` in place, so the Now Playing bar keeps showing the
interrupted track's title (matching Winamp, where Stop stops the music and the
title stays in the display).

**Approach.** Stop is a transport control, so — following the Previous/Next
precedent, which moved transport sequencing out of the service (the transport
plan's note: "the service models the library API, not the current playlist") —
it is handled in the UI against shared state, not as a new
`AppleMusicService` method. The service's private `pause`/`next_track`/
`previous_track` stubs stay untouched.

- `src/state.rs`: add `pub fn stop(&mut self)` on `AppState` that sets
  `self.is_playing = false` and changes nothing else — the sibling of
  `toggle_playing` (whose doc comment says the Play/Pause button is its only
  caller), keeping the stop semantics beside the field they mutate rather than
  inlined at the call site. Add a `#[cfg(test)]` test
  `stop_clears_playing_flag_and_keeps_current_track` mirroring
  `toggle_playing_flips_only_the_playback_flag`: after `stop()`, `is_playing`
  is false and `current_track` and `volume` are unchanged.
- `src/ui/mod.rs`:
  - Add a `Message::Stop` variant to the `Message` enum.
  - In `update`, add a `Message::Stop` arm calling
    `player.state.blocking_lock().stop()` and returning `Task::none()` —
    synchronous, exactly like the `Message::PlayPause` arm. (The stub has no
    playback position yet, so Stop's only observable effect is the cleared
    playing flag — identical to Pause today. The distinction is the seam:
    once real playback lands, Stop also resets the track position while Pause
    keeps it. Not a design question for this plan — just record it in the
    arm's comment.)
  - In the `#[cfg(test)]` module, add a plain test
    `stop_clears_is_playing` mirroring `play_pause_toggles_is_playing`: after
    `Message::Stop` the shared state's `is_playing` is false (it stays a plain
    test because the arm uses `blocking_lock`, which panics inside an async
    runtime).
- `src/ui/views.rs`: in `view_transport_controls`, insert a Stop button
  between the Play/Pause button and the Previous button, in Winamp's order:
  `Button::new(Text::new("Stop")).on_press(Message::Stop)`, separated by the
  existing `spacer(20.0)`. The label is static (Stop is always pressable, even
  when stopped, as in Winamp), so no `play_pause_label`-style helper is
  needed; `view_transport_controls`'s signature stays unchanged.

**Files touched.** `src/state.rs`, `src/ui/mod.rs`, `src/ui/views.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `stop_clears_playing_flag_and_keeps_current_track`
  (state.rs) and `stop_clears_is_playing` (ui/mod.rs) tests.
- `cargo fmt --check` passes.
- `Message::Stop` is wired on both ends: `grep -n 'Message::Stop' src` shows
  the emitter (the Stop button's `on_press` in `view_transport_controls`) and
  the handler (the `update` arm).
- `cargo run`: with a song playing, Stop flips the Play/Pause button back to
  "Play" (is_playing cleared) while the Now Playing bar keeps the interrupted
  track's title; pressing Play resumes it (manual check — build + tests are the
  primary gate).

### Show the song's title in the Now Playing bar, not its raw id (done 2026-10-05)

Found by plan 2026-10-05.

**Goal.** `AppState.current_track` stores a track *id*, and `WinampPlayer::view`
renders it verbatim — after playing "Opening" the bar reads
"Now Playing: song-1", not the title a Winamp-style player should show.
Resolve the id against the songs currently loaded in `WinampPlayer.songs`
(the same list the songs view renders and the Previous/Next plan sequences)
and display the matching `Song.title`, falling back to the id when the track
isn't in the loaded songs and to "Nothing" when nothing is current. Keep the
resolution pure so it is testable without the UI.

**Approach.**

- `src/ui/mod.rs`: add a private
  `fn now_playing_label(songs: &[Song], current_track: Option<&str>) -> String`
  beside `WinampPlayer::view` (the formatting is a view-time concern, so the
  helper stays with the player UI rather than in `views.rs`, whose stated
  contract is data → `Element`). Semantics: `current_track` is `None` →
  `"Nothing"`; `current_track` is `Some(id)` whose id matches a `Song` in
  `songs` → that song's `title` (cloned); `Some(id)` with no match → the raw
  `id` itself (a track is current, so "Nothing" would lie). Add a
  `#[cfg(test)] mod tests` covering: no current track → "Nothing"; current id
  found in `songs` → its title; current id not in `songs` → the id; empty
  `songs` with a current id → the id. The function is pure data → `String`, so
  it is testable without the UI.
- In `WinampPlayer::view`, replace the existing
  ```rust
  let now_playing = state
      .current_track
      .clone()
      .unwrap_or_else(|| "Nothing".to_string());
  ```
  with a call that reads the id while the `blocking_lock` guard is held and
  delegates the formatting to the helper:
  ```rust
  let now_playing = now_playing_label(&player.songs, state.current_track.as_deref());
  ```
  No `Message` change and no `AppState` change: `current_track` stays the
  id (the Previous/Next plan reads it as an id via
  `current_track.as_deref()`), and the title lookup is a view-time concern.
  A track played from album A stays resolvable after navigating to an album
  B's songs only if its id is still in `player.songs` (which isn't cleared on
  navigation) — the id fallback covers any miss; sourcing titles more
  robustly is a later plan.

**Files touched.** `src/ui/mod.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `now_playing_label` unit tests (no
  current track, title lookup, unknown-id fallback, empty songs list).
- `cargo fmt --check` passes.
- `cargo run`: after playing a song from the songs view, the Now Playing bar
  shows that song's title (e.g. "Opening") instead of its id ("song-1"); at
  boot it still shows "Nothing" (manual check — build + tests are the primary
  gate).

**Update (2026-10-06, organize).** `now_playing_label` (with its unit tests)
and the Now Playing bar / transport row construction moved to
`src/ui/views.rs`, grouped with the other widget builders; the plan's
original placement — the helper beside `view` in `src/ui/mod.rs` — is
superseded.

### Make Previous/Next step through the songs of the current album (done 2026-10-05)

Found by plan 2026-10-04.

**Goal.** The transport row renders Previous and Next buttons, but in
`WinampPlayer::update` the `Message::NextTrack | Message::PreviousTrack` arm
maps to `Task::none()` — pressing them does nothing. Land functional transport:
Previous/Next advance through the songs of the currently loaded album (the
`WinampPlayer.songs` list), reusing the existing `play_track` path so the Now
Playing bar updates. Sequencing is a player concern, so the index logic is a
pure, tested function beside the player, not in `AppleMusicService` (whose
`next_track`/`previous_track` private stubs stay unused and untouched — the
service models the library API, not the current playlist).

**Approach.**

- New file `src/ui/transport.rs`, a pure module mirroring `views.rs`:
  - `pub fn next_track_id(songs: &[Song], current: Option<&str>) -> Option<String>`
    and `pub fn previous_track_id(...) -> Option<String>`.
  - Semantics: empty `songs` → `None` (no-op). `current` is `None` or not
    found in `songs` → first song for next, last song for previous. `current`
    found at index `i` → `songs[(i + 1) % len].id` for next,
    `songs[(i + len - 1) % len].id` for previous (wrap in both directions;
    repeat/shuffle semantics are a later plan).
  - A `#[cfg(test)] mod tests` covering: empty list → `None`; `current` `None`
    → first/last; unknown `current` id → first/last; mid-list advance and
    reverse; wrap from last → first (next) and from first → last (previous).
- `src/ui/mod.rs`:
  - Add `mod transport;` beside `mod views;`.
  - In `update`, split the combined arm: `Message::NextTrack` reads
    `current_track` via `player.state.blocking_lock()`, calls
    `transport::next_track_id(&player.songs, current.as_deref())`, and returns
    `Task::done(Message::TrackSelected(id))` when `Some`, else `Task::none()`.
    `Message::PreviousTrack` does the same with `previous_track_id`.
  - Dispatching `Message::TrackSelected` (rather than calling `play_track`
    directly) reuses the one existing play path — it performs `play_track`,
    which sets `state.current_track`/`is_playing`, and re-renders via
    `TrackPlayed`; the same mechanism `boot` already uses with
    `Task::done(Message::LoadArtists)` feeds the message back into `update`.

**Files touched.** `src/ui/transport.rs` (new), `src/ui/mod.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `next_track_id` / `previous_track_id`
  unit tests (empty list, no current, unknown current, mid-list advance/reverse,
  wrap at both ends).
- `cargo fmt --check` passes.
- `Message::NextTrack` / `Message::PreviousTrack` are no longer dead: the
  `update` arms no longer map to `Task::none()`.
- `cargo run`: after loading an album's songs and playing one, Previous/Next
  move the Now Playing track through that album's songs, wrapping at the ends;
  at the artist/album list (no songs loaded) they do nothing (manual check —
  build + tests are the primary gate).

### Add a working volume slider to the transport controls (done 2026-10-05)

Found by plan 2026-10-04.

**Goal.** The transport row renders Play/Pause, Previous, and Next, and
`AppState` holds a `volume: f32` (default 0.5) that nothing currently reads or
changes — the iced 0.14 port dropped the `VolumeChange` message entirely, so
the volume control must be built from scratch (message variant, `update` arm,
and widget), not re-wired to a handler that no longer exists. Land the classic
Winamp-style volume slider: add iced's `Slider` widget to the transport row in
`WinampPlayer::view` wired to a new `Message::VolumeChange(f32)` variant and
`update` arm that stores the clamped value into `state.volume`, keeping the
clamping logic in a pure, tested helper. The iced 0.14 port has landed (done
2026-10-04), so the crate compiles on iced 0.14 as-is and this plan can build.

**Approach.**

- `src/state.rs`: add `pub fn clamp_volume(volume: f32) -> f32` returning
  `volume.clamp(0.0, 1.0)`, plus a `#[cfg(test)] mod tests` covering the three
  cases: below 0 → 0.0, above 1 → 1.0, in-range value unchanged. Volume is
  shared state, so the helper lives beside `AppState` rather than in the UI
  module.
- `src/ui/mod.rs`:
  - Extend the `iced::widget` import (`Button, Column, Row, Space, Text`) to
    include `Slider`.
  - In `WinampPlayer::view`, append `Slider::new(0.0..=1.0, state.volume,
    Message::VolumeChange)` with a fixed width (`Length::Fixed(100.0)` or
    similar) to the transport `Row` that currently holds the Play/Pause,
    Previous, and Next buttons.
  - In `WinampPlayer::update`, add a `Message::VolumeChange(volume)` arm — the
    port dropped this message, so no such arm exists today — that stores
    `state::clamp_volume(volume)` into `state.volume`.
  - Use iced 0.14's `Length::Fixed` spelling near the slider (e.g.
    `Length::Fixed(100.0)`): `Length::Pixels` does not exist in iced 0.14
    (the port's Done note records the rename from `Length::Units` to
    `Length::Fixed`).
  - In the `#[cfg(test)] mod tests`, add a `volume_change_clamps_value_before_storing`
    test driving the new `Message::VolumeChange` arm: out-of-range values are
    clamped before landing in `state.volume`, an in-range value is stored as-is
    (every other `update` arm in this module carries such a test).

**Files touched.** `src/state.rs`, `src/ui/mod.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `clamp_volume` tests (clamp below 0,
  above 1, pass through in-range) and the `volume_change_clamps_value_before_storing`
  update-arm test in `src/ui/mod.rs`.
- `cargo run` shows a volume slider in the transport row; dragging it changes
  the volume (manual check — build + tests are the primary gate).
- The new `Message::VolumeChange` is wired on both ends: a `grep -n
  'VolumeChange' src` shows both the emitter (`Slider` `on_changed` in
  `view`) and the handler in `update`.

### Make the app build on iced 0.14 and render a browsable sample library (done 2026-10-04)

Found by plan 2026-10-04.

**Goal.** The crate does not compile. `src/ui/mod.rs` implements iced's
`Application` trait against the iced ≤0.13 API — `iced_native::Command`,
`async fn update`, `Length::Units`, `WinampPlayer::run(Settings::default())` —
while `Cargo.toml` pins iced 0.14, whose `Program` requires a synchronous
`update` returning `iced::Task` and is booted via the `iced::application(boot,
update, view)` builder. Independently, the UI calls
`AppleMusicService::get_favorite_artists`, `get_albums_by_artist`,
`get_songs_from_album` and imports `Artist`, `Album`, `Song`, none of which
`src/library.rs` defines. Land the artist → album → song library-browse
flow end to end: add the missing model and service methods (backed by an
in-memory sample library; real Apple Music auth/API is a later, larger plan),
port the UI to iced 0.14, and get
`cargo build` + `cargo test` green.

**Approach.**

- `src/library.rs`:
  - Add `pub struct Artist { pub id: String, pub name: String }`,
    `pub struct Album { pub id: String, pub title: String, pub artist_id: String }`,
    `pub struct Song { pub id: String, pub title: String, pub album_id: String }`,
    each deriving `Clone, Debug, PartialEq, Serialize, Deserialize`.
- `src/apple_music.rs`:
  - Add a private builder `fn sample_library() -> SampleLibrary` returning a
    small in-memory library (3 artists, 3 albums, 5 songs) in one struct so
    the browse queries and the tests share the same data.
  - Add three `pub async fn(&self, ...) -> Result<Vec<…>, AppleMusicError>`
    methods on `AppleMusicService` (the crate-local error type the improve
    loop later introduced when it dropped the `reqwest` dependency):
    `get_favorite_artists` (all artists),
    `get_albums_by_artist(&self, artist_id: &str)` (albums whose
    `artist_id` matches), `get_songs_from_album(&self, album_id: &str)`
    (songs whose `album_id` matches); unknown id → empty `Vec`. `play_track`
    is made `pub` so the UI can call it.
  - Derive `Clone` on `AppleMusicService` and `AppleMusicToken` so the UI
    can move a cloned service into a `'static` task future.
  - Make `init_service` a synchronous module-level free function (it never
    awaited anything) and add a `#[cfg(test)] mod tests` covering: sample
    library is non-empty; `get_albums_by_artist` returns only matching
    albums and is empty for an unknown id; `get_songs_from_album` likewise.
- `src/ui/mod.rs`:
  - Drop the `iced_native`, `executor`, and `Settings` imports and the
    `Application` trait impl; boot with
    `iced::application(boot, update, view).title("nostalgia").run()`, where
    `update` is synchronous and returns `Task<Message>` and `view` returns
    `Element<'_, Message>`.
  - Replace the three `Command::perform(...)` load sites (`LoadArtists`,
    `ArtistSelected`, `AlbumSelected`) with `Task::perform`, moving a
    `service.clone()` and the id `String`s into `async move { ... }` blocks
    so the future satisfies `Task::perform`'s `'static + Send` bound.
    `TrackSelected` does the same around `play_track`, emitting a no-op
    `TrackPlayed` message so the Now Playing bar re-renders.
  - Replace `Length::Units(20)` with `Length::Fixed(20.0)` and
    `Space::with_width(...)` with `Space::new().width(...)` (both renamed in
    iced 0.14; `Length::Pixels` does not exist).
  - `view` copies the current track and play label out of the shared state
    into owned values (the `Text` widgets own their strings, so no borrow of
    the temporary `MutexGuard` escapes `view`); drop the `VolumeChange`
    message (no widget sends it).
  - Keep the `Message` enum and the `view_artists` / `view_albums` /
    `view_songs` helpers, which live in `src/ui/views.rs` as free functions
    taking `&[Artist]` / `&[Album]` / `&[Song]`.
- `src/main.rs`: `main` is now a plain synchronous `fn main() -> iced::Result`
  (no `#[tokio::main]`): iced drives its event loop synchronously on the
  calling thread, and `update`/`view` use `blocking_lock` on the shared state,
  which panics inside a tokio runtime. `ui::init_ui(state)` is no longer
  awaited.
- `Cargo.toml`: drop the `iced_native` dependency; enable tokio `macros` and
  `rt` features for the `#[tokio::test]` runtime (only `sync` was enabled).

**Files touched.** `Cargo.toml`, `Cargo.lock`, `src/library.rs`,
`src/apple_music.rs`, `src/ui/mod.rs`, `src/ui/views.rs`, `src/main.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new sample-library tests (non-empty
  library; correct filtering by `artist_id` / `album_id`; empty result for
  unknown ids).
- `cargo fmt --check` passes.
- `cargo run` starts without panicking and, with a display, shows the sample
  artists; selecting an artist lists that artist's albums, and selecting an
  album lists its songs (manual check — the build + tests are the primary
  gate).

**Update (2026-10-06, organize).** `SampleLibrary` (with its `index_by`
lookup builder, the `SAMPLE_LIBRARY` cache, and the `sample_library()`
accessor) moved out of `src/apple_music.rs` into a new `src/sample_library.rs`
module, so the in-memory stub data is separate from the Apple Music service
seam — `src/apple_music.rs` now holds only the service. The plan's original
placement — the builder inside `src/apple_music.rs` — is superseded.
