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

### Alt+E and Alt+G toggle the library and equalizer panels, like Winamp (planned 2026-10-25 by the plan role)

**Goal.** In Winamp 2.x, Alt+E shows or hides the playlist editor and Alt+G shows or hides the
graphical equalizer. Nostalgia's playlist-editor analogue is the library section behind the `PL`
button, and no key reaches it. Its equalizer panel answers only to plain `E` and the `EQ` button.
Bind Alt+E to `Message::ToggleLibraryPanel` and Alt+G to `Message::ToggleEqualizerPanel`, so
both windows toggle from the keyboard the Winamp way. Both remain layout-only toggles. In shade
mode both keys do nothing, as plain `E` does today.

**Approach.**

- `src/ui/shortcuts.rs`: add `fn panel_shortcut(physical_key: &key::Physical, modifiers:
  Modifiers) -> Option<Message>`, written like `file_info_shortcut`. It requires Alt held with
  Control, Shift, and Logo released. It maps `key::Physical::Code(key::Code::KeyE)` to
  `Message::ToggleLibraryPanel` and `key::Code::KeyG` to `Message::ToggleEqualizerPanel`. Match
  by physical key, not logical character, because on macOS Alt+E and Alt+G produce other
  logical characters (a dead key, `©`). The doc comment should say so. Chain it in
  `message_for` right after `file_info_shortcut`. Add Alt+E and Alt+G to `message_for`'s doc
  comment list. `shortcut` still returns `None` for every Alt chord, so
  `shortcut_ignores_chord_modifiers` stays true unchanged.
- `src/ui/handlers/mixer.rs`: `toggle_library_panel` should return `Task::none()` without
  flipping the flag while `player.shaded`, the same guard `toggle_equalizer_panel` has. Shade
  mode draws no `PL` button (only the main transport row in `views/transport.rs` pushes
  `library_panel_toggle()`), so the guard changes only the new key path. Update the doc comment.
- `docs/invariants.md`: in the EQ toggle bullet, add that Alt+G dispatches the same
  `Message::ToggleEqualizerPanel`. In the `PL` toggle bullet, add that Alt+E dispatches
  `Message::ToggleLibraryPanel`, that its other modifier chords stay unbound, and that shade mode
  changes nothing because the toggle leaves `library_panel_visible` alone while
  `WinampPlayer::shaded` holds. Add the new tests to that section's test list.
- `README.md` (Usage keys paragraph, outside the brief markers): after the sentence on E and
  the `EQ` button, add "Alt+E shows or hides the library section exactly like the `PL` button,
  and Alt+G the equalizer panel like `EQ`, as Winamp's Alt+E and Alt+G toggle its playlist
  editor and equalizer."

**Files touched.** `src/ui/shortcuts.rs` (function + inline tests), `src/ui/handlers/mixer.rs`,
`src/ui/tests/shortcuts_tests.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**

- Inline test `message_for_maps_alt_e_and_alt_g_to_the_panel_toggles` in `shortcuts.rs`
  (built with the `digit_pressed` helper or a sibling taking a `key::Code`). With
  `Modifiers::ALT`, physical `KeyE` maps to `ToggleLibraryPanel` and `KeyG` to
  `ToggleEqualizerPanel`. This must hold both when the logical key is `"e"`/`"g"` and when it
  is a different character (e.g. `"´"`, `"©"`). With `CTRL|ALT`, `SHIFT|ALT`, and `LOGO|ALT`,
  neither maps. Plain `E` still maps to `ToggleEqualizerPanel`, and plain `G` maps to nothing.
- `src/ui/tests/shortcuts_tests.rs`:
  - `alt_e_toggles_the_library_panel_and_back`. Map the Alt+E event through
    `shortcuts::message_for` and send it with `update`. `library_panel_visible` flips to false,
    and a second press sets it back to true. `assert_message_schedules_no_work` holds.
  - `alt_e_in_shade_mode_leaves_the_library_panel_alone`. With `player.shaded = true`, the
    mapped message schedules no work and `library_panel_visible` stays true. This mirrors
    `e_in_shade_mode_leaves_the_equalizer_panel_alone`.
  - `alt_g_toggles_the_equalizer_panel`. The mapped Alt+G message flips `eq_panel_visible`.
- `toggle_library_panel_hides_and_restores_the_library_section`, the settings persistence
  tests for the library flag, and every other existing test pass unchanged. `make check` passes.

## Done
