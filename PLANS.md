# Plans

## Planned

_None yet._

### Remove the selected song row from the showing list, like Winamp's playlist editor "Remove selected" (planned 2026-10-29 by the plan role)

**Goal.** Winamp 2.x's playlist editor exposes "Remove selected" through its context menu
(winamp-screenshots/README.md notes no capture of that menu exists), and Nostalgia's songs view
is the playlist-shaped artifact the landed stream already treats as one: Sort/Reverse/Ctrl+Shift+R
reorder it, the Save-list feature saves and reopens it, and saved-playlist deletion completed
this round. What it cannot do yet is drop a row the user does not want in the composed list; a
song can leave it today only by refetch, Sort can move but not remove, and the library keeps
every unwanted track. Add Winamp's Remove-selected as the playlist editor's Delete binding: the
deletion is local to the shared songs buffer, one-shot like Sort, and leaves everything else —
the library, the playing track, the played history, the saved playlists — alone. Out of scope: a
playlist context menu, an on-screen button, removing rows on the Artists/Albums/group views
(the entry is a no-op there, like Sort on a view outside the songs buffer), and any persistence
of the deletion beyond the showing list (a refetch or reopening the list rebuilds it). No new
keyboard shortcut style: Delete rides the identical single mapper gate Backspace already rides
(`shortcut()` in `src/ui/shortcuts.rs`), so the plan adds no search-box or focus rule.

