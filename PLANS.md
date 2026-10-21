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

_None yet._

### Add the playback-position readout to the Now Playing bar (Playback position, part 1/2: planned 2026-10-21 by the plan role)

**Goal.** The audio seam can report where the loaded preview is, so the Now Playing bar can later show elapsed against total. `AudioOutput::position()` answers `Option<Duration>`: `Some` with the loaded player's position (including `Some(Duration::ZERO)` when the worker has no player), or `None` when the backend cannot answer (the silent fallback, a default test stub, or a query the worker cannot reach within the seam's bound). `AppleMusicService::playback_position()` forwards that value. This part changes no UI; it gives part 2/2 a narrow, testable source for the elapsed readout.

**Approach.**

- `src/audio.rs`: import `std::time::Duration` and add a provided trait method after `seek_by`: `fn position(&self) -> Option<Duration> { None }`. Its doc says it returns the loaded preview's position, `Some(Duration::ZERO)` when the backend has no loaded player, and `None` when the backend cannot answer; the default keeps `SilentOutput` and every existing test fake compiling without a position of their own.
- `src/audio/player.rs`: add `Command::Position(std::sync::mpsc::Sender<Duration>)`. Add `fn position(&self) -> Duration` to `WorkerPlayer`, with `RodioPlayer` returning `self.player.get_pos()` (the same source `seek_player` already reads). In `serve_commands`, handle `Command::Position(reply)` by sending `player.as_ref().map_or(Duration::ZERO, WorkerPlayer::position)`; because the loop handles the command in channel order, a position query sent during a preview download is answered after that download installs the new player. In `impl AudioOutput for RodioOutput`, add `position`: send `Command::Position` through the existing `send`, then wait on the reply with `recv_timeout`, returning `None` when the send fails or the wait times out. Define `POSITION_QUERY_TIMEOUT` next to `DEVICE_OPEN_TIMEOUT` (100 ms) and document that a download can hold the worker for up to `preview::PREVIEW_TIMEOUT`, so the query must not block the caller indefinitely.
- `src/apple_music.rs`: add `pub(crate) fn playback_position(&self) -> Option<Duration> { self.audio.position() }` beside `seek_output`, documented as synchronous, touching no shared state, and bounded by the seam's query timeout.
- `src/audio/tests.rs`: give `RecordingPlayer` a `position` implementation returning a fixed non-zero `Duration`, then add `position_query_returns_the_loaded_players_position` (send `Command::Position` after a `Play`, assert the reply equals the fake's position) and `position_query_returns_zero_when_nothing_is_loaded` (send `Command::Position` with no player). Add `rodio_output_asks_the_worker_for_its_position` (a helper thread reads `Command::Position` from the command channel and replies, while the test calls `position()`) and extend `rodio_output_reports_a_closed_worker_for_every_command` with `position() == None`.
- `src/test_support.rs`: override `position` for `RecordingAudio` to return `Some(Duration::from_secs(7))`, documented as a fixed stub position; no other call recording changes.
- `src/apple_music/tests.rs`: add `playback_position_forwards_the_audio_output_position`, building the service with `Arc::new(RecordingAudio::default())` through `with_audio` and asserting the service returns `Some(Duration::from_secs(7))`.
- `docs/invariants.md`: unchanged; this part adds a seam, not a user-visible guarantee.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`.

**Acceptance criteria.**

- `make check` passes.
- With a player loaded, `serve_commands` answers `Command::Position` with that player's `position()`; with no player it answers `Duration::ZERO`.
- `RodioOutput::position` returns the reply the worker sends, and returns `None` when the command channel is closed or no reply arrives within `POSITION_QUERY_TIMEOUT`.
- `AppleMusicService::playback_position` returns exactly the injected backend's `position()` value.
- `SilentOutput` and all existing `AudioOutput` test fakes keep compiling without adding a method, because the trait method has a `None` default.
- The change touches only the six listed files plus this `PLANS.md` edit; no UI, README, or invariant text changes.

### Add the playback-position readout to the Now Playing bar (Playback position, part 2/2: planned 2026-10-21 by the plan role, requires part 1/2 landed)

**Goal.** The Now Playing bar's right-hand readout shows the current track's elapsed and total time as `"<elapsed>/<total>"` (for example `0:45/3:12`) while a preview is loaded, and keeps the existing `"--:--"` when no track is current. The elapsed value is the position part 1 reads from the audio backend, so pause freezes it, a seek steps it, and Stop or a new track resets it.

**Approach.**

- `src/ui/views/now_playing.rs`: add `pub fn format_position_time(position_ms: u64) -> String`, formatting zero-padded `m:ss` like `format_track_time` but returning `"0:00"` for zero (elapsed zero is valid, unlike an unknown track length). Add `pub fn now_playing_progress(position_ms: u64, total: &str) -> String` returning `format!("{}/{}", format_position_time(position_ms), total)`, where `total` is the cached length string from `KnownTrack::time`. Both are pure and take no iced renderer.
- `src/ui/mod.rs`: add a `playback_position: Option<Duration>` field to `WinampPlayer`, initialized to `None` in `new`. Add `Message::PositionTick` and `Message::PlaybackPosition(Option<Duration>)`. Merge a 1-second `iced::time::every(POSITION_POLL_INTERVAL).map(|_| Message::PositionTick)` into the existing keyboard subscription; define `POSITION_POLL_INTERVAL` with a doc note that it bounds the readout's refresh, not the audio. The `PositionTick` arm clones the service and returns `Task::perform(async move { service.playback_position() }, Message::PlaybackPosition)`, keeping the seam's bounded blocking query off the UI thread. The `PlaybackPosition(position)` arm stores `position` when it is `Some` and ignores `None`, so a query delayed by a preview download leaves the previous readout in place instead of flickering. Set `playback_position = Some(Duration::ZERO)` in `TrackSelected` so a newly selected track starts at `0:00`, and set the same value in the `Stop` arm so Stop resets the readout immediately; Pause leaves the frozen polled value in place. In `TrackPlayed`, when the shared state reports nothing is playing (a failed stream start), set `playback_position = None` so the bar falls back to the plain total.
- `WinampPlayer::now_playing_time`: when `playback_position` is `Some` and the current track is in `known_tracks`, return `Cow::Owned(now_playing_progress(position_ms, &track.time))`; otherwise keep the existing borrowed total or `"--:--"` fallback. Update its doc comment to describe the combined playing readout and note the playing case allocates the combined string once per view build.
- `src/ui/views/tests.rs`: add `format_position_time_formats_elapsed_and_zero`, asserting `"0:00"` for `0`, `"3:12"` for `192_000`, and `"1:05"` for `65_000`, and `now_playing_progress_joins_elapsed_and_total`, asserting `"3:12/12:38"` for `(192_000, "12:38")`.
- `src/ui/tests.rs`: add `now_playing_time_shows_elapsed_over_total_when_a_position_is_known` (record a `KnownTrack`, set `playback_position`, assert the combined string) and `now_playing_time_falls_back_to_total_and_nothing` (a known track with no position still reads its total; no current track still reads `"--:--"`).
- `README.md`: extend the Usage sentence about the Now Playing bar's length so it says the readout shows the elapsed and total time while playing. Keep the hunk to that sentence.
- `docs/invariants.md`: extend the Now Playing label section with the progress guarantee and cite the new pure formatter/progress tests plus the two `now_playing_time` tests.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes.
- `format_position_time` returns `"0:00"`, `"3:12"`, and `"1:05"` for zero, 192000 ms, and 65000 ms; `now_playing_progress(192_000, "12:38")` returns `"3:12/12:38"`.
- With a position known and a current known track, the Now Playing bar renders `elapsed/total`; with the position unknown, it renders the existing total string; with no current track it renders `"--:--"`.
- Pause freezes the readout because the poll reads the backend's paused position, a left/right seek is reflected by the next poll, and Stop or a new selection resets it to `0:00`.
- A `None` reply leaves the last readout unchanged, and the position query runs in the iced task rather than blocking `update`.
- The change touches only the six listed files plus this `PLANS.md` edit.

## Done
