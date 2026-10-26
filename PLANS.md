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

### Add Winamp's Ctrl+F1, Ctrl+A, and Ctrl+T chords for About, always-on-top, and the time display (planned 2026-10-26 by the plan role)

**Goal.** Winamp binds `Ctrl+F1` to its About box, `Ctrl+A` to always-on-top, and `Ctrl+T` to the
elapsed/remaining time-display toggle. Nostalgia already has all three actions — the title bar's
`M` button, the `A` key (and the LCD strip's `A`), and the `T` key (and a readout click) — but
none of Winamp's three chords reaches them. Add the three as aliases through the existing
`Message`s, so keyboard use matches Winamp and About gains a keyboard route. No new messages,
state, handlers, views, settings, or playback behavior.

**Approach.**

- `src/ui/shortcuts.rs`: in `shortcut`'s Control block (the
  `if modifiers.control() && !modifiers.alt() && !modifiers.logo()` match that already maps
  Ctrl+Z, Ctrl+B, Ctrl+R, Ctrl+W, and Ctrl+J), add arms before its `_ => {}`:
  - `Key::Character(character)` matching `character.eq_ignore_ascii_case("a")` returns
    `Message::ToggleAlwaysOnTop`.
  - `Key::Character(character)` matching `character.eq_ignore_ascii_case("t")` returns
    `Message::ToggleTimeDisplay`.
  - `Key::Named(key::Named::F1)` returns `Message::ToggleAbout`.
  Shift is allowed (as it is for Ctrl+Z/Ctrl+B/Ctrl+F); Control+Alt and Control+Logo keep
  returning `None` through the existing guard. Extend `shortcut`'s and the module's doc comments
  to list the three chords. No `message_for` dispatch change is needed because `shortcut` already
  runs from it.
- `src/ui/shortcuts.rs` tests:
  - Add `message_for_maps_ctrl_a_and_ctrl_t_to_the_classic_toggles`: lowercase and uppercase
    `a`/`t` with `Modifiers::CTRL` and `Modifiers::CTRL | Modifiers::SHIFT` map to
    `ToggleAlwaysOnTop`/`ToggleTimeDisplay`; `Ctrl+Alt` and `Ctrl+Logo` with either letter map to
    nothing.
  - Add `message_for_maps_ctrl_f1_to_about`: `Modifiers::CTRL` and
    `Modifiers::CTRL | Modifiers::SHIFT` with `Key::Named(key::Named::F1)` map to `ToggleAbout`;
    plain `F1`, `Alt+F1`, and `Logo+F1` map to nothing.
  - Update `shortcut_ignores_chord_modifiers`: remove `a` and `t` from the letters it asserts
    unbound under `Modifiers::CTRL` (they now map), while keeping the `Alt`/`Logo` assertions for
    them; leave `s`, `x`, `e`, and the named keys as they are.
- `docs/invariants.md`: in `## Settings persistence`, change the sentence that says the classic
  `A` key reaches `Message::ToggleAlwaysOnTop` to name `Ctrl+A` too, and add the new test to that
  section's `Pinned by` list. In `## Now Playing label`, change the classic `T` key sentence to
  name `Ctrl+T` too. In `## About panel`, add a bullet that `shortcuts::shortcut` maps Ctrl+F1
  (Control with optional Shift, no Alt or Logo) to `Message::ToggleAbout`, and add
  `message_for_maps_ctrl_f1_to_about` to its `Pinned by` list. Leave every guarantee otherwise
  unchanged.
- `docs/usage.md` (the keyboard sections): in the classic-keys paragraph, read
  `A (or Ctrl+A) toggles always-on-top and T (or Ctrl+T) switches …`, and in the About sentence,
  read ``The title bar's `M` button (or `Ctrl+F1`) opens …``. Leave the rest of the prose alone.

**Files touched.** `src/ui/shortcuts.rs`, `docs/invariants.md`, `docs/usage.md`.

**Acceptance criteria.**

- `Ctrl+A` and `Ctrl+Shift+A` publish `Message::ToggleAlwaysOnTop` exactly like `A`; `Ctrl+T` and
  `Ctrl+Shift+T` publish `Message::ToggleTimeDisplay` exactly like `T`; `Ctrl+F1` and
  `Ctrl+Shift+F1` publish `Message::ToggleAbout` exactly like the `M` button. The existing
  handlers therefore give the same toggles, About mutual exclusion, and shade restore with no new
  code path.
- `Ctrl+Alt` and `Ctrl+Logo` with `a` or `t` stay unbound, plain `F1`, `Alt+F1`, and `Logo+F1` stay
  unbound, and existing plain letters, Ctrl+R/Ctrl+Shift+R, Ctrl+W, Ctrl+J, Ctrl+Z, Ctrl+B, and
  Ctrl+F keep their current mappings.
- `docs/usage.md` and `docs/invariants.md` name the three chords and the new tests, and
  `make check` passes.

## Done
