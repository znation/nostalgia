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

### Ctrl+W toggles window-shade mode, like Winamp (planned 2026-10-25 by the plan role)

**Goal.** Winamp binds Ctrl+W to "Toggle windowshade mode" for the main window. Nostalgia rolls
the window up only through the title bar's shade button or a double-click on the bar or the
strip, so there is no keyboard route. Bind Ctrl+W (with or without Shift, no Alt or Logo) to the
existing `Message::ToggleWindowShade`, so the key rolls the window up and back exactly as the
shade button does. Plain W stays unbound.

**Approach.**
- `src/ui/shortcuts.rs`, `shortcut`: in the existing `modifiers.control() && !modifiers.alt() &&
  !modifiers.logo()` block (the Ctrl+Z / Ctrl+B / Ctrl+R arms), add an arm for
  `character.eq_ignore_ascii_case("w")` returning `Some(Message::ToggleWindowShade)`. Shift is
  allowed, the same as Ctrl+Z and Ctrl+B. Update the doc comments on `message_for` and `shortcut`
  and the block's inline comment to list Ctrl+W.
- No handler change: `handlers::window::toggle_window_shade` already flips `player.shaded`,
  measures on the first shade, and resizes; without a window id it only flips the flag. The key
  does not touch `about_visible` (or the File info flag, if that plan has merged): the view's
  existing `!player.shaded` guard in `src/ui/view.rs` stops the overlay drawing while shaded, and
  it draws again after an unshade. That matches what the shade button would do and needs no new
  code.
- `iced::keyboard::listen` skips events a focused widget captures. Do not add special handling
  for a focused `Search library` box.

**Tests.**
- `src/ui/shortcuts.rs` tests: add `message_for_maps_ctrl_w_to_toggle_window_shade`. Use
  `assert_maps!` to show that `w` and `W` with `Modifiers::CTRL`, and `w` with
  `Modifiers::CTRL | Modifiers::SHIFT`, map to `Message::ToggleWindowShade`. Then show that `w`
  with `Modifiers::NONE`, `SHIFT`, `ALT`, `LOGO`, `CTRL | ALT`, and `CTRL | LOGO` maps to `None`.
- `src/ui/tests/shortcuts_tests.rs`: add `ctrl_w_rolls_the_window_up_and_back`, following
  `a_toggles_always_on_top_from_the_keyboard`'s pattern. Build a `KeyPressed` event for `w` with
  `Modifiers::CTRL` (physical `key::Code::KeyW`, `Location::Standard`) and map it through
  `super::shortcuts::message_for`. Starting from `player_with_window_id()`, use
  `assert_message_schedules_work` to show that the mapped message sets `player.shaded`. Then
  set `player.unshaded_size = Some(iced::Size::new(800.0, 600.0))` and show that a second mapped
  press clears `player.shaded` and schedules work.

**Files touched.** `src/ui/shortcuts.rs`, `src/ui/tests/shortcuts_tests.rs`, `README.md` (the
keys paragraph in Usage, after the Ctrl+Z/Ctrl+B sentence: "Ctrl+W rolls the window up into
shade mode and back, like the title bar's shade button."), `docs/invariants.md` (`## Window
shade`: add a bullet that Ctrl+W, Control with optional Shift and without Alt or Logo, publishes
`Message::ToggleWindowShade`, pinned by the two new tests. In `## Search focus`, change "every
other Control chord remains the window manager's" so it no longer claims Ctrl+W).

**Acceptance criteria.**
- Ctrl+W and Ctrl+Shift+W map to `Message::ToggleWindowShade`. Plain W, Shift+W, Alt+W,
  Logo+W, Ctrl+Alt+W, and Ctrl+Logo+W map to nothing.
- Pressing Ctrl+W in normal mode shades the window, and pressing it again restores the window,
  through the unchanged `toggle_window_shade`.
- The existing shade, About, and shortcut tests still pass, and `make check` passes.

## Done
