# Bugs

Known bugs, found by any loop or the review gate. Each bug: symptom, how to
reproduce, suspected cause. Move fixed bugs to Fixed.

Each fix records its **Validation gap** in its Fixed entry. When that gap is
`unclear-invariant`, the reconstructed guarantee belongs in `docs/invariants.md`
(see its "How to use" note) as a named section with its enforcing code and tests,
not only in the Fixed entry.

## Open

### Sort renders below Top instead of beside it, contradicting the documented layout (found by qa 2026-10-23)

Symptom: the README's Usage section says "A `Sort` button beside `Top` reorders the showing
list", and the Sort invariant in `docs/invariants.md` says `views::view_sort_button` "draws
the button beside Top", but on the Artists list the two buttons render in consecutive rows at
the left edge, one above the other. `src/ui/mod.rs` pushes each button into the root `Column`
separately (`view_top_button()` around line 737, `view_sort_button()` around line 739), so no
width ever places them side by side. This is a layout-only divergence: the buttons are both
visible and labelled.

How to reproduce (X11/Xwayland, `DISPLAY=:1`, `WAYLAND_DISPLAY` unset):

1. `cargo build --locked` and run `target/debug/nostalgia` from a scratch directory, with the
   display exported. The app opens one window titled `nostalgia`; at the 1.5x scale of this
   machine it is 1536x1152 at root +1152+471 (window id `0x1600002` in this run).
2. Capture it: `DISPLAY=:1 xwd -id 0x1600002 -out app.xwd && convert app.xwd app.png`.
3. Inspect the left edge above the `Search library` box: `Top` and `Sort` occupy the same
   narrow x range at the far left (roughly x 0-73 image pixels) in consecutive ~40-pixel-tall
   rows, Top roughly y 440-480 and Sort roughly y 480-520 (image pixels; divide by 1.5 for
   logical layout pixels). Both share the same left edge because each is its own row.

Expected: `Top` and `Sort` on the same row, Sort immediately to the right of Top, as the README
and the invariant's "beside Top" wording promise.

Actual: Sort is the next row below Top, and `All Songs` is the row below that; every browse
control stacks vertically at the left edge.

Suspected cause: the root `Column` in `src/ui/mod.rs` gets the two buttons as two separate
`.push()` calls, and a `Column` stacks its children. Wrapping `view_top_button()` and
`view_sort_button()` in one `Row` before pushing it would render them beside each other.

## Fixed
