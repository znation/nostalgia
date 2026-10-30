# Plans

## Planned

Four sub-plans, split 2026-10-30 from the single pixel-font plan after the feature
role's **too large for one run** note (16 view modules plus 17 test modules landing
together was one run's worth of re-derived assertions too many). They cross-reference;
parts 2–4 each scope whole controls — a control's face, size and glyphs never land in
different parts, so nothing is left half-converted between merges.

### Re-base the browse list and equalizer text onto the pixel faces (part 3/4 of the pixel-font split, requires part 1/4 landed) (planned 2026-10-30 by the plan role)

**Goal.** The browse list's rows and footer and the equalizer's labels render in
the vendored faces at on-grid sizes, matching the playlist-editor text of
`winamp-screenshots/winamp-2.x-stack.png`. Independent of parts 2/4 and 4/4 except
for part 1/4's seam, so a runner can take it alongside part 2/4.

**Files touched.** `src/ui/views/browse/rows.rs`, `src/ui/views/equalizer.rs`,
plus the test modules that capture their fragments (`views/tests/browse.rs`,
`equalizer.rs`, and the `src/ui/tests/` readouts/browse modules that read their
text bounds).

**Approach.**

1. Face swap: every `Text` in `rows.rs` and the equalizer's labels takes
   `theme::ui_font()`; the playing row's `▶` prefix marker (`rows.rs`'s
   `row_prefix`) takes `theme::shape_font()` (Tiny5 covers `▶`, probed crisp;
   Silkscreen does not).
2. Size re-base onto the grid, per the split's mapping (18→16, 14→16, 12→16):
   `rows.rs`'s empty-label 18, row-prefix 18, and row label 18 → 16; the song-row
   label 14 and footer 14 → 16; the equalizer's two label sites and the
   `.text_size(12)` style → 16. Keep each row's fixed heights where the new
   glyphs fit them; where a row overflows, re-derive the height from the run and
   note the change (16px text in the current row boxes is expected to fit
   because the rows already leave padding, but the tests decide).
3. Re-derive the affected text-bound assertions as in part 2/4: relationships
   keep their claims with re-measured numbers; metric-only pins re-measured.
4. Compare against `winamp-screenshots/winamp-2.x-stack.png`'s playlist editor;
   the summary names the comparison.

**Acceptance criteria.**

- Every text size `rows.rs`/`equalizer.rs` hand iced is a multiple of 8, pinned
  by tests.
- A capture test proves the face: a browse row label draws with Silkscreen's
  advance and the playing `▶` marker with Tiny5's probed 8.0 px width at 16.
- The playing-row marker swap keeps `row_prefix`'s pinned behavior tests
  passing; `make check` passes.

### Re-base the remaining panels, make the face the app's default, and sync the docs (part 4/4 of the pixel-font split, requires part 1/4 landed, requires part 2/4 landed, requires part 3/4 landed) (planned 2026-10-30 by the plan role)

**Goal.** The small panels (about, file info, jump-to-file, jump-to-time, options,
save list, save preset) render in the face at on-grid sizes; the face becomes the
app-wide default so no future text can silently fall back to the system sans; the
docs record the embedded faces and licenses. Requires the earlier parts because the
default-font switch re-styles every text that has no explicit font, which must not
land while any scoped view is still unconverted.

**Files touched.** `src/ui/views/{about,file_info,jump_file,jump_time,options,
save_list,save_preset}.rs`, `src/ui/mod.rs` (the `.default_font` switch beside the
part 1/4 registration), `src/ui/layout_capture.rs` (the renderer default),
`README.md`, and any test module whose captured fragments move.

**Approach.**

1. Each of the seven panels face-swaps and re-bases its sizes per the split's
   mapping (18→16 here for every site; `jump_file.rs`'s four, others one each).
   The panel headings (`FILE INFO`, `JUMP TO FILE`, …) keep their tape-strip
   styling; compare against `winamp-screenshots/winamp-2.x-stack.png`.
2. `src/ui/mod.rs`: add `.default_font(theme::ui_font())` beside the
   registration. `src/ui/layout_capture.rs`: pass `theme::ui_font()` as
   `headless_renderer()`'s default font so captured tests resolve unlabeled text
   the same way the app now does (the seam this run probed:
   `Headless::new(Font::with_name("Tiny5"), ...)` resolved a `Font::default()`
   text to the named family). After parts 1–3 no view relies on the default, so
   the switch moves no captured number except any site a part missed — a capture
   test for the default (below) is the check.
3. Docs, same part: `README.md` names the two embedded faces and their license
   beside the screenshot section (the license record lives at
   `assets/fonts/README.md`); refresh `docs/screenshots/`'s captures if this
   part's metrics move the chrome's look; update `docs/usage.md` only where a
   control's described behavior changed (none is expected — the faces change
   rendering, not bindings).

**Acceptance criteria.**

- A capture test renders `NOSTALGIA` at size 16 with no `.font()` and pins
  Silkscreen's 100.0 px width, proving the app's default face is the embedded
  one and the test renderer resolves the same; that test is the split's
  end-state proof.
- The seven panels' sizes are multiples of 8, pinned by tests; their captured
  assertions re-derived per part 2/4's rule.
- `README.md` names the faces and `OFL`; `make check` passes.

### Vendor and register the pixel faces the chrome's text will render in (part 1/4 of the pixel-font split) (planned 2026-10-30 by the plan role) — done 2026-10-30 by the feature role

