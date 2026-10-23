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

### Add the LCD spectrum visualizer (Spectrum visualizer, part 1/2: planned 2026-10-23 by the plan role)

**Goal.** Add a narrow backend query that reports the loaded preview's live ten-band amplitude spectrum, computed from the samples the audio worker actually pulls. This is the deferred half of the mockup's LCD visualizer (`docs/screenshots/player.png` draws green bars beneath the elapsed time): it gives part 2 a real signal to draw instead of an animation, with no new dependency and no UI change in this part. The meter is observational only — it must not alter the samples reaching the equalizer, balance, or output. When no preview is loaded, or the backend cannot answer (the silent fallback, a test fake, a closed worker), the query reports nothing, so part 2 can keep its strip blank.

**Approach.**

- `src/audio/meter.rs` (new module declared in `src/audio.rs`): add `pub const SPECTRUM_WINDOW: usize = 2048`, `pub struct Spectrum { pub bands: [f32; crate::equalizer::BAND_COUNT] }`, and re-export `Spectrum` from `src/audio.rs`. A band value is a normalized `0.0..=1.0` amplitude in `crate::equalizer::BAND_HZ` order, so the visualizer and equalizer name the same ten frequencies.
- In the same module, add `pub(super) fn spectrum_bands(samples: &[f32], sample_rate: u32) -> [f32; crate::equalizer::BAND_COUNT]`: run a Goertzel magnitude at each `BAND_HZ` over the supplied mono window, normalize a full-scale sine to about `1.0` (`2 * magnitude / window_len`), clamp each band into `0.0..=1.0`, and report `0.0` for a band at or above Nyquist or for silence. Keep it pure so the frequency mapping is testable without an audio device.
- Add `SpectrumMeter` as the shared state: a `std::sync::Mutex<SpectrumWindow>` holding a fixed `[f32; SPECTRUM_WINDOW]` ring plus a downmix accumulator, and an `AtomicU32` sample rate. `push(sample, channels)` downmixes interleaved samples to mono and overwrites the oldest slot once the ring fills; `take()` copies the ring in chronological order under the lock and computes `spectrum_bands` after releasing it, so the audio thread never waits on the Goertzel work; `clear()` empties the ring; poisoned locks are recovered rather than panicking (the crate forbids production `unwrap`/`panic`).
- Add `pub(super) struct MeterSource<S> { inner: S, meter: Arc<SpectrumMeter> }`: `new` records `inner.sample_rate()`, `next()` pushes the sample and returns it unchanged, and the `Source` impl delegates `current_span_len`, `channels`, `sample_rate`, `total_duration`, and `size_hint` to the inner source. `try_seek` delegates and then clears the window, so a seek does not splice old audio into the new position's spectrum.
- `src/audio.rs`: add `fn spectrum(&self) -> Option<Spectrum> { None }` to `AudioOutput`, documented like `playback_format` (only while a preview is loaded; `None` for the silent fallback, a fake, or an unanswered query). The default keeps every existing implementor compiling.
- `src/audio/player.rs`: `RodioPlayer` gains `spectrum: Arc<SpectrumMeter>`, initialized in `new`. `WorkerPlayer` gains `fn spectrum(&self) -> Option<Spectrum> { None }`. In `append`, after reading the format and duration and before building `EqSource`, wrap the decoded preview with `MeterSource::new(preview, Arc::clone(&self.spectrum))`; leave the rest of the chain and the returned samples unchanged. `RodioPlayer::spectrum` reports `Some(self.spectrum.take())` only while the player holds a preview (`self.format.get().is_some()`), otherwise `None`. Add `Command::Spectrum(Sender<Option<Spectrum>>)`, handle it in `serve_commands` beside `PlaybackFormat`, and add `RodioOutput::spectrum` mirroring `RodioOutput::playback_format`, including the existing query timeout.
- `src/audio/tests.rs`: add `spectrum_bands_peak_at_the_matching_band_for_a_sine` (a 1 kHz stereo-free sine wins the `1_000.0` band and silence gives all zeros), `spectrum_bands_zero_a_band_at_or_above_nyquist` (at 22_050 Hz the `16_000.0` band is zero while 1 kHz remains), `meter_source_passes_samples_through_and_records_their_spectrum` (collect a sine `SamplesBuffer` through `MeterSource`, assert the output samples are unchanged and `take()` peaks at the sine's band), `spectrum_query_returns_the_loaded_players_spectrum` and `spectrum_query_reports_none_when_nothing_is_loaded` (give `RecordingPlayer` an overridable `spectrum`), `silent_output_reports_no_spectrum`, and extend `rodio_output_reports_a_closed_worker_for_every_command` with a `None` spectrum answer. Add a `MeterSource` delegation assertion for sample rate, channels, and total duration.
- `docs/invariants.md`: add a `## Preview spectrum` section after `## Preview download`: the backend reports ten normalized band amplitudes in `BAND_HZ` order only while a preview is loaded, computed from unchanged samples pulled through `MeterSource`; silence and bands at or above Nyquist read zero; Stop, a new selection, and a seek clear the window; the silent fallback and a closed worker report `None`. Name `audio::meter::Spectrum`/`spectrum_bands`/`MeterSource`, `audio::AudioOutput::spectrum`, and `audio::player::RodioPlayer::spectrum` as the enforcers, and list the new test names.

**Files touched.** `src/audio/meter.rs`, `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `docs/invariants.md`.

**Acceptance criteria.**

- `spectrum_bands` maps a 1 kHz sine to the `1K` band with the other bands lower, maps silence to all zeros, and reports zero for a band at or above Nyquist.
- `MeterSource` returns every sample it is given unchanged, delegates the source's sample rate, channels, length, and span, and records the same samples into its `SpectrumMeter`; a seek clears the recorded window.
- `AudioOutput::spectrum` answers `Some` for a loaded preview through `RodioOutput`, and `None` for `SilentOutput`, a recording fake without an override, and a closed worker; Stop and a new preview cannot report the previous preview's spectrum.
- No UI code reads the spectrum yet; the equalizer, balance, volume, and output samples are unchanged by the meter.
- `make check` passes (fmt, clippy, rustdoc, tests).

### Add the LCD spectrum visualizer (Spectrum visualizer, part 2/2: planned 2026-10-23 by the plan role, requires part 1/2 landed)

**Goal.** Complete the mockup's LCD (`docs/screenshots/player.png`): draw a fixed-height row of ten green bars beneath the Now Playing title, whose heights follow the loaded preview's live spectrum from part 1, updating about four times a second while playback runs. The strip is blank (but keeps its height) when no spectrum is reported, so the silent fallback and a mid-download selection do not jump the layout. Pause freezes the bars at their last levels; Stop and a new selection clear them through the same epoch/reset path the sample-rate status row uses. No new dependency and no change to playback or the browse list.

**Approach.**

- `src/apple_music/playback.rs`: add `pub(crate) fn playback_spectrum(&self) -> Option<audio::Spectrum>` forwarding to `self.audio.spectrum()`, documented like `playback_format`. Add `playback_spectrum_forwards_the_audio_output_spectrum` to `src/apple_music/tests.rs`, driving the injected `RecordingAudio`.
- `src/test_support.rs`: give `RecordingAudio` a fixed stub `spectrum` (a `Spectrum` with one distinctive nonzero band) so UI tests can pin what the backend reports, matching its stub position, format, and duration.
- `src/ui/mod.rs`: add `spectrum: Option<Spectrum>` to `WinampPlayer`, initialized `None`, and import `Spectrum` beside `PlaybackFormat`. Add `Message::SpectrumTick` (the timer's tick) and `Message::Spectrum { epoch: u64, spectrum: Option<Spectrum> }` (the query's reply). Add the `update` arms forwarding to the handlers below, add `needs_spectrum_poll` beside `needs_position_poll` (true only while `state.is_playing`), and pass `player.spectrum` to `views::view_now_playing`.
- `src/ui/timers.rs`: add `pub(super) const SPECTRUM_TICK: Duration = Duration::from_millis(250)` and mirror the position timer's plumbing: `spectrum_ticks`, `spectrum_tick_stream`/`spectrum_tick_stream_with`, `spawn_spectrum_thread`, `SPECTRUM_THREAD_NAME = "nostalgia-spectrum-poll"`, and a failure report naming the visualizer. Add a `spectrum_needed` argument to `subscriptions`, and pass `player.needs_spectrum_poll()` from `init_ui`, so the timer runs only while playback is running and is dropped on Pause/Stop.
- `src/ui/handlers/playback.rs`: add `spectrum_tick(player) -> Task<Message>`: when no transport transition is pending, capture `position_epoch` and schedule `apple_music_service.playback_spectrum()`, mapping the reply to `Message::Spectrum { epoch, spectrum }`; otherwise return `Task::none()`. Add `spectrum(player, epoch, spectrum) -> Task<Message>` storing the value only when the epoch is current and no transition is pending, so stale replies cannot restore an old preview's bars. Clear `player.spectrum` in `reset_position_readout` beside the existing position, format, and duration clears, so Stop and a new selection blank the strip while Pause leaves it frozen.
- `src/ui/views/now_playing.rs`: add the visualizer constants (`LCD_VISUALIZER_HEIGHT`, `LCD_VISUALIZER_BAR_WIDTH`, `LCD_VISUALIZER_BAR_GAP`) and `pub fn visualizer_bar_heights(spectrum: Option<Spectrum>) -> [f32; BAND_COUNT]`: `None` yields all-zero heights, and each band is clamped to `0.0..=1.0` and multiplied by the maximum bar height. Add `now_playing_visualizer_row` drawing exactly `BAND_COUNT` bottom-aligned `Container` bars left to right in `theme::LCD_GREEN` inside a row of fixed `LCD_VISUALIZER_HEIGHT`; an empty spectrum draws the same row with zero-height bars. Push that row between `now_playing_row` and `now_playing_status_row` in `now_playing_content`, and thread the `spectrum` argument through `view_now_playing` and `now_playing_content`.
- `src/ui/views/tests.rs`: add `visualizer_bar_heights_scale_and_clamp_the_reported_bands` (`None` all zero, bands `0.0`/`0.5`/`1.0`/`2.0` map to zero/half/full/full) and a `view_now_playing` construction test with `Some` and `None` spectra asserting the title and status text still draw and the visualizer row keeps its height in both cases.
- `src/ui/tests.rs`: add `spectrum_reply_stores_the_loaded_previews_spectrum`, `spectrum_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`, `stop_clears_the_spectrum_while_pause_keeps_it`, `spectrum_tick_queries_spectrum_and_carries_the_epoch`, and `needs_spectrum_poll_runs_only_while_playing`; update `position_tick_queries_position_and_format`-style timer tests for the new `subscriptions` argument where the signature changed.
- `README.md`: in the Usage paragraph describing the LCD, say a loaded preview drives a small green ten-bar spectrum beneath the title, that Pause freezes it, and that the strip stays blank (with its height) when no preview reports audio. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Preview spectrum`, add the UI half: the LCD draws exactly `BAND_COUNT` bars from the stored spectrum, heights clamp to the fixed row, Pause freezes them, Stop and a new selection clear them, stale or pre-transition replies are dropped, and the poll runs only while playing. Extend `## Now Playing label` with the visualizer bullet and add `views::now_playing::visualizer_bar_heights`/`now_playing_visualizer_row`, `WinampPlayer::needs_spectrum_poll`, and the new handler and timer test names to the enforcer and Pinned-by lists.

**Files touched.** `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/mod.rs`, `src/ui/timers.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `visualizer_bar_heights` returns exactly ten heights in `BAND_COUNT` order, all zero for `None` and for zero bands, half the maximum for `0.5`, and the maximum for values at or above `1.0`.
- The LCD draws the fixed-height visualizer row beneath the title and above the status row in both the reporting and blank states, so loading a preview or the silent fallback does not move the title or status text.
- The spectrum poll is subscribed only while playback runs, queries `AudioOutput::spectrum` through the service, and drops a reply whose epoch is stale or whose transport transition is still pending; Pause keeps the last bars, Stop and a new selection clear them.
- The silent fallback and a backend without a spectrum leave every bar at zero while keeping the row's height.
- `make check` passes (fmt, clippy, rustdoc, tests).

### Show the mini transport in window-shade mode (planned 2026-10-23 by the plan role)

**Goal.** Match the mockup's rolled-up window (`docs/screenshots/equalizer.png`, the "WINDOW SHADE" panel): shading the window shows, below the title bar, a compact strip with Previous, Play/Pause, Stop, Next, the current elapsed readout, and a volume slider, so playback stays controllable while rolled up. Normal mode is unchanged; the strip only reads existing state and emits existing messages, adding no new playback behavior.

**Approach.**

- Add `src/ui/views/shade.rs` (declare `mod shade;` and re-export it from `src/ui/views/mod.rs` beside `TITLE_BAR_HEIGHT`):
  - `pub(super) const SHADE_STRIP_HEIGHT: f32 = 28.0;` and `pub(super) const SHADED_HEIGHT: f32 = TITLE_BAR_HEIGHT + SHADE_STRIP_HEIGHT;`, so the strip height and the whole rolled-up height have one owner each.
  - `pub(super) fn play_pause_glyph(is_playing: bool) -> &'static str`: `"II"` while playing and `"▶"` otherwise, the compact twin of `transport::play_pause_label`.
  - `pub(super) fn shade_controls(is_playing: bool) -> [(&'static str, f32, Message); 5]`: the five `(label, fixed width, message)` specs in row order — `"◀◀"`/`Message::PreviousTrack`, [`play_pause_glyph`]/`Message::PlayPause`, `"■"`/`Message::Stop`, `"▶▶"`/`Message::NextTrack` — each with a fixed width (34, 30, 30, 34 px) so the glyph swap cannot reflow the strip. Pure, so labels, messages, and widths are testable without an iced renderer.
  - `pub fn view_shade_strip<'a>(is_playing: bool, volume: f32, elapsed: Cow<'a, str>) -> Element<'a, Message>`: one full-width row built from `fixed_width_button` per `shade_controls` entry with `spacer(6.0)` between them, the elapsed as `Text::new(elapsed).size(14).color(theme::LCD_GREEN)`, and `chrome_slider(VOLUME_MIN, VOLUME_MAX, volume, VOLUME_STEP, 80.0, Message::VolumeChange)` at the right. Paint the row with `style::title_bar_style()` inside a container fixed at `SHADE_STRIP_HEIGHT` (wrapped in `bevel::raised_panel` like the title bar) so the strip shares the title bar's blue chrome. No new message or handler is added.
- `src/ui/mod.rs` `view`: replace the shaded early return's bare title bar with a short state lock for `state.is_playing`, `state.volume()`, and `player.now_playing_elapsed(state.current_track.as_deref())`, then a `Column` of `views::view_title_bar(...)` and `views::view_shade_strip(...)`. Passing the same `now_playing_elapsed` string the LCD uses means the elapsed/remaining toggle carries into shade mode. Update the function's doc comment and inline comment to say the rolled-up tree is the title bar plus the strip. Leave the unshaded assembly untouched.
- `src/ui/handlers/window.rs`: `resize_to_shade` resizes to `views::SHADED_HEIGHT` instead of `views::TITLE_BAR_HEIGHT`, and its doc comment names the strip as part of the rolled-up window. The measurement and restore paths in `toggle_window_shade` and `window_shade_measured` are unchanged.
- `src/ui/views/tests.rs`: add `shade_play_pause_glyph_switches_with_playback_state`, `shade_controls_map_to_transport_messages_and_keep_fixed_widths`, `shade_height_pins_the_rolled_up_window` (`SHADED_HEIGHT == TITLE_BAR_HEIGHT + SHADE_STRIP_HEIGHT`, both positive), and `view_shade_strip_keeps_its_fixed_height_for_both_playback_states` (lay the widget out with the headless-renderer `Limits` pattern already used in this file and assert the resolved height is `SHADE_STRIP_HEIGHT`).
- `src/ui/tests.rs`: update `title_bar_height_pins_the_shade_strip`'s comment to point at `SHADED_HEIGHT`; add `shaded_root_view_is_the_title_bar_plus_the_strip` (lay out the root `view` with `player.shaded = true` using the headless tiny-skia pattern from `browse_nav_buttons_place_sort_beside_top`, and assert the resolved height is `views::SHADED_HEIGHT`) and `shade_strip_draws_only_while_shaded` (capture text bounds and assert the `"◀◀"` control appears in the shaded tree and not in the normal-mode tree). Keep the existing shade measurement/restore tests green.
- `README.md`: in the Usage paragraph, say shade mode rolls the window up to the title bar plus a compact transport strip with Previous, Play/Pause, Stop, Next, the current time readout, and a volume slider. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Window shade` section after `## Now Playing label`: shade mode builds exactly the title bar plus the fixed-height strip and resizes the window to `SHADED_HEIGHT`; the strip's five controls emit the same messages as their full transport-row twins, its Play/Pause glyph follows the playback state, its time label follows the elapsed/remaining choice, and its volume slider writes `Message::VolumeChange`; normal mode builds no strip. Name `views::shade::view_shade_strip`/`shade_controls`/`play_pause_glyph`/`SHADED_HEIGHT` and `handlers::window::resize_to_shade` as the enforcers, and list the new test names.

**Files touched.** `src/ui/views/shade.rs` (new), `src/ui/views/mod.rs`, `src/ui/mod.rs`, `src/ui/handlers/window.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- In shade mode the rendered tree is the title bar over a strip exactly `SHADE_STRIP_HEIGHT` tall, and the window resizes to `SHADED_HEIGHT`; the normal-mode tree contains no strip.
- The strip shows Previous, Play/Pause, Stop, Next, the current elapsed/remaining readout, and a volume slider, each emitting the same message as its full transport-row twin (`Message::PreviousTrack`, `Message::PlayPause`, `Message::Stop`, `Message::NextTrack`, `Message::VolumeChange`).
- The Play/Pause glyph is `"▶"` while stopped and `"II"` while playing, and every compact button keeps its fixed width across that swap.
- Laying out the strip and the shaded root view with a headless renderer resolves to the fixed heights above, so the view and `resize_to_shade` cannot drift apart.
- Normal-mode layout, the existing shade measurement/restore tests, and playback behavior are unchanged.
- `make check` passes (fmt, clippy, rustdoc, tests).

### Add a shared headless layout-capture harness so UI geometry bugs confirm offline (planned 2026-10-23 by the steward role as a gap promotion for `real-run-needed`)

**Goal.** Retire the `real-run-needed` validation gap for UI geometry: make the laid-out bounds of the real widget tree readable from a unit test, so a layout or label-reflow bug is confirmed by `make test` instead of a hand-run window measurement. Four Fixed entries carry the `real-run-needed` gap. The two non-layout ones now have offline coverage — the sign-in race through the injectable session seam and `AppleMusicService::wait_for_session`, and the nonce leak through the capturing opener in `authorize_keeps_the_state_nonce_out_of_the_opener_argument` — while the two layout ones, "Sort renders below Top instead of beside it, contradicting the documented layout" and "Transport buttons resize to their label text, so pressing Play/Pause reflows the whole control row", were confirmed only by running the GUI and measuring a captured window. The ad hoc `CapturedText` operation in `src/ui/views/tests.rs` and `CapturedTextBounds` in `src/ui/tests.rs` prove the offline path works; this plan promotes them into one shared harness so the next layout bug does not reconstruct it.

**Approach.**

- Add a test-only `src/ui/layout_capture.rs` (declared `#[cfg(test)] pub(crate) mod layout_capture;` in `src/ui/mod.rs`), holding one `CapturedText` widget `Operation` and a small API: a capture function that lays a prepared `Element` out under a fixed `Limits`, runs the operation, and returns each drawn `(text, Rectangle)` with its absolute bounds plus the root `Size`; a `find(label)` lookup that panics with the captured fragments when a label is missing; and `headless_renderer()`, which builds iced's tiny-skia `Headless` renderer with `futures::executor::block_on` (the pattern `browse_nav_buttons_place_sort_beside_top` already uses), so trees containing the fallback-renderer `bevel` widgets lay out too.
- Move the duplicate capture structs out of `src/ui/views/tests.rs` (`CapturedText`) and `src/ui/tests.rs` (`CapturedTextBounds`) and point both suites at the shared module, keeping the existing helpers' assertions (the Now Playing fragment order, the indicator strip's order and width) intact.
- Rewrite `browse_nav_buttons_place_sort_beside_top` on the harness without weakening it: it must still lay out the whole root `view` (not the nav row alone) and assert Sort shares Top's row and starts at or after Top's right edge.
- Add `transport_row_does_not_reflow_when_the_play_label_changes`: lay the root `view` out with the player stopped and then playing, and assert the `Stop` and `Repeat` labels keep the same absolute x, which fails if a label-driven width change reaches the row as it did before the fixed-width fix. Keep the builder-level `transport_buttons_keep_a_fixed_width_across_label_changes` and the measured-width pins unchanged.
- Add a harness self-test that lays out a small known column and asserts the captured fragments and root size, so a broken capture fails directly rather than as a confusing geometry assertion.
- `docs/invariants.md`: add a `## UI layout` section after `## Browse row order` stating that Sort lays out beside Top in the root view and that the transport row's faces keep their bounds across the Play/Pause, Repeat, and Shuffle label changes; name `views::view_browse_nav_buttons`, `views::view_transport_controls`, and `views::transport::transport_buttons` as the enforcers and cite the two layout tests and the two Fixed entries. Add a line to the "How to use" note pointing layout bugs at the harness.

