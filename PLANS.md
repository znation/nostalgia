# Plans

## Planned

_None yet._

### Save the showing song list as a named playlist the Playlists view reopens, like Winamp's Save list button (planned 2026-10-29 by the plan role)

**Goal.** Every song list the library shows — an album's songs, a server playlist's, a search
result, All Songs, a genre/year/composer group — is a one-shot view: a curve you like has its
named preset, but a list you assembled has no way to keep it. Winamp's playlist editor has a
Save list button beside its List options, and its load-list names reappear in the picker. Give
Nostalgia a `Save list` button in the browse button row: naming what is showing saves its song
ids under that name, the Playlists view lists the saved names under the library's playlists,
and opening one shows its songs like any song list. The saved lists survive relaunches in the
settings file. Editing a saved list (reordering, adding, deleting rows) stays out; saving the
same name again replaces its ids, as Winamp's save does. Server-side playlist creation is out
of scope too: the REST transport is read-only, so a saved list is local (`src/library.rs`'s
`Playlist` doc comment already calls the model a "user-created playlist").

**Approach.** The saved list stores only song ids — the same ids a row press resolves to
`play_track` — mirroring the `current_track` id persistence; opening one resolves each id
through the existing `resolve_song` seam and shows the songs through the in-memory group path
`show_group_songs` (`src/ui/handlers/browse.rs`) already used for genres and years.

- `src/settings.rs`: `pub struct SavedPlaylist { pub name: String, pub song_ids: Vec<String> }
  ` beside `LibraryView`, plus a `saved_playlists: Vec<SavedPlaylist>` field with the same
  getter/setter pair pattern the toggles use (`set_saved_playlists`). It follows the
  `visualizer_visible` pattern, not `Settings::from_state`: `persist_settings_if_changed`
  (`src/ui/player.rs`) writes it from a `WinampPlayer` field before the unchanged-snapshot
  comparison.
- `src/settings/text.rs`: `to_text` writes one `user_playlist=<name>:<id>,<id>,…` line per
  saved list, after `eq_preset=`; parsing gains a `parse_user_playlist` in the per-line match
  beside `parse_eq_preset`: a line without a `:`, a blank name, or one whose song list is empty
  is skipped like the other malformed lines; the text before the first `:` is the name and the
  comma-split, trimmed, non-empty tokens after it are the ids. Song ids are catalog digits or
  `song-N`-shaped sample ids, so comma-splitting is safe. Round-trip tests beside the existing
  `eq_preset` ones: two saved lists survive `to_text` → `from_text` with names and every id
  equal; a line with no colon or an empty id list yields an empty saved list; unknown keys stay
  ignored.
- `src/ui/player.rs`: `WinampPlayer` gains `saved_playlists: Vec<SavedPlaylist>` (loaded in
  `adopt_saved_settings`, written by `persist_settings_if_changed`) and the naming panel's two
  fields (`save_list_open: bool`, `save_list_name: String`), reset beside the Jump fields in
  `WinampPlayer::new`; the `SongsOrigin` enum gains a `UserPlaylist` variant (Back returns to
  `CurrentView::Playlists`, same as `SongsOrigin::Playlist`) and a `SAVED_PLAYLIST_ROW_ID`
  constant (value `nostalgia-saved-playlist`) for the sentinel row id below.
- `src/ui/message.rs`: mirror the Jump-to-time trio — `ToggleSaveList`,
  `SaveListNameChanged(String)`, `SaveListSubmitted` — plus `Message::UserPlaylistLoaded
  (Vec<Song>)`; the existing generic row-press message needs no widening.
- `src/ui/views/browse.rs`: a `view_save_list_button()` beside `view_sort_button()` in the
  Top/Sort/Reload row, publishing `Message::ToggleSaveList`. On a view that is not the shared
  songs buffer, or while its list is empty, loading, or failed, it is a no-op like Sort's
  no-ops; `playlists_loaded` (`src/ui/handlers/browse/fetch.rs`), after storing the fetched
  rows, appends one sentinel `Playlist` row per saved list (`id:
  SAVED_PLAYLIST_ROW_ID`, `name:` the saved name), so the Playlists view renders them with
  continued numbering through the ordinary row builder and the footer counts them. A load
  failure view is unchanged. Compare the button row against the docked-library reference
  (`winamp-screenshots/winamp-2.1-main-eq.gif`) and keep the existing button geometry.
