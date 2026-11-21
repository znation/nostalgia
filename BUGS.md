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

_None yet._

## Fixed
