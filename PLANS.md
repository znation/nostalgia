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

_None yet._

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

## Done
