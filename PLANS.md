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

### Ctrl+click Previous and Next to jump to the list's first and last song, like Ctrl+Z and Ctrl+B (planned 2026-10-27 by the plan role)

**Goal.** Winamp's Ctrl chords on the Previous and Next keys (Ctrl+Z, Ctrl+B) go to the start and
end of the list, and its transport buttons follow the same rule when Ctrl-clicked. Our keys
already do this: `shortcuts::shortcut` maps Ctrl+Z to `Message::FirstTrack` and Ctrl+B to
`Message::LastTrack`, handled by `handlers::playback::stepping::jump_to_edge_track`. The ◀◀ and
▶▶ buttons always step one song, though, because nothing in the UI knows a modifier is held:
`shortcuts::message_for` maps `keyboard::Event::ModifiersChanged` to `None`. Make a Ctrl-held
click on Previous play the first song and a Ctrl-held click on Next play the last song. This
applies in the main transport row, the shade strip, and the library footer.

**Approach.** Decided, so the implementer has no open questions:
- Track Control on the player. Add `pub(in crate::ui) control_held: bool` to `WinampPlayer`
  (src/ui/player.rs), `false` at construction, not saved in settings.
- Add a new always-on listener `modifiers_message(event, _status, _window)` in
  src/ui/subscriptions/listeners.rs, in the same shape as `mixer_key_message`. It maps
  `iced::Event::Keyboard(keyboard::Event::ModifiersChanged(m))` to
  `Message::ControlHeld(m.control())`, and `iced::Event::Window(iced::window::Event::Unfocused)`
  to `Message::ControlHeld(false)`, so a Ctrl released outside the window does not stick. Every
  other event maps to `None`. Re-export it from `subscriptions/mod.rs` next to the other
  listeners, and push it unconditionally in `subscriptions` with `iced::event::listen_with`.
  Leave `message_for` alone: its `ModifiersChanged → None` test stays valid.
- Add three variants to `Message` (src/ui/message.rs), each with a one-line comment like its
  neighbours: `ControlHeld(bool)`, `PreviousButton`, and `NextButton`. The update arms in
  src/ui/mod.rs: `ControlHeld(held)` sets `player.control_held` and returns `Task::none()`.
  `PreviousButton` / `NextButton` call new `stepping::previous_button(player)` /
  `stepping::next_button(player)` in src/ui/handlers/playback/stepping.rs.
  `previous_button` returns `jump_to_edge_track(player, false)` while `control_held`, otherwise
  `step_track(player, false)`. `next_button` does the same with `true`. A Ctrl-click therefore
  keeps every Ctrl+Z/Ctrl+B guarantee (it ignores Shuffle and Repeat, scrolls the landed row
  into view, and does nothing with an empty list).
- In `views::transport::transport_glyphs` (src/ui/views/transport.rs), the Previous and Next
  entries emit `Message::PreviousButton` and `Message::NextButton` in place of
  `PreviousTrack`/`NextTrack`. The shade strip and library footer pick this up through
  `views::mini_transport::mini_transport_controls`. Keys keep their messages: Z, B, keypad 4/6,
  and the media Previous/Next keys still send `PreviousTrack`/`NextTrack`, so a Ctrl-held media
  key does not turn into an edge jump. Update the doc comment on `transport_glyphs`.
- Only Control matters. Shift, Alt, and Logo clicks behave like plain clicks, and no other
  button changes.

**Files touched.** src/ui/player.rs, src/ui/message.rs, src/ui/mod.rs,
src/ui/subscriptions/{mod.rs,listeners.rs}, src/ui/handlers/playback/stepping.rs,
src/ui/views/transport.rs, src/ui/views/tests/transport.rs (the
`transport_glyphs_match_the_mockup_order_and_widths` message asserts), a new
`src/ui/tests/ctrl_click_tests.rs` registered in `src/ui/tests/mod.rs` (stepping_tests.rs is
already over 1000 lines), docs/usage.md, docs/invariants.md.

**Tests** (new file, using `player_stepping_from` and `assert_track_selected` from
`src/ui/tests/mod.rs`):
- `ctrl_click_previous_plays_the_first_song` and `ctrl_click_next_plays_the_last_song`: send
  `ControlHeld(true)` and then the button message, and assert the edge row is selected.
- `plain_button_clicks_step_one_song`: with `control_held` false, `PreviousButton` /
  `NextButton` select the same rows as `PreviousTrack` / `NextTrack`.
- `control_release_restores_plain_steps`: `ControlHeld(true)` then `ControlHeld(false)`, and the
  next `NextButton` steps one song.
- `modifiers_message_tracks_control_and_focus_loss`: `ModifiersChanged(Modifiers::CTRL)` gives
  `ControlHeld(true)`, `ModifiersChanged(Modifiers::SHIFT)` gives `ControlHeld(false)`, window
  `Unfocused` gives `ControlHeld(false)`, and a `KeyPressed` gives `None`.

**Docs.** docs/usage.md: after the sentence "Ctrl+Z plays the first song of the list Z and B step
through and Ctrl+B plays its last song.", add that a Ctrl-held click on the Previous or Next
button, in the transport row, the shade strip, or the library footer, does the same, while a
plain click steps one song. In docs/invariants.md `## List edge jumps`, extend the guarantee to
Ctrl-clicked Previous/Next, name `stepping::previous_button`/`next_button` and
`listeners::modifiers_message` under Enforced by, and add the new tests to Pinned by. In the
shade-mode section that lists the strip's button messages (`Message::PreviousTrack`, …), change
Previous/Next to `Message::PreviousButton`/`Message::NextButton`.

