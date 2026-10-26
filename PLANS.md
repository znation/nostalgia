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

### Split the transport's Play/Pause button into separate Play and Pause buttons, like Winamp (planned 2026-10-26 by the plan role)

**Goal.** `docs/screenshots/transport.png` and `hero.png` draw distinct Play (`▶`) and Pause (`II`)
buttons between Previous and Stop, but `transport_glyphs` in `src/ui/views/transport.rs` builds one
combined Play/Pause button whose glyph swaps with `is_playing`. Split it into two fixed buttons that
dispatch the existing explicit `Message::Play` (the X key's play/resume) and `Message::Pause` (the C
key's pause), in the main transport row, the shade strip, and the library footer. This is
presentation and message wiring only: no new `Message`, handler, state field, setting, or audio
behavior, and the LCD's own transport glyph, the X/C keys, and the system media keys stay as they
are. The mockup's sixth glyph (eject) stays out of scope because the app has no open-a-file action.

**Approach.**

- `src/ui/views/transport.rs`: make `transport_glyphs()` a five-entry, playback-independent spec in
  row order: Previous `("◀◀", "Previous", 34.0, Message::PreviousTrack)`, Play
  `("▶", "Play", 30.0, Message::Play)`, Pause `("II", "Pause", 30.0, Message::Pause)`, Stop
  `("■", "Stop", 30.0, Message::Stop)`, Next `("▶▶", "Next", 34.0, Message::NextTrack)`. Delete
  `play_pause_glyph` and `play_pause_word`, and drop `is_playing` from `transport_glyphs`,
  `transport_buttons`, and `view_transport_controls`. `transport_buttons` returns seven elements
  (the five tooltip-wrapped glyphs, then the Repeat and Shuffle toggles). Update the module and
  function docs from four glyphs to five. Also update the `Message::Play`/`Message::Pause` variant
  comments in `src/ui/message.rs` and the `play`/`pause` doc comments in
  `src/ui/handlers/playback.rs` to name the new buttons alongside X and C; the handlers themselves
  do not change.
- `src/ui/views/mini_transport.rs`: `mini_transport_controls()` loses its `is_playing` argument and
  returns five tuples; update its "four buttons" docs and the row's leading-comment wording. The
  loop in `mini_transport_row_with_optional_seek` and the seek/footer geometry stay put.
- `src/ui/view.rs`: drop the now-unused `is_playing` argument at the `view_transport_controls`
  call; no other root-view arm changes.
- Tests: in `src/ui/views/tests/transport.rs`, rewrite
  `transport_glyphs_match_the_mockup_order_and_widths` for the five glyphs, words, widths
  `[34.0, 30.0, 30.0, 30.0, 34.0]`, and messages (`PreviousTrack`, `Play`, `Pause`, `Stop`,
  `NextTrack`); extend `transport_buttons_keep_their_measured_face_widths` to the seven widths;
  delete `play_pause_glyph_and_word_mirror_playing_state`; drop the removed imports and
  `is_playing` arguments elsewhere in the file. In `src/ui/views/tests/shade.rs`, update
  `main_transport_and_shade_strip_share_glyphs_order_and_widths` and
  `shade_controls_map_to_transport_messages_and_keep_fixed_widths` to five entries and the new
  messages, replace `shade_play_pause_glyph_switches_with_playback_state` with a check that both
  `▶` and `II` are drawn in every playback state, and rename
  `library_strip_draws_the_four_controls_time_and_right_edge_volume` to its five-control twin,
  expecting `["◀◀", "▶", "II", "■", "▶▶"]` while the readout and right-edge volume checks stay.
  In `src/ui/tests/view_tests.rs`, replace
  `transport_row_does_not_reflow_when_the_play_glyph_changes` with a test that lays out the root
  view in stopped and playing states and asserts the Play and Pause glyphs keep the same bounds
  (the row no longer depends on playback). Remove the deleted helpers from
  `src/ui/views/tests/mod.rs`.
- Docs: `docs/usage.md`'s main-row, library-strip, and shade-strip sentences name Previous, Play,
  Pause, Stop, and Next, and say Play starts or resumes while Pause pauses (stopping a track whose
  source supplied no preview); X and C keep their current descriptions. `docs/invariants.md`
  `## UI layout`'s transport bullet names the five glyphs and their messages, replaces the
  "Play/Pause glyph swap cannot reflow" rationale with the static-row one (the fixed widths still
  pin every button), updates **Enforced by** and the **Pinned by** test names to the tests above,
  and adds a History line.

**Files touched.** `src/ui/views/transport.rs`, `src/ui/views/mini_transport.rs`,
`src/ui/view.rs`, `src/ui/message.rs`, `src/ui/handlers/playback.rs`,
`src/ui/views/tests/mod.rs`, `src/ui/views/tests/transport.rs`,
`src/ui/views/tests/shade.rs`, `src/ui/tests/view_tests.rs`, `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- The root view, the shade strip, and the library footer each draw five glyphs in the order
  Previous, Play, Pause, Stop, Next; Play dispatches `Message::Play` and Pause `Message::Pause`,
  while no drawn glyph dispatches `Message::PlayPause` (X/C and the system media Play/Pause keys
  keep their existing messages).
- Each row's laid-out content is identical in the stopped, playing, and paused states: the five
  glyphs, Repeat, Shuffle, volume, balance, EQ, and PL keep the same positions, and the compact
  strips still fit their five glyphs, time readout, and right-edge volume slider.
- Play resumes or restarts and Pause pauses through the same service calls as X and C; no LCD
  transport glyph, seek, volume, balance, equalizer, browse, or settings behavior changes.
- No new `Message`, handler, state field, setting, or dependency is added, and
  `play_pause_glyph` / `play_pause_word` no longer exist.
- The usage reference and the UI-layout invariant describe the five-glyph row, and the project's
  check passes.

## Done
