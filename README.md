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
with library search and flat All Songs, All Albums, and Playlists views. Apple Music
browse/search and preview playback sit behind injectable seams and fall back to the sample
library and silence.
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

Build and run with `cargo run`; the player opens an iced window showing the Now Playing bar (a
small decorative `O A I D V` strip down the LCD's left edge, a large elapsed-time readout at the
left for a loaded preview, shown while playing or paused and blank (though it keeps its space)
while stopped, except that the `--:--` placeholder stays visible when stopped with no current
track or no known time, the playing track's artist and title scrolled across the display when
the line is longer than it, and the track's plain length at the right); the transport glyph before
the readout is a green `▶` while playing, a green `II` while paused, and a red `■` while stopped.
While paused, the large elapsed readout blinks once per second at its frozen position; it stays
steady while playing and clears while stopped. A loaded preview's average
bitrate, sample rate, and mono/stereo mode appear in a small status row beneath the title
(`256 KBPS  44 KHZ  STEREO`) when the backend reports them, falling back to the sample rate and
mode alone (`44 KHZ  STEREO`) when it cannot compute the bitrate, and the row stays blank
otherwise. A loaded preview also drives a small green ten-bar spectrum visualizer beneath the
title, updating about four times a second while playback runs; Pause freezes the bars at their
last levels, Stop and a new selection clear them, and the strip stays blank (with its height)
when no preview reports a spectrum. Beneath the LCD, a chrome slider spans the loaded preview's
own length and tracks its playback position; dragging moves only the thumb, and releasing seeks
to that position. The seek slider is blank (but keeps its height) until the backend reports
both a playback position and a preview length, such as on the silent fallback; its scale is
the preview's length, not the catalog track's length shown at the LCD's right edge. Clicking
the large elapsed readout while playing or paused switches it to the track's remaining time
(`-02:37`) and back; the choice stays until clicked again. The bar sits above the Play/Pause/Stop/Previous/Next
controls, Repeat and Shuffle toggles, volume and balance sliders, an equalizer panel (an EQ on/off
button, a preset pick list, a response curve above a vertical preamp slider, a `+12 dB` / `0 dB` /
`-12 dB` scale, and ten band sliders), and a
browse list whose song rows read `artist - title` (or the title alone when the source supplies
no artist) and show each track's length, and whose album rows read `artist - title` (or the
title alone when the source supplies no artist), with each song list showing its song count
and total playing time beneath the rows and artist, album, and playlist lists showing their
item count beneath theirs. Each equalizer band slider's groove is tinted by that band's own
gain, from green at `-12 dB` through yellow at `0 dB` to red at `+12 dB`.
Every browse list numbers its rows from 1 like a Winamp playlist, with the playing song marked by
`▶` before its number. A `Search library` box above the browse list searches the library when you
press Enter, replacing the list with the matching songs; Back leaves a search for the artists
list. The Artists list also has `All Songs`, `All Albums`, and `Playlists` buttons: the first
opens a flat list of every song in the library, whose rows play like any other song row, the
second opens a flat list of every album, whose rows open that album's songs, and the third opens a
flat list of every playlist, whose rows open that playlist's songs. Back from any of these lists
returns to the artists list, Back from songs opened through All Albums returns to All Albums, Back
from songs opened through Playlists returns to Playlists, and Next/Previous step through the flat
song list while it is showing. A `Top` button above every browse list returns it to its first row,
and opening a list starts it at the top. A `Sort` button beside `Top` reorders the showing list
alphabetically by its rows' rendered text, ignoring case; the reorder is one-shot, so reopening
the list restores the library's order. The title bar's `A` button toggles always-on-top, its
shade button (or a double-click on the bar) rolls the window up to the title bar plus a compact
transport strip with Previous, Play/Pause, Stop, Next, the current time readout, and a volume
slider, and back. Its `–` and `✕` buttons minimize and close. With the window focused, the
classic Winamp keys drive playback: Z
and B step back and forward, X plays, C pauses, V stops, R toggles Repeat and S toggles Shuffle,
and the up/down arrows nudge the volume while the left/right arrows seek five seconds back or
forward. With Shuffle on, Previous retraces the tracks actually played before the current one
instead of stepping one row backward in the list. The volume slider and the up/down arrows set
the audio backend's playback level, the balance slider sets its stereo balance, and the
equalizer panel's toggles, sliders, and presets change the playing preview's curve live; the
volume and balance are remembered for the next track. Setting `APPLE_MUSIC_DEVELOPER_TOKEN` at
launch starts a browser-based MusicKit sign-in: browse and search queries then read your Apple
Music library through the REST API, and fall back to the built-in sample library when it is
unset. Playback plays the selected song's Apple Music preview through `rodio` when the library
supplies one, and updates the UI state either way. Pause and Stop silence the preview through
the audio backend; a pause with no loaded preview stops the transport instead, since there is
nothing to pause. Play resumes a paused preview or restarts a stopped one from the beginning.
When a loaded preview reaches its end while playing, the player steps forward through the
showing list: to the next song, to the first song with Repeat on at the end, or, with Shuffle
on, to a shuffled song when another exists. With Repeat off on the last song it stops instead
and keeps that song queued for Play.
There are no CLI flags or config files yet.

## Development

`make check` runs the whole landing gate in one command — `cargo fmt --check`, `cargo clippy
--all-targets -- -D warnings`, a `cargo doc` build that fails on any rustdoc warning, and the test
suite — and is the fastest way to see whether a change is merge-ready. The individual stages are
available separately (`make fmt`, `make lint`, `make docs`, `make test`); `make test-one
TEST=<name>` runs only the tests whose
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
