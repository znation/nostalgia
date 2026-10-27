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

## Done
