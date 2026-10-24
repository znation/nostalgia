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

### Restore the window from shade mode by double-clicking the strip (planned 2026-10-24 by the plan role)

**Goal.** In shade mode the rolled-up window is the title bar over the compact transport strip,
and only the 24px title bar answers a double-click by unshading (through
`views::title_bar::view_title_bar`'s `MouseArea::on_double_click`). The classic Winamp behavior
is that any double-click on the rolled-up window restores it, so a double-click on the much
larger strip should unshade too. Make the strip's blank chrome a double-click target for the
existing `Message::ToggleWindowShade`, without adding a message, handler, or state flag and
without changing any single-click, button, readout, slider, or wheel behavior.

**Approach.**

- `src/ui/views/shade.rs`: import `MouseArea`, then wrap the element returned by
  `view_shade_strip` in `MouseArea::new(...).on_double_click(Message::ToggleWindowShade)`. Keep
  the existing `bevel::raised_panel(...).style(style::title_bar_style())` as the `MouseArea`'s
  content so the strip's chrome and `SHADE_STRIP_HEIGHT` geometry stay exactly as they are.
  Extend `view_shade_strip`'s doc comment to name the double-click restore and to record that
  `MouseArea` forwards events to its content first, so the strip's buttons, time readout, seek
  slider, and volume slider keep their own behavior and a double-click on them does not unshade.
  Reuse `handlers::window::toggle_window_shade` as it stands; it already restores the saved size
  or measures the first shade.
- `src/ui/views/tests.rs`: add `shade_strip_double_click_restores_the_window`. Build
  `view_shade_strip` in `TransportMode::Stopped` with no position or duration, so no seek slider
  is drawn and the centered fill area is blank; lay it out with `layout_capture::headless_renderer`
  and drive `iced_runtime::user_interface::UserInterface::update` at the point
  `(capture.size().width / 2.0, capture.size().height / 2.0)` (the fill spacer between the
  readout and the volume slider, not a control). Send one left press and release and assert no
  `Message::ToggleWindowShade` is published, then send a second left press and assert exactly one
  `Message::ToggleWindowShade`, so the test proves both the wiring and that a single click stays
  inert.
- `README.md`: in the title-bar sentence, change the restore wording to name a double-click on
  the rolled-up strip alongside the shade button and the title bar's double-click. Do not edit
  the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Window shade`, add the guarantee that a double-click on the strip
  publishes the same `Message::ToggleWindowShade` as the title-bar double-click and restores the
  window; name `views::shade::view_shade_strip` and `handlers::window::toggle_window_shade` as
  the enforcers and the new test under Pinned by, and extend the History note.

**Files touched.** `src/ui/views/shade.rs`, `src/ui/views/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- A double-click on the shade strip's blank chrome publishes one `Message::ToggleWindowShade`
  and restores the window exactly as the shade button and the title bar's double-click do; a
  single click publishes nothing.
- The strip's four buttons, time readout, seek slider (when drawn), and volume slider keep their
  current presses, drags, toggles, and wheel behavior, and the strip keeps `SHADE_STRIP_HEIGHT`
  and its blue chrome unchanged.
- No new `Message` variant, handler, or player flag is added, and normal mode is untouched.
- The new test passes and `make check` passes.

## Done
