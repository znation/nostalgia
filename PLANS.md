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

### Alt+Up and Alt+Down move the selected browse row, like Winamp's playlist editor (planned 2026-10-25 by the plan role)

**Goal.** In Winamp's playlist editor, Alt+Up and Alt+Down move the selected entry one place up or
down. Nostalgia's browse list can be sorted, reversed, and shuffled one-shot (Sort, Ctrl+R,
Ctrl+Shift+R), but no single row can be moved. Bind Alt+Up and Alt+Down to move the selected row of
the showing list one place up or down. Make the move a one-shot reorder like the others: Back to
the parent list (or reopening the list) restores library order, and the selection stays on the
moved row so repeated presses keep moving it.

**Approach.**
- `src/ui/browse.rs`: add `BrowseList::move_selected(&mut self, down: bool) where T: Clone`. It
  is a no-op when nothing is selected, the list is loading or failed, or the selected row is
  already at the edge it would move past (row 0 for up, the last row for down). Otherwise it builds
  the identity permutation of `0..len` with the selected index `i` and its neighbour `j`
  (`i - 1` or `i + 1`) swapped. It passes that permutation to the existing `apply_reorder` (so
  cached `titles`/`labels` follow the rows, `library_positions` composes, and the epoch is bumped),
  then sets `self.selected = Some(j)`, because `apply_reorder` drops the selection. Document that
  it composes with `sort_rows_by` / `reverse_rows` / `shuffle_rows` and that
  `restore_library_order` undoes it. Add `move_selected` to `restore_library_order`'s doc list.
- `src/ui/message.rs`: add `MoveSelectedRowUp` and `MoveSelectedRowDown` beside `RandomizeBrowse`,
  each with a comment in the style of its neighbours.
- `src/ui/handlers/browse.rs`: add `move_selected_row(player, down: bool) -> Task<Message>`,
  matching `player.current_view` onto the six lists exactly like `reverse_browse`, and returning
  `Task::none()`. Route both messages in the `update` dispatch in `src/ui/mod.rs`
  (`handlers::browse::move_selected_row(player, false/true)`).
- `src/ui/shortcuts.rs`: add `move_row_shortcut(key: &Key, modifiers: Modifiers) -> Option<Message>`,
  chained in `message_for` before `numpad_shortcut`. It maps logical
  `Key::Named(key::Named::ArrowUp)` to `MoveSelectedRowUp` and `ArrowDown` to `MoveSelectedRowDown`
  only when Alt is held and Control, Shift, and Logo are released. It does not check the key's
  location. Plain arrows keep their volume bindings, because `shortcut` already returns `None` for
  any Alt chord. Mention the new keys in `message_for`'s doc comment.
- Do not auto-scroll the list to keep the moved row in view (out of scope; the epoch bump redraws
  it in place).

