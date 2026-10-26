# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the README's Usage section: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. Record any new drift here rather than appending
> another note.

## Planned

### Show the loaded preview's format in the File info panel, like Winamp (planned 2026-10-26 by the plan role)

**Goal.** Winamp's File info completes the current file's details with its format. Nostalgia's
panel names the title, artist, album, and length, and its LCD status row already shows the loaded
preview's average bitrate, sample rate, and channel mode. Add the same values to the File info
panel as one `Format: …` line when `WinampPlayer::playback_format` is `Some`, reusing
`views::now_playing::format_playback_prefix` and `views::now_playing::channel_mode_label`. No new
message, state, handler, or persistence behavior.

**Approach.**

- `src/ui/views/file_info.rs`:
  - Import `crate::audio::PlaybackFormat` and
    `super::now_playing::{channel_mode_label, format_playback_prefix}`; add
    `const FORMAT_LABEL: &str = "Format: ";` beside the existing labels.
  - Change `view_file_info_overlay` to take `format: Option<PlaybackFormat>` after `track`. After
    the length line and before the `Close` button, push
    `Text::new(format!("{FORMAT_LABEL}{} {}", format_playback_prefix(format),
    channel_mode_label(format.channels)))` only when `format` is `Some`. With `None` the panel
    draws no format line. The title line, the empty-artist and empty-album omissions, the length,
    and the `Close`/backdrop behavior stay as they are.
  - Update the module and function docs to list the format line and say it appears only while the
    loaded preview reports a format.
- `src/ui/view.rs`: in the File info overlay branch, pass `player.playback_format` as the third
  argument. This is the same value the LCD status row draws; it stays through Pause and is cleared
  by Stop or a new selection in `handlers::playback::reset_position_readout`, so the panel keeps
  its format line while paused and drops it once that preview is gone.
- `src/ui/views/tests/chrome.rs`: make the `file_info_element` helper take
  `Option<PlaybackFormat>` and pass `None` from the existing close/backdrop test. Leave
  `file_info_overlay_draws_the_title_artist_album_and_length` and
  `file_info_overlay_omits_the_album_line_when_the_source_supplied_none` passing `None`; add
  `file_info_overlay_draws_the_loaded_previews_format`, asserting
  `Format: 256 KBPS  44 KHZ STEREO` for
  `PlaybackFormat { sample_rate_hz: 44_100, channels: 2, bitrate_kbps: Some(256) }` and
  `Format: 44 KHZ MONO` for a format with `bitrate_kbps: None, channels: 1`, and add
  `file_info_overlay_omits_the_format_line_without_a_loaded_preview`, asserting no captured text
  starts with `"Format:"` when the argument is `None`.
- `src/ui/tests/view_tests.rs`: add a root-view test that seeds a `known_tracks` entry and
  `current_track` (as the shaded File info check already does), sets `file_info_visible` and a
  `PlaybackFormat`, and captures the root view; it must contain the `Format: …` line, and clearing
  `playback_format` must remove it. This pins the `ui::view::view` wiring, not just the pure view.
- `docs/invariants.md` `## File info panel`: add the `Format: …` line (only while
  `WinampPlayer::playback_format` is `Some`, formatted from
  `views::now_playing::format_playback_prefix` and
  `views::now_playing::channel_mode_label`) to the Guarantee, name those functions in
  **Enforced by**, and add the new chrome and root-view test names to **Pinned by**.
- `README.md` (Usage, outside the brief markers): extend the Alt+3 sentence to say the panel also
  shows the loaded preview's format (`256 KBPS  44 KHZ STEREO`) while one is reported. Do not touch
  the opening `O A I D V` sentence or the `I` route, which the waiting strip plan owns.

