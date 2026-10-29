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

### Show Back 5 seconds and Fwd 5 seconds in the options menu, like Winamp's Playback menu (planned 2026-10-29 by the plan role)

**Goal.** Winamp's Playback menu lists `Back 5 seconds` and `Fwd 5 seconds` (Left and Right).
Our five-second seeks (`Message::SeekBackward` / `Message::SeekForward`, dispatched in
`src/ui/mod.rs` to `handlers::mixer::seek_backward` / `seek_forward`, which call
`seek_preview` with `±views::SEEK_STEP_SECS`) are reachable only from those keys and the
seek-roller on the seek strip, so there is no mouse-only way to seek. Add two fixed options-menu
rows that run the same seeks. This plan is independent of the `Stop with fadeout` plan above:
that plan inserts its row between `LibraryPanel` and `StopAfterCurrent`, and this one inserts
between `JumpTracksForward` and `JumpTime`, so the two plans touch different insertion points.
Whichever lands second only updates the `OptionsRow::ALL` length, the length in its array
declaration, and the row lists.

**Approach.**
- `src/ui/message.rs`: add `OptionsChoice::SeekBackward` and `OptionsChoice::SeekForward` (doc
  comments: "Reuses [`Message::SeekBackward`]." / "… [`Message::SeekForward`].") and
  `OptionsRow::SeekBackward` / `OptionsRow::SeekForward`. Put the two rows in `OptionsRow::ALL`
  directly **before** `JumpTime`, after `JumpTracksBack`/`JumpTracksForward`, grouping the seek
  and jump rows (`ALL` grows from 15 to 17: …, `Shuffle`, `JumpTracksBack`, `JumpTracksForward`,
  `SeekBackward`, `SeekForward`, `JumpTime`, …). Map them in `OptionsRow::message` to
  `Message::OptionsChoice(OptionsChoice::SeekBackward)` / `(…SeekForward)`, and update
  `options_rows_map_to_mouse_messages_in_draw_order` in the same positions.
- `src/ui/views/options.rs`: `options_row_label` returns the fixed labels `Back 5 seconds` and
  `Fwd 5 seconds` (Winamp's wording, length understood as time like the LCD, not songs). No new
  state goes into `OptionsState`. Name the rows in the module doc (`//!`) and in the
  `view_options_overlay` doc wherever they list the fixed rows and the `OptionsChoice` rows.
- `src/ui/handlers/window.rs` `options_choice`: dispatch the new choices to
  `super::mixer::seek_backward(player)` / `super::mixer::seek_forward(player)` and return that
  task. `options_choice` already clears `options_visible` first, and a seek with no preview
  loaded schedules no work, so with nothing playing the row only closes the menu —
  `seek_messages_drive_the_audio_backend` pins that outcome for the arrow keys already.
- Tests:
  - `src/ui/tests/mixer_tests.rs` (which holds the seek tests): a new test,
    `options_back_and_fwd_seconds_rows_seek_like_the_arrows`. Use `player_with_recording()` with
    `options_visible = true`. Each row, by mouse (`row.message()`) and by keyboard
    (`options_selected` set to the row's index found in `OptionsRow::ALL` with
    `iter().position(|row| matches!(row, OptionsRow::SeekForward))`, since `OptionsRow` has no
    `PartialEq`, then `Message::OptionsActivate`), closes the menu, schedules no work, and sends
    the backend exactly one `AudioCall::SeekBy(-5)` (back) or `AudioCall::SeekBy(5)` (forward) —
    written as literals, like `seek_messages_drive_the_audio_backend`, to pin the five-second
    step.
  - `src/ui/tests/panels_tests.rs`: the exhaustive `match` over `OptionsChoice` gets a
    `OptionsChoice::SeekBackward | OptionsChoice::SeekForward` arm with an `unreachable!` naming
    `options_back_and_fwd_seconds_rows_seek_like_the_arrows`.
  - `src/ui/views/tests/chrome.rs`: add `"Back 5 seconds"` and `"Fwd 5 seconds"` to the label
    lists in `options_overlay_draws_each_row_with_its_state` and
    `options_overlay_publishes_each_choice_and_closes_from_close_and_backdrop`, with match arms
    asserting each publishes its `OptionsChoice`.
- Docs:
  - `docs/usage.md`: in the options-menu row list, insert `Back 5 seconds` and `Fwd 5 seconds`
    after `10 songs forward`, and say they seek five seconds back or forward exactly like the
    Left and Right arrows (only closing the menu with nothing playing).
  - `docs/invariants.md` `## Options panel`: add the two rows to the opening row summary and the
    row list, add `handlers::mixer::seek_backward` / `seek_forward` to the `options_choice`
    dispatch bullet, and pin the new test.

**Files touched.** `src/ui/message.rs`, `src/ui/views/options.rs`, `src/ui/handlers/window.rs`,
`src/ui/tests/mixer_tests.rs`, `src/ui/tests/panels_tests.rs`, `src/ui/views/tests/chrome.rs`,
`docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- The options menu draws `Back 5 seconds` and `Fwd 5 seconds` between `10 songs forward` and
  `Jump to time`. Up/Down reach them, and Enter or a click activates them.
- While a preview plays, choosing either row by mouse or by Enter closes the menu and seeks the
  playing preview by exactly five seconds back or forward in the backend, like the arrow keys.
- With no preview playing, the row only closes the menu and schedules no work.
- The Left/Right arrows, keypad 7/9, and seek-roller behavior is unchanged, and no new setting,
  state field, or dependency is added.
- `docs/usage.md` and `docs/invariants.md` list the rows, and `make check` passes.

## Done
