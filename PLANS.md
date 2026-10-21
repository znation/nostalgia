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

_None yet._

### Apply the balance slider to the audio output (planned 2026-10-20 by the plan role)

**Goal.** Moving the balance slider changes the stereo image of the playing preview, and a preview starts at the slider's balance. Today `Message::BalanceChange` only writes `AppState::balance`: `AudioOutput` has no balance method, so `mutate_state` never forwards the value and every preview plays centered. This is the balance twin of "Drive the volume slider through the audio backend" (done), whose approach deliberately left `BalanceChange` alone because rodio's `Player` has no panner. A small `Source` wrapper supplies the missing panner without a new dependency.

**Approach.**

- `src/audio.rs`:
  - Add `set_balance(&self, balance: f32) -> Result<(), AppleMusicError>` to `AudioOutput` (doc: `-1.0` is hard left, `0.0` centered, `1.0` hard right; it affects the current player and is remembered for the next `play`). `SilentOutput::set_balance` logs the value and returns `Ok`.
  - Add `Command::SetBalance(f32)`; `RodioOutput::set_balance` sends it.
  - Add a private `BalanceSource<S: Source>` adapter that wraps the decoded preview and holds the balance as an `Arc<AtomicU32>` of `f32` bits, read on every sample so a slider drag changes the current preview without restarting it. Delegate `current_span_len`, `channels`, `sample_rate`, `total_duration`, and `try_seek` to the inner source. Gain per channel: for a mono source `1.0 - balance.abs()`; otherwise even channel indices use `1.0 - balance.max(0.0)` (left) and odd indices use `1.0 + balance.min(0.0)` (right), so `-1.0` silences the right channel, `+1.0` the left, and `0.0` is unity. Do not use rodio's `ChannelVolume`/`Spatial`: both downmix the input to mono, which would collapse the preview's stereo image.
  - Add a private `RodioPlayer` struct holding the `rodio::Player` and its balance `Arc<AtomicU32>`; implement `WorkerPlayer` for it with `Preview = rodio::Decoder<Cursor<Vec<u8>>>`, appending the preview wrapped in `BalanceSource`, and add `set_balance` to the `WorkerPlayer` trait (the recording fake records it). `run_worker` builds a `RodioPlayer` instead of a bare `rodio::Player`.
  - In `serve_commands`, remember the balance (initial `0.0`) across players, map `Command::SetBalance` to storing it and calling `set_balance` on the current player when one exists, and call `set_balance(balance)` on a newly created player before `append`, as `set_volume` already does. Update the worker tests: the recording fake records `set_balance`, and a test pins that a balance set while stopped applies to the next player and one set while playing reaches the current player.
  - Add unit tests for `BalanceSource` using a two-channel `SamplesBuffer` with distinct left/right constants: `-1.0` keeps left and zeroes right, `+1.0` the reverse, `0.0` is unchanged, and changing the shared atomic between `next()` calls changes the following samples (the live-update contract).
  - Extend `silent_output_reports_success_for_every_command`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` to cover `set_balance`.
- `src/test_support.rs`: add `AudioCall::SetBalance(f32)` and record it in `RecordingAudio::set_balance`; `FailingAudio::set_balance` returns `Ok(())` like its volume twin. Extend `assert_preview_calls` (and its callers) so a preview start expects `SetVolume`, then `SetBalance`, then `Play`.
- `src/apple_music.rs`: add `pub(crate) fn set_output_balance(&self, balance: f32) -> Result<(), AppleMusicError>` forwarding to `self.audio.set_balance(balance)` and touching no shared state, exactly as `set_output_volume` does. In `play_track`, capture `state.balance()` before dropping the lock and apply it with the volume before `play`, so the first preview starts at the slider's balance.
- `src/ui/mod.rs`: add `mutate_balance(player, mutation) -> Task<Message>` beside `mutate_volume`: apply the mutation under one synchronous `blocking_lock`, read back the clamped `state.balance()`, forward it to `player.apple_music_service.set_output_balance(...)`, and log a backend error. Replace the `Message::BalanceChange` arm's `mutate_state` call with it and drop `BalanceChange` from `mutate_state`'s doc list.
- `README.md`: state in the usage paragraph that the balance slider sets the audio backend's stereo balance, beside the existing volume sentence.
- Tests: `src/apple_music/tests.rs` gains `set_output_balance_forwards_to_the_audio_backend` (one `AudioCall::SetBalance`, `AppState::balance` unchanged) and updates `play_track_starts_the_preview_through_the_audio_backend` to expect the balance call. `src/ui/tests.rs` gains `balance_messages_forward_to_the_audio_backend`, driving `Message::BalanceChange` through `update` over a `player_with_audio(RecordingAudio)` and asserting the recorded balance and the clamped `AppState::balance`, including an out-of-range value and `f32::NAN` (which stores and forwards `0.0`).

**Files touched.** `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `BalanceSource` at `-1.0` leaves the left channel unchanged and zeroes the right; at `+1.0` the reverse; at `0.0` both unchanged; a mono source is attenuated by `1.0 - balance.abs()`; mutating the shared balance between `next()` calls changes later samples without rebuilding the source.
- `RodioOutput::set_balance(0.4)` sends `Command::SetBalance(0.4)`; `SilentOutput::set_balance` returns `Ok`; a closed worker reports "the audio thread is gone" from `set_balance`.
- `serve_commands` applies a remembered balance to the player a later `Play` creates and to the current player on `SetBalance`.
- `AppleMusicService::set_output_balance(-0.4)` records one `AudioCall::SetBalance(-0.4)` and leaves `AppState::balance` unchanged; `play_track` records `SetVolume`, then `SetBalance` carrying the state's current balance, then `Play`.
- Driving `Message::BalanceChange(-0.4)` stores `-0.4` in `AppState::balance` and records `SetBalance(-0.4)`; an out-of-range value stores and forwards the clamped end, and `NaN` stores and forwards `0.0`.

## Done
