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

### Ctrl+Shift+1 sorts the showing browse list by title, like Winamp's playlist editor (planned 2026-10-25 by the plan role)

**Goal.** Winamp's playlist editor binds Ctrl+Shift+1 to "Sort list by title". Nostalgia's `Sort`
button (`Message::SortBrowse` → `browse::sort_browse`) already does that sort, by each row's rendered
text, but no key reaches it. Bind Ctrl+Shift+1 to the same `Message::SortBrowse`, so it follows every
rule in the `## Browse row order` invariant with no handler change. Match the top-row **physical**
`1` key (`key::Code::Digit1`), not the logical character. With Shift held the logical key is `"!"` on
US layouts, and on AZERTY it is the `1` key's shifted character. Winamp matches the key position too.
The key does not check shade mode, the same as `sort_browse` and `jump_to_edge_track`. No new
message, no new dependency. This entry is independent of the waiting "Ctrl+R reverses the showing
browse list" plan: it adds a separate function and leaves `shortcut`'s Control block alone. The two
plans both edit the `message_for` doc comment, so whichever lands second has a one-paragraph doc
merge.

**Approach.**
- `src/ui/shortcuts.rs`: add `fn playlist_sort_shortcut(physical_key: &key::Physical, location: Location, modifiers: Modifiers) -> Option<Message>`.
  It returns `Some(Message::SortBrowse)` only when `modifiers.control() && modifiers.shift() &&
  !modifiers.alt() && !modifiers.logo()`, `location == Location::Standard`, and
  `*physical_key == key::Physical::Code(key::Code::Digit1)`. Otherwise it returns `None`. Give it a doc
  comment explaining the physical-key choice. In `message_for`, also destructure `physical_key` from
  `Event::KeyPressed` and try this function first:
  `playlist_sort_shortcut(&physical_key, location, modifiers).or_else(|| numpad_shortcut(..)).or_else(|| shortcut(..))`.
  No other binding claims Ctrl+Shift+Digit1 (`shortcut` returns `None` for every Control chord except
  F/Z/B, and `numpad_shortcut` rejects Control), so the order changes no existing mapping. Add
  Ctrl+Shift+1 to the bound-keys list in `message_for`'s doc comment. Update the sentence in
  `shortcut`'s doc that says "top-row digits stay unbound here": they still are in `shortcut`, and
  Ctrl+Shift+1 is handled by `playlist_sort_shortcut`.
- Inline unit tests in `shortcuts.rs`'s `tests` module. Add a helper
  `digit_pressed(key: Key, code: key::Code, location: Location, modifiers: Modifiers) -> Event`
  shaped like `key_pressed`, then add:
  - `message_for_maps_ctrl_shift_1_to_sort`: physical `Digit1` with `Modifiers::CTRL | Modifiers::SHIFT`
    maps to `Message::SortBrowse` when the logical key is `"!"`, `"1"`, or `"&"`.
  - `ctrl_shift_1_needs_exactly_control_and_shift`: physical `Digit1` with `Modifiers::CTRL`,
    `Modifiers::SHIFT`, `Modifiers::NONE`, `CTRL | SHIFT | ALT`, and `CTRL | SHIFT | LOGO` maps to
    nothing. Physical `Digit2` with `CTRL | SHIFT` maps to nothing. Physical `Numpad1` at
    `Location::Numpad` with `CTRL | SHIFT` (logical `"1"`) maps to nothing.
- `src/ui/tests/shortcuts_tests.rs`: add `ctrl_shift_1_sorts_the_showing_list_from_the_keyboard`.
  Seed `player.artists.items` with three artists named `banana`, `Apple`, `cherry` (the
  `sort_reorders_artists_case_insensitively` fixture), set `player.artists.loading = false`, then build
  an `Event::KeyPressed` with logical `Key::Character("!".into())`, physical `Digit1`,
  `Location::Standard`, and `Modifiers::CTRL | Modifiers::SHIFT`. Assert that
  `super::shortcuts::message_for` yields `Message::SortBrowse`, `update` with it, and assert that the
  names read `Apple, banana, cherry`.
- `README.md` Usage: after the sentence on the `Sort` button's one-shot reorder, add that Ctrl+Shift+1
  does the same sort from the keyboard, like Winamp's playlist editor. Do not touch the tumwater
  markers.
- `docs/invariants.md` `## Browse row order`: in Guarantee, say that Ctrl+Shift+1 sends the same
  `Message::SortBrowse` as the button. Add `shortcuts::playlist_sort_shortcut` to Enforced by, and list
  the new tests under Pinned by.

**Files touched.** `src/ui/shortcuts.rs`, `src/ui/tests/shortcuts_tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**
- Pressing the top-row `1` key with Control and Shift (no Alt, no Logo) maps to
  `Message::SortBrowse`, whatever the logical character is. Ctrl+1, Shift+1, plain 1, Ctrl+Shift+Alt+1,
  Ctrl+Shift+Logo+1, Ctrl+Shift+2, and keypad 1 with Ctrl+Shift map to nothing.
- Every existing binding maps as before. The existing tests in `shortcuts.rs`, `shortcuts_tests.rs`,
  and `sort_tests.rs` pass unchanged.
- From the keyboard, Ctrl+Shift+1 sorts the showing Artists list the same way the `Sort` button does
  (integration test above).
- README and `docs/invariants.md` describe the key. `make check` passes.

## Done
