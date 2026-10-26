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

### Keypad 1 and 3 jump ten songs back or forward, like Winamp (planned 2026-10-25 by the plan role)

**Goal.** Winamp's keypad 1 jumps ten songs back in the playlist and keypad 3 ten songs forward.
Nostalgia binds keypad 2 and 4-9 (`shortcuts::numpad_shortcut`) but leaves 1 and 3 unbound, and
no message moves more than one song at a time. Bind them.

**Approach.**
- `src/ui/transport.rs`: add `pub const JUMP_TRACK_COUNT: usize = 10;` and a pure
  `#[must_use] pub fn jumped_track_index(songs: &[Song], current: Option<&str>, forward: bool) -> Option<usize>`:
  `None` for an empty list; with `current` found at index `i`, forward gives
  `min(i + JUMP_TRACK_COUNT, len - 1)` and backward `i.saturating_sub(JUMP_TRACK_COUNT)`. It clamps
  at the list edges and never wraps. With no `current` or one not in `songs`, forward gives `0` and
  backward gives `len - 1`, the same entry points `next_track_id` / `previous_track_id` use.
- `src/ui/message.rs`: add `JumpTracksBack` and `JumpTracksForward` after `LastTrack`, each with a
  one-line comment like its neighbours ("Keypad 1: play the song ten before the current one …").
- `src/ui/handlers/playback.rs`: add `jump_ten_tracks(player, forward)`. It reads
  `state.current_track` under `player.state.blocking_lock()`, gets the index from
  `transport::jumped_track_index` over `showing_songs(player)`'s list, and returns the same
  `Task::batch([Task::done(Message::TrackSelected { epoch, index }), scroll_browse_to_playing_row(player, index)])`
  that `jump_to_edge_track` returns. It returns `Task::none()` when there is no index. Drop the
  lock before building the scroll task, by cloning the current id out first or scoping the
  borrow. Like `jump_to_edge_track`, it ignores Shuffle, Repeat and the Previous restart rule,
  and it leaves `play_history` alone.
- `src/ui/mod.rs` `update`: route the two messages to `jump_ten_tracks(player, false/true)` next
  to the `FirstTrack`/`LastTrack` arms.
- `src/ui/shortcuts.rs` `numpad_shortcut`: map `"1"` to `JumpTracksBack` and `"3"` to
  `JumpTracksForward`, and update its doc comment (only keypad 0 and the operators stay unbound).
- Docs: in the README keys paragraph, after the keypad sentence, add "and 1 and 3 jump ten songs
  back or forward through the same list, stopping at its ends". In `docs/invariants.md`
  `## Numpad keys`, add 1 and 3 to the guarantee, make "keypad 0/1/3" read "keypad 0", and add a
  `## Ten-song jumps` section (Guarantee / Enforced by / Pinned by / History) shaped like
  `## List edge jumps`.

**Files touched.** `src/ui/transport.rs`, `src/ui/message.rs`, `src/ui/handlers/playback.rs`,
`src/ui/mod.rs`, `src/ui/shortcuts.rs`, new `src/ui/tests/jump_tests.rs` (registered in
`src/ui/tests/mod.rs`; `stepping_tests.rs` is already ~880 lines), `README.md`,
`docs/invariants.md`.

**Tests.** `stepping_songs()` has three songs, which is too few for a ten-song jump. Build a
local `numbered_songs(count)` helper (cloning `sample_song()` with ids `song-0`… and titles to
match) in the transport unit tests and in `jump_tests.rs`.
- Transport unit tests (`src/ui/transport.rs` `mod tests`): with 25 songs, forward from index
  2 gives 12 and backward from 12 gives 2. Forward from 20 clamps to 24, and backward from 4
  clamps to 0. No or unknown current gives 0 forward and 24 backward. An empty list gives `None`.
- `jump_tests.rs`: `jump_forward_selects_ten_songs_on` and `jump_back_selects_ten_songs_back`
  (via `assert_track_selected`); `jumps_clamp_at_the_list_ends`;
  `jumps_ignore_shuffle_and_repeat` (with both set, the result is the same as unset);
  `jumps_with_no_songs_loaded_do_nothing` (`assert_message_schedules_no_work`);
  `jumps_in_all_songs_use_the_flat_list` (epoch 1, as in `edge_jumps_in_all_songs_use_the_flat_list`).
- `src/ui/shortcuts.rs`: extend `message_for_maps_the_numpad_transport_digits` with 1 and 3
  (plain and Shift). Change `message_for_leaves_other_digits_unbound` so the keypad list is
  `["0", "+"]` and the top-row list also checks `"1"` and `"3"`.

**Acceptance criteria.**
- With NumLock on, keypad 1 and 3 (plain or with Shift) send `JumpTracksBack` /
  `JumpTracksForward`. Top-row 1/3, keypad 0, and Ctrl/Alt/Logo chords stay unbound.
- A jump plays the song ten places from the current track in the showing list, clamped to its
  first or last song, and scrolls the list to it. Shuffle, Repeat and the playback position do not
  change it. An empty list schedules nothing.
- README and `docs/invariants.md` describe the new keys. `make check` passes.

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

## Done
