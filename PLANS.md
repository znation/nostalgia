# Plans

## Planned

_None yet._

### Embed an openly-licensed pixel font so the chrome's text matches the base skin's bitmap face (planned 2026-10-30 by the plan role)

**Needs review 2026-10-30 by feature: too large for one run** — 16 view modules to re-size plus text-bound assertions re-derived across 17 test modules must land together or the chrome's metrics diverge mid-way.

**Goal.** Every label the app draws — the LCD readout, the marquee title, the transport
labels, the browse rows, the equalizer's scale — renders in iced's default face (the
system sans; `grep -rn '\.font(\|Font::' src/` finds no font set anywhere in production
code), so the app's most visible surface has none of the base skin's bitmap-text look that
`winamp-screenshots/`'s captures and `docs/screenshots/`'s mockups both show. Embed one
openly-licensed pixel face in the binary (no runtime font dependency, no new crate) and use
it app-wide at sizes that land on the face's pixel grid, so a 1:1 screenshot's text matches
the reference's.

**Ground truth checked this run.**

1. No font is set anywhere in production: the only `Font` in `src/` is
   `layout_capture.rs`'s `Font::default()` for the headless test renderer.
2. iced 0.14 can embed and register one: `Application::font(impl Into<Cow<'static, [u8]>>)`
   and `Application::default_font(Font)` (`iced-0.14.0/src/application.rs:234,245`), with
   `iced_winit` calling `compositor.load_font` per entry at startup
   (`iced_winit-0.14.0/src/lib.rs:602`) and `Compositor::load_font`'s default body loading
   the process-wide `iced_graphics::text::font_system()`
   (`iced_graphics-0.14.0/src/compositor.rs:69`) — the same system the headless renderer
   resolves against, so tests can load the same bytes.
3. Embedding was proven end to end with a throwaway test in `src/ui/layout_capture.rs`:
   Silkscreen's bytes loaded through
   `iced::advanced::graphics::text::font_system().write().load_font(...)`, then
   `text("NOSTALGIA").font(Font::with_name("Silkscreen")).size(16.0)` rendered through
   `headless_renderer()` and captured. Result: a 100×20.8 px fragment with 4 distinct
   colors (transparent, the pure fill, and two partial-alpha edge rows from the fractional
   line position). The same probe at sizes 14 and 18 produced 15–18 blended colors: an
   off-grid size blurs.
4. Candidate faces, each measured by parsing the shipped TTF and rendering it through that
   same stack:
   - **Silkscreen** — OFL-1.1, © 2001 The Silkscreen Project Authors
     (github.com/googlefonts/silkscreen); Regular 32 KB, Bold 31 KB; 226 codepoints; 5-px
     caps on an 8-px em (`unitsPerEm` 1000, 125-unit stroke, advances 375/750/875), so
     16/24/32 px are crisp (4 colors, 2 partial rows at both 16 and 32). In a side-by-side
     render against `winamp-2.x-stack.png`'s title text its squared terminals and stepped
     S/G/3 were the closest of the candidates, and its per-character advance at size 16
     (12 px for most glyphs, 14 px for the wide ones) is in the same range as the base
     skin's 5-px cell doubled to the app's ~2× chrome scale. **The recommendation.**
   - **Tiny5** — OFL-1.1, © 2022–2024 The Tiny5 Project Authors; 433 KB; 1940 codepoints;
     a variable font. 5-px caps on an 8-px em; the *only* candidate that rendered perfectly
     crisp (2 colors, 0 partial rows at 16 and 32 px) and the only one that covers every
     glyph the UI draws, including ▶ ◀ ■ ✕. The alternative if the glyph work in step 4 is
     unwanted, at 13× the bytes and a narrower rhythm.
   - **m5x7** — CC0-1.0, Daniel Linssen; 34 KB; 319 codepoints; 5×7 on a 16-px em; rounder
     letterforms; the runner-up and the license-cleanest choice.
   - **DSEG7 Classic** — OFL-1.1, © 2020 keshikan, Reserved Font Name "DSEG"; 23 KB —
     **rejected for the LCD readout**: through the same stack it renders 65–125 distinct
     colors (blurred) at 16 and 32 px, because its digit advance is 816/1000 em
     (6.528 design px), so every digit lands on a fractional x.
