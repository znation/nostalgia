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

### Ctrl+Shift+R randomizes the showing browse list, like Winamp's playlist editor (planned 2026-10-25 by the plan role)

**Goal.** Winamp's playlist editor binds Ctrl+Shift+R to "Randomize list". Ctrl+R already reverses
the showing list (`Message::ReverseBrowse` → `browse::reverse_browse` → `BrowseList::reverse_rows`),
and Ctrl+Shift+R is deliberately left unbound today. Bind Ctrl+Shift+R to shuffle the showing list
(Artists, Albums, Songs, All Songs, All Albums, or Playlists) as a one-shot reorder. It follows every
rule the `## Browse row order` invariant sets for Sort and Ctrl+R: the list's epoch is bumped, its
selection cleared, cached titles and labels move with their rows, Back to a randomized parent
restores library order, and an empty, loading, or failed list (or one with a single row) is left
alone. The randomness comes from the existing `transport::shuffle_roll` (std `RandomState`), so no
new dependency is needed. A pure seeded permutation keeps the tests deterministic. No on-screen
button. This plan is independent of the waiting "Ctrl+Shift+1 sorts the showing browse list by
title" plan. Both edit `message_for`'s doc comment and the `## Browse row order` invariant text, so
whichever lands second has a small doc merge.

**Approach.**
- `src/ui/browse.rs`: add a free function `pub(super) fn shuffled_order(len: usize, seed: u64) -> Vec<usize>`.
  It returns a permutation of `0..len` made by a Fisher–Yates shuffle driven by splitmix64. The state
  starts at `seed`. Each draw does `state = state.wrapping_add(0x9E37_79B9_7F4A_7C15)`, then
  `z = state`, `z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9)`,
  `z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB)`, and `z ^ (z >> 31)`. Loop `i` from
  `len - 1` down to `1`, swapping `order[i]` with `order[(draw % (i + 1) as u64) as usize]`. Use a
  saturating or empty-range form so `len` 0 and 1 return `[]` and `[0]` without underflow. Doc-comment
  it as pure and deterministic per seed. Add
  `pub(super) fn shuffle_rows(&mut self, seed: u64) where T: Clone` beside `reverse_rows`. It uses the
  same guard (fewer than two rows, `loading`, or `error.is_some()` → return unchanged), then calls
  `self.apply_reorder(shuffled_order(self.items.len(), seed))`. A seed whose permutation is the
  identity still bumps the epoch and clears the selection, the same as a Sort of an already-sorted
  list. Add `shuffle_rows` to the doc comments that today name `sort_rows_by` and `reverse_rows`: the
  `selected` and `library_positions` field docs and `restore_library_order`.
- `src/ui/message.rs`: add `RandomizeBrowse` after `ReverseBrowse`, with a comment in the same style
  ("Ctrl+Shift+R: shuffle the showing list's order, one-shot like Sort …").
- `src/ui/handlers/browse.rs`: add `pub(in crate::ui) fn randomize_browse(player: &mut WinampPlayer) -> Task<Message>`
  after `reverse_browse`. It draws `let seed = crate::ui::transport::shuffle_roll();` once, then
  matches `player.current_view` onto the same six lists and calls `shuffle_rows(seed)`. Return
  `Task::none()`. Use whatever import path `playback.rs` already uses for `transport`.
- `src/ui/mod.rs` `update`: route `Message::RandomizeBrowse => handlers::browse::randomize_browse(player)`
  next to the `ReverseBrowse` arm.
- `src/ui/shortcuts.rs` `shortcut`, Control block: next to the Ctrl+R arm, add an arm for
  `character.eq_ignore_ascii_case("r") && modifiers.shift()` returning `Message::RandomizeBrowse`.
  Update the block comment and the doc comments on `shortcut` and `message_for` that say Ctrl+Shift+R
  stays unbound. In the inline test `message_for_maps_ctrl_r_to_reverse_the_list`, drop
  `Modifiers::CTRL | Modifiers::SHIFT` from the must-not-map list. Add
  `message_for_maps_ctrl_shift_r_to_randomize_the_list`: `"r"` and `"R"` with `CTRL | SHIFT` map to
  `Message::RandomizeBrowse`. `CTRL | SHIFT | ALT`, `CTRL | SHIFT | LOGO`, and `SHIFT` alone do not map
  to it: with Shift alone, `R` still toggles Repeat, so assert `!= Some(Message::RandomizeBrowse)`
  rather than `is_none()` for that case.
