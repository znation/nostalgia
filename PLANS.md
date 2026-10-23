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

### Show the loaded preview's sample rate and channel mode in the Now Playing LCD (planned 2026-10-23 by the plan role)

**Goal.** Fill the real, queryable part of the mockup's LCD status line (`docs/screenshots/player.png`): while a preview is loaded, show its sample rate and mono/stereo mode in a small green row beneath the Now Playing title (`44 KHZ  STEREO`), and leave that row blank when no preview or backend reports a format. The value is presentation only and follows the loaded preview: Pause keeps it, Stop and a new selection clear it. Bitrate stays out of scope because `download_and_decode` discards the downloaded body length, so `256 KBPS` cannot be computed without a separate preview-metadata change. This is a separate LCD concern from the queued elapsed/remaining toggle — that plan changes the large left readout and `KnownTrack`, while this one adds an audio query and the status row — and the two are additive edits at distinct insertion points in the same files.

**Approach.**

- `src/audio.rs`: add `pub struct PlaybackFormat { pub sample_rate_hz: u32, pub channels: u16 }` (derive `Debug, Clone, Copy, PartialEq, Eq`, with doc comments on the type and both fields). Add `fn playback_format(&self) -> Option<PlaybackFormat> { None }` to `AudioOutput`, documented as reporting the loaded preview's format and answering `None` when no player is loaded or the backend cannot answer; the default keeps `SilentOutput` and every existing test fake compiling, and `SilentOutput` inherits the `None` answer.
- `src/audio/player.rs`:
  - Import `rodio::Source` and add `pub(super) fn format_for_source(source: &impl rodio::Source) -> PlaybackFormat` beside `seek_target`, reading `source.sample_rate().get()` and `source.channels().get()`. Give `RodioPlayer` a `format: OnceLock<PlaybackFormat>` initialized in `new`; in `append`, call `format_for_source(&preview)` before moving the preview and set the cell (each track builds a fresh player, so it is set once). Add `fn playback_format(&self) -> Option<PlaybackFormat>` to `WorkerPlayer` (default `None`, so the recording fake compiles) and have `RodioPlayer` return the stored copy.
  - Add `Command::PlaybackFormat(Sender<Option<PlaybackFormat>>)`. In `serve_commands`, answer it with `player.as_ref().and_then(WorkerPlayer::playback_format)`, so a stopped or failed player has no format.
  - Implement `AudioOutput::playback_format` on `RodioOutput` by sending the command and waiting on the reply with the existing `POSITION_QUERY_TIMEOUT` (`None` when the send fails or the wait times out). Extend that constant's doc to say it bounds both queued backend queries, since a download can hold the worker past it.
- `src/apple_music/playback.rs`: add `pub(crate) fn playback_format(&self) -> Option<audio::PlaybackFormat>` forwarding to `self.audio.playback_format()`, documented as synchronous and bounded by the seam (like `playback_position`).
- `src/ui/views/now_playing.rs`:
  - Add `pub fn format_playback_status(format: PlaybackFormat) -> String`, returning `"{khz} KHZ  {mode}"` where `khz = sample_rate_hz / 1000` and `mode` is `"MONO"` for one channel and `"STEREO"` for any other count. Document the integer-kilohertz truncation and that previews are mono or stereo.
  - Add `pub(super) fn now_playing_status_row<'a, Theme, Renderer>(format: Option<PlaybackFormat>) -> Row<'a, Message, Theme, Renderer>` with the same text-catalog and renderer bounds as `now_playing_row`. It is a fixed-height row (16px) with a `Length::Fill` spacer then a small (12px) `theme::LCD_GREEN` status text right-aligned; with `None` it draws no text but keeps the height, so the bar does not jump when a preview loads.
  - Change `view_now_playing` to take a sixth `format: Option<PlaybackFormat>` argument and wrap `Column::new().push(now_playing_row(...)).push(now_playing_status_row(format))` in `bevel::lcd_well`, so the status line sits beneath the title/total inside the same LCD well. Keep `now_playing_row` itself unchanged.
