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

### Remember always-on-top across launches (planned 2026-10-23 by the plan role)

**Goal.** The title bar's `A` and the LCD strip's `A` toggle `WinampPlayer::always_on_top` for the
current run only, so a user who pins the window must pin it again after every launch. Persist that
choice beside the mixer and equalizer settings, restore the flag before the first view, and apply
the saved window level once boot's `WindowIdResolved` supplies the id, so a saved-on launch starts
pinned. A missing, malformed, or false value keeps today's unpinned default. This is independent
of the queued window-title plan: it adds no title change, message, handler for playback, or
window geometry.

**Approach.**

- `src/settings.rs`: add a private `always_on_top: bool` to `Settings` beside
  `time_display_remaining`, defaulting to `false`; add `#[must_use] pub fn always_on_top(&self) ->
  bool` and `pub fn set_always_on_top(&mut self, always_on_top: bool)`, mirroring the readout
  mode's accessors. Write it as `always_on_top=true`/`false` in `to_text`, parse that key with
  `parse_into` in `from_text` (an unknown or malformed value keeps the default), and leave it
  untouched in `apply_to` and at `false` in `from_state`, like `time_display_remaining`, because
  the player owns it. Update the module, struct, `Default`, and `from_state` docs to say two
  UI-owned values are copied in by the player instead of one.
- `src/ui/player.rs`: in `persist_settings_if_changed`, after
  `snapshot.set_time_display_remaining(...)`, call
  `snapshot.set_always_on_top(self.always_on_top)`. In `adopt_saved_settings`, set
  `self.always_on_top = saved.always_on_top()` before storing `saved_settings`, and update its doc
  to name both player-owned values it restores.
- `src/ui/handlers/window.rs`: add a pure
  `pub(in crate::ui) fn window_level(always_on_top: bool) -> iced::window::Level` returning
  `AlwaysOnTop` or `Normal`, and use it in `toggle_always_on_top` instead of the inline match. In
  `window_id_resolved`, store the id as today, then, when the id is `Some` and
  `player.always_on_top` holds, return
  `iced::window::set_level(id, window_level(true))`; otherwise return `Task::none()`. Document
  that the saved level waits for the same id every other window action uses, so no `.window`
  settings or window geometry changes are involved.
- `src/settings.rs` tests: extend `to_text_round_trips_every_field` to set
  `always_on_top` true and assert it survives the round trip, and add
  `always_on_top_round_trips_and_ignores_malformed_values` covering `true`, `false`, and an
  unknown word falling back to `false`; add an `always_on_top=yes` line to
  `from_text_ignores_malformed_and_unknown_lines`'s text.
- `src/ui/tests.rs`: add `toggle_always_on_top_reaches_the_saved_snapshot` (attach a
  `MemorySettingsStore`, drive `Message::ToggleAlwaysOnTop`, assert one snapshot with
  `always_on_top()` true, then a second toggle with it false, and that `Message::Ignored` adds no
  snapshot); `saved_always_on_top_is_restored_on_boot` (adopt a snapshot with the flag set and
  assert `player.always_on_top` and `saved_settings`; a default snapshot clears it); and
  `window_id_resolved_applies_a_saved_always_on_top_level` (with the flag set, resolve an id and
  assert the id is stored and work is scheduled; with it clear, resolve an id and assert no work
  is scheduled). Add `window_level_selects_always_on_top` beside the existing toggle tests.
- `README.md`: add always-on-top to the saved-values sentence beside volume, balance, Repeat,
  Shuffle, the equalizer, and the readout mode, saying the title bar's `A` choice is remembered
  for the next launch. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Settings persistence`, add always-on-top to the persisted-values
  guarantee, state that it lives on `WinampPlayer::always_on_top`, that `adopt_saved_settings`
  restores it before the first view, that `window_id_resolved` applies its level once the id
  exists, and that it is still not window placement. Replace "the one persisted value not held in
  `AppState`" with the readout mode and this window flag, and name the new tests in `Pinned by`.

**Files touched.** `src/settings.rs`, `src/ui/player.rs`, `src/ui/handlers/window.rs`,
`src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- With `always_on_top=true` in the settings file, the restored player flag is set before the first
  view and boot's `WindowIdResolved` schedules `iced::window::set_level` with
  `Level::AlwaysOnTop`; a false, missing, or malformed value leaves the window at the existing
  unpinned default and schedules no level change.
- Pressing `A` in the title bar or LCD strip persists exactly one new snapshot with the new
  value, and an update that changes nothing else writes nothing; the existing settings
  round-trip, clamping, and fallback behavior is unchanged.
- `apply_to` still touches only `AppState`-backed values, so adopting a snapshot does not change
  playback, mixer, equalizer, or geometry behavior.
- README and `docs/invariants.md` describe the persisted choice, and `make check` passes.

## Done
