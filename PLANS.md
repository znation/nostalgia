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

## Done
