# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the README's Usage section: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. Record any new drift here rather than appending
> another note.

## Planned

### Restart the current preview with Previous after three seconds (planned 2026-10-24 by the plan role)

**Goal.** Classic Winamp's Previous restarts the track already playing once it has played at
least three seconds; only earlier does it step to the preceding song. Nostalgia always steps, so a
mistimed Previous skips the current track. Make Previous — the button, the Z key, and the media
Previous key, which all share `Message::PreviousTrack` — restart the loaded preview from zero when
its last reported position is at least three seconds, while keeping the existing step (including
Shuffle's recorded-play retrace) for earlier presses and when no preview reports a position. No
new message, player flag, or setting.

**Approach.**

- `src/ui/handlers/playback.rs`: add a private `PREVIOUS_RESTART_SECS: u64 = 3` constant. At the
  start of `step_track`, before taking the state lock, when `!forward`, `player.playback_position`
  is at least that threshold, and `player.apple_music_service.has_loaded_preview()` holds, clear
  `player.seek_preview`, call `player.apple_music_service.seek_output_to(Duration::ZERO)`, and on
  success bump `player.position_epoch` and store `player.playback_position = Some(Duration::ZERO)`,
  then return `Task::none()`. On error, log in the existing `eprintln!` style and return
  `Task::none()` with the position and epoch unchanged. The restart branch must precede the
  sequential/Shuffle computation, so a late Previous neither steps nor consumes a Shuffle history
  entry; earlier positions fall through unchanged. Update `step_track`'s doc comment to state the
  restart rule and that it uses the last reported position (the one the readout shows), not a
  fresh backend query.
- `src/ui/tests.rs`: add tests written against the literal `Duration::from_secs(3)` so a changed
  constant fails:
  - `previous_restarts_the_preview_after_three_seconds`: build `player_playing_preview`, set
    `player.songs.items = stepping_songs()` and `player.playback_position = Some(Duration::from_secs(3))`,
    drive `Message::PreviousTrack`, and assert no task, one `AudioCall::SeekTo(Duration::ZERO)`
    after the preview-start prefix, `player.playback_position == Some(Duration::ZERO)`,
    `player.seek_preview == None`, the epoch bumped, and `state.current_track` unchanged.
  - `previous_steps_before_the_restart_threshold`: the same fixture with
    `Some(Duration::from_secs(2))`; drive Previous, assert `TrackSelected` index 0 and no `SeekTo`
    call.
  - `previous_with_shuffle_restarts_after_the_threshold_instead_of_retracing`: the same fixture
    with `shuffle = true` and a recorded play; at three seconds assert no `TrackSelected`, one
    `SeekTo(Duration::ZERO)`, and the current track unchanged.
  - `a_failed_previous_restart_leaves_the_track_queued`: load the preview over
    `FailingAudio::failing_seek`, set the position to three seconds, drive Previous, and assert
    `attempts(FailingCommand::Seek) == 1`, no task, and unchanged position, epoch, and current
    track.
- `README.md`: in the keyboard/transport sentence, say Previous (and Z) restarts the playing
  preview once it has played at least three seconds and steps back before that; keep the Shuffle
  sentence describing the recorded-play retrace for earlier presses. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add `## Previous restart` after `## Play history` with the guarantee,
  `handlers::playback::step_track` (with `transport::previous_track_id` and
  `transport::PlayHistory` for the unchanged step paths) as **Enforced by**, the four test names
  under **Pinned by**, and a History line.

**Files touched.** `src/ui/handlers/playback.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- A Previous press at a reported position of at least three seconds with a loaded preview sends
  exactly one absolute seek to zero, clears any seek drag, resets the readout position,
  invalidates in-flight position replies, and leaves the current track and transport mode
  unchanged.
- A Previous press before three seconds, or with no loaded preview or reported position, steps
  exactly as today, including Repeat's edge behavior and Shuffle's recorded-play retrace, and
  sends no seek.
- A rejected restart seek is logged and leaves the track queued with the position and epoch
  unchanged; no step is scheduled.
- No new `Message` variant, player flag, or persisted setting is added.
- The new tests and `make check` pass.

## Done
