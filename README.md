# nostalgia

A (very close if not pixel-perfect) clone of the classic Winamp UI, built in Rust, using Apple
Music as the music library.

## Initial prompt

<!-- tumwater:prompt:start -->
Nostalgia is a (very close if not pixel-perfect) clone of the classic Winamp UI, but built in Rust, and using Apple Music as the music library.
<!-- tumwater:prompt:end -->

## Status

<!-- tumwater:status:start -->
Early skeleton (0.1.0): an iced (0.14) desktop clone of the Winamp 2.x UI, with a themed player
(Now Playing LCD, transport, volume and balance, equalizer) and an artist → album → song browser
with library search and flat All Songs and All Albums views. Apple Music browse/search and
preview playback sit behind injectable seams and fall back to the sample library and silence.
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

## Usage

Build and run with `cargo run`; the player opens an iced window showing the Now Playing bar (the
playing track's artist and title, with its length at the right), the Play/Pause/Stop/Previous/Next
controls, Repeat and Shuffle toggles, volume and balance sliders, an equalizer panel (an EQ on/off
button, a preset pick list, a preamp slider, and ten band sliders), and a browse list whose song
rows show each track's length. A `Search library` box above the browse list searches the library
when you press Enter, replacing the list with the matching songs; Back leaves a search for the
artists list. The Artists list also has `All Songs`, `All Albums`, and `Playlists` buttons: the
first opens a flat list of every song in the library, whose rows play like any other song row, the
second opens a flat list of every album, whose rows open that album's songs, and the third opens a
flat list of every playlist, whose rows open that playlist's songs. Back from any of these lists
returns to the artists list, Back from songs opened through All Albums returns to All Albums, Back
from songs opened through Playlists returns to Playlists, and Next/Previous step through the flat
song list while it is showing. The title bar's `A` button toggles always-on-top, its shade button
(or a double-click on the bar) rolls the window up to just that title bar and back, and its `–` and
`✕` buttons minimize and close. With the window focused, the classic Winamp keys drive playback: Z
and B step back and forward, X plays, C pauses, V stops, R toggles Repeat and S toggles Shuffle,
and the up/down arrows nudge the volume.
The volume slider and the arrow keys set the audio backend's playback level, and the balance slider
sets its stereo balance; both are remembered for the next track.
Setting `APPLE_MUSIC_DEVELOPER_TOKEN` at launch starts a browser-based MusicKit sign-in: browse and
search queries then read your Apple Music library through the REST API, and fall back to the
built-in sample library when it is unset. Playback plays the selected song's Apple Music preview
through `rodio` when the library supplies one, and updates the UI state either way. Pause and Stop
silence the preview through the audio backend, and Play resumes a paused
preview or restarts a stopped one from the beginning. There are no CLI flags or config files yet.

## Development

`make check` runs the whole landing gate in one command — `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, a `cargo doc` build that fails on any rustdoc warning, and the test suite — and is the fastest way to see whether a change is merge-ready. The individual stages are available separately (`make fmt`, `make lint`,
`make docs`, `make test`); `make test-one TEST=<name>` runs only the tests whose
names contain `<name>`, for a fast loop while iterating on one behavior;
`make fix` applies formatting and clippy's machine-applicable fixes in place,
and `make run` is the same as `cargo run`.

`docs/invariants.md` records each subsystem's guarantees, the code that enforces them, and the
tests that pin them; update a section when a change alters its guarantee.

On Linux the build links against the system audio library, so install the ALSA
development headers first — `libasound2-dev` on Debian/Ubuntu, `alsa-lib-devel`
on Fedora — or `rodio`'s `cpal` backend fails to build. CI
(`.github/workflows/ci.yml`) installs them and runs `make check` on every push
and pull request.
