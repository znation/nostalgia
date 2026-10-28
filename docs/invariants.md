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
A plain or Shift-held Backspace key press is the keyboard route to the same single-step Back
action, and is likewise a no-op at Artists; the focused `Search library` box's `TextInput`
captures it for editing, as it does Enter for `SearchSubmitted`.

Every level also offers Reload. Reload clears an active search, returns the browse view to
Artists, and refetches the artists list through the same clear-then-issue fetch path Retry uses,
so previous artists rows and any previous failure are dropped and the loading placeholder shows
until the reply lands; the reloaded list snaps to its first row. It leaves playback, the search
box's typed text, and the other browse buffers (Songs, All Songs, All Albums, Playlists) alone.
`Message::LoadArtists` keeps its boot, sign-in, and Retry behavior and does not change the
current browse level. A plain or Shift-held F5 key press takes the same `Message::ReloadLibrary`
path as the Reload button, so it performs the same refresh; Ctrl/Alt/Logo chords with F5 stay
unbound.

**Enforced by.** The `Message::Back` handler — `browse::back` in `src/ui/handlers/browse.rs`,
routed from the `update` dispatch table in `src/ui/mod.rs` — checks `search_active` first, then
matches on `current_view` and `songs_origin` to pick the one level up. `views::can_go_back` in
`src/ui/views/browse.rs` (re-exported by `src/ui/views/mod.rs`) decides whether the Back button is
shown; it returns false only for `CurrentView::Artists`. `shortcuts::message_for` in
`src/ui/shortcuts.rs` maps Backspace to `Message::Back`, after its Control/Alt/Logo chord guard.
`browse::reload_library` in `src/ui/handlers/browse.rs`, routed from `Message::ReloadLibrary`,
clears the search flag and view and calls `fetch_at_top(fetch::load_artists(player))`;
`views::browse::view_reload_button` in `src/ui/views/browse.rs` is pushed into the nav row by
`views::view_browse_nav_buttons`. `shortcuts::shortcut` in `src/ui/shortcuts.rs` maps F5 to
`Message::ReloadLibrary` after its Control/Alt/Logo chord guard.

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
- `backspace_steps_back_from_albums`
- `reload_library_returns_to_artists_and_refetches_them`
- `f5_reloads_the_library_from_the_keyboard`
- `reload_button_is_shown_on_every_browse_level`
- `browse_nav_buttons_place_reload_beside_sort`
  (all in `src/ui/tests/`)
- `message_for_maps_backspace_to_back` (`src/ui/shortcuts.rs`)
- `message_for_maps_f5_to_reload` (`src/ui/shortcuts.rs`)
- `shortcut_ignores_chord_modifiers` (`src/ui/shortcuts.rs`)
- `back_button_is_available_only_below_the_artist_list` (`src/ui/views/tests/browse.rs`)

**History.** Reconstructed from "Browse view can only move forward — no way back from
Albums/Songs, a dead end" in `BUGS-FIXED.md`. The All Songs, All Albums, and Playlists levels
were added later and are covered here too. Reload was added to refresh the artists list without
restarting the app; F5 later became its keyboard route.

## Search focus

**Guarantee.** Ctrl+F, with or without Shift, from any normal-mode browse level moves keyboard
focus to the `Search library` box without submitting a search. Focusing changes no browse view,
list rows or order, selection, typed query, search-active flag, or playback state; typing into the
focused box edits the query exactly as a click does. Plain F and the Alt+F and Logo+F chords stay
unbound, and every other Control chord except the player's own (Ctrl+Z, Ctrl+B, Ctrl+R, Ctrl+W)
remains the window manager's. Shade mode builds no search box, so Ctrl+F there is a no-op.

**Enforced by.** `views::browse::BROWSE_SEARCH_ID` in `src/ui/views/browse.rs`, applied with
`.id(BROWSE_SEARCH_ID)` by `views::browse::view_search_box` (used by the re-exported
`views::browse::view_search_row`), gives the text input a stable focus target.
`Message::FocusSearch` in `src/ui/message.rs` is routed by `src/ui/mod.rs` to
`browse::search::focus_search` in `src/ui/handlers/browse/search.rs`, which returns
`iced::widget::operation::focus(BROWSE_SEARCH_ID)` and mutates no player state.
`shortcuts::shortcut` in `src/ui/shortcuts.rs` claims a Control-held `f` before its
Control/Alt/Logo chord guard, while leaving Alt+F and Logo+F to the window manager.

**Pinned by.**

- `ctrl_f_focuses_the_search_box_and_changes_no_browse_state` (`src/ui/tests/search_tests.rs`)
- `ctrl_f_is_a_noop_in_shade_mode` (`src/ui/tests/search_tests.rs`)
- `message_for_maps_ctrl_f_to_focus_search` (`src/ui/shortcuts.rs`)
- `view_search_box_registers_its_focus_id` (`src/ui/views/tests/browse.rs`)

**History.** Added with the Ctrl+F keyboard affordance for the library search box.

## Library search clearing

**Guarantee.** A `Clear` button beside the `Search library` box on every normal-mode browse
level empties the typed query. With no search showing it changes only the box text: no browse
rows, view, selection, search-active flag, playback, fetch, or scroll change. While a search is
showing it leaves the search exactly as Back does — it empties the query, deactivates the search,
returns to the Artists view, restores the artists list's library order, and snaps the browse list
to its top — while leaving the search-result rows in the songs buffer and scheduling no fetch or
playback. Shade mode builds no search row, so it has no Clear button.

**Enforced by.** `Message::ClearSearch` in `src/ui/message.rs` is routed by `src/ui/mod.rs` to
`browse::search::clear_search` in `src/ui/handlers/browse/search.rs`; that handler empties
`search_query` and,
only while `search_active` is set, delegates to `handlers::browse::back`, which is what makes the
active case identical to the Back button. `views::browse::view_clear_search_button` renders the
button and `views::browse::view_search_row` places it after the search box; `view_search_row` is
re-exported by `src/ui/views/mod.rs`, and the view tests import the button and box builders from
`views::browse` directly. `src/ui/view/library.rs` builds that row where the bare search box used to sit.

**Pinned by.**

- `clear_search_without_an_active_search_clears_only_the_query` (`src/ui/tests/search_tests.rs`)
- `clear_search_with_an_empty_query_schedules_no_work` (`src/ui/tests/search_tests.rs`)
- `clear_search_while_a_search_is_active_returns_to_artists_and_clears_the_query`
  (`src/ui/tests/search_tests.rs`)
- `view_clear_search_button_constructs` (`src/ui/views/tests/browse.rs`)
- `view_search_row_draws_clear_after_the_search_box` (`src/ui/views/tests/browse.rs`)

**History.** Added the one-press way to empty the library search box, reusing Back for the
active-search case so the two affordances cannot drift apart.

## Library search matching

**Guarantee.** With no Apple Music session, a library search returns every sample-library song
whose title, artist, or album title contains the trimmed query case-insensitively, in library
order. A query that matches none of those fields returns an empty list, and a blank query (empty
or only whitespace) is rejected with an error rather than answered, so empty input never reads as
"everything matches". With a session, the REST search service's results are returned unchanged.

**Enforced by.** `sample_songs_matching` in `src/apple_music/sample.rs` filters the ordered
library walk against the folded query, using `sample_album_titles_by_id` to look up each song's
album title by its `album_id`. `AppleMusicService::search_songs` in `src/apple_music/browse.rs`
trims the query, rejects a blank one, and answers through the signed-in REST client or the sample
fallback.

**Pinned by.**

- `search_songs_matches_the_sample_library_by_title_or_artist` (`src/apple_music/tests/browse_sample.rs`)
- `search_songs_matches_the_sample_library_by_album_title` (`src/apple_music/tests/browse_sample.rs`)
- `search_songs_rejects_a_blank_query` (`src/apple_music/tests/browse_sample.rs`)
- `search_songs_uses_the_rest_library_when_signed_in` (`src/apple_music/tests/browse_rest.rs`)

**History.** Added the album-title match so typing an album name the browse lists show finds
that album's songs offline.

## Browse selection

**Guarantee.** Each browse list owns at most one selected row, and only the selected row draws the
blue `theme::ROW_HIGHLIGHT` bar; a single press on any artist, album, song, or playlist row selects
it without changing the view, playing a song, or scheduling a fetch. Ordinary rows draw green text
and keep the hover/press faces of the flat playlist row. The playing track's row draws white text,
keeps its `▶` marker, and draws no bar unless it is also selected; a row that is both playing and
selected keeps the bar and stays identifiable by its `▶` prefix. Only a matching epoch and an
in-range index can select, so a press rendered from a replaced list is a no-op. A second press on
the row that is already selected, the `Open` button, or a plain or Shift-held Enter key press
dispatches the existing artist/album/track/playlist open action, so a double-click reaches the same
drill-in or playback on its second press; with no selected row, Enter is a no-op that schedules no
fetch or playback task. The search box's own Enter is captured by its `TextInput::on_submit` and
remains `Message::SearchSubmitted` (the keyboard subscription receives only ignored events), so
Enter in the box searches rather than opening a row. The selection is cleared whenever the rows are
replaced, cleared, failed, sorted, reversed, shuffled, or restored to library order, because the
stored index then names a different row.

**Enforced by.** `BrowseList::select_row` in `src/ui/browse.rs` validates the press through
`BrowseList::select` and records it, returning `Some(true)` only for the second press on the
selected row; `BrowseList::selected` exposes it to the view. `replace_rows`, `clear`, and `fail`
in `src/ui/browse.rs`, and `sort_rows_by`, `reverse_rows`, `shuffle_rows`, and
`restore_library_order` in `src/ui/browse/ordering.rs`, clear it with the rows or order they change.
`browse::select_row` and `browse::open_selected` in `src/ui/handlers/browse.rs` route the two
messages and share `browse::open_row`, which emits the existing `ArtistSelected`,
`AlbumSelected`, `TrackSelected`, or `PlaylistSelected` message; `shortcuts::message_for` maps a
plain or Shift-held Enter key press to `Message::OpenSelected`. `style::playlist_row_style` in
`src/ui/style.rs` paints an ordinary row's green text over the flat hover/press face ladder, and
`views::browse::rows::browse_row_style` adds the selected bar and white playing/selected text.
`views::browse::rows::scrollable_list` puts `Message::SelectRow { epoch, index }` on every row
and applies that style to each one, and `views::view_open_button` (re-exported by
`src/ui/views/mod.rs`) builds the Open button.

**Pinned by.**

- `select_row_records_a_press_and_reports_the_second_one`
- `select_row_rejects_a_stale_epoch_or_out_of_range_index`
- `replacing_clearing_or_failing_rows_drops_the_selection`
- `sorting_or_restoring_rows_drops_the_selection`
  (all in `src/ui/browse.rs`)
- `a_single_row_press_selects_without_opening`
- `a_second_press_on_the_selected_row_opens_it`
- `open_selected_opens_the_selected_song`
- `open_selected_with_nothing_selected_is_a_noop`
- `enter_opens_the_selected_song`
- `replacing_or_sorting_rows_clears_the_selection`
- `reverse_bumps_the_epoch_and_clears_the_selection`
- `randomize_bumps_the_epoch_clears_the_selection_and_keeps_every_row`
- `browse_open_button_sits_above_the_browse_list`
  (all in `src/ui/tests/`)
- `message_for_maps_enter_to_open_selected`
  (in `src/ui/shortcuts.rs`)
- `browse_row_style_keeps_playing_rows_unbarred_with_white_text`
- `browse_row_style_paints_only_the_selected_row_blue`
  (all in `src/ui/views/tests/`)
- `now_playing_bar_and_browse_action_buttons_construct`
- `browse_open_button_constructs`
- `browse_views_construct_over_the_loaded_library`
- `browse_views_construct_over_an_empty_list`
  (all in `src/ui/views/tests/`)

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
  the `Message::BrowseReply` handler (`browse::fetch::browse_reply` in
  `src/ui/handlers/browse/fetch.rs`),
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
  (all in `src/ui/tests/`)

**History.** Reconstructed from "A browse reply is stored after a newer request was issued, so a
navigation shows the previous level's rows" in `BUGS-FIXED.md`.

## Browse row order

**Guarantee.** The Sort button reorders only the browse list currently showing, ascending by each
row's rendered text compared case-insensitively: Artists and Playlists by name, Albums and All
Albums by `artist - title`, and Songs and All Songs by `artist - title`. Rows whose keys compare
equal keep their library order. The song and album lists reorder their cached titles and labels
by the same permutation as their rows, so each row keeps its own title and duration; the artists
list does the same for its cached album-count labels (it caches no titles, because each name is
borrowed from the row). The reorder
is one-shot: it bumps only that list's epoch, so a selection press rendered before the sort is
rejected while the post-sort indices resolve to the sorted rows, and it leaves the other lists,
the current view, the search state, and playback unchanged. Navigating away and back restores the
library's order; no sort flag persists. A fresh reply that replaces the rows drops any saved
permutation, so a sort cannot reorder data stored after it. Sorting an empty, loading, or failed
list is a no-op: it neither reorders rows nor bumps the epoch.

Ctrl+Shift+1 (the top-row `1` by physical key, Control and Shift held, no Alt or Logo) sends the
same `Message::SortBrowse` as the button.

Ctrl+R (Control without Shift, Alt, or Logo) reverses, and Ctrl+Shift+R (Control and Shift, no Alt
or Logo) shuffles, the showing list's order under the same rules: one-shot, epoch bumped, selection
cleared, cached titles and labels reordered with the rows, other lists, view, search, and playback
unchanged, Back to the parent restoring library order (reversal, shuffle, and Sort compose), and a
no-op on an empty, loading, or failed list.

Alt+Up and Alt+Down (logical arrow keys with Alt held and Control, Shift, and Logo released) move
the showing list's selected row one place up or down. The move is one-shot like the others: cached
titles and labels follow the rows, the epoch is bumped, and it composes with Sort, reverse, and
shuffle so Back restores library order. Unlike them the moved row stays selected at its new index.
It is a no-op with no selection, at the edge the row would move past, or on a loading or failed
list. A move that changes the selection scrolls the list only far enough to show the moved row
fully (and not at all when it is already visible); a no-op move schedules no task.

**Enforced by.** `BrowseList::move_selected` in `src/ui/browse/ordering.rs`, `scroll::reveal_row_offset` and
`scroll::reveal_browse_row` in `src/ui/handlers/browse/scroll.rs`, `browse::move_selected_row`
in `src/ui/handlers/browse.rs` (routed from `Message::MoveSelectedRowUp` and
`Message::MoveSelectedRowDown`), and `shortcuts::move_row_shortcut` in `src/ui/shortcuts.rs`.
`BrowseList::reverse_rows` and the shared `BrowseList::apply_reorder` in
`src/ui/browse/ordering.rs`, `browse::reverse_browse` in `src/ui/handlers/browse.rs` (routed from
`Message::ReverseBrowse`), `BrowseList::shuffle_rows` and `browse::ordering::shuffled_order` (seeded by
`transport::shuffle_roll`) with `browse::randomize_browse` (routed from `Message::RandomizeBrowse`),
the Ctrl+R and Ctrl+Shift+R arms of `shortcut`, and `shortcuts::playlist_sort_shortcut`
(Ctrl+Shift+1), both in `src/ui/shortcuts.rs`. `BrowseList::sort_rows_by` in `src/ui/browse/ordering.rs`
builds a stable index permutation from the lowercased row keys and reorders `items`, `titles`, and
`labels` in place by it, bumping the epoch only when there are at least two rows and the list is
neither loading nor failed; it also records the cumulative permutation back to library order,
and `BrowseList::restore_library_order` in `src/ui/browse/ordering.rs` inverts it so the rows
(and their cached text) return to the order they were stored in, bumping the epoch as any index
change must. `browse::sort_browse` in `src/ui/handlers/browse.rs` matches the current view onto
its list and key, routed from `Message::SortBrowse` in the `update` dispatch table;
`browse::back` restores that order on the list Back returns to (those lists stay in memory while a
child is open), and `views::view_browse_nav_buttons` in `src/ui/views/browse.rs` pushes
`view_top_button`, `view_sort_button`, and `view_reload_button` into one `Row`, so Sort and Reload
draw beside Top.

**Pinned by.**

- `move_selected_swaps_with_the_neighbour_and_keeps_the_selection`
- `move_selected_swaps_cached_text_and_still_restores_library_order`
- `a_moved_artist_keeps_its_cached_album_label_and_back_restores_it`
- `reordering_moves_rows_without_cloning_them`
- `sort_walks_disjoint_cycles_and_keeps_cached_text_aligned`
- `move_selected_is_a_noop_without_a_selection_at_an_edge_or_while_loading`
- `message_for_maps_alt_arrows_to_move_the_selected_row`
- the tests in `src/ui/tests/move_row_tests.rs`
- `browse_nav_buttons_place_sort_beside_top`
- `browse_nav_buttons_place_reload_beside_sort`
- `sort_reorders_artists_case_insensitively`
- `sort_reorders_artists_and_keeps_their_cached_labels_aligned`
- `sort_is_stable_for_equal_keys_and_back_restores_them`
- `sort_reorders_albums_by_their_display_title`
- `sort_reorders_playlists_by_name`
- `sort_reorders_songs_and_keeps_their_cached_text_aligned`
- `sort_reorders_all_songs_and_keeps_their_cached_text_aligned`
- `sort_reorders_all_albums_by_their_display_title`
- `sort_reorders_all_albums_and_keeps_their_cached_text_aligned`
- `sort_bumps_the_epoch_so_a_pre_sort_press_is_rejected`
- `sort_with_an_empty_or_loading_list_is_a_noop`
- `sort_leaves_other_lists_view_search_and_playback_untouched`
- `back_restores_a_sorted_artists_list_to_library_order`
- `back_restores_sorted_artists_and_their_cached_album_labels`
- `a_fresh_artists_reply_drops_the_saved_sort_permutation`
- `back_from_songs_restores_a_sorted_albums_list_to_library_order`
- `back_from_all_albums_songs_restores_a_sorted_flat_list_to_library_order`
- `back_from_a_playlists_songs_restores_a_sorted_list_to_library_order`
- `reverse_flips_the_artists_list`
- `reverse_flips_songs_and_keeps_their_cached_text_aligned`
- `reverse_bumps_the_epoch_and_clears_the_selection`
- `reverse_with_an_empty_or_loading_list_is_a_noop`
- `back_restores_a_reversed_then_sorted_albums_list_to_library_order`
- `reverse_leaves_other_lists_view_search_and_playback_untouched`
- `message_for_maps_ctrl_r_to_reverse_the_list` (in `src/ui/shortcuts.rs`)
- `message_for_maps_ctrl_shift_r_to_randomize_the_list` (in `src/ui/shortcuts.rs`)
- `shuffled_order_is_a_deterministic_permutation`
- `shuffled_order_pins_a_known_seed`
- `shuffled_order_varies_with_the_seed`
- `shuffle_rows_moves_songs_with_their_cached_text`
- `randomize_bumps_the_epoch_clears_the_selection_and_keeps_every_row`
- `randomize_with_an_empty_or_loading_list_is_a_noop`
- `back_restores_a_randomized_albums_list_to_library_order`
- `message_for_maps_ctrl_shift_1_to_sort` (in `src/ui/shortcuts.rs`)
- `ctrl_shift_1_needs_exactly_control_and_shift` (in `src/ui/shortcuts.rs`)
- `ctrl_shift_1_sorts_the_showing_list_from_the_keyboard`
  (the rest in `src/ui/tests/`)

## Browse playing-row visibility

