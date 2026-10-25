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

### Ctrl+Z and Ctrl+B play the first and last song of the showing list, like Winamp (planned 2026-10-24 by the plan role)

**Goal.** In classic Winamp, Ctrl+Z goes to the start of the playlist and Ctrl+B to its end.
Nostalgia's Z and B already step back and forward through the list the current view draws from
(`handlers::playback::step_track`). With the window focused, Ctrl+Z should play the first song of
that same list and Ctrl+B its last, whatever Shuffle, Repeat, and the playback position are. An
empty list does nothing. Plain Z/B and Shift+Z/B keep their current meaning. Alt and Logo chords
stay unbound.

**Approach.**
- `src/ui/message.rs`: add `FirstTrack` and `LastTrack` variants next to `NextTrack` /
  `PreviousTrack`, each with a one-line doc comment.
- `src/ui/mod.rs` `update`: route `Message::FirstTrack` to
  `handlers::playback::jump_to_edge_track(player, false)` and `Message::LastTrack` to
  `jump_to_edge_track(player, true)`, beside the `NextTrack` / `PreviousTrack` arms.
- `src/ui/handlers/playback.rs`: add `pub(in crate::ui) fn jump_to_edge_track(player: &mut
  WinampPlayer, last: bool) -> Task<Message>`. Take `(items, epoch)` from the existing
  `showing_songs` helper. When `items` is empty, return `Task::none()`. Otherwise pick index `0`
  or `items.len() - 1` and return `Task::batch([Task::done(Message::TrackSelected { epoch, index
  }), super::browse::scroll::scroll_browse_to_playing_row(player, index)])`, the same pair
  `step_track` schedules. Copy `index` and `epoch` into locals before the scroll call so the
  `showing_songs` borrow ends first. Do not take the state lock, and do not touch `play_history`,
  `seek_preview`, or the Previous restart rule. The jump plays the edge row even when it is
  already the current track, matching a direct row play.
- `src/ui/shortcuts.rs` `shortcut`: extend the Ctrl+F exception. With `modifiers.control()`
  held and neither alt nor logo held, `z` (ignoring ASCII case) maps to `Message::FirstTrack`
  and `b` to `Message::LastTrack`. These checks go before the chord guard, which still returns
  `None` for every other Control chord. Update the doc comments on `message_for` and `shortcut`
  that list the bindings and say Ctrl+F is the only Control chord.

**Files touched.** `src/ui/message.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`,
`src/ui/shortcuts.rs`, `src/ui/tests/stepping_tests.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**
- New test `message_for_maps_ctrl_z_and_ctrl_b_to_the_list_edges` in `src/ui/shortcuts.rs`
  tests:
  - `Modifiers::CTRL` and `CTRL | SHIFT` with `z`/`Z` map to `Message::FirstTrack`, and with
    `b`/`B` to `Message::LastTrack`.
  - `ALT`, `LOGO`, `CTRL | ALT`, and `CTRL | LOGO` with `z` and `b` map to `None`.
- `shortcut_ignores_chord_modifiers` stays green. It currently loops `z` over `Modifiers::CTRL`
  as well, so move `z` out of that key list and into a loop over `ALT` and `LOGO` only. Update
  its comment to name Ctrl+Z and Ctrl+B as exceptions alongside Ctrl+F.
- `message_for_maps_each_bound_letter` still sees plain Z as `PreviousTrack` and plain B as
  `NextTrack`.
- New tests in `src/ui/tests/stepping_tests.rs`, built on `player_stepping_from` and
  `assert_track_selected`:
  - `first_track_selects_the_first_song`: current `song-1`, `FirstTrack` gives epoch 0,
    index 0.
  - `last_track_selects_the_last_song`: current `song-0`, `LastTrack` gives the last index of
    `stepping_songs()`.
  - `edge_jumps_ignore_shuffle_and_repeat`: Shuffle and Repeat on in shared state, both
    messages still land on index 0 and the last index.
  - `edge_jumps_with_no_songs_loaded_do_nothing`: an empty `songs` list gives `assert_no_task`
    for both.
  - `edge_jumps_in_all_songs_use_the_flat_list`: `CurrentView::AllSongs` with `all_songs`
    filled resolves against `all_songs` and its epoch (follow
    `next_track_in_all_songs_steps_the_flat_list`).
  - `edge_jumps_scroll_the_showing_song_list`: under `CurrentView::Songs`,
    `assert_task_scrolls_browse_list_to` sees `BROWSE_LIST_PADDING` for `FirstTrack` and
    `BROWSE_LIST_PADDING + (len - 1) * BROWSE_ROW_HEIGHT` for `LastTrack`.
- `docs/invariants.md`:
  - Add a `## List edge jumps` section (Guarantee / Enforced by / Pinned by / History, in the
    same shape as `## Media keys`) that names the shortcut tests and the new stepping tests.
  - In `## Browse playing-row visibility`, add Ctrl+Z/Ctrl+B to the moves that scroll the list
    to the landed row, and name `jump_to_edge_track`.
- README Usage: after the sentence beginning "With the window focused, the classic Winamp keys
  drive playback: Z and B step back and forward", add a sentence saying Ctrl+Z plays the first
  song of the list Z and B step through and Ctrl+B plays its last song. Do not touch the brief
  or status blocks.
- `make check` passes.

## Done
