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
