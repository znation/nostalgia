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

### Show the transport state on the Now Playing LCD (Transport glyph, part 1/2: planned 2026-10-23 by the plan role)

**Goal.** Match `docs/screenshots/transport.png`'s three LCD states: a green `▶` beside the elapsed readout while playing, a green `II` while paused, and a red `■` while stopped. The app draws no such glyph today, and `AppState` cannot tell Pause from Stop — both leave `is_playing == false` — so the view has no way to choose between `II` and `■`. Add that distinction to shared state and draw the glyph in a fixed-width slot before the elapsed readout. The paused blink is deliberately left to part 2/2.

**Approach.**

- `src/state.rs`: add `pub paused: bool` beside `is_playing`, documented as true only while the loaded preview is paused and false while playing or stopped; its `Default` value is `false`. Set it in the existing mutators: `toggle_playing` flips `is_playing` and then sets `paused` to the new `!is_playing` (so the pause half pauses and the resume half resumes), `stop` and `play` clear it, and `pause` sets it. Update the `Default` doc's starting-state sentence. Add a public `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum TransportMode { Playing, Paused, Stopped }` and `pub fn transport_mode(&self) -> TransportMode`, returning `Playing` when `is_playing`, `Paused` when `paused`, and `Stopped` otherwise.
- `src/apple_music/playback.rs`: in `play_track`, replace the direct `state.is_playing = true` commit with `state.play()`, and the direct `state.is_playing = false` backend-failure rollback with `state.stop()`; both keep `current_track` as the surrounding code expects, and routing through the setters keeps `paused` from going stale. Update the `pause`/`resume`/`stop` doc comments to name the paused flag.
- `src/ui/theme.rs`: add `pub const LCD_RED: Color = Color::from_rgb(1.0, 0.1, 0.1);` beside `LCD_GREEN`, documented as the stopped-transport glyph color.
- `src/ui/views/now_playing.rs`: import `crate::state::TransportMode` and add `LCD_TRANSPORT_SIZE: f32 = 22.0` and `LCD_TRANSPORT_WIDTH: f32 = 16.0`. Add pure `pub(super) fn transport_glyph(mode: TransportMode) -> &'static str` returning `▶`, `II`, and `■`, and `pub(super) fn transport_glyph_color(mode: TransportMode) -> Color` returning `theme::LCD_GREEN` for `Playing`/`Paused` and `theme::LCD_RED` for `Stopped`. In `now_playing_row`, prepend a `Container` of fixed `LCD_TRANSPORT_WIDTH` holding the glyph as a `Text` at `LCD_TRANSPORT_SIZE` in that color, before the elapsed `MouseArea`. The fixed slot keeps the elapsed readout at one x position as the glyph changes. Add a trailing `mode: TransportMode` parameter to `now_playing_row`, `now_playing_content`, and `view_now_playing`, threading it through unchanged.
- `src/ui/view.rs`: resolve `state.transport_mode()` in the normal-mode state destructure and pass it to `views::view_now_playing`. Shade mode needs no change: its Play/Pause glyph keys off `is_playing`, so paused already shows `▶`, which is correct for a control that resumes.
- `src/ui/views/tests.rs`: update every `now_playing_row`/`now_playing_content`/`view_now_playing` call for the new argument. Change `captured_now_playing_texts` to take a `TransportMode` and assert the glyph is the first drawn fragment, the elapsed readout the second. Add `transport_glyph_names_and_colors_each_mode` (exact strings and exact `LCD_GREEN`/`LCD_RED` colors), `now_playing_row_keeps_the_elapsed_readout_in_place_across_transport_modes` (the elapsed fragment's x is equal in all three modes while each glyph draws), and extend `now_playing_content_draws_the_indicator_strip_beside_the_title` to assert the glyph follows the `O A I D V` strip.
- `src/state.rs` tests: extend `default_state_is_stopped_at_half_volume` with `!state.paused` and `TransportMode::Stopped`, rewrite `toggle_playing_flips_only_the_playback_flag` as `toggle_playing_switches_between_playing_and_paused` (playing → paused → playing), and add `pause_sets_the_paused_mode_while_play_and_stop_clear_it` driving `pause`, `play`, and `stop` and checking `transport_mode()`.
- `src/apple_music/tests.rs`: assert the mode in `pause_stops_playing_but_keeps_current_track` (Paused), `resume_resumes_the_audio_backend` and `resume_after_stop_restarts_the_preview` (Playing), `stop_stops_the_audio_backend` (Stopped), and `toggle_play_pause_pauses_while_playing` (Paused).
- `README.md`: in the Usage paragraph that describes the large elapsed readout, add one sentence naming the green `▶`, green `II`, and red `■` states. Do not touch the equalizer sentence the queued tint plan edits.
- `docs/invariants.md`: in `## Now Playing label`, add a bullet stating the glyph rule, that Pause sets the paused mode while Play and Stop clear it, and that the glyph's fixed-width slot leaves the elapsed readout in place; name `state::TransportMode`/`AppState::transport_mode`, `views::now_playing::transport_glyph`/`transport_glyph_color`, and `now_playing_row` as the enforcers and the new tests as the pins.

**Files touched.** `src/state.rs`, `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `src/ui/theme.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/view.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `AppState::pause` leaves `is_playing == false` and sets `paused == true`; `play` and `stop` clear both; `toggle_playing` alternates `Playing` and `Paused`; `transport_mode()` reports `Stopped` for the default state.
- `transport_glyph` returns exactly `▶` for `Playing`, `II` for `Paused`, and `■` for `Stopped`; `transport_glyph_color` returns `theme::LCD_GREEN` for the first two and `theme::LCD_RED` for `Stopped`.
- The LCD row draws the glyph before the elapsed readout in all three modes, and the elapsed readout keeps the same x across the three glyphs because the slot is fixed-width.
- Pause and Stop remain distinguishable after a real Play/Pause/Stop sequence through `AppleMusicService`: Pause reports `Paused` and keeps the current track, while Stop reports `Stopped` and keeps it too.
- `README.md` and `docs/invariants.md` describe the glyph, and `make check` passes with no new dependency.

### Show the transport state on the Now Playing LCD (Transport glyph, part 2/2: planned 2026-10-23 by the plan role, requires part 1/2 landed)

**Goal.** Finish the paused state from `docs/screenshots/transport.png`: while the player is paused, the large elapsed readout blinks instead of sitting steady, while the position it shows stays frozen. Reuse the existing one-second position poll for the blink phase rather than adding a timer, and hide the readout by drawing it transparent rather than blanking it, so the row keeps its bounds and the title does not reflow.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) blink_visible: bool` to `WinampPlayer`, initialized `true`, documented as the paused readout's blink phase: `position_tick` toggles it while paused and holds it true whenever the transport is not paused. The view uses it only in the paused mode.
- `src/ui/handlers/playback.rs`: in `position_tick`, after the `pending_position_transitions` early return and before scheduling the queries, read `player.state.blocking_lock().transport_mode()` into a local and set `player.blink_visible = if mode == TransportMode::Paused { !player.blink_visible } else { true };` so the paused readout alternates once per position tick and a resumed or stopped player is always visible.
- `src/ui/views/now_playing.rs`: add `pub(super) fn elapsed_readout_color(mode: TransportMode, visible: bool) -> Color`, returning `Color::from_rgba(0.0, 0.0, 0.0, 0.0)` for `Paused` with `visible == false` and `theme::LCD_GREEN` otherwise. Add a trailing `elapsed_visible: bool` parameter to `now_playing_row`, `now_playing_content`, and `view_now_playing`; build the elapsed text with `lcd_elapsed_text(elapsed).color(elapsed_readout_color(mode, elapsed_visible))` so the text still lays out at the same width while transparent.
- `src/ui/view.rs`: pass `player.blink_visible` into `views::view_now_playing`.
- `src/ui/tests.rs`: add `position_tick_blinks_the_elapsed_readout_only_while_paused`, building a player with a loaded preview, setting `paused` through the state, and driving `position_tick`: the first tick hides the readout, the second shows it again, and a tick after `play` (or `stop`) leaves it visible.
- `src/ui/views/tests.rs`: add `elapsed_readout_color_blinks_only_while_paused` (`Paused`/hidden is transparent; `Paused`/visible, `Playing`, and `Stopped` in either phase are `LCD_GREEN`) and `now_playing_row_keeps_the_elapsed_bounds_when_the_blink_hides_it` (the paused visible and hidden captures draw the same elapsed string with the same bounds). Update the existing row/content/view calls for the new argument.
- `README.md`: extend the transport-glyph sentence with the paused blink. `docs/invariants.md`: extend the `## Now Playing label` glyph bullet with the blink guarantee and name `position_tick`'s phase flag and `elapsed_readout_color` as the enforcers and the new tests as the pins.

**Files touched.** `src/ui/player.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `src/ui/view.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- While `transport_mode()` is `Paused`, each `position_tick` flips `blink_visible`; while it is `Playing` or `Stopped`, a tick sets `blink_visible` to true, so playing and stopped always show the elapsed readout.
- `elapsed_readout_color(TransportMode::Paused, false)` is fully transparent, and every other mode/phase returns `theme::LCD_GREEN`.
- The elapsed readout keeps the same text and bounds in the paused visible and hidden states, so blinking does not move the title, total, or status row.
- The position poll keeps running while paused as it does today, and the blink adds no new subscription or timer.
- `README.md` and `docs/invariants.md` describe the paused blink, and `make check` passes.

## Done