- New `src/ui/tests/randomize_tests.rs`, registered in `src/ui/tests/mod.rs` beside `reverse_tests`.
  Use `reverse_tests.rs`'s fixture helpers `artist` and `album`, copied locally, and add:
  - `shuffled_order_is_a_deterministic_permutation`: for `len` in `0..=6` and several seeds, the
    sorted result equals `0..len`, and two calls with the same seed are equal.
  - `shuffled_order_pins_a_known_seed`: pin `shuffled_order(5, 0)` to the exact vector the algorithm
    above yields, computed once by the implementer, so a change to the algorithm is caught.
  - `shuffled_order_varies_with_the_seed`: over seeds `0..8` with `len` 5, at least two distinct orders
    appear.
  - `shuffle_rows_moves_songs_with_their_cached_text`: store songs with
    `store_with_row_text`, the way `reverse_flips_songs_and_keeps_their_cached_text_aligned` does,
    call `player.songs.shuffle_rows(seed)` with a seed whose order is not the identity, and assert
    that the item order equals `shuffled_order(len, seed)` applied to the library order, with titles
    and labels matching their rows.
  - `randomize_bumps_the_epoch_clears_the_selection_and_keeps_every_row`: modeled on
    `reverse_bumps_the_epoch_and_clears_the_selection`, but driven by
    `update(&mut player, Message::RandomizeBrowse)`. It asserts that the epoch changed, `selected()`
    is `None`, a stale-epoch `ArtistSelected` is rejected, and the sorted artist names are unchanged.
  - `randomize_with_an_empty_or_loading_list_is_a_noop`, modeled on `assert_reverse_is_a_noop_on`.
  - `back_restores_a_randomized_albums_list_to_library_order`, modeled on
    `back_restores_a_reversed_then_sorted_albums_list_to_library_order`, using `shuffle_rows(seed)`
    directly on `player.albums`.
- `README.md` Usage: after the Ctrl+R sentence ("Ctrl+R reverses the showing list's order the same
  one-shot way, like Winamp's playlist editor."), add that Ctrl+Shift+R shuffles it into a random
  order the same one-shot way. Do not touch the tumwater markers.
- `docs/invariants.md` `## Browse row order`: change "Ctrl+Shift+R stays unbound" to describe
  Ctrl+Shift+R randomizing under the same rules. Add `BrowseList::shuffle_rows`, `shuffled_order`,
  `browse::randomize_browse` (routed from `Message::RandomizeBrowse`), and the Ctrl+Shift+R arm to
  Enforced by. List the new tests under Pinned by.

**Files touched.** `src/ui/browse.rs`, `src/ui/message.rs`, `src/ui/handlers/browse.rs`,
`src/ui/mod.rs`, `src/ui/shortcuts.rs`, `src/ui/tests/randomize_tests.rs` (new), `src/ui/tests/mod.rs`,
`README.md`, `docs/invariants.md`.

**Acceptance criteria.**
- Ctrl+Shift+R (Control and Shift, no Alt or Logo, either letter case) maps to
  `Message::RandomizeBrowse`. Ctrl+R still maps to `Message::ReverseBrowse`, and plain or
  Shift-only `R` still toggles Repeat. Ctrl+Shift+Alt+R and Ctrl+Shift+Logo+R map to nothing.
- `shuffled_order` returns a permutation of `0..len` that is identical for the same seed, and is
  pinned for one seed.
- Randomizing the showing list keeps every row and moves cached titles and labels with their rows.
  It bumps only that list's epoch and clears its selection. Back to a randomized parent restores
  library order. An empty, loading, or failed list is a no-op.
- The existing reverse, sort, and shortcut tests pass, apart from the one Ctrl+Shift+R must-not-map
  case removed above. README and `docs/invariants.md` describe the key. `make check` passes.

## Done
