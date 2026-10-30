# Plans

## Planned

_None yet._

### Delete a named custom equalizer preset from the Save preset panel, like Winamp resets a saved preset (planned 2026-10-29 by the plan role)

**Goal.** The Save preset panel saves the equalizer's applied curve under a name, and saving
over an existing name keeps its row and replaces only the curve — so a preset saved under a
mistyped or obsolete name is stuck forever: the pick list offers it on every launch and the
only removal is hand-editing `eq_custom_preset=` lines. Winamp's saved-preset mechanism can
delete a saved preset; give the Save preset panel the same power. Out of scope: deleting a
built-in (the panel refuses those names already), editing a saved curve outside the save
overwrite, and any change to the AUTO genre rule or `eq_preset=`.

**Approach.** Extend the existing transient Save preset panel — which already pairs `Save`
and `Close` over a name box — with a `Delete` button, and route it through the same
validation-and-note flow `save_preset_submitted` uses in `src/ui/handlers/mixer.rs`.
Infrastructure is all in place: `WinampPlayer.custom_eq_presets` is the stored list,
`persist_settings_if_changed` already snapshots it via `set_eq_custom_presets` (src/ui/player.rs),
and `preset_choices` (src/ui/views/equalizer.rs) rebuilds the pick-list rows from that list
every refresh, so a removed preset's row disappears on the next view refresh with no extra
wiring.

- `src/ui/message.rs`: add `Message::DeletePresetSubmitted` beside `SavePresetSubmitted` with a
  doc comment noting it acts only while the Save preset panel is open and is not persisted, like
  the save-row comment does.
- `src/ui/views/save_preset.rs`: add `pub const NO_SUCH_PRESET_NOTE: &str = "No such preset"`
  beside the other note constants (doc comment: a name no saved custom matches keeps the
  panel open with this note and deletes nothing), and push
  `labeled_button("Delete", Message::DeletePresetSubmitted)` between `Save` and `Close` in
  the panel's button row, updating the view's doc comment to name the three buttons and the
  messages each publishes.
- `src/ui/handlers/mixer.rs`: add `pub(in crate::ui) fn delete_preset_submitted(player) ->
  Task<Message>` beside `save_preset_submitted`, mirroring its shape: a closed panel is a
  no-op; a blank trimmed name shows `Type a name`; a name containing `:` shows `Use it
  without a colon`; a name case-insensitively equal to a [`PRESETS`] built-in shows `Already
  a preset` (built-ins refuse deletion); a name case-insensitively matching a saved custom
  removes exactly that entry in stored order (the shared `iter().find` +
  `Vec::remove` over `player.custom_eq_presets`, keeping every other preset's position),
  clears `save_preset_name` and the note, and closes the panel; a name matching no saved
  custom shows `No such preset` and keeps the panel open. No dedicated write: the shared
  sweep persists the shortened list. Deleting a preset whose curve is applied touches only
  the saved list — `AppState` keeps the applied curve, `preset_pick_list_label` keeps
  reading `Custom`, and AUTO, `eq_preset=`, and the live sliders are untouched — so document
  that in the function's doc comment alongside the note rules.
- `src/ui/mod.rs`: route `Message::DeletePresetSubmitted` to `handlers::mixer::delete_preset_submitted`
  in the `Message::SavePresetSubmitted` arm's neighborhood of `update`.
- `src/ui/views/tests/chrome.rs`: extend the `save_preset_overlay_*` tests: the button row
  now draws `Save`, `Delete`, `Close` in order and the `Delete` click publishes
  `Message::DeletePresetSubmitted` (the backdrop and `Close` keep publishing
  `ToggleSavePreset`).
- `src/ui/tests/mixer_tests.rs`: behavior tests beside `save_preset_submitted_*` —
  `delete_preset_submitted_removes_the_named_custom_preset` (two saved customs, delete the
  first by name any case, the second keeps its stored position); a refused blank, colon,
  built-in, and unknown-name case each keeping the list, panel, and note as the matching
  save-refusal does (unknown shows the new note); `delete_preset_submitted_is_a_no_op_with_the_panel_closed`;
  and a case that deletes the currently applied custom's name and asserts the stored
  `eq_preamp`/`eq_bands` curve is unchanged.
- `docs/usage.md`: extend the Save preset panel paragraph — `Delete` sits between `Save` and
  `Close`; the box's Enter keeps submitting `Save`, and deleting is the button only: typing a
  saved custom's name (any case) and pressing `Delete` removes that custom and its pick-list
  row; an unmatching name shows the new note;
  deleting the applied custom's name changes nothing about the playing curve or the `Custom`
  label, and the shortened list survives relaunches.
- `docs/invariants.md`: extend `## Save preset panel` with the deletion guarantee (which
  note each refusal shows, that a matching custom is removed in place and persisted, and
  that the applied curve is untouched) in the same change as the behavior.
- The panel is Nostalgia's own modal over the existing chrome; keep the button row's
  geometry `labeled_button` already gives `Save`/`Close` and compare against the closest
  equalizer reference (`winamp-screenshots/winamp-2.1-main-eq.gif`) to confirm nothing else
  moved.

**Files touched.** `src/ui/message.rs`, `src/ui/views/save_preset.rs`, `src/ui/handlers/mixer.rs`,
`src/ui/mod.rs`, `src/ui/views/tests/chrome.rs`, `src/ui/tests/mixer_tests.rs`, `docs/usage.md`,
`docs/invariants.md`, plus the `PLANS.md` move. The equalizer's settings file format and
`src/settings/text.rs` parsing need no change.

**Acceptance criteria.**

- `make check` passes.
- Deleting a saved custom by name (any case) leaves the other customs in stored order; the
  deleted preset's pick-list row is gone on the next refresh and the shortened
  `eq_custom_preset=` list is written by the shared sweep.
- Each refusal (empty, colon, built-in, unknown name) shows its note, keeps the panel open,
  and removes nothing, exactly matching the save path's refusal shapes.
- With the panel closed, `DeletePresetSubmitted` does nothing at all.
- Deleting the applied custom's name leaves `eq_preamp`, `eq_bands`, the `Custom` pick-list
  label, `eq_preset=`, and AUTO behavior unchanged.
- `docs/usage.md` and `docs/invariants.md` carry the new behavior in the same change.

## Done