**Goal.** Every label the app draws renders in iced's default face (the system sans;
`grep -rn '\.font(\|Font::' src/` finds none set in production — the only `Font` in
`src/` is `layout_capture.rs`'s `Font::default()`), so the chrome has none of the
bitmap-text look `winamp-screenshots/`'s captures and `docs/screenshots/`'s mockups
both show. This part vendors the faces and builds the seam so the later parts
(parts 2/4, 3/4, 4/4) can adopt them control by control; it changes nothing visible.

**Ground truth verified this run (2026-10-30) through `layout_capture::headless_renderer()`.**

1. iced 0.14 registers embedded bytes via `iced::application(...).font(impl
   Into<Cow<'static, [u8]>>)` (`iced-0.14.0/src/application.rs:234`), resolves them
   app-side through `Compositor::load_font` into the process-wide
   `iced_graphics::text::font_system()` (`iced_graphics-0.14.0/src/compositor.rs:69`)
   — the same system the headless test renderer uses, so tests see the shipped faces.
2. The renderer's default-font parameter is the same seam `.default_font` uses:
   `Headless::new(Font::with_name("Tiny5"), ...)` makes a `Font::default()` text
   resolve to Tiny5 (probed: `▶` without `.font()` drew width 8.0, crisp) — part
   4/4 builds its catch-all on this.
3. Per-glyph fallback exists and never draws tofu: a glyph the named face lacks
   falls back to a system face (probed: `▶` with `Font::with_name("Silkscreen")`
   drew the fallback's anti-aliased run, width 12.3, 24 colors). Adopting the faces
   cannot blank a glyph; the shape-glyph swaps below, done for crispness, are not
   forced by tofu risk.
4. On the 8-px grid (only multiples of 8 are crisp — sizes 14/18 produced 15–18
   blended colors in the earlier run's probe), the vendored faces measure:
   Silkscreen `H` = 12.0 px advance, `NOSTALGIA` = 100 px at size 16, at most 4
   distinct colors; Tiny5 `▶`=8.0, `◀`=8.0, `■`=10.0, `✕`=12.0, `–`=10.0, `II`=8.0,
   `□`(U+25A1)=10.0, `≡`(U+2261)=8.0 — each exactly 2 colors. Neither face renders
   `▭` or `⚡`; the parts that draw those buttons substitute `□` and `≡`.
5. Silkscreen kept as the letterform face (squared terminals were the closest
   side-by-side against `winamp-screenshots/winamp-2.x-stack.png`), Tiny5 as the
   shape-glyph face: its covered shapes are crisp where Silkscreen's `▶`/`■`/`✕`
   are not, and adopting it for the few glyph strings avoids any hand-drawn-chrome
   glyph work.

**The bytes are already vendored.** `assets/fonts/{Silkscreen-Regular.ttf,Tiny5.ttf,
OFL.txt,README.md}` landed with this split (README.md records source URLs, copyright
lines, family names, and the probed crispness numbers). Do not re-download; the
implementation has no network dependency.

**Approach.**

1. `src/ui/theme.rs`: beside the palette constants, add
   `pub const UI_FONT_FAMILY: &str = "Silkscreen";`,
   `pub const SHAPE_FONT_FAMILY: &str = "Tiny5";` and the pure helpers
   `pub fn ui_font() -> iced::Font { iced::Font::with_name(UI_FONT_FAMILY) }` and
   `pub fn shape_font() -> iced::Font { iced::Font::with_name(SHAPE_FONT_FAMILY) }`,
   so no view spells a family as a literal.
2. `src/ui/mod.rs`, on the `iced::application(...)` builder beside `.theme(...)`
   (~line 87): `.font(include_bytes!("../../assets/fonts/Silkscreen-Regular.ttf").into())`
   and the Tiny5 bytes beside it (`.font` takes `impl Into<Cow<'static, [u8]>>`,
   which `include_bytes!(...).into()` satisfies; paths in `include_bytes!` are
   file-relative).
   No `.default_font` here — part 4/4 switches the default so intermediate parts
   never re-style unconverted text implicitly.
3. `src/ui/layout_capture.rs`: inside `headless_renderer()`, behind a
   `OnceLock`-guarded helper, load both files' bytes into the global
   `font_system()` the way this run's probe did
   (`iced::advanced::graphics::text::font_system().write().expect(...).load_font(...)`),
   so every capture test can address either face with `Font::with_name` without a
   per-test load, while `Font::default()` texts keep resolving to the system face
   (probed: name-less resolution is unaffected by registration).
4. A seam test in `layout_capture.rs`'s test module proves both faces load through
   `headless_renderer()` and render their probed runs (criteria below).

**Acceptance criteria.**

- A seam test renders `NOSTALGIA` at size 16 with `theme::ui_font()` through
  `headless_renderer()` and pins the fragment width at 100.0, and renders `▶` at
  size 16 with `theme::shape_font()` pinning width 8.0 and exactly 2 distinct
  colors in the pixel capture — losing the vendored bytes or the registration
  fails a test, not just a screenshot.
- No existing test's captured numbers move: registration alone does not re-style
  name-less text (probed), and no view sets a font yet.
- `make check` passes; the summary notes nothing visible changed by design.

**Size.** Two vendored TTFs (already in repo) plus `src/ui/theme.rs`,
`src/ui/mod.rs`, `src/ui/layout_capture.rs` and one test — a few dozen lines. No
new crate; the faces are bytes, not a dependency. Out of scope: a Bold weight
(no view requests weight), and the LCD readout's 7-segment face (`numbers.bmp` —
DSEG7 Classic measured blurred through this stack in the earlier probe; needs its
own plan).

## Done
