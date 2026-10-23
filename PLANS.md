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

_None yet._

### Tint each equalizer band slider by its gain, green through yellow to red (planned 2026-10-23 by the plan role)

**Goal.** Close the remaining visible gap in `docs/screenshots/equalizer.png`'s panel: the mockup paints each band slider's whole groove according to that band's stored gain (green at a cut, yellow at `0 dB`, red at a boost), while `view_equalizer` (`src/ui/views/equalizer.rs`) dresses every band in the same gray `chrome_slider_style`. Paint the existing sliders from their gain, with no custom widget, image asset, or dependency.

**Approach.**

- `src/ui/theme.rs`: add three public gain-stop colors beside the existing palette constants: `EQ_GAIN_LOW` (equal to `LCD_GREEN`), `EQ_GAIN_MID` (a yellow, e.g. `Color::from_rgb(0.85, 0.85, 0.0)`), and `EQ_GAIN_HIGH` (a red-orange, e.g. `Color::from_rgb(1.0, 0.2, 0.1)`), each with a doc comment naming the gain it stands for.
- `src/ui/style.rs`: add `pub fn band_gain_color(gain: f32) -> Color`. Run `gain` through `crate::equalizer::clamp_gain` first, so an out-of-range or NaN value cannot leave the range (NaN maps to the flat `0.0`, matching `clamp_gain`'s own fallback), then interpolate the RGB components linearly between `EQ_GAIN_LOW` and `EQ_GAIN_MID` across `GAIN_MIN_DB..=0.0` and between `EQ_GAIN_MID` and `EQ_GAIN_HIGH` across `0.0..=GAIN_MAX_DB`, keeping alpha `1.0`. The three named gains must return the stop colors exactly so the panel's flat preset reads yellow and its extremes read green/red.
- `src/ui/style.rs`: add `pub fn band_slider_style(gain: f32, status: slider::Status) -> slider::Style`. Start from `chrome_slider_style(status)` and replace both halves of `rail.backgrounds` with `Background::Color(band_gain_color(gain))`, leaving `rail.width`, `rail.border`, and the whole `handle` untouched. Both halves matter: iced's slider draws `backgrounds.0` from the range start to the thumb and `backgrounds.1` from the thumb to the range end, so painting only one half would leave a gray segment whose extent moves with the gain. The pure function keeps the color rule testable without building a widget, as the neighboring chrome styles are.
- `src/ui/views/equalizer.rs`: in `view_equalizer`'s band loop, copy `let gain = bands[index];` before constructing the `VerticalSlider` and change its style to `.style(move |_theme, status| style::band_slider_style(gain, status))`. Capture the gain by value rather than the index so the closure owns only a `f32`. Leave the preamp's `chrome_slider` call as it is: the mockup leaves the preamp groove uniform.
- `src/ui/style.rs` test module: import `crate::equalizer::{GAIN_MAX_DB, GAIN_MIN_DB}` and add `band_gain_color_runs_green_through_yellow_to_red` (the three named gains return `EQ_GAIN_LOW`, `EQ_GAIN_MID`, `EQ_GAIN_HIGH` exactly; the midpoint of each half sits between its two stops component-wise), `band_gain_color_clamps_and_maps_nan_to_flat` (far below `GAIN_MIN_DB` is the low stop, far above `GAIN_MAX_DB` the high stop, and `f32::NAN` the mid stop), and `band_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb` (both `rail.backgrounds` equal `Background::Color(band_gain_color(gain))` for a nonzero gain; `rail.width`, `rail.border`, and `handle` equal `chrome_slider_style(status)`'s for `Active`, `Hovered`, and `Dragged`).
- `README.md`: after the Usage sentence listing the equalizer panel's widgets, add one sentence: each band slider's groove is tinted by its own gain, from green at `-12 dB` through yellow at `0 dB` to red at `+12 dB`. Do not rewrite the widget-list sentence itself, because the queued response-curve plan also edits it.
- `docs/invariants.md`: add a short `## Equalizer band tint` section after `## UI layout`: the guarantee above, enforced by `style::band_gain_color` and `style::band_slider_style` and used by `views::view_equalizer`, pinned by the three style tests, with a one-line History noting the tint is presentation-only and does not change the curve the audio filter applies.

**Files touched.** `src/ui/theme.rs`, `src/ui/style.rs`, `src/ui/views/equalizer.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `band_gain_color(GAIN_MIN_DB)`, `band_gain_color(0.0)`, and `band_gain_color(GAIN_MAX_DB)` return `theme::EQ_GAIN_LOW`, `theme::EQ_GAIN_MID`, and `theme::EQ_GAIN_HIGH` exactly, and intermediate gains interpolate between the neighboring stops.
- `band_gain_color` clamps values outside `GAIN_MIN_DB..=GAIN_MAX_DB` to the end stops and maps `f32::NAN` to the flat mid stop instead of producing an invalid color.
- `band_slider_style` paints both rail halves with `band_gain_color(gain)` for every slider status and leaves the chrome rail width, border, and handle identical to `chrome_slider_style(status)`.
- The ten band sliders in `view_equalizer` use `band_slider_style` with their own stored gain, so the flat preset renders all-yellow grooves and the Rock preset renders green cuts and red boosts; the existing `equalizer_panel_constructs_for_both_states_and_gain_endpoints` and `equalizer_band_labels_leave_a_gap_between_frequencies` still pass.
- `README.md` describes the gain tint, and `make check` passes with no new dependency.

## Done
