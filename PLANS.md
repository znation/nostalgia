# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the README's Usage section: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. Record any new drift here rather than appending
> another note.

## Planned

### Add the preview seek slider to the window-shade strip (planned 2026-10-24 by the plan role)

**Goal.** The mockup's rolled-up window (`docs/screenshots/equalizer.png`, the "WINDOW SHADE"
panel) puts a position bar between the transport buttons and the volume slider, but the build's
shade strip has no seek control: once the window is rolled up the user can play, pause, stop,
step, and change volume, but cannot scrub the loaded preview. Add the same seek slider the Now
Playing bar already has to the shade strip, between the time readout and the volume slider. It
uses the existing `Message::SeekPreviewChanged` and `Message::SeekPreviewReleased` drag state, so
it adds no message, handler, or playback path; normal mode, the library footer, and the Now
Playing seek bar stay as they are.

**Approach.**

- `src/ui/views/seek_bar.rs`: extract the slider construction out of `view_seek_bar` into a
  `pub(super) fn seek_slider(position: Option<Duration>, duration: Option<Duration>) ->
  Option<Slider<'static, Message>>` that returns `None` unless `seek_bar_value` resolves, and
  otherwise builds the current `0.0..=length` slider with `step(1.0)`,
  `Message::SeekPreviewChanged`, `.on_release(Message::SeekPreviewReleased)`, and the chrome
  style. `view_seek_bar` calls it and wraps the result in the unchanged fixed-height container,
  so the Now Playing bar keeps its shape and the two seek controls share one scale and message
  pair.
- `src/ui/views/mini_transport.rs`: add `pub(super) const MINI_SEEK_ID: &str =
  "mini-transport-seek";` beside `MINI_VOLUME_ID`, and a
  `pub(super) fn mini_transport_row_with_seek<'a>(mode: TransportMode, volume: f32, elapsed:
  Cow<'a, str>, total: Cow<'a, str>, position: Option<Duration>, duration: Option<Duration>) ->
  Row<'a, Message>`. Refactor the shared row builder so both compact rows share the buttons,
  readout, volume slider, height, and padding, but this variant pushes the `seek_slider` in a
  `Container` tagged `MINI_SEEK_ID` with `Length::Fill` between the readout and the volume slider
  when a position and length resolve, and keeps the existing fill `Space` otherwise. Keep
  `mini_transport_row` and `view_library_strip` (and their signatures) building no seek slider,
  so the library footer is unchanged.
- `src/ui/views/shade.rs`: extend `view_shade_strip` with `position: Option<Duration>` and
  `duration: Option<Duration>` and build it from `mini_transport_row_with_seek`; update its doc
  comment to name the seek slider and the fact that a held drag previews through the readout the
  caller already derives from `WinampPlayer::readout_position`.
- `src/ui/view.rs`: in the shaded branch, resolve `player.readout_position()` and
  `player.playback_duration` beside the existing mode, volume, elapsed, and total values and pass
  them to `views::view_shade_strip`; extend the branch comment so the seek strip is named among
  the pieces shade mode draws. The elapsed string already comes from `now_playing_elapsed`, so a
  held drag updates the shade readout without new code.
- Tests. `src/ui/views/tests.rs`: add
  `shade_strip_shows_a_seek_slider_only_when_position_and_length_are_known`, using the
  `CapturedContainer` pattern to find `MINI_SEEK_ID`: with a position and non-zero length the
  slider is laid out between the readout and the volume slider and the volume stays at the
  right edge; with a missing position or a missing/zero length no seek container is laid out
  and the row keeps its old fill space. Update every `view_shade_strip` call site
  (`captured_shade_strip`, `view_shade_strip_keeps_its_fixed_height_for_both_playback_states`,
  `compact_readout_toggles_time_display_only_while_not_stopped`) to pass the two new arguments.
  `src/ui/tests.rs`: add `shade_root_readout_previews_the_seek_drag`, starting from the existing
  playing-preview fixture, setting `player.shaded = true` and a known preview length, driving
  `Message::SeekPreviewChanged(12.0)`, and capturing the shaded root view's text to assert the
  readout shows the drag target; keep the existing shade height and root-view tests green.