**Files touched.** `src/ui/views/file_info.rs`, `src/ui/view.rs`,
`src/ui/views/tests/chrome.rs`, `src/ui/tests/view_tests.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**

- `view_file_info_overlay` with `Some(format)` draws one `Format:` line after `Length:` and before
  `Close`, using the LCD status row's existing `"256 KBPS  44 KHZ"`/`"44 KHZ"` prefix plus
  `MONO`/`STEREO`; with `None` it draws no `Format:` line, and every existing line and the
  `Close`/backdrop presses are unchanged.
- The root `view` passes `player.playback_format` to the panel, so a File info panel open while a
  preview is loaded shows its format, and after Stop (which clears that value) the panel keeps its
  title/artist/album/length but drops the format line.
- No playback, browse, selection, search, settings, or persistence behavior changes.
- `make check` passes, including `cargo doc` with no broken intra-doc link.

### Jump to File from a showing song list with `J`, like Winamp (planned 2026-10-26 by the plan role)

**Goal.** Winamp's `J` opens a Jump to File dialog that filters the playlist and jumps to a match;
Nostalgia has Ctrl+J's Jump to time panel but no Jump to File. Add a transient Jump to File panel
over the showing song list (the browsed Songs list, including album/playlist songs and search
results, or the flat All Songs list): typing filters its cached row titles, Enter or `Jump` plays
the first match, and clicking a row plays it. It reuses the existing `TrackSelected` playback path,
so no library, REST, audio, browse-state, or persistence behavior changes.

**Approach.**

- `src/ui/views/jump_file.rs` (new):
  - `pub const JUMP_FILE_INPUT_ID: &str = "nostalgia-jump-file";` beside the Jump to time box's id.
  - `pub(crate) fn jump_file_matches(titles: &[String], query: &str) -> Vec<usize>`: trim the query;
    an empty query matches every index in order, otherwise keep the index of every title whose
    lowercased text contains the lowercased query, preserving list order. The view and the submit
    handler both call this one function, so the rows shown and the row Enter plays cannot drift.
  - `pub fn view_jump_file_overlay<'a>(base: Element<'a, Message>, query: &'a str, titles: &'a [String], epoch: u64) -> Element<'a, Message>`:
    build a `JUMP TO FILE` heading, a `TextInput` with `JUMP_FILE_INPUT_ID`, placeholder
    `"Filter songs"`, `Message::JumpFileChanged` on input and `Message::JumpFileSubmitted` on
    submit (styled with `style::chrome_text_input_style`), then a fixed-height `Scrollable` of one
    `Button` per match showing `titles[index]` and publishing
    `Message::JumpFileRowSelected { epoch, index }` (styled with `style::playlist_row_style`). When
    there are no matches, draw `No songs` for an empty list and `No matching songs` otherwise, then
    `Jump` (`Message::JumpFileSubmitted`) and `Close` (`Message::ToggleJumpFile`) buttons. Stack it
    with `modal::modal_overlay(..., Message::ToggleJumpFile)`. No state or playback is touched here.
- `src/ui/views/mod.rs`: declare `mod jump_file;` and re-export `JUMP_FILE_INPUT_ID`,
  `jump_file_matches`, and `view_jump_file_overlay`; extend the module doc's panel list.
- `src/ui/message.rs`: add `ToggleJumpFile`, `JumpFileChanged(String)`, `JumpFileSubmitted`, and
  `JumpFileRowSelected { epoch: u64, index: usize }`, with comments naming the panel and the
  `TrackSelected` dispatch.
- `src/ui/player.rs`: add `jump_file_visible: bool` and `jump_file_query: String` after the Jump to
  time fields, documented as transient, not persisted, and mutually exclusive with the other
  transient panels; initialize them in `WinampPlayer::new`.
