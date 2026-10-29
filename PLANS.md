# Plans

## Planned

_None yet._

### Show Start of list and End of list in the options menu, like Winamp's Playback menu (planned 2026-10-29 by the plan role)

**Goal.** Winamp's Playback menu lists `Start of list` and `End of list`. Our first/last-track
jumps (`Message::FirstTrack` / `Message::LastTrack`, bound to Ctrl+Z / Ctrl+B in
`src/ui/shortcuts.rs` and dispatched in `src/ui/mod.rs` to
`handlers::playback::stepping::jump_to_edge_track`) are reachable only from those keys, so
there is no mouse-only way to jump the showing list to its first or last song. Add two fixed
options-menu rows that run the same jumps. This plan stacks at the same spot as the planned
`Show Back 5 seconds and Fwd 5 seconds` entry above: its rows go directly **before**
`JumpTime`, after that plan's two seek rows if they have landed, so either land order works —
whichever plan lands second only updates the `OptionsRow::ALL` length, its array declaration,
the `options_rows_map_to_mouse_messages_in_draw_order` mapping, and the row lists.

**Approach.**
- `src/ui/message.rs`: add `OptionsChoice::StartOfList` and `OptionsChoice::EndOfList` (doc
  comments: "Reuses [`Message::FirstTrack`]." / "Reuses [`Message::LastTrack`].") and
  `OptionsRow::StartOfList` / `OptionsRow::EndOfList`. Put the two rows in `OptionsRow::ALL`
  directly **before** `JumpTime` (after `SeekBackward`/`SeekForward` when those exist),
  grouping the track-jump rows. Map them in `OptionsRow::message` to
  `Message::OptionsChoice(OptionsChoice::StartOfList)` / `(…EndOfList)`, and update
  `options_rows_map_to_mouse_messages_in_draw_order` in the same positions.
- `src/ui/views/options.rs`: `options_row_label` returns the fixed labels `Start of list` and
  `End of list` (Winamp's wording). No new state goes into `OptionsState`. Name the rows in the
  module doc (`//!`) and in the `view_options_overlay` doc wherever they list the fixed rows
  and the `OptionsChoice` rows.
- `src/ui/handlers/window.rs` `options_choice`: dispatch the new choices to
  `super::playback::stepping::jump_to_edge_track(player, false)` /
  `…(player, true)` and return that task. `options_choice` already clears `options_visible`
  first, and with no songs showing `jump_to_edge_track` schedules nothing (pinned for the keys
  already by `edge_jumps_with_no_songs_loaded_do_nothing`), so with no song list the row only
  closes the menu. Like the keys, the edge jumps ignore Shuffle and Repeat and restart the
  same track when it is already the first or last song.
- Tests:
  - `src/ui/tests/jump_tests.rs` (which holds `options_ten_song_rows_jump_like_the_keypad`):
    a new test, `options_start_and_end_of_list_rows_jump_like_the_keys`, in the same shape:
    `player_jumping_from("song-15")` with `options_visible = true`, each row by mouse
    (`row.message()`) and by keyboard (`options_position(row)` then
    `Message::OptionsActivate`) closes the menu and `assert_track_selected(task, 0, …)` —
    first row to index 0, last row to `stepping`-style expected last index — plus, with an
    empty player (`test_player()`, `options_visible = true`), the row closes the menu and
    `assert_no_task(task)`.
  - `src/ui/tests/panels_tests.rs`: the exhaustive `match` over `OptionsChoice` gets a
    `OptionsChoice::StartOfList | OptionsChoice::EndOfList` arm with an `unreachable!` naming
    `options_start_and_end_of_list_rows_jump_like_the_keys`.
  - `src/ui/views/tests/chrome.rs`: add `"Start of list"` and `"End of list"` to the label
    lists in `options_overlay_draws_each_row_with_its_state` and
    `options_overlay_publishes_each_choice_and_closes_from_close_and_backdrop`, with match arms
    asserting each publishes its `OptionsChoice`.
- Docs:
  - `docs/usage.md`: in the options-menu row list, insert `Start of list` and `End of list`
    after `Jump to time`, and say they jump to the first or last song of the list exactly like
    Ctrl+Z / Ctrl+B (only closing the menu with no song list showing).
  - `docs/invariants.md` `## Options panel`: add the two rows to the opening row summary and
    the row list, add the `jump_to_edge_track` dispatch to the `options_choice` bullet, and
    pin the new test.

**Files touched.** `src/ui/message.rs`, `src/ui/views/options.rs`, `src/ui/handlers/window.rs`,
`src/ui/tests/jump_tests.rs`, `src/ui/tests/panels_tests.rs`, `src/ui/views/tests/chrome.rs`,
`docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- The options menu draws `Start of list` and `End of list` directly above `Jump to time`.
  Up/Down reach them, and Enter or a click activates them.
- While a song list is showing, choosing either row by mouse or by Enter closes the menu and
  plays the first or last song of that list and scrolls the list to the row, exactly like
  Ctrl+Z / Ctrl+B — regardless of Shuffle or Repeat.
- With no song list showing, the row only closes the menu and schedules no work.
- The Ctrl+Z / Ctrl+B shortcuts, Home/End, Previous/Next track, and `play_history` behavior is
  unchanged, and no
  new setting, state field, or dependency is added.
- `docs/usage.md` and `docs/invariants.md` list the rows, and `make check` passes.

## Done
