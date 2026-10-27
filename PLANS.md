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

### Name the active library source in the About panel (planned 2026-10-26 by the plan role)

**Goal.** The README's status says Apple Music browse falls back to the built-in sample library until
a MusicKit session exists, but once the About panel is open there is no way to tell which source is
answering the browse queries. Add one line to the existing About panel — `Library: Apple Music`
while `AppleMusicService::session()` reports a stored session, `Library: Sample library` otherwise —
so a user can see whether the real library or the fallback is in use. This is presentation only: no
playback, browse, settings, or sign-in behavior changes, and no new `Message`, state field, or
dependency is added.

**Approach.**

- `src/ui/views/about.rs`: add a pure `pub(super) fn library_source_label(signed_in: bool) -> &'static str`
  returning `"Library: Apple Music"` for `true` and `"Library: Sample library"` for `false`, and
  give `view_about_overlay` a `signed_in: bool` argument that pushes a `Text` carrying that label
  beside the version and tagline. Update the module and function docs from the name/version/tagline
  list to include the library source.
- `src/ui/view.rs`: in the `player.about_visible && !player.shaded` branch, pass
  `player.apple_music_service.session().is_some()` to `views::view_about_overlay`. The view already
  runs per frame, so a sign-in that lands while the panel is open changes the label on the next
  rebuild without a new message; update the root-view doc's About sentence to name the line.
- Tests: in `src/ui/views/about.rs`, add
  `library_source_label_names_the_signed_in_and_sample_libraries` asserting both literal strings and
  that they differ. In `src/ui/views/tests/chrome.rs`, give `about_overlay_element` a `signed_in`
  parameter and update its callers, then extend the About text test (renaming it
  `about_overlay_draws_the_name_version_tagline_and_library_source`) to lay out both states and
  assert `Library: Sample library` for the unsigned base and `Library: Apple Music` for the signed-in
  one. In `src/ui/tests/view_tests.rs`, add `about_panel_names_the_active_library_source`: with
  `about_visible` set on a fresh `test_player`, capture the root view's text and assert the sample
  label; then store a fixture session with
  `player.apple_music_service.authenticate_with("dev", &|_| Ok(sign_in_session()))`, capture again,
  and assert the Apple Music label.
- Docs: `docs/usage.md`'s About-panel sentence adds that the panel names the active library source
  (`Library: Apple Music` when signed in, `Library: Sample library` otherwise).
  `docs/invariants.md` `## About panel` adds the line to its guarantee, names
  `views::about::library_source_label` and the `session()` read in **Enforced by**, adds the three
  test names above to **Pinned by**, and adds a History line.

**Files touched.** `src/ui/views/about.rs`, `src/ui/view.rs`, `src/ui/views/tests/chrome.rs`,
`src/ui/tests/view_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The About panel still draws `ABOUT NOSTALGIA`, the `v`-prefixed package version, the tagline, and
  `Close`, and additionally draws exactly `Library: Sample library` when the service has no stored
  session and `Library: Apple Music` once one is stored.
- The label reflects the service's current session on every rebuild while the panel is open, with
  no new `Message` or stored UI state carrying it.
- Closing/backdrop/Escape behavior, shading, playback, browse, settings, and sign-in are unchanged,
  and no dependency is added.
- The usage reference and the About-panel invariant describe the line, and the project's check
  passes.

## Done
