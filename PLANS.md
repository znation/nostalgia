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

### Mark a paused or stopped track in the window title, like Winamp's `[Paused]` and `[Stopped]` (planned 2026-10-27 by the plan role)

**Goal.** Winamp's taskbar title names the current song and adds the transport state when it is not
playing: `1. Artist - Title - Winamp [Paused]` while paused and `… - Winamp [Stopped]` while
stopped. Our window title (`WinampPlayer::window_title`) already names the track as
`<number>Artist - Title - nostalgia` but reads the same whether the song plays, pauses, or stops,
so the taskbar cannot tell a paused song from a playing one. Append ` [Paused]` or ` [Stopped]`
after `nostalgia` while a known track is current and the transport is paused or stopped. Leave the
playing title and the plain `nostalgia` fallback (no current track, or an unknown id) unchanged.

**Approach.**
- `src/ui/player/presentation.rs` `window_title`: it already holds the `AppState` lock as
  `state`. After pushing `APP_TITLE`, match `state.transport_mode()` (`crate::state::TransportMode`):
  `Playing` adds nothing, `Paused` pushes `" [Paused]"`, and `Stopped` pushes `" [Stopped]"`. Raise
  the `String::with_capacity` slack to cover the suffix. Leave the early `APP_TITLE` return alone:
  with no known track the title stays plain `nostalgia` whatever the mode, so a fresh launch does
  not read `nostalgia [Stopped]`. Update the doc comment to describe the suffix. Its sentence
  "Stop leaves `current_track` in place, so the interrupted track's name stays in the title" stays
  true; add that the title then ends in ` [Stopped]`.
- `src/ui/mod.rs`: the `init_ui` doc comment says the title follows the current track. Add "and
  its paused/stopped state" so it stays accurate. No code change there; iced already re-reads
  `.title(...)` after every update.
- Tests in `src/ui/tests/window_tests.rs`:
  - `window_title_names_the_current_track` currently asserts titles on a default `test_player()`,
    whose `AppState` is stopped. With the suffix, those known-track asserts would read
    `… - nostalgia [Stopped]`. Set `state.blocking_lock().is_playing = true` right after seeding
    `current_track`, so the existing number/artist assertions keep their playing-title strings.
    Change the final Stop assertion (after `state.blocking_lock().stop()`) to expect
    `"1. The Sample Band - Opening - nostalgia [Stopped]"`. Update its leading comment.
  - New test `window_title_marks_a_paused_or_stopped_track`: seed the same `KnownTrack` and
    `current_track`. Assert the title has no suffix after `AppState::play()`, ends in
    ` [Paused]` after `AppState::pause()`, and ends in
    ` [Stopped]` after `AppState::stop()`. Then set `current_track = None` and assert the title
    reads plain `nostalgia` while stopped and while paused, with no suffix. Reuse the existing
    `KnownTrack` literal, or the `make_current_track_known` helper in the same file (it seeds
    `song-1` as `1. The Sample Band - Opening`).
- `docs/usage.md`: in the window-title sentence (`<number>Artist - Title - nostalgia` …), say that
  the title ends in ` [Paused]` while paused and ` [Stopped]` while stopped, like Winamp's taskbar
  title. It still keeps the interrupted track's name across Stop and still reads just `nostalgia`
  when no known track is current.
- `docs/invariants.md` `## Now Playing label`: extend the "iced application window's title" bullet
  with the suffix rule (nothing while playing, ` [Paused]`, ` [Stopped]`, never on the plain
  `nostalgia` fallback). Add `window_title_marks_a_paused_or_stopped_track` to that section's
  pinned tests next to `window_title_names_the_current_track`.

