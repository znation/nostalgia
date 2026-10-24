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

_None yet._

### Remember the EQ and library panels' shown/hidden choice across launches (planned 2026-10-24 by the plan role)

**Goal.** Classic Winamp reopens with the equalizer and playlist windows shown or hidden as the
user last left them. Nostalgia's `EQ` button (and the `E` key) and `PL` button already toggle
`WinampPlayer::eq_panel_visible` and `WinampPlayer::library_panel_visible`, but both reset to open
on every launch. Save both flags in the same settings file and restore them before the first view,
the same way the visualizer's `V` choice is already saved. This reverses the earlier scope choice
that left the two layout flags unpersisted. The work stays presentation-only: it adds no message,
handler, shortcut, playback path, dependency, or window geometry. It does not overlap the waiting
EQ drag readout plan, which touches `views::equalizer` and `now_playing/mixer_readout.rs`.

**Approach.**

- `src/settings.rs`: add private `eq_panel_visible: bool` and `library_panel_visible: bool`
  fields to `Settings`, both defaulting to `true` in `Default` and `from_state` (the player's
  launch values). Add `pub fn eq_panel_visible(&self) -> bool`, `pub fn set_eq_panel_visible(&mut
  self, visible: bool)`, `pub fn library_panel_visible(&self) -> bool`, and `pub fn
  set_library_panel_visible(&mut self, visible: bool)`, documented like the
  `visualizer_visible` pair. Write `eq_panel_visible=<bool>` and `library_panel_visible=<bool>`
  in `to_text`, after `visualizer_visible`. Parse both in `from_text` with `parse_into`, so a
  malformed value leaves the default shown. Update the struct doc, which lists the UI-owned
  values and says what `from_state` leaves them as, and the `Default` doc, to name the two panel
  flags.
- `src/ui/player.rs`: in `persist_settings_if_changed`, after `set_visualizer_visible`, add
  `snapshot.set_eq_panel_visible(self.eq_panel_visible);` and
  `snapshot.set_library_panel_visible(self.library_panel_visible);`. In `adopt_saved_settings`,
  restore both fields from `saved`. Update both methods' docs from three UI-owned values to five,
  and rewrite the `eq_panel_visible` and `library_panel_visible` field docs to say each flag is
  open by default and saved and restored by those two methods. They are no longer "deliberately
  not persisted".
- `src/settings.rs` tests: in `to_text_round_trips_every_field`, hide both panels and assert the
  round trip keeps them hidden. Add `panel_visibility_round_trips_and_ignores_malformed_values`,
  modeled on `visualizer_visibility_round_trips_and_ignores_malformed_values`: both default to
  shown, an explicit `false` for each round-trips independently, and an unknown value such as
  `eq_panel_visible=maybe` falls back to shown. Add `eq_panel_visible=maybe` and
  `library_panel_visible=maybe` to `from_text_ignores_malformed_and_unknown_lines`.
- `src/ui/tests.rs`: beside `toggle_visualizer_reaches_the_saved_snapshot`, add
  `toggle_panels_reach_the_saved_snapshot`. Attach a `MemorySettingsStore`, then send
  `Message::ToggleEqualizerPanel` and assert one snapshot with `eq_panel_visible()` false and
  `library_panel_visible()` true. Send `Message::Ignored` and assert no new snapshot. Send
  `Message::ToggleLibraryPanel` and assert a second snapshot with both false. Beside
  `saved_visualizer_visibility_is_restored_on_boot`, add
  `saved_panel_visibility_is_restored_on_boot`. It calls `adopt_saved_settings` with a snapshot
  hiding both panels and then with `Settings::default()`, and asserts the two player flags and
  `player.saved_settings` follow each snapshot. Also add `shaded_eq_toggle_saves_nothing`: with a
  store attached and `player.shaded = true`, `Message::ToggleEqualizerPanel` writes no snapshot,
  because `handlers::mixer::toggle_equalizer_panel` leaves the flag alone in shade mode.
- `README.md`: in the final Usage settings paragraph, add the equalizer and library panels'
  shown/hidden choices to the saved values. In the `EQ`/`PL` sentences, change "which is open at
  launch" and "the library is open at launch" to say each panel is open on a first launch and
  then reopens as last left. Leave the `<!-- tumwater:status:start -->` block alone.
- `docs/invariants.md`: in `## Settings persistence`, add both panel choices to the guarantee's
  list. Change "the three persisted values not held in `AppState`" to five, and name
  `WinampPlayer::eq_panel_visible` and `WinampPlayer::library_panel_visible` there and in
  **Enforced by**. State that a missing or malformed line for either flag falls back to shown.
  Add the new tests to **Pinned by** and note the extension in **History**. In `## UI layout`'s
  `PL` bullet, replace "the flag is not persisted" with a reference to `## Settings persistence`.

**Files touched.** `src/settings.rs`, `src/ui/player.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- A settings file with `eq_panel_visible=false` launches with the `NOSTALGIA EQUALIZER` band and
  equalizer panel hidden, and the `EQ` button is drawn unsunken. A file with
  `library_panel_visible=false` launches with the library section hidden in the same way. A
  `true` value, a missing line, or an unknown value for either key launches that panel shown.
  `to_text` round-trips both booleans independently.
- Each `EQ`, `E`, or `PL` toggle that changes a flag writes exactly one new snapshot through
  `persist_settings_if_changed`. An unchanged update writes nothing, and so does the shaded `EQ`
  toggle, which leaves the flag alone.
- Restoring a hidden equalizer panel leaves the EQ's on/off state, curve, preset, and audio
  forwarding exactly as `Settings::apply_to` restored them. Restoring either hidden panel changes
  no browse, search, selection, or window-size state.
- `settings.rs` keeps its `AppState`-only `apply_to` and still does not import the UI.
- README and `docs/invariants.md` describe the saved choices, and `make check` passes.

## Done
