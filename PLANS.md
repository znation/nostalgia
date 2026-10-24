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

### Open the selected browse row with Enter (planned 2026-10-23 by the plan role)

**Goal.** Complete the browse selection flow with the keyboard: after a click selects a row,
pressing Enter should open it exactly as a second click or the `Open` button does. The
select-then-open plan already added `Message::OpenSelected` and
`handlers::browse::open_selected`, which resolves the showing list's selected row and dispatches
the existing artist/album/song/playlist action, but `shortcuts::shortcut` binds only the letters
and arrow keys, so Enter currently does nothing. This adds that one binding. It adds no message,
handler, fetch, or playback path, and it leaves the search box's Enter behavior alone.

**Approach.**

- `src/ui/shortcuts.rs`: in `shortcut`, add
`Key::Named(key::Named::Enter) => Some(Message::OpenSelected)` beside the arrow-key arms. Update
the `message_for` and `shortcut` docs to list Enter as the browse-row activation key. Enter
shares the existing Control/Alt/Logo rejection, so a chord still maps to nothing; a plain or
Shift-held Enter maps.
- `src/ui/shortcuts.rs` tests: add `message_for_maps_enter_to_open_selected`, using the existing
`key_pressed` helper and asserting `Message::OpenSelected`. Add `Key::Named(key::Named::Enter)`
to the key list in `shortcut_ignores_chord_modifiers`. The existing release/non-press test already
proves a key release maps to nothing.
- `src/ui/tests.rs`: add `enter_opens_the_selected_song`, building on
`open_selected_opens_the_selected_song`'s fixture. Seed the browse lists, show `CurrentView::Songs`,
select index 0 through `Message::SelectRow`, map a plain Enter key press through
`shortcuts::message_for`, and assert it is `Message::OpenSelected`; drive that message through
`update` and assert the same `TrackSelected` output and committed `song-1` playback the Open
button test asserts. This pins the key-to-action wiring end to end without introducing another
open path.
- `README.md`: in the Usage sentence describing browse selection, add Enter alongside the
second click and the `Open` button as a way to open the selected row. Do not edit the
`<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Browse selection`, add Enter as another activation route for the
selected row, with no selection being a no-op, and note that the search box's own Enter is
captured by `TextInput::on_submit` and remains `Message::SearchSubmitted` (the keyboard
subscription receives only ignored events). Name `shortcuts::message_for`, `Message::OpenSelected`,
and `handlers::browse::open_selected` as the enforcers, and add
`message_for_maps_enter_to_open_selected` and `enter_opens_the_selected_song` to `Pinned by`.

