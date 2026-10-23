# Bugs

Known bugs, found by any loop or the review gate. Each bug: symptom, how to
reproduce, suspected cause. Move fixed bugs to Fixed.

Each fix records its **Validation gap** in its Fixed entry. When that gap is
`unclear-invariant`, the reconstructed guarantee belongs in `docs/invariants.md`
(see its "How to use" note) as a named section with its enforcing code and tests,
not only in the Fixed entry.

## Open

### Equalizer frequency labels run together at the default window size (found by qa 2026-10-23)

Symptom: the equalizer's ten band labels are unreadable as separate frequencies. The row reads
`60 1703106001K 3K 6K 12K14K16K`: `170`, `310`, `600`, and `1K` touch, and so do `12K`, `14K`,
and `16K`. `docs/screenshots/player.png` shows the same ten labels separated under their own
sliders.

How to reproduce (X11/Xwayland, `DISPLAY=:1`, `WAYLAND_DISPLAY` unset):

1. `cargo build --locked` and run `target/debug/nostalgia` from a scratch directory. The app
   opens one window titled `nostalgia`; at this machine's 1.5x scale it is 1536x1152 at root
   +1152+471.
2. Capture the window (`DISPLAY=:1 import -window <id> app.png`; `xwd` also works but its
   frames can lag several seconds behind the app, so use `import` and re-capture before
   measuring).
3. Crop the band-label row, roughly x 0-420, y 458-503 in image pixels:
   `convert app.png -crop 420x45+0+458 +repage -resize 200% labels.png`.

Expected: ten separated labels, `60`, `170`, `310`, `600`, `1K`, `3K`, `6K`, `12K`, `14K`,
`16K`, each readable under its own slider as in `docs/screenshots/player.png`.

Actual: adjacent three-character labels have no visible gap and read as one string,
`1703106001K` and `12K14K16K`.

Suspected cause: `view_equalizer` in `src/ui/views/equalizer.rs` pushes each band as a
`Column` holding a `VerticalSlider` and a size-12 `Text` label into a `Row` with the default
zero spacing and no label width, so the labels touch wherever the text is wider than the
slider column.

**Validation gap:** real-run-needed — the suite has no layout assertion for the equalizer
labels; qa confirmed the collision only from the captured window, so a regression test laying
out the band row with iced's null renderer should pin the separated labels.

## Fixed
