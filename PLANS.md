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

### Ctrl+R reverses the showing browse list, like Winamp's playlist editor (planned 2026-10-25 by the plan role)

**Goal.** In Winamp's playlist editor, Ctrl+R ("Reverse list") flips the playlist's order. Nostalgia's
only reorder is the one-shot `Sort` button (`browse::sort_browse`), and no key reverses a list. Bind
Ctrl+R to reverse the list currently showing (Artists, Albums, Songs, All Songs, All Albums, or
Playlists), as a one-shot reorder that follows every rule the `## Browse row order` invariant already
sets for Sort: it bumps that list's epoch, clears its selection, keeps cached titles and labels
aligned, and Back to a reversed parent restores library order. Ctrl+Shift+R (Winamp's Randomize list)
stays unbound. No on-screen button, no new dependency.

**Approach.**
- `src/ui/browse.rs`: extract the tail of `BrowseList::sort_rows_by` — composing `order` into
  `library_positions`, clearing `selected`, calling `reorder_rows(&order)`, and bumping `epoch` with
  `wrapping_add(1)` — into a private `fn apply_reorder(&mut self, order: Vec<usize>) where T: Clone`, and
  have `sort_rows_by` call it after building its sorted permutation (behavior unchanged). Add
  `pub(super) fn reverse_rows(&mut self) where T: Clone`: the same guard as `sort_rows_by` (fewer than
  two rows, `loading`, or `error.is_some()` → return without change), then
  `self.apply_reorder((0..self.items.len()).rev().collect())`. Doc-comment it like `sort_rows_by`, and
  update the `library_positions` field doc and the `reorder_rows` / `restore_library_order` doc
  comments, which today name only `sort_rows_by`, to name `reverse_rows` too.
- `src/ui/handlers/browse.rs`: add `pub(in crate::ui) fn reverse_browse(player: &mut WinampPlayer) -> Task<Message>`
  after `sort_browse`, matching `player.current_view` onto the same six lists (`artists`, `albums`,
  `songs`, `all_songs`, `all_albums`, `playlists`) and calling `reverse_rows()`; return `Task::none()`.
  It does not check shade mode, the same as `jump_to_edge_track`.
- `src/ui/message.rs`: add `ReverseBrowse` after `SortBrowse`, with a comment in the style of its
  neighbour ("Ctrl+R: reverse the showing list's order, one-shot like Sort …").
  `src/ui/mod.rs`: route `Message::ReverseBrowse => handlers::browse::reverse_browse(player)` beside
  `SortBrowse` in `update`.
- `src/ui/shortcuts.rs`: in `shortcut`'s Control block (the one mapping Ctrl+Z / Ctrl+B, under the
  no-Alt, no-Logo rule), add `Key::Character(c) if c.eq_ignore_ascii_case("r") && !modifiers.shift()`
  → `Message::ReverseBrowse`. Ctrl+Shift+R falls through to the `modifiers.control() … return None`
  line. Update the doc comments on `message_for` and `shortcut` that list the Control exceptions.
  Unit tests inline: `message_for_maps_ctrl_r_to_reverse_the_list` (Ctrl with `r` and `R` maps;
  Ctrl+Shift, Alt, Logo, Ctrl+Alt, Ctrl+Logo with `r` do not), and in
  `shortcut_ignores_chord_modifiers` drop `r` from the `Modifiers::CTRL` iteration only (keep it for
  Alt and Logo — split the loop or skip `r` when `modifiers == Modifiers::CTRL`), updating its
  comment to name Ctrl+R among the exceptions. Plain R still maps to the Repeat toggle (the existing
  letter test covers it).
- New `src/ui/tests/reverse_tests.rs` (`sort_tests.rs` is already ~720 lines), registered in
  `src/ui/tests/mod.rs` beside `mod sort_tests;`, using `test_player` and `stepping_songs`:
  - `reverse_flips_the_artists_list`: three artists loaded (`loading = false`), `ReverseBrowse`,
    names read in reverse; a second `ReverseBrowse` restores the original order.
  - `reverse_flips_songs_and_keeps_their_cached_text_aligned`: `Message::SongsLoaded(stepping_songs())`,
    reverse, ids read Three, Two, One and `songs.titles` / `songs.labels` reorder with them; same
    for `Message::AllSongsLoaded` over `all_songs`.
  - `reverse_bumps_the_epoch_and_clears_the_selection`: select a row, capture the epoch, reverse;
    epoch differs and `selected` is `None`; an `ArtistSelected` with the old epoch leaves the view
    on Artists.
  - `reverse_with_an_empty_or_loading_list_is_a_noop`: for each of the six views before rows load,
    and for an artists list holding rows with `loading = true`, reverse leaves rows and epoch alone.
  - `back_restores_a_reversed_then_sorted_albums_list_to_library_order`: load albums, open one
    (Songs), Back — then on Albums reverse and Sort, open an album, Back, and assert albums are in
    library order (proves `apply_reorder` composes both reorders into `library_positions`).
  - `reverse_leaves_other_lists_view_search_and_playback_untouched`: seed two lists and a current
    track, reverse on one view, assert the other list, `current_view`, `search_active`, and
    `current_track` are unchanged.
- `README.md` Usage: after the `Sort` sentence, add that Ctrl+R reverses the showing list's order
  the same one-shot way (reopening the list restores library order), like Winamp's playlist editor.
  Do not touch the tumwater markers.
- `docs/invariants.md` `## Browse row order`: extend the guarantee to Ctrl+R's reverse (same epoch,
  selection, cache, restore and no-op rules; Ctrl+Shift+R unbound), add `BrowseList::reverse_rows`,
  `apply_reorder`, `browse::reverse_browse`, and the `shortcut` Ctrl+R arm to Enforced by, and list the
  new tests under Pinned by.

**Files touched.** `src/ui/browse.rs`, `src/ui/handlers/browse.rs`, `src/ui/message.rs`,
`src/ui/mod.rs`, `src/ui/shortcuts.rs`, `src/ui/tests/reverse_tests.rs` (new), `src/ui/tests/mod.rs`,
`README.md`, `docs/invariants.md`.

**Acceptance criteria.**
- Ctrl+R (`r` or `R`, Control without Shift, Alt, or Logo) maps to `Message::ReverseBrowse`;
  Ctrl+Shift+R, Alt+R, and Logo+R map to nothing; plain R still toggles Repeat.
- `ReverseBrowse` reverses the showing list's `items` with its `titles` and `labels` for every one of
  the six views, bumps that list's epoch, clears its selection, and changes no other list, the view,
  the search state, or playback; reversing twice restores the original order.
- An empty, loading, or failed list is unchanged by `ReverseBrowse`, epoch included.
- Back to a parent that was reversed (alone or combined with Sort) restores its library order.
- Existing Sort tests pass unchanged. README and `docs/invariants.md` describe the key. `make check`
  passes.

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
