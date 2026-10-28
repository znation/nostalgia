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

### Let the LCD spectrum bars fall off gradually, like Winamp's analyzer falloff (planned 2026-10-27 by the plan role)

**Goal.** In Winamp the analyzer bars jump up to a louder level right away but sink back down
over several frames ("analyzer falloff"). Their peak markers fall more slowly still. Ours behave
differently. `handlers::playback::preview::spectrum` (`src/ui/handlers/playback/preview.rs`)
stores each reply as-is (`player.spectrum = spectrum`), so a band that goes quiet drops to the
rail in one poll, about 250 ms. Only the peak markers (`next_spectrum_peaks`) hold and fall. Make
the bars fall too, faster than the markers.

**Approach.**
- `src/ui/views/now_playing/visualizer.rs`: add `const LCD_VISUALIZER_BAR_FALL: f32 = 0.25;`
  next to `LCD_VISUALIZER_PEAK_FALL`, with a doc comment: the most a bar may drop per spectrum
  reply, so a band falls from full to the rail in four polls (about one second at the four-a-second
  poll). Move the body of `next_spectrum_peaks` into a private
  `fn fall_toward(previous: Option<Spectrum>, spectrum: Option<Spectrum>, fall: f32) -> Option<Spectrum>`
  that applies the same rule with `fall` in place of the constant: missing `spectrum` → `None`;
  missing `previous` counts as zeros; each band is clamped `0.0..=1.0` with NaN as zero, then
  `level.max(held - fall).max(0.0)`. `next_spectrum_peaks` becomes `fall_toward(previous,
  spectrum, LCD_VISUALIZER_PEAK_FALL)`. Add `pub fn next_spectrum_bars(previous, spectrum)` =
  `fall_toward(previous, spectrum, LCD_VISUALIZER_BAR_FALL)`, documented like the peaks function.
  Re-export it beside `next_spectrum_peaks` wherever that is re-exported (`views` module, and the
  `use` list in `src/ui/views/tests/mod.rs`).
- `src/ui/handlers/playback/preview.rs` `spectrum`: inside the existing `reply_is_current`
  guard, keep computing the peaks from the raw reply, then set
  `player.spectrum = views::next_spectrum_bars(player.spectrum, spectrum)`. Update the doc
  comment: the reply advances the bars' falloff and the peaks' hold-and-fall. The peaks fall
  0.125 per reply and the bars 0.25, so a peak never ends up below its bar. Nothing else changes:
  Stop and a new selection still clear `spectrum` (`handlers/playback.rs`), Pause still freezes
  the bars because the poll is not subscribed while paused, and `visible_spectrum` still hides
  them.
- Tests:
  - In `src/ui/views/tests/now_playing.rs`, add
    `next_spectrum_bars_rise_at_once_and_fall_a_quarter_per_reply`. A band at 1.0 then a reply
    of 0.0 gives 0.75, then 0.5. A louder reply replaces the falling level immediately. NaN and
    out-of-range levels clamp. A `None` reply gives `None`. The bar falls further per reply than
    `next_spectrum_peaks` does from the same inputs.
  - In `src/ui/tests/readouts_tests.rs`, update `spectrum_reply_stores_the_loaded_previews_spectrum`.
    After the lower reply (band 2 goes 0.5 → 0.0), `player.spectrum.unwrap().bands[2]` is 0.25,
    not `lower`. The peak is still 0.375, and a `None` reply still clears both. Rename it
    `spectrum_reply_stores_the_bars_and_lets_a_quieter_band_fall`.
  - `spectrum_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped` should pass
    unchanged. Its kept `recording_spectrum()` (band 2 = 0.5) falls to 0.25, which equals the
    stale value `[0.25; BAND_COUNT]` it then expects. Confirm this when you run it.
- `docs/usage.md`: in the LCD visualizer paragraph, after "A thin gray peak marker above each bar
  jumps up with the bar and then drops back slowly", say the bars also jump up at once but sink
  back over about a second when a band goes quiet, faster than their markers.
- `docs/invariants.md`, LCD visualizer bullet (the one naming `next_spectrum_peaks`): state that
  `WinampPlayer::spectrum` holds the drawn bars, advanced by `next_spectrum_bars` (the higher of
  the band and the previous bar less 0.25), inside the same reply guard. Pin it with the new test
  and the renamed readouts test. Note that the peaks fall slower (0.125), so a peak is never below
  its bar.

**Acceptance criteria.**
- While playing, a band that goes quiet drops 0.25 of the strip per spectrum reply instead of
  falling to the rail at once. A louder reply raises it immediately. The peak markers still hold
  and fall 0.125 per reply.
- Pause still freezes the bars, while Stop and a new selection still clear them. A stale reply or
  one that arrives during a transition is still dropped, and hiding the visualizer still hides
  the bars.
- The new and updated tests pass, docs/usage.md and docs/invariants.md describe the falloff, and
  `make check` passes.

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