**Acceptance criteria.**
- With Ctrl held, clicking ◀◀ in any of the three transport rows plays the showing list's first
  song, and clicking ▶▶ plays its last song, whatever Shuffle and Repeat are set to.
- Without Ctrl, both buttons step exactly as today, including Previous's restart after three
  seconds and Shuffle's retrace.
- Losing window focus clears the held state.
- Every keyboard binding maps to the same message as before.
- The new tests pass, the docs describe the Ctrl-click, and `make check` passes.

### Right-click the player to open the options menu, like Winamp's main-window menu (planned 2026-10-27 by the plan role)

**Goal.** In Winamp 2.x, right-clicking anywhere on the main window (or its rolled-up shade
strip) pops up the main menu, and the playlist editor keeps its own right-click menu. Our closest
analog to that menu is the `OPTIONS` overlay, which today opens only from the LCD's `O`, Alt+O,
or the transport row's lightning button. A right-press on the player panels should toggle it
too. The library section stays out of this area.

**Approach.** The player panels already sit inside one outer `MouseArea` that carries the volume
wheel: `MouseArea::new(player_panels).on_scroll(views::volume_wheel_message)` in
`src/ui/view/normal.rs` `view_normal`, and the title bar plus shade strip
`MouseArea::new(...).on_scroll(views::volume_wheel_message)` in `src/ui/view/shade.rs`
`view_shade`. Add `.on_right_press(Message::ToggleOptions)` to both wrappers. No new message or
handler is needed: `handlers::window::toggle_options` already closes the other transient panels,
resets `options_selected`, and restores a shaded window before the menu draws. A second
right-press does not close the menu: while it is open, `view_options_overlay` maps the base to
`Message::Ignored`, and the backdrop (`modal_overlay`) reacts only to a left press. Close, a
left click on the backdrop, Escape, or Alt+O still close it.
iced 0.14's `MouseArea::update` (iced_widget-0.14.2 `src/mouse_area.rs`) lets the content handle
the event first and then publishes `on_right_press` for `ButtonPressed(Right)`. The nested
widgets (buttons, sliders, the preset pick list, and the inner `MouseArea`s for the LCD title
drag, elapsed readout, visualizer, `O A I D V` letters, title-bar drag region, and shade-strip
double-click) react only to the left button, so a right-press over any of them reaches the outer
wrapper. The library column is pushed outside that wrapper in `view_normal`, so a right-press over
the browse list or library footer publishes nothing new. Update the comment above the wrapper in
`view_normal` so it covers the right-press as well as the wheel.

**Files touched.**
- `src/ui/view/normal.rs`, `src/ui/view/shade.rs`: the two `.on_right_press` calls and comments.
- `src/ui/tests/mod.rs`: add a helper `root_right_press_messages(player, bounds) -> Vec<Message>`
  beside `root_wheel_messages`. It uses `layout_capture::publish_at` on `view(player)` with
  `ButtonPressed(Right)` then `ButtonReleased(Right)` at the bounds' center.
- `src/ui/tests/panels_tests.rs` (or `window_tests.rs`, whichever holds the options tests; keep
  each test file focused): add
  - `right_press_on_the_player_panels_toggles_options`: with `test_player()`, a right-press at
    `captured_root_view_container(&player, views::SEEK_STRIP_ID)` and at
    `views::BALANCE_SLIDER_ID` each publishes exactly one `Message::ToggleOptions`.
  - `right_press_on_the_library_does_not_toggle_options`: a right-press at
    `views::BROWSE_LIST_ID` and at the library footer's `views::MINI_VOLUME_ID`
    (`captured_root_view_volume_slider`) publishes no `ToggleOptions`.
  - `right_press_on_the_shade_strip_toggles_options`: with `player.shaded = true`, a right-press at
    the strip's `views::MINI_VOLUME_ID` bounds publishes one `ToggleOptions`. Confirm first that
    `mini_transport_strip_with_seek` tags the shade strip's volume slider with that id. If it
    does not, press at the centre of the laid-out root view instead, as
    `shade_strip_double_click_restores_the_window` does.
  - Also run `update` with the published message on the shaded player and assert that
    `options_visible` is set and `shaded` is cleared. The existing
    `toggle_options_while_shaded_restores_the_window_first` already pins that handler, so a
    published-message check is enough.
- `docs/usage.md`: in the options-menu paragraph ("The LCD's `O` letter, Alt+O, or the lightning
  button at the end of the transport row opens an `OPTIONS` menu …"), add a right-click anywhere on
  the player (title bar, LCD, seek strip, transport row, equalizer, or the rolled-up shade strip),
  but not on the library, like Winamp's main-window menu.
- `docs/invariants.md` `## Options panel`: add the right-press to the Guarantee's opening
  sentence and first bullet, add the two `on_right_press` wrappers in `src/ui/view/normal.rs` and
  `src/ui/view/shade.rs` to Enforced by, and add the three new tests to Pinned by. In
  `## Volume wheel`'s Enforced by, note that the same two wrappers also carry the options
  right-press.

**Acceptance criteria.**
- A right-press on the title bar, Now Playing LCD, seek strip, transport row, or equalizer
  section opens the options menu with its first row highlighted. While the menu is open, a
  right-press on the base does nothing.
- A right-press on the shade strip restores the window and opens the menu.
- A right-press on the browse list, search row, or library footer does not open it.
- Every left-click, wheel, and keyboard behavior is unchanged; the existing view and wheel tests
  still pass.
- The new tests pass, docs/usage.md and docs/invariants.md describe the right-press, and
  `make check` passes.

## Done
