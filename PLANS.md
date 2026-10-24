# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the README's Usage section: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. Record any new drift here rather than appending
> another note.

## Planned

### Snap the browse list to its ends with Home and End (planned 2026-10-23 by the plan role)

**Goal.** The browse list can be scrolled with the mouse wheel and sent back to its first row with
`Top`, and the keyboard already covers opening (`Enter`), going back (`Backspace`), and focusing
search (`Ctrl+F`), but there is no keyboard route to the first or last row. Bind `Home` to the
same snap-to-first-row action as `Top` and add `End` as the matching snap-to-last-row action, from
any normal-mode browse level. Both are view-scroll operations only: they change no view, rows,
selection, search text or active flag, or playback, and schedule no fetch or playback task. In
shade mode the browse list is not in the tree, so both are no-ops; while the `Search library` box
is focused, its `TextInput` keeps `Home` and `End` for editing, as it keeps `Backspace`.

**Approach.**

- `src/ui/message.rs`: add `ScrollBrowseToBottom` beside `ScrollBrowseToTop`, documented as the
  `End` key's counterpart to the Top button: it schedules a snap of the browse list to its last
  row and changes no state.
- `src/ui/handlers/browse.rs`: factor the existing `scroll_browse_to_top`'s
  `iced::widget::operation::snap_to(views::BROWSE_LIST_ID, offset)` call into a private
  `fn snap_browse_list(offset: RelativeOffset) -> Task<Message>`; keep
  `pub(in crate::ui) fn scroll_browse_to_top()` calling it with `RelativeOffset::START`, and add
  `pub(in crate::ui) fn scroll_browse_to_bottom()` calling it with `RelativeOffset::END`. Both
  target the stable `views::BROWSE_LIST_ID` every level's `scrollable_list` carries. Update the
  `scroll_browse_to_top` doc to name the Home binding and the new function's doc to name End, and
  state that neither touches rows, selection, the view, search, or playback.
- `src/ui/mod.rs`: route `Message::ScrollBrowseToBottom` to
  `handlers::browse::scroll_browse_to_bottom()` beside the `ScrollBrowseToTop` arm.
- `src/ui/shortcuts.rs`: in `shortcut`, map `Key::Named(key::Named::Home)` to
  `Message::ScrollBrowseToTop` and `Key::Named(key::Named::End)` to
  `Message::ScrollBrowseToBottom`, beside the `Enter`/`Backspace` arms. The existing
  Control/Alt/Logo guard stays first, so `Ctrl+Home`, `Ctrl+End`, and their Alt/Logo variants
  remain unbound. Update the `message_for` and `shortcut` docs to list Home and End with the other
  browse keys.
- Tests. `src/ui/shortcuts.rs`: add `message_for_maps_home_and_end_to_browse_scroll` covering
  plain and Shift-held Home and End, and extend `shortcut_ignores_chord_modifiers` (or add those
  assertions beside it) so Ctrl/Alt/Logo + Home and Ctrl/Alt/Logo + End return `None`.
  `src/ui/tests.rs`: generalize `assert_task_snaps_browse_list_to_top` over the expected
  `RelativeOffset` (for example a new `assert_task_snaps_browse_list_to(offset, task, what)` with
  the existing top helper delegating to it), add
  `assert_task_snaps_browse_list_to_end`, and add
  `scroll_browse_to_bottom_schedules_a_scroll_task` mirroring
  `scroll_browse_to_top_schedules_a_scroll_task`: drive `Message::ScrollBrowseToBottom` through
  `update`, assert the captured operation snaps `BROWSE_LIST_ID` to `RelativeOffset::END`, and
  assert the current view and shared playback state are unchanged. Optionally cover both messages
  in one test, but pin both offsets.
- `README.md`: in the Usage keyboard sentence, after the Backspace/Back paragraph, say `Home`
  snaps the browse list to its first row and `End` to its last, like `Top`, and that the focused
  `Search library` box keeps both keys for editing its query. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a short `## Browse keyboard scrolling` section after
  `## Browse playing-row visibility` stating that a plain or Shift-held `Home` snaps the showing
  browse list to its first row and `End` to its last row through `snap_to` on `BROWSE_LIST_ID`;
  neither changes the view, rows, selection, search state, or playback, and both are no-ops in
  shade mode where no browse list is built; the focused search box's `TextInput` captures Home and
  End for editing, and Ctrl/Alt/Logo chords stay unbound. Name `shortcuts::shortcut`,
  `Message::ScrollBrowseToBottom`, `handlers::browse::scroll_browse_to_top`/
  `scroll_browse_to_bottom`, and `views::BROWSE_LIST_ID` as the enforcers, add the new test names
  to **Pinned by**, and record the addition in **History**.

**Files touched.** `src/ui/message.rs`, `src/ui/handlers/browse.rs`, `src/ui/mod.rs`,
`src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Plain or Shift-held `Home` snaps the showing browse list to its first row on every normal-mode
  browse level, exactly as the `Top` button does; plain or Shift-held `End` snaps it to its last
  row, and a list shorter than the viewport stays put.
- Neither key changes the current view, rows, selection, typed query, search-active flag, or
  playback, and neither schedules a fetch or playback task; in shade mode both are no-ops, and
  Ctrl/Alt/Logo chords remain unbound.
- While the `Search library` box is focused, Home and End keep editing its query rather than
  scrolling the list, matching Backspace's existing capture.
- The new tests pass, `make check` passes, and README and `docs/invariants.md` describe the keys
  and their bounds.

## Done
