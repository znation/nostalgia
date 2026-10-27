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

### Show Repeat and Shuffle in the options menu, like Winamp's Options menu (planned 2026-10-27 by the plan role)

**Goal.** Winamp's Options menu lists Repeat and Shuffle as checked items. Our options menu lists
`Stop after current` but neither of these. Add `Repeat: On/Off` and `Shuffle: On/Off` rows after
`Stop after current` and before `About Nostalgia`. Each row toggles its flag and closes the menu,
like the other rows.

**Approach.**

- `src/ui/message.rs`: add `OptionsChoice::Repeat` (doc: reuses `Message::ToggleRepeat`) and
  `OptionsChoice::Shuffle` (doc: reuses `Message::ToggleShuffle`), plus `OptionsRow::Repeat` and
  `OptionsRow::Shuffle`. Place them in `OptionsRow::ALL` after `StopAfterCurrent`, so the array
  grows from 8 to 10, and map them in `OptionsRow::message`. Update
  `options_rows_map_to_mouse_messages_in_draw_order`.
- `src/ui/handlers/window.rs`, `options_choice`: send the new choices to
  `super::playback::toggle_repeat` and `super::playback::toggle_shuffle`. Both already exist and
  flip `AppState::repeat` / `AppState::shuffle` through `mutate_state`, so persistence and the
  transport buttons follow without further changes. Keyboard selection already clamps against
  `OptionsRow::ALL.len()`.
- `src/ui/views/options.rs`: add `repeat_label(bool)` (`"Repeat: On"` / `"Repeat: Off"`) and
  `shuffle_label(bool)` (`"Shuffle: On"` / `"Shuffle: Off"`). Add documented `repeat` and
  `shuffle` bool fields to `OptionsState`, and arms for both rows in `options_row_label`.
- `src/ui/view/overlay.rs`, `apply_overlays`: in the options branch, read `repeat` and `shuffle`
  from `player.state.blocking_lock()` into locals before building `views::OptionsState`. This is
  the same read `src/ui/view/player_panels.rs` does for the transport buttons. Drop the guard
  before calling `view_options_overlay`.

**Tests.**

- `src/ui/views/options.rs`: `repeat_and_shuffle_labels_mirror_their_flags`.
- `src/ui/views/tests/chrome.rs`: `options_overlay_element` sets `repeat` and `shuffle` to
  `all_on`. `options_overlay_draws_each_row_with_its_state` expects `Repeat: Off/On` and
  `Shuffle: Off/On` after the stop-after-current label in each case.
  `options_overlay_publishes_each_choice_and_closes_from_close_and_backdrop` presses both new
  labels.
- `src/ui/tests/panels_tests.rs`:
  - Add `options_repeat_and_shuffle_toggle_shared_state_and_close_the_panel`. Choosing each one
    by message flips only that `AppState` flag and closes the menu. Activating it by keyboard,
    with `options_selected` set to its position in `OptionsRow::ALL`, flips it back.
  - `options_choice_applies_each_toggle_and_closes_the_panel` matches exhaustively on
    `OptionsChoice`. Give the new variants `unreachable!` arms that name the new test, as
    `StopAfterCurrent` already does.
  - `options_keyboard_selection_clamps_and_activates_each_row` hardcodes the last index (7) and
    the loop bound (`1..8`). Change them to 9 and `1..10`. Find About's index (now 8) with
    `OptionsRow::ALL.iter().position(...)` instead of the literal 6.

**Docs.** In `docs/usage.md`, the options-menu sentence lists `Repeat: On/Off` and
`Shuffle: On/Off` between `Stop after current: On/Off` and `About Nostalgia`. In
`docs/invariants.md`, under `## Options panel`, the rows list, the `options_choice` dispatch
bullet (adding `toggle_repeat` and `toggle_shuffle`), the overlay bullet (repeat and shuffle read
from the shared state), the label-helper count (six becomes eight), and the Pinned-by list all
cover the new rows and tests.

**Acceptance criteria.**

- The options menu shows `Repeat: On`/`Off` and `Shuffle: On`/`Off` matching the shared flags,
  between the Stop after current and About rows. Clicking either row, or pressing Enter on it,
  toggles that flag, closes the menu, and updates the transport row's Repeat/Shuffle button.
- Every other row's label, relative order, and message are unchanged, and `make check` passes.

### Drag the scrolling LCD title to scroll it by hand, like Winamp (planned 2026-10-27 by the plan role)

**Goal.** In Winamp 2.x you can press on the scrolling song title in the main window and drag it
left or right to scroll it by hand. Automatic scrolling pauses during the drag and resumes from
the dragged position on release. Our Now Playing marquee only scrolls on its own. Add the drag.
It applies only while the line is too long for the LCD, which is when `push_title` windows it
through `marquee_text`. A title that fits stays a plain, unclickable label.

**Approach.**

- `src/ui/views/now_playing/text.rs`: add `TITLE_DRAG_PX_PER_CHAR: f32 = 10.0`. The LCD text is
  size 20, so this is roughly one glyph. Add `marquee_cycle_len(label, artist, number) -> usize`,
  which is `display_len(..) + MARQUEE_GAP` (the period `marquee_text` wraps on). Add
  `dragged_marquee_offset(start: usize, dx: f32, cycle: usize) -> usize`. It computes
  `steps = (-dx / TITLE_DRAG_PX_PER_CHAR).trunc()` and returns
  `((start % cycle) as i64 + steps as i64).rem_euclid(cycle as i64) as usize`. Dragging left
  (negative `dx`) reveals later characters, so the text follows the pointer. Re-export both
  through `views` the way `needs_marquee` is re-exported.
