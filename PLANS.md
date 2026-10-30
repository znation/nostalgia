# Plans

## Planned

_None yet._

### Move the browse list's selection bar with Ctrl+Up and Ctrl+Down, like Winamp's playlist editor's selection keys (planned 2026-10-30 by the plan role)

**Goal.** The browse list's selection bar moves only by mouse press
(`BrowseList::select_row`, `src/ui/browse.rs`): the plain up/down arrows nudge the volume
(`shortcut()` in `src/ui/shortcuts.rs`, the plain-arrow arms) and the row's classic move keys
take Alt+Up/Down, so the classic playlist editor's way of moving the selection — the arrow keys
on it and on the selection bar — has no keyboard path. The bound chords the open QUESTIONS.md
planner question surfaced for it — `Ctrl+Up`/`Ctrl+Down`, free today: the Ctrl block in
`shortcut()` guards only the letters F/Z/B/R/W/J/A/T/V and Shift+Delete and Ctrl+F1 before the
`if modifiers.control() … return None` gate at its bottom, so Ctrl+ArrowUp/ArrowDown bind
nothing — move the selection bar one row up and down over the showing list, revealing the moved
row like the row moves do, and answer that question's own recommended option 2. The
question's remaining option ordering stays the user's; this entry does not close it.

**Ground truth verified this run (2026-10-30).** The Ctrl block in
`src/ui/shortcuts.rs::shortcut` matches only `Key::Character` arms and
`Key::Named(key::Named::Delete)`/`F1`; a Ctrl + named-arrow press reaches the bottom gate and
yields `None` (grep for `MoveSelectedRowUp` shows it is claimed only under
`Modifiers::ALT` — see the `assert_maps!` test there). The selection itself lives in
`BrowseList::selected` / `select_row` (`src/ui/browse.rs`), the row moves' reveal goes through
`handlers::browse::scroll::reveal_browse_row` (`src/ui/handlers/browse/scroll.rs`), and the
Alt-move handler this plan mirrors is `handlers::browse::ordering::move_selected_row`
(`src/ui/handlers/browse/ordering.rs`), whose `move_in` helper pair defines the no-op rules:
no selection, an edge repeat, or a loading/failed list change nothing and schedule no task.

**Approach.**

1. `src/ui/browse.rs`: beside `select_row`, add
   `pub(super) fn move_selection(&mut self, down: bool) -> Option<usize>` — a load-time-free
   selection step: `None` when the list is loading (`self.loading`), failed (`self.error` is
   set), or nothing is selected; otherwise the row at `index ± 1` clamped to the list's own
   bounds — the selection stays at an edge and `None` is returned when a step would leave it,
   mirroring `move_in`'s no-op rules — and `Some(new_index)` when a row is newly selected. No
   epoch parameter: the move relative to the selection in the showing list is not a stale
   index-carrying press, and `select_row`'s epoch contract stays for the mouse presses.
2. `src/ui/message.rs`: beside `MoveSelectedRowUp`/`MoveSelectedRowDown`, add
   `MoveBrowseSelectionUp` and `MoveBrowseSelectionDown` with the doc comments naming the
   chords and citing this entry.
3. `src/ui/shortcuts.rs`: inside the existing Ctrl block of `shortcut()`, before the bottom
   gate, `Key::Named(key::Named::ArrowUp)` maps to `Message::MoveBrowseSelectionUp` and
   `ArrowDown` to `Message::MoveBrowseSelectionDown`. A Shift held with the chord maps the
   same way: the arrows carry no shift behavior anywhere today. Update `shortcut()`'s
   doc comment (the one that lists every claimed chord and the volume-nudge arrows, and the
   `/// Ctrl+F is the one Control chord…` note when it says so) to name the two new
   chord claims alongside the Ctrl row moves it lists.
4. `src/ui/handlers/browse/selection.rs`: add `move_browse_selection(player, down)` — the
   `match player.current_view` over the eleven views calling
   `move_selection_in(&mut player.…)` exactly as `ordering::move_selected_row` does, and its
   index-mapped-through `scroll::reveal_browse_row` when the selection changed. Route the two
   new messages from the `Message` update arms in `src/ui/mod.rs` beside the
   `Message::MoveSelectedRowUp =>` arm's neighbors.
5. `docs/usage.md`: extend the keyboard paragraph (the one that runs "R toggles Repeat… and
   the up/down arrows nudge the volume while the left/right arrows seek") with the two
   chords and their no-op rules, beside the row moves' sentence if one names them.
6. `docs/invariants.md`: extend the `## Browse selection` guarantee (the section naming
   `BrowseList::select_row` and the blue `theme::ROW_HIGHLIGHT` bar) with the keyboard path
   and pin the new tests below.

**Acceptance criteria.**

- `make test-one TEST=shortcuts_tests` proves the mapping: `assert_maps!`
  `Key::Named(key::Named::ArrowUp)`/`Modifiers::CONTROL` → `Message::MoveBrowseSelectionUp`
  (and the Down twin), the plain arrows still map to `VolumeUp`/`VolumeDown`, Alt+Up still
  maps to `Message::MoveSelectedRowUp`, and Ctrl+Shift+Up/Down map like plain Ctrl ones.
- A `move_browse_selection` test in `src/ui/tests/` (the row-move module
  `move_row_tests.rs` is the closest anchor) drives a loaded list: the first press selects the
  bar (existing test names cover the mouse path), the chords select the neighbor, the reveal
  task is scheduled exactly once per moved selection (the row-move reveal test is the
  anchor), and the no-op rules above (`move_in`'s pair) schedule no scroll task and leave the
  bar unchanged — an edge repeat, a no-selection list, and a loading or failed list included.
- A `move_selection` boundary test in `src/ui/browse.rs`'s test module documents the clamp
  rule itself (at the first/last row a step returns `None` and keeps the selection).
- `make check` passes.

**Size.** One message pair, one matcher arm pair, one handler with an eleven-arm view
match, one `BrowseList::move_selection`, tests, and three doc anchors — a few dozen lines
each, mirroring the landed Alt-move pair's shape. Out of scope: any other chord for the
selection (the question's own recommendation named only Ctrl+Up/Ctrl+Down), any change to
what Enter or the row moves do, and the parked options 3–6 that question still lists.

## Done

Four sub-plans, split 2026-10-30 from the single pixel-font plan after the feature
role's **too large for one run** note (16 view modules plus 17 test modules landing
together was one run's worth of re-derived assertions too many). They cross-reference;
parts 2–4 each scope whole controls — a control's face, size and glyphs never land in
different parts, so nothing is left half-converted between merges.
