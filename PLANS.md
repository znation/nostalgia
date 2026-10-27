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

### Add Winamp's options menu from the LCD's `O` cell and the transport row (planned 2026-10-26 by the plan role)

**Goal.** The mockups draw the LCD's classic `O` indicator cell and a lightning menu button at the
end of the transport row, but `lcd_indicator_message` leaves `O` inert and the transport row has no
menu button. Make both open a Winamp-style options menu: a transient, unpersisted panel listing the
app's existing on/off choices plus an About row. Picking a row applies that choice and closes the
menu, like a Winamp menu item; `Close`, the backdrop, or Escape dismisses it. `D` stays decorative
(double-size is not implemented). No setting is added or persisted, and playback, browse, search,
equalizer audio, and every existing control's non-menu behavior are otherwise unchanged.

**Approach.**

- `src/ui/message.rs`: add `pub(in crate::ui) enum OptionsChoice` with `#[derive(Debug, Clone,
  Copy, PartialEq, Eq)]` and variants `AlwaysOnTop`, `TimeDisplay`, `Visualizer`,
  `EqualizerPanel`, and `LibraryPanel`, each documented with the handler it reuses. Add
  `Message::ToggleOptions` (the `O` cell and lightning button: flip `options_visible`, unshading
  first) and `Message::OptionsChoice(OptionsChoice)` (a menu row: apply the choice, then close the
  menu). Document both beside `ToggleAbout`, and update `EscapePressed`'s comment from four panels
  to five.
- `src/ui/player.rs`: add `pub(in crate::ui) options_visible: bool`, defaulting false in
  `WinampPlayer::new` and documented like `about_visible` (transient, not persisted, never drawn in
  shade mode). Add it to `close_transient_panels` and `transient_panel_open`, extending both docs
  from four panels to five, and add it to the `At most one of …` lists on the existing
  `about_visible`, `file_info_visible`, `jump_time_visible`, and `jump_file_visible` fields.
- `src/ui/handlers/window.rs`: add `pub(in crate::ui) fn toggle_options(player) -> Task<Message>`
  mirroring `toggle_about` (flip; on open call `close_transient_panels` first, set the flag, and
  unshade through `toggle_window_shade`). Add `pub(in crate::ui) fn options_choice(player,
  choice: OptionsChoice) -> Task<Message>` that clears `options_visible` first, then dispatches to
  the existing `toggle_always_on_top`, `handlers::playback::toggle_time_display`,
  `handlers::playback::toggle_visualizer`, `handlers::mixer::toggle_equalizer_panel`, or
  `handlers::mixer::toggle_library_panel` and returns its `Task`. Update the module and
  `escape_pressed` docs.
- `src/ui/mod.rs`: dispatch `ToggleOptions` and `OptionsChoice`.
- `src/ui/views/options.rs` (new, modeled on `views/about.rs`): pure `pub(super)` label helpers
  `always_on_top_label(bool)`, `time_label(bool)`, `visualizer_label(bool)`,
  `equalizer_label(bool)`, and `library_label(bool)` returning the exact strings below, plus
  `pub fn view_options_overlay(base, always_on_top, time_remaining, visualizer_visible,
  eq_panel_visible, library_panel_visible) -> Element` stacking a `modal_overlay` panel titled
  `OPTIONS` with one `labeled_button` per row:
  - `Always on Top: On/Off` → `Message::OptionsChoice(OptionsChoice::AlwaysOnTop)`
  - `Time: Remaining/Elapsed` → `OptionsChoice::TimeDisplay`
  - `Visualizer: Shown/Hidden` → `OptionsChoice::Visualizer`
  - `Equalizer: Shown/Hidden` → `OptionsChoice::EqualizerPanel`
  - `Library: Shown/Hidden` → `OptionsChoice::LibraryPanel`
  - `About Nostalgia` → `Message::ToggleAbout` (the existing handler closes the menu through
    `close_transient_panels`)
  - `Close` → `Message::ToggleOptions`
  Give the module its own unit tests for the five label pairs.
- `src/ui/views/mod.rs`: `mod options;` and `pub use options::view_options_overlay;`.
- `src/ui/views/transport.rs`: add `pub(super) fn options_button() -> Element<'static, Message>`
  returning a fixed-width `⚡` button publishing `Message::ToggleOptions`, with a documented
  `OPTIONS_BUTTON_WIDTH`, and push it after the `PL` toggle in `view_transport_controls`, separated
  by the existing `spacer`. Update the module, `view_transport_controls`, and button docs so the row
  reads Previous, Play, Pause, Stop, Next, Repeat, Shuffle, volume, balance, EQ, PL, menu.
