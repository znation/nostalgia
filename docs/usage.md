# Usage

The full control reference for the running app: what each part of the window does, every
keyboard and mouse binding, and how settings, Apple Music sign-in, and playback behave. The
project's summary, status, screenshots, and build instructions stay in
[README.md](../README.md).

Build and run with `cargo run`; the player opens an iced window showing the Now Playing bar (a
small `O A I D V` strip down the LCD's left edge, whose `O` opens the options menu, whose `A`
toggles always-on-top, whose `I` opens the File info panel (like Alt+3: for the selected song
row, or the current track without one), and whose `V` hides and restores the
visualizer while `D` stays decorative; each live letter is lit while its mode is on (`O` while
the options menu is open, `A` for always-on-top, `I` while File info is open, `V` while the
visualizer is shown) and dim otherwise, and `D` stays dim;
then a large elapsed-time readout at the left for a loaded preview, shown while
playing or paused and blank
(though it keeps its space)
while stopped, except that the `--:--` placeholder stays visible when stopped with no current
track or no known time, the playing track's 1-based position in the list it was played from
(`1. The Sample Band - Opening`) followed by its artist and title, scrolled across the
display when the line is longer than it (repeating after a `  ***  ` separator, like Winamp), and the track's plain length at the right; pressing on a
scrolling title and dragging left or right scrolls it by hand, one character per 10 pixels,
pausing the automatic scroll until you release, which then resumes from there, like Winamp); the
transport glyph before the readout is a green `▶` while playing, a green `II` while paused, and
a red `■` while stopped.
While paused, the large elapsed readout blinks once per second at its frozen position; it stays
steady while playing and clears while stopped. The compact `elapsed/total` readouts in the library
footer and the shade strip blink with it while paused. A loaded preview's average bitrate and sample rate
appear at the left of a small status row beneath the title, with `MONO` and `STEREO` side by side
at its right; the reported mode is lit green and the other dim. The prefix (`256 KBPS  44 KHZ`)
appears only when the backend reports a format, falling back to the sample rate alone (`44 KHZ`)
when it cannot compute the bitrate; with no reported format both labels stay dim and the prefix is
absent. A loaded preview also drives a small ten-bar spectrum visualizer beneath the
title, shaded from dark green at the rail through yellow to orange at its peaks and updating
about four times a second while playback runs. A thin gray peak marker above each bar jumps up
with the bar and then drops back slowly. The bars also jump up at once but sink back over about a
second when a band goes quiet, faster than their markers. Pause freezes the bars and markers at their
last levels, Stop and a new selection clear them, and the strip stays blank (with its height)
when no preview reports a spectrum. The strip's `V` hides the bars and their markers and shows them again from the
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
the loaded preview (restarting it from its start while it is already playing, like Winamp) and Pause pauses it, or resumes it while paused, like Winamp (stopping a track whose source supplied no preview),
then the Repeat and Shuffle
toggles (each sinks while on), a volume slider whose groove runs from green when silent through orange at half volume to
red at full volume, a balance slider whose groove is green at center and reddens toward either side,
an equalizer panel (an EQ on/off button and an AUTO on/off button, each sunken while on, a preset pick list, a response curve that eases smoothly
between the stored band gains above ten band
sliders, with a vertical preamp slider and a `+12 dB` / `0 dB` / `-12 dB` scale to their left; the
plot marks the 0 dB midline and the ten bands with dim guides behind the green trace, and its
preset pick list names the applied preset and reads `Custom` when no preset is applied, whether
on a fresh launch or after a slider move makes the curve your own, and offers the `Save preset…`
row and the curves saved through it after the built-ins),
a lightning button at the end of the row,
and a browse list whose song rows read `artist - title` (or the title alone when the source
supplies no artist) and show each track's length at the right, whose album rows read
`artist - title` (or the title alone when the source supplies no artist) and show the album's
release year (when known) and total running time at the right (`2001  10:30`, `10:30` with no known year, or `--:--` when the source supplied no songs), and whose
artist rows show their album count at the right (`2 albums`, `1 album`, and no count for an
artist with none), with each song list showing its song count and total playing time beneath
the rows (while a song row is selected, the footer shows that song's length before the total,
`2 songs - 3:30/4:15`, like Winamp's playlist editor; a total covering a song with no known
length, or such a selected song's length, gets a trailing `+`, as in `2 songs - 3:30+`), album lists showing
their album count and total playing time beneath theirs (`3 albums - 45:30`, with the same
trailing `+` when an album has no known running time; while an album row is selected, the footer
shows that album's running time before the total, `3 albums - 10:30/45:30`), the artists list showing its
artist count and the total of its rows' album counts beneath it (`3 artists - 5 albums`, or just
`3 artists` when no artist has a counted album; while an artist with albums is selected, the footer
shows its album count before the total, `3 artists - 2/5 albums`), the Genres, Years, and Composers lists showing their count and total playing time beneath
theirs (`2 genres - 7:10`, with the same trailing `+`; while a row is selected, the footer
shows that group's playing time before the total, `2 genres - 3:30/7:10`), and playlist lists showing their item
count beneath theirs. An `EQ` button beside the balance slider shows and
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
press Enter, replacing the list with the matching songs (with no Apple Music session, matching,
for each whitespace-separated word of the query, case-insensitively against a song's title, artist, album title, genre, composer, or release year, in any order, so `rock 2001` finds the 2001 rock songs); Back leaves a
search for the artists
list. A `Clear` button beside the search box empties the typed query; when a search is showing it
also returns to the artists list exactly as Back does, and otherwise it changes only the box's
text. The Artists list also has `All Songs`, `All Albums`, `Playlists`, `Genres`, `Years`, `Composers`, `Recently Added`, and `Recently Played` buttons: the first
opens a flat list of every song in the library, whose rows play like any other song row, the
second opens a flat list of every album, whose rows open that album's songs, the third opens a
flat list of every playlist, whose rows open that playlist's songs (the song lists you saved with
`Save list` list beneath them, each opening its saved songs, numbered from 1 and counted in the
footer like any song list, and Back from them returns to Playlists), the fourth lists the library's genres (described below), the fifth lists the library's release years (described below), the sixth lists the library's composers (described below), the seventh lists the library's
recently added albums newest first (the sample library's albums in reverse order when
signed out), whose rows open that album's songs, and the eighth lists the songs you played most recently
first (while signed out, the songs whose plays committed this run, newest first — the sample
library's songs in reverse order until something has played this run, and the list saved in the
settings file after a relaunch — and the service's own list once signed in), whose rows play like
any other song row. Every open of the list recomputes it, like the other flat lists, so newly
played songs take their spots at the top the next time you reopen it. `Genres` lists the library's genres by name, each with its song count and total playing time and any
songs without a genre under `Unknown` last; opening a genre lists its songs, which play like any
other song row. `Years` lists the library's release years oldest first, each with its song count and total playing time and
any songs without a year under `Unknown` last; opening a year lists its songs, which play like any
other song row. `Composers` lists the library's composers by name ignoring case, each with its song
count and total playing time and any songs without a composer under `Unknown` last; opening a composer lists its songs,
which play like any other song row. Back from any of these lists
returns to the artists list, Back from songs opened through All Albums returns to All Albums, Back from songs opened through
Recently Added returns to Recently Added, Back
from songs opened through Playlists returns to Playlists, Back from a genre's songs returns to Genres, Back from a year's songs returns to Years, Back from a composer's songs returns to Composers, and Next/Previous step through the flat
song list (All Songs or Recently Played) while it is showing. A `Top` button above every browse list returns it to its first row,
and opening a list starts it at the top. A `Sort` button beside `Top` reorders the showing list
alphabetically by its rows' rendered text, ignoring case; the reorder is one-shot, so reopening
the list restores the library's order; Ctrl+Shift+1 does the same sort from the keyboard, like
Winamp's playlist editor. Ctrl+R reverses the showing list's order the same
one-shot way, like Winamp's playlist editor, and Ctrl+Shift+R shuffles it into a random order the
same one-shot way. Alt+Up and Alt+Down move the selected row of the showing list one place up or
down, the same one-shot way, keeping the row selected and scrolling the list only as far as needed
to keep it in view. Alt+3 opens a transient File info panel naming the showing song list's
selected row's song when one is picked (the browsed album's or playlist's songs, a search-result
list sharing that song list, All Songs, or Recently Played — the selection decides, not the
playing track) and the current track otherwise — its title, artist, album, year, genre, track
number, disc number, composer, content rating (`Rating: Explicit` or `Rating: Clean`), and
length, like Winamp (the album, year, genre, track-number, disc-number, composer, and rating
lines are left out when the source supplies none). A selected row's panel shows no `Format:`
line, because no preview is loaded for it; a current track's panel adds the `Format:` line
(`256 KBPS  44 KHZ STEREO`) while a loaded preview reports its format and `44 KHZ MONO` when it
reports no bitrate. Its `Close` button, a click on the backdrop, or Alt+3 again closes it, and it
opens only with a song row selected in a showing song list or a current track. Opening it never
plays the selected row and does not change the Now Playing bar, the marquee, or the window title. Ctrl+J opens a transient Jump to time panel whose box opens holding the current position (`1:35`), selected so typing replaces it (empty when no position is known): type seconds
(`95`) or `M:SS` (`1:35`) and press Enter or `Jump` to seek the loaded preview there (clamped to its
length) and close the panel; `Close`, a click on the backdrop, or Ctrl+J again closes it, and it
opens only while a preview with a known length is loaded. Plain `J` opens a transient Jump to File
panel over the showing song list (a browsed album or playlist's songs, search results, or All
Songs): each row is numbered like the browse list, typing filters its cached row titles
(each typed word must appear in the row's artist-and-title text, in any order, case-insensitively), Up and Down move the selection bar through the matches (the first
match until you move it), and Page Up and Page Down move it one page of visible matches at a
time, clamped like Up and Down; every highlight move scrolls the panel's list only as far as
needed to keep the row in view, and Enter or `Jump` plays the highlighted row — or a row click plays
that row — closing the panel; `Close` or a click on the backdrop closes it, and it opens only on
those song lists. A `Reload` button beside `Sort`
refetches the library's artists list and returns to the Artists list, clearing any active search
while leaving the typed query in the search box. A `Save list` button beside `Reload` opens a
transient naming panel over the showing list, like Winamp's playlist editor's Save list button:
type a name and press Enter (or the panel's `Save` button) to save the showing song list's song
ids under that name in showing order — the rows themselves are not copied — with an empty box
keeping the panel open with an inline `Type a name` note, a name containing `:` showing
`Use it without a colon` (the settings line's separator is the colon), and a name carrying a line
break (a multi-line paste can put one in the box) showing `Use one line`; typing clears the note.
Saving over an existing list's name (any case) keeps one saved list and replaces only its ids,
keeping the name as first saved, and the showing list is left showing. The panel's button row reads
`Save`, `Delete`, `Close`: the box's Enter keeps submitting `Save`, and deleting is the button only
— typing a saved list's name (any case) and pressing `Delete` removes that list and its
Playlists-view row, keeping every other saved list in stored order, and the shortened list survives
relaunches, and a name carrying a line break (a multi-line paste can put one in the box) shows
the same `Use one line` note the save flow uses. An unmatching name shows an inline `No such
playlist` note and removes nothing (with the panel staying open, like every other refused name);
deleting a list whose songs are showing
changes nothing about the showing rows or playback — it touches only the saved list. The panel's
backdrop, `Close` button, or Escape closes it without saving, and opening any other panel closes it too.
The press changes nothing when the showing list is not a song list the shared songs buffer holds
(an album's or server playlist's songs, a search result, a genre/year/composer group, or a saved
list's songs) or when that list is empty, loading, or failed — like Sort's no-ops, no panel
opens, nothing is persisted, and no fetch is disturbed. The title bar's `M` button (or `Ctrl+F1`) opens
a transient About panel naming the app, showing its `v`-prefixed package version, the line
`The classic Winamp look, rebuilt in Rust — with Apple Music as the library.`, and the active
library source (`Library: Apple Music` once a MusicKit session is stored, `Library: Sample
library` otherwise); `M` (through the backdrop over its slot), `Ctrl+F1`, the panel's `Close`
button, or a click on the backdrop outside the panel closes it, and opening it while the window is
rolled up restores the window first. The LCD's `O` letter, Alt+O, the lightning button at the
end of the transport row, or a right-click anywhere on the player (title bar, LCD, seek strip,
transport row, equalizer, or the rolled-up shade strip, but not the library), like Winamp's
main-window menu, opens an `OPTIONS` menu over the player listing `Always on Top: On/Off`,
`Time: Remaining/Elapsed`, `Autoscroll song name: On/Off`, `Visualizer: Shown/Hidden`, `Equalizer: Shown/Hidden`,
`Library: Shown/Hidden`, `Stop with fadeout`, `Stop after current: On/Off`, `Repeat: On/Off`, `Shuffle: On/Off`, `Manual playlist advance: On/Off`, `10 songs back`, `10 songs forward`, `Back 5 seconds`, `Fwd 5 seconds`, `Start of list`, `End of list`, `Jump to time`, `Jump to file`, `File info`, `About Nostalgia`, and `Close`, with each label reflecting the current
state. `Autoscroll song name: Off` stops a too-long title's automatic scrolling — the roller coasts to a stop at the offset it had, the hand drag still scrolls the title by holding the left button on it, and turning the row back on resumes the scroll from the stopped offset — while its LCD tick duties (the mixer-level readout flashes and the fade-out countdown) keep running. Choosing a row applies that one toggle and closes the menu; `Stop with fadeout` fades out and stops exactly like Shift+V while a preview plays and stops like V while paused or stopped; `10 songs back` and `10 songs forward` jump ten songs back or forward exactly like keypad 1 and 3 (doing nothing beyond closing the menu with no song list showing); `Back 5 seconds` and `Fwd 5 seconds` seek five seconds back or forward exactly like the Left and Right arrows (scheduling no work, exactly like the arrows); `Start of list` and `End of list` jump to the first or last song of the list exactly like Ctrl+Z and Ctrl+B (only closing the menu with no song list showing); `Jump to time` and `Jump to file` open the Ctrl+J and J panels (doing nothing beyond closing
the menu while that panel is unavailable); `File info` opens the Alt+3 File info panel for the showing song list's selected row when one
is picked and for the current track otherwise (doing nothing beyond closing the menu when there
is neither a song row selected nor a current track); `About Nostalgia` opens the
About panel, and `Close`, a click on the backdrop, or Escape closes the menu. Opening it while the
window is rolled up restores the window first, and the menu is not saved. Alt+O also closes it
and works while the search box has focus, without changing the query. Opening highlights the
first row; Up and Down move the highlight, stopping at either end, and Enter invokes the
highlighted row even if the covered search box has focus, without submitting a search. Mouse
clicks keep the same behavior. A
plain or Shift-held Escape press closes whichever transient panel is open, like its `Close`
button or the backdrop; with no panel open Escape keeps its existing behavior. Its
shade button, a double-click on the bar, or a double-click on the rolled-up strip rolls the window
up to the title bar plus a compact transport strip with Previous, Play, Pause, Stop, Next, the same
elapsed-over-total time readout, a seek slider over the preview's own length, and a volume slider,
where Play starts or resumes and Pause pauses, and back. Its `–` and `✕` buttons minimize and close. The window title (the name the desktop
environment shows for the app) names the current track as
`<number>Artist - Title - nostalgia` (omitting the number or artist when empty), ending in ` [Paused]` while paused and ` [Stopped]`
while stopped, like Winamp's taskbar title. It keeps the interrupted track's name across Stop and
reads just `nostalgia` when no known track is current.
With the window focused, the classic Winamp keys drive playback: Z and B step back and forward
(a Z press restarts the loaded preview from its start once it has played at least three seconds),
X plays, C pauses (or resumes while paused), V stops, Shift+V fades the playing song out over about two seconds and then stops (the volume setting is left unchanged; while paused or stopped it acts as Stop; a Shift-held click on the Stop button, in the transport row, the shade strip, or the library footer, and the options menu's `Stop with fadeout` row do the same, while a plain click stops at once), Ctrl+V arms (or disarms) Stop after current — the playing song then
plays to its natural end and playback stops instead of advancing, once, shown only by the
options-menu row (`Stop after current: On/Off`), R toggles Repeat and S toggles Shuffle, A (or Ctrl+A) toggles
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
and Ctrl+B plays its last song. A Ctrl-held click on the Previous or Next button, in the
transport row, the shade strip, or the library footer, does the same, while a plain click steps one
song. Ctrl+W rolls the window up into shade mode and back, like the title
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
toggles a loaded preview, starts a relaunched session's restored last-played track when nothing
is loaded and the service remembers no URL, and otherwise stops a track with none. Previous — the button, the Z key, and the system Previous track key — restarts the
loaded preview from its start once it has played at least three seconds, and steps back before
that; with Shuffle on, Next picks a song not yet played this round (every song plays once before
any repeats) and such an earlier Previous retraces the
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
(`Seek to: 01:23/03:30 (39%)`) the same way. An up/down-arrow, keypad 8/2, or wheel volume nudge
shows the same `Volume: N%` readout for about a second, like Winamp. A wheel roll over the balance
slider, the preamp, or a band shows that slider's readout the same way. The equalizer panel's on/off button,
sliders, and presets change the playing preview's curve live. The `AUTO` button loads a preset
automatically when a song is selected: while it reads `AUTO: On`, a song whose trimmed primary
genre matches a preset name case-insensitively (say `rock` or `Rock`) applies that preset's whole
curve and names it in the pick list, while a blank or unmatched genre leaves the current curve
alone; `AUTO: Off` never changes the curve. Clicking `+12 DB`, `0 DB`, or
`-12 DB` sets all ten bands to that gain and leaves the preamp alone; the volume and balance are
remembered for the next track. The pick list's top row, `Save preset…`, opens a naming panel:
type a name and press Enter (or the panel's `Save` button) to save the equalizer's applied curve
— its stored preamp and band gains — under that name, and the panel's backdrop, `Close` button,
Escape, or opening any other panel closes it. An empty name keeps the panel open with an inline
`Type a name` note, a name already used by a built-in (any case) shows an inline `Already a
preset` note and saves nothing, a name containing `:` shows `Use it without a colon` (the
settings line's separator is the colon), and a name carrying a line break (a multi-line paste
can put one in the box) shows `Use one line`; typing clears the note. Saving over an existing custom's
name (any case) keeps one row and replaces only its curve, keeping the name as first saved. The
panel's button row reads `Save`, `Delete`, `Close`: the box's Enter keeps submitting `Save`, and
deleting is the button only — typing a saved custom's name (any case) and pressing `Delete`
removes that custom and its pick-list row, keeping every other saved custom in stored order, and
the shortened list survives relaunches. A name no saved custom carries shows an inline `No such
preset` note and removes nothing (with the panel staying open, like every other refused name);
deleting the applied custom's name changes nothing about the playing curve or the `Custom`
label — it touches only the saved list, and AUTO, `eq_preset=`, and the live sliders are
untouched.
The pick list then offers the saved curves after the built-ins; choosing one applies its whole curve
(and the closed label reads `Custom` while it is applied, exactly as a hand-tuned curve does),
and the saved list survives relaunches in the settings file; an applied custom relaunches with
the label `Custom` and its curve restored, since the saved `eq_preset=` name keeps naming only
built-ins. Setting
`APPLE_MUSIC_DEVELOPER_TOKEN` at launch starts a browser-based MusicKit sign-in: browse and search
queries then read your Apple Music library through the REST API, and fall back to the built-in
sample library when it is unset. The settings file also saves the last successful sign-in's token
in a `music_kit_user_token=` line, and a later launch that has a saved token plus the environment
variable reuses that sign-in — no browser, no sign-in page, and the first library fetch is already
the signed-in one. A blank, malformed, or stale saved token (one Apple Music has stopped
answering) is ignored or fails over exactly like the unset case: the sign-in page opens again on a
stale or malformed one, a stale one's queries are answered by the sample library, and the saved
line is left alone until a fresh browser sign-in overwrites it; a user who wants a fresh flow
anyway deletes the `music_kit_user_token=` line (with the variable still set) and relaunches. The
token is stored unencrypted in the settings file like the rest of its content — delete the line to
force the browser flow — and the developer token itself is never saved: it keeps coming from the
environment, so a saved-token relaunch with the variable unset skips sign-in as today. Playback plays the selected song's Apple Music preview through
`rodio` when the library supplies one, and updates the UI state either way. Pause and Stop silence
the preview through the audio backend; a pause with no loaded preview stops the transport instead,
since there is nothing to pause. Play resumes a paused preview, restarts a stopped one from the
beginning, and restarts a playing one from its start. When a loaded preview reaches its end while playing, the player steps forward through the
showing list: to the next song, to the first song with Repeat on at the end, or, with Shuffle on, to
a shuffled song when another exists. Shuffle plays each song of the showing list once before any
repeats, like Winamp. With Repeat off on the last song it stops instead and keeps
that song queued for Play; with Shuffle on and Repeat off it likewise stops once every song of the
round has played, and Repeat on or Next starts a new round. With Manual playlist advance on (the
options menu's `Manual playlist advance: On/Off` row, persisted in the settings file like Repeat and
Shuffle), a song that plays to its end stops instead of advancing, whatever Repeat or Shuffle say;
Repeat, Next, Previous, Ctrl+Z/Ctrl+B, and the jump rows still move you, and turning it off restores
the normal end-of-song advance. The volume, balance, Repeat, Shuffle, and Manual playlist advance flags, the equalizer's on/off
state, AUTO choice, curve, and preset, the equalizer's saved custom preset curves, the song lists saved through the browse
`Save list` button (`user_playlist=<name>:<id>,<id>,…` lines, one per list), the songs you played
while signed out (`recently_played=<id>,<id>,…` one line, ids newest first), the elapsed/remaining readout choice, the visualizer's
shown/hidden choice, the autoscroll song-name choice, the equalizer and library panels' shown/hidden choices, the always-on-top
choice, the window-shade mode, the window's last screen position, the last-played track, the saved
MusicKit sign-in token, and the library's top-level browse list (Artists, All
Songs, All Albums, Recently Added, Recently Played, Playlists, Genres, Years, or Composers; a list opened from one of them
saves that one) are saved to `$XDG_CONFIG_HOME/nostalgia/settings.ini` (or
`$HOME/.config/nostalgia/settings.ini`) and restored at the next launch; `NOSTALGIA_SETTINGS_PATH`
overrides the path. The saved song lists survive relaunches too: the Playlists view lists them
again after the library's playlists, and opening one shows the saved order (ids the library no
longer answers are dropped); a `user_playlist=` line without a `:`, with a blank name, or with an
empty id list is skipped like every other malformed line, and the other lines still load. The signed-out Recently Played list is restored
at the next launch too, instead of starting over from the sample stand-in: the saved ids are
re-resolved at launch and the list reopens with those songs, ids the library no longer answers
being dropped like a saved list's stale ids; with a saved sign-in session the view keeps fetching
the service's own Recently Played list, and nothing is restored (a sign-in that races the
restore's answer drops the restored rows the same way). A blank or all-empty `recently_played=`
value is skipped like every other malformed line, and the other lines still load. The last-played track is restored at the next launch stopped: the Now Playing bar shows it, names it by title and artist once the launch can find the song — immediately from the built-in sample library when the saved id is a sample-library id, and after signing in from your Apple Music library otherwise — while the saved id text shows until then, Play — the transport button, the X key, or the media play/pause key — starts it again from the beginning, and the position within it is not remembered. The window reopens at its saved position without checking that a monitor is still there; delete the `window_position` line to reset an off-screen window. Wayland reports no window moves, so the position is not saved or restored there. A relative `XDG_CONFIG_HOME` is ignored (the XDG spec requires an absolute one), so the `$HOME/.config` location is used instead (or persistence is disabled when `$HOME` is unset). A missing or malformed entry falls back to its default (a `NaN` number counts
as malformed; a byte that is not UTF-8 spoils only its own line, and a leading UTF-8 BOM is
ignored), and a settings file that exists but cannot be read falls back to defaults with one line
on stderr naming the file; the player then refuses to save for the rest of that run, printing a
line that names the file each time, so the unread values are not overwritten with defaults. A path
that is not a regular file (a FIFO or device, say) and a file larger than 1 MiB are treated as
unreadable the same way, so a stray FIFO or device is not read; on Unix the open is nonblocking so
a FIFO cannot wait for a writer. A larger file stops at the cap plus one detection byte instead of
being loaded whole. Repair or remove that file and restart to save settings again. Each save
replaces the file atomically; a symlinked settings file stays a link, with the write going to
the file it points at and that file's mode kept. The always-on-top choice
(the LCD strip's `A` or the `A` key) is remembered for the next launch, and a saved-on launch starts
with the window pinned once boot resolves its window id. The shade mode (the title bar's shade
button, a double-click on the bar or the rolled-up strip, or Ctrl+W) is remembered too: a
saved-shaded launch opens rolled up and the same controls unroll it to the size the window opened
at. The library reopens on its saved top-level list, and a list showing when sign-in completes is
refetched from the signed-in library. `nostalgia --version` (or `-V`) prints the version and `nostalgia --help` (or `-h`) prints the
flags and environment variables, each exiting without opening the window; any other argument
prints an error and exits with status 2.
