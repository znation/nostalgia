# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; updated 2026-10-07).** The initial prompt
> names two commitments — a "very close if not pixel-perfect" classic Winamp UI
> and Apple Music as the library. The first is now moving: the base-skin
> palette landed ("Add a Winamp 2.x base-skin palette and apply it as the app
> theme", done 2026-10-06), and the reusable bevel layer followed ("Add a
> Winamp two-tone bevel layer and frame the Now Playing and equalizer panels",
> done 2026-10-06), framing those two panels through `bevel::lcd_well` and
> `bevel::raised_panel` in `src/ui/bevel.rs`. `## Planned` is no longer empty:
> it holds "Style the transport buttons as raised Winamp chrome", which
> restyles the five transport buttons and the browse Back button — all built
> through `labeled_button`, still a default iced `Button`. Still unplanned is
> the rest of widget-level fidelity: slider chrome (the bevel entry and the
> transport plan both defer it), a custom title bar, and playlist chrome. The
> library side is still all stub: `AppleMusicService` answers every browse
> query (`get_favorite_artists`, `get_albums_by_artist`,
> `get_songs_from_album`) from `sample_library()`, `play_track` only mutates
> shared state, and the token field is unset — and no PLANS.md entry plans the
> real integration. Risk: the longer Apple Music stays stubbed the more UI
> grows around the sample library's shape, while the remaining chrome work is
> easier to sequence now that the bevel layer exists. Recommend the plan loop
> keep scheduling the remaining fidelity plans (slider chrome, title bar,
> playlist chrome) and put the Apple Music integration plan on the near
> horizon.

## Planned

### Add an All Albums view listing the whole library flat (planned 2026-10-20 by the plan role)

**Goal.** A new top-level `All Albums` browse view lists every album in the user's library in one flat list, in library order. Pressing an album opens its songs exactly as the per-artist Albums view does, and Back from those songs returns to All Albums rather than to the artist list. It is reached by an `All Albums` button on the Artists view, beside `All Songs`, and Back from All Albums returns to Artists. Today albums are reachable only by drilling Artists → Albums, so there is no whole-library album view. The REST half reads `/v1/me/library/albums` through the same paginated collection helper as the other browse queries; the sample half walks the sample library's artists → albums in library order. Like `get_all_songs`, an all-albums resource has no artist id available in the collection mapping, so `Album::artist_id` is left empty; nothing that opens an all-albums row reads it (`get_songs_from_album` uses only the album id). Independent of the All Songs and search work, so it can land on its own.

**Approach.**

- `src/apple_music/rest.rs`:
  - Add `pub fn get_all_albums(&self, session: &MusicKitSession) -> Result<Vec<Album>, AppleMusicError>` requesting `{API_BASE}/me/library/albums` through `self.fetch`. Map each resource with `required_id("album")` and `required_name("album")`, and set `artist_id: String::new()`; document why the artist id is empty, as `get_all_songs` documents its empty `album_id`.
- `src/apple_music.rs`:
  - Add `pub async fn get_all_albums(&self) -> Result<Vec<Album>, AppleMusicError>` through the existing `browse` helper: `self.browse(|rest, session| rest.get_all_albums(session), sample_all_albums)`.
  - Add a private `fn sample_albums_in_library_order() -> impl Iterator<Item = &'static Album>` walking `sample_library().artists` then `albums_by_artist`, and make `sample_songs_in_library_order` start from that same album walk so the two orders cannot drift. Add `fn sample_all_albums() -> Vec<Album>` from the album walk.
