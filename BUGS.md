# Bugs

Known bugs, found by any loop or the review gate. Each bug: symptom, how to
reproduce, suspected cause. Move fixed bugs to Fixed.

Each fix records its **Validation gap** in its Fixed entry. When that gap is
`unclear-invariant`, the reconstructed guarantee belongs in `docs/invariants.md`
(see its "How to use" note) as a named section with its enforcing code and tests,
not only in the Fixed entry.

## Open

Entries below come from a reference-comparison pass (2026-10-29) that captured the
running app under X11/Xwayland at 1:1 scale (`WINIT_X11_SCALE_FACTOR=1`, so the
default 1024×768 window's pixels are logical pixels) and sampled it against the
ground-truth captures in `winamp-screenshots/`. Each entry names the app pixel, the
reference file and pixel, and the measured values on both sides. Reference sample
coordinates in `winamp-screenshots/winamp-2.x-stack.png` are relative to that file:
its main window occupies y 0..115, the equalizer y 116..231, and the playlist
y 232..347.

### The whole chrome renders neutral gray where the base skin — and the repo's own mockups — are steel blue (found by the reference-comparison pass 2026-10-29)

**Symptom.** Every chrome surface the app paints is a neutral gray: the window face is
`#2E2E2E` (46,46,46), the wells are `#0D0D0D` (13,13,13), the button faces `#4C4C4C`
(76,76,76), and the bevels `#8C8C8C` (140,140,140) and `#0D0D0D`. The base skin's chrome
is steel blue: the face is `#181829` (24,24,41), the equalizer panel `#31315A` (49,49,90),
the bevel highlights are blue-tinted (66,82,90) and (140,156,181), and the outer edge is
a near-black (0,0,8) outline *outside* the light bevel. The repo's own mockups
(`docs/screenshots/player.png`) are steel blue too: face (53,53,84)/(56,56,88), bevel
highlight (98,98,118) — so the app matches neither the target nor its own design.

**How to reproduce.** Run the app (`cargo run`; the window opens 1024×768 at 1:1), capture
it with any screenshot tool, and sample: the window face at (600,120) reads (46,46,46);
the top bevel at (200,0) reads (140,140,140). Against the reference: `winamp-2.x-stack.png`
(3,30) reads (24,24,41), (0,30) reads (0,0,8), (1,30) reads (66,82,90), (271,13) reads
(140,156,181), and its equalizer panel (20,180) reads (49,49,90). `docs/screenshots/player.png`
(5,20) reads (53,53,84).

**Suspected cause.** `src/ui/theme.rs`'s palette constants are documented as the Winamp
2.x base skin (`WINDOW_BACKGROUND` "The dark gray window face", the module doc "The
Winamp 2.x base-skin palette") but hold neutral grays: `WINDOW_BACKGROUND =
Color::from_rgb(0.18, 0.18, 0.18)`, `LCD_BACKGROUND = Color::from_rgb(0.05, 0.05, 0.05)`,
`BUTTON_FACE = Color::from_rgb(0.30, 0.30, 0.30)`, `PANEL_EDGE_LIGHT =
Color::from_rgb(0.55, 0.55, 0.55)`, `PANEL_EDGE_DARK = Color::from_rgb(0.05, 0.05, 0.05)`.
No constant in the module carries the base skin's (24,24,41)/(49,49,74) blues, and the
window's outer edge is drawn as a light bevel only, with no dark outline outside it.

**Direction.** Re-sample the base-skin faces and bevels from `winamp-screenshots/` (or
from the mockups, which agree on hue) and replace the gray constants with them; keep the
two-part outer border (dark outline outside the light bevel).

### Transport buttons draw dark faces with light glyphs, inverting the base skin's light metal buttons (found by the reference-comparison pass 2026-10-29)

**Symptom.** Every transport-row button (Previous, Play, Pause, Stop, Next, Repeat,
Shuffle, EQ, PL) is a dark `#4C4C4C` (76,76,76) tile with a light `#DEDEDE` (222,222,222)
glyph. The base skin's buttons are light metal — faces (189,198,198)/(173,181,181)/
(198,206,206) — with the glyph engraved dark and a white highlight; the repo's mockup
(`docs/screenshots/player.png`) draws them the same way (light face, dark glyph). The
app's rendering reads as the photographic negative of both.

**How to reproduce.** Run the app and sample the transport row: the Previous button's
face at (5,137) reads (76,76,76) and its glyph at (14,147) reads (222,222,222). In
`winamp-2.x-stack.png` the same button (x 16..40, y 88..103) has face pixels
(189,198,198)/(173,181,181) and near-black glyph pixels with (239,255,255) highlights.

**Suspected cause.** `theme.rs`'s `BUTTON_FACE = Color::from_rgb(0.30, 0.30, 0.30)`
(plus `BUTTON_FACE_HOVERED`/`BUTTON_FACE_PRESSED`) is the gray that every button style
fills with, and the button text/glyph styles use the light `TEXT`/`PLAYING_ROW_TEXT`
colors, so the light-face/dark-glyph convention was never encoded.

### The transport row omits the base skin's Eject button and its LED-marked Shuffle/Repeat buttons (found by the reference-comparison pass 2026-10-29)

**Symptom.** The app's transport row is Previous, Play, Pause, Stop, Next, then two wide
text toggles reading `Repeat: Off` and `Shuffle: Off`, then the volume and balance
sliders, `EQ`, `PL`, and the lightning button. The base skin's row is Previous, Play,
Pause, Stop, Next, **Eject**, then a `SHUFFLE` button and a Repeat icon button, each with
a green LED dot; `winamp-screenshots/README.md`'s 2.95 capture and the repo's own mockup
(`docs/screenshots/player.png`) both show that row. The app has no Eject control at all,
and its Repeat/Shuffle state is carried by the label text rather than an indicator LED.

**How to reproduce.** Run the app and read the transport row left to right (or compare
`docs/screenshots/player.png`'s transport row with a fresh capture): no Eject glyph
appears between Next and Repeat, and Repeat/Shuffle render as ~340 px wide text buttons
at a 1024 px window width (measured at y 133..162: buttons at x 0..34, 54..84, 104..134,
154..184, 204..234, then the text toggles from x 258).

**Suspected cause.** The transport row's control list (`src/ui/views/transport.rs`, whose
`transport_glyphs()` holds only the five playback glyphs, plus the Repeat/Shuffle text
toggles beside it) was built from the app's own feature list — Repeat/Shuffle as text
toggles are documented in `docs/usage.md` — and never carried the base skin's Eject button
or per-toggle LED indicators.

### The title band is 23 px tall, bright blue, with long tan tapes; the base skin's title bar is 12 px, dark navy, with white text (found by the reference-comparison pass 2026-10-29)

**Symptom.** Each section's band is 23 px tall (y 0..22 in the player section), filled
`#00008C` (0,0,140), with a 7 px gold tape `#EDCE7A` (237,206,122) running x 24..442 and
x 533..951 and a light-gray (222,222,222) label drawn on the blue between the tapes. The
base skin's title bar is 12 px tall (y 0..11), filled (24,24,41)/(33,33,49), with a 5 px
gold tape (225,222,117) running x 21..110 and x 159..238 and **white** (255,255,255) text
between them; the tape's yellow is greener than the app's tan.

**How to reproduce.** Sample the running app's band: (600,5) reads (0,0,140), (600,12)
reads (237,206,122), and the band's last row is y=22. In `winamp-2.x-stack.png` the title
bar ends at y=11, (60,3) reads (33,33,49), the tape at (40,6) reads (225,222,117), and the
title text at (120,6) reads (255,255,255).

**Suspected cause.** `theme.rs`'s `TITLE_BLUE = Color::from_rgb(0.0, 0.0, 0.55)`,
`TITLE_TAPE = Color::from_rgb(0.93, 0.81, 0.48)`, `TITLE_TAPE_SHADOW`, and
`src/ui/views/title_bar.rs`'s `TITLE_BAR_HEIGHT` (24 px, of which the band's own pixels are
23) with `title_tape()`'s 8 px tape: the band was drawn from the mockups' proportions
rather than the base skin's 12 px bar, and its label uses `TEXT` rather than the base
skin's white.

### The display well is a flat near-black field; the base skin's display area is face-colored with black insets (found by the reference-comparison pass 2026-10-29)

**Symptom.** The app paints one uniform `#0D0D0D` (13,13,13) field behind the whole
display area (indicator letters, elapsed readout, title, status row, visualizer). The base
skin's display area is the window face color (24,24,41) with black (0,0,0) inset regions
(the visualizer and time blocks) and (0,0,16) dark edging, so its display reads as
insets cut into the face rather than one black panel.

**How to reproduce.** Sample the app's display area: (300,30), (300,60) and (300,100) all
read (13,13,13). In `winamp-2.x-stack.png` the same area reads the face color —
(16,20) and (30,40) read (24,24,41) — with (0,0,0) appearing only inside the visualizer
and readout blocks.

**Suspected cause.** `theme.rs`'s `LCD_BACKGROUND = Color::from_rgb(0.05, 0.05, 0.05)`
used as the single well fill behind the whole display area. The browse list's well reuses
the same fill: its rows sit on (13,13,13) where the base skin's playlist editor sits on
pure black — `winamp-2.x-stack.png` (200,300) reads (0,0,0).

### The LCD and marquee green is pure `#00FF00`; the base skin's green is `#00C600` (found by the reference-comparison pass 2026-10-29)

**Symptom.** The app's green — the Now Playing title, browse-list row text, footer
readouts — is `#00FF00` (0,255,0). The base skin's ordinary green is (0,198,0) with a
(0,214,0) lighter stop: in `winamp-2.x-stack.png` every playlist row glyph outside the
current track is (0,198,0), and only the currently playing row uses (0,255,0); the shade
strip's marquee glyphs in `winamp-2.05-windowshade.gif` are (0,198,0) as well. The app
uses the bright green everywhere instead of reserving it for the current row.

**How to reproduce.** Sample the app's Now Playing title (e.g. (57,45) of the player
window) or a song row (e.g. (100,563) with All Songs open): both read (0,255,0). In
`winamp-2.x-stack.png` a playlist row glyph (28,254) reads (0,198,0), as do the shade
strip's marquee glyphs in `winamp-2.05-windowshade.gif`.

