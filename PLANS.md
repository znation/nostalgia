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
> glyph row landed on the same day. The select-then-open plan has since landed
> (done 2026-10-23).

## Planned

### Paint browse rows green, the playing row white, and only the selected row blue (planned 2026-10-23 by the plan role)

**Goal.** Match `docs/screenshots/library.png` and `docs/screenshots/hero.png`: their browse rows
are bright green, the playing row's text is white, and the blue bar marks the selected row only.
Today `style::playlist_row_style` paints every row `theme::TEXT` (light gray), and
`views::browse::rows::current_row_style` forces `theme::PLAYING_ROW_HIGHLIGHT` behind both the
playing row and the selected row, so the playing row and the selection look identical and
neither matches the mockups. This is a row-color change only: numbering, the `▶` marker,
selection and open behavior, row text, hover/press faces, layout, and scrolling stay as they are.

**Approach.**

- `src/ui/theme.rs`: add `pub const PLAYING_ROW_TEXT: Color = Color::from_rgb(1.0, 1.0, 1.0);`,
documented as the white text of the playing browse row. Rename
`PLAYING_ROW_HIGHLIGHT` to `SELECTED_ROW_HIGHLIGHT` and reword its doc to say it is the blue bar
behind the selected browse row; keep its RGB value. Update every reference.
- `src/ui/style.rs`: in `playlist_row_style`, change `text_color` from `theme::TEXT` to
`theme::LCD_GREEN`, keeping the hover/press background ladder and the flat border/shadow. Update
the doc from "flat light text" to the mockups' flat green row text.
- `src/ui/views/browse/rows.rs`: replace `current_row_style(status)` with `pub(in
crate::ui::views) fn browse_row_style(status: button::Status, is_current: bool, is_selected:
bool) -> button::Style`. Start from `style::playlist_row_style(status)`; when `is_selected`,
set `background = Some(Background::Color(theme::SELECTED_ROW_HIGHLIGHT))`; when `is_current ||
is_selected`, set `text_color = theme::PLAYING_ROW_TEXT`. A row that is both playing and selected
therefore gets the selection bar and white text, while its `▶` prefix still identifies it. In
`scrollable_list`, compute `is_selected = selected == Some(position)` and apply one `.style(move
|_theme, status| browse_row_style(status, is_current, is_selected))` to every row instead of the
current playing-or-selected branch. Update the module and function docs to say unselected rows
are green, the playing row is white without a bar, and the selected row is blue with white text.
- `src/ui/views/tests.rs`: import `browse_row_style` in place of `current_row_style`. Replace
`current_row_style_paints_the_selection_bar_over_every_row_state` with two pure-seam tests.
`browse_row_style_keeps_playing_rows_unbarred_with_white_text` asserts, for every button status,
that `browse_row_style(status, true, false)` keeps `style::playlist_row_style(status).background`
(so the resting playing row has no bar and hover/press still lift and sink) and uses
`theme::PLAYING_ROW_TEXT`, which differs from the base row's green text.
`browse_row_style_paints_only_the_selected_row_blue` asserts, for every status and for
`is_current` both false and true, that a selected row's background is
`theme::SELECTED_ROW_HIGHLIGHT` and its text is white, while `browse_row_style(status, false,
false)` keeps the base background and `theme::LCD_GREEN` text; it also asserts the selected
background differs from the plain resting background.
- `src/ui/style.rs` tests: update `playlist_row_style_is_a_flat_entry_that_lifts_and_sinks` to
assert `theme::LCD_GREEN` text for every status instead of `theme::TEXT`.
- `src/ui/theme.rs` tests: rename the highlight assertions to `SELECTED_ROW_HIGHLIGHT`, add the
`PLAYING_ROW_TEXT` literal, and assert it differs from `theme::LCD_GREEN` so the playing row
cannot become indistinguishable from an ordinary row.
- `README.md`: in the Usage browse sentence, say rows draw green text, the playing song's row is
white with its `▶` marker and no bar, and a click selects a row and paints its blue bar with
white text; a second click on the selected row or `Open` opens it. Do not edit the
`<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Browse selection`, state that unselected rows draw green text, the
playing row keeps its `▶` marker and white text without a bar unless selected, and only the
selected row draws the blue `SELECTED_ROW_HIGHLIGHT` bar with white text. Name
`style::playlist_row_style` and `views::browse::rows::browse_row_style`/`scrollable_list` as the
enforcers, and add the two new tests to `Pinned by`.

**Files touched.** `src/ui/theme.rs`, `src/ui/style.rs`, `src/ui/views/browse/rows.rs`,
`src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Every unselected, non-playing browse row in every list draws `theme::LCD_GREEN` text with no
  blue bar, keeping the existing hover and press faces.
- The playing row keeps its `▶` prefix and hover/press faces but draws no blue bar, and its text
  is `theme::PLAYING_ROW_TEXT` (white), so it is distinguishable from an ordinary green row.
- Only the selected row draws `theme::SELECTED_ROW_HIGHLIGHT` behind it, in every hover/press
  state, with white text; a row that is both playing and selected gets the blue bar and stays
  identifiable by its `▶` prefix.
- Row numbering, titles, labels, footers, selection and open messages, scrolling, and layout are
  unchanged; README and `docs/invariants.md` describe the three row appearances, and `make check`
  passes.

## Done
