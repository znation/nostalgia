# Plans

## Planned

_None yet._

### Move the selected row all the way to the start or end of a showing list, the edge move of Winamp's Alt+Up/Down row move (planned 2026-10-29 by the plan role)

**Goal.** The project's row move (`Alt+Up`/`Alt+Down`, one place per press) exists, but the
corner rows cost one press per row: Winamp's same move with Shift held (Shift+Alt+Up and
Shift+Alt+Down) takes the selection to the list's start or end in one press, and this row move
is the natural widening of the machinery already there. Extend it key-only, mirroring the
one-place move's rules exactly — every view the one-place move works on keeps working, the
selection follows the row the way one step already does, and the reorders' restore-to-library-
order composition keeps behaving. Out of scope: no new on-screen control, no menu row, a
changed scroll policy (the moved row is revealed exactly like a one-place move), and no new
effect on playback, the fetched library, or any other row list.

This plan is independent of the waiting Remove-all plan and can land in either order. The only
shared file with it is `src/ui/shortcuts.rs`, touched in different places (here inside
`move_row_shortcut`, there in `shortcut()`'s Control block), and `docs/invariants.md` is touched
in different sections (here `## Browse row order`, there `## Browse row removal`); both plans
are worded so either landing works.

**Approach.** The move machinery already composes two adjacent steps' swaps in one call path;
this change reaches the edge by repeating that pattern over the stretch the press must cross —
no storage shape changes, and the cached-titles-and-labels / library-permutation lockstep the
one-place move pins is kept by construction.

- `src/ui/browse/ordering.rs`: on `BrowseList`, add `pub(in crate::ui) fn
  move_selected_to_edge(&mut self, bottom: bool)` beside `move_selected`, in the same
  swap-in-place style and with the same one-shot rules: no-ops — returning without touching
  anything, epoch included — on no selection, on a loading or failed list, on a selection
  outside `0..self.items.len()`, or when the selected row already sits at the target edge
  (`from == 0` for top, `from == self.items.len() - 1` for bottom), which also makes a held-key
  repeat after the first press a no-op. Otherwise: let `to = 0` (or `self.items.len() - 1` when
  `bottom`), walk `from` → `to` one place at a time with `move_selected`'s exact swap pattern —
  swap `items[cur]`/`items[next]`, swap the same index pair in `titles` and `labels` when
  they are populated, and swap the same pair in `library_positions` (allocating the identity
  permutation once before the loop when it is `None`, exactly like `move_selected`'s on-
  demand allocation) — then set `selected` to `to` and bump `epoch` with `wrapping_add(1)`.
  Document the no-op conditions, the selection-follows-edge behavior, and the epoch bump in a
  doc comment beside `move_selected`'s, and note why the per-press cost is bounded: the loop is
  O(rows crossed), a one-shot press that a redo at the edge turns into a no-op, not a held-key
  hot loop like the one-place move's O(1)-per-repeat contract.
- `src/ui/message.rs`: add `Message::MoveSelectedRowToTop` and `Message::MoveSelectedRowToBottom`
  beside `Message::MoveSelectedRowUp`/`Down`, with doc comments in the neighbors' shape
  (Alt+Shift+Up: like `MoveSelectedRowUp`, all the way to the first row; Alt+Shift+Down: like
  `MoveSelectedRowDown`, all the way to the last row; same no-ops, one-shot, local to the
  showing list, changes no settings or persistence).
- `src/ui/handlers/browse/ordering.rs`: beside `move_selected_row` (which routes the one-place
  move across every view) add `pub(in crate::ui) fn move_selected_row_to_edge(player: &mut
  WinampPlayer, bottom: bool) -> Task<Message>`, with the same full-view-match body routed
  through a new `move_edge_in` helper that mirrors `move_in`'s before/after snapshot of
  `list.selected()` and feeds a changed index to `scroll::reveal_browse_row` — the reveal,
  not a full-page scroll, like the one-place move.
- `src/ui/mod.rs`: route `Message::MoveSelectedRowToTop` → `move_selected_row_to_edge(player,
  false)` and `Message::MoveSelectedRowToBottom` → `move_selected_row_to_edge(player, true)`
  beside the existing `MoveSelectedRowUp`/`Down` arms.
- `src/ui/shortcuts.rs`: restructure `move_row_shortcut` — today's guard rejects Shift
  outright — to require Alt held with Control and Logo released (Shift no longer an early
  return), then match Shift as part of the arm: `Shift+Alt+Up` → `MoveSelectedRowToTop`,
  `Shift+Alt+Down` → `MoveSelectedRowToBottom`, plain-Shift `Alt+Up`/`Down` unchanged. Extend
  the module doc comment's binding list beside the Alt+Up/Down line, and add mapping tests
  beside the existing move-row tests: Alt+Shift+Up/Down mapped; Ctrl+Alt (with and without
  Shift) and Alt+Logo rejected as OS chords; `Shift+Alt+Left/Right` still unbound so a plain
  Shift+arrow keeps the volume/seek arms unclaimed.
- `src/ui/tests/move_row_tests.rs`: extend the message-level move-row file (already registered
  in `src/ui/tests/mod.rs`) with edge-move tests beside the one-place ones; unit tests for
  `move_selected_to_edge` in `src/ui/browse/ordering.rs`'s `#[cfg(test)] mod tests` beside
  `move_selected`'s: selecting a row in a stored song list and moving it to each edge shifts
  `items`, populated `titles`/`labels`, and `library_positions` in step and leaves the
  selection on the moved row at the edge; moving to the edge an already-edge-selected row
  (top-selected with `bottom: false`, bottom-selected with `bottom: true`), a list with no
  selection, and a loading or failed list changes nothing and bumps no epoch; a held-repeat
  second press after the first no-ops; `sort_rows_by` → `move_selected_to_edge` →
  `restore_library_order` returns every row to library order, with no row lost or duplicated
  and the permutation still in the state one more move composes with; and the handler's view
  routing (message-level) leaves every list untouched on a view the one-place move's arms do
  not reach.
- `docs/usage.md`: in the row-move sentence (the `Alt+Up and Alt+Down move the selected row…`
  paragraph), add the Shift variant: "Alt+Shift+Up and Alt+Shift+Down move the selected row
  all the way to the first or last row of the showing list, with the same no-op rules
  (nothing selected, already at that edge, or a loading or failed list)."
- `docs/invariants.md`: extend the `## Browse row order` section (after its one-place-move
  guarantee) with the edge move: `BrowseList::move_selected_to_edge` composes with the same
  restores Sort/Reverse/Shuffle use, the selection following the row to the edge, the
  lockstep between rows and saved library permutation, the one-press one-epoch bump, and the
  no-ops (nothing selected, edge already, loading, failed), citing `move_row_tests.rs` and the
  ordering.rs unit tests.
- No chrome, panel, menu, or readout changes — key-only like the one-place move, so nothing to
  compare against a reference screenshot beyond confirming no chrome moved.

**Files touched.** `src/ui/browse/ordering.rs`, `src/ui/message.rs`,
`src/ui/handlers/browse/ordering.rs`, `src/ui/mod.rs`, `src/ui/shortcuts.rs`,
`src/ui/tests/move_row_tests.rs`, `docs/usage.md`, `docs/invariants.md`, plus the `PLANS.md`
move.

**Acceptance criteria.**

- `make check` passes.
- On any view whose showing list has rows (the one-place move's full set of views), a selected
  row moved to the top or bottom with Alt+Shift+Up/Down shifts items, cached titles and labels
  (where populated), and the library permutation in step, leaves the selection on the moved
  row at the edge, and bumps the epoch once.
- Alt+Shift+Up/Down on a list with no selection, with the selection already at the target
  edge, loading, or failed, changes nothing and bumps no epoch; held-key repeat after the
  first press is a no-op at the edge.
- Sort or another reorder → edge move → navigate Back restores the parent's library order and
  a later `restore_library_order` returns every moved row to library order with nothing lost
  or duplicated.
- Music keeps playing and the fetched library, saved playlists, and search state are untouched
  by an edge move; the moved row is revealed (not full-page scrolled) like a one-place move.
- The move-row keyboard gate keeps Ctrl+Alt and Alt+Logo arrow chords unbound (OS chord
  territory) and Shift+Alt+Left/Right unbound, exactly as today's shortcut tests pin.
- `docs/invariants.md`'s `## Browse row order` section names the edge move, and
  `docs/usage.md` carries the Alt+Shift+Up/Down binding.

## Done