- `src/ui/view.rs`: in the normal-mode overlay chain, draw `views::view_options_overlay` while
  `player.options_visible && !player.shaded`, passing `player.always_on_top`,
  `player.time_display == views::TimeDisplay::Remaining`, `player.visualizer_visible`,
  `player.eq_panel_visible`, and `player.library_panel_visible`; add
  `options_visible: player.options_visible` to the `IndicatorModes` it builds. Update the
  root-view doc.
- `src/ui/views/now_playing/indicator.rs`: `lcd_indicator_message("O")` returns
  `Some(Message::ToggleOptions)`; add `options_visible: bool` to `IndicatorModes`; light `O` through
  `indicator_label_color` while that flag holds; update the docs so `O` is live and only `D` stays
  decorative.
- Tests: in `src/ui/views/tests/chrome.rs` add
  `options_overlay_draws_each_row_with_its_state` (all-off/elapsed/hidden and
  all-on/remaining/shown layouts, asserting the exact row strings, `About Nostalgia`, and
  `Close`), `options_overlay_publishes_each_choice_and_closes_from_close_and_backdrop` (click every
  row, asserting its `Message::OptionsChoice(..)` or `ToggleAbout`; `Close` and the backdrop publish
  `ToggleOptions`; panel chrome publishes only `Ignored`), and
  `transport_options_button_publishes_toggle_options` (lay out `view_transport_controls`, assert
  `⚡` sits after `PL`, and click it). In `src/ui/views/tests/now_playing.rs`, extend
  `indicator_label_color_lights_only_the_active_mode`,
  `lcd_indicator_strip_lights_the_active_letters_and_dims_the_rest`, and the indicator press test
  for `options_visible`/`O`, renaming
  `lcd_indicator_message_arms_a_i_and_v_and_leaves_o_d_inert` to
  `lcd_indicator_message_arms_o_a_i_and_v_and_leaves_d_inert` and extending the strip press test's
  name to include `options_on_o`. In `src/ui/tests/panels_tests.rs` add
  `options_choice_applies_each_toggle_and_closes_the_panel` (every `OptionsChoice` flips exactly
  its target flag, clears `options_visible`, and leaves the other flags and playback state alone)
  and `toggle_options_opens_and_closes_and_excludes_each_other_panel` (`ToggleOptions` while
  unshaded schedules no work; opening About/File info/Jump to time/Jump to File closes it and vice
  versa), and add `Message::ToggleOptions` to the toggle list in
  `escape_closes_each_transient_panel_without_touching_anything_else`. In
  `src/ui/tests/view_tests.rs` add `options_panel_draws_over_the_player_and_drops_out_in_shade_mode`
  (a row string appears with `options_visible`, and disappears once `shaded` holds).
- Docs: `docs/usage.md`'s Now Playing sentence makes `O` open the options menu and light while it
  is open (only `D` stays decorative), and adds the menu's rows, the close-on-choice rule, the
  shade restore, and the transport row's lightning button. `docs/invariants.md`: update
  `## Now Playing label` (`O` live and lighting, `D` decorative), `## UI layout` (the menu button
  after `PL`), and `## Transient panel dismissal` (five panels); add `## Options panel` after
  `## About panel` with the guarantee, **Enforced by** the symbols above, **Pinned by** the new test
  names, and a History line. No settings section changes because the menu is not persisted.

**Files touched.** `src/ui/message.rs`, `src/ui/player.rs`, `src/ui/handlers/window.rs`,
`src/ui/mod.rs`, `src/ui/views/options.rs`, `src/ui/views/mod.rs`, `src/ui/views/transport.rs`,
`src/ui/view.rs`, `src/ui/views/now_playing/indicator.rs`, `src/ui/views/tests/chrome.rs`,
`src/ui/views/tests/now_playing.rs`, `src/ui/tests/panels_tests.rs`, `src/ui/tests/view_tests.rs`,
`docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Pressing the LCD's `O` or the transport row's `⚡` opens an `OPTIONS` panel over the normal-mode
  screen; while unshaded, opening or closing it schedules no work. Opening it closes any other
  transient panel, and opening it while shaded restores the window first.
- The panel draws exactly the rows above with labels reflecting the current state on every
  rebuild; pressing a row applies that one choice and closes the panel, `About Nostalgia` opens the
  About panel, and `Close`, the backdrop, or Escape closes it.
- The `O` letter lights `theme::LCD_GREEN` while the menu is open and dims otherwise; `A`, `I`, and
  `V` keep their existing behavior, and `D` stays decorative and publishes nothing.
- The menu is not persisted, adds no setting, and leaves playback, transport, browse, search,
  equalizer audio, volume/balance, and every existing toggle's non-menu behavior unchanged.
- Every existing About route still works: the `M` button, Ctrl+F1, `Close`, the backdrop, Escape,
  and the library-source line.
- The usage reference and the four invariant sections describe the menu and the live `O`, and the
  project's check passes.

## Done
