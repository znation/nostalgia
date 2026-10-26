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

### Choose the Jump to File match with Up and Down, like Winamp (planned 2026-10-26 by the plan role)

**Goal.** The Jump to File panel's selection bar still marks only the first filtered match, and
Enter or `Jump` always plays that first match. Let plain Up and Down move the highlighted row
through the filtered matches and make Enter, `Jump`, and the selection bar all name that
highlighted row: the first match is highlighted until the user moves, Down from it selects the
second match, Up from it stays on the first, and repeated presses stop at the first or last match
without wrapping. Typing a new filter or reopening the panel puts the highlight back on the first
match. This is the interaction the numbered rows and selection bar already set up; it changes no
playback, browse, filter, settings, or persistence behavior outside the panel.

**Approach.**

- `src/ui/message.rs`: add payload-free `JumpFileSelectionUp` and `JumpFileSelectionDown` beside
  `JumpFileChanged`/`JumpFileSubmitted`, both documented as acting only while the panel is open
  and changing no playback state.
- `src/ui/player.rs`: add `pub(in crate::ui) jump_file_selected: Option<usize>` after
  `jump_file_query`, documented as the highlighted match's source index (`None` means the first
  match, as on opening or after typing), initialize it to `None` in `WinampPlayer::new`, and reset
  it in `handlers::jump::toggle_jump_file` and `jump_file_changed`. It is layout-only and not
  persisted.
- `src/ui/views/jump_file.rs`:
  - Add `pub(crate) fn jump_file_highlighted(matches: &[usize], selected: Option<usize>) ->
    Option<usize>` returning `selected` when it appears in `matches`, otherwise the first match,
    and `None` for an empty list. The view and the submit handler both call it, so the row drawn
    with the selection bar and the row Enter plays cannot drift.
  - Change `view_jump_file_overlay` to take `selected: Option<usize>` after `epoch`, compute
    `let highlighted = jump_file_highlighted(&matches, selected);` before consuming `matches`, and
    style each row `jump_file_row_style(highlighted == Some(index), status)`. Update its doc from
    the first match being the Enter target to the highlighted match, defaulting to the first.
  - Rename `jump_file_row_style`'s `is_default` parameter to `highlighted` (same signature and
    colors) and update its doc and test name; the blue bar and white text stay the browse
    selection style, and every other row keeps the flat green playlist colors.
- `src/ui/views/mod.rs`: re-export `jump_file_highlighted` beside the existing
  `jump_file_matches` `pub(crate) use`.
- `src/ui/view.rs`: pass `player.jump_file_selected` to both `view_jump_file_overlay` calls (the
  Songs and All Songs branches).
- `src/ui/handlers/jump.rs`:
  - Add `jump_file_selection_up` and `jump_file_selection_down`, or one private
    `move_jump_file_selection(player, up: bool)` with both thin public handlers. Each no-ops unless
    `jump_file_visible`; computes `jump_file_matches` from the current query; leaves an empty match
    list with `jump_file_selected = None`; otherwise takes the highlighted match's position in
    `matches` (a stale or `None` selection counts as position 0) and moves it to
    `saturating_sub(1)` for Up or `(position + 1).min(matches.len() - 1)` for Down, storing
    `Some(matches[position])`. Both return `Task::none()` and touch nothing else.
  - Change `jump_file_submitted` to dispatch `views::jump_file_highlighted(&matches,
    player.jump_file_selected)` through `TrackSelected`; no match still leaves the panel, query,
    and selection alone, and a successful submit still closes the panel.
  - `jump_file_changed` resets `jump_file_selected` to `None` after storing the text, so a new
    filter always starts at its first match.
- `src/ui/shortcuts.rs`: add `pub(super) fn message_for_jump_file(event: Event) -> Option<Message>`
  beside `message_for`. For a key press with Control, Alt, and Logo released, `Key::Named(ArrowUp)`
  maps to `JumpFileSelectionUp` and `ArrowDown` to `JumpFileSelectionDown` (Shift allowed, like
  the other letter bindings); every other event falls through to `message_for`, so J, Ctrl+J, and
  the rest keep their existing mappings. iced's `TextInput` does not consume Up/Down (it handles
  Enter, Backspace/Delete, Home/End, Left/Right, and Escape), so the filter box keeps focus while
  these keys reach the listener.
