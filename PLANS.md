# Plans

## Planned

_None yet._

### Delete a saved song list from the Save list panel, like the Save preset panel deletes a saved preset (planned 2026-10-29 by the plan role)

**Goal.** The browse `Save list` panel saves the showing song list under a name, and saving over
an existing name keeps one list and replaces only its ids — so a list saved under a mistyped or
obsolete name is stuck forever: the Playlists view offers it on every launch and the only removal
is hand-editing `user_playlist=` lines. The equalizer's Save preset panel just gained `Delete`
(PLANS-DONE.md 2026-10-29); give the Save list panel the same power, so a saved song list can be
removed exactly the pear-shaped-preset one can. Out of scope: editing a saved list's ids outside
the save overwrite, renaming a saved list, deleting a saved list from the Playlists view itself,
refetching the showing browse level on delete, and any change to the `user_playlist=` codec —
deletion is in-memory removal plus the shared sweep, like the preset deletion.

**Approach.** Extend the existing transient Save list panel — which already pairs `Save` and
`Close` over a name box (`src/ui/views/save_list.rs`) — with a `Delete` button, and route it
through the same validation-and-note flow `save_list_submitted` uses in
`src/ui/handlers/save_list.rs`. Infrastructure is all in place: `WinampPlayer.saved_playlists`
is the stored list, `persist_settings_if_changed` already snapshots it via `set_saved_playlists`
(src/ui/player.rs), and the Playlists level's rows build from `saved_playlists` in
`handlers/browse/fetch.rs`'s playlists arm at every fetch, so a removed list's row disappears
the next time that level is fetched with no extra wiring.

- `src/ui/message.rs`: add `Message::DeleteListSubmitted` beside `SaveListSubmitted` with a doc
  comment noting it acts only while the Save list panel is open and is not persisted, like the
  save-row comment does.
- `src/ui/views/save_list.rs`: add `pub const SAVE_LIST_NO_SUCH_NOTE: &str = "No such playlist"`
  beside the other note constants (doc comment: a name no saved list matches keeps the panel
  open with this note and deletes nothing), and push
  `labeled_button("Delete", Message::DeleteListSubmitted)` between `Save` and `Close` in the
  panel's button row, updating the view's doc comment to name the three buttons and the messages
  each publishes.
- `src/ui/handlers/save_list.rs`: add `pub(in crate::ui) fn delete_list_submitted(player) ->
  Task<Message>` beside `save_list_submitted`, mirroring its shape: a closed panel is a no-op; a
  blank trimmed name shows `Type a name`; a name containing `:` shows `Use it without a colon`;
  a name case-insensitively matching a saved list removes exactly that entry in stored order
  (the shared `iter().position` + `Vec::remove` over `player.saved_playlists`, keeping every
  other list's position), clears `save_list_name` and the note, and closes the panel; a name
  matching no saved list shows `No such playlist` and keeps the panel open. No dedicated write:
  the shared sweep persists the shortened list. Deleting a list whose songs are currently
  showing touches only the saved list — the showing rows, the playing track, and the transport
  are untouched, like a removed preset's applied curve — so document that in the function's doc
  comment alongside the note rules.
- `src/ui/mod.rs`: route `Message::DeleteListSubmitted` to
  `handlers::save_list::delete_list_submitted` in the `Message::SaveListSubmitted` arm's
  neighborhood of `update`.
- `src/ui/views/tests/chrome.rs`: extend the `save_list_overlay_*` tests: the button row now
  draws `Save`, `Delete`, `Close` in order and the `Delete` click publishes
  `Message::DeleteListSubmitted` (the backdrop and `Close` keep publishing `ToggleSaveList`).
- `src/ui/tests/save_list_tests.rs`: behavior tests beside `save_list_submitted_*` —
  `delete_list_submitted_removes_the_named_saved_list` (two saved lists, delete the first by
  name any case, the second keeps its stored position and its ids); a refused blank, colon, and
  unknown-name case each keeping the list, panel open, and the typed name (blank shows
  `Type a name`, colon `Use it without a colon`, unknown the new note);
  `delete_list_submitted_is_a_no_op_with_the_panel_closed`; and a case that deletes the saved
  list whose songs are currently showing and asserts `player.songs` still holds them.
- `docs/usage.md`: extend the Save list paragraph — `Delete` sits between `Save` and `Close`; the
  box's Enter keeps submitting `Save`, and deleting is the button only: typing a saved list's
  name (any case) and pressing `Delete` removes that list and its Playlists-view row; an
  unmatching name shows the new note; deleting a list whose songs are showing changes nothing
  about the showing rows or playback, and the shortened list survives relaunches.
- `docs/invariants.md`: extend `## Saved playlists` with the deletion guarantee (which note each
  refusal shows, that a matching list is removed in place and persisted, and that the showing
  rows are untouched) in the same change as the behavior.
- The panel is Nostalgia's own modal over the existing chrome; keep the button row's geometry
  `labeled_button` already gives `Save`/`Close`, so the two naming panels look the same, and
  compare the widened `Save`/`Delete`/`Close` row against the Save preset panel's,
  `docs/screenshots/hero.png` being the closest in-repo reference.

**Files touched.** `src/ui/message.rs`, `src/ui/views/save_list.rs`, `src/ui/handlers/save_list.rs`,
`src/ui/mod.rs`, `src/ui/views/tests/chrome.rs`, `src/ui/tests/save_list_tests.rs`, `docs/usage.md`,
`docs/invariants.md`, plus the `PLANS.md` move.

**Acceptance criteria.**

- `make check` passes.
- With the Save list panel open, a name case-insensitively matching a saved list removes exactly
  that list in stored order, closes the panel, and the next settings snapshot has one `user_playlist=`
  line fewer; the other lists keep their stored order.
- A blank, colon-bearing, or unmatching name shows the same notes the save flow refuses with
  (`Type a name`, `Use it without a colon`, or `No such playlist`), keeps the panel open and
  the typed name, and removes nothing.
- A panel closed, backdrop, or `Close` press deletes nothing; deleting the showing saved list's
  name leaves the showing rows and playback untouched.
- The `user_playlist=` codec, `SavedPlaylist`'s shape, and the saved-list open path
  (`user_playlist_selected`) are untouched.
- `docs/usage.md` and `docs/invariants.md` carry the new behavior in the same change.

## Done