**Guarantee.** When Next, Previous (including Shuffle's retrace of the tracks actually played),
the preview-end advance, a Ctrl+Z / Ctrl+B edge jump, or a keypad 1/3 ten-song jump lands on a
song while a song list (`CurrentView::Songs` or `CurrentView::AllSongs`) is showing, the update
loop reveals that row in the browse list — scrolling only as far as needed to show it fully: to
the row's top when the row is above the viewport, to its bottom edge when it is below, and not at
all when it is already fully visible — in addition to scheduling the existing `TrackSelected`.
Row `index` spans from `BROWSE_LIST_PADDING + index * BROWSE_ROW_HEIGHT` for one
`BROWSE_ROW_HEIGHT`. `BROWSE_ROW_HEIGHT` is the laid-out vertical pitch between consecutive browse
rows: every shared row title stays on one line — hard line breaks are flattened to spaces and a
title wider than its column is clipped there — so no title can make its row taller or draw under the
right-aligned label. `BROWSE_LIST_PADDING` is the list's top padding, which the row geometry
includes. A later row therefore cannot drift out of view from a stale constant; iced clamps a
reveal that would pass the list's top or end. A direct row press does not scroll — the pressed row
is already in view — and stepping while another browse level is showing does not scroll, since
only a song list carries the `▶` playing marker. Top, Back, and every list-opening navigation keep
their existing snap-to-top behavior, and playback, selection, sorting, row text, and audio behavior
are unchanged.

**Enforced by.** `views::BROWSE_ROW_HEIGHT` and `views::BROWSE_LIST_PADDING` in
`src/ui/views/browse/rows.rs` are the one pitch and padding `scrollable_list` lays its rows
out with, and `scrollable_list` flattens title line breaks and clips each title to its
column, so every row keeps that pitch and stays inside its column;
`handlers::browse::scroll::reveal_browse_row` in `src/ui/handlers/browse/scroll.rs` reads the
list's current offset and viewport height and schedules the minimal absolute offset computed by
`reveal_row_offset`, and `handlers::browse::scroll::scroll_browse_to_playing_row` gates it on the
current view.
`handlers::playback::stepping::step_track`,
`handlers::playback::stepping::jump_to_edge_track`,
`handlers::playback::stepping::jump_ten_tracks`, and
`handlers::playback::preview::playback_finished` batch that reveal with their `TrackSelected`,
so Next, Previous, a shuffled retrace, an edge jump, a ten-song jump, and a natural preview end
all follow the landed row.

**Pinned by.**

- `scrollable_list_clips_an_overlong_title_to_its_column`
  (`src/ui/views/tests/browse.rs`)
- `browse_row_height_matches_the_laid_out_row_pitch`
- `a_hard_line_break_in_a_title_does_not_change_the_row_pitch`
- `a_carriage_return_in_a_title_does_not_change_the_row_pitch`
  (all three in `src/ui/tests/view_tests.rs`)
- `next_and_previous_reveal_the_landed_row_only_when_it_leaves_the_viewport`
- `edge_jumps_scroll_the_showing_song_list` (`src/ui/tests/stepping_tests.rs`)
- `a_finished_preview_scrolls_the_song_list_to_the_advanced_row`
- `a_finished_preview_within_the_viewport_does_not_scroll`
- the inline `reveal_row_offset` tests in `src/ui/handlers/browse/scroll.rs`
- `stepping_while_another_browse_level_shows_does_not_scroll`
- `a_row_press_does_not_scroll_the_list`

## Browse keyboard scrolling

**Guarantee.** A plain or Shift-held `Home` key press snaps the showing browse list to its first
row, exactly like the `Top` button, and `End` snaps it to its last row. `Page Up` and `Page Down`
(plain or Shift-held) instead scroll that list one page of `BROWSE_PAGE_HEIGHT` — eight
`BROWSE_ROW_HEIGHT` rows — in the matching direction, relative to the list's current offset.
Every one of these keys is a view-scroll operation only: they change no current view, rows or row
order, selection, typed query, search-active flag, or playback state, and they schedule no fetch
or playback task. In shade mode, and while the `PL` button has hidden the library section, no
browse list is built, so the scroll operation has no target and nothing moves; iced clamps the
offset at the list's bounds, so paging past either end leaves
the list at that end and a short list stays put. While the `Search library` box is focused, its
`TextInput` captures Home and End for editing the query rather than scrolling the list, as it
captures Backspace. Ctrl/Alt/Logo chords with Home, End, Page Up, or Page Down stay unbound.

**Enforced by.** `shortcuts::shortcut` in `src/ui/shortcuts.rs` maps `Home` to
`Message::ScrollBrowseToTop`, `End` to `Message::ScrollBrowseToBottom`, `Page Down` to
`Message::ScrollBrowsePageDown`, and `Page Up` to `Message::ScrollBrowsePageUp` after its
Control/Alt/Logo chord guard. `src/ui/mod.rs` routes the four messages to
`handlers::browse::scroll::scroll_browse_to_top`/`_bottom` and `scroll_browse_page_up`/`_down`.
The snap handlers in `src/ui/handlers/browse/scroll.rs` share a `snap_to` call, while the page
handlers share
`scroll_browse_by`, which calls iced's `scroll_by` with `y = ±views::BROWSE_PAGE_HEIGHT`; both
target the stable `views::BROWSE_LIST_ID` that every `views::browse::scrollable_list` carries.

**Pinned by.**

- `scroll_browse_to_bottom_schedules_a_scroll_task` (`src/ui/tests/scroll_tests.rs`)
- `scroll_browse_page_keys_schedule_page_sized_scrolls` (`src/ui/tests/scroll_tests.rs`)
- `shade_mode_builds_no_browse_scroll_target` (`src/ui/tests/`)
- `toggle_library_panel_hides_and_restores_the_library_section` (`src/ui/tests/`)
- `message_for_maps_home_and_end_to_browse_scroll` (`src/ui/shortcuts.rs`)
- `message_for_maps_page_keys_to_browse_paging` (`src/ui/shortcuts.rs`)
- `shortcut_ignores_chord_modifiers` (`src/ui/shortcuts.rs`)

**History.** Added with the Home and End keyboard scrolling alongside the existing Top button;
extended with the Page Up and Page Down one-page scrolling.

## Media keys

**Guarantee.** A plain or Shift-held system media Play/Pause key press maps to
`Message::PlayPause`; Play to `Message::Play`, Pause to `Message::Pause`, Stop to
`Message::Stop`, Next track to `Message::NextTrack`, and Previous track to
`Message::PreviousTrack`. The Play, Pause, Stop, Next, and Previous mappings are the same
explicit messages the on-screen controls and the classic X, C, V, B, and Z letters send, so each
of those keys obeys that control's existing Shuffle, Repeat, and end-of-list rules and changes no
browse, search, or LCD state beyond what the control already changes; the media Play/Pause key
instead toggles playback through `Message::PlayPause`, which no on-screen control or letter
dispatches. Ctrl/Alt/Logo chords with these keys stay unbound, and a key release binds nothing.

**Enforced by.** `shortcuts::shortcut` in `src/ui/shortcuts.rs` maps the six `key::Named::Media…`
variants to those messages after its Control/Alt/Logo chord guard.

**Pinned by.**

- `message_for_maps_media_keys_to_the_transport` (`src/ui/shortcuts.rs`)
- `media_play_pause_key_toggles_playback` (`src/ui/tests/shortcuts_tests.rs`)
- `shortcut_ignores_chord_modifiers` (`src/ui/shortcuts.rs`)

**History.** Added with the system media-key transport bindings.

## List edge jumps

**Guarantee.** With the window focused, Ctrl+Z (also with Shift) maps to `Message::FirstTrack` and
Ctrl+B to `Message::LastTrack`. They play the first and last song of the list the current view
draws from (`all_songs` in the flat All Songs view, `songs` otherwise), whatever Shuffle, Repeat,
the current track, and the playback position are, and scroll the list to that row. An empty list
schedules nothing. Plain and Shift Z/B keep stepping, and Alt or Logo chords with them stay unbound.
A Ctrl-held click on the Previous or Next button (transport row, shade strip, library footer) does
the same jump; a plain click steps one song. Losing window focus clears the held Control.

**Enforced by.** `shortcuts::shortcut` in `src/ui/shortcuts.rs` checks the two Control chords
before its chord guard; `handlers::playback::stepping::jump_to_edge_track` picks the edge row and
epoch from the showing list without reading shared state or touching the play history.
`stepping::previous_button` / `next_button` choose between it and `step_track` from
`WinampPlayer::control_held`, which `listeners::modifiers_message` keeps current.

**Pinned by.**

- `message_for_maps_ctrl_z_and_ctrl_b_to_the_list_edges` and `shortcut_ignores_chord_modifiers`
  (`src/ui/shortcuts.rs`)
- `first_track_selects_the_first_song`, `last_track_selects_the_last_song`,
  `edge_jumps_ignore_shuffle_and_repeat`, `edge_jumps_with_no_songs_loaded_do_nothing`,
  `edge_jumps_in_all_songs_use_the_flat_list`, and `edge_jumps_scroll_the_showing_song_list`
  (`src/ui/tests/stepping_tests.rs`)
- `ctrl_click_previous_plays_the_first_song`, `ctrl_click_next_plays_the_last_song`,
  `plain_button_clicks_step_one_song`, `control_release_restores_plain_steps`, and
  `modifiers_message_tracks_control_and_focus_loss` (`src/ui/tests/ctrl_click_tests.rs`)

**History.** Added with the Ctrl+Z / Ctrl+B list-edge shortcuts.

## Ten-song jumps

**Guarantee.** `JumpTracksBack` (keypad 1) and `JumpTracksForward` (keypad 3) play the song ten
places before or after the current track in the list the current view draws from (`all_songs` in the
flat All Songs view, `songs` otherwise), clamped to its first or last song and never wrapping, and
scroll the list to that row. With no current track, or one not in the list, forward plays the first
song and backward the last. Shuffle, Repeat, and the playback position do not change the result, and
`play_history` is untouched. An empty list schedules nothing.

**Enforced by.** `transport::jumped_track_index` computes the index;
`handlers::playback::stepping::jump_ten_tracks` reads the current track under the state lock and
schedules the selection and scroll.

**Pinned by.**

- `jump_moves_ten_songs_either_way`, `jump_clamps_at_the_list_ends`,
  `jump_without_a_known_current_uses_the_step_entry_points`, and `jump_over_an_empty_list_is_none`
  (`src/ui/transport.rs`)
- `jump_forward_selects_ten_songs_on`, `jump_back_selects_ten_songs_back`,
  `jumps_clamp_at_the_list_ends`, `jumps_ignore_shuffle_and_repeat`,
  `jumps_with_no_songs_loaded_do_nothing`, and `jumps_in_all_songs_use_the_flat_list`
  (`src/ui/tests/jump_tests.rs`)
- `message_for_maps_the_numpad_transport_digits` (`src/ui/shortcuts.rs`)

**History.** Added with the keypad 1 / 3 ten-song jumps.

## Numpad keys

**Guarantee.** With the window focused and NumLock on, keypad 4, 5, 6, 7, 9, 8, 2, 1, and 3 (plain
or with Shift) map to `PreviousTrack`, `Play`, `NextTrack`, `SeekBackward`, `SeekForward`,
`VolumeUp`, `VolumeDown`, `JumpTracksBack`, and `JumpTracksForward` — the first seven are the
messages Z, X, B, and the arrows send, so they follow those messages' rules (see `## Ten-song jumps`
for the last two). Top-row digits, keypad 0 and the keypad operators, and Ctrl/Alt/Logo chords with
the keypad stay unbound. With NumLock off the keypad sends arrow keys, which keep their arrow
bindings.

**Enforced by.** `shortcuts::numpad_shortcut` in `src/ui/shortcuts.rs` matches on
`Location::Numpad`, and `shortcuts::message_for` tries it before `shortcut`.

**Pinned by.**

- `message_for_maps_the_numpad_transport_digits`, `message_for_leaves_other_digits_unbound`,
  `message_for_ignores_numpad_chords`, and `message_for_maps_numpad_arrows_like_the_arrow_keys`
  (`src/ui/shortcuts.rs`)

**History.** Added with the Winamp numpad bindings.

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
is skipped by serde and deserializes to `0`. The loaded Artists list caches each row's
album-count label at store time through `BrowseList::store_with_labels` (`artists_loaded` in
`src/ui/handlers/browse/fetch.rs`), and `artist_row` borrows it, falling back to composing the
label for an index without a cached value, so a view refresh does not reformat every count. The
cache holds only labels; each row's name stays borrowed from the artist, so no names are cloned.

**Enforced by.** `Artist::album_count` in `src/library.rs`;
`library::album_counts_by_artist` counts non-empty `Album::artist` names;
`AppleMusicService::get_favorite_artists` in `src/apple_music/browse.rs` fills the counts from
the sample library or from the two signed-in requests; and
`views::browse::rows::text::album_count_label` in `src/ui/views/browse/rows/text.rs` renders the
singular, plural, and empty labels that `artist_row` uses as its secondary text, and
`scrollable_list` pins every row's secondary label to the right edge. The artists fetch stores
those labels through `BrowseList::store_with_labels`, so `artist_row` can borrow one per row.

**Pinned by.**

- `artist_row_uses_name_and_album_count`
- `artist_row_prefers_the_lists_cached_album_count`
- `album_count_label_singular_plural_and_none`
- `artist_row_puts_its_album_count_at_the_right_edge`
  (all in `src/ui/views/tests/`)
- `artists_loaded_populates_list`
- `sort_reorders_artists_and_keeps_their_cached_labels_aligned`
- `back_restores_sorted_artists_and_their_cached_album_labels`
- `a_moved_artist_keeps_its_cached_album_label_and_back_restores_it`
  (all four in `src/ui/tests/`)
- `sample_library_artists_carry_their_album_counts`
  (in `src/sample_library.rs`)
- `get_favorite_artists_uses_the_rest_library_when_signed_in`
- `get_favorite_artists_counts_albums_by_artist_name_from_the_rest_library`
  (both in `src/apple_music/tests/browse_rest.rs`)
- `album_counts_by_artist_counts_non_empty_names`
- `artist_album_count_is_not_serialized_and_defaults_to_zero`
  (both in `src/library.rs`)

## Browse row caching

**Guarantee.** Each browse row's built widget is reused across view rebuilds while its browse level, list epoch,
position, playing flag, and selected flag stay the same, so a playback poll reply that changes none
of them does not rebuild the row's button, text, or containers. The epoch is the list's row-text
revision as well: `BrowseList` bumps it on every store and reorder, together with the cached titles
and labels, so stored text changes revise the key within that list. A changed key rebuilds that row, so
the rendered text, layout, press message, and style remain the ones the current list and selection
describe. The cache lives in the iced widget slots of the library column, so each refresh of the
same browse level diffs new keys against those slots instead of allocating fresh rows. Switching to
or from Artists changes the buttons pushed above the list (Back, Retry, and the flat-list buttons
depend on the view), which shifts the list's position in the column and builds its rows anew; every
level below Artists pushes the same buttons, so a direct switch between them keeps the list's slot
and rebuilds its rows because the browse level in each key changes. Hiding the library panel or shading the window stops
building the column and drops the cached rows with it.

**Enforced by.** `views::browse::rows::scrollable_list` wraps every row in
`views::browse::rows::lazy::LazyRow`, keyed by `RowKey` (browse-level discriminant, epoch, position, playing and selected
flags); `LazyRow` builds its element from that key and keeps it while the key hashes the same. The
key deliberately does not hash the row's title and label: doing so rescanned the whole list's text
on every refresh even though `BrowseList::replace_rows` and the reorder methods already bump the
epoch whenever cached text changes. A row reports the browse button's full-width, content-height
size before its element is built, so `Column::push` sees the same hint the uncached button gave it.

**Pinned by.** `scrollable_list_reuses_unchanged_rows_and_rebuilds_only_changed_keys`
(`src/ui/views/tests/browse.rs`), which builds the real list twice and varies the title, label,
selection, and epoch; `switching_lists_at_the_same_epoch_rebuilds_cached_rows`
(`src/ui/views/tests/browse.rs`), which switches Albums and All Albums at equal epochs
with identical and different text; and `reuses_the_element_until_the_dependency_changes` and
`an_unbuilt_row_reports_a_full_width_content_height_hint`
(`src/ui/views/browse/rows/lazy.rs`), which pin the widget itself and its pre-build size hint.
The existing browse-row text, clipping, and right-edge tests in `src/ui/views/tests/browse.rs` and
the pitch tests
(`browse_row_height_matches_the_laid_out_row_pitch`,
`a_hard_line_break_in_a_title_does_not_change_the_row_pitch`,
`a_carriage_return_in_a_title_does_not_change_the_row_pitch`) in `src/ui/tests/view_tests.rs` pin
the rendered result, not cache reuse.

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
existing album-load failure and its Retry path. An absent or unusable duration — negative,
fractional, or above `u64` — counts as `0` instead of failing the response. `Album::duration_ms`
is derived data, not an Apple Music album field: it is skipped by serde and deserializes to `0`.

**Enforced by.** `Album::duration_ms` in `src/library.rs`; `SampleLibrary::new` in
`src/sample_library.rs` derives each sample album's total from its indexed songs;
`RestLibrary::get_album_durations` in `src/apple_music/rest.rs` groups and sums the library songs;
`browse::with_album_durations` in `src/apple_music/browse.rs` fetches the whole-library durations
and fills both signed-in album lists. `views::browse::rows::text::album_row` in
`src/ui/views/browse/rows/text.rs` renders the
formatted time or placeholder as the row's secondary text. The loaded album lists cache each
row's display title and formatted running time through `BrowseList::store_with_row_text`
(`store_albums` in `src/ui/handlers/browse/fetch.rs`), and `album_row` borrows them, falling back
to composing the title and formatting the duration for an index without a cached value, so a view
refresh does not re-join and reformat every row.

**Pinned by.**

- `album_row_uses_title_and_duration`
- `album_row_prefers_the_lists_cached_title_and_running_time`
- `album_row_shows_the_title_alone_when_the_artist_is_empty`
- `album_row_shows_a_placeholder_when_the_duration_is_unknown`
  (all in `src/ui/views/tests/`)
- `albums_loaded_populates_list`
- `all_albums_loaded_stores_the_flat_list`
  (both in `src/ui/tests/`)
- `albums_carry_their_songs_total_durations`
  (in `src/sample_library.rs`)
- `album_duration_is_not_serialized_and_defaults_to_zero`
  (in `src/library.rs`)
- `get_album_durations_groups_and_sums_library_songs`
- `get_album_durations_counts_an_unusable_duration_as_zero`
  (both in `src/apple_music/rest/tests/library.rs`)
- `get_albums_by_artist_uses_the_rest_library_when_signed_in`
- `get_albums_by_artist_fails_when_the_albums_request_fails`
- `get_albums_by_artist_fails_when_the_album_durations_request_fails`
- `get_all_albums_uses_the_rest_library_when_signed_in`
- `get_all_albums_leaves_an_album_without_matching_songs_at_zero_duration`
- `get_all_albums_fails_when_the_albums_request_fails`
- `get_all_albums_fails_when_the_album_durations_request_fails`
  (all in `src/apple_music/tests/browse_rest.rs`)

## UI layout

**Guarantee.** The root `view`'s layout keeps these geometry guarantees:

- The browse navigation row lays Sort and Reload beside Top, not stacked below them: all three
  share the same `y`, Sort starts at or after Top's right edge, and Reload starts at or after
  Sort's right edge.
- The transport row draws, in order, Previous, Play, Pause, Stop, and Next as the compact glyphs
  `◀◀`, `▶`, `II`, `■`, and `▶▶`, each with a word hover tooltip (`Previous`, `Play`, `Pause`,
  `Stop`, `Next`). Play dispatches `Message::Play` and Pause `Message::Pause`, the same explicit
  calls as the X and C keys; no drawn glyph dispatches `Message::PlayPause`. The five glyph
  buttons are pinned to fixed widths 34, 30, 30, 30, and 34 px, and the row is playback-independent,
  so it cannot reflow when playback starts. Repeat and Shuffle remain the `Repeat:
  Off/On` and `Shuffle: Off/On` text toggles, pinned to 104 and 112 px, so their label swaps
  cannot reflow it either; each draws the sunken `theme::BUTTON_FACE_PRESSED` chrome while on and
  the raised `theme::BUTTON_FACE` while off. The shade strip and the library footer build their five buttons
  from the same glyph spec.
- The main transport row's fixed-width `EQ` toggle follows the balance slider in every playback
  state and is sunken while the equalizer panel is shown, ordinary chrome while it is hidden.
  Plain or Shift-held `E` dispatches the same `Message::ToggleEqualizerPanel` from the keyboard;
  Alt+G dispatches the same message by physical key; Ctrl, Alt, and Logo chords with E stay
  unbound for the equalizer, and shade mode changes nothing because the toggle leaves the flag
  alone while `WinampPlayer::shaded` holds. The root `view` draws the
  `NOSTALGIA EQUALIZER` band and `view_equalizer` exactly when
  `WinampPlayer::eq_panel_visible` holds. Hiding them moves the `NOSTALGIA LIBRARY` band and the
  browse list up into the freed height without resizing the window, and leaves the equalizer's
  on/off state, preamp, band gains, preset, and audio forwarding untouched.
- The main transport row's fixed-width `PL` toggle follows `EQ` directly in every playback state
  and is sunken while the library section is shown, ordinary chrome while it is hidden. The root
  `view` draws the whole normal-mode library column — the `NOSTALGIA LIBRARY` band, the
  Back/Retry/Open/Top/Sort/Reload and flat-list buttons, the search row, the browse list, and
  the library footer — exactly when `WinampPlayer::library_panel_visible` holds. The library
  section is open on a first launch, the flag is saved per `## Settings persistence`, and hiding it
  keeps the title bar, Now Playing bar, seek bar, transport row, and equalizer section drawn without
  changing any playback, equalizer, browse, search, selection, or window-size state. Alt+E
  dispatches `Message::ToggleLibraryPanel` by physical key, its other modifier chords stay unbound,
  and shade mode changes nothing because the toggle leaves `library_panel_visible` alone while
  `WinampPlayer::shaded` holds.
- The main transport row's fixed-width lightning button follows `PL`, separated by the row's
  usual gap, and publishes `Message::ToggleOptions`, the same message as the LCD strip's `O`.
  The root `view` draws the options menu only while `WinampPlayer::options_visible` holds outside
  shade mode, so the button never changes playback, browse, search, equalizer audio, volume, or
  balance.
- The equalizer band row puts the vertical preamp slider leftmost, then the `+12 DB` / `0 DB` /
  `-12 DB` scale, then the ten band sliders. The scale reads top to bottom beside the rail's top,
  middle, and bottom, and the preamp label shares the band labels' text row. The response curve
  above them starts at the first band, not over the preamp and scale, and draws a dim
  `theme::LCD_GREEN_DIM` guide behind its `theme::LCD_GREEN` trace: a dotted line on the 0 dB
  midline and one full-height marker at each band position, evenly spaced from the plot's left
  edge to its right edge. The trace passes exactly through each band's gain and is eased between
  adjacent bands with a cubic smoothstep, so it meets every band at a zero slope rather than
  leaving the corners straight segments would.
- The equalizer header's `EQ` and `AUTO` toggles draw the sunken `theme::BUTTON_FACE_PRESSED`
  chrome while on and the raised `theme::BUTTON_FACE` while off, built by
  `views::equalizer::eq_toggle` (via `toggle_button`) and `views::equalizer::auto_toggle`.
- The equalizer's preset pick list shows the applied preset's name, or `Custom` when
  `AppState::eq_preset` is `None`, drawn in `theme::TEXT` so a hand-tuned curve reads as a value
  rather than an empty control.
- The root `view` draws the browse level named by `CurrentView` from that level's own buffer and
  cached row text, with its loading, failure, or empty-search placeholder chosen from that same
  level's flags, and marks the playing row only in the Songs and All Songs lists.
