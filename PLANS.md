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

### Shift+click Stop fades the song out and then stops, like Winamp (planned 2026-10-28 by the plan role)

**Goal.** In Winamp a Shift-held click on the Stop button fades the playing song out before
stopping, the same as Shift+V. Our Shift+V already does this (`Message::FadeOutAndStop`, handled by
`handlers::playback::fade::fade_out_and_stop`), but the ■ button always sends `Message::Stop`, so a
Shift-click stops at once. Make a Shift-held click on Stop fade out and stop in the main transport
row, the shade strip, and the library footer. This plan does not touch the waiting "Reopen the
library at the last top-level browse list" plan's files beyond `src/ui/mod.rs` and
`src/ui/message.rs` (separate arms and variants).

**Approach.** Decided, so the implementer has no open questions. It follows the Ctrl-click
Previous/Next pattern (`control_held`, `listeners::modifiers_message`, `stepping::previous_button`).
- Carry Shift with Control. In `src/ui/message.rs`, replace `ControlHeld(bool)` with
  `ModifiersHeld { control: bool, shift: bool }` (comment: whether Control and Shift are held; both
  false when the window loses focus), and add `StopButton` next to `NextButton`/`PreviousButton`
  (comment: a click on Stop: fade out and stop while Shift is held, otherwise stop at once).
- `src/ui/player.rs` `WinampPlayer`: add `pub(in crate::ui) shift_held: bool` beside
  `control_held` (doc: tracked from modifier events so a Shift-click on Stop fades out; not saved in
  settings), `false` at construction.
- `src/ui/subscriptions/listeners.rs` `modifiers_message`: `ModifiersChanged(m)` gives
  `ModifiersHeld { control: m.control(), shift: m.shift() }`; window `Unfocused` gives both false.
  Update its doc comment to name Shift and the Stop button.
- `src/ui/mod.rs` update: the `ModifiersHeld { control, shift }` arm sets `player.control_held` and
  `player.shift_held` and returns `Task::none()`. `StopButton` calls a new
  `handlers::playback::fade::stop_button(player)`, which returns `fade_out_and_stop(player)` while
  `shift_held` and `stop(player)` otherwise. A Shift-click therefore keeps every Shift+V guarantee
  (acts as Stop while paused or stopped; a second Shift-click mid-fade leaves the countdown alone),
  and a plain click mid-fade keeps Stop's stop-then-restore-volume behavior.
- `src/ui/views/transport.rs` `transport_glyphs`: the Stop entry sends `Message::StopButton`; the
  shade strip and library footer pick it up through `views::mini_transport`. Extend the doc
  comment's last sentence to say Stop sends its button message, which fades out while Shift is
  held. Keys keep their messages: V and the media Stop key still send `Message::Stop`.
- Only Shift matters for Stop, and only Control for Previous/Next: a Shift-click on Previous/Next
  and a Ctrl-click on Stop behave like plain clicks. No other button changes.

**Files touched.** src/ui/message.rs, src/ui/player.rs, src/ui/mod.rs,
src/ui/subscriptions/listeners.rs, src/ui/handlers/playback/fade.rs, src/ui/views/transport.rs,
src/ui/views/tests/transport.rs (`glyphs[3].3` asserts `Message::StopButton`),
src/ui/views/tests/shade.rs (`messages[3]` asserts `Message::StopButton`),
src/ui/tests/ctrl_click_tests.rs (switch its `ControlHeld(..)` sends and asserts to
`ModifiersHeld { .. }`), src/ui/tests/fade_tests.rs, docs/usage.md, docs/invariants.md.

**Tests.**
- In src/ui/tests/fade_tests.rs, using its `playing_with_recording`, `tick`, and `volumes` helpers:
  `shift_click_stop_starts_the_fade` (send `ModifiersHeld { control: false, shift: true }` then
  `StopButton` while playing: no backend call on the click, `fade_out_ticks == FADE_OUT_TICKS`,
  and ticking runs the same volume steps and final Stop as Shift+V);
  `plain_stop_click_stops_at_once` (with `shift_held` false, `StopButton` records `AudioCall::Stop`
  and starts no fade); `shift_release_restores_a_plain_stop_click` (Shift held then released, then
  `StopButton` stops at once).
- In src/ui/tests/ctrl_click_tests.rs, `modifiers_message_tracks_control_and_focus_loss` asserts
  `ModifiersChanged(Modifiers::CTRL)` gives `{ control: true, shift: false }`,
  `ModifiersChanged(Modifiers::SHIFT)` gives `{ control: false, shift: true }`, `Unfocused` gives
  both false, and a `KeyPressed` gives `None`; the Ctrl-click tests keep passing with
  `ModifiersHeld { control: true, shift: false }`, and a new `shift_click_previous_steps_one_song`
  checks Shift alone leaves `PreviousButton` a one-song step.

