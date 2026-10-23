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
