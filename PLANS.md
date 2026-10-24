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

### Show "Seek to: MM:SS/MM:SS (NN%)" on the LCD while dragging the seek slider (planned 2026-10-24 by the plan role)

**Goal.** In classic Winamp, dragging the position bar swaps the song title for the drag target,
e.g. `Seek to: 01:23/03:30 (39%)`, until you let go. Nostalgia already does this for volume,
balance, and equalizer drags through `views::mixer_readout_text`. A seek drag currently moves only
the large elapsed readout (`WinampPlayer::readout_position`). Show the seek target text in the
same LCD title slot while `WinampPlayer::seek_preview` is held, and bring the title back on release.
Independent of the EQ/library panel persistence plan: this one touches no settings code.

**Approach.**
- `src/ui/views/now_playing/mixer_readout.rs`: add a pure
  `pub fn seek_readout_text(target: Duration, length: Duration) -> String` that returns
  `format!("Seek to: {}/{} ({pct}%)", format_lcd_elapsed(target_ms), format_lcd_elapsed(length_ms))`,
  using `super::text::format_lcd_elapsed` (zero-padded `MM:SS`, the same form as the large readout).
  `pct` is `target_ms * 100 / length_ms` (integer floor), clamped to 100, and is `0` when
  `length` is zero. Clamp `target` to `length` before formatting, so the target never reads past the
  length. Widen the module doc so it covers the seek slider too. Re-export the function next to
  `mixer_readout_text` in `src/ui/views/now_playing.rs` and `src/ui/views/mod.rs`.
- `src/ui/view.rs`: the `view_now_playing` call currently passes
  `player.mixer_drag.map(|drag| views::mixer_readout_text(..))`. Chain
  `.or_else(|| player.seek_preview.zip(player.playback_duration).map(|(t, l)| views::seek_readout_text(t, l)))`
  onto it, so a mixer drag wins if both were ever set. Keep using the existing `mixer_readout`
  parameter of `view_now_playing` / `now_playing_row`; add no new parameter. The scale is
  `playback_duration` (the preview's own length, the same scale the seek slider uses), not the
  catalog length at the LCD's right edge.
- `src/ui/player.rs` `WinampPlayer::needs_marquee`: return `false` while `seek_preview.is_some()`
  too, matching how it already handles `mixer_drag.is_some()`.
- Do not add a seek drag to `timers::subscriptions`' `mixer_readout_showing` argument, and do not
  change `seek_preview_changed` / `seek_preview_released`. Release and Stop/new selection already
  clear `seek_preview` (`seek_preview_released_seeks_once_to_the_dragged_position`,
  `stop_and_track_selection_clear_the_playback_duration`).
- The shade strip has no title slot. Leave it unchanged.

**Files touched.** `src/ui/views/now_playing/mixer_readout.rs` (function + unit test),
`src/ui/views/now_playing.rs` and `src/ui/views/mod.rs` (re-exports), `src/ui/view.rs`,
`src/ui/player.rs` (`needs_marquee`), `src/ui/tests.rs`, `docs/invariants.md`
(`## Mixer drag readout`), README.md Usage (outside the brief markers).

**Acceptance criteria.**
- Unit test `seek_readout_shows_target_length_and_floored_percent` in `mixer_readout.rs`:
  (12.5 s, 30 s) reads `Seek to: 00:12/00:30 (41%)`; (0, 30 s) reads `Seek to: 00:00/00:30 (0%)`;
  (30 s, 30 s) reads `(100%)`; a target past the length reads the length and `100%`; a zero
  length reads `Seek to: 00:00/00:00 (0%)`.
- `src/ui/tests.rs` test `dragging_the_seek_slider_shows_the_target_until_release`: use
  `seed_current_title` with title `Tune` and set `playback_duration = Some(30 s)`. After
  `Message::SeekPreviewChanged(12.5)`, `root_view_draws(&player, "Seek to: 00:12/00:30 (41%)")`
  holds and `root_view_draws(&player, "Tune")` does not. After `Message::SeekPreviewReleased`, the
  title draws again and the seek text does not.
- A test (or an extension of `the_marquee_pauses_during_a_mixer_drag`) shows `needs_marquee()` is
  false while a seek drag is held and true again after release, for a title too long for the LCD.
- With no `playback_duration`, a stored `seek_preview` draws no seek text.
- `now_playing_elapsed_previews_the_seek_drag` and the existing mixer readout tests still pass
  unchanged.
- `docs/invariants.md` `## Mixer drag readout` names the seek text, `views::seek_readout_text`, and
  the new tests. The README sentence listing the drag readouts adds the seek example.
  `make check` passes.

## Done
