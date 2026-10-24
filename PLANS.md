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

### Clear the library search from a button beside the search box (planned 2026-10-23 by the plan role)

**Goal.** The `Search library` box stores its typed text and searches on Enter; `Back` leaves an
active search, `Reload` clears the active flag but keeps the typed query, and a blank submit is
deliberately a no-op. There is no one-press way to empty the box. Add a `Clear` button beside the
search box on every normal-mode browse level: it empties the typed query and, when a search is
showing, leaves that search exactly as `Back` does (returns to Artists, restores the artists
list's library order, and snaps the list to its top). With no search active it changes only the
box text. It schedules no fetch and touches no playback. Shade mode builds no search row, so it
has no Clear button.

**Approach.**

- `src/ui/message.rs`: add `ClearSearch` after `FocusSearch`, documented as the Clear button's
  payload-free message: empty the box's query and, when a search is active, leave it exactly as
  Back does; schedule no fetch or playback.
- `src/ui/handlers/browse.rs`: add
  `pub(in crate::ui) fn clear_search(player: &mut WinampPlayer) -> Task<Message>` that clears
  `player.search_query` and, only when `player.search_active` is set, returns `back(player)`;
  otherwise it returns `Task::none()`. Document that reusing `back` is what makes the active case
  identical to the Back button (deactivate the search, switch to Artists,
  `restore_library_order`, `scroll_browse_to_top`) and that the inactive case mutates only the
  query. It must not refetch or clear any browse rows.
- `src/ui/mod.rs`: route `Message::ClearSearch` to `handlers::browse::clear_search` beside the
  `FocusSearch` arm.
- `src/ui/views/browse.rs`: add
  `pub fn view_clear_search_button() -> Element<'static, Message>` built with
  `labeled_button("Clear", Message::ClearSearch)` like `view_top_button`, and add
  `pub fn view_search_row(query: &str) -> Element<'_, Message>` returning a `Row` holding the
  existing `view_search_box(query)` with `Length::Fill` followed by
  `view_clear_search_button()`, so the box keeps the width it had and Clear follows it. Update
  the module and search-box docs to name the Clear button; `view_search_box` keeps its current
  callbacks and id.
- `src/ui/views/mod.rs`: re-export `view_clear_search_button` and `view_search_row` beside
  `view_search_box`.
- `src/ui/view.rs`: replace the `views::view_search_box(&player.search_query)` push with
  `views::view_search_row(&player.search_query)` and update the comment to say the row carries
  Clear.
- Tests. `src/ui/tests.rs`: add
  `clear_search_without_an_active_search_clears_only_the_query` (seed a query, a non-Artists
  view, and rows; drive `Message::ClearSearch`; assert with `assert_message_schedules_no_work`
  that it schedules nothing, then that the query is empty while the view, rows, selection,
  `search_active`, and playback are unchanged), and
  `clear_search_while_a_search_is_active_returns_to_artists_and_clears_the_query` (set
  `search_active`, `CurrentView::Songs`, a query, and search-result songs; drive the message;
  assert the query is empty, the flag is false, and the view is Artists, and use
  `assert_task_snaps_browse_list_to_top` to pin the same snap-to-top task Back schedules, while
  the search-result rows stay in the songs buffer). Add
  `clear_search_with_an_empty_query_schedules_no_work` for the empty inactive case.
  `src/ui/views/tests.rs`: add `view_clear_search_button_constructs` and
  `view_search_row_draws_clear_after_the_search_box` (lay out
  `view_search_row("Opening")` with the headless renderer; capture the `TextInput` bounds through
  a small `Operation::text_input` implementation, as `view_search_box_registers_its_focus_id`
  captures `Operation::focusable`, and capture the `Clear` label with
  `layout_capture::capture_text`; assert `Clear`'s left edge is at or after the text input's
  right edge and the two vertical ranges overlap), and extend the existing browse-action
  construct test with the new builders.
- `README.md`: in the sentence documenting the search box, say a `Clear` button beside the box
  empties the typed query; when a search is showing it also returns to the artists list like
  Back, and otherwise it changes only the text. Do not edit the `<!-- tumwater:status:start -->`
  block.
- `docs/invariants.md`: add a short `## Library search clearing` section after `## Search
  focus`, stating the guarantee above; name `Message::ClearSearch`,
  `handlers::browse::clear_search`, `handlers::browse::back`,
  `views::browse::view_clear_search_button`, and `views::browse::view_search_row` as the
  enforcers; list the new test names under **Pinned by**; and record the addition in
  **History**.

**Files touched.** `src/ui/message.rs`, `src/ui/handlers/browse.rs`, `src/ui/mod.rs`,
`src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/view.rs`, `src/ui/tests.rs`,
`src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The `Clear` button renders beside the search box on every normal-mode browse level; shade mode
  builds neither the box nor the button.
- With no active search, pressing it empties `search_query` and changes nothing else: no task,
  fetch, scroll, view or row change, selection change, or playback change.
- With a search active, pressing it empties the query, deactivates the search, returns to the
  Artists view, restores the artists list's library order, and snaps the browse list to its top,
  exactly as Back does; it schedules no fetch or playback and leaves the search-result rows in
  the songs buffer.
- Pressing it with an empty query and no active search is a no-op that schedules no work.
- The new tests pass, `make check` passes, and README and `docs/invariants.md` describe the button
  and the two cases.

### Match album titles in the library search (planned 2026-10-23 by the plan role)

**Goal.** The built-in sample library's search currently matches a song only when the query
appears in its title or artist, so typing an album title that the browse lists show (for example
`Second Record`) returns nothing. Extend `sample_songs_matching` to also match the title of the
album a song belongs to, case-insensitively, while keeping the existing title and artist matches
and the library-order result. This makes the offline search find an album's songs by the name the
user sees; the signed-in Apple Music path keeps returning whatever the REST search service
decides.

**Approach.**

- `src/apple_music/sample.rs`: add a small private helper
  `sample_album_titles_by_id() -> std::collections::HashMap<&'static str, String>` that walks
  `sample_albums_in_library_order()` and maps each `Album::id` to its case-folded
  `Album::title`. In `sample_songs_matching`, build the map once per call and keep a song when
  its title, its artist, or its album's folded title contains the folded query; keep the
  existing `sample_songs_in_library_order()` walk and clone, so the result order does not change.
  Update the helper's doc comment to name title, artist, and album title.
- `src/apple_music/browse.rs`: update `AppleMusicService::search_songs`'s doc comment (and the
  `sample_songs_matching` description beside it) to say the sample fallback matches title,
  artist, or album title. No logic changes: the blank-query guard, trimming, and the signed-in
  REST branch stay as they are.
