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

### Tint the equalizer preamp groove by its gain like the band sliders (planned 2026-10-24 by the plan role)

**Goal.** In the classic Winamp 2.x base skin, the preamp slider uses the same colored bar
sprites as the ten band sliders, so its groove runs from green at `-12 dB` through yellow at
`0 dB` to red at `+12 dB`. Nostalgia tints each band through `style::band_slider_style`, but
`views::equalizer::preamp_column` still passes the neutral `style::chrome_slider_style`. Make the
preamp groove follow the stored preamp gain with exactly the band tint. This change affects
presentation only. The preamp's range, step, `Message::EqPreampChange`, thumb, label, layout, and
the audio path stay as they are. It is independent of the level-tint plan above, which changes
only the transport and shade volume/balance fills. Both plans edit the `## Level slider fill`
section's closing sentence in `docs/invariants.md`, so whichever merges second rebases that one
sentence.

**Approach.**
- In `src/ui/views/equalizer.rs`, add
  `pub(super) fn preamp_slider_style(preamp: f32, status: slider::Status) -> slider::Style`
  that returns `style::band_slider_style(preamp, status)`. Make `preamp_column` pass
  `move |status| preamp_slider_style(preamp, status)` to `eq_slider_column`. A named function
  gives the test something to call, because a built `Column` does not expose its slider's
  style closure.
- Rewrite the doc comments on `eq_slider_column` and `preamp_column`. They currently say the
  preamp keeps the neutral/uniform gray `chrome_slider_style`. They should say the preamp and
  every band share the gain tint.
- Leave `src/ui/style.rs` unchanged. `band_slider_style` already clamps an out-of-range gain and
  maps NaN to the flat yellow through `band_gain_color`.

**Files touched.** `src/ui/views/equalizer.rs`, `src/ui/views/tests.rs`, `docs/invariants.md`,
`README.md`.

**Tests.** In `src/ui/views/tests.rs`, import `preamp_slider_style` through the existing
`super::equalizer::{…}` use and add
`preamp_slider_style_matches_the_band_tint_for_its_gain`. For each gain in `-12.0`, `-6.0`,
`0.0`, `6.0`, `12.0`, `20.0`, and `f32::NAN`, and for each of `slider::Status::Active`, `Hovered`,
and `Dragged`, assert that `preamp_slider_style(gain, status)` equals
`style::band_slider_style(gain, status)` on `rail.backgrounds`, `rail.width`, `rail.border`, and
`handle`. Also assert that at `-12.0` the rail's first background is
`Background::Color(style::band_gain_color(-12.0))`, and that it differs from
`style::chrome_slider_style(slider::Status::Active).rail.backgrounds.0`, so the test fails
against the old neutral groove.

**Docs.**
- `docs/invariants.md` `## Equalizer band tint`: replace "The vertical preamp slider in the band
  row keeps its uniform gray groove." with a sentence saying the vertical preamp slider's groove
  takes the same tint from the stored preamp gain. Add `views::equalizer::preamp_slider_style` to
  **Enforced by**, and the new test to **Pinned by**.
- `## Level slider fill`: change the closing sentence so that only the seek bar keeps the neutral
  `theme::LCD_BACKGROUND` groove.
- `README.md` Usage: extend the sentence "Each equalizer band slider's groove is tinted by that
  band's own gain…" so it says the preamp slider's groove is tinted the same way by the preamp
  gain. Do not touch the brief block between the `tumwater:prompt` markers.

**Acceptance criteria.**
- With the preamp at `-12 dB`, `0 dB`, and `+12 dB`, the preamp groove shows the same green,
  yellow, and red as a band slider at that gain, on both rail halves and in every interaction
  status. The chrome thumb, rail width, and border match `chrome_slider_style`.
- `preamp_column` no longer calls `style::chrome_slider_style`. A grep of
  `src/ui/views/equalizer.rs` for `chrome_slider_style` finds only doc-comment mentions or none.
  The seek bar in `src/ui/views/seek_bar.rs` still uses `chrome_slider_style`.
- `preamp_slider_style_matches_the_band_tint_for_its_gain` passes. The existing
  `band_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb`,
  `equalizer_preamp_and_db_scale_sit_left_of_the_band_sliders`, and
  `equalizer_slider_spec_matches_the_gain_clamp_and_steps_whole_decibels` pass unchanged.
- README and `docs/invariants.md` are updated as above, and `make check` passes.

## Done
