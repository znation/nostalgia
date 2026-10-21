# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; updated 2026-10-07).** The initial prompt
> names two commitments — a "very close if not pixel-perfect" classic Winamp UI
> and Apple Music as the library. The first is now moving: the base-skin
> palette landed ("Add a Winamp 2.x base-skin palette and apply it as the app
> theme", done 2026-10-06), and the reusable bevel layer followed ("Add a
> Winamp two-tone bevel layer and frame the Now Playing and equalizer panels",
> done 2026-10-06), framing those two panels through `bevel::lcd_well` and
> `bevel::raised_panel` in `src/ui/bevel.rs`. `## Planned` is no longer empty:
> it holds "Style the transport buttons as raised Winamp chrome", which
> restyles the five transport buttons and the browse Back button — all built
> through `labeled_button`, still a default iced `Button`. Still unplanned is
> the rest of widget-level fidelity: slider chrome (the bevel entry and the
> transport plan both defer it), a custom title bar, and playlist chrome. The
> library side is still all stub: `AppleMusicService` answers every browse
> query (`get_favorite_artists`, `get_albums_by_artist`,
> `get_songs_from_album`) from `sample_library()`, `play_track` only mutates
> shared state, and the token field is unset — and no PLANS.md entry plans the
> real integration. Risk: the longer Apple Music stays stubbed the more UI
> grows around the sample library's shape, while the remaining chrome work is
> easier to sequence now that the bevel layer exists. Recommend the plan loop
> keep scheduling the remaining fidelity plans (slider chrome, title bar,
> playlist chrome) and put the Apple Music integration plan on the near
> horizon.

## Planned

### Apply the equalizer curve to the audio output (Equalizer audio, part 1/2: planned 2026-10-20 by the plan role)

**Goal.** A preview starts through the equalizer's ten band filters at the curve the panel currently shows. Today `AppState` stores `eq_enabled`, `eq_preamp`, and `eq_bands`, and the equalizer update arms only write them; `AudioOutput` has no equalizer method, so every preview plays flat. This part adds the filter chain and applies the stored curve when a preview starts; part 2 forwards changes made while a preview is already playing.

**Approach.**

