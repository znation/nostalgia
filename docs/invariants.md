# Invariants

Project guarantees that bugs have had to reconstruct, one section per subsystem. Each section
states the guarantee, names the code that enforces it, and names the tests that fail if it is
removed. When a change alters a guarantee, update its section in the same commit. Each section
cites the `BUGS-FIXED.md` entry it was reconstructed from.

**How to use.** Before reconstructing an invariant that a fix's `unclear-invariant` validation
gap names, read this file for the matching subsystem section — it already states the guarantee,
the enforcing code, and the tests that pin it. When a fix's gap is `unclear-invariant`, or a
change alters a guarantee that has a section here, update that section in the same commit, or add
a new section when none fits. `BUGS.md`'s header points here for the tag.

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

**Enforced by.** The `Message::Back` handler — `browse::back` in `src/ui/handlers/browse.rs`,
routed from the `update` dispatch table in `src/ui/mod.rs` — checks `search_active` first, then
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
  the `Message::BrowseReply` handler (`browse::browse_reply` in `src/ui/handlers/browse.rs`),
  closing the window where a navigation issues a newer request between the executor check and
  the store.
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

## Browse row order

**Guarantee.** The Sort button reorders only the browse list currently showing, ascending by each
row's rendered text compared case-insensitively: Artists and Playlists by name, Albums and All
Albums by `artist - title`, and Songs and All Songs by `artist - title`. Rows whose keys compare
equal keep their library order. A song list reorders its cached titles and labels by the same
permutation as its songs, so each row keeps its own title and duration. The reorder is one-shot:
it bumps only that list's epoch, so a selection press rendered before the sort is rejected while
the post-sort indices resolve to the sorted rows, and it leaves the other lists, the current
view, the search state, and playback unchanged. Navigating away and back reloads the library's
order; no sort flag persists. Sorting an empty, loading, or failed list is a no-op: it neither
reorders rows nor bumps the epoch.

**Enforced by.** `BrowseList::sort_rows_by` in `src/ui/browse.rs` builds a stable index permutation
from the lowercased row keys and rebuilds `items`, `titles`, and `labels` by it, bumping the epoch
only when there are at least two rows and the list is not loading. `browse::sort_browse` in
`src/ui/handlers/browse.rs` matches the current view onto its list and key, routed from
`Message::SortBrowse` in the `update` dispatch table; `views::view_sort_button` in
`src/ui/views/browse.rs` draws the button beside Top.

**Pinned by.**

- `sort_reorders_artists_case_insensitively`
- `sort_reorders_albums_by_their_display_title`
- `sort_reorders_playlists_by_name`
- `sort_reorders_songs_and_keeps_their_cached_text_aligned`
- `sort_reorders_all_songs_and_keeps_their_cached_text_aligned`
- `sort_reorders_all_albums_by_their_display_title`
- `sort_bumps_the_epoch_so_a_pre_sort_press_is_rejected`
- `sort_with_an_empty_or_loading_list_is_a_noop`
  (all in `src/ui/tests.rs`)

## Now Playing label

**Guarantee.** The Now Playing bar shows the current track's human-readable title and artist, not
its internal id, whenever the id is in the known-track index:

- A known id renders that track's title and artist, borrowed from the index.
- An unknown id falls back to the raw id for the label and to the empty string for the artist.
- No current track renders the label "Nothing" and an empty artist.
- Stop keeps the interrupted track's title (the current track is not cleared by Stop).
- The LCD splits its time readouts: by default the large left-hand readout shows the backend's
  reported position as zero-padded `MM:SS` (for example `00:53`), and the right edge always shows
  the plain cached total (for example `3:30`). Clicking the large readout switches it to the
  track's remaining time as `-MM:SS` (for example `-02:37`) and clicking again returns to elapsed.
  A known track with no reported position (no preview
  loaded, a failed start, or a backend that cannot answer) reads `00:00` at the left and the
  cached total at the right; no current track, or an unknown id, reads `--:--` at both edges.
  Pause keeps the stored position, so the paused player's frozen elapsed value stays on the bar;
  Stop and a new selection clear it immediately to `00:00`. A position reply issued before a Stop
  or a new selection is dropped rather than restoring the previous elapsed value, and the poll
  stays suspended while any backend transport transition is in flight, so it cannot read the
  previous player under the new epoch. A selection whose song has no preview stops the previous
  preview, so the backend reports no position for the new track and the elapsed readout reads
  `00:00` while the right edge keeps the cached total.
- The chosen time mode is presentation-only and persists across Pause, Stop, Next/Previous, and a
  new selection until the readout is clicked again. In remaining mode a known track counts down
  from its own length, treating a missing position as zero and saturating at `-00:00` when the
  position is past the total; a known track whose source supplied no length (`duration_ms == 0`)
  and any unknown or absent track read `--:--` at the left. The right edge keeps the plain total in
  both modes.
