# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the usage reference: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. The README's detailed Usage reference moved to
> `docs/usage.md` on 2026-10-26, so plans that change usage prose edit that file, not the README.
> Record any new drift here rather than appending another note.

## Planned

_None yet._

### Implement Winamp's AUTO equalizer toggle that loads a genre-matched preset (planned 2026-10-26 by the plan role)

**Goal.** The equalizer mockups (`docs/screenshots/equalizer.png` and `player.png`) draw an `AUTO`
button beside the `ON` button, and Winamp's AUTO button auto-loads the preset named after the
playing track's primary genre. Add that toggle: while AUTO is on, selecting a song whose genre
matches a preset name applies that preset's curve; a song whose genre matches no preset leaves the
current curve alone, and AUTO off never changes it. The choice is persisted like the other mixer
settings. This changes only which preset is applied when a track is selected — playback, browse,
search, and the audio backend's existing equalizer path are otherwise untouched.

**Approach.**

- `src/equalizer.rs`: add pure `pub fn preset_for_genre(genre: &str) -> Option<Preset>` that trims
  the genre and matches a `PRESETS` name case-insensitively (`eq_ignore_ascii_case`), returning
  `None` for blank or unmatched text. Add a unit test covering an exact match, surrounding
  whitespace and mixed case, a blank genre, and an unmatched genre such as `Alternative`.
- `src/state.rs`: add `pub eq_auto: bool` beside `eq_enabled`, defaulting false, documented as
  whether a genre-matched preset is applied when a track is selected; add
  `pub fn toggle_eq_auto(&mut self)` that flips it, next to `toggle_equalizer`, and a test that it
  flips only that flag.
- `src/settings.rs`: carry `eq_auto` in `Settings` (field, `Default` false, `from_state`,
  `apply_to`). Write it as `eq_auto={}` in `to_text` after `eq_enabled`, parse it in `from_text`, and
  update the struct/default/`apply_to` docs from "the equalizer's on/off state" to include the auto
  choice. Extend `state_with_every_field_set` so the round-trip test covers it, add
  `eq_auto_round_trips_and_ignores_malformed_values` (missing, `true`, explicit `false`, and an
  unknown word fall back/parse like `shade`), and add `eq_auto=maybe` to
  `from_text_ignores_malformed_and_unknown_lines`.
- `src/ui/message.rs`: add `ToggleEqAuto` beside `ToggleEqualizer`, documented as the equalizer
  panel's AUTO button. `src/ui/mod.rs` dispatches it to `handlers::mixer::toggle_eq_auto`, which uses
  `mutate_state(player, AppState::toggle_eq_auto)` and schedules no work (the toggle alone changes
  no gain, so nothing is forwarded to the backend).
- `src/ui/handlers/mixer.rs`: add `pub(in crate::ui) fn apply_auto_preset(player: &WinampPlayer,
  genre: &str) -> Task<Message>`: return no task unless
  `equalizer::preset_for_genre(genre)` matches and `player.state.blocking_lock().eq_auto` holds,
  then apply the preset through the existing `mutate_equalizer` path so the pick list, stored
  curve, and a running preview's filters all update together.
- `src/ui/handlers/playback.rs`: in `track_selected`, after the selected `Song` is resolved and
  before the play request is issued, call `super::mixer::apply_auto_preset(player, &song.genre)`.
  Every route that plays a song goes through `TrackSelected` — browse rows, Open, Jump to File,
  Next/Previous, the edge and ten-song jumps, and the preview-end advance — so one call site covers
  them all. Update its doc comment.
- `src/ui/views/equalizer.rs`: add pure `auto_label(bool) -> &'static str` (`"AUTO: On"` /
  `"AUTO: Off"`); give `view_equalizer` an `auto: bool` parameter and push a
  `fixed_width_button(auto_label(auto), AUTO_BUTTON_WIDTH, Message::ToggleEqAuto)` after the
  on/off button, before the preset pick list, with the existing spacer between buttons. Pin
  `AUTO_BUTTON_WIDTH` wide enough for `AUTO: Off` so the label swap cannot reflow the pick list.
  Update the module and function docs.
- `src/ui/view.rs`: capture `state.eq_auto` with the other equalizer values and pass it to
  `views::view_equalizer`.
- Tests: update the five `view_equalizer` call sites in `src/ui/views/tests/equalizer.rs` for the
  new argument and add `auto_button_draws_its_state_and_publishes_toggle_eq_auto`, laying out the
  panel with AUTO off and on through `layout_capture`, asserting the exact `AUTO: Off`/`AUTO: On`
  labels, and clicking the on-state button to observe `Message::ToggleEqAuto`. In
  `src/ui/tests/mixer_tests.rs` add `toggle_eq_auto_flips_only_the_auto_flag` (no scheduled work)
  and `auto_on_applies_a_genre_matched_preset_on_track_selection`, plus cases showing AUTO off and
  an unmatched genre leave a previously applied Rock preset and its bands unchanged. In
  `src/ui/tests/settings_tests.rs` add `toggle_eq_auto_reaches_the_saved_snapshot` and a boot
  restore assertion through `Settings::apply_to`.
- Docs: `docs/usage.md`'s equalizer paragraph names the `AUTO` toggle and its genre-matching rule,
  and the persisted-settings list includes the auto choice. `docs/invariants.md` gains a
  `## Auto-load presets` section after `## Equalizer scale shortcuts` (guarantee, **Enforced by**
  naming `equalizer::preset_for_genre`, `AppState::eq_auto`, `handlers::mixer::apply_auto_preset`,
  and the `track_selected` call, **Pinned by** the new test names, and a History line); update
  `## Settings persistence` for the new key and appended values, and add a History sentence there.

**Files touched.** `src/equalizer.rs`, `src/state.rs`, `src/settings.rs`, `src/ui/message.rs`,
`src/ui/mod.rs`, `src/ui/handlers/mixer.rs`, `src/ui/handlers/playback.rs`,
`src/ui/views/equalizer.rs`, `src/ui/view.rs`, `src/ui/views/tests/equalizer.rs`,
`src/ui/tests/mixer_tests.rs`, `src/ui/tests/settings_tests.rs`, `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- The equalizer panel draws an `AUTO: Off` toggle while auto-loading is off and `AUTO: On` while it
  is on, beside the existing on/off button; pressing it toggles only `AppState::eq_auto`, schedules
  no work, and leaves the gains, preset, and equalizer on/off state alone.
- While AUTO is on, selecting a song whose trimmed primary genre matches a preset name
  case-insensitively applies that preset's preamp and all ten bands and names it in the preset pick
  list; selecting a song with a blank or unmatched genre leaves the current curve and preset exactly
  as they were.
- While AUTO is off, selecting any song changes no equalizer value.
- The auto choice is written as `eq_auto=true|false`, restored on the next launch, and a missing or
  malformed value falls back to off.
- The applied preset reaches a running preview through the existing equalizer forwarding path, and
  playback, browse, search, panel visibility, and the other settings keep their current behavior.
- The usage reference and the two invariant sections describe the toggle and the new persisted
  choice, and the project's check passes.

## Done
