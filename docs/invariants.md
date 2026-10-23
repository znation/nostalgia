# Invariants

Project guarantees that bugs have had to reconstruct, one section per subsystem. Each section
states the guarantee, names the code that enforces it, and names the tests that fail if it is
removed. When a change alters a guarantee, update its section in the same commit. Each section
cites the `BUGS-FIXED.md` entry it was reconstructed from.

**How to use.** Before reconstructing an invariant that a fix's `unclear-invariant` validation
gap names, read this file for the matching subsystem section — it already states the guarantee,
the enforcing code, and the tests that pin it. When a fix's gap is `unclear-invariant`, or a
change alters a guarantee that has a section here, update that section in the same commit, or add
a new section when none fits. `BUGS.md`'s header points here for the tag. For a UI geometry or
label-reflow bug, lay the real tree out through the test-only `src/ui/layout_capture.rs` harness,
which returns each drawn text fragment's absolute bounds and the root size; do not reconstruct a
per-test capture operation.

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
- `album_selected_in_all_albums_loads_its_songs_and_back_returns_to_all_albums`
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
view, the search state, and playback unchanged. Navigating away and back restores the library's
order; no sort flag persists. A fresh reply that replaces the rows drops any saved permutation, so
a sort cannot reorder data stored after it. Sorting an empty, loading, or failed list is a no-op:
it neither reorders rows nor bumps the epoch.

**Enforced by.** `BrowseList::sort_rows_by` in `src/ui/browse.rs` builds a stable index permutation
from the lowercased row keys and rebuilds `items`, `titles`, and `labels` by it, bumping the epoch
only when there are at least two rows and the list is not loading; it also records the cumulative
permutation back to library order, and `BrowseList::restore_library_order` inverts it so the rows
(and their cached text) return to the order they were stored in, bumping the epoch as any index
change must. `browse::sort_browse` in `src/ui/handlers/browse.rs` matches the current view onto
its list and key, routed from `Message::SortBrowse` in the `update` dispatch table;
`browse::back` restores that order on the list Back returns to (those lists stay in memory while a
child is open), and `views::view_browse_nav_buttons` in `src/ui/views/browse.rs` pushes
`view_top_button` and `view_sort_button` into one `Row`, so Sort draws beside Top.

**Pinned by.**

- `browse_nav_buttons_place_sort_beside_top`
- `sort_reorders_artists_case_insensitively`
- `sort_is_stable_for_equal_keys_and_back_restores_them`
- `sort_reorders_albums_by_their_display_title`
- `sort_reorders_playlists_by_name`
- `sort_reorders_songs_and_keeps_their_cached_text_aligned`
- `sort_reorders_all_songs_and_keeps_their_cached_text_aligned`
- `sort_reorders_all_albums_by_their_display_title`
- `sort_bumps_the_epoch_so_a_pre_sort_press_is_rejected`
- `sort_with_an_empty_or_loading_list_is_a_noop`
- `sort_leaves_other_lists_view_search_and_playback_untouched`
- `back_restores_a_sorted_artists_list_to_library_order`
- `a_fresh_artists_reply_drops_the_saved_sort_permutation`
- `back_from_songs_restores_a_sorted_albums_list_to_library_order`
- `back_from_all_albums_songs_restores_a_sorted_flat_list_to_library_order`
- `back_from_a_playlists_songs_restores_a_sorted_list_to_library_order`
  (all in `src/ui/tests.rs`)

## Browse row labels

**Guarantee.** Each artist row shows the number of albums the loaded library holds for that
artist at the right: `1 album` for one, `N albums` for more, and no label at all for an artist
with none. The count is display data only — the row still selects the artist by index and epoch,
and Sort still orders artists by name. A signed-out service answers from the sample library,
whose counts are `2`, `1`, and `0` in artist order. A signed-in service reads the artist list
first and the whole albums collection second (one extra paginated request), counts albums by
exact `Album::artist` display name (skipping albums whose artist name is empty), and matches an
artist by `Artist::name`; a failure in either request surfaces as the existing artists load
failure and its Retry path. `Artist::album_count` is derived data, not an Apple Music field: it
is skipped by serde and deserializes to `0`.

**Enforced by.** `Artist::album_count` in `src/library.rs`;
`library::album_counts_by_artist` counts non-empty `Album::artist` names;
`AppleMusicService::get_favorite_artists` in `src/apple_music/browse.rs` fills the counts from
the sample library or from the two signed-in requests; and
`views::browse::rows::album_count_label` in `src/ui/views/browse/rows.rs` renders the singular,
plural, and empty labels that `artist_row` uses as its secondary text, and `scrollable_list`
pins every row's secondary label to the right edge.

**Pinned by.**

- `artist_row_uses_name_and_selects_the_artist_by_index`
- `album_count_label_singular_plural_and_none`
- `artist_row_puts_its_album_count_at_the_right_edge`
  (all in `src/ui/views/tests.rs`)
- `sample_library_artists_carry_their_album_counts`
  (in `src/sample_library.rs`)
- `get_favorite_artists_uses_the_rest_library_when_signed_in`
- `get_favorite_artists_counts_albums_by_artist_name_from_the_rest_library`
  (both in `src/apple_music/tests.rs`)
- `album_counts_by_artist_counts_non_empty_names`
- `artist_album_count_is_not_serialized_and_defaults_to_zero`
  (both in `src/library.rs`)

## Browse album running times

