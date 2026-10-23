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
> The equalizer-audio and seek plans this note pointed at have since landed
> (both 2026-10-20); the 2026-10-22 note below carries the current queue.

> **Steward drift note (2026-10-22 by the steward role): the built Now
> Playing bar and browse rows do not match the brief's mockups, and the two
> queued plans settle that difference in opposite directions — the
> playback-position plan adds a readout the mockup shows nowhere, while the
> album-row plan matches the mockup's `<artist> - <title>` rows but leaves the
> running time it draws out of scope.**
>
> The brief promises a "very close if not pixel-perfect" Winamp, and the plan
> role has been citing `docs/screenshots/` as the spec — the landed plans for
> the scrolling title, the artist-prefixed rows, the `TOP` button and the list
> counts each name a mockup feature. The main window's mockup
> (`docs/screenshots/player.png`) draws the classic LCD: a vertical `O C I D V`
> strip at the left, a large elapsed-time readout (`00:53`) over a green
> visualizer, a song-title box reading `<ARTIST> - <TITLE> (<length>)`, and a
> `256 KBPS 44 KHZ` / `MONO STEREO` status row. The build renders one row
> instead: `now_playing_row` (`src/ui/views/now_playing.rs`) pushes a
> `Now Playing: ` caption, the artist and title, a fill, and the track length
> at the right edge of the LCD well, and `view` (`src/ui/mod.rs`) stacks that
> bar above the transport row — no elapsed-time readout, no `O C I D V` strip,
> no status row.
>
> The first entry under `## Planned`, "Add the playback-position readout to
> the Now Playing bar (Playback position, part 3/3)", widens that right-edge
> slot to `<elapsed>/<total>`, a value the mockup shows nowhere: its title
> line carries the length in parentheses and the elapsed time in the big left
> readout. Recommend the plan role schedule that elapsed-time display and
> settle where part 3/3's readout belongs before the queue drains.
>
> The browse rows drift the same way: `docs/screenshots/library.png` draws
> each artist row with its album count at the right (`2 albums`) and each
> album row with its running time (`12:38`), while the build shows the
> `View Albums` and `View Songs` hints (`src/ui/views/browse.rs`) and the
> total count beneath the rows. The queued album-row plan, "Show each album
> row's artist beside its title", puts album running times out of scope
> because an album query does not supply its songs' durations, so the
> mockup's per-album time would need a songs fetch per row — a decision for
> the plan role, not a detail to drop.

## Planned

_None yet._

### Add a draggable seek slider beneath the Now Playing LCD (planned 2026-10-23 by the plan role)

**Goal.** Fill the horizontal groove `docs/screenshots/player.png` draws between the LCD well and the transport row: a chrome slider whose thumb tracks the loaded preview's playback position, and which seeks when the user drags and releases it. The bar spans the loaded preview's own backend-reported length rather than the catalog track's length, because a preview asset can be shorter than the song (the right-edge LCD total keeps showing the catalog length). It is drawn only while the backend reports both a position and a preview length — the production `rodio` backend does; the silent fallback does not — and otherwise renders as an empty fixed-height strip so the layout does not jump. Dragging moves only the thumb; releasing sends exactly one absolute seek. This is independent of the queued sample-rate/channel status row: that plan adds a row inside the LCD well and a format query, while this one adds a sibling strip below the well plus a duration query and an absolute-seek seam. Both plans edit `position_tick` and `reset_position_readout`, so whichever lands second extends the existing batch and clear rather than replacing them.

**Approach.**

