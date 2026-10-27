# Usage

The full control reference for the running app: what each part of the window does, every
keyboard and mouse binding, and how settings, Apple Music sign-in, and playback behave. The
project's summary, status, screenshots, and build instructions stay in
[README.md](../README.md).

Build and run with `cargo run`; the player opens an iced window showing the Now Playing bar (a
small `O A I D V` strip down the LCD's left edge, whose `O` opens the options menu, whose `A`
toggles always-on-top, whose `I` opens the File info panel, and whose `V` hides and restores the
visualizer while `D` stays decorative; each live letter is lit while its mode is on (`O` while
the options menu is open, `A` for always-on-top, `I` while File info is open, `V` while the
visualizer is shown) and dim otherwise, and `D` stays dim;
then a large elapsed-time readout at the left for a loaded preview, shown while
playing or paused and blank
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
absent. A loaded preview also drives a small ten-bar spectrum visualizer beneath the
title, shaded from dark green at the rail through yellow to orange at its peaks and updating
about four times a second while playback runs; Pause freezes the bars at their
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
Play, Pause, Stop, and Next glyph controls with word tooltips: Play starts or resumes
the loaded preview and Pause pauses it (stopping a track whose source supplied no preview),
then the Repeat and Shuffle
toggles, a volume slider whose groove runs from green when silent through orange at half volume to
red at full volume, a balance slider whose groove is green at center and reddens toward either side,
an equalizer panel (an EQ on/off button, an AUTO on/off button, a preset pick list, a response curve that eases smoothly
between the stored band gains above ten band
sliders, with a vertical preamp slider and a `+12 dB` / `0 dB` / `-12 dB` scale to their left; the
plot marks the 0 dB midline and the ten bands with dim guides behind the green trace, and its
preset pick list names the applied preset and reads `Custom` when no preset is applied, whether
on a fresh launch or after a slider move makes the curve your own),
a lightning button at the end of the row,
and a browse list whose song rows read `artist - title` (or the title alone when the source
supplies no artist) and show each track's length at the right, whose album rows read
`artist - title` (or the title alone when the source supplies no artist) and show the album's
total running time at the right (`10:30`, or `--:--` when the source supplied none), and whose
artist rows show their album count at the right (`2 albums`, `1 album`, and no count for an
artist with none), with each song list showing its song count and total playing time beneath
the rows (while a song row is selected, the footer shows that song's length before the total,
`2 songs - 3:30/4:15`, like Winamp's playlist editor) and artist, album, and playlist lists
showing their item count beneath theirs. An `EQ` button beside the balance slider shows and
hides the equalizer panel, which is open on a first launch and then reopens as last left;
hiding it gives its height to the browse list without changing EQ processing. A `PL` button
directly beside `EQ` shows and hides the whole library section — its `NOSTALGIA LIBRARY` band,
browse controls, search box, list, and footer — while the title bar, Now Playing bar, seek bar,
transport row, and equalizer stay drawn; the library is open on a first
launch and then reopens as last left, the toggle changes no playback,
equalizer, browse, or search state, and it does not resize the window. A
compact transport strip sits beneath the browse list with Previous, Play, Pause, Stop, Next,
an elapsed-over-total time readout, and a volume slider — the window-shade strip's controls
without its seek slider. Play starts or resumes and Pause pauses, like the main row. The readout shows the elapsed (or remaining) time over the track's
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
one-shot way, like Winamp's playlist editor, and Ctrl+Shift+R shuffles it into a random order the
same one-shot way. Alt+Up and Alt+Down move the selected row of the showing list one place up or
down, the same one-shot way, keeping the row selected and scrolling the list only as far as needed
to keep it in view. Alt+3 opens a transient File info panel naming the current track's title,
artist, album, year, genre, track number, and length, like Winamp (the album, year, genre, and
track-number lines are left out
when the source supplies none), plus a `Format:` line
(`256 KBPS  44 KHZ STEREO`) while a loaded preview reports its format and `44 KHZ MONO` when it
reports no bitrate; its `Close` button, a click on the backdrop, or Alt+3 again closes it, and it
opens only while a track is current. Ctrl+J opens a transient Jump to time panel: type seconds
(`95`) or `M:SS` (`1:35`) and press Enter or `Jump` to seek the loaded preview there (clamped to its
length) and close the panel; `Close`, a click on the backdrop, or Ctrl+J again closes it, and it
opens only while a preview with a known length is loaded. Plain `J` opens a transient Jump to File
panel over the showing song list (a browsed album or playlist's songs, search results, or All
Songs): each row is numbered like the browse list, typing filters its cached row titles
case-insensitively, Up and Down move the selection bar through the matches (the first
match until you move it), and Page Up and Page Down move it one page of visible matches at a
time, clamped like Up and Down; every highlight move scrolls the panel's list only as far as
needed to keep the row in view, and Enter or `Jump` plays the highlighted row — or a row click plays
that row — closing the panel; `Close` or a click on the backdrop closes it, and it opens only on
those song lists. A `Reload` button beside `Sort`
refetches the library's artists list and returns to the Artists list, clearing any active search
while leaving the typed query in the search box. The title bar's `M` button (or `Ctrl+F1`) opens
a transient About panel naming the app, showing its `v`-prefixed package version, the line
`The classic Winamp look, rebuilt in Rust — with Apple Music as the library.`, and the active
library source (`Library: Apple Music` once a MusicKit session is stored, `Library: Sample
library` otherwise); `M` (through the
backdrop over its slot), `Ctrl+F1`, the panel's `Close` button, or a click on the backdrop outside
the panel closes it, and opening it while the window is rolled up restores the window first. The
LCD's `O` letter or the lightning button at the end of the transport row opens an `OPTIONS` menu
over the player listing `Always on Top: On/Off`, `Time: Remaining/Elapsed`, `Visualizer:
Shown/Hidden`, `Equalizer: Shown/Hidden`, `Library: Shown/Hidden`, `About Nostalgia`, and `Close`,
with each label reflecting the current state. Choosing a row applies that one toggle and closes
the menu; `About Nostalgia` opens the About panel, and `Close`, a click on the backdrop, or Escape
closes the menu. Opening it while the window is rolled up restores the window first, and the menu
is not saved. A
plain or Shift-held Escape press closes whichever transient panel is open, like its `Close`
button or the backdrop; with no panel open Escape keeps its existing behavior. Its
shade button, a double-click on the bar, or a double-click on the rolled-up strip rolls the window
up to the title bar plus a compact transport strip with Previous, Play, Pause, Stop, Next, the same
elapsed-over-total time readout, a seek slider over the preview's own length, and a volume slider,
where Play starts or resumes and Pause pauses, and back. Its `–` and `✕` buttons minimize and close. The window title (the name the desktop
environment shows for the app) names the current track as
`<number>Artist - Title - nostalgia` (omitting the number or artist when empty), keeping the
interrupted track's name across Stop and reading just `nostalgia` when no known track is current.
With the window focused, the classic Winamp keys drive playback: Z and B step back and forward
(a Z press restarts the loaded preview from its start once it has played at least three seconds),
X plays, C pauses, V stops, R toggles Repeat and S toggles Shuffle, A (or Ctrl+A) toggles
always-on-top and T (or Ctrl+T) switches the large readout between elapsed and remaining time (the
same actions as the on-screen A
controls and a readout click), E shows or hides the equalizer panel exactly like the `EQ` button
beside the balance slider (Alt+E shows or hides the library section exactly like the `PL` button,
and Alt+G the equalizer panel like `EQ`, as Winamp's Alt+E and Alt+G toggle its playlist editor and
equalizer), and the up/down arrows nudge the volume while the left/right arrows
seek five seconds back or forward. With NumLock on, keypad 4, 5, and 6 play the previous,
current, and next song, 7 and 9 seek five seconds back or forward, and 8 and 2 nudge the volume,
and 1 and 3 jump ten songs back or forward through the same list, stopping at its ends, like Winamp.
Ctrl+Z plays the first song of the list Z and B step through
and Ctrl+B plays its last song. Ctrl+W rolls the window up into shade mode and back, like the title
bar's shade button. Backspace steps the browse list back one level exactly like the
`Back` button, and does nothing at the Artists list; Home snaps the browse list to its first row
and End to its last, like `Top`, and Page Up and Page Down scroll it one page at a time in the
matching direction; while the `Search library` box is focused it keeps Backspace,
Home, and End for editing its query instead of navigating. Ctrl+F moves keyboard focus to the
`Search library` box from any normal-mode browse level without searching or changing the current
view, list, selection, or typed query, and the box then takes typed keys as usual; in window-shade
mode there is no search box, so Ctrl+F does nothing. F5 refetches the library and returns to the
Artists list, exactly like the `Reload` button. While the window is focused, the system media
keys Play/Pause, Play, Pause, Stop, Next track, and Previous track drive the transport: Play,
Pause, Stop, Next track, and Previous track match the on-screen controls, while Play/Pause
toggles a loaded preview and stops a track with none. Previous — the button, the Z key, and the system Previous track key — restarts the
loaded preview from its start once it has played at least three seconds, and steps back before
that; with Shuffle on, such an earlier Previous retraces the
tracks actually played before the current one instead of stepping one row backward in the list.
While a song list is showing, Next, Previous (including a shuffled retrace), the preview-end
advance, Ctrl+Z/Ctrl+B edge jumps, and keypad 1/3 ten-song jumps scroll that list only as far as
needed to keep the landed row in view, like Winamp's playlist editor. The
volume slider and the up/down arrows set the audio backend's playback
level, and rolling the mouse wheel over the player panels nudges
the volume by one slider step in the roll's vertical direction, except that a roll over the drawn
seek slider, in the Now Playing bar or the shade strip, seeks five seconds forward or back instead;
a roll over the balance slider moves the balance by one step, and a roll over the equalizer's
preamp or a band moves that gain by one step; over the browse list the wheel keeps scrolling the
list instead. The balance slider sets its stereo
balance; a drag moving back toward the middle snaps to exact center within 10% of it. While a volume
or balance slider is dragged, the LCD shows its level (`Volume: 72%`,
`Balance: 31% Left`) in place of the title until release; an equalizer band or preamp drag does
the same (`EQ: 60HZ +3 DB`, `Preamp: -12 DB`), and dragging the seek slider shows the target
(`Seek to: 01:23/03:30 (39%)`) the same way. The equalizer panel's on/off button,
sliders, and presets change the playing preview's curve live. The `AUTO` button loads a preset
automatically when a song is selected: while it reads `AUTO: On`, a song whose trimmed primary
genre matches a preset name case-insensitively (say `rock` or `Rock`) applies that preset's whole
curve and names it in the pick list, while a blank or unmatched genre leaves the current curve
alone; `AUTO: Off` never changes the curve. Clicking `+12 DB`, `0 DB`, or
`-12 DB` sets all ten bands to that gain and leaves the preamp alone; the volume and balance are
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
state, AUTO choice, curve, and preset, the elapsed/remaining readout choice, the visualizer's shown/hidden
choice, the equalizer and library panels' shown/hidden choices, the always-on-top choice, and the
window-shade mode are saved to `$XDG_CONFIG_HOME/nostalgia/settings.ini` (or `$HOME/.config/nostalgia/settings.ini`)
and restored at the next launch; `NOSTALGIA_SETTINGS_PATH`
overrides the path. A missing or malformed entry falls back to its default (a `NaN` number
counts as malformed; a byte that is not UTF-8 spoils only its own line, and a leading UTF-8 BOM
is ignored), and a settings file
that exists but cannot be
read falls back to defaults with one line on stderr naming the file; the player then refuses to
save for the rest of that run, printing a line that names the file each time, so the unread values
are not overwritten with defaults. Repair or remove that file and restart to save settings again.
Each save replaces
the file atomically; a symlinked settings file stays a link, with the write going to
the file it points at and that file's mode kept. The always-on-top choice
(the LCD strip's `A` or the `A` key) is remembered for the next launch, and a saved-on launch starts
with the window pinned once boot resolves its window id. The shade mode (the title bar's shade
button, a double-click on the bar or the rolled-up strip, or Ctrl+W) is remembered too: a
saved-shaded launch opens rolled up and the same controls unroll it to the size the window opened
at. There are no CLI flags yet.
