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
> at the right edge of the LCD well, and `view` (`src/ui/mod.rs`) stacks that
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

### Show the mini transport in window-shade mode (planned 2026-10-23 by the plan role)

**Goal.** Match the mockup's rolled-up window (`docs/screenshots/equalizer.png`, the "WINDOW SHADE" panel): shading the window shows, below the title bar, a compact strip with Previous, Play/Pause, Stop, Next, the current elapsed readout, and a volume slider, so playback stays controllable while rolled up. Normal mode is unchanged; the strip only reads existing state and emits existing messages, adding no new playback behavior.

**Approach.**

- Add `src/ui/views/shade.rs` (declare `mod shade;` and re-export it from `src/ui/views/mod.rs` beside `TITLE_BAR_HEIGHT`):
  - `pub(super) const SHADE_STRIP_HEIGHT: f32 = 28.0;` and `pub(super) const SHADED_HEIGHT: f32 = TITLE_BAR_HEIGHT + SHADE_STRIP_HEIGHT;`, so the strip height and the whole rolled-up height have one owner each.
  - `pub(super) fn play_pause_glyph(is_playing: bool) -> &'static str`: `"II"` while playing and `"▶"` otherwise, the compact twin of `transport::play_pause_label`.
  - `pub(super) fn shade_controls(is_playing: bool) -> [(&'static str, f32, Message); 5]`: the five `(label, fixed width, message)` specs in row order — `"◀◀"`/`Message::PreviousTrack`, [`play_pause_glyph`]/`Message::PlayPause`, `"■"`/`Message::Stop`, `"▶▶"`/`Message::NextTrack` — each with a fixed width (34, 30, 30, 34 px) so the glyph swap cannot reflow the strip. Pure, so labels, messages, and widths are testable without an iced renderer.
  - `pub fn view_shade_strip<'a>(is_playing: bool, volume: f32, elapsed: Cow<'a, str>) -> Element<'a, Message>`: one full-width row built from `fixed_width_button` per `shade_controls` entry with `spacer(6.0)` between them, the elapsed as `Text::new(elapsed).size(14).color(theme::LCD_GREEN)`, and `chrome_slider(VOLUME_MIN, VOLUME_MAX, volume, VOLUME_STEP, 80.0, Message::VolumeChange)` at the right. Paint the row with `style::title_bar_style()` inside a container fixed at `SHADE_STRIP_HEIGHT` (wrapped in `bevel::raised_panel` like the title bar) so the strip shares the title bar's blue chrome. No new message or handler is added.
- `src/ui/mod.rs` `view`: replace the shaded early return's bare title bar with a short state lock for `state.is_playing`, `state.volume()`, and `player.now_playing_elapsed(state.current_track.as_deref())`, then a `Column` of `views::view_title_bar(...)` and `views::view_shade_strip(...)`. Passing the same `now_playing_elapsed` string the LCD uses means the elapsed/remaining toggle carries into shade mode. Update the function's doc comment and inline comment to say the rolled-up tree is the title bar plus the strip. Leave the unshaded assembly untouched.
- `src/ui/handlers/window.rs`: `resize_to_shade` resizes to `views::SHADED_HEIGHT` instead of `views::TITLE_BAR_HEIGHT`, and its doc comment names the strip as part of the rolled-up window. The measurement and restore paths in `toggle_window_shade` and `window_shade_measured` are unchanged.
- `src/ui/views/tests.rs`: add `shade_play_pause_glyph_switches_with_playback_state`, `shade_controls_map_to_transport_messages_and_keep_fixed_widths`, `shade_height_pins_the_rolled_up_window` (`SHADED_HEIGHT == TITLE_BAR_HEIGHT + SHADE_STRIP_HEIGHT`, both positive), and `view_shade_strip_keeps_its_fixed_height_for_both_playback_states` (lay the widget out with the headless-renderer `Limits` pattern already used in this file and assert the resolved height is `SHADE_STRIP_HEIGHT`).
- `src/ui/tests.rs`: update `title_bar_height_pins_the_shade_strip`'s comment to point at `SHADED_HEIGHT`; add `shaded_root_view_is_the_title_bar_plus_the_strip` (lay out the root `view` with `player.shaded = true` using the headless tiny-skia pattern from `browse_nav_buttons_place_sort_beside_top`, and assert the resolved height is `views::SHADED_HEIGHT`) and `shade_strip_draws_only_while_shaded` (capture text bounds and assert the `"◀◀"` control appears in the shaded tree and not in the normal-mode tree). Keep the existing shade measurement/restore tests green.
- `README.md`: in the Usage paragraph, say shade mode rolls the window up to the title bar plus a compact transport strip with Previous, Play/Pause, Stop, Next, the current time readout, and a volume slider. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Window shade` section after `## Now Playing label`: shade mode builds exactly the title bar plus the fixed-height strip and resizes the window to `SHADED_HEIGHT`; the strip's five controls emit the same messages as their full transport-row twins, its Play/Pause glyph follows the playback state, its time label follows the elapsed/remaining choice, and its volume slider writes `Message::VolumeChange`; normal mode builds no strip. Name `views::shade::view_shade_strip`/`shade_controls`/`play_pause_glyph`/`SHADED_HEIGHT` and `handlers::window::resize_to_shade` as the enforcers, and list the new test names.

