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
  (all in `src/ui/tests.rs`)
- `message_for_maps_backspace_to_back` (`src/ui/shortcuts.rs`)
- `message_for_maps_f5_to_reload` (`src/ui/shortcuts.rs`)
- `shortcut_ignores_chord_modifiers` (`src/ui/shortcuts.rs`)
- `back_button_is_available_only_below_the_artist_list` (`src/ui/views/tests.rs`)

**History.** Reconstructed from "Browse view can only move forward — no way back from
Albums/Songs, a dead end" in `BUGS-FIXED.md`. The All Songs, All Albums, and Playlists levels
were added later and are covered here too. Reload was added to refresh the artists list without
restarting the app; F5 later became its keyboard route.

## Search focus

**Guarantee.** Ctrl+F, with or without Shift, from any normal-mode browse level moves keyboard
focus to the `Search library` box without submitting a search. Focusing changes no browse view,
list rows or order, selection, typed query, search-active flag, or playback state; typing into the
focused box edits the query exactly as a click does. Plain F and the Alt+F and Logo+F chords stay
unbound, and every other Control chord remains the window manager's. Shade mode builds no search
box, so Ctrl+F there is a no-op.

**Enforced by.** `views::browse::BROWSE_SEARCH_ID` in `src/ui/views/browse.rs`, applied with
`.id(BROWSE_SEARCH_ID)` by `views::browse::view_search_box` (used by the re-exported
`views::browse::view_search_row`), gives the text input a stable focus target.
`Message::FocusSearch` in `src/ui/message.rs` is routed by `src/ui/mod.rs` to
`browse::search::focus_search` in `src/ui/handlers/browse/search.rs`, which returns
`iced::widget::operation::focus(BROWSE_SEARCH_ID)` and mutates no player state.
`shortcuts::shortcut` in `src/ui/shortcuts.rs` claims a Control-held `f` before its
Control/Alt/Logo chord guard, while leaving Alt+F and Logo+F to the window manager.

**Pinned by.**

- `ctrl_f_focuses_the_search_box_and_changes_no_browse_state` (`src/ui/tests.rs`)
- `ctrl_f_is_a_noop_in_shade_mode` (`src/ui/tests.rs`)
- `message_for_maps_ctrl_f_to_focus_search` (`src/ui/shortcuts.rs`)
- `view_search_box_registers_its_focus_id` (`src/ui/views/tests.rs`)

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
`views::browse` directly. `src/ui/view.rs` builds that row where the bare search box used to sit.

**Pinned by.**

- `clear_search_without_an_active_search_clears_only_the_query` (`src/ui/tests.rs`)
- `clear_search_with_an_empty_query_schedules_no_work` (`src/ui/tests.rs`)
- `clear_search_while_a_search_is_active_returns_to_artists_and_clears_the_query`
  (`src/ui/tests.rs`)
- `view_clear_search_button_constructs` (`src/ui/views/tests.rs`)
- `view_search_row_draws_clear_after_the_search_box` (`src/ui/views/tests.rs`)

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

- `search_songs_matches_the_sample_library_by_title_or_artist` (`src/apple_music/tests.rs`)
- `search_songs_matches_the_sample_library_by_album_title` (`src/apple_music/tests.rs`)
- `search_songs_rejects_a_blank_query` (`src/apple_music/tests.rs`)
- `search_songs_uses_the_rest_library_when_signed_in` (`src/apple_music/tests.rs`)

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
replaced, cleared, failed, sorted, or restored to library order, because the stored index then names
a different row.

**Enforced by.** `BrowseList::select_row` in `src/ui/browse.rs` validates the press through
`BrowseList::select` and records it, returning `Some(true)` only for the second press on the
selected row; `BrowseList::selected` exposes it to the view, and `replace_rows`, `clear`, `fail`,
`sort_rows_by`, and `restore_library_order` clear it with the rows or order they change.
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
- `browse_open_button_sits_above_the_browse_list`
  (all in `src/ui/tests.rs`)
- `message_for_maps_enter_to_open_selected`
  (in `src/ui/shortcuts.rs`)
- `browse_row_style_keeps_playing_rows_unbarred_with_white_text`
- `browse_row_style_paints_only_the_selected_row_blue`
  (all in `src/ui/views/tests.rs`)
- `now_playing_bar_and_browse_action_buttons_construct`
- `browse_open_button_constructs`
- `browse_views_construct_over_the_loaded_library`
- `browse_views_construct_over_an_empty_list`
  (all in `src/ui/views/tests.rs`)

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
  (all in `src/ui/tests.rs`)

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

**Enforced by.** `BrowseList::sort_rows_by` in `src/ui/browse.rs` builds a stable index permutation
from the lowercased row keys and rebuilds `items`, `titles`, and `labels` by it, bumping the epoch
only when there are at least two rows and the list is not loading; it also records the cumulative
permutation back to library order, and `BrowseList::restore_library_order` inverts it so the rows
(and their cached text) return to the order they were stored in, bumping the epoch as any index
change must. `browse::sort_browse` in `src/ui/handlers/browse.rs` matches the current view onto
its list and key, routed from `Message::SortBrowse` in the `update` dispatch table;
`browse::back` restores that order on the list Back returns to (those lists stay in memory while a
child is open), and `views::view_browse_nav_buttons` in `src/ui/views/browse.rs` pushes
`view_top_button`, `view_sort_button`, and `view_reload_button` into one `Row`, so Sort and Reload
draw beside Top.

**Pinned by.**

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
  (all in `src/ui/tests.rs`)

## Browse playing-row visibility

