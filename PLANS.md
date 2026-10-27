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

### Remember the rolled-up window-shade mode across launches (planned 2026-10-26 by the plan role)

**Goal.** Every other layout toggle — the readout mode, the visualizer, the EQ and library panels,
and always-on-top — is saved and restored, but `WinampPlayer::shaded` starts `false` on every
launch, so a user who closes the window rolled up gets the full window back. Persist the shade
mode in the settings file the same way: a launch whose file says `shade=true` draws the title bar
plus compact strip from the first view and rolls the window down to `SHADED_HEIGHT`, and its first
unshade restores the size the window opened at. This is a UI mode, not window placement: the
window's size and position stay deliberately unpersisted.

**Approach.**

- `src/settings.rs`: add a private `shade: bool` field beside `always_on_top`; `Settings::default`
  and `Settings::from_state` leave it `false`; add `shaded()` / `set_shaded(bool)` accessors in the
  same style as the other player-owned flags; write `shade={}\n` from `to_text`; and parse the
  `"shade"` key through `parse_into`, so a missing or malformed value falls back to unshaded.
  Update the module and struct docs: six UI-owned values now, naming the shade mode and keeping
  the statement that window size and position are not persisted.
- `src/ui/player.rs`: `persist_settings_if_changed` copies `self.shaded` through
  `snapshot.set_shaded`; `adopt_saved_settings` sets `self.shaded = saved.shaded()` before the
  first view. Update both methods' docs and the `WinampPlayer::shaded` field doc from the
  current "resets each launch"/five-value wording to the persisted six.
- `src/ui/handlers/window.rs`: `window_id_resolved` must capture the launch window's size when the
  restored mode is shaded, because `unshaded_size` is `None` at boot. After storing the id, when
  `player.shaded && player.unshaded_size.is_none()`, also schedule
  `iced::window::size(id).map(Message::WindowShadeMeasured)`; batch it with the saved
  always-on-top `set_level` call when both apply, return exactly one task when only one applies,
  and `Task::none()` when neither does. The existing `window_shade_measured` then stores the
  default size and resizes to the strip, so the first unshade restores it; no new message or
  handler.
- Tests: in `src/settings.rs`, extend `to_text_round_trips_every_field` for the shade flag and add
  `shade_round_trips_and_ignores_malformed_values` (default false, explicit true round-trips, an
  unknown word falls back to unshaded). In `src/ui/tests/settings_tests.rs`, add
  `toggle_window_shade_reaches_the_saved_snapshot` (with no window id, toggling flips `shaded`
  and saves once; another toggle saves the cleared value; `Message::Ignored` saves nothing) and
  `saved_shade_is_restored_on_boot` (adopting a shaded snapshot sets `player.shaded`). Align
  `shaded_eq_toggle_saves_nothing` with the new snapshot by setting `player.saved_settings`'s
  shade flag alongside `player.shaded`, so the update sees no shade change. In
  `src/ui/tests/window_tests.rs`, add `window_id_resolved_measures_a_restored_shade`: a shaded
  player with no captured size schedules work, while an unshaded unpinned one still schedules
  none; then `Message::WindowShadeMeasured(iced::Size::new(800.0, 600.0))` stores that size and
  keeps `shaded` true.
- Docs: `docs/usage.md` adds the shade mode to the saved-values sentence and says a saved-shaded
  launch opens rolled up and unrolls with the same controls. `docs/invariants.md`
  `## Settings persistence` adds the shade mode to the persisted list and the six UI-owned values,
  removes shade state from the "not window placement" sentence (size and position stay
  unpersisted), and updates **Enforced by**, **Pinned by**, and **History**.
  `## Window shade` adds the boot restore bullet (restore before the first view, measure the
  launch size through `window_id_resolved`, resize to `SHADED_HEIGHT`, no measurement for an
  unshaded launch) and updates **Enforced by**, **Pinned by**, and **History**.

**Files touched.** `src/settings.rs`, `src/ui/player.rs`, `src/ui/handlers/window.rs`,
`src/ui/tests/settings_tests.rs`, `src/ui/tests/window_tests.rs`, `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- `shade=true` survives `Settings::to_text` / `from_text`; a missing, empty, or malformed line
  falls back to unshaded, and the file's other keys keep parsing as before.
- Toggling shade writes exactly one new snapshot whose `shaded()` matches the new state, a second
  toggle writes the cleared state, and a message that changes no value writes nothing.
- A launch adopting a shaded snapshot has `player.shaded` set before the first view, so the root
  view draws the title bar plus compact strip; once `WindowIdResolved` lands, it schedules the
  size query (batched with a saved always-on-top level when both apply), and the resulting
  `WindowShadeMeasured` stores that size and resizes to `SHADED_HEIGHT`, leaving the following
  unshade able to restore it. A saved-unshaded, unpinned launch schedules no window work.
- No new `Message`, `WinampPlayer` field, dependency, or window-placement persistence is added;
  the settings file only gains one `shade=` line.
- The usage reference and the settings and window-shade invariants describe the restored shade
  mode, and the project's check passes.

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
