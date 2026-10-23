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
> is known. The LCD `MONO`/`STEREO` pair has since landed (done 2026-10-23),
> which brought the status row to `docs/screenshots/player.png` and
> `docs/screenshots/transport.png`, and the volume and balance slider fill
> has since landed (done 2026-10-23) with the orange and green grooves those
> mockups draw, and the playlist-number title prefix landed too (done
> 2026-10-23), matching `docs/screenshots/player.png`, and the transport
> glyph row landed on the same day. No planned entry remains.

## Planned

_None yet._

### Make the O A I D V strip's A and V controls work (planned 2026-10-23 by the plan role)

**Goal.** The mockup's right-hand copy says "try … the O A I D V strip", but the LCD's strip is
only decoration today: `views::now_playing::lcd_indicator_strip` draws five plain `Text`s and the
`## Now Playing label` invariant says the strip emits no message. Give the two letters the app can
already support a real action: `A` toggles always-on-top through the existing
`Message::ToggleAlwaysOnTop` (the classic clutter bar's A, in addition to the title bar's A), and
`V` turns the LCD spectrum visualizer off and back on without touching playback, the stored
spectrum, or its poll. `O`, `I`, and `D` stay decorative: an options menu, a file-info panel, and
pixel-doubled window mode have no implementation here (iced 0.14 exposes no runtime scale-factor
setter). This is independent of the queued mouse-wheel plan: it touches the LCD strip and one
display flag, not the view tree's scroll area.

**Approach.**

- `src/ui/views/now_playing.rs`: add `pub(in crate::ui::views) fn lcd_indicator_message(label:
  &str) -> Option<Message>` returning `Some(Message::ToggleAlwaysOnTop)` for `"A"`,
  `Some(Message::ToggleVisualizer)` for `"V"`, and `None` for `"O"`, `"I"`, and `"D"`. Keep it
  pure so the mapping is testable without an iced renderer.
- In `lcd_indicator_strip`, build the same green `Text` for every label, but wrap the `A` and `V`
  letters in `MouseArea::new(...).on_press(message).interaction(Interaction::Pointer)` and leave
  the other three as plain `Text`. Keep the column at `LCD_INDICATOR_WIDTH` and add no padding, so
  the letters' order, their x position, and the strip's width and height stay as they are.
- Add `pub(in crate::ui::views) fn visible_spectrum(spectrum: Option<Spectrum>,
  visualizer_visible: bool) -> Option<Spectrum>`: `Some` only when the flag is true and a
  spectrum is stored, otherwise `None`. Document that hiding changes only what the LCD draws —
  `WinampPlayer::spectrum` and the spectrum poll keep running, so turning it back on shows the
  current bars.
- Update the `LCD_INDICATOR_LABELS`, `lcd_indicator_strip`, and module docs to say A and V are
  live while O, I, and D stay decorative.
- `src/ui/message.rs`: add `ToggleVisualizer` beside `ToggleTimeDisplay`, documented as the strip
  V control: it flips only the LCD's display flag, changes no playback state, and schedules no
  work.
- `src/ui/handlers/playback.rs`: add `pub(in crate::ui) fn toggle_visualizer(player: &
  mut WinampPlayer) -> Task<Message>` beside `toggle_time_display`; flip
  `player.visualizer_visible` and return `Task::none()`.
- `src/ui/player.rs`: add `pub(in crate::ui) visualizer_visible: bool`, initialized `true` in
  `WinampPlayer::new`, documented as the indicator strip's V state and as leaving the stored
  spectrum untouched.
- `src/ui/mod.rs`: route `Message::ToggleVisualizer =>
  handlers::playback::toggle_visualizer(player)` beside the `ToggleTimeDisplay` arm.
- `src/ui/view.rs`: in the normal branch, pass `views::visible_spectrum(spectrum,
  player.visualizer_visible)` to `views::view_now_playing` where the stored spectrum goes today.
  The shade branch builds no LCD and is untouched.
- `src/ui/views/mod.rs`: re-export `lcd_indicator_message` and `visible_spectrum` from
  `now_playing` beside the other `now_playing` exports.
- `src/ui/views/tests.rs`: add `lcd_indicator_message_arms_a_and_v_and_leaves_o_i_d_inert`;
  `lcd_indicator_strip_publishes_always_on_top_on_a_and_visualizer_on_v`, using the headless
  renderer and `UserInterface::update` press pattern from `elapsed_readout_toggles_only_while_not_stopped`
  to assert that a press on A publishes `ToggleAlwaysOnTop`, on V publishes `ToggleVisualizer`, and
  on O publishes nothing; and `visible_spectrum_hides_the_stored_bars_while_the_visualizer_is_off`
  covering `Some`/`None` spectra with the flag on and off. Update the comments around
  `lcd_indicator_strip_draws_each_label_in_order_at_a_fixed_width` so the five letters and the
  fixed width are still pinned while the interactivity is documented.
- `src/ui/tests.rs`: add `toggle_visualizer_hides_and_shows_the_bars_without_clearing_the_spectrum`:
  store a known spectrum, drive `Message::ToggleVisualizer` through `update` with
  `assert_message_schedules_no_work`, and assert the flag flips twice while `player.spectrum` and
  the transport/mixer state stay unchanged.
- `README.md`: replace the "small decorative `O A I D V` strip" wording with a description of the
  strip's A and V actions and the still-decorative O, I, and D, and add to the visualizer sentence
  that V hides and restores the bars. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Preview spectrum`, state that the LCD draws the stored spectrum only
  while `WinampPlayer::visualizer_visible` holds, that hiding it keeps both the stored spectrum and
  the poll, and name `views::visible_spectrum`, `Message::ToggleVisualizer`, and
  `handlers::playback::toggle_visualizer` as enforcers. In `## Now Playing label`, replace the
  "strip is decorative: it emits no message" bullet with one saying A and V are mouse areas that
  publish `ToggleAlwaysOnTop` and `ToggleVisualizer`, O/I/D stay decorative, and the strip keeps
  its fixed width and letter order. Add the new test names to the Pinned-by lists.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`,
`src/ui/message.rs`, `src/ui/handlers/playback.rs`, `src/ui/player.rs`, `src/ui/mod.rs`,
`src/ui/view.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Pressing the strip's A publishes `Message::ToggleAlwaysOnTop`, the same handler as the title
  bar's A; pressing V hides the LCD bars and pressing it again restores them from the still-stored
  spectrum, without clearing `WinampPlayer::spectrum`, changing transport or mixer state,
  scheduling work, or stopping the spectrum poll.
- O, I, and D publish no message; the strip still draws `O`, `A`, `I`, `D`, `V` in that order at
  `LCD_INDICATOR_WIDTH`, and the content beside it keeps its bounds in every playback state.
- The toggle is pinned at the mapping, press, and handler levels, and `make check` passes.
- The README and `docs/invariants.md` describe the same live letters and the hidden-but-still-polled
  behavior; the title bar, shade strip, and playback keep their current behavior, and no dependency
  is added.

## Done