**Guarantee.** When Next, Previous (including Shuffle's retrace of the tracks actually played), or
the preview-end advance lands on a song while a song list (`CurrentView::Songs` or
`CurrentView::AllSongs`) is showing, the update loop scrolls the browse list to that row's offset,
`BROWSE_LIST_PADDING + index * BROWSE_ROW_HEIGHT`, in addition to scheduling the existing
`TrackSelected`. `BROWSE_ROW_HEIGHT` is the laid-out vertical pitch between consecutive browse rows:
every shared row title stays on one line — hard line breaks are flattened to spaces and a title
wider than its column is clipped there — so no title can make its row taller or draw under the
right-aligned label. `BROWSE_LIST_PADDING` is the list's top padding, which the offset includes. A
later row therefore cannot drift out of view from a stale constant; iced clamps the absolute offset
to the list's content. A direct row press does not scroll — the pressed row is already in view — and
stepping while another browse level is showing does not scroll, since only a song list carries the
`▶` playing marker. Top, Back, and every list-opening navigation keep their existing snap-to-top
behavior, and playback, selection, sorting, row text, and audio behavior are unchanged.

**Enforced by.** `views::BROWSE_ROW_HEIGHT` and `views::BROWSE_LIST_PADDING` in
`src/ui/views/browse/rows.rs` are the one pitch and padding `scrollable_list` lays its rows
out with, and `scrollable_list` flattens title line breaks and clips each title to its
column, so every row keeps that pitch and stays inside its column;
`handlers::browse::scroll::scroll_browse_to_row` in `src/ui/handlers/browse/scroll.rs` schedules
the `iced::widget::operation::scroll_to` absolute offset, and
`handlers::browse::scroll::scroll_browse_to_playing_row` gates it on the current view.
`handlers::playback::step_track` and
`handlers::playback::preview::playback_finished` batch that scroll with their `TrackSelected`,
so Next, Previous, a shuffled retrace, and a natural preview end all follow the landed row.

**Pinned by.**

- `browse_row_height_matches_the_laid_out_row_pitch`
- `scrollable_list_clips_an_overlong_title_to_its_column`
  (`src/ui/views/tests.rs`)
- `a_hard_line_break_in_a_title_does_not_change_the_row_pitch`
- `a_carriage_return_in_a_title_does_not_change_the_row_pitch`
- `next_and_previous_scroll_the_showing_song_list_to_the_landed_row`
- `a_finished_preview_scrolls_the_song_list_to_the_advanced_row`
- `stepping_while_another_browse_level_shows_does_not_scroll`
- `a_row_press_does_not_scroll_the_list`

## Browse keyboard scrolling

**Guarantee.** A plain or Shift-held `Home` key press snaps the showing browse list to its first
row, exactly like the `Top` button, and `End` snaps it to its last row. `Page Up` and `Page Down`
(plain or Shift-held) instead scroll that list one page of `BROWSE_PAGE_HEIGHT` — eight
`BROWSE_ROW_HEIGHT` rows — in the matching direction, relative to the list's current offset.
Every one of these keys is a view-scroll operation only: they change no current view, rows or row
order, selection, typed query, search-active flag, or playback state, and they schedule no fetch
or playback task. In shade mode no browse list is built, so the scroll operation has no target
and nothing moves; iced clamps the offset at the list's bounds, so paging past either end leaves
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

- `scroll_browse_to_bottom_schedules_a_scroll_task` (`src/ui/tests.rs`)
- `scroll_browse_page_keys_schedule_page_sized_scrolls` (`src/ui/tests.rs`)
- `message_for_maps_home_and_end_to_browse_scroll` (`src/ui/shortcuts.rs`)
- `message_for_maps_page_keys_to_browse_paging` (`src/ui/shortcuts.rs`)
- `shortcut_ignores_chord_modifiers` (`src/ui/shortcuts.rs`)

**History.** Added with the Home and End keyboard scrolling alongside the existing Top button;
extended with the Page Up and Page Down one-page scrolling.

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
  (all in `src/ui/views/tests.rs`)
- `artists_loaded_populates_list`
- `sort_reorders_artists_and_keeps_their_cached_labels_aligned`
- `back_restores_sorted_artists_and_their_cached_album_labels`
  (all three in `src/ui/tests.rs`)
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
and `views::browse::rows::text::album_row` in `src/ui/views/browse/rows/text.rs` renders the
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
  (all in `src/ui/views/tests.rs`)
- `albums_loaded_populates_list`
- `all_albums_loaded_stores_the_flat_list`
  (both in `src/ui/tests.rs`)
- `albums_carry_their_songs_total_durations`
  (in `src/sample_library.rs`)
- `album_duration_is_not_serialized_and_defaults_to_zero`
  (in `src/library.rs`)
- `get_album_durations_groups_and_sums_library_songs`
  (in `src/apple_music/rest/tests.rs`)
- `get_albums_by_artist_uses_the_rest_library_when_signed_in`
- `get_albums_by_artist_fails_when_the_albums_request_fails`
- `get_albums_by_artist_fails_when_the_album_durations_request_fails`
- `get_all_albums_uses_the_rest_library_when_signed_in`
- `get_all_albums_leaves_an_album_without_matching_songs_at_zero_duration`
- `get_all_albums_fails_when_the_albums_request_fails`
- `get_all_albums_fails_when_the_album_durations_request_fails`
  (all in `src/apple_music/tests.rs`)

## UI layout

**Guarantee.** The root `view`'s layout keeps these geometry guarantees:

- The browse navigation row lays Sort and Reload beside Top, not stacked below them: all three
  share the same `y`, Sort starts at or after Top's right edge, and Reload starts at or after
  Sort's right edge.
- The transport row draws, in order, Previous, Play/Pause, Stop, and Next as the compact glyphs
  `◀◀`, `▶`/`II`, `■`, and `▶▶`, each with a word hover tooltip (`Previous`, `Play`/`Pause`,
  `Stop`, `Next`). The four glyph buttons are pinned to fixed widths 34, 30, 30, and 34 px, so
  the Play/Pause glyph swap cannot reflow the row. Repeat and Shuffle remain the `Repeat:
  Off/On` and `Shuffle: Off/On` text toggles, pinned to 104 and 112 px, so their label swaps
  cannot reflow it either. The shade strip builds its four buttons from the same glyph spec.
- The main transport row's fixed-width `EQ` toggle follows the balance slider in every playback
  state and is sunken while the equalizer panel is shown, ordinary chrome while it is hidden.
  The root `view` draws the `NOSTALGIA EQUALIZER` band and `view_equalizer` exactly when
  `WinampPlayer::eq_panel_visible` holds. Hiding them moves the `NOSTALGIA LIBRARY` band and the
  browse list up into the freed height without resizing the window, and leaves the equalizer's
  on/off state, preamp, band gains, preset, and audio forwarding untouched.
- The equalizer band row puts the vertical preamp slider leftmost, then the `+12 DB` / `0 DB` /
  `-12 DB` scale, then the ten band sliders. The scale reads top to bottom beside the rail's top,
  middle, and bottom, and the preamp label shares the band labels' text row. The response curve
  above them starts at the first band, not over the preamp and scale.
- The root `view` draws the browse level named by `CurrentView` from that level's own buffer and
  cached row text, and marks the playing row only in the Songs and All Songs lists.
- The root `view` draws Back on every level below Artists, Retry only on Artists while its fetch
  has failed, and the All Songs, All Albums, and Playlists buttons only on Artists.
- The app bar and the two section bands are `TITLE_BAR_HEIGHT` tall. The app bar centers
  `NOSTALGIA` between gold tapes with its A and shade/minimize/close controls pinned at the
  edges. Normal mode draws `NOSTALGIA EQUALIZER` immediately above the equalizer and
  `NOSTALGIA LIBRARY` immediately above the browse controls; the section bands emit no messages
  and are absent in shade mode.

**Enforced by.** `views::view_browse_nav_buttons` in `src/ui/views/browse.rs` pushes the Top, Sort,
and Reload buttons into one `Row`. `views::view_transport_controls`,
`views::transport::transport_buttons`, and `views::transport::transport_glyphs` in
`src/ui/views/transport.rs` build the glyph row from `fixed_width_button`s and
`iced::widget::Tooltip`s; `views::transport::eq_panel_toggle` and
`views::transport::eq_panel_button_style` build and style the EQ toggle, and
`handlers::mixer::toggle_equalizer_panel` flips the layout flag; `view` pushes the equalizer section
only while that flag holds; `views::mini_transport::mini_transport_controls` projects the same glyph
spec for the shade strip and the library footer. `views::equalizer::preamp_column`,
`views::equalizer::db_scale_column`, and `views::view_equalizer` in `src/ui/views/equalizer.rs`
build the equalizer band row and align the curve with it. The browse and transport layout tests lay
out the whole root `view` through the test-only `src/ui/layout_capture.rs` harness; the equalizer
layout test lays out `view_equalizer` directly. `view` in `src/ui/view.rs` matches on `CurrentView`
and passes each arm that level's buffer, caches, and current-track id, and gates its Back, Retry,
and flat-list buttons on `can_go_back`, `can_retry_artists`, and a `CurrentView::Artists` match.
`views::view_title_band` and `views::title_bar::title_tape` in `src/ui/views/title_bar.rs` build the
section bands and the app bar's tape/label row from `theme::TITLE_TAPE` and
`theme::TITLE_TAPE_SHADOW`; `view` in `src/ui/view.rs` pushes the two bands into the normal-mode
column only. The harness records each drawn text fragment's absolute bounds with iced's headless
tiny-skia renderer.

**Pinned by.** `browse_nav_buttons_place_sort_beside_top`,
`browse_nav_buttons_place_reload_beside_sort`,
`transport_row_does_not_reflow_when_the_play_glyph_changes`,
`root_view_draws_the_transport_glyph_row`,
`root_view_offers_back_retry_and_flat_list_buttons_only_where_they_belong`,
`root_view_draws_the_current_browse_levels_own_rows`,
`root_view_draws_the_equalizer_and_library_title_bands_in_order`,
`toggle_equalizer_panel_hides_and_restores_the_section`, and
`shaded_root_view_draws_no_section_title_bands` (`src/ui/tests.rs`), plus
`title_band_centers_its_label_at_the_title_bar_height` and
`title_bar_centers_its_name_between_its_edge_controls` (`src/ui/views/tests.rs`) for the title
bands and app bar, `transport_glyphs_match_the_mockup_order_and_widths`,
`transport_buttons_keep_a_fixed_width_across_label_changes`,
`transport_buttons_keep_their_measured_face_widths`,
`main_transport_and_shade_strip_share_glyphs_order_and_widths`,
`eq_panel_toggle_targets_the_visibility_flag`,
`eq_panel_button_style_sinks_while_the_panel_is_open`, and
`equalizer_preamp_and_db_scale_sit_left_of_the_band_sliders` (`src/ui/views/tests.rs`) for the
glyph row, the Repeat and Shuffle faces, and the equalizer band row.

**History.** Reconstructed from "Sort renders below Top instead of beside it, contradicting the
documented layout" and "Transport buttons resize to their label text, so pressing Play/Pause
reflows the whole control row" in `BUGS-FIXED.md`. Both carried a `real-run-needed` validation
gap; the shared harness closes it.

## Settings persistence

**Guarantee.** The values the user sets that outlive a track — volume, balance, Repeat, Shuffle, the
equalizer's on/off state, preamp, ten band gains, and selected preset, the large readout's
elapsed/remaining choice, the visualizer's shown/hidden choice, and the title bar's always-on-top
choice — are loaded once before the
first view is drawn and rewritten after any update that changes their snapshot. An update that
leaves the snapshot unchanged writes nothing. Reads and writes go through the state's clamped
setters, so out-of-range values from a file cannot push shared state outside its documented ranges.
A missing or unreadable file, or a malformed line, falls back per key without failing startup; an
unwritable file is logged rather than crashing the player. The readout mode, the visualizer's
shown/hidden choice, and the always-on-top choice are the three persisted values not held in
`AppState`: they live on `WinampPlayer::time_display`, `WinampPlayer::visualizer_visible`, and
`WinampPlayer::always_on_top`, `main` passes the loaded snapshot through to
`WinampPlayer::adopt_saved_settings` (which restores all three before the first view), and
`persist_settings_if_changed` copies them into the snapshot before comparing and saving. A missing
or malformed `time_display` line falls back to elapsed, a missing or malformed `always_on_top` line
falls back to unpinned, and a missing or malformed `visualizer_visible` line falls back to shown.
The classic `A` key reaches the same `Message::ToggleAlwaysOnTop` arm through
`shortcuts::message_for`, so it flips the flag and schedules the same window-level call as the
on-screen A controls. The saved always-on-top level waits for the window id every other window
action uses: `window_id_resolved` stores the id, then schedules `iced::window::set_level` when the
restored flag holds. It is still not window placement: the window's size, position, and shade state
are not persisted. Playback position, the browse level, the search box, the current track, and
window placement are deliberately not persisted.

**Enforced by.** `src/main.rs` resolves the path with
`settings::file_store::FileStore::from_environment`, loads it once with `FileStore::load`, applies
the result with `settings::Settings::apply_to`, and
passes the store and the loaded snapshot (or a `Settings::from_state` snapshot when the file is
absent) to `ui::init_ui`. The loaded
file's text is parsed by `Settings::from_text`, which ignores unknown keys and malformed values
per key, and `apply_to` writes through `AppState::set_volume`, `set_balance`, `set_eq_preamp`, and
`set_eq_band` before restoring a preset through `apply_eq_preset`. `ui::init_ui` attaches the
store to `WinampPlayer` and restores the snapshot through
`WinampPlayer::adopt_saved_settings`, which restores `time_display`, `visualizer_visible`, and
`always_on_top` and
records the snapshot; `update` calls
`WinampPlayer::persist_settings_if_changed` once after every message, and that method copies
the three player-owned values into `Settings::from_state`, then compares that snapshot against the
last one before calling `settings::file_store::SettingsWriter::save`. Boot's `WindowIdResolved` is
handled by
`handlers::window::window_id_resolved`, which applies the restored level through
`window_level` once the id exists.
`settings::file_store::FileStore`'s `SettingsWriter` impl creates the parent directory and logs a
failed create or write.
With no store attached the method returns before taking the state lock, so tests keep the default
no-op behavior. `shortcuts::message_for` maps `A` to `Message::ToggleAlwaysOnTop`, the arm those
controls press.

**Pinned by.** `to_text_round_trips_every_field`,
`time_display_round_trips_both_words_and_ignores_unknown_values`,
`visualizer_visibility_round_trips_and_ignores_malformed_values`,
`always_on_top_round_trips_and_ignores_malformed_values`,
`from_text_ignores_malformed_and_unknown_lines`,
`apply_to_clamps_out_of_range_values_through_the_state_setters`,
`apply_to_restores_a_saved_preset`,
`settings_path_resolves_override_then_xdg_then_home_then_none`, and
`file_store_save_then_load_returns_the_same_settings` (`src/settings/file_store.rs`), plus
`a_changed_setting_is_saved_once_and_unchanged_updates_are_not`,
`repeat_and_equalizer_changes_reach_the_saved_snapshot`,
`toggle_time_display_reaches_the_saved_snapshot`,
`saved_time_display_mode_is_restored_on_boot`,
`toggle_visualizer_reaches_the_saved_snapshot`,
`saved_visualizer_visibility_is_restored_on_boot`,
`toggle_always_on_top_reaches_the_saved_snapshot`,
`saved_always_on_top_is_restored_on_boot`,
`window_level_selects_always_on_top`, and
`window_id_resolved_applies_a_saved_always_on_top_level` (`src/ui/tests.rs`). The classic `A` key
reaching the same arm is pinned by `a_toggles_always_on_top_from_the_keyboard` (`src/ui/tests.rs`).

**History.** Added with the settings-persistence plan on 2026-10-23; the player previously kept
these values in memory only and started every launch at `AppState::default()`. The readout's
elapsed/remaining mode and the always-on-top choice joined the file later the same day. The
visualizer's shown/hidden choice joined it with the same plan, restoring before the first view
while the stored spectrum and its poll stay transient.

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

## Volume wheel

**Guarantee.** A wheel roll over the player panels (the title bar, Now Playing bar, seek strip,
transport row, equalizer title band, and equalizer panel, plus the title bar and shade strip in
shade mode) moves the volume by exactly one `views::VOLUME_STEP` in the roll's vertical
direction: a positive `y` raises it through `handlers::mixer::volume_up`, a negative `y` lowers it
through `handlers::mixer::volume_down`, and both go through `AppState::nudge_volume` and the same
audio-backend forward as the arrow keys, so the clamp and the step cannot drift from them. Only
the sign of the delta matters, so one line or pixel event is one step. The browse list is outside
that area, so its `Scrollable` keeps the wheel and the volume is unchanged; a horizontal-only
roll or a zero direction changes no volume and schedules no work.

**Enforced by.** `views::volume_wheel_message` in `src/ui/views/mod.rs` (the sign mapping),
`Message::VolumeWheel` in `src/ui/message.rs`, `handlers::mixer::volume_wheel` in
`src/ui/handlers/mixer.rs` (the zero/NaN no-op), and the two `MouseArea::on_scroll` wrappers in
`src/ui/view.rs` (the normal and shaded trees).

**Pinned by.** `volume_wheel_message_maps_a_vertical_roll_to_a_direction` and
`volume_wheel_message_zeroes_a_horizontal_only_roll` in `src/ui/views/tests.rs`, plus
`volume_wheel_nudges_by_one_slider_step_and_ignores_zero` and `volume_wheel_clamps_at_the_ends`
in `src/ui/tests.rs`.

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
  `<number>Artist - Title - nostalgia` and omits an empty number or artist. It keeps the
  interrupted track across Stop and falls back to the plain `nostalgia` for no current track or
  an unknown id.
- The number prefix counts toward the marquee character budget alongside the artist and title,
  and an overlong line's scrolling window includes it, so it neither overflows the LCD nor stays
  fixed while the rest of the line scrolls.
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
  elapsed and remaining while playing or paused. The classic `T` key reaches the same
  `Message::ToggleTimeDisplay` through `shortcuts::message_for` in every transport state; the
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
- The LCD's visualizer row draws exactly `BAND_COUNT` green bars, left to right in band order,
  bottom-aligned inside a fixed-height strip between the title and the status row. The bar heights
  come from the loaded preview's stored spectrum, each band clamped to `0.0..=1.0`; no reported
  spectrum, including the silent fallback, draws zero-height bars while keeping the row's height,
  so loading a preview does not move the title or status text. The bars are drawn only while
  `WinampPlayer::visualizer_visible` holds; hiding them changes only what the LCD draws, keeping
  both the stored spectrum and the poll, so showing them again draws the current bars. Pause
  keeps the last bars frozen, while Stop and a new selection clear them. A spectrum reply issued
  before a Stop or a new selection, or landing while a transport transition is in flight, is
  dropped. The spectrum poll is subscribed only while playback is running.
- The LCD's left edge always draws the five classic indicator letters `O`, `A`, `I`, `D`, `V`,
  in that order, at a fixed width, in every playback state including the empty `--:--` bar. `A`
  and `V` are mouse areas: pressing `A` publishes `Message::ToggleAlwaysOnTop` (the title bar's
  always-on-top toggle) and pressing `V` publishes `Message::ToggleVisualizer`, which flips only
  the LCD's display flag. `O`, `I`, and `D` stay decorative and publish nothing. The strip keeps
  its fixed width and letter order, so the content beside it does not move.
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
  the `--:--` placeholder, which still shows while stopped. The blink adds no timer or subscription.
  Pinned by `transport_glyph_names_and_colors_each_mode`,
  `now_playing_row_keeps_the_elapsed_readout_in_place_across_transport_modes`,
  `now_playing_content_draws_the_indicator_strip_beside_the_title`,
  `elapsed_readout_color_blinks_when_paused_and_clears_when_stopped`,
  `elapsed_readout_color_keeps_the_placeholder_visible_while_stopped`,
  `elapsed_readout_is_clickable_only_while_not_stopped`,
  `elapsed_readout_toggles_only_while_not_stopped`,
  `compact_readout_toggles_time_display_only_while_not_stopped`, and
  `now_playing_row_keeps_the_elapsed_bounds_when_the_blink_hides_it` in `src/ui/views/tests.rs`;
  `lcd_indicator_message_arms_a_and_v_and_leaves_o_i_d_inert` and
  `lcd_indicator_strip_publishes_always_on_top_on_a_and_visualizer_on_v` in `src/ui/views/tests.rs`;
  `toggle_visualizer_hides_and_shows_the_bars_without_clearing_the_spectrum` in `src/ui/tests.rs`;
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
- A number-prefixed artist/title line longer than `MARQUEE_WIDTH` characters is windowed by
  `views::now_playing::marquee_text`, advanced one character per `Message::MarqueeTick`, and
  reset to the start by `TrackSelected`; a line that fits is shown whole and borrowed.
- The marquee timer is subscribed only while the line needs to scroll: `WinampPlayer::needs_marquee`
  measures the same number, label, and artist the bar renders, so a line that fits leaves the timer
  out (every message makes iced rebuild the whole screen, browse list included).
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
`playback_position` field on `WinampPlayer` stores the reply the `PlaybackPosition` handler
received, which is stamped with `position_epoch`, and `WinampPlayer::readout_position` prefers an
in-progress seek drag over that value so the large readout and the seek thumb share one position
source; `reset_position_readout` bumps that epoch and clears the value on Stop and on a new
selection. `pending_position_transitions` counts the backend transport transitions that are still in
flight: every transport handler in `src/ui/handlers/playback.rs` that schedules one begins a
transition, and the completion that reports that same transition ends it; while the count is nonzero
the `handlers::playback::preview` poll handlers issue no query and its reply handlers store no
reply. `AppleMusicService::play_track` stops the backend's previous preview when a selection has no
preview URL. `views::now_playing::lcd_indicator_strip` draws
`views::now_playing::LCD_INDICATOR_LABELS` down the LCD's left edge in a fixed-width column; its `A`
and `V` cells publish `Message::ToggleAlwaysOnTop` and `Message::ToggleVisualizer` through
`views::now_playing::lcd_indicator_message`, while `O`, `I`, and `D` publish nothing.
`views::now_playing::visualizer_bar_heights` scales and clamps the stored spectrum, and
`views::now_playing::now_playing_visualizer_row` draws its `BAND_COUNT` bars at fixed width inside
the row `views::view_now_playing` places between the title and status rows. The `Spectrum` handler
in `src/ui/handlers/playback/preview.rs` stores the reply under the same epoch and
pending-transition guards as the position, format, and duration, `spectrum_tick` schedules the
backend query, and `reset_position_readout` clears the bars on Stop and a new selection.
`ui::timers::subscriptions` subscribes `spectrum_ticks` only while
`WinampPlayer::needs_spectrum_poll` reports that playback is running.
`views::now_playing::now_playing_number` resolves `KnownTrack::number`,
`handlers::playback::track_selected` records the selecting row's 1-based number, and
`views::now_playing::now_playing_row` draws it (separately when the line fits, inside the window
when it scrolls). `views::now_playing::marquee_text` windows an overlong line, and
`WinampPlayer::marquee_offset` carries the advance through `Message::MarqueeTick`. The
`WinampPlayer::time_display` field holds the readout's mode, and
`handlers::playback::toggle_time_display` (routed from `Message::ToggleTimeDisplay`, pressed by the
`MouseArea` wrapping the readout in `now_playing_row` and mapped from the classic `T` key by
`shortcuts::message_for`) flips it without touching playback state.
`views::now_playing::display_len` and `views::now_playing::needs_marquee` measure the line, and
`ui::timers::subscriptions` subscribes `marquee_ticks` only while that measure exceeds
`MARQUEE_WIDTH`, and `position_ticks` only while `WinampPlayer::needs_position_poll` reports a
preview whose position can still change. `audio::PlaybackFormat` carries the preview's sample rate,
channel count, and average bitrate; `audio::preview::DecodedPreview` hands the download's encoded
byte count to `format_for_source` in `src/audio/player/format.rs`, which reads the rate and channels
from the decoded `rodio` source and computes the bitrate through `preview_bitrate_kbps` from the
source's own total duration, and
`views::now_playing::text::format_playback_prefix`/`text::channel_mode_label` format them, and
`views::now_playing::CHANNEL_LABELS`/`channel_label_color`/`now_playing_status_row` render them in
the LCD's status row built by `views::view_now_playing`. The `PlaybackFormat` handler in
`src/ui/handlers/playback/preview.rs` stores the reply beside the position under the same epoch and
pending-transition guards, and `reset_position_readout` clears the format on Stop and a new
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
- `now_playing_number_resolves_known_track_to_its_cached_prefix`
- `now_playing_time_resolves_known_track_to_cached_label`
- `now_playing_time_returns_the_plain_total_even_with_a_position`
- `now_playing_elapsed_reads_placeholder_zero_or_position`
- `now_playing_elapsed_remaining_counts_down_from_the_total`
- `format_lcd_elapsed_pads_minutes_and_seconds`
- `format_lcd_remaining_prefixes_a_minus_and_pads`
  (all in `src/ui/views/tests.rs`)
- `toggle_time_display_switches_to_remaining_and_persists`
- `toggle_time_display_with_no_track_keeps_the_placeholder`
- `t_toggles_the_time_display_from_the_keyboard`
  (all in `src/ui/tests.rs`)
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
  (all in `src/ui/views/tests.rs`)
- `now_playing_label_keeps_the_track_name_after_browsing_to_another_album` (`src/ui/tests.rs`)
- `window_title_names_the_current_track` (`src/ui/tests.rs`)
- `now_playing_number_survives_browsing_to_another_album` (`src/ui/tests.rs`)
- `track_selected_records_the_selected_rows_number`
- `replaying_a_track_from_another_row_updates_its_number`
- `the_player_knows_when_the_now_playing_line_needs_the_marquee`
- `now_playing_time_shows_the_played_tracks_duration_after_browsing_to_another_album`
  (`src/ui/tests.rs`)
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
- `a_paused_preview_needs_the_position_poll_before_its_first_reply` (`src/ui/tests.rs`)
- `play_track_without_a_preview_url_stops_the_previous_preview` (`src/apple_music/tests.rs`)
- `position_query_reports_none_when_nothing_is_loaded`
- `a_seek_is_reflected_by_the_next_position_query`
- `a_pause_keeps_the_loaded_player_reporting_its_position`
- `format_for_source_reads_sample_rate_and_channels`
- `preview_bitrate_kbps_computes_encoded_bits_over_the_preview_length`
- `preview_bitrate_kbps_rounds_to_the_nearest_kilobit_and_rejects_overflow`
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
  strip keeps a fixed `SHADE_STRIP_HEIGHT`, which aliases the shared
  `MINI_TRANSPORT_HEIGHT`, and normal mode builds no shade strip.
- The strip shows Previous, Play/Pause, Stop, Next, the elapsed-over-total time readout, and a
  volume slider. Its four buttons emit the same messages as their full transport-row twins
  (`Message::PreviousTrack`, `Message::PlayPause`, `Message::Stop`, `Message::NextTrack`), each
  pinned to a fixed width so the glyph swap cannot reflow the strip. The strip builds them from
  the shared compact row's `mini_transport_controls`, which projects the full row's
  `transport_glyphs` spec, so the shade strip, the library footer, and the full row share the
  same glyphs, order, widths, and messages; the Play/Pause glyph is `"▶"` while stopped and
  `"II"` while playing. A fill spacer pushes the slider to the right edge, after the
  left-aligned buttons and time readout.
- The time readout joins the same `WinampPlayer::now_playing_elapsed` and
  `WinampPlayer::now_playing_time` strings the Now Playing bar draws as `<elapsed>/<total>`, so
  the elapsed/remaining choice and the track's total carry into shade mode. When the total is the
  `--:--` placeholder the readout is just the elapsed string, never `--:--/--:--`. The volume
  slider writes `Message::VolumeChange` and reads the shared volume.
- While playing or paused, the time readout is a click target that publishes
  `Message::ToggleTimeDisplay`, flipping the same `WinampPlayer::time_display` mode the LCD uses;
  while stopped it publishes nothing, so a stopped press cannot silently switch the persisted
  mode. Because shade mode builds no LCD, this is the only time toggle available there. The
  shared row applies the LCD's own `views::now_playing::elapsed_readout_clickable` guard rather
  than a second rule.

**Enforced by.** `views::shade::view_shade_strip` wraps
`views::mini_transport::mini_transport_row` on the title bar's blue chrome;
`views::mini_transport::mini_transport_controls` projects
`views::transport::transport_glyphs` (including `views::transport::play_pause_glyph`) into the
compact buttons; `views::mini_transport::mini_transport_readout` joins the elapsed and total
strings and keeps a single time string when the total is unknown;
`views::mini_transport::mini_transport_row` wraps the readout in a
`MouseArea::on_press(Message::ToggleTimeDisplay)` only when
`views::now_playing::elapsed_readout_clickable` holds for the strip's mode, and leaves it a plain
`Text` while stopped; `views::mini_transport::mini_volume_spec` fixes the slider's shared range
and `Message::VolumeChange`; `views::mini_transport::MINI_TRANSPORT_HEIGHT` owns the height, and
`views::shade::SHADE_STRIP_HEIGHT` aliases it while `views::shade::SHADED_HEIGHT` adds the title
bar; the shaded branch of `ui::view::view` stacks the bar and strip; and
`handlers::window::resize_to_shade` resizes to `handlers::window::shade_target_size`, which reads
`SHADED_HEIGHT`. `views::view_shade_strip` is re-exported by `src/ui/views/mod.rs`.

**Pinned by.** `shade_play_pause_glyph_switches_with_playback_state`,
`shade_controls_map_to_transport_messages_and_keep_fixed_widths`,
`main_transport_and_shade_strip_share_glyphs_order_and_widths`,
`shade_height_pins_the_rolled_up_window`,
`shade_volume_slider_spec_matches_the_shared_volume_range_and_message`,
`shade_strip_draws_the_time_readout_and_puts_the_volume_slider_at_the_right_edge`,
`mini_transport_readout_appends_the_known_total`,
`compact_readout_toggles_time_display_only_while_not_stopped`, and
`view_shade_strip_keeps_its_fixed_height_for_both_playback_states` (`src/ui/views/tests.rs`);
`shaded_root_view_is_the_title_bar_plus_the_strip`, `shade_strip_draws_only_while_shaded`,
`shade_target_size_is_the_title_bar_plus_the_strip`,
`title_bar_height_pins_the_shade_strip`, and `root_view_footer_shows_elapsed_over_total`
(`src/ui/tests.rs`).

**History.** Added with the window-shade transport strip; the earlier shade mode built the title
bar alone. The strip's controls now come from the shared compact row the library footer uses.

## Library footer

**Guarantee.** Normal mode draws exactly one compact transport row beneath the browse list:

- The row is exactly `MINI_TRANSPORT_HEIGHT` tall and sits on the window face's raised chrome
  below the browse list.
- It shows Previous, Play/Pause, Stop, Next, the elapsed-over-total time readout, and a volume
  slider. The four buttons emit `Message::PreviousTrack`, `Message::PlayPause`, `Message::Stop`,
  and `Message::NextTrack`, and the Play/Pause glyph follows `is_playing`. The time label joins
  the same `WinampPlayer::now_playing_elapsed` and `WinampPlayer::now_playing_time` strings the
  LCD shows as `<elapsed>/<total>`, so the elapsed/remaining choice and the total cannot disagree
  between the two; when the total is the `--:--` placeholder the readout is just the elapsed
  string, never `<elapsed>/--:--`.
- The volume slider is tagged `MINI_VOLUME_ID`, pinned to `MINI_VOLUME_WIDTH` at the row's right
  edge, and writes `Message::VolumeChange`. No new message, handler, or playback path exists.
- The footer and the shade strip build from the same row builder, so a glyph, order, width,
  message, or slider change shows up in both. Normal mode draws no shade strip, and shade mode
  draws no library footer.
- While playing or paused, the footer's time readout is a click target that publishes
  `Message::ToggleTimeDisplay`, flipping the same `WinampPlayer::time_display` mode the LCD uses;
  while stopped it publishes nothing. The shared row applies the LCD's own
  `views::now_playing::elapsed_readout_clickable` guard rather than a second rule.

**Enforced by.** `views::mini_transport::view_library_strip` wraps
`views::mini_transport::mini_transport_row` for normal mode;
`views::mini_transport::mini_transport_controls` and
`views::mini_transport::mini_volume_spec` are the shared control and slider specs;
`views::mini_transport::mini_transport_readout` joins the elapsed and total strings and keeps a
single time string when the total is unknown;
`views::mini_transport::mini_transport_row` wraps the readout in a
`MouseArea::on_press(Message::ToggleTimeDisplay)` only when
`views::now_playing::elapsed_readout_clickable` holds for the strip's mode, and leaves it a plain
`Text` while stopped;
`views::mini_transport::MINI_TRANSPORT_HEIGHT`, `MINI_VOLUME_WIDTH`, and
`MINI_VOLUME_ID` own the geometry and test seam. The normal branch of `ui::view::view` pushes the
footer after the browse content with clones of the LCD's elapsed and total strings, while its
shaded branch returns before that push.

**Pinned by.** `view_library_strip_keeps_its_fixed_height`,
`library_strip_draws_the_four_controls_time_and_right_edge_volume`,
`mini_transport_readout_appends_the_known_total`,
`compact_readout_toggles_time_display_only_while_not_stopped`, and
`main_transport_and_shade_strip_share_glyphs_order_and_widths` (`src/ui/views/tests.rs`);
`normal_root_view_draws_the_library_strip_beneath_the_browse_list`,
`shade_strip_draws_only_while_shaded`, and `root_view_footer_shows_elapsed_over_total`
(`src/ui/tests.rs`).

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
in a `Vec` of `PendingConnection`, which is defined in `src/music_kit_auth/server.rs`. The loop's
helpers live in `src/music_kit_auth/server.rs`: every pass calls `serve_pending`, which polls each
pending connection through `http::RequestReader::poll` and serves the ones whose request has
arrived, and then `accept_pending`, which takes on the connections that are waiting.
`http::RequestReader` owns a non-blocking socket and parses the request from the bytes received so
far, so polling a silent peer reports "nothing yet" instead of waiting for it; `serve_pending` drops
a connection whose budget is spent. `BootstrapPage::write` in `src/music_kit_auth/bootstrap.rs`
calls `sweep_stale_bootstrap_dirs` before creating the new owner-only directory; the sweep reads one
level of the temp dir, removes only aged directories whose names carry `BOOTSTRAP_DIR_PREFIX`, and
reports a removal failure without failing the new sign-in.

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

- `a_response_of_exactly_the_cap_is_accepted` (`src/apple_music/rest/tests.rs`)
- `a_compressed_response_larger_than_the_cap_is_rejected` (`src/apple_music/rest/tests.rs`)
- `a_response_with_more_wire_bytes_than_the_cap_is_rejected` (`src/apple_music/rest/tests.rs`)
- `a_text_response_with_invalid_utf8_keeps_the_lossy_replacement` (`src/apple_music/rest/tests.rs`)

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

**Enforced by.** `download_and_decode` in `src/audio/preview.rs` calls
`response.body_mut().with_config().limit(MAX_PREVIEW_BYTES + 1).reader()` for the wire limit, then
reads it through `take(MAX_PREVIEW_BYTES + 1)` and checks the decoded length before
`rodio::Decoder::new_mp4`. It does not use `Body::read_to_vec`, whose limiter sits under `ureq`'s
gzip decoder and so counts compressed bytes.

**Pinned by.**

- `a_compressed_preview_larger_than_the_cap_is_rejected` (`src/audio/tests.rs`)
- `a_preview_with_more_wire_bytes_than_the_cap_is_rejected` (`src/audio/tests.rs`)
- `a_preview_of_exactly_the_cap_reaches_the_decoder` (`src/audio/tests.rs`)

**History.** Added 2026-10-23 after the wire-byte cap was found to let a small gzip body expand
past 10 MiB before decoding; the decoded-byte fix keeps the original wire cap alongside it. The
wire limit was later raised one byte above the cap so an exactly-at-cap preview is read instead
of failing the limit on its last byte, the same boundary the REST transport raised.

## Preview host guard

**Guarantee.** A preview URL handed to the audio backend may reach only an address
`ip_is_internal` does not classify as internal. The URL must be absolute `https` and carry no
control character, its host must not be the local host name or a legacy numeric address form
(`2130706433`, `0x7f000001`, `0177.0.0.1`, `127.1`), and every address a hostname resolves to —
or an address literal the URL spells — must pass `ip_is_internal`. For IPv4 it refuses private,
loopback, link-local, unspecified, broadcast, documentation, multicast, `0.0.0.0/8`, shared
(CGNAT), benchmarking (`198.18.0.0/15`), reserved (`240.0.0.0/4`), IETF protocol-assignment
(`192.0.0.0/24`, the whole block, including the globally reachable `.9` and `.10` anycast
addresses), and 6to4 relay (`192.88.99.0/24`) ranges. For IPv6 it refuses loopback, unspecified,
multicast, unique-local, link-local, and the IPv4 address embedded in an IPv6 transition prefix
(6to4, NAT64 `/96` and `/48`, and Teredo); it does not refuse IPv6 documentation
(`2001:db8::/32`) or benchmarking (`2001:2::/48`) addresses. Redirects are not followed, and a
proxy cannot take over resolution.

**Enforced by.** `preview_url_problem`, `is_numeric_host_form`, and `ip_is_internal` in
`src/apple_music/validation.rs` (the last re-exported by `src/apple_music.rs`), applied at the
seam by `ensure_preview_url_is_valid`; `audio::preview::PublicAddressResolver` applies
`ip_is_internal` to every resolved address before connecting, and `preview_agent` follows no
redirects and disables the proxy.

**Pinned by.**

- `ip_is_internal_refuses_every_internal_range_and_allows_public_addresses`
- `preview_url_problem_unwraps_a_disguised_internal_host`
- `preview_url_problem_rejects_a_url_without_a_usable_https_host`
- `is_numeric_host_form_recognizes_legacy_spellings_and_leaves_names_alone`
  (all in `src/apple_music/tests.rs`)
- `the_preview_agent_refuses_a_hostname_that_resolves_to_loopback`
- `the_preview_resolver_allows_a_public_address_literal`
  (both in `src/audio/tests.rs`)

**History.** Reconstructed from "Preview URL guard accepts IPv4 multicast and reserved ranges" in
`BUGS-FIXED.md` (2026-10-23); the v6 branch already refused multicast while the v4 branch did not.

## Preview spectrum

**Guarantee.** The backend reports ten normalized amplitude bands in
`crate::equalizer::BAND_HZ` order only while a preview is loaded, computed from the samples the
worker actually pulls through `audio::meter::MeterSource`; those samples reach the equalizer,
balance, and output unchanged. A band at or above Nyquist reads zero, as do silence and an empty
window. A seek clears the recorded window without resetting the partial downmix frame, so the
sample rodio leaves on the same channel stays in phase; a player that has not appended a preview
reports nothing, so Stop and a new selection cannot report the previous preview's spectrum. The
silent fallback and a closed worker report `None`. The UI half stores that reply in
`WinampPlayer::spectrum` and draws exactly `BAND_COUNT` bars from it while
`WinampPlayer::visualizer_visible` holds; the indicator strip's `V` hides and shows them without
clearing the stored spectrum or stopping the poll. Their heights clamp to the fixed visualizer
row, Pause freezes them, Stop and a new selection clear them, stale or pre-transition replies
are dropped, and the poll runs only while playing.

**Enforced by.** `audio::meter::spectrum_bands` (the pure ten-band Goertzel analysis) and
`audio::meter::SpectrumMeter` (`push` downmixes and fills the fixed ring, `take` copies it and
analyzes outside the lock, `clear` empties the ring while keeping the partial downmix frame) in
`src/audio/meter.rs`; `MeterSource` records each sample while returning it unchanged and clears
the ring on `try_seek` without resetting that frame. `AudioOutput::spectrum`
in `src/audio.rs` defaults to `None`, and `RodioPlayer::spectrum` in
`src/audio/player/rodio_player.rs` answers `Some` only once the player holds a preview's format,
through the worker's `Command::Spectrum`
handled beside `Command::PlaybackFormat`; `RodioOutput::spectrum` bounds the query like the other
backend queries. On the UI side, `views::now_playing::visualizer_bar_heights` and
`views::now_playing::now_playing_visualizer_row` scale and draw the bands,
`views::visible_spectrum` applies `WinampPlayer::visualizer_visible`,
`Message::ToggleVisualizer` and `handlers::playback::toggle_visualizer` flip that flag,
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
- `visible_spectrum_hides_the_stored_bars_while_the_visualizer_is_off` (`src/ui/views/tests.rs`)
- `lcd_indicator_strip_publishes_always_on_top_on_a_and_visualizer_on_v`
  (`src/ui/views/tests.rs`)
- `toggle_visualizer_hides_and_shows_the_bars_without_clearing_the_spectrum` (`src/ui/tests.rs`)
- `hidden_visualizer_draws_no_bars_in_the_root_view` (`src/ui/tests.rs`)
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
`audio::player::rodio_player::RodioPlayer::is_finished` in `src/audio/player/rodio_player.rs`
answers true only once the player holds a preview's format and `rodio::Player::empty` reports the
queued source has drained; `RodioOutput::is_finished` bounds the worker's `Command::Finished` reply
like the other backend queries, mapping a missing reply to false.
`AppleMusicService::preview_finished` in `src/apple_music/playback/output.rs` forwards the backend's
answer without touching shared state. `handlers::playback::preview::position_tick` schedules the
finished query, `Message::PlaybackFinished` carries its reply, and
`handlers::playback::preview::playback_finished` advances the transport;
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