- `src/audio.rs`: add `fn seek_to(&self, position: Duration) -> Result<(), AppleMusicError>` and `fn duration(&self) -> Option<Duration> { None }` to `AudioOutput`. Document `seek_to` as an absolute seek of the current player, clamped by the backend to the loaded preview's length and doing nothing while stopped; document `duration` as the loaded preview's own length, answering `None` when no player is loaded or the backend cannot answer, with the default keeping existing fakes compiling. `SilentOutput::seek_to` logs and returns `Ok`; `duration` inherits `None`.
- `src/audio/player.rs`:
  - Add `Command::SeekTo(Duration)` and `Command::Duration(Sender<Option<Duration>>)`.
  - Add `fn seek_to(&self, position: Duration)` and `fn duration(&self) -> Option<Duration>` to `WorkerPlayer`.
  - Add `pub(super) fn duration_for_source(source: &impl rodio::Source) -> Option<Duration>` beside `seek_target`, returning `source.total_duration()`. Give `RodioPlayer` a `duration: OnceLock<Option<Duration>>`; in `append`, set it from `duration_for_source(&preview)` before the preview is wrapped and moved. `RodioPlayer::duration` returns the stored value, and `RodioPlayer::seek_to` clamps the target to that value (a `clamp_seek(position, duration)` free function beside `seek_target`, with `None` meaning no clamp) before calling `self.player.try_seek`, logging a rejected seek without changing state.
  - In `serve_commands`, handle `Command::SeekTo(position)` by calling `player.seek_to(position)` only when a player is installed, and `Command::Duration(reply)` by sending `player.as_ref().and_then(WorkerPlayer::duration)`.
  - Implement `AudioOutput::seek_to` on `RodioOutput` by sending `Command::SeekTo`, and `AudioOutput::duration` by sending `Command::Duration` and waiting on the reply with the existing `POSITION_QUERY_TIMEOUT` (`None` when the send fails or the wait times out). Extend that constant's doc to say it bounds the position and duration queries, without renaming it.
- `src/apple_music/playback.rs`: add `pub(crate) fn seek_output_to(&self, position: Duration) -> Result<(), AppleMusicError>` forwarding to `self.audio.seek_to(position)`, and `pub(crate) fn playback_duration(&self) -> Option<Duration>` forwarding to `self.audio.duration()`. Document both as synchronous and bounded by the seam, like `playback_position`.
- `src/ui/views/seek_bar.rs` (new small module): add `pub fn seek_bar_value(position: Option<Duration>, duration: Option<Duration>) -> Option<(f32, f32)>`, returning `(thumb seconds, length seconds)` with the thumb clamped into `0..=length` and a missing position treated as zero, or `None` when the duration is absent or zero. Add `pub fn view_seek_bar(position: Option<Duration>, duration: Option<Duration>) -> Element<'static, Message>`: when `seek_bar_value` answers, build a `Slider::new(0.0..=length, thumb, Message::SeekPreviewChanged)` with `.step(1.0)`, `.on_release(Message::SeekPreviewReleased)`, `Length::Fill` width, and `style::chrome_slider_style`; when it does not, build an empty `Space` of the same fixed height. Wrap either in a fixed-height `Container` (`SEEK_BAR_HEIGHT = 16.0`) so the strip never changes height. Construct the `Slider` here rather than widening the fixed-width `views::chrome_slider` helper, because this slider fills the row.
- `src/ui/views/mod.rs`: declare `mod seek_bar;` and re-export `seek_bar_value`/`view_seek_bar` beside the other view builders.
- `src/ui/mod.rs`: add `playback_duration: Option<Duration>` and `seek_preview: Option<Duration>` to `WinampPlayer`, both initialized `None` and documented as the backend-reported preview length and the in-progress drag. Add `Message::PlaybackDuration { epoch: u64, duration: Option<Duration> }`, `Message::SeekPreviewChanged(f32)`, and `Message::SeekPreviewReleased`, routed from the `update` table to `handlers::playback`. In `view`, push `views::view_seek_bar(player.seek_preview.or(player.playback_position), player.playback_duration)` after `view_now_playing` and before the transport row; both fields are plain player state, so no state lock is needed.
- `src/ui/handlers/playback.rs`:
  - `position_tick` batches the existing position query with a second query of a cloned service's `playback_duration()` mapped to `Message::PlaybackDuration`, and returns `Task::batch`; both are skipped while `pending_position_transitions` is nonzero.
  - `playback_duration` stores the reply only when `epoch == player.position_epoch` and `pending_position_transitions == 0`, mirroring `playback_position`.
  - `seek_preview_changed` stores `Duration::from_secs_f32(value.max(0.0))` in `seek_preview` and returns no task, so dragging never touches the backend.
  - `seek_preview_released` takes the stored drag: when it is `Some(position)`, call `player.apple_music_service.seek_output_to(position)`; on success bump `position_epoch` (dropping any position or duration reply already in flight), set `playback_position = Some(position)`, and clear `seek_preview`; on failure log the backend error and clear `seek_preview` so the next poll restores the real position. When no drag is stored it only clears the field.
  - `reset_position_readout` also clears `playback_duration` and `seek_preview`, so Stop and a new selection remove the bar's scale and any drag with the position.
