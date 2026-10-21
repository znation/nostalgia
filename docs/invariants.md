# Invariants

Project guarantees that bugs have had to reconstruct, one section per subsystem. Each section
states the guarantee, names the code that enforces it, and names the tests that fail if it is
removed. When a change alters a guarantee, update its section in the same commit. Each section
cites the `BUGS-FIXED.md` entry it was reconstructed from.

## Browse hierarchy

**Guarantee.** Artists is the top browse level. Every level below it has a Back action that steps
exactly one level up, and Back never falls through to the wrong level:

- Albums → Artists.
- Songs → Albums when the songs were opened from an artist's album list.
- Songs → All Albums when the songs were opened from the All Albums list.
- Songs → Playlists when the songs were opened from a playlist.
- All Songs → Artists.
- All Albums → Artists.
- Playlists → Artists.
- An active search's result list → Artists, not the album the search was issued from.

Artists itself has no level above it, so Back there is a no-op (and the Back button is not shown).

**Enforced by.** The `Message::Back` arm in `src/ui/mod.rs` checks `search_active` first, then
matches on `current_view` and `songs_origin` to pick the one level up. `views::can_go_back` in
`src/ui/views/browse.rs` (re-exported by `src/ui/views/mod.rs`) decides whether the Back button is
shown; it returns false only for `CurrentView::Artists`.

**Pinned by.**

- `back_from_albums_returns_to_artists`
- `back_from_songs_returns_to_albums`
- `back_from_artists_is_a_noop`
- `back_from_all_songs_returns_to_the_artists_list`
- `back_from_all_albums_returns_to_the_artists_list`
- `back_from_playlists_returns_to_the_artists_list`
- `playlist_selected_in_playlists_loads_its_songs_and_back_returns_to_playlists`
- `back_leaves_an_active_search_for_the_artists_list`
  (all in `src/ui/tests.rs`)
- `back_button_is_available_only_below_the_artist_list` (`src/ui/views/tests.rs`)

**History.** Reconstructed from "Browse view can only move forward — no way back from
Albums/Songs, a dead end" in `BUGS-FIXED.md`. The All Songs, All Albums, and Playlists levels
were added later and are covered here too.

## Browse reply freshness

**Guarantee.** A fetched browse reply — success or failure — is stored only when no newer request
has been issued for that list. A slow reply from an abandoned request must not overwrite the newer
list or record its error over it. A selection press likewise resolves a row only while the list
that rendered it is still current: a press that outlived its list is a no-op.

**Enforced by.** Two checks on `RequestGeneration` in `src/ui/loading.rs` and `BrowseList` in
`src/ui/browse.rs`:

- `RequestGeneration::is_current` runs on iced's executor thread when the fetch result is mapped
  to a message; a superseded result maps to `Message::Ignored` instead of a `*Loaded`/`*LoadFailed`
  message (see `loading::fetch_into`).
- `BrowseList::is_current` re-checks the same request counter at the store on the UI thread, in
  the `Message::BrowseReply` arm of `update` in `src/ui/mod.rs`, closing the window where a
  navigation issues a newer request between the executor check and the store.
- `BrowseList::select` rejects a selection whose epoch no longer matches the list's epoch (the
  list was replaced after the row rendered) or whose index no longer names a row.
  `store`/`store_with_row_text` bump the epoch on replacement; `clear` deliberately leaves it alone,
  so the replacing list's epoch still differs from the cleared one's. Selection messages carry the
  epoch of the list that rendered the pressed row.

**Pinned by.**

- `a_slow_stale_browse_reply_does_not_overwrite_the_newer_list`
- `a_slow_stale_browse_failure_does_not_overwrite_the_newer_list_or_error`
- `a_reply_superseded_between_the_mapper_and_update_is_not_stored`
- `a_stale_songs_reply_superseded_before_update_is_not_stored`
- `a_stale_albums_reply_superseded_before_update_is_not_stored`
- `stale_flat_list_replies_superseded_before_update_are_not_stored`
- `selection_messages_with_a_stale_epoch_or_index_do_nothing`
  (all in `src/ui/tests.rs`)

**History.** Reconstructed from "A browse reply is stored after a newer request was issued, so a
navigation shows the previous level's rows" in `BUGS-FIXED.md`.

## Now Playing label

**Guarantee.** The Now Playing bar shows the current track's human-readable title and artist, not
its internal id, whenever the id is in the known-track index:

- A known id renders that track's title and artist, borrowed from the index.
- An unknown id falls back to the raw id for the label and to the empty string for the artist.
- No current track renders the label "Nothing" and an empty artist.
- Stop keeps the interrupted track's title (the current track is not cleared by Stop).

**Enforced by.** `views::now_playing_label` and `views::now_playing_artist` in
`src/ui/views/now_playing.rs` (re-exported by `src/ui/views/mod.rs`), pure functions over the
known-track index. `WinampPlayer::now_playing_label` and `WinampPlayer::now_playing_artist` in
`src/ui/mod.rs` resolve against `known_tracks`, so the bar keeps naming a playing track after
browsing to another album replaced the visible song list.

**Pinned by.**

- `now_playing_label_resolves_known_track_to_title`
- `now_playing_label_falls_back_to_id_when_track_not_in_tracks`
- `now_playing_label_falls_back_to_id_when_tracks_is_empty`
- `now_playing_label_shows_nothing_with_no_current_track`
- `now_playing_artist_resolves_known_track_to_artist`
- `now_playing_artist_is_empty_when_track_not_in_tracks`
- `now_playing_artist_is_empty_with_no_current_track`
- `now_playing_label_borrows_the_title_and_nothing_literal`
- `now_playing_artist_borrows_the_artist`
  (all in `src/ui/views/tests.rs`)
- `now_playing_label_keeps_the_track_name_after_browsing_to_another_album` (`src/ui/tests.rs`)

**History.** Reconstructed from "Now Playing bar shows the track id instead of the song title" in
`BUGS-FIXED.md`.
