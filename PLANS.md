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

### Flash the balance or equalizer level on the LCD after a wheel roll over its slider (planned 2026-10-27 by the plan role)

**Goal.** A drag on the balance, preamp, or an equalizer band slider shows its level
(`Balance: 31% Left`, `Preamp: -12 DB`, `EQ: 60HZ +3 DB`) in the LCD title's place, and since the
volume-flash plan a volume key or wheel nudge shows `Volume: N%` for about a second. A wheel roll
over the balance, preamp, or a band slider changes that level silently, and also cancels a volume
flash that is showing. That leaves the player inconsistent: the only way to read the new level is
to drag. After such a roll, show that slider's own drag readout for the same four
`MARQUEE_TICK`s, through the volume flash's existing timer. The slider-wheel handlers left
`mixer_drag` unset only because nothing would clear it without a release (see PLANS-DONE.md, "Point
the mouse wheel at the balance and equalizer sliders"). The timed flash removes that problem. This
plan touches none of the files the waiting Shuffle-round-stop plan touches, so either can merge
first.

**Approach.**

- `src/ui/player.rs`: rename `VOLUME_FLASH_TICKS` to `MIXER_FLASH_TICKS` and the field
  `volume_flash_ticks` to `mixer_flash_ticks`, updating their doc comments to say "a key or wheel
  nudge of a mixer level". Add `pub(in crate::ui) mixer_flash: views::MixerDrag`, initialized to
  `views::MixerDrag::Volume` beside it. Its doc comment should say it names the slider whose
  readout the flash shows, means nothing while `mixer_flash_ticks` is 0, and is never saved.
  `needs_lcd_tick` reads the renamed field; update its doc comment ("a mixer flash").
- `src/ui/handlers/mixer.rs`: add a private helper
  `fn flash(player: &mut WinampPlayer, slider: views::MixerDrag)`. It sets `mixer_flash = slider`
  and `mixer_flash_ticks = MIXER_FLASH_TICKS`. `volume_up` and `volume_down` call
  `flash(player, MixerDrag::Volume)` where they set the ticks today. `balance_wheel`,
  `eq_preamp_wheel`, and `eq_band_wheel` replace their `volume_flash_ticks = 0` lines with
  `flash(player, MixerDrag::Balance)`, `flash(player, MixerDrag::EqPreamp)`, and
  `flash(player, MixerDrag::EqBand(band))`. Each call stays after the zero/NaN/out-of-range early
  return, so a no-op roll still flashes nothing. Update those three handlers' doc comments: they
  still leave `mixer_drag` alone, and they flash the readout instead. As today, `mixer_released`
  must not touch the flash.
- `src/ui/handlers/playback.rs` `marquee_tick`: use the renamed field. Its behavior is unchanged.
- `src/ui/view/player_panels.rs`: the last `or_else` of the readout chain passes
  `player.mixer_flash` to `views::mixer_readout_text` instead of `views::MixerDrag::Volume`, still
  only while `mixer_flash_ticks > 0`. A drag readout and a seek readout still take precedence.

**Files touched.** `src/ui/player.rs`, `src/ui/handlers/mixer.rs`, `src/ui/handlers/playback.rs`,
`src/ui/view/player_panels.rs`, `src/ui/tests/volume_flash_tests.rs` (renamed fields, plus the
new tests below; retitle its module doc to cover every mixer flash), `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.** The tests are synchronous `#[test]`s in `volume_flash_tests.rs` using its
`nudged` helper (seeded title `Tune`) and `root_view_draws`.
- `Message::BalanceWheel(1.0)` makes the view draw the `Balance:` readout for the new balance
  (`Balance: N% Right`, the exact `views::mixer_readout_text` output) and not `Tune`. After four
  `Message::MarqueeTick`s the view draws `Tune` again and `mixer_flash_ticks` is 0.
  `player.mixer_drag` stays `None` throughout.
- `Message::EqPreampWheel(1.0)` draws the `Preamp:` readout, and `Message::EqBandWheel(0, 1.0)`
  draws band 0's `EQ: ` readout (the `views::mixer_readout_text(MixerDrag::EqBand(0), …)` text).
- A zero and a `NaN` direction for each of the three messages, and `Message::EqBandWheel` with an
  out-of-range band (`BAND_COUNT`), leave `mixer_flash_ticks` at 0.
- Replace `a_balance_or_preamp_roll_clears_the_flash` with a test that a volume nudge followed by
  `Message::BalanceWheel(1.0)` draws `Balance:` and not `Volume:`, and that a balance roll
  followed by `Message::VolumeUp` draws `Volume:` and not `Balance:`. Each readout replaces the
  other and restarts the four-tick count.
- The existing volume-flash tests pass with only the field renames, and so do mixer_tests.rs's
  wheel tests (which assert `mixer_drag == None`).
- `docs/usage.md`: after the sentence saying a volume nudge shows the `Volume: N%` readout for
  about a second, add that a wheel roll over the balance slider, the preamp, or a band shows that
  slider's readout the same way. `docs/invariants.md`: in `## Volume wheel`, rename the constant
  and field, and replace "a balance or equalizer wheel roll cancels it" with "a balance or
  equalizer wheel roll replaces it with that slider's readout". In `## Slider wheel`, replace "so
  the LCD keeps its title and shows no readout" with a sentence saying the LCD shows the slider's
  readout for `MIXER_FLASH_TICKS` marquee ticks through `WinampPlayer::mixer_flash`, and list the
  new tests under Pinned by. `make check` passes.

## Done
