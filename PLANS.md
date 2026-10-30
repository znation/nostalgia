# Plans

## Planned

Four sub-plans, split 2026-10-30 from the single pixel-font plan after the feature
role's **too large for one run** note (16 view modules plus 17 test modules landing
together was one run's worth of re-derived assertions too many). They cross-reference;
parts 2–4 each scope whole controls — a control's face, size and glyphs never land in
different parts, so nothing is left half-converted between merges.

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

### Move the browse list's selection bar with Ctrl+Up and Ctrl+Down, like Winamp's playlist editor's selection keys (planned 2026-10-30 by the plan role)

**Goal.** The browse list's selection bar moves only by mouse press
(`BrowseList::select_row`, `src/ui/browse.rs`): the plain up/down arrows nudge the volume
(`shortcut()` in `src/ui/shortcuts.rs`, the plain-arrow arms) and the row's classic move keys
take Alt+Up/Down, so the classic playlist editor's way of moving the selection — the arrow keys
on it and on the selection bar — has no keyboard path. The bound chords the open QUESTIONS.md
planner question surfaced for it — `Ctrl+Up`/`Ctrl+Down`, free today: the Ctrl block in
`shortcut()` guards only the letters F/Z/B/R/W/J/A/T/V and Shift+Delete and Ctrl+F1 before the
`if modifiers.control() … return None` gate at its bottom, so Ctrl+ArrowUp/ArrowDown bind
nothing — move the selection bar one row up and down over the showing list, revealing the moved
row like the row moves do, and answer that question's own recommended option 2. The
question's remaining option ordering stays the user's; this entry does not close it.

**Ground truth verified this run (2026-10-30).** The Ctrl block in
`src/ui/shortcuts.rs::shortcut` matches only `Key::Character` arms and
`Key::Named(key::Named::Delete)`/`F1`; a Ctrl + named-arrow press reaches the bottom gate and
yields `None` (grep for `MoveSelectedRowUp` shows it is claimed only under
`Modifiers::ALT` — see the `assert_maps!` test there). The selection itself lives in
`BrowseList::selected` / `select_row` (`src/ui/browse.rs`), the row moves' reveal goes through
`handlers::browse::scroll::reveal_browse_row` (`src/ui/handlers/browse/scroll.rs`), and the
Alt-move handler this plan mirrors is `handlers::browse::ordering::move_selected_row`
(`src/ui/handlers/browse/ordering.rs`), whose `move_in` helper pair defines the no-op rules:
no selection, an edge repeat, or a loading/failed list change nothing and schedule no task.

**Approach.**

1. `src/ui/browse.rs`: beside `select_row`, add
   `pub(super) fn move_selection(&mut self, down: bool) -> Option<usize>` — a load-time-free
   selection step: `None` when the list is loading (`self.loading`), failed (`self.error` is
   set), or nothing is selected; otherwise the row at `index ± 1` clamped to the list's own
   bounds — the selection stays at an edge and `None` is returned when a step would leave it,
   mirroring `move_in`'s no-op rules — and `Some(new_index)` when a row is newly selected. No
   epoch parameter: the move relative to the selection in the showing list is not a stale
   index-carrying press, and `select_row`'s epoch contract stays for the mouse presses.
2. `src/ui/message.rs`: beside `MoveSelectedRowUp`/`MoveSelectedRowDown`, add
   `MoveBrowseSelectionUp` and `MoveBrowseSelectionDown` with the doc comments naming the
   chords and citing this entry.
3. `src/ui/shortcuts.rs`: inside the existing Ctrl block of `shortcut()`, before the bottom
   gate, `Key::Named(key::Named::ArrowUp)` maps to `Message::MoveBrowseSelectionUp` and
   `ArrowDown` to `Message::MoveBrowseSelectionDown`. A Shift held with the chord maps the
   same way: the arrows carry no shift behavior anywhere today. Update `shortcut()`'s
   doc comment (the one that lists every claimed chord and the volume-nudge arrows, and the
   `/// Ctrl+F is the one Control chord…` note when it says so) to name the two new
   chord claims alongside the Ctrl row moves it lists.
4. `src/ui/handlers/browse/selection.rs`: add `move_browse_selection(player, down)` — the
   `match player.current_view` over the eleven views calling
   `move_selection_in(&mut player.…)` exactly as `ordering::move_selected_row` does, and its
   index-mapped-through `scroll::reveal_browse_row` when the selection changed. Route the two
   new messages from the `Message` update arms in `src/ui/mod.rs` beside the
   `Message::MoveSelectedRowUp =>` arm's neighbors.
5. `docs/usage.md`: extend the keyboard paragraph (the one that runs "R toggles Repeat… and
   the up/down arrows nudge the volume while the left/right arrows seek") with the two
   chords and their no-op rules, beside the row moves' sentence if one names them.
6. `docs/invariants.md`: extend the `## Browse selection` guarantee (the section naming
   `BrowseList::select_row` and the blue `theme::ROW_HIGHLIGHT` bar) with the keyboard path
   and pin the new tests below.

**Acceptance criteria.**

- `make test-one TEST=shortcuts_tests` proves the mapping: `assert_maps!`
  `Key::Named(key::Named::ArrowUp)`/`Modifiers::CONTROL` → `Message::MoveBrowseSelectionUp`
  (and the Down twin), the plain arrows still map to `VolumeUp`/`VolumeDown`, Alt+Up still
  maps to `Message::MoveSelectedRowUp`, and Ctrl+Shift+Up/Down map like plain Ctrl ones.
- A `move_browse_selection` test in `src/ui/tests/` (the row-move module
  `move_row_tests.rs` is the closest anchor) drives a loaded list: the first press selects the
  bar (existing test names cover the mouse path), the chords select the neighbor, the reveal
  task is scheduled exactly once per moved selection (the row-move reveal test is the
  anchor), and the no-op rules above (`move_in`'s pair) schedule no scroll task and leave the
  bar unchanged — an edge repeat, a no-selection list, and a loading or failed list included.
- A `move_selection` boundary test in `src/ui/browse.rs`'s test module documents the clamp
  rule itself (at the first/last row a step returns `None` and keeps the selection).
- `make check` passes.

**Size.** One message pair, one matcher arm pair, one handler with an eleven-arm view
match, one `BrowseList::move_selection`, tests, and three doc anchors — a few dozen lines
each, mirroring the landed Alt-move pair's shape. Out of scope: any other chord for the
selection (the question's own recommendation named only Ctrl+Up/Ctrl+Down), any change to
what Enter or the row moves do, and the parked options 3–6 that question still lists.

## Done
