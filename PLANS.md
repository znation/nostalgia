# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; updated 2026-10-20).** The initial prompt
> names two commitments — a "very close if not pixel-perfect" classic Winamp UI
> and Apple Music as the library. The first is now moving: the base-skin
> palette landed ("Add a Winamp 2.x base-skin palette and apply it as the app
> theme", done 2026-10-06), and the reusable bevel layer followed ("Add a
> Winamp two-tone bevel layer and frame the Now Playing and equalizer panels",
> done 2026-10-06), framing those two panels through `bevel::lcd_well` and
> `bevel::raised_panel` in `src/ui/bevel.rs`. The transport-button chrome plan
> landed ("Style the transport buttons as raised Winamp chrome", done
> 2026-10-07), and both transport toggles are reachable: the Repeat toggle
> landed 2026-10-06 and the Shuffle toggle 2026-10-08, and their R and S
> keyboard bindings followed ("Add the R and S keyboard bindings for Repeat
> and Shuffle", done 2026-10-20). The rest of widget-level fidelity has
> landed too: slider chrome ("Style the volume and
> equalizer sliders as sunken Winamp grooves with raised chrome thumbs", done
> 2026-10-07), a custom title bar ("Add a Winamp custom title bar and drop the
> OS window frame", done 2026-10-07), and playlist chrome ("Frame the browse
> list as a sunken Winamp playlist editor", done 2026-10-07). The library side
> is real:
> `AppleMusicService` answers its browse and search queries through the REST
> client once a `MusicKit` session is stored, falling back to
> `sample_library()` otherwise — playlist queries included — and `play_track`
> plays the selected preview through the audio seam. Risk: the Winamp chrome
> arrived piecemeal, so later changes still need one consistent vocabulary.
> The equalizer-audio and seek plans this note pointed at have since landed
> (both 2026-10-20); the 2026-10-22 note below carries the current queue.

> **Steward drift note (2026-10-22 by the steward role): the built Now
> Playing bar and browse rows do not match the brief's mockups, and the two
> queued plans settle that difference in opposite directions — the
> playback-position plan adds a readout the mockup shows nowhere, while the
> album-row plan matches the mockup's `<artist> - <title>` rows but leaves the
> running time it draws out of scope.**
>
> The brief promises a "very close if not pixel-perfect" Winamp, and the plan
> role has been citing `docs/screenshots/` as the spec — the landed plans for
> the scrolling title, the artist-prefixed rows, the `TOP` button and the list
> counts each name a mockup feature. The main window's mockup
> (`docs/screenshots/player.png`) draws the classic LCD: a vertical `O C I D V`
> strip at the left, a large elapsed-time readout (`00:53`) over a green
> visualizer, a song-title box reading `<ARTIST> - <TITLE> (<length>)`, and a
> `256 KBPS 44 KHZ` / `MONO STEREO` status row. The build renders one row
> instead: `now_playing_row` (`src/ui/views/now_playing.rs`) pushes a
> `Now Playing: ` caption, the artist and title, a fill, and the track length
> at the right edge of the LCD well, and `view` (`src/ui/mod.rs`) stacks that
> bar above the transport row — no elapsed-time readout, no `O C I D V` strip,
> no status row.
>
> The first entry under `## Planned`, "Add the playback-position readout to
> the Now Playing bar (Playback position, part 3/3)", widens that right-edge
> slot to `<elapsed>/<total>`, a value the mockup shows nowhere: its title
> line carries the length in parentheses and the elapsed time in the big left
> readout. Recommend the plan role schedule that elapsed-time display and
> settle where part 3/3's readout belongs before the queue drains.
>
> The browse rows drift the same way: `docs/screenshots/library.png` draws
> each artist row with its album count at the right (`2 albums`) and each
> album row with its running time (`12:38`), while the build shows the
> `View Albums` and `View Songs` hints (`src/ui/views/browse.rs`) and the
> total count beneath the rows. The queued album-row plan, "Show each album
> row's artist beside its title", puts album running times out of scope
> because an album query does not supply its songs' durations, so the
> mockup's per-album time would need a songs fetch per row — a decision for
> the plan role, not a detail to drop.

## Planned

_None yet._

### Sort the current browse list alphabetically from a Sort button (planned 2026-10-23 by the plan role)

**Goal.** Add the library window's `SORT LIB` button from `docs/screenshots/player.png` as a title-case `Sort` button above every browse level, beside `Top`. Pressing it reorders the list currently showing (Artists, Albums, Songs, All Songs, All Albums, or Playlists) alphabetically by each row's rendered text, case-insensitively, leaving every other list and the player state alone. The sort is one-shot like the classic button: it changes only the loaded list in place, so reopening the level fetches its library order again. No persistent sort mode or new dependency is needed.

**Approach.**

- `src/ui/browse.rs`: add `pub(super) fn sort_rows_by(&mut self, key: impl Fn(&T) -> String)` on `BrowseList<T>`, requiring `T: Clone` (the library models derive it). It builds an index permutation (`0..items.len()`) and stable-sorts it by each row's key lowercased with `to_lowercase()`; Rust's stable sort keeps library order for keys that compare equal. Rebuild `items` and, when they are non-empty, `titles` and `labels` by the same permutation so the cached song text stays index-aligned with the reordered songs (the artist/album/playlist lists leave both caches empty). After reordering, bump `epoch` with the same `wrapping_add(1)` `replace_rows` uses, since the row indices a press carries have changed; do not touch `loading`, `error`, or the request generation. A list with fewer than two rows is a no-op (no reorder, no epoch bump), so pressing Sort while a level is loading, empty, or failed changes nothing.
- `src/ui/handlers/browse.rs`: add `pub(in crate::ui) fn sort_browse(player: &mut WinampPlayer) -> Task<Message>`, matching `player.current_view` and calling `sort_rows_by` on that view's list: artists and playlists by `name`, albums (both the per-artist and All Albums lists) by `Album::display_title` (so `artist - title` is the sort key), and songs (both the Songs list, including search results, and All Songs) by `Song::display_title`. Return `Task::none()`; the sort changes no other state, and a reload after navigation restores the fetch's order.
- `src/ui/mod.rs`: add `SortBrowse` to `Message` with a doc comment naming the one-shot reorder and its epoch bump, route it in the `update` dispatch table to `handlers::browse::sort_browse`, and push `views::view_sort_button()` immediately after `views::view_top_button()` so it is present above every browse level.
- `src/ui/views/browse.rs`: add `pub fn view_sort_button() -> Element<'static, Message>` using `labeled_button("Sort", Message::SortBrowse)` beside `view_top_button`, with a doc comment saying it reorders the visible list on press. In `src/ui/views/mod.rs`, export `view_sort_button` and add it to the `labeled_button` doc comment's list of browse buttons.
- Tests in `src/ui/tests.rs` (the browse state and handler tests live there, not in `src/ui/browse.rs`):
  - `sort_reorders_artists_case_insensitively`: drive `Message::ArtistsLoaded` with local `Artist` rows named `"banana"`, `"Apple"`, `"cherry"`, press `Message::SortBrowse`, and assert the names read `Apple`, `banana`, `cherry` — the lowercase key proves the comparison ignores case rather than sorting uppercase first.
  - `sort_reorders_albums_by_their_display_title` and `sort_reorders_playlists_by_name`: load local rows out of order, sort, and assert the rendered display order.
  - `sort_reorders_songs_and_keeps_their_cached_text_aligned`: drive `Message::SongsLoaded(stepping_songs())` (library order One, Two, Three), sort, and assert the ids order One, Three, Two while `songs.titles` and `songs.labels` reorder with them (`"The Sample Band - One"`/`"3:30"`, `"The Sample Band - Three"`/`"1:35"`, `"The Sample Band - Two"`/`"2:05"` in the new order), so no row shows another track's length. Repeat the same assertion for `Message::AllSongsLoaded` over the `all_songs` list.
  - `sort_reorders_all_albums_by_their_display_title`: load local `Album` rows into `all_albums`, sort, and assert their `artist - title` order. This covers the sixth `CurrentView` list, so every browse level has a sort test.
  - `sort_bumps_the_epoch_so_a_pre_sort_press_is_rejected`: capture the artists list's epoch before sorting, sort, then drive `Message::ArtistSelected { epoch: old_epoch, index: 0 }` and assert `player.current_view` is still `CurrentView::Artists`; drive the same press with the new epoch and index and assert it switches to `CurrentView::Albums`. This pins that row indices and the epoch move together.
  - `sort_with_an_empty_or_loading_list_is_a_noop`: press `Message::SortBrowse` on each view before any rows load and assert no panic, no epoch change, and the view stays where it was.
- `README.md`: add to the Usage paragraph that describes the browse lists that a `Sort` button above every list reorders the showing list alphabetically by its row text, case-insensitively, and that reopening the list restores library order. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Browse row order` section after `## Browse reply freshness` stating that Sort reorders only the visible list by its rendered text, keeps equal keys in library order, reorders the cached song titles and labels with the songs, bumps the epoch so a pre-sort press is rejected, and leaves the other lists and playback state unchanged; name `BrowseList::sort_rows_by` and `browse::sort_browse` as the enforcers and cite the new tests.

**Files touched.** `src/ui/browse.rs`, `src/ui/handlers/browse.rs`, `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A `Sort` button is drawn above every browse level beside `Top`; pressing it leaves the current view, current track, playback, search state, and every other list unchanged.
- The showing list is reordered ascending by its rows' rendered text, compared case-insensitively, for Artists, Albums, Songs, All Songs, All Albums, and Playlists; equal keys keep their library order.
- Song lists reorder `items`, `titles`, and `labels` by the same permutation, so each row still shows its own title and duration after a sort.
- A sort bumps the current list's epoch: a `TrackSelected`/`ArtistSelected`/`AlbumSelected`/`PlaylistSelected` press carrying the pre-sort epoch is ignored, while the post-sort indices resolve to the sorted rows.
- Pressing Sort on an empty, loading, or failed list is a no-op that neither reorders rows nor bumps the epoch.
- Navigating away and back reloads library order; no sort flag persists between fetches.
- `make check` passes (fmt, clippy, rustdoc, tests), and the README Usage text and the new `docs/invariants.md` section describe the sort in the same change.

## Done
