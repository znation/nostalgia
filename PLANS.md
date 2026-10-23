# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; updated 2026-10-20).** The initial prompt
> names two commitments — a "very close if not pixel-perfect" classic Winamp UI
> and Apple Music as the library. The first is now moving: the base-skin
> palette landed ("Add a Winamp 2.x base-skin palette and apply it as the app
> theme", done 2026-10-06), and the reusable bevel layer followed ("Add a
> Winamp two-tone bevel layer and frame the Now Playing and equalizer panels",
> done 2026-10-06), framing those two panels through `bevel::lcd_well` and
> `bevel::raised_panel` in `src/ui/bevel.rs`. The transport-button chrome plan
> landed ("Style the transport buttons as raised Winamp chrome", done
> 2026-10-07), and both transport toggles are reachable: the Repeat toggle
> landed 2026-10-06 and the Shuffle toggle 2026-10-08, and their R and S
> keyboard bindings followed ("Add the R and S keyboard bindings for Repeat
> and Shuffle", done 2026-10-20). The rest of widget-level fidelity has
> landed too: slider chrome ("Style the volume and
> equalizer sliders as sunken Winamp grooves with raised chrome thumbs", done
> 2026-10-07), a custom title bar ("Add a Winamp custom title bar and drop the
> OS window frame", done 2026-10-07), and playlist chrome ("Frame the browse
> list as a sunken Winamp playlist editor", done 2026-10-07). The library side
> is real:
> `AppleMusicService` answers its browse and search queries through the REST
> client once a `MusicKit` session is stored, falling back to
> `sample_library()` otherwise — playlist queries included — and `play_track`
> plays the selected preview through the audio seam. Risk: the Winamp chrome
> arrived piecemeal, so later changes still need one consistent vocabulary.
> The equalizer-audio and seek plans this note pointed at have since landed
> (both 2026-10-20); the 2026-10-22 note below records drift that has since
> been resolved.

> **Steward drift note (2026-10-22 by the steward role; resolved 2026-10-23
> by the steward role).** That note recorded the built Now Playing bar and
> browse rows drifting from the brief's mockups — no elapsed readout, no
> `O A I D V` strip, no status row, album rows without running time, and a
> queued playback-position plan that put an `<elapsed>/<total>` readout at the
> LCD's right edge. Those drifts have since landed: "Add the classic elapsed
> readout to the Now Playing LCD and restore the plain total at its right
> edge", "Draw the classic O A I D V strip at the LCD's left edge", "Show the
> loaded preview's bitrate in the LCD status row", "Show each album row's
> running time", and "Show each artist row's album count" are done, and the
> playback-position plan is no longer queued. No current brief-vs-build drift
> is known; the only planned entry is the LCD `MONO`/`STEREO` pair below,
> which brings the status row to `docs/screenshots/player.png` and
> `docs/screenshots/transport.png`.

## Planned

### Draw the LCD's MONO and STEREO indicators as a lit pair (planned 2026-10-23 by the plan role)

**Goal.** Match `docs/screenshots/player.png` and `docs/screenshots/transport.png`, which draw the channel indicators as a pair of labels, `MONO` and `STEREO`, with the mode the loaded preview reports lit green and the other dimmed. Today `now_playing_status_row` (`src/ui/views/now_playing.rs`) draws one right-aligned green string ending in the active mode only (`256 KBPS  44 KHZ  STEREO`), and `format_playback_status` (`src/ui/views/now_playing/text.rs`) builds that whole string. Split the status row so the bitrate/sample-rate prefix keeps its place and the two channel labels draw beside it; when no format is reported, both labels stay dim and the prefix is absent. This is presentation only: the format value, its Pause/Stop clearing rules, and the status row's fixed height are unchanged.

**Approach.**

- `src/ui/theme.rs`: add `pub const LCD_GREEN_DIM: Color = Color::from_rgb(0.0, 0.35, 0.0);`, documented as the unlit LCD indicator color. Extend the existing palette/color tests with its exact value.
- `src/ui/views/now_playing/text.rs`: replace `format_playback_status` with two pure helpers. `pub fn format_playback_prefix(format: PlaybackFormat) -> String` returns `"{kbps} KBPS  {khz} KHZ"`, or `"{khz} KHZ"` when the bitrate is unknown, still truncating the rate to whole kilohertz. `pub fn channel_mode_label(channels: u16) -> &'static str` returns `"MONO"` for one channel and `"STEREO"` otherwise. Update the module docs to say the row composes a prefix and a label pair.
- `src/ui/views/now_playing.rs`: import the new helpers and re-export them from the module. Add `pub(super) const CHANNEL_LABELS: [&str; 2] = ["MONO", "STEREO"];` and `pub(super) fn channel_label_color(channels: Option<u16>, label: &str) -> Color`, returning `theme::LCD_GREEN` only when `channels` is `Some` and `label == channel_mode_label(channels)`, and `theme::LCD_GREEN_DIM` otherwise. In `now_playing_status_row`, keep the fill spacer and fixed `LCD_STATUS_HEIGHT`; when `format` is `Some`, push `format_playback_prefix(format)` in `theme::LCD_GREEN` then a small gap; then always push both labels at `LCD_STATUS_SIZE`, each colored by `channel_label_color(format.map(|format| format.channels), label)`. The row therefore draws the dim pair even with no format instead of nothing, matching the mockup, while the height stays fixed.
- Tests. `src/ui/views/tests.rs`: replace `format_playback_status_formats_khz_and_channel_mode` with `format_playback_prefix_formats_khz_and_bitrate` (256/44.1 kHz → `"256 KBPS  44 KHZ"`, unknown bitrate → `"44 KHZ"`, 48 kHz → `"48 KHZ"`) and `channel_mode_label_names_the_reported_channels` (1 → `MONO`, 2 → `STEREO`). Add `channel_label_color_lights_only_the_reported_mode` asserting stereo lights `STEREO` and dims `MONO`, mono the reverse, and `None` dims both, with exact `LCD_GREEN`/`LCD_GREEN_DIM` values. Update `captured_status_row`/`now_playing_status_row_draws_the_format_or_nothing` to `now_playing_status_row_lights_the_reported_channel_mode`: a stereo format draws the prefix plus both labels, while `None` draws both labels and the same height. Update `now_playing_content_keeps_the_visualizer_height_with_and_without_a_spectrum` to locate the `"256 KBPS  44 KHZ"` fragment and assert both labels also draw below the visualizer row.
- `README.md`: in the Usage sentence describing the LCD status row, say it always draws `MONO` and `STEREO` side by side, lights the reported mode green and dims the other, and shows the bitrate/sample-rate prefix only when the backend reports a format. Keep the existing bitrate-fallback wording for that prefix.
- `docs/invariants.md`: in the `## Now Playing label` status-row bullet, replace the single-mode/blank-row wording with the pair guarantee: both labels always draw, the reported mode is green and the other dim, both are dim with no format, the prefix appears only with a reported format, and Pause/Stop/new-selection and stale-reply rules are unchanged. Name `theme::LCD_GREEN_DIM`, `views::now_playing::CHANNEL_LABELS`/`channel_label_color`, and `text::format_playback_prefix`/`channel_mode_label` as the enforcers and add the new tests to the Pinned-by list.

**Files touched.** `src/ui/theme.rs`, `src/ui/views/now_playing/text.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The LCD status row always draws both `MONO` and `STEREO` labels in the mockup's order and keeps its fixed height; with a reported format it draws the bitrate/sample-rate prefix before them.
- The reported channel mode is `theme::LCD_GREEN` and the other label is `theme::LCD_GREEN_DIM`; with no reported format both labels are dim and no prefix is drawn.
- `format_playback_prefix` still truncates 44.1 kHz to `44`, formats 48 kHz as `48`, omits an unknown bitrate, and leaves the existing bitrate value unchanged.
- Pause keeps the reported format lit, while Stop and a new selection clear it so both labels go dim; no state field, poll, message, or audio behavior changes.
- `README.md` and `docs/invariants.md` describe the pair, `make check` passes, and the new tests cover the prefix, the label mapping, the exact lit/dim colors, and the row's fragments and height.

## Done
