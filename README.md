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
small `O A I D V` strip down the LCD's left edge, whose `A` toggles always-on-top and whose `V`
hides and restores the visualizer while `O`, `I`, and `D` stay decorative; then a large
elapsed-time readout at the left for a loaded preview, shown while playing or paused and blank
(though it keeps its space)
while stopped, except that the `--:--` placeholder stays visible when stopped with no current
track or no known time, the playing track's 1-based position in the list it was played from
(`1. The Sample Band - Opening`) followed by its artist and title, scrolled across the
display when the line is longer than it, and the track's plain length at the right); the
transport glyph before the readout is a green `▶` while playing, a green `II` while paused, and
a red `■` while stopped.
While paused, the large elapsed readout blinks once per second at its frozen position; it stays
steady while playing and clears while stopped. A loaded preview's average bitrate and sample rate
appear at the left of a small status row beneath the title, with `MONO` and `STEREO` side by side
at its right; the reported mode is lit green and the other dim. The prefix (`256 KBPS  44 KHZ`)
appears only when the backend reports a format, falling back to the sample rate alone (`44 KHZ`)
when it cannot compute the bitrate; with no reported format both labels stay dim and the prefix is
absent. A loaded preview also drives a small green ten-bar spectrum visualizer beneath the
title, updating about four times a second while playback runs; Pause freezes the bars at their
last levels, Stop and a new selection clear them, and the strip stays blank (with its height)
when no preview reports a spectrum. The strip's `V` hides the bars and shows them again from the
still-stored spectrum, so the poll keeps running while they are hidden; a click on the bars'
strip does the same, hidden or shown. Beneath the LCD, a chrome
slider spans the loaded preview's own length and tracks its playback position; dragging moves
the thumb and previews that
position in the large elapsed readout (counting down from it when the readout shows remaining
time), and releasing seeks to it. The seek slider is blank (but keeps its height) until the
backend reports both a playback position and a preview length, such as on the silent fallback;
its scale is the preview's length, not the catalog track's length shown at the LCD's right edge.
Clicking the large elapsed readout while playing or paused switches it to the track's remaining
time (`-02:37`) and back; the choice stays until clicked again. The compact readouts in the library
footer and the shade strip are click targets too while playing or paused, switching the same mode;
a stopped readout ignores the click. The bar sits above Previous,
Play/Pause, Stop, and Next glyph controls with word tooltips, then the Repeat and Shuffle
toggles, a volume slider whose groove runs from green when silent through orange at half volume to red at full volume, a balance slider whose groove is green at center and reddens toward either side,
an equalizer panel (an EQ on/off button, a preset pick list, a response curve above ten band
sliders, with a vertical preamp slider and a `+12 dB` / `0 dB` / `-12 dB` scale to their left),
and a browse list whose song rows read `artist - title` (or the title alone when the source
supplies no artist) and show each track's length at the right, whose album rows read
`artist - title` (or the title alone when the source supplies no artist) and show the album's
total running time at the right (`10:30`, or `--:--` when the source supplied none), and whose
artist rows show their album count at the right (`2 albums`, `1 album`, and no count for an
artist with none), with each song list showing its song count and total playing time beneath
the rows (while a song row is selected, the footer shows that song's length before the total,
`2 songs - 3:30/4:15`, like Winamp's playlist editor) and artist, album, and playlist lists showing their item count beneath theirs. An `EQ`
button beside the balance slider shows and hides the equalizer panel, which is open on a first launch and then reopens as last left;
hiding it gives its height to the browse list without changing EQ processing. A `PL` button
directly beside `EQ` shows and hides the whole library section — its `NOSTALGIA LIBRARY` band,
browse controls, search box, list, and footer — while the title bar, Now Playing bar, seek bar,
transport row, and equalizer stay drawn; the library is open on a first launch and then reopens as last left, the toggle changes no
playback, equalizer, browse, or search state, and it does not resize the window. A
compact transport strip sits beneath the browse list with Previous, Play/Pause, Stop, Next,
an elapsed-over-total time readout, and a volume slider — the window-shade strip's controls
without its seek slider. The readout shows the elapsed (or remaining) time over the track's
total (`00:53/3:30`) and shows just the elapsed string when no total is known, so the empty
state reads the plain `--:--`.
Each
equalizer band slider's groove is tinted by that band's own gain, from green at `-12 dB` through
yellow at `0 dB` to red at `+12 dB`; the preamp slider's groove is tinted the same way by the
preamp gain. The app bar sets `NOSTALGIA` between gold tape strips, and
the equalizer and browse sections are each introduced by a matching title band — `NOSTALGIA
EQUALIZER` above the equalizer panel and `NOSTALGIA LIBRARY` above the browse list.
Every browse list numbers its rows from 1 like a Winamp playlist and draws ordinary rows in green
text, with the playing song marked by `▶` before its number and drawn in white without a bar. A
click selects a row and paints its blue bar with white text, without opening it; a second click
on the selected row (a double-click), Enter, or the `Open` button
opens it — drilling into an artist, album, or playlist, or playing a song — while Back still
climbs out. A `Search library` box above the browse list searches the library when you
press Enter, replacing the list with the matching songs (with no Apple Music session, matching
the query case-insensitively against each song's title, artist, and album title); Back leaves a
search for the artists
list. A `Clear` button beside the search box empties the typed query; when a search is showing it
also returns to the artists list exactly as Back does, and otherwise it changes only the box's
text. The Artists list also has `All Songs`, `All Albums`, and `Playlists` buttons: the first
opens a flat list of every song in the library, whose rows play like any other song row, the
second opens a flat list of every album, whose rows open that album's songs, and the third opens a
flat list of every playlist, whose rows open that playlist's songs. Back from any of these lists
returns to the artists list, Back from songs opened through All Albums returns to All Albums, Back
from songs opened through Playlists returns to Playlists, and Next/Previous step through the flat
song list while it is showing. A `Top` button above every browse list returns it to its first row,
and opening a list starts it at the top. A `Sort` button beside `Top` reorders the showing list
alphabetically by its rows' rendered text, ignoring case; the reorder is one-shot, so reopening
the list restores the library's order; Ctrl+Shift+1 does the same sort from the keyboard, like
Winamp's playlist editor. Ctrl+R reverses the showing list's order the same
one-shot way, like Winamp's playlist editor, and Ctrl+Shift+R shuffles it into a random order the same
one-shot way. Alt+3 opens a transient File info panel naming the current track's title, artist, and length, like
Winamp; its `Close` button, a click on the backdrop, or Alt+3 again closes it, and it opens only while a track is current. A `Reload` button beside `Sort` refetches the library's
artists list and returns to the Artists list, clearing any active search while leaving the typed
query in the search box. The title bar's `M` button opens a transient About panel naming the app,
showing its `v`-prefixed package version and the line `The classic Winamp look, rebuilt in
Rust — with Apple Music as the library.`; `M` (through the backdrop over its slot), the panel's
`Close` button, or a click on the backdrop outside the panel closes it, and opening it while the
window is rolled up restores the window first. Its
shade button, a double-click on the bar, or a double-click on the rolled-up strip rolls the window
up to the title bar plus a compact
transport strip with Previous, Play/Pause, Stop, Next, the same elapsed-over-total time readout, a
seek slider over the preview's own length, and a volume slider, and back. Its `–` and `✕`
buttons minimize and close. The window title
(the name the desktop environment shows for the app) names the current track as
`<number>Artist - Title - nostalgia` (omitting the number or artist when empty), keeping the
interrupted track's name across Stop and reading just `nostalgia` when no known track is current.
With the window focused, the classic Winamp keys drive playback: Z and B step back and forward
(a Z press restarts the loaded preview from its start once it has played at least three seconds),
X plays, C pauses, V stops, R toggles Repeat and S toggles Shuffle, A toggles always-on-top and T
switches the large readout between elapsed and remaining time (the same actions as the on-screen A
controls and a readout click), E shows or hides the equalizer panel exactly like the `EQ` button
beside the balance slider, and the up/down arrows nudge the volume while the left/right arrows
seek five seconds back or forward. With NumLock on, keypad 4, 5, and 6 play the previous,
current, and next song, 7 and 9 seek five seconds back or forward, and 8 and 2 nudge the volume,
and 1 and 3 jump ten songs back or forward through the same list, stopping at its ends, like Winamp. Ctrl+Z plays the first song of the list Z and B step through
and Ctrl+B plays its last song. Ctrl+W rolls the window up into shade mode and back, like the title bar's shade button. Backspace steps the browse list back one level exactly like the
`Back` button, and does nothing at the Artists list; Home snaps the browse list to its first row
and End to its last, like `Top`, and Page Up and Page Down scroll it one page at a time in the
matching direction; while the `Search library` box is focused it keeps Backspace,
Home, and End for editing its query instead of navigating. Ctrl+F moves keyboard focus to the
`Search library` box from any normal-mode browse level without searching or changing the current
view, list, selection, or typed query, and the box then takes typed keys as usual; in window-shade
mode there is no search box, so Ctrl+F does nothing. F5 refetches the library and returns to the
Artists list, exactly like the `Reload` button. While the window is focused, the system media
keys Play/Pause, Play, Pause, Stop, Next track, and Previous track drive those same transport
controls. Previous — the button, the Z key, and the system Previous track key — restarts the
loaded preview from its start once it has played at least three seconds, and steps back before
that; with Shuffle on, such an earlier Previous retraces the
tracks actually played before the current one instead of stepping one row backward in the list.
While a song list is showing, Next, Previous (including a shuffled retrace), and the preview-end
advance scroll that list so the landed row stays in view. The volume slider and the up/down arrows
set the audio backend's playback level, and rolling the mouse wheel over the player panels nudges
the volume by one slider step in the roll's vertical direction, except that a roll over the drawn
seek slider, in the Now Playing bar or the shade strip, seeks five seconds forward or back instead;
over the browse list the wheel keeps scrolling the list instead. The balance slider sets its stereo
balance; a drag moving back toward the middle snaps to exact center within 10% of it. While a volume or balance slider is dragged, the LCD shows its level (`Volume: 72%`,
`Balance: 31% Left`) in place of the title until release; an equalizer band or preamp drag does the same
(`EQ: 60HZ +3 DB`, `Preamp: -12 DB`), and dragging the seek slider shows the target
(`Seek to: 01:23/03:30 (39%)`) the same way. The equalizer panel's on/off button,
sliders, and presets change the playing preview's curve live; clicking `+12 DB`, `0 DB`, or `-12 DB` sets all ten bands to that gain and
leaves the preamp alone; the volume and balance are
remembered for the next track. Setting
`APPLE_MUSIC_DEVELOPER_TOKEN` at launch starts a browser-based MusicKit sign-in: browse and search
queries then read your Apple Music library through the REST API, and fall back to the built-in
sample library when it is unset. Playback plays the selected song's Apple Music preview through
`rodio` when the library supplies one, and updates the UI state either way. Pause and Stop silence
the preview through the audio backend; a pause with no loaded preview stops the transport instead,
since there is nothing to pause. Play resumes a paused preview or restarts a stopped one from the
beginning. When a loaded preview reaches its end while playing, the player steps forward through the
showing list: to the next song, to the first song with Repeat on at the end, or, with Shuffle on, to
a shuffled song when another exists. With Repeat off on the last song it stops instead and keeps
that song queued for Play. The volume, balance, Repeat and Shuffle flags, the equalizer's on/off
state, curve, and preset, the elapsed/remaining readout choice, the visualizer's shown/hidden
choice, the equalizer and library panels' shown/hidden choices, and the always-on-top choice are saved to `$XDG_CONFIG_HOME/nostalgia/settings.ini` (or
`$HOME/.config/nostalgia/settings.ini`) and restored at the next launch; `NOSTALGIA_SETTINGS_PATH`
overrides the path. A missing or malformed entry falls back to its default (a byte that is not UTF-8 spoils only its own
line), and a settings file that exists but cannot be read falls back to defaults with one line on stderr naming the file. The always-on-top choice
(the LCD strip's `A` or the `A` key) is remembered for the next launch, and a saved-on launch starts
with the window pinned once boot resolves its window id. There are no CLI flags yet.

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