**Docs.** docs/usage.md: after the Shift+V clause in the keyboard paragraph ("…while paused or
stopped it acts as Stop)"), add that a Shift-held click on the Stop button, in the transport row,
the shade strip, or the library footer, does the same, while a plain click stops at once.
docs/invariants.md `## Fade-out stop`: extend the guarantee to a Shift-clicked Stop
(`Message::StopButton`), name `fade::stop_button` and `listeners::modifiers_message` under
Enforced by, and add the new tests to Pinned by. In `## List edge jumps`, rename the
`control_held` sentence's message to the new `ModifiersHeld`. In the two sections that list the
strips' button messages (`Message::PreviousButton`, `Message::Play`, `Message::Pause`,
`Message::Stop`, `Message::NextButton`), change `Message::Stop` to `Message::StopButton`.

**Acceptance criteria.**
- With Shift held, clicking ■ in any of the three transport rows while a preview plays fades the
  output volume down over about two seconds and then stops, exactly like Shift+V, leaving the saved
  volume unchanged; while paused or stopped it stops at once.
- Without Shift, the Stop button stops exactly as today; Ctrl-click on Previous/Next still jumps to
  the list's edge, and Shift alone does not.
- Losing window focus clears both held flags.
- Every keyboard binding maps to the same message as before (V and media Stop send `Message::Stop`).
- The new tests pass, the docs describe the Shift-click, and `make check` passes.

### Browse the library by genre from a Genres list, like Winamp's Media Library (part 1/2) (planned 2026-10-29 by the plan role)

**Goal.** Winamp's Media Library lets you filter the audio library by Genre. Nostalgia shows each
song's primary genre in File info and uses it for AUTO (`Song::genre`, filled by
`primary_genre` in `src/apple_music/rest/wire/mapping.rs`), but you can't browse by it. Add a
`Genres` button to the Artists view. It opens a Genres list with one row per genre and the
genre's song count. Opening a row shows that genre's songs in the Songs view, and Back returns to
Genres. Part 2/2 ("Reopen the library at the Genres list across launches") makes the list
persist across launches.

**Approach.**
- New file `src/ui/genres.rs` (declared in `src/ui/mod.rs`): `pub(in crate::ui) struct Genre
  { name: String, songs: Vec<Song> }` (derive `Debug, Clone, PartialEq`) and a pure
  `pub(in crate::ui) fn group_by_genre(songs: Vec<Song>) -> Vec<Genre>`. Group by
  `song.genre.trim()`, comparing names case-insensitively, and name the group after the first
  spelling seen. Order groups by lowercase name. Songs with a blank genre go in a group named
  `Unknown`, always last; the sample library sets no genres, so it shows that one row. Songs
  inside a group keep their input (library) order.
- `src/ui/player.rs`: add `CurrentView::Genres`, `SongsOrigin::Genre`, and a field
  `genres: BrowseList<Genre>` (start it with `BrowseList::new(false)` in the constructor, like
  `playlists`). In `library_view`, map both `CurrentView::Genres` and `SongsOrigin::Genre` to
  `LibraryView::Artists` for now; part 2/2 replaces that.
- `src/ui/message.rs`: add `ShowGenres`, `GenresLoaded(Vec<Genre>)`, `GenresLoadFailed(String)`,
  and `GenreSelected { epoch: u64, index: usize }`, documented like their Playlists twins. Route
  them in `src/ui/mod.rs`'s `update`.
- `src/ui/handlers/browse.rs`:
  - `show_genres`: a copy of `show_playlists` that calls
    `fetch_level(&mut player.genres, …, "loading library genres", |service| async move {
    service.get_all_songs().await.map(group_by_genre) }, Message::GenresLoaded,
    Message::GenresLoadFailed)`.
  - `genre_selected`: resolve the row with `player.genres.select(epoch, index)` and clone its
    songs. Call `player.songs.begin_fetch()` so an in-flight album or playlist `SongsLoaded`
    reply goes stale. Store the songs with `fetch::store_songs` (widen it to `pub(super)`), set
    `songs_origin = SongsOrigin::Genre`, `switch_view(player, CurrentView::Songs)`, and return
    `scroll::scroll_browse_to_top()`. No fetch is needed.
  - Add a `CurrentView::Genres` arm to `select_row`, `open_selected`, `open_row` (sends
    `GenreSelected`), both `match`es in `back` (`Genres → Artists`, `SongsOrigin::Genre →
    Genres`, and `player.genres.restore_library_order()`), `sort_browse` (by name),
    `reverse_browse`, `randomize_browse`, and `move_selected_row`.
