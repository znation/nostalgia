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

### Show Stop with fadeout in the options menu, like Winamp's Playback menu (planned 2026-10-29 by the plan role)

**Goal.** Winamp's Playback menu lists "Stop w/ fadeout  Shift+V" just above "Stop after current".
Our options menu already carries `Stop after current: On/Off`, but the fade-out stop is reachable
only from Shift+V and a Shift-held click on Stop. Add one fixed row, `Stop with fadeout`, between
the `Library: Shown/Hidden` row and `Stop after current: On/Off`. This plan is independent of the
File info row plan above. That plan inserts its row between `Jump to file` and `About Nostalgia`,
so the two plans add rows in different places. Whichever lands second only updates the row count
and the row lists.

**Approach.**
- `src/ui/message.rs`: add `OptionsChoice::FadeOutAndStop` (doc: "Reuses
  [`Message::FadeOutAndStop`].") and `OptionsRow::FadeOutAndStop` between `LibraryPanel` and
  `StopAfterCurrent`, both in the enum and in `OptionsRow::ALL`. The array length grows by one.
  `OptionsRow::message` maps the new row to
  `Message::OptionsChoice(OptionsChoice::FadeOutAndStop)`, so `options_choice` closes the menu
  first, like every other choice row. Add the new message at the matching position in
  `options_rows_map_to_mouse_messages_in_draw_order`.
- `src/ui/handlers/window.rs` `options_choice`: add `OptionsChoice::FadeOutAndStop =>
  super::playback::fade::fade_out_and_stop(player)`. That handler already covers both cases.
  While a preview plays it starts the countdown with no backend call. While paused or stopped it
  acts as Stop. `options_activate` needs no change.
- `src/ui/views/options.rs` `options_row_label`: `OptionsRow::FadeOutAndStop => "Stop with
  fadeout"`. It is a fixed label, so it needs no `OptionsState` field and no label helper. Name
  the row in the module doc (`//!`) and in the `view_options_overlay` doc wherever they list the
  fixed rows.
- Tests:
  - `src/ui/views/tests/chrome.rs`: add `Stop with fadeout` between the `Library: …` and `Stop
    after current: …` rows in `options_overlay_draws_each_row_with_its_state`. In
    `options_overlay_publishes_each_choice_and_closes_from_close_and_backdrop`, add the label and
    an arm asserting it publishes `Message::OptionsChoice(OptionsChoice::FadeOutAndStop)`.
  - `src/ui/tests/panels_tests.rs` `options_choice_applies_each_toggle_and_closes_the_panel`: its
    exhaustive `match` needs an `OptionsChoice::FadeOutAndStop` arm, an `unreachable!` naming the
    new test below.
  - New test in `src/ui/tests/fade_tests.rs`,
    `options_stop_with_fadeout_row_fades_or_stops_and_closes`. It runs each case by mouse
    (`OptionsRow::FadeOutAndStop.message()`) and by keyboard (`options_selected` set to the row's
    index in `OptionsRow::ALL`, found with `iter().position(|row| matches!(row,
    OptionsRow::FadeOutAndStop))`, since `OptionsRow` has no `PartialEq`, then
    `Message::OptionsActivate`). Each case starts with `options_visible = true`:
    - On `playing_with_recording()`: the menu closes, `fade_out_ticks == FADE_OUT_TICKS`,
      `recording.calls()` is empty, and the state still reports playing.
    - With `is_playing` cleared, as in `fade_out_while_not_playing_acts_as_stop`: the menu closes,
      the task settles via `assert_transport_settled(task, "stop")`, `fade_out_ticks` stays 0, and
      the calls are `[AudioCall::Stop]`.
- Docs:
  - `docs/usage.md`: in the options-menu row list, put `Stop with fadeout` before `Stop after
    current: On/Off`. In the Shift+V sentence, mention the options-menu row as another way to
    fade out and stop.
  - `docs/invariants.md`:
    - `## Options panel`: add the row to the Guarantee's opening row summary and to the
      `views::view_options_overlay` row list, add `handlers::playback::fade::fade_out_and_stop` to
      the `options_choice` dispatch bullet, and add the new test to **Pinned by**.
    - `## Fade-out stop`: mention the options row in the Guarantee and list the new test there too.

**Files touched.** `src/ui/message.rs`, `src/ui/handlers/window.rs`, `src/ui/views/options.rs`,
`src/ui/views/tests/chrome.rs`, `src/ui/tests/panels_tests.rs`, `src/ui/tests/fade_tests.rs`,
`docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- The options overlay draws `Stop with fadeout` between `Library: Shown/Hidden` and `Stop after
  current: On/Off`, and clicking it publishes `OptionsChoice::FadeOutAndStop`.
- While a preview plays, choosing the row by mouse or by Enter closes the menu and starts the same
  fade Shift+V starts. While paused or stopped, it closes the menu and stops.
- Shift+V and the Shift-held Stop click behave as before. No new setting, state field, or
  dependency is added.
- `docs/usage.md` and `docs/invariants.md` list the row, and `make check` passes.

## Done