**Guarantee.** Each album row in the per-artist Albums view and the flat All Albums view shows the
summed running time of the songs the library reports for that album at the right (`10:30`), or the
`--:--` placeholder when it reports none, instead of the old `View Songs` hint. The time is
display data only — the row still selects the album by index and epoch, and Sort, Back, and
selection are unchanged. A signed-out service answers from the sample library, whose derived
totals are `10:30` for `First Record`, `1:35` for `Second Record`, and `3:20` for `Debut`. A
signed-in service reads the album collection first and the whole-library songs collection second
(one extra paginated request), groups the songs by exact `(album title, artist display name)`,
and sums their durations with `saturating_add`; a song without a non-blank `albumName` is
skipped, an album with no matching song is `0`, and a failure in either request surfaces as the
existing album-load failure and its Retry path. `Album::duration_ms` is derived data, not an
Apple Music album field: it is skipped by serde and deserializes to `0`.

**Enforced by.** `Album::duration_ms` in `src/library.rs`; `SampleLibrary::new` in
`src/sample_library.rs` derives each sample album's total from its indexed songs;
`RestLibrary::get_album_durations` in `src/apple_music/rest.rs` groups and sums the library songs;
`browse::apply_album_durations` in `src/apple_music/browse.rs` fills the signed-in album lists;
and `views::browse::rows::album_row` in `src/ui/views/browse/rows.rs` renders the formatted time
or placeholder as the row's secondary text.

**Pinned by.**

- `album_row_uses_title_and_selects_the_album_by_index`
- `album_row_shows_the_title_alone_when_the_artist_is_empty`
- `album_row_shows_a_placeholder_when_the_duration_is_unknown`
  (all in `src/ui/views/tests.rs`)
- `albums_carry_their_songs_total_durations`
  (in `src/sample_library.rs`)
- `album_duration_is_not_serialized_and_defaults_to_zero`
  (in `src/library.rs`)
- `get_album_durations_groups_and_sums_library_songs`
  (in `src/apple_music/rest/tests.rs`)
- `get_albums_by_artist_uses_the_rest_library_when_signed_in`
- `get_all_albums_uses_the_rest_library_when_signed_in`
- `get_all_albums_fails_when_the_album_durations_request_fails`
  (all in `src/apple_music/tests.rs`)

## UI layout

**Guarantee.** The root `view`'s layout keeps these geometry guarantees:

- The browse navigation row lays Sort beside Top, not stacked below it: both share the same
  `y`, and Sort starts at or after Top's right edge.
- The transport row's faces keep their absolute positions when the Play/Pause label changes:
  the Play button's Play and Pause faces are pinned to one fixed width. The Repeat: Off/On and
  Shuffle: Off/On faces are pinned the same way, so their label swaps cannot reflow the row.
- The equalizer band row puts the vertical preamp slider leftmost, then the `+12 DB` / `0 DB` /
  `-12 DB` scale, then the ten band sliders. The scale reads top to bottom beside the rail's top,
  middle, and bottom, and the preamp label shares the band labels' text row. The response curve
  above them starts at the first band, not over the preamp and scale.
- The root `view` draws the browse level named by `CurrentView` from that level's own buffer and
  cached row text, and marks the playing row only in the Songs and All Songs lists.
- The root `view` draws Back on every level below Artists, Retry only on Artists while its fetch
  has failed, and the All Songs, All Albums, and Playlists buttons only on Artists.

**Enforced by.** `views::view_browse_nav_buttons` in `src/ui/views/browse.rs` pushes the Top
and Sort buttons into one `Row`. `views::view_transport_controls` and
`views::transport::transport_buttons` in `src/ui/views/transport.rs` build the transport row
from `fixed_width_button`s. `views::equalizer::preamp_column`,
`views::equalizer::db_scale_column`, and `views::view_equalizer` in
`src/ui/views/equalizer.rs` build the equalizer band row and align the curve with it. The
browse and transport layout tests lay out the whole root `view` through the test-only
`src/ui/layout_capture.rs` harness; the equalizer layout test lays out `view_equalizer`
directly. `view` in `src/ui/view.rs` matches on `CurrentView` and passes each arm that
level's buffer, caches, and current-track id, and gates its Back, Retry, and flat-list buttons on
`can_go_back`, `can_retry_artists`, and a `CurrentView::Artists` match. The harness records each
drawn text fragment's absolute bounds with iced's headless tiny-skia renderer.

**Pinned by.** `browse_nav_buttons_place_sort_beside_top`,
`transport_row_does_not_reflow_when_the_play_label_changes`,
`root_view_offers_back_retry_and_flat_list_buttons_only_where_they_belong`, and
`root_view_draws_the_current_browse_levels_own_rows` (`src/ui/tests.rs`), plus
`transport_buttons_keep_a_fixed_width_across_label_changes` and
`equalizer_preamp_and_db_scale_sit_left_of_the_band_sliders` (`src/ui/views/tests.rs`) for the
Repeat and Shuffle faces and the equalizer band row.

**History.** Reconstructed from "Sort renders below Top instead of beside it, contradicting the
documented layout" and "Transport buttons resize to their label text, so pressing Play/Pause
reflows the whole control row" in `BUGS-FIXED.md`. Both carried a `real-run-needed` validation
gap; the shared harness closes it.

## Equalizer band tint

**Guarantee.** Each equalizer band slider's groove is tinted by that band's stored gain: green at
`GAIN_MIN_DB` (`-12 dB`), yellow at `0 dB`, and red at `GAIN_MAX_DB` (`+12 dB`), interpolating
between adjacent stops. Both rail halves carry the tint, so no gray segment remains on either
side of the thumb. A gain outside the range clamps to the matching end stop, and a NaN gain
renders the flat yellow instead of an invalid color. The vertical preamp slider in the band row
keeps its uniform gray groove.

**Enforced by.** `style::band_gain_color` and `style::band_slider_style` in `src/ui/style.rs`,
used by `views::view_equalizer` in `src/ui/views/equalizer.rs`.