- `src/ui/handlers/jump.rs`: add a private `song_list(player) -> Option<(&[String], u64)>` that
  returns `songs.titles`/`songs.epoch` for `CurrentView::Songs` and
  `all_songs.titles`/`all_songs.epoch` for `CurrentView::AllSongs`, and `None` otherwise (the same
  view-to-buffer rule `track_selected` and `showing_songs` use). Then:
  - `toggle_jump_file`: if visible, clear the flag and return no task; otherwise return no task
    unless `song_list` is `Some`. Opening sets the flag, clears the query, closes About, File info,
    and Jump to time, focuses `JUMP_FILE_INPUT_ID`, and while shaded calls `toggle_window_shade`
    first (exactly `toggle_jump_time`'s shape).
  - `jump_file_changed`: store the typed text; the view re-filters live.
  - `jump_file_submitted`: when the panel is open, call `jump_file_matches` on the current list; a
    missing first match leaves the panel and query alone, otherwise clear the flag and return
    `Task::done(Message::TrackSelected { epoch, index })` for the first match's source index.
  - `jump_file_row_selected`: when the panel is open, clear the flag and return
    `Task::done(Message::TrackSelected { epoch, index })`; the existing epoch/index guards in
    `TrackSelected` reject a stale press.
  - In `toggle_jump_time`'s opening branch, also clear `jump_file_visible`.
- `src/ui/handlers/window.rs`: in the opening branches of `toggle_about` and `toggle_file_info`,
  also clear `jump_file_visible`; leave their other behavior alone.
- `src/ui/mod.rs`: route the new variants in `update` to the handlers above.
- `src/ui/view.rs`: after the Jump to time branch, add a `jump_file_visible && !shaded` branch that
  matches `CurrentView::Songs`/`AllSongs` onto `view_jump_file_overlay` with that list's cached
  `titles` and `epoch`, falling back to `base` for any other view.
- `src/ui/shortcuts.rs`: map plain `j`/`J` (Shift allowed, Control/Alt/Logo rejected by the existing
  guard) to `Message::ToggleJumpFile` in `shortcut`, leaving Ctrl+J on `ToggleJumpTime` and
  Ctrl+Shift+J unbound. Update the module and `shortcut` doc comments to list `J`.
- `src/ui/views/tests/jump_file.rs` (new; declare in `src/ui/views/tests/mod.rs`): test
  `jump_file_matches` for blank/all, case-insensitive and trimmed matches, order, and no match;
  capture the overlay and assert `JUMP TO FILE`, the matching row title, the omitted non-matching
  title, `Jump`, and `Close`; drive presses on a row, `Jump`, `Close`, and the backdrop and assert
  `JumpFileRowSelected { epoch, index }`, `JumpFileSubmitted`, and `ToggleJumpFile` respectively;
  assert the empty-list and no-match wordings.
- `src/ui/tests/jump_file_tests.rs` (new; declare in `src/ui/tests/mod.rs`): with a `Songs` fixture
  built from `stepping_songs` and aligned titles, assert opening clears a stale query and closing
  works; opening from Artists/Albums/Playlists schedules nothing; submit plays the first match via
  `TrackSelected` and closes (`assert_track_selected`); a no-match submit leaves the panel and query
  alone; a row press dispatches the clicked index; Jump to File and About/File info/Jump to time
  close each other; opening while shaded restores the window; and a root-view capture over a
  `Songs` fixture with the flag set draws `JUMP TO FILE`, the filtered row, and not the filtered-out
  row.
- `src/ui/shortcuts.rs` tests: add `message_for_maps_j_to_jump_file`, and update
  `message_for_maps_ctrl_j_to_jump_time` so plain `j` now expects `ToggleJumpFile` while the
  Control chords still expect `ToggleJumpTime` or nothing.
- `docs/invariants.md`: add a `## Jump to File panel` section after `## Jump to time panel` with the
  shortcut, the Songs/All Songs gate, the filter wording, Enter/Jump and row-click dispatch through
  `TrackSelected`, the mutual-exclusion rule with the other transient panels (and the shade
  restore), the no-persistence rule, the enforcers, and the new test names.
- `README.md` (Usage, outside the brief markers): after the Ctrl+J sentence, add a `J` sentence
  naming the filtered showing song list, Enter/`Jump` and row-click playback, the closing routes,
  and the song-list-only gate. Leave the Alt+3 sentence for the waiting format plan.

**Files touched.** `src/ui/views/jump_file.rs`, `src/ui/views/mod.rs`,
`src/ui/views/tests/jump_file.rs`, `src/ui/views/tests/mod.rs`, `src/ui/message.rs`,
`src/ui/player.rs`, `src/ui/handlers/jump.rs`, `src/ui/handlers/window.rs`, `src/ui/mod.rs`,
`src/ui/view.rs`, `src/ui/shortcuts.rs`, `src/ui/tests/jump_file_tests.rs`,
`src/ui/tests/mod.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**

- Plain `J` publishes `ToggleJumpFile`; Ctrl+J still publishes `ToggleJumpTime`, and the Control
  chord variants of plain J stay unbound. `J` opens only while `CurrentView::Songs` or
  `CurrentView::AllSongs` is showing; elsewhere it schedules nothing and leaves the flag false.
- The panel lists exactly the showing list's cached titles that contain the trimmed query
  case-insensitively, in list order; a blank query lists every title. An empty list reads
  `No songs` and a non-empty list with no match reads `No matching songs`.
- Enter or `Jump` plays the first matching row; clicking a row plays that row. Both close the panel
  and dispatch `TrackSelected { epoch, index }` with the showing list's epoch and the match's source
  index, so playback, `known_tracks`, and scroll behavior are the existing song-row path. A submit
  with no match keeps the panel open and leaves the query untouched.
- Opening Jump to File closes About, File info, and Jump to time; opening any of those closes Jump
  to File. Opening while shaded restores the window first. The flag is not persisted.
- No library model, REST, audio, browse list order, selection, search, settings, or other playback
  behavior changes.
- `make check` passes, including `cargo doc` with no broken intra-doc link.

## Done