- A combined artist/title longer than `MARQUEE_WIDTH` characters is windowed by
  `views::now_playing::marquee_text`, advanced one character per `Message::MarqueeTick`, and
  reset to the start by `TrackSelected`; a line that fits is shown whole and borrowed.
- The marquee timer is subscribed only while the line needs to scroll: `WinampPlayer::needs_marquee`
  measures the same label and artist the bar renders, so a line that fits leaves the timer out
  (every message makes iced rebuild the whole screen, browse list included).

**Enforced by.** `views::now_playing_label` and `views::now_playing_artist` in
`src/ui/views/now_playing.rs` (re-exported by `src/ui/views/mod.rs`), pure functions over the
known-track index. `WinampPlayer::now_playing_label` and `WinampPlayer::now_playing_artist` in
`src/ui/mod.rs` resolve against `known_tracks`, so the bar keeps naming a playing track after
browsing to another album replaced the visible song list. `views::now_playing::now_playing_elapsed`
(through `format_lcd_elapsed` for elapsed mode and `format_lcd_remaining` for remaining mode)
formats the left-hand readout as chosen by `views::now_playing::TimeDisplay`, while
`views::now_playing::now_playing_time` returns the plain cached total for the right edge; the
`playback_position` field on `WinampPlayer` stores the reply the
`PlaybackPosition` handler received, which is stamped with `position_epoch`, and
`reset_position_readout` bumps that epoch and clears the value
on Stop and on a new selection. `pending_position_transitions` counts the backend transport
transitions that are still in flight: every handler in `src/ui/handlers/playback.rs` that
schedules one begins a transition, and the completion that reports that same transition ends it;
while the count is nonzero the `PositionTick` handler issues no query and the
`PlaybackPosition` handler stores no reply. `AppleMusicService::play_track`
stops the backend's previous preview when a selection has no preview URL.
`views::now_playing::marquee_text`
windows an overlong line, and `WinampPlayer::marquee_offset` carries the advance through
`Message::MarqueeTick`. The `WinampPlayer::time_display` field holds the readout's mode, and
`handlers::playback::toggle_time_display` (routed from `Message::ToggleTimeDisplay`, pressed by
the `MouseArea` wrapping the readout in `now_playing_row`) flips it without touching playback
state. `views::now_playing::display_len` and `views::now_playing::needs_marquee`
measure the line, and `ui::timers::subscriptions` subscribes `marquee_ticks` only while that measure
exceeds `MARQUEE_WIDTH`.

The scrolling timer's stream is built by `ui::ticks::tick_stream` in `src/ui/ticks.rs`, and its
thread starts through `spawn_marquee_thread` in `src/ui/timers.rs`, which delegates
to `crate::threads::spawn_named`, the fallible `std::thread::Builder::spawn` every worker in the app
uses. `std::thread::spawn` panics when
the OS refuses a thread, and iced builds the subscription's stream on its main thread while it
updates the subscriptions, so a refused timer thread is reported and the title stops scrolling
rather than the panic unwinding the event loop.

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
- `now_playing_time_resolves_known_track_to_cached_label`
- `now_playing_time_returns_the_plain_total_even_with_a_position`
- `now_playing_elapsed_reads_placeholder_zero_or_position`
- `now_playing_elapsed_remaining_counts_down_from_the_total`
- `format_lcd_elapsed_pads_minutes_and_seconds`
- `format_lcd_remaining_prefixes_a_minus_and_pads`
- `toggle_time_display_switches_to_remaining_and_persists`
- `toggle_time_display_with_no_track_keeps_the_placeholder`
- `marquee_text_keeps_a_line_that_fits`
- `marquee_text_windows_from_the_offset`
- `marquee_text_wraps_through_the_gap_to_the_start`
- `marquee_text_counts_characters_not_bytes`
- `display_len_counts_the_label_the_artist_and_the_separator`
- `needs_marquee_only_for_a_line_too_long_for_the_lcd`
- `overlong_now_playing_line_renders_a_new_window_as_the_offset_advances`
  (all in `src/ui/views/tests.rs`)