- `src/ui/mod.rs`:
  - Add `CurrentView::AllAlbums`, `Message::ShowAllAlbums`, `Message::AllAlbumsLoaded(Vec<Album>)`, and `Message::AllAlbumsLoadFailed(String)`.
  - Add `all_albums: BrowseList<Album>` to `WinampPlayer`, initialised `BrowseList::new(false)` in `WinampPlayer::new`.
  - Add a `SongsOrigin { Album, AllAlbums }` enum and a `songs_origin: SongsOrigin` field initialised to `SongsOrigin::Album`. The `AlbumSelected` arm must resolve the pressed row against `player.all_albums` when `player.current_view == CurrentView::AllAlbums` and against `player.albums` otherwise, clone the selected album id, then set `songs_origin` from that same view before moving to `CurrentView::Songs`. The `Back` arm's `CurrentView::Songs` match uses `songs_origin` to return to `AllAlbums` or `Albums`; `AllAlbums` on Back returns to `Artists`.
  - `Message::ShowAllAlbums` arm: clear `search_active`, set `current_view = CurrentView::AllAlbums`, and call `fetch_level` on `player.all_albums` with the report `"loading all library albums"`, `|service| async move { service.get_all_albums().await }`, and the two new messages.
  - `AllAlbumsLoaded`/`AllAlbumsLoadFailed` store/fail through `player.all_albums`; the `BrowseReply` arm re-checks `player.all_albums.is_current(issued)` beside the existing lists.
  - `view`'s browse match gains a `CurrentView::AllAlbums` arm rendering `views::view_all_albums(&player.all_albums.items, …)` (no current-track argument, like the Albums arm), and pushes `views::view_all_albums_button()` on the Artists view beside `view_all_songs_button`.
- `src/ui/views.rs`:
  - Add `pub fn view_all_albums(albums, epoch, loading, error)` built by `browse_view(CurrentView::AllAlbums, …, false)` from `album_row`, and `pub fn view_all_albums_button()` returning `labeled_button("All Albums", Message::ShowAllAlbums)`.
  - Add `CurrentView::AllAlbums => "No albums"` to `empty_list_label` and `CurrentView::AllAlbums` to `BROWSE_VIEWS`.
- Tests:
  - `src/apple_music/rest/tests.rs`: `get_all_albums_maps_the_library_albums_collection` (pin `/v1/me/library/albums`, the mapped id/title, and the empty `artist_id`); `get_all_albums_follows_a_next_page`; `get_all_albums_reports_a_transport_error_bare`; `get_all_albums_rejects_a_nameless_album`.
  - `src/apple_music/tests.rs`: `get_all_albums_returns_every_sample_album_in_library_order` (album-1, album-2, album-3); `get_all_albums_uses_the_rest_library_when_signed_in`.
  - `src/ui/tests.rs`: `show_all_albums_fetches_the_flat_list_and_switches_view`; `all_albums_loaded_stores_the_flat_list`; `back_from_all_albums_returns_to_the_artists_list`; `show_all_albums_clears_an_active_search`; `album_selected_in_all_albums_loads_its_songs_and_back_returns_to_all_albums` (proves the press resolves against `all_albums` and the Back origin). Extend `seed_browse_lists`, `assert_browse_lists_empty`, `each_browse_failure_stores_its_report_in_its_own_list`, and `view_constructs_over_the_apps_full_input_space`'s failure shape to cover `all_albums`.
  - `src/ui/views.rs`'s suite: a `view_all_albums_button` construction check, the `empty_list_label` arm, and the extended `BROWSE_VIEWS` loops.

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views.rs`, `src/ui/tests.rs`.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary::get_all_albums` requests `/v1/me/library/albums`, maps each resource's id and title, leaves `artist_id` empty, and follows a same-origin `next` page; a transport failure propagates its cause and a nameless album is an error naming its id.
- With no session, `AppleMusicService::get_all_albums` returns every sample-library album in library order (album-1, album-2, album-3); with a session it returns the REST collection.
- Pressing `All Albums` on the Artists view clears the buffer, clears an active search, switches to `CurrentView::AllAlbums`, and fetches; the loaded reply stores rows and a failed reply records the report on `all_albums` alone.
- Back from `CurrentView::AllAlbums` returns to `CurrentView::Artists`.
- Pressing an album row in `CurrentView::AllAlbums` fetches that album's songs into `songs` and shows `CurrentView::Songs`; Back then returns to `CurrentView::AllAlbums`, while Back from songs opened through the per-artist Albums view still returns to `CurrentView::Albums`.
- The Artists view shows the `All Albums` button and the view assembles every browse level, including an empty/failed `all_albums`, without panicking.

## Done
