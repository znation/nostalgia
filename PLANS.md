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

### Tint the volume groove by level and the balance groove by offset from center (planned 2026-10-24 by the plan role)

**Goal.** In the classic Winamp 2.x base skin, the volume slider's groove changes color with the
level, from green when silent to red at full volume. The balance groove is green at center and
turns red as it moves toward either side. Nostalgia paints fixed fills instead:
`theme::VOLUME_SLIDER_FILL` (orange) and `theme::BALANCE_SLIDER_FILL` (green). Make each level
slider's groove follow its value. The defaults must still match the mockups: half volume
(`AppState` default `0.5`) stays exactly the orange fill, and center balance stays exactly the
green fill. This change affects presentation only. Slider ranges, steps, messages, thumb
geometry, the seek bar, the preamp, and the EQ band tint all stay as they are. This plan is
independent of the LCD mixer-readout plan above, which changes how `chrome_slider` reports a
release. This plan changes only the `fill` that callers pass in.

**Approach.**
- New `src/ui/style/level_tint.rs`, declared from `src/ui/style.rs` as `mod level_tint;` with
  `pub use level_tint::{balance_level_color, volume_level_color};`. Rust resolves a child of
  `style.rs` to `src/ui/style/`. Putting the code there keeps the 800-line `style.rs` from
  growing. The child module may call the parent's private `mix_channel`
  (`super::mix_channel`).
  - `pub fn volume_level_color(volume: f32) -> Color`: first apply
    `crate::state::clamp_volume`, which maps NaN to `0.0`. For `v <= 0.5`, mix
    `theme::EQ_GAIN_LOW` → `theme::VOLUME_SLIDER_FILL` at `t = v / 0.5`. Above that, mix
    `theme::VOLUME_SLIDER_FILL` → `theme::EQ_GAIN_HIGH` at `t = (v - 0.5) / 0.5`. Mix each
    channel through `mix_channel`, with alpha `1.0`. The endpoint form returns each stop
    exactly.
  - `pub fn balance_level_color(balance: f32) -> Color`: compute
    `t = crate::state::clamp_balance(balance).abs()`, where NaN maps to center. Mix
    `theme::BALANCE_SLIDER_FILL` → `theme::EQ_GAIN_HIGH` at `t`, so the color is symmetric
    left and right.
  - Give each function a doc comment that names its stops, matching the style of
    `band_gain_color`.
- `src/ui/views/transport.rs` `view_transport_controls`: pass `style::volume_level_color(volume)`
  and `style::balance_level_color(balance)` to the two `chrome_slider` calls in place of the
  theme constants.
- `src/ui/views/mini_transport.rs` `mini_volume_slider`: pass `style::volume_level_color(volume)`,
  and update its doc comment, which still names the fixed `VOLUME_SLIDER_FILL` groove.
- `src/ui/views/mod.rs` `chrome_slider` doc comment: say callers pass a level-dependent fill. Do
  not change the signature.
- Keep the theme constants. They become the mid and center stops, so
  `base_skin_colors_keep_their_documented_literal_values` is unchanged.

**Files touched.** `src/ui/style/level_tint.rs` (new, with its `#[cfg(test)] mod tests`),
`src/ui/style.rs` (module declaration and re-export), `src/ui/views/transport.rs`,
`src/ui/views/mini_transport.rs`, `src/ui/views/mod.rs` (doc only), `README.md`,
`docs/invariants.md`.

**Docs.**
- `README.md` Usage currently says "a volume slider over an orange groove and a balance slider over
  a green groove". Change it to describe the shifting tint: the volume groove runs from green
  when silent through orange at half volume to red at full volume, and the balance groove is
  green at center and reddens toward either side. Do not touch the brief or status blocks.
- `docs/invariants.md` `## Level slider fill`: restate the guarantee in terms of the two
  functions and their exact stops. Add `level_tint.rs` to **Enforced by**, and list the new tests
  under **Pinned by**. Keep the sentence saying the seek bar and preamp stay on the neutral
  groove.

**Acceptance criteria.**
- Tests in `level_tint.rs` assert the following:
  - `volume_level_color` returns exactly `EQ_GAIN_LOW` at `0.0`, exactly `VOLUME_SLIDER_FILL`
    at `0.5`, and exactly `EQ_GAIN_HIGH` at `1.0`.
  - NaN and `-1.0` give `EQ_GAIN_LOW`, and `2.0` gives `EQ_GAIN_HIGH`.
  - At `0.25` and `0.75`, each channel lies between its two neighbouring stops.
  - `balance_level_color(0.0)` and `balance_level_color(f32::NAN)` are exactly
    `BALANCE_SLIDER_FILL`.
  - `balance_level_color(-1.0)` and `balance_level_color(1.0)` are exactly `EQ_GAIN_HIGH`.
  - `balance_level_color(-0.4) == balance_level_color(0.4)`.
  - Both functions return alpha `1.0`.
- The transport volume, transport balance, and mini/shade volume sliders get their fill from these
  functions. Grepping `src/ui/views` for `VOLUME_SLIDER_FILL` and `BALANCE_SLIDER_FILL` finds
  no remaining fill argument.
- `level_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb` and the EQ band-tint tests
  still pass unchanged.
- README and `docs/invariants.md` updated as above; `make check` passes.

## Done
