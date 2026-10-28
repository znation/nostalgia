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

### Open the Jump to time box holding the current position, selected, like Winamp (planned 2026-10-28 by the plan role)

**Goal.** Winamp's Jump to time dialog opens with its box already holding the playing track's
current position, so you can see where you are and type over it. Nostalgia's Ctrl+J panel opens
with an empty box (`handlers::jump::toggle_jump_time` calls `player.jump_time_query.clear()`).
Opening should instead fill the box with the current position in the `M:SS` form that
`parse_jump_time` already accepts, and select all of it, so typing replaces it and Enter on the
untouched text seeks to where playback already is.

**Approach.**
- `src/ui/views/mod.rs`: make `format_minutes_seconds` `pub(in crate::ui)`, so the handler can
  reuse it. Keep its doc comment and add the Jump to time prefill to the list of its users. Do not
  use `format_track_time`, because it turns `0` into `--:--`, which `parse_jump_time` rejects.
- `src/ui/handlers/jump.rs` `toggle_jump_time`: replace `player.jump_time_query.clear()` with:
  set `jump_time_query` to `views::format_minutes_seconds(ms)` when `player.playback_position` is
  `Some(position)`, where `ms` is `u64::try_from(position.as_millis()).unwrap_or(u64::MAX)` (the
  same conversion `view_jump_time_overlay` uses for the length). When it is `None`, clear the query.
  Leave the position unclamped: `jump_time_submitted` already clamps to `playback_duration`.
  Replace the `focus` task with
  `Task::batch([iced::widget::operation::focus(views::JUMP_TIME_INPUT_ID), iced::widget::operation::select_all(views::JUMP_TIME_INPUT_ID)])`
  (`select_all` is in iced_runtime 0.14's `widget::operation`, next to `focus`). Keep the
  existing shaded branch, which batches `toggle_window_shade` before the focus task. Update the
  doc comment: opening now fills the query with the position, or clears it when there is none,
  where it used to just clear the query. The Options menu's `Jump to time` row calls this same
  function, so it gets the prefill with no change of its own.
- `src/ui/tests/jump_time_tests.rs`:
  - Keep `toggle_opens_with_an_empty_query_and_closes_again` for the `None` position.
    `player_with_duration` sets no position, so it must still see an empty query after a stale one.
  - Add `toggle_opens_holding_the_current_position`. With `playback_position = Some(95 s)`, opening
    leaves `jump_time_query == "1:35"` and schedules work.
  - Add `toggle_opens_holding_zero_at_the_start`. `Some(Duration::ZERO)` gives `"0:00"`, not `--:--`.
  - Add `submitting_the_prefilled_query_seeks_to_the_current_position`. Open at `Some(12 s)` with
    the existing 30 s duration, send `JumpTimeSubmitted` without typing, and assert three things,
    the way `submit_seeks_records_the_position_and_closes` does: `playback_position` is still
    `Some(12 s)`, `position_epoch` went up by one (so a seek really ran), and the panel is closed.
- `docs/usage.md`: in the Ctrl+J sentence, say the box opens holding the current position (`1:35`),
  selected so typing replaces it, or empty when no position is known.
- `docs/invariants.md` `## Jump to time panel`: change "Opening clears the query" to "Opening
  sets the query to `format_minutes_seconds` of `playback_position`, or clears it when that is
  `None`, and focuses and selects the box". Add the new tests to **Pinned by**.

**Files touched.** `src/ui/handlers/jump.rs`, `src/ui/views/mod.rs`,
`src/ui/tests/jump_time_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- With a known preview length and a playback position of 95 s, Ctrl+J (or the Options menu's
  `Jump to time` row) opens the panel with `1:35` in the box. A position of 0 shows `0:00`.
- With no known position, the box opens empty, as it does today.
- Opening focuses the box and selects its text, so a typed time replaces the prefill.
- Enter on the untouched prefill seeks to the current position and closes the panel.
- Closing, the other panels' mutual closing, and the shaded restore behave as before. The existing
  tests in `jump_time_tests.rs` pass unchanged.
- `docs/usage.md` and `docs/invariants.md` describe the prefill.
- `make check` passes.

## Done
