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

### Stop after current, part 1/2: Ctrl+V stops after the current song, like Winamp (planned 2026-10-27 by the plan role)

**Goal.** Winamp's Ctrl+V arms "Stop after current": the playing song plays to its end, and then
playback stops instead of advancing; the choice is used up when it fires. Nostalgia binds plain V
to Stop (`shortcut` in `src/ui/shortcuts.rs`) but leaves Ctrl+V unbound, and
`handlers::playback::preview::playback_finished` always advances when a next song exists. Add the
chord and the one-shot stop. Part 2/2 ("Stop after current, part 2/2: show it in the options menu") adds the
visible menu row; this part has no on-screen indicator, like Winamp's main window.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) stop_after_current: bool` to `WinampPlayer`, doc'd as
  the transient one-shot flag (false at launch, not persisted, cleared when a natural preview end
  consumes it), and initialise it to `false` in `WinampPlayer::new`.
- `src/ui/message.rs`: add `Message::ToggleStopAfterCurrent`, documented beside `Message::Stop`.
- `src/ui/shortcuts.rs`: in `shortcut`'s Control block (the no-Alt, no-Logo match that already holds
  Ctrl+Z/B/R/W/J/A/T), map `v`/`V` with or without Shift to `Message::ToggleStopAfterCurrent`,
  and add `v` to that block's leading comment. Plain V stays `Message::Stop`.
- `src/ui/handlers/playback.rs`: add `pub(in crate::ui) fn toggle_stop_after_current(player)` that
  flips the flag and returns `Task::none()` — no transport, playback, or window change. Dispatch it
  from `update` in `src/ui/mod.rs`.
- `src/ui/handlers/playback/preview.rs`, `playback_finished`: after the existing early returns
  (not finished, stale epoch / pending transition, not playing) and before picking the next song,
  when `player.stop_after_current` holds, set it to `false` and return the existing `stop(player)`
  task. Extend the function's doc comment with this rule. A dropped reply leaves the flag armed,
  and manual Play, Pause, Stop, Next, and Previous neither read nor clear it.

**Tests.**

- `src/ui/shortcuts.rs` tests: a `message_for_maps_ctrl_v_to_stop_after_current` test covering
  `v`/`V` under `Modifiers::CTRL` and `CTRL | SHIFT`, and `Modifiers::CTRL | ALT` and
  `CTRL | LOGO` mapping to none; `assert_letter_bindings`' plain `v` → `Message::Stop` still holds.
- `src/ui/tests/stepping_tests.rs`, reusing `player_playing_stepping_song` and
  `drive_playback_finished` (which yields `Message::TransportSettled` on the stop path, as
  `preview_end_at_the_last_song_stops_when_repeat_is_off` asserts, and `TrackSelected` on an
  advance):
  - `preview_end_with_stop_after_current_stops_and_clears_it`: playing `song-1` of the stepping
    list with a next song available, flag armed → after the finished reply, `is_playing` is false,
    `current_track` is still `song-1`, and `stop_after_current` is false.
  - `preview_end_after_stop_after_current_fired_advances_again`: a second song played after the
    flag fired advances normally on its natural end.
  - `stop_after_current_survives_a_dropped_finished_reply`: a stale-epoch reply leaves the flag
    armed.
  - `toggle_stop_after_current_flips_only_the_flag`: two `Message::ToggleStopAfterCurrent` updates
    go true then false, and the returned task produces no message; playing state is unchanged.

**Docs.** `docs/usage.md`'s keyboard paragraph (beside "X plays, C pauses, V stops") gains Ctrl+V;
`docs/invariants.md` `## Preview end` records the one-shot rule and pins the new stepping tests.

**Acceptance criteria.**

- Ctrl+V (and Ctrl+Shift+V) toggles `stop_after_current`; Ctrl+Alt+V and Ctrl+Logo+V do nothing;
  plain V still stops.
- With it armed, a natural preview end stops playback through the existing `stop` path, keeps the
  song current for Play, and disarms; with it off, natural-end behavior is unchanged (the existing
  `preview_end_*` tests pass untouched).
- No setting is added or persisted, and `make check` passes.

### Stop after current, part 2/2: show it in the options menu (planned 2026-10-27 by the plan role; "requires part 1/2 landed")

**Goal.** Part 1/2 ("Stop after current, part 1/2: Ctrl+V stops after the current song") arms
`WinampPlayer::stop_after_current` with no on-screen state. Winamp shows that choice as a checked
menu item, so add a `Stop after current: On/Off` row to the options menu, after `Library` and
before `About Nostalgia`, which toggles the flag and closes the menu like the other rows.

**Approach.**

- `src/ui/message.rs`: add `OptionsChoice::StopAfterCurrent` (doc: reuses
  `Message::ToggleStopAfterCurrent`) and `OptionsRow::StopAfterCurrent`, placed in
  `OptionsRow::ALL` after `LibraryPanel` (the array grows to 8), mapped in `OptionsRow::message`;
  update `options_rows_map_to_mouse_messages_in_draw_order`.
- `src/ui/handlers/window.rs`, `options_choice`: route the new choice to
  `super::playback::toggle_stop_after_current`. Keyboard selection already clamps against
  `OptionsRow::ALL.len()`, so Up/Down/Enter reach the row unchanged.
- `src/ui/views/options.rs`: adding a seventh bool would push `view_options_overlay` to eight
  parameters and trip clippy's `too_many_arguments` under `make check`. So replace its five bools
  with one `#[derive(Debug, Clone, Copy)] pub struct OptionsState { always_on_top, time_remaining,
  visualizer_visible, eq_panel_visible, library_panel_visible, stop_after_current }` (all `bool`,
  documented). `options_row_label(row, state: OptionsState)` and
  `view_options_overlay(base, state: OptionsState, selected)` take it. Add
  `stop_after_current_label(bool)` → `"Stop after current: On"` / `"Stop after current: Off"`.
  Re-export it beside the existing `pub use options::view_options_overlay;` in
  `src/ui/views/mod.rs` so `src/ui/view/overlay.rs` can name it.
- `src/ui/view/overlay.rs`, `apply_overlays`: build the `OptionsState` from the player fields it
  passes today plus `player.stop_after_current`.

**Tests.**

- `src/ui/views/options.rs`: `stop_after_current_label_mirrors_the_flag`.
- `src/ui/views/tests/chrome.rs`: `options_overlay_element` builds an `OptionsState` with every
  field `all_on`, and `options_overlay_draws_each_row_with_its_state` expects the new row in both
  cases, in draw order.
- `src/ui/tests/panels_tests.rs`: choosing `OptionsChoice::StopAfterCurrent` arms the flag and
  closes the menu (`options_visible` false), and choosing it again disarms it.

**Docs.** `docs/usage.md`'s options-menu sentence lists `Stop after current: On/Off` between
`Library: Shown/Hidden` and `About Nostalgia`; `docs/invariants.md` `## Options panel` names the
row and pins the new tests.

**Acceptance criteria.**

- The options menu shows `Stop after current: On` while the flag is armed and `Off` otherwise,
  between the Library and About rows; clicking it or Enter on it toggles the flag and closes the
  menu.
- Every other row's label, order, and message are unchanged, and `make check` passes.

## Done