**Files touched.** `src/ui/browse.rs`, `src/ui/message.rs`, `src/ui/mod.rs` (dispatch and the
new test module's `mod` line), `src/ui/handlers/browse.rs`, `src/ui/shortcuts.rs`, new
`src/ui/tests/move_row_tests.rs`, `docs/invariants.md` (`## Browse row order`: a paragraph for the
move plus its enforcing symbols and pinning tests), and `README.md` (one sentence after the
Ctrl+Shift+R sentence in Usage, outside the tumwater:prompt markers).

**Acceptance criteria.**
- Inline tests in `src/ui/browse.rs`: `move_selected` swaps the selected row with its neighbour
  in both directions, and the selection follows it to the new index. It is a no-op (same rows,
  same epoch, same selection) with no selection, at the top for up, at the bottom for down, and on
  a loading list. After any move, `restore_library_order` returns the stored order.
- `src/ui/tests/move_row_tests.rs`, through `update`:
  - With `Message::SongsLoaded(stepping_songs())` showing under `CurrentView::Songs` and a
    row selected via `Message::SelectRow`, `MoveSelectedRowDown` swaps that song with the next.
    The `titles` and `labels` move with their songs, `selected()` names the new index, and the
    epoch is bumped, so a `SelectRow` carrying the old epoch is rejected.
  - Two `MoveSelectedRowUp` presses carry an artists-list row up two places.
  - Back from Albums restores a moved artists list to library order.
  - A move leaves the current track, the transport state, `current_view`, and the other lists
    unchanged.
- Inline tests in `src/ui/shortcuts.rs`: `message_for` maps Alt+ArrowUp and Alt+ArrowDown to the
  two messages. Alt with Control, Shift, or Logo also held maps to neither. Plain ArrowUp and
  ArrowDown still map to `VolumeUp` and `VolumeDown`.
- `docs/invariants.md` and `README.md` describe the keys as above, and `make check` passes.

### Ctrl+J opens a Jump to time panel that seeks the loaded preview, like Winamp (planned 2026-10-25 by the plan role)

**Goal.** In Winamp, Ctrl+J opens a "Jump to time" dialog: you type a time and the playing track
seeks to it. Nostalgia can seek only by dragging the slider, or with the arrows, the keypad, or the
wheel, five seconds at a time. Bind Ctrl+J to flip a transient, unpersisted Jump to time panel. It
has a time box. Enter or a `Jump` button seeks the loaded preview to the typed time and closes the
panel. Like About and File info, it is a modal overlay closed by `Close` or by a backdrop click.

**Approach.**
- `src/ui/handlers/jump.rs` (new; register `pub(super) mod jump;` in `src/ui/handlers/mod.rs`):
  - `pub(in crate::ui) fn parse_jump_time(text: &str) -> Option<Duration>`. It trims `text`, then
    accepts either plain ASCII-digit seconds (`"95"` → 95 s) or `M:SS`: one or more minute digits,
    a colon, and one or two second digits whose value is below 60 (`"1:35"` and `"01:05"` parse,
    `"1:5"` → 65 s). Anything else gives `None`: an empty string, `"1:60"`, `"1:"`, `":30"`,
    `"1:2:3"`, a sign, or a letter. Use the standard library only. A value that overflows `u64`
    gives `None`.
  - `pub(in crate::ui) fn toggle_jump_time(player) -> Task<Message>`. Opening needs
    `player.playback_duration` to be `Some`, that is, a loaded preview with a known length.
    Without one it returns `Task::none()` and leaves the flag false. Opening sets
    `jump_time_visible = true`, clears `jump_time_query`, and clears `about_visible` and
    `file_info_visible`. It returns `iced::widget::operation::focus(views::JUMP_TIME_INPUT_ID)`.
    When opening while `player.shaded`, it returns
    `Task::batch([toggle_window_shade(player), focus])` instead (`toggle_window_shade` is in
    `handlers::window`). Closing only clears the flag and returns `Task::none()`.
  - `jump_time_changed(player, text: String)` stores `text` in `jump_time_query`.
  - `jump_time_submitted(player)`: when the panel is open, the query parses, and
    `playback_duration` is still `Some`, clamp the parsed time to that duration. Then seek through
    the helper described next, and close the panel. An unparsable query keeps the panel open with
    the query unchanged, and seeks nothing.
- `src/ui/handlers/playback/preview.rs`: move the body of `seek_preview_released` after its
  `take()` into `pub(in crate::ui) fn seek_preview_to(player, position: Duration)`. The body is
  the `seek_output_to` call, the `position_epoch` bump, the optimistic `playback_position`, and
  the logged error. `seek_preview_released` then calls the helper, so its tests pass unedited.
  `jump_time_submitted` calls the same helper.
- `src/ui/player.rs`: next to `file_info_visible`, add `jump_time_visible: bool` and
  `jump_time_query: String` (in the same doc style: closed at launch, not persisted), initialized
  to `false` and empty. `src/ui/handlers/window.rs`: opening About in `toggle_about`, and opening
  File info in `toggle_file_info`, also set `jump_time_visible = false`. At most one panel shows.
- `src/ui/message.rs`: add `ToggleJumpTime`, `JumpTimeChanged(String)`, and `JumpTimeSubmitted`
  beside `ToggleFileInfo`, with a comment in the same style. Route them in `src/ui/mod.rs`
  `update` to the three `handlers::jump` functions.
- `src/ui/views/jump_time.rs` (new; `mod jump_time;` and its exports in `src/ui/views/mod.rs`):
  `pub const JUMP_TIME_INPUT_ID: &str = "nostalgia-jump-time";` and
  `pub fn view_jump_time_overlay<'a>(base, query: &'a str, length: Duration) -> Element<'a, Message>`.
  The panel shows a `JUMP TO TIME` heading (size 18, `theme::TITLE_TAPE`, like `FILE INFO`) and a
  `Track length: <m:ss>` line formatted with `views::format_track_time`. Below them is a
  `TextInput::new("mm:ss", query)` with `.id(JUMP_TIME_INPUT_ID)`,
  `.on_input(Message::JumpTimeChanged)`, `.on_submit(Message::JumpTimeSubmitted)`, and the
  search box's `style::chrome_text_input_style`. Last comes a `Row` holding
  `labeled_button("Jump", Message::JumpTimeSubmitted)` and
  `labeled_button("Close", Message::ToggleJumpTime)`. Wrap the panel with
  `modal::modal_overlay(.., Message::ToggleJumpTime)`.
- `src/ui/view.rs` `view`: add a branch after File info. When `player.jump_time_visible &&
  !player.shaded` and `player.playback_duration` is `Some(length)`, wrap `base` in
  `views::view_jump_time_overlay(base, &player.jump_time_query, length)`. Otherwise draw `base`.
- `src/ui/shortcuts.rs` `shortcut`: in the Control block (Control held, Alt and Logo released),
  map `j`, case-insensitively and only with Shift released, to `Message::ToggleJumpTime`. Update
  the block's comment and `message_for`'s doc comment.

**Files touched.** `src/ui/handlers/jump.rs` (new), `src/ui/handlers/mod.rs`,
`src/ui/handlers/playback/preview.rs`, `src/ui/handlers/window.rs`, `src/ui/views/jump_time.rs`
(new), `src/ui/views/mod.rs`, `src/ui/view.rs`, `src/ui/message.rs`, `src/ui/mod.rs`,
`src/ui/player.rs`, and `src/ui/shortcuts.rs`. Tests go in a new `src/ui/tests/jump_tests.rs`
(add it to the mod list in `src/ui/tests/mod.rs`), `src/ui/views/tests/chrome.rs`, and inline
in `jump.rs` and `shortcuts.rs`. Also `README.md` (the keys paragraph) and `docs/invariants.md`
(a new `## Jump to time panel` section after `## File info panel`).

**Acceptance criteria.**
- Inline `parse_jump_time` tests pin `"95"` → 95 s, `" 1:35 "` → 95 s, `"01:05"` → 65 s, and
  `"0:00"` → 0 s. They also pin `None` for `""`, `"1:60"`, `"1:"`, `":30"`, `"1:2:3"`, `"-5"`,
  `"1m"`, and a value too large for `u64`.
- Inline shortcut tests: Control with `j`, and Control with the character `J` (Shift released),
  map to `Message::ToggleJumpTime`. Ctrl+Shift+J, Ctrl+Alt+J, Ctrl+Logo+J, and a plain `j` map to nothing.
- `jump_tests.rs`, with `playback_duration` set to `Some(30 s)` on `test_player()`:
  - `ToggleJumpTime` opens the panel with an empty query and schedules work (the focus task).
    A second `ToggleJumpTime` closes it.
  - With `playback_duration` `None`, `ToggleJumpTime` leaves `jump_time_visible` false.
  - With the query `"0:12"`, `JumpTimeSubmitted` sets `playback_position` to `Some(12 s)`, bumps
    `position_epoch`, and closes the panel. A query of `"5:00"` clamps to `Some(30 s)`.
  - With the query `"abc"`, `JumpTimeSubmitted` keeps the panel open, keeps the query, and leaves
    `playback_position` and `position_epoch` unchanged.
  - Opening Jump to time closes About and File info, and opening either of them closes Jump to
    time.
  - Opening while shaded sets the flag and leaves the window restoring, the same way
    `toggle_file_info_while_shaded_restores_the_window_first` checks it.
- The existing `seek_preview_released` tests pass unedited.
- A chrome test builds `view_jump_time_overlay` over a blank base and captures its text. It
  finds `JUMP TO TIME`, `Track length: 0:30`, `Jump`, and `Close`. `Close` and a backdrop press
  each publish one `ToggleJumpTime`, and `Jump` publishes `JumpTimeSubmitted`.
- `view_constructs_when_the_window_is_shaded` (or a sibling) shows that a set
  `jump_time_visible` flag draws no `JUMP TO TIME` text in shade mode.
- README and `docs/invariants.md` describe the key and the panel. `make check` passes.

## Done
