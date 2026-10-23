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

### Fill the volume and balance sliders with the mockup's orange and green rails (planned 2026-10-23 by the plan role)

**Goal.** Match `docs/screenshots/player.png` and `docs/screenshots/transport.png`, which draw the transport's volume slider over an orange groove and its balance slider over a green groove, each with the same chrome thumb. The build styles both through `chrome_slider` (`src/ui/views/mod.rs`) with `style::chrome_slider_style`, whose near-black `LCD_BACKGROUND` rail makes the two sliders look identical. Give the level sliders their own filled rail while leaving the equalizer preamp and the seek slider on the neutral chrome groove. This is presentation only: slider ranges, steps, messages, thumb geometry, and every playback behavior stay unchanged.

**Approach.**

- `src/ui/theme.rs`: add `pub const VOLUME_SLIDER_FILL: Color = Color::from_rgb(0.88, 0.49, 0.15);` and `pub const BALANCE_SLIDER_FILL: Color = Color::from_rgb(0.09, 0.57, 0.04);`, documented as the mockup's orange volume groove and green balance groove. Extend `base_skin_colors_keep_their_documented_literal_values` with both exact values and assert they differ from each other and from `LCD_BACKGROUND`.
- `src/ui/style.rs`: add `pub fn level_slider_style(fill: Color, status: slider::Status) -> slider::Style` beside `band_slider_style`. It starts from `chrome_slider_style(status)` and sets `rail.backgrounds` to `(Background::Color(fill), Background::Color(fill))`, so both rail halves carry the fill; the 4px rail, 1px dark border, square geometry, and chrome handle stay exactly `chrome_slider_style`'s, with only the handle face following hover/drag. Add `level_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb`, iterating `VOLUME_SLIDER_FILL`, `BALANCE_SLIDER_FILL`, and the Active/Hovered/Dragged statuses, asserting the exact rail backgrounds and equality of the rail width, rail border, and handle with `chrome_slider_style`.
- `src/ui/views/mod.rs`: give `chrome_slider` a `fill: Color` parameter after `width` and style the widget with `move |_theme, status| style::level_slider_style(fill, status)`. Update its doc comment to say it builds the filled level sliders (transport volume and balance, shade volume) rather than a neutral chrome groove.
- `src/ui/views/transport.rs`: pass `theme::VOLUME_SLIDER_FILL` to the volume slider and `theme::BALANCE_SLIDER_FILL` to the balance slider, importing `theme` beside `Message`.
- `src/ui/views/shade.rs`: in `shade_volume_slider`, pass `theme::VOLUME_SLIDER_FILL` to `chrome_slider`; keep `shade_volume_spec`'s range, width, and message tuple unchanged.
- Leave `src/ui/views/seek_bar.rs` and the equalizer preamp's direct `style::chrome_slider_style` calls on the neutral groove, so only the level sliders gain a fill.
- `README.md`: in the Usage sentence naming the volume and balance sliders, say the volume slider's groove is orange and the balance slider's is green, matching the mockup. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Level slider fill` section after `## Equalizer band tint`: the transport volume and shade volume rails are `theme::VOLUME_SLIDER_FILL`, the transport balance rail is `theme::BALANCE_SLIDER_FILL`, both rail halves carry the fill in every interaction status, the chrome thumb and rail geometry are unchanged, and the seek bar and equalizer preamp keep the neutral `LCD_BACKGROUND` groove. Name `style::level_slider_style` and `views::chrome_slider` as the enforcers and list the theme and style tests.

**Files touched.** `src/ui/theme.rs`, `src/ui/style.rs`, `src/ui/views/mod.rs`, `src/ui/views/transport.rs`, `src/ui/views/shade.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The transport volume slider draws both halves of its rail in `theme::VOLUME_SLIDER_FILL` (orange) and the balance slider in `theme::BALANCE_SLIDER_FILL` (green); the shade-mode volume slider uses the same orange fill.
- The chrome thumb, 4px rail width, 1px dark rail border, and square geometry are unchanged from `chrome_slider_style`, and hover/drag still change only the thumb face.
- The seek bar and the equalizer preamp keep the neutral `LCD_BACKGROUND` groove; no range, step, message, state, or playback behavior changes.
- `base_skin_colors_keep_their_documented_literal_values` and `level_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb` pin the exact fills and the style mapping, `make check` passes, and the README and invariants section describe the same behavior.

## Done