**Files touched.** `src/ui/layout_capture.rs` (new), `src/ui/mod.rs`, `src/ui/tests.rs`, `src/ui/views/tests.rs`, `docs/invariants.md`.

**Acceptance criteria.**

- One shared test module returns every drawn text fragment with its absolute `Rectangle` and the root `Size` for both a null-renderer view and the tiny-skia root view; both existing capture helpers are deleted and their tests run through it.
- `browse_nav_buttons_place_sort_beside_top` still lays out the root `view` and fails if Sort is stacked below Top or drawn before Top's right edge.
- `transport_row_does_not_reflow_when_the_play_label_changes` lays the root view out in both playback states and asserts Stop and Repeat keep their x, so the historical reflow is confirmed offline.
- The new `## UI layout` invariant states both guarantees, names their enforcing code, and lists the tests that pin them; the harness is discoverable from `docs/invariants.md`'s "How to use" note.
- `make check` passes (fmt, clippy, rustdoc, tests), and the change adds no dependency.

**Cited entries.** `BUGS-FIXED.md`: the four `real-run-needed` entries — "Sort renders below Top instead of beside it, contradicting the documented layout", "Transport buttons resize to their label text, so pressing Play/Pause reflows the whole control row", "The boot-time artists fetch races the async sign-in, so a signed-in user sees the sample library", and "The sign-in nonce travels in the browser opener's command line, so another local user can read it and fetch the page's developer token".

## Done