- Tests. `src/audio/tests.rs`: extend `every_command` with `seek_to` (and the closed-worker and command-mapping tests with it); add `silent_output_reports_no_playback_duration` asserting `duration().is_none()`; add `duration_for_source_reads_a_samples_buffers_length`, `clamp_seek_clamps_to_the_preview_length_and_passes_through_without_one`, `rodio_output_asks_the_worker_for_its_duration`, `rodio_output_reports_none_when_the_duration_reply_times_out`, `seek_to_command_reaches_the_loaded_player`, `duration_query_returns_the_loaded_players_duration`, and `duration_query_reports_none_when_nothing_is_loaded`; give `RecordingPlayer` a settable absolute position and duration, make `seek_to` set the position, and keep its no-seek position at `42s` for the existing position tests. `src/test_support.rs`: add `AudioCall::SeekTo(Duration)`, record it in `RecordingAudio::seek_to`, return a fixed `Some(Duration::from_secs(30))` from `RecordingAudio::duration`, and implement `FailingAudio::seek_to` through the existing `FailingCommand::Seek` with `duration` answering `None`. `src/apple_music/tests.rs`: add `seek_output_to_forwards_an_absolute_position`, `seek_output_to_propagates_an_audio_backend_failure`, and `playback_duration_forwards_the_audio_output_duration`. `src/ui/views/tests.rs`: add `seek_bar_value_clamps_the_thumb_and_hides_without_a_length` (position 45s/length 30s → `(30.0, 30.0)`, `None` position → `(0.0, 30.0)`, absent/zero length → `None`) and call `view_seek_bar` with a known and an unknown length to pin that both construct without panicking. `src/ui/tests.rs`: add `playback_duration_reply_stores_the_loaded_previews_length`, `playback_duration_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`, `stop_and_track_selection_clear_the_playback_duration`, `seek_preview_changed_tracks_the_drag_without_touching_the_backend`, `seek_preview_released_seeks_once_to_the_dragged_position` (asserting one `AudioCall::SeekTo`, the optimistic `playback_position`, and a bumped epoch), `seek_preview_released_without_a_drag_does_nothing`, `seek_preview_released_invalidates_an_in_flight_position_reply`, and `position_tick_queries_position_and_duration` (drive the batched task's stream to its end and assert both replies arrive).
- `README.md`: in the Usage paragraph that describes the Now Playing bar, say a slider beneath it tracks the loaded preview and drags to seek, spans the preview's own length, and stays blank when the backend reports no length. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the `## Now Playing label` section, add the seek-slider guarantee (the bar measures the backend-reported preview length, the thumb follows the stored position or the drag in progress, only a release sends `AudioOutput::seek_to`, the seek is clamped to that length, a successful seek records the position optimistically and invalidates in-flight replies, a failed seek lets the next poll correct the thumb, and Stop/new selection clear the length and drag) and name `seek_bar_value`, `view_seek_bar`, the `SeekPreviewChanged`/`SeekPreviewReleased`/`PlaybackDuration` handlers, `seek_output_to`, and the duration query as the enforcers. Add the new test names to the Pinned-by list.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/views/seek_bar.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `seek_bar_value` returns `(30.0, 30.0)` for a 45-second position on a 30-second preview, `(0.0, 30.0)` for a missing position, and `None` for a missing or zero preview length; the thumb never exceeds the bar's length.
- The slider is drawn beneath the LCD only while both a position and a positive preview length are reported; with no preview, the silent backend, Stop, or a new selection the strip is blank and keeps its fixed height.
- Dragging the slider stores the drag value and sends no backend command; releasing sends exactly one absolute `seek_to` with the dragged position, which the backend clamps to the preview length, and the bar shows that position until the next backend reply.
- Stop and a new selection clear the stored preview length and any drag immediately; a `PlaybackDuration` reply stamped with an older epoch or landing while a transport transition is in flight is dropped.
- `position_tick` asks the backend for both the position and the duration, and asks for neither while a transport transition is in flight.
- The right-edge LCD total continues to show the catalog track's length, not the preview length, and the README and `docs/invariants.md` describe the slider that way; `make check` passes (fmt, clippy, rustdoc, tests).

## Done