**Suspected cause.** `theme.rs`'s `LCD_GREEN = Color::from_rgb(0.0, 1.0, 0.0)`; the
mockups' brighter green (0,248,0) was taken as the base skin's, which is a stop dimmer.

### The browse-list selection bar is light sky blue; the playlist editor's is a deep saturated blue (found by the reference-comparison pass 2026-10-29)

**Symptom.** A clicked row's bar is `#4080FF` (64,128,255) — a bright sky blue with the
row's text turned white. The base skin's playlist selection bar is `#0000BC` (0,0,188),
a deep blue; the repo's mockup (`docs/screenshots/library.png`) uses (0,0,198), agreeing
with the reference. The app's bar is visibly lighter and more saturated than both.

**How to reproduce.** Open All Songs in the running app, click any row, and sample the
bar: (300,563) reads (64,128,255). In `winamp-2.x-stack.png` the playlist's selection bar
(100,261) reads (0,0,188); in `docs/screenshots/library.png` the selection pixels are
(0,0,198).

**Suspected cause.** `theme.rs`'s `ROW_HIGHLIGHT = Color::from_rgb(0.25, 0.5, 1.0)`,
documented as "echoing Winamp's playlist selection bar (a saturated blue)" — the value is
a light sky blue, not the bar's deep blue.

### The rolled-up shade strip renders 52 px tall; the base skin's is 14 px (found by the reference-comparison pass 2026-10-29)

**Symptom.** `Ctrl+W` (or the title bar's shade button) rolls the window into a 52 px
tall strip that keeps the full-height blue title band (23 px) plus a second blue row
(y 24..51). The base skin's rolled-up strip is a single 14 px bar
(`winamp-2.05-windowshade.gif`, whose two docked strips are each 275×14).

**How to reproduce.** Launch the app, press Ctrl+W, and measure the window: it is
1024×52 (captured as such for this pass). In `winamp-2.05-windowshade.gif` each rolled-up
window is 14 px tall, with the marquee, time, and volume controls all inside that height.

**Suspected cause.** `src/ui/views/shade.rs`'s `SHADED_HEIGHT = TITLE_BAR_HEIGHT +
SHADE_STRIP_HEIGHT` (24 + 28 px) and its strip's `style::title_bar_style()` chrome: the
strip was built at the app's own chrome scale — roughly twice the base skin's — rather
than the base skin's single 14 px bar, and it paints a full 28 px blue band where the base
skin keeps its controls on the one bar.

## Fixed
