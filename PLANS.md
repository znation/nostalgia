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

### Press Pause again to resume a paused song, like Winamp's Pause/Unpause (planned 2026-10-27 by the plan role)

**Goal.** In Winamp 2.x the Pause button and the C key toggle the pause: pressing either while
paused unpauses and playback continues from where it stopped. Nostalgia's Pause does nothing on
a second press. `AppleMusicService::pause` (`src/apple_music/playback.rs`) always calls
`audio.pause()` and `AppState::pause` (`src/state.rs`), whose doc comment says it is "an
explicit, idempotent set, so a second press keeps the player paused rather than resuming".
`play_and_pause_set_the_playback_flag_explicitly` (`src/ui/tests/transport_tests.rs`) pins that
with its "pause again" step. Make Pause while paused resume the loaded preview. Pause while
playing still pauses, and Pause while stopped still leaves the transport stopped.

**Approach.**
- `src/apple_music/playback.rs`: in `AppleMusicService::pause`, take the transport lock as
  `let mut transport` and read `self.state.lock().await.paused` under it, dropping that state
  guard before going on. When `paused && transport.loaded`, return
  `self.resume_locked(&mut transport).await`. That calls `audio.resume()` and `AppState::play`,
  the same path `toggle_play_pause` takes while paused. Otherwise keep today's body unchanged:
  `audio.pause()`, then `state.pause()` when loaded or `state.stop()` when not. Doing the read and
  the call under the one transport lock means two Pause presses close together run one after
  the other, the way `concurrent_toggles_serialize_on_the_transport_lock` pins for the toggle.
  Update the method's doc comment to describe the unpause.
- `src/state.rs`: `AppState::pause` itself stays idempotent, because the service now chooses
  between pause and resume. Reword its doc comment so it no longer says a second press of the
  button or C key keeps the player paused. Say instead that the service resumes a paused preview
  before calling this.
- `src/ui/handlers/playback.rs`: reword the `pause` and `play_pause` doc comments. Pause now
  unpauses a paused preview, so it differs from the media Play/Pause toggle only in what it does
  while stopped (it does not start playback). No code change is needed here.
- `Message::Pause` is already sent by every Pause control: the transport row and shade strip `II`
  glyphs, the C key, and the media Pause key (`shortcuts::shortcut`). All of them get the
  unpause, which matches Winamp, whose Pause command toggles too. No new message or mapping is
  needed.

**Files touched.** `src/apple_music/playback.rs`, `src/state.rs`,
`src/ui/handlers/playback.rs`, `src/apple_music/tests/transport.rs`,
`src/ui/tests/transport_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Tests.**
- `src/apple_music/tests/transport.rs`: add `pause_while_paused_resumes_the_preview`, modeled on
  `toggle_play_pause_resumes_while_paused`. Build `recording_service_with_preview` and call
  `service.pause()` twice. Assert that the recorded calls are exactly
  `[AudioCall::Pause, AudioCall::Resume]`, that `assert_playback_state(&state, Some("song-1"), true)`
  holds, and that `assert_transport_mode` gives `TransportMode::Playing`. Keep
  `pause_after_stop_leaves_the_transport_stopped` and
  `pause_without_a_loaded_preview_leaves_the_transport_stopped` passing unchanged; they pin that
  Pause never starts a stopped transport.
- `src/ui/tests/transport_tests.rs`: in `play_and_pause_set_the_playback_flag_explicitly`, change
  the "pause again" step's assertion so the second `Message::Pause` leaves `is_playing` true.
  Update the step's comment (today "C pauses even when already paused") to say C unpauses a
  paused song. Then add a third `Message::Pause` that pauses again, so the test pins the full
  pause → unpause → pause cycle.

**Docs.**
- `docs/usage.md`: in the transport paragraph ("Play starts or resumes the loaded preview and
  Pause pauses it"), say that Pause while paused resumes, like Winamp. In the keys paragraph,
  change "C pauses" to "C pauses, or resumes while paused".
- `docs/invariants.md`, `## Media keys`: the guarantee says the Pause mapping is the same
  explicit message the on-screen control sends. Add that `Message::Pause` resumes a paused
  preview, and pin it with `pause_while_paused_resumes_the_preview`.

**Acceptance criteria.**
- With a preview playing, `Message::Pause` pauses it (mode `Paused`). A second `Message::Pause`
  resumes it (mode `Playing`, backend records `Resume`). A third pauses it again.
- `Message::Pause` while stopped, or with no loaded preview, still leaves the transport `Stopped`.
  The existing tests for both cases pass unchanged.
- X (`Message::Play`) and the media Play/Pause toggle behave as before.
- The new and updated tests pass, docs/usage.md and docs/invariants.md describe the unpause, and
  `make check` passes.

### Press Play while a song plays to restart it from the start, like Winamp (planned 2026-10-27 by the plan role)

