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