- `src/ui/timers.rs`: give `subscriptions` a `jump_file_open: bool` argument and install
  `keyboard::listen().filter_map(if jump_file_open { message_for_jump_file } else {
  message_for })`, documenting that Up/Down choose a Jump to File match only while that panel is
  open and otherwise stay volume. `src/ui/mod.rs` passes `player.jump_file_visible` and routes the
  two new messages to the handlers. Update every `subscriptions(...)` call in
  `src/ui/tests/timers_tests.rs` with the new argument.
- `src/ui/views/tests/jump_file.rs`: give the `overlay` helper `selected: Option<usize>` (existing
  calls pass `None`); add `jump_file_highlighted_prefers_a_selected_match_and_falls_back_to_first`
  (selected match, stale selection, `None`, empty matches) and
  `overlay_highlights_the_selected_row`, which lays out three unfiltered rows with `Some(2)` and
  uses `layout_capture::capture_text_and_pixels` with `max_red_in` on the selected and a plain row's
  text bounds to prove the selected row draws the white selection text rather than plain green.
- `src/ui/tests/jump_file_tests.rs`: add `down_and_up_move_the_selection_through_the_matches`
  (open, Down selects the second match then the last and stops, Up walks back and stops at the
  first, each step scheduling no task), `typing_a_filter_resets_the_selection_to_the_first_match`,
  and `submit_plays_the_selected_match`. Keep the existing first-match submit test as the default
  behavior.
- `docs/usage.md`: in the Jump to File sentence, say Up and Down move the selection bar through the
  matches and Enter or `Jump` plays the highlighted row, the first match until you move it.
- `docs/invariants.md` `## Jump to File panel`: replace the first-match guarantee with the
  highlighted-match guarantee — `jump_file_highlighted` resolves a still-valid
  `jump_file_selected` or the first match, Up/Down clamp within the filtered matches without
  wrapping, a new query or opening resets the selection, and Enter/`Jump` plays the highlighted
  row — and add `shortcuts::message_for_jump_file`, `handlers::jump`, and `ui::timers` to the
  enforcers plus the new tests to `Pinned by`.

**Files touched.** `src/ui/message.rs`, `src/ui/player.rs`, `src/ui/views/jump_file.rs`,
`src/ui/views/mod.rs`, `src/ui/view.rs`, `src/ui/handlers/jump.rs`, `src/ui/shortcuts.rs`,
`src/ui/timers.rs`, `src/ui/mod.rs`, `src/ui/views/tests/jump_file.rs`,
`src/ui/tests/jump_file_tests.rs`, `src/ui/tests/timers_tests.rs`, `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- With the panel open on an unfiltered three-song list, Down selects the second match and then the
  third and stays there; Up walks back to the second and the first and stays there. The selection
  is the source index, so a filtered row keeps the number its source list gave it.
- Typing a new filter and reopening the panel both put the highlight back on the first match, and
  Enter or `Jump` plays that row.
- Enter or `Jump` plays the highlighted match rather than always the first; with no highlight it
  still plays the first match and closes the panel exactly as before, and a no-match submit still
  leaves the panel and query alone.
- The overlay draws the highlighted row with `theme::ROW_HIGHLIGHT` behind
  `theme::PLAYING_ROW_TEXT` and every other row in the flat green row style; the default selection
  is the first match.
- Up/Down move the Jump to File selection only while the panel is open; with the panel closed they
  still nudge the volume, and every other keyboard shortcut and media key keeps its mapping.
- No playback, audio, browse, filter, settings, or persistence behavior changes; `jump_file_selected`
  is not saved, and closing the panel through `Close`, the backdrop, a row press, or a successful
  submit behaves as today.
- `make check` passes.

## Done
