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

_None yet._

### Let Shuffle play every song once before repeating any, like Winamp (planned 2026-10-27 by the plan role)

**Goal.** Winamp's Shuffle plays the playlist in a shuffled order, so every song plays once
before any song plays again. Today a shuffled forward step (`stepping::forward_track_id`, used by
Next in `step_track` and by the preview-end advance in `handlers/playback/preview.rs`) calls
`transport::shuffled_track_id`, which picks any song except the current one. In a three-song list
it can go song-1, song-2, song-1 and never reach song-3. Fix this by tracking a shuffle *round*:
a shuffled forward step picks only from the showing list's songs not yet played this round.
Once every song in the list has played, a new round starts.

**Approach.**
- Add `src/ui/transport/round.rs` with `pub struct ShuffleRound { played: HashSet<String> }`
  (`#[derive(Debug, Default)]`), declared beside `mod history;` in `src/ui/transport.rs` and
  re-exported like `PlayHistory`. It has two methods:
  - `record(&mut self, track_id: &str, songs: &[Song])` inserts `track_id`, then drops `played`
    ids that `songs` no longer contains (so the set stays bounded by the showing list). If every
    song in a non-empty `songs` is now in `played`, the round is complete: clear `played` and
    re-insert just `track_id`, so the new round starts with the song now playing.
  - `pick(&self, songs: &[Song], current: Option<&str>, roll: u64) -> Option<String>` (`#[must_use]`)
    takes candidates in list order: songs that are neither `current` nor in `played`. If any exist,
    return `candidates[roll % len]`. Otherwise fall back to
    `transport::shuffled_track_id(songs, current, roll)`, so behavior with no unplayed song left
    (for example a one-song list, or a list whose plays were never recorded) is unchanged.
- Add `pub(in crate::ui) shuffle_round: transport::ShuffleRound` to `WinampPlayer` in
  `src/ui/player.rs`, defaulted where `play_history` is.
- `handlers::playback::track_played` (src/ui/handlers/playback.rs): in the same committed branch
  that calls `player.play_history.record(current)`, also call `player.shuffle_round.record(current,
  items)`. Here `items` is the list the current view shows (`all_songs.items` for
  `CurrentView::AllSongs`, otherwise `songs.items`). Pick it through the disjoint fields as
  `step_track` does, not through `showing_songs(player)`, so that the mutable borrow of
  `shuffle_round` does not conflict. Record every committed play, whether or not Shuffle is on: a
  song played in order before Shuffle was switched on has still been heard this round. Failed and
  superseded completions record nothing, as for the history.
- `stepping::forward_track_id` gains a `round: &transport::ShuffleRound` parameter. Its shuffle
  branch calls `round.pick(items, current, transport::shuffle_roll())`. Update its doc comment and
  both callers (`step_track` passes `&player.shuffle_round`, and the preview-end advance in
  `preview.rs` does the same next to `showing_songs(player)`; both borrows are shared). The
  preview-end advance's existing same-song filter stays as it is.
