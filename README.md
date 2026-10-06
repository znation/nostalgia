# nostalgia

A (very close if not pixel-perfect) clone of the classic Winamp UI, built in Rust, using Apple
Music as the music library.

## Initial prompt

<!-- tumwater:prompt:start -->
Nostalgia is a (very close if not pixel-perfect) clone of the classic Winamp UI, but built in Rust, and using Apple Music as the music library.
<!-- tumwater:prompt:end -->

## Status

<!-- tumwater:status:start -->
Early skeleton (0.1.0): an iced (0.14) desktop shell — a Now Playing bar showing the real song
title, transport controls with Play/Pause/Stop and Previous/Next stepping through the current album
(a Winamp-style Repeat toggle, off by default, makes them wrap at the album's ends) and a
Winamp-style volume slider, and an artist → album → song browser with Back navigation that marks the
currently playing song in the Songs view over an in-memory sample library; the Apple
Music API is not wired up yet. Open work lives in PLANS.md, BUGS.md, and QUESTIONS.md.
<!-- tumwater:status:end -->

## Screenshots

Design mockups of the look Nostalgia is aiming for, matched against the classic Winamp 2.x base
skin and shown with the built-in sample library. The running app is still the plain iced shell
described above; the equalizer and Repeat are not built yet.

![Main window, equalizer and library window docked together](docs/screenshots/hero.png)

Transport states — playing, paused, stopped, and Repeat on:

![Main window in playing, paused, stopped and repeat states](docs/screenshots/transport.png)

The playlist editor as a library browser, drilling from Artists to Albums to Songs:

![Library window showing artists, albums and songs](docs/screenshots/library.png)

The equalizer with presets, plus window-shade mode:

![Equalizer with flat and rock presets, and windows rolled up into shade mode](docs/screenshots/equalizer.png)

The player on its own: [docs/screenshots/player.png](docs/screenshots/player.png).

## Usage

Build and run with `cargo run`; the player opens an iced window showing the Now Playing bar, the
Play/Pause/Stop/Previous/Next controls, a Repeat toggle, a volume slider, and a browse list. There
are no CLI flags or config files yet.

## Development

`make check` runs the whole landing gate in one command — `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and the test suite — and is the fastest way to see whether a change is merge-ready. The individual stages are available separately (`make fmt`, `make lint`, `make test`), and `make run` is the same as `cargo run`.
