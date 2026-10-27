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
