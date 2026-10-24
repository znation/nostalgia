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

### Remember the visualizer's shown/hidden choice across launches (planned 2026-10-23 by the plan role)

**Goal.** The LCD strip's `V` control already toggles whether the spectrum bars draw, but the
choice resets on every launch: `WinampPlayer::visualizer_visible` lives only on the player, while
`Settings` snapshots `AppState` plus the two UI-owned display values (`time_display` and
`always_on_top`). Save the `V` choice in the same settings file, restore it before the first view,
and leave the stored spectrum and its poll behavior untouched. This is presentation-only and
independent of the queued A/T shortcut plan: it adds no message, handler, shortcut, playback
path, or window geometry, and adds no dependency.

**Approach.**

- `src/settings.rs`: add a private `visualizer_visible: bool` field to `Settings`, defaulting to
  `true` (the player's launch value), with `pub fn visualizer_visible(&self) -> bool` and
  `pub fn set_visualizer_visible(&mut self, visible: bool)` accessors. Write
  `visualizer_visible=true` or `visualizer_visible=false` in `to_text`; parse it in `from_text`
  with the shared `parse_into` helper so any other value leaves the default shown. Keep
  `from_state` leaving it `true` and the player overwriting it from its own flag before saving,
  exactly as `time_display_remaining` and `always_on_top` already work. Update the module and
  struct docs from "two UI-owned values" to three and say a missing or malformed line falls back
  to shown.
- `src/ui/player.rs`: update the `visualizer_visible` field doc to say the choice is saved and
  restored. In `persist_settings_if_changed`, after the snapshot's time and always-on-top values
  are set, add `snapshot.set_visualizer_visible(self.visualizer_visible);`. In
  `adopt_saved_settings`, restore `self.visualizer_visible = saved.visualizer_visible();` and
  update that method's doc from two UI-owned values to three. Correct the `eq_panel_visible`
  field doc, which currently says it is not persisted "like `visualizer_visible`": the EQ panel
  stays deliberately unpersisted, while the visualizer choice now persists.
- `src/settings.rs` tests: extend `to_text_round_trips_every_field` to set the visualizer hidden
  and assert the round trip preserves it; add
  `visualizer_visibility_round_trips_and_ignores_malformed_values` covering the default shown,
  an explicit `false`, and an unknown value falling back to shown; add
  `visualizer_visible=maybe` to `from_text_ignores_malformed_and_unknown_lines`.
- `src/ui/tests.rs`: add `toggle_visualizer_reaches_the_saved_snapshot` with the existing
  `MemorySettingsStore`, asserting one saved snapshot with `visualizer_visible()` false after one
  `Message::ToggleVisualizer`, no save after `Message::Ignored`, and true after a second toggle;
  add `saved_visualizer_visibility_is_restored_on_boot` calling `adopt_saved_settings` with a
  hidden snapshot and with `Settings::default()`, asserting `player.visualizer_visible` false then
  true and `player.saved_settings` follows each snapshot.
- `README.md`: in the final Usage settings paragraph, add the visualizer's shown/hidden choice to
  the values saved and restored at the next launch. Do not touch the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Settings persistence`, add the visualizer's shown/hidden choice to
  the persisted list, change "the two persisted values not held in `AppState`" to three, name
  `WinampPlayer::visualizer_visible` alongside the readout mode and always-on-top flag, state
  that a missing or malformed `visualizer_visible` line falls back to shown, add the two new
  player tests to **Pinned by**, and note the extension in **History**. Keep the existing
  statement that window placement and the EQ panel's visibility are deliberately not persisted.

**Files touched.** `src/settings.rs`, `src/ui/player.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- A settings file with `visualizer_visible=false` restores the hidden visualizer before the
  first view; `visualizer_visible=true`, a missing line, or an unknown value restores the shown
  visualizer, and `to_text` round-trips both booleans.
- Pressing the strip's `V` writes exactly one new snapshot per change through
  `persist_settings_if_changed`; an unchanged update writes nothing, and toggling back restores
  the shown state in the file. Hiding still leaves `WinampPlayer::spectrum` and the spectrum poll
  untouched, so a restored-hidden launch that is shown again draws the current bars.
- No new message, handler, playback path, dependency, or window geometry is added; `settings.rs`
  keeps its `AppState`-only `apply_to` and does not import the UI.
- README and `docs/invariants.md` describe the saved choice, and `make check` passes.

## Done