- `src/equalizer.rs`: add `pub const BAND_HZ: [f32; BAND_COUNT]` holding the numeric centres matching `BAND_FREQUENCIES` (`60.0` through `16_000.0`, with `1K` as `1_000.0`) and `pub struct BandFilter` with `new(sample_rate)`, `set_curve(enabled, preamp_db, bands_db)`, and `process(sample)`. Build each band as an RBJ peaking biquad at its `BAND_HZ` centre with a fixed `Q = 1.41` (roughly one octave), rebuild the coefficients in `set_curve`, and return the sample unchanged when disabled. When enabled, multiply by the preamp gain `10^(preamp_db / 20)` and run the sample through all ten biquads. Keep `Biquad` private to the module; `BandFilter` is the public type the audio source uses.
- `src/audio.rs`: add `pub struct EqSnapshot { pub enabled: bool, pub preamp_db: f32, pub bands_db: [f32; BAND_COUNT] }` (derive `Debug, Clone, Copy, PartialEq`) and private `EqSettings` holding those values in `AtomicBool`/`AtomicU32`s plus an `AtomicU64` version. Add `EqSource<S>` holding `inner`, `Arc<EqSettings>`, the last seen version, one `BandFilter` per channel from `inner.channels()`, and a channel cursor; on each `next`, rebuild every channel filter when the version changed, then process the sample through the current channel's filter. Delegate `current_span_len`, `channels`, `sample_rate`, `total_duration`, and `try_seek` like `BalanceSource`, resetting the filters on a successful seek. Add `set_equalizer(&self, equalizer: EqSnapshot)` to `AudioOutput`, `Command::SetEqualizer(EqSnapshot)`, and `WorkerPlayer::set_equalizer`; `SilentOutput` logs and returns `Ok`, `RodioPlayer` stores the snapshot in its `Arc<EqSettings>`, and `append` wraps each preview as `BalanceSource::new(EqSource::new(preview, eq), balance)` so equalization runs before the stereo balance. `serve_commands` remembers the snapshot across players and applies it to every newly created player, exactly as it remembers volume and balance.
- `src/test_support.rs`: add `AudioCall::SetEqualizer(EqSnapshot)`, record it in `RecordingAudio::set_equalizer`, and return `Ok(())` from `FailingAudio::set_equalizer`. Extend `assert_preview_calls` so a preview start expects `SetVolume(0.5)`, `SetBalance(0.0)`, `SetEqualizer(default)`, then `Play`.
- `src/apple_music.rs`: add `pub(crate) fn set_output_equalizer(&self, equalizer: EqSnapshot) -> Result<(), AppleMusicError>` forwarding to `self.audio.set_equalizer(equalizer)` and touching no shared state, like `set_output_volume`. In `play_track`, read `state.eq_enabled`, `state.eq_preamp()`, and `state.eq_bands()` under the state lock, then chain `set_volume(volume)`, `set_balance(balance)`, `set_equalizer(snapshot)`, and `play(url)` so the first preview starts at the stored curve rather than flat. Keep the existing rollback on a backend error.
- `src/audio.rs` tests: update `RecordingPlayer` to implement `set_equalizer`; add `a_new_preview_starts_at_the_remembered_equalizer` beside `a_new_preview_starts_at_the_remembered_balance`; extend `silent_output_reports_success_for_every_command`, `audio_with_fallback_falls_back_to_silence_when_open_fails`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` with the new method. Add `eq_source_disabled_passes_samples_through`, `eq_source_enabled_flat_curve_passes_samples_through`, `eq_source_preamp_scales_by_the_decibel_gain`, `eq_source_filters_each_channel_with_its_own_state`, and `eq_source_follows_the_curve_changed_while_playing`.
- `src/equalizer.rs` tests: add `band_frequencies_have_matching_numeric_centres`, `band_filter_boost_and_cut_at_the_center_frequency` (a generated 1 kHz sine through a `+12 dB` and a `-12 dB` 1 kHz band, comparing settled RMS), and `band_filter_preamp_is_a_flat_gain`.
- `src/apple_music/tests.rs`: add `set_output_equalizer_forwards_to_the_audio_backend` and update `play_track_starts_the_preview_through_the_audio_backend` and `assert_preview_calls` callers to expect the equalizer snapshot before `Play`.
- `README.md`: state in the usage paragraph that the equalizer panel's curve is applied to preview playback.

**Files touched.** `src/equalizer.rs`, `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- A disabled `BandFilter` returns each input sample unchanged; an enabled flat curve (zero preamp, zero bands) also passes samples through within floating-point error.
- A `+12 dB` 1 kHz band raises a settled 1 kHz sine's RMS, a `-12 dB` band lowers it, and a `+6.0206 dB` preamp produces approximately twice the input amplitude; each channel of a stereo source keeps its own filter state.
- `EqSource` reports its inner source's channel count, sample rate, span, duration, and seek result, and follows a curve changed through its shared `EqSettings` between samples.
- `RodioOutput::set_equalizer` sends `Command::SetEqualizer` with the same snapshot; `SilentOutput::set_equalizer` returns `Ok`; a closed worker reports "the audio thread is gone".
- `AppleMusicService::set_output_equalizer` records one `AudioCall::SetEqualizer` and leaves `AppState` unchanged; `play_track` records `SetVolume`, `SetBalance`, `SetEqualizer` carrying the state's current curve, then `Play`.

### Apply the equalizer curve to the audio output (Equalizer audio, part 2/2: planned 2026-10-20 by the plan role, requires part 1/2 landed)

**Goal.** Toggling the equalizer, dragging the preamp or a band slider, or picking a preset changes the sound of the preview already playing, without restarting it. Part 1 applies the stored curve when a preview starts; the four equalizer update arms still call `mutate_state`, so a change made mid-track never reaches the audio backend.

**Approach.**

- `src/ui/mod.rs`: add `fn mutate_equalizer(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message>` beside `mutate_volume` and `mutate_balance`: lock the state, apply the mutation, read `eq_enabled`, `eq_preamp()`, and `eq_bands()`, build an `EqSnapshot`, and call `player.apple_music_service.set_output_equalizer(snapshot)`, logging a backend error with the same `eprintln!` shape as the volume and balance helpers. Route `ToggleEqualizer`, `EqPreampChange`, `EqBandChange`, and `EqPresetSelected` through it instead of `mutate_state`. Because `EqSource` reads its shared `EqSettings` between samples, the running preview picks up the new curve without a new `Play`.
- `src/ui/tests.rs`: add `equalizer_messages_forward_to_the_audio_backend`, driving `ToggleEqualizer`, `EqPreampChange(3.0)`, `EqBandChange(0, 4.0)`, and `EqPresetSelected(rock_preset())` through `update` over a `player_with_audio` built with a `RecordingAudio` fake and asserting the recorded `SetEqualizer` snapshots match the clamped `AppState` values; include an out-of-range and a `f32::NAN` gain in the band case, as the balance test does.
- `README.md`: extend the equalizer sentence to say the sliders and presets change the playing preview live, beside the existing volume and balance sentences.

