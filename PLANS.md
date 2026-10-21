# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; updated 2026-10-20).** The initial prompt
> names two commitments — a "very close if not pixel-perfect" classic Winamp UI
> and Apple Music as the library. The first is now moving: the base-skin
> palette landed ("Add a Winamp 2.x base-skin palette and apply it as the app
> theme", done 2026-10-06), and the reusable bevel layer followed ("Add a
> Winamp two-tone bevel layer and frame the Now Playing and equalizer panels",
> done 2026-10-06), framing those two panels through `bevel::lcd_well` and
> `bevel::raised_panel` in `src/ui/bevel.rs`. The transport-button chrome plan
> landed ("Style the transport buttons as raised Winamp chrome", done
> 2026-10-07), and both transport toggles are reachable: the Repeat toggle
> landed 2026-10-06 and the Shuffle toggle 2026-10-08, and their R and S
> keyboard bindings followed ("Add the R and S keyboard bindings for Repeat
> and Shuffle", done 2026-10-20). The rest of widget-level fidelity has
> landed too: slider chrome ("Style the volume and
> equalizer sliders as sunken Winamp grooves with raised chrome thumbs", done
> 2026-10-07), a custom title bar ("Add a Winamp custom title bar and drop the
> OS window frame", done 2026-10-07), and playlist chrome ("Frame the browse
> list as a sunken Winamp playlist editor", done 2026-10-07). The library side
> is real:
> `AppleMusicService` answers its browse and search queries through the REST
> client once a `MusicKit` session is stored, falling back to
> `sample_library()` otherwise — playlist queries included — and `play_track`
> plays the selected preview through the audio seam. Risk: the Winamp chrome
> arrived piecemeal, so later changes still need one consistent vocabulary.
> Recommend the plan loop keep scheduling the open equalizer and seek plans.

## Planned

### Seek the playing preview five seconds with the left and right arrow keys (planned 2026-10-20 by the plan role)

**Goal.** Pressing Left seeks the playing preview back five seconds and pressing Right seeks it forward five seconds, matching classic Winamp's keyboard bindings. Today `shortcuts::shortcut` maps only ArrowUp/ArrowDown (to `Message::VolumeUp`/`VolumeDown`), `Message` has no seek variant, and `AudioOutput` exposes only play/pause/resume/stop/`set_volume`/`set_balance`. `rodio` 0.22's `Player` already provides `get_pos` and `try_seek`, but nothing in `src/audio.rs` calls them, so the left/right keys currently do nothing.

**Approach.**

- **Seek-time channel phase.** rodio 0.22.2's `Source::try_seek` implementations leave the next sample on the same channel as before the seek: `SamplesBuffer::try_seek` offsets the target by the current channel and `SymphoniaDecoder::try_seek` re-advances to the active channel. The wrappers' cursors must stay in that phase. `BalanceSource::try_seek` already only delegates, so leave it that way and say in its doc comment why it must not reset `channel`; `EqSource::try_seek` currently assigns `self.channel = 0` after seeking, so remove that assignment but keep `rebuild_filters()` so a seek still clears the filter ringing.
- `src/audio.rs`: add `fn seek_by(&self, offset_secs: i64) -> Result<(), AppleMusicError>` to `AudioOutput`, documented as a relative move (positive seeks forward, negative backward), a no-op when no player is loaded, and a non-blocking channel send like `set_volume`. Implement it for `SilentOutput` (log and return `Ok`), add `Command::SeekBy(i64)`, and add `fn seek_by(&self, offset_secs: i64)` to `WorkerPlayer`. Add a private pure `fn seek_target(position: Duration, offset_secs: i64) -> Duration` that saturating-adds or saturating-subtracts `Duration::from_secs(offset_secs.unsigned_abs())`, so a backward seek past the start lands at zero. `RodioPlayer::seek_by` reads `self.player.get_pos()`, calls `self.player.try_seek(seek_target(position, offset_secs))`, and logs a `try_seek` error with `eprintln!` without changing any state. `serve_commands` handles `Command::SeekBy` by calling `player.seek_by(offset_secs)` only when a player is installed; the offset is not remembered across players, so a seek while stopped does nothing.
- `src/test_support.rs`: add `AudioCall::SeekBy(i64)`, record it in `RecordingAudio::seek_by`, and return `Ok(())` from `FailingAudio::seek_by`.
- `src/apple_music.rs`: add `pub(crate) fn seek_output(&self, offset_secs: i64) -> Result<(), AppleMusicError>` beside `set_output_volume` and `set_output_balance`, forwarding to `self.audio.seek_by(offset_secs)` and touching no shared state.
- `src/apple_music/tests.rs`: implement `seek_by` for `GatedAudio` and `RejectingAudio`, recording `AudioCall::SeekBy` and succeeding (like their other non-rejected commands). Add `seek_output_forwards_to_the_audio_backend`, asserting one `AudioCall::SeekBy(-5)` and that `AppState` is unchanged.
- `src/ui/mod.rs`: add `SeekBackward` and `SeekForward` variants beside `VolumeUp`/`VolumeDown`, and add a private `seek_preview(player, offset_secs)` helper that calls `player.apple_music_service.seek_output(offset_secs)`, logs a backend error like `mutate_volume`, and returns `Task::none()`. Route the two new arms through it with `-views::SEEK_STEP_SECS` and `views::SEEK_STEP_SECS`.
- `src/ui/views/mod.rs`: add `pub(super) const SEEK_STEP_SECS: i64 = 5;` beside `VOLUME_STEP`, documenting Winamp's five-second seek step.
- `src/ui/shortcuts.rs`: map `Key::Named(key::Named::ArrowLeft)` to `Message::SeekBackward` and ArrowRight to `Message::SeekForward`, updating the `message_for`/`shortcut` doc lists. Extend `message_for_maps_the_arrow_keys` and add both arrows to `shortcut_ignores_chord_modifiers`.
- `src/ui/tests.rs`: add `seek_messages_drive_the_audio_backend`, driving both messages through `update` over a `RecordingAudio` and asserting one `AudioCall::SeekBy(-SEEK_STEP_SECS)` and one `SeekBy(SEEK_STEP_SECS)` with no scheduled task.
- `src/audio.rs` tests: extend `silent_output_reports_success_for_every_command`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` with `seek_by`; add `seek_target_moves_forward_and_backward` (10s +5 → 15s, 10s -5 → 5s) and `seek_target_saturates_at_the_start` (3s -5 → 0); extend the recording `WorkerPlayer` with `seek_by` and `transport_volume_and_balance_commands_drive_the_installed_player` with a `Command::SeekBy(-5)` before its `Stop`, asserting the installed player received it. Change the `SeekableSource` test double's `try_seek` to land on the target frame plus its current channel instead of resetting to a frame boundary, so it models rodio's contract. Rewrite `eq_source_seek_clears_filter_state` to seek from a frame boundary and assert the channel is unchanged, and add `eq_source_seek_keeps_the_channel_cursor` and `balance_source_seek_keeps_the_channel_cursor`, which seek from mid-frame and assert the next sample is still treated as the same channel.
- `README.md`: change the arrow-key sentence to say the up/down arrows nudge the volume and the left/right arrows seek five seconds back or forward.

**Files touched.** `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views/mod.rs`, `src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `seek_target` moves forward and backward by the requested seconds and clamps a backward seek past the start at zero.
- `RodioOutput::seek_by` sends `Command::SeekBy` carrying the same signed offset; `SilentOutput::seek_by` returns `Ok`; a closed worker reports "the audio thread is gone".
- `serve_commands` applies `SeekBy` only to an installed player; the recording player receives `-5` from the extended transport test.
- `AppleMusicService::seek_output` records exactly one `AudioCall::SeekBy` and leaves `AppState` unchanged.
- `message_for` maps ArrowLeft to `Message::SeekBackward` and ArrowRight to `Message::SeekForward`; Control, Alt, and Logo chords map neither.
- Driving both seek messages through `update` records exactly one `AudioCall::SeekBy(-5)` and one `AudioCall::SeekBy(5)` and schedules no task.
- The README documents the left/right five-second seek keys.
- `EqSource::try_seek` clears filter state without changing `channel`, and `BalanceSource::try_seek` leaves `channel` unchanged; a mid-frame seek through either wrapper keeps the next sample on the same channel, and the updated `SeekableSource` models that contract.
- The implementation diff against the pre-change commit changes only the nine files listed above for the feature itself (the move of this entry to `PLANS-DONE.md` aside), and the README hunk is limited to the arrow-key sentence.

### Make Previous retrace the play history when Shuffle is on (planned 2026-10-20 by the plan role)

**Goal.** With Shuffle on, pressing Previous returns to the track actually played before the current one, retracing the play order like classic Winamp, instead of stepping one row backward in the visible list. With Shuffle off, Previous keeps its sequential and Repeat-aware behavior. Today `step_track` in `src/ui/mod.rs` sends only a forward step through `transport::shuffled_track_id`; every backward step goes through `transport::previous_track_id`. The `step_track` doc comment and `previous_with_shuffle_still_steps_backward` in `src/ui/tests.rs` pin that Previous ignores Shuffle because no play history exists.

**Approach.**

- `src/ui/transport.rs`: beside `shuffled_track_id`, add `pub(super) const HISTORY_LIMIT: usize = 100` and `pub struct PlayHistory { entries: Vec<String>, cursor: Option<usize> }` deriving `Debug` and `Default`. Add two methods:
  - `pub fn record(&mut self, track_id: &str)`: when the cursor already names `track_id`, do nothing, so a history jump's completion or a re-press of the current row neither duplicates the entry nor moves the cursor; otherwise truncate entries after the cursor, push `track_id`, point the cursor at it, then drop the oldest entries while more than `HISTORY_LIMIT` remain, keeping the cursor on the same track.
  - `pub fn previous(&mut self, songs: &[Song], current: Option<&str>) -> Option<String>`: start just before the cursor when the cursor names `current` (otherwise at the end of the entries), scan backward for the first entry whose id is in `songs`, point the cursor at it, and return its id; return `None` and leave the cursor unchanged when no earlier usable entry remains. Skipping entries absent from `songs` lets a browse to another album fall back instead of selecting a row the visible list does not contain.
- `src/ui/mod.rs`: add `play_history: transport::PlayHistory` to `WinampPlayer` and initialize it with `Default::default()` in `WinampPlayer::new`. Change `step_track` to take `&mut WinampPlayer`: when `!forward && state.shuffle`, first try `player.play_history.previous(items, state.current_track.as_deref())`, use its id when present, and otherwise fall through to `transport::previous_track_id` as today; the forward Shuffle branch and both sequential branches stay as they are. In the `Message::TrackPlayed` arm, inside the existing current-generation guard, record `state.current_track` only when `state.is_playing` is true, so a failed play never enters the history; a history jump's completion records the target its cursor already names, which `record` treats as a no-op. Update the `step_track` doc comment to describe the history-backed Previous behavior instead of "Previous ignores Shuffle".
- `src/ui/tests.rs`: replace `previous_with_shuffle_still_steps_backward` with tests for the new behavior, and drive `Message::TrackPlayed` to exercise the recording path. Cover a shuffled Previous that retraces two recorded plays, a shuffled Previous with no usable history that falls back to the sequential row, a history entry absent from the visible list being skipped, and a `TrackPlayed` with `is_playing` false recording nothing.
- `src/ui/transport.rs` tests: cover recording order and the repeated-current no-op, walking backward on repeated `previous` calls, skipping ids absent from the supplied `songs`, returning `None` without moving at the start of history, dropping forward entries when a new track is recorded after a back step, and the `HISTORY_LIMIT` bound.
- `README.md`: in the Usage paragraph, say that with Shuffle on Previous retraces the tracks played before the current one.
- `docs/invariants.md` has no transport-stepping section (its headings cover the browse hierarchy, browse reply freshness, and the Now Playing label), so this plan leaves it unchanged.

**Files touched.** `src/ui/transport.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `PlayHistory::record` keeps chronological play order, treats a repeated current id as a no-op, drops forward entries after a back step when a new track is recorded, and keeps at most `HISTORY_LIMIT` entries.
- `PlayHistory::previous` returns the nearest earlier entry present in `songs`, walks further back on repeated calls, skips entries from a list the player no longer shows, and returns `None` without moving when no earlier usable entry exists.
- With Shuffle on, Previous selects the recorded previous track rather than the row before the current one; with no usable history it falls back to `previous_track_id`. With Shuffle off, Previous still follows `previous_track_id` and the shared Repeat flag.
- A current-generation `TrackPlayed` with `is_playing` true records the committed track; one with `is_playing` false, or a superseded generation, records nothing.
- The new tests named in the approach pass alongside the existing Next/Previous wiring tests, and the README documents the Shuffle-on Previous behavior.

## Done