- `now_playing_label_keeps_the_track_name_after_browsing_to_another_album` (`src/ui/tests.rs`)
- `now_playing_time_shows_the_played_tracks_duration_after_browsing_to_another_album` (`src/ui/tests.rs`)
- `now_playing_elapsed_shows_the_position_while_it_is_known`
- `pause_keeps_the_stored_elapsed_readout`
- `a_position_reply_after_a_seek_updates_the_readout`
- `track_selected_clears_the_elapsed_readout`
- `stop_clears_the_elapsed_readout`
- `position_replies_store_and_stale_ones_are_dropped`
- `position_poll_waits_for_a_stop_to_reach_the_backend`
- `position_poll_waits_for_a_selected_tracks_play_to_settle`
- `an_unrelated_transport_completion_keeps_the_poll_waiting`
- `position_tick_schedules_a_query`
- `position_tick_stream_emits_position_ticks`
- `a_refused_position_thread_ends_the_tick_stream_instead_of_panicking`
- `a_refused_position_thread_reports_the_cause_and_the_loss`
- `the_position_thread_is_named_nostalgia_position_poll` (`src/ui/tests.rs`)
- `play_track_without_a_preview_url_stops_the_previous_preview` (`src/apple_music/tests.rs`)
- `position_query_reports_none_when_nothing_is_loaded`
- `a_seek_is_reflected_by_the_next_position_query`
- `a_pause_keeps_the_loaded_player_reporting_its_position` (`src/audio/tests.rs`)
- `marquee_tick_advances_the_offset`
- `marquee_ticks_keep_ticking_when_the_receiver_falls_behind`
- `the_marquee_timer_runs_only_while_the_line_needs_to_scroll`
- `the_player_knows_when_the_now_playing_line_needs_the_marquee`
- `track_selected_resets_the_marquee_offset` (`src/ui/tests.rs`)
- `a_refused_marquee_thread_ends_the_tick_stream_instead_of_panicking`
- `a_refused_marquee_thread_reports_the_cause_and_the_loss`
- `the_marquee_thread_is_named_nostalgia_marquee` (all in `src/ui/tests.rs`)

**History.** Reconstructed from "Now Playing bar shows the track id instead of the song title" in
`BUGS-FIXED.md`.

## Sign-in server

**Guarantee.** The loopback sign-in server never waits for one connection to finish before it reads
the next. A local client that opens a connection and never finishes its request does not hold the
browser's `POST /token` callback behind it:

- The accepted connections are read together — each one is polled without blocking — so one
  connection's cost is not serialized behind the previous one's.
- A connection is dropped when its own budget (`CONNECTION_READ_TIMEOUT`, five seconds) is spent
  from the moment it was accepted; the flow's deadline (`AUTH_TIMEOUT`) is the outer bound.
- The pending connections are capped (`MAX_PENDING_CONNECTIONS`), so a flood of connections cannot
  grow the server's share of the process's file descriptors without bound; the oldest pending
  connection makes way for the newest, so a flood cannot push the browser's callback out of the
  list either.
- One pass of the accept loop takes on at most `MAX_ACCEPTS_PER_PASS` connections, and a connection
  taken on in a pass is served in the next one, so a callback that is queued behind N connections is
  served within `ceil((N + 1) / MAX_ACCEPTS_PER_PASS) + 1` poll intervals (`ACCEPT_POLL_INTERVAL`,
  25 ms) — one poll interval for every eight connections queued ahead of it. The flow's deadline
  remains the outer bound.
- A bootstrap directory left in the temp dir by a process that died before its guard could remove it
  is swept by the next sign-in once it is older than `BOOTSTRAP_STALE_AFTER` (twice `AUTH_TIMEOUT`).
  A fresher bootstrap directory (which may belong to another running instance), an entry with a
  different name, and a non-directory entry are left alone; a symlink is not followed.

**Enforced by.** `authorize_with_bounds` in `src/music_kit_auth.rs` keeps the accepted connections
in a `Vec<PendingConnection>`. Every pass calls `serve_pending`, which polls each pending
connection through `http::RequestReader::poll` and serves the ones whose request has arrived, and
then `accept_pending`, which takes on the connections that are waiting. `http::RequestReader` owns a
non-blocking socket and parses the request from the bytes received so far, so polling a silent peer
reports "nothing yet" instead of waiting for it; `serve_pending` drops a connection whose budget is
spent. `BootstrapPage::write` calls `sweep_stale_bootstrap_dirs` before creating the new owner-only
directory; the sweep reads one level of the temp dir, removes only aged directories whose names
carry `BOOTSTRAP_DIR_PREFIX`, and reports a removal failure without failing the new sign-in.

**Pinned by.**

- `authorize_serves_the_callback_after_repeated_stalled_connections`
- `authorize_serves_the_callback_after_a_stalled_connection_is_dropped`
- `a_stalled_connection_is_dropped_when_its_budget_is_spent`
- `the_pending_list_is_capped_and_the_oldest_connection_makes_way`
- `one_pass_takes_on_a_bounded_number_of_connections`
- `the_reader_reports_a_partial_request_as_incomplete`
- `the_bootstrap_sweep_removes_only_old_bootstrap_directories`
- `a_new_bootstrap_write_sweeps_a_stale_directory_from_a_crashed_flow`
  (all in `src/music_kit_auth/tests.rs`)

**History.** Reconstructed from "A stalled connection is served for its whole budget before the next
one, so repeated stalls starve the sign-in callback" in `BUGS-FIXED.md`.
