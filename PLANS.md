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



### Reopen the library at the Composers list across launches (part 2/2) ("requires part 1/2 landed") (planned 2026-10-29 by the plan role)

**Goal.** Part 1/2 ("Browse the library by composer from a Composers list") saves the Composers
list and a composer's Songs as `LibraryView::Artists`. Save and restore Composers like every
other top-level list (`LibraryView` in `src/settings/library_view.rs`, saved as `library_view`).

**Approach.**
- `src/settings/library_view.rs`: add `LibraryView::Composers` with key `"composers"`, and add
  it to the `from_key` array and the `keys_round_trip_for_every_variant` test.
- `src/ui/player.rs` `library_view`: map `CurrentView::Composers` and `SongsOrigin::Composer` to
  `LibraryView::Composers`.
- `src/ui/handlers/browse.rs`: add `LibraryView::Composers => show_composers(player)` to
  `restore_view`, and add `CurrentView::Composers` to the flat-list arm of `session_ready` so
  sign-in refetches it.
- Docs: in `docs/usage.md`, add Composers to the settings paragraph's list of remembered lists.
  In `docs/invariants.md` `## Settings persistence`, add the key (beside `years`) and name the
  new tests.

**Tests.**
- In `src/ui/tests/settings_tests.rs`, following the Years cases: showing Composers, and a
  composer's Songs, saves `library_view = composers`; booting with `composers` saved opens the
  Composers list and issues its fetch.
- In the test file holding the `session_ready` refetch cases: a session landing while Composers
  shows refetches it.

**Acceptance criteria.**
- Quitting on the Composers list, or on a composer's songs, reopens the library at Composers on
  the next launch. Every other saved list behaves as before, and an unknown stored word still
  falls back to Artists.
- The new tests pass, the docs name Composers, and `make check` passes.

## Done
