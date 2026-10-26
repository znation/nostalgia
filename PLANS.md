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

_None yet._


### Keep the row Alt+Up/Alt+Down moves in view, like Winamp's playlist editor (planned 2026-10-25 by the plan role)

**Goal.** Alt+Up and Alt+Down move the selected browse row and keep it selected. They do not
scroll, though. Repeated presses can carry the row past the viewport's top or bottom edge, and it
keeps moving out of sight. In Winamp's playlist editor the list follows the moved entry. After a
move, scroll the browse list only as far as needed to show the moved row fully. Leave the list
alone when the row is already visible. A full jump to the row would jerk the list on every press,
so do not use one.

**Approach.**
- `src/ui/handlers/browse/scroll.rs`:
  - Add a pure `pub(in crate::ui) fn reveal_row_offset(index: usize, scroll_top: f32,
    viewport_height: f32) -> Option<f32>`. It uses the same row geometry as
    `scroll_browse_to_row`: `row_top = views::BROWSE_LIST_PADDING + index as f32 *
    views::BROWSE_ROW_HEIGHT` and `row_bottom = row_top + views::BROWSE_ROW_HEIGHT`. It returns
    `Some(row_top)` when `row_top < scroll_top`. Otherwise it returns
    `Some(row_bottom - viewport_height)` when `row_bottom > scroll_top + viewport_height`.
    Otherwise it returns `None`. Check the top edge first, so a viewport shorter than one row
    shows the row's top.
  - Add `pub(in crate::ui) fn reveal_browse_row(index: usize) -> Task<Message>`. Build it with
    `iced::advanced::widget::operate` (the `advanced` feature is already on) around a small
    private struct that implements `iced::advanced::widget::Operation<T>` for every `T`, following
    iced's own `scroll_to` operation. In that impl, `traverse` calls `operate(self)`. `scrollable`
    acts only when `id == Some(&Id::new(views::BROWSE_LIST_ID))`. Then it calls
    `reveal_row_offset(index, translation.y, bounds.height)`, and on `Some(y)` it calls
    `state.scroll_to(AbsoluteOffset { x: None, y: Some(y) })`. iced passes the scrollable's
    current offset as `translation` and its viewport as `bounds`. In shade mode no browse list is
    built, so the operation finds no target and does nothing. Update the module doc comment to
    mention the reveal.
- `src/ui/handlers/browse.rs` `move_selected_row`: read the showing list's `selected()` before
  and after its `move_selected(down)` call. A small generic helper over `BrowseList<T: Clone>`
  that returns `Option<usize>` (the new index only when it changed) keeps the six-arm `match`
  flat. When the index changed, return `scroll::reveal_browse_row(new_index)`. Otherwise return
  `Task::none()`, so a no-op press (no selection, at an edge, or a loading or failed list) still
  schedules nothing. Update the doc comment.
- `src/ui/tests/mod.rs`: give `browse_scroll_effects_by` `bounds: iced::Rectangle` and
  `translation: iced::Vector` parameters in place of the defaults it passes to
  `operation.scrollable`. Add an async `browse_scroll_effects_at(task, what, viewport_height,
  scroll_top)` that drives a task the way `browse_scroll_effects` does, but passes a viewport of
  that height and `Vector::new(0.0, scroll_top)`. Existing callers keep passing the defaults.
- `docs/invariants.md` `## Browse row order`: extend the Alt+Up/Alt+Down paragraph. A move that
  changes the selection scrolls the list only far enough to show the moved row, and a no-op
  move schedules no task. Add the new function and tests to its Enforced/Tested lines. Also
  update the README sentence on Alt+Up and Alt+Down.

**Acceptance criteria.**
- Inline unit tests in `scroll.rs` pin `reveal_row_offset`. A row above the viewport gives its
  top. A row below gives `row_bottom - viewport_height`. A fully visible row gives `None`, and
  so does a row whose bottom edge exactly meets the viewport's bottom.
- `src/ui/tests/move_row_tests.rs` adds these tests:
  - `a_move_past_the_viewport_bottom_scrolls_the_row_into_view`. Load 20 artists, select row 10,
    and send `MoveSelectedRowDown`. Driven through `browse_scroll_effects_at` with a
    5-row-high viewport at offset 0, the task scrolls to `y = Some(BROWSE_LIST_PADDING + 12.0 *
    BROWSE_ROW_HEIGHT - 5.0 * BROWSE_ROW_HEIGHT)`, with `x: None`.
  - `a_move_within_the_viewport_does_not_scroll`. Make the same move with a viewport tall enough
    to show every row. The captured effects are empty.
  - `a_move_above_the_viewport_top_scrolls_up_to_the_row`. Select row 10, send
    `MoveSelectedRowUp`, and use an offset past row 9. The task scrolls to row 9's top.
  - `a_no_op_move_schedules_no_task`. With no selection, `assert_no_task` holds for the
    returned task.
