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
> (both 2026-10-20); the 2026-10-22 note below carries the current queue.

> **Steward drift note (2026-10-22 by the steward role): the built Now
> Playing bar and browse rows do not match the brief's mockups, and the two
> queued plans settle that difference in opposite directions — the
> playback-position plan adds a readout the mockup shows nowhere, while the
> album-row plan matches the mockup's `<artist> - <title>` rows but leaves the
> running time it draws out of scope.**
>
> The brief promises a "very close if not pixel-perfect" Winamp, and the plan
> role has been citing `docs/screenshots/` as the spec — the landed plans for
> the scrolling title, the artist-prefixed rows, the `TOP` button and the list
> counts each name a mockup feature. The main window's mockup
> (`docs/screenshots/player.png`) draws the classic LCD: a vertical `O C I D V`
> strip at the left, a large elapsed-time readout (`00:53`) over a green
> visualizer, a song-title box reading `<ARTIST> - <TITLE> (<length>)`, and a
> `256 KBPS 44 KHZ` / `MONO STEREO` status row. The build renders one row
> instead: `now_playing_row` (`src/ui/views/now_playing.rs`) pushes a
> `Now Playing: ` caption, the artist and title, a fill, and the track length
> at the right edge of the LCD well, and `view` (`src/ui/view.rs`) stacks that
> bar above the transport row — no elapsed-time readout, no `O C I D V` strip,
> no status row.
>
> The first entry under `## Planned`, "Add the playback-position readout to
> the Now Playing bar (Playback position, part 3/3)", widens that right-edge
> slot to `<elapsed>/<total>`, a value the mockup shows nowhere: its title
> line carries the length in parentheses and the elapsed time in the big left
> readout. Recommend the plan role schedule that elapsed-time display and
> settle where part 3/3's readout belongs before the queue drains.
>
> The browse rows drift the same way: `docs/screenshots/library.png` draws
> each artist row with its album count at the right (`2 albums`) and each
> album row with its running time (`12:38`), while the build shows the
> `View Albums` and `View Songs` hints (`src/ui/views/browse.rs`) and the
> total count beneath the rows. The queued album-row plan, "Show each album
> row's artist beside its title", puts album running times out of scope
> because an album query does not supply its songs' durations, so the
> mockup's per-album time would need a songs fetch per row — a decision for
> the plan role, not a detail to drop.

## Planned

### Clear the stopped Now Playing LCD readout (planned 2026-10-23 by the plan role)

**Goal.** Match `docs/screenshots/transport.png`'s stopped state: "Time, bitrate and spectrum clear and the seek thumb hides; the current song stays queued for Play." The status row, spectrum, and seek strip already clear through `reset_position_readout`, but the large elapsed readout still draws `00:00` in green because `elapsed_readout_color` returns `theme::LCD_GREEN` for `TransportMode::Stopped`. Draw the stopped readout transparent, keeping its text and bounds so the title and total do not move, and make it non-interactive while stopped so a click cannot silently switch the persisted time mode. This is independent of the queued Preview end plans: it reads the `TransportMode` that already exists and changes only the Now Playing view.

**Approach.**

