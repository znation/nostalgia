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

### Scroll the browse list one page with Page Up and Page Down (planned 2026-10-23 by the plan role)

**Goal.** The browse list scrolls with the mouse wheel, snaps to its ends with `Home`/`End`, and
tracks the playing row while stepping, but the keyboard cannot page through a long list. Bind
plain or Shift-held `Page Down` to scroll the showing browse list forward by one page, and
`Page Up` to scroll it back by one page, on every normal-mode browse level. A page is a fixed
`BROWSE_PAGE_HEIGHT`, defined as eight `BROWSE_ROW_HEIGHT` rows, so the step does not depend on
the viewport size. Both keys are view-scroll operations only: they change no view, rows or row
order, selection, typed query, search-active flag, or playback, and schedule no fetch or
playback task. In shade mode no browse list exists, so both are no-ops; iced clamps the result,
so paging past either end leaves the list at that end and a short list stays put.

**Approach.**

- `src/ui/views/browse/rows.rs`: add
  `pub const BROWSE_PAGE_HEIGHT: f32 = 8.0 * BROWSE_ROW_HEIGHT;` beside `BROWSE_ROW_HEIGHT`, with a
  doc comment saying one keyboard page is eight rows of the same pitch, that the page keys scroll
  by this amount rather than by the viewport height, and that `iced` clamps the result. Extend the
  `BROWSE_LIST_ID` doc to name the page-scroll operation beside the snap-to-top and scroll-to-row
  operations.
- `src/ui/views/browse.rs`: re-export `BROWSE_PAGE_HEIGHT` from `rows` beside the existing
  `BROWSE_ROW_HEIGHT`.
- `src/ui/views/mod.rs`: include `BROWSE_PAGE_HEIGHT` in the `pub use browse::{...}` list.
- `src/ui/message.rs`: add `ScrollBrowsePageUp` and `ScrollBrowsePageDown` beside
  `ScrollBrowseToTop`/`ScrollBrowseToBottom`, documented as `Page Up`/`Page Down` scheduling a
  one-page `scroll_by` on the browse list and changing no state.
- `src/ui/handlers/browse.rs`: add a private
  `fn scroll_browse_by(delta: f32) -> Task<Message>` that calls
  `iced::widget::operation::scroll_by(views::BROWSE_LIST_ID, AbsoluteOffset { x: 0.0, y: delta })`,
  then `pub(in crate::ui) fn scroll_browse_page_up() -> Task<Message>` calling it with
  `-views::BROWSE_PAGE_HEIGHT` and `scroll_browse_page_down()` calling it with
  `views::BROWSE_PAGE_HEIGHT`. Document that the page step follows the list's current offset
  (unlike the absolute `scroll_browse_to_row`), touches only the scroll offset, and has no target
  in shade mode.
- `src/ui/mod.rs`: route `Message::ScrollBrowsePageUp` and `Message::ScrollBrowsePageDown` to those
  handlers beside the existing `ScrollBrowseToTop`/`ScrollBrowseToBottom` arms.
- `src/ui/shortcuts.rs`: map `Key::Named(key::Named::PageDown)` to
  `Message::ScrollBrowsePageDown` and `Key::Named(key::Named::PageUp)` to
  `Message::ScrollBrowsePageUp` beside the Home/End arms. The existing Control/Alt/Logo guard
  stays first, so chords with Page Up or Page Down remain unbound. Update the `message_for` and
  `shortcut` docs to name the page keys.
- Tests. `src/ui/shortcuts.rs`: add `message_for_maps_page_keys_to_browse_paging` covering plain
  and Shift-held Page Up and Page Down, and extend `shortcut_ignores_chord_modifiers`'s key list
  with both page keys so Ctrl/Alt/Logo chords return `None`. `src/ui/tests.rs`: extend
  `BrowseScrollEffects` with `scrolled_by: Option<AbsoluteOffset>` (the non-optional form
  `operation::scroll_by` takes), record it in the `ScrollableOperation::scroll_by` capture, and
  include it in `any()`; make the existing snap and scroll-to helpers also assert
  `scrolled_by == None` so an arm cannot schedule two scroll kinds. Add
  `assert_task_scrolls_browse_list_by(task, what, expected_y)` that captures the operation and
  asserts `x == 0.0`, `y == expected_y`, and no snap or scroll-to effect, then add
  `scroll_browse_page_keys_schedule_page_sized_scrolls`: drive `Message::ScrollBrowsePageDown`
  and assert `+BROWSE_PAGE_HEIGHT`, drive `Message::ScrollBrowsePageUp` and assert
  `-BROWSE_PAGE_HEIGHT`, and assert the view and shared playback state are unchanged. Import
  `BROWSE_PAGE_HEIGHT` from `super::views` in the test module.
- `README.md`: in the Usage keyboard sentence, after the Home/End clause, say `Page Up` and
  `Page Down` scroll the browse list one page at a time in the matching direction. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: extend the existing `## Browse keyboard scrolling` section (rather than
  adding a duplicate section) so its guarantee also covers plain or Shift-held Page Up/Page Down
  scrolling the showing browse list by `BROWSE_PAGE_HEIGHT` through iced's `scroll_by` on
  `BROWSE_LIST_ID`, with the view, rows, selection, search state, and playback untouched, no fetch
  or playback task scheduled, no target in shade mode, and iced clamping at the ends. Name
  `shortcuts::shortcut`, the two new `Message` variants, the three `handlers::browse` scroll
  functions, `views::BROWSE_PAGE_HEIGHT`, and `views::BROWSE_LIST_ID` as the enforcers; add the new
  shortcut and UI test names to **Pinned by**; record the addition in **History**.

**Files touched.** `src/ui/views/browse/rows.rs`, `src/ui/views/browse.rs`,
`src/ui/views/mod.rs`, `src/ui/message.rs`, `src/ui/handlers/browse.rs`, `src/ui/mod.rs`,
`src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Plain or Shift-held `Page Down` schedules exactly one `scroll_by` on `BROWSE_LIST_ID` with
  `y = +BROWSE_PAGE_HEIGHT`, and `Page Up` one with `y = -BROWSE_PAGE_HEIGHT`; the step is
  `8.0 * BROWSE_ROW_HEIGHT` and is relative to the list's current offset.
- Neither key changes the current view, rows or row order, selection, typed query, search-active
  flag, or playback, and neither schedules a fetch or playback task; in shade mode both are no-ops,
  and Ctrl/Alt/Logo chords with either key remain unbound.
- Paging past either end leaves the list at that end (iced clamps the offset), and a list shorter
  than the viewport does not move.
- The new tests pass, `make check` passes, and README and `docs/invariants.md` describe the page
  keys and their bounds.

## Done
