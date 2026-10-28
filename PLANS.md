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

### Shift+V fades the playing song out and then stops, like Winamp's "Fade out and stop" (planned 2026-10-28 by the plan role)

**Goal.** Winamp's Shift+V fades the playing song out and then stops. Nostalgia binds V to Stop.
The plain-key match in `src/ui/shortcuts.rs` `shortcut` matches the letter ignoring case and allows
Shift, so Shift+V is a plain Stop today. Add the fade: Shift+V while a preview plays lowers the
output volume to silence over two seconds, then stops exactly like Stop. The saved volume never
changes, so the slider, the settings file, and the next play keep the user's level.

**Approach.**
- Fixed fade length: `FADE_OUT_TICKS: u8 = 8` steps of the existing LCD timer (`MARQUEE_TICK`,
  250 ms), so about 2 s. This plan makes that call (it fits 30-second previews), so no question
  is left open. Put the constant beside `MIXER_FLASH_TICKS` in `src/ui/player.rs`. Add a
  `fade_out_ticks: u8` field to `WinampPlayer` (start 0, never saved). Make
  `WinampPlayer::needs_lcd_tick` also return true while `fade_out_ticks > 0`, so the marquee
  subscription runs during a fade (`src/ui/mod.rs` already passes `needs_lcd_tick()` to
  `subscriptions`).
- New `Message::FadeOutAndStop` in `src/ui/message.rs`, dispatched in `src/ui/mod.rs` `update`
  next to `Message::Stop`. In `shortcut`'s plain-key match, add an arm before the `"v"` → `Stop`
  arm: `"v"` with `modifiers.shift()` gives `FadeOutAndStop`. Plain V stays Stop. Ctrl+V and
  Ctrl+Shift+V stay `ToggleStopAfterCurrent`, because the Control block runs first.
- New module `src/ui/handlers/playback/fade.rs`, declared beside `preview`/`stepping`/
  `title_drag` in `src/ui/handlers/playback.rs`, holding:
  - `fade_out_and_stop(player)`: when shared state `is_playing` is false (stopped or paused;
    Pause clears it), behave exactly like `playback::stop`. When a fade is already running, do
    nothing. Otherwise set `fade_out_ticks = FADE_OUT_TICKS` and return `Task::none()`; no
    backend call happens on the press itself.
  - `fade_out_tick(player) -> Task<Message>`: called at the top of `playback::marquee_tick`,
    with the existing flash/offset logic left as it is. With `fade_out_ticks > 0`, decrement it.
    If ticks remain, forward `state.volume() * remaining / FADE_OUT_TICKS` through
    `AppleMusicService::set_output_volume`. Log a failure the way `seek_preview` does, and never
    write shared state. When it reaches 0, return `stop_restoring_volume(player)`.
    `marquee_tick` returns that task.
  - `stop_restoring_volume(player)`: does what `playback::stop` does
    (`reset_position_readout`, `begin_position_transition`, then `transport_into`). Its
    action is `service.stop().await?; service.set_output_volume(volume)`, where `volume` is
    `state.volume()` read before spawning. The restore therefore reaches the backend only after
    the audio has stopped, so a muted backend cannot carry over into a later Play-after-Stop
    resume (`resume_locked` does not re-send the volume).
  - `cancel_fade_out(player)`: when `fade_out_ticks > 0`, zero it and forward `state.volume()`
    to the backend. Otherwise do nothing.
- Interruptions: `playback::play`, `pause`, and `play_pause` call `cancel_fade_out` first.
  `track_selected` calls it once the song resolves (after its `let Some(song)` early return), so
  Next, Previous, row plays, and preview-end advances keep playing at full volume. `playback::stop`
  with a fade running zeroes the ticks and returns `stop_restoring_volume(player)`. With no fade it
  stays unchanged, so the existing `AudioCall::Stop`-only assertions still hold. A volume nudge
  during a fade stores and forwards the new level as today, and the next tick rescales from it.
- The options menu gets no row (it is a full, non-scrolling column).

**Files touched.** `src/ui/shortcuts.rs` (binding and its tests), `src/ui/message.rs`,
`src/ui/mod.rs`, `src/ui/player.rs`, `src/ui/handlers/playback.rs`, new
`src/ui/handlers/playback/fade.rs`, new `src/ui/tests/fade_tests.rs` (registered in
`src/ui/tests/mod.rs`; drive it like `transport_tests.rs`/`mixer_tests.rs` with the recording
audio fake and `AudioCall`), `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- `message_for` maps Shift+V (`"V"` and `"v"` with `Modifiers::SHIFT`) to
  `Message::FadeOutAndStop`, and plain `v` to `Message::Stop`. Ctrl+V and Ctrl+Shift+V still map
  to `ToggleStopAfterCurrent`. `assert_letter_bindings` is shared with the Shift-held uppercase
  mapper, so its `v` → `Stop` check moves to the unshifted test only.
- While playing, `FadeOutAndStop` records no audio call and makes `needs_lcd_tick()` true. Each of
  the next seven `MarqueeTick`s records exactly one `AudioCall::SetVolume` lower than the last
  (7/8, 6/8, … 1/8 of the saved volume). The eighth runs the returned task and records
  `AudioCall::Stop` followed by `SetVolume(saved volume)`. Afterwards shared state is stopped,
  `state.volume()` equals its value before the fade, and `needs_lcd_tick()` reflects only the
  title/flash again.
- While paused or stopped, `FadeOutAndStop` records the same calls as `Message::Stop` and starts
  no fade (`fade_out_ticks` stays 0).
- A second `FadeOutAndStop` mid-fade does not restart the countdown.
- `Play` (and a `TrackSelected` play) mid-fade records `SetVolume(saved volume)`, zeroes the fade,
  and later ticks record no `SetVolume` and no `Stop`. `Stop` mid-fade records `Stop` and then
  `SetVolume(saved volume)`.
- The settings file written after a full fade holds the pre-fade volume.
- `docs/usage.md`'s keyboard paragraph (beside "V stops, Ctrl+V arms …") says Shift+V fades the
  playing song out over about two seconds and then stops, leaving the volume setting unchanged,
  and acts as Stop while paused or stopped. `docs/invariants.md` gets a short `## Fade-out stop`
  section stating those guarantees and naming the new tests under Pinned by.
- `make check` passes.

## Done
