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


### Declare the project's deterministic check (planned 2026-10-25 by the plan role)

**Declares the project check**

**Goal.** Tumwater's doctor reports the `no-check` alert because the project declares no
deterministic check, so merges run without the safety gate. The project already has one: the
`Makefile`'s `check` target (`fmt lint docs test test-one-guard-test`), which the README's
Development section names as the landing gate and `.github/workflows/ci.yml` runs on every push.
Declare that exact command to tumwater.

**Approach.**
- Command: `make check`, run from the project root (it needs the ALSA headers the README's
  Development section names).
- Route: `check.command`. There is no `package.json` at the project root or above it (it is a
  Cargo crate), so the package.json `test`/`typecheck`/`build` script route does not apply. Set it
  from the team root with `tumwater config set check.command "make check"` (run in
  `/home/zach/nostalgia`; it writes `tumwater.json`, which roles must not edit, so this step is
  the user's or tumwater's own).
- Change no source, `Makefile`, or CI file: the gate already exists.

**Files touched.** `tumwater.json` (via `tumwater config set` only); nothing in the crate.

**Acceptance criteria.**
- `tumwater config get check.command` prints `make check`.
- `tumwater doctor` shows its `check` row as ok, and the `no-check` alert is gone.
- `make check` still exits 0 on main.

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

## Done
