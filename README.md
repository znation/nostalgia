# nostalgia

A (very close if not pixel-perfect) clone of the classic Winamp UI, built in Rust, using Apple
Music as the music library.

## Initial prompt

<!-- tumwater:prompt:start -->
Nostalgia is a (very close if not pixel-perfect) clone of the classic Winamp UI, but built in Rust, and using Apple Music as the music library.
<!-- tumwater:prompt:end -->

## Status

<!-- tumwater:status:start -->
Early skeleton (0.1.0): an iced (0.14) desktop shell themed with a Winamp 2.x base-skin palette —
a custom raised title bar with minimize, close, and window-shade roll-up, a Now Playing LCD bar,
chrome transport controls with Repeat, a volume slider, an equalizer panel with a preset pick
list, and an artist → album → song browser. A MusicKit loopback sign-in runs at startup when
`APPLE_MUSIC_DEVELOPER_TOKEN` is set, but the browse queries and playback still answer from the
in-memory sample library. Open work lives in PLANS.md, BUGS.md, and QUESTIONS.md.
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

## Usage

Build and run with `cargo run`; the player opens an iced window showing the Now Playing bar, the
Play/Pause/Stop/Previous/Next controls, a Repeat toggle, a volume slider, an equalizer panel
(an EQ on/off button, a preset pick list, a preamp slider, and ten band sliders), and a browse
list. Double-clicking the title bar rolls the window up to just that title bar, and double-clicking
it again restores it. With the window focused, the classic Winamp keys drive playback: Z and B
step back and forward, X plays, C pauses, V stops, and the up/down arrows nudge the volume. At
launch, setting the `APPLE_MUSIC_DEVELOPER_TOKEN` environment variable starts a browser-based
MusicKit sign-in; with it unset the built-in sample library is used. There are no CLI flags or
config files yet.

## Development

`make check` runs the whole landing gate in one command — `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, a `cargo doc` build that fails on any rustdoc warning, and the test suite — and is the fastest way to see whether a change is merge-ready. The individual stages are available separately (`make fmt`, `make lint`,
`make docs`, `make test`); `make fix` applies formatting and clippy's
machine-applicable fixes in place, and `make run` is the same as `cargo run`.
