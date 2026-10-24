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

### Focus the library search box with Ctrl+F (planned 2026-10-23 by the plan role)

**Goal.** The `Search library` box has no keyboard route: the user must click it before typing a
query, and the classic keys deliberately leave Control chords to the window manager. Add Ctrl+F
to focus that box from any normal-mode browse level, without changing
the current view, list, selection, typed query, or playback and without submitting a search. This
is a keyboard affordance only; clicking the box keeps working exactly as it does now.

**Approach.**

- `src/ui/views/browse.rs`: add `pub const BROWSE_SEARCH_ID: &str = "nostalgia-search-box";`
  beside `view_search_box`, and give the `TextInput` in `view_search_box` that id
  (`TextInput::new("Search library", query).id(BROWSE_SEARCH_ID)`). The id is static and the
  builder stays pure. Update the builder and module docs to name the id and the Ctrl+F route.
- `src/ui/views/mod.rs`: re-export `BROWSE_SEARCH_ID` with the other browse exports.
- `src/ui/message.rs`: add `Message::FocusSearch` beside the search messages, documented as
  focusing the `Search library` box via `BROWSE_SEARCH_ID`; it changes no browse or playback state
  and submits nothing.
- `src/ui/handlers/browse.rs`: add `pub(in crate::ui) fn focus_search() -> Task<Message>`
  returning `iced::widget::operation::focus(views::BROWSE_SEARCH_ID)`. It schedules only the focus
  operation and mutates no state; in shade mode the search box is not in the tree, so the
  operation has no matching focusable and the key is a no-op.
- `src/ui/mod.rs`: route `Message::FocusSearch` to `handlers::browse::focus_search` beside the
  `SearchChanged`/`SearchSubmitted` arms.
- `src/ui/shortcuts.rs`: in `shortcut`, before the existing Control/Alt/Logo chord guard, map a
  Control-held `f` (ASCII-case-insensitive, with or without Shift) to `Message::FocusSearch`,
  while Alt+F and Logo+F stay unbound and every other Control/Alt/Logo chord keeps returning
  `None`. Update the module and `message_for`/`shortcut` docs to name the one Control exception.
- Tests. `src/ui/shortcuts.rs`: add `message_for_maps_ctrl_f_to_focus_search` covering Ctrl+F and
  Ctrl+Shift+F, and assert plain `f`, Alt+F, and Logo+F remain unbound (extend
  `shortcut_ignores_chord_modifiers` or add those assertions beside it). `src/ui/tests.rs`: add
  `ctrl_f_focuses_the_search_box_and_changes_no_browse_state` — start from an Albums view with a
  non-empty `search_query`, a selected row, and a current track, lay out the root `view`, drive
  `Message::FocusSearch` through `update`, and apply the emitted focus operation to the laid-out
  tree. Then run a custom focusable capture over that tree and assert exactly one focusable
  reports focused, the one tagged `BROWSE_SEARCH_ID`. Also assert the view, list rows, selection,
  `search_query`, `search_active`, and shared playback state are unchanged. Use the custom
  `Operation`/`Focusable` capture shape of
  `captured_root_view_volume_slider` in the same file. `src/ui/views/tests.rs`: add
  `view_search_box_registers_its_focus_id`, laying out `view_search_box` and asserting a
  `BROWSE_SEARCH_ID` focusable is present.
- `README.md`: in the Usage keyboard sentence, after Backspace, add that Ctrl+F focuses the
  `Search library` box without searching or changing the current list, and that the box then takes
  typed keys as usual. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Search focus` section after `## Browse hierarchy` stating that
  Ctrl+F from any normal-mode browse level focuses the `Search library` box, submits nothing, and
  changes no query, view, list, selection, or playback; Alt/Logo+F and other Control chords stay
  unbound, and shade mode has no search box so the key is a no-op. Name `shortcuts::shortcut`,
  `Message::FocusSearch`, `browse::focus_search`, and `views::view_search_box`/
  `BROWSE_SEARCH_ID` as the enforcers, add the new tests to **Pinned by**, and record the addition
  in **History**.

**Files touched.** `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/message.rs`,
`src/ui/handlers/browse.rs`, `src/ui/mod.rs`, `src/ui/shortcuts.rs`, `src/ui/tests.rs`,
`src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Ctrl+F (with or without Shift) from every normal-mode browse level moves keyboard focus to the
  `Search library` box; plain F and every Alt/Logo chord keep their current behavior, other
  Control chords remain unbound, and in shade mode Ctrl+F does nothing.
- Focusing the box changes no browse view, list rows, selection, typed query, search-active flag,
  or playback state, and submits no search; typing in the focused box edits the query exactly as a
  click does.
- The new tests pass, `make check` passes, and README and `docs/invariants.md` describe the key and
  its bounds.

## Done