- `src/ui/message.rs`: add `TitleDragPressed`, `TitleDragMoved(f32)` (window-space cursor x) and
  `TitleDragReleased`, each with a doc comment.
- `src/ui/views/now_playing.rs`, `push_title`: in the overlong branch, wrap the `marquee_text`
  label in a `MouseArea` with `.on_press(Message::TitleDragPressed)` and
  `.interaction(Interaction::Grab)`. The fitting branch is unchanged.
- `src/ui/player.rs`: add a documented field `title_drag: Option<TitleDrag>`, initialised to
  `None`. `TitleDrag { start_offset: usize, anchor_x: Option<f32> }` is defined in the new
  handler module. `needs_marquee` also returns `false` while `title_drag.is_some()`, alongside
  the `mixer_drag` and `seek_preview` checks.
- New `src/ui/handlers/playback/title_drag.rs` (declared from `playback.rs`), with a module doc:
  - `title_drag_pressed` stores `TitleDrag { start_offset: player.marquee_offset, anchor_x: None }`.
  - `title_drag_moved(x)` does nothing without a drag. On the first move it records `anchor_x`.
    On later moves it sets `marquee_offset` to `dragged_marquee_offset(start_offset, x - anchor,
    cycle)`. `cycle` comes from `views::marquee_cycle_len` over the label, artist and number,
    resolved exactly as `needs_marquee` resolves them. A `cycle` of 0 cannot happen because the
    gap is nonzero.
  - `title_drag_released` clears the drag.
  - Route the three messages in `src/ui/mod.rs`'s `update` dispatch.
- `src/ui/handlers/playback.rs`: where a new selection sets `player.marquee_offset = 0`, also set
  `player.title_drag = None`, so a drag cannot carry an old song's offset into a new one.
- `src/ui/subscriptions/listeners.rs`: add `title_drag_message(event, _status, _window)`.
  - `mouse::Event::CursorMoved { position }` maps to `TitleDragMoved(position.x)`.
  - A left-button `ButtonReleased` and `mouse::Event::CursorLeft` map to `TitleDragReleased`.
  - Every other event maps to nothing. Status is ignored, because the drag must follow the
    pointer off the title.
- `src/ui/subscriptions/mod.rs`, `subscriptions`: add a trailing `title_drag_active: bool`
  parameter. When it is true, push `iced::event::listen_with(title_drag_message)`.
  `src/ui/mod.rs` passes `player.title_drag.is_some()`. Update every `subscriptions(...)` call in
  `src/ui/tests/subscriptions_tests.rs` with a trailing `false`.

**Tests.**

- `text.rs` unit tests:
  - `dragged_marquee_offset_follows_the_pointer_and_wraps`: dragging left 25 px from 0 gives 2.
    Dragging right 10 px from 0 wraps to `cycle - 1`. A 9 px move gives the start unchanged.
  - `marquee_cycle_len_is_the_line_plus_its_gap`.
- `src/ui/views/tests/now_playing.rs`, `the_overlong_title_publishes_a_drag_press_and_a_fitting_one_does_not`:
  a `publish_at` left press on the title text publishes `TitleDragPressed` for a line over
  `MARQUEE_WIDTH`, and publishes nothing for a short line.
- `src/ui/tests/mixer_tests.rs` (beside `the_marquee_pauses_during_a_mixer_drag`), or a new
  `src/ui/tests/title_drag_tests.rs` if that file is crowded. Use `seed_current_title` with an
  overlong title. Add `the_marquee_pauses_while_the_title_is_dragged_and_resumes_from_there`:
  - Pressed: `needs_marquee()` is false.
  - `TitleDragMoved(100.0)` then `TitleDragMoved(70.0)`: the offset advances by 3.
  - `TitleDragReleased`: `needs_marquee()` is true and the offset stays at 3.
  - A `TitleDragMoved` without a press changes nothing.
  - A new selection mid-drag clears `title_drag`.
- `subscriptions_tests.rs`: the listener count grows by one only with `title_drag_active`.
  `title_drag_message` maps a cursor move, a left release and `CursorLeft` as above, and ignores
  a key press and a right-button release.

**Docs.**

- `docs/usage.md`: after "scrolled across the display when the line is longer than it", say that
  pressing on the scrolling title and dragging left or right scrolls it by hand (one character
  per 10 pixels). Automatic scrolling pauses until release and resumes from there, like Winamp.
- `docs/invariants.md`: in `## Now Playing label`, next to the marquee-timer bullets, add that
  `needs_marquee` is false during a title drag, and that `title_drag_message` is subscribed only
  while one is active. Add the new tests to that section's pinned-by list.

**Acceptance criteria.**

- With a title too long for the LCD, pressing on it and dragging moves the shown window one
  character per 10 px, with the text following the pointer and wrapping through the gap. The
  automatic scroll is paused for the whole drag, and on release it resumes from the dragged
  offset. Releasing off the title, or the cursor leaving the window, also ends the drag.
- A title that fits the LCD is not a drag target. The elapsed-readout click, the mixer and seek
  readouts, and marquee behaviour outside a drag are unchanged. `make check` passes.

## Done
