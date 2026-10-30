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

### The transport row omits the base skin's Eject button (found by the reference-comparison pass 2026-10-29)

**Symptom.** The app's transport row is Previous, Play, Pause, Stop, Next, then the LED-marked
Repeat and Shuffle toggles, then the volume and balance sliders, `EQ`, `PL`, and the lightning
button. The base skin's row is Previous, Play, Pause, Stop, Next, **Eject**, then a `SHUFFLE`
button and a Repeat icon button, each with a green LED dot — `winamp-screenshots/README.md`'s
2.95 capture and the repo's own mockup (`docs/screenshots/player.png`) both show that row, with
the eject button exactly between Next and Shuffle. The app has no Eject control at all. This
entry's LED half (the toggles carrying their on/off state in label text instead of a square LED)
was split off and fixed the same day — see the entry under `## Fixed`.

**How to reproduce.** Run the app and read the transport row left to right (or compare
`docs/screenshots/player.png`'s transport row with a fresh capture): no Eject glyph appears
between Next and Repeat.

**Suspected cause.** The transport row's control list (`src/ui/views/transport.rs`, whose
`transport_glyphs()` holds only the five playback glyphs, plus the Repeat/Shuffle LED toggles
beside it) was built from the app's own feature list and never carried the base skin's Eject
button, which has no behavior to wire to: Winamp's Eject opens Add File(s), and this app plays
the Apple Music library and the sample library only.

**Direction.** Waiting on the open question "What should the transport row's Eject button do
when it is added?" (posted by bugfix 2026-10-30 in `QUESTIONS.md`); add the button only once
that is answered, so a control never ships claiming behavior it does not have.


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