**Goal.** In Winamp 2.x, pressing Play (or X) while a song is playing restarts that song from the
beginning. In Nostalgia, `Message::Play` while playing does nothing audible.
`AppleMusicService::resume` (`src/apple_music/playback.rs`) goes straight to `resume_locked`,
which calls `audio.resume()` on a player that is already running. Make Play while playing seek the
loaded preview back to zero. Play while paused still resumes from the paused position. Play while
stopped still restarts from the remembered URL. The media Play/Pause toggle is unchanged. Decision:
Winamp's Play-while-paused restart is out of scope; resuming a paused song is documented
behavior today, and the waiting Pause/Unpause plan builds on it.

**Approach.**
- `src/apple_music/playback.rs`, `AppleMusicService::resume`: after taking the transport lock,
  read `self.state.lock().await.is_playing` and drop that guard. When `transport.loaded` and
  playing are both true, call `self.seek_output_to(Duration::ZERO)?`. Then
  `println!("Restarted playback")` and return `Ok(())`, leaving the shared state alone (it stays
  playing). Otherwise call `self.resume_locked(&mut transport).await` as today. Leave
  `resume_locked` alone, since `toggle_play_pause` reaches it only when not playing. Update
  `resume`'s doc comment to describe the restart. Import `std::time::Duration`.
- Do the check in the service, not in the UI handler. `handlers::playback::play`
  (`src/ui/handlers/playback.rs`) cannot read `is_playing` with `blocking_lock`: the
  `#[tokio::test]` transport tests drive `update(…, Message::Play)` inside a runtime, where
  `blocking_lock` panics. Leave `play` unchanged except for its doc comment ("Play resumes (or
  restarts) the preview") — mention that it restarts a playing preview. Its
  `begin_position_transition` already holds the position poll until `TransportSettled`. The next
  once-per-second poll then shows the restarted position, so the readout catches up within about
  a second. Do not try to reset `playback_position` from the UI.
- This entry and the waiting "Press Pause again to resume a paused song" entry both edit
  `play_and_pause_set_the_playback_flag_explicitly` and the same usage.md transport sentence. Each
  edits a different half, so whichever merges second adapts its edit to the other's text.

**Files touched.** `src/apple_music/playback.rs`, `src/ui/handlers/playback.rs` (doc comment
only), `src/apple_music/tests/transport.rs`, `src/ui/tests/transport_tests.rs`, `docs/usage.md`,
`docs/invariants.md`.

**Tests.**
- `src/apple_music/tests/transport.rs`: add `resume_while_playing_restarts_the_preview_from_zero`,
  modeled on `resume_resumes_the_audio_backend`. Build `recording_service_with_preview` and call
  `service.resume()` once without pausing. Assert the recorded calls are exactly
  `[AudioCall::SeekTo(Duration::ZERO)]`, `assert_playback_state(&state, Some("song-1"), true)`, and
  `assert_transport_mode(&state, TransportMode::Playing)`.
- Same file: add `resume_while_playing_reports_a_seek_failure`. Use
  `service_with_preview_playing(Arc::clone(&audio) as …)` over `FailingAudio::failing_seek()`
  (see `assert_seek_failure_propagates` in `src/apple_music/tests/audio_forwarding.rs`). Assert
  that `resume()` returns the error "the audio device is gone",
  `audio.attempts(FailingCommand::Seek) == 1`, and the state is still playing `song-1`.
- `src/ui/tests/transport_tests.rs`, `play_and_pause_set_the_playback_flag_explicitly`: the
  "play again" step keeps `is_playing` true. Change its comment ("X plays even when already
  playing…") to say X restarts a playing song. In `transport_messages_drive_the_audio_backend`,
  add a Play-while-playing case over a fresh `RecordingAudio`. Drive `Message::Play` through
  `assert_transport_settled` without pausing first, and assert the recorded calls are
  `[AudioCall::SeekTo(Duration::ZERO)]`.
- The existing `resume_resumes_the_audio_backend`, `resume_after_stop_restarts_the_preview`, and
  `toggle_play_pause_*` tests pass unchanged. They pin that paused, stopped, and toggle paths
  did not move.

**Docs.**
- `docs/usage.md`: in the transport paragraph ("Play starts or resumes the loaded preview"), add
  that Play restarts it from its start while it is already playing, like Winamp. Do the same in
  the playback paragraph's "Play resumes a paused preview or restarts a stopped one from the
  beginning".
- `docs/invariants.md`: add a `## Play restart` section after `## Previous restart`. Guarantee:
  Play on a loaded, playing preview sends exactly one absolute seek to zero and leaves the
  current track and `Playing` mode unchanged. Paused and stopped Play keep their resume and replay
  paths, and a rejected seek is reported without changing state. Enforced by
  `AppleMusicService::resume`. Pinned by the two new service tests and the new
  `transport_messages_drive_the_audio_backend` case.

**Acceptance criteria.**
- With a preview playing, `Message::Play` (Play button, X, keypad 5, media Play key) records one
  `SeekTo(Duration::ZERO)` and no `Resume` or `Play`, and the transport stays `Playing` on the
  same track.
- Play while paused still records `Resume`. Play after Stop still replays the remembered URL. The
  media Play/Pause toggle still pauses while playing.
- A backend that rejects the seek makes `resume` return its error after one attempt, with the
  state unchanged.
- The new and updated tests pass, docs/usage.md and docs/invariants.md describe the restart, and
  `make check` passes.

## Done