- Out of scope: Shuffle's Previous (`PlayHistory`) is unchanged, and so is Shuffle with Repeat
  off, which still keeps going into a new round rather than stopping (the documented "a shuffled
  song when another exists").

**Files touched.** `src/ui/transport/round.rs` (new, with its in-file `#[cfg(test)] mod tests`
like `history.rs`), `src/ui/transport.rs`, `src/ui/player.rs`, `src/ui/handlers/playback.rs`,
`src/ui/handlers/playback/stepping.rs`, `src/ui/handlers/playback/preview.rs`,
`src/ui/tests/stepping_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- Unit tests in `round.rs` over `test_support::stepping_songs()` (song-1..song-3):
  - `pick_skips_songs_played_this_round`: after `record("song-1")` and `record("song-2")`,
    `pick(.., Some("song-2"), roll)` is `Some("song-3")` for every roll in `0..10`.
  - `a_completed_round_starts_over_from_the_current_song`: after recording all three songs ending
    with song-3, `pick(.., Some("song-3"), roll)` over `0..10` returns both song-1 and song-2 and
    never song-3.
  - `pick_falls_back_when_no_unplayed_song_remains`: a one-song list with that song recorded and
    current returns that song's id, matching `shuffled_track_id`.
  - `record_forgets_songs_the_list_no_longer_shows`: a song recorded from a different list does
    not keep a song of the current list from being picked, and does not count toward completing
    its round.
- UI test `next_with_shuffle_plays_the_unplayed_song` in `src/ui/tests/stepping_tests.rs`: build
  the player with `player_stepping_from(None)`, turn Shuffle on, and `record_a_play` song-1 then
  song-2. `Message::NextTrack` must then select index 2, checked with `assert_track_selected`.
- A preview-end test alongside the existing Shuffle preview-end tests in `stepping_tests.rs`. It
  records song-1 and song-2 as plays, then a finished reply while song-2 plays with Shuffle on.
  The advance must land on song-3 (index 2).
- `next_with_shuffle_never_reselects_the_current_track` and the existing Shuffle/Previous
  tests pass unchanged.
- docs/usage.md's Shuffle sentences (the preview-end advance paragraph and Next's description)
  say Shuffle plays each song of the showing list once before any repeats, like Winamp. A new
  `## Shuffle rounds` section in docs/invariants.md states the guarantee, names `ShuffleRound`
  and `track_played` as the enforcement, and lists the new tests. `make check` passes.

### Flash the volume on the LCD after a key or wheel nudge, like Winamp (planned 2026-10-27 by the plan role)

**Goal.** Winamp's title display briefly reads `VOLUME: 72%` when the volume changes from the
keyboard or the mouse wheel. Today only a volume *drag* swaps the LCD title for the level, through
`player.mixer_drag` and `views::mixer_readout_text`. A nudge (`handlers::mixer::volume_up` /
`volume_down`, reached from the Up/Down arrows, keypad 8/2, and `volume_wheel`) changes the level
and leaves the title alone. After a nudge, show the drag's own `Volume: N%` text in the title's
place for about one second (four `MARQUEE_TICK`s of 250 ms), then bring the title back. The
waiting Shuffle-rounds plan touches none of these files, so either can merge first.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) volume_flash_ticks: u8` (initialized to 0 beside
  `mixer_drag`) with a doc comment. It holds the number of LCD ticks the nudge readout has left and
  is never saved. Add `pub(in crate::ui) const VOLUME_FLASH_TICKS: u8 = 4;` with a doc comment
  saying it is one second at `MARQUEE_TICK`. Add `needs_lcd_tick(&self) -> bool`, which returns
  `self.volume_flash_ticks > 0 || self.needs_marquee()`. Leave `needs_marquee` unchanged.
- `src/ui/mod.rs`: pass `player.needs_lcd_tick()` as `subscriptions`' first argument instead of
  `player.needs_marquee()`. Then the existing marquee timer also runs while a flash is counting
  down, and no new thread or subscription module is needed. Update the `subscriptions` doc
  comment in `src/ui/subscriptions/mod.rs` to say the timer also runs while a volume flash shows.
- `src/ui/handlers/mixer.rs`: `volume_up` and `volume_down` set
  `player.volume_flash_ticks = VOLUME_FLASH_TICKS` before `mutate_volume`, so `volume_wheel`
  inherits it, and its zero/NaN no-op path still sets nothing. `balance_wheel`, `eq_preamp_wheel`
  and `eq_band_wheel` set it to 0 on their non-no-op paths, so a balance or gain roll never shows a
  stale volume. `mixer_released` must **not** touch it: `mixer_key_message`
  (`src/ui/subscriptions/listeners.rs`) turns the same Up/Down key press into
  `Message::MixerReleased`, and clearing the flash there would cancel it immediately.
- `src/ui/handlers/playback.rs` `marquee_tick`: while `volume_flash_ticks > 0`, decrement it and
  leave `marquee_offset` alone, so the title resumes where it paused (as it does after a drag
  readout). Otherwise advance the offset as today.
- `src/ui/view/player_panels.rs`: in the readout chain passed to `views::view_now_playing`, after
  the `mixer_drag` readout and the `seek_preview` readout, add one more `or_else`. It shows
  `views::mixer_readout_text(views::MixerDrag::Volume, volume, balance, eq_preamp, &eq_bands)`
  while `player.volume_flash_ticks > 0`. A drag or a seek drag therefore takes precedence.

**Files touched.** `src/ui/player.rs`, `src/ui/mod.rs`, `src/ui/subscriptions/mod.rs` (doc
only), `src/ui/handlers/mixer.rs`, `src/ui/handlers/playback.rs`, `src/ui/view/player_panels.rs`.
New test file `src/ui/tests/volume_flash_tests.rs`, registered with `mod volume_flash_tests;` in
`src/ui/tests/mod.rs` (`mixer_tests.rs` is already over 1000 lines). Plus `docs/usage.md` and
`docs/invariants.md`.

**Acceptance criteria.** The tests live in `volume_flash_tests.rs` and are synchronous
`#[test]`s using `test_player`, `seed_current_title` and `root_view_draws`.
- `Message::VolumeUp` (and separately `Message::VolumeDown`) with a seeded title `Tune` makes
  the root view draw `Volume: N%` for the new level and not `Tune`. `player.needs_lcd_tick()`
  is true.
- After three `Message::MarqueeTick`s the readout still shows. After the fourth the view draws
  `Tune` again, `volume_flash_ticks` is 0, and `needs_lcd_tick()` matches `needs_marquee()`.
- With a title long enough that `needs_marquee()` holds, a nudge followed by four ticks leaves
  `marquee_offset` unchanged, and a fifth tick advances it by one.
- `Message::VolumeWheel(1.0)` shows the readout. `Message::VolumeWheel(0.0)` and
  `Message::VolumeWheel(f32::NAN)` leave `volume_flash_ticks` at 0.
- `Message::MixerReleased` sent after a nudge leaves the readout showing.
- A volume nudge followed by `Message::BalanceWheel(1.0)` clears the flash, and the view draws
  `Tune`. The same holds for `Message::EqPreampWheel(1.0)`.
- While `mixer_drag` is `Some(views::MixerDrag::Balance)` during a flash, the view draws the
  `Balance:` readout, not `Volume:`.
- The existing `dragging_volume_shows_the_level_until_release`,
  `a_key_or_wheel_event_ends_a_mixer_readout`, and the volume-wheel tests pass unchanged.
- `docs/usage.md`: in the sentence about the drag readouts ("While a volume or balance slider is
  dragged, the LCD shows its level …"), add that an up/down-arrow, keypad 8/2, or wheel volume
  nudge shows the same `Volume: N%` readout for about a second, like Winamp. `docs/invariants.md`
  `## Volume wheel`: add the flash to the guarantee, name `volume_flash_ticks` and `marquee_tick`
  as the enforcement, and list the new tests under Pinned by. `make check` passes.

## Done