- The root `view` draws Back on every level below Artists, Retry only on Artists while its fetch
  has failed, and the All Songs, All Albums, and Playlists buttons only on Artists.
- The app bar and the two section bands are `TITLE_BAR_HEIGHT` tall. The app bar centers
  `NOSTALGIA` between gold tapes with its M and shade/minimize/close controls pinned at the
  edges. Normal mode draws `NOSTALGIA EQUALIZER` immediately above the equalizer and
  `NOSTALGIA LIBRARY` immediately above the browse controls; the section bands emit no messages
  and are absent in shade mode.

**Enforced by.** `views::view_browse_nav_buttons` in `src/ui/views/browse.rs` pushes the Top, Sort,
and Reload buttons into one `Row`. `views::view_transport_controls`,
`views::transport::transport_buttons`, and `views::transport::transport_glyphs` in
`src/ui/views/transport.rs` build the glyph row from `fixed_width_button`s, with Repeat and Shuffle
built by `repeat_toggle`/`shuffle_toggle` through `fixed_width_toggle_button` (`src/ui/views/mod.rs`), and
`iced::widget::Tooltip`s; `views::transport::eq_panel_toggle` and
`views::transport::library_panel_toggle` spec the EQ and PL toggles, which
`views::transport::panel_toggle_button` builds and `style::toggle_button_style`
in `src/ui/style.rs` sinks while its panel is shown,
`handlers::window::toggle_equalizer_panel` and `handlers::window::toggle_library_panel` flip the
layout flags; `shortcuts::shortcut` in
`src/ui/shortcuts.rs` maps plain and Shift-held E to that same message; `view` pushes the
equalizer section only while that flag holds; `views::mini_transport::mini_transport_controls`
projects the same glyph spec for the shade strip and the library footer.
`views::equalizer::eq_toggle` (through `toggle_button`) and `views::equalizer::auto_toggle`
(through `fixed_width_toggle_button`) sink the header's EQ and AUTO toggles while on;
`views::equalizer::preamp_column`, `views::equalizer::db_scale_column`,
`views::equalizer::response_curve_offsets`, `views::equalizer::response_curve_row`, and
`views::view_equalizer`
in `src/ui/views/equalizer.rs` build the equalizer band row, ease the trace between bands, and
align the curve with it;
`views::view_equalizer` passes `views::equalizer::CUSTOM_PRESET_LABEL` to the pick list's
placeholder, which `style::chrome_pick_list_style` in `src/ui/style.rs` draws in `theme::TEXT`. The browse
and transport layout tests lay out the whole root `view` through the test-only
`src/ui/layout_capture.rs` harness; the equalizer layout test lays out `view_equalizer` directly.
`view_library` in `src/ui/view/library.rs` matches on `CurrentView` and passes each arm that level's buffer, caches,
and current-track id, and gates its Back, Retry, and flat-list buttons on `can_go_back`,
`can_retry_artists`, and a `CurrentView::Artists` match. `views::view_title_band` and
`views::title_bar::title_tape` in `src/ui/views/title_bar.rs` build the
section bands and the app bar's tape/label row from `theme::TITLE_TAPE` and
`theme::TITLE_TAPE_SHADOW`; `view_player_panels` in `src/ui/view/player_panels.rs` and `view_library` in
`src/ui/view/library.rs` push their section bands into the normal-mode tree only. The harness records each drawn text fragment's absolute bounds with iced's headless
tiny-skia renderer.

**Pinned by.** `browse_nav_buttons_place_sort_beside_top`,
`repeat_and_shuffle_toggles_sink_while_on`,
`eq_and_auto_toggles_sink_while_on`,
`browse_nav_buttons_place_reload_beside_sort`,
`transport_row_keeps_play_and_pause_glyphs_fixed_across_playback_states`,
`root_view_draws_the_transport_glyph_row`,
`root_view_offers_back_retry_and_flat_list_buttons_only_where_they_belong`,
`root_view_draws_the_current_browse_levels_own_rows`,
`root_view_draws_each_browse_levels_own_placeholder`,
`root_view_draws_the_equalizer_and_library_title_bands_in_order`,
`toggle_equalizer_panel_hides_and_restores_the_section`,
`toggle_library_panel_hides_and_restores_the_library_section`,
`e_toggles_the_equalizer_panel_from_the_keyboard`,
`e_in_shade_mode_leaves_the_equalizer_panel_alone`,
`alt_e_toggles_the_library_panel_and_back`,
`alt_e_in_shade_mode_leaves_the_library_panel_alone`,
`alt_g_toggles_the_equalizer_panel`, and
`shaded_root_view_draws_no_section_title_bands` (`src/ui/tests/`), plus
`title_band_centers_its_label_at_the_title_bar_height` and
`title_bar_centers_its_name_between_its_edge_controls` (`src/ui/views/tests/chrome.rs`) for the
title bands and app bar, `transport_glyphs_match_the_mockup_order_and_widths`,
`transport_buttons_keep_a_fixed_width_across_label_changes`,
`transport_buttons_keep_their_measured_face_widths`,
`eq_panel_toggle_targets_the_visibility_flag`, and
`library_panel_toggle_targets_the_visibility_flag` (`src/ui/views/tests/transport.rs`),
`main_transport_and_shade_strip_share_glyphs_order_and_widths` (`src/ui/views/tests/shade.rs`),
`equalizer_preamp_and_db_scale_sit_left_of_the_band_sliders` and
`equalizer_pick_list_shows_custom_for_a_hand_tuned_curve` (`src/ui/views/tests/equalizer.rs`),
`response_curve_offsets_follow_the_band_gains` and
`response_curve_guide_draws_the_zero_db_line_and_band_markers_behind_the_trace`
(`src/ui/views/tests/equalizer.rs`),
and `chrome_pick_list_style_sinks_the_face_when_opened` and `toggle_button_style_sinks_while_on`
(`src/ui/style.rs`) for the pick-list label and the EQ and PL toggles.

**History.** Reconstructed from "Sort renders below Top instead of beside it, contradicting the
documented layout" and "Transport buttons resize to their label text, so pressing Play/Pause
reflows the whole control row" in `BUGS-FIXED.md`. Both carried a `real-run-needed` validation
gap; the shared harness closes it. The E key joined the `EQ` toggle as the same message
(2026-10-24). The `PL` toggle joined the row beside `EQ` as a layout-only gate on the whole
library column (2026-10-24). The response curve gained its dim 0 dB and band guide behind the
trace (2026-10-26). Its spans are eased between adjacent bands with a cubic smoothstep, so the
trace meets every band at a zero slope (2026-10-26). The transport row's combined Play/Pause
glyph split into separate Play and Pause buttons that dispatch the explicit messages the X and C
keys use, leaving the row playback-independent (2026-10-26).
Split root assembly into branch modules, retaining the shared LCD/footer snapshot and the
outer OptionsShortcut wrapper (2026-10-26 by the feature role).
Repeat and Shuffle sink while on, through `fixed_width_toggle_button` (2026-10-27).
The equalizer's EQ and AUTO buttons sink while on, through `eq_toggle` and `auto_toggle` (2026-10-27).

## Settings persistence

**Guarantee.** The values the user sets that outlive a track — volume, balance, Repeat, Shuffle, the
equalizer's on/off state, AUTO choice, preamp, ten band gains, and selected preset, the large readout's
elapsed/remaining choice, the visualizer's shown/hidden choice, the equalizer and library panels'
shown/hidden choices, the LCD strip's always-on-top choice, and the window's shade mode — are
loaded once before the first view is drawn and rewritten after any update that changes their snapshot, with the newest
queued snapshot written once the worker is free: a burst of changes, such as a slider drag or key
repeat, may collapse to its newest value instead of writing every intermediate one. An update that leaves the snapshot
unchanged writes nothing. Reads and writes go through the state's clamped
setters, so out-of-range values from a file cannot push shared state outside its documented ranges.
The path is `NOSTALGIA_SETTINGS_PATH` when set and non-empty, else an absolute `$XDG_CONFIG_HOME`'s
`nostalgia/settings.ini`, else `$HOME/.config/nostalgia/settings.ini`, else persistence is
disabled; a relative `XDG_CONFIG_HOME` is ignored per the XDG spec.
A missing file, or a malformed line (including a `NaN` number and bytes that are not UTF-8,
which are decoded lossily so they spoil only their own line), falls back per key without failing
startup; an unreadable file
falls back the same way and prints one stderr line naming the path (`read_failure_message` in
`src/settings/file_store.rs`, which stays silent for a missing file), and because its values are
unknown the store then refuses every later save for the rest of the run, printing one line per
refusal that names the file and says to repair or remove it and restart, so a file the app could
not read is never replaced by a snapshot built from defaults; a missing file records no failure,
so a first run still creates the file. A path that is not a regular file (a FIFO or device, say)
and a regular file larger than 1 MiB are treated as unreadable too: the non-regular path is
refused using metadata from the opened handle, with a nonblocking open on Unix so a FIFO
cannot wait for a writer even when the path changes during load. The read is capped at 1 MiB
plus one detection byte so an oversized file is not loaded whole. An
unwritable file is logged
rather than crashing the player. Each save stages the whole snapshot in
a sibling temp file of the file it writes, flushes it to disk, gives the staged file the existing
target's mode, and renames it over that target and then syncs its directory, so a failed,
interrupted, or crashed write leaves the previous file intact instead of a truncated one; a failed
write or rename removes the temp file, and a load sweeps temp files a killed save left behind once
they are old enough not to belong to an in-flight save. A symlinked settings file stays a link: the
save, the directory sync, and the stale-temp sweep all follow it to the file and directory it points
at, so the link is never replaced; a dangling link or a missing file is used as the path given, and
a new file takes the platform's default mode. The readout mode, the visualizer's shown/hidden
choice, the two panel choices, the always-on-top choice, and the shade mode are the six persisted
values not held in `AppState`: they live on `WinampPlayer::time_display`,
`WinampPlayer::visualizer_visible`, `WinampPlayer::eq_panel_visible`,
`WinampPlayer::library_panel_visible`, `WinampPlayer::always_on_top`, and
`WinampPlayer::shaded`, `main` passes the loaded snapshot through to
`WinampPlayer::adopt_saved_settings` (which restores all six before the first view), and
`persist_settings_if_changed` copies them into the snapshot before comparing and saving. A missing
or malformed `time_display` line falls back to elapsed, a missing or malformed `always_on_top` line
falls back to unpinned, a missing or malformed `shade` line falls back to unshaded, and a missing
or malformed `visualizer_visible`, `eq_panel_visible`, or `library_panel_visible` line falls back
to shown. The classic `A` key and Winamp's Ctrl+A reach
the same `Message::ToggleAlwaysOnTop` arm through `shortcuts::message_for`, so they flip the flag
and schedule the same window-level call as the on-screen A controls. The saved always-on-top level
waits for the window id every other window action uses: `window_id_resolved` stores the id, then
schedules `iced::window::set_level` when the restored flag holds. A restored shade mode waits for
the same id: no pre-shade size was captured at boot, so `window_id_resolved` also schedules
`iced::window::size`, and the resulting measurement stores the launch size before rolling the
window up (see `## Window shade`). It is still not window placement: the window's size and position
are not persisted. Playback position, the browse level, the search box, and the current track are
deliberately not persisted.

**Enforced by.** `src/main.rs` resolves the path with
`settings::file_store::FileStore::from_environment`, loads it once with `FileStore::load`, applies
the result with `settings::Settings::apply_to`, and
passes the store and the loaded snapshot (or a `Settings::from_state` snapshot when the file is
absent) to `ui::init_ui`. The loaded
file's text is parsed by `Settings::from_text`, which ignores unknown keys and malformed values
per key, and `apply_to` writes through `AppState::set_volume`, `set_balance`, `set_eq_preamp`, and
`set_eq_band` before restoring a preset through `apply_eq_preset`. `ui::init_ui` attaches the
store to `WinampPlayer` and restores the snapshot through
`WinampPlayer::adopt_saved_settings`, which restores `time_display`, `visualizer_visible`,
`eq_panel_visible`, `library_panel_visible`, `always_on_top`, and `shaded` and
records the snapshot; `update` calls
`WinampPlayer::persist_settings_if_changed` once after every message, and that method copies
the six player-owned values into `Settings::from_state`, then compares that snapshot against the
last one before calling `settings::file_store::SettingsWriter::save`. Production wraps the store
in `settings::writer::BackgroundWriter`, so that `save` stores the snapshot in a one-slot mailbox
and wakes its worker instead of writing on the update thread; the worker writes the newest queued
snapshot with the wrapped store's blocking `save`, and dropping the writer closes the mailbox,
writes any pending snapshot, and joins the worker before returning. Boot's `WindowIdResolved` is
handled by
`handlers::window::window_id_resolved`, which applies the restored level through
`window_level` once the id exists and, when the restored shade mode is still waiting for its
launch size, batches in the `iced::window::size` query.
`settings::file_store::FileStore`'s `SettingsWriter` impl creates the parent directory, stages the
text with `write_atomically`, and logs a failed create or write. `FileStore::load` reads through
`read_settings_file`, which opens nonblocking on Unix and checks the opened handle's metadata
(refusing one that is not a regular file), then `read_settings_bytes` caps the read at
`MAX_SETTINGS_BYTES` plus one detection byte. The store records a failed
read in a flag shared across the store's clones (an `Arc<AtomicBool>`, so the `BackgroundWriter`'s
clone sees it), and `save` checks it before creating a directory or a temp file and refuses with
`unreadable_settings_save_report`.
With no store attached the method returns before taking the state lock, so tests keep the default
no-op behavior. `shortcuts::message_for` maps `A` and Ctrl+A to `Message::ToggleAlwaysOnTop`, the
arm those controls press.

**Pinned by.** `to_text_round_trips_every_field`,
`time_display_round_trips_both_words_and_ignores_unknown_values`,
`visualizer_visibility_round_trips_and_ignores_malformed_values`,
`panel_visibility_round_trips_and_ignores_malformed_values`,
`always_on_top_round_trips_and_ignores_malformed_values`,
`shade_round_trips_and_ignores_malformed_values`,
`from_text_ignores_malformed_and_unknown_lines`,
`from_text_ignores_nan_numbers`,
`apply_to_clamps_out_of_range_values_through_the_state_setters`,
`apply_to_restores_a_saved_preset`,
`eq_auto_round_trips_and_ignores_malformed_values`,
`settings_path_resolves_override_then_xdg_then_home_then_none`,
`settings_path_ignores_a_relative_xdg_config_home`,
`file_store_save_then_load_returns_the_same_settings`,
`file_store_saves_and_loads_a_bare_file_name_in_the_current_directory`,
`file_store_keeps_the_previous_file_when_the_replacement_cannot_be_staged`,
`file_store_sweeps_a_stale_temp_file_left_by_a_killed_save`,
`the_sweep_covers_a_bare_file_name_in_the_current_directory`, and
`the_sweep_follows_a_symlink_to_the_targets_directory` (`src/settings/file_store.rs`),
`a_failed_half_write_leaves_the_previous_settings_file_intact`,
`staged_settings_are_synced_before_the_rename_and_a_failed_sync_keeps_the_old_file`,
`saving_through_a_symlink_updates_its_target_and_keeps_mode`, and
`the_directory_sync_follows_a_symlink_to_the_target` (`src/settings/atomic_write.rs`),
`a_settings_file_that_could_not_be_read_is_not_overwritten`,
`a_writer_clone_shares_the_failed_read_and_refuses_to_overwrite`,
`a_missing_settings_file_still_allows_the_first_save`,
`an_oversized_settings_file_is_refused_without_being_read_or_overwritten`,
`a_non_regular_settings_path_is_treated_as_unreadable`, and
`a_fifo_settings_path_does_not_wait_for_a_writer` (Unix),
`understated_settings_lengths_still_enforce_the_read_cap`, and
`unreadable_settings_save_report_names_the_file_and_the_recovery` (`src/settings/file_store.rs`),
`a_burst_of_saves_collapses_to_the_latest_snapshot`,
`dropping_the_writer_flushes_the_pending_snapshot`, and
`a_refused_writer_thread_falls_back_to_saving_on_the_caller` (`src/settings/writer.rs`), plus
`a_changed_setting_is_saved_once_and_unchanged_updates_are_not`,
`repeat_and_equalizer_changes_reach_the_saved_snapshot`,
`toggle_time_display_reaches_the_saved_snapshot`,
`saved_time_display_mode_is_restored_on_boot`,
`toggle_visualizer_reaches_the_saved_snapshot`,
`saved_visualizer_visibility_is_restored_on_boot`,
`toggle_panels_reach_the_saved_snapshot`, `saved_panel_visibility_is_restored_on_boot`,
`shaded_eq_toggle_saves_nothing`,
`toggle_always_on_top_reaches_the_saved_snapshot`,
`saved_always_on_top_is_restored_on_boot`,
`toggle_window_shade_reaches_the_saved_snapshot`,
`saved_shade_is_restored_on_boot`,
`toggle_eq_auto_reaches_the_saved_snapshot`,
`saved_eq_auto_is_restored_through_apply_to`,
`window_level_selects_always_on_top`, and
`window_id_resolved_applies_a_saved_always_on_top_level` (`src/ui/tests/`). The classic `A` key
and Ctrl+A reaching the same arm are pinned by `a_toggles_always_on_top_from_the_keyboard`
(`src/ui/tests/`) and `message_for_maps_ctrl_a_and_ctrl_t_to_the_classic_toggles`
(`src/ui/shortcuts.rs`).

**History.** Added with the settings-persistence plan on 2026-10-23; the player previously kept
these values in memory only and started every launch at `AppState::default()`. The readout's
elapsed/remaining mode and the always-on-top choice joined the file later the same day. The
visualizer's shown/hidden choice joined it with the same plan, restoring before the first view
while the stored spectrum and its poll stay transient. The equalizer and library panels'
shown/hidden choices joined it on 2026-10-24; both previously reopened on every launch. Saves
became temp-then-rename the same day, after a failed write could truncate the only copy. On
2026-10-25 a save began following a symlinked settings file to its target and keeping that target's
mode, where replacing the link would have silently detached a dotfile setup. On 2026-10-26 the
production writer moved its blocking write to a background thread, because a slider drag changed
the snapshot on every movement and each synchronous save fsyncs the file and its directory, so the
update thread stalled on the disk repeatedly during a drag. On 2026-10-26 the window's shade
mode joined the file, so a saved-shaded launch opens rolled up and its first unshade restores the
launch size; the window's size and position remain unpersisted. On 2026-10-26 the equalizer's
AUTO choice joined the file as `eq_auto`, restoring the genre-matched preset behavior at launch.
On 2026-10-26 a settings file that exists but cannot be read began refusing later saves for that
run, because the app starts from defaults in that case and the first settings change would
otherwise overwrite the still-intact file with those defaults. On 2026-10-26 the load also began
refusing a path that is not a regular file and capping the file at 1 MiB, with a nonblocking
open on Unix to avoid waiting for a FIFO writer and a read bound independent of reported length.

## Equalizer band tint

**Guarantee.** Each equalizer band slider's groove is tinted by that band's stored gain: green at
`GAIN_MIN_DB` (`-12 dB`), yellow at `0 dB`, and red at `GAIN_MAX_DB` (`+12 dB`), interpolating
between adjacent stops. Both rail halves carry the tint, so no gray segment remains on either
side of the thumb. A gain outside the range clamps to the matching end stop, and a NaN gain
renders the flat yellow instead of an invalid color. The vertical preamp slider's groove in the band
row takes the same tint from the stored preamp gain.

**Enforced by.** `style::band_gain_color` and `style::band_slider_style` in
`src/ui/style/slider_style.rs`, used by `views::view_equalizer` and
`views::equalizer::preamp_slider_style` in `src/ui/views/equalizer.rs`.

**Pinned by.** `preamp_slider_style_matches_the_band_tint_for_its_gain` in
`src/ui/views/tests/equalizer.rs`, `band_gain_color_runs_green_through_yellow_to_red`,
`band_gain_color_clamps_and_maps_nan_to_flat`, and
`band_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb` in
`src/ui/style/slider_style.rs`.

**History.** The tint is presentation-only: it reads the same stored gains the sliders and
`BandFilter` use but does not change the curve the audio filter applies.

## Equalizer scale shortcuts

**Guarantee.** A press on an equalizer dB scale label (`+12 DB`, `0 DB`, `-12 DB`) stores that
label's clamped gain in all ten bands, clears the preset selection, leaves the preamp, the on/off
flag and the LCD readout untouched, and forwards the curve to the audio backend.

**Enforced by.** `db_scale_column` in `src/ui/views/equalizer.rs`,
`handlers::mixer::eq_bands_set_all`, and `AppState::set_all_eq_bands`.

**Pinned by.** `set_all_eq_bands_clamps_every_band_and_clears_the_preset` in `src/state.rs`,
`eq_scale_click_sets_every_band_and_keeps_the_preamp` in `src/ui/tests/mixer_tests.rs`, and
`pressing_a_db_scale_label_sets_every_band_to_its_gain` in `src/ui/views/tests/equalizer.rs`.

## Auto-load presets

**Guarantee.** While AUTO is on, selecting a track whose trimmed primary genre matches a preset
name case-insensitively applies that preset's whole curve — preamp and all ten bands — names it in
the pick list, and forwards the curve to the running preview through the equalizer's existing
forwarding path. A blank or unmatched genre changes nothing, and AUTO off never changes the curve
on selection. The AUTO button flips only `AppState::eq_auto`, schedules no work, and leaves the
gains, preset, and equalizer on/off state alone.

