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

### Show an equalizer band's or the preamp's gain on the LCD while dragging its slider (planned 2026-10-24 by the plan role)

**Goal.** In classic Winamp, dragging an equalizer band or the preamp replaces the scrolling title
with the gain being set until the mouse is released, the same way the volume and balance sliders
do. Nostalgia already does this for the mixer (`## Mixer drag readout` in `docs/invariants.md`),
but its eleven equalizer sliders change gain without showing anything. Extend that readout to
cover them. A band drag reads `EQ: 60HZ +3 DB`, and the preamp reads `Preamp: -12 DB`. The audio
path, settings, and slider ranges stay as they are. This plan does not depend on the preamp-tint
plan above. Both edit `views::equalizer::preamp_column`: that plan swaps its style closure, and
this one leaves the body alone. They also both touch `eq_slider_column`'s doc comment, so
whichever merges second rebases those lines.

**Approach.**
- `src/ui/views/now_playing/mixer_readout.rs`:
  - Add the variants `EqPreamp` and `EqBand(usize)` to `MixerDrag`. It stays `Copy`/`Eq`.
  - Extend the signature to
    `mixer_readout_text(drag, volume, balance, preamp: f32, bands: &[f32; BAND_COUNT])`. Import
    `crate::equalizer::{BAND_COUNT, BAND_FREQUENCIES}`.
  - Round the gain once with `let db = gain.round() as i32;`. This gives no `-0`, and NaN becomes
    `0`.
  - `EqPreamp` → `format!("Preamp: {db:+} DB")`.
  - `EqBand(i)` → `format!("EQ: {}HZ {db:+} DB", BAND_FREQUENCIES[i])`, using `bands[i]`.
  - An out-of-range `i` returns `String::new()`. Use `.get()` and never index directly.
  - Update the module doc to cover the EQ forms, and pass `0.0, &[0.0; BAND_COUNT]` in the two
    existing tests.
- `src/ui/handlers/mixer.rs`:
  - `eq_preamp_change` sets `player.mixer_drag = Some(views::MixerDrag::EqPreamp)` before its
    `mutate_equalizer` call.
  - `eq_band_change` sets `Some(views::MixerDrag::EqBand(band))`, but only when
    `band < BAND_COUNT`. An out-of-range band is already a no-op in `AppState::set_eq_band`.
  - `toggle_equalizer` and `eq_preset_selected` stay untouched, so the on/off button and a preset
    pick never show the readout.
  - Update `mixer_released`'s doc comment to say "a mixer or equalizer slider drag".
- `src/ui/views/equalizer.rs` `eq_slider_column`: add `.on_release(Message::MixerReleased)` to the
  `VerticalSlider`. iced 0.14's `VerticalSlider::on_release` exists. This one call covers the
  preamp and all ten bands. Mention the release in the doc comment.
- `src/ui/message.rs`: reword the `MixerReleased` doc to "The release of any `chrome_slider` or
  equalizer slider drag".
- `src/ui/view.rs`: the normal-mode closure calls
  `views::mixer_readout_text(drag, volume, balance, eq_preamp, &eq_bands)`. Both values are
  already read under the lock in the same function. Check whether they are moved into
  `view_equalizer` before the LCD is built, and borrow or reorder if so.
- Nothing else needs to change. `needs_marquee`, `timers::subscriptions` (via
  `mixer_drag.is_some()`), `mixer_key_message`, and `mixer_wheel_message` all already key off
  `mixer_drag`. So a focused EQ slider's arrow-key or Ctrl+wheel change shows the readout, and
  the next navigation key or wheel roll clears it, exactly as it does for the mixer.

**Files touched.** `src/ui/views/now_playing/mixer_readout.rs`, `src/ui/handlers/mixer.rs`,
`src/ui/views/equalizer.rs`, `src/ui/message.rs`, `src/ui/view.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Tests.**
- In `mixer_readout.rs`'s test module, add `eq_readouts_show_the_band_label_and_signed_gain`.
  It asserts:
  - `EqPreamp` at `-12.0` → `Preamp: -12 DB` and at `0.0` → `Preamp: +0 DB`.
  - `EqBand(0)` with `bands[0] = 3.0` → `EQ: 60HZ +3 DB`.
  - `EqBand(4)` with `bands[4] = -0.4` → `EQ: 1KHZ +0 DB`.
  - `EqBand(9)` with `bands[9] = 12.0` → `EQ: 16KHZ +12 DB`.
  - `EqBand(BAND_COUNT)` → `""`.
- In `src/ui/tests.rs`, next to `dragging_volume_shows_the_level_until_release`, using
  `seed_current_title` and `root_view_draws`:
  - `dragging_an_eq_band_shows_its_gain_until_release`: `EqBandChange(0, 5.0)` sets
    `mixer_drag` to `EqBand(0)`. The root view draws `EQ: 60HZ +5 DB` and not the title.
    `MixerReleased` clears the readout, and the title is drawn again.
  - `dragging_the_preamp_shows_its_gain`: `EqPreampChange(-3.0)` sets `EqPreamp` and draws
    `Preamp: -3 DB`.
  - `eq_toggle_and_preset_do_not_show_the_readout`: `ToggleEqualizer` and an
    `EqPresetSelected(..)` with any preset from `PRESETS` both leave `mixer_drag` `None`.
  - `an_out_of_range_eq_band_shows_no_readout`: `EqBandChange(BAND_COUNT, 5.0)` leaves
    `mixer_drag` `None`.

**Docs.**
- `docs/invariants.md` `## Mixer drag readout`: widen the guarantee from "a volume or balance
  `chrome_slider`" so that it also covers the preamp and band sliders, and give the two EQ text
  forms. State that the EQ on/off button and preset picks never set the readout. Add
  `eq_preamp_change`, `eq_band_change`, and `eq_slider_column`'s `on_release` to **Enforced by**,
  and the new tests to **Pinned by**.
- `README.md` Usage: after the existing sentence about the volume/balance drag readout (grep
  `Volume: `), add one sentence saying an equalizer band or preamp drag shows
  `EQ: 60HZ +3 DB` / `Preamp: -12 DB` the same way. Do not touch the brief block.

**Acceptance criteria.**
- While the mouse holds any band slider or the preamp slider, the LCD's title slot shows that
  slider's live rounded gain in the forms above. The glyph, elapsed readout, and total stay.
  Release restores the title.
- The EQ button, the preset list, and the volume/balance keys and wheel never set the readout.
  An out-of-range band index sets nothing.
- The existing mixer-readout tests (`dragging_volume_shows_the_level_until_release`,
  `dragging_balance_shows_the_side`, `volume_keys_and_wheel_do_not_show_the_readout`,
  `the_marquee_pauses_during_a_mixer_drag`, and the `mixer_readout.rs` unit tests) pass with only
  the new arguments added. The new tests pass.
- README and `docs/invariants.md` are updated as above, and `make check` passes.

## Done
