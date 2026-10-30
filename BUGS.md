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
