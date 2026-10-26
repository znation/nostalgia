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

### Alt+3 opens a File info panel for the current track, like Winamp (planned 2026-10-25 by the plan role)

**Goal.** Winamp binds Alt+3 to "File info" for the current song. Nostalgia has no way to see the
current track's details apart from the scrolling LCD line. Bind Alt+3 to toggle a transient,
unpersisted File info panel that names the current track (the same track the Now Playing bar and
the window title name). The panel works like the About panel: a modal overlay over the
normal-mode screen that closes from a `Close` button, from a backdrop click, or from Alt+3 again.

**Approach.**
- `src/ui/views/modal.rs` (new): move the backdrop/panel/`Stack` code out of
  `views::about::view_about_overlay` into
  `pub(super) fn modal_overlay<'a>(base: Element<'a, Message>, content: Column<'a, Message>, close: Message) -> Element<'a, Message>`.
  It keeps the `BACKDROP` scrim, the `PANEL_MAX_WIDTH` cap, `bevel::raised_panel` on
  `theme::WINDOW_BACKGROUND` with 16px padding, a backdrop press publishing `close`, and
  `Message::Ignored` for presses on the panel and for wheel rolls on the panel or backdrop.
  `view_about_overlay` builds its unchanged content `Column` and calls
  `modal_overlay(base, content, Message::ToggleAbout)`. This is a pure move: the existing About
  chrome tests in `src/ui/views/tests/chrome.rs` must pass unedited. Register `mod modal;` in
  `src/ui/views/mod.rs`.
- `src/ui/views/file_info.rs` (new):
  `pub fn view_file_info_overlay<'a>(base: Element<'a, Message>, track: &'a KnownTrack) -> Element<'a, Message>`.
  It draws a `FILE INFO` heading (size 18, `theme::TITLE_TAPE`, like `ABOUT NOSTALGIA`), then
  `Title: <track.title>`, then `Artist: <track.artist>` (left out when `track.artist` is empty),
  then `Length: <track.time>` (already `--:--` for an unknown duration, via
  `views::format_track_time`), and finally `labeled_button("Close", Message::ToggleFileInfo)`. It
  wraps everything with `modal_overlay(.., Message::ToggleFileInfo)`. Export it from
  `views/mod.rs` next to `view_about_overlay`. Name the label prefixes as consts.
- `src/ui/message.rs`: add `ToggleFileInfo` beside `ToggleAbout`, with a comment in the same style.
  `src/ui/mod.rs` `update`: route it to `handlers::window::toggle_file_info`.
- `src/ui/player.rs`: add `pub(in crate::ui) file_info_visible: bool` next to `about_visible`
  (same doc style: closed at launch, not persisted), initialized `false`.
- `src/ui/handlers/window.rs`: add `toggle_file_info(player) -> Task<Message>`, modeled on
  `toggle_about`. When opening and there is no current track known to `player.known_tracks` (look
  up `state.current_track` the same way as `WinampPlayer::window_title`), it does nothing and
  returns `Task::none()`. Otherwise it flips `file_info_visible`. Opening also sets
  `about_visible = false`, and opening while `player.shaded` returns `toggle_window_shade(player)`,
  exactly as About does. In `toggle_about`, opening also sets `file_info_visible = false`, so at
  most one panel shows at a time.
- `src/ui/view.rs` `view`: after the About branch, when `player.file_info_visible && !player.shaded`
  and the current track resolves in `known_tracks`, wrap `base` in `views::view_file_info_overlay`.
  An unresolved track draws the plain base. Do not hold the `state` lock across building the
  element: copy the `current_track` id out first, or reuse a lookup already done in `view`.
- `src/ui/shortcuts.rs`: add `file_info_shortcut(physical_key, location, modifiers)`, matching the
  top-row `3` by physical key (`key::Code::Digit3`, `Location::Standard`) with Alt held and
  Control, Shift, and Logo all released. Model it on `playlist_sort_shortcut` and chain it into
  `message_for`. Mention Alt+3 in `message_for`'s doc comment.

**Files touched.** `src/ui/views/modal.rs` (new), `src/ui/views/file_info.rs` (new),
`src/ui/views/about.rs`, `src/ui/views/mod.rs`, `src/ui/message.rs`, `src/ui/mod.rs`,
`src/ui/player.rs`, `src/ui/handlers/window.rs`, `src/ui/view.rs`, `src/ui/shortcuts.rs`, tests in
`src/ui/views/tests/chrome.rs`, `src/ui/tests/window_tests.rs`, and the inline shortcut tests,
`README.md` (the keys paragraph), and `docs/invariants.md` (a new `## File info panel` section
after `## About panel`).

**Acceptance criteria.**
- Alt+3 (top row, Alt only) maps to `Message::ToggleFileInfo`. These map to nothing: Ctrl+Alt+3,
  Shift+Alt+3, Logo+Alt+3, keypad 3 with Alt (`Location::Numpad`), and a plain `3`. The inline
  shortcut tests pin each case.
- In `window_tests.rs`, with a known current track, `ToggleFileInfo` opens and then closes the
  panel and schedules no work. Playback, volume, equalizer, browse, and search state stay
  unchanged, asserted the way `toggle_about_shows_and_hides_the_panel_without_touching_playback`
  asserts them.
- With no current track, or with a current id missing from `known_tracks`, `ToggleFileInfo`
  leaves `file_info_visible` false.
- Opening File info closes About, and opening About closes File info. Opening File info while
  shaded schedules the restore and sets the flag.
- A chrome test builds `view_file_info_overlay` over a blank base and captures its text. The
  heading, `Title: …`, `Artist: …`, `Length: …`, and `Close` are present, and the `Artist:` line
  is absent for an artist-less track. `Close` and a backdrop press each publish one
  `ToggleFileInfo`. A press on the panel's text publishes nothing that closes it.
- The About chrome tests pass unchanged after the `modal_overlay` extraction.
- `view_constructs_when_the_window_is_shaded` (or a sibling test) shows that a set
  `file_info_visible` flag draws no `FILE INFO` text in shade mode.
- README and `docs/invariants.md` describe the key and the panel. `make check` passes.

## Done
