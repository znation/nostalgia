# Plans

## Planned

_None yet._

### Clear the showing song list, like Winamp's playlist editor "Remove all" (planned 2026-10-29 by the plan role)

**Goal.** Winamp 2.x's playlist editor REM submenu holds two actions, "Remove selected" and
"Remove all"; the planned sibling of this entry adds the first as the songs view's Delete key,
but the showing song list can still not be emptied short of navigating away — Sort, Reverse,
Shuffle, and the Alt+Arrow move can rearrange the composed list and none of them can clear it.
Add Winamp's Remove all as the widening sibling: Ctrl+Shift+Delete clears the shared songs
buffer in place, one-shot like Sort, leaving the library, every other list, the playing track,
the played history, and the saved playlists alone. Out of scope: an on-screen button or a menu
row (the REM submenu's capture is itself a documented gap in `winamp-screenshots/README.md`, so
the binding stays key-only like Ctrl+R and Ctrl+B, which the usage reference documents as its
own bindings), removal on any view other than Songs, and any persistence — a refetch, Back-
satisfied restore, or reopening a list rebuilds the rows. This plan is independent of its
sibling and can land in either order; the exact detail where they meet (the `docs/invariants.md`
section title and Delete's usage-reference row) is spelled out below so either landing works.

**Approach.** The list-composition mutations all live in one module and their handler layer is
two tiny functions routed from the message router, so the change is four one-arm edits beside the
existing siblings plus a `BrowseList` method.

- `src/ui/browse/ordering.rs`: on `BrowseList`, add `pub(in crate::ui) fn
  remove_all_rows(&mut self) -> bool` beside `move_selected` (mirroring the sibling plan's
  `remove_selected_row` in shape), returning whether it removed anything. No-ops (returning
  false, epoch untouched, like `sort_rows_by`'s `reorder_with` guard) when the list is loading
  (`self.loading`), failed (`self.error.is_some()`), or empty. Otherwise: clear `items`,
  `titles`, and `labels` together (the lockstep `clear()`/`fail()` keep), set `selected` to
  `None`, set `library_positions` to `None` — an emptied list is trivially back in library
  order, so no permutation survives to be restored over nothing — set `loading` to false and
  `error` to `None`, and bump `epoch` with `wrapping_add(1)` (exactly what `replace_rows` does
  on every replacement) so a row press rendered before the clear is rejected. The request
  `generation` is left alone, exactly like the reorder one-shots: a later fetch for this level
  keeps its own staleness protection (`clear()` before `begin_fetch` still runs on navigation).
  Document the rules (no-op conditions, buffer lockstep, permutation drop, epoch bump) in its
  doc comment as the invariant-bearing neighbor of `move_selected`'s comment.
- `src/ui/message.rs`: add `Message::RemoveAllBrowse` beside `Message::ReverseBrowse` (doc
  comment in the neighbors' shape: clears the showing list, one-shot like Sort, bumps that
  list's epoch, no-op on an empty, loading, or failed list or on any non-Songs view, leaves the
  other lists, the current view, the search state, playback, the played history, and the saved
  playlists alone; changes no settings or persistence).
- `src/ui/handlers/browse/ordering.rs`: add `pub(in crate::ui) fn remove_all(player: &mut
  WinampPlayer) -> Task<Message>` beside the sibling plan's `remove_selected`: it calls
  `player.songs.remove_all_rows()` only under `CurrentView::Songs` and returns `Task::none()`
  on every other view (the same guard shape Sort's no-ops use), so Artists, Albums, and the
  group views can never be emptied by a reflex.
- `src/ui/mod.rs`: route `Message::RemoveAllBrowse` to it in the update arm beside
  `Message::ReverseBrowse`.
- `src/ui/shortcuts.rs`: in `shortcut()`'s Control block (where Ctrl+R/Ctrl+Shift+R live, under
  the same no-Alt, no-Logo rule), map `Key::Named(key::Named::Delete)` with Control and Shift
  held to `Message::RemoveAllBrowse`; extend `shortcut`'s doc comment's binding list and add
  mapping tests beside the Ctrl+Z/Ctrl+R tests (Ctrl+Shift+Delete mapped; every other
  modifier combination on Delete rejected — plain Delete may carry the sibling plan's
  Remove-selected and is not claimed here).
- `src/ui/tests/`: message-level tests in a new `src/ui/tests/remove_all_tests.rs` matching the
  suite's per-feature file shape (registered beside `sort_tests.rs`), and unit tests in
  `src/ui/browse/ordering.rs`'s `#[cfg(test)] mod tests` beside `move_selected_*`: clearing a
  loaded list empties items, titles, and labels together, drops the selection, drops any
  permutation, bumps the epoch once, and leaves `loading`/`error` false/none; clearing twice
  sees the second press no-op (already empty, epoch untouched); no-ops while loading or failed;
  a non-Songs view (e.g. `CurrentView::Albums`) leaves every list untouched and the songs list
  still clearable once back on Songs; a cleared shared songs buffer leaves playback and the Now
  Playing line alone (`state.current_track` and the `known_tracks`-backed label do not read the
  browse list); and Sort → clear → navigate Back restores the parent without touching the
  cleared songs buffer.
- `docs/usage.md`: add Ctrl+Shift+Delete to the keyboard-binding list in Delete's neighborhood
  (Delete's row may not exist yet — the sibling plan adds it — so anchor the position by the
  Backspace row): "Ctrl+Shift+Delete clears the showing song list (one-shot, like Sort; a
  no-op when the list is empty, loading, or failed, or on any list the shared songs buffer does
  not show, and it does nothing on the other views)".
- `docs/invariants.md`: put the guarantee under `## Browse row removal` (located after `##
  Browse row order`): if the section is absent when this lands, add it, and if the sibling plan
  added it, extend its "Enforced by" list with `BrowseList::remove_all_rows`,
  `browse::ordering::remove_all` routed from `Message::RemoveAllBrowse`, and the
  Ctrl+Shift+Delete arm of `shortcut`, naming the no-ops, the epoch bump, the selection and
  permutation drops, and the list-lockstep requirement.
- No visible chrome, panel, menu, or readout changes — key-only like Ctrl+R and Ctrl+B, so
  nothing to compare against a reference screenshot beyond confirming no chrome moved.

**Files touched.** `src/ui/browse/ordering.rs`, `src/ui/message.rs`,
`src/ui/handlers/browse/ordering.rs`, `src/ui/mod.rs`, `src/ui/shortcuts.rs`,
`src/ui/tests/` (new `remove_all_tests.rs`), `docs/usage.md`, `docs/invariants.md`, plus the
`PLANS.md` move.

**Acceptance criteria.**

- `make check` passes.
- On the Songs view, Ctrl+Shift+Delete empties the showing song list: no rows, no cached titles
  or labels, no selection, no permutation, `Loading…` replaced by the empty-list wording (the
  list is not loading and not failed), and a row press rendered before the clear is rejected
  via the bumped epoch.
- Ctrl+Shift+Delete on an empty, loading, or failed list, or on any non-Songs view (Artists,
  Albums, All Songs, a group, a saved playlist row in the Playlists view) changes no list and
  bumps no epoch, like Sort's no-ops.
- Clearing a list that is currently playing changes nothing about playback, the Now Playing
  line, or the played history — the playing track is not named by any browse-list index.
- Sort or Shuffle → Ctrl+Shift+Delete → navigate Back: the parent's library order is restored
  and the cleared songs buffer stays cleared; no row is resurrected.
- `## Browse row removal` exists in `docs/invariants.md` after `## Browse row order` (added here
  or extended if the sibling plan landed first), and `docs/usage.md` carries the binding.

## Done