- `src/apple_music/tests.rs`: add
  `search_songs_matches_the_sample_library_by_album_title`, using `test_service()` and
  `assert_ids`. Assert `"  SECOND RECORD  "` returns exactly `song-4` (the only song on
  `Second Record`), and `"first record"` returns `song-1`, `song-2`, and `song-3` in library
  order. Update the comment above
  `search_songs_matches_the_sample_library_by_title_or_artist` so it says this test pins the
  title and artist matches while the new test pins the album-title match; keep its assertions
  unchanged.
- `README.md`: in the search-box sentence, add that the sample fallback matches the query
  case-insensitively against a song's title, artist, and album title. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a short `## Library search matching` section after `## Search
  focus`. State that a sample-library search returns every song whose title, artist, or album
  title contains the trimmed query case-insensitively, in library order; that a blank query is
  rejected; and that the signed-in path returns the REST service's results. Name
  `sample_songs_matching` and `sample_album_titles_by_id` in `src/apple_music/sample.rs` and
  `AppleMusicService::search_songs` in `src/apple_music/browse.rs` as the enforcers, list the
  title/artist/album tests and the blank-query test under **Pinned by**, and record the addition
  in **History**.

**Files touched.** `src/apple_music/sample.rs`, `src/apple_music/browse.rs`,
`src/apple_music/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- With no session, `search_songs` matches a sample song when the query appears case-insensitively
  in its title, its artist, or the title of the album named by its `album_id`.
- `"Second Record"`, `"second record"`, and the trimmed query all return exactly `song-4`;
  `"first record"` returns `song-1`, `song-2`, and `song-3` in library order; title and artist
  matches are unchanged.
- A query matching no title, artist, or album title still returns an empty list, and a blank
  query is still rejected with the existing error.
- No new message, handler, state, dependency, or UI change is introduced; the signed-in REST
  search path is untouched.
- The new test passes, `make check` passes, and README and `docs/invariants.md` describe the
  album-title match.

## Done