**Files touched.** `src/ui/player/presentation.rs`, `src/ui/mod.rs` (doc comment only),
`src/ui/tests/window_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- With a known current track, the window title reads `<number>Artist - Title - nostalgia` while
  playing, gains ` [Paused]` while paused, and gains ` [Stopped]` while stopped (including after
  Stop and after a preview ends with nothing to advance to).
- With no current track or an unknown id, the title is plain `nostalgia` in every transport mode.
- The number and artist rules are unchanged: an empty number or artist is still omitted.
- `docs/usage.md` and `docs/invariants.md` describe the suffix, and `make check` passes.

### Show 10 songs back and 10 songs forward in the options menu, like Winamp's Playback menu (planned 2026-10-27 by the plan role)

**Goal.** Winamp's Playback menu lists `10 tracks back` and `10 tracks fwd` (keypad 1 and 3)
right before `Jump to time` and `Jump to file`. Our ten-song jumps (`Message::JumpTracksBack` /
`Message::JumpTracksForward`, handled by `handlers::playback::stepping::jump_ten_tracks`) are
reachable only from the numeric keypad (`numpad_shortcut` in `src/ui/shortcuts.rs`), so on a
keyboard with no keypad, which includes most Mac laptops, there is no way to use them. Add two
options-menu rows that run the same jumps.

**Approach.**
- `src/ui/message.rs`: add `OptionsChoice::JumpTracksBack` and `OptionsChoice::JumpTracksForward`
  (doc comments: "Reuses [`Message::JumpTracksBack`]." / "… [`Message::JumpTracksForward`].") and
  `OptionsRow::JumpTracksBack` / `OptionsRow::JumpTracksForward`. Put the two rows in
  `OptionsRow::ALL` directly **before** `JumpTime`, following Winamp's order (`ALL` grows from 12 to 14:
  …, `Shuffle`, `JumpTracksBack`, `JumpTracksForward`, `JumpTime`, `JumpFile`, `About`, `Close`).
  Map them in `OptionsRow::message` to `Message::OptionsChoice(OptionsChoice::JumpTracksBack)` /
  `(…JumpTracksForward)`, and update `options_rows_map_to_mouse_messages_in_draw_order`.
- `src/ui/views/options.rs`: `options_row_label` returns the fixed labels `10 songs back` and
  `10 songs forward` ("songs" matches the rest of the app's wording). No new state goes into
  `OptionsState`. Update the `view_options_overlay` doc comment, which lists which rows publish
  `OptionsChoice`.
- `src/ui/handlers/window.rs` `options_choice`: dispatch the new choices to
  `super::playback::stepping::jump_ten_tracks(player, false)` / `(player, true)` and return that
  task. The function already clears `options_visible` first, so an empty list or no song list
  only closes the menu, because `jump_ten_tracks` schedules nothing then.
- Tests:
  - `src/ui/tests/jump_tests.rs`: a new test,
    `options_ten_song_rows_jump_like_the_keypad`. Use `player_jumping_from("song-15")` with
    `options_visible = true`. Each row, by mouse (`row.message()`) and by keyboard
    (`options_selected` set to the row's index found in `OptionsRow::ALL`, then
    `Message::OptionsActivate`), closes the menu and yields
    `assert_track_selected(task, 0, 5)` for back and `(task, 0, 24)` for forward (15+10 clamps to 24).
    A second case uses `test_player()` with no songs: the row closes the menu and
    schedules no work (`assert_no_task`). The existing
    `options_jump_rows_open_their_panels_or_just_close` hard-codes positions 8 and 9. Change it to
    look the positions up in `OptionsRow::ALL`, or bump them to 10 and 11.
  - `src/ui/tests/panels_tests.rs`: the exhaustive `match choice` (the arm listing
    `OptionsChoice::JumpTime | OptionsChoice::JumpFile`) gets the two new variants in an
    `unreachable!("covered by options_ten_song_rows_jump_like_the_keypad")` arm.
  - `src/ui/views/tests/chrome.rs`: add `"10 songs back"` and `"10 songs forward"` to the label
    lists in `options_overlay_draws_each_row_with_its_state` and
    `options_overlay_publishes_each_choice_and_closes_from_close_and_backdrop`, with match arms
    asserting each publishes its `OptionsChoice`.
- Docs: in `docs/usage.md`'s options-menu sentence, insert `10 songs back`, `10 songs forward`
  before `Jump to time` in the row list and say they jump ten songs back or forward exactly like
  keypad 1 and 3 (doing nothing beyond closing the menu with no song list showing). In
  `docs/invariants.md` `## Options panel`, add the two rows to the row list and the guarantee's
  summary line, and add `handlers::playback::stepping::jump_ten_tracks` to the `options_choice`
  dispatch bullet. Pin the new test there too.

**Files touched.** `src/ui/message.rs`, `src/ui/views/options.rs`, `src/ui/handlers/window.rs`,
`src/ui/tests/jump_tests.rs`, `src/ui/tests/panels_tests.rs`, `src/ui/views/tests/chrome.rs`,
`docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- The options menu draws `10 songs back` and `10 songs forward` between `Shuffle: On/Off` and
  `Jump to time`. Up/Down reach them, and Enter or a click activates them.
- Choosing either row closes the menu and plays the song ten places back or forward in the
  showing list, clamped at its ends, ignoring Shuffle and Repeat. This is the same
  `TrackSelected` and scroll that keypad 1 / 3 produce.
- With no songs showing, either row only closes the menu.
- Keypad 1 / 3 behavior is unchanged, and so are the other rows.
- `docs/usage.md` and `docs/invariants.md` describe the rows, and `make check` passes.

## Done
