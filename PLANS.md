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

### Show Jump to time and Jump to file in the options menu, like Winamp's Playback menu (planned 2026-10-27 by the plan role)

**Goal.** Winamp's main menu lists "Jump to time  Ctrl+J" and "Jump to file  J" under Playback.
Our options menu already carries Winamp menu items beyond Options proper (Stop after current came
from that same Playback menu), but our two jump panels open only from the keyboard. Add two rows,
`Jump to time` and `Jump to file`, after `Shuffle: On/Off` and before `About Nostalgia`, so a
mouse-only user can reach both panels.

**Approach.**
- `src/ui/message.rs`: add `OptionsChoice::JumpTime` and `OptionsChoice::JumpFile`, and
  `OptionsRow::JumpTime` / `OptionsRow::JumpFile` between `Shuffle` and `About` in the enum and in
  `OptionsRow::ALL` (its length goes from 10 to 12). `OptionsRow::message` maps them to
  `Message::OptionsChoice(OptionsChoice::JumpTime)` / `(OptionsChoice::JumpFile)`. Route them
  through `OptionsChoice`, not straight to `Message::ToggleJumpTime` / `ToggleJumpFile` the way
  About maps to `ToggleAbout`. The reason: `toggle_jump_time` returns early when
  `playback_duration` is `None`, and `toggle_jump_file` returns early when `song_list` is `None`.
  An early return skips `close_transient_panels`, so with a direct mapping an unavailable row would
  leave the menu open. `options_choice` clears `options_visible` first, so the menu always closes.
  Extend `options_rows_map_to_mouse_messages_in_draw_order` with the two new messages.
- `src/ui/handlers/window.rs` `options_choice`: add `OptionsChoice::JumpTime =>
  super::jump::toggle_jump_time(player)` and `OptionsChoice::JumpFile =>
  super::jump::toggle_jump_file(player)`, returning their tasks (the input focus, and the shade
  restore inside them). `options_choice` has already cleared `options_visible`, and opening the
  menu ran `close_transient_panels`, so neither jump flag is set and each toggle takes its opening
  branch. `handlers::jump` is a `pub(super)` sibling of `handlers::playback`, so `super::jump::…`
  resolves from `window.rs` the way `super::playback::…` already does. `options_activate` needs no
  change: it already forwards any `OptionsChoice` to `options_choice`.
- `src/ui/views/options.rs` `options_row_label`: `OptionsRow::JumpTime => "Jump to time"` and
  `OptionsRow::JumpFile => "Jump to file"`. They are fixed labels, like `About Nostalgia`, so no
  `OptionsState` field and no label helper. Update the module and `view_options_overlay` doc
  comments, which describe the menu as on/off choices plus About.
- Tests:
  - `src/ui/views/tests/chrome.rs`: add `Jump to time` and `Jump to file` to the fixed rows that
    `options_overlay_draws_each_row_with_its_state` chains with About and Close. In
    `options_overlay_publishes_each_choice_and_closes_from_close_and_backdrop`, add both labels
    and the arms that assert their `OptionsChoice`.
  - `src/ui/tests/panels_tests.rs`: `options_choice_applies_each_toggle_and_closes_the_panel`'s
    exhaustive `match` needs an `unreachable!` arm naming the new test below.
  - New test in `src/ui/tests/jump_tests.rs`, `options_jump_rows_open_their_panels_or_just_close`.
    For each row, by mouse (`Message::OptionsChoice(..)`) and by keyboard (`options_selected` set
    to the row's position in `OptionsRow::ALL`, then `Message::OptionsActivate`), on a player
    with `options_visible = true`:
    - With the panel available (`playback_duration = Some(..)` for time; `SongsLoaded` plus
      `current_view = CurrentView::Songs` for file, as `jump_file_tests::player_in_songs` does),
      the menu closes, that panel's flag is set, and the other jump flag stays clear.
    - With it unavailable (default `test_player()`), the menu closes and neither jump flag is set.
- Docs:
  - `docs/usage.md`: in the options-menu sentence, list `Jump to time` and `Jump to file` before
    `About Nostalgia`, and say they open the Ctrl+J and J panels (doing nothing beyond closing
    the menu while that panel is unavailable).
  - `docs/invariants.md` `## Options panel`: add the two rows to the row list, add both dispatch
    targets to the `options_choice` bullet, and change "listing the app's existing on/off choices
    plus About and Close" to mention the two jump rows. The "eight label helpers" count stays
    correct because the new rows use none. Add the new test to **Pinned by**.

**Files touched.** `src/ui/message.rs`, `src/ui/handlers/window.rs`, `src/ui/views/options.rs`,
`src/ui/views/tests/chrome.rs`, `src/ui/tests/panels_tests.rs`, `src/ui/tests/jump_tests.rs`,
`docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- The options overlay draws `Jump to time` and `Jump to file` between the Shuffle row and
  `About Nostalgia`, and clicking either publishes its `OptionsChoice`.
- Choosing `Jump to time` with a preview of known length loaded closes the menu and opens the
  Jump to time panel. Choosing `Jump to file` on a showing song list closes the menu and opens
  Jump to File. Mouse and Enter behave the same.
- Choosing either row while its panel is unavailable closes the menu and opens nothing.
- The existing rows keep their order, labels, and behavior. Up/Down still clamp to the first row
  and the new last row (`Close`).
- `docs/usage.md` and `docs/invariants.md` describe the new rows, and `make check` passes.

## Done