- `src/ui/handlers/browse/fetch.rs`: add `genres_loaded` (`player.genres.store_with_labels(genres,
  count label)`; the label is `1 song` or `N songs`) and `genres_load_failed`. Add a
  `GenresLoaded | GenresLoadFailed` arm to `browse_reply` checking `player.genres.is_current`.
- Views: in `src/ui/views/browse.rs`, add `view_genres` (a copy of `view_playlists` that uses
  the cached labels, as the album lists do, with count nouns `genre`/`genres`) and
  `view_genres_button` (`labeled_button("Genres", Message::ShowGenres)`). Add `Genres` to that
  file's list of every `CurrentView`. In `src/ui/views/browse/rows/text.rs`, add a
  `"No genres"` arm to `empty_list_label`. In `src/ui/view/library.rs`, render the view and push
  the button after `view_playlists_button()` on the Artists view.
- The playback, jump-to-file, and overlay matches (`handlers/playback.rs`, `handlers/jump.rs`,
  `view/overlay.rs`) already treat `CurrentView::Songs` as `player.songs` and everything else
  through `_`, so a genre's songs play, step, and jump with no change there. Fix any other
  non-exhaustive match the compiler flags in the same style.
- Docs: in `docs/usage.md`, name the `Genres` button in the Artists-list buttons sentence. Say
  what its rows show (name, song count, `Unknown` last), that opening a row lists its songs, and
  that Back returns to Genres. In `docs/invariants.md`, add `Genres → Artists` and
  `Songs → Genres` to `## Browse hierarchy`, add Genres to the `## UI layout` sentence on the
  Artists-only buttons, and list the new tests under Pinned by.

**Tests.**
- Unit tests in `src/ui/genres.rs`:
  - grouping and ordering;
  - case-insensitive merging that keeps the first spelling;
  - blank and whitespace-only genres go to a last `Unknown` group;
  - songs keep their library order inside a group.
- In `src/ui/tests/flat_lists_tests.rs`, following the Playlists tests:
  - `ShowGenres` then `GenresLoaded` fills `player.genres` with labels like `2 songs`;
  - `GenreSelected` shows exactly that genre's songs in `CurrentView::Songs`;
  - Back returns to Genres and then to Artists;
  - a stale-epoch `GenreSelected` does nothing;
  - a `TrackSelected` on a genre's song plays it;
  - a superseded `GenresLoaded` is dropped.
- Add a `sort_reorders_genres_by_name` case in `sort_tests.rs`.

**Acceptance criteria.**
- The Artists view shows a `Genres` button that lists the library's genres with song counts,
  sorted by name, with `Unknown` last when any song lacks a genre.
- Opening a genre shows only its songs, which play, step with Next/Previous, and jump like any
  Songs list. Back returns to Genres, and Back again returns to Artists.
- Sort, reverse, randomize, and Alt+Up/Down work on the Genres list like on Playlists.
- The new tests pass, docs/usage.md and docs/invariants.md describe the view, and `make check`
  passes.

### Reopen the library at the Genres list across launches (part 2/2) ("requires part 1/2 landed") (planned 2026-10-29 by the plan role)

**Goal.** The library reopens at the last top-level list (`LibraryView` in
`src/settings/library_view.rs`, saved as `library_view`), but part 1/2 ("Browse the library by
genre from a Genres list") saves the Genres list and a genre's Songs as `Artists`. Save and
restore Genres like every other top-level list.

**Approach.**
- `src/settings/library_view.rs`: add `LibraryView::Genres` with key `"genres"`, and add it to
  the `from_key` array and the `keys_round_trip_for_every_variant` test.
- `src/ui/player.rs` `library_view`: map `CurrentView::Genres` and `SongsOrigin::Genre` to
  `LibraryView::Genres`.
- `src/ui/handlers/browse.rs`: add `LibraryView::Genres => show_genres(player)` to
  `restore_view`, and add `CurrentView::Genres` to the flat-list arm of `session_ready` so
  sign-in refetches it.
- Docs: in `docs/usage.md`, add Genres to the settings paragraph's list of remembered lists.
  In `docs/invariants.md` `## Settings persistence`, add the key and name the new tests.

**Tests.**
- In `src/ui/tests/settings_tests.rs`, following the Playlists cases:
  - showing Genres, and a genre's Songs, saves `library_view = genres`;
  - booting with `genres` saved opens the Genres list and issues its fetch.
- In `loading_tests.rs` or the matching `session_ready` test file: a session landing while
  Genres shows refetches it.

**Acceptance criteria.**
- Quitting on the Genres list, or on a genre's songs, reopens the library at Genres on the next
  launch. Every other saved list behaves as before.
- An unknown stored word still falls back to Artists.
- The new tests pass, the docs name Genres, and `make check` passes.

## Done
