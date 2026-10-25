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

_None yet._

### Click the LCD visualizer to hide or show it, like Winamp's vis area (planned 2026-10-24 by the plan role)

**Goal.** In Winamp 2.x a left click on the main window's visualizer area cycles its mode
(spectrum, oscilloscope, off). Nostalgia has one mode, so the equivalent is a toggle: a left
press on the LCD's ten-bar visualizer strip publishes the same `Message::ToggleVisualizer` the
strip's `V` letter already publishes. The hidden strip keeps its fixed height and width (it
draws zero-height bars), so a press on the empty strip shows the bars again. No new message,
handler, state, or setting: `handlers::playback::toggle_visualizer` and the saved
`visualizer_visible` flag are reused as they are.

**Approach.**
- `src/ui/views/now_playing.rs`, `now_playing_visualizer_row`: keep building the same `Row`,
  then wrap it as `MouseArea::new(row).on_press(Message::ToggleVisualizer)
  .interaction(Interaction::Pointer)` (the pattern `lcd_indicator_strip` and the elapsed
  readout in `now_playing_row` already use). Change the return type from `Row<…>` to
  `MouseArea<'a, Message, Theme, Renderer>`; `now_playing_content` pushes it unchanged. Extend
  the doc comment to say a press toggles the visualizer. The press is armed in every playback
  state and whether or not the bars are visible, because `V` is too.
- `now_playing_content` passes `visible_spectrum(…)` output in as before; nothing about what
  is drawn changes.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests/lcd_display.rs` (or
`src/ui/views/tests/now_playing.rs`, beside
`lcd_indicator_strip_publishes_always_on_top_on_a_and_visualizer_on_v`), `docs/invariants.md`,
`README.md`.

**Acceptance criteria.**
- New test `visualizer_row_press_publishes_toggle_visualizer`: for both `None` and a
  `Some(Spectrum { … })` with non-zero bands, build `now_playing_visualizer_row(spectrum)` as an
  `iced::Element<'_, Message, iced::Theme, iced::Renderer>`, lay it out to find its bounds, and
  drive a left `ButtonPressed` at the bounds' centre through `layout_capture::publish_at`;
  assert exactly one `Message::ToggleVisualizer` is published.
- The existing `now_playing_visualizer_row_keeps_its_height_without_a_spectrum` and
  `now_playing_visualizer_row_draws_one_fixed_width_bar_per_band` still pass unchanged (the
  wrapper adds no height or container), as do the indicator-strip and
  `elapsed_readout_toggles_only_while_not_stopped` press tests.
- `docs/invariants.md`, `## Now Playing label`: in the bullet beginning "The LCD's visualizer
  row draws exactly `BAND_COUNT` green bars", add that the row is a mouse area whose left press
  publishes `Message::ToggleVisualizer` (the same message as the `V` letter) whether the bars
  are shown or hidden, and name the new test.
- README Usage: after the sentence "The strip's `V` hides the bars and shows them again from
  the still-stored spectrum, so the poll keeps running while they are hidden.", add a sentence
  that a click on the bars' strip does the same, hidden or shown. Do not touch the brief or
  status blocks.
- `make check` passes.

## Done
