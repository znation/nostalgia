# Plans

## Planned

_None yet._

### Show a selected song row's info in the File info panel, like Winamp's playlist editor (planned 2026-10-29 by the plan role)

**Goal.** The File info panel today names only the loaded track: it opens from Alt+3, the LCD
strip's `I`, and the options-menu `File info` row, and all three gate on
`WinampPlayer::current_known_track`. Winamp's file info dialogs act on the selection in the
list they belong to — the playlist editor's Alt+3 shows the highlighted row's item, not the
playing one — and Nostalgia's song lists are the playlist analog, so a user who wants a
song's year, composer, or rating currently has to play it (touching `known_tracks`, the
marquee, and the window title) just to read it. Show the showing song list's selected row
instead when one is picked, keeping the loaded track as the fallback and leaving playback
untouched.

**Approach.** One pure resolver decides what the panel names, and the gate that guards the
panel switches to it; every open path (Alt+3, `I`, the options row) shares it, so all three
act on the selection when one is picked and on the current track otherwise, exactly like
Winamp's per-window info actions sit on one resolution.
- `src/ui/views/now_playing/text.rs` (where [`KnownTrack`] lives): add a constructor
  `KnownTrack::from_song(song: &crate::library::Song, number: String)` that maps a song's
  fields exactly as `handlers::playback::track_selected`'s `known_tracks.insert` does today
  — title, artist, album, year, genre, track number, disc number, composer, content rating,
  `duration_ms`, `time` from `crate::ui::views::format_track_time(song.duration_ms)`, and
  `number` as passed. Document that the File info resolver passes `""` for `number` (the
  panel never renders the playlist-number prefix) and that a display format line is not part
  of `KnownTrack`.
- `src/ui/handlers/playback.rs`: in `track_selected`, replace the struct literal at the
  `player.known_tracks.insert` site with `views::KnownTrack::from_song(song, format!("{}. ",
  index + 1))`. The mapping is unchanged — the existing
  `known_tracks_record_the_played_songs_*` tests in `src/ui/tests/readouts_tests.rs` pin it
  — and the two construction sites can never drift apart.
- `src/ui/player/presentation.rs`: add
  `pub(in crate::ui) fn file_info_track(&self) -> Option<(views::KnownTrack,
  Option<crate::audio::PlaybackFormat>)>`, beside `current_known_track`. It matches the
  showing view: `CurrentView::Songs` reads `player.songs`, `CurrentView::AllSongs` reads
  `player.all_songs`, `CurrentView::RecentlyPlayed` reads `player.recently_played`; when that
  list's `selected()` is `Some` and in range, it resolves the row's `Song` and returns
  `(KnownTrack::from_song(song, String::new()), None)` — an ephemeral entry, never stored
  into `known_tracks`, with no format line because no preview is loaded for it. Any other
  current view, no selection, and an unmatched current view falls back to the current track:
  `current_known_track()` cloned with `self.playback_format`. `BrowseList::selected` already
  drops stale selections whenever the rows are replaced or reordered
  (`src/ui/browse.rs`), so no staleness re-check is needed; say so in the doc comment.
- `src/ui/view/overlay.rs`: the `file_info_visible` branch resolves through
  `player.file_info_track()` instead of `current_known_track()`, passing the returned track
  and format (which stays `Some` only for the current-track fallback) to
  `views::view_file_info_overlay(base, &track, format)`.
- `src/ui/handlers/window.rs`: `toggle_file_info`'s open gate becomes
  `player.file_info_track(None-guard)`: when `file_info_track()` is `None` the flag stays as
  it was (closing is untouched). Its doc comment loses "needs a current track resolved in
  known_tracks" for "needs either a current track resolved in `known_tracks` or a selected
  song row in the showing song list". Nothing else in the handler changes: opening still
  closes About and Options, opens through shade restore, and is not persisted.
- Tests:
  - `src/ui/tests/readouts_tests.rs` (where the known-track tests live), in their style:
    `file_info_track_resolves_the_selected_song_row` (a player with `all_songs` loaded, a row
    selected through `select_row`, both with and without a current track → the returned
    title, artist, album, year, and formatted length name the selected song, `number` is
    empty, and the format part is `None`) and
    `file_info_track_falls_back_to_the_current_track_without_a_selection` (selection cleared,
    and a selection in the Genres/Years list readings → the current track's entry and
    `playback_format` come back).
  - `src/ui/tests/view_tests.rs` (which holds the known-track root-view tests seeded via
    `known_tracks`, per the finished plan at PLANS-DONE.md's root-view seed entry):
    `file_info_overlay_names_the_selected_rows_song` — a song list showing with a selected
    row, `file_info_visible = true`, and a different track playing → the panel's text says
    the selected song's title, not the playing one's, and no `Format:` line renders; and the
    no-selection case still names the playing track with its format line, as today.
- Docs:
  - `docs/usage.md`: in the options-menu sentence, amend the `File info` clause: it opens the
    File info panel for the showing song list's selected row when one is picked (album songs,
    All Songs, or Recently Played — a search-result list included, since it shares its song
    list), and for the current track otherwise; with neither, the menu closes and nothing
    opens. In the keyboard paragraph at the Alt+3 mention, say the same for the `I` letter:
    both act on the selection and name the loaded track without one, and the selected row's
    panel shows no `Format:` line because no preview is loaded for it. Note that opening it
    does not play the row or touch the Now Playing bar or window title.
  - `docs/invariants.md` `## File info panel`: in the Guarantee's gate bullet, change
    "Opening needs a current track found in `known_tracks`
    (`WinampPlayer::current_known_track`)" to name the two resolution sources through
    `WinampPlayer::file_info_track` (current track from `known_tracks`, or the showing song
    list's selected row built by `KnownTrack::from_song` and never stored), keep the rest of
    the panel's lines as-is, and add that the `Format:` line renders only under the
    current-track fallback.

**Files touched.** `src/ui/views/now_playing/text.rs`, `src/ui/handlers/playback.rs`,
`src/ui/player/presentation.rs`, `src/ui/view/overlay.rs`, `src/ui/handlers/window.rs`,
`src/ui/tests/readouts_tests.rs`, `src/ui/tests/view_tests.rs`, `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**
- With a song list showing (album songs, All Songs, a playlist's songs, Recently Played, or a
  search-result list sharing it) and a row selected, Alt+3, the LCD `I`, and the options-menu
  `File info` row open File info naming that row's song — its title, artist, album, year,
  genre, track and disc numbers, composer, rating, and formatted length, and no `Format:`
  line; the playing track, the marquee, the window title, and `known_tracks` are untouched,
  and pressing the row still only selects as the existing double-click-to-play rules say.
- With no song list showing, with no row selected, or with a selected row in Artists, Albums,
  Playlists, or the Genres/Years/Composers group lists, the panel names the current loaded
  track exactly as today, including the `Format:` line while a preview reports one; with
  neither a selection nor a current track, none of the three paths open anything.
- Escape, Close, and the backdrop dismiss the panel as before; nothing about the panel is
  persisted; Alt+3 combos other than plain Alt+3 keep mapping to nothing per
  `shortcuts::file_info_shortcut`.
- `make check` passes.

## Done