**Files touched.** `src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- A plain or Shift-held Enter key press maps to `Message::OpenSelected`; a key release,
modifier-only event, or Control/Alt/Logo chord still maps to nothing.
- With a row selected on any browse level, Enter opens it through the existing `open_selected`
path, producing the same action as the `Open` button; with no selected row it is a no-op that
schedules no fetch or playback task.
- Enter while the search box is focused remains a search submit, and the search box's widget and
message wiring are unchanged.
- README and `docs/invariants.md` describe Enter as an activation route, and `make check` passes.

### Toggle elapsed and remaining time from the compact transport readout (planned 2026-10-23 by the plan role)

**Goal.** The compact transport row shared by the library footer and the shade strip shows the
elapsed or remaining time over the track's total, but only the large LCD readout can switch that
mode. In shade mode the LCD is gone, so a rolled-up player offers no way to switch the readout at
all. Make the compact readout a click target that publishes the existing
`Message::ToggleTimeDisplay`, using the LCD's own stopped guard
(`views::now_playing::elapsed_readout_clickable`): a press while playing or paused flips elapsed
to remaining (and back); a stopped press does nothing. This adds no message, handler, playback
path, or geometry.

**Approach.**

- `src/ui/views/mini_transport.rs`: change `mini_transport_row` to take
  `mode: TransportMode` instead of `is_playing: bool`, derive the controls' `is_playing` as
  `mode == TransportMode::Playing`, and wrap the readout `Text` in
  `iced::widget::MouseArea::new(...)` with `.on_press(Message::ToggleTimeDisplay)` and
  `.interaction(iced::mouse::Interaction::Pointer)` only when
  `super::now_playing::elapsed_readout_clickable(mode)` holds; when it does not, push the plain
  `Text` so a stopped readout stays inert. Keep the text built by `mini_transport_readout`
  unchanged, so the `--:--` fallback, bounds, and fixed row height do not move. Give
  `view_library_strip` the same `mode: TransportMode` parameter and forward it. Update the module
  and function docs to name the readout click and the stopped guard.
- `src/ui/views/shade.rs`: change `view_shade_strip` to take `mode: TransportMode`, pass it to
  `mini_transport_row`, and update its docs to say the shade readout toggles the elapsed/remaining
  mode while the LCD is hidden.
- `src/ui/view.rs`: in the normal branch, pass the existing `transport_mode` state value to
  `view_library_strip` instead of `is_playing` (the full transport row still uses `is_playing`).
  In the shaded early return, lock `state.transport_mode()` instead of `state.is_playing` and pass
  it to `view_shade_strip`. Update the `view` docs and the shade comment to record that the
  compact readout carries the time toggle into shade mode.
- `src/ui/views/tests.rs`: update the existing strip call sites (`captured_shade_strip`,
  `captured_library_strip`, `view_shade_strip_keeps_its_fixed_height_for_both_playback_states`,
  `view_library_strip_keeps_its_fixed_height`, and the other direct `view_shade_strip` /
  `view_library_strip` calls) to pass `TransportMode::Playing`/`Paused`/`Stopped`. Add
  `compact_readout_toggles_time_display_only_while_not_stopped`: for each of the two strip
  builders and each `TransportMode`, lay out the strip with the headless renderer, find the
  `<elapsed>/<total>` text, drive a real left press at its bounds, and assert the press publishes
  exactly one `Message::ToggleTimeDisplay` for `Playing` and `Paused` and none for `Stopped`.
  Keep the existing `elapsed_readout_is_clickable_only_while_not_stopped` as the guard's pure
  test; the new test pins that the shared row actually wires it up.
- `README.md`: extend the click sentence in the Usage paragraph to say the compact readouts in the
  library footer and the shade strip are click targets too, while playing or paused, and that a
  stopped readout ignores the click. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Window shade` and `## Library footer`, add that the shared compact
  readout publishes `Message::ToggleTimeDisplay` only while not stopped, names the same
  `WinampPlayer::time_display` mode the LCD uses, and is the only time toggle available in shade
  mode; name `views::mini_transport::mini_transport_row` and
  `views::now_playing::elapsed_readout_clickable` as the enforcers. In `## Now Playing label`,
  note that the compact readouts share the mode and its guard, and add the new test name to the
  `Pinned by` lists of each section it covers.

**Files touched.** `src/ui/views/mini_transport.rs`, `src/ui/views/shade.rs`, `src/ui/view.rs`,
`src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A left press on the compact time readout in either the library footer or the shade strip while
  playing or paused publishes exactly one `Message::ToggleTimeDisplay`; the same press while
  stopped publishes none. Because that message already flips `WinampPlayer::time_display`, both
  strips re-render in the selected mode.
- The compact row still draws the four transport glyphs, the readout, and the right-edge volume
  slider in the same order and fixed `MINI_TRANSPORT_HEIGHT`; the Play/Pause glyph still follows
  the playing state, and the readout's `--:--` fallback is unchanged.
- No new message, handler, or playback path is introduced; the stopped guard is the LCD's
  `views::now_playing::elapsed_readout_clickable` rather than a second rule, and shade mode can
  now switch the mode while the LCD is hidden.
- README and `docs/invariants.md` describe the compact readout as a guarded toggle, and
  `make check` passes.

## Done
