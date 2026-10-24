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

### Bind the classic A and T keys for always-on-top and the time display (planned 2026-10-23 by the plan role)

**Goal.** The title bar's `A` and the LCD strip's `A` toggle always-on-top, and clicking any
real-time readout switches it between elapsed and remaining, but neither classic Winamp main-window
hotkey is bound: a keyboard-driven user must reach for the mouse. Bind `A` to
`Message::ToggleAlwaysOnTop` and `T` to `Message::ToggleTimeDisplay` so the classic key set covers
both existing actions. No new message, handler, state, or playback path: `handlers::window::
toggle_always_on_top` and `handlers::playback::toggle_time_display` already implement them.

**Approach.**

- `src/ui/shortcuts.rs`: in `shortcut`, add a `Key::Character` arm matching `"a"` for
  `Message::ToggleAlwaysOnTop` and a `"t"` arm for `Message::ToggleTimeDisplay`, beside the
  existing R/S letter arms and before the named-key arms. The existing Control/Alt/Logo guard runs
  first, so a chorded A/T stays with the OS and window manager and Shift is allowed, as for the
  other letters. Update the `message_for` and `shortcut` doc comments to list A and T beside
  Z/X/C/V/B/R/S.
- `src/ui/shortcuts.rs` tests: add `message_for_maps_a_and_t_to_the_classic_toggles`, driving a
  plain and a Shift-held `Key::Character("a")` and `Key::Character("t")` `KeyPressed` event
  through `message_for` with the existing `key_pressed` helper and asserting the two messages; add
  both keys to the key list in `shortcut_ignores_chord_modifiers`, and add uppercase A/T asserts to
  `shortcut_matches_letters_case_insensitively`. Leave `assert_letter_bindings` and its doc alone:
  it covers the transport letters, while A and T dispatch view and window messages.
- `src/ui/tests.rs`: add `a_toggles_always_on_top_from_the_keyboard`, starting from the existing
  `player_with_window_id()` fixture, building an `A` `KeyPressed` event as
  `enter_opens_the_selected_song` builds its Enter event, asserting `super::shortcuts::message_for`
  returns `Message::ToggleAlwaysOnTop`, driving it through `update`, and asserting the flag flipped
  and the arm scheduled work; and `t_toggles_the_time_display_from_the_keyboard`, building a `T`
  event the same way and asserting `player.time_display` flips after `update`. These pin the
  shortcut-to-handler wiring, not just the table.
- `README.md`: in the Usage keyboard sentence, say `A` toggles always-on-top and `T` switches the
  large readout between elapsed and remaining time, and that both are the same actions as the
  on-screen controls. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, state that `shortcuts::message_for` maps T to
  `Message::ToggleTimeDisplay`, so the stored mode flips in every transport state; the stopped
  readout's click guard makes only the readout inert, not the key. In `## Settings persistence`,
  state that the same mapper maps A to `Message::ToggleAlwaysOnTop`, the arm the title bar's A and
  the strip's A already press, with the window-level schedule unchanged. Name
  `shortcuts::message_for` in **Enforced by** and add the two new tests to **Pinned by** in the
  matching section.

**Files touched.** `src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A plain or Shift-held A press dispatches `Message::ToggleAlwaysOnTop` through `update`, flipping
  `WinampPlayer::always_on_top` and scheduling the same `iced::window::set_level` call the title
  bar's and strip's A presses schedule; before the window id resolves it flips the flag without a
  window call, matching the existing arm.
- A plain or Shift-held T press dispatches `Message::ToggleTimeDisplay` through `update`, flipping
  `WinampPlayer::time_display` exactly as a readout click does. Unlike the stopped readout's click
  target it is not gated by transport mode, because the key maps to the existing message and the
  mode is presentation-only.
- Control, Alt, and Logo chords on A and T map to nothing, and every existing letter, Enter,
  arrow, and seek binding is unchanged.
- The focused `Search library` box keeps typed A and T for editing its query, as it already keeps
  Enter and Backspace, because iced's `TextInput` captures them before the window subscription.
- README and `docs/invariants.md` describe both keys, and `make check` passes.

## Done
