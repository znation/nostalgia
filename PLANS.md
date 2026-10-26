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

### Open the File info panel from the LCD strip's `I`, like Winamp (planned 2026-10-26 by the plan role)

**Goal.** Winamp's main-window indicator strip has a live `I` cell that opens File info for the
current track. Nostalgia draws the same `O A I D V` strip and already has the File info panel via
Alt+3, but `lcd_indicator_message` leaves `I` inert along with `O` and `D`. Publish the existing
`Message::ToggleFileInfo` from the strip's `I` cell, so the current-track gate, panel closing,
About exclusivity, and shade restore all come from the existing handler. No new message, state,
shortcut, or persistence behavior.

**Approach.**

- `src/ui/views/now_playing.rs`:
  - In `lcd_indicator_message`, add `"I" => Some(Message::ToggleFileInfo)` beside the existing
    `"A"` and `"V"` arms, leaving `O` and `D` to the `_ => None` arm.
  - Update the doc comments on `LCD_INDICATOR_LABELS`, `lcd_indicator_message`, and
    `lcd_indicator_strip`: `A`, `I`, and `V` are live controls, `O` and `D` stay decorative, and
    `I` opens the same File info panel as Alt+3.
- `src/ui/views/tests/now_playing.rs`:
  - Rename `lcd_indicator_message_arms_a_and_v_and_leaves_o_i_d_inert` to
    `lcd_indicator_message_arms_a_i_and_v_and_leaves_o_d_inert`. Assert `A` maps to
    `Message::ToggleAlwaysOnTop`, `V` to `Message::ToggleVisualizer`, and `I` to
    `Message::ToggleFileInfo`; keep `O` and `D` inert in the loop.
  - Rename `lcd_indicator_strip_publishes_always_on_top_on_a_and_visualizer_on_v` to
    `lcd_indicator_strip_publishes_always_on_top_on_a_visualizer_on_v_and_file_info_on_i`.
    Drive a real left press at `A`, `I`, `V`, and `O` as it does now: `I` must publish
    `Message::ToggleFileInfo`, `O` must publish nothing, and the `A`/`V` assertions stay
    unchanged.
  - Leave `lcd_indicator_strip_draws_each_label_in_order_at_a_fixed_width` unchanged; the five
    letters and the fixed width do not change.
- `src/ui/handlers/window.rs`: the `toggle_file_info` doc comment starts "Alt+3: flip …"; say
  Alt+3 or the LCD strip's `I`, and leave the handler body alone.
- `src/ui/views/file_info.rs`: the module doc says the panel names the track without naming a
  route, so no change is needed; this plan adds no album or format line.
- `docs/invariants.md`:
  - In `## Now Playing label`, change the indicator-strip bullet and the `Enforced by` sentence
    that say `A` and `V` are live while `O`, `I`, and `D` are decorative: `A`, `I`, and `V`
    publish messages through `lcd_indicator_message`, `I` publishes `Message::ToggleFileInfo`,
    and only `O` and `D` stay decorative.
  - Replace the old test names in that section's `Pinned by` list with the two renamed tests.
  - In `## File info panel`, add a bullet to the Guarantee: pressing the LCD strip's `I`
    publishes the same `Message::ToggleFileInfo` as Alt+3 and goes through the same
    `handlers::window::toggle_file_info` gate, so it does not open without a known current
    track. Add `views::now_playing::lcd_indicator_message` to `Enforced by` and the renamed
    strip-press test to `Pinned by`.
- `README.md` (Usage, outside the brief markers): in the opening sentence about the
  `O A I D V` strip, say `A` toggles always-on-top, `I` opens the File info panel, and `V` hides
  and restores the visualizer, while only `O` and `D` stay decorative. Leave the later Alt+3
  sentence for the waiting album plan, which already edits it.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests/now_playing.rs`,
`src/ui/handlers/window.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**

- `lcd_indicator_message("I")` returns `Some(Message::ToggleFileInfo)`; `"A"` and `"V"` keep
  their existing messages, and `"O"` and `"D"` still return `None`.
- A left press on the laid-out strip's `I` publishes `Message::ToggleFileInfo`; the `A` and `V`
  presses publish their existing messages, and `O` publishes nothing.
- The panel still opens only while `current_known_track()` resolves: the existing
  `toggle_file_info_needs_a_known_current_track`, About-exclusivity, shade-restore, and
  overlay tests pass unchanged.
- No other strip letter, LCD label, playback, settings, or persistence behavior changes.
- `make check` passes, including `cargo doc` with no broken intra-doc link.

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
