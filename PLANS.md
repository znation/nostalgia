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

## Done
