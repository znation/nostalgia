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

### Show the loaded preview's format in the File info panel, like Winamp (planned 2026-10-26 by the plan role)

**Goal.** Winamp's File info completes the current file's details with its format. Nostalgia's
panel names the title, artist, album, and length, and its LCD status row already shows the loaded
preview's average bitrate, sample rate, and channel mode. Add the same values to the File info
panel as one `Format: …` line when `WinampPlayer::playback_format` is `Some`, reusing
`views::now_playing::format_playback_prefix` and `views::now_playing::channel_mode_label`. No new
message, state, handler, or persistence behavior.

**Approach.**

- `src/ui/views/file_info.rs`:
  - Import `crate::audio::PlaybackFormat` and
    `super::now_playing::{channel_mode_label, format_playback_prefix}`; add
    `const FORMAT_LABEL: &str = "Format: ";` beside the existing labels.
  - Change `view_file_info_overlay` to take `format: Option<PlaybackFormat>` after `track`. After
    the length line and before the `Close` button, push
    `Text::new(format!("{FORMAT_LABEL}{} {}", format_playback_prefix(format),
    channel_mode_label(format.channels)))` only when `format` is `Some`. With `None` the panel
    draws no format line. The title line, the empty-artist and empty-album omissions, the length,
    and the `Close`/backdrop behavior stay as they are.
  - Update the module and function docs to list the format line and say it appears only while the
    loaded preview reports a format.
- `src/ui/view.rs`: in the File info overlay branch, pass `player.playback_format` as the third
  argument. This is the same value the LCD status row draws; it stays through Pause and is cleared
  by Stop or a new selection in `handlers::playback::reset_position_readout`, so the panel keeps
  its format line while paused and drops it once that preview is gone.
- `src/ui/views/tests/chrome.rs`: make the `file_info_element` helper take
  `Option<PlaybackFormat>` and pass `None` from the existing close/backdrop test. Leave
  `file_info_overlay_draws_the_title_artist_album_and_length` and
  `file_info_overlay_omits_the_album_line_when_the_source_supplied_none` passing `None`; add
  `file_info_overlay_draws_the_loaded_previews_format`, asserting
  `Format: 256 KBPS  44 KHZ STEREO` for
  `PlaybackFormat { sample_rate_hz: 44_100, channels: 2, bitrate_kbps: Some(256) }` and
  `Format: 44 KHZ MONO` for a format with `bitrate_kbps: None, channels: 1`, and add
  `file_info_overlay_omits_the_format_line_without_a_loaded_preview`, asserting no captured text
  starts with `"Format:"` when the argument is `None`.
- `src/ui/tests/view_tests.rs`: add a root-view test that seeds a `known_tracks` entry and
  `current_track` (as the shaded File info check already does), sets `file_info_visible` and a
  `PlaybackFormat`, and captures the root view; it must contain the `Format: …` line, and clearing
  `playback_format` must remove it. This pins the `ui::view::view` wiring, not just the pure view.
- `docs/invariants.md` `## File info panel`: add the `Format: …` line (only while
  `WinampPlayer::playback_format` is `Some`, formatted from
  `views::now_playing::format_playback_prefix` and
  `views::now_playing::channel_mode_label`) to the Guarantee, name those functions in
  **Enforced by**, and add the new chrome and root-view test names to **Pinned by**.
- `README.md` (Usage, outside the brief markers): extend the Alt+3 sentence to say the panel also
  shows the loaded preview's format (`256 KBPS  44 KHZ STEREO`) while one is reported. Do not touch
  the opening `O A I D V` sentence or the `I` route, which the waiting strip plan owns.

**Files touched.** `src/ui/views/file_info.rs`, `src/ui/view.rs`,
`src/ui/views/tests/chrome.rs`, `src/ui/tests/view_tests.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**

- `view_file_info_overlay` with `Some(format)` draws one `Format:` line after `Length:` and before
  `Close`, using the LCD status row's existing `"256 KBPS  44 KHZ"`/`"44 KHZ"` prefix plus
  `MONO`/`STEREO`; with `None` it draws no `Format:` line, and every existing line and the
  `Close`/backdrop presses are unchanged.
- The root `view` passes `player.playback_format` to the panel, so a File info panel open while a
  preview is loaded shows its format, and after Stop (which clears that value) the panel keeps its
  title/artist/album/length but drops the format line.
- No playback, browse, selection, search, settings, or persistence behavior changes.
- `make check` passes, including `cargo doc` with no broken intra-doc link.

## Done