**Files touched.** `src/ui/views/shade.rs` (new), `src/ui/views/mod.rs`, `src/ui/mod.rs`, `src/ui/handlers/window.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- In shade mode the rendered tree is the title bar over a strip exactly `SHADE_STRIP_HEIGHT` tall, and the window resizes to `SHADED_HEIGHT`; the normal-mode tree contains no strip.
- The strip shows Previous, Play/Pause, Stop, Next, the current elapsed/remaining readout, and a volume slider, each emitting the same message as its full transport-row twin (`Message::PreviousTrack`, `Message::PlayPause`, `Message::Stop`, `Message::NextTrack`, `Message::VolumeChange`).
- The Play/Pause glyph is `"▶"` while stopped and `"II"` while playing, and every compact button keeps its fixed width across that swap.
- Laying out the strip and the shaded root view with a headless renderer resolves to the fixed heights above, so the view and `resize_to_shade` cannot drift apart.
- Normal-mode layout, the existing shade measurement/restore tests, and playback behavior are unchanged.
- `make check` passes (fmt, clippy, rustdoc, tests).

### Add a shared headless layout-capture harness so UI geometry bugs confirm offline (planned 2026-10-23 by the steward role as a gap promotion for `real-run-needed`)

**Goal.** Retire the `real-run-needed` validation gap for UI geometry: make the laid-out bounds of the real widget tree readable from a unit test, so a layout or label-reflow bug is confirmed by `make test` instead of a hand-run window measurement. Four Fixed entries carry the `real-run-needed` gap. The two non-layout ones now have offline coverage — the sign-in race through the injectable session seam and `AppleMusicService::wait_for_session`, and the nonce leak through the capturing opener in `authorize_keeps_the_state_nonce_out_of_the_opener_argument` — while the two layout ones, "Sort renders below Top instead of beside it, contradicting the documented layout" and "Transport buttons resize to their label text, so pressing Play/Pause reflows the whole control row", were confirmed only by running the GUI and measuring a captured window. The ad hoc `CapturedText` operation in `src/ui/views/tests.rs` and `CapturedTextBounds` in `src/ui/tests.rs` prove the offline path works; this plan promotes them into one shared harness so the next layout bug does not reconstruct it.

**Approach.**

- Add a test-only `src/ui/layout_capture.rs` (declared `#[cfg(test)] pub(crate) mod layout_capture;` in `src/ui/mod.rs`), holding one `CapturedText` widget `Operation` and a small API: a capture function that lays a prepared `Element` out under a fixed `Limits`, runs the operation, and returns each drawn `(text, Rectangle)` with its absolute bounds plus the root `Size`; a `find(label)` lookup that panics with the captured fragments when a label is missing; and `headless_renderer()`, which builds iced's tiny-skia `Headless` renderer with `futures::executor::block_on` (the pattern `browse_nav_buttons_place_sort_beside_top` already uses), so trees containing the fallback-renderer `bevel` widgets lay out too.
- Move the duplicate capture structs out of `src/ui/views/tests.rs` (`CapturedText`) and `src/ui/tests.rs` (`CapturedTextBounds`) and point both suites at the shared module, keeping the existing helpers' assertions (the Now Playing fragment order, the indicator strip's order and width) intact.
- Rewrite `browse_nav_buttons_place_sort_beside_top` on the harness without weakening it: it must still lay out the whole root `view` (not the nav row alone) and assert Sort shares Top's row and starts at or after Top's right edge.
- Add `transport_row_does_not_reflow_when_the_play_label_changes`: lay the root `view` out with the player stopped and then playing, and assert the `Stop` and `Repeat` labels keep the same absolute x, which fails if a label-driven width change reaches the row as it did before the fixed-width fix. Keep the builder-level `transport_buttons_keep_a_fixed_width_across_label_changes` and the measured-width pins unchanged.
- Add a harness self-test that lays out a small known column and asserts the captured fragments and root size, so a broken capture fails directly rather than as a confusing geometry assertion.
- `docs/invariants.md`: add a `## UI layout` section after `## Browse row order` stating that Sort lays out beside Top in the root view and that the transport row's faces keep their bounds across the Play/Pause, Repeat, and Shuffle label changes; name `views::view_browse_nav_buttons`, `views::view_transport_controls`, and `views::transport::transport_buttons` as the enforcers and cite the two layout tests and the two Fixed entries. Add a line to the "How to use" note pointing layout bugs at the harness.

**Files touched.** `src/ui/layout_capture.rs` (new), `src/ui/mod.rs`, `src/ui/tests.rs`, `src/ui/views/tests.rs`, `docs/invariants.md`.

**Acceptance criteria.**

- One shared test module returns every drawn text fragment with its absolute `Rectangle` and the root `Size` for both a null-renderer view and the tiny-skia root view; both existing capture helpers are deleted and their tests run through it.
- `browse_nav_buttons_place_sort_beside_top` still lays out the root `view` and fails if Sort is stacked below Top or drawn before Top's right edge.
- `transport_row_does_not_reflow_when_the_play_label_changes` lays the root view out in both playback states and asserts Stop and Repeat keep their x, so the historical reflow is confirmed offline.
- The new `## UI layout` invariant states both guarantees, names their enforcing code, and lists the tests that pin them; the harness is discoverable from `docs/invariants.md`'s "How to use" note.
- `make check` passes (fmt, clippy, rustdoc, tests), and the change adds no dependency.

**Cited entries.** `BUGS-FIXED.md`: the four `real-run-needed` entries — "Sort renders below Top instead of beside it, contradicting the documented layout", "Transport buttons resize to their label text, so pressing Play/Pause reflows the whole control row", "The boot-time artists fetch races the async sign-in, so a signed-in user sees the sample library", and "The sign-in nonce travels in the browser opener's command line, so another local user can read it and fetch the page's developer token".

## Done
