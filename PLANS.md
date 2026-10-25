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


### Click an equalizer dB scale label to set every band to that gain (planned 2026-10-24 by the plan role)

**Goal.** Bring over the classic Winamp equalizer shortcut. In Winamp 2.x, clicking the dB
labels beside the preamp sets every band slider at once: the top label sets them all to
maximum, the middle one to zero, and the bottom one to minimum. Nostalgia draws the same three
labels (`+12 DB`, `0 DB`, `-12 DB` in `db_scale_column`, `src/ui/views/equalizer.rs`), but they
are plain text. Right now, flattening a curve means dragging ten sliders or picking the Flat
preset, and Flat also resets the preamp. After this change, a left press on a label stores that
label's gain in all ten bands. The preamp gain and the EQ on/off flag stay as they are. The
preset selection clears, the same as for a hand-moved slider. The new curve goes to the audio
backend at once, like any other EQ change. This plan does not depend on the balance-detent
plan above.

**Approach.**
- `src/state.rs`: add `pub fn set_all_eq_bands(&mut self, gain: f32)` next to `set_eq_band`.
  It stores `equalizer::clamp_gain(gain)` in every slot of `eq_bands` and sets
  `eq_preset = None`. Its doc comment says the dB scale labels are its only caller, and that
  it leaves the preamp alone, as Winamp's labels do. Also update the field-clamp wording in
  the `set_eq_preamp`/`set_eq_band` doc comments, which currently say "both writers", so
  they list this third writer too.
- `src/ui/message.rs`: add `EqBandsSetAll(f32)` after `EqBandChange`, with a `//` comment
  in the style of its neighbours: it comes from a press on a dB scale label and carries that
  label's gain.
- `src/ui/handlers/mixer.rs`: add
  `pub(in crate::ui) fn eq_bands_set_all(player: &mut WinampPlayer, gain: f32) -> Task<Message>`
  that returns `mutate_equalizer(player, |state| state.set_all_eq_bands(gain))`. It does
  **not** set `player.mixer_drag`, because a click has no release to clear an LCD readout.
  Add a dispatch arm in `src/ui/mod.rs` beside the `Message::EqBandChange` arm.
- `src/ui/views/equalizer.rs` `db_scale_column`: wrap each of the three `Text`s in
  `MouseArea::new(..).on_press(Message::EqBandsSetAll(GAIN))` with
  `.interaction(Interaction::Pointer)`. The gains are `GAIN_MAX_DB`, `0.0` and `GAIN_MIN_DB`.
  Copy the readout's `MouseArea` usage in `src/ui/views/mini_transport.rs` and import what is
  missing. Keep the two fill `Space`s and the fixed `EQ_SLIDER_HEIGHT`, so the layout does not
  change. Update the doc comment to mention the click.
- Settings need no change: `persist_settings_if_changed` already snapshots `eq_bands()` and
  `eq_preset()`.

**Files touched.** `src/state.rs` (method and unit test), `src/ui/message.rs`, `src/ui/mod.rs`,
`src/ui/handlers/mixer.rs`, `src/ui/views/equalizer.rs`, `src/ui/tests/mixer_tests.rs`,
`src/ui/views/tests/equalizer.rs`, `docs/invariants.md`, `README.md` (Usage only).

**Acceptance criteria.**
- `src/state.rs` `tests`: add `set_all_eq_bands_clamps_every_band_and_clears_the_preset`.
  Apply a preset with `apply_eq_preset`, then call `set_all_eq_bands(99.0)`. Assert that
  `eq_bands() == [GAIN_MAX_DB; BAND_COUNT]`, that `eq_preamp()` still equals the preset's
  preamp, and that `eq_preset() == None`. Then call `set_all_eq_bands(0.0)` and assert every
  band is `0.0`.
- `src/ui/tests/mixer_tests.rs`: add `eq_scale_click_sets_every_band_and_keeps_the_preamp`.
  Use `test_player`, send `Message::EqPresetSelected(rock_preset())`, then send
  `Message::EqBandsSetAll(GAIN_MIN_DB)` through `update`. Assert that every band is
  `GAIN_MIN_DB`, that the preamp is still `rock_preset().preamp`, that `eq_preset()` is
  `None`, and that `player.mixer_drag` is `None`.
- `src/ui/views/tests/equalizer.rs`: add `pressing_a_db_scale_label_sets_every_band_to_its_gain`.
  For each `(label, gain)` in `[("+12 DB", GAIN_MAX_DB), ("0 DB", 0.0), ("-12 DB", GAIN_MIN_DB)]`:
  build `view_equalizer(true, 0.0, &[0.0; BAND_COUNT], None)`, find the label with
  `layout_capture::capture_text`, and send a left `ButtonPressed` to the label's centre with
  `layout_capture::publish_at` (the pattern is in `src/ui/views/tests/now_playing.rs`'s
  readout press test). Assert that exactly one published message is
  `Message::EqBandsSetAll(g)` with `g == gain`.
  `equalizer_preamp_and_db_scale_sit_left_of_the_band_sliders` still passes unchanged.
- `docs/invariants.md`: add a `## Equalizer scale shortcuts` section after
  `## Equalizer band tint`, in the layout of the neighbouring sections. Its guarantee: a press
  on a scale label stores that label's clamped gain in all ten bands, clears the preset,
  leaves the preamp, the on/off flag and the LCD readout untouched, and forwards the curve to
  the backend. It is enforced by `db_scale_column`, `handlers::mixer::eq_bands_set_all` and
  `AppState::set_all_eq_bands`, and pinned by the three tests above.
- README Usage: after the sentence that ends "change the playing preview's curve live;", add
  one sentence saying that clicking `+12 DB`, `0 DB` or `-12 DB` sets all ten bands to that
  gain and leaves the preamp alone. Do not touch the brief or status blocks.
- `make check` passes.

## Done
