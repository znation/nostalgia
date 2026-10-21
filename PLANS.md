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

### Add a Playlists browse level backed by the sample library (Playlists browse, part 1/2: planned 2026-10-20 by the plan role)

**Goal.** A `Playlists` button on the Artists view opens a flat list of the user's library playlists; pressing a playlist opens its songs in the existing Songs view, and those rows play exactly like any other song row. Back from the songs returns to Playlists, and Back from Playlists returns to Artists. The Playlists level is a new top-level browse sibling of the All Songs and All Albums flat lists. This first half answers the two new service queries from the sample library, the same sample-first split "Add a library search box to the playlist editor" (done) used before its REST sibling landed; the sibling "Back the playlists browse with the Apple Music REST endpoints" (Playlists browse, part 2/2) points the same two methods at the signed-in library. Until that sibling lands, a signed-in service also answers playlists from the sample library — documented in the method docs, not left implicit.

**Approach.**

- `src/library.rs`: add `pub struct Playlist { pub id: String, pub name: String }`, deriving `Debug, Clone, PartialEq, Serialize, Deserialize` like `Artist`, and extend the module doc's model list. Add tests mirroring the `Artist` pins: a hand-written `playlist_payload()` (`{"id":"playlist-1","name":"Morning Drive"}`), serialized-field-name/round-trip, unknown-field tolerance, and `assert_every_field_required`.
- `src/test_support.rs`: add `sample_playlist()` beside `sample_artist`/`sample_album`/`sample_song`, carrying id `playlist-1` and name `Morning Drive`, for the `library` and `apple_music` suites.
- `src/sample_library.rs`: add `pub playlists: Vec<Playlist>` and `pub songs_by_playlist: HashMap<String, Vec<Song>>` to `SampleLibrary`. Build three playlists in render order — `playlist-1` "Morning Drive", `playlist-2` "Focus", `playlist-3` "Sample Favorites" — and index their songs from the existing sample songs, preserving each playlist's named order (`playlist-1` → `song-1`, `song-4`; `playlist-2` → `song-2`, `song-5`; `playlist-3` → `song-1`, `song-2`, `song-3`). Add tests mirroring `albums_are_indexed_by_artist_in_library_order_with_no_empty_groups`: playlists in order, three non-empty groups, and each group's song ids in the documented order. Update the module doc's browse-query list.
- `src/apple_music.rs`: add `pub async fn get_playlists(&self) -> Result<Vec<Playlist>, AppleMusicError>` returning `sample_library().playlists.clone()`, and `pub async fn get_songs_from_playlist(&self, playlist_id: &str) -> Result<Vec<Song>, AppleMusicError>` calling `ensure_id_is_valid(playlist_id, IdKind::Playlist)` then `lookup(&sample_library().songs_by_playlist, playlist_id)`. Add `IdKind::Playlist` and its `"playlist"` `Display` arm. Document on both methods that they answer from the sample library even when a session is stored, and that the REST sibling rewires them through `browse`; the async signatures and id guard already match that end state.
- `src/ui/mod.rs`: add `Playlist` to the `use crate::library::{Album, Artist, Song}` import, `CurrentView::Playlists`, `Message::ShowPlaylists`, `Message::PlaylistsLoaded(Vec<Playlist>)`, `Message::PlaylistsLoadFailed(String)`, and `Message::PlaylistSelected { epoch, index }`; add `playlists: BrowseList<Playlist>` to `WinampPlayer`, built with `BrowseList::new(false)` in `WinampPlayer::new`; add `SongsOrigin::Playlist`. The `ShowPlaylists` arm clears `search_active`, sets `CurrentView::Playlists`, and calls `fetch_level` on `player.playlists` with report `"loading library playlists"` and `|service| async move { service.get_playlists().await }`. The `PlaylistSelected` arm mirrors `AlbumSelected`: resolve through `player.playlists.select(epoch, index)`, clone the id, set `songs_origin = SongsOrigin::Playlist`, switch to `CurrentView::Songs`, clear any active search, and `fetch_level` the songs with `"loading songs from playlist {playlist_id:?}"` and `get_songs_from_playlist`. The `Back` arm's `CurrentView::Songs` match adds `SongsOrigin::Playlist => CurrentView::Playlists`, and `CurrentView::Playlists => CurrentView::Artists`. Store/fail through `player.playlists` in the loaded/failed arms, and map the two new reply messages to `player.playlists.is_current(issued)` in the `BrowseReply` arm.
- `src/ui/views/browse.rs`: add `playlist_row` (name, `"View Songs"`, `Message::PlaylistSelected`, never current) and `view_playlists` built through `browse_view(CurrentView::Playlists, …)`; add `view_playlists_button` returning `labeled_button("Playlists", Message::ShowPlaylists)`; add `CurrentView::Playlists => "No playlists"` to `empty_list_label` and `Playlists` to `BROWSE_VIEWS` (six entries).
- `src/ui/views/mod.rs`: re-export `view_playlists` and `view_playlists_button`.
- `src/ui/mod.rs`'s `view`: add the `CurrentView::Playlists` browse arm rendering `views::view_playlists(&player.playlists.items, player.playlists.epoch, player.playlists.loading, player.playlists.error.as_deref())`, and push `views::view_playlists_button()` beside the All Songs and All Albums buttons on the Artists view.
- `docs/invariants.md`: add `Playlists → Artists` and `Songs → Playlists when the songs were opened from a playlist` to the Browse hierarchy guarantee, and name `back_from_playlists_returns_to_the_artists_list` and `playlist_selected_in_playlists_loads_its_songs_and_back_returns_to_playlists` under "Pinned by".
- `README.md`: in the usage paragraph, describe the `Playlists` button and its Back path, and state that playlist data comes from the sample library until the REST endpoints land (the sibling plan removes that caveat).
- Tests: `src/apple_music/tests.rs` gains `get_playlists_returns_the_sample_playlists_in_order`, `get_songs_from_playlist_returns_its_sample_songs_in_order`, `get_songs_from_playlist_rejects_a_blank_id` (empty, whitespace-only, control character), and `get_songs_from_playlist_returns_empty_for_an_unknown_playlist`; the session variants pin the documented sample-only behavior so the sibling's change is deliberate. `src/ui/tests.rs` gains `show_playlists_fetches_and_switches_view`, `playlists_loaded_stores_the_list`, `playlist_selected_in_playlists_loads_its_songs_and_back_returns_to_playlists`, `back_from_playlists_returns_to_the_artists_list`, and `show_playlists_clears_an_active_search`, and extends `seed_browse_lists`, `assert_browse_lists_empty`, `each_browse_failure_stores_its_report_in_its_own_list`, and `view_constructs_over_the_apps_full_input_space` to cover `playlists`. `src/ui/views/tests.rs` pins `playlist_row`, `view_playlists_button`, the `empty_list_label` arm, and the extended `BROWSE_VIEWS` loops.

