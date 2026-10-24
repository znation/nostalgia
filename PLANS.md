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

### Persist the elapsed/remaining time-display choice across launches (planned 2026-10-23 by the plan role)

**Goal.** The large LCD readout's elapsed/remaining mode already survives Pause, Stop,
Next/Previous, and a new selection, but a restart always brings back elapsed because `Settings`
snapshots only `AppState` while `WinampPlayer::time_display` lives on the player. Save the mode in
the same settings file as volume, balance, Repeat, Shuffle, and the equalizer, and restore it
before the first view, so a user who left the readout counting down gets the same readout next
launch. This is presentation-only and independent of the queued compact-readout toggle: it adds
no message, handler, geometry, or playback path.

**Approach.**

- `src/settings.rs`: add a private `time_display_remaining: bool` field to `Settings` (default
  `false`, i.e. elapsed) and `pub fn time_display_remaining(&self) -> bool` /
  `pub fn set_time_display_remaining(&mut self, remaining: bool)` accessors. Keep `settings.rs`
  independent of the UI module rather than storing `views::TimeDisplay`; the player maps the bool
  at its boundary. Write `time_display=elapsed` or `time_display=remaining` in `to_text`; parse
  only those two words in `from_text` and ignore anything else per key. Update the module and
  struct docs: this is the one persisted value not held in `AppState`, so `from_state` leaves it
  elapsed and the player overwrites it from its own mode before saving.
- `src/main.rs`: keep the loaded snapshot instead of recomputing it from state alone
  (`let loaded = store.load(); ... let saved = loaded.unwrap_or_else(||
  settings::Settings::from_state(&initial_state));`) so the saved time mode reaches `init_ui`;
  document why.
- `src/ui/player.rs`: in `persist_settings_if_changed`, after `Settings::from_state(&state)`, set
  the snapshot's time mode from `self.time_display == views::TimeDisplay::Remaining`. Add
  `pub(in crate::ui) fn adopt_saved_settings(&mut self, saved: Settings)` that sets
  `self.time_display` from `saved.time_display_remaining()` and stores the snapshot in
  `self.saved_settings`; document that the state-backed values were already applied by `main`
  and this restores only the player-owned mode.
- `src/ui/mod.rs`: replace `player.saved_settings = saved.clone()` in `init_ui` with
  `player.adopt_saved_settings(saved.clone())`, and mention the time mode in the `init_ui` doc.
- Tests. `src/settings.rs`: extend `to_text_round_trips_every_field` to set remaining and
  round-trip it; add `time_display_round_trips_both_words_and_ignores_unknown_values`; add
  `time_display=sideways` to `from_text_ignores_malformed_and_unknown_lines`. `src/ui/tests.rs`:
  add `toggle_time_display_reaches_the_saved_snapshot` (attach `MemorySettingsStore`, toggle to
  remaining and back, assert the two saved snapshots' `time_display_remaining()` are true then
  false and an unchanged update saves nothing) and `saved_time_display_mode_is_restored_on_boot`
  (call `adopt_saved_settings` with a remaining snapshot and with the default, then assert
  `player.time_display` and `player.saved_settings`).
- `README.md`: in the settings paragraph at the end of Usage, add the readout's
  elapsed/remaining choice to the saved values and say it is restored at the next launch. Do not
  touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Settings persistence`, add the readout's elapsed/remaining choice
  to the persisted list, state that it is the one value not held in `AppState` (snapshotted from
  `WinampPlayer::time_display` and restored by `WinampPlayer::adopt_saved_settings`), that a
  missing or malformed `time_display` line falls back to elapsed, and name the new tests in
  Pinned by.

**Files touched.** `src/settings.rs`, `src/main.rs`, `src/ui/player.rs`, `src/ui/mod.rs`,
`src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A settings file with `time_display=remaining` restores the remaining readout before the first
  view; `time_display=elapsed`, a missing line, or an unknown value restores elapsed, and
  `to_text` round-trips both words.
- Clicking the LCD readout writes the new mode through `persist_settings_if_changed` exactly
  once per change; an unchanged update writes nothing, and the mode still survives Pause, Stop,
  Next/Previous, and a new selection in memory.
- No new message, handler, playback path, dependency, or window geometry is added; `settings.rs`
  keeps its `AppState`-only `apply_to` and does not import the UI's `TimeDisplay`.
- README and `docs/invariants.md` describe the mode as persisted, and `make check` passes.

## Done
