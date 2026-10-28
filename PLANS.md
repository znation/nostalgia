# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the usage reference: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. The README's detailed Usage reference moved to
> `docs/usage.md` on 2026-10-26, so plans that change usage prose edit that file, not the README.
> Record any new drift here rather than appending another note.

## Planned

_None yet._

### Reopen the library at the last top-level browse list across launches, like Winamp's Media Library (planned 2026-10-28 by the plan role)

**Goal.** Winamp's Media Library reopens on the node it last showed. Nostalgia always opens on
Artists, so a user who lives in All Songs or Playlists clicks through on every launch. Save which
top-level browse list was showing and reopen it at the next launch. Persist only the top-level
list, never a drilled-in album, playlist, or search: those ids can go stale between launches.

**Approach.**
1. New file `src/settings/library_view.rs` (declare it `pub mod library_view;` in
   `src/settings.rs`). It holds `pub enum LibraryView { Artists, AllSongs, AllAlbums, RecentlyAdded,
   RecentlyPlayed, Playlists }` (derive `Debug, Clone, Copy, PartialEq, Eq, Default`, with
   `#[default] Artists`), plus `fn key(self) -> &'static str` returning `artists`, `all_songs`,
   `all_albums`, `recently_added`, `recently_played`, `playlists`, and
   `fn from_key(&str) -> Option<Self>`, its exact inverse.
2. `src/settings.rs` `Settings`: add a `library_view: LibraryView` field, defaulting to `Artists`
   in `Default` and in `from_state`, with a `library_view()` getter and a `set_library_view` setter
   beside `window_position`. Update the struct doc comment's list of player-owned fields.
   `src/settings/text.rs`: `to_text` writes `library_view=<key>` on every save. `from_text` gets a
   `"library_view"` arm that sets the field when `LibraryView::from_key(value.trim())` is `Some` and
   otherwise keeps the default.
3. `src/ui/player.rs`: add `pub(in crate::ui) fn library_view(&self) -> LibraryView`, which maps the
   browse state to its top-level list. An active search (`search_active`) gives `Artists`, because
   Back from a search goes to Artists. `CurrentView::Artists` and `Albums` give `Artists`. Each flat
   view gives itself. `Songs` gives the list it came from by `songs_origin`: `Album` gives
   `Artists`, `AllAlbums` gives `AllAlbums`, `RecentlyAdded` gives `RecentlyAdded`, and `Playlist`
   gives `Playlists`. `persist_settings_if_changed` calls
   `snapshot.set_library_view(self.library_view())`. `adopt_saved_settings` leaves the view alone
   (the restore is a task, step 4). Update its doc comment.
4. `src/ui/handlers/browse.rs`: add `pub(in crate::ui) fn restore_view(player, view: LibraryView)
   -> Task<Message>`. It returns `Task::none()` for `Artists`. For the other views it returns the
   matching existing handler (`show_all_songs`, `show_all_albums`, `show_recently_added`,
   `show_recently_played`, `show_playlists`). In `src/ui/mod.rs` `init_ui`'s boot closure, after
   `adopt_saved_settings`, batch the boot task with `restore_view(&mut player,
   saved.library_view())`. `boot` keeps its `LoadArtists`, because Back from a restored list
   returns to Artists and needs those rows.
5. Signed-in launches. `boot` starts the first fetches before the background sign-in stores a
   session, so they answer from the sample library. Today only the artists list is refetched when
   the session arrives: `wait_for_session` maps to `Message::LoadArtists`. A restored All Songs
   list would keep its sample rows. Add `Message::SessionReady` in `src/ui/message.rs` and map
   `boot`'s `wait_for_session` future to it in place of `LoadArtists`. Route it in `update` to a new
   `handlers::browse::session_ready(player)`. That handler batches `fetch::load_artists(player)`
   with, when `!player.search_active` and `current_view` is one of the five flat views,
   `restore_view` for that view, which refetches it and snaps it to the top. Albums, Songs, and
   search results are left as they are today. Update the `wait_for_session` doc comment in
   `src/apple_music.rs`, which names `Message::LoadArtists`.

**Files touched.** `src/settings/library_view.rs` (new), `src/settings.rs`,
`src/settings/text.rs`, `src/ui/player.rs`, `src/ui/handlers/browse.rs`, `src/ui/mod.rs`,
`src/ui/message.rs`, `src/apple_music.rs` (doc comment only), `src/ui/tests/settings_tests.rs`,
`src/ui/tests/loading_tests.rs` with the `await_boot_artist_loads` helper in
`src/ui/tests/mod.rs`, `docs/usage.md`, and `docs/invariants.md`.

**Acceptance criteria.**
- `LibraryView::from_key(v.key()) == Some(v)` for each of the six variants, and
  `from_key("albums")` and `from_key("")` are `None`.
- A `Settings` with `set_library_view(LibraryView::Playlists)` round-trips through
  `to_text`/`from_text`. `from_text("library_view=bogus\n")` and text with no `library_view` line
  both give `LibraryView::Artists`.
- Settings tests on `test_player()` with a `MemorySettingsStore`, in the same way as
  `toggle_panels_reach_the_saved_snapshot`: after `update(.., Message::ShowAllSongs)` the latest
  snapshot's `library_view()` is `AllSongs`. A
  Songs view opened from Playlists saves `Playlists`, a Songs view opened from an artist's album
  saves `Artists`, and an active search saves `Artists`.
- `restore_view(&mut player, LibraryView::RecentlyAdded)` leaves `current_view ==
  CurrentView::RecentlyAdded`, and `restore_view(.., Artists)` leaves `CurrentView::Artists`.
- `session_ready` on a player showing All Albums schedules both the artists fetch and an All
  Albums refetch. On the Albums view or with a search active, it leaves `current_view` and
  `search_active` unchanged.
- `boot_schedules_loading_the_artist_list` still passes unchanged.
  `boot_reloads_artists_after_the_startup_sign_in_stores_a_session` (and the
  `await_boot_artist_loads` helper it uses) is updated to wait for a `Message::SessionReady` output
  after the first `LoadArtists`, in place of a second `LoadArtists`, and passes.
- `docs/usage.md`'s settings paragraph adds "the library's top-level browse list (Artists, All
  Songs, All Albums, Recently Added, Recently Played, or Playlists; a list opened from one of them
  saves that one)" to the list of saved values, and says a list showing when sign-in completes is
  refetched from the signed-in library. `docs/invariants.md` `## Settings persistence` names
  `library_view` beside the other player-owned values and the new tests.
- `make check` passes.

## Done