- `src/ui/handlers/save_list.rs` (new, beside `handlers/jump.rs`): `toggle_save_list` obeys the
  transient-panel dismissal rules `docs/invariants.md`'s `## Transient panel dismissal` pins
  (closes the other panels, closes on backdrop and Escape-equivalents, no-op on non-song or
  empty/loading/failed lists); `save_list_name_changed`; `save_list_submitted` — trim the box;
  an empty name keeps the panel open with an inline `Type a name` note; a name containing `:`
  shows an inline `Use it without a colon` note (the settings line's separator); a name equal
  case-insensitively to an existing saved list replaces that list's ids in place; otherwise
  append. On success snapshot `player.songs.items`' ids in showing order, persist through the
  settings writer, and close the panel; the showing list is untouched (Winamp's save does not
  clear the editor). The panel's text input and notes mirror the Jump-to-time panel's build
  (`src/ui/views/jump_time.rs`) with its view wiring in `src/ui/view/overlay.rs`.
- `src/ui/handlers/browse.rs`: `playlist_selected` routes a pressed row whose playlist id is
  `SAVED_PLAYLIST_ROW_ID` to a new `user_playlist_selected` — it computes the saved list's
  index from the row's position among the trailing sentinel rows, clones the service (the
  cloneable `AppleMusicService` pattern other handlers use), and performs one future that
  resolves each id in order with `resolve_song` (ids the library no longer answers are
  dropped) and delivers `Message::UserPlaylistLoaded`; that handler stores the songs through
  `show_group_songs` with `SongsOrigin::UserPlaylist` (its `begin_fetch` makes an in-flight
  reply stale, as for genres). Rows in an opened saved list play, sort, reverse, shuffle, and
  report through the song-list footer exactly like genre songs.
- Document it: `docs/usage.md` — the button row paragraph (what `Save list` does, its no-op
  rules), the naming panel's Enter/backdrop rules and per-note errors, same-name save replaces,
  the Playlists view's saved rows under the library's, and the relaunch story; `docs/
invariants.md` gains a `## Saved playlists` section pinning the guarantee (id-only storage,
  case-insensitive same-name replace, colon refusal, persistence round trip, dropped unresolvable
  ids at open) to the new tests.

**Files touched.** `src/settings.rs`, `src/settings/text.rs`, `src/ui/player.rs`,
`src/ui/message.rs`, `src/ui/handlers/save_list.rs`, `src/ui/handlers/browse.rs`,
`src/ui/handlers/browse/fetch.rs`, `src/ui/views/browse.rs`, `src/ui/views/save_list.rs`,
`src/ui/view/overlay.rs`, `src/ui/mod.rs`, `docs/usage.md`, `docs/invariants.md`, and the
tests beside `src/settings.rs`'s round-trip block, `src/ui/tests/`, and `src/ui/views/tests/`.

**Acceptance criteria.**
- Open an album's songs, press `Save list`, type `Morning Drive`, press Enter — the panel
  closes, the Playlists view then lists `Morning Drive` under the library's playlists with
  continued row numbering, and the footer count includes it.
- Opening `Morning Drive` shows its songs numbered from 1, playing a row works, Back returns
  to the Playlists view, and Sort, reverse, Ctrl+Shift+R, and the song-list footer behave on
  it exactly as on a genre's songs.
- Saving again with `morning drive` (any case) replaces the saved ids in place; typing a name
  with `:` shows the `Use it without a colon` note; an empty box keeps the panel open with the
  `Type a name` note; a backdrop click or other transient-panel dismissal closes it exactly
  like Jump to time.
- Quit and relaunch: the Playlists view still lists `Morning Drive`, and opening it reproduces
  the saved order (ids that no longer resolve are dropped, in settings-parse and open rules
  alike); a `user_playlist=` line with no colon, a blank name, or an empty id list is skipped
  and the other lines still load.
- `Save list` on Artists, on a failed or loading list, and on an empty list changes nothing
  (no panel, no persist, no epoch bump).
- The server playlists' browse, press, and failure paths are unchanged for rows whose id is
  not the sentinel; the existing browse and settings tests pass unchanged.
- `make check` passes.

## Done