- `src/ui/views/now_playing.rs`: extend `elapsed_readout_color` so `TransportMode::Stopped` is fully transparent in both blink phases as well as in `TransportMode::Paused` with `visible == false`; `TransportMode::Playing`, and `TransportMode::Paused` with `visible == true`, stay `theme::LCD_GREEN`. Update its doc comment to say the stopped readout is cleared rather than shown, and that drawing transparent keeps its bounds so the title, total, and status row stay in place.
- In the same module, add `pub(super) fn elapsed_readout_clickable(mode: TransportMode) -> bool`, returning false only for `TransportMode::Stopped` and true for `Playing`/`Paused`. In `now_playing_row`, build the elapsed `MouseArea` from the existing `lcd_elapsed_text(elapsed).color(elapsed_readout_color(...))` but attach `.on_press(Message::ToggleTimeDisplay).interaction(Interaction::Pointer)` only when `elapsed_readout_clickable(mode)` is true; otherwise use `.interaction(Interaction::None)` with no message. Keep the widget in the row so its layout slot and the text's bounds are unchanged.
- `src/ui/views/tests.rs`: extend and rename `elapsed_readout_color_blinks_only_while_paused` to `elapsed_readout_color_blinks_when_paused_and_clears_when_stopped`, asserting transparent for `(Paused, false)` and both `(Stopped, false)` and `(Stopped, true)`, and `theme::LCD_GREEN` for `(Playing, false)`, `(Playing, true)`, and `(Paused, true)`. Add `elapsed_readout_is_clickable_only_while_not_stopped` asserting true for Playing/Paused and false for Stopped. Update the comment on `now_playing_row_keeps_the_elapsed_readout_in_place_across_transport_modes` to say the stopped case still lays out the transparent text; its existing equal-x assertion then pins that stopping does not move the readout.
- `src/ui/tests.rs`: correct the comments and assertion messages in `position_tick_blinks_the_elapsed_readout_only_while_paused` that currently claim a stopped tick "shows the readout": the tick still holds `blink_visible` true while stopped, but the view hides the stopped readout regardless of that flag. Do not change the assertions.
- `README.md`: in the Usage paragraph, say the large readout is shown while playing or paused and blank (though it keeps its space) while stopped, and change "it stays steady while playing or stopped" to "it stays steady while playing and clears while stopped". Also scope the click sentence to while the readout is shown. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, update the time-readout bullet to state that while the transport is stopped the large readout draws transparent in both time modes and both blink phases, keeping its bounds and clickability off, so stopped shows no elapsed or remaining time; the stored value is still the `00:00` Stop clears it to, and Play/Pause draw it again. Update the transport-glyph bullet's "playing and stopped always show the readout" sentence to playing only, name `views::now_playing::elapsed_readout_color` and `elapsed_readout_clickable` as the enforcers, and add the new test names. Update the position-poll bullet's reason for leaving a stopped player out of the subscription to say the view clears its readout while stopped rather than that it already shows `00:00`.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `elapsed_readout_color` returns the fully transparent color for `TransportMode::Stopped` in both blink phases, and for `TransportMode::Paused` only when `visible` is false; playing and visible paused stay `theme::LCD_GREEN`.
- While stopped, `now_playing_row` still draws the elapsed text at the same bounds as in the playing and paused states, so the title, total, and status row do not move; the captured stopped row keeps the same elapsed fragment as the other modes.
- `elapsed_readout_clickable` is false only for `Stopped`, and the stopped readout attaches no `ToggleTimeDisplay` message, so clicking it cannot change the persisted `TimeDisplay`; playing and paused readouts still toggle it.
- Stop, a new selection, Pause, the position poll, the marquee, the status row, the spectrum, and the seek strip keep their current behavior; no state field, timer, subscription, or dependency is added, and the shade strip is deliberately unchanged.
- `README.md` and `docs/invariants.md` describe the stopped readout, and `make check` passes.

### Match the equalizer panel's preamp column and dB scale to the mockup (planned 2026-10-23 by the plan role)

**Goal.** Match `docs/screenshots/hero.png` and `docs/screenshots/player.png`, which draw the equalizer's preamp as a vertical slider in the same row as the ten band sliders, with a `+12 DB` / `0 DB` / `-12 DB` scale beside it. The build (`view_equalizer` in `src/ui/views/equalizer.rs`) instead lays the preamp out as a horizontal 150 px slider in its own row above the response curve, and draws no gain scale, so the panel's slider positions and green/yellow/red band tint cannot be read as decibels. Move the preamp into the band row as a `VerticalSlider` and add the three scale labels; no gain, message, preset, curve, tint, or audio behavior changes.

**Approach.**

