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

### Draw the mockups' gold title tape and the equalizer and library title bands (planned 2026-10-23 by the plan role)

**Goal.** Match the title chrome every mockup draws (`docs/screenshots/hero.png`, `player.png`,
`transport.png`, `equalizer.png`, and `library.png`): a blue band with a gold two-tone tape strip
on each side of a centered label. The main bar today is a plain blue `NOSTALGIA` band with no tape,
and the equalizer and browse sections have no headers at all. Add the tape to the app bar and draw
`NOSTALGIA EQUALIZER` above the equalizer and `NOSTALGIA LIBRARY` above the browse controls. The
section bands carry no window buttons: this app is one real window whose controls already live in
the app bar, so a second set would be clickable-looking dead chrome. This is presentation only —
no message, state, playback, audio, browse-order, or scroll behavior changes — and it is
independent of the queued playing-row plan (that plan changes where the list scrolls; this one
adds two full-width header rows).

**Approach.**

- `src/ui/theme.rs`: add `TITLE_TAPE` (the mockup's bright gold face, sampled as `(236, 206, 122)` →
  `Color::from_rgb(0.93, 0.81, 0.48)`) and `TITLE_TAPE_SHADOW` (its darker bottom edge, sampled as
  `(163, 148, 106)` → `0.64, 0.58, 0.42`), each documented as a title-tape color. Extend the
  existing base-skin color test to pin both values and that they differ from `TITLE_BLUE`.
- `src/ui/views/title_bar.rs`: add a private `title_tape() -> Element<'static, Message>`: an
  8px-tall `Length::Fill` strip, vertically centered, with the gold face between a lighter top
  edge and the `TITLE_TAPE_SHADOW` bottom edge (a 1px top rule and 1px bottom rule around a
  `Length::Fill` face is enough; the mockup's tape rows measure 2px each). Add `pub fn view_title_band(label: &'static str) -> Element<'static, Message>`: a
  `bevel::raised_panel` over a `Container` painted with `style::title_bar_style()`, height
  `TITLE_BAR_HEIGHT`, holding a `Row` with spacing and padding of `title_tape()`, the label as
  `Text::new(label).size(14).color(theme::TEXT)`, and another `title_tape()`, so the two tapes
  fill equally on each side and the label sits centered between them. Document that the band is
  decorative and deliberately has no buttons. Refactor `view_title_bar` to put the same
  tape/label/tape `Row` inside its existing `MouseArea` drag region (so the app name becomes
  centered between the tapes, as in the mockup, while the A and shade/minimize/close controls keep
  their current pinned widths and order); the drag press and double-click still cover exactly
  that middle span. Update the module doc.
- `src/ui/views/mod.rs`: re-export `view_title_band` beside `view_title_bar` and
  `TITLE_BAR_HEIGHT`.
- `src/ui/view.rs`: in the normal-mode column, push
  `views::view_title_band("NOSTALGIA EQUALIZER")` immediately before
  `views::view_equalizer(...)`, and `views::view_title_band("NOSTALGIA LIBRARY")` immediately
  before the Back/Retry/Top/Sort/search/list group. Leave the shaded early return untouched, so
  shade mode still builds only the app bar and the strip and neither section band appears.
- `src/ui/views/tests.rs`: add `title_band_centers_its_label_at_the_title_bar_height`, capturing
  `view_title_band("NOSTALGIA EQUALIZER")` with `layout_capture` and asserting the captured height
  equals `TITLE_BAR_HEIGHT`, the label is the only text fragment, and the label's center is the
  captured width's center. Add `title_bar_centers_its_name_between_its_edge_controls`, capturing
  `view_title_bar(false)` and asserting `NOSTALGIA`'s center is the midpoint between the A
  button's right edge and the shade button's left edge; keep the existing pinned-width test.
- `src/ui/tests.rs`: add `root_view_draws_the_equalizer_and_library_title_bands_in_order`: capture
  the normal root view with an artist list loaded, find `NOSTALGIA EQUALIZER`, `NOSTALGIA
  LIBRARY`, and the first browse row's text, and assert the equalizer band's `y` is above the
  library band's, which is above the first row's. Add
  `shaded_root_view_draws_no_section_title_bands`, asserting the shaded root view contains neither
  section label while the app-bar `NOSTALGIA` still draws. Keep the existing shade and root-view
  tests green.
- `README.md`: in the Usage paragraph that describes the equalizer and browse list, say those two
  sections are introduced by `NOSTALGIA EQUALIZER` and `NOSTALGIA LIBRARY` title bands and that
  the app bar sets `NOSTALGIA` between gold tapes. Do not touch the `<!-- tumwater:status:start
  -->` block.
- `docs/invariants.md`: in `## UI layout`, add a bullet: the app bar and the two section bands are
  `TITLE_BAR_HEIGHT` tall; the app bar centers `NOSTALGIA` between gold tapes with its A and
  shade/minimize/close controls pinned at the edges; normal mode draws `NOSTALGIA EQUALIZER`
  immediately above the equalizer and `NOSTALGIA LIBRARY` immediately above the browse controls;
  the section bands emit no messages and are absent in shade mode. Name `views::view_title_band`,
  `views::title_bar::title_tape`, `theme::TITLE_TAPE`/`TITLE_TAPE_SHADOW`, and the new tests as
  the enforcers and pins.

**Files touched.** `src/ui/theme.rs`, `src/ui/views/title_bar.rs`, `src/ui/views/mod.rs`,
`src/ui/view.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- The app bar draws `NOSTALGIA` centered between two gold tape strips on the blue band, keeps its A
  and shade/minimize/close controls in their current order and pinned widths, and still drags on
  press and toggles shade on double-click across the tape/label span.
- Normal mode draws a `TITLE_BAR_HEIGHT`-tall `NOSTALGIA EQUALIZER` band directly above the
  equalizer and a `NOSTALGIA LIBRARY` band directly above the browse controls, both blue with the
  same tape and no clickable controls; shade mode draws neither band.
- The band test pins label centering and height, the app-bar test pins the name's centered span
  between its edge controls, and the root-view tests pin the two labels' vertical order above the
  first browse row and their absence when shaded.
- No message, state, playback, audio, browse-order, or existing control-order change; no
  dependency is added; `make check` passes; the README and the new invariant bullet describe the
  same chrome.

## Done