- `README.md`: in the Usage sentence that rolls the window up, add the seek slider over the
  preview's own length between the time readout and the volume slider. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Window shade`, add the seek slider to the strip's control list and
  its guarantee: it appears only when the backend reports both a position and a preview length,
  spans that preview length, emits the shared seek drag/release messages, and blanks (keeping
  the row's layout) otherwise; name `seek_bar::seek_slider`,
  `mini_transport::mini_transport_row_with_seek`/`MINI_SEEK_ID`, and `views::shade::view_shade_strip`
  as enforcers, and list the new test names plus `view_shade_strip_keeps_its_fixed_height_for_both_playback_states`.

**Files touched.** `src/ui/views/seek_bar.rs`, `src/ui/views/mini_transport.rs`,
`src/ui/views/shade.rs`, `src/ui/view.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- Rolling the window up shows a seek slider between the elapsed/remaining readout and the volume
  slider whenever the backend reports both a playback position and a non-zero preview length;
  its scale is that preview length rather than the catalog track's length, its thumb is clamped
  into `0..=length`, and dragging previews the target in the shaded readout.
- Dragging the shade slider emits only `Message::SeekPreviewChanged` and releasing it emits
  `Message::SeekPreviewReleased`, the same backend seek path as the Now Playing slider; with a
  missing position, missing length, or zero length, no seek slider is drawn and the row keeps the
  fill space that puts the volume slider at the right edge.
- The shade strip keeps `MINI_TRANSPORT_HEIGHT`/`SHADE_STRIP_HEIGHT`/`SHADED_HEIGHT` unchanged,
  and the library footer still builds no seek slider and keeps its current layout.
- The Now Playing seek bar's scale, drag messages, and fixed height are unchanged, and the new
  and existing shade tests plus `make check` pass.

### Show and hide the library section with a PL button (planned 2026-10-24 by the plan role)

**Goal.** The mockups' main window (`docs/screenshots/hero.png` and `player.png`) puts a `PL`
button directly after the `EQ` button beside the balance slider, but the build draws only `EQ`.
In normal mode the library section — its `NOSTALGIA LIBRARY` band, Back/Retry/Open/Top/Sort/Reload
and flat-list buttons, search row, browse list, and footer — is therefore always on screen, and
the player panels can never fill the window on their own. Add the `PL` toggle in the mockup's
slot: pressing it hides the whole library section while the title bar, Now Playing bar, seek bar,
transport row, and equalizer panel stay drawn, and pressing it again restores them. It is a
layout-only toggle like the EQ panel's: it changes no playback, equalizer, browse, search, or
selection state, resizes no window, and is open at launch and not persisted.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) library_panel_visible: bool` beside
  `eq_panel_visible`, initialized `true` in `WinampPlayer::new`. Document that
  `Message::ToggleLibraryPanel` flips it, that it gates the whole normal-mode library column, and
  that it is deliberately not persisted, like the EQ panel flag.
- `src/ui/message.rs`: add `ToggleLibraryPanel` beside `ToggleEqualizerPanel`, with a doc comment
  saying it shows or hides the normal-mode library section and schedules no work.
- `src/ui/handlers/mixer.rs`: add `pub(in crate::ui) fn toggle_library_panel(player: &mut
  WinampPlayer) -> Task<Message>` beside `toggle_equalizer_panel` that flips the flag and returns
  `Task::none()`; extend the module doc to name both panel-visibility toggles.
- `src/ui/mod.rs`: route `Message::ToggleLibraryPanel` to that handler beside the EQ-panel arm.
- `src/ui/views/transport.rs`: add `pub(super) const LIBRARY_PANEL_BUTTON_WIDTH: f32 = 30.0;`,
  `pub(super) fn library_panel_toggle() -> (&'static str, f32, Message)` returning `("PL",
  LIBRARY_PANEL_BUTTON_WIDTH, Message::ToggleLibraryPanel)`, and
  `pub(super) fn library_panel_button_style(open: bool, status: button::Status) -> button::Style`
  with the EQ toggle's sunken-while-open rule. Extend `view_transport_controls` with a
  `library_panel_visible: bool` argument and push a fixed-width `PL` `Button` directly after the
  `EQ` one, using the new spec and style; update the doc comment.
- `src/ui/view.rs`: pass `player.library_panel_visible` to `view_transport_controls`, and gate the
  library column on it. Build the existing normal-mode tail into a local `let mut library =
  Column::new().push(views::view_title_band("NOSTALGIA LIBRARY"))` and move the existing
  conditional Back/Retry/Open/nav/flat-list pushes, the search row, `main_content`, and
  `view_library_strip` onto it, then push `library` onto the root column only while the flag
  holds. The player-panels `MouseArea` and the EQ section stay unconditional, and the
  `library_elapsed`/`library_total` clones stay where they are. Update the function doc to name
  the PL toggle among the controls.
- Tests. `src/ui/views/tests.rs`: import `LIBRARY_PANEL_BUTTON_WIDTH`, `library_panel_button_style`,
  and `library_panel_toggle`; add `library_panel_toggle_targets_the_visibility_flag` (label `PL`,
  width the exported constant and 30.0, message `Message::ToggleLibraryPanel`) and
  `library_panel_button_style_sinks_while_the_panel_is_open` (the EQ style test's four-field
  comparison against the pressed and ordinary chrome faces); extend
  `transport_controls_construct_for_both_play_states_volume_endpoints_and_repeat_states` with a
  `library_panel_visible` loop and the new argument. `src/ui/tests.rs`: add
  `toggle_library_panel_hides_and_restores_the_library_section`, seeded with a uniquely named
  artist row. Lay out the root through `captured_root_view_text_bounds` and
  `captured_root_view_volume_slider`: assert `PL` shares `EQ`'s row and starts at or after its
  right edge; assert `library_panel_visible` starts true; drive `Message::ToggleLibraryPanel`
  through `assert_message_schedules_no_work` and assert the flag is false; in the hidden capture
  assert the `NOSTALGIA LIBRARY` band, the seeded artist row, the `Open`/`Top`/`Clear` buttons,
  and the footer slider (`captured_root_view_volume_slider` returns `None`) are gone while `PL`,
  `EQ`, `NOSTALGIA EQUALIZER`, and the transport glyphs remain; toggle back and assert the
  library band, seeded row, and footer slider land at their original bounds; and assert the
  shared `AppState` (transport mode, current track, volume, balance, EQ enable/curve/preset) and
  `CurrentView` are unchanged.
- `README.md`: after the sentence about the `EQ` button, add the `PL` button: it sits beside `EQ`,
  shows and hides the library section (title band, browse controls, search box, list, and
  footer), leaves the player panels drawn, changes no playback, equalizer, browse, or search
  state, and is open at launch. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## UI layout`, add the `PL` toggle's guarantee — fixed width, drawn
  after `EQ` in every playback state, sunken while the library is shown, gating the whole
  library column, default open, not persisted, no state or window-size change — and name
  `views::transport::library_panel_toggle`/`library_panel_button_style`,
  `handlers::mixer::toggle_library_panel`, and `ui::view::view` as enforcers, with the new test
  names under Pinned by. In `## Library footer`, qualify its opening guarantee as holding while
  the library panel is shown and name the PL gate. Extend the History notes.

**Files touched.** `src/ui/player.rs`, `src/ui/message.rs`, `src/ui/handlers/mixer.rs`,
`src/ui/mod.rs`, `src/ui/views/transport.rs`, `src/ui/view.rs`, `src/ui/views/tests.rs`,
`src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The transport row draws a fixed-width `PL` button directly after `EQ` on the balance slider's
  row, sunken while the library section is shown and ordinary chrome while it is hidden; pressing
  it emits `Message::ToggleLibraryPanel` and schedules no work.
- The library section is open at launch. Pressing `PL` removes the `NOSTALGIA LIBRARY` band, the
  browse buttons, the search row, the browse list, and the library footer while the title bar,
  Now Playing bar, seek bar, transport row, and equalizer section stay drawn; pressing it again
  restores every removed element at its original bounds.
- Toggling changes no playback state, current track, volume, balance, equalizer state or curve,
  browse view, list contents, row selection, or search query, and does not resize the window; a
  fresh player starts with the library section open.
- The new and updated tests pass and `make check` passes.

## Done
