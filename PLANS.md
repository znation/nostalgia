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

### Draw the equalizer's response curve above the band sliders (planned 2026-10-23 by the plan role)

**Goal.** Close a visible gap in `docs/screenshots/equalizer.png`'s active panel: the mockup draws a green response line above the ten band sliders (flat for the Flat preset, a traced curve for Rock), while `view_equalizer` (`src/ui/views/equalizer.rs`) renders only the header, preamp slider, and band sliders. Draw the line from the stored band gains without adding a dependency, using a column approximation instead of `iced`'s `canvas` feature (which would pull `lyon_path`).

**Approach.**

- In `src/ui/views/equalizer.rs`, add the plot constants `pub(super) const RESPONSE_CURVE_COLUMNS: usize = 64`, `pub(super) const RESPONSE_CURVE_HEIGHT: f32 = 24.0`, and `RESPONSE_CURVE_LINE_HEIGHT: f32 = 2.0`, sized so the extra row fits the panel without changing the slider rows.
- Add the pure `pub(super) fn response_curve_offsets(bands: &[f32; BAND_COUNT], columns: usize) -> Vec<f32>`: returns exactly `columns` offsets, each a height from the plot's bottom (`0.0` at `GAIN_MIN_DB`, `1.0` at `GAIN_MAX_DB`). Column 0 sits on the first band and the last column on the last band; each intermediate column linearly interpolates between the two adjacent band gains, so the line follows the same gains the sliders and `BandFilter` use. Run each endpoint gain through `clamp_gain` before normalizing, so a NaN or an out-of-range value cannot leave the plot. `columns == 0` returns an empty vector; `columns == 1` returns the first band's offset rather than dividing by zero.
- Add `fn response_curve_row(bands: &[f32; BAND_COUNT]) -> Element<'static, Message>`: a `Container` filling the width at `RESPONSE_CURVE_HEIGHT`, with `theme::LCD_BACKGROUND`, holding a `Row` of `RESPONSE_CURVE_COLUMNS` equal-width columns. Each column is a `Column` of a top `Space` of `(1.0 - offset) * (RESPONSE_CURVE_HEIGHT - RESPONSE_CURVE_LINE_HEIGHT)`, a `RESPONSE_CURVE_LINE_HEIGHT` bar in `theme::LCD_GREEN`, and a bottom `Space` of the remainder; adjacent bars touch so the dots read as one line. The preamp is deliberately not plotted: it is a uniform gain offset, not a bend in the curve.
- Push `response_curve_row(bands)` into `view_equalizer` between the preamp row and `band_row`, so the plot spans the ten sliders and reads at the same place as the mockup. It draws whether the EQ is on or off, tracing the stored gains in both states.
- `src/ui/views/tests.rs`: import the new helpers and add `response_curve_offsets_follow_the_band_gains` (a flat `0.0` curve is `0.5` at every column; all-`GAIN_MIN_DB` is `0.0`; all-`GAIN_MAX_DB` is `1.0`; the first and last offsets equal the first and last gain's normalized values; a two-band `-12.0, 12.0` curve with three columns yields `0.0, 0.5, 1.0`; `0` columns yields none and `1` column does not panic). Add `response_curve_row_keeps_its_height_and_draws_one_segment_per_column`, laying the row out with the null renderer (the `now_playing_visualizer_row_keeps_its_height_without_a_spectrum` pattern), asserting the root height is `RESPONSE_CURVE_HEIGHT` and `CapturedContainers` holds `RESPONSE_CURVE_COLUMNS` containers at `RESPONSE_CURVE_LINE_HEIGHT` for a shaped curve including zero-offset columns.
- `README.md`: add the response curve to the Usage sentence that lists the equalizer panel's widgets (`EQ on/off button, preset pick list, preamp slider, and a response curve above ten band sliders`).

**Files touched.** `src/ui/views/equalizer.rs`, `src/ui/views/tests.rs`, `README.md`.

**Acceptance criteria.**

- `response_curve_offsets` returns exactly the requested column count, `0.0`/`0.5`/`1.0` for all-min, flat, and all-max curves, endpoints equal to the first and last band gains' normalized positions, and linear interpolation between adjacent bands (`-12.0, 12.0` across three columns is `0.0, 0.5, 1.0`).
- The equalizer panel draws the curve row between the preamp and band rows at the fixed `RESPONSE_CURVE_HEIGHT`, with one line segment per `RESPONSE_CURVE_COLUMNS`; the existing `equalizer_panel_constructs_for_both_states_and_gain_endpoints` and `equalizer_band_labels_leave_a_gap_between_frequencies` still pass.
- The mockup's flat and Rock shapes are both traced: a flat stored curve draws a centered line, and a curved preset draws a line whose high columns sit above its low columns.
- `README.md` lists the curve among the equalizer panel's widgets.
- `make check` passes (fmt, clippy, rustdoc, tests), and the change adds no dependency or `iced` feature.

## Done