- `src/ui/views/equalizer.rs`: add `pub(super) const EQ_SLIDER_HEIGHT: f32 = 100.0;` and use it for the band sliders' `.height(...)` (replacing the literal `100.0`), the new preamp slider, and the scale column, so the three stay aligned.
- Add `pub(super) fn db_scale_column() -> Column<'static, Message>`: a width-shrink `Column` at `Length::Fixed(EQ_SLIDER_HEIGHT)` holding `Text::new("+12 DB").size(12)`, a `Space` of `Length::Fill`, `Text::new("0 DB").size(12)`, another `Length::Fill` `Space`, and `Text::new("-12 DB").size(12)`. The two equal fill spaces center the middle label on the rail, so the endpoints read at the rail's top and bottom.
- Add `pub(super) fn preamp_column(preamp: f32) -> Column<'static, Message>`: a `VerticalSlider::new(GAIN_MIN_DB..=GAIN_MAX_DB, preamp, Message::EqPreampChange).step(EQ_STEP).height(Length::Fixed(EQ_SLIDER_HEIGHT)).style(|_theme, status| style::chrome_slider_style(status))` over `Text::new("Preamp").size(12)`. Keep `chrome_slider_style`, not `band_slider_style`, so only the ten band grooves carry the gain tint.
- In `view_equalizer`, delete the `Row::new().push(Text::new("Preamp")).push(chrome_slider(...))` row. Build `band_row` starting with `preamp_column(preamp)` then `db_scale_column()` then the existing per-band columns (whose sliders use `EQ_SLIDER_HEIGHT`), all separated by the existing `BAND_ROW_SPACING`. Leave the header row (on/off button and preset `PickList`) and the `response_curve_row(bands)` position above `band_row` unchanged.
- Update the module and `view_equalizer` docs to describe the new order: on/off button and preset pick list, response curve, then one row of vertical preamp, dB scale, and ten band sliders. In `src/ui/views/mod.rs`, update `chrome_slider`'s doc to drop the equalizer preamp from its horizontal callers and name the vertical preamp beside the band sliders as the `VerticalSlider` twin.
- `src/ui/views/tests.rs`: import `EQ_SLIDER_HEIGHT` alongside the existing equalizer items and add `equalizer_preamp_and_db_scale_sit_left_of_the_band_sliders`. Lay out `view_equalizer(false, 0.0, &[0.0; BAND_COUNT], None)` with `layout_capture`, then assert all three labels are drawn; assert `capture.find("+12 DB").y < capture.find("0 DB").y < capture.find("-12 DB").y` (the scale reads top to bottom); assert the `Preamp` label's `y` is within 0.5 px of the first `BAND_FREQUENCIES` label's `y` (both sit beneath a slider in the same row, so the preamp is vertical there rather than in its old separate row); and assert every scale label's right edge is at or left of the first frequency label's left edge (the scale sits before the bands). Leave `equalizer_band_labels_leave_a_gap_between_frequencies` and `equalizer_panel_constructs_for_both_states_and_gain_endpoints` unchanged.
- `README.md`: in the Usage sentence listing the equalizer panel, replace "a preamp slider, a response curve above ten band sliders" with "a response curve above a vertical preamp slider, a `+12 dB` / `0 dB` / `-12 dB` scale, and ten band sliders". Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Equalizer band tint`, change "The preamp slider keeps its uniform gray groove" to say the vertical preamp slider keeps its uniform gray groove. In `## UI layout`, add a bullet that the equalizer band row puts the vertical preamp leftmost, the three dB labels next, then the ten band sliders, with `+12 DB` above `0 DB` above `-12 DB` beside the rail's top, middle, and bottom; name `views::equalizer::preamp_column`, `views::equalizer::db_scale_column`, and `views::view_equalizer` as the enforcers and the new layout test as pinned by.

**Files touched.** `src/ui/views/equalizer.rs`, `src/ui/views/mod.rs` (doc comment only), `src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The equalizer panel draws the preamp as a vertical slider in the same row as the band sliders, left of the `+12 DB` / `0 DB` / `-12 DB` scale, which is left of the ten band sliders; the response curve, on/off button, and preset pick list keep their positions and behavior.
- The three scale labels are drawn top to bottom as `+12 DB`, `0 DB`, `-12 DB`, and the `Preamp` and first band-frequency labels share one text-baseline row, proving the preamp moved into the band row.
- Dragging the preamp still emits `Message::EqPreampChange`, still spans `GAIN_MIN_DB..=GAIN_MAX_DB` in `EQ_STEP` increments, and its groove stays the uniform gray `chrome_slider_style`; the band sliders keep their gain tint and `band_slider_style`.
- No gain, preset, curve, audio, or state behavior changes: `eq_preamp_change_clamps_value_before_storing`, `equalizer_messages_forward_to_the_audio_backend`, `response_curve_offsets_follow_the_band_gains`, and the band-tint tests pass unchanged.
- `README.md` and `docs/invariants.md` describe the new preamp column and dB scale, and `make check` passes.

## Done
