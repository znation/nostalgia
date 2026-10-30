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



## Fixed