**Pinned by.** `band_gain_color_runs_green_through_yellow_to_red`,
`band_gain_color_clamps_and_maps_nan_to_flat`, and
`band_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb` in `src/ui/style.rs`.

**History.** The tint is presentation-only: it reads the same stored gains the sliders and
`BandFilter` use but does not change the curve the audio filter applies.

## Level slider fill

**Guarantee.** The transport volume slider and the shade-mode volume slider paint both halves of
their rail `theme::VOLUME_SLIDER_FILL` (the mockup's orange), and the transport balance slider
paints both halves in `theme::BALANCE_SLIDER_FILL` (the mockup's green). Both rail halves carry
the fill in every interaction status, so no gray segment remains on either side of the thumb.
The chrome thumb, the 4px rail width, the 1px dark rail border, and the square geometry are
unchanged from `chrome_slider_style`, and hover and drag still change only the handle face. The
seek bar and the equalizer preamp keep the neutral `theme::LCD_BACKGROUND` groove.

**Enforced by.** `style::level_slider_style` in `src/ui/style.rs`, used by `views::chrome_slider`
in `src/ui/views/mod.rs` (the transport row and shade strip pass their fills there).

**Pinned by.** `base_skin_colors_keep_their_documented_literal_values` in `src/ui/theme.rs` and
`level_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb` in `src/ui/style.rs`.

**History.** The fill is presentation-only: slider ranges, steps, messages, thumb geometry, and
playback behavior are unchanged.

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
  Stop and a new selection clear it immediately to `00:00`. While the transport is stopped the
  large readout for the current track draws transparent in both time modes and both blink phases
  whenever its text is not the `--:--` placeholder, keeping its bounds and its clickability off,
  so a stopped player shows no elapsed or remaining time other than the placeholder; the stored
  value is still the `00:00` Stop clears it to, and Play draws it again. The `--:--`
  placeholder still shows while stopped when no track is current, the id is unknown, or a known
  track in `Remaining` mode has no length, since it reports that there is no time to clear. A position reply issued before a Stop
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
- The LCD's small status row always draws both `MONO` and `STEREO` labels side by side in that
  order at its right edge and keeps its fixed height. A loaded preview whose backend reports a
  format draws the average bitrate and sample-rate prefix (`256 KBPS  44 KHZ`) at the row's left
  edge, lights the reported channel mode `theme::LCD_GREEN`, and dims the other label
  `theme::LCD_GREEN_DIM`. The bitrate is
  derived from the downloaded body's length and the decoded source's own duration and is omitted,
  leaving `44 KHZ`, when either is unknown; the sample rate and channel values come from the
  backend's own decoded source. With no reported format both labels stay dim and the prefix is
  absent. Pause keeps the reported format; Stop and a new selection clear it. A format reply issued
  before a Stop or a new selection, or landing while a transport transition is in flight, is
  dropped.
- The LCD's visualizer row draws exactly `BAND_COUNT` green bars, left to right in band order,
  bottom-aligned inside a fixed-height strip between the title and the status row. The bar heights
  come from the loaded preview's stored spectrum, each band clamped to `0.0..=1.0`; no reported
  spectrum, including the silent fallback, draws zero-height bars while keeping the row's height,
  so loading a preview does not move the title or status text. Pause keeps the last bars frozen,
  while Stop and a new selection clear them. A spectrum reply issued before a Stop or a new
  selection, or landing while a transport transition is in flight, is dropped. The spectrum poll
  is subscribed only while playback is running.
- The LCD's left edge always draws the five classic indicator letters `O`, `A`, `I`, `D`, `V`,
  in that order, at a fixed width, in every playback state including the empty `--:--` bar. The
  strip is decorative: it emits no message and changes nothing.
- A transport glyph leads the row in a fixed `LCD_TRANSPORT_WIDTH` slot before the large elapsed
  readout, so the readout keeps its x position as the glyph changes: green `▶` while playing,
  green `II` while paused, and red `■` while stopped. Pause of a loaded preview sets the shared
  paused mode while Play and Stop clear it; a pause with no loaded preview does not set it, so a
  Paused readout always has a preview Play can resume. `AppState::is_playing` alone no longer
  distinguishes Pause from Stop. `state::TransportMode` and `AppState::transport_mode` resolve the mode,
  `views::now_playing::transport_glyph` and `transport_glyph_color` choose the glyph and color,
  and `views::now_playing::now_playing_row` draws the slot. While the mode is `Paused`, the large
  elapsed readout blinks at the frozen position: `handlers::playback::preview::position_tick` flips
  `WinampPlayer::blink_visible` on each one-second position poll, and
  `views::now_playing::elapsed_readout_color` draws the text transparent when the phase is hidden
  or the transport is stopped and the text is not the `--:--` placeholder instead of removing it,
  and
  `views::now_playing::elapsed_readout_clickable` turns off its click while stopped, so the text
  keeps its bounds and the title, total, and status row do not move. A poll after Play or Stop
  holds the phase visible, so playing always shows the readout and the stopped view clears it
  unless it is the `--:--` placeholder, which still shows while stopped. The blink adds no
  timer or subscription. Pinned by
  `transport_glyph_names_and_colors_each_mode`,
  `now_playing_row_keeps_the_elapsed_readout_in_place_across_transport_modes`,
  `now_playing_content_draws_the_indicator_strip_beside_the_title`,
  `elapsed_readout_color_blinks_when_paused_and_clears_when_stopped`,
  `elapsed_readout_color_keeps_the_placeholder_visible_while_stopped`,
  `elapsed_readout_is_clickable_only_while_not_stopped`,
  `elapsed_readout_toggles_only_while_not_stopped`, and
  `now_playing_row_keeps_the_elapsed_bounds_when_the_blink_hides_it` in `src/ui/views/tests.rs`;
  `position_tick_blinks_the_elapsed_readout_only_while_paused` in `src/ui/tests.rs`; and
  `toggle_playing_switches_between_playing_and_paused` and
  `pause_sets_the_paused_mode_while_play_and_stop_clear_it` in `src/state.rs`; and
  `pause_stops_playing_but_keeps_current_track`,
  `pause_without_a_loaded_preview_leaves_the_transport_stopped`,
  `toggle_without_a_loaded_preview_stops_the_transport`,
  `pause_after_stop_leaves_the_transport_stopped`, `resume_resumes_the_audio_backend`,
  `stop_stops_the_audio_backend`, `toggle_play_pause_pauses_while_playing`, and
  `resume_after_stop_restarts_the_preview` in `src/apple_music/tests.rs`.
- The seek slider beneath the LCD measures the backend-reported preview length, not the catalog
  track's length at the LCD's right edge. Its thumb follows the stored position, or the drag in
  progress while one is held; the thumb and the large elapsed readout both resolve their position
  through `WinampPlayer::readout_position`, so an in-progress drag wins in both time modes and the
  readout previews its target time (counting down from it in remaining mode) while the right-edge
  total stays unchanged. Only a release sends `AudioOutput::seek_to`, exactly once, with the
  dragged position. The backend clamps that absolute seek to the preview length. A successful
  seek records the position optimistically and bumps `position_epoch`, so position and duration
  replies already in flight are dropped; a failed seek is logged and lets the next poll correct
  the thumb. Stop and a new selection clear the length, any drag, and the reported position
  through `reset_position_readout`, so the readout returns to the cleared state (`00:00` in
  elapsed mode for a known track) rather than falling back to a reported position. The strip stays
  blank and keeps its fixed height until the backend reports both a position and a positive preview
  length.
- A combined artist/title longer than `MARQUEE_WIDTH` characters is windowed by
  `views::now_playing::marquee_text`, advanced one character per `Message::MarqueeTick`, and
  reset to the start by `TrackSelected`; a line that fits is shown whole and borrowed.
- The marquee timer is subscribed only while the line needs to scroll: `WinampPlayer::needs_marquee`
  measures the same label and artist the bar renders, so a line that fits leaves the timer out
  (every message makes iced rebuild the whole screen, browse list included).
- The position poll is subscribed only while there is a position to refresh:
  `WinampPlayer::needs_position_poll` includes it while playback is running, the backend still holds
  a preview (paused, even before its first position reply, so a seek can still move it), or a stored
  position is still shown, and leaves it out for a stopped player, whose view clears its current
  track's readout while stopped so there is nothing to poll. Without the gate, every tick and
  reply would rebuild the whole screen twice a second for an unchanged readout.

**Enforced by.** `views::now_playing_label` and `views::now_playing_artist` in
`src/ui/views/now_playing/text.rs` (re-exported by `src/ui/views/mod.rs`), pure functions over the
known-track index. `WinampPlayer::now_playing_label` and `WinampPlayer::now_playing_artist` in
`src/ui/player.rs` resolve against `known_tracks`, so the bar keeps naming a playing track after
browsing to another album replaced the visible song list. `views::now_playing::now_playing_elapsed`
(through `format_lcd_elapsed` for elapsed mode and `format_lcd_remaining` for remaining mode)
formats the left-hand readout as chosen by `views::now_playing::TimeDisplay`, while
`views::now_playing::now_playing_time` returns the plain cached total for the right edge; the
`playback_position` field on `WinampPlayer` stores the reply the
`PlaybackPosition` handler received, which is stamped with `position_epoch`, and
`WinampPlayer::readout_position` prefers an in-progress seek drag over that value so the large
readout and the seek thumb share one position source;
`reset_position_readout` bumps that epoch and clears the value
on Stop and on a new selection. `pending_position_transitions` counts the backend transport
transitions that are still in flight: every transport handler in `src/ui/handlers/playback.rs`
that schedules one begins a transition, and the completion that reports that same transition ends
it; while the count is nonzero the `handlers::playback::preview` poll handlers issue no query and
its reply handlers store no reply. `AppleMusicService::play_track`
stops the backend's previous preview when a selection has no preview URL.
`views::now_playing::lcd_indicator_strip` draws `views::now_playing::LCD_INDICATOR_LABELS` down
the LCD's left edge in a fixed-width column, with no message and no handler.
`views::now_playing::visualizer_bar_heights` scales and clamps the stored spectrum, and
`views::now_playing::now_playing_visualizer_row` draws its `BAND_COUNT` bars at fixed width inside
the row `views::view_now_playing` places between the title and status rows. The `Spectrum` handler
in `src/ui/handlers/playback/preview.rs` stores the reply under the same epoch and pending-transition
guards as the position, format, and duration, `spectrum_tick` schedules the backend query, and
`reset_position_readout` clears the bars on Stop and a new selection. `ui::timers::subscriptions`
subscribes `spectrum_ticks` only while `WinampPlayer::needs_spectrum_poll` reports that playback is
running.
`views::now_playing::marquee_text`
windows an overlong line, and `WinampPlayer::marquee_offset` carries the advance through
`Message::MarqueeTick`. The `WinampPlayer::time_display` field holds the readout's mode, and
`handlers::playback::toggle_time_display` (routed from `Message::ToggleTimeDisplay`, pressed by
the `MouseArea` wrapping the readout in `now_playing_row`) flips it without touching playback
state. `views::now_playing::display_len` and `views::now_playing::needs_marquee`
measure the line, and `ui::timers::subscriptions` subscribes `marquee_ticks` only while that measure
exceeds `MARQUEE_WIDTH`, and `position_ticks` only while `WinampPlayer::needs_position_poll` reports
a preview whose position can still change. `audio::PlaybackFormat` carries the preview's sample rate,
channel count, and average bitrate; `audio::preview::DecodedPreview` hands the download's encoded
byte count to `audio::player::format_for_source`, which reads the rate and channels from the
decoded `rodio` source and computes the bitrate through `audio::player::preview_bitrate_kbps` from
the source's own total duration, and
`views::now_playing::text::format_playback_prefix`/`text::channel_mode_label` format them, and
`views::now_playing::CHANNEL_LABELS`/`channel_label_color`/`now_playing_status_row` render them in
the LCD's status row built by `views::view_now_playing`. The `PlaybackFormat` handler in
`src/ui/handlers/playback/preview.rs` stores the reply beside the position under the same epoch and
pending-transition guards, and `reset_position_readout` clears the format on Stop and a new
selection;
`position_tick` batches all three backend queries: the position, the format, and the preview's
length. `views::seek_bar::seek_bar_value` resolves the thumb and scale from the position and the
`playback_duration` field, and `views::view_seek_bar` builds the strip: a full-width
`Message::SeekPreviewChanged`/`Message::SeekPreviewReleased` slider when that scale exists, or an
empty fixed-height stand-in otherwise. The `PlaybackDuration` handler stores that reply under the
same epoch and pending-transition guards as the position and format, `seek_preview_changed`
stores the drag without reaching the backend, and `seek_preview_released` calls
`AppleMusicService::seek_output_to`, which forwards to `AudioOutput::seek_to`; the `duration`
query reaches it through `AudioOutput::duration`. `reset_position_readout` clears the duration and
the drag on Stop and a new selection, and `RodioOutput::seek_to`/`duration` query the worker,
whose `RodioPlayer` records the decoded source's length and clamps each absolute seek to it.

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
- `now_playing_elapsed_previews_the_seek_drag`
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
- `the_position_thread_is_named_nostalgia_position_poll`
- `the_position_timer_runs_only_while_there_is_a_position_to_refresh`
- `a_paused_preview_needs_the_position_poll_before_its_first_reply` (`src/ui/tests.rs`)
- `play_track_without_a_preview_url_stops_the_previous_preview` (`src/apple_music/tests.rs`)
- `position_query_reports_none_when_nothing_is_loaded`
- `a_seek_is_reflected_by_the_next_position_query`
- `a_pause_keeps_the_loaded_player_reporting_its_position`
- `format_for_source_reads_sample_rate_and_channels`
- `preview_bitrate_kbps_computes_encoded_bits_over_the_preview_length`
- `playback_format_query_returns_the_loaded_players_format`
- `playback_format_query_reports_none_when_nothing_is_loaded`
- `rodio_output_asks_the_worker_for_its_format`
- `silent_output_reports_no_playback_format` (`src/audio/tests.rs`)
- `playback_format_forwards_the_audio_output_format` (`src/apple_music/tests.rs`)
- `format_playback_prefix_formats_khz_and_bitrate`
- `channel_mode_label_names_the_reported_channels`
- `channel_label_color_lights_only_the_reported_mode`
- `now_playing_status_row_lights_the_reported_channel_mode` (`src/ui/views/tests.rs`)
- `lcd_indicator_strip_draws_each_label_in_order_at_a_fixed_width`
- `now_playing_content_draws_the_indicator_strip_beside_the_title`
  (`src/ui/views/tests.rs`)
- `seek_bar_value_clamps_the_thumb_and_hides_without_a_length`
- `view_seek_bar_constructs_with_and_without_a_length` (`src/ui/views/tests.rs`)
- `playback_duration_reply_stores_the_loaded_previews_length`
- `playback_duration_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`
- `stop_and_track_selection_clear_the_playback_duration`
- `seek_preview_changed_tracks_the_drag_without_touching_the_backend`
- `seek_preview_released_seeks_once_to_the_dragged_position`
- `seek_preview_released_without_a_drag_does_nothing`
- `seek_preview_released_invalidates_an_in_flight_position_reply`
- `seek_preview_released_backend_failure_leaves_the_position_alone`
- `position_tick_queries_position_and_duration` (`src/ui/tests.rs`)
- `playback_duration_forwards_the_audio_output_duration` (`src/apple_music/tests.rs`)
- `seek_output_to_forwards_an_absolute_position`
- `seek_output_to_propagates_an_audio_backend_failure` (`src/apple_music/tests.rs`)
- `silent_output_reports_no_playback_duration`
- `duration_for_source_reads_a_samples_buffers_length`
- `clamp_seek_clamps_to_the_preview_length_and_passes_through_without_one`
- `rodio_output_asks_the_worker_for_its_duration`
- `rodio_output_reports_none_when_the_duration_reply_times_out`
- `seek_to_command_reaches_the_loaded_player`
- `duration_query_returns_the_loaded_players_duration`
- `duration_query_reports_none_when_nothing_is_loaded` (`src/audio/tests.rs`)
- `playback_format_reply_stores_the_loaded_previews_format`
- `playback_format_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`
- `stop_clears_the_playback_format_while_pause_keeps_it`
- `track_selected_clears_the_playback_format`
- `position_tick_queries_position_and_format` (`src/ui/tests.rs`)
- `marquee_tick_advances_the_offset`
- `marquee_ticks_keep_ticking_when_the_receiver_falls_behind`
- `visualizer_bar_heights_scale_and_clamp_the_reported_bands`
- `now_playing_content_keeps_the_visualizer_height_with_and_without_a_spectrum`
- `now_playing_visualizer_row_keeps_its_height_without_a_spectrum`
- `spectrum_reply_stores_the_loaded_previews_spectrum`
- `spectrum_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`
- `stop_clears_the_spectrum_while_pause_keeps_it`
- `spectrum_tick_queries_spectrum_and_carries_the_epoch`
- `the_spectrum_timer_runs_only_while_playback_runs` (`src/ui/tests.rs`)
- `a_refused_spectrum_thread_ends_the_tick_stream_instead_of_panicking` (`src/ui/tests.rs`)
- `a_refused_spectrum_thread_reports_the_cause_and_the_loss` (`src/ui/tests.rs`)
- `the_spectrum_thread_is_named_nostalgia_spectrum_poll` (`src/ui/tests.rs`)
- `playback_spectrum_forwards_the_audio_output_spectrum` (`src/apple_music/tests.rs`)
- `the_marquee_timer_runs_only_while_the_line_needs_to_scroll`
- `the_player_knows_when_the_now_playing_line_needs_the_marquee`
- `track_selected_resets_the_marquee_offset` (`src/ui/tests.rs`)
- `a_refused_marquee_thread_ends_the_tick_stream_instead_of_panicking`
- `a_refused_marquee_thread_reports_the_cause_and_the_loss`
- `the_marquee_thread_is_named_nostalgia_marquee` (all in `src/ui/tests.rs`)
- `dropping_the_receiver_ends_the_tick_thread` (`src/ui/ticks.rs`)

**History.** Reconstructed from "Now Playing bar shows the track id instead of the song title" in
`BUGS-FIXED.md`.

## Window shade

**Guarantee.** Rolling the window up into shade mode keeps playback controllable:

- Shade mode builds exactly the title bar over the compact transport strip, and the window is
  resized to that same height (`SHADED_HEIGHT = TITLE_BAR_HEIGHT + SHADE_STRIP_HEIGHT`). The
  strip keeps a fixed `SHADE_STRIP_HEIGHT`, and normal mode builds no strip at all.
- The strip shows Previous, Play/Pause, Stop, Next, the current time readout, and a volume
  slider. Its four buttons emit the same messages as their full transport-row twins
  (`Message::PreviousTrack`, `Message::PlayPause`, `Message::Stop`, `Message::NextTrack`), each
  pinned to a fixed width so the glyph swap cannot reflow the strip. The Play/Pause glyph is
  `"▶"` while stopped and `"II"` while playing. A fill spacer pushes the slider to the right
  edge, after the left-aligned buttons and time readout.
- The time readout is the same `WinampPlayer::now_playing_elapsed` string the Now Playing bar
  draws, so the elapsed/remaining choice carries into shade mode. The volume slider writes
  `Message::VolumeChange` and reads the shared volume.

**Enforced by.** `views::shade::view_shade_strip` builds the strip and
`views::shade::shade_controls`/`views::shade::play_pause_glyph` define its buttons;
`views::shade::shade_volume_spec` fixes the slider's shared range and `Message::VolumeChange`;
`views::shade::SHADE_STRIP_HEIGHT` and `views::shade::SHADED_HEIGHT` own the heights; the shaded
branch of `ui::view::view` stacks the bar and strip; and `handlers::window::resize_to_shade` resizes to
`handlers::window::shade_target_size`, which reads `SHADED_HEIGHT`. `views::view_shade_strip` is
re-exported by `src/ui/views/mod.rs`.

**Pinned by.** `shade_play_pause_glyph_switches_with_playback_state`,
`shade_controls_map_to_transport_messages_and_keep_fixed_widths`,
`shade_height_pins_the_rolled_up_window`,
`shade_volume_slider_spec_matches_the_shared_volume_range_and_message`,
`shade_strip_draws_the_time_readout_and_puts_the_volume_slider_at_the_right_edge`, and
`view_shade_strip_keeps_its_fixed_height_for_both_playback_states` (`src/ui/views/tests.rs`);
`shaded_root_view_is_the_title_bar_plus_the_strip`, `shade_strip_draws_only_while_shaded`,
`shade_target_size_is_the_title_bar_plus_the_strip`, and
`title_bar_height_pins_the_shade_strip` (`src/ui/tests.rs`).

**History.** Added with the window-shade transport strip; the earlier shade mode built the title
bar alone.

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
in a `Vec` of `PendingConnection`, which is defined in `src/music_kit_auth/server.rs`. The loop's helpers live in `src/music_kit_auth/server.rs`: every pass
calls `serve_pending`, which polls each pending
connection through `http::RequestReader::poll` and serves the ones whose request has arrived, and
then `accept_pending`, which takes on the connections that are waiting. `http::RequestReader` owns a
non-blocking socket and parses the request from the bytes received so far, so polling a silent peer
reports "nothing yet" instead of waiting for it; `serve_pending` drops a connection whose budget is
spent. `BootstrapPage::write` in `src/music_kit_auth/bootstrap.rs` calls `sweep_stale_bootstrap_dirs`
before creating the new owner-only
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
- `the_bootstrap_sweep_skips_a_symlink_instead_of_following_it`
- `a_new_bootstrap_write_sweeps_a_stale_directory_from_a_crashed_flow`
  (all in `src/music_kit_auth/tests.rs`)

**History.** Reconstructed from "A stalled connection is served for its whole budget before the next
one, so repeated stalls starve the sign-in callback" in `BUGS-FIXED.md`.

## Preview download

**Guarantee.** The preview downloader bounds both the *wire* body and the *decoded* body at
`MAX_PREVIEW_BYTES` (10 MiB). `ureq`'s reader limit stops a hostile server from streaming more
than the cap on the wire, and reading that reader through `std::io::Read::take(MAX_PREVIEW_BYTES +
1)` stops a `Content-Encoding: gzip` response smaller than the cap from expanding the buffered
preview past it; the decoded take rejects before the decoder when its extra byte is present. The
decoded over-cap failure names the cap; other read failures, including the wire limit, keep the
`reading the preview failed:` prefix.

**Enforced by.** `download_and_decode` in `src/audio/preview.rs` calls
`response.body_mut().with_config().limit(MAX_PREVIEW_BYTES).reader()` for the wire limit, then
reads it through `take(MAX_PREVIEW_BYTES + 1)` and checks the decoded length before
`rodio::Decoder::new_mp4`. It does not use `Body::read_to_vec`, whose limiter sits under `ureq`'s
gzip decoder and so counts compressed bytes.

**Pinned by.**

- `a_compressed_preview_larger_than_the_cap_is_rejected` (`src/audio/tests.rs`)
- `a_preview_with_more_wire_bytes_than_the_cap_is_rejected` (`src/audio/tests.rs`)

**History.** Added 2026-10-23 after the wire-byte cap was found to let a small gzip body expand
past 10 MiB before decoding; the decoded-byte fix keeps the original wire cap alongside it.

## Preview spectrum

**Guarantee.** The backend reports ten normalized amplitude bands in
`crate::equalizer::BAND_HZ` order only while a preview is loaded, computed from the samples the
worker actually pulls through `audio::meter::MeterSource`; those samples reach the equalizer,
balance, and output unchanged. A band at or above Nyquist reads zero, as do silence and an empty
window. A seek clears the recorded window without resetting the partial downmix frame, so the
sample rodio leaves on the same channel stays in phase; a player that has not appended a preview
reports nothing, so Stop and a new selection cannot report the previous preview's spectrum. The
silent fallback and a closed worker report `None`. The UI half stores that reply in
`WinampPlayer::spectrum` and draws exactly `BAND_COUNT` bars from it: their heights clamp to the
fixed visualizer row, Pause freezes them, Stop and a new selection clear them, stale or
pre-transition replies are dropped, and the poll runs only while playing.

**Enforced by.** `audio::meter::spectrum_bands` (the pure ten-band Goertzel analysis) and
`audio::meter::SpectrumMeter` (`push` downmixes and fills the fixed ring, `take` copies it and
analyzes outside the lock, `clear` empties the ring while keeping the partial downmix frame) in
`src/audio/meter.rs`; `MeterSource` records each sample while returning it unchanged and clears
the ring on `try_seek` without resetting that frame. `AudioOutput::spectrum`
in `src/audio.rs` defaults to `None`, and `RodioPlayer::spectrum` in `src/audio/player.rs` answers
`Some` only once the player holds a preview's format, through the worker's `Command::Spectrum`
handled beside `Command::PlaybackFormat`; `RodioOutput::spectrum` bounds the query like the other
backend queries. On the UI side, `views::now_playing::visualizer_bar_heights` and
`views::now_playing::now_playing_visualizer_row` scale and draw the bands,
`WinampPlayer::needs_spectrum_poll` gates the poll, `ui::timers::spectrum_ticks` feeds it, and
`handlers::playback::preview::spectrum_tick`/`spectrum` issue and guard the query while
`reset_position_readout` clears the stored value.

**Pinned by.**

- `spectrum_bands_peak_at_the_matching_band_for_a_sine` (`src/audio/tests.rs`)
- `spectrum_bands_zero_a_band_at_or_above_nyquist` (`src/audio/tests.rs`)
- `meter_source_passes_samples_through_and_records_their_spectrum` (`src/audio/tests.rs`)
- `meter_source_seek_keeps_the_downmix_phase` (`src/audio/tests.rs`)
- `meter_window_keeps_the_newest_samples_in_play_order` (`src/audio/tests.rs`)
- `spectrum_query_returns_the_loaded_players_spectrum` (`src/audio/tests.rs`)
- `spectrum_query_reports_none_when_nothing_is_loaded` (`src/audio/tests.rs`)
- `loaded_player_reports_its_meter_but_an_unloaded_one_reports_nothing` (`src/audio/tests.rs`)
- `append_records_the_appended_source_through_the_players_meter` (`src/audio/tests.rs`)
- `stop_removes_the_player_so_a_spectrum_query_reports_none` (`src/audio/tests.rs`)
- `silent_output_reports_no_spectrum` (`src/audio/tests.rs`)
- `rodio_output_asks_the_worker_for_its_spectrum` (`src/audio/tests.rs`)
- `rodio_output_reports_a_closed_worker_for_every_command` (`src/audio/tests.rs`)
- `playback_spectrum_forwards_the_audio_output_spectrum` (`src/apple_music/tests.rs`)
- `visualizer_bar_heights_scale_and_clamp_the_reported_bands`
- `visualizer_bar_heights_maps_a_nan_band_to_empty` (`src/ui/views/tests.rs`)
- `now_playing_content_keeps_the_visualizer_height_with_and_without_a_spectrum`
- `now_playing_visualizer_row_keeps_its_height_without_a_spectrum` (`src/ui/views/tests.rs`)
- `now_playing_visualizer_row_draws_one_fixed_width_bar_per_band` (`src/ui/views/tests.rs`)
- `spectrum_tick_stream_emits_spectrum_ticks` (`src/ui/tests.rs`)
- `a_refused_spectrum_thread_ends_the_tick_stream_instead_of_panicking` (`src/ui/tests.rs`)
- `a_refused_spectrum_thread_reports_the_cause_and_the_loss` (`src/ui/tests.rs`)
- `the_spectrum_thread_is_named_nostalgia_spectrum_poll` (`src/ui/tests.rs`)
- `spectrum_reply_stores_the_loaded_previews_spectrum`
- `spectrum_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`
- `stop_clears_the_spectrum_while_pause_keeps_it`
- `spectrum_tick_queries_spectrum_and_carries_the_epoch`
- `the_spectrum_timer_runs_only_while_playback_runs` (`src/ui/tests.rs`)

## Preview end

**Guarantee.** The backend reports whether the loaded preview has played to its end, and only
while a preview is loaded. A paused but unended preview, a stopped player, the silent fallback, a
closed worker, and an unanswered query all report not finished, so the UI can never advance off a
state that is not a completed preview. Reading the state changes no output and issues no audio
command. The UI polls the finished state on the existing one-second position poll: a finished
preview while playing advances to the song Next would select from the showing list — sequential
when Shuffle and Repeat are off, the first song when Repeat is on and the last is current, and a
non-current song when Shuffle is on. With Shuffle off, Repeat wraps a one-song list back to
itself, so it repeats that song rather than stopping; Shuffle with no other song still takes the
Stop path. A finished preview on the last song with Repeat off takes
the Stop path, so playback becomes stopped, the song stays queued for Play, and the elapsed
readout, status row's reported format, visualizer, and seek bar clear together. Pause, Stop, a
stale epoch, a pending transition, and a not-finished reply never advance.

**Enforced by.** `audio::AudioOutput::is_finished` in `src/audio.rs` defaults to false, and
`audio::player::RodioPlayer::is_finished` in `src/audio/player.rs` answers true only once the
player holds a preview's format and `rodio::Player::empty` reports the queued source has drained;
`RodioOutput::is_finished` bounds the worker's `Command::Finished` reply like the other backend
queries, mapping a missing reply to false. `AppleMusicService::preview_finished` in
`src/apple_music/playback.rs` forwards the backend's answer without touching shared state.
`handlers::playback::preview::position_tick` schedules the finished query, `Message::PlaybackFinished`
carries its reply, and `handlers::playback::preview::playback_finished` advances the transport;
`handlers::playback::forward_track_id` (shared with `handlers::playback::step_track`) applies the
same list/Shuffle/Repeat rules as Next, and the no-next-song case reuses `handlers::playback::stop`.

**Pinned by.**

- `finished_query_returns_the_loaded_players_finished_state` (`src/audio/tests.rs`)
- `finished_query_reports_none_when_nothing_is_loaded` (`src/audio/tests.rs`)
- `rodio_player_reports_finished_only_after_its_appended_preview_drains` (`src/audio/tests.rs`)
- `a_paused_preview_reports_unfinished` (`src/audio/tests.rs`)
- `silent_output_reports_an_unfinished_preview` (`src/audio/tests.rs`)
- `rodio_output_asks_the_worker_for_its_finished_state` (`src/audio/tests.rs`)
- `rodio_output_reports_none_when_the_finished_reply_times_out` (`src/audio/tests.rs`)
- `rodio_output_reports_a_closed_worker_for_every_command` (`src/audio/tests.rs`)
- `stop_removes_the_player_so_a_finished_query_reports_none` (`src/audio/tests.rs`)
- `preview_finished_forwards_the_audio_output_state` (`src/apple_music/tests.rs`)
- `position_tick_queries_preview_finished` (`src/ui/tests.rs`)
- `preview_end_advances_to_the_next_song` (`src/ui/tests.rs`)
- `preview_end_in_all_songs_steps_the_flat_list` (`src/ui/tests.rs`)
- `preview_end_wraps_to_the_first_song_with_repeat_on` (`src/ui/tests.rs`)
- `preview_end_repeats_a_single_song_with_repeat_on` (`src/ui/tests.rs`)
- `preview_end_at_the_last_song_stops_when_repeat_is_off` (`src/ui/tests.rs`)
- `preview_end_with_shuffle_on_picks_the_other_song` (`src/ui/tests.rs`)
- `preview_end_with_shuffle_on_a_single_song_stops` (`src/ui/tests.rs`)
- `preview_end_while_paused_or_stopped_does_not_advance` (`src/ui/tests.rs`)
- `preview_end_with_a_stale_epoch_or_pending_transition_is_dropped` (`src/ui/tests.rs`)

## Play history

**Guarantee.** Shuffle's Previous retraces the tracks whose playback actually committed, and only
those. The play completion carries the play's own outcome: `Message::TrackPlayed`'s `committed`
flag is true only when the play future returned `Ok`, so a Pause or Stop processed after the play
committed but before the completion reaches `update` cannot make a committed play look failed. The
shared `is_playing` and `paused` flags are mutable transport state and are not consulted for this
decision. A committed completion of the latest generation records the current track; a failed or
timed-out one records nothing and keeps a pending history jump available for retry; a completion a
newer selection superseded records and prunes nothing. Any current-generation completion prunes
the known-track index to the current track, or clears it when no track is current.

**Enforced by.** `ui::loading::play_into` maps the play result to `committed` in
`src/ui/loading.rs`; `Message::TrackPlayed` carries it in `src/ui/message.rs`;
`handlers::playback::track_played` in `src/ui/handlers/playback.rs` records only a committed play
of the current generation and otherwise calls `PlayHistory::retry_pending`; `transport::PlayHistory`
in `src/ui/transport.rs` implements the recorded-play walk.

**Pinned by.**

- `track_played_records_a_play_paused_before_its_completion` (`src/ui/tests.rs`)
- `track_played_ignores_a_failed_completion` (`src/ui/tests.rs`)
- `track_played_ignores_a_superseded_completion` (`src/ui/tests.rs`)
- `a_failed_history_jump_retries_the_target` (`src/ui/tests.rs`)
- `previous_with_shuffle_resumes_from_the_last_recorded_play_after_a_new_track` (`src/ui/tests.rs`)
- `track_selected_starts_playback_of_the_selected_track` (`src/ui/tests.rs`)
- `play_into_maps_a_hanging_play_to_a_completed_message` (`src/ui/tests.rs`)
- `play_into_reports_a_failed_play_and_still_completes` (`src/ui/tests.rs`)
