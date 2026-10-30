# nostalgia

A (very close if not pixel-perfect) clone of the classic Winamp UI, built in Rust, using Apple
Music as the music library.

## Initial prompt

<!-- tumwater:prompt:start -->
Nostalgia is a (very close if not pixel-perfect) clone of the classic Winamp UI, but built in Rust, and using Apple Music as the music library.

The UI to clone is the classic Winamp 2.x base skin. Reference screenshots of the real thing —
Winamp 0.2a through 2.95 — live in `winamp-screenshots/`, indexed with their sources in
`winamp-screenshots/README.md`. When a change touches anything visible (a window, panel, button,
readout, color, or layout), compare the result against the closest reference screenshot and match
it; name the screenshot you compared against in the change's summary. `docs/screenshots/` holds
Nostalgia's own captures, not the target.
<!-- tumwater:prompt:end -->

## Status

<!-- tumwater:status:start -->
Early skeleton (0.1.0): an iced (0.14) desktop clone of the Winamp 2.x UI, with a themed player
(Now Playing LCD, transport, volume and balance, equalizer) and an artist → album → song browser
with library search and flat All Songs, All Albums, Playlists, Genres, Years, Composers, Recently
Added, and Recently Played views. Apple Music browse/search and preview playback sit behind injectable seams and fall back to
the sample library and silence.
Open work lives in PLANS.md, BUGS.md, and QUESTIONS.md.
<!-- tumwater:status:end -->

## Screenshots

Design mockups of the look Nostalgia is aiming for, matched against the classic Winamp 2.x base
skin and shown with the built-in sample library. The running app builds the transport controls
(Repeat included), the equalizer panel (preset curves included), and window-shade mode.

![Main window, equalizer and library window docked together](docs/screenshots/hero.png)

Transport states — playing, paused, stopped, and Repeat on:

![Main window in playing, paused, stopped and repeat states](docs/screenshots/transport.png)

The playlist editor as a library browser, drilling from Artists to Albums to Songs:

![Library window showing artists, albums and songs](docs/screenshots/library.png)

The equalizer with presets, plus window-shade mode:

![Equalizer with flat and rock presets, and windows rolled up into shade mode](docs/screenshots/equalizer.png)

The player on its own: [docs/screenshots/player.png](docs/screenshots/player.png).

Everything above is Nostalgia's own output. The target it is measured against is a set of
reference captures of the real thing in
[winamp-screenshots/](winamp-screenshots/README.md) — classic Winamp from 0.2a through 2.95,
indexed by window and version with their sources:

![Classic Winamp 2.95 with its main window, equalizer and playlist](winamp-screenshots/winamp-2.95-stack.png)

![Classic Winamp 2.05 main window](winamp-screenshots/winamp-2.05-main.jpg)

The chrome's text renders in two embedded pixel faces: [Silkscreen](https://fonts.google.com/specimen/Silkscreen)
for the letterforms and [Tiny5](https://fonts.google.com/specimen/Tiny5) for
shape-glyph markers such as ▶ and II. Both are SIL Open Font License 1.1 (the
full record — sources, copyright and the vendored bytes — lives in
[assets/fonts/README.md](assets/fonts/README.md)).

When a change touches anything visible, compare it against the closest of those captures (the
windowshade strip, the docked stack, the equalizer states) before calling it done.

## Usage

Build and run with `cargo run`; the player opens an iced window with the Winamp-style Now
Playing bar, transport, volume and balance, equalizer, and artist → album → song library
browser, with repeat and shuffle. Apple Music browse/search and preview playback sit behind
injectable seams that fall back to the built-in sample library and silence until a MusicKit
sign-in starts.

[docs/usage.md](docs/usage.md) is the full control reference: each part of the window, the
keyboard and mouse bindings, the settings file, and how sign-in and playback work.

## Development

`make check` runs the whole landing gate in one command — `cargo fmt --check`, `cargo clippy
--all-targets -- -D warnings`, a `cargo doc` build that fails on any rustdoc warning, the test
suite, and a self-check of `make test-one` — and is the fastest way to see whether a change is merge-ready. The individual stages are
available separately (`make fmt`, `make lint`, `make docs`, `make test`); `make test-one
TEST=<name>` runs only the tests whose names contain `<name>`, for a fast loop while iterating on
one behavior; `make fix` applies formatting and clippy's machine-applicable fixes in place, and
`make run` is the same as `cargo run`.

`docs/invariants.md` records each subsystem's guarantees, the code that enforces them, and the
tests that pin them; update a section when a change alters its guarantee.

`winamp-screenshots/` is the visual ground truth for UI work: real Winamp captures from 0.2a
through 2.95, indexed by window and version in
[winamp-screenshots/README.md](winamp-screenshots/README.md). Compare against them when
changing anything visible.

On Linux the build links against the system audio library, so install the ALSA
development headers first — `libasound2-dev` on Debian/Ubuntu, `alsa-lib-devel`
on Fedora — or `rodio`'s `cpal` backend fails to build. CI
(`.github/workflows/ci.yml`) installs them and runs `make check` on every push
and pull request.