5. Coverage gaps, read from each font's cmap: the UI draws ▶ ◀ ■ ✕ ⚡ ▭ and – in string
   literals (`views/transport.rs`'s `transport_glyphs()` and its lightning button,
   `views/title_bar.rs`'s shade/minimize/close buttons, `views/now_playing.rs`'s transport
   glyph, `browse/rows/text.rs`'s playing marker). Silkscreen lacks ▶ ◀ ■ ✕ ⚡ ▭; Tiny5
   lacks only ⚡ ▭; m5x7 lacks those plus – … •.
6. Text sizes in use: `LCD_INDICATOR_SIZE` 10, `LCD_STATUS_SIZE` 12,
   `LCD_TRANSPORT_SIZE` 22, `LCD_ELAPSED_SIZE` 34, plus inline 12/14/18/20 sizes. The
   face's grid is 8 px, so only multiples of 8 are crisp.

**Approach.**

1. Vendor the face: `assets/fonts/Silkscreen-Regular.ttf` (and `-Bold.ttf` if a bold weight
   is used) with `assets/fonts/OFL.txt` — the license text must travel with the font — and
   an `assets/fonts/README.md` naming each file's source URL, copyright line, and license.
   `src/ui/theme.rs` gets the family beside the palette constants (e.g.
   `pub const FONT_FAMILY: &str = "Silkscreen";` and a `pub fn font(weight: Weight) ->
   iced::Font` helper) so no view spells the family as a literal.
2. `src/ui/mod.rs`: on the `iced::application(...)` builder, beside `.theme(...)`, add
   `.font(include_bytes!("../../assets/fonts/Silkscreen-Regular.ttf"))` (plus the Bold
   bytes if used) and `.default_font(theme::font(Weight::Normal))`.
3. Re-base every text size onto the 8-px grid: 10→8, 12→16, 14→16, 18→16, 20→24, 22→24,
   34→32, choosing per element so its row still fits; keep each control's box height where
   the new glyphs fit it and adjust the row/panel heights where they do not, re-measuring
   the geometry the tests and `docs/screenshots/`'s proportions pin.
4. Glyph coverage for the six shape glyphs Silkscreen lacks. Prefer drawing them as chrome
   — the repo already draws bevels and slider thumbs by hand, and Winamp's own buttons are
   bitmaps, not glyphs — starting with the transport row's ◀◀ ▶ II ■ ▶▶ and the title
   bar's ▭/–/✕; for the purely decorative ones (the options ⚡, the browse rows' playing ▶
   marker) a covered substitute is acceptable. Either way the strings change in one place
   per control (`transport_glyphs()`, `title_bar.rs`, `browse/rows/text.rs`) and their
   tests pin the new values.
5. Keep the test renderer on the shipped face: `layout_capture::headless_renderer()` loads
   the same bytes into the global `font_system()` once (a `OnceLock`-guarded helper) and
   passes `theme::font(Weight::Normal)` as its default font, so every layout test measures
   the face the app ships instead of the system sans.
6. Re-derive the text-bound assertions: 201 call sites across the 17 test modules that
   touch `layout_capture` read captured text bounds (`captured_root_view_text_bounds`,
   `capture_text`/`capture_text_and_pixels`, `TextCapture::find`, `texts()`), and the
   numbers move with the face. Re-derive rather than re-pin: a test whose claim is a
   relationship (a gap between neighbouring labels, a row that does not reflow between
   states, one control left of another) keeps that claim and takes its new numbers from the
   run; a test that pinned nothing but the old font's metrics is deleted, with the reason
   in the change's summary.
7. Docs, same run: `README.md` names the embedded face and its license beside the
   screenshot section, and the change's summary names the reference screenshot compared,
   per README's rule.

**Acceptance criteria.**

- Every text size the UI hands iced is a multiple of 8 (a unit test over the size constants
  and the inline sites).
- `headless_renderer()`'s default font is the embedded face, and a test renders a label
  through it and asserts the captured fragment's width equals the face's advance for that
  string (`NOSTALGIA` at size 16 is 100 px, the number the probe measured), so losing the
  font fails a test rather than only a screenshot.
- The transport row, the title bar, and a song row render with no fallback face: a capture
  test per control pins the glyph-bearing labels' fragment widths, reviewed against the
  face's metrics when they land.
- A 1:1 screenshot of the player section (`WINIT_X11_SCALE_FACTOR=1`) shows pixel text; the
  change's summary records the comparison against `winamp-screenshots/winamp-2.x-stack.png`,
  and `docs/screenshots/`'s captures are refreshed if the text's metrics move the chrome's
  look.
- `make check` passes.

**Size.** Two vendored TTFs (~63 KB) with their license and README; `src/ui/mod.rs`,
`src/ui/theme.rs`, `src/ui/layout_capture.rs`, and every module that passes a text size —
16 files under `src/ui/views/` plus `src/ui/bevel.rs` and `src/ui/view/options_shortcut.rs`
— plus the re-derived assertions across the 17 test modules that read captured text. No
dependency added — the font is bytes, not a crate. Out of scope: the LCD readout's
7-segment face (`numbers.bmp`), which needs its own plan because the one candidate face
measured (DSEG7 Classic) cannot render crisply through iced's stack; and the chrome palette
differences BUGS.md's reference pass filed, which the face does not touch.


## Done
