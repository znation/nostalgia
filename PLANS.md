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

### Add the R and S keyboard bindings for Repeat and Shuffle (planned 2026-10-20 by the plan role)

**Goal.** Pressing R toggles Repeat and pressing S toggles Shuffle from the keyboard, matching classic Winamp, so both transport toggles are reachable without the mouse. Today `shortcuts::shortcut` maps Z/X/C/V/B and the arrow keys only: R and S fall through to `None`, even though `Message::ToggleRepeat`, `Message::ToggleShuffle`, and their `update` arms already exist.

**Approach.**

- `src/ui/shortcuts.rs`: add two arms to `shortcut` beside the existing letter arms — `r` returns `Message::ToggleRepeat` and `s` returns `Message::ToggleShuffle` — using the same `eq_ignore_ascii_case` guard as the other letters, so uppercase and Shift work and the existing Control/Alt/Logo rejection applies. Update the doc comments on `message_for` and `shortcut` to list the two new bindings.
- `README.md`: extend the usage paragraph's classic-key sentence after the V entry with "R toggles Repeat and S toggles Shuffle".
- Tests in `src/ui/shortcuts.rs`: extend the `assert_letter_bindings` helper so the existing `message_for_maps_each_bound_letter` and `shortcut_matches_letters_case_insensitively` tests also assert that `r`/`R` map to `Message::ToggleRepeat` and `s`/`S` map to `Message::ToggleShuffle`; add both keys to the key list in `shortcut_ignores_chord_modifiers`, so a Control, Alt, or Logo chord still maps neither.

**Files touched.** `src/ui/shortcuts.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `message_for` maps a pressed `Key::Character("r")` to `Message::ToggleRepeat` and `Key::Character("s")` to `Message::ToggleShuffle`, with or without Shift, and maps their uppercase forms the same way.
- Control, Alt, and Logo chords over R or S still map to `None`, and the existing unbound-key and key-release behavior is unchanged.
- The README's classic-key sentence lists R and S beside the existing transport and volume keys.

## Done