**Files touched.** `src/library.rs`, `src/test_support.rs`, `src/sample_library.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `Playlist` serializes as exactly `{"id", "name"}`, round-trips, tolerates an unknown field, and rejects a payload missing either field.
- The sample library exposes `playlist-1`/`playlist-2`/`playlist-3` in that order with the documented song ids in each `songs_by_playlist` group; no group is empty.
- `AppleMusicService::get_playlists` returns those three playlists in order with no session and with a session; `get_songs_from_playlist("playlist-1")` returns `song-1` then `song-4`; an unknown id returns an empty list; a blank, whitespace-only, or control-character id returns an error naming `playlist`.
- Pressing `Playlists` clears an active search, switches to `CurrentView::Playlists`, and fetches into `playlists`; a loaded reply stores the rows and a failed reply records the report on `playlists` alone.
- Pressing a playlist row fetches that playlist's songs into `songs` and shows `CurrentView::Songs`; Back returns to `CurrentView::Playlists`, while Back from playlist songs reached through any other origin still returns to Albums or All Albums as before.
- A stale playlist press (wrong epoch or out-of-range index) issues no fetch and leaves the view unchanged.
- The Artists view shows the `Playlists` button, and all six browse levels assemble, including empty and failed `playlists`, without panicking.

### Back the playlists browse with the Apple Music REST endpoints (Playlists browse, part 2/2: planned 2026-10-20 by the plan role, requires part 1/2 landed)

**Goal.** Point the part-1 playlist queries at the signed-in user's Apple Music library, so a stored session shows that user's real playlists and each playlist's real tracks instead of the sample data, while a session-less service keeps its sample fallback. This is the second half of the playlist browse, the twin of "Back the library search with the Apple Music REST endpoint" (done); land it only once the Playlists view from part 1/2 is the running build.

**Approach.**

- `src/apple_music/rest.rs`: add `Playlist` to the `use crate::library::{Album, Artist, Song}` import and the module doc's model list, then add `pub fn get_all_playlists(&self, session: &MusicKitSession) -> Result<Vec<Playlist>, AppleMusicError>` requesting `{API_BASE}/me/library/playlists` through `self.fetch` and mapping each resource with `Resource::into_playlist`; add `pub fn get_songs_from_playlist(&self, session: &MusicKitSession, playlist_id: &str) -> Result<Vec<Song>, AppleMusicError>` requesting `{API_BASE}/me/library/playlists/{}/tracks` with `encode_path_segment(playlist_id)` and mapping with `Resource::into_song("")` — a library playlist track carries no album id, the same shape `get_all_songs` gives its songs. Add `Resource::into_playlist`, which moves the resource's `id` and `attributes.name` into the model through `required_id(id, "playlist")` and `required_name(…, &id, "playlist")`, exactly as `into_album` does for albums. Document the endpoint, the empty `album_id`, and the id encoding on both methods.
- `src/apple_music.rs`: add `Playlist` to the `use crate::library::{Album, Artist, Song}` import, rewrite `get_playlists` as `self.browse(|rest, session| rest.get_all_playlists(session), sample_all_playlists)` and `get_songs_from_playlist` as `self.browse(move |rest, session| rest.get_songs_from_playlist(session, &id), || lookup(&sample_library().songs_by_playlist, playlist_id))` after the existing `ensure_id_is_valid` guard, keeping the guard's error contract. Add `fn sample_all_playlists() -> Vec<Playlist>` cloning `sample_library().playlists`, and remove the part-1 doc caveat that the queries ignore a stored session.
- `README.md`: remove the "playlist data comes from the sample library until the REST endpoints land" caveat, leaving playlists described alongside the other signed-in browse queries.
- Tests: `src/apple_music/rest/tests.rs` gains `get_all_playlists_maps_the_library_playlists_collection` (pin `/v1/me/library/playlists` and the mapped id/name), `get_all_playlists_follows_a_next_page`, `get_all_playlists_reports_a_transport_error_bare`, `get_all_playlists_rejects_a_nameless_playlist`, `get_songs_from_playlist_maps_the_playlist_tracks_collection` (pin the id-scoped path and empty `album_id`), `get_songs_from_playlist_percent_encodes_the_playlist_id`, and `get_songs_from_playlist_reports_a_transport_error_bare`. `src/apple_music/tests.rs` adds `get_playlists_uses_the_rest_library_when_signed_in` and `get_songs_from_playlist_uses_the_rest_library_when_signed_in` over the stub transport, and keeps the existing no-session sample tests green (updating only the two part-1 tests that pinned the sample-only signed-in behavior).

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary::get_all_playlists` requests `/v1/me/library/playlists`, maps each resource's id and name, and follows a same-origin `next` page; a nameless playlist is an error naming its id and a transport failure propagates its cause unchanged.
- `RestLibrary::get_songs_from_playlist` requests `/v1/me/library/playlists/{encoded-id}/tracks`, maps songs with an empty `album_id`, percent-encodes a reserved character in the id, and propagates a transport failure.
- With a stored session, `AppleMusicService::get_playlists` and `get_songs_from_playlist` return the REST results; with no session they return the sample playlists and playlist songs in the part-1 order.
- A blank, whitespace-only, or control-character playlist id is still rejected by the service before any REST call is issued.

## Done
