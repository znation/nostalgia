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

### Step back out of a browse level with Backspace (planned 2026-10-23 by the plan role)

**Goal.** The browse list's `Back` button climbs one level up, and Enter opens the selected row
from the keyboard, but no key triggers `Message::Back`; a user working from the keyboard must
reach for the mouse. Map a Backspace key press to `Message::Back`, so it steps exactly one level
up with the same rules and no-op at the Artists list as the button. Escape stays unbound, and the
`Search library` box keeps Backspace for editing its query, as it already keeps Enter for
`SearchSubmitted`.

**Approach.**

- `src/ui/shortcuts.rs`: in `shortcut`, add
  `Key::Named(key::Named::Backspace) => Some(Message::Back)` beside the Enter arm, after the
  existing Control/Alt/Logo chord guard, so a chorded Backspace still belongs to the OS and Shift
is allowed like the other bindings. Update the `message_for` and `shortcut` doc comments to list
Backspace as the browse Back key beside Enter. Add no `Message` variant, update arm, fetch, or
playback path: `Message::Back` and `browse::back` already implement the hierarchy.
- `src/ui/shortcuts.rs` tests: add `message_for_maps_backspace_to_back`, driving a plain and a
  Shift-held `key::Named::Backspace` `KeyPressed` event through `message_for` with the existing
  `key_pressed` helper and asserting `Message::Back`; add `Key::Named(key::Named::Backspace)` to
  the key list in `shortcut_ignores_chord_modifiers`. Keep the Escape assertion in
  `message_for_ignores_unbound_keys_and_non_press_events` as it is, because Escape remains
  unbound.
- `src/ui/tests.rs`: add `backspace_steps_back_from_albums`, starting from the existing
  `player_in_albums()` fixture, building a Backspace `KeyPressed` event as `enter_opens_the_selected_song`
  builds its Enter event, asserting `super::shortcuts::message_for` returns `Message::Back`, then
  driving that message through `update` and asserting `CurrentView::Artists`. This pins the
  shortcut-to-handler wiring, not just the table.
- `README.md`: in the Usage keyboard sentence, say Backspace steps the browse list back one level
  exactly like the `Back` button, does nothing at the Artists list, and that the `Search library`
  box keeps Backspace for editing its query while focused. Do not touch the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Browse hierarchy`, state that Backspace is the keyboard route to
  the same single-step Back action, that it is captured by the focused search box's `TextInput`
  like Enter, and that Artists stays a no-op; name `shortcuts::message_for` in **Enforced by** and
add the two new tests to **Pinned by**.

**Files touched.** `src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A plain or Shift-held Backspace press below the Artists list steps exactly one browse level up
  through the existing `Message::Back` arm, with the same Albums/Songs/All Songs/All Albums/
  Playlists/search origins and the same snap-to-top follow-up task the Back button gets; at the
  Artists list it is the existing no-op.
- Control, Alt, and Logo chords on Backspace map to nothing, Escape remains unbound, and the
  focused `Search library` box keeps Backspace for editing rather than navigating.
- Existing letter, Enter, arrow, and seek bindings are unchanged; no message, handler, fetch,
  playback, or window path is added.
- README and `docs/invariants.md` describe the key, and `make check` passes.

## Done