**Files touched.** `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- Each equalizer message records exactly one `SetEqualizer` call whose enabled flag, preamp, and ten band gains equal the clamped values now in `AppState`; no call restarts the preview.
- A backend error from `set_output_equalizer` is logged and the state mutation still commits, as with volume and balance.
- The README says equalizer changes apply to the playing preview.

### Seek the playing preview five seconds with the left and right arrow keys (planned 2026-10-20 by the plan role)

**Goal.** Pressing Left seeks the playing preview back five seconds and pressing Right seeks it forward five seconds, matching classic Winamp's keyboard bindings. Today `shortcuts::shortcut` maps only ArrowUp/ArrowDown (to `Message::VolumeUp`/`VolumeDown`), `Message` has no seek variant, and `AudioOutput` exposes only play/pause/resume/stop/`set_volume`/`set_balance`. `rodio` 0.22's `Player` already provides `get_pos` and `try_seek`, but nothing in `src/audio.rs` calls them, so the left/right keys currently do nothing.

**Approach.**

- `src/audio.rs`: add `fn seek_by(&self, offset_secs: i64) -> Result<(), AppleMusicError>` to `AudioOutput`, documented as a relative move (positive seeks forward, negative backward), a no-op when no player is loaded, and a non-blocking channel send like `set_volume`. Implement it for `SilentOutput` (log and return `Ok`), add `Command::SeekBy(i64)`, and add `fn seek_by(&self, offset_secs: i64)` to `WorkerPlayer`. Add a private pure `fn seek_target(position: Duration, offset_secs: i64) -> Duration` that saturating-adds or saturating-subtracts `Duration::from_secs(offset_secs.unsigned_abs())`, so a backward seek past the start lands at zero. `RodioPlayer::seek_by` reads `self.player.get_pos()`, calls `self.player.try_seek(seek_target(position, offset_secs))`, and logs a `try_seek` error with `eprintln!` without changing any state. `serve_commands` handles `Command::SeekBy` by calling `player.seek_by(offset_secs)` only when a player is installed; the offset is not remembered across players, so a seek while stopped does nothing.
- `src/test_support.rs`: add `AudioCall::SeekBy(i64)`, record it in `RecordingAudio::seek_by`, and return `Ok(())` from `FailingAudio::seek_by`.
- `src/apple_music.rs`: add `pub(crate) fn seek_output(&self, offset_secs: i64) -> Result<(), AppleMusicError>` beside `set_output_volume` and `set_output_balance`, forwarding to `self.audio.seek_by(offset_secs)` and touching no shared state.
- `src/apple_music/tests.rs`: implement `seek_by` for `GatedAudio` and `RejectingAudio`, recording `AudioCall::SeekBy` and succeeding (like their other non-rejected commands). Add `seek_output_forwards_to_the_audio_backend`, asserting one `AudioCall::SeekBy(-5)` and that `AppState` is unchanged.
- `src/ui/mod.rs`: add `SeekBackward` and `SeekForward` variants beside `VolumeUp`/`VolumeDown`, and add a private `seek_preview(player, offset_secs)` helper that calls `player.apple_music_service.seek_output(offset_secs)`, logs a backend error like `mutate_volume`, and returns `Task::none()`. Route the two new arms through it with `-views::SEEK_STEP_SECS` and `views::SEEK_STEP_SECS`.
- `src/ui/views/mod.rs`: add `pub(super) const SEEK_STEP_SECS: i64 = 5;` beside `VOLUME_STEP`, documenting Winamp's five-second seek step.
- `src/ui/shortcuts.rs`: map `Key::Named(key::Named::ArrowLeft)` to `Message::SeekBackward` and ArrowRight to `Message::SeekForward`, updating the `message_for`/`shortcut` doc lists. Extend `message_for_maps_the_arrow_keys` and add both arrows to `shortcut_ignores_chord_modifiers`.
- `src/ui/tests.rs`: add `seek_messages_drive_the_audio_backend`, driving both messages through `update` over a `RecordingAudio` and asserting one `AudioCall::SeekBy(-SEEK_STEP_SECS)` and one `SeekBy(SEEK_STEP_SECS)` with no scheduled task.
- `src/audio.rs` tests: extend `silent_output_reports_success_for_every_command`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` with `seek_by`; add `seek_target_moves_forward_and_backward` (10s +5 → 15s, 10s -5 → 5s) and `seek_target_saturates_at_the_start` (3s -5 → 0); extend the recording `WorkerPlayer` with `seek_by` and `transport_volume_and_balance_commands_drive_the_installed_player` with a `Command::SeekBy(-5)` before its `Stop`, asserting the installed player received it.
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

## Done
