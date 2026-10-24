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

_None yet._

### Show the volume or balance level on the LCD while dragging its slider (planned 2026-10-24 by the plan role)

**Goal.** In classic Winamp, dragging the volume or balance slider swaps the scrolling title for
the level being set (`Volume: 72%`, `Balance: Center`, `Balance: 31% Left`) until the mouse is
released. Nostalgia's sliders change the level silently, and nothing shows the exact value. Make a
drag on any mixer slider show its live level in the Now Playing title slot, and bring the title
back on release. Arrow keys and the mouse wheel do not show the readout. No persisted setting
and no audio-backend change.

**Approach.**
- New file `src/ui/views/now_playing/mixer_readout.rs` (declared `pub(super) mod mixer_readout;`
  in `src/ui/views/now_playing.rs`, re-exported through `now_playing.rs` and `views/mod.rs` the
  same way as `TimeDisplay`). It holds:
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum MixerDrag { Volume, Balance }`.
  - `pub fn mixer_readout_text(drag: MixerDrag, volume: f32, balance: f32) -> String`.
    `Volume` → `format!("Volume: {}%", (volume * 100.0).round() as u32)`. `Balance` → let
    `pct = (balance * 100.0).round() as i32`. Then `0` → `"Balance: Center"`, negative →
    `"Balance: {-pct}% Left"`, positive → `"Balance: {pct}% Right"`. This matches
    `audio/balance.rs`, where `-1.0` is hard left. A balance that rounds to zero, such as
    `-0.004`, reads `Center`, never `-0%`. The inputs are already clamped by
    `AppState::volume()`/`balance()`.
- `src/ui/player.rs`: add a field `pub(in crate::ui) mixer_drag: Option<views::MixerDrag>`,
  initialised `None` and documented as transient (not saved to settings).
- `src/ui/message.rs`: add `MixerReleased`, commented as the release of any `chrome_slider`
  drag. `src/ui/mod.rs` routes it to a new `handlers::mixer::mixer_released`, which sets
  `mixer_drag = None` and returns `Task::none()`.
- `src/ui/handlers/mixer.rs`: `volume_change` sets `player.mixer_drag =
  Some(MixerDrag::Volume)` and `balance_change` sets `Some(MixerDrag::Balance)`, both before
  their existing `mutate_*` call. `volume_up`, `volume_down`, and `volume_wheel` stay untouched,
  so keys and the wheel never show the readout.
- `src/ui/views/mod.rs` `chrome_slider`: add `.on_release(Message::MixerReleased)`. That one call
  covers the transport volume and balance sliders (`views/transport.rs`) and the compact volume
  slider (`mini_transport.rs`, used by the library footer and the shade strip). The equalizer and
  seek sliders do not use `chrome_slider`, so they are unaffected. A release while the readout is
  already `None` is a harmless no-op.
- LCD: add a trailing parameter `mixer_readout: Option<String>` to `view_now_playing`,
  `now_playing_content`, and `now_playing_row`, and pass `None` at each existing test call site.
  In `now_playing_row`, `Some(text)` pushes one `lcd_text(text)` where the `"Now Playing: "`
  caption and the number/artist/title (or marquee window) would go. The transport glyph, elapsed
  readout, and right-edge total stay as they are. `None` keeps today's row exactly.
- `src/ui/view.rs` (normal tree): pass
  `player.mixer_drag.map(|drag| views::mixer_readout_text(drag, volume, balance))`, using the
  `volume` and `balance` already read under the lock. The text therefore follows the live
  clamped value on every `VolumeChange`/`BalanceChange`.
- `WinampPlayer::needs_marquee` returns `false` while `mixer_drag.is_some()`, so the marquee
  timer pauses during a drag. `marquee_offset` is left unchanged.
- `WinampPlayer::window_title` is not changed.
- Known limit, recorded in the invariant: if a slider were removed mid-drag, iced would send no
  release. The readout would then stay until the next mixer-slider release. No current control
  removes the transport row or footer while a mouse button is held.

**Files touched.** `src/ui/views/now_playing/mixer_readout.rs` (new), `src/ui/views/now_playing.rs`,
`src/ui/views/mod.rs`, `src/ui/view.rs`, `src/ui/player.rs`, `src/ui/message.rs`, `src/ui/mod.rs`,
`src/ui/handlers/mixer.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md` (`## Usage`:
one sentence after the volume/balance slider description), `docs/invariants.md` (new
`## Mixer drag readout` section: guarantee, enforced by, pinned by).

**Acceptance criteria.**
- Unit test in `src/ui/views/tests.rs` (or the new file's own `#[cfg(test)]` block):
  `mixer_readout_text` gives `Volume: 72%` for `0.72`, `Volume: 0%`/`Volume: 100%` at the ends,
  `Balance: Center` for `0.0` and `-0.004`, `Balance: 31% Left` for `-0.31`, and
  `Balance: 100% Right` for `1.0`.
- `captured_now_playing_texts`-style test: `now_playing_row` with `Some("Volume: 72%")` draws
  that fragment and no `Now Playing: ` caption, title, artist, or number. The glyph, elapsed
  readout, and `3:30` total stay. A marquee-length title with a readout set draws no marquee
  window.
- Root tests in `src/ui/tests.rs`:
  - `Message::VolumeChange(0.4)` sets `mixer_drag` to `Volume`, and the root view text
    (`captured_root_view_text_bounds`) contains `Volume: 40%` but not the current track's title.
  - `Message::MixerReleased` then clears it, and the title is drawn again.
  - `BalanceChange(-0.25)` shows `Balance: 25% Left`.
  - `VolumeUp` and `VolumeWheel(1.0)` leave `mixer_drag` `None`.
  - `needs_marquee()` is `false` during a drag of an overlong title and `true` again after
    release.
- Volume/balance audio forwarding and settings saving behave as before (the existing mixer tests
  still pass).
- README and `docs/invariants.md` updated as above; `make check` passes.

## Done
