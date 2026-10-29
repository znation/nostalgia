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

### Show File info in the options menu, like Winamp's main-menu "View file info" (planned 2026-10-29 by the plan role)

**Goal.** Winamp's main menu lists "View file info…  Alt+3". Our options menu already carries
Winamp main-menu and Playback-menu items (`10 songs back`, `Jump to time`, `Jump to file`, `About
Nostalgia`), but the File info panel opens only from Alt+3 and the LCD strip's `I`. Add one fixed
row, `File info`, between `Jump to file` and `About Nostalgia`.

**Approach.**
- `src/ui/message.rs`: add `OptionsChoice::FileInfo` (doc: "Opens the File info panel while a
  known track is current.") and `OptionsRow::FileInfo` between `JumpFile` and `About`, in the enum
  and in `OptionsRow::ALL` (length 14 → 15). `OptionsRow::message` maps it to
  `Message::OptionsChoice(OptionsChoice::FileInfo)`. Route it through `OptionsChoice`, not
  straight to `Message::ToggleFileInfo`. `handlers::window::toggle_file_info` returns early
  when `current_known_track()` is `None`, and that early return skips `close_transient_panels`, so
  a direct mapping would leave the menu open with no track current. `options_choice` clears
  `options_visible` first, so the menu always closes. This is the same reasoning the Jump rows
  use. Add the new message to `options_rows_map_to_mouse_messages_in_draw_order`.
- `src/ui/handlers/window.rs` `options_choice`: add `OptionsChoice::FileInfo =>
  toggle_file_info(player)`, a same-module call. Opening the menu ran `close_transient_panels`,
  so `file_info_visible` is clear and the toggle takes its opening branch (including the shade
  restore, though the menu cannot be open while shaded). `options_activate` needs no change.
- `src/ui/views/options.rs` `options_row_label`: `OptionsRow::FileInfo => "File info"`. It is a
  fixed label, so it needs no `OptionsState` field and no label helper. Update the module doc
  (`//!`) and the `view_options_overlay` doc, which list Jump to time, Jump to file, and About, so
  they name File info too.
- Tests:
  - `src/ui/views/tests/chrome.rs`: add `File info` between `Jump to file` and `About Nostalgia`
    in `options_overlay_draws_each_row_with_its_state`'s fixed rows. In
    `options_overlay_publishes_each_choice_and_closes_from_close_and_backdrop`, add the label to
    the list, plus an arm asserting it publishes `Message::OptionsChoice(OptionsChoice::FileInfo)`.
  - `src/ui/tests/panels_tests.rs` `options_choice_applies_each_toggle_and_closes_the_panel`: its
    exhaustive `match` needs an `OptionsChoice::FileInfo` arm, an `unreachable!` naming the new
    test below.
  - New test in `src/ui/tests/window_tests.rs`, `options_file_info_row_opens_the_panel_or_just_closes`,
    placed beside the `toggle_file_info_*` tests. It runs each case by mouse (`OptionsRow::FileInfo.message()`)
    and by keyboard (`options_selected` set to the row's index in `OptionsRow::ALL`, then
    `Message::OptionsActivate`), on a `test_player()` with `options_visible = true`:
    - After `make_current_track_known`, the menu closes and `file_info_visible` is set.
    - With no current track (plain `test_player()`), the menu closes and `file_info_visible`
      stays clear.
- Docs:
  - `docs/usage.md`: in the options-menu sentence, list `File info` between `Jump to file` and
    `About Nostalgia`, and say it opens the Alt+3 File info panel (doing nothing beyond closing
    the menu while no track is current).
  - `docs/invariants.md` `## Options panel`: add `File info` to the Guarantee's opening row
    summary and to the `views::view_options_overlay` row list. Add
    `handlers::window::toggle_file_info` to the `options_choice` dispatch bullet. The "eight label
    helpers" count stays correct. Add the new test to **Pinned by**.

**Files touched.** `src/ui/message.rs`, `src/ui/handlers/window.rs`, `src/ui/views/options.rs`,
`src/ui/views/tests/chrome.rs`, `src/ui/tests/panels_tests.rs`, `src/ui/tests/window_tests.rs`,
`docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- The options overlay draws `File info` between `Jump to file` and `About Nostalgia`, and
  clicking it publishes `OptionsChoice::FileInfo`.
- With a known track current, choosing the row by mouse or by Enter closes the menu and opens the
  File info panel. With no current track, it closes the menu and opens nothing.
- Alt+3 and the LCD `I` behave as before. No new setting, state field, or dependency is added.
- `docs/usage.md` and `docs/invariants.md` list the row, and `make check` passes.

## Done
