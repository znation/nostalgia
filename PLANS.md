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

### Add docs/invariants.md pinning browse and Now Playing guarantees to their tests (planned 2026-10-20 by the steward role as a gap promotion for `unclear-invariant`)

**Goal.** Create `docs/invariants.md`, the record the validation-gap policy tells every role to read and update but which the repository does not have. Three Fixed entries carry the `unclear-invariant` tag because the guarantee had to be reconstructed before the bug could be confirmed: "Browse view can only move forward — no way back from Albums/Songs, a dead end", "A browse reply is stored after a newer request was issued, so a navigation shows the previous level's rows", and "Now Playing bar shows the track id instead of the song title". The document states each guarantee, names the code that enforces it, and names the tests that pin it, so a later bug in these areas reads the invariant instead of reconstructing it. No behavior changes.

**Approach.**

- Create `docs/invariants.md` with a short header explaining the file: one section per subsystem, each stating the guarantee, the enforcing code, and the tests that fail if it is removed; a change to a guarantee updates its section in the same commit.
- `## Browse hierarchy`: every browse level above Artists has a Back action that steps one level up, Artists is the top level, and Back from an active search leaves the search for the Artists list. Enforced by the `Message::Back` arm in `src/ui/mod.rs` and `views::can_go_back` in `src/ui/views.rs`. Pinned by `back_from_albums_returns_to_artists`, `back_from_songs_returns_to_albums`, `back_from_artists_is_a_noop`, `back_from_all_songs_returns_to_the_artists_list`, `back_leaves_an_active_search_for_the_artists_list`, and `back_button_is_available_only_below_the_artist_list`.
- `## Browse reply freshness`: a fetched reply is stored only when no newer request was issued for that list. `RequestGeneration::is_current` drops it on iced's executor thread and `BrowseList::is_current` re-checks at the store on the UI thread; a selection also carries its list's epoch so a press that outlived its list is ignored. Enforced by `RequestGeneration` in `src/ui/loading.rs` and `BrowseList::is_current`, `select`, and `store` in `src/ui/browse.rs`. Pinned by `a_slow_stale_browse_reply_does_not_overwrite_the_newer_list`, `a_slow_stale_browse_failure_does_not_overwrite_the_newer_list_or_error`, `a_reply_superseded_between_the_mapper_and_update_is_not_stored`, `a_stale_songs_reply_superseded_before_update_is_not_stored`, `a_stale_albums_reply_superseded_before_update_is_not_stored`, and `selection_messages_with_a_stale_epoch_or_index_do_nothing`.
- `## Now Playing label`: the bar shows the known current track's human-readable title and artist, never the internal id when the id is in the known-track index; an unknown id falls back to the id, no current track shows "Nothing", and Stop keeps the interrupted track's title. Enforced by `now_playing_label` and `now_playing_artist` in `src/ui/views.rs`. Pinned by `now_playing_label_resolves_known_track_to_title`, `now_playing_label_falls_back_to_id_when_track_not_in_tracks`, `now_playing_label_shows_nothing_with_no_current_track`, `now_playing_artist_resolves_known_track_to_artist`, `now_playing_label_keeps_the_track_name_after_browsing_to_another_album`, and the borrow tests.
- Each section cites the Fixed entry it came from (title and `BUGS-FIXED.md`) so the reconstruction history stays traceable.

**Files touched.** `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes; the change is documentation only and touches no source.
- `docs/invariants.md` exists with the three sections above, each naming its enforcing code and at least one test.
- Every cited test name is a real `fn` in the named file, checked with `make test-one TEST=<name>` (or by `grep` before running it).
- The browse section records a Back path from every level above Artists; the freshness section records both the executor-thread check and the store-time re-check; the Now Playing section records the unknown-id fallback and the "Nothing" contract.

## Done