**Enforced by.** `equalizer::preset_for_genre` in `src/equalizer.rs`, `AppState::eq_auto` and
`AppState::toggle_eq_auto` in `src/state.rs`, `handlers::mixer::toggle_eq_auto` and
`handlers::mixer::apply_auto_preset` in `src/ui/handlers/mixer.rs`, the
`apply_auto_preset` call in `handlers::playback::track_selected`, and `views::equalizer::auto_label`
with `AUTO_BUTTON_WIDTH` in `src/ui/views/equalizer.rs`.

**Pinned by.** `preset_for_genre_matches_trimmed_case_insensitive_names` in `src/equalizer.rs`,
`toggle_eq_auto_flips_only_the_auto_flag_and_keeps_the_curve` in `src/state.rs`,
`toggle_eq_auto_flips_only_the_auto_flag`,
`auto_on_applies_a_genre_matched_preset_on_track_selection` in `src/ui/tests/mixer_tests.rs`, and
`auto_button_draws_its_state_and_publishes_toggle_eq_auto` in `src/ui/views/tests/equalizer.rs`.

**History.** Added with the AUTO-toggle plan on 2026-10-26; the panel previously applied a preset
only when the user picked one, so a song's genre never reached the equalizer.

## Level slider fill

**Guarantee.** The transport volume slider and the shade-mode volume slider paint both halves of
their rail `style::volume_level_color(volume)`: `theme::EQ_GAIN_LOW` (green) at 0.0,
`theme::VOLUME_SLIDER_FILL` (the mockup's orange) exactly at 0.5, and `theme::EQ_GAIN_HIGH` (red)
at 1.0. The transport balance slider paints both halves in `style::balance_level_color(balance)`:
`theme::BALANCE_SLIDER_FILL` (the mockup's green) exactly at center, `theme::EQ_GAIN_HIGH` at
either extreme, symmetric left and right. Both rail halves carry
the fill in every interaction status, so no gray segment remains on either side of the thumb.
The chrome thumb, the 4px rail width, the 1px dark rail border, and the square geometry are
unchanged from `chrome_slider_style`, and hover and drag still change only the handle face. The
seek bar keeps the neutral `theme::LCD_BACKGROUND` groove.

**Enforced by.** `style::level_slider_style` in `src/ui/style/slider_style.rs`, used by
`views::chrome_slider` in `src/ui/views/mod.rs` (the transport row and shade strip pass their fills
there), with the fills computed in `src/ui/style/level_tint.rs`.

**Pinned by.** `base_skin_colors_keep_their_documented_literal_values` in `src/ui/theme.rs` and
`level_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb` in
`src/ui/style/slider_style.rs`, and in `src/ui/style/level_tint.rs` the tests
`volume_color_hits_its_three_stops_exactly`, `volume_color_clamps_and_maps_nan_to_silence`,
`volume_color_between_stops_lies_between_neighbours`,
`balance_color_is_green_at_center_and_red_at_the_sides`, `balance_color_is_symmetric`, and
`level_colors_are_opaque`.

**History.** The fill is presentation-only: slider ranges, steps, messages, thumb geometry, and
playback behavior are unchanged.

## Volume wheel

**Guarantee.** A wheel roll over the player panels (the title bar, Now Playing bar, seek strip,
transport row, equalizer title band, and equalizer panel, plus the title bar and shade strip in
shade mode) moves the volume by exactly one `views::VOLUME_STEP` in the roll's vertical
direction: a positive `y` raises it through `handlers::mixer::volume_up`, a negative `y` lowers it
through `handlers::mixer::volume_down`, and both go through `AppState::nudge_volume` and the same
audio-backend forward as the arrow keys, so the clamp and the step cannot drift from them. Only
the sign of the delta matters, so one line or pixel event is one step. The browse list is outside
that area, so its `Scrollable` keeps the wheel and the volume is unchanged; a horizontal-only
roll or a zero direction changes no volume and schedules no work. A drawn seek slider is one
exception: it seeks instead (see `## Seek wheel`). The balance slider, the equalizer preamp, and
each equalizer band are the others: a roll over one moves that slider instead (see
`## Slider wheel`).

**Enforced by.** `views::volume_wheel_message` and its `views::wheel_direction` sign mapping in
`src/ui/views/mod.rs`, `Message::VolumeWheel` in `src/ui/message.rs`,
`handlers::mixer::volume_wheel` in `src/ui/handlers/mixer.rs` (the zero/NaN no-op), and the two
`MouseArea::on_scroll` wrappers in `src/ui/view/normal.rs` and `src/ui/view/shade.rs` (which also carry the options right-press), which the
balance, preamp, and band wrappers nest inside.

**Pinned by.** `volume_wheel_message_maps_a_vertical_roll_to_a_direction` and
`volume_wheel_message_zeroes_a_horizontal_only_roll` in `src/ui/views/tests/transport.rs`, plus
`volume_wheel_nudges_by_one_slider_step_and_ignores_zero` and `volume_wheel_clamps_at_the_ends`
in `src/ui/tests/mixer_tests.rs`.

## Seek wheel

**Guarantee.** A wheel roll over a *drawn* seek slider, in the Now Playing seek strip or the shade
strip, seeks instead of nudging the volume: a positive `y` publishes `Message::SeekForward` and a
negative `y` publishes `Message::SeekBackward` (the arrow keys' five-second step), and the roll
does not also reach the volume area around it. A horizontal-only, zero, or non-finite roll
publishes `Message::Ignored`. Only a drawn slider captures the wheel: a blank seek strip (no
position or no preview length) and every other player panel keep the volume nudge.

**Enforced by.** `seek_wheel_message` and the `MouseArea::on_scroll` around the slider in
`view_seek_bar` (`src/ui/views/seek_bar.rs`), and the same wrapper inside `MINI_SEEK_ID` in
`mini_transport_row_with_seek` (`src/ui/views/mini_transport.rs`).

**Pinned by.** `seek_wheel_message_maps_a_vertical_roll_to_a_direction`,
`seek_wheel_message_ignores_a_horizontal_only_roll` in `src/ui/views/tests/seek_wheel.rs`, and
`wheel_over_the_shade_seek_slider_seeks` (also in `seek_wheel.rs`), plus
`wheel_over_the_root_seek_slider_seeks_without_nudging_the_volume`
and `wheel_over_a_blank_seek_strip_still_nudges_the_volume` in `src/ui/tests/mixer_tests.rs`.

## Slider wheel

**Guarantee.** A plain wheel roll over the balance slider moves the balance by exactly one
`views::BALANCE_STEP` in the roll's vertical direction; a roll over the equalizer's preamp or any
band moves only that gain by exactly one `views::EQ_STEP` the same way. The sign mapping is
`views::wheel_direction`, so a positive `y` raises/moves right, a negative `y` lowers/moves left,
and a magnitude never counts as more than one step. Each move goes through the same clamped
setter and audio-backend forward as that slider's drag, so it cannot drift from the drag's range
or granularity; a hand move also clears the equalizer preset selection, as a drag does. These
rolls never set `WinampPlayer::mixer_drag`, so the LCD keeps its title and shows no readout. A
zero, horizontal-only, or `NaN` roll and an out-of-range band index are no-ops that schedule no
work, and these wrappers keep the roll from reaching the volume area around them. Every other
player panel still nudges the volume (see `## Volume wheel`), and the drawn seek slider still
seeks (see `## Seek wheel`).

**Enforced by.** `views::balance_wheel_message`, `views::eq_preamp_wheel_message`, and
`views::eq_band_wheel_message` (all through `views::wheel_direction`) in
`src/ui/views/mod.rs`; the `BalanceWheel`, `EqPreampWheel`, and `EqBandWheel` variants in
`src/ui/message.rs`; `handlers::mixer::{balance_wheel, eq_preamp_wheel, eq_band_wheel}` in
`src/ui/handlers/mixer.rs`; and the `MouseArea::on_scroll` wrappers around the balance slider in
`src/ui/views/transport.rs` and around the preamp and band sliders in
`src/ui/views/equalizer.rs`.

**Pinned by.** `balance_wheel_message_maps_a_vertical_roll_to_a_direction`
(`src/ui/views/tests/transport.rs`), `equalizer_wheel_messages_map_the_roll_direction_and_band`
(`src/ui/views/tests/equalizer.rs`), and
`balance_wheel_nudges_by_one_step_and_ignores_zero_and_nan`,
`eq_preamp_wheel_nudges_by_one_db_and_ignores_zero_and_nan`,
`eq_band_wheel_nudges_only_its_band_and_ignores_zero_nan_and_an_out_of_range_index`,
`slider_wheels_forward_their_new_values_to_the_audio_backend`,
`wheel_over_the_balance_slider_nudges_the_balance_not_the_volume`, and
`wheel_over_the_preamp_and_every_band_nudges_that_slider_not_the_volume` in
`src/ui/tests/mixer_tests.rs`.

## Now Playing label

**Guarantee.** The Now Playing bar shows the current track's human-readable title and artist, not
its internal id, whenever the id is in the known-track index:

- A known id renders that track's title and artist, borrowed from the index.
- A known id also renders the playlist number prefix (`<n>. `) cached in `KnownTrack::number`,
  drawn after the `Now Playing: ` caption and before the artist/title. The number is the 1-based
  position of the row that selected the track, so every new selection (Next/Previous and the
  preview-end advance included) records the new row's number, and re-playing the same id from
  another row or list refreshes it. It persists across browsing to another list and Stop.
- An unknown id falls back to the raw id for the label, the empty string for the artist, and no
  number prefix.
- No current track renders the label "Nothing", an empty artist, and no number prefix.
- Stop keeps the interrupted track's title and number (the current track is not cleared by Stop).
- The iced application window's title mirrors a known current track through
  `WinampPlayer::window_title`, using the same index and number prefix as the LCD, so it reads
  `<number>Artist - Title - nostalgia` and omits an empty number or artist. It ends in
  ` [Paused]` while paused and ` [Stopped]` while stopped, and nothing while playing. It keeps the
  interrupted track across Stop (then ending in ` [Stopped]`) and falls back to the plain
  `nostalgia`, never suffixed, for no current track or an unknown id.
- The number prefix counts toward the marquee character budget alongside the artist and title,
  and an overlong line's scrolling window includes it, so it neither overflows the LCD nor stays
  fixed while the rest of the line scrolls.
- An overlong title can be scrolled by hand: a left press on it (`Message::TitleDragPressed`) starts
  a drag, `needs_marquee` is false while `title_drag` is set, and `title_drag_message` is
  subscribed only while one is active. The first cursor move anchors, later moves set the offset one
  character per 10 px (wrapping on the line plus its gap), a left release or `CursorLeft` ends the
  drag, and a new selection clears it. A title that fits is not a drag target. Pinned by
  `dragged_marquee_offset_follows_the_pointer_and_wraps`,
  `marquee_cycle_len_is_the_line_plus_its_gap`,
  `the_overlong_title_publishes_a_drag_press_and_a_fitting_one_does_not`,
  `the_marquee_pauses_while_the_title_is_dragged_and_resumes_from_there`,
  `a_new_selection_clears_a_title_drag`, and
  `title_drag_listener_is_subscribed_only_while_dragging_and_maps_pointer_events`.
- The LCD splits its time readouts: by default the large left-hand readout shows the backend's
  reported position as zero-padded `MM:SS` (for example `00:53`), and the right edge always shows
  the plain cached total (for example `3:30`). Clicking the large readout switches it to the track's
  remaining time as `-MM:SS` (for example `-02:37`) and clicking again returns to elapsed. A known
  track with no reported position (no preview loaded, a failed start, or a backend that cannot
  answer) reads `00:00` at the left and the cached total at the right; no current track, or an
  unknown id, reads `--:--` at both edges. Pause keeps the stored position, so the paused player's
  frozen elapsed value stays on the bar; Stop and a new selection clear it immediately to `00:00`.
  While the transport is stopped the large readout for the current track draws transparent in both
  time modes and both blink phases whenever its text is not the `--:--` placeholder, keeping its
  bounds and its clickability off, so a stopped player shows no elapsed or remaining time other than
  the placeholder; the stored value is still the `00:00` Stop clears it to, and Play draws it again.
  The `--:--` placeholder still shows while stopped when no track is current, the id is unknown, or
  a known track in `Remaining` mode has no length, since it reports that there is no time to clear.
  A position reply issued before a Stop or a new selection is dropped rather than restoring the
  previous elapsed value, and the poll stays suspended while any backend transport transition is in
  flight, so it cannot read the previous player under the new epoch. A selection whose song has no
  preview stops the previous preview, so the backend reports no position for the new track and the
  elapsed readout reads `00:00` while the right edge keeps the cached total.
- The chosen time mode is presentation-only and persists across Pause, Stop, Next/Previous, and a
  new selection until the readout is clicked again. In remaining mode a known track counts down
  from its own length, treating a missing position as zero and saturating at `-00:00` when the
  position is past the total; a known track whose source supplied no length (`duration_ms == 0`)
  and any unknown or absent track read `--:--` at the left. The right edge keeps the plain total in
  both modes. The compact readouts in the library footer and the shade strip share this mode and
  its `views::now_playing::elapsed_readout_clickable` stopped guard, so clicking either can flip
  elapsed and remaining while playing or paused. The classic `T` key and Winamp's Ctrl+T reach the
  same `Message::ToggleTimeDisplay` through `shortcuts::message_for` in every transport state; the
  stopped readout's click guard makes only the readout inert, not the key.
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
  dropped. The poll asks for the format and the preview length only while the value it supplies is
  missing, so a steady preview's tick queries the position and the finished state alone: both are
  read once from the decoded source, and Stop or a new selection clears them.
- The LCD's visualizer row draws one fixed-width slot per band (`BAND_COUNT`), left to right in
  band order, bottom-aligned inside a fixed-height strip between the title and the status row.
  Each non-empty bar is shaded bottom to top in up to twelve 2 px ramp steps, from dark green at
  the rail through yellow to orange at its peak; the step is an absolute row above the rail, so
  the color follows the bar's height rather than being stretched over the bar. The bar heights
  come from the loaded preview's stored spectrum, each band clamped to `0.0..=1.0`; no reported
  spectrum, including the silent fallback, draws zero-height bars while keeping the row's height,
  so loading a preview does not move the title or status text. A silent band still reserves its
  fixed-width slot and simply draws no ramp segment. The row is a mouse area whose
  left press publishes `Message::ToggleVisualizer` (the same message as the `V` letter) whether the
  bars are shown or hidden, pinned by `visualizer_row_press_publishes_toggle_visualizer`. The bars
  are drawn only while `WinampPlayer::visualizer_visible` holds; hiding them changes only what the
  LCD draws, keeping both the stored spectrum and the poll, so showing them again draws the current
  bars. Pause keeps the last bars frozen, while Stop and a new selection clear them. A spectrum
  reply issued before a Stop or a new selection, or landing while a transport transition is in
  flight, is dropped. The spectrum poll is subscribed only while playback is running.
  Each band with a positive peak also draws a 1 px gray marker above its bar: it never sits
  inside its bar and always stays inside the strip (`visualizer_peak_marker_bottom`, pinned by
  `visualizer_peak_marker_bottom_sits_on_or_above_its_bar_inside_the_strip` and
  `now_playing_visualizer_row_draws_a_gray_peak_marker_above_a_fallen_bar`, with
  `now_playing_visualizer_row_keeps_its_height_with_peaks` for the row height). The peaks are
  `WinampPlayer::spectrum_peaks`, advanced by `next_spectrum_peaks` (hold the higher of the band
  and the previous peak less 0.125; pinned by
  `next_spectrum_peaks_follows_a_rising_band_and_falls_slowly_after_it`) inside the same reply
  guard as the spectrum (`a_stale_spectrum_reply_leaves_the_peaks_alone`,
  `spectrum_reply_stores_the_loaded_previews_spectrum`). Pause keeps them and Stop clears them
  (`stop_clears_the_spectrum_while_pause_keeps_it`), and hiding the visualizer hides them
  while keeping the stored levels
  (`toggle_visualizer_hides_and_shows_the_bars_without_clearing_the_spectrum`).
- The LCD's left edge always draws the five classic indicator letters `O`, `A`, `I`, `D`, `V`,
  in that order, at a fixed width, in every playback state including the empty `--:--` bar. `O`,
  `A`, `I`, and `V` are mouse areas: pressing `O` publishes `Message::ToggleOptions` (the same
  options menu as the transport row's lightning button), pressing `A` publishes
  `Message::ToggleAlwaysOnTop` (the LCD strip's own always-on-top control), pressing `I`
  publishes `Message::ToggleFileInfo` (the same panel as Alt+3), and pressing `V` publishes
  `Message::ToggleVisualizer`, which flips only the LCD's display flag. `D` stays decorative and
  publishes nothing. Each live letter draws `theme::LCD_GREEN` exactly while its mode holds (`O`
  for `WinampPlayer::options_visible`, `A` for `WinampPlayer::always_on_top`, `I` for
  `file_info_visible`, `V` for `visualizer_visible`), and every other letter, including the
  decorative `D` and an inactive live letter, draws `theme::LCD_GREEN_DIM`. The strip keeps its
  fixed width and letter order, so the content beside it does not move.
- A transport glyph leads the row in a fixed `LCD_TRANSPORT_WIDTH` slot before the large elapsed
  readout, so the readout keeps its x position as the glyph changes: green `▶` while playing, green
  `II` while paused, and red `■` while stopped. Pause of a loaded preview sets the shared paused
  mode while Play and Stop clear it; a pause with no loaded preview does not set it, so a Paused
  readout always has a preview Play can resume. `AppState::is_playing` alone no longer distinguishes
  Pause from Stop. `state::TransportMode` and `AppState::transport_mode` resolve the mode,
  `views::now_playing::transport_glyph` and `transport_glyph_color` choose the glyph and color, and
  `views::now_playing::now_playing_row` draws the slot. While the mode is `Paused`, the large
  elapsed readout blinks at the frozen position: `handlers::playback::preview::position_tick` flips
  `WinampPlayer::blink_visible` on each one-second position poll, and
  `views::now_playing::elapsed_readout_color` draws the text transparent when the phase is hidden or
  the transport is stopped and the text is not the `--:--` placeholder instead of removing it, and
  `views::now_playing::elapsed_readout_clickable` turns off its click while stopped, so the text
  keeps its bounds and the title, total, and status row do not move. A poll after Play or Stop holds
  the phase visible, so playing always shows the readout and the stopped view clears it unless it is
  the `--:--` placeholder, which still shows while stopped. The compact readout in the shade strip
  and library footer blinks in the same phase: `views::mini_transport::compact_readout_color` draws
  it transparent in the hidden phase without removing it, so the strip's controls do not move. The
  blink adds no timer or subscription.
  Pinned by `transport_glyph_names_and_colors_each_mode`,
  `now_playing_row_keeps_the_elapsed_readout_in_place_across_transport_modes`,
  `now_playing_content_draws_the_indicator_strip_beside_the_title`,
  `elapsed_readout_color_blinks_when_paused_and_clears_when_stopped`,
  `elapsed_readout_color_keeps_the_placeholder_visible_while_stopped`,
  `elapsed_readout_is_clickable_only_while_not_stopped`,
  `elapsed_readout_toggles_only_while_not_stopped`,
  and `now_playing_row_keeps_the_elapsed_bounds_when_the_blink_hides_it` in
  `src/ui/views/tests/now_playing.rs`, and
  `compact_readout_toggles_time_display_only_while_not_stopped`,
  `compact_readout_color_blinks_only_while_paused`, and
  `compact_readout_keeps_its_bounds_when_the_blink_hides_it` in `src/ui/views/tests/shade.rs`;
  `lcd_indicator_message_arms_o_a_i_and_v_and_leaves_d_inert` and
  `lcd_indicator_strip_publishes_options_on_o_always_on_top_on_a_visualizer_on_v_and_file_info_on_i` in
  `src/ui/views/tests/now_playing.rs`;
  `toggle_visualizer_hides_and_shows_the_bars_without_clearing_the_spectrum` in
  `src/ui/tests/panels_tests.rs`; `position_tick_blinks_the_elapsed_readout_only_while_paused` in
  `src/ui/tests/`; and `toggle_playing_switches_between_playing_and_paused` and
  `pause_sets_the_paused_mode_while_play_and_stop_clear_it` in `src/state.rs`; and
  `pause_stops_playing_but_keeps_current_track`,
  `pause_without_a_loaded_preview_leaves_the_transport_stopped`,
  `toggle_without_a_loaded_preview_stops_the_transport`,
  `pause_after_stop_leaves_the_transport_stopped`, `resume_resumes_the_audio_backend`,
  `stop_stops_the_audio_backend`, `toggle_play_pause_pauses_while_playing`, and
  `resume_after_stop_restarts_the_preview` in `src/apple_music/tests/transport.rs`.
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
- A number-prefixed artist/title line longer than `MARQUEE_WIDTH` characters is windowed by
  `views::now_playing::marquee_text`, advanced one character per `Message::MarqueeTick`, and
  reset to the start by `TrackSelected`; a line that fits is shown whole and borrowed.
- The marquee timer is subscribed only while the line needs to scroll: `WinampPlayer::needs_marquee`
  measures the same number, label, and artist the bar renders, so a line that fits leaves the timer
  out (every message makes iced rebuild the whole visible screen, the library panel included while
  it shows).
- The position poll is subscribed only while there is a position to refresh:
  `WinampPlayer::needs_position_poll` includes it while playback is running, the backend still holds
  a preview (paused, even before its first position reply, so a seek can still move it), or a stored
  position is still shown, and leaves it out for a stopped player, whose view clears its current
  track's readout while stopped so there is nothing to poll. Without the gate, every tick and
  reply would rebuild the whole screen twice a second for an unchanged readout.

**Enforced by.** `views::now_playing_label` and `views::now_playing_artist` in
`src/ui/views/now_playing/text.rs` (re-exported by `src/ui/views/mod.rs`), pure functions over the
known-track index. `WinampPlayer::now_playing_label` and `WinampPlayer::now_playing_artist` in
`src/ui/player/presentation.rs` resolve against `known_tracks`, so the bar keeps naming a playing
track after
browsing to another album replaced the visible song list. `views::now_playing::now_playing_elapsed`
(through `format_lcd_elapsed` for elapsed mode and `format_lcd_remaining` for remaining mode)
formats the left-hand readout as chosen by `views::now_playing::TimeDisplay`, while
`views::now_playing::now_playing_time` returns the plain cached total for the right edge; the
`playback_position` field on `WinampPlayer` stores the reply the `PlaybackPosition` handler
received, which is stamped with `position_epoch`, and `WinampPlayer::readout_position` prefers an
in-progress seek drag over that value so the large readout and the seek thumb share one position
source; `reset_position_readout` bumps that epoch and clears the value on Stop and on a new
selection. `pending_position_transitions` counts the backend transport transitions that are still in
flight: every transport handler in `src/ui/handlers/playback.rs` that schedules one begins a
transition, and the completion that reports that same transition ends it; while the count is nonzero
the `handlers::playback::preview` poll handlers issue no query and its reply handlers store no
reply. `AppleMusicService::play_track` stops the backend's previous preview when a selection has no
preview URL. `views::now_playing::indicator::lcd_indicator_strip` draws
`views::now_playing::indicator::LCD_INDICATOR_LABELS` down the LCD's left edge in a fixed-width
column, coloring each letter through `views::now_playing::indicator::indicator_label_color` from
the `views::now_playing::indicator::IndicatorModes` flags `views::view_now_playing` passes
(`options_visible`, `always_on_top`, `file_info_visible`, and `visualizer_visible`); its `O`,
`A`, `I`, and `V` cells publish `Message::ToggleOptions`, `Message::ToggleAlwaysOnTop`,
`Message::ToggleFileInfo`, and
`Message::ToggleVisualizer` through `views::now_playing::indicator::lcd_indicator_message`,
while `D` publishes nothing. `views::now_playing::visualizer::visualizer_bar_heights` scales
and clamps the stored spectrum, and `views::now_playing::visualizer::now_playing_visualizer_row`
draws its `BAND_COUNT` bars at fixed width inside the row `views::view_now_playing` places between
the title and status rows. The `Spectrum` handler in `src/ui/handlers/playback/preview.rs` stores
the reply under the same epoch and pending-transition guards as the position, format, and duration,
`spectrum_tick` schedules the backend query, and `reset_position_readout` clears the bars on Stop
and a new selection. `ui::subscriptions::subscriptions` subscribes `spectrum_ticks` only while
`WinampPlayer::needs_spectrum_poll` reports that playback is running.
`views::now_playing::now_playing_number` resolves `KnownTrack::number`,
`handlers::playback::track_selected` records the selecting row's 1-based number, and
`views::now_playing::now_playing_row` draws it (separately when the line fits, inside the window
when it scrolls). `views::now_playing::marquee_text` windows an overlong line, and
`WinampPlayer::marquee_offset` carries the advance through `Message::MarqueeTick`. The
`WinampPlayer::time_display` field holds the readout's mode, and
`handlers::playback::toggle_time_display` (routed from `Message::ToggleTimeDisplay`, pressed by the
`MouseArea` wrapping the readout in `now_playing_row` and mapped from the classic `T` key (and
Ctrl+T) by `shortcuts::message_for`) flips it without touching playback state.
`views::now_playing::display_len` and `views::now_playing::needs_marquee` measure the line, and
`ui::subscriptions::subscriptions` subscribes `marquee_ticks` only while that measure exceeds
`MARQUEE_WIDTH`, and `position_ticks` only while `WinampPlayer::needs_position_poll` reports a
preview whose position can still change. `audio::PlaybackFormat` carries the preview's sample rate,
channel count, and average bitrate; `audio::preview::DecodedPreview` hands the download's encoded
byte count to `format_for_source` in `src/audio/player/format.rs`, which reads the rate and channels
from the decoded `rodio` source and computes the bitrate through `preview_bitrate_kbps` from the
source's own total duration, and
`views::now_playing::text::format_playback_prefix`/`text::channel_mode_label` format them, and
`views::now_playing::status::CHANNEL_LABELS`/`status::channel_label_color`/`status::now_playing_status_row`
render them in the LCD's status row built by `views::view_now_playing`. The `PlaybackFormat` handler
in `src/ui/handlers/playback/preview.rs` stores the reply beside the position under the same epoch
and pending-transition guards, and `reset_position_readout` clears the format on Stop and a new
selection; `position_tick` queries the position and the finished state on every tick, and the format
and the preview's length only while each is still unknown. `views::seek_bar::seek_bar_value`
resolves the thumb and scale from the position and the `playback_duration` field, and
`views::view_seek_bar` builds the strip: a full-width
`Message::SeekPreviewChanged`/`Message::SeekPreviewReleased` slider when that scale exists, or an
empty fixed-height stand-in otherwise. The `PlaybackDuration` handler stores that reply under the
same epoch and pending-transition guards as the position and format, `seek_preview_changed` stores
the drag without reaching the backend, and `seek_preview_released` calls
`AppleMusicService::seek_output_to`, which forwards to `AudioOutput::seek_to`; the `duration` query
reaches it through `AudioOutput::duration`. `reset_position_readout` clears the duration and the
drag on Stop and a new selection, and `RodioOutput::seek_to`/`duration` query the worker, whose
`RodioPlayer` records the decoded source's length and clamps each absolute seek to it.

The scrolling timer's stream is built by `ui::ticks::tick_stream` in `src/ui/ticks.rs`, and its
thread starts through `spawn_marquee_thread` in `src/ui/subscriptions/marquee.rs`, which delegates
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
- `now_playing_number_resolves_known_track_to_its_cached_prefix`
- `now_playing_time_resolves_known_track_to_cached_label`
- `now_playing_time_returns_the_plain_total_even_with_a_position`
- `now_playing_elapsed_reads_placeholder_zero_or_position`
- `now_playing_elapsed_remaining_counts_down_from_the_total`
- `format_lcd_elapsed_pads_minutes_and_seconds`
- `format_lcd_remaining_prefixes_a_minus_and_pads`
  (all in `src/ui/views/tests/`)
- `toggle_time_display_switches_to_remaining_and_persists`
- `toggle_time_display_with_no_track_keeps_the_placeholder`
- `t_toggles_the_time_display_from_the_keyboard`
  (all in `src/ui/tests/`)
- `marquee_text_keeps_a_line_that_fits`
- `marquee_text_windows_from_the_offset`
- `marquee_text_wraps_through_the_gap_to_the_start`
- `marquee_text_counts_characters_not_bytes`
- `display_len_counts_the_number_the_label_the_artist_and_the_separator`
- `needs_marquee_only_for_a_line_too_long_for_the_lcd`
- `overlong_now_playing_line_renders_a_new_window_as_the_offset_advances`
- `now_playing_row_draws_the_number_between_the_caption_and_the_artist`
- `an_overlong_now_playing_line_windows_the_number_with_the_title`
- `now_playing_row_draws_no_number_when_none_is_known`
  (all in `src/ui/views/tests/`)
- `now_playing_label_keeps_the_track_name_after_browsing_to_another_album` (`src/ui/tests/readouts_tests.rs`)
- `window_title_names_the_current_track` (`src/ui/tests/`)
- `window_title_marks_a_paused_or_stopped_track` (`src/ui/tests/`)
- `now_playing_number_survives_browsing_to_another_album` (`src/ui/tests/`)
- `track_selected_records_the_selected_rows_number`
- `replaying_a_track_from_another_row_updates_its_number`
- `the_player_knows_when_the_now_playing_line_needs_the_marquee`
- `now_playing_time_shows_the_played_tracks_duration_after_browsing_to_another_album`
  (`src/ui/tests/`)
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
- `position_tick_skips_the_format_and_length_once_both_are_known`
- `position_tick_queries_only_the_missing_value`
- `position_tick_stream_emits_position_ticks`
- `a_refused_position_thread_ends_the_tick_stream_instead_of_panicking`
- `a_refused_position_thread_reports_the_cause_and_the_loss`
- `the_position_thread_is_named_nostalgia_position_poll`
- `the_position_timer_runs_only_while_there_is_a_position_to_refresh`
- `a_paused_preview_needs_the_position_poll_before_its_first_reply` (`src/ui/tests/`)
- `play_track_without_a_preview_url_stops_the_previous_preview` (`src/apple_music/tests/play_track.rs`)
- `position_query_reports_none_when_nothing_is_loaded`
- `a_seek_is_reflected_by_the_next_position_query`
- `a_pause_keeps_the_loaded_player_reporting_its_position`
- `format_for_source_reads_sample_rate_and_channels`
- `preview_bitrate_kbps_computes_encoded_bits_over_the_preview_length`
- `preview_bitrate_kbps_rounds_to_the_nearest_kilobit_and_rejects_overflow`
- `playback_format_query_returns_the_loaded_players_format`
- `playback_format_query_reports_none_when_nothing_is_loaded`
- `rodio_output_asks_the_worker_for_its_format`
- `silent_output_reports_no_playback_format` (`src/audio/tests/rodio_output.rs`)
- `playback_format_forwards_the_audio_output_format` (`src/apple_music/tests/audio_forwarding.rs`)
- `format_playback_prefix_formats_khz_and_bitrate` (`src/ui/views/tests/now_playing.rs`)
- `channel_mode_label_names_the_reported_channels`
- `channel_label_color_lights_only_the_reported_mode`
- `now_playing_status_row_lights_the_reported_channel_mode` (`src/ui/views/tests/lcd_display.rs`)
- `lcd_indicator_strip_draws_each_label_in_order_at_a_fixed_width`
- `indicator_label_color_lights_only_the_active_mode`
- `lcd_indicator_strip_lights_the_active_letters_and_dims_the_rest`
- `now_playing_content_draws_the_indicator_strip_beside_the_title`
  (`src/ui/views/tests/now_playing.rs`)
- `seek_bar_value_clamps_the_thumb_and_hides_without_a_length`
- `view_seek_bar_constructs_with_and_without_a_length` (`src/ui/views/tests/lcd_display.rs`)
- `playback_duration_reply_stores_the_loaded_previews_length`
- `playback_duration_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`
- `stop_and_track_selection_clear_the_playback_duration`
- `seek_preview_changed_tracks_the_drag_without_touching_the_backend`
- `seek_preview_released_seeks_once_to_the_dragged_position`
- `seek_preview_released_without_a_drag_does_nothing`
- `seek_preview_released_invalidates_an_in_flight_position_reply`
- `seek_preview_released_backend_failure_leaves_the_position_alone`
- `position_tick_queries_position_and_duration` (`src/ui/tests/`)
- `playback_duration_forwards_the_audio_output_duration` (`src/apple_music/tests/audio_forwarding.rs`)
- `seek_output_to_forwards_an_absolute_position`
- `seek_output_to_propagates_an_audio_backend_failure` (`src/apple_music/tests/audio_forwarding.rs`)
- `silent_output_reports_no_playback_duration`
- `duration_for_source_reads_a_samples_buffers_length`
- `clamp_seek_clamps_to_the_preview_length_and_passes_through_without_one`
- `rodio_output_asks_the_worker_for_its_duration`
- `rodio_output_reports_none_when_the_duration_reply_times_out`
- `seek_to_command_reaches_the_loaded_player`
- `duration_query_returns_the_loaded_players_duration`
- `duration_query_reports_none_when_nothing_is_loaded` (`src/audio/tests/worker.rs`)
- `playback_format_reply_stores_the_loaded_previews_format`
- `playback_format_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`
- `stop_clears_the_playback_format_while_pause_keeps_it`
- `track_selected_clears_the_playback_format`
- `position_tick_queries_position_and_format` (`src/ui/tests/readouts_tests.rs`)
- `marquee_tick_advances_the_offset`
- `marquee_ticks_keep_ticking_when_the_receiver_falls_behind`
- `visualizer_bar_heights_scale_and_clamp_the_reported_bands`
- `visualizer_bar_heights_maps_a_nan_band_to_empty` (`src/ui/views/tests/lcd_display.rs`)
- `visualizer_bar_segments_ramp_from_dark_green_to_orange` (`src/ui/views/tests/lcd_display.rs`)
- `now_playing_content_keeps_the_visualizer_height_with_and_without_a_spectrum`
- `now_playing_visualizer_row_keeps_its_height_without_a_spectrum` (`src/ui/views/tests/lcd_display.rs`)
- `now_playing_visualizer_row_draws_one_fixed_width_bar_per_band` (`src/ui/views/tests/lcd_display.rs`)
- `now_playing_visualizer_row_paints_its_ramp_from_green_to_orange` (`src/ui/views/tests/lcd_display.rs`)
- `spectrum_reply_stores_the_loaded_previews_spectrum`
- `spectrum_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`
- `stop_clears_the_spectrum_while_pause_keeps_it`
- `spectrum_tick_queries_spectrum_and_carries_the_epoch`
- `the_spectrum_timer_runs_only_while_playback_runs` (`src/ui/tests/`)
- `a_refused_spectrum_thread_ends_the_tick_stream_instead_of_panicking` (`src/ui/tests/`)
- `a_refused_spectrum_thread_reports_the_cause_and_the_loss` (`src/ui/tests/`)
- `the_spectrum_thread_is_named_nostalgia_spectrum_poll` (`src/ui/tests/`)
- `playback_spectrum_forwards_the_audio_output_spectrum` (`src/apple_music/tests/audio_forwarding.rs`)
- `the_marquee_timer_runs_only_while_the_line_needs_to_scroll`
- `the_player_knows_when_the_now_playing_line_needs_the_marquee`
- `track_selected_resets_the_marquee_offset` (`src/ui/tests/`)
- `a_refused_marquee_thread_ends_the_tick_stream_instead_of_panicking`
- `a_refused_marquee_thread_reports_the_cause_and_the_loss`
- `the_marquee_thread_is_named_nostalgia_marquee` (all in `src/ui/tests/`)
- `dropping_the_receiver_ends_the_tick_thread` (`src/ui/ticks.rs`)

**History.** Reconstructed from "Now Playing bar shows the track id instead of the song title" in
`BUGS-FIXED.md`.

## Window shade

**Guarantee.** Rolling the window up into shade mode keeps playback controllable:

- Ctrl+W (Control, with optional Shift, and without Alt or Logo) publishes
  `Message::ToggleWindowShade`, the same message as the shade button; plain W stays unbound.
  Pinned by `message_for_maps_ctrl_w_to_toggle_window_shade` and
  `ctrl_w_rolls_the_window_up_and_back`.

- A launch whose settings say the window was shaded opens rolled up: `adopt_saved_settings`
  restores `WinampPlayer::shaded` before the first view, so the root draws the title bar plus
  strip; once `WindowIdResolved` lands, `window_id_resolved` schedules `iced::window::size`
  (batched with a saved always-on-top level when both apply), and `WindowShadeMeasured` stores
  the launch size before resizing to `SHADED_HEIGHT`, so the first unshade restores it. An
  unshaded launch schedules no measurement, and the size and position themselves stay
  unpersisted.

- Shade mode builds exactly the title bar over the compact transport strip, and the window is
  resized to that same height (`SHADED_HEIGHT = TITLE_BAR_HEIGHT + SHADE_STRIP_HEIGHT`). The
  strip keeps a fixed `SHADE_STRIP_HEIGHT`, which aliases the shared
  `MINI_TRANSPORT_HEIGHT`, and normal mode builds no shade strip.
- The strip shows Previous, Play, Pause, Stop, Next, the elapsed-over-total time readout, a seek
  slider while a preview position and length are known, and a volume slider. Its five buttons
  emit the same messages as their full transport-row twins
  (`Message::PreviousButton`, `Message::Play`, `Message::Pause`, `Message::Stop`,
  `Message::NextButton`), each pinned to a fixed width so the row cannot reflow. The strip builds
  them from the shared compact row's `mini_transport_controls`, which projects the full row's
  `transport_glyphs` spec, so the shade strip, the library footer, and the full row share the
  same glyphs, order, widths, and messages; the row is playback-independent and always draws
  both `▶` and `II`. Without a seek slider a fill spacer pushes the volume slider to the right
  edge, after the left-aligned buttons and time readout.
- The time readout joins the same `WinampPlayer::now_playing_elapsed` and
  `WinampPlayer::now_playing_time` strings the Now Playing bar draws as `<elapsed>/<total>`, so
  the elapsed/remaining choice and the track's total carry into shade mode. When the total is the
  `--:--` placeholder the readout is just the elapsed string, never `--:--/--:--`. The volume
  slider writes `Message::VolumeChange` and reads the shared volume.
- While the backend reports both a playback position and a non-zero preview length, the strip
  draws a seek slider between the time readout and the volume slider, over that preview length
  rather than the catalog track's length, with its thumb clamped into `0..=length`. It comes from
  `views::seek_bar::seek_slider`, the same constructor the Now Playing bar uses, so it emits only
  `Message::SeekPreviewChanged` while dragging and `Message::SeekPreviewReleased` on release: the
  Now Playing bar's backend seek path. A held drag previews through the shaded readout because
  the caller resolves the time string through `WinampPlayer::readout_position`. With no position,
  no length, or a zero length, no seek slider is drawn and the row keeps the fill space that pins
  the volume slider to the right edge. The library footer builds no seek slider.
- While playing or paused, the time readout is a click target that publishes
  `Message::ToggleTimeDisplay`, flipping the same `WinampPlayer::time_display` mode the LCD uses;
  while stopped a single press publishes nothing and a double-click publishes only the no-op
  `Message::Ignored`, so a stopped press cannot silently switch the persisted mode. Because shade
  mode builds no LCD, this is the only time toggle available there. The shared row applies the
  LCD's own `views::now_playing::elapsed_readout_clickable` guard rather than a second rule.
- A double-click on the strip's blank chrome publishes the same
  `Message::ToggleWindowShade` as the title bar's double-click and the shade button, restoring the
  window through `handlers::window::toggle_window_shade`; a single click publishes nothing. The
  strip's five buttons, time readout, seek slider, and volume slider keep their own presses,
  drags, toggles, and wheel behavior because `MouseArea` forwards events to its content first and
  stops when the content captures them; the inert stopped readout swallows the double-click
  through its own no-op handler instead of restoring, and the strip keeps `SHADE_STRIP_HEIGHT` and
  its blue chrome.
- Opening the About panel while shaded restores the window first through the same
  `handlers::window::toggle_window_shade`, so the shade tree stays the title bar plus strip and
  never draws the panel.

**Enforced by.** `views::shade::view_shade_strip` wraps
`views::mini_transport::mini_transport_strip_with_seek` on the title bar's blue chrome in a
`MouseArea::on_double_click(Message::ToggleWindowShade)` (the library footer keeps
`mini_transport_strip`, which builds no seek slider);
`views::mini_transport::mini_transport_controls` projects
`views::transport::transport_glyphs` into the
compact buttons; `views::mini_transport::mini_transport_readout` joins the elapsed and total
strings and keeps a single time string when the total is unknown;
`views::mini_transport::mini_transport_row_with_seek` tags its seek container
`views::mini_transport::MINI_SEEK_ID` and builds the slider from
`views::seek_bar::seek_slider`, which returns `None` unless `views::seek_bar::seek_bar_value`
resolves a position and non-zero length; the shared
`views::mini_transport::mini_transport_row_with_optional_seek` wraps the readout
in a `MouseArea::on_press(Message::ToggleTimeDisplay)` only when
`views::now_playing::elapsed_readout_clickable` holds for the strip's mode, and wraps a stopped
readout in `MouseArea::on_double_click(Message::Ignored)` so it captures the strip's restore press
with a no-op; `views::mini_transport::mini_volume_spec` fixes the slider's shared range
and `Message::VolumeChange`; `views::mini_transport::MINI_TRANSPORT_HEIGHT` owns the height, and
`views::shade::SHADE_STRIP_HEIGHT` aliases it while `views::shade::SHADED_HEIGHT` adds the title
bar; the shaded branch of `ui::view::view` stacks the bar and strip; and
`handlers::window::resize_to_shade` resizes to `handlers::window::shade_target_size`, which reads
`SHADED_HEIGHT`; `handlers::window::window_id_resolved` additionally schedules the launch-size
query for a restored shade and batches it with the saved always-on-top level when both apply.
`views::view_shade_strip` is re-exported by `src/ui/views/mod.rs`.

**Pinned by.** `transport_controls_draw_play_and_pause_in_every_playback_state`,
`shade_controls_map_to_transport_messages_and_keep_fixed_widths`,
`main_transport_and_shade_strip_share_glyphs_order_and_widths`,
`shade_height_pins_the_rolled_up_window`,
`shade_volume_slider_spec_matches_the_shared_volume_range_and_message`,
`shade_strip_draws_the_time_readout_and_puts_the_volume_slider_at_the_right_edge`,
`shade_strip_shows_a_seek_slider_only_when_position_and_length_are_known`,
`shade_strip_double_click_restores_the_window`,
`shade_strip_double_clicks_on_controls_do_not_restore_the_window`,
`mini_transport_readout_appends_the_known_total`,
`compact_readout_toggles_time_display_only_while_not_stopped`, and
`view_shade_strip_keeps_its_fixed_height_for_both_playback_states` (`src/ui/views/tests/shade.rs`);
`shaded_root_view_is_the_title_bar_plus_the_strip`, `shade_strip_draws_only_while_shaded`,
`shade_root_readout_previews_the_seek_drag`,
`shade_target_size_is_the_title_bar_plus_the_strip`,
`title_bar_height_pins_the_shade_strip`, and `root_view_footer_shows_elapsed_over_total`
(`src/ui/tests/view_tests.rs`), `window_id_resolved_measures_a_restored_shade`
(`src/ui/tests/window_tests.rs`), and `toggle_window_shade_reaches_the_saved_snapshot` and
`saved_shade_is_restored_on_boot` (`src/ui/tests/settings_tests.rs`).

**History.** Added with the window-shade transport strip; the earlier shade mode built the title
bar alone. The strip's controls now come from the shared compact row the library footer uses; the
shade strip additionally draws the Now Playing bar's seek slider from `seek_bar::seek_slider`.
A double-click on the strip now restores the window, matching the title bar's double-click. Opening
About now restores the window first, so the panel never draws in shade mode (2026-10-24). A saved
shade mode now restores at boot: the rolled-up tree draws from the first view, the launch size is
measured once the window id resolves, and the first unshade restores it (2026-10-26).

## About panel

**Guarantee.** The app bar's leftmost `M` button opens a transient, unpersisted About panel:

- `views::view_title_bar` draws `M` in the left slot and publishes `Message::ToggleAbout` on a
  press. The LCD strip's `A` keeps publishing `Message::ToggleAlwaysOnTop`; the title bar no
  longer has an always-on-top control.
- `shortcuts::shortcut` maps Ctrl+F1 (Control with optional Shift, no Alt or Logo) to
  `Message::ToggleAbout`, so About has a keyboard route like the `M` button; Ctrl+Alt+F1 and
  Ctrl+Logo+F1 stay unbound.
- `handlers::window::toggle_about` flips `WinampPlayer::about_visible`. Opening it while
  `WinampPlayer::shaded` holds first calls `toggle_window_shade` and returns that task, so the
  window restores before the panel draws; the flag is layout-only and not persisted.
- While the flag holds outside shade mode, `views::view_about_overlay` stacks the panel over the
  otherwise unchanged normal-mode base: `views::about::ABOUT_VERSION` (a `v`-prefixed
  `CARGO_PKG_VERSION`), `ABOUT NOSTALGIA`, the mockup tagline, the active library source from
  `views::about::library_source_label`, and a `Close` button. The label reads `Library: Apple
  Music` when the service has a stored session and `Library: Sample library` otherwise, and the
  overlay assembly in `src/ui/view/overlay.rs` passes the current `AppleMusicService::session()` result
  on every rebuild, so a sign-in that lands while the panel is open updates it without a new
  message or stored UI state. `Close` and
  the full-window backdrop each publish one `Message::ToggleAbout`; a press on the panel's own
  chrome publishes `Message::Ignored` and leaves it open. The shade branch never draws the panel.
- A plain or Shift-held Escape press also closes it (see `## Transient panel dismissal`).

**Enforced by.** `views::title_bar::view_title_bar`, `views::about::view_about_overlay` and
`views::about::library_source_label` (over
`views::modal::modal_overlay`), the `AppleMusicService::session()` read in the overlay assembly
in `src/ui/view/overlay.rs`, `Message::ToggleAbout`, `handlers::window::toggle_about`,
`WinampPlayer::about_visible`, and the overlay selection in `src/ui/view/overlay.rs`,
which wraps the base tree only while `about_visible && !shaded` holds.

**Pinned by.** `title_bar_m_button_publishes_toggle_about`,
`about_overlay_draws_the_name_version_tagline_and_library_source`,
`about_overlay_closes_from_close_and_backdrop_but_ignores_its_chrome`,
and `library_source_label_names_the_signed_in_and_sample_libraries`
(`src/ui/views/tests/chrome.rs` and `src/ui/views/about.rs`); `message_for_maps_ctrl_f1_to_about` (`src/ui/shortcuts.rs`);
`toggle_about_shows_and_hides_the_panel_without_touching_playback`
and `toggle_about_while_shaded_restores_the_window_first` (`src/ui/tests/window_tests.rs`);
`view_constructs_over_the_apps_full_input_space`, `view_constructs_when_the_window_is_shaded`,
`about_panel_names_the_active_library_source`, and
`title_bar_constructs_for_both_about_states` (`src/ui/tests/view_tests.rs`);
`the_escape_listener_runs_only_while_a_transient_panel_is_open`,
`escape_key_message_maps_escape_but_not_its_chords_or_other_keys`
(`src/ui/tests/subscriptions_tests.rs`), and
`escape_closes_each_transient_panel_without_touching_anything_else`
(`src/ui/tests/panels_tests.rs`) pin the Escape route.

**History.** Added the library-source line (2026-10-26): it reads the service's current session
on each rebuild rather than caching the source in UI state.

## Options panel

**Guarantee.** The LCD strip's `O`, the transport row's lightning button, and a right-press on the player panels or shade strip (not the library) open a transient,
unpersisted options menu listing the app's existing on/off choices plus the 10 songs back, 10 songs forward, Jump to time and Jump to file rows, About, and Close:

- Alt+O (without Control, Shift, or Logo) toggles the menu, as does a right-press on the player panels (`MouseArea::on_right_press(Message::ToggleOptions)`); with the menu open the base is ignored, so a second right-press does nothing. Opening resets the layout-only
  `options_selected` index to zero. Up/Down clamp the highlight to the first/last row; Enter
  invokes the highlighted row through its existing mouse handler. Closed-menu navigation
  messages do nothing. The selected row uses blue `ROW_HIGHLIGHT` and white `PLAYING_ROW_TEXT`
  in each button status; unselected rows retain chrome styling.
  The root `OptionsShortcut` wrapper keeps Alt+O from reaching focused inputs,
  preserving the search query before the subscription toggles the menu.
  Alt+O receives captured key events with the menu open or closed, as does menu
  navigation while open; messages from the covered base
  are discarded, so a focused search box cannot submit behind the menu.
- `handlers::window::toggle_options` flips `WinampPlayer::options_visible`. Opening it first calls
  `close_transient_panels`, and while `WinampPlayer::shaded` holds it first calls
  `toggle_window_shade`, so the window restores before the menu draws. The flag is layout-only and
  not persisted; the menu adds no setting.
- `views::view_options_overlay` stacks an `OPTIONS` panel over the otherwise unchanged normal-mode
  base through `views::modal::modal_overlay`. Its rows are `Always on Top: On/Off`, `Time:
  Remaining/Elapsed`, `Visualizer: Shown/Hidden`, `Equalizer: Shown/Hidden`, `Library:
  Shown/Hidden`, `Stop after current: On/Off`, `Repeat: On/Off`, `Shuffle: On/Off`, `10 songs back`, `10 songs forward`, `Jump to time`, `Jump to file`, `About Nostalgia`, and `Close`, with the labels resolved from the passed current
  state on every rebuild. Each choice row publishes `Message::OptionsChoice(..)`; the About row
  publishes `Message::ToggleAbout`; `Close`, the full-window backdrop, and Escape close the menu.
- `handlers::window::options_choice` clears `options_visible` first, then dispatches to the
  existing `toggle_always_on_top`, `handlers::playback::toggle_time_display`,
  `handlers::playback::toggle_visualizer`, `handlers::window::toggle_equalizer_panel`, or
  `handlers::window::toggle_library_panel`, `handlers::playback::toggle_stop_after_current`, `handlers::playback::toggle_repeat`, `handlers::playback::toggle_shuffle`, `handlers::playback::stepping::jump_ten_tracks` (back or forward, pinned by `options_ten_song_rows_jump_like_the_keypad`), `handlers::jump::toggle_jump_time`, or `handlers::jump::toggle_jump_file` and returns its task, so each row's non-menu behavior is
  exactly that toggle's.
- The overlay assembly in `src/ui/view/overlay.rs` wraps the base tree only while `options_visible &&
  !shaded` holds and passes the current always-on-top, elapsed/remaining, visualizer, equalizer
  panel, library panel, stop-after-current, repeat, and shuffle state (repeat and shuffle read from the shared state) through `views::OptionsState`; the shade branch never draws it.
- A plain or Shift-held Escape press also closes it (see `## Transient panel dismissal`).

**Enforced by.** `views::options::view_options_overlay`, its eight label helpers, and `Message::ToggleOptions`
and `Message::OptionsChoice`; `handlers::window::toggle_options` and
`handlers::window::options_choice`; `WinampPlayer::options_visible`; the LCD strip's `O`
mapping and the transport row's `options_button`; the two `on_right_press` wrappers in `src/ui/view/normal.rs` and `src/ui/view/shade.rs`; and the normal-mode branch of
`src/ui/view/overlay.rs`. `OptionsRow::ALL` and `OptionsRow::message` share row order and messages;
`options_row_label` and `options_row_style` draw them. `options_shortcut` and
`message_for_options` map the keys; `options_selection_up`, `options_selection_down`, and
`options_activate` handle navigation. The subscription chooses Jump to File first, then the
`options_open` mapper, then normal shortcuts.

**Pinned by.** `options_shortcut_maps_alt_o_and_rejects_other_chords` and
`alt_o_routes_captured_text_with_options_open_or_closed` and
`options_navigation_routes_captured_search_keys_without_submitting`
(`src/ui/tests/subscriptions_tests.rs`);
`message_for_options_maps_up_down_enter_and_falls_back` (`src/ui/shortcuts.rs`);
`options_rows_map_to_mouse_messages_in_draw_order` (`src/ui/message.rs`);
`options_row_style_highlights_selection_in_each_status` (`src/ui/views/options.rs`);
`options_jump_rows_open_their_panels_or_just_close` (`src/ui/tests/jump_tests.rs`);
`options_keyboard_selection_clamps_and_activates_each_row` and the mouse/keyboard toggle
expectations in `options_choice_applies_each_toggle_and_closes_the_panel`
(`src/ui/tests/panels_tests.rs`); `options_overlay_draws_each_row_with_its_state`,
`options_overlay_publishes_each_choice_and_closes_from_close_and_backdrop`, and
`transport_options_button_publishes_toggle_options` (`src/ui/views/tests/chrome.rs`); the
label tests in `src/ui/views/options.rs` (including `stop_after_current_label_mirrors_the_flag` and `repeat_and_shuffle_labels_mirror_their_flags`); `options_repeat_and_shuffle_toggle_shared_state_and_close_the_panel` (`src/ui/tests/panels_tests.rs`); `options_stop_after_current_toggles_the_flag_and_closes_the_panel`
(`src/ui/tests/panels_tests.rs`); `lcd_indicator_message_arms_o_a_i_and_v_and_leaves_d_inert`,
`indicator_label_color_lights_only_the_active_mode`, and
`lcd_indicator_strip_publishes_options_on_o_always_on_top_on_a_visualizer_on_v_and_file_info_on_i`
(`src/ui/views/tests/now_playing.rs`); `options_choice_applies_each_toggle_and_closes_the_panel`,
`toggle_options_opens_and_closes_and_excludes_each_other_panel`, and
`escape_closes_each_transient_panel_without_touching_anything_else`
(`src/ui/tests/panels_tests.rs`);
`toggle_options_while_shaded_restores_the_window_first`
(`src/ui/tests/window_tests.rs`); `right_press_on_the_player_panels_toggles_options`,
`right_press_on_the_library_does_not_toggle_options`, and
`right_press_on_the_shade_strip_toggles_options` (`src/ui/tests/panels_tests.rs`); and
`options_panel_draws_over_the_player_and_drops_out_in_shade_mode`
(`src/ui/tests/view_tests.rs`).

**History.** Added 2026-10-26 with the Winamp-style options menu; the menu is transient and
unpersisted because every row reuses an existing toggle.
Keyboard access and the transient row highlight added 2026-10-26 by the feature role.

## File info panel

**Guarantee.** Alt+3 flips a transient, unpersisted File info panel for the current track:

- `shortcuts::file_info_shortcut` maps the top-row `3` (by physical key, `Location::Standard`) with
  Alt held and Control, Shift, and Logo released to `Message::ToggleFileInfo`; Ctrl+Alt+3,
  Shift+Alt+3, Logo+Alt+3, keypad 3 with Alt, and a plain `3` map to nothing.
- Pressing the LCD strip's `I` publishes the same `Message::ToggleFileInfo` as Alt+3 and goes
  through the same `handlers::window::toggle_file_info` gate, so it does not open without a
  known current track.
- `handlers::window::toggle_file_info` flips `WinampPlayer::file_info_visible`. Opening needs a
  current track found in `known_tracks` (`WinampPlayer::current_known_track`); without one the flag
  stays false. Opening clears `about_visible` and `options_visible`, and opening About or Options
  clears `file_info_visible`.
  Opening while `shaded` holds first calls `toggle_window_shade` and returns that task.
- A plain or Shift-held Escape press also closes it (see `## Transient panel dismissal`).
- While the flag holds outside shade mode and the track resolves, `views::view_file_info_overlay`
  stacks `FILE INFO`, `Title: …`, `Artist: …` (left out for an empty artist), `Album: …` (left
  out for an empty album), `Year: …` (left out when `KnownTrack::year` is `None`), `Genre: …`
  (left out when `KnownTrack::genre` is empty), `Track #: …` (left out when
  `KnownTrack::track_number` is `None`), `Length: …`,
  and a `Close` button over the base through `views::modal::modal_overlay`, the chrome About
  shares. `KnownTrack::year` is copied from `Song::year`, which `Resource::into_song` parses from
  the resource's `releaseDate` (only its leading four ASCII-digit year is kept).
  `KnownTrack::genre` is copied from `Song::genre`, which `Resource::into_song` takes from the
  resource's `attributes.genreNames` through `primary_genre` — the first non-blank entry, trimmed,
  or `""` when the array is absent, empty, or all blank.
  `KnownTrack::track_number` is copied from `Song::track_number`, which `Resource::into_song`
  takes from the resource's `attributes.trackNumber` with an absent, `null`, non-positive, or
  above-`u32::MAX` number mapped to `None`.
  Between `Length:` and `Close`, it also draws one `Format: …` line (`256 KBPS  44 KHZ STEREO`,
  or `44 KHZ MONO` when the backend reports no bitrate) only while
  `WinampPlayer::playback_format` is `Some`, formatted from
  `views::now_playing::format_playback_prefix` and `views::now_playing::channel_mode_label`;
  with `None` the panel draws no format line. `Close` and the backdrop each publish one
  `Message::ToggleFileInfo`; a press on the panel's
  chrome publishes `Message::Ignored`. The shade branch never draws the panel.

**Enforced by.** `shortcuts::file_info_shortcut`,
`views::now_playing::indicator::lcd_indicator_message`, `handlers::window::toggle_file_info`,
`views::file_info::view_file_info_overlay`, `views::modal::modal_overlay`,
`views::now_playing::format_playback_prefix`, `views::now_playing::channel_mode_label`, and the
overlay branch of `ui::view::view`.

**Pinned by.** `message_for_maps_alt_3_to_file_info` (`src/ui/shortcuts.rs`);
`lcd_indicator_message_arms_o_a_i_and_v_and_leaves_d_inert` and
`lcd_indicator_strip_publishes_options_on_o_always_on_top_on_a_visualizer_on_v_and_file_info_on_i`
(`src/ui/views/tests/now_playing.rs`);
`toggle_file_info_shows_and_hides_the_panel_without_touching_playback`,
`toggle_file_info_needs_a_known_current_track`, `file_info_and_about_close_each_other`, and
`toggle_file_info_while_shaded_restores_the_window_first` (`src/ui/tests/window_tests.rs`);
`file_info_overlay_draws_the_title_artist_album_and_length`,
`file_info_overlay_omits_the_album_line_when_the_source_supplied_none`,
`file_info_overlay_draws_the_tracks_year`, `file_info_overlay_omits_the_year_line_without_one`,
`file_info_overlay_draws_the_tracks_genre`,
`file_info_overlay_omits_the_genre_line_when_the_source_supplied_none`,
`file_info_overlay_draws_the_tracks_track_number`,
`file_info_overlay_omits_the_track_number_line_when_the_source_supplied_none`,
`file_info_overlay_draws_the_loaded_previews_format`,
`file_info_overlay_omits_the_format_line_without_a_loaded_preview`, and
`file_info_overlay_closes_from_close_and_backdrop_but_ignores_its_chrome`
(`src/ui/views/tests/chrome.rs`); `known_tracks_record_the_played_songs_year` and
`known_tracks_record_the_played_songs_genre`, and
`known_tracks_record_the_played_songs_track_number` (`src/ui/tests/readouts_tests.rs`);
`release_date_maps_to_the_song_year` and
`release_date_absent_or_malformed_maps_to_no_year`, `songs_from_album_maps_the_genre`,
`songs_from_album_maps_a_missing_or_blank_genre_to_empty`,
`songs_from_album_maps_the_track_number`,
`songs_from_album_maps_a_missing_or_zero_track_number_to_none`,
`songs_from_album_maps_a_negative_track_number_to_none`, and
`songs_from_album_maps_an_oversized_track_number_to_none`
(`src/apple_music/rest/tests/library.rs`); `song_round_trips_its_year`,
`song_year_defaults_to_none_when_the_payload_omits_it`, `song_round_trips_its_genre`, and
`song_genre_defaults_to_empty_when_the_payload_omits_it`, `song_round_trips_its_track_number`,
and `song_track_number_defaults_to_none_when_the_payload_omits_it` (`src/library.rs`);
`songs_from_album_maps_the_album_name` and
`songs_from_album_maps_a_missing_or_blank_album_name_to_empty`
(`src/apple_music/rest/tests/library.rs`); `songs_carry_the_documented_genres` and
`songs_carry_the_documented_track_numbers`
(`src/sample_library.rs`); `file_info_panel_draws_the_loaded_previews_format` and
`view_constructs_when_the_window_is_shaded` (`src/ui/tests/view_tests.rs`);
`the_escape_listener_runs_only_while_a_transient_panel_is_open`,
`escape_key_message_maps_escape_but_not_its_chords_or_other_keys`
(`src/ui/tests/subscriptions_tests.rs`), and
`escape_closes_each_transient_panel_without_touching_anything_else`
(`src/ui/tests/panels_tests.rs`) pin the Escape route.

## Jump to time panel

**Guarantee.** Ctrl+J flips a transient, unpersisted Jump to time panel that seeks the loaded
preview:

- `shortcuts::shortcut` maps `j` or `J` with Control held and Shift, Alt, and Logo released to
  `Message::ToggleJumpTime`; Ctrl+Shift+J, Ctrl+Alt+J, and Ctrl+Logo+J map to nothing (plain
  `j`/`J` opens Jump to File instead, see `## Jump to File panel`).
- `handlers::jump::toggle_jump_time` flips `WinampPlayer::jump_time_visible`. Opening needs
  `playback_duration` to be `Some`; without it the flag stays false. Opening clears the query,
  `about_visible`, `options_visible`, `file_info_visible`, and `jump_file_visible`, and opening
  About, Options, File info, or Jump to File clears `jump_time_visible`. Opening while `shaded`
  holds first calls `toggle_window_shade`.
- A plain or Shift-held Escape press also closes it (see `## Transient panel dismissal`).
- `handlers::jump::parse_jump_time` accepts ASCII-digit seconds or `M:SS`; `jump_time_submitted`
  seeks through `playback::preview::seek_preview_to` (clamped to the length), bumping
  `position_epoch`, and closes the panel. An unparsable query changes nothing and keeps it open.
- `views::view_jump_time_overlay` stacks `JUMP TO TIME`, `Track length: …`, the time box, and
  `Jump` and `Close` buttons through `modal_overlay`. `Close` and the backdrop each publish one
  `ToggleJumpTime`; `Jump` publishes `JumpTimeSubmitted`. The shade branch never draws it.

**Enforced by.** `handlers::jump`, `handlers::playback::preview::seek_preview_to`,
`views::jump_time::view_jump_time_overlay`, and the overlay branch of `ui::view::view`.

**Pinned by.** `parses_seconds_and_minutes_seconds` and `rejects_malformed_times`
(`src/ui/handlers/jump.rs`); `message_for_maps_ctrl_j_to_jump_time` (`src/ui/shortcuts.rs`); the
tests in `src/ui/tests/jump_time_tests.rs`; `jump_time_overlay_draws_its_heading_length_and_buttons`
and `jump_time_overlay_buttons_and_backdrop_publish_their_messages`
(`src/ui/views/tests/chrome.rs`); `view_constructs_when_the_window_is_shaded`
(`src/ui/tests/view_tests.rs`); `the_escape_listener_runs_only_while_a_transient_panel_is_open`,
`escape_key_message_maps_escape_but_not_its_chords_or_other_keys`
(`src/ui/tests/subscriptions_tests.rs`), and
`escape_closes_each_transient_panel_without_touching_anything_else`
(`src/ui/tests/panels_tests.rs`) pin the Escape route.

## Jump to File panel

**Guarantee.** Plain J flips a transient, unpersisted Jump to File panel over the showing song list:

- `shortcuts::shortcut` maps plain `j` or `J`, with Shift allowed and Control, Alt, and Logo
  released, to `Message::ToggleJumpFile`; Ctrl+J still maps to `ToggleJumpTime`, and Ctrl+Shift+J
  stays unbound. While the panel is open, `ui::subscriptions` installs
  `shortcuts::message_for_jump_file`, which maps Up, Down, Page Up, and Page Down (Shift allowed,
  Control, Alt, and Logo released) to `JumpFileSelectionUp`/`JumpFileSelectionDown` and
  `JumpFileSelectionPageUp`/`JumpFileSelectionPageDown`, and falls through to `message_for` for
  every other event; with the panel closed those keys keep their volume and browse-scroll
  mappings, and Home and End stay with the focused filter box. The panel's filter box holds
  focus, so a focused `TextInput` captures printable keys (including `j`) and
  `keyboard::listen`'s ignored-only feed never turns them into shortcuts; typing therefore
  filters, and `Close` or the backdrop closes the panel.
- `handlers::jump::song_list` names the cached titles and epoch of the showing list: the browsed
  Songs buffer or the flat All Songs buffer, and no list on every other view. `toggle_jump_file`
  opens only when that list exists; opening clears the query, closes About, Options, File info, and
  Jump to time, and while `shaded` first calls `toggle_window_shade`. Opening any of those panels
  closes Jump to File, and the flag is never persisted. Navigation that changes the showing view
  (Reload, Back, search, and the All Songs/All Albums/Playlists buttons) clears it, so a panel left
  open cannot reappear over a later song list. A plain or Shift-held Escape press also closes it (see
  `## Transient panel dismissal`).
- `views::jump_file::jump_file_matches` splits the query on whitespace; an index matches when its
  title contains every word as a case-insensitive substring, in any order. A blank query has no
  words and matches every index. Matches come in list order. The view and the submit handler
  call it, so the rows drawn and the row Enter plays agree.
- `views::jump_file::jump_file_row_label` prefixes a row with `index + 1`, the source position the
  same 0-based index has in the showing list, followed by `". "` and the cached title. Filtering
  therefore keeps each match's original number instead of renumbering the shown rows.
- `views::jump_file::jump_file_highlighted` resolves the highlighted match: `jump_file_selected`
  when it is still one of the filtered matches, otherwise the first match, and `None` for an
  empty match list. Opening the panel or typing a new query resets the selection to `None`, so
  the first match is highlighted until Up, Down, Page Up, or Page Down moves it; Up and Down move
  one match and Page Up and Page Down move `views::JUMP_FILE_PAGE_ROWS` matches (five, the rows
  that fit the fixed `JUMP_FILE_LIST_HEIGHT`), all clamped at the first and last match without
  wrapping and storing the match's source index.
- Every highlight move returns `reveal_jump_file_row`, which scrolls the match list only as far
  as needed to show the landed row and not at all when the row is already fully visible. The row
  geometry comes from `views::JUMP_FILE_LIST_PADDING`, `JUMP_FILE_ROW_HEIGHT`, and
  `JUMP_FILE_ROW_SPACING`, and the minimal-reveal arithmetic is the browse list's shared
  `handlers::browse::scroll::reveal_offset`, so the list is not scrolled past what the move needs.
  A closed panel or an empty match list schedules no scroll.
- `views::jump_file::jump_file_row_style` draws the highlighted row — the row Enter or `Jump`
  plays — with the browse selection style, the blue `theme::ROW_HIGHLIGHT` bar behind white
  `theme::PLAYING_ROW_TEXT`; every other row keeps the flat green `style::playlist_row_style`
  colors and its hover/press face ladder.
- `jump_file_submitted` plays the highlighted match through `Message::TrackSelected { epoch,
  index }` and closes the panel; no match changes nothing. `jump_file_row_selected` dispatches the
  clicked source index the same way. The existing `TrackSelected` epoch/index guards reject a
  stale press.
- `views::view_jump_file_overlay` stacks `JUMP TO FILE`, the `Filter songs` box, one numbered row per
  match, and `Jump` and `Close` buttons through `modal_overlay`. An empty list reads `No songs`; a
  list with no match reads `No matching songs`, and neither draws a row. `Close` and the backdrop
  publish `ToggleJumpFile`, `Jump` publishes `JumpFileSubmitted`, and each row publishes
  `JumpFileRowSelected`. The shade branch never draws it. The match rows are built through
  `views::browse::rows::lazy::LazyRow`, keyed by `MatchListKey` (list epoch, hash of the typed
  query, title count, and the highlighted selection), so a view rebuild caused by a playback poll
  or marquee tick reuses the built rows instead of re-filtering every title and formatting every
  row; a changed filter, list epoch, or highlight rebuilds the list.

**Enforced by.** `handlers::jump`, `handlers::browse`,
`handlers::browse::scroll::reveal_offset`, `ui::subscriptions`,
`shortcuts::message_for_jump_file`, `views::jump_file::view_jump_file_overlay`, and the
jump-file branch of `ui::view::view`.

**Pinned by.** `matches_blank_all_case_insensitive_trimmed_in_order`,
`matches_every_whitespace_separated_word_in_any_order`,
`jump_file_row_label_numbers_the_showing_list_position`,
`jump_file_highlighted_prefers_a_selected_match_and_falls_back_to_first`,
`jump_file_row_style_highlights_only_the_highlighted_row`,
`jump_file_page_step_fits_the_visible_list`,
`overlay_draws_the_heading_matching_rows_and_buttons`,
`overlay_highlights_the_selected_row`,
`row_and_buttons_publish_their_messages`, and
`empty_and_no_match_lists_have_their_own_wording` (`src/ui/views/tests/jump_file.rs`);
`the_match_list_reuses_its_rows_until_the_filter_or_highlight_changes`
(`src/ui/views/tests/jump_file.rs`) pins the match-row cache;
`message_for_maps_j_to_jump_file`, `message_for_maps_ctrl_j_to_jump_time`, and
`message_for_jump_file_maps_up_and_down_and_falls_back` (`src/ui/shortcuts.rs`);
`submit_matches_the_artist_and_title_words_together`,
`page_up_and_down_move_the_highlight_by_one_page_and_clamp`,
`a_highlight_move_reveals_only_the_landed_row_in_the_match_list`,
`jump_file_row_top_starts_at_the_padding_and_steps_by_one_row_and_gap`, and the other tests in
`src/ui/tests/jump_file_tests.rs`; `the_escape_listener_runs_only_while_a_transient_panel_is_open`,
`escape_key_message_maps_escape_but_not_its_chords_or_other_keys`
(`src/ui/tests/subscriptions_tests.rs`), and
`escape_closes_each_transient_panel_without_touching_anything_else`
(`src/ui/tests/panels_tests.rs`) pin the Escape route.

## Transient panel dismissal

**Guarantee.** A plain or Shift-held Escape press closes whichever transient panel is showing —
About, Options, File info, Jump to time, or Jump to File — including while its filter or time box
has focus. It changes only the five visibility flags; no query, selection, playback, transport,
mixer, browse, or settings state changes, no work is scheduled, and nothing is persisted. Escape
with Control, Alt, or Logo held, and Escape releases, map to nothing. With no panel open the
listener is not subscribed, so Escape stays unbound there and a focused `Search library` box keeps
it for clearing its own focus; `Message::EscapePressed` is still a no-op.

- `WinampPlayer::transient_panel_open` is the read side of the five flags
  `WinampPlayer::close_transient_panels` clears, so the subscription gate and the closer share one
  set.
- `ui::subscriptions::escape_key_message` maps the press through `iced::event::listen_with`, ignoring the
  event status so a focused `TextInput` that captures Escape cannot swallow it;
  `ui::subscriptions::subscriptions` adds that listener only while `transient_panel_open` holds.
- `Message::EscapePressed` routes to `handlers::window::escape_pressed`, which calls
  `close_transient_panels` and schedules nothing. It is not a toggle, so it never opens a panel.
- `shortcuts::message_for` still returns `None` for a plain Escape press, leaving every existing
  keyboard and media-key binding unchanged.

**Enforced by.** `WinampPlayer::transient_panel_open`, `WinampPlayer::close_transient_panels`,
`ui::subscriptions::escape_key_message`, `ui::subscriptions::subscriptions`, `Message::EscapePressed`, and
`handlers::window::escape_pressed`.

**Pinned by.** `the_escape_listener_runs_only_while_a_transient_panel_is_open` and
`escape_key_message_maps_escape_but_not_its_chords_or_other_keys`
(`src/ui/tests/subscriptions_tests.rs`);
`escape_closes_each_transient_panel_without_touching_anything_else`
(`src/ui/tests/panels_tests.rs`); `message_for`'s plain-Escape case
(`src/ui/shortcuts.rs`).

## Song-list footer

**Guarantee.** A non-empty song list draws a footer beneath its rows. With no selection it reads
`<count> song(s) - <total>`. While the list's own `selected` index names a song, it reads
`<count> song(s) - <selected length>/<total>`, for example `2 songs - 3:30/4:15`; a zero-length
selected song reads `0:00`. An index past the list falls back to the plain form.

**Enforced by.** `views::browse::rows::text::song_list_summary` formats both forms;
`views::browse::rows::song_list` passes its `selected` argument to it.

**Pinned by.** `song_list_summary_counts_songs_and_sums_durations` and the other existing
`song_list_summary_*` tests (plain form, `None`),
`song_list_summary_puts_the_selected_length_before_the_total`,
`song_list_summary_reads_a_zero_length_selection_as_zero`, and
`song_list_summary_ignores_a_selection_past_the_list` (`src/ui/views/tests/browse.rs`).

**History.** Added with the selected-length footer, like Winamp's playlist status line.

## Library footer

**Guarantee.** While the library panel is shown in normal mode, it draws exactly one compact
transport row beneath the browse list; the transport row's `PL` toggle gates the whole library
column, so hiding it drops this footer with the rest of the section. The footer itself:

- The row is exactly `MINI_TRANSPORT_HEIGHT` tall and sits on the window face's raised chrome
  below the browse list.
- It shows Previous, Play, Pause, Stop, Next, the elapsed-over-total time readout, and a volume
  slider. The five buttons emit `Message::PreviousButton`, `Message::Play`, `Message::Pause`,
  `Message::Stop`, and `Message::NextButton`, and the row is playback-independent, drawing both
  `▶` and `II` in every mode. The time label joins
  the same `WinampPlayer::now_playing_elapsed` and `WinampPlayer::now_playing_time` strings the
  LCD shows as `<elapsed>/<total>`, so the elapsed/remaining choice and the total cannot disagree
  between the two; when the total is the `--:--` placeholder the readout is just the elapsed
  string, never `<elapsed>/--:--`.
- The volume slider is tagged `MINI_VOLUME_ID`, pinned to `MINI_VOLUME_WIDTH` at the row's right
  edge, and writes `Message::VolumeChange`. No new message, handler, or playback path exists.
- The footer and the shade strip build from the same compact row, so a glyph, order, width,
  message, or volume-slider change shows up in both; the shade strip adds only its seek slider,
  which the footer never builds. Normal mode draws no shade strip, and shade mode draws no
  library footer.
- While playing or paused, the footer's time readout is a click target that publishes
  `Message::ToggleTimeDisplay`, flipping the same `WinampPlayer::time_display` mode the LCD uses;
  while stopped a single press publishes nothing and a double-click publishes only the no-op
  `Message::Ignored`. The shared row applies the LCD's own
  `views::now_playing::elapsed_readout_clickable` guard rather than a second rule.

**Enforced by.** `views::mini_transport::view_library_strip` wraps
`views::mini_transport::mini_transport_strip` for normal mode;
`views::mini_transport::mini_transport_controls` and
`views::mini_transport::mini_volume_spec` are the shared control and slider specs;
`views::mini_transport::mini_transport_readout` joins the elapsed and total strings and keeps a
single time string when the total is unknown;
`views::mini_transport::mini_transport_row_with_optional_seek` wraps the readout in a
`MouseArea::on_press(Message::ToggleTimeDisplay)` only when
`views::now_playing::elapsed_readout_clickable` holds for the strip's mode, and wraps a stopped
readout in `MouseArea::on_double_click(Message::Ignored)` so it captures a press with a no-op;
`views::mini_transport::MINI_TRANSPORT_HEIGHT`, `MINI_VOLUME_WIDTH`, and
`MINI_VOLUME_ID` own the geometry and test seam. The normal branch of `ui::view::view` builds the
library column, including the footer, and pushes it onto the screen only while
`WinampPlayer::library_panel_visible` holds — hiding the panel skips building the whole column,
so an unseen browse list of any length costs nothing per frame — with clones of the LCD's elapsed
and total strings; its shaded branch returns before that push.

**Pinned by.** `view_library_strip_keeps_its_fixed_height`,
`library_strip_draws_the_five_controls_time_and_right_edge_volume`,
`mini_transport_readout_appends_the_known_total`,
`compact_readout_toggles_time_display_only_while_not_stopped`, and
`main_transport_and_shade_strip_share_glyphs_order_and_widths` (`src/ui/views/tests/shade.rs`);
`normal_root_view_draws_the_library_strip_beneath_the_browse_list`,
`shade_strip_draws_only_while_shaded`, and `root_view_footer_shows_elapsed_over_total`
(`src/ui/tests/view_tests.rs`); `toggle_library_panel_hides_and_restores_the_library_section`
(`src/ui/tests/panels_tests.rs`).

**History.** Added 2026-10-23 with the mockups' library-panel transport row; the shade strip was
refactored onto the same shared row in the same change.

## Sign-in server

**Guarantee.** The loopback sign-in server never waits for one connection to finish before it reads
the next. A local client that opens a connection and never finishes its request does not hold the
browser's `POST /token` callback behind it:

- The accepted connections are read together — each one is polled without blocking — so one
  connection's cost is not serialized behind the previous one's.
- A connection is dropped when its own budget (`CONNECTION_READ_TIMEOUT`, five seconds) is spent
  from the moment it was accepted; the flow's deadline (`AUTH_TIMEOUT`) is the outer bound.
- The pending connections are capped (`MAX_PENDING_CONNECTIONS`), so a flood of connections cannot
  grow the server's share of the process's file descriptors without bound. When the cap is reached
  the accept loop takes on no more connections until a pending one is served or its budget is spent,
  leaving the rest in the kernel's accept queue; a connection the server has already accepted is
  never displaced by a newer one, so a flood cannot evict the browser's callback after it has been
  accepted.
- One pass of the accept loop takes on at most `MAX_ACCEPTS_PER_PASS` connections, and a connection
  taken on in a pass is served in the next one, so a callback that is queued behind N connections
  the pending list has room for is served within `ceil((N + 1) / MAX_ACCEPTS_PER_PASS) + 1` poll
  intervals (`ACCEPT_POLL_INTERVAL`, 25 ms) — one poll interval for every eight connections queued
  ahead of it. While the list is full the callback waits in the kernel's accept queue until a
  pending connection is served or its budget is spent; the flow's deadline remains the outer bound.
- A bootstrap directory left in the temp dir by a process that died before its guard could remove it
  is swept by the next sign-in once it is older than `BOOTSTRAP_STALE_AFTER` (twice `AUTH_TIMEOUT`).
  A fresher bootstrap directory (which may belong to another running instance), an entry with a
  different name, and a non-directory entry are left alone; a symlink is not followed. A removal
  failure is reported with the directory name escaped, so an attacker-chosen control character in
  a shared temp-directory name cannot reach the terminal raw.
- A transient `accept` failure — `Interrupted` (a signal arrived before a connection could be
  accepted), `ConnectionAborted`, or `ConnectionReset` (a queued peer reset its connection; the
  platform chooses which kind to report) — does not end the flow: the accept loop retries it within
  the pass, up to `MAX_ACCEPTS_PER_PASS`, instead of returning it. Only a failure of the listener
  itself is returned.

**Enforced by.** `authorize_with_bounds` in `src/music_kit_auth.rs` keeps the accepted connections
in a `Vec` of `PendingConnection`, which is defined in `src/music_kit_auth/server.rs`. The loop's
helpers live in `src/music_kit_auth/server.rs`: every pass calls `serve_pending`, which polls each
pending connection through `http::RequestReader::poll` and serves the ones whose request has
arrived, and then `accept_pending`, which takes on the connections that are waiting.
`accept_pending` delegates the loop to `accept_pending_with` so a test can inject a transient
failure; its `accept_error_is_transient` check retries `Interrupted`, `ConnectionAborted`, and
`ConnectionReset` and returns every other error. `http::RequestReader` owns a non-blocking socket
and parses the request from the bytes received so far, so polling a silent peer reports "nothing
yet" instead of waiting for it; `serve_pending` drops a connection whose budget is spent.
`BootstrapPage::write` in `src/music_kit_auth/bootstrap.rs` calls `sweep_stale_bootstrap_dirs`
before creating the new owner-only directory; the sweep reads one
level of the temp dir, removes only aged directories whose names carry `BOOTSTRAP_DIR_PREFIX`, and
reports a removal failure without failing the new sign-in.

**Pinned by.**

- `authorize_serves_the_callback_after_repeated_stalled_connections`
- `authorize_serves_the_callback_after_a_stalled_connection_is_dropped`
- `a_stalled_connection_is_dropped_when_its_budget_is_spent`
- `the_pending_list_is_capped_without_displacing_an_accepted_connection`
- `one_pass_takes_on_a_bounded_number_of_connections`
- `an_expired_connection_is_dropped_even_when_its_request_has_arrived`
- `a_transient_accept_failure_is_retried_instead_of_ending_the_flow`
- `a_listener_failure_is_returned_instead_of_being_retried`
  (all in `src/music_kit_auth/tests/server.rs`)
- `the_reader_reports_a_partial_request_as_incomplete`
- `the_reader_preserves_a_partial_body_between_polls`
- `the_reader_finds_a_header_terminator_split_across_polls`
  (in `src/music_kit_auth/tests/http_request.rs`)
- `the_bootstrap_sweep_removes_only_old_bootstrap_directories`
- `the_bootstrap_sweep_skips_a_symlink_instead_of_following_it`
- `the_bootstrap_sweep_escapes_a_hostile_directory_name_in_its_report`
- `a_new_bootstrap_write_sweeps_a_stale_directory_from_a_crashed_flow`
  (all in `src/music_kit_auth/tests/bootstrap.rs`)

**History.** Reconstructed from "A stalled connection is served for its whole budget before the next
one, so repeated stalls starve the sign-in callback" in `BUGS-FIXED.md`. On 2026-10-24 the cap
stopped evicting the oldest accepted connection: eviction could drop the browser's callback between
its header and body reads, so a full list now refuses new accepts until a slot frees.

## REST transport

**Guarantee.** The REST transport buffers at most `MAX_RESPONSE_BYTES + 1` bytes from either the
wire reader or the decoded stream, and accepts a body only when its decoded length is at most
`MAX_RESPONSE_BYTES` (8 MiB). `ureq`'s reader limit is set one byte above the cap so a body of
exactly the cap reaches EOF instead of tripping the limit on the read past it. Reading the reader
through `std::io::Read::take(MAX_RESPONSE_BYTES + 1)` bounds the decoded buffer and stops a
`Content-Encoding: gzip` response smaller than the cap from expanding the buffered body past it; a
decoded length above the cap is rejected with a message naming the cap before the body becomes a
`String`. A read that hits the wire limit keeps the `reading the response body failed:` prefix.
`text/*` responses keep `ureq`'s lossy UTF-8 replacement, as `Body::read_to_string()` had.

**Enforced by.** `UreqTransport::request` in `src/apple_music/rest/transport.rs` calls
`response.body_mut().with_config().limit(MAX_RESPONSE_BYTES + 1).lossy_utf8(true).reader()`, then
reads it through `take(MAX_RESPONSE_BYTES + 1)` and checks the decoded length before
`String::from_utf8`.

**Pinned by.**

- `a_response_of_exactly_the_cap_is_accepted` (`src/apple_music/rest/tests/transport.rs`)
- `a_compressed_response_larger_than_the_cap_is_rejected` (`src/apple_music/rest/tests/transport.rs`)
- `a_response_with_more_wire_bytes_than_the_cap_is_rejected` (`src/apple_music/rest/tests/transport.rs`)
- `a_text_response_with_invalid_utf8_keeps_the_lossy_replacement` (`src/apple_music/rest/tests/transport.rs`)

**History.** Added 2026-10-23 after `Body::read_to_string()` was found to limit only wire bytes
(its 10 MiB `MAX_BODY_SIZE` sits under `ureq`'s gzip decoder), leaving a compressed REST reply
unbounded after decoding — the same exposure the preview downloader had closed.

## Preview download

**Guarantee.** The preview download buffers at most `MAX_PREVIEW_BYTES + 1` bytes from either the
wire reader or the decoded stream, and accepts a body only when its decoded length is at most
`MAX_PREVIEW_BYTES` (10 MiB). `ureq`'s reader limit is set one byte above the cap so a body of
exactly the cap reaches EOF instead of tripping the limit on its last byte, while a larger wire
body still fails the read. Reading that reader through `std::io::Read::take(MAX_PREVIEW_BYTES +
1)` stops a `Content-Encoding: gzip` response smaller than the cap from expanding the buffered
preview past it; the decoded take rejects before the decoder when its extra byte is present. The
decoded over-cap failure names the cap; other read failures, including one past the wire limit,
keep the `reading the preview failed:` prefix.

**Enforced by.** `download_preview` in `src/audio/preview.rs` calls
`response.body_mut().with_config().limit(MAX_PREVIEW_BYTES + 1).reader()` for the wire limit, then
reads it through `take(MAX_PREVIEW_BYTES + 1)` and checks the decoded length before
`rodio::Decoder::new_mp4` in `decode_preview`; `download_and_decode` composes the two. It does not
use `Body::read_to_vec`, whose limiter sits under `ureq`'s gzip decoder and so counts compressed
bytes.

**Pinned by.**

- `a_compressed_preview_larger_than_the_cap_is_rejected` (`src/audio/tests/preview_download.rs`)
- `a_preview_with_more_wire_bytes_than_the_cap_is_rejected` (`src/audio/tests/preview_download.rs`)
- `a_preview_of_exactly_the_cap_reaches_the_decoder` (`src/audio/tests/preview_download.rs`)
- `a_downloaded_preview_decodes_and_reports_its_encoded_length` (`src/audio/tests/preview_download.rs`)

**History.** Added 2026-10-23 after the wire-byte cap was found to let a small gzip body expand
past 10 MiB before decoding; the decoded-byte fix keeps the original wire cap alongside it. The
wire limit was later raised one byte above the cap so an exactly-at-cap preview is read instead
of failing the limit on its last byte, the same boundary the REST transport raised.

## Preview replay cache

**Guarantee.** The audio worker keeps the last successfully decoded preview body, keyed by its URL,
and decodes that body again instead of downloading when the same URL is replayed — after a Stop
whose remembered preview URL Play restarts, or when Repeat replays a one-song list. The cache holds
one body, bounded by `MAX_PREVIEW_BYTES`, and a different URL replaces it, so it does not grow with
the tracks played. A download or decode that fails is not cached, so the next play tries again.

**Enforced by.** `audio::preview::PreviewCache::load_or_decode` in `src/audio/preview.rs`, used by
`download_and_decode_cached` from the `run_worker` fetch closure in
`src/audio/player/worker.rs`. `DecodedPreview` carries the encoded body it decodes, so a cache miss
stores the body the download produced.

**Pinned by.**

- `preview_cache_replays_the_same_url_without_loading_it_again` (`src/audio/tests/preview_download.rs`)
- `preview_cache_keeps_only_the_last_url_and_never_caches_a_failed_load` (`src/audio/tests/preview_download.rs`)
- `a_cached_preview_replays_the_same_decodable_body_without_the_server` (`src/audio/tests/preview_download.rs`)

## Preview host guard

**Guarantee.** A preview URL handed to the audio backend may reach only an address
`ip_is_internal` does not classify as internal. The URL must be absolute `https` and carry no
control character, its host must not be the local host name or a legacy numeric address form
(`2130706433`, `0x7f000001`, `0177.0.0.1`, `127.1`), and every address a hostname resolves to —
or an address literal the URL spells — must pass `ip_is_internal`. An IPv6 literal carrying a zone
identifier (`%lo`, or RFC 6874's `%25lo`) is refused as well, because `IpAddr::parse` cannot read
the suffix and the literal would otherwise pass as an ordinary name. For IPv4 it refuses private,
loopback, link-local, unspecified, broadcast, documentation, multicast, `0.0.0.0/8`, shared
(CGNAT), benchmarking (`198.18.0.0/15`), reserved (`240.0.0.0/4`), IETF protocol-assignment
(`192.0.0.0/24`, the whole block, including the globally reachable `.9` and `.10` anycast
addresses), and 6to4 relay (`192.88.99.0/24`) ranges. For IPv6 it refuses loopback, unspecified,
multicast, unique-local, link-local, and the IPv4 address embedded in an IPv6 transition form
(6to4, NAT64 `/96` and `/48`, Teredo, IPv4-translated `::ffff:0:0:0/96`, and the ISATAP interface
identifier `0:5efe` or `200:5efe` on an address outside those fixed prefixes); for Teredo it checks both the plain server
IPv4 in bytes 4..8 and the complemented client IPv4 in bytes 12..16. It refuses deprecated site-local
(`fec0::/10`) addresses too, because they are still routed inside a site. It does not refuse IPv6 documentation
(`2001:db8::/32`) or benchmarking (`2001:2::/48`) addresses. Redirects are not followed, and a
proxy cannot take over resolution.

**Enforced by.** `preview_url_problem` in `src/apple_music/validation.rs`, with `is_numeric_host_form` and
`ip_is_internal` in `src/apple_music/address.rs` (`ip_is_internal` is re-exported by
`src/apple_music.rs`), applied at the
seam by `ensure_preview_url_is_valid`; `audio::preview::PublicAddressResolver` applies
`ip_is_internal` to every resolved address before connecting, and `preview_agent` follows no
redirects and disables the proxy.

**Pinned by.**

- `ip_is_internal_refuses_every_internal_range_and_allows_public_addresses`
- `preview_url_problem_unwraps_a_disguised_internal_host`
- `preview_url_problem_rejects_a_url_without_a_usable_https_host`
- `preview_url_problem_rejects_a_scoped_ipv6_literal`
- `is_numeric_host_form_recognizes_legacy_spellings_and_leaves_names_alone`
  (all in `src/apple_music/tests/validation.rs`)
- `the_preview_agent_refuses_a_hostname_that_resolves_to_loopback`
- `the_preview_resolver_allows_a_public_address_literal`
- `the_preview_resolver_checks_ipv6_transition_literals`
  (in `src/audio/tests/preview_download.rs`)

**History.** Reconstructed from "Preview URL guard accepts IPv4 multicast and reserved ranges" in
`BUGS-FIXED.md` (2026-10-23); the v6 branch already refused multicast while the v4 branch did not.

## Preview spectrum

**Guarantee.** The backend reports ten normalized amplitude bands in
`crate::equalizer::BAND_HZ` order only while a preview is loaded, computed from the samples the
worker actually pulls through `audio::meter::MeterSource`; those samples reach the equalizer,
balance, and output unchanged. A band at or above Nyquist reads zero, as do silence and an empty
window. A seek clears the recorded window without resetting the partial downmix frame, so the
sample rodio leaves on the same channel stays in phase; a seek the backend rejects leaves the
recorded window untouched, so a failed seek does not blank the audio still playing. A player that
has not appended a preview
reports nothing, so Stop and a new selection cannot report the previous preview's spectrum. The
silent fallback and a closed worker report `None`. The UI half stores that reply in
`WinampPlayer::spectrum` and draws one fixed-width slot per band (`BAND_COUNT`) from it while
`WinampPlayer::visualizer_visible` holds; each bar's color follows its height above the rail in
absolute 2 px ramp steps from dark green through yellow to orange, while the bar heights, widths,
gaps, row height, and visible/hidden behavior stay unchanged. The indicator strip's `V` hides and
shows them without clearing the stored spectrum or stopping the poll. Their heights clamp to the
fixed visualizer row, Pause freezes them, Stop and a new selection clear them, stale or
pre-transition replies are dropped, and the poll runs only while playing.

**Enforced by.** `audio::meter::spectrum_bands` (the pure ten-band Goertzel analysis) and
`audio::meter::SpectrumMeter` (`push` downmixes and fills the fixed ring, `take` copies it and
analyzes outside the lock, `clear` empties the ring while keeping the partial downmix frame) in
`src/audio/meter.rs`; `MeterSource` records each sample while returning it unchanged and clears
the ring on `try_seek` without resetting that frame. `AudioOutput::spectrum`
in `src/audio.rs` defaults to `None`, and `RodioPlayer::spectrum` in
`src/audio/player/rodio_player.rs` answers `Some` only once the player holds a preview's format,
through the worker's `Command::Spectrum`
handled beside `Command::PlaybackFormat`; `RodioOutput::spectrum` bounds the query like the other
backend queries. On the UI side, `views::now_playing::visualizer::visualizer_bar_heights` and
`views::now_playing::visualizer::visualizer_bar_segments` scale the bands and split each bar into
ramp steps, and `views::now_playing::visualizer::now_playing_visualizer_row` draws them at fixed
width, while `views::visible_spectrum` applies `WinampPlayer::visualizer_visible`,
`Message::ToggleVisualizer` and `handlers::playback::toggle_visualizer` flip that flag,
`WinampPlayer::needs_spectrum_poll` gates the poll, `ui::subscriptions::spectrum_ticks` feeds it, and
`handlers::playback::preview::spectrum_tick`/`spectrum` issue and guard the query while
`reset_position_readout` clears the stored value.

**Pinned by.**

- `spectrum_bands_peak_at_the_matching_band_for_a_sine` (`src/audio/tests/sources.rs`)
- `spectrum_bands_zero_a_band_at_or_above_nyquist` (`src/audio/tests/sources.rs`)
- `meter_source_passes_samples_through_and_records_their_spectrum` (`src/audio/tests/sources.rs`)
- `meter_source_failed_seek_keeps_the_recorded_window` (`src/audio/tests/sources.rs`)
- `meter_source_seek_keeps_the_downmix_phase` (`src/audio/tests/sources.rs`)
- `meter_window_keeps_the_newest_samples_in_play_order` (`src/audio/tests/sources.rs`)
- `spectrum_query_returns_the_loaded_players_spectrum` (`src/audio/tests/worker.rs`)
- `spectrum_query_reports_none_when_nothing_is_loaded` (`src/audio/tests/worker.rs`)
- `loaded_player_reports_its_meter_but_an_unloaded_one_reports_nothing` (`src/audio/tests/worker.rs`)
- `append_records_the_appended_source_through_the_players_meter` (`src/audio/tests/worker.rs`)
- `stop_removes_the_player_so_a_spectrum_query_reports_none` (`src/audio/tests/worker.rs`)
- `silent_output_reports_no_spectrum` (`src/audio/tests/rodio_output.rs`)
- `rodio_output_asks_the_worker_for_its_spectrum` (`src/audio/tests/rodio_output.rs`)
- `rodio_output_reports_a_closed_worker_for_every_command` (`src/audio/tests/rodio_output.rs`)
- `playback_spectrum_forwards_the_audio_output_spectrum` (`src/apple_music/tests/audio_forwarding.rs`)
- `visualizer_bar_heights_scale_and_clamp_the_reported_bands`
- `visualizer_bar_heights_maps_a_nan_band_to_empty` (`src/ui/views/tests/lcd_display.rs`)
- `visualizer_bar_segments_ramp_from_dark_green_to_orange` (`src/ui/views/tests/lcd_display.rs`)
- `now_playing_content_keeps_the_visualizer_height_with_and_without_a_spectrum`
- `now_playing_visualizer_row_keeps_its_height_without_a_spectrum` (`src/ui/views/tests/lcd_display.rs`)
- `now_playing_visualizer_row_draws_one_fixed_width_bar_per_band` (`src/ui/views/tests/lcd_display.rs`)
- `now_playing_visualizer_row_paints_its_ramp_from_green_to_orange` (`src/ui/views/tests/lcd_display.rs`)
- `visible_spectrum_hides_the_stored_bars_while_the_visualizer_is_off` (`src/ui/views/tests/now_playing.rs`)
- `lcd_indicator_strip_publishes_options_on_o_always_on_top_on_a_visualizer_on_v_and_file_info_on_i`
  (`src/ui/views/tests/now_playing.rs`)
- `toggle_visualizer_hides_and_shows_the_bars_without_clearing_the_spectrum` (`src/ui/tests/panels_tests.rs`)
- `hidden_visualizer_draws_no_bars_in_the_root_view` (`src/ui/tests/panels_tests.rs`)
- `spectrum_tick_stream_emits_spectrum_ticks` (`src/ui/tests/`)
- `a_refused_spectrum_thread_ends_the_tick_stream_instead_of_panicking` (`src/ui/tests/`)
- `a_refused_spectrum_thread_reports_the_cause_and_the_loss` (`src/ui/tests/`)
- `the_spectrum_thread_is_named_nostalgia_spectrum_poll` (`src/ui/tests/`)
- `spectrum_reply_stores_the_loaded_previews_spectrum`
- `spectrum_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`
- `stop_clears_the_spectrum_while_pause_keeps_it`
- `spectrum_tick_queries_spectrum_and_carries_the_epoch`
- `the_spectrum_timer_runs_only_while_playback_runs` (`src/ui/tests/`)

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
stale epoch, a pending transition, and a not-finished reply never advance. Ctrl+V arms a one-shot
"Stop after current" flag (`WinampPlayer::stop_after_current`, not persisted): a finished reply that
passes those checks then takes the Stop path instead of advancing and disarms the flag, while a
dropped reply leaves it armed.

**Enforced by.** `audio::AudioOutput::is_finished` in `src/audio.rs` defaults to false, and
`audio::player::rodio_player::RodioPlayer::is_finished` in `src/audio/player/rodio_player.rs`
answers true only once the player holds a preview's format and `rodio::Player::empty` reports the
queued source has drained; `RodioOutput::is_finished` bounds the worker's `Command::Finished` reply
like the other backend queries, mapping a missing reply to false.
`AppleMusicService::preview_finished` in `src/apple_music/playback/output.rs` forwards the backend's
answer without touching shared state. `handlers::playback::preview::position_tick` schedules the
finished query, `Message::PlaybackFinished` carries its reply, and
`handlers::playback::preview::playback_finished` advances the transport;
`handlers::playback::stepping::forward_track_id` (shared with
`handlers::playback::stepping::step_track`) applies the
same list/Shuffle/Repeat rules as Next, and the no-next-song case reuses `handlers::playback::stop`.

**Pinned by.**

- `finished_query_returns_the_loaded_players_finished_state` (`src/audio/tests/worker.rs`)
- `finished_query_reports_none_when_nothing_is_loaded` (`src/audio/tests/worker.rs`)
- `rodio_player_reports_finished_only_after_its_appended_preview_drains` (`src/audio/tests/worker.rs`)
- `a_paused_preview_reports_unfinished` (`src/audio/tests/worker.rs`)
- `silent_output_reports_an_unfinished_preview` (`src/audio/tests/rodio_output.rs`)
- `rodio_output_asks_the_worker_for_its_finished_state` (`src/audio/tests/rodio_output.rs`)
- `rodio_output_reports_none_when_the_finished_reply_times_out` (`src/audio/tests/rodio_output.rs`)
- `rodio_output_reports_a_closed_worker_for_every_command` (`src/audio/tests/rodio_output.rs`)
- `stop_removes_the_player_so_a_finished_query_reports_none` (`src/audio/tests/worker.rs`)
- `preview_finished_forwards_the_audio_output_state` (`src/apple_music/tests/audio_forwarding.rs`)
- `position_tick_queries_preview_finished` (`src/ui/tests/readouts_tests.rs`)
- `preview_end_advances_to_the_next_song` (`src/ui/tests/`)
- `preview_end_in_all_songs_steps_the_flat_list` (`src/ui/tests/`)
- `preview_end_wraps_to_the_first_song_with_repeat_on` (`src/ui/tests/`)
- `preview_end_repeats_a_single_song_with_repeat_on` (`src/ui/tests/`)
- `preview_end_at_the_last_song_stops_when_repeat_is_off` (`src/ui/tests/`)
- `preview_end_with_shuffle_on_picks_the_other_song` (`src/ui/tests/`)
- `preview_end_with_shuffle_on_a_single_song_stops` (`src/ui/tests/`)
- `preview_end_while_paused_or_stopped_does_not_advance` (`src/ui/tests/`)
- `preview_end_with_a_stale_epoch_or_pending_transition_is_dropped` (`src/ui/tests/`)
- `preview_end_with_stop_after_current_stops_and_clears_it` (`src/ui/tests/stepping_tests.rs`)
- `preview_end_after_stop_after_current_fired_advances_again` (`src/ui/tests/stepping_tests.rs`)
- `stop_after_current_survives_a_dropped_finished_reply` (`src/ui/tests/stepping_tests.rs`)
- `toggle_stop_after_current_flips_only_the_flag` (`src/ui/tests/stepping_tests.rs`)
- `message_for_maps_ctrl_v_to_stop_after_current` (`src/ui/shortcuts.rs`)

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
in `src/ui/transport/history.rs` implements the recorded-play walk.

**Pinned by.**

- `track_played_records_a_play_paused_before_its_completion` (`src/ui/tests/stepping_tests.rs`)
- `track_played_ignores_a_failed_completion` (`src/ui/tests/stepping_tests.rs`)
- `track_played_ignores_a_superseded_completion` (`src/ui/tests/stepping_tests.rs`)
- `a_failed_history_jump_retries_the_target` (`src/ui/tests/stepping_tests.rs`)
- `previous_with_shuffle_resumes_from_the_last_recorded_play_after_a_new_track` (`src/ui/tests/stepping_tests.rs`)
- `track_selected_starts_playback_of_the_selected_track` (`src/ui/tests/`)
- `play_into_maps_a_hanging_play_to_a_completed_message` (`src/ui/tests/`)
- `play_into_reports_a_failed_play_and_still_completes` (`src/ui/tests/`)

## Previous restart

**Guarantee.** A Previous press on a loaded preview whose last reported position is at least three
seconds restarts that preview from zero: exactly one absolute seek to zero, any seek drag cleared,
the readout position reset to zero, and the position epoch bumped so in-flight replies are dropped.
The current track and transport mode stay unchanged. Earlier positions, a backend with no loaded
preview, or no reported position step through the showing list as before, including Repeat's edge
behavior and Shuffle's recorded-play retrace. A rejected restart seek is logged, naming the track
and the position that triggered the restart, and leaves the track queued with its position and
epoch unchanged; no step is scheduled. The decision uses the
last reported position — the one the readout shows — not a fresh backend query.

**Enforced by.** `handlers::playback::stepping::step_track` in
`src/ui/handlers/playback/stepping.rs` applies the restart branch before the stepping arithmetic,
consulting `AppleMusicService::has_loaded_preview` and `AppleMusicService::seek_output_to`;
`transport::previous_track_id` in `src/ui/transport.rs` and `transport::PlayHistory` in
`src/ui/transport/history.rs` implement the
unchanged step paths.

**Pinned by.**

- `previous_restarts_the_preview_after_three_seconds` (`src/ui/tests/stepping_tests.rs`)
- `previous_steps_before_the_restart_threshold` (`src/ui/tests/stepping_tests.rs`)
- `previous_with_shuffle_restarts_after_the_threshold_instead_of_retracing` (`src/ui/tests/stepping_tests.rs`)
- `a_failed_previous_restart_leaves_the_track_queued` (`src/ui/tests/stepping_tests.rs`)
- `a_failed_restart_report_names_the_track_and_position` (`src/ui/tests/stepping_tests.rs`)
- `a_failed_restart_report_without_a_track_names_the_loaded_preview` (`src/ui/tests/stepping_tests.rs`)

**History.** Added 2026-10-24 with the Previous restart plan, matching classic Winamp's rule that a
late Previous restarts the track already playing instead of stepping back.

## Mixer drag readout

**Guarantee.** While a volume or balance `chrome_slider`, or an equalizer band or preamp slider, is
held, the Now Playing LCD draws that slider's live level (`Volume: 72%`, `Balance: Center`,
`Balance: 31% Left`, `EQ: 60HZ +3 DB`, `Preamp: -12 DB`) in place of the
`Now Playing: ` caption and title; dragging the seek slider likewise reads `Seek to: 00:12/00:30
(41%)` (target over the preview's `playback_duration`, floored percent) while `seek_preview` is
held, and a mixer drag wins if both were set; without a `playback_duration` the title stays; the
glyph, elapsed readout, and total stay. The release shows the
title again. The EQ on/off button and preset picks never set the readout. The global arrow-key
handlers and wheel never set the readout; a slider's own arrow-key or Ctrl+wheel change message
does. `subscriptions::mixer_key_message` turns a navigation key press
(arrows, Home/End, Page) into `MixerReleased` and is always subscribed, because the slider sets the
readout during the event it must then see; other keys send nothing. `subscriptions::mixer_wheel_message`
does the same for a wheel roll but is subscribed only while a readout shows (the
`mixer_readout_showing` argument of `subscriptions`), so plain list scrolling rebuilds no screen.
Known limit: the Ctrl+wheel change that sets a readout leaves it until the next wheel roll or
navigation key. The marquee timer is off during a drag, seek drags included. Nothing is persisted.
Known limit: iced sends no release if a slider is removed mid-drag, so
the readout would then stay until the next mixer-slider release.

**Enforced by.** `handlers::mixer::{volume_change, balance_change, eq_preamp_change, eq_band_change,
mixer_released}`, `eq_slider_column`'s `on_release`, `WinampPlayer::mixer_drag` and `needs_marquee`,
`chrome_slider`'s `on_release`, and `views::mixer_readout_text` / `views::seek_readout_text` /
`now_playing_row`'s `mixer_readout` parameter.

**Pinned by.**

- `volume_readout_rounds_to_whole_percent`, `balance_readout_names_the_side`,
  `eq_readouts_show_the_band_label_and_signed_gain`,
  `seek_readout_shows_target_length_and_floored_percent`
  (`src/ui/views/now_playing/mixer_readout.rs`)
- `now_playing_row_with_a_readout_replaces_the_title` (`src/ui/views/tests/now_playing.rs`)
- `a_key_or_wheel_event_ends_a_mixer_readout` (`src/ui/tests/mixer_tests.rs`)
- `dragging_volume_shows_the_level_until_release`, `dragging_balance_shows_the_side`,
  `volume_keys_and_wheel_do_not_show_the_readout`, `the_marquee_pauses_during_a_mixer_drag`,
  `dragging_an_eq_band_shows_its_gain_until_release`, `dragging_the_preamp_shows_its_gain`,
  `eq_toggle_and_preset_do_not_show_the_readout`, `an_out_of_range_eq_band_shows_no_readout`,
  `dragging_the_seek_slider_shows_the_target_until_release`,
  `a_seek_drag_without_a_duration_shows_no_seek_text`, `the_marquee_pauses_during_a_seek_drag`
  (`src/ui/tests/mixer_tests.rs`)

**History.** Added 2026-10-24 with the mixer readout plan.

## Balance center detent

**Guarantee.** A `BalanceChange` that moves toward center and lands with magnitude below
`BALANCE_CENTER_SNAP` (0.1) stores exactly `0.0`; the boundary value and any move away from
center are stored as clamped ("toward" is judged against the drag's previous requested value,
cleared on release, so a continuous inward drag stays snapped), so the slider's arrow keys can still
leave center. Settings restore (`adopt_saved_settings`) goes through `AppState::set_balance` and is
never snapped.

**Enforced by.** `state::snap_balance_to_center` and `handlers::mixer::balance_change`.

**Pinned by.**

- `snap_balance_to_center_snaps_an_inward_move_near_center`,
  `snap_balance_to_center_never_snaps_an_outward_move` (`src/state.rs`)
- `dragging_balance_into_the_middle_snaps_to_center`, `a_continuous_inward_balance_drag_stays_snapped`,
  `a_balance_step_after_release_leaves_center` (`src/ui/tests/mixer_tests.rs`)

**History.** Added 2026-10-24 with the balance center detent plan.