- The existing move-row, scroll, and shortcut tests pass unchanged except for the
  `browse_scroll_effects_by` signature. `make check` passes.

### Alt+E and Alt+G toggle the library and equalizer panels, like Winamp (planned 2026-10-25 by the plan role)

**Goal.** In Winamp 2.x, Alt+E shows or hides the playlist editor and Alt+G shows or hides the
graphical equalizer. Nostalgia's playlist-editor analogue is the library section behind the `PL`
button, and no key reaches it. Its equalizer panel answers only to plain `E` and the `EQ` button.
Bind Alt+E to `Message::ToggleLibraryPanel` and Alt+G to `Message::ToggleEqualizerPanel`, so
both windows toggle from the keyboard the Winamp way. Both remain layout-only toggles. In shade
mode both keys do nothing, as plain `E` does today.

**Approach.**

- `src/ui/shortcuts.rs`: add `fn panel_shortcut(physical_key: &key::Physical, modifiers:
  Modifiers) -> Option<Message>`, written like `file_info_shortcut`. It requires Alt held with
  Control, Shift, and Logo released. It maps `key::Physical::Code(key::Code::KeyE)` to
  `Message::ToggleLibraryPanel` and `key::Code::KeyG` to `Message::ToggleEqualizerPanel`. Match
  by physical key, not logical character, because on macOS Alt+E and Alt+G produce other
  logical characters (a dead key, `©`). The doc comment should say so. Chain it in
  `message_for` right after `file_info_shortcut`. Add Alt+E and Alt+G to `message_for`'s doc
  comment list. `shortcut` still returns `None` for every Alt chord, so
  `shortcut_ignores_chord_modifiers` stays true unchanged.
- `src/ui/handlers/mixer.rs`: `toggle_library_panel` should return `Task::none()` without
  flipping the flag while `player.shaded`, the same guard `toggle_equalizer_panel` has. Shade
  mode draws no `PL` button (only the main transport row in `views/transport.rs` pushes
  `library_panel_toggle()`), so the guard changes only the new key path. Update the doc comment.
- `docs/invariants.md`: in the EQ toggle bullet, add that Alt+G dispatches the same
  `Message::ToggleEqualizerPanel`. In the `PL` toggle bullet, add that Alt+E dispatches
  `Message::ToggleLibraryPanel`, that its other modifier chords stay unbound, and that shade mode
  changes nothing because the toggle leaves `library_panel_visible` alone while
  `WinampPlayer::shaded` holds. Add the new tests to that section's test list.
- `README.md` (Usage keys paragraph, outside the brief markers): after the sentence on E and
  the `EQ` button, add "Alt+E shows or hides the library section exactly like the `PL` button,
  and Alt+G the equalizer panel like `EQ`, as Winamp's Alt+E and Alt+G toggle its playlist
  editor and equalizer."

**Files touched.** `src/ui/shortcuts.rs` (function + inline tests), `src/ui/handlers/mixer.rs`,
`src/ui/tests/shortcuts_tests.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**

- Inline test `message_for_maps_alt_e_and_alt_g_to_the_panel_toggles` in `shortcuts.rs`
  (built with the `digit_pressed` helper or a sibling taking a `key::Code`). With
  `Modifiers::ALT`, physical `KeyE` maps to `ToggleLibraryPanel` and `KeyG` to
  `ToggleEqualizerPanel`. This must hold both when the logical key is `"e"`/`"g"` and when it
  is a different character (e.g. `"´"`, `"©"`). With `CTRL|ALT`, `SHIFT|ALT`, and `LOGO|ALT`,
  neither maps. Plain `E` still maps to `ToggleEqualizerPanel`, and plain `G` maps to nothing.
- `src/ui/tests/shortcuts_tests.rs`:
  - `alt_e_toggles_the_library_panel_and_back`. Map the Alt+E event through
    `shortcuts::message_for` and send it with `update`. `library_panel_visible` flips to false,
    and a second press sets it back to true. `assert_message_schedules_no_work` holds.
  - `alt_e_in_shade_mode_leaves_the_library_panel_alone`. With `player.shaded = true`, the
    mapped message schedules no work and `library_panel_visible` stays true. This mirrors
    `e_in_shade_mode_leaves_the_equalizer_panel_alone`.
  - `alt_g_toggles_the_equalizer_panel`. The mapped Alt+G message flips `eq_panel_visible`.
- `toggle_library_panel_hides_and_restores_the_library_section`, the settings persistence
  tests for the library flag, and every other existing test pass unchanged. `make check` passes.

## Done
