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

### Stop at the end of a Shuffle round when Repeat is off, like Winamp (planned 2026-10-27 by the plan role)

**Goal.** In Winamp, Shuffle with Repeat off plays the list once in shuffled order and then stops,
just as an unshuffled list stops after its last song. Today a natural preview end with Shuffle on
always moves on to another song. The Shuffle-rounds plan (see PLANS-DONE.md, "Let Shuffle play
every song once before repeating any") deliberately left this behavior unchanged:
`ShuffleRound::record` (src/ui/transport/round.rs) clears the round once every song has played,
and `ShuffleRound::pick` falls back to `transport::shuffled_track_id`. As a result,
`handlers::playback::preview::playback_finished` never sees a round end. After this change, a
natural end on the song that completed the round, with Shuffle on and Repeat off, takes the
existing `stop` path and keeps that song current for Play, the same way Repeat off stops on an
unshuffled list's last song. Next, the Next button, and Repeat on keep starting a new round as
they do now; only the preview-end advance stops.

**Approach.**
- `src/ui/transport/round.rs`: add a field `completed_by: Option<String>` to `ShuffleRound`; the
  derived `Default` gives `None`. In `record`, set it to `Some(track_id.to_owned())` in the branch
  that clears `played` (this play completed the round). Otherwise set it to `None`, so any later
  committed play clears it. That includes Play restarting the same song, which then counts as the
  first play of the new round. Add
  `#[must_use] pub fn ends_round(&self, current: Option<&str>) -> bool`, which returns true when
  `current` is `Some` and equals `completed_by`. Its doc comment says that the round is the list
  that was showing when that song's play was recorded. Leave `pick` unchanged, along with the
  existing eager clear and restart.
- `src/ui/handlers/playback/preview.rs` `playback_finished`: under the same lock, after the
  `stop_after_current` check and before `forward_track_id`, add a check: if `state.shuffle &&
  !state.repeat && player.shuffle_round.ends_round(state.current_track.as_deref())`, drop the
  state lock and `return stop(player)`, in the same way as the `stop_after_current` branch. Extend
  the function's doc comment to cover this. Leave `stepping::forward_track_id` and `step_track`
  untouched, so Next still starts a new round.

**Files touched.** `src/ui/transport/round.rs` (with its in-file tests),
`src/ui/handlers/playback/preview.rs`, `src/ui/tests/stepping_tests.rs`, `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**
- Unit tests in `round.rs` over `stepping_songs()`:
  - `the_play_completing_a_round_ends_it`: after recording song-1, song-2, and song-3,
    `ends_round(Some("song-3"))` is true, and `ends_round(Some("song-1"))` and
    `ends_round(None)` are false.
  - `a_later_play_clears_the_round_end`: recording song-3 again after that makes
    `ends_round(Some("song-3"))` false. Before the round completes (only song-1 and song-2
    recorded), `ends_round(Some("song-2"))` is false.
- UI tests in `src/ui/tests/stepping_tests.rs`, next to the Shuffle preview-end tests, using
  `player_playing_stepping_song`, `record_a_play`, and `drive_playback_finished` as
  `preview_end_with_shuffle_on_a_single_song_stops` does:
  - `preview_end_with_shuffle_on_and_repeat_off_stops_after_the_round`: song-3 is playing with
    Shuffle on and Repeat off, and song-1, song-2, and song-3 have been recorded. The finished
    reply takes the Stop path (`Message::TransportSettled`, and `recording.calls()` is
    `[AudioCall::Stop]`), `is_playing` is false, and `current_track` stays `song-3`.
  - `preview_end_with_shuffle_and_repeat_on_starts_a_new_round`: the same setup with Repeat on
    advances instead of stopping. The landing is random, so check that `drive_playback_finished`
    returns `Message::TrackSelected` with `index` 0 or 1 (song-1 or song-2), not the Stop path.
  - `next_with_shuffle_after_a_completed_round_keeps_playing`: with the same recorded plays and
    Repeat off, `Message::NextTrack` selects a song other than song-3, with no Stop.
- The existing Shuffle tests in `round.rs` and `stepping_tests.rs` pass unchanged, including
  `preview_end_with_shuffle_on_picks_the_other_song`,
  `preview_end_with_shuffle_on_plays_the_unplayed_song`, and
  `preview_end_with_shuffle_on_a_single_song_stops`.
- `docs/usage.md`, in the preview-end advance paragraph: "or, with Shuffle on, to a shuffled song
  when another exists" now says that with Shuffle on and Repeat off it stops once every song of
  the round has played, like Winamp, keeping that song queued for Play, and that Repeat on starts
  a new round. In `docs/invariants.md` `## Shuffle rounds`, add the stop to the guarantee, name
  `ShuffleRound::ends_round` and `playback_finished` under Enforced by, and list the new tests
  under Pinned by. `make check` passes.

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