**Approach.** The row-order edits live in one module (`src/ui/browse/ordering.rs`'s
`sort_browse`/`reverse_browse`/`randomize_browse` and `BrowseList`'s reorder helpers), routed from
their `Message` arms beside `Message::ReverseBrowse` in `src/ui/mod.rs`, with the key mapping in
`shortcut()`. A removal is the one destructive sibling of that family, and its correctness lives
entirely in the `BrowseList` mutation: drop the selected row, drop its cached title and label,
and keep `library_positions` a gap-free permutation so a later `restore_library_order` restores
library order *minus* the deleted row instead of collapsing (its inverse builds
`inverse[library] = current`, so a gap like `positions = [0, 2, 3]` would already corrupt at
entry 1 — the plan's core requirement is the decrement pass). The selection drops, like every
other transition that invalidates what an index names.

- `src/ui/browse/ordering.rs`: on `BrowseList`, add `pub(in crate::ui) fn
  remove_selected_row(&mut self) -> bool` beside `move_selected`, returning whether it removed
  anything. No-ops (returning false, epoch untouched, like `move_selected`'s no-ops) when the
  list is loading (`self.loading`), failed (`self.error.is_some()`), empty, or OOB: with no
  selection. Otherwise: remove the selected row from `items` (the index is always in range —
  `select_row` rejects out-of-range presses); when `titles`/`labels` cache text (their length
  equals `items`' length before the removal) drop the same index from them too, keeping the
  three buffers in lockstep as `clear`/`fail`/`replace_rows` do; on `library_positions`, when
  `Some(positions)`: remove entry `i`, note its library index `p`, decrement every remaining
  entry greater than `p`, so the permutation stays a bijection over `0..items.len()` for
  `restore_library_order`; when `None`, the rows were still in library order, so leave `None`
  (no materialization needed); set `selected: None`; bump `epoch` with `wrapping_add(1)` so a
  press rendered before the removal is rejected, exactly like Sort's bump. Document the rules
  (selection drop, permutation adjustment, no-op conditions, epoch bump) in its doc comment as
  the invariant-bearing neighbor of `move_selected`'s comment.
- `src/ui/message.rs`: add `Message::RemoveSelectedBrowse` beside `Message::ReverseBrowse`
  (doc comment in the neighbors' shape: deletes the showing list's selected row, one-shot like
  Sort, bumps that list's epoch, no-op with no selection or on any non-Songs view or a loading
  or failed list, leaves the other lists, the current view, the search state, playback, the
  played history, and the saved playlists alone; changes no settings or persistence).
- `src/ui/handlers/browse/ordering.rs`: add `pub(in crate::ui) fn
  remove_selected(player) -> Task<Message>` beside `reverse_browse`: it calls
  `player.songs.remove_selected_row()` only under `CurrentView::Songs` and returns
  `Task::none()` on every other view (the shared-songs-buffer guard the Save-list button and
  Sort's no-ops use).
- `src/ui/mod.rs`: route `Message::RemoveSelectedBrowse` to it in the update arm beside
  `Message::ReverseBrowse`.
- `src/ui/shortcuts.rs`: in `shortcut()`, map a plain `Key::Named(key::Named::Delete)` press —
  no Control, Alt, Logo, or Shift — to `Message::RemoveSelectedBrowse`, next to the Backspace
  mapping; extend `shortcut`'s doc comment's binding list and add the mapping test beside the
  Ctrl+Z/Ctrl+B/Backspace tests (Delete mapped, every chord modifier rejected).
- `src/ui/tests/sort_tests.rs` (or a neighboring `src/ui/tests/removal_tests.rs` matching the
  suite's per-feature file shape): message-level tests beside `sort_browse`'s cases — a
  non-Songs view (e.g. `CurrentView::Albums`) leaves every list untouched and the songs list
  still removable once back on Songs; deleting a playing row leaves playback and the Now Playing
  line alone (`state.current_track` and the `known_tracks`-backed label survive the removal); a
  deletion composes with `restore_library_order` (sort, delete, Back's restore proves the
  permutation did not collapse); and the Jump to File panel's follow-up is the existing behavior
  (a removed song's highlight resolves to `None` through
  `jump_file_highlighted`'s `matches.iter().position`, already verified).
- `src/ui/browse/ordering.rs`'s `#[cfg(test)] mod tests`: unit tests beside
  `move_selected_*` — removal drops the selection and bumps the epoch; cached titles and labels
  stay aligned; the permutation adjustment (delete from a sorted list, `restore_library_order`
  returns library order minus the deleted row); and the no-op conditions.
- `docs/usage.md`: add Delete to the keyboard-binding list beside Backspace — "Delete removes
  the selected song row from the showing list (one-shot, like Sort; a no-op with no selection or
  on any list the shared songs buffer does not show)" — and note in the songs-view paragraph
  that the deletion touches only the showing list and refetching restores it.
- `docs/invariants.md`: add a `## Browse row removal` section after `## Browse row order` with
  the guarantee and its "Enforced by" list (`BrowseList::remove_selected_row`,
  `browse::ordering::remove_selected` routed from `Message::RemoveSelectedBrowse`, and the
  Delete arm of `shortcut`), naming the no-ops, the epoch bump, the selection drop, and the
  permutation requirement.
- No visible chrome, panel, menu, or readout changes — nothing to compare against a reference
  screenshot beyond confirming no chrome moved (the relevant Winamp capture, the playlist
  context menu, is itself a documented gap in `winamp-screenshots/README.md`).

**Files touched.** `src/ui/browse/ordering.rs`, `src/ui/message.rs`,
`src/ui/handlers/browse/ordering.rs`, `src/ui/mod.rs`, `src/ui/shortcuts.rs`,
`src/ui/tests/sort_tests.rs` (or `src/ui/tests/removal_tests.rs`), `docs/usage.md`,
`docs/invariants.md`, plus the `PLANS.md` move.

**Acceptance criteria.**

- `make check` passes.
- On the Songs view, Delete (plain, no modifiers) removes the selected song row; the selection
  clears, the row press rendered before the removal is rejected via the bumped epoch, and the
  list reads its post-removal rows.
- A Delete with no selected row, on a loading or failed list, or on any non-Songs view changes
  no list and bumps no epoch, like Sort's no-ops.
- Deleting a row that is currently playing changes nothing about playback, the Now Playing line,
  or the played history — the playing track is not named by any browse-list index.
- Sort → Delete → navigate Back compares against library order minus the deleted row: no row is
  resurrected and no permutation panics or collapses.
- The other lists, the current view, the search state, the saved playlists, and the settings
  file are untouched; nothing persists — a refetch or reopening the list rebuilds the rows.
- `docs/usage.md` and `docs/invariants.md` carry the behavior in the same change.

## Done
