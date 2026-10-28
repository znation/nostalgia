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

### Flash the volume on the LCD after a key or wheel nudge, like Winamp (planned 2026-10-27 by the plan role)

**Goal.** Winamp's title display briefly reads `VOLUME: 72%` when the volume changes from the
keyboard or the mouse wheel. Today only a volume *drag* swaps the LCD title for the level, through
`player.mixer_drag` and `views::mixer_readout_text`. A nudge (`handlers::mixer::volume_up` /
`volume_down`, reached from the Up/Down arrows, keypad 8/2, and `volume_wheel`) changes the level
and leaves the title alone. After a nudge, show the drag's own `Volume: N%` text in the title's
place for about one second (four `MARQUEE_TICK`s of 250 ms), then bring the title back. The
waiting Shuffle-rounds plan touches none of these files, so either can merge first.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) volume_flash_ticks: u8` (initialized to 0 beside
  `mixer_drag`) with a doc comment. It holds the number of LCD ticks the nudge readout has left and
  is never saved. Add `pub(in crate::ui) const VOLUME_FLASH_TICKS: u8 = 4;` with a doc comment
  saying it is one second at `MARQUEE_TICK`. Add `needs_lcd_tick(&self) -> bool`, which returns
  `self.volume_flash_ticks > 0 || self.needs_marquee()`. Leave `needs_marquee` unchanged.
- `src/ui/mod.rs`: pass `player.needs_lcd_tick()` as `subscriptions`' first argument instead of
  `player.needs_marquee()`. Then the existing marquee timer also runs while a flash is counting
  down, and no new thread or subscription module is needed. Update the `subscriptions` doc
  comment in `src/ui/subscriptions/mod.rs` to say the timer also runs while a volume flash shows.
- `src/ui/handlers/mixer.rs`: `volume_up` and `volume_down` set
  `player.volume_flash_ticks = VOLUME_FLASH_TICKS` before `mutate_volume`, so `volume_wheel`
  inherits it, and its zero/NaN no-op path still sets nothing. `balance_wheel`, `eq_preamp_wheel`
  and `eq_band_wheel` set it to 0 on their non-no-op paths, so a balance or gain roll never shows a
  stale volume. `mixer_released` must **not** touch it: `mixer_key_message`
  (`src/ui/subscriptions/listeners.rs`) turns the same Up/Down key press into
  `Message::MixerReleased`, and clearing the flash there would cancel it immediately.
- `src/ui/handlers/playback.rs` `marquee_tick`: while `volume_flash_ticks > 0`, decrement it and
  leave `marquee_offset` alone, so the title resumes where it paused (as it does after a drag
  readout). Otherwise advance the offset as today.
- `src/ui/view/player_panels.rs`: in the readout chain passed to `views::view_now_playing`, after
  the `mixer_drag` readout and the `seek_preview` readout, add one more `or_else`. It shows
  `views::mixer_readout_text(views::MixerDrag::Volume, volume, balance, eq_preamp, &eq_bands)`
  while `player.volume_flash_ticks > 0`. A drag or a seek drag therefore takes precedence.

**Files touched.** `src/ui/player.rs`, `src/ui/mod.rs`, `src/ui/subscriptions/mod.rs` (doc
only), `src/ui/handlers/mixer.rs`, `src/ui/handlers/playback.rs`, `src/ui/view/player_panels.rs`.
New test file `src/ui/tests/volume_flash_tests.rs`, registered with `mod volume_flash_tests;` in
`src/ui/tests/mod.rs` (`mixer_tests.rs` is already over 1000 lines). Plus `docs/usage.md` and
`docs/invariants.md`.

**Acceptance criteria.** The tests live in `volume_flash_tests.rs` and are synchronous
`#[test]`s using `test_player`, `seed_current_title` and `root_view_draws`.
- `Message::VolumeUp` (and separately `Message::VolumeDown`) with a seeded title `Tune` makes
  the root view draw `Volume: N%` for the new level and not `Tune`. `player.needs_lcd_tick()`
  is true.
- After three `Message::MarqueeTick`s the readout still shows. After the fourth the view draws
  `Tune` again, `volume_flash_ticks` is 0, and `needs_lcd_tick()` matches `needs_marquee()`.
- With a title long enough that `needs_marquee()` holds, a nudge followed by four ticks leaves
  `marquee_offset` unchanged, and a fifth tick advances it by one.
- `Message::VolumeWheel(1.0)` shows the readout. `Message::VolumeWheel(0.0)` and
  `Message::VolumeWheel(f32::NAN)` leave `volume_flash_ticks` at 0.
- `Message::MixerReleased` sent after a nudge leaves the readout showing.
- A volume nudge followed by `Message::BalanceWheel(1.0)` clears the flash, and the view draws
  `Tune`. The same holds for `Message::EqPreampWheel(1.0)`.
- While `mixer_drag` is `Some(views::MixerDrag::Balance)` during a flash, the view draws the
  `Balance:` readout, not `Volume:`.
- The existing `dragging_volume_shows_the_level_until_release`,
  `a_key_or_wheel_event_ends_a_mixer_readout`, and the volume-wheel tests pass unchanged.
- `docs/usage.md`: in the sentence about the drag readouts ("While a volume or balance slider is
  dragged, the LCD shows its level …"), add that an up/down-arrow, keypad 8/2, or wheel volume
  nudge shows the same `Volume: N%` readout for about a second, like Winamp. `docs/invariants.md`
  `## Volume wheel`: add the flash to the guarantee, name `volume_flash_ticks` and `marquee_tick`
  as the enforcement, and list the new tests under Pinned by. `make check` passes.

## Done