- `src/ui/mod.rs`: add `playback_format: Option<PlaybackFormat>` to `WinampPlayer`, initialized `None` and documented as the loaded preview's backend-reported format (Pause keeps it; Stop and a new selection clear it). Add `Message::PlaybackFormat { epoch: u64, format: Option<PlaybackFormat> }` with a doc comment naming the epoch guard, route it from the `update` table to `handlers::playback::playback_format`, and add the field to the tuple `view` resolves under the state lock before passing it to `views::view_now_playing`.
- `src/ui/handlers/playback.rs`: in `position_tick`, after the pending-transition guard, build the existing position query and a second query of a cloned service's `playback_format()` mapped to `Message::PlaybackFormat`, and return `Task::batch([position, format])`; both are therefore skipped while a transition is in flight. Add `pub(in crate::ui) fn playback_format(player, epoch, format) -> Task<Message>` that stores `format` only when `epoch == player.position_epoch` and `pending_position_transitions == 0`. In `reset_position_readout`, also set `player.playback_format = None`, so Stop and a new selection clear the row immediately; Pause deliberately leaves the existing reset path alone.
- Tests. `src/audio/tests.rs`: add `format_for_source_reads_sample_rate_and_channels` over a mono and a stereo `SamplesBuffer`; give `RecordingPlayer` a `playback_format` returning a fixed format and add `playback_format_query_returns_the_loaded_players_format` and `playback_format_query_reports_none_when_nothing_is_loaded`; add `rodio_output_asks_the_worker_for_its_format`; extend `silent_output_reports_no_playback_position` (or a sibling) to assert `playback_format().is_none()`, and extend `rodio_output_reports_a_closed_worker_for_every_command` with a `None` format answer. `src/test_support.rs`: have `RecordingAudio::playback_format` return `Some(PlaybackFormat { sample_rate_hz: 44_100, channels: 2 })`; `src/apple_music/tests.rs`: add `playback_format_forwards_the_audio_output_format`. `src/ui/views/tests.rs`: import the new helpers and cover `format_playback_status_formats_khz_and_channel_mode` (`44_100`/2 → `44 KHZ  STEREO`, `44_100`/1 → `44 KHZ  MONO`, `48_000`/2 → `48 KHZ  STEREO`) and `now_playing_status_row_draws_the_format_or_nothing` by laying out the row with the existing null renderer and reading its text; update the three `view_now_playing` calls in `now_playing_bar_and_browse_action_buttons_construct` with the new argument. `src/ui/tests.rs`: add `playback_format_reply_stores_the_loaded_previews_format`, `playback_format_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`, `stop_clears_the_playback_format_while_pause_keeps_it`, and `position_tick_queries_position_and_format` (drive the batched task's stream to its end and assert both reply messages arrive).
- `README.md`: in the Usage sentence that describes the Now Playing bar, say a loaded preview's sample rate and mono/stereo mode appear in a small status row beneath the title when the backend reports them. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the `## Now Playing label` section, add the status-row bullet (format shown only while a preview is loaded, `44 KHZ  STEREO`/`MONO` from the backend's own source, blank when absent, kept across Pause, cleared by Stop/new selection, stale/pre-transition replies dropped, bitrate not shown) and name `PlaybackFormat`, `format_for_source`, `views::format_playback_status`/`now_playing_status_row`, and the `PlaybackFormat` handler plus `reset_position_readout` as the enforcers. Add the new test names to the Pinned-by list.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/views/now_playing.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `format_playback_status` renders `44 KHZ  STEREO` for a 44.1 kHz stereo format, `44 KHZ  MONO` for one channel, and `48 KHZ  STEREO` for 48 kHz.
- The LCD draws that text in a small green row beneath the title only while the backend reports a format; with no reported format the row is blank and keeps the bar's height.
- Stop and a new selection clear the status text immediately; Pause keeps it; a reply stamped with an older epoch or landing while a transport transition is in flight is dropped.
- `position_tick` asks the backend for both the position and the format, and asks for neither while a transport transition is in flight.
- No bitrate is displayed; the README and `docs/invariants.md` say the row carries sample rate and channel mode only, and `make check` passes (fmt, clippy, rustdoc, tests).

## Done
