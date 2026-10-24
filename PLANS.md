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

### Reload the library from the keyboard with F5 (planned 2026-10-23 by the plan role)

**Goal.** Every browse level offers a `Reload` button, but the keyboard has no route to it. Bind
plain or Shift-held `F5` to the same `Message::ReloadLibrary` the button sends, so F5 clears an
active search, returns the browse view to Artists, and refetches the artists list through the
existing clear-then-issue path. The reload drops the previous artists rows and any earlier
failure, shows the loading placeholder until the reply lands, and starts the reloaded list at its
first row, exactly as the button does; it leaves the typed search query, the other browse
buffers, and playback alone. This adds only a key binding: the message, handler, and button stay
unchanged. Ctrl/Alt/Logo chords with F5 remain unbound, and F5 is a library refresh rather than
a text-editing key, so this plan changes no search-box editing behavior.

**Approach.**

- `src/ui/shortcuts.rs`: in `shortcut`, map `Key::Named(key::Named::F5)` to
  `Message::ReloadLibrary` beside the Home/End arms, after the existing Control/Alt/Logo guard,
  so plain and Shift-held F5 reload and chords stay the window manager's. Update the `message_for`
  and `shortcut` docs to list F5 with the other browse keys.
- Tests. `src/ui/shortcuts.rs`: add `message_for_maps_f5_to_reload` covering plain and Shift-held
  F5, and extend `shortcut_ignores_chord_modifiers`'s key list with
  `Key::Named(key::Named::F5)`. `src/ui/tests.rs`: add
  `f5_reloads_the_library_from_the_keyboard`, modeled on `backspace_steps_back_from_albums`: put
  the player in a non-Artists view with an active search, a typed query, stale artists rows, and
  a populated other browse buffer; build the F5 key event, map it through
  `super::shortcuts::message_for`, assert it is `Message::ReloadLibrary`, drive it through
  `update`, assert the arm schedules work, then assert the same observable state as
  `reload_library_returns_to_artists_and_refetches_them` (Artists view, search inactive, typed
  query and other buffer intact, artists rows and error cleared, loading set).
- `README.md`: in the Usage keyboard sentence, after the Ctrl+F clause, say F5 refetches the
  library and returns to the Artists list, exactly like the `Reload` button. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: extend the existing `## Browse hierarchy` section (rather than adding a
  sibling) so its Reload guarantee also covers plain or Shift-held F5 taking the same
  `Message::ReloadLibrary` path as the button, with Ctrl/Alt/Logo chords unbound. Name
  `shortcuts::shortcut` as the enforcer, add the two new test names to **Pinned by**, and record
  the addition in **History**.

**Files touched.** `src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- Plain or Shift-held `F5` maps to `Message::ReloadLibrary`; Ctrl/Alt/Logo chords with F5 map to
  nothing.
- Driving the mapped message through `update` produces the Reload button's observable behavior:
  the Artists view shows, an active search is cleared, the typed query and the other browse
  buffers and playback are unchanged, the artists rows and error are dropped, the artists list
  is loading, and the arm schedules the artists fetch.
- The new tests pass, `make check` passes, and README and `docs/invariants.md` describe the F5
  route to Reload.

## Done
