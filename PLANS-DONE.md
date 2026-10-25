# Done plans

Finished plans, newest first, moved here by tumwater from PLANS.md's `## Done`. This file
only grows: search it with grep, or read one entry by its line range. Never read it whole.

## Done

### Show the selected song's length before the total in the song-list footer, like Winamp (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** Classic Winamp's playlist editor status line reads `<selected time>/<total time>`.
Nostalgia's song-list footer (`song_list_summary`, `src/ui/views/browse/rows/text.rs`) shows
only `<count> song(s) - <total>`. While a song row is selected (its blue bar painted), the footer
should read `<count> song(s) - <selected length>/<total>`, for example `2 songs - 3:30/4:15`.
With no selection, or a selected index outside the list, it stays exactly as today. This touches
only the footer text: no message, handler, selection, or playback change. It is independent of
the waiting Ctrl+Z/Ctrl+B plan (no shared files besides README and `docs/invariants.md`, in
different paragraphs and sections).

**Approach.**
- `src/ui/views/browse/rows/text.rs` `song_list_summary`: change the signature to
  `song_list_summary(songs: &[Song], selected: Option<usize>) -> String`. Compute the total as
  today. When `selected.and_then(|index| songs.get(index))` is `Some(song)`, format
  `"{count} {noun} - {selected}/{total}"` where `selected` is
  `format_minutes_seconds(song.duration_ms)` (so a zero-length selected song reads `0:00`, never
  `--:--`); otherwise keep the current `"{count} {noun} - {total}"`. Update its doc comment to
  describe both forms and the out-of-range fallback.
- `src/ui/views/browse/rows.rs` `song_list`: pass its existing `selected` argument:
  `with_footer(list, song_list_summary(songs, selected))`. Add one clause to its doc comment
  saying the footer includes the selected row's length.
- Do not change `list_count_summary`, the artist/album/playlist footers, or `view_songs` /
  `view_all_songs` (they already pass `selected` through to `song_list`).

**Files touched.** `src/ui/views/browse/rows/text.rs`, `src/ui/views/browse/rows.rs`,
`src/ui/views/tests/browse.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**
- Every existing `song_list_summary` test in `src/ui/views/tests/browse.rs` passes `None` as
  the new argument and keeps its expected string.
- New tests in `src/ui/views/tests/browse.rs`, built on `summary_song`:
  - `song_list_summary_puts_the_selected_length_before_the_total`: songs of 210_000 and 45_000
    ms with `Some(1)` give `"2 songs - 0:45/4:15"`, and `Some(0)` gives `"2 songs - 3:30/4:15"`.
  - `song_list_summary_reads_a_zero_length_selection_as_zero`: songs of 210_000 and 0 ms with
    `Some(1)` give `"2 songs - 0:00/3:30"`.
  - `song_list_summary_ignores_a_selection_past_the_list`: one 210_000 ms song with `Some(1)`
    gives `"1 song - 3:30"`, and `&[]` with `Some(0)` gives `"0 songs - 0:00"`.
- `docs/invariants.md`: add a `## Song-list footer` section (Guarantee / Enforced by / Pinned
  by / History, the same shape as `## Library footer`) stating the two footer forms, that the
  selected length comes from the list's own `selected` index, and that an out-of-range index
  falls back to the plain form; name `song_list_summary` and `song_list` under Enforced by, and
  the existing and new `song_list_summary_*` tests under Pinned by.
- README Usage: after the clause "with each song list showing its song count and total playing
  time beneath the rows", add that while a song row is selected the footer shows that
  song's length before the total (`2 songs - 3:30/4:15`), like Winamp's playlist editor. Do not
  touch the brief or status blocks.
- `make check` passes.

### Ctrl+Z and Ctrl+B play the first and last song of the showing list, like Winamp (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** In classic Winamp, Ctrl+Z goes to the start of the playlist and Ctrl+B to its end.
Nostalgia's Z and B already step back and forward through the list the current view draws from
(`handlers::playback::step_track`). With the window focused, Ctrl+Z should play the first song of
that same list and Ctrl+B its last, whatever Shuffle, Repeat, and the playback position are. An
empty list does nothing. Plain Z/B and Shift+Z/B keep their current meaning. Alt and Logo chords
stay unbound.

**Approach.**
- `src/ui/message.rs`: add `FirstTrack` and `LastTrack` variants next to `NextTrack` /
  `PreviousTrack`, each with a one-line doc comment.
- `src/ui/mod.rs` `update`: route `Message::FirstTrack` to
  `handlers::playback::jump_to_edge_track(player, false)` and `Message::LastTrack` to
  `jump_to_edge_track(player, true)`, beside the `NextTrack` / `PreviousTrack` arms.
- `src/ui/handlers/playback.rs`: add `pub(in crate::ui) fn jump_to_edge_track(player: &mut
  WinampPlayer, last: bool) -> Task<Message>`. Take `(items, epoch)` from the existing
  `showing_songs` helper. When `items` is empty, return `Task::none()`. Otherwise pick index `0`
  or `items.len() - 1` and return `Task::batch([Task::done(Message::TrackSelected { epoch, index
  }), super::browse::scroll::scroll_browse_to_playing_row(player, index)])`, the same pair
  `step_track` schedules. Copy `index` and `epoch` into locals before the scroll call so the
  `showing_songs` borrow ends first. Do not take the state lock, and do not touch `play_history`,
  `seek_preview`, or the Previous restart rule. The jump plays the edge row even when it is
  already the current track, matching a direct row play.
- `src/ui/shortcuts.rs` `shortcut`: extend the Ctrl+F exception. With `modifiers.control()`
  held and neither alt nor logo held, `z` (ignoring ASCII case) maps to `Message::FirstTrack`
  and `b` to `Message::LastTrack`. These checks go before the chord guard, which still returns
  `None` for every other Control chord. Update the doc comments on `message_for` and `shortcut`
  that list the bindings and say Ctrl+F is the only Control chord.

**Files touched.** `src/ui/message.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`,
`src/ui/shortcuts.rs`, `src/ui/tests/stepping_tests.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**
- New test `message_for_maps_ctrl_z_and_ctrl_b_to_the_list_edges` in `src/ui/shortcuts.rs`
  tests:
  - `Modifiers::CTRL` and `CTRL | SHIFT` with `z`/`Z` map to `Message::FirstTrack`, and with
    `b`/`B` to `Message::LastTrack`.
  - `ALT`, `LOGO`, `CTRL | ALT`, and `CTRL | LOGO` with `z` and `b` map to `None`.
- `shortcut_ignores_chord_modifiers` stays green. It currently loops `z` over `Modifiers::CTRL`
  as well, so move `z` out of that key list and into a loop over `ALT` and `LOGO` only. Update
  its comment to name Ctrl+Z and Ctrl+B as exceptions alongside Ctrl+F.
- `message_for_maps_each_bound_letter` still sees plain Z as `PreviousTrack` and plain B as
  `NextTrack`.
- New tests in `src/ui/tests/stepping_tests.rs`, built on `player_stepping_from` and
  `assert_track_selected`:
  - `first_track_selects_the_first_song`: current `song-1`, `FirstTrack` gives epoch 0,
    index 0.
  - `last_track_selects_the_last_song`: current `song-0`, `LastTrack` gives the last index of
    `stepping_songs()`.
  - `edge_jumps_ignore_shuffle_and_repeat`: Shuffle and Repeat on in shared state, both
    messages still land on index 0 and the last index.
  - `edge_jumps_with_no_songs_loaded_do_nothing`: an empty `songs` list gives `assert_no_task`
    for both.
  - `edge_jumps_in_all_songs_use_the_flat_list`: `CurrentView::AllSongs` with `all_songs`
    filled resolves against `all_songs` and its epoch (follow
    `next_track_in_all_songs_steps_the_flat_list`).
  - `edge_jumps_scroll_the_showing_song_list`: under `CurrentView::Songs`,
    `assert_task_scrolls_browse_list_to` sees `BROWSE_LIST_PADDING` for `FirstTrack` and
    `BROWSE_LIST_PADDING + (len - 1) * BROWSE_ROW_HEIGHT` for `LastTrack`.
- `docs/invariants.md`:
  - Add a `## List edge jumps` section (Guarantee / Enforced by / Pinned by / History, in the
    same shape as `## Media keys`) that names the shortcut tests and the new stepping tests.
  - In `## Browse playing-row visibility`, add Ctrl+Z/Ctrl+B to the moves that scroll the list
    to the landed row, and name `jump_to_edge_track`.
- README Usage: after the sentence beginning "With the window focused, the classic Winamp keys
  drive playback: Z and B step back and forward", add a sentence saying Ctrl+Z plays the first
  song of the list Z and B step through and Ctrl+B plays its last song. Do not touch the brief
  or status blocks.
- `make check` passes.

### Click the LCD visualizer to hide or show it, like Winamp's vis area (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** In Winamp 2.x a left click on the main window's visualizer area cycles its mode
(spectrum, oscilloscope, off). Nostalgia has one mode, so the equivalent is a toggle: a left
press on the LCD's ten-bar visualizer strip publishes the same `Message::ToggleVisualizer` the
strip's `V` letter already publishes. The hidden strip keeps its fixed height and width (it
draws zero-height bars), so a press on the empty strip shows the bars again. No new message,
handler, state, or setting: `handlers::playback::toggle_visualizer` and the saved
`visualizer_visible` flag are reused as they are.

**Approach.**
- `src/ui/views/now_playing.rs`, `now_playing_visualizer_row`: keep building the same `Row`,
  then wrap it as `MouseArea::new(row).on_press(Message::ToggleVisualizer)
  .interaction(Interaction::Pointer)` (the pattern `lcd_indicator_strip` and the elapsed
  readout in `now_playing_row` already use). Change the return type from `Row<…>` to
  `MouseArea<'a, Message, Theme, Renderer>`; `now_playing_content` pushes it unchanged. Extend
  the doc comment to say a press toggles the visualizer. The press is armed in every playback
  state and whether or not the bars are visible, because `V` is too.
- `now_playing_content` passes `visible_spectrum(…)` output in as before; nothing about what
  is drawn changes.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests/lcd_display.rs` (or
`src/ui/views/tests/now_playing.rs`, beside
`lcd_indicator_strip_publishes_always_on_top_on_a_and_visualizer_on_v`), `docs/invariants.md`,
`README.md`.

**Acceptance criteria.**
- New test `visualizer_row_press_publishes_toggle_visualizer`: for both `None` and a
  `Some(Spectrum { … })` with non-zero bands, build `now_playing_visualizer_row(spectrum)` as an
  `iced::Element<'_, Message, iced::Theme, iced::Renderer>`, lay it out to find its bounds, and
  drive a left `ButtonPressed` at the bounds' centre through `layout_capture::publish_at`;
  assert exactly one `Message::ToggleVisualizer` is published.
- The existing `now_playing_visualizer_row_keeps_its_height_without_a_spectrum` and
  `now_playing_visualizer_row_draws_one_fixed_width_bar_per_band` still pass unchanged (the
  wrapper adds no height or container), as do the indicator-strip and
  `elapsed_readout_toggles_only_while_not_stopped` press tests.
- `docs/invariants.md`, `## Now Playing label`: in the bullet beginning "The LCD's visualizer
  row draws exactly `BAND_COUNT` green bars", add that the row is a mouse area whose left press
  publishes `Message::ToggleVisualizer` (the same message as the `V` letter) whether the bars
  are shown or hidden, and name the new test.
- README Usage: after the sentence "The strip's `V` hides the bars and shows them again from
  the still-stored spectrum, so the poll keeps running while they are hidden.", add a sentence
  that a click on the bars' strip does the same, hidden or shown. Do not touch the brief or
  status blocks.
- `make check` passes.

### Click an equalizer dB scale label to set every band to that gain (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** Bring over the classic Winamp equalizer shortcut. In Winamp 2.x, clicking the dB
labels beside the preamp sets every band slider at once: the top label sets them all to
maximum, the middle one to zero, and the bottom one to minimum. Nostalgia draws the same three
labels (`+12 DB`, `0 DB`, `-12 DB` in `db_scale_column`, `src/ui/views/equalizer.rs`), but they
are plain text. Right now, flattening a curve means dragging ten sliders or picking the Flat
preset, and Flat also resets the preamp. After this change, a left press on a label stores that
label's gain in all ten bands. The preamp gain and the EQ on/off flag stay as they are. The
preset selection clears, the same as for a hand-moved slider. The new curve goes to the audio
backend at once, like any other EQ change. This plan does not depend on the balance-detent
plan above.

**Approach.**
- `src/state.rs`: add `pub fn set_all_eq_bands(&mut self, gain: f32)` next to `set_eq_band`.
  It stores `equalizer::clamp_gain(gain)` in every slot of `eq_bands` and sets
  `eq_preset = None`. Its doc comment says the dB scale labels are its only caller, and that
  it leaves the preamp alone, as Winamp's labels do. Also update the field-clamp wording in
  the `set_eq_preamp`/`set_eq_band` doc comments, which currently say "both writers", so
  they list this third writer too.
- `src/ui/message.rs`: add `EqBandsSetAll(f32)` after `EqBandChange`, with a `//` comment
  in the style of its neighbours: it comes from a press on a dB scale label and carries that
  label's gain.
- `src/ui/handlers/mixer.rs`: add
  `pub(in crate::ui) fn eq_bands_set_all(player: &mut WinampPlayer, gain: f32) -> Task<Message>`
  that returns `mutate_equalizer(player, |state| state.set_all_eq_bands(gain))`. It does
  **not** set `player.mixer_drag`, because a click has no release to clear an LCD readout.
  Add a dispatch arm in `src/ui/mod.rs` beside the `Message::EqBandChange` arm.
- `src/ui/views/equalizer.rs` `db_scale_column`: wrap each of the three `Text`s in
  `MouseArea::new(..).on_press(Message::EqBandsSetAll(GAIN))` with
  `.interaction(Interaction::Pointer)`. The gains are `GAIN_MAX_DB`, `0.0` and `GAIN_MIN_DB`.
  Copy the readout's `MouseArea` usage in `src/ui/views/mini_transport.rs` and import what is
  missing. Keep the two fill `Space`s and the fixed `EQ_SLIDER_HEIGHT`, so the layout does not
  change. Update the doc comment to mention the click.
- Settings need no change: `persist_settings_if_changed` already snapshots `eq_bands()` and
  `eq_preset()`.

**Files touched.** `src/state.rs` (method and unit test), `src/ui/message.rs`, `src/ui/mod.rs`,
`src/ui/handlers/mixer.rs`, `src/ui/views/equalizer.rs`, `src/ui/tests/mixer_tests.rs`,
`src/ui/views/tests/equalizer.rs`, `docs/invariants.md`, `README.md` (Usage only).

**Acceptance criteria.**
- `src/state.rs` `tests`: add `set_all_eq_bands_clamps_every_band_and_clears_the_preset`.
  Apply a preset with `apply_eq_preset`, then call `set_all_eq_bands(99.0)`. Assert that
  `eq_bands() == [GAIN_MAX_DB; BAND_COUNT]`, that `eq_preamp()` still equals the preset's
  preamp, and that `eq_preset() == None`. Then call `set_all_eq_bands(0.0)` and assert every
  band is `0.0`.
- `src/ui/tests/mixer_tests.rs`: add `eq_scale_click_sets_every_band_and_keeps_the_preamp`.
  Use `test_player`, send `Message::EqPresetSelected(rock_preset())`, then send
  `Message::EqBandsSetAll(GAIN_MIN_DB)` through `update`. Assert that every band is
  `GAIN_MIN_DB`, that the preamp is still `rock_preset().preamp`, that `eq_preset()` is
  `None`, and that `player.mixer_drag` is `None`.
- `src/ui/views/tests/equalizer.rs`: add `pressing_a_db_scale_label_sets_every_band_to_its_gain`.
  For each `(label, gain)` in `[("+12 DB", GAIN_MAX_DB), ("0 DB", 0.0), ("-12 DB", GAIN_MIN_DB)]`:
  build `view_equalizer(true, 0.0, &[0.0; BAND_COUNT], None)`, find the label with
  `layout_capture::capture_text`, and send a left `ButtonPressed` to the label's centre with
  `layout_capture::publish_at` (the pattern is in `src/ui/views/tests/now_playing.rs`'s
  readout press test). Assert that exactly one published message is
  `Message::EqBandsSetAll(g)` with `g == gain`.
  `equalizer_preamp_and_db_scale_sit_left_of_the_band_sliders` still passes unchanged.
- `docs/invariants.md`: add a `## Equalizer scale shortcuts` section after
  `## Equalizer band tint`, in the layout of the neighbouring sections. Its guarantee: a press
  on a scale label stores that label's clamped gain in all ten bands, clears the preset,
  leaves the preamp, the on/off flag and the LCD readout untouched, and forwards the curve to
  the backend. It is enforced by `db_scale_column`, `handlers::mixer::eq_bands_set_all` and
  `AppState::set_all_eq_bands`, and pinned by the three tests above.
- README Usage: after the sentence that ends "change the playing preview's curve live;", add
  one sentence saying that clicking `+12 DB`, `0 DB` or `-12 DB` sets all ten bands to that
  gain and leaves the preamp alone. Do not touch the brief or status blocks.
- `make check` passes.

### Snap the balance slider to center when a drag moves into the middle zone (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** Make it easy to set the balance back to exactly center. In the classic player the
balance slider has a center detent. Nostalgia's balance slider moves in 1% steps
(`views::BALANCE_STEP`), so getting back to an exact `0.0` (`Balance: Center`, the green center
tint) takes pixel-precise dragging. Add a detent: a balance-slider change that moves toward
center and lands within `0.1` of it stores exactly `0.0`. A change that moves away from center
is never snapped. Without that rule the slider's own arrow keys could never leave center,
because a 1% step out would snap straight back.

**Approach.**
- `src/state.rs`: next to `clamp_balance`, add
  `pub const BALANCE_CENTER_SNAP: f32 = 0.1;` and
  `pub fn snap_balance_to_center(current: f32, requested: f32) -> f32`. The function clamps
  `requested` with `clamp_balance` (so NaN becomes `0.0`). It returns `0.0` when the clamped
  value's magnitude is below `BALANCE_CENTER_SNAP` **and** below `current.abs()`. Otherwise it
  returns the clamped value. Give it a doc comment that states the inward-only rule and why.
  Leave `AppState::set_balance` unchanged, so settings restore (`adopt_saved_settings`) still
  stores a saved off-center value as-is.
- `src/ui/handlers/mixer.rs` `balance_change`: replace the closure with
  `|state| state.set_balance(crate::state::snap_balance_to_center(state.balance(), balance))`.
  `mutate_balance` already forwards `state.balance()` to `set_output_balance`, so the audio
  backend gets the snapped value. The LCD readout (`mixer_readout_text`) and the groove tint
  (`style::balance_level_color`) also read the stored value, so they show `Balance: Center` and
  the center colour with no further change. Update the doc comment to mention the detent.
- No new message, no view change, and no settings-format change.

**Files touched.** `src/state.rs` (the function, the constant, unit tests), `src/ui/handlers/mixer.rs`,
`src/ui/tests/mixer_tests.rs` (one update-level test; the plan said `src/ui/tests.rs`, since split into per-concern modules), `docs/invariants.md`, `README.md` (Usage only).

**Acceptance criteria.**
- Unit tests in `src/state.rs`'s `tests` module, named
  `snap_balance_to_center_snaps_an_inward_move_near_center` and
  `snap_balance_to_center_never_snaps_an_outward_move`:
  - `(0.3, 0.05)` → `0.0`, and `(-0.3, -0.09)` → `0.0`.
  - `(0.3, 0.1)` → `0.1`, since the boundary is not snapped.
  - `(0.0, 0.01)` → `0.01`, and `(0.0, -0.05)` → `-0.05`.
  - `(0.05, 0.06)` → `0.06`.
  - `(0.5, f32::NAN)` → `0.0`, and `(0.5, 2.0)` → `1.0`.
- `src/ui/tests.rs`: add `dragging_balance_into_the_middle_snaps_to_center`. Use `test_player`
  and `seed_current_title`, then send `Message::BalanceChange(-0.4)`, then
  `Message::BalanceChange(-0.05)`. Assert `state.blocking_lock().balance() == 0.0` and
  `root_view_draws(&player, "Balance: Center")`. The existing `balance_change_clamps_value_before_storing`,
  `dragging_balance_shows_the_side`, and the `assert_message_schedules_no_work` balance cases
  still pass unchanged. Each one starts from center, so its moves are outward.
- `docs/invariants.md`: add a `## Balance center detent` section after `## Mixer drag readout`.
  Its guarantee is the inward-only snap below `BALANCE_CENTER_SNAP`, applied only to
  `BalanceChange` and not to settings restore. It is enforced by
  `state::snap_balance_to_center` and `handlers::mixer::balance_change`, and pinned by the
  tests above.
- README Usage: after "The balance slider sets its stereo balance.", add a sentence saying
  that a drag moving back toward the middle snaps to exact center within 10% of it. Do not
  touch the brief or status blocks.
- `make check` passes.

### Show "Seek to: MM:SS/MM:SS (NN%)" on the LCD while dragging the seek slider (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** In classic Winamp, dragging the position bar swaps the song title for the drag target,
e.g. `Seek to: 01:23/03:30 (39%)`, until you let go. Nostalgia already does this for volume,
balance, and equalizer drags through `views::mixer_readout_text`. A seek drag currently moves only
the large elapsed readout (`WinampPlayer::readout_position`). Show the seek target text in the
same LCD title slot while `WinampPlayer::seek_preview` is held, and bring the title back on release.
Independent of the EQ/library panel persistence plan: this one touches no settings code.

**Approach.**
- `src/ui/views/now_playing/mixer_readout.rs`: add a pure
  `pub fn seek_readout_text(target: Duration, length: Duration) -> String` that returns
  `format!("Seek to: {}/{} ({pct}%)", format_lcd_elapsed(target_ms), format_lcd_elapsed(length_ms))`,
  using `super::text::format_lcd_elapsed` (zero-padded `MM:SS`, the same form as the large readout).
  `pct` is `target_ms * 100 / length_ms` (integer floor), clamped to 100, and is `0` when
  `length` is zero. Clamp `target` to `length` before formatting, so the target never reads past the
  length. Widen the module doc so it covers the seek slider too. Re-export the function next to
  `mixer_readout_text` in `src/ui/views/now_playing.rs` and `src/ui/views/mod.rs`.
- `src/ui/view.rs`: the `view_now_playing` call currently passes
  `player.mixer_drag.map(|drag| views::mixer_readout_text(..))`. Chain
  `.or_else(|| player.seek_preview.zip(player.playback_duration).map(|(t, l)| views::seek_readout_text(t, l)))`
  onto it, so a mixer drag wins if both were ever set. Keep using the existing `mixer_readout`
  parameter of `view_now_playing` / `now_playing_row`; add no new parameter. The scale is
  `playback_duration` (the preview's own length, the same scale the seek slider uses), not the
  catalog length at the LCD's right edge.
- `src/ui/player.rs` `WinampPlayer::needs_marquee`: return `false` while `seek_preview.is_some()`
  too, matching how it already handles `mixer_drag.is_some()`.
- Do not add a seek drag to `timers::subscriptions`' `mixer_readout_showing` argument, and do not
  change `seek_preview_changed` / `seek_preview_released`. Release and Stop/new selection already
  clear `seek_preview` (`seek_preview_released_seeks_once_to_the_dragged_position`,
  `stop_and_track_selection_clear_the_playback_duration`).
- The shade strip has no title slot. Leave it unchanged.

**Files touched.** `src/ui/views/now_playing/mixer_readout.rs` (function + unit test),
`src/ui/views/now_playing.rs` and `src/ui/views/mod.rs` (re-exports), `src/ui/view.rs`,
`src/ui/player.rs` (`needs_marquee`), `src/ui/tests.rs`, `docs/invariants.md`
(`## Mixer drag readout`), README.md Usage (outside the brief markers).

**Acceptance criteria.**
- Unit test `seek_readout_shows_target_length_and_floored_percent` in `mixer_readout.rs`:
  (12.5 s, 30 s) reads `Seek to: 00:12/00:30 (41%)`; (0, 30 s) reads `Seek to: 00:00/00:30 (0%)`;
  (30 s, 30 s) reads `(100%)`; a target past the length reads the length and `100%`; a zero
  length reads `Seek to: 00:00/00:00 (0%)`.
- `src/ui/tests.rs` test `dragging_the_seek_slider_shows_the_target_until_release`: use
  `seed_current_title` with title `Tune` and set `playback_duration = Some(30 s)`. After
  `Message::SeekPreviewChanged(12.5)`, `root_view_draws(&player, "Seek to: 00:12/00:30 (41%)")`
  holds and `root_view_draws(&player, "Tune")` does not. After `Message::SeekPreviewReleased`, the
  title draws again and the seek text does not.
- A test (or an extension of `the_marquee_pauses_during_a_mixer_drag`) shows `needs_marquee()` is
  false while a seek drag is held and true again after release, for a title too long for the LCD.
- With no `playback_duration`, a stored `seek_preview` draws no seek text.
- `now_playing_elapsed_previews_the_seek_drag` and the existing mixer readout tests still pass
  unchanged.
- `docs/invariants.md` `## Mixer drag readout` names the seek text, `views::seek_readout_text`, and
  the new tests. The README sentence listing the drag readouts adds the seek example.
  `make check` passes.

### Remember the EQ and library panels' shown/hidden choice across launches (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** Classic Winamp reopens with the equalizer and playlist windows shown or hidden as the
user last left them. Nostalgia's `EQ` button (and the `E` key) and `PL` button already toggle
`WinampPlayer::eq_panel_visible` and `WinampPlayer::library_panel_visible`, but both reset to open
on every launch. Save both flags in the same settings file and restore them before the first view,
the same way the visualizer's `V` choice is already saved. This reverses the earlier scope choice
that left the two layout flags unpersisted. The work stays presentation-only: it adds no message,
handler, shortcut, playback path, dependency, or window geometry. It does not overlap the waiting
EQ drag readout plan, which touches `views::equalizer` and `now_playing/mixer_readout.rs`.

**Approach.**

- `src/settings.rs`: add private `eq_panel_visible: bool` and `library_panel_visible: bool`
  fields to `Settings`, both defaulting to `true` in `Default` and `from_state` (the player's
  launch values). Add `pub fn eq_panel_visible(&self) -> bool`, `pub fn set_eq_panel_visible(&mut
  self, visible: bool)`, `pub fn library_panel_visible(&self) -> bool`, and `pub fn
  set_library_panel_visible(&mut self, visible: bool)`, documented like the
  `visualizer_visible` pair. Write `eq_panel_visible=<bool>` and `library_panel_visible=<bool>`
  in `to_text`, after `visualizer_visible`. Parse both in `from_text` with `parse_into`, so a
  malformed value leaves the default shown. Update the struct doc, which lists the UI-owned
  values and says what `from_state` leaves them as, and the `Default` doc, to name the two panel
  flags.
- `src/ui/player.rs`: in `persist_settings_if_changed`, after `set_visualizer_visible`, add
  `snapshot.set_eq_panel_visible(self.eq_panel_visible);` and
  `snapshot.set_library_panel_visible(self.library_panel_visible);`. In `adopt_saved_settings`,
  restore both fields from `saved`. Update both methods' docs from three UI-owned values to five,
  and rewrite the `eq_panel_visible` and `library_panel_visible` field docs to say each flag is
  open by default and saved and restored by those two methods. They are no longer "deliberately
  not persisted".
- `src/settings.rs` tests: in `to_text_round_trips_every_field`, hide both panels and assert the
  round trip keeps them hidden. Add `panel_visibility_round_trips_and_ignores_malformed_values`,
  modeled on `visualizer_visibility_round_trips_and_ignores_malformed_values`: both default to
  shown, an explicit `false` for each round-trips independently, and an unknown value such as
  `eq_panel_visible=maybe` falls back to shown. Add `eq_panel_visible=maybe` and
  `library_panel_visible=maybe` to `from_text_ignores_malformed_and_unknown_lines`.
- `src/ui/tests.rs`: beside `toggle_visualizer_reaches_the_saved_snapshot`, add
  `toggle_panels_reach_the_saved_snapshot`. Attach a `MemorySettingsStore`, then send
  `Message::ToggleEqualizerPanel` and assert one snapshot with `eq_panel_visible()` false and
  `library_panel_visible()` true. Send `Message::Ignored` and assert no new snapshot. Send
  `Message::ToggleLibraryPanel` and assert a second snapshot with both false. Beside
  `saved_visualizer_visibility_is_restored_on_boot`, add
  `saved_panel_visibility_is_restored_on_boot`. It calls `adopt_saved_settings` with a snapshot
  hiding both panels and then with `Settings::default()`, and asserts the two player flags and
  `player.saved_settings` follow each snapshot. Also add `shaded_eq_toggle_saves_nothing`: with a
  store attached and `player.shaded = true`, `Message::ToggleEqualizerPanel` writes no snapshot,
  because `handlers::mixer::toggle_equalizer_panel` leaves the flag alone in shade mode.
- `README.md`: in the final Usage settings paragraph, add the equalizer and library panels'
  shown/hidden choices to the saved values. In the `EQ`/`PL` sentences, change "which is open at
  launch" and "the library is open at launch" to say each panel is open on a first launch and
  then reopens as last left. Leave the `<!-- tumwater:status:start -->` block alone.
- `docs/invariants.md`: in `## Settings persistence`, add both panel choices to the guarantee's
  list. Change "the three persisted values not held in `AppState`" to five, and name
  `WinampPlayer::eq_panel_visible` and `WinampPlayer::library_panel_visible` there and in
  **Enforced by**. State that a missing or malformed line for either flag falls back to shown.
  Add the new tests to **Pinned by** and note the extension in **History**. In `## UI layout`'s
  `PL` bullet, replace "the flag is not persisted" with a reference to `## Settings persistence`.

**Files touched.** `src/settings.rs`, `src/ui/player.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- A settings file with `eq_panel_visible=false` launches with the `NOSTALGIA EQUALIZER` band and
  equalizer panel hidden, and the `EQ` button is drawn unsunken. A file with
  `library_panel_visible=false` launches with the library section hidden in the same way. A
  `true` value, a missing line, or an unknown value for either key launches that panel shown.
  `to_text` round-trips both booleans independently.
- Each `EQ`, `E`, or `PL` toggle that changes a flag writes exactly one new snapshot through
  `persist_settings_if_changed`. An unchanged update writes nothing, and so does the shaded `EQ`
  toggle, which leaves the flag alone.
- Restoring a hidden equalizer panel leaves the EQ's on/off state, curve, preset, and audio
  forwarding exactly as `Settings::apply_to` restored them. Restoring either hidden panel changes
  no browse, search, selection, or window-size state.
- `settings.rs` keeps its `AppState`-only `apply_to` and still does not import the UI.
- README and `docs/invariants.md` describe the saved choices, and `make check` passes.

### Show an equalizer band's or the preamp's gain on the LCD while dragging its slider (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** In classic Winamp, dragging an equalizer band or the preamp replaces the scrolling title
with the gain being set until the mouse is released, the same way the volume and balance sliders
do. Nostalgia already does this for the mixer (`## Mixer drag readout` in `docs/invariants.md`),
but its eleven equalizer sliders change gain without showing anything. Extend that readout to
cover them. A band drag reads `EQ: 60HZ +3 DB`, and the preamp reads `Preamp: -12 DB`. The audio
path, settings, and slider ranges stay as they are. This plan does not depend on the preamp-tint
plan above. Both edit `views::equalizer::preamp_column`: that plan swaps its style closure, and
this one leaves the body alone. They also both touch `eq_slider_column`'s doc comment, so
whichever merges second rebases those lines.

**Approach.**
- `src/ui/views/now_playing/mixer_readout.rs`:
  - Add the variants `EqPreamp` and `EqBand(usize)` to `MixerDrag`. It stays `Copy`/`Eq`.
  - Extend the signature to
    `mixer_readout_text(drag, volume, balance, preamp: f32, bands: &[f32; BAND_COUNT])`. Import
    `crate::equalizer::{BAND_COUNT, BAND_FREQUENCIES}`.
  - Round the gain once with `let db = gain.round() as i32;`. This gives no `-0`, and NaN becomes
    `0`.
  - `EqPreamp` → `format!("Preamp: {db:+} DB")`.
  - `EqBand(i)` → `format!("EQ: {}HZ {db:+} DB", BAND_FREQUENCIES[i])`, using `bands[i]`.
  - An out-of-range `i` returns `String::new()`. Use `.get()` and never index directly.
  - Update the module doc to cover the EQ forms, and pass `0.0, &[0.0; BAND_COUNT]` in the two
    existing tests.
- `src/ui/handlers/mixer.rs`:
  - `eq_preamp_change` sets `player.mixer_drag = Some(views::MixerDrag::EqPreamp)` before its
    `mutate_equalizer` call.
  - `eq_band_change` sets `Some(views::MixerDrag::EqBand(band))`, but only when
    `band < BAND_COUNT`. An out-of-range band is already a no-op in `AppState::set_eq_band`.
  - `toggle_equalizer` and `eq_preset_selected` stay untouched, so the on/off button and a preset
    pick never show the readout.
  - Update `mixer_released`'s doc comment to say "a mixer or equalizer slider drag".
- `src/ui/views/equalizer.rs` `eq_slider_column`: add `.on_release(Message::MixerReleased)` to the
  `VerticalSlider`. iced 0.14's `VerticalSlider::on_release` exists. This one call covers the
  preamp and all ten bands. Mention the release in the doc comment.
- `src/ui/message.rs`: reword the `MixerReleased` doc to "The release of any `chrome_slider` or
  equalizer slider drag".
- `src/ui/view.rs`: the normal-mode closure calls
  `views::mixer_readout_text(drag, volume, balance, eq_preamp, &eq_bands)`. Both values are
  already read under the lock in the same function. Check whether they are moved into
  `view_equalizer` before the LCD is built, and borrow or reorder if so.
- Nothing else needs to change. `needs_marquee`, `timers::subscriptions` (via
  `mixer_drag.is_some()`), `mixer_key_message`, and `mixer_wheel_message` all already key off
  `mixer_drag`. So a focused EQ slider's arrow-key or Ctrl+wheel change shows the readout, and
  the next navigation key or wheel roll clears it, exactly as it does for the mixer.

**Files touched.** `src/ui/views/now_playing/mixer_readout.rs`, `src/ui/handlers/mixer.rs`,
`src/ui/views/equalizer.rs`, `src/ui/message.rs`, `src/ui/view.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Tests.**
- In `mixer_readout.rs`'s test module, add `eq_readouts_show_the_band_label_and_signed_gain`.
  It asserts:
  - `EqPreamp` at `-12.0` → `Preamp: -12 DB` and at `0.0` → `Preamp: +0 DB`.
  - `EqBand(0)` with `bands[0] = 3.0` → `EQ: 60HZ +3 DB`.
  - `EqBand(4)` with `bands[4] = -0.4` → `EQ: 1KHZ +0 DB`.
  - `EqBand(9)` with `bands[9] = 12.0` → `EQ: 16KHZ +12 DB`.
  - `EqBand(BAND_COUNT)` → `""`.
- In `src/ui/tests.rs`, next to `dragging_volume_shows_the_level_until_release`, using
  `seed_current_title` and `root_view_draws`:
  - `dragging_an_eq_band_shows_its_gain_until_release`: `EqBandChange(0, 5.0)` sets
    `mixer_drag` to `EqBand(0)`. The root view draws `EQ: 60HZ +5 DB` and not the title.
    `MixerReleased` clears the readout, and the title is drawn again.
  - `dragging_the_preamp_shows_its_gain`: `EqPreampChange(-3.0)` sets `EqPreamp` and draws
    `Preamp: -3 DB`.
  - `eq_toggle_and_preset_do_not_show_the_readout`: `ToggleEqualizer` and an
    `EqPresetSelected(..)` with any preset from `PRESETS` both leave `mixer_drag` `None`.
  - `an_out_of_range_eq_band_shows_no_readout`: `EqBandChange(BAND_COUNT, 5.0)` leaves
    `mixer_drag` `None`.

**Docs.**
- `docs/invariants.md` `## Mixer drag readout`: widen the guarantee from "a volume or balance
  `chrome_slider`" so that it also covers the preamp and band sliders, and give the two EQ text
  forms. State that the EQ on/off button and preset picks never set the readout. Add
  `eq_preamp_change`, `eq_band_change`, and `eq_slider_column`'s `on_release` to **Enforced by**,
  and the new tests to **Pinned by**.
- `README.md` Usage: after the existing sentence about the volume/balance drag readout (grep
  `Volume: `), add one sentence saying an equalizer band or preamp drag shows
  `EQ: 60HZ +3 DB` / `Preamp: -12 DB` the same way. Do not touch the brief block.

**Acceptance criteria.**
- While the mouse holds any band slider or the preamp slider, the LCD's title slot shows that
  slider's live rounded gain in the forms above. The glyph, elapsed readout, and total stay.
  Release restores the title.
- The EQ button, the preset list, and the volume/balance keys and wheel never set the readout.
  An out-of-range band index sets nothing.
- The existing mixer-readout tests (`dragging_volume_shows_the_level_until_release`,
  `dragging_balance_shows_the_side`, `volume_keys_and_wheel_do_not_show_the_readout`,
  `the_marquee_pauses_during_a_mixer_drag`, and the `mixer_readout.rs` unit tests) pass with only
  the new arguments added. The new tests pass.
- README and `docs/invariants.md` are updated as above, and `make check` passes.

### Tint the equalizer preamp groove by its gain like the band sliders (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** In the classic Winamp 2.x base skin, the preamp slider uses the same colored bar
sprites as the ten band sliders, so its groove runs from green at `-12 dB` through yellow at
`0 dB` to red at `+12 dB`. Nostalgia tints each band through `style::band_slider_style`, but
`views::equalizer::preamp_column` still passes the neutral `style::chrome_slider_style`. Make the
preamp groove follow the stored preamp gain with exactly the band tint. This change affects
presentation only. The preamp's range, step, `Message::EqPreampChange`, thumb, label, layout, and
the audio path stay as they are. It is independent of the level-tint plan above, which changes
only the transport and shade volume/balance fills. Both plans edit the `## Level slider fill`
section's closing sentence in `docs/invariants.md`, so whichever merges second rebases that one
sentence.

**Approach.**
- In `src/ui/views/equalizer.rs`, add
  `pub(super) fn preamp_slider_style(preamp: f32, status: slider::Status) -> slider::Style`
  that returns `style::band_slider_style(preamp, status)`. Make `preamp_column` pass
  `move |status| preamp_slider_style(preamp, status)` to `eq_slider_column`. A named function
  gives the test something to call, because a built `Column` does not expose its slider's
  style closure.
- Rewrite the doc comments on `eq_slider_column` and `preamp_column`. They currently say the
  preamp keeps the neutral/uniform gray `chrome_slider_style`. They should say the preamp and
  every band share the gain tint.
- Leave `src/ui/style.rs` unchanged. `band_slider_style` already clamps an out-of-range gain and
  maps NaN to the flat yellow through `band_gain_color`.

**Files touched.** `src/ui/views/equalizer.rs`, `src/ui/views/tests.rs`, `docs/invariants.md`,
`README.md`.

**Tests.** In `src/ui/views/tests.rs`, import `preamp_slider_style` through the existing
`super::equalizer::{…}` use and add
`preamp_slider_style_matches_the_band_tint_for_its_gain`. For each gain in `-12.0`, `-6.0`,
`0.0`, `6.0`, `12.0`, `20.0`, and `f32::NAN`, and for each of `slider::Status::Active`, `Hovered`,
and `Dragged`, assert that `preamp_slider_style(gain, status)` equals
`style::band_slider_style(gain, status)` on `rail.backgrounds`, `rail.width`, `rail.border`, and
`handle`. Also assert that at `-12.0` the rail's first background is
`Background::Color(style::band_gain_color(-12.0))`, and that it differs from
`style::chrome_slider_style(slider::Status::Active).rail.backgrounds.0`, so the test fails
against the old neutral groove.

**Docs.**
- `docs/invariants.md` `## Equalizer band tint`: replace "The vertical preamp slider in the band
  row keeps its uniform gray groove." with a sentence saying the vertical preamp slider's groove
  takes the same tint from the stored preamp gain. Add `views::equalizer::preamp_slider_style` to
  **Enforced by**, and the new test to **Pinned by**.
- `## Level slider fill`: change the closing sentence so that only the seek bar keeps the neutral
  `theme::LCD_BACKGROUND` groove.
- `README.md` Usage: extend the sentence "Each equalizer band slider's groove is tinted by that
  band's own gain…" so it says the preamp slider's groove is tinted the same way by the preamp
  gain. Do not touch the brief block between the `tumwater:prompt` markers.

**Acceptance criteria.**
- With the preamp at `-12 dB`, `0 dB`, and `+12 dB`, the preamp groove shows the same green,
  yellow, and red as a band slider at that gain, on both rail halves and in every interaction
  status. The chrome thumb, rail width, and border match `chrome_slider_style`.
- `preamp_column` no longer calls `style::chrome_slider_style`. A grep of
  `src/ui/views/equalizer.rs` for `chrome_slider_style` finds only doc-comment mentions or none.
  The seek bar in `src/ui/views/seek_bar.rs` still uses `chrome_slider_style`.
- `preamp_slider_style_matches_the_band_tint_for_its_gain` passes. The existing
  `band_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb`,
  `equalizer_preamp_and_db_scale_sit_left_of_the_band_sliders`, and
  `equalizer_slider_spec_matches_the_gain_clamp_and_steps_whole_decibels` pass unchanged.
- README and `docs/invariants.md` are updated as above, and `make check` passes.

### Tint the volume groove by level and the balance groove by offset from center (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** In the classic Winamp 2.x base skin, the volume slider's groove changes color with the
level, from green when silent to red at full volume. The balance groove is green at center and
turns red as it moves toward either side. Nostalgia paints fixed fills instead:
`theme::VOLUME_SLIDER_FILL` (orange) and `theme::BALANCE_SLIDER_FILL` (green). Make each level
slider's groove follow its value. The defaults must still match the mockups: half volume
(`AppState` default `0.5`) stays exactly the orange fill, and center balance stays exactly the
green fill. This change affects presentation only. Slider ranges, steps, messages, thumb
geometry, the seek bar, the preamp, and the EQ band tint all stay as they are. This plan is
independent of the LCD mixer-readout plan above, which changes how `chrome_slider` reports a
release. This plan changes only the `fill` that callers pass in.

**Approach.**
- New `src/ui/style/level_tint.rs`, declared from `src/ui/style.rs` as `mod level_tint;` with
  `pub use level_tint::{balance_level_color, volume_level_color};`. Rust resolves a child of
  `style.rs` to `src/ui/style/`. Putting the code there keeps the 800-line `style.rs` from
  growing. The child module may call the parent's private `mix_channel`
  (`super::mix_channel`).
  - `pub fn volume_level_color(volume: f32) -> Color`: first apply
    `crate::state::clamp_volume`, which maps NaN to `0.0`. For `v <= 0.5`, mix
    `theme::EQ_GAIN_LOW` → `theme::VOLUME_SLIDER_FILL` at `t = v / 0.5`. Above that, mix
    `theme::VOLUME_SLIDER_FILL` → `theme::EQ_GAIN_HIGH` at `t = (v - 0.5) / 0.5`. Mix each
    channel through `mix_channel`, with alpha `1.0`. The endpoint form returns each stop
    exactly.
  - `pub fn balance_level_color(balance: f32) -> Color`: compute
    `t = crate::state::clamp_balance(balance).abs()`, where NaN maps to center. Mix
    `theme::BALANCE_SLIDER_FILL` → `theme::EQ_GAIN_HIGH` at `t`, so the color is symmetric
    left and right.
  - Give each function a doc comment that names its stops, matching the style of
    `band_gain_color`.
- `src/ui/views/transport.rs` `view_transport_controls`: pass `style::volume_level_color(volume)`
  and `style::balance_level_color(balance)` to the two `chrome_slider` calls in place of the
  theme constants.
- `src/ui/views/mini_transport.rs` `mini_volume_slider`: pass `style::volume_level_color(volume)`,
  and update its doc comment, which still names the fixed `VOLUME_SLIDER_FILL` groove.
- `src/ui/views/mod.rs` `chrome_slider` doc comment: say callers pass a level-dependent fill. Do
  not change the signature.
- Keep the theme constants. They become the mid and center stops, so
  `base_skin_colors_keep_their_documented_literal_values` is unchanged.

**Files touched.** `src/ui/style/level_tint.rs` (new, with its `#[cfg(test)] mod tests`),
`src/ui/style.rs` (module declaration and re-export), `src/ui/views/transport.rs`,
`src/ui/views/mini_transport.rs`, `src/ui/views/mod.rs` (doc only), `README.md`,
`docs/invariants.md`.

**Docs.**
- `README.md` Usage currently says "a volume slider over an orange groove and a balance slider over
  a green groove". Change it to describe the shifting tint: the volume groove runs from green
  when silent through orange at half volume to red at full volume, and the balance groove is
  green at center and reddens toward either side. Do not touch the brief or status blocks.
- `docs/invariants.md` `## Level slider fill`: restate the guarantee in terms of the two
  functions and their exact stops. Add `level_tint.rs` to **Enforced by**, and list the new tests
  under **Pinned by**. Keep the sentence saying the seek bar and preamp stay on the neutral
  groove.

**Acceptance criteria.**
- Tests in `level_tint.rs` assert the following:
  - `volume_level_color` returns exactly `EQ_GAIN_LOW` at `0.0`, exactly `VOLUME_SLIDER_FILL`
    at `0.5`, and exactly `EQ_GAIN_HIGH` at `1.0`.
  - NaN and `-1.0` give `EQ_GAIN_LOW`, and `2.0` gives `EQ_GAIN_HIGH`.
  - At `0.25` and `0.75`, each channel lies between its two neighbouring stops.
  - `balance_level_color(0.0)` and `balance_level_color(f32::NAN)` are exactly
    `BALANCE_SLIDER_FILL`.
  - `balance_level_color(-1.0)` and `balance_level_color(1.0)` are exactly `EQ_GAIN_HIGH`.
  - `balance_level_color(-0.4) == balance_level_color(0.4)`.
  - Both functions return alpha `1.0`.
- The transport volume, transport balance, and mini/shade volume sliders get their fill from these
  functions. Grepping `src/ui/views` for `VOLUME_SLIDER_FILL` and `BALANCE_SLIDER_FILL` finds
  no remaining fill argument.
- `level_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb` and the EQ band-tint tests
  still pass unchanged.
- README and `docs/invariants.md` updated as above; `make check` passes.

### Show the volume or balance level on the LCD while dragging its slider (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** In classic Winamp, dragging the volume or balance slider swaps the scrolling title for
the level being set (`Volume: 72%`, `Balance: Center`, `Balance: 31% Left`) until the mouse is
released. Nostalgia's sliders change the level silently, and nothing shows the exact value. Make a
drag on any mixer slider show its live level in the Now Playing title slot, and bring the title
back on release. Arrow keys and the mouse wheel do not show the readout. No persisted setting
and no audio-backend change.

**Approach.**
- New file `src/ui/views/now_playing/mixer_readout.rs` (declared `pub(super) mod mixer_readout;`
  in `src/ui/views/now_playing.rs`, re-exported through `now_playing.rs` and `views/mod.rs` the
  same way as `TimeDisplay`). It holds:
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum MixerDrag { Volume, Balance }`.
  - `pub fn mixer_readout_text(drag: MixerDrag, volume: f32, balance: f32) -> String`.
    `Volume` → `format!("Volume: {}%", (volume * 100.0).round() as u32)`. `Balance` → let
    `pct = (balance * 100.0).round() as i32`. Then `0` → `"Balance: Center"`, negative →
    `"Balance: {-pct}% Left"`, positive → `"Balance: {pct}% Right"`. This matches
    `audio/balance.rs`, where `-1.0` is hard left. A balance that rounds to zero, such as
    `-0.004`, reads `Center`, never `-0%`. The inputs are already clamped by
    `AppState::volume()`/`balance()`.
- `src/ui/player.rs`: add a field `pub(in crate::ui) mixer_drag: Option<views::MixerDrag>`,
  initialised `None` and documented as transient (not saved to settings).
- `src/ui/message.rs`: add `MixerReleased`, commented as the release of any `chrome_slider`
  drag. `src/ui/mod.rs` routes it to a new `handlers::mixer::mixer_released`, which sets
  `mixer_drag = None` and returns `Task::none()`.
- `src/ui/handlers/mixer.rs`: `volume_change` sets `player.mixer_drag =
  Some(MixerDrag::Volume)` and `balance_change` sets `Some(MixerDrag::Balance)`, both before
  their existing `mutate_*` call. `volume_up`, `volume_down`, and `volume_wheel` stay untouched,
  so keys and the wheel never show the readout.
- `src/ui/views/mod.rs` `chrome_slider`: add `.on_release(Message::MixerReleased)`. That one call
  covers the transport volume and balance sliders (`views/transport.rs`) and the compact volume
  slider (`mini_transport.rs`, used by the library footer and the shade strip). The equalizer and
  seek sliders do not use `chrome_slider`, so they are unaffected. A release while the readout is
  already `None` is a harmless no-op.
- LCD: add a trailing parameter `mixer_readout: Option<String>` to `view_now_playing`,
  `now_playing_content`, and `now_playing_row`, and pass `None` at each existing test call site.
  In `now_playing_row`, `Some(text)` pushes one `lcd_text(text)` where the `"Now Playing: "`
  caption and the number/artist/title (or marquee window) would go. The transport glyph, elapsed
  readout, and right-edge total stay as they are. `None` keeps today's row exactly.
- `src/ui/view.rs` (normal tree): pass
  `player.mixer_drag.map(|drag| views::mixer_readout_text(drag, volume, balance))`, using the
  `volume` and `balance` already read under the lock. The text therefore follows the live
  clamped value on every `VolumeChange`/`BalanceChange`.
- `WinampPlayer::needs_marquee` returns `false` while `mixer_drag.is_some()`, so the marquee
  timer pauses during a drag. `marquee_offset` is left unchanged.
- `WinampPlayer::window_title` is not changed.
- Known limit, recorded in the invariant: if a slider were removed mid-drag, iced would send no
  release. The readout would then stay until the next mixer-slider release. No current control
  removes the transport row or footer while a mouse button is held.

**Files touched.** `src/ui/views/now_playing/mixer_readout.rs` (new), `src/ui/views/now_playing.rs`,
`src/ui/views/mod.rs`, `src/ui/view.rs`, `src/ui/player.rs`, `src/ui/message.rs`, `src/ui/mod.rs`,
`src/ui/handlers/mixer.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md` (`## Usage`:
one sentence after the volume/balance slider description), `docs/invariants.md` (new
`## Mixer drag readout` section: guarantee, enforced by, pinned by).

**Acceptance criteria.**
- Unit test in `src/ui/views/tests.rs` (or the new file's own `#[cfg(test)]` block):
  `mixer_readout_text` gives `Volume: 72%` for `0.72`, `Volume: 0%`/`Volume: 100%` at the ends,
  `Balance: Center` for `0.0` and `-0.004`, `Balance: 31% Left` for `-0.31`, and
  `Balance: 100% Right` for `1.0`.
- `captured_now_playing_texts`-style test: `now_playing_row` with `Some("Volume: 72%")` draws
  that fragment and no `Now Playing: ` caption, title, artist, or number. The glyph, elapsed
  readout, and `3:30` total stay. A marquee-length title with a readout set draws no marquee
  window.
- Root tests in `src/ui/tests.rs`:
  - `Message::VolumeChange(0.4)` sets `mixer_drag` to `Volume`, and the root view text
    (`captured_root_view_text_bounds`) contains `Volume: 40%` but not the current track's title.
  - `Message::MixerReleased` then clears it, and the title is drawn again.
  - `BalanceChange(-0.25)` shows `Balance: 25% Left`.
  - `VolumeUp` and `VolumeWheel(1.0)` leave `mixer_drag` `None`.
  - `needs_marquee()` is `false` during a drag of an overlong title and `true` again after
    release.
- Volume/balance audio forwarding and settings saving behave as before (the existing mixer tests
  still pass).
- README and `docs/invariants.md` updated as above; `make check` passes.

### Seek with the mouse wheel over a drawn seek slider (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** The existing player-panel wheel area sends every roll to the volume, including a roll
over the Now Playing seek slider and the shade strip's seek slider. Make a roll over a *drawn*
seek slider seek five seconds forward for a positive `y` or back for a negative `y`, like the
classic position bar, while every other player panel keeps its volume nudge and a blank seek
strip stays part of the volume area. Reuse `Message::SeekForward`, `Message::SeekBackward`, and
`views::SEEK_STEP_SECS`; add no message, player field, persisted setting, or backend path.

**Approach.**

- `src/ui/views/seek_bar.rs`: add `pub(super) const SEEK_STRIP_ID: &str = "now-playing-seek-strip"`
  for the strip's outer container and `pub(super) const SEEK_BAR_ID: &str =
  "now-playing-seek"` for its drawn slider. Add `pub(super) fn seek_wheel_message(delta:
  ScrollDelta) -> Message`: a positive `y` (iced's wheel-up direction, as in
  `views::volume_wheel_message`) publishes `Message::SeekForward`, a negative `y` publishes
  `Message::SeekBackward`, and a horizontal-only, zero, or non-finite roll publishes
  `Message::Ignored`. In `view_seek_bar`, keep the outer container tagged `SEEK_STRIP_ID`; when
  `seek_slider` returns a slider, wrap it in a fill-width inner container tagged `SEEK_BAR_ID`
  around `MouseArea::new(slider).on_scroll(seek_wheel_message)`. Leave the blank `Space` case
  unwrapped, so only a drawn slider captures the wheel.
- `src/ui/views/mini_transport.rs`: in `mini_transport_row_with_seek`, wrap the `Slider` inside
  the existing `MINI_SEEK_ID` container in `MouseArea::new(..).on_scroll(seek_wheel_message)`, so
  the shade strip's drawn slider seeks too. The library footer builds `mini_transport_row`
  without a seek slider and stays unchanged.
- `src/ui/views/mod.rs`: re-export `seek_wheel_message`, `SEEK_STRIP_ID`, and `SEEK_BAR_ID` at
  `pub(super)`/`pub(crate)` visibility for the views and root-view tests, following the existing
  `seek_bar_value` and `MINI_SEEK_ID` exports.
- `src/ui/views/tests.rs`: add `seek_wheel_message_maps_a_vertical_roll_to_a_direction` (both
  `Lines` and `Pixels`, both signs) and `seek_wheel_message_ignores_a_horizontal_only_roll`;
  add `wheel_over_the_shade_seek_slider_seeks` using the existing `MINI_SEEK_ID` bounds capture
  and a `mouse::Event::WheelScrolled`, asserting exactly one `SeekForward`/`SeekBackward` and no
  `VolumeWheel`.
- `src/ui/tests.rs`: with a player whose `playback_position` and `playback_duration` are set,
  use `captured_root_view_container(player, SEEK_BAR_ID)` and a `WheelScrolled` at its center to
  assert one `SeekForward`/`SeekBackward` and no `VolumeWheel`; with both cleared, use
  `SEEK_STRIP_ID` to assert the blank strip publishes one `VolumeWheel` and no seek. Name them
  `wheel_over_the_root_seek_slider_seeks_without_nudging_the_volume` and
  `wheel_over_a_blank_seek_strip_still_nudges_the_volume`.
- `README.md`: in the wheel sentence in `## Usage`, note that a roll over the drawn seek slider
  seeks five seconds forward or back instead of changing the volume, in both the Now Playing bar
  and the shade strip; leave the browse list's wheel behavior and the rest of the volume rule as
  they are.
- `docs/invariants.md`: amend `## Volume wheel` to name the drawn seek slider as the exception,
  and add `## Seek wheel` after it stating the sign mapping, the `Message::Ignored` no-op, the
  drawn-only capture, and that a blank strip and every other panel keep the volume nudge, with
  the enforcing code and the new test names.

**Files touched.** `src/ui/views/seek_bar.rs`, `src/ui/views/mini_transport.rs`,
`src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- A vertical wheel roll over the drawn Now Playing seek slider publishes exactly one
  `Message::SeekForward` for a positive `y` and one `Message::SeekBackward` for a negative `y`,
  and no `Message::VolumeWheel`; a horizontal-only roll publishes no seek.
- A vertical wheel roll over the shade strip's drawn seek slider does the same.
- A roll over the blank Now Playing seek strip, with no position or preview length, publishes
  the panel's one `Message::VolumeWheel` and no seek, so the exception needs a drawn slider.
- Every other player panel and the browse list keep their current wheel behavior, and the
  existing seek, volume, and shade-strip tests still pass.
- No new `Message` variant, `WinampPlayer` field, persisted setting, or backend call is added;
  `make check` passes.

### Open an About panel from the title bar's M button (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** `docs/screenshots/hero.png` shows the app bar's leftmost button as `M`, the classic
Winamp main-menu slot, but `views::view_title_bar` draws an `A` there (always-on-top, which the
LCD strip's `A` already does) and the app has no About UI. Replace the title bar's `A` with `M`,
make it toggle a modal About panel that names the app, shows the package version from
`CARGO_PKG_VERSION`, and carries the mockup's one-line description, and keep the LCD strip's `A`
as the always-on-top control. The panel is transient and not persisted; opening it while shaded
restores the window first, and it closes from M, its `Close` button, or a click on the backdrop.
No new dependency, shortcut, or playback behavior.

**Approach.**

- `src/ui/views/title_bar.rs`: rename `CLUTTER_BUTTON_WIDTH` to `TITLE_BAR_LEFT_BUTTON_WIDTH`
  (keep 18 px) and change `view_title_bar`'s argument to `about_visible: bool`. Draw `M` in the
  left slot with `Message::ToggleAbout` and `style::toggle_button_style(about_visible, status)`;
  update the module and function docs, the width assertion, and the comments that call the slot
  the A toggle.
- `src/ui/message.rs`: add `ToggleAbout` beside the window actions, documented as the title bar's
  M button; it only flips the panel flag (after unshading when needed).
- `src/ui/player.rs`: add `about_visible: bool` beside `library_panel_visible`, initialized
  `false`, documented as transient and not persisted.
- `src/ui/handlers/window.rs`: add `toggle_about(player) -> Task<Message>`: flip
  `about_visible`; when opening while `player.shaded` holds, call the existing
  `toggle_window_shade(player)` first and return its task so the window restores; otherwise
  `Task::none()`. Document that the panel never coexists with shade mode.
- `src/ui/mod.rs`: route `Message::ToggleAbout` beside the other window actions.
- New `src/ui/views/about.rs`, declared and re-exported by `src/ui/views/mod.rs`: a private
  `const ABOUT_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));` and
  `view_about_overlay(base) -> Element` that stacks `base` under a full-window `MouseArea`
  backdrop (a translucent `container::Style` built in the module) publishing
  `Message::ToggleAbout`, and above it a centered `bevel::raised_panel` holding
  `ABOUT NOSTALGIA`, `ABOUT_VERSION`, the mockup line `The classic Winamp look, rebuilt in Rust —
  with Apple Music as the library.`, and a `Close` button publishing `Message::ToggleAbout`. Wrap
  the panel itself in a `MouseArea` that swallows presses with `Message::Ignored`, so only the
  backdrop dismisses. Reuse the existing chrome and style helpers; add no theme constant outside
  the module.
- `src/ui/view.rs`: build the shade or normal base tree into one `Element` (replacing the shade
  branch's early return) and, while `player.about_visible && !player.shaded` holds, pass it
  through `views::view_about_overlay`; otherwise return it unchanged. Pass `player.about_visible`
  (not `always_on_top`) to `views::view_title_bar` in both branches.
- `src/ui/views/tests.rs`: update `title_bar_centers_its_name_between_its_edge_controls` for the
  renamed M slot; add `title_bar_m_button_publishes_toggle_about` (headless press) and
  `about_overlay_draws_the_name_version_and_tagline` (layout capture), plus a press on the panel's
  `Close` and on a backdrop point outside the panel each publishing exactly one `ToggleAbout`.
- `src/ui/tests.rs`: replace `title_bar_constructs_for_both_always_on_top_states` with an
  About-state construction test; add
  `toggle_about_shows_and_hides_the_panel_without_touching_playback` (drive `Message::ToggleAbout`,
  assert the flag flips, no work is scheduled, and transport, mixer, and browse state are
  unchanged) and `toggle_about_while_shaded_restores_the_window_first` (set `shaded`, give a
  window id, drive it, assert `!shaded`, `about_visible`, and scheduled resize work); render with
  the panel open in both `view_constructs_over_the_apps_full_input_space` and
  `view_constructs_when_the_window_is_shaded`.
- `README.md`: replace the title bar's `A` claim with the M/About description (name, version,
  tagline, and the three ways to close it), keeping the LCD strip's `A` as the always-on-top
  control. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: update `## UI layout`'s app-bar bullet (M and shade/minimize/close, not
  A); add `## About panel` after `## Window shade` stating that the panel is transient and
  unpersisted, draws only while `about_visible` holds outside shade mode, names `ABOUT_VERSION`
  from the package, and is enforced by `views::view_about_overlay`,
  `handlers::window::toggle_about`, and `Message::ToggleAbout`, with the new test names; note in
  `## Window shade` that opening About restores the window (the shade tree stays title bar plus
  strip).

**Files touched.** `src/ui/views/title_bar.rs`, `src/ui/views/about.rs`, `src/ui/views/mod.rs`,
`src/ui/views/tests.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/transport.rs`,
`src/ui/message.rs`, `src/ui/player.rs`,
`src/ui/handlers/window.rs`, `src/ui/mod.rs`, `src/ui/view.rs`, `src/ui/style.rs`,
`src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The title bar's left slot draws `M` and publishes exactly one `Message::ToggleAbout` on a press;
  the LCD strip's `A` still publishes `Message::ToggleAlwaysOnTop` exactly as before.
- While `about_visible` holds in normal mode, the root view draws the About panel over the
  otherwise unchanged base, showing `ABOUT NOSTALGIA`, the `v`-prefixed `CARGO_PKG_VERSION`, the
  tagline, and `Close`.
- Pressing M, `Close`, or the backdrop flips `about_visible` exactly once; pressing the panel's
  non-button chrome publishes `Message::Ignored` and leaves it open.
- Opening from shade mode schedules the existing unshade resize and leaves `shaded` false with
  `about_visible` true; the view never draws the panel in the shade branch.
- The title bar no longer draws an always-on-top toggle; the strip's `A` and the saved
  always-on-top behavior are unchanged.
- No new dependency, shortcut, persisted setting, or playback change is added; the new tests and
  `make check` pass.

### Restart the current preview with Previous after three seconds (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** Classic Winamp's Previous restarts the track already playing once it has played at
least three seconds; only earlier does it step to the preceding song. Nostalgia always steps, so a
mistimed Previous skips the current track. Make Previous — the button, the Z key, and the media
Previous key, which all share `Message::PreviousTrack` — restart the loaded preview from zero when
its last reported position is at least three seconds, while keeping the existing step (including
Shuffle's recorded-play retrace) for earlier presses and when no preview reports a position. No
new message, player flag, or setting.

**Approach.**

- `src/ui/handlers/playback.rs`: add a private `PREVIOUS_RESTART_SECS: u64 = 3` constant. At the
  start of `step_track`, before taking the state lock, when `!forward`, `player.playback_position`
  is at least that threshold, and `player.apple_music_service.has_loaded_preview()` holds, clear
  `player.seek_preview`, call `player.apple_music_service.seek_output_to(Duration::ZERO)`, and on
  success bump `player.position_epoch` and store `player.playback_position = Some(Duration::ZERO)`,
  then return `Task::none()`. On error, log in the existing `eprintln!` style and return
  `Task::none()` with the position and epoch unchanged. The restart branch must precede the
  sequential/Shuffle computation, so a late Previous neither steps nor consumes a Shuffle history
  entry; earlier positions fall through unchanged. Update `step_track`'s doc comment to state the
  restart rule and that it uses the last reported position (the one the readout shows), not a
  fresh backend query.
- `src/ui/tests.rs`: add tests written against the literal `Duration::from_secs(3)` so a changed
  constant fails:
  - `previous_restarts_the_preview_after_three_seconds`: build `player_playing_preview`, set
    `player.songs.items = stepping_songs()` and `player.playback_position = Some(Duration::from_secs(3))`,
    drive `Message::PreviousTrack`, and assert no task, one `AudioCall::SeekTo(Duration::ZERO)`
    after the preview-start prefix, `player.playback_position == Some(Duration::ZERO)`,
    `player.seek_preview == None`, the epoch bumped, and `state.current_track` unchanged.
  - `previous_steps_before_the_restart_threshold`: the same fixture with
    `Some(Duration::from_secs(2))`; drive Previous, assert `TrackSelected` index 0 and no `SeekTo`
    call.
  - `previous_with_shuffle_restarts_after_the_threshold_instead_of_retracing`: the same fixture
    with `shuffle = true` and a recorded play; at three seconds assert no `TrackSelected`, one
    `SeekTo(Duration::ZERO)`, and the current track unchanged.
  - `a_failed_previous_restart_leaves_the_track_queued`: load the preview over
    `FailingAudio::failing_seek`, set the position to three seconds, drive Previous, and assert
    `attempts(FailingCommand::Seek) == 1`, no task, and unchanged position, epoch, and current
    track.
- `README.md`: in the keyboard/transport sentence, say Previous (and Z) restarts the playing
  preview once it has played at least three seconds and steps back before that; keep the Shuffle
  sentence describing the recorded-play retrace for earlier presses. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add `## Previous restart` after `## Play history` with the guarantee,
  `handlers::playback::step_track` (with `transport::previous_track_id` and
  `transport::PlayHistory` for the unchanged step paths) as **Enforced by**, the four test names
  under **Pinned by**, and a History line.

**Files touched.** `src/ui/handlers/playback.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- A Previous press at a reported position of at least three seconds with a loaded preview sends
  exactly one absolute seek to zero, clears any seek drag, resets the readout position,
  invalidates in-flight position replies, and leaves the current track and transport mode
  unchanged.
- A Previous press before three seconds, or with no loaded preview or reported position, steps
  exactly as today, including Repeat's edge behavior and Shuffle's recorded-play retrace, and
  sends no seek.
- A rejected restart seek is logged and leaves the track queued with the position and epoch
  unchanged; no step is scheduled.
- No new `Message` variant, player flag, or persisted setting is added.
- The new tests and `make check` pass.

### Restore the window from shade mode by double-clicking the strip (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** In shade mode the rolled-up window is the title bar over the compact transport strip,
and only the 24px title bar answers a double-click by unshading (through
`views::title_bar::view_title_bar`'s `MouseArea::on_double_click`). The classic Winamp behavior
is that any double-click on the rolled-up window restores it, so a double-click on the much
larger strip should unshade too. Make the strip's blank chrome a double-click target for the
existing `Message::ToggleWindowShade`, without adding a message, handler, or state flag and
without changing any single-click, button, readout, slider, or wheel behavior.

**Approach.**

- `src/ui/views/shade.rs`: import `MouseArea`, then wrap the element returned by
  `view_shade_strip` in `MouseArea::new(...).on_double_click(Message::ToggleWindowShade)`. Keep
  the existing `bevel::raised_panel(...).style(style::title_bar_style())` as the `MouseArea`'s
  content so the strip's chrome and `SHADE_STRIP_HEIGHT` geometry stay exactly as they are.
  Extend `view_shade_strip`'s doc comment to name the double-click restore and to record that
  `MouseArea` forwards events to its content first, so the strip's buttons, time readout, seek
  slider, and volume slider keep their own behavior and a double-click on them does not unshade.
  Reuse `handlers::window::toggle_window_shade` as it stands; it already restores the saved size
  or measures the first shade.
- `src/ui/views/mini_transport.rs`: while stopped, `mini_transport_row_with_optional_seek` wraps
  the readout in `MouseArea::new(readout).on_double_click(Message::Ignored)` so the shared compact
  row captures the press with the existing no-op handler instead of letting it reach the shade
  strip's restore; a playing or paused readout keeps its `MouseArea::on_press` publishing
  `Message::ToggleTimeDisplay`. The library footer shares the row and gets the same no-op wrapper.
- `src/ui/views/tests.rs`: add `shade_strip_double_click_restores_the_window`. Build
  `view_shade_strip` in `TransportMode::Stopped` with no position or duration, so no seek slider
  is drawn and the centered fill area is blank; lay it out with `layout_capture::headless_renderer`
  and drive `iced_runtime::user_interface::UserInterface::update` at the point
  `(capture.size().width / 2.0, capture.size().height / 2.0)` (the fill spacer between the
  readout and the volume slider, not a control). Send one left press and release and assert no
  `Message::ToggleWindowShade` is published, then send a second left press and assert exactly one
  `Message::ToggleWindowShade`, so the test proves both the wiring and that a single click stays
  inert. Add `shade_strip_double_clicks_on_controls_do_not_restore_the_window`, which captures
  the stopped readout, Play/Pause button, seek slider (`MINI_SEEK_ID`), and volume slider
  (`MINI_VOLUME_ID`) bounds and drives press/release/press/release at each: none publishes
  `ToggleWindowShade`, the stopped readout publishes only `Message::Ignored`, and the button and
  sliders keep their own messages.
- `README.md`: in the title-bar sentence, change the restore wording to name a double-click on
  the rolled-up strip alongside the shade button and the title bar's double-click. Do not edit
  the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Window shade`, add the guarantee that a double-click on the strip
  publishes the same `Message::ToggleWindowShade` as the title-bar double-click and restores the
  window; name `views::shade::view_shade_strip` and `handlers::window::toggle_window_shade` as
  the enforcers and the new test under Pinned by, and extend the History note.

**Files touched.** `src/ui/views/shade.rs`, `src/ui/views/mini_transport.rs`,
`src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A double-click on the shade strip's blank chrome publishes one `Message::ToggleWindowShade`
  and restores the window exactly as the shade button and the title bar's double-click do; a
  single click publishes nothing.
- The strip's four buttons, time readout, seek slider (when drawn), and volume slider keep their
  current presses, drags, toggles, and wheel behavior, and the strip keeps `SHADE_STRIP_HEIGHT`
  and its blue chrome unchanged.
- No new `Message` variant, handler, or player flag is added, and normal mode is untouched.
- The new test passes and `make check` passes.

### Show and hide the library section with a PL button (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

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

### Add the preview seek slider to the window-shade strip (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

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

### Drive the transport with the system media keys (planned 2026-10-24 by the plan role) — done 2026-10-24 by the feature role

**Goal.** A focused Nostalgia window answers the classic Winamp letters and arrows, but the
system media keys a keyboard or headset sends are unbound. Bind the transport media keys to the
same `Message`s the on-screen controls use: `MediaPlayPause` to `Message::PlayPause` (the button's
toggle), `MediaPlay` to `Message::Play`, `MediaPause` to `Message::Pause`, `MediaStop` to
`Message::Stop`, `MediaTrackNext` to `Message::NextTrack`, and `MediaTrackPrevious` to
`Message::PreviousTrack`. The keys therefore behave exactly like pressing Play/Pause, Stop,
Next, or Previous, including their existing Shuffle, Repeat, and end-of-list rules; they change
no browse, search, or LCD state beyond what those controls already change.

**Approach.**

- `src/ui/shortcuts.rs`: add the six `Key::Named(key::Named::Media…)` arms to `shortcut` beside
  the existing named-key arms, returning the messages above. Update the `message_for` and
  `shortcut` doc comments to list the media-key transport route along with the classic letters.
  The existing chord guard stays first, so Ctrl/Alt/Logo chords with a media key remain unbound,
  and Shift-held media keys work like the plain keys.
- `src/ui/message.rs`: extend the comments on `Play` and `Pause` to say the explicit media keys
  share the X and C routes, rather than naming only the keyboard letters.
- Tests. `src/ui/shortcuts.rs`: add `message_for_maps_media_keys_to_the_transport`, mapping each
  of the six named keys with `Modifiers::NONE` and `Modifiers::SHIFT` through `message_for` and
  asserting the matching `Message` with `matches!`. Add the six keys to the `shortcut_ignores_chord_modifiers`
  key list so Ctrl/Alt/Logo + media key stay unbound. `src/ui/tests.rs`: add
  `media_play_pause_key_toggles_playback`, modeled on `play_pause_toggles_is_playing`: start from
  `player_playing_preview` with a `RecordingAudio`, map a `Key::Named(key::Named::MediaPlayPause)`
  press through `super::shortcuts::message_for`, assert it is `Message::PlayPause`, drive it
  through `update` with `assert_transport_settled`, and assert the shared `is_playing` flag flips
  each time. Leave the existing `Message::Play`, `Pause`, `Stop`, `NextTrack`, and `PreviousTrack`
  handler tests to pin the behavior each media key reaches.
- `README.md`: in the Usage keyboard sentence, after the F5 clause, say the system media keys
  Play/Pause, Stop, Next track, and Previous track drive the same transport while the window is
  focused. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a short `## Media keys` section after `## Browse keyboard scrolling`
  with the guarantee, `shortcuts::shortcut` as **Enforced by**, the two new test names as
  **Pinned by**, and a **History** line for this addition.

**Files touched.** `src/ui/shortcuts.rs`, `src/ui/message.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- Plain or Shift-held `MediaPlayPause`, `MediaPlay`, `MediaPause`, `MediaStop`, `MediaTrackNext`,
  and `MediaTrackPrevious` map to `PlayPause`, `Play`, `Pause`, `Stop`, `NextTrack`, and
  `PreviousTrack` respectively; Ctrl/Alt/Logo chords with those keys map to nothing.
- Driving the mapped media Play/Pause message through `update` toggles playback exactly like the
  on-screen Play/Pause button, and the remaining media keys reach the same already-tested
  transport handlers as their controls and classic letters.
- The existing classic letter, arrow, and named-key bindings are unchanged.
- The new tests pass, `make check` passes, and README and `docs/invariants.md` describe the
  media-key transport route.

### Show and hide the equalizer panel with E (planned 2026-10-23 by the plan role) — done 2026-10-24 by the feature role

**Goal.** The normal-mode transport row has an `EQ` button beside the balance slider that shows
and hides the equalizer panel, but the keyboard has no route to it. Bind plain or Shift-held `E`
to the same `Message::ToggleEqualizerPanel` the button sends, matching Winamp's equalizer key.
The panel and its `NOSTALGIA EQUALIZER` title band hide or reappear, the browse list moves into
the freed height without resizing the window, and the equalizer's on/off state, preamp, band
gains, preset, and audio forwarding are untouched. Shade mode builds no equalizer panel, so `E`
there leaves the visibility flag unchanged instead of flipping a hidden setting. Ctrl/Alt/Logo
chords with E remain unbound, and because E is a character key the focused `Search library` box
keeps it for editing its query like every typed character.

**Approach.**

- `src/ui/shortcuts.rs`: add a `Key::Character(character)` arm matching `"e"` beside the existing
  A and T arms, returning `Message::ToggleEqualizerPanel`. Update the `message_for` and `shortcut`
  docs to list E with the other letter bindings.
- `src/ui/handlers/mixer.rs`: guard `toggle_equalizer_panel` with `if player.shaded { return
  Task::none(); }` before flipping `player.eq_panel_visible`, and update its doc comment to say
  the E key shares the arm and that shade mode is a no-op. The normal-mode `EQ` button path is
  otherwise unchanged: the handler still only flips the layout flag and schedules no work.
- Tests. `src/ui/shortcuts.rs`: add `"e"` to the `assert_letter_bindings` helper so
  `message_for_maps_each_bound_letter` and `shortcut_matches_letters_case_insensitively` cover it,
  and add `Key::Character("e".into())` to `shortcut_ignores_chord_modifiers`'s key list so
  Ctrl/Alt/Logo + E stay unbound. `src/ui/tests.rs`: add
  `e_toggles_the_equalizer_panel_from_the_keyboard`, modeled on
  `t_toggles_the_time_display_from_the_keyboard`: start from `test_player()` with the panel open,
  build a plain and a Shift-held E key event, map each through
  `super::shortcuts::message_for`, assert it is `Message::ToggleEqualizerPanel`, drive it through
  `update`, and assert `eq_panel_visible` flips each time. Add
  `e_in_shade_mode_leaves_the_equalizer_panel_alone`: set `player.shaded`, drive the mapped E
  message, and assert `eq_panel_visible` is unchanged and no work is scheduled. Leave
  `toggle_equalizer_panel_hides_and_restores_the_section` to pin the panel layout and the
  untouched equalizer state.
- `README.md`: in the Usage keyboard sentence, after the A and T clause, say E shows or hides the
  equalizer panel, exactly like the `EQ` button beside the balance slider. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: extend the `## UI layout` equalizer bullet (rather than adding a sibling
  section) so the `EQ` toggle's guarantee also covers plain or Shift-held E dispatching the same
  `Message::ToggleEqualizerPanel`, with the panel layout, the untouched equalizer state, the
  shade-mode no-op, and Ctrl/Alt/Logo chords unbound. Name `shortcuts::shortcut` beside
  `handlers::mixer::toggle_equalizer_panel` in **Enforced by**, add the two new test names to
  **Pinned by**, and record the addition in **History**.

**Files touched.** `src/ui/shortcuts.rs`, `src/ui/handlers/mixer.rs`, `src/ui/tests.rs`,
`README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Plain or Shift-held `E` maps to `Message::ToggleEqualizerPanel`; Ctrl/Alt/Logo chords with E map
  to nothing.
- Driving the mapped message in normal mode flips `WinampPlayer::eq_panel_visible`, so the
  `NOSTALGIA EQUALIZER` band and panel hide or reappear and the browse list takes the freed
  height, while the equalizer's on/off state, curve, preset, and audio forwarding stay unchanged
  and no task is scheduled.
- In shade mode the mapped message leaves `eq_panel_visible` unchanged and schedules no work.
- The new tests pass, `make check` passes, and README and `docs/invariants.md` describe the E
  route to the equalizer panel toggle.

### Reload the library from the keyboard with F5 (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Every browse level offers a `Reload` button, but the keyboard has no route to it. Bind
plain or Shift-held `F5` to the same `Message::ReloadLibrary` the button sends, so F5 clears an
active search, returns the browse view to Artists, and refetches the artists list through the
existing clear-then-issue path. The reload drops the previous artists rows and any earlier
failure, shows the loading placeholder until the reply lands, and starts the reloaded list at its
first row, exactly as the button does; it leaves the typed search query, the other browse
buffers, and playback alone. This adds only a key binding: the message, handler, and button stay
unchanged. Ctrl/Alt/Logo chords with F5 remain unbound, and F5 is a library refresh rather than
a text-editing key, so this plan changes no search-box editing behavior.

**Approach.**

- `src/ui/shortcuts.rs`: in `shortcut`, map `Key::Named(key::Named::F5)` to
  `Message::ReloadLibrary` beside the Home/End arms, after the existing Control/Alt/Logo guard,
  so plain and Shift-held F5 reload and chords stay the window manager's. Update the `message_for`
  and `shortcut` docs to list F5 with the other browse keys.
- Tests. `src/ui/shortcuts.rs`: add `message_for_maps_f5_to_reload` covering plain and Shift-held
  F5, and extend `shortcut_ignores_chord_modifiers`'s key list with
  `Key::Named(key::Named::F5)`. `src/ui/tests.rs`: add
  `f5_reloads_the_library_from_the_keyboard`, modeled on `backspace_steps_back_from_albums`: put
  the player in a non-Artists view with an active search, a typed query, stale artists rows, and
  a populated other browse buffer; build the F5 key event, map it through
  `super::shortcuts::message_for`, assert it is `Message::ReloadLibrary`, drive it through
  `update`, assert the arm schedules work, then assert the same observable state as
  `reload_library_returns_to_artists_and_refetches_them` (Artists view, search inactive, typed
  query and other buffer intact, artists rows and error cleared, loading set).
- `README.md`: in the Usage keyboard sentence, after the Ctrl+F clause, say F5 refetches the
  library and returns to the Artists list, exactly like the `Reload` button. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: extend the existing `## Browse hierarchy` section (rather than adding a
  sibling) so its Reload guarantee also covers plain or Shift-held F5 taking the same
  `Message::ReloadLibrary` path as the button, with Ctrl/Alt/Logo chords unbound. Name
  `shortcuts::shortcut` as the enforcer, add the two new test names to **Pinned by**, and record
  the addition in **History**.

**Files touched.** `src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- Plain or Shift-held `F5` maps to `Message::ReloadLibrary`; Ctrl/Alt/Logo chords with F5 map to
  nothing.
- Driving the mapped message through `update` produces the Reload button's observable behavior:
  the Artists view shows, an active search is cleared, the typed query and the other browse
  buffers and playback are unchanged, the artists rows and error are dropped, the artists list
  is loading, and the arm schedules the artists fetch.
- The new tests pass, `make check` passes, and README and `docs/invariants.md` describe the F5
  route to Reload.

### Scroll the browse list one page with Page Up and Page Down (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** The browse list scrolls with the mouse wheel, snaps to its ends with `Home`/`End`, and
tracks the playing row while stepping, but the keyboard cannot page through a long list. Bind
plain or Shift-held `Page Down` to scroll the showing browse list forward by one page, and
`Page Up` to scroll it back by one page, on every normal-mode browse level. A page is a fixed
`BROWSE_PAGE_HEIGHT`, defined as eight `BROWSE_ROW_HEIGHT` rows, so the step does not depend on
the viewport size. Both keys are view-scroll operations only: they change no view, rows or row
order, selection, typed query, search-active flag, or playback, and schedule no fetch or
playback task. In shade mode no browse list exists, so both are no-ops; iced clamps the result,
so paging past either end leaves the list at that end and a short list stays put.

**Approach.**

- `src/ui/views/browse/rows.rs`: add
  `pub const BROWSE_PAGE_HEIGHT: f32 = 8.0 * BROWSE_ROW_HEIGHT;` beside `BROWSE_ROW_HEIGHT`, with a
  doc comment saying one keyboard page is eight rows of the same pitch, that the page keys scroll
  by this amount rather than by the viewport height, and that `iced` clamps the result. Extend the
  `BROWSE_LIST_ID` doc to name the page-scroll operation beside the snap-to-top and scroll-to-row
  operations.
- `src/ui/views/browse.rs`: re-export `BROWSE_PAGE_HEIGHT` from `rows` beside the existing
  `BROWSE_ROW_HEIGHT`.
- `src/ui/views/mod.rs`: include `BROWSE_PAGE_HEIGHT` in the `pub use browse::{...}` list.
- `src/ui/message.rs`: add `ScrollBrowsePageUp` and `ScrollBrowsePageDown` beside
  `ScrollBrowseToTop`/`ScrollBrowseToBottom`, documented as `Page Up`/`Page Down` scheduling a
  one-page `scroll_by` on the browse list and changing no state.
- `src/ui/handlers/browse.rs`: add a private
  `fn scroll_browse_by(delta: f32) -> Task<Message>` that calls
  `iced::widget::operation::scroll_by(views::BROWSE_LIST_ID, AbsoluteOffset { x: 0.0, y: delta })`,
  then `pub(in crate::ui) fn scroll_browse_page_up() -> Task<Message>` calling it with
  `-views::BROWSE_PAGE_HEIGHT` and `scroll_browse_page_down()` calling it with
  `views::BROWSE_PAGE_HEIGHT`. Document that the page step follows the list's current offset
  (unlike the absolute `scroll_browse_to_row`), touches only the scroll offset, and has no target
  in shade mode.
- `src/ui/mod.rs`: route `Message::ScrollBrowsePageUp` and `Message::ScrollBrowsePageDown` to those
  handlers beside the existing `ScrollBrowseToTop`/`ScrollBrowseToBottom` arms.
- `src/ui/shortcuts.rs`: map `Key::Named(key::Named::PageDown)` to
  `Message::ScrollBrowsePageDown` and `Key::Named(key::Named::PageUp)` to
  `Message::ScrollBrowsePageUp` beside the Home/End arms. The existing Control/Alt/Logo guard
  stays first, so chords with Page Up or Page Down remain unbound. Update the `message_for` and
  `shortcut` docs to name the page keys.
- Tests. `src/ui/shortcuts.rs`: add `message_for_maps_page_keys_to_browse_paging` covering plain
  and Shift-held Page Up and Page Down, and extend `shortcut_ignores_chord_modifiers`'s key list
  with both page keys so Ctrl/Alt/Logo chords return `None`. `src/ui/tests.rs`: extend
  `BrowseScrollEffects` with `scrolled_by: Option<AbsoluteOffset>` (the non-optional form
  `operation::scroll_by` takes), record it in the `ScrollableOperation::scroll_by` capture, and
  include it in `any()`; make the existing snap and scroll-to helpers also assert
  `scrolled_by == None` so an arm cannot schedule two scroll kinds. Add
  `assert_task_scrolls_browse_list_by(task, what, expected_y)` that captures the operation and
  asserts `x == 0.0`, `y == expected_y`, and no snap or scroll-to effect, then add
  `scroll_browse_page_keys_schedule_page_sized_scrolls`: drive `Message::ScrollBrowsePageDown`
  and assert `+BROWSE_PAGE_HEIGHT`, drive `Message::ScrollBrowsePageUp` and assert
  `-BROWSE_PAGE_HEIGHT`, and assert the view and shared playback state are unchanged. Import
  `BROWSE_PAGE_HEIGHT` from `super::views` in the test module.
- `README.md`: in the Usage keyboard sentence, after the Home/End clause, say `Page Up` and
  `Page Down` scroll the browse list one page at a time in the matching direction. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: extend the existing `## Browse keyboard scrolling` section (rather than
  adding a duplicate section) so its guarantee also covers plain or Shift-held Page Up/Page Down
  scrolling the showing browse list by `BROWSE_PAGE_HEIGHT` through iced's `scroll_by` on
  `BROWSE_LIST_ID`, with the view, rows, selection, search state, and playback untouched, no fetch
  or playback task scheduled, no target in shade mode, and iced clamping at the ends. Name
  `shortcuts::shortcut`, the two new `Message` variants, the three `handlers::browse` scroll
  functions, `views::BROWSE_PAGE_HEIGHT`, and `views::BROWSE_LIST_ID` as the enforcers; add the new
  shortcut and UI test names to **Pinned by**; record the addition in **History**.

**Files touched.** `src/ui/views/browse/rows.rs`, `src/ui/views/browse.rs`,
`src/ui/views/mod.rs`, `src/ui/message.rs`, `src/ui/handlers/browse.rs`, `src/ui/mod.rs`,
`src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Plain or Shift-held `Page Down` schedules exactly one `scroll_by` on `BROWSE_LIST_ID` with
  `y = +BROWSE_PAGE_HEIGHT`, and `Page Up` one with `y = -BROWSE_PAGE_HEIGHT`; the step is
  `8.0 * BROWSE_ROW_HEIGHT` and is relative to the list's current offset.
- Neither key changes the current view, rows or row order, selection, typed query, search-active
  flag, or playback, and neither schedules a fetch or playback task; in shade mode both are no-ops,
  and Ctrl/Alt/Logo chords with either key remain unbound.
- Paging past either end leaves the list at that end (iced clamps the offset), and a list shorter
  than the viewport does not move.
- The new tests pass, `make check` passes, and README and `docs/invariants.md` describe the page
  keys and their bounds.

### Match album titles in the library search (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** The built-in sample library's search currently matches a song only when the query
appears in its title or artist, so typing an album title that the browse lists show (for example
`Second Record`) returns nothing. Extend `sample_songs_matching` to also match the title of the
album a song belongs to, case-insensitively, while keeping the existing title and artist matches
and the library-order result. This makes the offline search find an album's songs by the name the
user sees; the signed-in Apple Music path keeps returning whatever the REST search service
decides.

**Approach.**

- `src/apple_music/sample.rs`: add a small private helper
  `sample_album_titles_by_id() -> std::collections::HashMap<&'static str, String>` that walks
  `sample_albums_in_library_order()` and maps each `Album::id` to its case-folded
  `Album::title`. In `sample_songs_matching`, build the map once per call and keep a song when
  its title, its artist, or its album's folded title contains the folded query; keep the
  existing `sample_songs_in_library_order()` walk and clone, so the result order does not change.
  Update the helper's doc comment to name title, artist, and album title.
- `src/apple_music/browse.rs`: update `AppleMusicService::search_songs`'s doc comment (and the
  `sample_songs_matching` description beside it) to say the sample fallback matches title,
  artist, or album title. No logic changes: the blank-query guard, trimming, and the signed-in
  REST branch stay as they are.
- `src/apple_music/tests.rs`: add
  `search_songs_matches_the_sample_library_by_album_title`, using `test_service()` and
  `assert_ids`. Assert `"  SECOND RECORD  "` returns exactly `song-4` (the only song on
  `Second Record`), and `"first record"` returns `song-1`, `song-2`, and `song-3` in library
  order. Update the comment above
  `search_songs_matches_the_sample_library_by_title_or_artist` so it says this test pins the
  title and artist matches while the new test pins the album-title match; keep its assertions
  unchanged.
- `README.md`: in the search-box sentence, add that the sample fallback matches the query
  case-insensitively against a song's title, artist, and album title. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a short `## Library search matching` section after `## Search
  focus`. State that a sample-library search returns every song whose title, artist, or album
  title contains the trimmed query case-insensitively, in library order; that a blank query is
  rejected; and that the signed-in path returns the REST service's results. Name
  `sample_songs_matching` and `sample_album_titles_by_id` in `src/apple_music/sample.rs` and
  `AppleMusicService::search_songs` in `src/apple_music/browse.rs` as the enforcers, list the
  title/artist/album tests and the blank-query test under **Pinned by**, and record the addition
  in **History**.

**Files touched.** `src/apple_music/sample.rs`, `src/apple_music/browse.rs`,
`src/apple_music/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- With no session, `search_songs` matches a sample song when the query appears case-insensitively
  in its title, its artist, or the title of the album named by its `album_id`.
- `"Second Record"`, `"second record"`, and the trimmed query all return exactly `song-4`;
  `"first record"` returns `song-1`, `song-2`, and `song-3` in library order; title and artist
  matches are unchanged.
- A query matching no title, artist, or album title still returns an empty list, and a blank
  query is still rejected with the existing error.
- No new message, handler, state, dependency, or UI change is introduced; the signed-in REST
  search path is untouched.
- The new test passes, `make check` passes, and README and `docs/invariants.md` describe the
  album-title match.

### Clear the library search from a button beside the search box (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** The `Search library` box stores its typed text and searches on Enter; `Back` leaves an
active search, `Reload` clears the active flag but keeps the typed query, and a blank submit is
deliberately a no-op. There is no one-press way to empty the box. Add a `Clear` button beside the
search box on every normal-mode browse level: it empties the typed query and, when a search is
showing, leaves that search exactly as `Back` does (returns to Artists, restores the artists
list's library order, and snaps the list to its top). With no search active it changes only the
box text. It schedules no fetch and touches no playback. Shade mode builds no search row, so it
has no Clear button.

**Approach.**

- `src/ui/message.rs`: add `ClearSearch` after `FocusSearch`, documented as the Clear button's
  payload-free message: empty the box's query and, when a search is active, leave it exactly as
  Back does; schedule no fetch or playback.
- `src/ui/handlers/browse.rs`: add
  `pub(in crate::ui) fn clear_search(player: &mut WinampPlayer) -> Task<Message>` that clears
  `player.search_query` and, only when `player.search_active` is set, returns `back(player)`;
  otherwise it returns `Task::none()`. Document that reusing `back` is what makes the active case
  identical to the Back button (deactivate the search, switch to Artists,
  `restore_library_order`, `scroll_browse_to_top`) and that the inactive case mutates only the
  query. It must not refetch or clear any browse rows.
- `src/ui/mod.rs`: route `Message::ClearSearch` to `handlers::browse::clear_search` beside the
  `FocusSearch` arm.
- `src/ui/views/browse.rs`: add
  `pub fn view_clear_search_button() -> Element<'static, Message>` built with
  `labeled_button("Clear", Message::ClearSearch)` like `view_top_button`, and add
  `pub fn view_search_row(query: &str) -> Element<'_, Message>` returning a `Row` holding the
  existing `view_search_box(query)` with `Length::Fill` followed by
  `view_clear_search_button()`, so the box keeps the width it had and Clear follows it. Update
  the module and search-box docs to name the Clear button; `view_search_box` keeps its current
  callbacks and id.
- `src/ui/views/mod.rs`: re-export `view_search_row` beside the other browse builders; the view
  tests import `view_clear_search_button` and `view_search_box` directly from `views::browse`, as
  they already do for `view_reload_button`, because neither builder has a non-test caller.
- `src/ui/view.rs`: replace the `views::view_search_box(&player.search_query)` push with
  `views::view_search_row(&player.search_query)` and update the comment to say the row carries
  Clear.
- Tests. `src/ui/tests.rs`: add
  `clear_search_without_an_active_search_clears_only_the_query` (seed a query, a non-Artists
  view, and rows; drive `Message::ClearSearch`; assert with `assert_message_schedules_no_work`
  that it schedules nothing, then that the query is empty while the view, rows, selection,
  `search_active`, and playback are unchanged), and
  `clear_search_while_a_search_is_active_returns_to_artists_and_clears_the_query` (set
  `search_active`, `CurrentView::Songs`, a query, and search-result songs; drive the message;
  assert the query is empty, the flag is false, and the view is Artists, and use
  `assert_task_snaps_browse_list_to_top` to pin the same snap-to-top task Back schedules, while
  the search-result rows stay in the songs buffer). Add
  `clear_search_with_an_empty_query_schedules_no_work` for the empty inactive case.
  `src/ui/views/tests.rs`: add `view_clear_search_button_constructs` and
  `view_search_row_draws_clear_after_the_search_box` (lay out
  `view_search_row("Opening")` with the headless renderer; capture the `TextInput` bounds through
  a small `Operation::text_input` implementation, as `view_search_box_registers_its_focus_id`
  captures `Operation::focusable`, and capture the `Clear` label with
  `layout_capture::capture_text`; assert `Clear`'s left edge is at or after the text input's
  right edge and the two vertical ranges overlap), and extend the existing browse-action
  construct test with the new builders.
- `README.md`: in the sentence documenting the search box, say a `Clear` button beside the box
  empties the typed query; when a search is showing it also returns to the artists list like
  Back, and otherwise it changes only the text. Do not edit the `<!-- tumwater:status:start -->`
  block.
- `docs/invariants.md`: add a short `## Library search clearing` section after `## Search
  focus`, stating the guarantee above; name `Message::ClearSearch`,
  `handlers::browse::clear_search`, `handlers::browse::back`,
  `views::browse::view_clear_search_button`, and `views::browse::view_search_row` as the
  enforcers; list the new test names under **Pinned by**; and record the addition in
  **History**.

**Files touched.** `src/ui/message.rs`, `src/ui/handlers/browse.rs`, `src/ui/mod.rs`,
`src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/view.rs`, `src/ui/tests.rs`,
`src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The `Clear` button renders beside the search box on every normal-mode browse level; shade mode
  builds neither the box nor the button.
- With no active search, pressing it empties `search_query` and changes nothing else: no task,
  fetch, scroll, view or row change, selection change, or playback change.
- With a search active, pressing it empties the query, deactivates the search, returns to the
  Artists view, restores the artists list's library order, and snaps the browse list to its top,
  exactly as Back does; it schedules no fetch or playback and leaves the search-result rows in
  the songs buffer.
- Pressing it with an empty query and no active search is a no-op that schedules no work.
- The new tests pass, `make check` passes, and README and `docs/invariants.md` describe the button
  and the two cases.

### Snap the browse list to its ends with Home and End (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** The browse list can be scrolled with the mouse wheel and sent back to its first row with
`Top`, and the keyboard already covers opening (`Enter`), going back (`Backspace`), and focusing
search (`Ctrl+F`), but there is no keyboard route to the first or last row. Bind `Home` to the
same snap-to-first-row action as `Top` and add `End` as the matching snap-to-last-row action, from
any normal-mode browse level. Both are view-scroll operations only: they change no view, rows,
selection, search text or active flag, or playback, and schedule no fetch or playback task. In
shade mode the browse list is not in the tree, so both are no-ops; while the `Search library` box
is focused, its `TextInput` keeps `Home` and `End` for editing, as it keeps `Backspace`.

**Approach.**

- `src/ui/message.rs`: add `ScrollBrowseToBottom` beside `ScrollBrowseToTop`, documented as the
  `End` key's counterpart to the Top button: it schedules a snap of the browse list to its last
  row and changes no state.
- `src/ui/handlers/browse.rs`: factor the existing `scroll_browse_to_top`'s
  `iced::widget::operation::snap_to(views::BROWSE_LIST_ID, offset)` call into a private
  `fn snap_browse_list(offset: RelativeOffset) -> Task<Message>`; keep
  `pub(in crate::ui) fn scroll_browse_to_top()` calling it with `RelativeOffset::START`, and add
  `pub(in crate::ui) fn scroll_browse_to_bottom()` calling it with `RelativeOffset::END`. Both
  target the stable `views::BROWSE_LIST_ID` every level's `scrollable_list` carries. Update the
  `scroll_browse_to_top` doc to name the Home binding and the new function's doc to name End, and
  state that neither touches rows, selection, the view, search, or playback.
- `src/ui/mod.rs`: route `Message::ScrollBrowseToBottom` to
  `handlers::browse::scroll_browse_to_bottom()` beside the `ScrollBrowseToTop` arm.
- `src/ui/shortcuts.rs`: in `shortcut`, map `Key::Named(key::Named::Home)` to
  `Message::ScrollBrowseToTop` and `Key::Named(key::Named::End)` to
  `Message::ScrollBrowseToBottom`, beside the `Enter`/`Backspace` arms. The existing
  Control/Alt/Logo guard stays first, so `Ctrl+Home`, `Ctrl+End`, and their Alt/Logo variants
  remain unbound. Update the `message_for` and `shortcut` docs to list Home and End with the other
  browse keys.
- Tests. `src/ui/shortcuts.rs`: add `message_for_maps_home_and_end_to_browse_scroll` covering
  plain and Shift-held Home and End, and extend `shortcut_ignores_chord_modifiers` (or add those
  assertions beside it) so Ctrl/Alt/Logo + Home and Ctrl/Alt/Logo + End return `None`.
  `src/ui/tests.rs`: generalize `assert_task_snaps_browse_list_to_top` over the expected
  `RelativeOffset` (for example a new `assert_task_snaps_browse_list_to(offset, task, what)` with
  the existing top helper delegating to it), add
  `assert_task_snaps_browse_list_to_end`, and add
  `scroll_browse_to_bottom_schedules_a_scroll_task` mirroring
  `scroll_browse_to_top_schedules_a_scroll_task`: drive `Message::ScrollBrowseToBottom` through
  `update`, assert the captured operation snaps `BROWSE_LIST_ID` to `RelativeOffset::END`, and
  assert the current view and shared playback state are unchanged. Optionally cover both messages
  in one test, but pin both offsets.
- `README.md`: in the Usage keyboard sentence, after the Backspace/Back paragraph, say `Home`
  snaps the browse list to its first row and `End` to its last, like `Top`, and that the focused
  `Search library` box keeps both keys for editing its query. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a short `## Browse keyboard scrolling` section after
  `## Browse playing-row visibility` stating that a plain or Shift-held `Home` snaps the showing
  browse list to its first row and `End` to its last row through `snap_to` on `BROWSE_LIST_ID`;
  neither changes the view, rows, selection, search state, or playback, and both are no-ops in
  shade mode where no browse list is built; the focused search box's `TextInput` captures Home and
  End for editing, and Ctrl/Alt/Logo chords stay unbound. Name `shortcuts::shortcut`,
  `Message::ScrollBrowseToBottom`, `handlers::browse::scroll_browse_to_top`/
  `scroll_browse_to_bottom`, and `views::BROWSE_LIST_ID` as the enforcers, add the new test names
  to **Pinned by**, and record the addition in **History**.

**Files touched.** `src/ui/message.rs`, `src/ui/handlers/browse.rs`, `src/ui/mod.rs`,
`src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Plain or Shift-held `Home` snaps the showing browse list to its first row on every normal-mode
  browse level, exactly as the `Top` button does; plain or Shift-held `End` snaps it to its last
  row, and a list shorter than the viewport stays put.
- Neither key changes the current view, rows, selection, typed query, search-active flag, or
  playback, and neither schedules a fetch or playback task; in shade mode both are no-ops, and
  Ctrl/Alt/Logo chords remain unbound.
- While the `Search library` box is focused, Home and End keep editing its query rather than
  scrolling the list, matching Backspace's existing capture.
- The new tests pass, `make check` passes, and README and `docs/invariants.md` describe the keys
  and their bounds.

### Focus the library search box with Ctrl+F (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** The `Search library` box has no keyboard route: the user must click it before typing a
query, and the classic keys deliberately leave Control chords to the window manager. Add Ctrl+F
to focus that box from any normal-mode browse level, without changing
the current view, list, selection, typed query, or playback and without submitting a search. This
is a keyboard affordance only; clicking the box keeps working exactly as it does now.

**Approach.**

- `src/ui/views/browse.rs`: add `pub const BROWSE_SEARCH_ID: &str = "nostalgia-search-box";`
  beside `view_search_box`, and give the `TextInput` in `view_search_box` that id
  (`TextInput::new("Search library", query).id(BROWSE_SEARCH_ID)`). The id is static and the
  builder stays pure. Update the builder and module docs to name the id and the Ctrl+F route.
- `src/ui/views/mod.rs`: re-export `BROWSE_SEARCH_ID` with the other browse exports.
- `src/ui/message.rs`: add `Message::FocusSearch` beside the search messages, documented as
  focusing the `Search library` box via `BROWSE_SEARCH_ID`; it changes no browse or playback state
  and submits nothing.
- `src/ui/handlers/browse.rs`: add `pub(in crate::ui) fn focus_search() -> Task<Message>`
  returning `iced::widget::operation::focus(views::BROWSE_SEARCH_ID)`. It schedules only the focus
  operation and mutates no state; in shade mode the search box is not in the tree, so the
  operation has no matching focusable and the key is a no-op.
- `src/ui/mod.rs`: route `Message::FocusSearch` to `handlers::browse::focus_search` beside the
  `SearchChanged`/`SearchSubmitted` arms.
- `src/ui/shortcuts.rs`: in `shortcut`, before the existing Control/Alt/Logo chord guard, map a
  Control-held `f` (ASCII-case-insensitive, with or without Shift) to `Message::FocusSearch`,
  while Alt+F and Logo+F stay unbound and every other Control/Alt/Logo chord keeps returning
  `None`. Update the module and `message_for`/`shortcut` docs to name the one Control exception.
- Tests. `src/ui/shortcuts.rs`: add `message_for_maps_ctrl_f_to_focus_search` covering Ctrl+F and
  Ctrl+Shift+F, and assert plain `f`, Alt+F, and Logo+F remain unbound (extend
  `shortcut_ignores_chord_modifiers` or add those assertions beside it). `src/ui/tests.rs`: add
  `ctrl_f_focuses_the_search_box_and_changes_no_browse_state` — start from an Albums view with a
  non-empty `search_query`, a selected row, and a current track, lay out the root `view`, drive
  `Message::FocusSearch` through `update`, and apply the emitted focus operation to the laid-out
  tree. Then run a custom focusable capture over that tree and assert exactly one focusable
  reports focused, the one tagged `BROWSE_SEARCH_ID`. Also assert the view, list rows, selection,
  `search_query`, `search_active`, and shared playback state are unchanged. Use the custom
  `Operation`/`Focusable` capture shape of
  `captured_root_view_volume_slider` in the same file. `src/ui/views/tests.rs`: add
  `view_search_box_registers_its_focus_id`, laying out `view_search_box` and asserting a
  `BROWSE_SEARCH_ID` focusable is present.
- `README.md`: in the Usage keyboard sentence, after Backspace, add that Ctrl+F focuses the
  `Search library` box without searching or changing the current list, and that the box then takes
  typed keys as usual. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Search focus` section after `## Browse hierarchy` stating that
  Ctrl+F from any normal-mode browse level focuses the `Search library` box, submits nothing, and
  changes no query, view, list, selection, or playback; Alt/Logo+F and other Control chords stay
  unbound, and shade mode has no search box so the key is a no-op. Name `shortcuts::shortcut`,
  `Message::FocusSearch`, `browse::focus_search`, and `views::view_search_box`/
  `BROWSE_SEARCH_ID` as the enforcers, add the new tests to **Pinned by**, and record the addition
  in **History**.

**Files touched.** `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/message.rs`,
`src/ui/handlers/browse.rs`, `src/ui/mod.rs`, `src/ui/shortcuts.rs`, `src/ui/tests.rs`,
`src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Ctrl+F (with or without Shift) from every normal-mode browse level moves keyboard focus to the
  `Search library` box; plain F and every Alt/Logo chord keep their current behavior, other
  Control chords remain unbound, and in shade mode Ctrl+F does nothing.
- Focusing the box changes no browse view, list rows, selection, typed query, search-active flag,
  or playback state, and submits no search; typing in the focused box edits the query exactly as a
  click does.
- The new tests pass, `make check` passes, and README and `docs/invariants.md` describe the key and
  its bounds.

### Reload the library from a button above the browse list (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** The artists list is fetched at boot and again only when the startup sign-in stores a
session (or when Retry re-runs it after a failure), while the other browse levels refetch only
when they are opened, so a library that changed on another device, or a stale sample-library
list, can only be replaced by restarting the app. Add a `Reload` button
beside `Top` and `Sort` that refetches the top-level artists list, clears an active search, and
returns the browse view to Artists. It reuses the existing fetch path and request generations,
adds no dependency, and leaves playback and the other browse buffers alone (the flat lists
refetch when next opened).

**Approach.**

- `src/ui/message.rs`: add a `Message::ReloadLibrary` variant beside the browse-navigation
  messages (`ScrollBrowseToTop`, `SortBrowse`), documented as refetching the top-level artists
  list and returning to Artists, clearing an active search but not the search box's text, and
  leaving playback and the other lists alone. Do not widen `Message::LoadArtists`: boot and the
  post-sign-in reload both send it and must not reset the user's current browse level.
- `src/ui/handlers/browse.rs`: add `pub(in crate::ui) fn reload_library(player: &mut
  WinampPlayer) -> Task<Message>`. It sets `player.search_active = false`, sets
  `player.current_view = CurrentView::Artists`, and returns `fetch_at_top(fetch::load_artists(player))`
  so the normal clear-before-fetch, loading flag, error clearing, request generation, artists
  store, and snap-to-top all come from the existing paths. It must not touch
  `player.songs_origin`, the other `BrowseList` buffers, or `AppState`.
- `src/ui/views/browse.rs`: add `pub fn view_reload_button() -> Element<'static, Message>` built
  with the existing `labeled_button("Reload", Message::ReloadLibrary)`, and push it after
  `view_sort_button()` in `view_browse_nav_buttons` so Top, Sort, and Reload share one row. Update
  the module and nav-row docs to name Reload and its return-to-Artists behavior.
- `src/ui/views/mod.rs`: no re-export: the builder is reached through the nav row, and a
  module-private re-export used only by tests would trip the unused-import lint. The views tests
  import `view_reload_button` from `browse` directly.
- `src/ui/view.rs`: update the nav-row comment from "Top and Sort" to "Top, Sort, and Reload"; the
  existing unconditional `view_browse_nav_buttons()` push makes the button available on every
  browse level without a new root branch.
- `src/ui/mod.rs`: route `Message::ReloadLibrary` to `handlers::browse::reload_library` beside the
  other browse-navigation arms.
- Tests. `src/ui/tests.rs`: add `reload_library_returns_to_artists_and_refetches_them` using
  `test_player`, seeding `CurrentView::AllSongs`, an active search with `search_query` set, an
  existing `all_songs` row, and an artists error plus rows. Drive `Message::ReloadLibrary`; assert
  the view is Artists, `search_active` is false, `search_query` is unchanged, the artists buffer
  is empty and loading with no error, the `all_songs` row and the shared state's current track /
  playing flag are untouched, then use `drive_fetch_and_assert_loaded` to assert the sample
  artists (`artist-1`, `artist-2`, `artist-3`) are stored. Add
  `reload_button_is_shown_on_every_browse_level` laying out the root view for each `BROWSE_VIEWS`
  entry and finding the `Reload` label, and add `browse_nav_buttons_place_reload_beside_sort`
  asserting Reload shares Top's and Sort's row and starts at or after Sort's right edge, the same
  shape as `browse_nav_buttons_place_sort_beside_top`. `src/ui/views/tests.rs`: build
  `view_reload_button()` in `now_playing_bar_and_browse_action_buttons_construct` and import it.
- `README.md`: in the browse-controls sentence that documents Top and Sort, add the Reload button,
  saying it refetches the library's artists and returns to the Artists list, clearing any active
  search while leaving the typed query in the box. Do not touch the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Browse hierarchy`, state that every level also offers Reload, which
  clears an active search, returns to Artists, refetches the artists list through the same fetch
  Retry uses, snaps the list to its first row, and leaves playback, the search text, and the other
  browse buffers alone; note that `Message::LoadArtists` keeps its boot, sign-in, and Retry
  behavior. Name `browse::reload_library`, `fetch::load_artists`, `fetch_at_top`,
  `views::browse::view_reload_button`, and `Message::ReloadLibrary` as the enforcers, add the
  three new tests to **Pinned by**, and record the addition in **History**. The views tests import
  the builder from `browse` rather than through a `views` re-export.

**Files touched.** `src/ui/message.rs`, `src/ui/mod.rs`, `src/ui/handlers/browse.rs`,
`src/ui/views/browse.rs`, `src/ui/view.rs`, `src/ui/tests.rs`, `src/ui/views/tests.rs`,
`README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Pressing `Reload` on any browse level clears an active search, switches the view to Artists,
  clears any previous artists rows and error, shows the loading placeholder, snaps the browse
  list to its first row, and stores the refetched artists when the reply lands. The typed search
  text, playback state, and the songs/albums/flat lists are unchanged.
- The Reload button renders on every browse level in the same row as Top and Sort, after Sort;
  the Artists Retry button still appears only after a failed artists fetch, and boot and the
  post-sign-in artists reload still do not change the current browse level.
- `make check` passes, and README and `docs/invariants.md` describe the action and its bounds.

### Remember the visualizer's shown/hidden choice across launches (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** The LCD strip's `V` control already toggles whether the spectrum bars draw, but the
choice resets on every launch: `WinampPlayer::visualizer_visible` lives only on the player, while
`Settings` snapshots `AppState` plus the two UI-owned display values (`time_display` and
`always_on_top`). Save the `V` choice in the same settings file, restore it before the first view,
and leave the stored spectrum and its poll behavior untouched. This is presentation-only and
independent of the queued A/T shortcut plan: it adds no message, handler, shortcut, playback
path, or window geometry, and adds no dependency.

**Approach.**

- `src/settings.rs`: add a private `visualizer_visible: bool` field to `Settings`, defaulting to
  `true` (the player's launch value), with `pub fn visualizer_visible(&self) -> bool` and
  `pub fn set_visualizer_visible(&mut self, visible: bool)` accessors. Write
  `visualizer_visible=true` or `visualizer_visible=false` in `to_text`; parse it in `from_text`
  with the shared `parse_into` helper so any other value leaves the default shown. Keep
  `from_state` leaving it `true` and the player overwriting it from its own flag before saving,
  exactly as `time_display_remaining` and `always_on_top` already work. Update the module and
  struct docs from "two UI-owned values" to three and say a missing or malformed line falls back
  to shown.
- `src/ui/player.rs`: update the `visualizer_visible` field doc to say the choice is saved and
  restored. In `persist_settings_if_changed`, after the snapshot's time and always-on-top values
  are set, add `snapshot.set_visualizer_visible(self.visualizer_visible);`. In
  `adopt_saved_settings`, restore `self.visualizer_visible = saved.visualizer_visible();` and
  update that method's doc from two UI-owned values to three. Correct the `eq_panel_visible`
  field doc, which currently says it is not persisted "like `visualizer_visible`": the EQ panel
  stays deliberately unpersisted, while the visualizer choice now persists.
- `src/settings.rs` tests: extend `to_text_round_trips_every_field` to set the visualizer hidden
  and assert the round trip preserves it; add
  `visualizer_visibility_round_trips_and_ignores_malformed_values` covering the default shown,
  an explicit `false`, and an unknown value falling back to shown; add
  `visualizer_visible=maybe` to `from_text_ignores_malformed_and_unknown_lines`.
- `src/ui/tests.rs`: add `toggle_visualizer_reaches_the_saved_snapshot` with the existing
  `MemorySettingsStore`, asserting one saved snapshot with `visualizer_visible()` false after one
  `Message::ToggleVisualizer`, no save after `Message::Ignored`, and true after a second toggle;
  add `saved_visualizer_visibility_is_restored_on_boot` calling `adopt_saved_settings` with a
  hidden snapshot and with `Settings::default()`, asserting `player.visualizer_visible` false then
  true and `player.saved_settings` follows each snapshot.
- `README.md`: in the final Usage settings paragraph, add the visualizer's shown/hidden choice to
  the values saved and restored at the next launch. Do not touch the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Settings persistence`, add the visualizer's shown/hidden choice to
  the persisted list, change "the two persisted values not held in `AppState`" to three, name
  `WinampPlayer::visualizer_visible` alongside the readout mode and always-on-top flag, state
  that a missing or malformed `visualizer_visible` line falls back to shown, add the two new
  player tests to **Pinned by**, and note the extension in **History**. Keep the existing
  statement that window placement and the EQ panel's visibility are deliberately not persisted.

**Files touched.** `src/settings.rs`, `src/ui/player.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- A settings file with `visualizer_visible=false` restores the hidden visualizer before the
  first view; `visualizer_visible=true`, a missing line, or an unknown value restores the shown
  visualizer, and `to_text` round-trips both booleans.
- Pressing the strip's `V` writes exactly one new snapshot per change through
  `persist_settings_if_changed`; an unchanged update writes nothing, and toggling back restores
  the shown state in the file. Hiding still leaves `WinampPlayer::spectrum` and the spectrum poll
  untouched, so a restored-hidden launch that is shown again draws the current bars.
- No new message, handler, playback path, dependency, or window geometry is added; `settings.rs`
  keeps its `AppState`-only `apply_to` and does not import the UI.
- README and `docs/invariants.md` describe the saved choice, and `make check` passes.

### Bind the classic A and T keys for always-on-top and the time display (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** The title bar's `A` and the LCD strip's `A` toggle always-on-top, and clicking any
real-time readout switches it between elapsed and remaining, but neither classic Winamp main-window
hotkey is bound: a keyboard-driven user must reach for the mouse. Bind `A` to
`Message::ToggleAlwaysOnTop` and `T` to `Message::ToggleTimeDisplay` so the classic key set covers
both existing actions. No new message, handler, state, or playback path: `handlers::window::
toggle_always_on_top` and `handlers::playback::toggle_time_display` already implement them.

**Approach.**

- `src/ui/shortcuts.rs`: in `shortcut`, add a `Key::Character` arm matching `"a"` for
  `Message::ToggleAlwaysOnTop` and a `"t"` arm for `Message::ToggleTimeDisplay`, beside the
  existing R/S letter arms and before the named-key arms. The existing Control/Alt/Logo guard runs
  first, so a chorded A/T stays with the OS and window manager and Shift is allowed, as for the
  other letters. Update the `message_for` and `shortcut` doc comments to list A and T beside
  Z/X/C/V/B/R/S.
- `src/ui/shortcuts.rs` tests: add `message_for_maps_a_and_t_to_the_classic_toggles`, driving a
  plain and a Shift-held `Key::Character("a")` and `Key::Character("t")` `KeyPressed` event
  through `message_for` with the existing `key_pressed` helper and asserting the two messages; add
  both keys to the key list in `shortcut_ignores_chord_modifiers`, and add uppercase A/T asserts to
  `shortcut_matches_letters_case_insensitively`. Leave `assert_letter_bindings` and its doc alone:
  it covers the transport letters, while A and T dispatch view and window messages.
- `src/ui/tests.rs`: add `a_toggles_always_on_top_from_the_keyboard`, starting from the existing
  `player_with_window_id()` fixture, building an `A` `KeyPressed` event as
  `enter_opens_the_selected_song` builds its Enter event, asserting `super::shortcuts::message_for`
  returns `Message::ToggleAlwaysOnTop`, driving it through `update`, and asserting the flag flipped
  and the arm scheduled work; and `t_toggles_the_time_display_from_the_keyboard`, building a `T`
  event the same way and asserting `player.time_display` flips after `update`. These pin the
  shortcut-to-handler wiring, not just the table.
- `README.md`: in the Usage keyboard sentence, say `A` toggles always-on-top and `T` switches the
  large readout between elapsed and remaining time, and that both are the same actions as the
  on-screen controls. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, state that `shortcuts::message_for` maps T to
  `Message::ToggleTimeDisplay`, so the stored mode flips in every transport state; the stopped
  readout's click guard makes only the readout inert, not the key. In `## Settings persistence`,
  state that the same mapper maps A to `Message::ToggleAlwaysOnTop`, the arm the title bar's A and
  the strip's A already press, with the window-level schedule unchanged. Name
  `shortcuts::message_for` in **Enforced by** and add the two new tests to **Pinned by** in the
  matching section.

**Files touched.** `src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A plain or Shift-held A press dispatches `Message::ToggleAlwaysOnTop` through `update`, flipping
  `WinampPlayer::always_on_top` and scheduling the same `iced::window::set_level` call the title
  bar's and strip's A presses schedule; before the window id resolves it flips the flag without a
  window call, matching the existing arm.
- A plain or Shift-held T press dispatches `Message::ToggleTimeDisplay` through `update`, flipping
  `WinampPlayer::time_display` exactly as a readout click does. Unlike the stopped readout's click
  target it is not gated by transport mode, because the key maps to the existing message and the
  mode is presentation-only.
- Control, Alt, and Logo chords on A and T map to nothing, and every existing letter, Enter,
  arrow, and seek binding is unchanged.
- The focused `Search library` box keeps typed A and T for editing its query, as it already keeps
  Enter and Backspace, because iced's `TextInput` captures them before the window subscription.
- README and `docs/invariants.md` describe both keys, and `make check` passes.

### Step back out of a browse level with Backspace (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** The browse list's `Back` button climbs one level up, and Enter opens the selected row
from the keyboard, but no key triggers `Message::Back`; a user working from the keyboard must
reach for the mouse. Map a Backspace key press to `Message::Back`, so it steps exactly one level
up with the same rules and no-op at the Artists list as the button. Escape stays unbound, and the
`Search library` box keeps Backspace for editing its query, as it already keeps Enter for
`SearchSubmitted`.

**Approach.**

- `src/ui/shortcuts.rs`: in `shortcut`, add
  `Key::Named(key::Named::Backspace) => Some(Message::Back)` beside the Enter arm, after the
  existing Control/Alt/Logo chord guard, so a chorded Backspace still belongs to the OS and Shift
is allowed like the other bindings. Update the `message_for` and `shortcut` doc comments to list
Backspace as the browse Back key beside Enter. Add no `Message` variant, update arm, fetch, or
playback path: `Message::Back` and `browse::back` already implement the hierarchy.
- `src/ui/shortcuts.rs` tests: add `message_for_maps_backspace_to_back`, driving a plain and a
  Shift-held `key::Named::Backspace` `KeyPressed` event through `message_for` with the existing
  `key_pressed` helper and asserting `Message::Back`; add `Key::Named(key::Named::Backspace)` to
  the key list in `shortcut_ignores_chord_modifiers`. Keep the Escape assertion in
  `message_for_ignores_unbound_keys_and_non_press_events` as it is, because Escape remains
  unbound.
- `src/ui/tests.rs`: add `backspace_steps_back_from_albums`, starting from the existing
  `player_in_albums()` fixture, building a Backspace `KeyPressed` event as `enter_opens_the_selected_song`
  builds its Enter event, asserting `super::shortcuts::message_for` returns `Message::Back`, then
  driving that message through `update` and asserting `CurrentView::Artists`. This pins the
  shortcut-to-handler wiring, not just the table.
- `README.md`: in the Usage keyboard sentence, say Backspace steps the browse list back one level
  exactly like the `Back` button, does nothing at the Artists list, and that the `Search library`
  box keeps Backspace for editing its query while focused. Do not touch the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Browse hierarchy`, state that Backspace is the keyboard route to
  the same single-step Back action, that it is captured by the focused search box's `TextInput`
  like Enter, and that Artists stays a no-op; name `shortcuts::message_for` in **Enforced by** and
add the two new tests to **Pinned by**.

**Files touched.** `src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A plain or Shift-held Backspace press below the Artists list steps exactly one browse level up
  through the existing `Message::Back` arm, with the same Albums/Songs/All Songs/All Albums/
  Playlists/search origins and the same snap-to-top follow-up task the Back button gets; at the
  Artists list it is the existing no-op.
- Control, Alt, and Logo chords on Backspace map to nothing, Escape remains unbound, and the
  focused `Search library` box keeps Backspace for editing rather than navigating.
- Existing letter, Enter, arrow, and seek bindings are unchanged; no message, handler, fetch,
  playback, or window path is added.
- README and `docs/invariants.md` describe the key, and `make check` passes.

### Remember always-on-top across launches (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** The title bar's `A` and the LCD strip's `A` toggle `WinampPlayer::always_on_top` for the
current run only, so a user who pins the window must pin it again after every launch. Persist that
choice beside the mixer and equalizer settings, restore the flag before the first view, and apply
the saved window level once boot's `WindowIdResolved` supplies the id, so a saved-on launch starts
pinned. A missing, malformed, or false value keeps today's unpinned default. This is independent
of the queued window-title plan: it adds no title change, message, handler for playback, or
window geometry.

**Approach.**

- `src/settings.rs`: add a private `always_on_top: bool` to `Settings` beside
  `time_display_remaining`, defaulting to `false`; add `#[must_use] pub fn always_on_top(&self) ->
  bool` and `pub fn set_always_on_top(&mut self, always_on_top: bool)`, mirroring the readout
  mode's accessors. Write it as `always_on_top=true`/`false` in `to_text`, parse that key with
  `parse_into` in `from_text` (an unknown or malformed value keeps the default), and leave it
  untouched in `apply_to` and at `false` in `from_state`, like `time_display_remaining`, because
  the player owns it. Update the module, struct, `Default`, and `from_state` docs to say two
  UI-owned values are copied in by the player instead of one.
- `src/ui/player.rs`: in `persist_settings_if_changed`, after
  `snapshot.set_time_display_remaining(...)`, call
  `snapshot.set_always_on_top(self.always_on_top)`. In `adopt_saved_settings`, set
  `self.always_on_top = saved.always_on_top()` before storing `saved_settings`, and update its doc
  to name both player-owned values it restores.
- `src/ui/handlers/window.rs`: add a pure
  `pub(in crate::ui) fn window_level(always_on_top: bool) -> iced::window::Level` returning
  `AlwaysOnTop` or `Normal`, and use it in `toggle_always_on_top` instead of the inline match. In
  `window_id_resolved`, store the id as today, then, when the id is `Some` and
  `player.always_on_top` holds, return
  `iced::window::set_level(id, window_level(true))`; otherwise return `Task::none()`. Document
  that the saved level waits for the same id every other window action uses, so no `.window`
  settings or window geometry changes are involved.
- `src/settings.rs` tests: extend `to_text_round_trips_every_field` to set
  `always_on_top` true and assert it survives the round trip, and add
  `always_on_top_round_trips_and_ignores_malformed_values` covering `true`, `false`, and an
  unknown word falling back to `false`; add an `always_on_top=yes` line to
  `from_text_ignores_malformed_and_unknown_lines`'s text.
- `src/ui/tests.rs`: add `toggle_always_on_top_reaches_the_saved_snapshot` (attach a
  `MemorySettingsStore`, drive `Message::ToggleAlwaysOnTop`, assert one snapshot with
  `always_on_top()` true, then a second toggle with it false, and that `Message::Ignored` adds no
  snapshot); `saved_always_on_top_is_restored_on_boot` (adopt a snapshot with the flag set and
  assert `player.always_on_top` and `saved_settings`; a default snapshot clears it); and
  `window_id_resolved_applies_a_saved_always_on_top_level` (with the flag set, resolve an id and
  assert the id is stored and work is scheduled; with it clear, resolve an id and assert no work
  is scheduled). Add `window_level_selects_always_on_top` beside the existing toggle tests.
- `README.md`: add always-on-top to the saved-values sentence beside volume, balance, Repeat,
  Shuffle, the equalizer, and the readout mode, saying the title bar's `A` choice is remembered
  for the next launch. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Settings persistence`, add always-on-top to the persisted-values
  guarantee, state that it lives on `WinampPlayer::always_on_top`, that `adopt_saved_settings`
  restores it before the first view, that `window_id_resolved` applies its level once the id
  exists, and that it is still not window placement. Replace "the one persisted value not held in
  `AppState`" with the readout mode and this window flag, and name the new tests in `Pinned by`.

**Files touched.** `src/settings.rs`, `src/ui/player.rs`, `src/ui/handlers/window.rs`,
`src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- With `always_on_top=true` in the settings file, the restored player flag is set before the first
  view and boot's `WindowIdResolved` schedules `iced::window::set_level` with
  `Level::AlwaysOnTop`; a false, missing, or malformed value leaves the window at the existing
  unpinned default and schedules no level change.
- Pressing `A` in the title bar or LCD strip persists exactly one new snapshot with the new
  value, and an update that changes nothing else writes nothing; the existing settings
  round-trip, clamping, and fallback behavior is unchanged.
- `apply_to` still touches only `AppState`-backed values, so adopting a snapshot does not change
  playback, mixer, equalizer, or geometry behavior.
- README and `docs/invariants.md` describe the persisted choice, and `make check` passes.

### Name the current track in the window title (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Classic Winamp names the playing track in its window title, but this app still passes
iced's static `"nostalgia"` to `.title`, so the desktop environment always shows the app name.
Build the title from the same `known_tracks` index the Now Playing LCD uses, so selecting a track
(Next/Previous and the preview-end advance included) names it in the title, Stop keeps the
interrupted track's name as the LCD does, and no known track restores the plain `nostalgia`.
This is presentation-only: it adds no message, handler, playback path, dependency, or window
geometry.

**Approach.**

- `src/ui/player.rs`: add a private `const APP_TITLE: &str = "nostalgia";` near the struct and
  `pub(in crate::ui) fn window_title(&self) -> String`. Lock the shared state, read `current_track`,
  and look the id up in `self.known_tracks`; no current track or an unknown id returns
  `APP_TITLE.to_string()`. For a known track, compose
  `"<number><artist - ><title> - <APP_TITLE>"` in the LCD's order: append `KnownTrack::number`,
  then `artist` plus `" - "` only when the artist is non-empty, then `title`, then `" - "` and
  `APP_TITLE`. Document that this is the iced window title, not the custom title bar's fixed
  `NOSTALGIA`, and that Stop keeps the interrupted track.
- `src/ui/mod.rs`: replace `.title("nostalgia")` in `init_ui` with
  `.title(|player: &WinampPlayer| player.window_title())`, and mention the track-following window
  title in the `init_ui` doc.
- `src/ui/tests.rs`: add `window_title_names_the_current_track` beside the other Now Playing
  resolution tests. Seed `player.known_tracks["song-1"]` with number `"1. "`, artist
  `"The Sample Band"`, and title `"Opening"` (the shape the marquee test already builds), set
  `state.current_track`, and assert `player.window_title()` is
  `"1. The Sample Band - Opening - nostalgia"`. Then clear the number
  (`"The Sample Band - Opening - nostalgia"`), clear the artist (`"1. Opening - nostalgia"`), set
  `current_track` to an unknown id (`"nostalgia"`), and set it to `None` (`"nostalgia"`). Finally
  restore the known track and call `AppState::stop()`, asserting the title still names it.
- `README.md`: in the Usage paragraph about the title bar, add one sentence that the window title
  (the name the desktop environment shows for the app) names the current track as
  `<number>Artist - Title - nostalgia`, and is just `nostalgia` when no known track is current.
  Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, add a bullet stating that the window title
  mirrors a known current track through `WinampPlayer::window_title`, using the same index and
  number prefix as the LCD, keeps the interrupted track across Stop, and falls back to
  `nostalgia` for no current track or an unknown id; name the new test in that section's
  `Pinned by` list.

**Files touched.** `src/ui/player.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- The application window title reads `1. The Sample Band - Opening - nostalgia` for that known
  current track, omits the number or artist when empty, stays on the interrupted track across
  Stop, and is the plain `nostalgia` with no current track or an unknown id.
- The title resolves `AppState::current_track` against `WinampPlayer::known_tracks`, so a track
  selected through the browse rows, Next/Previous, or the preview-end advance updates it; no new
  message, handler, dependency, or window geometry is added.
- README and `docs/invariants.md` describe the title, and `make check` passes.

### Persist the elapsed/remaining time-display choice across launches (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** The large LCD readout's elapsed/remaining mode already survives Pause, Stop,
Next/Previous, and a new selection, but a restart always brings back elapsed because `Settings`
snapshots only `AppState` while `WinampPlayer::time_display` lives on the player. Save the mode in
the same settings file as volume, balance, Repeat, Shuffle, and the equalizer, and restore it
before the first view, so a user who left the readout counting down gets the same readout next
launch. This is presentation-only and independent of the queued compact-readout toggle: it adds
no message, handler, geometry, or playback path.

**Approach.**

- `src/settings.rs`: add a private `time_display_remaining: bool` field to `Settings` (default
  `false`, i.e. elapsed) and `pub fn time_display_remaining(&self) -> bool` /
  `pub fn set_time_display_remaining(&mut self, remaining: bool)` accessors. Keep `settings.rs`
  independent of the UI module rather than storing `views::TimeDisplay`; the player maps the bool
  at its boundary. Write `time_display=elapsed` or `time_display=remaining` in `to_text`; parse
  only those two words in `from_text` and ignore anything else per key. Update the module and
  struct docs: this is the one persisted value not held in `AppState`, so `from_state` leaves it
  elapsed and the player overwrites it from its own mode before saving.
- `src/main.rs`: keep the loaded snapshot instead of recomputing it from state alone
  (`let loaded = store.load(); ... let saved = loaded.unwrap_or_else(||
  settings::Settings::from_state(&initial_state));`) so the saved time mode reaches `init_ui`;
  document why.
- `src/ui/player.rs`: in `persist_settings_if_changed`, after `Settings::from_state(&state)`, set
  the snapshot's time mode from `self.time_display == views::TimeDisplay::Remaining`. Add
  `pub(in crate::ui) fn adopt_saved_settings(&mut self, saved: Settings)` that sets
  `self.time_display` from `saved.time_display_remaining()` and stores the snapshot in
  `self.saved_settings`; document that the state-backed values were already applied by `main`
  and this restores only the player-owned mode.
- `src/ui/mod.rs`: replace `player.saved_settings = saved.clone()` in `init_ui` with
  `player.adopt_saved_settings(saved.clone())`, and mention the time mode in the `init_ui` doc.
- Tests. `src/settings.rs`: extend `to_text_round_trips_every_field` to set remaining and
  round-trip it; add `time_display_round_trips_both_words_and_ignores_unknown_values`; add
  `time_display=sideways` to `from_text_ignores_malformed_and_unknown_lines`. `src/ui/tests.rs`:
  add `toggle_time_display_reaches_the_saved_snapshot` (attach `MemorySettingsStore`, toggle to
  remaining and back, assert the two saved snapshots' `time_display_remaining()` are true then
  false and an unchanged update saves nothing) and `saved_time_display_mode_is_restored_on_boot`
  (call `adopt_saved_settings` with a remaining snapshot and with the default, then assert
  `player.time_display` and `player.saved_settings`).
- `README.md`: in the settings paragraph at the end of Usage, add the readout's
  elapsed/remaining choice to the saved values and say it is restored at the next launch. Do not
  touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Settings persistence`, add the readout's elapsed/remaining choice
  to the persisted list, state that it is the one value not held in `AppState` (snapshotted from
  `WinampPlayer::time_display` and restored by `WinampPlayer::adopt_saved_settings`), that a
  missing or malformed `time_display` line falls back to elapsed, and name the new tests in
  Pinned by.

**Files touched.** `src/settings.rs`, `src/main.rs`, `src/ui/player.rs`, `src/ui/mod.rs`,
`src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A settings file with `time_display=remaining` restores the remaining readout before the first
  view; `time_display=elapsed`, a missing line, or an unknown value restores elapsed, and
  `to_text` round-trips both words.
- Clicking the LCD readout writes the new mode through `persist_settings_if_changed` exactly
  once per change; an unchanged update writes nothing, and the mode still survives Pause, Stop,
  Next/Previous, and a new selection in memory.
- No new message, handler, playback path, dependency, or window geometry is added; `settings.rs`
  keeps its `AppState`-only `apply_to` and does not import the UI's `TimeDisplay`.
- README and `docs/invariants.md` describe the mode as persisted, and `make check` passes.

### Toggle elapsed and remaining time from the compact transport readout (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

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

### Open the selected browse row with Enter (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

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

### Show and hide the equalizer panel from an EQ button on the player (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match `docs/screenshots/equalizer.png`'s "MAIN + EQ" caption — "The EQ button on the
main window opens and closes the equalizer docked beneath it" — and the `EQ` square beside the
balance slider in `docs/screenshots/player.png`, `hero.png`, and `library.png`. Today `view`
always pushes the `NOSTALGIA EQUALIZER` band and `view_equalizer` panel, and the main transport
row has no EQ button, so the panel cannot be closed. This adds a player-row `EQ` toggle for the
panel's visibility. It is a layout-only flag: the equalizer's on/off state, curve, preset, and
audio forwarding are untouched, the window is not resized, and hiding the section hands its
height to the browse list. The `PL` button the mockups also draw is out of scope; the library
section is unchanged.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) eq_panel_visible: bool` to `WinampPlayer`,
documented as whether the equalizer section is drawn; initialize it true in `new()` (open at
launch, like the mockup) and do not add it to settings, since shade and visualizer visibility
are not persisted either.
- `src/ui/message.rs`: add `ToggleEqualizerPanel` beside `ToggleEqualizer`, with a comment saying
it flips only the panel-visibility flag — not the EQ's on/off state or its curve — and schedules
no work.
- `src/ui/mod.rs`: dispatch `Message::ToggleEqualizerPanel` to
`handlers::mixer::toggle_equalizer_panel`.
- `src/ui/handlers/mixer.rs`: add `pub(in crate::ui) fn toggle_equalizer_panel(player: &mut
WinampPlayer) -> Task<Message>` next to `toggle_equalizer`; it flips
`player.eq_panel_visible` and returns `Task::none()`. Its doc distinguishes the audio on/off
button (`toggle_equalizer`) from this layout-only toggle.
- `src/ui/views/transport.rs`: add
`pub(super) fn eq_panel_toggle() -> (&'static str, f32, Message)` returning `("EQ",
EQ_PANEL_BUTTON_WIDTH, Message::ToggleEqualizerPanel)` with `EQ_PANEL_BUTTON_WIDTH` = 30.0, and
`pub(super) fn eq_panel_button_style(open: bool, status: button::Status) -> button::Style`
returning `style::chrome_button_style(button::Status::Pressed)` while open and
`style::chrome_button_style(status)` otherwise (the title bar's A toggle sunken-while-on rule).
Extend `view_transport_controls` with an `eq_panel_visible: bool` parameter and, after the
balance slider and a `spacer(20.0)`, push the fixed-width `EQ` button built from those helpers.
Update the module and function docs to name the added button.
- `src/ui/view.rs`: make `player_panels` mutable, pass `player.eq_panel_visible` to
`view_transport_controls`, and push the `NOSTALGIA EQUALIZER` band and `view_equalizer` only when
`player.eq_panel_visible`. Update the module and `view` docs: the equalizer section is normal-mode
chrome the player's EQ button shows and hides, and hiding it lets the browse list take that
height without an `iced::window::resize`.
- `src/ui/views/tests.rs`: import `eq_panel_button_style` and `eq_panel_toggle`; update
`transport_controls_construct_for_both_play_states_volume_endpoints_and_repeat_states` to pass
both visibility states; add `eq_panel_toggle_targets_the_visibility_flag` pinning the label,
width, and `Message::ToggleEqualizerPanel`, and
`eq_panel_button_style_sinks_while_the_panel_is_open`, asserting the open style's background,
border, and shadow match `style::chrome_button_style(button::Status::Pressed)` for a resting
status while the closed style follows the status passed in.
- `src/ui/tests.rs`: add `toggle_equalizer_panel_hides_and_restores_the_section`: lay out the root,
assert `NOSTALGIA EQUALIZER` and `EQ: On` draw; drive `Message::ToggleEqualizerPanel` through
`update`, assert both are gone while `NOSTALGIA LIBRARY`, the browse list, and the row's `EQ`
button remain and the library band's `y` moved up; toggle back and assert both return at their
original `y`; also assert `AppState::eq_enabled`, `eq_preamp()`, and `eq_bands()` are unchanged
across both toggles.
- `README.md`: in the Usage paragraph's equalizer sentence, say an `EQ` button beside the balance
slider shows and hides the equalizer panel (open at launch), and that hiding it gives the height
to the browse list without changing EQ processing. Do not edit the
`<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## UI layout`, add a bullet: the main transport row's fixed-width `EQ`
toggle is present in every playback state and is sunken while the panel is shown; `view` draws
the `NOSTALGIA EQUALIZER` band and `view_equalizer` exactly when `WinampPlayer::eq_panel_visible`
holds, and hiding them moves the library band and browse list up without resizing the window or
changing EQ state. Name `views::transport::eq_panel_toggle`/`eq_panel_button_style`,
`handlers::mixer::toggle_equalizer_panel`, and `view` as the enforcers, and add the new test
names to `Pinned by`.

**Files touched.** `src/ui/player.rs`, `src/ui/message.rs`, `src/ui/mod.rs`,
`src/ui/handlers/mixer.rs`, `src/ui/views/transport.rs`, `src/ui/view.rs`,
`src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The main transport row draws a fixed-width `EQ` button after the balance slider in every
playback state; pressing it emits `Message::ToggleEqualizerPanel`, and the button is sunken while
the panel is open and follows the normal chrome faces while closed.
- The `NOSTALGIA EQUALIZER` title band and equalizer panel are drawn only while
`eq_panel_visible` is true. Hiding them removes both, keeps `NOSTALGIA LIBRARY`, the browse
controls, and the browse list, and moves the library section up to use the freed height; the
window is not resized.
- Toggling the panel leaves `AppState::eq_enabled`, the preamp, the band gains, and the preset
unchanged and forwards nothing to the audio backend, so hiding the panel does not bypass the
curve.
- The flag starts true at launch and is not read from or written to settings; shade mode still
drops both sections as today, and hiding the panel before shading does not change the shaded
strip.
- README and `docs/invariants.md` describe the toggle and the layout guarantee, and `make check`
passes.

### Paint browse rows green, the playing row white, and only the selected row blue (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match `docs/screenshots/library.png` and `docs/screenshots/hero.png`: their browse rows
are bright green, the playing row's text is white, and the blue bar marks the selected row only.
Today `style::playlist_row_style` paints every row `theme::TEXT` (light gray), and
`views::browse::rows::highlighted_row_style` forces `theme::ROW_HIGHLIGHT` behind both the
playing row and the selected row, so the playing row and the selection look identical and
neither matches the mockups. This is a row-color change only: numbering, the `▶` marker,
selection and open behavior, row text, hover/press faces, layout, and scrolling stay as they are.

**Approach.**

- `src/ui/theme.rs`: add `pub const PLAYING_ROW_TEXT: Color = Color::from_rgb(1.0, 1.0, 1.0);`,
documented as the white text of the playing browse row. Keep the
`ROW_HIGHLIGHT` constant and its RGB value, and reword its doc to say it is the blue bar
behind the selected browse row only.
- `src/ui/style.rs`: in `playlist_row_style`, change `text_color` from `theme::TEXT` to
`theme::LCD_GREEN`, keeping the hover/press background ladder and the flat border/shadow. Update
the doc from "flat light text" to the mockups' flat green row text.
- `src/ui/views/browse/rows.rs`: replace `highlighted_row_style(status)` with `pub(in
crate::ui::views) fn browse_row_style(status: button::Status, is_current: bool, is_selected:
bool) -> button::Style`. Start from `style::playlist_row_style(status)`; when `is_selected`,
set `background = Some(Background::Color(theme::ROW_HIGHLIGHT))`; when `is_current ||
is_selected`, set `text_color = theme::PLAYING_ROW_TEXT`. A row that is both playing and selected
therefore gets the selection bar and white text, while its `▶` prefix still identifies it. In
`scrollable_list`, compute `is_selected = selected == Some(position)` and apply one `.style(move
|_theme, status| browse_row_style(status, is_current, is_selected))` to every row instead of the
current playing-or-selected branch. Update the module and function docs to say unselected rows
are green, the playing row is white without a bar, and the selected row is blue with white text.
- `src/ui/views/tests.rs`: import `browse_row_style` in place of `highlighted_row_style`. Replace
`highlighted_row_style_paints_the_selection_bar_over_every_row_state` with two pure-seam tests.
`browse_row_style_keeps_playing_rows_unbarred_with_white_text` asserts, for every button status,
that `browse_row_style(status, true, false)` keeps `style::playlist_row_style(status).background`
(so the resting playing row has no bar and hover/press still lift and sink) and uses
`theme::PLAYING_ROW_TEXT`, which differs from the base row's green text.
`browse_row_style_paints_only_the_selected_row_blue` asserts, for every status and for
`is_current` both false and true, that a selected row's background is
`theme::ROW_HIGHLIGHT` and its text is white, while `browse_row_style(status, false,
false)` keeps the base background and `theme::LCD_GREEN` text; it also asserts the selected
background differs from the plain resting background.
- `src/ui/style.rs` tests: update `playlist_row_style_is_a_flat_entry_that_lifts_and_sinks` to
assert `theme::LCD_GREEN` text for every status instead of `theme::TEXT`.
- `src/ui/theme.rs` tests: keep the highlight assertions on `ROW_HIGHLIGHT`, add the
`PLAYING_ROW_TEXT` literal, and assert it differs from `theme::LCD_GREEN` so the playing row
cannot become indistinguishable from an ordinary row.
- `README.md`: in the Usage browse sentence, say rows draw green text, the playing song's row is
white with its `▶` marker and no bar, and a click selects a row and paints its blue bar with
white text; a second click on the selected row or `Open` opens it. Do not edit the
`<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Browse selection`, state that unselected rows draw green text, the
playing row keeps its `▶` marker and white text without a bar unless selected, and only the
selected row draws the blue `ROW_HIGHLIGHT` bar with white text. Name
`style::playlist_row_style` and `views::browse::rows::browse_row_style`/`scrollable_list` as the
enforcers, and add the two new tests to `Pinned by`.

**Files touched.** `src/ui/theme.rs`, `src/ui/style.rs`, `src/ui/views/browse/rows.rs`,
`src/ui/views/tests.rs`, `src/ui/browse.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Every unselected, non-playing browse row in every list draws `theme::LCD_GREEN` text with no
  blue bar, keeping the existing hover and press faces.
- The playing row keeps its `▶` prefix and hover/press faces but draws no blue bar, and its text
  is `theme::PLAYING_ROW_TEXT` (white), so it is distinguishable from an ordinary green row.
- Only the selected row draws `theme::ROW_HIGHLIGHT` behind it, in every hover/press
  state, with white text; a row that is both playing and selected gets the blue bar and stays
  identifiable by its `▶` prefix.
- Row numbering, titles, labels, footers, selection and open messages, scrolling, and layout are
  unchanged; README and `docs/invariants.md` describe the three row appearances, and `make check`
  passes.

### Show the compact transport readout as elapsed over the track's total (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match `docs/screenshots/library.png` and `docs/screenshots/hero.png`, whose compact
footer readout reads `4:05/12:33`: show the current time over the playing track's total in the
shared compact row used by the library footer and the window-shade strip. The left value stays
the same `WinampPlayer::now_playing_elapsed` string (so the elapsed/remaining toggle still applies,
and remaining mode reads `-02:37/3:30`), the right value is the same
`WinampPlayer::now_playing_time` total the LCD's right edge shows, and when that total is the
`--:--` placeholder the readout stays that single string rather than `--:--/--:--`. This changes
only the compact row's text: the LCD, playback, messages, glyphs, volume slider, heights, and
layout are unchanged.

**Approach.**

- `src/ui/views/mini_transport.rs`: add `pub(super) fn mini_transport_readout<'a>(elapsed:
  Cow<'a, str>, total: Cow<'a, str>) -> Cow<'a, str>`. It returns `elapsed` unchanged when
  `total` is `"--:--"` (so a missing or unknown track keeps its current placeholder) and
  otherwise returns `Cow::Owned(format!("{elapsed}/{total}"))`. Give `mini_transport_row` a
  `total: Cow<'a, str>` parameter beside `elapsed`, build the drawn `Text` from this helper, and
  give `view_library_strip` the same parameter. Update the module and function docs to name both
  time strings and the placeholder rule.
- `src/ui/views/shade.rs`: give `view_shade_strip` a `total: Cow<'a, str>` parameter beside
  `elapsed` and pass it to `mini_transport_row`; update its docs to say the readout is
  elapsed-over-total and keeps the shared toggle.
- `src/ui/view.rs`: in the shaded branch, resolve `now_playing_time` from the same state lock
  tuple (using `state.current_track.as_deref()`), and pass it to `view_shade_strip`. In the normal
  branch, clone the already-resolved `now_playing_time` before it moves into `view_now_playing`
  (the way `library_elapsed` is cloned) and pass the clone to `view_library_strip`, so the footer
  and the LCD's right edge read the same total.
- `src/ui/views/tests.rs`: add `mini_transport_readout_appends_the_known_total` covering
  `00:53`/`3:30` → `00:53/3:30`, `-02:37`/`3:30` → `-02:37/3:30`, `00:53`/`--:--` → `00:53`, and
  `--:--`/`--:--` → `--:--`. Update `captured_shade_strip`, `captured_library_strip`, and the
  existing height and geometry tests to pass a total (for example `Cow::Borrowed("3:30")`) and
  assert the combined readout reaches the laid-out row. Keep the glyph order, fixed height, and
  right-edge volume assertions unchanged.
- `src/ui/tests.rs`: add `root_view_footer_shows_elapsed_over_total`, using
  `play_song_1_then_browse_to_album_2` and `captured_root_view_text_bounds` with `shaded = false`
  and then `true`, asserting both trees draw `00:00/3:30` (the known song's cached total) and
  that neither draws `--:--/--:--`. This pins the two `view` call sites, not just the builders.
- `README.md`: in the Usage prose, extend the library-footer and shade-strip sentences to say the
  compact readout shows the elapsed (or remaining) time over the track's total, and shows just
  the elapsed string (the plain `--:--` placeholder when no track is current) when no total is
  known. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Library footer` and `## Window shade`, replace the bullet that
  calls the compact label "the current time readout" / the same elapsed string with the
  elapsed-over-total rule and its single-time-string fallback. Name
  `views::mini_transport::mini_transport_readout` as the enforcer, note that both strips take the
  same `now_playing_elapsed` and `now_playing_time` strings the LCD resolves, and add the new
  tests to both `Pinned by` lists.

**Files touched.** `src/ui/views/mini_transport.rs`, `src/ui/views/shade.rs`, `src/ui/view.rs`,
`src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Both the library footer and the shade strip draw `<elapsed>/<total>`, such as `00:53/3:30`, when
  the current track's total is known; remaining mode still prefixes the elapsed value with `-`
  (`-02:37/3:30`), and a known total with no reported position reads `00:00/3:30`.
- When `now_playing_time` returns `--:--` (no current track, or an unknown one), the readout is
  just the elapsed string, so the empty state still reads `--:--` and never `--:--/--:--`.
- The compact row's four glyphs, messages, heights, right-edge `MINI_VOLUME_ID` slider, and
  elapsed/remaining toggle behavior are unchanged, and the LCD's large left readout and
  right-edge total are unchanged.
- The root `view` passes the same total string to the strip that it passes to the LCD, in both
  normal and shaded modes; README and `docs/invariants.md` describe the combined readout, and
  `make check` passes.

### Select a browse row, then open it with Open or a second click (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match `docs/screenshots/library.png`'s interaction caption — "Click selects,
double-click (or OPEN) drills in, BACK climbs out" — without changing what opening a row does
today. A single press on any browse row selects it and paints the mockup's blue selection bar
instead of drilling or playing; pressing that selected row again (which is what the second press
of a double-click is), or pressing the new `Open` button, opens it through the existing artist,
album, playlist, and song handlers. The playing row keeps its `▶` marker and highlight, keyboard
Next/Previous and the preview-end advance keep playing and scrolling as they do now, and opening
a row keeps its existing fetch/playback path. This is one focused selection layer over every
browse level; it adds no new fetch, playback, sort, or scroll behavior.

**Approach.**

- `src/ui/browse.rs`: give `BrowseList<T>` a `selected: Option<usize>` field beside the rows and
  the epoch. Add `pub(super) fn select_row(&mut self, epoch: u64, index: usize) -> Option<bool>`: it
  returns `None` for a stale epoch or out-of-range index (through the existing
  `BrowseList::select` check), otherwise records `Some(index)` and returns `Some(false)`, or
  `Some(true)` when that row was already selected. Add `pub(super) fn selected(&self) -> Option<usize>`. Clear `selected` in
  `replace_rows` (so `store`/`store_with_row_text` reset it), `clear`, and `fail`, and clear it
  in `sort_rows_by` and `restore_library_order` whenever they actually reorder rows, since those
  already bump the epoch because indices changed. A list whose rows and order are unchanged keeps
  its selection, so Back to an unsorted parent can still show the row the user left selected.
- `src/ui/message.rs`: add `SelectRow { epoch, index }`, the row-press message, and
  `OpenSelected`, the Open button's message. The existing `ArtistSelected`, `AlbumSelected`,
  `TrackSelected`, and `PlaylistSelected` remain the open actions, also emitted by Next/Previous
  and the preview-end advance where they already are; they are no longer emitted by a row's
  single press.
- `src/ui/handlers/browse.rs`: add `pub(in crate::ui) fn select_row(player, epoch, index) ->
  Task<Message>`. It calls the current view's list `select_row`, and returns `Task::none()` unless
  that reports the row was already selected; then it opens the pressed row. Add `pub(in
  crate::ui) fn open_selected(player) -> Task<Message>`: read the current view's list's
  `selected()` and `epoch`, and return `Task::none()` when nothing is selected, otherwise open
  that row. Both share one private `open_row(player, epoch, index) -> Task<Message>` that maps
  `current_view` to `Task::done(Message::ArtistSelected { .. })`, `AlbumSelected`, `TrackSelected`
  (for both `Songs` and `AllSongs`), or `PlaylistSelected`. Keeping the open action in the
  existing handlers means the epoch/index staleness checks, fetch clearing, and playback path
  stay exactly where they are.
- `src/ui/mod.rs`: route `Message::SelectRow` to `handlers::browse::select_row` and
  `Message::OpenSelected` to `handlers::browse::open_selected`.
- `src/ui/views/browse/rows.rs`: drop the open `Message` (and the now-unused `epoch`/`index`
  arguments) from `artist_row`, `album_row`, `playlist_row`, and `song_row`; each returns
  `(title, label, is_current)`. `scrollable_list` gains `epoch: u64` and `selected:
  Option<usize>` parameters and puts `Message::SelectRow { epoch, index }` on each row button
  instead of the tuple's removed message. Highlight a row when `is_current || selected ==
  Some(position)` using the existing `current_row_style` blue bar, while keeping `row_prefix`'s
  `▶` strictly tied to `is_current`. Update the doc comments to say a press selects and opening
  goes through `SelectRow`/`OpenSelected`.
- `src/ui/views/browse.rs`: add `pub fn view_open_button() -> Element<'static, Message>` using
  `labeled_button("Open", Message::OpenSelected)`, documented beside the Back/Top/Sort builders.
  Thread `selected: Option<usize>` through `view_artists`, `view_albums`, `view_songs`,
  `view_all_songs`, `view_all_albums`, `view_playlists`, `album_list`, and `song_list` into
  `scrollable_list`. Keep every row's title, label, numbering, playing marker, and footer as they
  are.
- `src/ui/views/mod.rs`: re-export `view_open_button` with the other browse builders.
- `src/ui/view.rs`: pass each list's `selected()` into its view builder in the `current_view`
  match. Push `views::view_open_button()` above the browse list for every browse view — after
  the Back/Retry controls, before the Top/Sort row — so Open is present at every level and a press
  with no selection is a no-op.
- `README.md`: in the Usage paragraph that describes the browse list, say a click selects a row,
  a second click on the selected row (a double-click) or the `Open` button opens it (drills into
  an artist, album, or playlist, or plays a song), the selected row gets the blue bar, and Back
  still climbs out. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Browse selection` section after `## Browse hierarchy`. State
  that each `BrowseList` owns at most one selected row, that only a matching epoch and in-range
  index can select, that a second press on the selected row or `OpenSelected` dispatches the
  existing open message, that a single press never opens/plays/fetches, and that selection is
  cleared when the rows are replaced, cleared, failed, sorted, or restored. Name
  `BrowseList::select_row`/`selected`, `handlers::browse::select_row`/`open_selected`/`open_row`,
  `views::browse::scrollable_list`, and `views::view_open_button` as the enforcers, and list the
  new tests as pinned.
- Tests. `src/ui/browse.rs`: add a `#[cfg(test)] mod tests` covering `select_row`'s
  select-then-report-second-press behavior, stale-epoch and out-of-range rejection, and the
  clearing paths. `src/ui/tests.rs`: add
  `a_single_row_press_selects_without_opening` (artist view unchanged, no task scheduled),
  `a_second_press_on_the_selected_row_opens_it` (the artist view steps to Albums),
  `open_selected_opens_the_selected_song` (playback task/current track follow the existing
  `TrackSelected` path), `open_selected_with_nothing_selected_is_a_noop`, and
  `replacing_or_sorting_rows_clears_the_selection`; reuse `seed_browse_lists`. `src/ui/views/tests.rs`:
  update the row-builder and `scrollable_list` calls to the new signatures, keep their title,
  label, prefix, and current-track assertions, and add
  `browse_open_button_constructs` to the view-construction test. `src/ui/tests.rs`'s root-layout
  tests: add `browse_open_button_sits_above_the_browse_list` using the existing
  `captured_root_view_text_bounds`, and keep Top beside Sort unchanged.

**Files touched.** `src/ui/browse.rs`, `src/ui/message.rs`, `src/ui/mod.rs`,
`src/ui/handlers/browse.rs`, `src/ui/views/browse.rs`, `src/ui/views/browse/rows.rs`,
`src/ui/views/mod.rs`, `src/ui/view.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`,
`README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- One press on any artist, album, song, or playlist row selects it and does not change the view,
  play a song, or schedule a fetch; the selected row draws the same blue bar as the playing row,
  and the `▶` marker stays on the playing row only.
- A second press on the row that is already selected opens it exactly as today's single press
  did: Artists → Albums, Albums/All Albums → Songs, Playlists → Songs, and a song starts playing
  through `handlers::playback::track_selected`; a double-click reaches this on its second press.
- The new `Open` button sits above every browse list and dispatches the same open action for the
  selected row; with no selection it does nothing and schedules no task.
- A stale epoch or out-of-range index cannot select, and replacing, clearing, failing, sorting,
  or restoring a list's rows clears its selection; the playing marker, numbering, scrolling and
  wheel behavior, Sort, Top, Back, and keyboard Next/Previous are unchanged.
- README and `docs/invariants.md` describe the select-then-open interaction, and `make check`
  passes.

### Show the mockups' mini transport in the library footer (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** The mockups' library panel (`docs/screenshots/library.png` and `hero.png`) ends in a
compact transport row beneath the browse list: Previous, Play/Pause, Stop, Next, the current time
readout, and a volume slider. The build has that row only in the window-shade strip today, so the
normal window's library footer ends at the browse list's count line. Draw the same row beneath
the browse list in normal mode, sharing the shade strip's control spec so the two cannot drift on
glyphs, order, fixed widths, or messages. This is presentation only: every control emits the
existing transport or volume message, no new message or handler is added, playback and browsing
(including the wheel's list scrolling) are unchanged, and the shade strip keeps its current blue
chrome and fixed height. It is independent of the queued O A I D V strip plan: that touches the
LCD and one display flag, this touches the root column below the browse list.

**Approach.**

- Add `src/ui/views/mini_transport.rs` as the one owner of the shared compact row:
  - `pub(crate) const MINI_TRANSPORT_HEIGHT: f32 = 28.0;` and
    `pub(super) const MINI_VOLUME_WIDTH: f32 = 80.0;`, matching the shade strip's current
    measured height and slider width so the new footer does not change the shade strip's
    geometry.
  - `pub(super) const MINI_VOLUME_ID: &str = "mini-transport-volume";`, the id both strips tag
    their volume-slider container with. Only one strip is in the tree at a time (the library
    strip is normal mode, the shade strip is shade mode), so the shared id never collides.
  - `pub(super) fn mini_transport_controls(is_playing: bool) -> [(&'static str, f32, Message);
    4]`, projecting `transport::transport_glyphs` by dropping each entry's tooltip word. The
    result stays `"◀◀"`, Play/Pause glyph (`"▶"`/`"II"`), `"■"`, `"▶▶"` with the current
    fixed widths 34/30/30/34 and the same four messages.
  - `pub(super) fn mini_volume_spec() -> (f32, f32, f32, f32, fn(f32) -> Message)`: the shared
    `VOLUME_MIN`, `VOLUME_MAX`, and `VOLUME_STEP`, `MINI_VOLUME_WIDTH`, and
    `Message::VolumeChange`, exactly as the shade strip builds its slider today.
  - `pub(super) fn mini_transport_row<'a>(is_playing: bool, volume: f32, elapsed: Cow<'a,
    str>) -> Row<'a, Message>`: one full-width row of the four fixed-width buttons separated by
    `spacer(6.0)`, then the elapsed text as `Text::new(elapsed).size(14).color(theme::LCD_GREEN)`,
    a fill `Space`, and the volume slider (`chrome_slider` with `theme::VOLUME_SLIDER_FILL`)
    tagged `MINI_VOLUME_ID` and pinned to `MINI_VOLUME_WIDTH` at the right edge.
  - `pub fn view_library_strip<'a>(is_playing: bool, volume: f32, elapsed: Cow<'a, str>) ->
    Element<'a, Message>`: wrap `mini_transport_row` in a `Container` fixed at
    `MINI_TRANSPORT_HEIGHT`, full width, vertically centered, padded `[0, 6]`, then in
    `bevel::raised_panel` with no extra background so it composes with the window face.
- `src/ui/views/shade.rs`: keep `SHADE_STRIP_HEIGHT` as an alias of
  `mini_transport::MINI_TRANSPORT_HEIGHT` and keep `SHADED_HEIGHT` and the blue
  `style::title_bar_style()` panel. Replace the local `shade_controls`, row construction, and
  volume-slider spec with calls to `mini_transport::mini_transport_row` and its spec, so the shade
  strip and the library strip build the same controls. Remove the imports that move to the new
  module (`chrome_slider`, the volume constants, and any now-unused widget imports).
- `src/ui/views/mod.rs`: declare `mod mini_transport;` beside the other panel modules and
  re-export `view_library_strip` publicly. The `views` tests reach the shared constants and spec
  through `super::mini_transport`, as they already reach `super::shade`'s helpers.
- `src/ui/view.rs`: in the normal-mode branch only, after `column.push(main_content)`, push
  `views::view_library_strip(is_playing, volume, elapsed)`. Clone the `now_playing_elapsed` Cow
  once before it is moved into `view_now_playing` and give the library strip the same string, so
  the LCD and the footer cannot disagree about elapsed versus remaining mode. Update the function
  doc comment to name the library footer; leave the shade early return untouched.
- `README.md`: in the Usage paragraph that describes the browse list, say a compact transport
  strip sits beneath it with Previous, Play/Pause, Stop, Next, the current time readout, and a
  volume slider, matching the shade strip's controls. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Library footer` section after `## Window shade`: normal mode
  draws exactly one compact transport row beneath the browse list, the four buttons emit
  `PreviousTrack`, `PlayPause`, `Stop`, and `NextTrack`, the Play/Pause glyph follows `is_playing`,
  the time label follows the elapsed/remaining choice, and the slider writes `VolumeChange`;
  shade mode draws the shade strip from the same row builder and normal mode draws no shade strip.
  Name `views::mini_transport::view_library_strip`/`mini_transport_row`/`mini_transport_controls`
  and `views::shade::view_shade_strip` as the enforcers. In `## Window shade`, replace any
  wording that normal mode builds no compact controls with the distinction above.
- `src/ui/views/tests.rs`: update the shade imports to the moved `mini_transport_*` names; keep
  `main_transport_and_shade_strip_share_glyphs_order_and_widths` and extend it to compare both
  strips against `mini_transport_controls`. Add `view_library_strip_keeps_its_fixed_height`
  (lay it out with the headless renderer and assert `MINI_TRANSPORT_HEIGHT`), and
  `library_strip_draws_the_four_controls_time_and_right_edge_volume` (capture the drawn text and
  the `MINI_VOLUME_ID` container; assert the four glyphs in order, the given time string, the
  slider width `MINI_VOLUME_WIDTH`, and its right edge at the strip's padding). Update the
  existing shade geometry/spec tests for the moved constants and functions without changing
  their assertions.
- `src/ui/tests.rs`: add `normal_root_view_draws_the_library_strip_beneath_the_browse_list`: lay
  out the root view, capture text bounds, and assert there are two `"◀◀"` fragments in normal
  mode, that the second sits below the browse `Top` button and inside the window, and that the
  library strip's time text and `MINI_VOLUME_ID` container are present. Extend
  `shade_strip_draws_only_while_shaded` only where needed so the shaded tree still has exactly
  the shade strip and the normal tree's first `◀◀` is the full transport row, not the footer.

**Files touched.** `src/ui/views/mini_transport.rs` (new), `src/ui/views/shade.rs`,
`src/ui/views/mod.rs`, `src/ui/view.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- In normal mode the root view draws one compact transport row below the browse list: the four
  glyphs `◀◀`, `▶`/`II`, `■`, `▶▶` in that order at their existing fixed widths, the same
  elapsed/remaining string the LCD shows, and a volume slider tagged `MINI_VOLUME_ID` at the row's
  right edge; the row is exactly `MINI_TRANSPORT_HEIGHT` tall.
- Pressing the footer's four buttons publishes `Message::PreviousTrack`, `Message::PlayPause`,
  `Message::Stop`, and `Message::NextTrack`, and dragging its slider publishes
  `Message::VolumeChange`; no new message, handler, or playback path exists.
- The shade strip keeps its current blue chrome, `SHADE_STRIP_HEIGHT`/`SHADED_HEIGHT` geometry,
  and control messages, and its controls come from the same `mini_transport_controls` spec as the
  footer, so a glyph, order, width, or message change shows up in both.
- Browsing, list scrolling and the wheel, playback, the LCD, and the browse count footer keep
  their current behavior; `make check` passes.
- README and `docs/invariants.md` describe the footer and shade strip as the same compact row, and
  the new tests pin the footer's geometry and message contract.

### Make the O A I D V strip's A and V controls work (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** The mockup's right-hand copy says "try … the O A I D V strip", but the LCD's strip is
only decoration today: `views::now_playing::lcd_indicator_strip` draws five plain `Text`s and the
`## Now Playing label` invariant says the strip emits no message. Give the two letters the app can
already support a real action: `A` toggles always-on-top through the existing
`Message::ToggleAlwaysOnTop` (the classic clutter bar's A, in addition to the title bar's A), and
`V` turns the LCD spectrum visualizer off and back on without touching playback, the stored
spectrum, or its poll. `O`, `I`, and `D` stay decorative: an options menu, a file-info panel, and
pixel-doubled window mode have no implementation here (iced 0.14 exposes no runtime scale-factor
setter). This is independent of the queued mouse-wheel plan: it touches the LCD strip and one
display flag, not the view tree's scroll area.

**Approach.**

- `src/ui/views/now_playing.rs`: add `pub(in crate::ui::views) fn lcd_indicator_message(label:
  &str) -> Option<Message>` returning `Some(Message::ToggleAlwaysOnTop)` for `"A"`,
  `Some(Message::ToggleVisualizer)` for `"V"`, and `None` for `"O"`, `"I"`, and `"D"`. Keep it
  pure so the mapping is testable without an iced renderer.
- In `lcd_indicator_strip`, build the same green `Text` for every label, but wrap the `A` and `V`
  letters in `MouseArea::new(...).on_press(message).interaction(Interaction::Pointer)` and leave
  the other three as plain `Text`. Keep the column at `LCD_INDICATOR_WIDTH` and add no padding, so
  the letters' order, their x position, and the strip's width and height stay as they are.
- Add `pub(in crate::ui::views) fn visible_spectrum(spectrum: Option<Spectrum>,
  visualizer_visible: bool) -> Option<Spectrum>`: `Some` only when the flag is true and a
  spectrum is stored, otherwise `None`. Document that hiding changes only what the LCD draws —
  `WinampPlayer::spectrum` and the spectrum poll keep running, so turning it back on shows the
  current bars.
- Update the `LCD_INDICATOR_LABELS`, `lcd_indicator_strip`, and module docs to say A and V are
  live while O, I, and D stay decorative.
- `src/ui/message.rs`: add `ToggleVisualizer` beside `ToggleTimeDisplay`, documented as the strip
  V control: it flips only the LCD's display flag, changes no playback state, and schedules no
  work.
- `src/ui/handlers/playback.rs`: add `pub(in crate::ui) fn toggle_visualizer(player: &
  mut WinampPlayer) -> Task<Message>` beside `toggle_time_display`; flip
  `player.visualizer_visible` and return `Task::none()`.
- `src/ui/player.rs`: add `pub(in crate::ui) visualizer_visible: bool`, initialized `true` in
  `WinampPlayer::new`, documented as the indicator strip's V state and as leaving the stored
  spectrum untouched.
- `src/ui/mod.rs`: route `Message::ToggleVisualizer =>
  handlers::playback::toggle_visualizer(player)` beside the `ToggleTimeDisplay` arm.
- `src/ui/view.rs`: in the normal branch, pass `views::visible_spectrum(spectrum,
  player.visualizer_visible)` to `views::view_now_playing` where the stored spectrum goes today.
  The shade branch builds no LCD and is untouched.
- `src/ui/views/mod.rs`: re-export `visible_spectrum` from `now_playing` beside the other
  `now_playing` exports, so `view.rs` can apply the flag. `lcd_indicator_message` stays in
  `now_playing` (the tests import it there): both helpers are `pub` inside the private module
  rather than `pub(in crate::ui::views)`, because a `pub use` cannot re-export an item narrower
  than the module it leaves.
- `src/ui/views/tests.rs`: add `lcd_indicator_message_arms_a_and_v_and_leaves_o_i_d_inert`;
  `lcd_indicator_strip_publishes_always_on_top_on_a_and_visualizer_on_v`, using the headless
  renderer and `UserInterface::update` press pattern from `elapsed_readout_toggles_only_while_not_stopped`
  to assert that a press on A publishes `ToggleAlwaysOnTop`, on V publishes `ToggleVisualizer`, and
  on O publishes nothing; and `visible_spectrum_hides_the_stored_bars_while_the_visualizer_is_off`
  covering `Some`/`None` spectra with the flag on and off. Update the comments around
  `lcd_indicator_strip_draws_each_label_in_order_at_a_fixed_width` so the five letters and the
  fixed width are still pinned while the interactivity is documented.
- `src/ui/tests.rs`: add `toggle_visualizer_hides_and_shows_the_bars_without_clearing_the_spectrum`:
  store a known spectrum, drive `Message::ToggleVisualizer` through `update` with
  `assert_message_schedules_no_work`, and assert the flag flips twice while `player.spectrum` and
  the transport/mixer state stay unchanged.
- `README.md`: replace the "small decorative `O A I D V` strip" wording with a description of the
  strip's A and V actions and the still-decorative O, I, and D, and add to the visualizer sentence
  that V hides and restores the bars. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Preview spectrum`, state that the LCD draws the stored spectrum only
  while `WinampPlayer::visualizer_visible` holds, that hiding it keeps both the stored spectrum and
  the poll, and name `views::visible_spectrum`, `Message::ToggleVisualizer`, and
  `handlers::playback::toggle_visualizer` as enforcers. In `## Now Playing label`, replace the
  "strip is decorative: it emits no message" bullet with one saying A and V are mouse areas that
  publish `ToggleAlwaysOnTop` and `ToggleVisualizer`, O/I/D stay decorative, and the strip keeps
  its fixed width and letter order. Add the new test names to the Pinned-by lists.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`,
`src/ui/message.rs`, `src/ui/handlers/playback.rs`, `src/ui/player.rs`, `src/ui/mod.rs`,
`src/ui/view.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Pressing the strip's A publishes `Message::ToggleAlwaysOnTop`, the same handler as the title
  bar's A; pressing V hides the LCD bars and pressing it again restores them from the still-stored
  spectrum, without clearing `WinampPlayer::spectrum`, changing transport or mixer state,
  scheduling work, or stopping the spectrum poll.
- O, I, and D publish no message; the strip still draws `O`, `A`, `I`, `D`, `V` in that order at
  `LCD_INDICATOR_WIDTH`, and the content beside it keeps its bounds in every playback state.
- The toggle is pinned at the mapping, press, and handler levels, and `make check` passes.
- The README and `docs/invariants.md` describe the same live letters and the hidden-but-still-polled
  behavior; the title bar, shade strip, and playback keep their current behavior, and no dependency
  is added.

### Adjust the volume with the mouse wheel (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Classic Winamp changes the volume when you roll the mouse wheel over the main window,
and Nostalgia currently has no wheel behavior outside the browse list's own scrollbar. Make a roll
over the player panels raise or lower the volume by one slider step, using the same
`AppState::nudge_volume` path and backend forward the volume slider and up/down arrow keys already
use, so the thumb and the playing preview follow immediately. A roll over the browse list must keep
scrolling the list and leave the volume alone; a horizontal-only roll must do nothing. This touches
only the view tree, one message, and its handler; it is independent of the queued settings plan.

**Approach.**

- `src/ui/views/mod.rs`: add `pub(super) fn volume_wheel_message(delta: iced::mouse::ScrollDelta)
  -> Message`. For both `ScrollDelta::Lines` and `ScrollDelta::Pixels`, return
  `Message::VolumeWheel(1.0)` when the vertical delta is positive, `Message::VolumeWheel(-1.0)`
  when it is negative, and `Message::VolumeWheel(0.0)` for a horizontal-only roll (`y == 0.0`).
  Document that iced passes winit's positive-y wheel direction (the direction that scrolls up) and
  that the sign, not the magnitude, is the step, so one event moves the slider once regardless of
  the backend's delta units. One event is one step, like one arrow press.
- `src/ui/message.rs`: add `VolumeWheel(f32)` with a doc comment naming it the mouse wheel's
  vertical direction over the player panels (positive raises, negative lowers, zero is a no-op),
  and saying the browse list is deliberately outside the area that publishes it.
- `src/ui/mod.rs`: route `Message::VolumeWheel(direction)` in the `update` match to
  `handlers::mixer::volume_wheel`.
- `src/ui/handlers/mixer.rs`: add `pub(in crate::ui) fn volume_wheel(player: &mut WinampPlayer,
  direction: f32) -> Task<Message>`. Return `Task::none()` for a zero or non-finite direction
  (the `direction > 0.0` / `direction < 0.0` test leaves NaN on the no-op path), otherwise call the
  existing `volume_up` or `volume_down` handler unchanged. That reuses `views::VOLUME_STEP`, the
  `nudge_volume` clamp, and the audio-backend forward, so the wheel cannot drift from the arrows.
- `src/ui/view.rs`: in the normal branch, move the title bar, Now Playing bar, seek strip,
  transport row, equalizer title band, and equalizer panel into one `Column`, wrap that column in
  `iced::widget::MouseArea::new(...).on_scroll(views::volume_wheel_message)`, and push the wrapped
  area where those panels were. Leave the library title band, Back/Retry, the All Songs/All
  Albums/Playlists buttons, the Top/Sort row, the search box, and the browse list outside the
  `MouseArea`, so the list's own `Scrollable` still takes the wheel. In the shade branch, wrap the
  title-bar-plus-shade-strip `Column` the same way, since the strip carries a volume slider. The
  `MouseArea` forwards events to its content first, so the title bar's drag/double-click area,
  the LCD's clickable readout, the sliders, and the buttons keep their existing behavior.
- `src/ui/views/tests.rs`: add `volume_wheel_message_maps_a_vertical_roll_to_a_direction`, covering
  positive and negative `Lines` and `Pixels` deltas, and
  `volume_wheel_message_zeroes_a_horizontal_only_roll`, covering `Lines { x: 1.0, y: 0.0 }` and its
  `Pixels` twin.
- `src/ui/tests.rs`: add `volume_wheel_nudges_by_one_slider_step_and_ignores_zero`, driving
  `Message::VolumeWheel(1.0)`, `Message::VolumeWheel(-1.0)`, and `Message::VolumeWheel(0.0)`
  through `assert_message_schedules_no_work` and asserting the shared volume moves by exactly
  `views::VOLUME_STEP` for the first two and is unchanged for zero. Add
  `volume_wheel_clamps_at_the_ends` beside `volume_keys_clamp_at_the_ends`.
- `README.md`: after the Usage sentence saying the volume slider and up/down arrows set the playback
  level, add that rolling the mouse wheel over the player panels nudges the volume by one slider
  step in that direction, while over the browse list the wheel keeps scrolling the list. Do not
  touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Volume wheel` section after `## Level slider fill` stating that a
  wheel roll over the player panels moves the volume by exactly one `VOLUME_STEP` in the roll's
  vertical direction, through `AppState::nudge_volume` and the same backend forward as the arrows;
  that the browse list is outside that area, so its `Scrollable` keeps the wheel and the volume is
  unchanged; and that a horizontal-only roll or a zero direction is a no-op. Name
  `views::volume_wheel_message`, `Message::VolumeWheel`, `handlers::mixer::volume_wheel`, and the
  two `MouseArea::on_scroll` wrappers in `ui::view` as enforcers, and the new test names as pins.

**Files touched.** `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/message.rs`,
`src/ui/mod.rs`, `src/ui/handlers/mixer.rs`, `src/ui/view.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- A wheel roll over the title bar, Now Playing bar, seek strip, transport row, or equalizer panel
  raises the volume by one `views::VOLUME_STEP` when the vertical delta is positive and lowers it by
  one step when it is negative, for both line and pixel deltas, with the shared state clamping at
  the ends and forwarding the level to the audio backend exactly as the arrow keys do.
- A wheel roll over the browse list still scrolls the list and changes no volume; a horizontal-only
  roll and a zero direction change no volume and schedule no work.
- The wheel uses the existing `VolumeUp`/`VolumeDown` behavior through `handlers::mixer::volume_wheel`,
  so no second clamping rule, step size, backend path, timer, or subscription is added; no dependency
  is added.
- The mapping tests cover both delta kinds and the horizontal-only case, the handler tests cover
  both directions, zero, and the clamps, and `make check` passes.
- The README and the new invariant section describe the same behavior, and the browse list's
  scrolling, the title bar's drag/shade, the LCD click, and the sliders keep their current behavior.

### Save volume, balance, Repeat, Shuffle, and the equalizer across launches (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** The player already keeps its mixer and equalizer settings in `AppState` while it runs —
volume, balance, Repeat, Shuffle, the equalizer's on/off state, preamp, ten band gains, and the
selected preset — but every launch starts again at the `AppState::default()` values, and the README
says plainly that there are no config files. Persist exactly those user-set values in one small text
file, load them before the first frame, and rewrite the file whenever one of them changes, so a
restart restores the mix and curve. Playback position, the showing browse level, the search box,
window placement, and the current track are deliberately not persisted. No new dependency, no
network, and no UI change; this is independent of the title-band work (that change touched the
view tree only).

**Approach.**

- `src/settings.rs` (new): define `pub struct Settings` with private fields `volume`, `balance`,
  `repeat`, `shuffle`, `eq_enabled`, `eq_preamp`, `eq_bands`, and `eq_preset`, deriving
  `Debug, Clone, PartialEq`. Its `Default` mirrors `AppState::default` (0.5 volume, centered balance,
  both toggles off, equalizer off with a flat curve and no preset); document that the two defaults
  must change together. Add `pub fn from_state(&AppState) -> Settings`, reading the existing public
  getters and fields, and `pub fn apply_to(&self, &mut AppState)`, writing through the existing
  clamped setters (`set_volume`, `set_balance`, `set_eq_preamp`, each `set_eq_band`, then
  `apply_eq_preset` when `eq_preset` is `Some`) rather than assigning the private state fields.
- In the same module add private `to_text`/`from_text` for a line-based `key=value` format with keys
  `volume`, `balance`, `repeat`, `shuffle`, `eq_enabled`, `eq_preamp`, `eq_band_0` through
  `eq_band_9`, and `eq_preset`. Booleans are `true`/`false`; the preset is its `Preset::name` or an
  empty value for a custom curve. `from_text` starts from `Settings::default()`, ignores unknown
  keys, malformed numbers, bad booleans, out-of-range band indexes, and an unrecognized preset name,
  and matches a preset name against `equalizer::PRESETS`.
- In the same module add `pub trait SettingsWriter: Send + Sync { fn save(&self, settings: &Settings); }`
  and `pub struct FileStore { path: Option<PathBuf> }` (deriving `Clone`). `FileStore::from_environment`
  resolves `NOSTALGIA_SETTINGS_PATH` when set and non-empty, else `$XDG_CONFIG_HOME/nostalgia/settings.ini`,
  else `$HOME/.config/nostalgia/settings.ini`, else no path (persistence disabled); document that the
  environment variable is the cross-platform override and is read once at startup. Add
  `FileStore::at(PathBuf)` for tests, `FileStore::load(&self) -> Option<Settings>` (a missing or
  unreadable file returns `None`; the text is parsed with `from_text`), and the `SettingsWriter` impl
  whose `save` creates the parent directory when it has one, writes `to_text`, and logs a failed
  create/write with `eprintln!` instead of panicking.
- `src/main.rs`: declare `pub mod settings;`. Build the state, then load once before wrapping it in
  the shared mutex: `let store = settings::FileStore::from_environment(); let saved =
  store.load().unwrap_or_default(); saved.apply_to(&mut state);` and pass both the store and `saved`
  to `ui::init_ui`.
- `src/ui/mod.rs`: extend `init_ui` to take the store and the loaded `Settings`. In the iced
  application builder closure, after `boot`, set `player.settings_store = Some(Box::new(store.clone()))`
  and `player.saved_settings = saved.clone()`. In `update`, bind the dispatch `match` to a variable,
  call `player.persist_settings_if_changed()` once after it, and return the task, so every message
  path is covered without adding a save call to each handler.
- `src/ui/player.rs`: add `pub(in crate::ui) settings_store: Option<Box<dyn SettingsWriter>>` and
  `pub(in crate::ui) saved_settings: Settings` fields, initialized in `WinampPlayer::new` to `None`
  and `Settings::default()` (production `init_ui` replaces both; tests keep the no-op default). Add
  `pub(in crate::ui) fn persist_settings_if_changed(&mut self)`: return immediately when no store is
  attached; otherwise take one state lock, build `Settings::from_state`, return when it equals
  `saved_settings`, else call the store's `save` and store the new snapshot. Document that the
  snapshot comparison is what keeps timer and polling messages from writing.
- `src/ui/tests.rs`: add a test-only `MemorySettingsStore` over `std::sync::Mutex<Vec<Settings>>`
  implementing `SettingsWriter`. Add `a_changed_setting_is_saved_once_and_unchanged_updates_are_not`
  (attach the store, send `Message::VolumeChange(0.8)`, assert the saved snapshot equals
  `Settings::from_state` of the shared state, then send `Message::Ignored` and assert no second
  entry) and `repeat_and_equalizer_changes_reach_the_saved_snapshot` (send `Message::ToggleRepeat`
  and `Message::EqPresetSelected(rock_preset())` and assert the saved snapshot carries both). Keep
  these as plain `#[test]` functions, because `persist_settings_if_changed` uses `blocking_lock`.
- `src/settings.rs` module tests: pin `to_text`/`from_text` round-tripping every field including a
  selected preset; unknown keys, a non-numeric volume, a bad boolean, a bad band index, and an
  unknown preset name each leaving the default; `apply_to` clamping out-of-range file values through
  the state setters; and `FileStore::save` followed by `load` returning the same `Settings` through a
  unique path under `std::env::temp_dir()` (with the file removed afterward).
- `README.md`: replace the closing "There are no CLI flags or config files yet." sentence with a
  description of the settings file: volume, balance, Repeat, Shuffle, and the equalizer's on/off
  state, curve, and preset are restored at the next launch, the path is `$XDG_CONFIG_HOME/nostalgia/settings.ini`
  (or `$HOME/.config/...`), `NOSTALGIA_SETTINGS_PATH` overrides it, and missing or malformed entries
  fall back to the defaults. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Settings persistence` section after `## UI layout` stating that the
  listed mixer/equalizer values are loaded once before the first view and rewritten after any update
  that changes their snapshot; that reads and writes go through the state's clamped setters; that a
  missing/unreadable file or a malformed line falls back per key without failing startup; and that
  playback position, browse level, search, current track, and window placement are not persisted.
  Name `settings::Settings::from_state`/`apply_to`, `settings::FileStore::load`/`save`,
  `ui::init_ui`, and `WinampPlayer::persist_settings_if_changed` as enforcers and the new test names
  as the pins.

**Files touched.** `src/settings.rs` (new), `src/main.rs`, `src/ui/mod.rs`, `src/ui/player.rs`,
`src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- At startup the player restores the volume, balance, Repeat and Shuffle flags, equalizer on/off
  state, preamp, ten band gains, and selected preset from the resolved settings file before the
  first view is drawn; a missing or unreadable file leaves `AppState::default()` values, and a
  malformed line affects only its own key.
- Changing any of those values through the UI writes the current snapshot to the file once; an
  update that leaves the snapshot unchanged writes nothing; the parent directory is created when
  needed and a write failure is logged rather than crashing.
- The text format round-trips every field (preset included), out-of-range values are clamped when
  loaded, and `NOSTALGIA_SETTINGS_PATH`, `$XDG_CONFIG_HOME`, and `$HOME/.config` resolve in that
  order with the first two honored only when non-empty.
- The settings tests pin the round-trip, the ignored malformed input, the clamping, and the file
  save/load; the UI tests pin save-on-change and no-save-on-unchanged; `make check` passes; the
  README and the new invariant section describe the same behavior; no dependency is added.

### Draw the mockups' gold title tape and the equalizer and library title bands (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match the title chrome every mockup draws (`docs/screenshots/hero.png`, `player.png`,
`transport.png`, `equalizer.png`, and `library.png`): a blue band with a gold two-tone tape strip
on each side of a centered label. The main bar today is a plain blue `NOSTALGIA` band with no tape,
and the equalizer and browse sections have no headers at all. Add the tape to the app bar and draw
`NOSTALGIA EQUALIZER` above the equalizer and `NOSTALGIA LIBRARY` above the browse controls. The
section bands carry no window buttons: this app is one real window whose controls already live in
the app bar, so a second set would be clickable-looking dead chrome. This is presentation only —
no message, state, playback, audio, browse-order, or scroll behavior changes — and it is
independent of the queued playing-row plan (that plan changes where the list scrolls; this one
adds two full-width header rows).

**Approach.**

- `src/ui/theme.rs`: add `TITLE_TAPE` (the mockup's bright gold face, sampled as `(236, 206, 122)` →
  `Color::from_rgb(0.93, 0.81, 0.48)`) and `TITLE_TAPE_SHADOW` (its darker bottom edge, sampled as
  `(163, 148, 106)` → `0.64, 0.58, 0.42`), each documented as a title-tape color. Extend the
  existing base-skin color test to pin both values and that they differ from `TITLE_BLUE`.
- `src/ui/views/title_bar.rs`: add a private `title_tape() -> Element<'static, Message>`: an
  8px-tall `Length::Fill` strip, vertically centered, with the gold face between a lighter top
  edge and the `TITLE_TAPE_SHADOW` bottom edge (a 1px top rule and 1px bottom rule around a
  `Length::Fill` face is enough; the mockup's tape rows measure 2px each). Add `pub fn view_title_band(label: &'static str) -> Element<'static, Message>`: a
  `bevel::raised_panel` over a `Container` painted with `style::title_bar_style()`, height
  `TITLE_BAR_HEIGHT`, holding a `Row` with spacing and padding of `title_tape()`, the label as
  `Text::new(label).size(14).color(theme::TEXT)`, and another `title_tape()`, so the two tapes
  fill equally on each side and the label sits centered between them. Document that the band is
  decorative and deliberately has no buttons. Refactor `view_title_bar` to put the same
  tape/label/tape `Row` inside its existing `MouseArea` drag region (so the app name becomes
  centered between the tapes, as in the mockup, while the A and shade/minimize/close controls keep
  their current pinned widths and order); the drag press and double-click still cover exactly
  that middle span. Update the module doc.
- `src/ui/views/mod.rs`: re-export `view_title_band` beside `view_title_bar` and
  `TITLE_BAR_HEIGHT`.
- `src/ui/view.rs`: in the normal-mode column, push
  `views::view_title_band("NOSTALGIA EQUALIZER")` immediately before
  `views::view_equalizer(...)`, and `views::view_title_band("NOSTALGIA LIBRARY")` immediately
  before the Back/Retry/Top/Sort/search/list group. Leave the shaded early return untouched, so
  shade mode still builds only the app bar and the strip and neither section band appears.
- `src/ui/views/tests.rs`: add `title_band_centers_its_label_at_the_title_bar_height`, capturing
  `view_title_band("NOSTALGIA EQUALIZER")` with `layout_capture` and asserting the captured height
  equals `TITLE_BAR_HEIGHT`, the label is the only text fragment, and the label's center is the
  captured width's center. Add `title_bar_centers_its_name_between_its_edge_controls`, capturing
  `view_title_bar(false)` and asserting `NOSTALGIA`'s center is the midpoint between the A
  button's right edge and the shade button's left edge; keep the existing pinned-width test.
- `src/ui/tests.rs`: add `root_view_draws_the_equalizer_and_library_title_bands_in_order`: capture
  the normal root view with an artist list loaded, find `NOSTALGIA EQUALIZER`, `NOSTALGIA
  LIBRARY`, and the first browse row's text, and assert the equalizer band's `y` is above the
  library band's, which is above the first row's. Add
  `shaded_root_view_draws_no_section_title_bands`, asserting the shaded root view contains neither
  section label while the app-bar `NOSTALGIA` still draws. Keep the existing shade and root-view
  tests green.
- `README.md`: in the Usage paragraph that describes the equalizer and browse list, say those two
  sections are introduced by `NOSTALGIA EQUALIZER` and `NOSTALGIA LIBRARY` title bands and that
  the app bar sets `NOSTALGIA` between gold tapes. Do not touch the `<!-- tumwater:status:start
  -->` block.
- `docs/invariants.md`: in `## UI layout`, add a bullet: the app bar and the two section bands are
  `TITLE_BAR_HEIGHT` tall; the app bar centers `NOSTALGIA` between gold tapes with its A and
  shade/minimize/close controls pinned at the edges; normal mode draws `NOSTALGIA EQUALIZER`
  immediately above the equalizer and `NOSTALGIA LIBRARY` immediately above the browse controls;
  the section bands emit no messages and are absent in shade mode. Name `views::view_title_band`,
  `views::title_bar::title_tape`, `theme::TITLE_TAPE`/`TITLE_TAPE_SHADOW`, and the new tests as
  the enforcers and pins.

**Files touched.** `src/ui/theme.rs`, `src/ui/views/title_bar.rs`, `src/ui/views/mod.rs`,
`src/ui/view.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- The app bar draws `NOSTALGIA` centered between two gold tape strips on the blue band, keeps its A
  and shade/minimize/close controls in their current order and pinned widths, and still drags on
  press and toggles shade on double-click across the tape/label span.
- Normal mode draws a `TITLE_BAR_HEIGHT`-tall `NOSTALGIA EQUALIZER` band directly above the
  equalizer and a `NOSTALGIA LIBRARY` band directly above the browse controls, both blue with the
  same tape and no clickable controls; shade mode draws neither band.
- The band test pins label centering and height, the app-bar test pins the name's centered span
  between its edge controls, and the root-view tests pin the two labels' vertical order above the
  first browse row and their absence when shaded.
- No message, state, playback, audio, browse-order, or existing control-order change; no
  dependency is added; `make check` passes; the README and the new invariant bullet describe the
  same chrome.

### Keep the playing row in view when the transport steps (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match the classic Winamp playlist editor's follow-the-playing-row behavior. The browse list
today scrolls only on Top, Back, and the navigation that opens a fresh list; pressing Next/Previous
(a shuffled step and a shuffled Previous retrace included), or letting a preview run out, can move the
playing row outside the visible list while the `▶` marker stays on a row the user cannot see. When
such a playback step lands on a song while a song list (`CurrentView::Songs` or
`CurrentView::AllSongs`) is showing, scroll that list so the landed row is visible. Pressing a row
directly keeps the list where it is (the pressed row is already in view), and browse navigation keeps
its existing snap-to-top behavior. No playback, selection, sort, row-content, or audio behavior
changes, and no dependency is added. This plan is independent of the queued transport-glyph plan.

**Approach.**

- `src/ui/views/browse/rows.rs`: add `pub const BROWSE_ROW_HEIGHT: f32` beside `BROWSE_LIST_ID`, documented
  as the vertical pitch between consecutive laid-out browse rows (button padding plus row text height).
  The implementer measures the actual pitch with the new layout test below rather than guessing a value,
  and keeps the constant as the single source for the scroll offset. Re-export it from
  `src/ui/views/browse.rs` and `src/ui/views/mod.rs` beside `BROWSE_LIST_ID`.
- `src/ui/handlers/browse.rs`: beside `scroll_browse_to_top`, add `pub(in crate::ui) fn
  scroll_browse_to_row(index: usize) -> Task<Message>` that schedules
  `iced::widget::operation::scroll_to(views::BROWSE_LIST_ID,
  iced::widget::scrollable::AbsoluteOffset { x: None, y: Some(index as f32 * views::BROWSE_ROW_HEIGHT) })`,
  so iced clamps the offset to the list's content. Add `pub(in crate::ui) fn
  scroll_browse_to_playing_row(player: &WinampPlayer, index: usize) -> Task<Message>` that returns
  `Task::none()` unless `player.current_view` is `CurrentView::Songs` or `CurrentView::AllSongs`, and
  otherwise returns `scroll_browse_to_row(index)`. Document that the row press path deliberately does
  not call it.
- `src/ui/handlers/playback.rs`: in `step_track`'s `Some(index)` arm, return
  `Task::batch([Task::done(Message::TrackSelected { epoch, index }),
  scroll_browse_to_playing_row(player, index)])` instead of only the selection task. This covers Next
  and Previous, including the shuffled Previous retrace, because all of them resolve to that arm.
- `src/ui/handlers/playback/preview.rs`: in `playback_finished`'s `Some(index)` arm, batch
  `Message::TrackSelected` with the same `scroll_browse_to_playing_row(player, index)`, so a natural
  preview end follows the new row exactly as Next does; the `None` arm still takes the existing
  `stop` path.
- `src/ui/tests.rs`: generalize the existing scroll capture in
  `assert_task_snaps_browse_list_to_top` (or add a sibling helper) so a test can observe an absolute
  `scroll_to` offset as well as a relative `snap_to`. Add
  `browse_row_height_matches_the_laid_out_row_pitch`, which lays out the root `view` with
  `stepping_songs()` loaded and `CurrentView::Songs` showing, finds the `1. ` and `2. ` row prefixes
  through `captured_root_view_text_bounds`, and asserts their y delta equals `views::BROWSE_ROW_HEIGHT`
  within a small epsilon. Add
  `next_and_previous_scroll_the_showing_song_list_to_the_landed_row` (from `song-2`, Next scrolls to
  `2 * BROWSE_ROW_HEIGHT` and Previous to `0.0`), and
  `a_finished_preview_scrolls_the_song_list_to_the_advanced_row` (from `song-1`, a finished preview
  scrolls to `BROWSE_ROW_HEIGHT`). Add
  `stepping_while_another_browse_level_shows_does_not_scroll` (current view Artists, Next still
  schedules its selection but no scroll operation) and `a_row_press_does_not_scroll_the_list`
  (`Message::TrackSelected` schedules no scroll operation).
- `README.md`: in the Usage paragraph's transport sentence, add that Next, Previous, and a finished
  preview scroll the showing song list to keep the playing row visible. Do not touch the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a short `## Browse playing-row visibility` section after
  `## Browse row order`: when Next, Previous (including Shuffle's retrace), or the preview-end advance
  lands on a song while a song list is showing, the update loop scrolls `BROWSE_LIST_ID` to
  `index * BROWSE_ROW_HEIGHT`, `BROWSE_ROW_HEIGHT` equals the laid-out row pitch, direct row presses
  and other browse levels do not scroll, and browse navigation keeps its snap-to-top behavior. Name
  `handlers::browse::scroll_browse_to_row` / `scroll_browse_to_playing_row`,
  `handlers::playback::step_track`, `handlers::playback::preview::playback_finished`, and
  `views::BROWSE_ROW_HEIGHT` as the enforcers, and list the new test names as the pins.

**Files touched.** `src/ui/views/browse/rows.rs`, `src/ui/views/browse.rs`, `src/ui/views/mod.rs`,
`src/ui/handlers/browse.rs`, `src/ui/handlers/playback.rs`,
`src/ui/handlers/playback/preview.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Next, Previous (including a shuffled Previous retrace), and the preview-end advance each schedule a
  scroll of the showing song list to the landed row's `index * BROWSE_ROW_HEIGHT`, in addition to the
  existing `TrackSelected`; the row is therefore brought into view for any list longer than the window.
- The absolute offset matches the laid-out row pitch: the root-view layout test finds consecutive row
  prefixes one `BROWSE_ROW_HEIGHT` apart, so a later row cannot drift out of view from a stale constant.
- Starting a step while a non-song browse level is showing, or pressing a row directly, schedules no
  scroll; Top, Back, and every list-opening navigation still snap the list to the top exactly as before.
- Playback, selection, sorting, row text, and audio behavior are unchanged, no dependency is added,
  `make check` passes, and the README and the new invariant section describe the same behavior.

### Draw the main transport row with the mockup's classic glyphs (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match `docs/screenshots/player.png`, `docs/screenshots/hero.png`, and `docs/screenshots/transport.png`, whose main transport row draws Previous, Play/Pause, Stop, and Next as compact glyphs in that order. The build draws the same four controls as the words `Play`/`Pause`, `Stop`, `Previous`, and `Next`. Replace those four faces with `◀◀`, `▶`/`II`, `■`, and `▶▶`, sharing the exact glyphs, order, widths, and messages the shade strip already uses, and keep a word label on each glyph button's hover tooltip. Repeat and Shuffle remain the existing visible `Repeat: On/Off` and `Shuffle: On/Off` text toggles, and every button keeps its fixed width so the glyph swap cannot reflow the row. This is presentation only: no message, state, playback, or audio behavior changes, and no dependency is added.

**Approach.**

- `src/ui/views/transport.rs`: add `pub(super) fn play_pause_glyph(is_playing: bool) -> &'static str` (`"II"` while playing, `"▶"` otherwise) and `pub(super) fn play_pause_word(is_playing: bool) -> &'static str` (`"Pause"` while playing, `"Play"` otherwise), moving the glyph helper out of `shade`. Add `pub(super) fn transport_glyphs(is_playing: bool) -> [(&'static str, &'static str, f32, Message); 4]`, the pure row order and spec: Previous `("◀◀", "Previous", 34.0, Message::PreviousTrack)`, Play/Pause `(play_pause_glyph, play_pause_word, 30.0, Message::PlayPause)`, Stop `("■", "Stop", 30.0, Message::Stop)`, Next `("▶▶", "Next", 34.0, Message::NextTrack)`. Rebuild `transport_buttons` from that spec, wrapping each `fixed_width_button` in `iced::widget::Tooltip` with the word as a small `Text` tooltip above it; then push the unchanged fixed-width `repeat_label` and `shuffle_label` buttons (104 and 112 px) in that order. Update the module and function docs for the glyph row and tooltips.
- `src/ui/views/shade.rs`: remove the local `play_pause_glyph`; build `shade_controls` from `transport_glyphs` by dropping each entry's tooltip word, so the strip and the full row cannot drift on glyph, order, width, or message. Update the shade doc comment and keep the strip's own no-tooltip layout.
- `src/ui/views/mod.rs`: update the `labeled_button`/`fixed_width_button` doc references from `play_pause_label` to the glyph helpers; no code change is needed there.
- `src/ui/views/tests.rs`: change the import to `play_pause_glyph`/`play_pause_word`/`transport_glyphs`; replace `play_pause_label_mirrors_playing_state` with glyph and word assertions; add `transport_glyphs_match_the_mockup_order_and_widths` asserting the four glyph/word/width/message tuples exactly; keep `transport_buttons_keep_a_fixed_width_across_label_changes` and update `transport_buttons_keep_their_measured_face_widths` to `[34.0, 30.0, 30.0, 34.0, 104.0, 112.0]`; update the shade tests for the moved helper and add `main_transport_and_shade_strip_share_glyphs_order_and_widths`, comparing `transport_glyphs` with `shade_controls`.
- `src/ui/tests.rs`: change `transport_row_does_not_reflow_when_the_play_label_changes` to `transport_row_does_not_reflow_when_the_play_glyph_changes`, asserting the `Repeat: Off` and `Shuffle: Off` x positions stay fixed across stopped and playing (the `Stop` word is no longer drawn), and add `root_view_draws_the_transport_glyph_row`, which lays out the root `view` through `captured_root_view_text_bounds` and asserts `◀◀` and `▶▶` draw in the same row and in Previous-before-Next order. Update any existing root-view text lookup that used a removed transport word.
- `README.md`: in the Usage paragraph's transport sentence, say the bar sits above Previous, Play/Pause, Stop, and Next glyph controls with word tooltips, then the Repeat and Shuffle toggles, the sliders, the equalizer, and the browse list. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## UI layout`, replace the transport-label bullet with the glyph-row guarantee (order, exact glyphs, fixed widths, tooltip words, Repeat/Shuffle text unchanged) and name `views::transport::transport_glyphs`/`transport_buttons` and the new tests. In `## Window shade`, change the enforcer reference from `views::shade::play_pause_glyph` to the shared `views::transport::play_pause_glyph` and state that the strip and full row share the glyph/order/width spec.

**Files touched.** `src/ui/views/transport.rs`, `src/ui/views/shade.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The main transport row draws, in order, `◀◀`, the state-dependent `▶`/`II`, `■`, and `▶▶`, each with the same `Message` as today and a hover tooltip reading `Previous`, `Play`/`Pause`, `Stop`, or `Next`; Repeat and Shuffle remain the existing text toggles.
- The four glyph buttons use the fixed widths 34, 30, 30, and 34 px, so Play/Pause swapping `▶` and `II` leaves every later control's x position unchanged; Repeat and Shuffle keep their 104 and 112 px widths.
- The shade strip builds its four buttons from the same `transport_glyphs` spec, so its glyphs, order, widths, and messages match the main row.
- The root layout test finds the `◀◀` and `▶▶` glyphs in one row, and the reflow test confirms the later toggles do not move across the playback-state swap.
- No playback, state, browse, or audio behavior changes, no dependency is added, `make check` passes, and the README and both invariant sections describe the same row.

### Prefix the Now Playing LCD title with the track's playlist number (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match the LCD title line in `docs/screenshots/player.png`, which reads `1. THE SAMPLE BAND - OPENING` before the right-edge total, while the build draws only `Now Playing: The Sample Band - Opening`. Record the 1-based position of the row that selected the current track and draw its prefix (`1. `) after the `Now Playing: ` caption and before the artist/title. The number follows every new selection, including Next/Previous and the preview-end advance, stays with the track after browsing to another list (it is cached in `KnownTrack` beside the title and artist), and is absent when no track is current or the id is unknown. This is presentation only: playback, browse order, selection resolution, sorting, and the marquee timer's schedule are unchanged.

**Approach.**

- `src/ui/views/now_playing/text.rs`: add `pub number: String` to `KnownTrack`, documented as the cached `"<n>. "` prefix (`""` when none). Add `pub fn now_playing_number<'a>(tracks, current_track) -> Cow<'a, str>` mirroring `now_playing_artist`: borrow the known track's `number`, or `""` when no track is current or the id is unknown. Change `display_len` to take the number prefix and add its character count, and `needs_marquee` to take and pass it, so the prefix counts against `MARQUEE_WIDTH` and the timer runs exactly when the drawn line overflows. Update the doc comments to mention the prefix.
- `src/ui/handlers/playback.rs`: in `track_selected`, write the resolved row's number into its `KnownTrack` entry as `format!("{}. ", index + 1)`; replace the `.entry(...).or_insert_with` with an insert/entry update so re-selecting the same id from a different row or list refreshes the number with the other cached fields. Keep `track_played`'s prune-to-current behavior, which retains the current track's number.
- `src/ui/views/now_playing.rs`: re-export `now_playing_number` (and from `src/ui/views/mod.rs`); add a `number: Cow<'a, str>` parameter after `artist` to `view_now_playing`, `now_playing_content`, and `now_playing_row`; and thread it through. Measure `display_len(&label, &artist, &number)`. In the fitting branch, push `lcd_text(number)` after the caption only when it is non-empty, then the artist and title as today. In the scrolling branch, build `format!("{number}{}", display_title_for(&artist, &label))` and window that, so the number scrolls with the line instead of overflowing the budget. Update the module and function docs.
- `src/ui/player.rs`: add a `now_playing_number` method beside `now_playing_artist`, and make `needs_marquee` resolve the number from the same known-track index before calling `views::needs_marquee(&label, &artist, &number)`.
- `src/ui/view.rs`: resolve `now_playing_number` inside the existing state-lock block and pass it to `views::view_now_playing`; the bar therefore uses the cached number, not the currently drawn list.
- `src/ui/views/tests.rs`: extend `captured_now_playing_texts` with a number argument (empty in the existing calls) and add coverage that `1. ` draws between the `Now Playing: ` caption and the artist; that an overlong line's marquee window starts with the prefix; that `display_len`/`needs_marquee` count the prefix; and that an artist-less, unknown, or absent track draws no prefix. Add the number field to every `KnownTrack` literal.
- `src/ui/tests.rs`: update the `KnownTrack` literals and add `track_selected_records_the_selected_rows_number` (index 0 → `"1. "`, index 1 → `"2. "`), `replaying_a_track_from_another_row_updates_its_number`, and `now_playing_number_survives_browsing_to_another_album` (reuse the `play_song_1_then_browse_to_album_2` fixture). Extend `the_player_knows_when_the_now_playing_line_needs_the_marquee` so the prefix can push a borderline line over the budget.
- `README.md`: in the Usage sentence that describes the LCD title, say the line is prefixed by the playing track's 1-based position in the list it was played from (`1. The Sample Band - Opening`); do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, add the number guarantee: the known track's line draws `<n>. ` before its artist/title from the row that selected it, the number follows every new selection (Next/Previous and the preview-end advance included), persists across browsing away and Stop, and is absent for an unknown id or no current track; the prefix counts toward the marquee budget. Name `KnownTrack::number`, `views::now_playing_number`, `handlers::playback::track_selected`, `now_playing_row`, `display_len`, and `needs_marquee` as the enforcers and list the new tests.

**Files touched.** `src/ui/views/now_playing/text.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/player.rs`, `src/ui/view.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Selecting the first row of any song list makes the LCD read `Now Playing: 1. <artist> - <title>`, and the second row reads `2. `; Next/Previous and the preview-end advance update the number with the new row, while re-playing the same track from another row updates it too.
- The number survives browsing to another list and Stop, and is absent when no track is current or the id is unknown.
- The prefix counts toward `MARQUEE_WIDTH`, and an overlong line scrolls a window that includes it; no playback, browse, sort, selection, or audio behavior changes.
- The new view and handler tests pin the prefix, persistence, and marquee behavior, `make check` passes, and the README and invariant section describe the same behavior.

### Fill the volume and balance sliders with the mockup's orange and green rails (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match `docs/screenshots/player.png` and `docs/screenshots/transport.png`, which draw the transport's volume slider over an orange groove and its balance slider over a green groove, each with the same chrome thumb. The build styles both through `chrome_slider` (`src/ui/views/mod.rs`) with `style::chrome_slider_style`, whose near-black `LCD_BACKGROUND` rail makes the two sliders look identical. Give the level sliders their own filled rail while leaving the equalizer preamp and the seek slider on the neutral chrome groove. This is presentation only: slider ranges, steps, messages, thumb geometry, and every playback behavior stay unchanged.

**Approach.**

- `src/ui/theme.rs`: add `pub const VOLUME_SLIDER_FILL: Color = Color::from_rgb(0.88, 0.49, 0.15);` and `pub const BALANCE_SLIDER_FILL: Color = Color::from_rgb(0.09, 0.57, 0.04);`, documented as the mockup's orange volume groove and green balance groove. Extend `base_skin_colors_keep_their_documented_literal_values` with both exact values and assert they differ from each other and from `LCD_BACKGROUND`.
- `src/ui/style.rs`: add `pub fn level_slider_style(fill: Color, status: slider::Status) -> slider::Style` beside `band_slider_style`. It starts from `chrome_slider_style(status)` and sets `rail.backgrounds` to `(Background::Color(fill), Background::Color(fill))`, so both rail halves carry the fill; the 4px rail, 1px dark border, square geometry, and chrome handle stay exactly `chrome_slider_style`'s, with only the handle face following hover/drag. Add `level_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb`, iterating `VOLUME_SLIDER_FILL`, `BALANCE_SLIDER_FILL`, and the Active/Hovered/Dragged statuses, asserting the exact rail backgrounds and equality of the rail width, rail border, and handle with `chrome_slider_style`.
- `src/ui/views/mod.rs`: give `chrome_slider` a `fill: Color` parameter after `width` and style the widget with `move |_theme, status| style::level_slider_style(fill, status)`. Update its doc comment to say it builds the filled level sliders (transport volume and balance, shade volume) rather than a neutral chrome groove.
- `src/ui/views/transport.rs`: pass `theme::VOLUME_SLIDER_FILL` to the volume slider and `theme::BALANCE_SLIDER_FILL` to the balance slider, importing `theme` beside `Message`.
- `src/ui/views/shade.rs`: in `shade_volume_slider`, pass `theme::VOLUME_SLIDER_FILL` to `chrome_slider`; keep `shade_volume_spec`'s range, width, and message tuple unchanged.
- Leave `src/ui/views/seek_bar.rs` and the equalizer preamp's direct `style::chrome_slider_style` calls on the neutral groove, so only the level sliders gain a fill.
- `README.md`: in the Usage sentence naming the volume and balance sliders, say the volume slider's groove is orange and the balance slider's is green, matching the mockup. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Level slider fill` section after `## Equalizer band tint`: the transport volume and shade volume rails are `theme::VOLUME_SLIDER_FILL`, the transport balance rail is `theme::BALANCE_SLIDER_FILL`, both rail halves carry the fill in every interaction status, the chrome thumb and rail geometry are unchanged, and the seek bar and equalizer preamp keep the neutral `LCD_BACKGROUND` groove. Name `style::level_slider_style` and `views::chrome_slider` as the enforcers and list the theme and style tests.

**Files touched.** `src/ui/theme.rs`, `src/ui/style.rs`, `src/ui/views/mod.rs`, `src/ui/views/transport.rs`, `src/ui/views/shade.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The transport volume slider draws both halves of its rail in `theme::VOLUME_SLIDER_FILL` (orange) and the balance slider in `theme::BALANCE_SLIDER_FILL` (green); the shade-mode volume slider uses the same orange fill.
- The chrome thumb, 4px rail width, 1px dark rail border, and square geometry are unchanged from `chrome_slider_style`, and hover/drag still change only the thumb face.
- The seek bar and the equalizer preamp keep the neutral `LCD_BACKGROUND` groove; no range, step, message, state, or playback behavior changes.
- `base_skin_colors_keep_their_documented_literal_values` and `level_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb` pin the exact fills and the style mapping, `make check` passes, and the README and invariants section describe the same behavior.

### Draw the LCD's MONO and STEREO indicators as a lit pair (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match `docs/screenshots/player.png` and `docs/screenshots/transport.png`, which draw the channel indicators as a pair of labels, `MONO` and `STEREO`, with the mode the loaded preview reports lit green and the other dimmed. Today `now_playing_status_row` (`src/ui/views/now_playing.rs`) draws one right-aligned green string ending in the active mode only (`256 KBPS  44 KHZ  STEREO`), and `format_playback_status` (`src/ui/views/now_playing/text.rs`) builds that whole string. Split the status row so the bitrate/sample-rate prefix keeps its place and the two channel labels draw beside it; when no format is reported, both labels stay dim and the prefix is absent. This is presentation only: the format value, its Pause/Stop clearing rules, and the status row's fixed height are unchanged.

**Approach.**

- `src/ui/theme.rs`: add `pub const LCD_GREEN_DIM: Color = Color::from_rgb(0.0, 0.35, 0.0);`, documented as the unlit LCD indicator color. Extend the existing palette/color tests with its exact value.
- `src/ui/views/now_playing/text.rs`: replace `format_playback_status` with two pure helpers. `pub fn format_playback_prefix(format: PlaybackFormat) -> String` returns `"{kbps} KBPS  {khz} KHZ"`, or `"{khz} KHZ"` when the bitrate is unknown, still truncating the rate to whole kilohertz. `pub fn channel_mode_label(channels: u16) -> &'static str` returns `"MONO"` for one channel and `"STEREO"` otherwise. Update the module docs to say the row composes a prefix and a label pair.
- `src/ui/views/now_playing.rs`: import the new helpers and re-export them from the module. Add `pub(super) const CHANNEL_LABELS: [&str; 2] = ["MONO", "STEREO"];` and `pub(super) fn channel_label_color(channels: Option<u16>, label: &str) -> Color`, returning `theme::LCD_GREEN` only when `channels` is `Some` and `label == channel_mode_label(channels)`, and `theme::LCD_GREEN_DIM` otherwise. In `now_playing_status_row`, keep the fill spacer and fixed `LCD_STATUS_HEIGHT`; when `format` is `Some`, push `format_playback_prefix(format)` in `theme::LCD_GREEN` then a small gap; then always push both labels at `LCD_STATUS_SIZE`, each colored by `channel_label_color(format.map(|format| format.channels), label)`. The row therefore draws the dim pair even with no format instead of nothing, matching the mockup, while the height stays fixed.
- Tests. `src/ui/views/tests.rs`: replace `format_playback_status_formats_khz_and_channel_mode` with `format_playback_prefix_formats_khz_and_bitrate` (256/44.1 kHz → `"256 KBPS  44 KHZ"`, unknown bitrate → `"44 KHZ"`, 48 kHz → `"48 KHZ"`) and `channel_mode_label_names_the_reported_channels` (1 → `MONO`, 2 → `STEREO`). Add `channel_label_color_lights_only_the_reported_mode` asserting stereo lights `STEREO` and dims `MONO`, mono the reverse, and `None` dims both, with exact `LCD_GREEN`/`LCD_GREEN_DIM` values. Update `captured_status_row`/`now_playing_status_row_draws_the_format_or_nothing` to `now_playing_status_row_lights_the_reported_channel_mode`: a stereo format draws the prefix plus both labels, while `None` draws both labels and the same height. Update `now_playing_content_keeps_the_visualizer_height_with_and_without_a_spectrum` to locate the `"256 KBPS  44 KHZ"` fragment and assert both labels also draw below the visualizer row.
- `README.md`: in the Usage sentence describing the LCD status row, say it always draws `MONO` and `STEREO` side by side, lights the reported mode green and dims the other, and shows the bitrate/sample-rate prefix only when the backend reports a format. Keep the existing bitrate-fallback wording for that prefix.
- `docs/invariants.md`: in the `## Now Playing label` status-row bullet, replace the single-mode/blank-row wording with the pair guarantee: both labels always draw, the reported mode is green and the other dim, both are dim with no format, the prefix appears only with a reported format, and Pause/Stop/new-selection and stale-reply rules are unchanged. Name `theme::LCD_GREEN_DIM`, `views::now_playing::CHANNEL_LABELS`/`channel_label_color`, and `text::format_playback_prefix`/`channel_mode_label` as the enforcers and add the new tests to the Pinned-by list.

**Files touched.** `src/ui/theme.rs`, `src/ui/views/now_playing/text.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The LCD status row always draws both `MONO` and `STEREO` labels in the mockup's order and keeps its fixed height; with a reported format it draws the bitrate/sample-rate prefix before them.
- The reported channel mode is `theme::LCD_GREEN` and the other label is `theme::LCD_GREEN_DIM`; with no reported format both labels are dim and no prefix is drawn.
- `format_playback_prefix` still truncates 44.1 kHz to `44`, formats 48 kHz as `48`, omits an unknown bitrate, and leaves the existing bitrate value unchanged.
- Pause keeps the reported format lit, while Stop and a new selection clear it so both labels go dim; no state field, poll, message, or audio behavior changes.
- `README.md` and `docs/invariants.md` describe the pair, `make check` passes, and the new tests cover the prefix, the label mapping, the exact lit/dim colors, and the row's fragments and height.

### Show each album row's running time (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match `docs/screenshots/library.png`'s Albums panel: each album row shows its total running time at the right (`12:38`, `2:58`), replacing the current `View Songs` hint. For the bundled sample library the time is the sum of the album's actual sample-song durations, so `First Record` reads `10:30`, `Second Record` `1:35`, and `Debut` `3:20`; the mockup's decorative numbers do not match the sample data and are not copied. An album with no known songs shows `--:--`, the same placeholder song rows use. The time is display data only: the row still emits `Message::AlbumSelected { epoch, index }`, and Sort, Back, and selection behavior are unchanged.

**Approach.** The fetch shape is settled here, without a per-album request or an unproven REST include: one whole-library songs read is grouped by exact `(album title, artist display name)` and summed. This mirrors the queued artist-count plan's use of one extra collection request, and leaves a song resource without `albumName` unmatched (its album shows `--:--`) instead of failing the whole list.

- `src/library.rs`: add `#[serde(skip)] pub duration_ms: u64` to `Album`, documented as the summed duration of the songs the library reports for the album and `0` when none. It is derived display data, not an Apple Music album field, so it deserializes to `0` and stays out of the serialized payload; existing album payload tests need no field change. Add `Album::duration_ms` to every `Album` literal (default `0` in fixtures).
- `src/sample_library.rs`: after building `songs`, build `songs_by_album` first and set each sample album's `duration_ms` by summing `songs_by_album[album.id]` with `saturating_add` before indexing `albums_by_artist`; pin the three totals in the sample-content tests.
- `src/apple_music/rest/wire.rs`: add `album_name` (`#[serde(rename = "albumName")]`) to the private `Attributes`, and add `Resource::into_album_duration(self) -> Option<(String, String, u64)>` returning the exact non-blank album name, the artist display name (empty when blank, matching `into_album`'s sentinel), and the duration (`0` when absent).
- `src/apple_music/rest.rs`: add `RestLibrary::get_album_durations(&self, session) -> Result<HashMap<(String, String), u64>, AppleMusicError>`, reading `/v1/me/library/songs` through the existing `fetch` pagination and summing durations per key with `saturating_add`. Document that it relies on the library song's `albumName` attribute and that a resource without one is skipped.
- `src/apple_music/browse.rs`: add a private `apply_album_durations(albums: &mut [Album], durations: &HashMap<(String, String), u64>)` that sets each album's `duration_ms` from its `(title, artist)` key and `0` on a miss. In the REST closures of `get_albums_by_artist` and `get_all_albums`, fetch the list, then `rest.get_album_durations(session)?`, then apply; a failed songs request fails the album load through the existing Retry path. The signed-out closures keep returning the sample library, whose albums now carry durations.
- `src/ui/views/browse/rows.rs`: make `album_row`'s secondary label `Cow::Owned(format_track_time(album.duration_ms))` instead of the `"View Songs"` constant, and update its doc comment. `playlist_row` keeps its hint.
- Tests. `src/ui/views/tests.rs`: update `album_row_uses_title_and_selects_the_album_by_index` and `album_row_shows_the_title_alone_when_the_artist_is_empty` to set a duration and expect its formatted time; add `album_row_shows_a_placeholder_when_the_duration_is_unknown` for `0` → `--:--`. `src/sample_library.rs`: pin `10:30`, `1:35`, and `3:20` from the derived durations. `src/apple_music/rest/tests.rs`: add a `get_album_durations` test whose stub song data carries `albumName`, `artistName`, and `durationInMillis`, asserting the summed `(title, artist)` map and the `/v1/me/library/songs` URL. `src/apple_music/tests.rs`: update `get_albums_by_artist_uses_the_rest_library_when_signed_in` and `get_all_albums_uses_the_rest_library_when_signed_in` to use `StubTransport::returning_bodies` (album list, then songs) and assert the assigned duration and both URLs; add a test that a failing songs request fails `get_all_albums`.
- `README.md`: in the Usage browse-list sentence, say each album row also shows its running time at the right (`10:30`), with `--:--` when the source supplied none.
- `docs/invariants.md`: add a `## Browse album running times` section after the browse-row sections, naming `Album::duration_ms`, `SampleLibrary::new`, `RestLibrary::get_album_durations`, `browse::apply_album_durations`, and `views::browse::rows::album_row` as the enforcers, and the new tests as pinned.

**Files touched.** `src/library.rs`, `src/sample_library.rs`, `src/apple_music/rest/wire.rs`, `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music/browse.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/views/browse/rows.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Every album row in the per-artist Albums view and the flat All Albums view shows its running time (or `--:--` for an unknown total) instead of `View Songs`; the row still emits the same `Message::AlbumSelected { epoch, index }`, and sort, Back, and selection are unchanged.
- The sample library reports `First Record` `10:30`, `Second Record` `1:35`, and `Debut` `3:20`, derived from the sample songs rather than hardcoded.
- The signed-in service returns albums whose `duration_ms` is the sum of the matching library songs' durations, fetching the album collection first and the whole-library songs collection second; an unmatched album is `0`, and a failed songs request surfaces as the existing album-load failure.
- `Album::duration_ms` is skipped by serde and defaults to `0`, so the model's existing wire payload tests stay unchanged.
- `README.md` and `docs/invariants.md` document the labels, `make check` passes, and the new tests cover the row formatting, the sample totals, the REST grouping, and the extra-request failure.

### Show each artist row's album count (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match `docs/screenshots/library.png`'s Artists panel: each artist row shows how many albums the library holds for that artist at the right (`2 albums`, `1 album`), replacing the current `View Albums` hint. An artist with no albums shows no count, as the mockup's third row does. The count is display data only: the row still selects the artist by index and epoch, Sort still orders artists by name, and no browse or playback behavior changes. The count is derived from albums the service already knows, so this adds no new Apple Music endpoint shape.

**Approach.**

- `src/library.rs`: add `pub album_count: usize` to `Artist`, documented as the number of albums the loaded library holds for the artist and `0` when none. Mark it `#[serde(skip)]` so it stays out of the wire payload (a derived value, not an Apple Music field) and deserializes to `0`. Add a pure `pub fn album_counts_by_artist(albums: &[Album]) -> HashMap<&str, usize>` that counts non-empty `Album::artist` display names, ignoring empty names.
- `src/sample_library.rs`: build `albums` first, then derive each sample artist's `album_count` from `albums` by `artist_id` (The Sample Band 2, Echo Chamber 1, Mono Tones 0) before storing the artist list. Add a test pinning those three counts.
- `src/apple_music/rest/wire.rs`: `Resource::into_artist` initializes `album_count: 0`; the artist resource carries no album count.
- `src/apple_music/browse.rs`: in the REST branch of `AppleMusicService::get_favorite_artists`, after `rest.get_favorite_artists(session)?`, also call `rest.get_all_albums(session)?`, run `album_counts_by_artist`, and assign each artist's count by matching `Artist::name` to the albums' `Album::artist` name. Keep the signed-out fallback at `sample_library().artists.clone()`, whose counts are now populated. Document that the signed-in load makes one extra paginated album request, that a failure in either request fails the artists load through the existing Retry path, and that exact display-name matching is the only link because `get_all_albums` deliberately leaves `artist_id` empty.
- `src/ui/views/browse/rows.rs`: add `pub(in crate::ui::views) fn album_count_label(count: usize) -> Cow<'static, str>` returning `""` for 0, `"1 album"` for 1, and `"N albums"` otherwise; use it as `artist_row`'s secondary label in place of the `"View Albums"` constant and update that row's doc comment.
- Tests. `src/ui/views/tests.rs`: update `artist_row_uses_name_and_selects_the_artist_by_index` to give its artist two albums and expect `"2 albums"`; add `album_count_label_singular_plural_and_none` covering 0, 1, and 2. `src/library.rs` tests: cover `album_counts_by_artist` (zero, one, many, and the empty-name skip) and that `album_count` is not serialized. `src/sample_library.rs`: pin the three sample counts. `src/apple_music/tests.rs`: add `get_favorite_artists_counts_albums_by_artist_name_from_the_rest_library`, using a two-body `StubTransport` (artists then albums) and asserting the resulting counts and both requested URLs; update `get_favorite_artists_uses_the_rest_library_when_signed_in` for the second request and its zero counts. Add `album_count: 0` to the `Artist` literals in `src/test_support.rs`, `src/ui/tests.rs`, and `src/apple_music/rest/tests.rs` unless the literal is about the count.
- `README.md`: in the Usage browse-list sentence, say each artist row shows its album count at the right (`2 albums`, `1 album`, and no count for an artist with none), while song rows keep their lengths and album rows keep `<artist> - <title>`.
- `docs/invariants.md`: add a `## Browse row labels` section after `## Browse row order` stating the artist-row count guarantee, naming `Artist::album_count`, `album_counts_by_artist`, `AppleMusicService::get_favorite_artists`, and `views::browse::rows::album_count_label` as the enforcers, and listing the new tests as pinned.

**Files touched.** `src/library.rs`, `src/sample_library.rs`, `src/apple_music/browse.rs`, `src/apple_music/rest/wire.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/views/browse/rows.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `src/apple_music/rest/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The Artists list renders `1 album`, `N albums`, or no count; the row still emits the same `Message::ArtistSelected { epoch, index }`, and Sort and Back behavior are unchanged.
- The signed-out service returns the sample artists with counts 2, 1, and 0; the signed-in service fetches the artist list first and the all-albums collection second, counts albums by exact `Album::artist` name, and surfaces a failed album request as the existing artists load failure (Retry).
- `Artist::album_count` is skipped by serde and defaults to `0` when absent, so the model's wire payload tests stay unchanged.
- `README.md` and `docs/invariants.md` document the labels, `make check` passes, and the new tests cover the labels, the sample counts, the REST count, and the serialization skip.

### Preview a seek drag's target time in the Now Playing readout (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** While the user drags the seek slider beneath the Now Playing LCD, show the time the thumb points at in the large left-hand readout, so seeking a long preview is not blind. Today `WinampPlayer::now_playing_elapsed` in `src/ui/player.rs` reads only `playback_position`, while the slider thumb reads `seek_preview.or(playback_position)` inline in `src/ui/view.rs`; a drag therefore moves the thumb but leaves the readout on the backend's last position. Releasing already seeks to the drag and records that position, so the readout lands where the thumb pointed with no playback change. It touches only the Now Playing readout and the seek bar's shared position source.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) fn readout_position(&self) -> Option<Duration>` returning `self.seek_preview.or(self.playback_position)`. Document it as the one position the seek thumb and the large readout both show: an in-progress drag wins, otherwise the backend-reported position. Use it in `WinampPlayer::now_playing_elapsed` in place of `self.playback_position`, so elapsed mode previews the drag and remaining mode counts down from it through the existing `views::now_playing_elapsed` paths. Extend the `seek_preview` field's doc to say the held drag also previews in the readout.
- `src/ui/view.rs`: pass `player.readout_position()` to `views::view_seek_bar` instead of the inline `player.seek_preview.or(player.playback_position)`, so the thumb and readout cannot drift apart.
- Leave the right-edge total, marquee, paused blink, position/format/duration/spectrum tick batches, and the seek-on-release path unchanged. `reset_position_readout` still clears `seek_preview` on Stop and a new selection, so the readout returns to the cleared state then.
- `src/ui/tests.rs`: add `now_playing_elapsed_previews_the_seek_drag` beside `now_playing_elapsed_shows_the_position_while_it_is_known`. Build a player with `play_song_1_then_browse_to_album_2()` (song 1 is 3:30), set `playback_position = Some(Duration::from_secs(45))`, then send `Message::SeekPreviewChanged(12.0)`; assert `player.readout_position()` is `Some(Duration::from_secs(12))`, the elapsed readout is `"00:12"`, and `player.now_playing_time(...)` is still `"3:30"`. Set `player.time_display = views::TimeDisplay::Remaining` and assert `"-03:18"`. Set `player.seek_preview = None` and assert the readout falls back to `"00:45"` in elapsed mode (and `"-02:45"` in remaining mode), proving the drag is a preview, not a replacement.
- `README.md`: in the Usage sentence about the seek slider, replace "dragging moves only the thumb, and releasing seeks to that position" with wording that dragging moves the thumb and previews that position in the large elapsed readout (counting down from it in remaining-time mode), and releasing seeks to it. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the `## Now Playing label` seek-slider bullet, add that the large readout follows the in-progress drag in both time modes while the right-edge total does not, and that the thumb and readout share `WinampPlayer::readout_position`; add the new test to that bullet's Pinned-by list.

**Files touched.** `src/ui/player.rs`, `src/ui/view.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- While `seek_preview` holds a drag, the large readout shows that time in elapsed mode and counts down from it in remaining mode; the right-edge total is unchanged.
- The seek slider's thumb and the large readout both resolve their position through `WinampPlayer::readout_position`, so an in-progress drag wins over `playback_position` and clearing the drag falls back to the reported position.
- A release still sends exactly one `seek_output_to` with the dragged position and records it; Stop and a new selection still clear the drag and readout through `reset_position_readout`. No marquee, blink, tick, format, duration, spectrum, or playback behavior changes.
- `README.md` and `docs/invariants.md` describe the preview, `make check` passes, and the new test covers the drag, the remaining-mode countdown, and the fallback.

### Match the equalizer panel's preamp column and dB scale to the mockup (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match `docs/screenshots/hero.png` and `docs/screenshots/player.png`, which draw the equalizer's preamp as a vertical slider in the same row as the ten band sliders, with a `+12 DB` / `0 DB` / `-12 DB` scale beside it. The build (`view_equalizer` in `src/ui/views/equalizer.rs`) instead lays the preamp out as a horizontal 150 px slider in its own row above the response curve, and draws no gain scale, so the panel's slider positions and green/yellow/red band tint cannot be read as decibels. Move the preamp into the band row as a `VerticalSlider` and add the three scale labels; no gain, message, preset, curve shape, tint, or audio behavior changes.

**Approach.**

- `src/ui/views/equalizer.rs`: add `pub(super) const EQ_SLIDER_HEIGHT: f32 = 100.0;` and use it for the band sliders' `.height(...)` (replacing the literal `100.0`), the new preamp slider, and the scale column, so the three stay aligned.
- Add `pub(super) fn db_scale_column() -> Column<'static, Message>`: a width-shrink `Column` at `Length::Fixed(EQ_SLIDER_HEIGHT)` holding `Text::new("+12 DB").size(12)`, a `Space` of `Length::Fill`, `Text::new("0 DB").size(12)`, another `Length::Fill` `Space`, and `Text::new("-12 DB").size(12)`. The two equal fill spaces center the middle label on the rail, so the endpoints read at the rail's top and bottom.
- Add `pub(super) fn preamp_column(preamp: f32) -> Column<'static, Message>`: a `VerticalSlider::new(GAIN_MIN_DB..=GAIN_MAX_DB, preamp, Message::EqPreampChange).step(EQ_STEP).height(Length::Fixed(EQ_SLIDER_HEIGHT)).style(|_theme, status| style::chrome_slider_style(status))` over `Text::new("Preamp").size(12)`. Keep `chrome_slider_style`, not `band_slider_style`, so only the ten band grooves carry the gain tint.
- In `view_equalizer`, delete the `Row::new().push(Text::new("Preamp")).push(chrome_slider(...))` row. Build the preamp and scale as one spaced row, and the ten band columns as another. Put both inside a `Row` whose left column is a curve-height `Space` over the preamp/scale row and whose right column is `response_curve_row(bands)` over the band row, so the preamp and scale stay in the band row while the curve's column 0 starts on the first band instead of over the preamp. Leave the header row (on/off button and preset `PickList`) unchanged.
- Update the module and `view_equalizer` docs to describe the new order: on/off button and preset pick list, response curve, then one row of vertical preamp, dB scale, and ten band sliders. In `src/ui/views/mod.rs`, update `chrome_slider`'s doc to drop the equalizer preamp from its horizontal callers and name the vertical preamp beside the band sliders as the `VerticalSlider` twin.
- `src/ui/views/tests.rs`: import `EQ_SLIDER_HEIGHT` alongside the existing equalizer items and add `equalizer_preamp_and_db_scale_sit_left_of_the_band_sliders`. Lay out `view_equalizer(false, 0.0, &[0.0; BAND_COUNT], None)` with `layout_capture`, then assert all three labels are drawn; assert `capture.find("+12 DB").y < capture.find("0 DB").y < capture.find("-12 DB").y` (the scale reads top to bottom); assert the `Preamp` label's `y` is within 0.5 px of the first `BAND_FREQUENCIES` label's `y` (both sit beneath a slider in the same row, so the preamp is vertical there rather than in its old separate row); and assert every scale label's right edge is at or left of the first frequency label's left edge (the scale sits before the bands). It also captures the curve's containers and asserts the first segment's x matches the first band label's x, so the curve starts on the band area rather than over the preamp. Leave `equalizer_band_labels_leave_a_gap_between_frequencies` and `equalizer_panel_constructs_for_both_states_and_gain_endpoints` unchanged.
- `README.md`: in the Usage sentence listing the equalizer panel, replace "a preamp slider, a response curve above ten band sliders" with "a response curve above a vertical preamp slider, a `+12 dB` / `0 dB` / `-12 dB` scale, and ten band sliders". Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Equalizer band tint`, change "The preamp slider keeps its uniform gray groove" to say the vertical preamp slider keeps its uniform gray groove. In `## UI layout`, add a bullet that the equalizer band row puts the vertical preamp leftmost, the three dB labels next, then the ten band sliders, with `+12 DB` above `0 DB` above `-12 DB` beside the rail's top, middle, and bottom; name `views::equalizer::preamp_column`, `views::equalizer::db_scale_column`, and `views::view_equalizer` as the enforcers and the new layout test as pinned by.

**Files touched.** `src/ui/views/equalizer.rs`, `src/ui/views/mod.rs` (doc comment only), `src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The equalizer panel draws the preamp as a vertical slider in the same row as the band sliders, left of the `+12 DB` / `0 DB` / `-12 DB` scale, which is left of the ten band sliders; the response curve starts at the first band, and the on/off button and preset pick list keep their positions and behavior.
- The three scale labels are drawn top to bottom as `+12 DB`, `0 DB`, `-12 DB`, the `Preamp` and first band-frequency labels share one text-baseline row, proving the preamp moved into the band row, and the response curve's first segment starts at the first band's x.
- Dragging the preamp still emits `Message::EqPreampChange`, still spans `GAIN_MIN_DB..=GAIN_MAX_DB` in `EQ_STEP` increments, and its groove stays the uniform gray `chrome_slider_style`; the band sliders keep their gain tint and `band_slider_style`.
- No gain, preset, curve, audio, or state behavior changes: `eq_preamp_change_clamps_value_before_storing`, `equalizer_messages_forward_to_the_audio_backend`, `response_curve_offsets_follow_the_band_gains`, and the band-tint tests pass unchanged.
- `README.md` and `docs/invariants.md` describe the new preamp column and dB scale, and `make check` passes.

### Clear the stopped Now Playing LCD readout (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match `docs/screenshots/transport.png`'s stopped state: "Time, bitrate and spectrum clear and the seek thumb hides; the current song stays queued for Play." The status row, spectrum, and seek strip already clear through `reset_position_readout`, but the large elapsed readout still draws `00:00` in green because `elapsed_readout_color` returns `theme::LCD_GREEN` for `TransportMode::Stopped`. Draw the stopped readout transparent, keeping its text and bounds so the title and total do not move, and make it non-interactive while stopped so a click cannot silently switch the persisted time mode. This is independent of the queued Preview end plans: it reads the `TransportMode` that already exists and changes only the Now Playing view.

**Approach.**

- `src/ui/views/now_playing.rs`: extend `elapsed_readout_color` so `TransportMode::Stopped` is fully transparent in both blink phases whenever the text is not the `--:--` placeholder, as well as in `TransportMode::Paused` with `visible == false`; `TransportMode::Playing`, and `TransportMode::Paused` with `visible == true`, stay `theme::LCD_GREEN`. Update its doc comment to say the stopped readout is cleared rather than shown, that the `--:--` placeholder stays green because it reports no time to clear, and that drawing transparent keeps its bounds so the title, total, and status row stay in place.
- In the same module, add `pub(super) fn elapsed_readout_clickable(mode: TransportMode) -> bool`, returning false only for `TransportMode::Stopped` and true for `Playing`/`Paused`. In `now_playing_row`, build the elapsed `MouseArea` from the existing `lcd_elapsed_text(elapsed).color(elapsed_readout_color(...))` but attach `.on_press(Message::ToggleTimeDisplay).interaction(Interaction::Pointer)` only when `elapsed_readout_clickable(mode)` is true; otherwise use `.interaction(Interaction::None)` with no message. Keep the widget in the row so its layout slot and the text's bounds are unchanged.
- `src/ui/views/tests.rs`: extend and rename `elapsed_readout_color_blinks_only_while_paused` to `elapsed_readout_color_blinks_when_paused_and_clears_when_stopped`, asserting transparent for `(Paused, false)` and for both `(Stopped, false)` and `(Stopped, true)` with a real time, and `theme::LCD_GREEN` for `(Playing, false)`, `(Playing, true)`, and `(Paused, true)`. Add `elapsed_readout_color_keeps_the_placeholder_visible_while_stopped` asserting `--:--` stays `theme::LCD_GREEN` while stopped, `elapsed_readout_is_clickable_only_while_not_stopped` asserting true for Playing/Paused and false for Stopped, and `elapsed_readout_toggles_only_while_not_stopped`, which drives a real left press at the elapsed text's bounds and asserts that stopped publishes no `ToggleTimeDisplay` while playing and paused do. Update the comment on `now_playing_row_keeps_the_elapsed_readout_in_place_across_transport_modes` to say the stopped case still lays out the transparent text; its existing equal-x assertion then pins that stopping does not move the readout.
- `src/ui/tests.rs`: correct the comments and assertion messages in `position_tick_blinks_the_elapsed_readout_only_while_paused` that currently claim a stopped tick "shows the readout": the tick still holds `blink_visible` true while stopped, but the view hides the stopped readout regardless of that flag. Do not change the assertions.
- `README.md`: in the Usage paragraph, say the large readout is shown while playing or paused and blank (though it keeps its space) while stopped, and change "it stays steady while playing or stopped" to "it stays steady while playing and clears while stopped". Also scope the click sentence to while the readout is shown. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, update the time-readout bullet to state that while the transport is stopped the large readout draws transparent in both time modes and both blink phases whenever its text is not the `--:--` placeholder, keeping its bounds and clickability off, so stopped shows no elapsed or remaining time other than the placeholder; the stored value is still the `00:00` Stop clears it to, and Play draws it again. Update the transport-glyph bullet's "playing and stopped always show the readout" sentence to playing only, name `views::now_playing::elapsed_readout_color` and `elapsed_readout_clickable` as the enforcers, and add the new test names. Update the position-poll bullet's reason for leaving a stopped player out of the subscription to say the view clears its readout while stopped rather than that it already shows `00:00`.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `src/ui/player.rs`, `src/ui/timers.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `elapsed_readout_color` returns the fully transparent color for `TransportMode::Stopped` in both blink phases when the text is not the `--:--` placeholder, and for `TransportMode::Paused` only when `visible` is false; playing, visible paused, and the stopped placeholder stay `theme::LCD_GREEN`.
- While stopped, `now_playing_row` still draws the elapsed text at the same bounds as in the playing and paused states, so the title, total, and status row do not move; the captured stopped row keeps the same elapsed fragment as the other modes.
- `elapsed_readout_clickable` is false only for `Stopped`, and the stopped readout attaches no `ToggleTimeDisplay` message, so clicking it cannot change the persisted `TimeDisplay`; playing and paused readouts still toggle it.
- Stop, a new selection, Pause, the position poll, the marquee, the status row, the spectrum, and the seek strip keep their current behavior; no state field, timer, subscription, or dependency is added, and the shade strip is deliberately unchanged.
- `README.md` and `docs/invariants.md` describe the stopped readout, and `make check` passes.

### Handle the end of a preview (Preview end, part 2/2: planned 2026-10-23 by the plan role, requires part 1/2 landed) — done 2026-10-23 by the feature role

**Goal.** Make the end of a preview follow the transport's own rules instead of leaving the UI claiming playback against a silent sink. When a loaded preview finishes while playing, behave exactly like Next: step forward through the showing list with Shuffle choosing a non-current song and Repeat wrapping at the end. With Repeat off on the last song, take the existing Stop path so playback becomes stopped, the current song stays queued for Play, and the elapsed readout, status row, visualizer, and seek bar clear together. Pause and Stop never advance. Reuse the one-second position poll; add no timer or subscription.

**Approach.**

- `src/ui/message.rs`: add `PlaybackFinished { epoch: u64, finished: bool }`, documented as the position poll's completion reply stamped with the epoch that issued the query.
- `src/ui/handlers/playback.rs`: in `position_tick`, schedule a fourth task from `service.preview_finished()` with the same epoch, mapping to `Message::PlaybackFinished`; update the tick's doc to name the finished query. Leave the early return while `pending_position_transitions > 0` in place, so a transition in flight suppresses the query too.
- In the same module, extract the forward half of `step_track`'s selection into a private pure helper, for example `fn forward_track_id(items: &[Song], current: Option<&str>, shuffle: bool, repeat: bool) -> Option<String>`: it returns `transport::shuffled_track_id(items, current, transport::shuffle_roll())` when `shuffle` is on and `transport::next_track_id(items, current, repeat)` otherwise. `step_track` calls it under its existing state lock for `forward` (using the same list the current view draws from: `all_songs` for `CurrentView::AllSongs`, `songs` otherwise) and keeps its existing backward branch and `TrackSelected` index resolution, so the Next button and keyboard behavior do not change. `playback_finished` reads the same list and flags under one lock, calls the helper, and resolves the returned id to its index before releasing the lock; taking arguments rather than a player keeps it from trying to lock recursively.
- Add `pub(in crate::ui) fn playback_finished(player: &mut WinampPlayer, epoch: u64, finished: bool) -> Task<Message>`. It returns `Task::none()` unless `finished` is true, no transition is pending, the epoch is current, and `state.is_playing` is true. Otherwise it calls `forward_track_id`; on `Some(id)` it resolves the id to its position in that same list and schedules `Message::TrackSelected { epoch: list_epoch, index }`; on `None`, when the id no longer appears in the list, or when the pick is still the current song (`transport::next_track_id`'s Repeat-off edge behavior, and a one-song shuffled list), it calls [`stop`] and returns its task, so the Stop path clears the readout and keeps `current_track`. Do not record the ended track in `PlayHistory`; a natural end is a forward step, not a Previous.
- `src/ui/mod.rs`: route `Message::PlaybackFinished` to `playback_finished`.
- `src/ui/timers.rs`: update the position-poll module docs that enumerate the queries (position, format, length) to include the finished state; the interval stays `POSITION_POLL_INTERVAL`.
- `src/ui/tests.rs`: add `position_tick_queries_preview_finished` (the reply carries the tick's epoch and `RecordingAudio::set_finished`'s value), `preview_end_advances_to_the_next_song` (seed `stepping_songs` with `song-1` current, drive the finished reply and then its task, assert the next song's play commits), `preview_end_wraps_to_the_first_song_with_repeat_on`, `preview_end_at_the_last_song_stops_when_repeat_is_off` (assert `is_playing` false, `current_track` still the last song, the `AudioCall::Stop` recorded, and the position/format/duration/spectrum cleared), `preview_end_with_shuffle_on_picks_the_other_song` (a two-song list makes the pick deterministic), `preview_end_while_paused_or_stopped_does_not_advance`, and `preview_end_with_a_stale_epoch_or_pending_transition_is_dropped`. Drive `playback_finished` through `update` and settle the scheduled play or stop task with the existing helpers.
- `README.md`: in the Usage paragraph describing preview playback, add one sentence: when a preview reaches its end, the player advances to the next song of the showing list (Shuffle picks another, Repeat wraps), and with Repeat off on the last song it stops and keeps that song queued for Play.
- `docs/invariants.md`: extend `## Preview end` with the UI behavior: the position poll reports the finished state, a finished preview while playing advances under the same list/Shuffle/Repeat rules as Next, Repeat off on the last song takes the Stop path, and Pause, Stop, a stale epoch, or a pending transition never advance. Name `handlers::playback::playback_finished`, `handlers::playback::forward_track_id`/`step_track`, and `Message::PlaybackFinished` as the enforcers and list the new tests.

**Files touched.** `src/ui/message.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/timers.rs`, `src/ui/tests.rs`, `src/apple_music/playback.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A finished preview while playing advances to the song Next would select from the showing list: sequential when Shuffle and Repeat are off, the first song when Repeat is on and the last is current, and a non-current song when Shuffle is on.
- A finished preview on the last song with Repeat off takes the Stop path: `is_playing` becomes false, `current_track` still names that song, and the elapsed readout, status row, visualizer bars, and seek bar are cleared.
- A finished reply is ignored while paused or stopped, when `pending_position_transitions > 0`, or when its epoch is stale, so a reply in flight across a transport change cannot advance.
- The finished state is queried on the existing one-second position poll; no new timer, subscription, or dependency is added, and the Next/Previous buttons and keys keep their current behavior.
- `README.md` and `docs/invariants.md` describe the behavior, and `make check` passes.

### Handle the end of a preview (Preview end, part 1/2: planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** A loaded Apple Music preview has no completion signal today: once its ~30-second asset drains, the backend has nothing left to play, but `AudioOutput` cannot say so, so the UI keeps `is_playing` true and polls a silent sink forever. Add a narrow query, `AudioOutput::is_finished`, that reports whether the loaded preview has played to its end, and forward it as `AppleMusicService::preview_finished`. This is the backend half of the Preview end series and changes no UI behavior; part 2/2 consumes the signal to advance the transport.

**Approach.**

- `src/audio.rs`: add `fn is_finished(&self) -> bool { false }` to `AudioOutput`, documented like `duration`/`position`: true only while a preview is loaded and its source has played to the end; false when no player is loaded (including after Stop), while a paused preview is still mid-asset, for the silent fallback, and when the worker cannot answer within its bound. The default keeps every existing implementor compiling without a finished state of its own.
- `src/audio/player.rs`: add `fn is_finished(&self) -> bool { false }` to `WorkerPlayer`. Override it for `RodioPlayer` as `self.format.get().is_some() && self.player.empty()`: rodio 0.22.2's `Player::empty` becomes true once the queued source has drained, and the format gate keeps a player that has never had a preview from reporting finished. Add `Command::Finished(Sender<Option<bool>>)` beside `Duration`/`PlaybackFormat`/`Spectrum`, handle it in `serve_commands` with `reply.send(player.as_ref().map(WorkerPlayer::is_finished))`, and add `RodioOutput::is_finished` through the existing `query` helper, mapping a missing reply to `false`. Extend the `WORKER_QUERY_TIMEOUT` doc (which names the queued backend queries) and the `AudioOutput` trait doc to include the finished query.
- `src/test_support.rs`: give `RecordingAudio` a `finished: AtomicBool` (default false), a `pub(crate) fn set_finished(&self, finished: bool)`, and an `is_finished` override returning it, so part 2 and the service test can drive both values.
- `src/apple_music/playback.rs`: add `pub(crate) fn preview_finished(&self) -> bool` beside `playback_spectrum`, forwarding to `self.audio.is_finished()` and documenting that it is observational and touches no shared state.
- `src/audio/tests.rs`: give `RecordingPlayer` a settable `finished` flag (an `Arc<AtomicBool>` field with a default like `spectrum`) and an `is_finished` override. Add `finished_query_returns_the_loaded_players_finished_state` (Play then a `Command::Finished` query answers `Some(true)` when the installed player's flag is set and `Some(false)` when it is not), `finished_query_reports_none_when_nothing_is_loaded` (no Play, query answers `None` and creates no player), `silent_output_reports_an_unfinished_preview` (`SilentOutput::is_finished()` is false), and `rodio_output_asks_the_worker_for_its_finished_state` (a worker thread answering `Command::Finished` with `Some(true)` makes `RodioOutput::is_finished()` true). Extend `rodio_output_reports_a_closed_worker_for_every_command` with `!output.is_finished()`.
- `src/apple_music/tests.rs`: add `preview_finished_forwards_the_audio_output_state` over `service_with_audio` and a `RecordingAudio`: false by default, true after `set_finished(true)`, false again after `set_finished(false)`, with no `AudioCall` recorded.
- `docs/invariants.md`: add a `## Preview end` section after `## Preview spectrum`: the backend reports whether the loaded preview has played to its end, only while a preview is loaded; a paused but unended preview, a stopped player, the silent fallback, a closed worker, and an unanswered query all report not finished; reading it changes no output. Name `audio::AudioOutput::is_finished`, `audio::player::RodioPlayer::is_finished`, and `AppleMusicService::preview_finished` as the enforcers and list the new tests. Say that part 2/2 is the UI consumer.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/test_support.rs`, `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `docs/invariants.md`.

**Acceptance criteria.**

- `AudioOutput::is_finished` defaults to false; `SilentOutput`, a closed worker, and an unanswered query all report false.
- The worker answers `Some(<player answer>)` for a loaded player and `None` when no player is installed; `RodioPlayer::is_finished` is false before a preview is appended and follows `rodio::Player::empty` once one is.
- `AppleMusicService::preview_finished` returns exactly what the injected backend's `is_finished` returns, and calling it records no audio command.
- No UI code reads the signal yet, so playback, the Now Playing readout, and the browse views are unchanged.
- `docs/invariants.md` records the guarantee, and `make check` passes with no new dependency.

### Show the transport state on the Now Playing LCD (Transport glyph, part 2/2: planned 2026-10-23 by the plan role, requires part 1/2 landed) — done 2026-10-23 by the feature role

**Goal.** Finish the paused state from `docs/screenshots/transport.png`: while the player is paused, the large elapsed readout blinks instead of sitting steady, while the position it shows stays frozen. Reuse the existing one-second position poll for the blink phase rather than adding a timer, and hide the readout by drawing it transparent rather than blanking it, so the row keeps its bounds and the title does not reflow.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) blink_visible: bool` to `WinampPlayer`, initialized `true`, documented as the paused readout's blink phase: `position_tick` toggles it while paused and holds it true whenever the transport is not paused. The view uses it only in the paused mode.
- `src/ui/handlers/playback.rs`: in `position_tick`, after the `pending_position_transitions` early return and before scheduling the queries, read `player.state.blocking_lock().transport_mode()` into a local and set `player.blink_visible = if mode == TransportMode::Paused { !player.blink_visible } else { true };` so the paused readout alternates once per position tick and a resumed or stopped player is always visible.
- `src/ui/views/now_playing.rs`: add `pub(super) fn elapsed_readout_color(mode: TransportMode, visible: bool) -> Color`, returning `Color::from_rgba(0.0, 0.0, 0.0, 0.0)` for `Paused` with `visible == false` and `theme::LCD_GREEN` otherwise. Add a trailing `elapsed_visible: bool` parameter to `now_playing_row`, `now_playing_content`, and `view_now_playing`; build the elapsed text with `lcd_elapsed_text(elapsed).color(elapsed_readout_color(mode, elapsed_visible))` so the text still lays out at the same width while transparent.
- `src/ui/view.rs`: pass `player.blink_visible` into `views::view_now_playing`.
- `src/ui/tests.rs`: add `position_tick_blinks_the_elapsed_readout_only_while_paused`, building a player with a loaded preview, setting `paused` through the state, and driving `position_tick`: the first tick hides the readout, the second shows it again, and a tick after `play` (or `stop`) leaves it visible.
- `src/ui/views/tests.rs`: add `elapsed_readout_color_blinks_only_while_paused` (`Paused`/hidden is transparent; `Paused`/visible, `Playing`, and `Stopped` in either phase are `LCD_GREEN`) and `now_playing_row_keeps_the_elapsed_bounds_when_the_blink_hides_it` (the paused visible and hidden captures draw the same elapsed string with the same bounds). Update the existing row/content/view calls for the new argument.
- `README.md`: extend the transport-glyph sentence with the paused blink. `docs/invariants.md`: extend the `## Now Playing label` glyph bullet with the blink guarantee and name `position_tick`'s phase flag and `elapsed_readout_color` as the enforcers and the new tests as the pins.

**Files touched.** `src/ui/player.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `src/ui/view.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- While `transport_mode()` is `Paused`, each `position_tick` flips `blink_visible`; while it is `Playing` or `Stopped`, a tick sets `blink_visible` to true, so playing and stopped always show the elapsed readout.
- `elapsed_readout_color(TransportMode::Paused, false)` is fully transparent, and every other mode/phase returns `theme::LCD_GREEN`.
- The elapsed readout keeps the same text and bounds in the paused visible and hidden states, so blinking does not move the title, total, or status row.
- The position poll keeps running while paused as it does today, and the blink adds no new subscription or timer.
- `README.md` and `docs/invariants.md` describe the paused blink, and `make check` passes.

### Show the transport state on the Now Playing LCD (Transport glyph, part 1/2: planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match `docs/screenshots/transport.png`'s three LCD states: a green `▶` beside the elapsed readout while playing, a green `II` while paused, and a red `■` while stopped. The app draws no such glyph today, and `AppState` cannot tell Pause from Stop — both leave `is_playing == false` — so the view has no way to choose between `II` and `■`. Add that distinction to shared state and draw the glyph in a fixed-width slot before the elapsed readout. The paused blink is deliberately left to part 2/2.

**Approach.**

- `src/state.rs`: add `pub paused: bool` beside `is_playing`, documented as true only while the loaded preview is paused and false while playing or stopped; its `Default` value is `false`. Set it in the existing mutators: `toggle_playing` flips `is_playing` and then sets `paused` to the new `!is_playing` (so the pause half pauses and the resume half resumes), `stop` and `play` clear it, and `pause` sets it. Update the `Default` doc's starting-state sentence. Add a public `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum TransportMode { Playing, Paused, Stopped }` and `pub fn transport_mode(&self) -> TransportMode`, returning `Playing` when `is_playing`, `Paused` when `paused`, and `Stopped` otherwise.
- `src/apple_music/playback.rs`: in `play_track`, replace the direct `state.is_playing = true` commit with `state.play()`, and the direct `state.is_playing = false` backend-failure rollback with `state.stop()`; both keep `current_track` as the surrounding code expects, and routing through the setters keeps `paused` from going stale. Update the `pause`/`resume`/`stop` doc comments to name the paused flag.
- `src/ui/theme.rs`: add `pub const LCD_RED: Color = Color::from_rgb(1.0, 0.1, 0.1);` beside `LCD_GREEN`, documented as the stopped-transport glyph color.
- `src/ui/views/now_playing.rs`: import `crate::state::TransportMode` and add `LCD_TRANSPORT_SIZE: f32 = 22.0` and `LCD_TRANSPORT_WIDTH: f32 = 16.0`. Add pure `pub(super) fn transport_glyph(mode: TransportMode) -> &'static str` returning `▶`, `II`, and `■`, and `pub(super) fn transport_glyph_color(mode: TransportMode) -> Color` returning `theme::LCD_GREEN` for `Playing`/`Paused` and `theme::LCD_RED` for `Stopped`. In `now_playing_row`, prepend a `Container` of fixed `LCD_TRANSPORT_WIDTH` holding the glyph as a `Text` at `LCD_TRANSPORT_SIZE` in that color, before the elapsed `MouseArea`. The fixed slot keeps the elapsed readout at one x position as the glyph changes. Add a trailing `mode: TransportMode` parameter to `now_playing_row`, `now_playing_content`, and `view_now_playing`, threading it through unchanged.
- `src/ui/view.rs`: resolve `state.transport_mode()` in the normal-mode state destructure and pass it to `views::view_now_playing`. Shade mode needs no change: its Play/Pause glyph keys off `is_playing`, so paused already shows `▶`, which is correct for a control that resumes.
- `src/ui/views/tests.rs`: update every `now_playing_row`/`now_playing_content`/`view_now_playing` call for the new argument. Change `captured_now_playing_texts` to take a `TransportMode` and assert the glyph is the first drawn fragment, the elapsed readout the second. Add `transport_glyph_names_and_colors_each_mode` (exact strings and exact `LCD_GREEN`/`LCD_RED` colors), `now_playing_row_keeps_the_elapsed_readout_in_place_across_transport_modes` (the elapsed fragment's x is equal in all three modes while each glyph draws), and extend `now_playing_content_draws_the_indicator_strip_beside_the_title` to assert the glyph follows the `O A I D V` strip.
- `src/state.rs` tests: extend `default_state_is_stopped_at_half_volume` with `!state.paused` and `TransportMode::Stopped`, rewrite `toggle_playing_flips_only_the_playback_flag` as `toggle_playing_switches_between_playing_and_paused` (playing → paused → playing), and add `pause_sets_the_paused_mode_while_play_and_stop_clear_it` driving `pause`, `play`, and `stop` and checking `transport_mode()`.
- `src/apple_music/tests.rs`: assert the mode in `pause_stops_playing_but_keeps_current_track` (Paused), `resume_resumes_the_audio_backend` and `resume_after_stop_restarts_the_preview` (Playing), `stop_stops_the_audio_backend` (Stopped), and `toggle_play_pause_pauses_while_playing` (Paused).
- `README.md`: in the Usage paragraph that describes the large elapsed readout, add one sentence naming the green `▶`, green `II`, and red `■` states. Do not touch the equalizer sentence the queued tint plan edits.
- `docs/invariants.md`: in `## Now Playing label`, add a bullet stating the glyph rule, that Pause sets the paused mode while Play and Stop clear it, and that the glyph's fixed-width slot leaves the elapsed readout in place; name `state::TransportMode`/`AppState::transport_mode`, `views::now_playing::transport_glyph`/`transport_glyph_color`, and `now_playing_row` as the enforcers and the new tests as the pins.

**Files touched.** `src/state.rs`, `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `src/ui/theme.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/view.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `AppState::pause` leaves `is_playing == false` and sets `paused == true`; `play` and `stop` clear both; `toggle_playing` alternates `Playing` and `Paused`; `transport_mode()` reports `Stopped` for the default state.
- `transport_glyph` returns exactly `▶` for `Playing`, `II` for `Paused`, and `■` for `Stopped`; `transport_glyph_color` returns `theme::LCD_GREEN` for the first two and `theme::LCD_RED` for `Stopped`.
- The LCD row draws the glyph before the elapsed readout in all three modes, and the elapsed readout keeps the same x across the three glyphs because the slot is fixed-width.
- Pause and Stop remain distinguishable after a real Play/Pause/Stop sequence through `AppleMusicService`: Pause reports `Paused` and keeps the current track, while Stop reports `Stopped` and keeps it too.
- `README.md` and `docs/invariants.md` describe the glyph, and `make check` passes with no new dependency.

### Tint each equalizer band slider by its gain, green through yellow to red (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Close the remaining visible gap in `docs/screenshots/equalizer.png`'s panel: the mockup paints each band slider's whole groove according to that band's stored gain (green at a cut, yellow at `0 dB`, red at a boost), while `view_equalizer` (`src/ui/views/equalizer.rs`) dresses every band in the same gray `chrome_slider_style`. Paint the existing sliders from their gain, with no custom widget, image asset, or dependency.

**Approach.**

- `src/ui/theme.rs`: add three public gain-stop colors beside the existing palette constants: `EQ_GAIN_LOW` (equal to `LCD_GREEN`), `EQ_GAIN_MID` (a yellow, e.g. `Color::from_rgb(0.85, 0.85, 0.0)`), and `EQ_GAIN_HIGH` (a red-orange, e.g. `Color::from_rgb(1.0, 0.2, 0.1)`), each with a doc comment naming the gain it stands for.
- `src/ui/style.rs`: add `pub fn band_gain_color(gain: f32) -> Color`. Run `gain` through `crate::equalizer::clamp_gain` first, so an out-of-range or NaN value cannot leave the range (NaN maps to the flat `0.0`, matching `clamp_gain`'s own fallback), then interpolate the RGB components linearly between `EQ_GAIN_LOW` and `EQ_GAIN_MID` across `GAIN_MIN_DB..=0.0` and between `EQ_GAIN_MID` and `EQ_GAIN_HIGH` across `0.0..=GAIN_MAX_DB`, keeping alpha `1.0`. The three named gains must return the stop colors exactly so the panel's flat preset reads yellow and its extremes read green/red.
- `src/ui/style.rs`: add `pub fn band_slider_style(gain: f32, status: slider::Status) -> slider::Style`. Start from `chrome_slider_style(status)` and replace both halves of `rail.backgrounds` with `Background::Color(band_gain_color(gain))`, leaving `rail.width`, `rail.border`, and the whole `handle` untouched. Both halves matter: iced's slider draws `backgrounds.0` from the range start to the thumb and `backgrounds.1` from the thumb to the range end, so painting only one half would leave a gray segment whose extent moves with the gain. The pure function keeps the color rule testable without building a widget, as the neighboring chrome styles are.
- `src/ui/views/equalizer.rs`: in `view_equalizer`'s band loop, copy `let gain = bands[index];` before constructing the `VerticalSlider` and change its style to `.style(move |_theme, status| style::band_slider_style(gain, status))`. Capture the gain by value rather than the index so the closure owns only a `f32`. Leave the preamp's `chrome_slider` call as it is: the mockup leaves the preamp groove uniform.
- `src/ui/style.rs` test module: import `crate::equalizer::{GAIN_MAX_DB, GAIN_MIN_DB}` and add `band_gain_color_runs_green_through_yellow_to_red` (the three named gains return `EQ_GAIN_LOW`, `EQ_GAIN_MID`, `EQ_GAIN_HIGH` exactly; the midpoint of each half sits between its two stops component-wise), `band_gain_color_clamps_and_maps_nan_to_flat` (far below `GAIN_MIN_DB` is the low stop, far above `GAIN_MAX_DB` the high stop, and `f32::NAN` the mid stop), and `band_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb` (both `rail.backgrounds` equal `Background::Color(band_gain_color(gain))` for a nonzero gain; `rail.width`, `rail.border`, and `handle` equal `chrome_slider_style(status)`'s for `Active`, `Hovered`, and `Dragged`).
- `README.md`: after the Usage sentence listing the equalizer panel's widgets, add one sentence: each band slider's groove is tinted by its own gain, from green at `-12 dB` through yellow at `0 dB` to red at `+12 dB`. Do not rewrite the widget-list sentence itself, because the queued response-curve plan also edits it.
- `docs/invariants.md`: add a short `## Equalizer band tint` section after `## UI layout`: the guarantee above, enforced by `style::band_gain_color` and `style::band_slider_style` and used by `views::view_equalizer`, pinned by the three style tests, with a one-line History noting the tint is presentation-only and does not change the curve the audio filter applies.

**Files touched.** `src/ui/theme.rs`, `src/ui/style.rs`, `src/ui/views/equalizer.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `band_gain_color(GAIN_MIN_DB)`, `band_gain_color(0.0)`, and `band_gain_color(GAIN_MAX_DB)` return `theme::EQ_GAIN_LOW`, `theme::EQ_GAIN_MID`, and `theme::EQ_GAIN_HIGH` exactly, and intermediate gains interpolate between the neighboring stops.
- `band_gain_color` clamps values outside `GAIN_MIN_DB..=GAIN_MAX_DB` to the end stops and maps `f32::NAN` to the flat mid stop instead of producing an invalid color.
- `band_slider_style` paints both rail halves with `band_gain_color(gain)` for every slider status and leaves the chrome rail width, border, and handle identical to `chrome_slider_style(status)`.
- The ten band sliders in `view_equalizer` use `band_slider_style` with their own stored gain, so the flat preset renders all-yellow grooves and the Rock preset renders green cuts and red boosts; the existing `equalizer_panel_constructs_for_both_states_and_gain_endpoints` and `equalizer_band_labels_leave_a_gap_between_frequencies` still pass.
- `README.md` describes the gain tint, and `make check` passes with no new dependency.

### Draw the equalizer's response curve above the band sliders (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Close a visible gap in `docs/screenshots/equalizer.png`'s active panel: the mockup draws a green response line above the ten band sliders (flat for the Flat preset, a traced curve for Rock), while `view_equalizer` (`src/ui/views/equalizer.rs`) renders only the header, preamp slider, and band sliders. Draw the line from the stored band gains without adding a dependency, using a column approximation instead of `iced`'s `canvas` feature (which would pull `lyon_path`).

**Approach.**

- In `src/ui/views/equalizer.rs`, add the plot constants `pub(super) const RESPONSE_CURVE_COLUMNS: usize = 64`, `pub(super) const RESPONSE_CURVE_HEIGHT: f32 = 24.0`, and `RESPONSE_CURVE_LINE_HEIGHT: f32 = 2.0`, sized so the extra row fits the panel without changing the slider rows.
- Add the pure `pub(super) fn response_curve_offsets(bands: &[f32; BAND_COUNT], columns: usize) -> Vec<f32>`: returns exactly `columns` offsets, each a height from the plot's bottom (`0.0` at `GAIN_MIN_DB`, `1.0` at `GAIN_MAX_DB`). Column 0 sits on the first band and the last column on the last band; each intermediate column linearly interpolates between the two adjacent band gains, so the line follows the same gains the sliders and `BandFilter` use. Run each endpoint gain through `clamp_gain` before normalizing, so a NaN or an out-of-range value cannot leave the plot. `columns == 0` returns an empty vector; `columns == 1` returns the first band's offset rather than dividing by zero.
- Add `pub(super) fn response_curve_row<'a, Theme, Renderer>(bands: &[f32; BAND_COUNT]) -> Element<'a, Message, Theme, Renderer>`, generic over the theme and renderer like `now_playing_visualizer_row` so the tests can lay it out with iced's null renderer: a `Container` filling the width at `RESPONSE_CURVE_HEIGHT`, with `theme::LCD_BACKGROUND`, holding a `Row` of `RESPONSE_CURVE_COLUMNS` equal-width columns. Each column is a `Column` of a top `Space` of `(1.0 - offset) * (RESPONSE_CURVE_HEIGHT - RESPONSE_CURVE_LINE_HEIGHT)`, a `RESPONSE_CURVE_LINE_HEIGHT` bar in `theme::LCD_GREEN`, and a bottom `Space` of the remainder; adjacent bars touch so the dots read as one line. The preamp is deliberately not plotted: it is a uniform gain offset, not a bend in the curve.
- Push `response_curve_row(bands)` into `view_equalizer` between the preamp row and `band_row`, so the plot spans the ten sliders and reads at the same place as the mockup. It draws whether the EQ is on or off, tracing the stored gains in both states.
- `src/ui/views/tests.rs`: import the new helpers and add `response_curve_offsets_follow_the_band_gains` (a flat `0.0` curve is `0.5` at every column; all-`GAIN_MIN_DB` is `0.0`; all-`GAIN_MAX_DB` is `1.0`; the first and last offsets equal the first and last gain's normalized values; a two-band `-12.0, 12.0` curve with three columns yields `0.0, 0.5, 1.0`; `0` columns yields none and `1` column does not panic). Add `response_curve_row_keeps_its_height_and_draws_one_segment_per_column`, laying the row out with the null renderer (the `now_playing_visualizer_row_keeps_its_height_without_a_spectrum` pattern), asserting the root height is `RESPONSE_CURVE_HEIGHT` and `CapturedContainers` holds `RESPONSE_CURVE_COLUMNS` containers at `RESPONSE_CURVE_LINE_HEIGHT` for a shaped curve including zero-offset columns, with the rising curve's first segment on the bottom edge and its last on the top, then doing the same for a flat curve and requiring every segment on the midline.
- `README.md`: add the response curve to the Usage sentence that lists the equalizer panel's widgets (`EQ on/off button, preset pick list, preamp slider, and a response curve above ten band sliders`).

**Files touched.** `src/ui/views/equalizer.rs`, `src/ui/views/tests.rs`, `README.md`.

**Acceptance criteria.**

- `response_curve_offsets` returns exactly the requested column count, `0.0`/`0.5`/`1.0` for all-min, flat, and all-max curves, endpoints equal to the first and last band gains' normalized positions, and linear interpolation between adjacent bands (`-12.0, 12.0` across three columns is `0.0, 0.5, 1.0`).
- The equalizer panel draws the curve row between the preamp and band rows at the fixed `RESPONSE_CURVE_HEIGHT`, with one line segment per `RESPONSE_CURVE_COLUMNS`; the existing `equalizer_panel_constructs_for_both_states_and_gain_endpoints` and `equalizer_band_labels_leave_a_gap_between_frequencies` still pass.
- The mockup's flat and Rock shapes are both traced: a flat stored curve draws a centered line, and a curved preset draws a line whose high columns sit above its low columns.
- `README.md` lists the curve among the equalizer panel's widgets.
- `make check` passes (fmt, clippy, rustdoc, tests), and the change adds no dependency or `iced` feature.

### Add a shared headless layout-capture harness so UI geometry bugs confirm offline (planned 2026-10-23 by the steward role as a gap promotion for `real-run-needed`) — done 2026-10-23 by the feature role

**Goal.** Retire the `real-run-needed` validation gap for UI geometry: make the laid-out bounds of the real widget tree readable from a unit test, so a layout or label-reflow bug is confirmed by `make test` instead of a hand-run window measurement. Four Fixed entries carry the `real-run-needed` gap. The two non-layout ones now have offline coverage — the sign-in race through the injectable session seam and `AppleMusicService::wait_for_session`, and the nonce leak through the capturing opener in `authorize_keeps_the_state_nonce_out_of_the_opener_argument` — while the two layout ones, "Sort renders below Top instead of beside it, contradicting the documented layout" and "Transport buttons resize to their label text, so pressing Play/Pause reflows the whole control row", were confirmed only by running the GUI and measuring a captured window. The ad hoc `CapturedText` operation in `src/ui/views/tests.rs` and `CapturedTextBounds` in `src/ui/tests.rs` prove the offline path works; this plan promotes them into one shared harness so the next layout bug does not reconstruct it.

**Approach.**

- Add a test-only `src/ui/layout_capture.rs` (declared `#[cfg(test)] pub(crate) mod layout_capture;` in `src/ui/mod.rs`), holding one `CapturedText` widget `Operation` and a small API: a capture function that lays a prepared `Element` out under a fixed `Limits`, runs the operation, and returns each drawn `(text, Rectangle)` with its absolute bounds plus the root `Size`; a `find(label)` lookup that panics with the captured fragments when a label is missing; and `headless_renderer()`, which builds iced's tiny-skia `Headless` renderer with `futures::executor::block_on` (the pattern `browse_nav_buttons_place_sort_beside_top` already uses), so trees containing the fallback-renderer `bevel` widgets lay out too.
- Move the duplicate capture structs out of `src/ui/views/tests.rs` (`CapturedText`) and `src/ui/tests.rs` (`CapturedTextBounds`) and point both suites at the shared module, keeping the existing helpers' assertions (the Now Playing fragment order, the indicator strip's order and width) intact.
- Rewrite `browse_nav_buttons_place_sort_beside_top` on the harness without weakening it: it must still lay out the whole root `view` (not the nav row alone) and assert Sort shares Top's row and starts at or after Top's right edge.
- Add `transport_row_does_not_reflow_when_the_play_label_changes`: lay the root `view` out with the player stopped and then playing, and assert the `Stop` and `Repeat` labels keep the same absolute x, which fails if a label-driven width change reaches the row as it did before the fixed-width fix. Keep the builder-level `transport_buttons_keep_a_fixed_width_across_label_changes` and the measured-width pins unchanged.
- Add a harness self-test that lays out a small known column and asserts the captured fragments and root size, so a broken capture fails directly rather than as a confusing geometry assertion.
- `docs/invariants.md`: add a `## UI layout` section after `## Browse row order` stating that Sort lays out beside Top in the root view and that the transport row's faces keep their bounds across the Play/Pause, Repeat, and Shuffle label changes; name `views::view_browse_nav_buttons`, `views::view_transport_controls`, and `views::transport::transport_buttons` as the enforcers and cite the two layout tests and the two Fixed entries. Add a line to the "How to use" note pointing layout bugs at the harness.

**Files touched.** `src/ui/layout_capture.rs` (new), `src/ui/mod.rs`, `src/ui/tests.rs`, `src/ui/views/tests.rs`, `docs/invariants.md`.

**Acceptance criteria.**

- One shared test module returns every drawn text fragment with its absolute `Rectangle` and the root `Size` for both a null-renderer view and the tiny-skia root view; both existing capture helpers are deleted and their tests run through it.
- `browse_nav_buttons_place_sort_beside_top` still lays out the root `view` and fails if Sort is stacked below Top or drawn before Top's right edge.
- `transport_row_does_not_reflow_when_the_play_label_changes` lays the root view out in both playback states and asserts Stop and Repeat keep their x, so the historical reflow is confirmed offline.
- The new `## UI layout` invariant states both guarantees, names their enforcing code, and lists the tests that pin them; the harness is discoverable from `docs/invariants.md`'s "How to use" note.
- `make check` passes (fmt, clippy, rustdoc, tests), and the change adds no dependency.

**Cited entries.** `BUGS-FIXED.md`: the four `real-run-needed` entries — "Sort renders below Top instead of beside it, contradicting the documented layout", "Transport buttons resize to their label text, so pressing Play/Pause reflows the whole control row", "The boot-time artists fetch races the async sign-in, so a signed-in user sees the sample library", and "The sign-in nonce travels in the browser opener's command line, so another local user can read it and fetch the page's developer token".

### Show the mini transport in window-shade mode (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Match the mockup's rolled-up window (`docs/screenshots/equalizer.png`, the "WINDOW SHADE" panel): shading the window shows, below the title bar, a compact strip with Previous, Play/Pause, Stop, Next, the current elapsed readout, and a volume slider, so playback stays controllable while rolled up. Normal mode is unchanged; the strip only reads existing state and emits existing messages, adding no new playback behavior.

**Approach.**

- Add `src/ui/views/shade.rs` (declare `mod shade;` and re-export it from `src/ui/views/mod.rs` beside `TITLE_BAR_HEIGHT`):
  - `pub(crate) const SHADE_STRIP_HEIGHT: f32 = 28.0;` and `pub(crate) const SHADED_HEIGHT: f32 = TITLE_BAR_HEIGHT + SHADE_STRIP_HEIGHT;`, so the strip height and the whole rolled-up height have one owner each (crate visibility lets `resize_to_shade` read `SHADED_HEIGHT` through `views`).
  - `pub(super) fn play_pause_glyph(is_playing: bool) -> &'static str`: `"II"` while playing and `"▶"` otherwise, the compact twin of `transport::play_pause_label`.
  - `pub(super) fn shade_controls(is_playing: bool) -> [(&'static str, f32, Message); 4]`: the four `(label, fixed width, message)` specs in row order — `"◀◀"`/`Message::PreviousTrack`, [`play_pause_glyph`]/`Message::PlayPause`, `"■"`/`Message::Stop`, `"▶▶"`/`Message::NextTrack` — each with a fixed width (34, 30, 30, 34 px) so the glyph swap cannot reflow the strip. Pure, so labels, messages, and widths are testable without an iced renderer.
  - `pub fn view_shade_strip<'a>(is_playing: bool, volume: f32, elapsed: Cow<'a, str>) -> Element<'a, Message>`: one full-width row built from `fixed_width_button` per `shade_controls` entry with `spacer(6.0)` between them, the elapsed as `Text::new(elapsed).size(14).color(theme::LCD_GREEN)`, and `chrome_slider(VOLUME_MIN, VOLUME_MAX, volume, VOLUME_STEP, 80.0, Message::VolumeChange)` at the right. Paint the row with `style::title_bar_style()` inside a container fixed at `SHADE_STRIP_HEIGHT` (wrapped in `bevel::raised_panel` like the title bar) so the strip shares the title bar's blue chrome. No new message or handler is added.
- `src/ui/mod.rs` `view`: replace the shaded early return's bare title bar with a short state lock for `state.is_playing`, `state.volume()`, and `player.now_playing_elapsed(state.current_track.as_deref())`, then a `Column` of `views::view_title_bar(...)` and `views::view_shade_strip(...)`. Passing the same `now_playing_elapsed` string the LCD uses means the elapsed/remaining toggle carries into shade mode. Update the function's doc comment and inline comment to say the rolled-up tree is the title bar plus the strip. Leave the unshaded assembly untouched.
- `src/ui/handlers/window.rs`: `resize_to_shade` resizes to `views::SHADED_HEIGHT` instead of `views::TITLE_BAR_HEIGHT`, and its doc comment names the strip as part of the rolled-up window. The measurement and restore paths in `toggle_window_shade` and `window_shade_measured` are unchanged.
- `src/ui/views/tests.rs`: add `shade_play_pause_glyph_switches_with_playback_state`, `shade_controls_map_to_transport_messages_and_keep_fixed_widths`, `shade_height_pins_the_rolled_up_window` (`SHADE_STRIP_HEIGHT` pinned to 28.0 and `SHADED_HEIGHT` to their sum), and `view_shade_strip_keeps_its_fixed_height_for_both_playback_states` (lay the widget out with the headless-renderer `Limits` pattern already used in this file and assert the resolved height is `SHADE_STRIP_HEIGHT`).
- `src/ui/tests.rs`: update `title_bar_height_pins_the_shade_strip`'s comment to point at `SHADED_HEIGHT`; add `shaded_root_view_is_the_title_bar_plus_the_strip` (lay out the root `view` with `player.shaded = true` using the headless tiny-skia pattern from `browse_nav_buttons_place_sort_beside_top`, and assert the resolved height is `views::SHADED_HEIGHT`) and `shade_strip_draws_only_while_shaded` (capture text bounds and assert the `"◀◀"` control appears in the shaded tree and not in the normal-mode tree). Keep the existing shade measurement/restore tests green.
- `README.md`: in the Usage paragraph, say shade mode rolls the window up to the title bar plus a compact transport strip with Previous, Play/Pause, Stop, Next, the current time readout, and a volume slider. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Window shade` section after `## Now Playing label`: shade mode builds exactly the title bar plus the fixed-height strip and resizes the window to `SHADED_HEIGHT`; the strip's five controls emit the same messages as their full transport-row twins, its Play/Pause glyph follows the playback state, its time label follows the elapsed/remaining choice, and its volume slider writes `Message::VolumeChange`; normal mode builds no strip. Name `views::shade::view_shade_strip`/`shade_controls`/`play_pause_glyph`/`SHADED_HEIGHT` and `handlers::window::resize_to_shade` as the enforcers, and list the new test names.

**Files touched.** `src/ui/views/shade.rs` (new), `src/ui/views/mod.rs`, `src/ui/mod.rs`, `src/ui/handlers/window.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- In shade mode the rendered tree is the title bar over a strip exactly `SHADE_STRIP_HEIGHT` tall, and the window resizes to `SHADED_HEIGHT`; the normal-mode tree contains no strip.
- The strip shows Previous, Play/Pause, Stop, Next, the current elapsed/remaining readout, and a volume slider, each emitting the same message as its full transport-row twin (`Message::PreviousTrack`, `Message::PlayPause`, `Message::Stop`, `Message::NextTrack`, `Message::VolumeChange`).
- The Play/Pause glyph is `"▶"` while stopped and `"II"` while playing, and every compact button keeps its fixed width across that swap.
- Laying out the strip and the shaded root view with a headless renderer resolves to the fixed heights above, so the view and `resize_to_shade` cannot drift apart.
- Normal-mode layout, the existing shade measurement/restore tests, and playback behavior are unchanged.
- `make check` passes (fmt, clippy, rustdoc, tests).

### Add the LCD spectrum visualizer (Spectrum visualizer, part 2/2: planned 2026-10-23 by the plan role, requires part 1/2 landed) — done 2026-10-23 by the feature role

**Goal.** Complete the mockup's LCD (`docs/screenshots/player.png`): draw a fixed-height row of ten green bars beneath the Now Playing title, whose heights follow the loaded preview's live spectrum from part 1, updating about four times a second while playback runs. The strip is blank (but keeps its height) when no spectrum is reported, so the silent fallback and a mid-download selection do not jump the layout. Pause freezes the bars at their last levels; Stop and a new selection clear them through the same epoch/reset path the sample-rate status row uses. No new dependency and no change to playback or the browse list.

**Approach.**

- `src/apple_music/playback.rs`: add `pub(crate) fn playback_spectrum(&self) -> Option<audio::Spectrum>` forwarding to `self.audio.spectrum()`, documented like `playback_format`. Add `playback_spectrum_forwards_the_audio_output_spectrum` to `src/apple_music/tests.rs`, driving the injected `RecordingAudio`.
- `src/test_support.rs`: give `RecordingAudio` a fixed stub `spectrum` (a `Spectrum` with one distinctive nonzero band) so UI tests can pin what the backend reports, matching its stub position, format, and duration.
- `src/ui/mod.rs`: add `spectrum: Option<Spectrum>` to `WinampPlayer`, initialized `None`, and import `Spectrum` beside `PlaybackFormat`. Add `Message::SpectrumTick` (the timer's tick) and `Message::Spectrum { epoch: u64, spectrum: Option<Spectrum> }` (the query's reply). Add the `update` arms forwarding to the handlers below, add `needs_spectrum_poll` beside `needs_position_poll` (true only while `state.is_playing`), and pass `player.spectrum` to `views::view_now_playing`.
- `src/ui/timers.rs`: add `pub(super) const SPECTRUM_TICK: Duration = Duration::from_millis(250)` and mirror the position timer's plumbing: `spectrum_ticks`, `spectrum_tick_stream`/`spectrum_tick_stream_with`, `spawn_spectrum_thread`, `SPECTRUM_THREAD_NAME = "nostalgia-spectrum-poll"`, and a failure report naming the visualizer. Add a `spectrum_needed` argument to `subscriptions`, and pass `player.needs_spectrum_poll()` from `init_ui`, so the timer runs only while playback is running and is dropped on Pause/Stop.
- `src/ui/handlers/playback.rs`: add `spectrum_tick(player) -> Task<Message>`: when no transport transition is pending, capture `position_epoch` and schedule `apple_music_service.playback_spectrum()`, mapping the reply to `Message::Spectrum { epoch, spectrum }`; otherwise return `Task::none()`. Add `spectrum(player, epoch, spectrum) -> Task<Message>` storing the value only when the epoch is current and no transition is pending, so stale replies cannot restore an old preview's bars. Clear `player.spectrum` in `reset_position_readout` beside the existing position, format, and duration clears, so Stop and a new selection blank the strip while Pause leaves it frozen.
- `src/ui/views/now_playing.rs`: add the visualizer constants (`LCD_VISUALIZER_HEIGHT`, `LCD_VISUALIZER_BAR_WIDTH`, `LCD_VISUALIZER_BAR_GAP`) and `pub fn visualizer_bar_heights(spectrum: Option<Spectrum>) -> [f32; BAND_COUNT]`: `None` yields all-zero heights, and each band is clamped to `0.0..=1.0` and multiplied by the maximum bar height. Add `now_playing_visualizer_row` drawing exactly `BAND_COUNT` bottom-aligned `Container` bars left to right in `theme::LCD_GREEN` inside a row of fixed `LCD_VISUALIZER_HEIGHT`; an empty spectrum draws the same row with zero-height bars. Push that row between `now_playing_row` and `now_playing_status_row` in `now_playing_content`, and thread the `spectrum` argument through `view_now_playing` and `now_playing_content`.
- `src/ui/views/tests.rs`: add `visualizer_bar_heights_scale_and_clamp_the_reported_bands` (`None` all zero, bands `0.0`/`0.5`/`1.0`/`2.0` map to zero/half/full/full) and a `view_now_playing` construction test with `Some` and `None` spectra asserting the title and status text still draw and the visualizer row keeps its height in both cases.
- `src/ui/tests.rs`: add `spectrum_reply_stores_the_loaded_previews_spectrum`, `spectrum_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`, `stop_clears_the_spectrum_while_pause_keeps_it`, `spectrum_tick_queries_spectrum_and_carries_the_epoch`, and `the_spectrum_timer_runs_only_while_playback_runs`; update `position_tick_queries_position_and_format`-style timer tests for the new `subscriptions` argument where the signature changed.
- `README.md`: in the Usage paragraph describing the LCD, say a loaded preview drives a small green ten-bar spectrum beneath the title, that Pause freezes it, and that the strip stays blank (with its height) when no preview reports audio. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Preview spectrum`, add the UI half: the LCD draws exactly `BAND_COUNT` bars from the stored spectrum, heights clamp to the fixed row, Pause freezes them, Stop and a new selection clear them, stale or pre-transition replies are dropped, and the poll runs only while playing. Extend `## Now Playing label` with the visualizer bullet and add `views::now_playing::visualizer_bar_heights`/`now_playing_visualizer_row`, `WinampPlayer::needs_spectrum_poll`, and the new handler and timer test names to the enforcer and Pinned-by lists.

**Files touched.** `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/mod.rs`, `src/ui/timers.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `visualizer_bar_heights` returns exactly ten heights in `BAND_COUNT` order, all zero for `None` and for zero bands, half the maximum for `0.5`, and the maximum for values at or above `1.0`.
- The LCD draws the fixed-height visualizer row beneath the title and above the status row in both the reporting and blank states, so loading a preview or the silent fallback does not move the title or status text.
- The spectrum poll is subscribed only while playback runs, queries `AudioOutput::spectrum` through the service, and drops a reply whose epoch is stale or whose transport transition is still pending; Pause keeps the last bars, Stop and a new selection clear them.
- The silent fallback and a backend without a spectrum leave every bar at zero while keeping the row's height.
- `make check` passes (fmt, clippy, rustdoc, tests).

### Add the LCD spectrum visualizer (Spectrum visualizer, part 1/2: planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Add a narrow backend query that reports the loaded preview's live ten-band amplitude spectrum, computed from the samples the audio worker actually pulls. This is the deferred half of the mockup's LCD visualizer (`docs/screenshots/player.png` draws green bars beneath the elapsed time): it gives part 2 a real signal to draw instead of an animation, with no new dependency and no UI change in this part. The meter is observational only — it must not alter the samples reaching the equalizer, balance, or output. When no preview is loaded, or the backend cannot answer (the silent fallback, a test fake, a closed worker), the query reports nothing, so part 2 can keep its strip blank.

**Approach.**

- `src/audio/meter.rs` (new module declared in `src/audio.rs`): add `pub const SPECTRUM_WINDOW: usize = 2048`, `pub struct Spectrum { pub bands: [f32; crate::equalizer::BAND_COUNT] }`, and re-export `Spectrum` from `src/audio.rs`. A band value is a normalized `0.0..=1.0` amplitude in `crate::equalizer::BAND_HZ` order, so the visualizer and equalizer name the same ten frequencies.
- In the same module, add `pub(super) fn spectrum_bands(samples: &[f32], sample_rate: u32) -> [f32; crate::equalizer::BAND_COUNT]`: run a Goertzel magnitude at each `BAND_HZ` over the supplied mono window, normalize a full-scale sine to about `1.0` (`2 * magnitude / window_len`), clamp each band into `0.0..=1.0`, and report `0.0` for a band at or above Nyquist or for silence. Keep it pure so the frequency mapping is testable without an audio device.
- Add `SpectrumMeter` as the shared state: a `std::sync::Mutex<SpectrumWindow>` holding a fixed `[f32; SPECTRUM_WINDOW]` ring plus a downmix accumulator, and an `AtomicU32` sample rate. `push(sample, channels)` downmixes interleaved samples to mono and overwrites the oldest slot once the ring fills; `take()` copies the ring in chronological order under the lock and computes `spectrum_bands` after releasing it, so the audio thread never waits on the Goertzel work; `clear()` empties the ring; poisoned locks are recovered rather than panicking (the crate forbids production `unwrap`/`panic`).
- Add `pub(super) struct MeterSource<S> { inner: S, meter: Arc<SpectrumMeter> }`: `new` records `inner.sample_rate()`, `next()` pushes the sample and returns it unchanged, and the `Source` impl delegates `current_span_len`, `channels`, `sample_rate`, `total_duration`, and `size_hint` to the inner source. `try_seek` delegates and then clears the window, so a seek does not splice old audio into the new position's spectrum.
- `src/audio.rs`: add `fn spectrum(&self) -> Option<Spectrum> { None }` to `AudioOutput`, documented like `playback_format` (only while a preview is loaded; `None` for the silent fallback, a fake, or an unanswered query). The default keeps every existing implementor compiling.
- `src/audio/player.rs`: `RodioPlayer` gains `spectrum: Arc<SpectrumMeter>`, initialized in `new`. `WorkerPlayer` gains `fn spectrum(&self) -> Option<Spectrum> { None }`. In `append`, after reading the format and duration and before building `EqSource`, wrap the decoded preview with `MeterSource::new(preview, Arc::clone(&self.spectrum))`; leave the rest of the chain and the returned samples unchanged. `RodioPlayer::spectrum` reports `Some(self.spectrum.take())` only while the player holds a preview (`self.format.get().is_some()`), otherwise `None`. Add `Command::Spectrum(Sender<Option<Spectrum>>)`, handle it in `serve_commands` beside `PlaybackFormat`, and add `RodioOutput::spectrum` mirroring `RodioOutput::playback_format`, including the existing query timeout.
- `src/audio/tests.rs`: add `spectrum_bands_peak_at_the_matching_band_for_a_sine` (a 1 kHz stereo-free sine wins the `1_000.0` band and silence gives all zeros), `spectrum_bands_zero_a_band_at_or_above_nyquist` (at 22_050 Hz the `16_000.0` band is zero while 1 kHz remains), `meter_source_passes_samples_through_and_records_their_spectrum` (collect a sine `SamplesBuffer` through `MeterSource`, assert the output samples are unchanged and `take()` peaks at the sine's band), `spectrum_query_returns_the_loaded_players_spectrum` and `spectrum_query_reports_none_when_nothing_is_loaded` (give `RecordingPlayer` an overridable `spectrum`), `silent_output_reports_no_spectrum`, and extend `rodio_output_reports_a_closed_worker_for_every_command` with a `None` spectrum answer. Add a `MeterSource` delegation assertion for sample rate, channels, and total duration.
- `docs/invariants.md`: add a `## Preview spectrum` section after `## Preview download`: the backend reports ten normalized band amplitudes in `BAND_HZ` order only while a preview is loaded, computed from unchanged samples pulled through `MeterSource`; silence and bands at or above Nyquist read zero; Stop, a new selection, and a seek clear the window; the silent fallback and a closed worker report `None`. Name `audio::meter::Spectrum`/`spectrum_bands`/`MeterSource`, `audio::AudioOutput::spectrum`, and `audio::player::RodioPlayer::spectrum` as the enforcers, and list the new test names.

**Files touched.** `src/audio/meter.rs`, `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `docs/invariants.md`.

**Acceptance criteria.**

- `spectrum_bands` maps a 1 kHz sine to the `1K` band with the other bands lower, maps silence to all zeros, and reports zero for a band at or above Nyquist.
- `MeterSource` returns every sample it is given unchanged, delegates the source's sample rate, channels, length, and span, and records the same samples into its `SpectrumMeter`; a seek clears the recorded window.
- `AudioOutput::spectrum` answers `Some` for a loaded preview through `RodioOutput`, and `None` for `SilentOutput`, a recording fake without an override, and a closed worker; Stop and a new preview cannot report the previous preview's spectrum.
- No UI code reads the spectrum yet; the equalizer, balance, volume, and output samples are unchanged by the meter.
- `make check` passes (fmt, clippy, rustdoc, tests).

### Show the loaded preview's bitrate in the LCD status row (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Complete the mockup's LCD status line (`docs/screenshots/player.png`, `256 KBPS 44 KHZ`) by showing the loaded preview's average bitrate beside the sample rate and channel mode: `256 KBPS  44 KHZ  STEREO`. When the backend cannot compute it, the row falls back to today's `44 KHZ  STEREO`. The value is presentation only, derived from the downloaded encoded preview body and the decoded source's own total duration; it follows the loaded preview as the sample rate does (Pause keeps it, Stop and a new selection clear it). The landed status-row plan left bitrate out because `preview::download_and_decode` discards the body length, so this plan carries that length through to `PlaybackFormat`. It is independent of the queued `O A I D V` strip: that adds a column inside the well, while this changes the format value and the existing status text.

**Approach.**

- `src/audio/preview.rs`: add `pub(super) struct DecodedPreview { pub source: rodio::Decoder<Cursor<Vec<u8>>>, pub encoded_bytes: u64 }`. In `download_and_decode`, record `bytes.len()` as `encoded_bytes` before the bytes move into `Cursor`, and return the struct after `rodio::Decoder::new_mp4`. Keep the existing status check, wire/decoded caps, and error wording unchanged.
- `src/audio.rs`: add `pub bitrate_kbps: Option<u32>` to `PlaybackFormat`, documented as the preview's average encoded bitrate in kilobits per second and `None` when the backend cannot report the decoded source's length.
- `src/audio/player.rs`: change `RodioPlayer`'s `WorkerPlayer::Preview` to `DecodedPreview`. Add the pure helper `pub(super) fn preview_bitrate_kbps(encoded_bytes: u64, duration: Duration) -> Option<u32>`: multiply bytes by 8, divide by `duration.as_millis()` (one bit per millisecond is one kilobit per second), round to nearest, and return `None` for zero duration or a result that does not fit `u32`. Change `format_for_source` to take the encoded byte count, fill `bitrate_kbps` by calling `preview_bitrate_kbps` with `source.total_duration()` and passing through `None` when the source reports no length, and keep reading sample rate and channels as it does today. In `append`, destructure `DecodedPreview`, pass `encoded_bytes` to `format_for_source`, and read the same source's duration for `duration_for_source` as today.
- `src/ui/views/now_playing.rs`: change `format_playback_status` to prefix `"{kbps} KBPS  "` when `bitrate_kbps` is `Some`; keep `"{khz} KHZ  {mode}"` when it is `None`. Remove the doc sentence that says bitrate is deliberately absent, and note the fallback. `now_playing_status_row` and its fixed height stay as they are.
- `src/audio/tests.rs`: add `preview_bitrate_kbps_computes_encoded_bits_over_the_preview_length` (`960_000` bytes over 30 seconds is `Some(256)`; zero duration is `None`). Update `format_for_source_reads_sample_rate_and_channels` for the new argument and assert the computed bitrate for a buffer whose total duration is known, and add `bitrate_kbps: None` to every other `PlaybackFormat` literal in the file unless the test is about bitrate.
- `src/ui/views/tests.rs`: extend `format_playback_status_formats_khz_and_channel_mode` with `Some(256)` reading `"256 KBPS  44 KHZ  STEREO"` and a `None` case reading `"44 KHZ  STEREO"`; extend `now_playing_status_row_draws_the_format_or_nothing` to capture the bitrate text; add `bitrate_kbps` to the remaining literals.
- `src/ui/tests.rs`, `src/apple_music/tests.rs`, and `src/test_support.rs`: add `bitrate_kbps: None` (or a value where the fake is meant to represent a loaded preview with a known rate) to every `PlaybackFormat` literal so the suite compiles.
- `README.md`: in the Usage sentence describing the status row, say it shows the preview's bitrate, sample rate, and channel mode (`256 KBPS  44 KHZ  STEREO`) when the backend reports them, and falls back to the sample rate and mode alone when it cannot compute the bitrate. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, replace the status-row bullet's "Bitrate is not shown" sentence with the guarantee that a loaded preview's average bitrate is shown when the download length and decoded duration are both known, is omitted otherwise, and follows the same Pause/Stop/epoch rules as the sample rate. Name `audio::preview::DecodedPreview`, `audio::player::preview_bitrate_kbps`/`format_for_source`, and `views::now_playing::format_playback_status` as the enforcers, and add the new test names to the Pinned-by list.

**Files touched.** `src/audio/preview.rs`, `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `preview_bitrate_kbps(960_000, Duration::from_secs(30)) == Some(256)`, a zero-length duration returns `None`, and `format_for_source` fills `bitrate_kbps` from the encoded byte count and the source's own total duration.
- `download_and_decode` returns a `DecodedPreview` whose `encoded_bytes` is the length of the downloaded body and whose `source` is the decoded preview; the existing download-failure and cap errors are unchanged.
- `format_playback_status` renders `256 KBPS  44 KHZ  STEREO` for `Some(256)`, 44_100 Hz, two channels, and keeps `44 KHZ  STEREO` when the bitrate is `None`; the status row draws that text and keeps its fixed height and blank state.
- Bitrate follows the existing preview lifecycle with no new state: Pause keeps it, Stop and a new selection clear it through the existing `PlaybackFormat` reset and epoch guards, and the silent fallback or a backend without a duration reports no bitrate.
- `make check` passes (fmt, clippy, rustdoc, tests).

### Draw the classic O A I D V strip at the LCD's left edge (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Draw the mockup's still-missing static LCD element (`docs/screenshots/player.png`): a vertical column of the classic `O`, `A`, `I`, `D`, `V` letters down the well's left edge, beside the large elapsed readout and the title/status rows. The strip is decorative — it emits no messages and toggles nothing, because the app already exposes always-on-top through the title bar's `A` button — and it is drawn in every Now Playing state, including the empty `--:--` bar, so the LCD matches the mockup without duplicating controls. It has a fixed width so the content beside it keeps its bounds: the title starts one strip-width further right, and the right-edge total stays at the edge because the content column fills the remaining width. This is independent of the queued seek slider: that plan adds a strip below the LCD well, while this one adds content inside it.

**Approach.**

- `src/ui/views/now_playing.rs`: add `pub(super) const LCD_INDICATOR_LABELS: [&str; 5] = ["O", "A", "I", "D", "V"]` (the mockup's order; the steward's 2026-10-22 note spells the strip `O C I D V`, which the screenshot shows is a typo), a fixed-width constant (`LCD_INDICATOR_WIDTH = 12.0`), and a small text-size constant (`LCD_INDICATOR_SIZE = 10.0`). Add `pub(super) fn lcd_indicator_strip<'a, Theme, Renderer>() -> Column<'a, Message, Theme, Renderer>` with the same text-catalog and renderer bounds as `lcd_text`, building one small [`theme::LCD_GREEN`] `Text` per label in a fixed-width column with tight spacing, top-aligned and vertically arranged. Change `view_now_playing` to put the strip and the existing title/status `Column` in a `Row` before wrapping that row in `bevel::lcd_well`, so the letters sit inside the same recessed well; keep `now_playing_row`, `now_playing_status_row`, and all readout logic unchanged. Add a `pub(super) fn now_playing_content` generic builder holding that `Row`, because `bevel::lcd_well` returns an element tied to iced's fallback renderer and so cannot be laid out with the null renderer the tests use; `view_now_playing` becomes a one-line wrap of it. Update the function's doc comment to mention the decorative strip.
- `src/ui/views/tests.rs`: import `lcd_indicator_strip` and `now_playing_content` (not the constants). Extend `CapturedText` to record each fragment's absolute bounds alongside its text, and add a `captured_indicator_texts` helper that lays out `lcd_indicator_strip()` with iced's null renderer through that operation, returning the drawn text and the laid-out width. Add `lcd_indicator_strip_draws_each_label_in_order_at_a_fixed_width`, asserting the captured text equals the literal `["O", "A", "I", "D", "V"]` and the width equals the literal `12.0`, so changing either production constant fails the test. Add `now_playing_content_draws_the_indicator_strip_beside_the_title`, which lays out `now_playing_content` (a fitting track, e.g. `"00:53"` / `"Opening"` / `"Band"` / `"3:30"` with no format) through the same operation, asserts the first five drawn fragments are the five literal letters, asserts each letter's `x` sits left of the elapsed value, the title, and the total, and asserts the total's right edge reaches the laid-out content width, so a future refactor cannot drop the strip or displace the right-edge total while leaving its builder tested.
- `README.md`: in the Usage sentence that describes the Now Playing bar, say a small vertical `O A I D V` strip runs down the LCD's left edge as in the mockup and is decorative. Keep the hunk to that sentence; do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the `## Now Playing label` section, add a bullet stating that the LCD's left edge always draws the five classic indicator letters `O`, `A`, `I`, `D`, `V` in that order at a fixed width, in every playback state, and that the strip is decorative (it emits no message and changes nothing); name `views::now_playing::LCD_INDICATOR_LABELS` and `lcd_indicator_strip` as the enforcers and add the two new test names to the Pinned-by list.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `LCD_INDICATOR_LABELS` is exactly `["O", "A", "I", "D", "V"]`, and `lcd_indicator_strip` draws them in that order, one letter per line, in the LCD green at a fixed `LCD_INDICATOR_WIDTH` width.
- `view_now_playing` lays the strip out at the left of the existing elapsed/title/status content inside `bevel::lcd_well` (through `now_playing_content`, which the null-renderer test lays out); laying out that content captures all five letters and the unchanged elapsed, title, and total text.
- The strip is present in the empty and loaded states, has no `Message` and no event handler, and does not alter the large readout, the marquee, the status row, or the right-edge total.
- The README and `docs/invariants.md` describe the strip as decorative, and `make check` passes (fmt, clippy, rustdoc, tests).

### Add a draggable seek slider beneath the Now Playing LCD (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Fill the horizontal groove `docs/screenshots/player.png` draws between the LCD well and the transport row: a chrome slider whose thumb tracks the loaded preview's playback position, and which seeks when the user drags and releases it. The bar spans the loaded preview's own backend-reported length rather than the catalog track's length, because a preview asset can be shorter than the song (the right-edge LCD total keeps showing the catalog length). It is drawn only while the backend reports both a position and a preview length — the production `rodio` backend does; the silent fallback does not — and otherwise renders as an empty fixed-height strip so the layout does not jump. Dragging moves only the thumb; releasing sends exactly one absolute seek. This is independent of the queued sample-rate/channel status row: that plan adds a row inside the LCD well and a format query, while this one adds a sibling strip below the well plus a duration query and an absolute-seek seam. Both plans edit `position_tick` and `reset_position_readout`, so whichever lands second extends the existing batch and clear rather than replacing them.

**Approach.**

- `src/audio.rs`: add `fn seek_to(&self, position: Duration) -> Result<(), AppleMusicError>` and `fn duration(&self) -> Option<Duration> { None }` to `AudioOutput`. Document `seek_to` as an absolute seek of the current player, clamped by the backend to the loaded preview's length and doing nothing while stopped; document `duration` as the loaded preview's own length, answering `None` when no player is loaded or the backend cannot answer, with the default keeping existing fakes compiling. `SilentOutput::seek_to` logs and returns `Ok`; `duration` inherits `None`.
- `src/audio/player.rs`:
  - Add `Command::SeekTo(Duration)` and `Command::Duration(Sender<Option<Duration>>)`.
  - Add `fn seek_to(&self, position: Duration)` and `fn duration(&self) -> Option<Duration>` to `WorkerPlayer`.
  - Add `pub(super) fn duration_for_source(source: &impl rodio::Source) -> Option<Duration>` beside `seek_target`, returning `source.total_duration()`. Give `RodioPlayer` a `duration: OnceLock<Option<Duration>>`; in `append`, set it from `duration_for_source(&preview)` before the preview is wrapped and moved. `RodioPlayer::duration` returns the stored value, and `RodioPlayer::seek_to` clamps the target to that value (a `clamp_seek(position, duration)` free function beside `seek_target`, with `None` meaning no clamp) before calling `self.player.try_seek`, logging a rejected seek without changing state.
  - In `serve_commands`, handle `Command::SeekTo(position)` by calling `player.seek_to(position)` only when a player is installed, and `Command::Duration(reply)` by sending `player.as_ref().and_then(WorkerPlayer::duration)`.
  - Implement `AudioOutput::seek_to` on `RodioOutput` by sending `Command::SeekTo`, and `AudioOutput::duration` by sending `Command::Duration` and waiting on the reply with the existing `POSITION_QUERY_TIMEOUT` (`None` when the send fails or the wait times out). Extend that constant's doc to say it bounds the position and duration queries, without renaming it.
- `src/apple_music/playback.rs`: add `pub(crate) fn seek_output_to(&self, position: Duration) -> Result<(), AppleMusicError>` forwarding to `self.audio.seek_to(position)`, and `pub(crate) fn playback_duration(&self) -> Option<Duration>` forwarding to `self.audio.duration()`. Document both as synchronous and bounded by the seam, like `playback_position`.
- `src/ui/views/seek_bar.rs` (new small module): add `pub fn seek_bar_value(position: Option<Duration>, duration: Option<Duration>) -> Option<(f32, f32)>`, returning `(thumb seconds, length seconds)` with the thumb clamped into `0..=length` only when both a position and a positive duration are reported, or `None` when either is absent (or the duration is zero), so the bar appears only once the backend reports a position to place its thumb. Add `pub fn view_seek_bar(position: Option<Duration>, duration: Option<Duration>) -> Element<'static, Message>`: when `seek_bar_value` answers, build a `Slider::new(0.0..=length, thumb, Message::SeekPreviewChanged)` with `.step(1.0)`, `.on_release(Message::SeekPreviewReleased)`, `Length::Fill` width, and `style::chrome_slider_style`; when it does not, build an empty `Space` of the same fixed height. Wrap either in a fixed-height `Container` (`SEEK_BAR_HEIGHT = 16.0`) so the strip never changes height. Construct the `Slider` here rather than widening the fixed-width `views::chrome_slider` helper, because this slider fills the row.
- `src/ui/views/mod.rs`: declare `mod seek_bar;` and re-export `seek_bar_value`/`view_seek_bar` beside the other view builders.
- `src/ui/mod.rs`: add `playback_duration: Option<Duration>` and `seek_preview: Option<Duration>` to `WinampPlayer`, both initialized `None` and documented as the backend-reported preview length and the in-progress drag. Add `Message::PlaybackDuration { epoch: u64, duration: Option<Duration> }`, `Message::SeekPreviewChanged(f32)`, and `Message::SeekPreviewReleased`, routed from the `update` table to `handlers::playback`. In `view`, push `views::view_seek_bar(player.seek_preview.or(player.playback_position), player.playback_duration)` after `view_now_playing` and before the transport row; both fields are plain player state, so no state lock is needed.
- `src/ui/handlers/playback.rs`:
  - `position_tick` batches the existing position query with a second query of a cloned service's `playback_duration()` mapped to `Message::PlaybackDuration`, and returns `Task::batch`; both are skipped while `pending_position_transitions` is nonzero.
  - `playback_duration` stores the reply only when `epoch == player.position_epoch` and `pending_position_transitions == 0`, mirroring `playback_position`.
  - `seek_preview_changed` stores `Duration::from_secs_f32(value.max(0.0))` in `seek_preview` and returns no task, so dragging never touches the backend.
  - `seek_preview_released` takes the stored drag: when it is `Some(position)`, call `player.apple_music_service.seek_output_to(position)`; on success bump `position_epoch` (dropping any position or duration reply already in flight), set `playback_position = Some(position)`, and clear `seek_preview`; on failure log the backend error and clear `seek_preview` so the next poll restores the real position. When no drag is stored it only clears the field.
  - `reset_position_readout` also clears `playback_duration` and `seek_preview`, so Stop and a new selection remove the bar's scale and any drag with the position.
- Tests. `src/audio/tests.rs`: extend `every_command` with `seek_to` (and the closed-worker and command-mapping tests with it); add `silent_output_reports_no_playback_duration` asserting `duration().is_none()`; add `duration_for_source_reads_a_samples_buffers_length`, `clamp_seek_clamps_to_the_preview_length_and_passes_through_without_one`, `rodio_output_asks_the_worker_for_its_duration`, `rodio_output_reports_none_when_the_duration_reply_times_out`, `seek_to_command_reaches_the_loaded_player`, `duration_query_returns_the_loaded_players_duration`, and `duration_query_reports_none_when_nothing_is_loaded`; give `RecordingPlayer` a settable absolute position and duration, make `seek_to` set the position, and keep its no-seek position at `42s` for the existing position tests. `src/test_support.rs`: add `AudioCall::SeekTo(Duration)`, record it in `RecordingAudio::seek_to`, return a fixed `Some(Duration::from_secs(30))` from `RecordingAudio::duration`, and implement `FailingAudio::seek_to` through the existing `FailingCommand::Seek` with `duration` answering `None`. `src/apple_music/tests.rs`: add `seek_output_to_forwards_an_absolute_position`, `seek_output_to_propagates_an_audio_backend_failure`, and `playback_duration_forwards_the_audio_output_duration`. `src/ui/views/tests.rs`: add `seek_bar_value_clamps_the_thumb_and_hides_without_a_length` (position 45s/length 30s → `(30.0, 30.0)`, `None` position → `None`, absent/zero length → `None`) and call `view_seek_bar` with a known and an unknown length to pin that both construct without panicking. `src/ui/tests.rs`: add `playback_duration_reply_stores_the_loaded_previews_length`, `playback_duration_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`, `stop_and_track_selection_clear_the_playback_duration`, `seek_preview_changed_tracks_the_drag_without_touching_the_backend`, `seek_preview_released_seeks_once_to_the_dragged_position` (asserting one `AudioCall::SeekTo`, the optimistic `playback_position`, and a bumped epoch), `seek_preview_released_without_a_drag_does_nothing`, `seek_preview_released_invalidates_an_in_flight_position_reply`, `seek_preview_released_backend_failure_leaves_the_position_alone`, and `position_tick_queries_position_and_duration` (drive the batched task's stream to its end and assert both replies arrive).
- `README.md`: in the Usage paragraph that describes the Now Playing bar, say a slider beneath it tracks the loaded preview and drags to seek, spans the preview's own length, and stays blank when the backend reports no length. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the `## Now Playing label` section, add the seek-slider guarantee (the bar measures the backend-reported preview length, the thumb follows the stored position or the drag in progress, only a release sends `AudioOutput::seek_to`, the seek is clamped to that length, a successful seek records the position optimistically and invalidates in-flight replies, a failed seek lets the next poll correct the thumb, and Stop/new selection clear the length and drag) and name `seek_bar_value`, `view_seek_bar`, the `SeekPreviewChanged`/`SeekPreviewReleased`/`PlaybackDuration` handlers, `seek_output_to`, and the duration query as the enforcers. Add the new test names to the Pinned-by list.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/views/seek_bar.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `seek_bar_value` returns `(30.0, 30.0)` for a 45-second position on a 30-second preview, and `None` for a missing position or a missing or zero preview length; the thumb never exceeds the bar's length.
- The slider is drawn beneath the LCD only while both a position and a positive preview length are reported; with no preview, the silent backend, Stop, or a new selection the strip is blank and keeps its fixed height.
- Dragging the slider stores the drag value and sends no backend command; releasing sends exactly one absolute `seek_to` with the dragged position, which the backend clamps to the preview length, and the bar shows that position until the next backend reply.
- Stop and a new selection clear the stored preview length and any drag immediately; a `PlaybackDuration` reply stamped with an older epoch or landing while a transport transition is in flight is dropped.
- `position_tick` asks the backend for both the position and the duration, and asks for neither while a transport transition is in flight.
- The right-edge LCD total continues to show the catalog track's length, not the preview length, and the README and `docs/invariants.md` describe the slider that way; `make check` passes (fmt, clippy, rustdoc, tests).

### Show the loaded preview's sample rate and channel mode in the Now Playing LCD (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Fill the real, queryable part of the mockup's LCD status line (`docs/screenshots/player.png`): while a preview is loaded, show its sample rate and mono/stereo mode in a small green row beneath the Now Playing title (`44 KHZ  STEREO`), and leave that row blank when no preview or backend reports a format. The value is presentation only and follows the loaded preview: Pause keeps it, Stop and a new selection clear it. Bitrate stays out of scope because `download_and_decode` discards the downloaded body length, so `256 KBPS` cannot be computed without a separate preview-metadata change. This is a separate LCD concern from the queued elapsed/remaining toggle — that plan changes the large left readout and `KnownTrack`, while this one adds an audio query and the status row — and the two are additive edits at distinct insertion points in the same files.

**Approach.**

- `src/audio.rs`: add `pub struct PlaybackFormat { pub sample_rate_hz: u32, pub channels: u16 }` (derive `Debug, Clone, Copy, PartialEq, Eq`, with doc comments on the type and both fields). Add `fn playback_format(&self) -> Option<PlaybackFormat> { None }` to `AudioOutput`, documented as reporting the loaded preview's format and answering `None` when no player is loaded or the backend cannot answer; the default keeps `SilentOutput` and every existing test fake compiling, and `SilentOutput` inherits the `None` answer.
- `src/audio/player.rs`:
  - Import `rodio::Source` and add `pub(super) fn format_for_source(source: &impl rodio::Source) -> PlaybackFormat` beside `seek_target`, reading `source.sample_rate().get()` and `source.channels().get()`. Give `RodioPlayer` a `format: OnceLock<PlaybackFormat>` initialized in `new`; in `append`, call `format_for_source(&preview)` before moving the preview and set the cell (each track builds a fresh player, so it is set once). Add `fn playback_format(&self) -> Option<PlaybackFormat>` to `WorkerPlayer` (default `None`, so the recording fake compiles) and have `RodioPlayer` return the stored copy.
  - Add `Command::PlaybackFormat(Sender<Option<PlaybackFormat>>)`. In `serve_commands`, answer it with `player.as_ref().and_then(WorkerPlayer::playback_format)`, so a stopped or failed player has no format.
  - Implement `AudioOutput::playback_format` on `RodioOutput` by sending the command and waiting on the reply with the existing `POSITION_QUERY_TIMEOUT` (`None` when the send fails or the wait times out). Extend that constant's doc to say it bounds both queued backend queries, since a download can hold the worker past it.
- `src/apple_music/playback.rs`: add `pub(crate) fn playback_format(&self) -> Option<audio::PlaybackFormat>` forwarding to `self.audio.playback_format()`, documented as synchronous and bounded by the seam (like `playback_position`).
- `src/ui/views/now_playing.rs`:
  - Add `pub fn format_playback_status(format: PlaybackFormat) -> String`, returning `"{khz} KHZ  {mode}"` where `khz = sample_rate_hz / 1000` and `mode` is `"MONO"` for one channel and `"STEREO"` for any other count. Document the integer-kilohertz truncation and that previews are mono or stereo.
  - Add `pub(super) fn now_playing_status_row<'a, Theme, Renderer>(format: Option<PlaybackFormat>) -> Row<'a, Message, Theme, Renderer>` with the same text-catalog and renderer bounds as `now_playing_row`. It is a fixed-height row (16px) with a `Length::Fill` spacer then a small (12px) `theme::LCD_GREEN` status text right-aligned; with `None` it draws no text but keeps the height, so the bar does not jump when a preview loads.
  - Change `view_now_playing` to take a sixth `format: Option<PlaybackFormat>` argument and wrap `Column::new().push(now_playing_row(...)).push(now_playing_status_row(format))` in `bevel::lcd_well`, so the status line sits beneath the title/total inside the same LCD well. Keep `now_playing_row` itself unchanged.
- `src/ui/mod.rs`: add `playback_format: Option<PlaybackFormat>` to `WinampPlayer`, initialized `None` and documented as the loaded preview's backend-reported format (Pause keeps it; Stop and a new selection clear it). Add `Message::PlaybackFormat { epoch: u64, format: Option<PlaybackFormat> }` with a doc comment naming the epoch guard, route it from the `update` table to `handlers::playback::playback_format`, and add the field to the tuple `view` resolves under the state lock before passing it to `views::view_now_playing`.
- `src/ui/handlers/playback.rs`: in `position_tick`, after the pending-transition guard, build the existing position query and a second query of a cloned service's `playback_format()` mapped to `Message::PlaybackFormat`, and return `Task::batch([position, format])`; both are therefore skipped while a transition is in flight. Add `pub(in crate::ui) fn playback_format(player, epoch, format) -> Task<Message>` that stores `format` only when `epoch == player.position_epoch` and `pending_position_transitions == 0`. In `reset_position_readout`, also set `player.playback_format = None`, so Stop and a new selection clear the row immediately; Pause deliberately leaves the existing reset path alone.
- Tests. `src/audio/tests.rs`: add `format_for_source_reads_sample_rate_and_channels` over a mono and a stereo `SamplesBuffer`; give `RecordingPlayer` a `playback_format` returning a fixed format and add `playback_format_query_returns_the_loaded_players_format` and `playback_format_query_reports_none_when_nothing_is_loaded`; add `rodio_output_asks_the_worker_for_its_format`; extend `silent_output_reports_no_playback_position` (or a sibling) to assert `playback_format().is_none()`, and extend `rodio_output_reports_a_closed_worker_for_every_command` with a `None` format answer. `src/test_support.rs`: have `RecordingAudio::playback_format` return `Some(PlaybackFormat { sample_rate_hz: 44_100, channels: 2 })`; `src/apple_music/tests.rs`: add `playback_format_forwards_the_audio_output_format`. `src/ui/views/tests.rs`: import the new helpers and cover `format_playback_status_formats_khz_and_channel_mode` (`44_100`/2 → `44 KHZ  STEREO`, `44_100`/1 → `44 KHZ  MONO`, `48_000`/2 → `48 KHZ  STEREO`) and `now_playing_status_row_draws_the_format_or_nothing` by laying out the row with the existing null renderer and reading its text; update the three `view_now_playing` calls in `now_playing_bar_and_browse_action_buttons_construct` with the new argument. `src/ui/tests.rs`: add `playback_format_reply_stores_the_loaded_previews_format`, `playback_format_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`, `stop_clears_the_playback_format_while_pause_keeps_it`, and `position_tick_queries_position_and_format` (drive the batched task's stream to its end and assert both reply messages arrive).
- `README.md`: in the Usage sentence that describes the Now Playing bar, say a loaded preview's sample rate and mono/stereo mode appear in a small status row beneath the title when the backend reports them. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the `## Now Playing label` section, add the status-row bullet (format shown only while a preview is loaded, `44 KHZ  STEREO`/`MONO` from the backend's own source, blank when absent, kept across Pause, cleared by Stop/new selection, stale/pre-transition replies dropped, bitrate not shown) and name `PlaybackFormat`, `format_for_source`, `views::format_playback_status`/`now_playing_status_row`, and the `PlaybackFormat` handler plus `reset_position_readout` as the enforcers. Add the new test names to the Pinned-by list.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/views/now_playing.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `format_playback_status` renders `44 KHZ  STEREO` for a 44.1 kHz stereo format, `44 KHZ  MONO` for one channel, and `48 KHZ  STEREO` for 48 kHz.
- The LCD draws that text in a small green row beneath the title only while the backend reports a format; with no reported format the row is blank and keeps the bar's height.
- Stop and a new selection clear the status text immediately; Pause keeps it; a reply stamped with an older epoch or landing while a transport transition is in flight is dropped.
- `position_tick` asks the backend for both the position and the format, and asks for neither while a transport transition is in flight.
- No bitrate is displayed; the README and `docs/invariants.md` say the row carries sample rate and channel mode only, and `make check` passes (fmt, clippy, rustdoc, tests).

### Toggle the Now Playing elapsed readout between elapsed and remaining time (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Make the large left-hand LCD readout behave like Winamp's: clicking it switches from the elapsed time (`00:53`) to the time remaining (`-02:37`), and clicking again returns to elapsed. The mode is presentation-only, survives Pause, Stop, Next/Previous, and a new selection until clicked again, and changes nothing about playback, the right-edge total, or the marquee. This is independent of the queued Sort plan: it touches the Now Playing machinery, not the browse lists.

**Approach.**

- `src/ui/views/now_playing.rs`:
  - Add `pub enum TimeDisplay { Elapsed, Remaining }` (derive `Debug, Clone, Copy, PartialEq, Eq, Default`, with `Elapsed` as `#[default]`) and a `pub fn toggled(self) -> Self` beside it (public so `handlers::playback` can flip it; `pub(super)` would be confined to `views`). Re-export `TimeDisplay` from `src/ui/views/mod.rs` like `KnownTrack`.
  - Add `duration_ms: u64` to `KnownTrack` and document it as the raw length the remaining readout subtracts from (the existing `time` field stays the formatted total for the right edge). `src/ui/handlers/playback.rs`'s `track_selected` records `duration_ms: song.duration_ms` in the same entry it already builds.
  - Add `pub fn format_lcd_remaining(remaining_ms: u64) -> String`, formatting the shared core of `format_lcd_elapsed` with a leading `-`: `0` → `"-00:00"`, `157_000` → `"-02:37"`, `3_600_000` → `"-60:00"`.
  - Change `now_playing_elapsed` to take the display mode as its last argument. Unknown/absent track still returns borrowed `"--:--"`. In `TimeDisplay::Elapsed` keep today's behavior exactly (`00:00` with no reported position). In `TimeDisplay::Remaining`, return borrowed `"--:--"` when the known track's `duration_ms` is `0` (no length to count down from), otherwise `Cow::Owned(format_lcd_remaining(duration_ms.saturating_sub(position_ms)))`, treating a missing position as zero (so a stopped track shows its full length remaining). Position milliseconds come from the existing `duration_millis` helper. Document the mode's persistence and the saturating subtraction.
  - Wrap the elapsed fragment in `now_playing_row` in `iced::widget::MouseArea::new(lcd_elapsed_text(elapsed)).on_press(Message::ToggleTimeDisplay).interaction(iced::mouse::Interaction::Pointer)`, keeping it the row's first child so the existing captured-text tests still see it first. The helper's existing `text::Renderer` and theme bounds already satisfy `MouseArea`, and `view_now_playing` keeps its signature.
- `src/ui/mod.rs`:
  - Add `Message::ToggleTimeDisplay` next to `PlaybackPosition` with a doc comment saying it only flips the LCD's time mode, and dispatch it to `handlers::playback::toggle_time_display` beside the position arms (not among the browse arms, so the Sort plan's message edits stay separate).
  - Add `time_display: TimeDisplay` to `WinampPlayer`, initialized `TimeDisplay::Elapsed` in `new` (or via `Default`), documented as presentation-only and preserved across tracks. Pass it to `views::now_playing_elapsed` in `WinampPlayer::now_playing_elapsed`; the accessor keeps its one-argument signature.
- `src/ui/handlers/playback.rs`: add `pub(in crate::ui) fn toggle_time_display(player: &mut WinampPlayer) -> Task<Message>` that assigns `player.time_display = player.time_display.toggled()` and returns `Task::none()`. It touches no playback state and clears no position.
- Tests. `src/ui/views/tests.rs`: give the `known_tracks` helper `duration_ms: 0`; update its existing `now_playing_elapsed` calls with `TimeDisplay::Elapsed`; add `format_lcd_remaining_prefixes_a_minus_and_pads` and `now_playing_elapsed_remaining_counts_down_from_the_total` (known 210_000 ms track with a 53 s position → `-02:37`, no position → `-03:30`, `duration_ms` 0 → `--:--`, unknown id → `--:--`, a position past the total → `-00:00`). `src/ui/tests.rs`: add `duration_ms` to the two hand-built `KnownTrack` literals, then add `toggle_time_display_switches_to_remaining_and_persists` beside the elapsed tests: play song 1 (210 s), set `playback_position` to 53 s, send `Message::ToggleTimeDisplay`, assert `-02:37` and an unchanged right-edge total; send `Message::Pause` and assert the remaining value survives; select song 2 and assert the new track counts down from its own total (`-02:05`); toggle back and assert elapsed mode again (`00:00` after the selection). Add `toggle_time_display_with_no_track_keeps_the_placeholder` (a fresh player toggles and still reads `--:--`).
- `README.md`: in the Usage paragraph describing the Now Playing bar, say clicking the large elapsed readout switches it to the track's remaining time (`-02:37`) and back, and that the choice stays until clicked again. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: extend the `## Now Playing label` split-readout bullet with the toggle: the left readout shows elapsed by default, clicking it shows `-MM:SS` remaining (saturating at `-00:00`, `--:--` when the track's length is unknown), the right edge keeps the plain total in both modes, and the mode persists across Pause, Stop, Next/Previous, and a new selection. Name `TimeDisplay`, `views::now_playing_elapsed`, and `format_lcd_remaining` as the enforcers, and add the new tests to the Pinned-by list.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/mod.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The large left readout still shows elapsed `MM:SS` by default, unchanged for every existing position case.
- Pressing the readout flips it to `-MM:SS` remaining and pressing it again flips back; no other control, playback call, track selection, marquee offset, or right-edge total changes.
- Remaining is `total - position`, saturating at `-00:00`; a known track with `duration_ms == 0` and any unknown/absent track read `--:--`.
- The chosen mode survives Pause, Stop, Next/Previous, and selecting another track, and applies to that track's own length.
- The README and `docs/invariants.md` describe the toggle in the same change, and `make check` passes (fmt, clippy, rustdoc, tests).

### Sort the current browse list alphabetically from a Sort button (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Add the library window's `SORT LIB` button from `docs/screenshots/player.png` as a title-case `Sort` button above every browse level, beside `Top`. Pressing it reorders the list currently showing (Artists, Albums, Songs, All Songs, All Albums, or Playlists) alphabetically by each row's rendered text, case-insensitively, leaving every other list and the player state alone. The sort is one-shot like the classic button: it changes only the loaded list in place, so reopening the level fetches its library order again. No persistent sort mode or new dependency is needed.

**Approach.**

- `src/ui/browse.rs`: add `pub(super) fn sort_rows_by(&mut self, key: impl Fn(&T) -> String)` on `BrowseList<T>`, requiring `T: Clone` (the library models derive it). It builds an index permutation (`0..items.len()`) and stable-sorts it by each row's key lowercased with `to_lowercase()`; Rust's stable sort keeps library order for keys that compare equal. Rebuild `items` and, when they are non-empty, `titles` and `labels` by the same permutation so the cached song text stays index-aligned with the reordered songs (the artist/album/playlist lists leave both caches empty). After reordering, bump `epoch` with the same `wrapping_add(1)` `replace_rows` uses, since the row indices a press carries have changed; do not touch `loading`, `error`, or the request generation. A list with fewer than two rows is a no-op (no reorder, no epoch bump), so pressing Sort while a level is loading, empty, or failed changes nothing.
- `src/ui/handlers/browse.rs`: add `pub(in crate::ui) fn sort_browse(player: &mut WinampPlayer) -> Task<Message>`, matching `player.current_view` and calling `sort_rows_by` on that view's list: artists and playlists by `name`, albums (both the per-artist and All Albums lists) by `Album::display_title` (so `artist - title` is the sort key), and songs (both the Songs list, including search results, and All Songs) by `Song::display_title`. Return `Task::none()`; the sort changes no other state, and a reload after navigation restores the fetch's order.
- `src/ui/mod.rs`: add `SortBrowse` to `Message` with a doc comment naming the one-shot reorder and its epoch bump, route it in the `update` dispatch table to `handlers::browse::sort_browse`, and push `views::view_sort_button()` immediately after `views::view_top_button()` so it is present above every browse level.
- `src/ui/views/browse.rs`: add `pub fn view_sort_button() -> Element<'static, Message>` using `labeled_button("Sort", Message::SortBrowse)` beside `view_top_button`, with a doc comment saying it reorders the visible list on press. In `src/ui/views/mod.rs`, export `view_sort_button` and add it to the `labeled_button` doc comment's list of browse buttons.
- Tests in `src/ui/tests.rs` (the browse state and handler tests live there, not in `src/ui/browse.rs`):
  - `sort_reorders_artists_case_insensitively`: drive `Message::ArtistsLoaded` with local `Artist` rows named `"banana"`, `"Apple"`, `"cherry"`, press `Message::SortBrowse`, and assert the names read `Apple`, `banana`, `cherry` — the lowercase key proves the comparison ignores case rather than sorting uppercase first.
  - `sort_reorders_albums_by_their_display_title` and `sort_reorders_playlists_by_name`: load local rows out of order, sort, and assert the rendered display order.
  - `sort_reorders_songs_and_keeps_their_cached_text_aligned`: drive `Message::SongsLoaded(stepping_songs())` (library order One, Two, Three), sort, and assert the ids order One, Three, Two while `songs.titles` and `songs.labels` reorder with them (`"The Sample Band - One"`/`"3:30"`, `"The Sample Band - Three"`/`"1:35"`, `"The Sample Band - Two"`/`"2:05"` in the new order), so no row shows another track's length. Repeat the same assertion for `Message::AllSongsLoaded` over the `all_songs` list.
  - `sort_reorders_all_albums_by_their_display_title`: load local `Album` rows into `all_albums`, sort, and assert their `artist - title` order. This covers the sixth `CurrentView` list, so every browse level has a sort test.
  - `sort_bumps_the_epoch_so_a_pre_sort_press_is_rejected`: capture the artists list's epoch before sorting, sort, then drive `Message::ArtistSelected { epoch: old_epoch, index: 0 }` and assert `player.current_view` is still `CurrentView::Artists`; drive the same press with the new epoch and index and assert it switches to `CurrentView::Albums`. This pins that row indices and the epoch move together.
  - `sort_with_an_empty_or_loading_list_is_a_noop`: press `Message::SortBrowse` on each view before any rows load and assert no panic, no epoch change, and the view stays where it was.
- `README.md`: add to the Usage paragraph that describes the browse lists that a `Sort` button above every list reorders the showing list alphabetically by its row text, case-insensitively, and that reopening the list restores library order. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Browse row order` section after `## Browse reply freshness` stating that Sort reorders only the visible list by its rendered text, keeps equal keys in library order, reorders the cached song titles and labels with the songs, bumps the epoch so a pre-sort press is rejected, and leaves the other lists and playback state unchanged; name `BrowseList::sort_rows_by` and `browse::sort_browse` as the enforcers and cite the new tests.

**Files touched.** `src/ui/browse.rs`, `src/ui/handlers/browse.rs`, `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A `Sort` button is drawn above every browse level beside `Top`; pressing it leaves the current view, current track, playback, search state, and every other list unchanged.
- The showing list is reordered ascending by its rows' rendered text, compared case-insensitively, for Artists, Albums, Songs, All Songs, All Albums, and Playlists; equal keys keep their library order.
- Song lists reorder `items`, `titles`, and `labels` by the same permutation, so each row still shows its own title and duration after a sort.
- A sort bumps the current list's epoch: a `TrackSelected`/`ArtistSelected`/`AlbumSelected`/`PlaylistSelected` press carrying the pre-sort epoch is ignored, while the post-sort indices resolve to the sorted rows.
- Pressing Sort on an empty, loading, or failed list is a no-op that neither reorders rows nor bumps the epoch.
- Navigating away and back reloads library order; no sort flag persists between fetches.
- `make check` passes (fmt, clippy, rustdoc, tests), and the README Usage text and the new `docs/invariants.md` section describe the sort in the same change.

### Add the classic elapsed readout to the Now Playing LCD and restore the plain total at its right edge (planned 2026-10-23 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Split the LCD well the way `docs/screenshots/player.png` draws it: a large left-hand elapsed-time readout (`00:53`, zero-padded minutes and seconds) and the plain total length at the right edge. The landed playback-position series already supplies the position, but it currently renders `<elapsed>/<total>` in the right-edge slot — a value the mockup shows nowhere. Move that elapsed value into the big left readout and put the right edge back to the plain total, settling the drift recorded in the steward's 2026-10-22 note. The `O C I D V` strip, the visualizer, and the `256 KBPS 44 KHZ` / `MONO STEREO` status row stay out of scope, as do artist-row album counts.

**Approach.**

- `src/ui/views/now_playing.rs`:
  - Replace `format_position_time` and `now_playing_progress` with `format_lcd_elapsed(position_ms: u64) -> String`, which zero-pads both fields: `0` → `"00:00"`, `53_000` → `"00:53"`, `192_000` → `"03:12"`. Minutes stay unbounded, so an hour reads `"60:00"`. Delete the two replaced helpers and their tests rather than leaving dead public functions.
  - Add `now_playing_elapsed<'a>(tracks, current_track, position: Option<Duration>) -> Cow<'a, str>` beside [`now_playing_time`]: `"--:--"` when `known_track_for` finds nothing for `current_track`; `"00:00"` when the track is known but the backend reports no position (no preview, a failed start, or the silent backend); `Cow::Owned(format_lcd_elapsed(duration_millis(position)))` when a position is reported.
  - Change `now_playing_time` to drop its `position` parameter and always return the cached total (`KnownTrack::time`, borrowed) or `"--:--"` for an unknown/absent track. Update its doc comment to say the right edge is the plain total.
  - Add `LCD_ELAPSED_SIZE` (34) and an `lcd_elapsed_text` helper beside `lcd_text`, using the same `theme::LCD_GREEN`. Change `now_playing_row` to take `elapsed: Cow<'a, str>` as its first argument and push `lcd_elapsed_text(elapsed)` followed by a fixed-width `Space` (12 px) before the existing title line. Reduce `MARQUEE_WIDTH` from 28 to 22 and update its doc comment so the scrolling title line still fits the well beside the new readout; the `display_len`/`needs_marquee` tests key off the constant, so adjust only the comments that hard-code 28.
- `src/ui/mod.rs`: add `WinampPlayer::now_playing_elapsed` borrowing from `views::now_playing_elapsed`; change `WinampPlayer::now_playing_time` to call the two-argument view helper. In `view`, add `now_playing_elapsed` to the state-lock destructuring tuple and pass it as the new first argument to `views::view_now_playing`; update the `now_playing_time` doc comment there.
- Tests. `src/ui/views/tests.rs`: update the `captured_now_playing_texts` helper to take an elapsed string and assert the readout's position in the captured fragments; add `format_lcd_elapsed_pads_minutes_and_seconds` and `now_playing_elapsed_reads_placeholder_zero_or_position` (unknown id → `"--:--"`, known track with `None` → `"00:00"`, `Some(53s)` → `"00:53"`); add `now_playing_time_returns_the_plain_total_even_with_a_position` (the helper no longer takes a position). `src/ui/tests.rs`: drop the position argument from every `player.now_playing_time(...)` call; repoint the elapsed assertions at `player.now_playing_elapsed(...)` (known position → `"00:53"`-style, pause keeps the frozen value, Stop and a new selection read `"00:00"`, stale-epoch replies are still dropped); keep the epoch/poll tests unchanged. Update `now_playing_bar_and_browse_action_buttons_construct` for the new `view_now_playing` arity.
- `README.md`: in the Usage paragraph that describes the Now Playing bar, say the big readout at the left shows the loaded preview's elapsed time and the length at the right is the plain total; remove the elapsed/total sentence. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, replace the `<elapsed>/<total>` guarantee with the split one (left readout `MM:SS`, plain total at the right, `00:00` for a known track without a position, `--:--` with no current track), name `views::now_playing_elapsed`/`format_lcd_elapsed`/`now_playing_time` and the `PlaybackPosition` handler as the enforcers, and cite the new tests plus the existing position-epoch tests.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/mod.rs`, `src/ui/mod.rs`, `src/ui/timers.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`. (`src/ui/views/mod.rs` re-exports the new helper; `src/ui/timers.rs` had a comment calling the old combined readout elapsed/total.)

**Acceptance criteria.**

- The LCD row's first drawn text is the elapsed readout: `00:53` for a 53-second position, `03:12` for 192 seconds, `00:00` for a known track with no reported position, and `--:--` when no track or an unknown id is current.
- The right edge shows the cached plain total (`3:30`) for a known track regardless of any reported position, and `--:--` for no/unknown track; no code path renders `<elapsed>/<total>` and `now_playing_progress`/`format_position_time` no longer exist.
- Pausing keeps the frozen elapsed readout, Stop and a new selection reset it to `00:00`, and a `PlaybackPosition` reply stamped with an older epoch is still dropped (the existing position tests pass unchanged apart from the elapsed accessor).
- `make check` passes (fmt, clippy, rustdoc, tests).
- The README Usage paragraph and the `docs/invariants.md` Now Playing guarantee describe the split readout in the same change.

### Retire the `unclear-invariant` validation gap by pinning subsystem contracts in docs/invariants.md (planned 2026-10-23 by the steward role) — done 2026-10-23 by the feature role

**Goal.** Three Fixed entries (the stale browse reply store-time invariant, the bidirectional browse contract, and the Now Playing title contract) each reconstructed a guarantee that nothing had written down, recorded as `gap: unclear-invariant`. Promote that recurring tag into infrastructure: make `docs/invariants.md` the single, discoverable home for every subsystem contract that a bug has had to reconstruct, so the next bug in those areas finds the invariant instead of rebuilding it, and so the tag stops recurring.

**Approach.**

- Audit `docs/invariants.md` against the three Fixed entries cited below and ensure each reconstructed guarantee is stated as a named, test-cited section (Browse hierarchy, Now Playing label, stale reply store-time window), adding any that are missing.
- Add a short "How to use" note at the top of `docs/invariants.md` pointing readers (and the bugfix role) at it before reconstructing an invariant, and cross-reference it from the BUGS.md header's Validation gap description.
- No code change; this is a docs-only plan whose acceptance is `make check` plus the three cited guarantees each having a section.

**Cited entries.** `BUGS-FIXED.md`: "A browse reply is stored after a newer request was issued …" (unclear-invariant, store-time invariant), "Now Playing bar shows the track id instead of the song title" (unclear-invariant, Now Playing contract), and the bidirectional-browse entry (unclear-invariant, browse hierarchy).

**Files touched.** `docs/invariants.md`, `BUGS.md`.

**Acceptance criteria.**

- `make check` passes.
- Every `unclear-invariant` Fixed entry cited above has a matching section in `docs/invariants.md` naming the enforcing code and the tests that fail if it is removed.
- The BUGS.md header cross-references `docs/invariants.md` for the `unclear-invariant` tag.

### Show each album row's artist beside its title (planned 2026-10-21 by the plan role) — done 2026-10-23 by the feature role

**Goal.** Every album row reads `<artist> - <title>` when the album carries an artist and just the title when it does not, so the per-artist Albums view and the flat All Albums view match the library mockup's album rows (`docs/screenshots/library.png`). The secondary `View Songs` hint, row numbering, the playing-row marker, selection messages, and ordering stay exactly as they are. This is independent of the playback-position series: it touches the album model, the REST album mapping, and browse rows, not the audio seam or the Now Playing bar. Album row running times are out of scope because an album query does not supply its songs' durations.

**Approach.**

- `src/library.rs`: add `pub artist: String` to `Album`, with `#[serde(default)]` and a doc comment saying `""` means the source supplied no artist (the same sentinel as `Song::artist`). Add `pub fn display_title(&self) -> Cow<'_, str>` mirroring `Song::display_title`: borrow the title when the artist is empty, otherwise return an owned `"<artist> - <title>"`. In the tests, add `"artist": "The Sample Band"` to `album_payload` and to the payload in `album_deserialization_ignores_unknown_fields` (which compares against `sample_album`); add an `album_required_fields_payload` that omits the optional artist and point `album_deserialization_rejects_missing_required_fields` at it; add `album_deserialization_defaults_a_missing_artist_to_empty`; and add `album_display_title_joins_the_artist_and_title` plus `album_display_title_borrows_the_title_when_the_artist_is_empty`, mirroring the two `display_title` song tests.
- `src/test_support.rs`: give `sample_album`'s new field `artist: "The Sample Band"`.
- `src/sample_library.rs`: fill the three sample `Album` literals with their owner's name — `"The Sample Band"` for `album-1` and `album-2`, `"Echo Chamber"` for `album-3`.
- `src/apple_music/rest.rs`: in `Resource::into_album`, take `attributes.artist_name` and store it in `Album.artist`, filtering out a blank or whitespace-only name to `""` exactly as `Resource::into_song` does; document that a resource without an artist name maps to the empty sentinel. Both `get_albums_by_artist` and `get_all_albums` already map through this function, so no query changes are needed.
- `src/apple_music/rest/tests.rs`: add `"artistName":"The Sample Band"` to the album stubs in `albums_by_artist_map_library_json_and_set_the_artist_id` and `get_all_albums_maps_the_library_albums_collection`, and update their expected `Album` values (including the one spelled out in `get_all_albums_maps_the_library_albums_collection`) with the mapped artist. Add `get_all_albums_leaves_a_missing_album_artist_empty`, using a stub whose album attributes omit `artistName`, and assert `artist` is `""`.
- `src/apple_music/tests.rs`: update the two signed-in browse expectations that construct an `Album` in `get_albums_by_artist_uses_the_rest_library_when_signed_in` and `get_all_albums_uses_the_rest_library_when_signed_in`; supply `artistName` in their stubs and assert the mapped names so the service-level path is pinned too.
- `src/ui/tests.rs`: add the new field to the `flat_album` literal in `album_selected_in_all_albums_loads_its_songs_and_back_returns_to_all_albums` and the `album-3` literal in `album_selected_fetches_the_albums_songs_into_the_player` (use the sample library's `"Echo Chamber"`); other `Album` values come from `sample_album`.
- `src/ui/views/browse.rs`: change `album_row`'s title from `Cow::Borrowed(album.title.as_str())` to `album.display_title()`, and update its doc to say an album with an artist reads `<artist> - <title>` while an artist-less one reads the title alone.
- `src/ui/views/tests.rs`: update `album_row_uses_title_and_selects_the_album_by_index` to expect `"The Sample Band - First Record"`; add `album_row_shows_the_title_alone_when_the_artist_is_empty`, blanking `sample_album().artist` and asserting the title is the bare `"First Record"` and borrows.
- `README.md`: extend the Usage sentence that describes browse rows so it also says album rows read `<artist> - <title>`, falling back to the title when the source supplies no artist. Keep the diff to the Usage paragraph; do not touch the `<!-- tumwater:status:start -->` block.
- Completion: move this entry from `PLANS.md` `## Planned` to `PLANS-DONE.md` `## Done` with a `done <landing date>` stamp.

**Files touched.** `src/library.rs`, `src/test_support.rs`, `src/sample_library.rs`, `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music/tests.rs`, `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, plus the `PLANS.md`/`PLANS-DONE.md` move on completion.

**Acceptance criteria.**

- `make check` passes.
- `Album::display_title` returns `"The Sample Band - First Record"` for the sample album, borrows the title alone for an album whose artist is empty, and never emits a dangling `" - "` prefix.
- `album_row` returns the combined artist/title for both the per-artist Albums list and the All Albums list, since both build through `album_row`, and returns the bare title when the artist is empty; the secondary label and selection message are unchanged.
- `Resource::into_album` maps a non-blank `artistName` into `Album.artist`, and a missing or blank one leaves `""`; the two signed-in browse tests and the two REST mapping tests show the name reaching the album returned to the UI.
- `Album` round-trips with the new field, still tolerates unknown fields, and its missing-required-field probe still rejects a payload without an id, title, or artist id while a missing artist defaults to `""`.
- The README change is confined to the Usage paragraph, and the status block is byte-for-byte unchanged.
- The implementation changes only the ten listed files plus the `PLANS.md`/`PLANS-DONE.md` move, and the commit body names them consistently with `git diff --stat`.

### Add the playback-position readout to the Now Playing bar (Playback position, part 3/3: planned 2026-10-21 by the plan role, requires part 2/3 landed) — done 2026-10-22 by the feature role

**Goal.** The Now Playing bar's right-hand readout shows the current track's elapsed and total time as `"<elapsed>/<total>"` (for example `0:45/3:12`) whenever the audio backend reports a loaded preview's position, and falls back to the existing plain total (`"--:--"` when no track is current) when it does not. Pause leaves the stored value and the paused player's position in place, a seek is reflected by the next position reply, Stop and a new selection clear the readout, and a play whose preview fails to install leaves the plain total rather than a frozen `0:00`.

This is the second half of the former part 2/2 and consumes part 2/3's authoritative `AudioOutput::position`: the backend answers `None` when no preview is loaded, so the UI keeps no playback guard and a selected track with no preview URL never turns one on.

**Approach.**

- `src/apple_music/playback.rs`: update `playback_position`'s doc to say it reports the loaded preview's position and `None` when no preview is loaded or the backend cannot answer, and remove its `#[allow(dead_code)]` now that the UI calls it. `play_track` also stops the backend's previously loaded preview when the selected song has no preview URL, so the old track neither keeps sounding nor keeps answering the position query under the new track's name; `transport.loaded` then records that no preview started.
- `src/apple_music/tests.rs`: replace `play_track_without_a_preview_url_commits_state_without_audio` with `play_track_without_a_preview_url_stops_the_previous_preview`, which plays a preview first and then selects a song without one, and pins the backend call sequence (`Stop`) and the committed state; `pause_pauses_the_audio_backend` now expects that `Stop` before its `Pause`.
- `src/ui/views/now_playing.rs`: add `pub fn format_position_time(position_ms: u64) -> String`, calling the existing `format_minutes_seconds` (which already yields `"0:00"` for zero and keeps minutes unbounded). Add `pub fn now_playing_progress(position_ms: u64, total: &str) -> String` returning `format!("{}/{}", format_position_time(position_ms), total)`. Change `now_playing_time` to take `position: Option<Duration>`: when `current_track` is in `tracks` and `position` is `Some`, return `Cow::Owned(now_playing_progress(...))` with the duration converted saturating to milliseconds; otherwise keep today's borrowed `KnownTrack::time` or `"--:--"` fallback. Update its doc comment to say the combined case allocates.
- `src/ui/mod.rs`: add `playback_position: Option<Duration>` and `position_epoch: u64` to `WinampPlayer`, initialized `None`/`0` in `new`, and define `POSITION_POLL_INTERVAL` (`Duration::from_secs(1)`) with a doc note that it bounds the readout refresh, not the audio. Add `Message::PositionTick` and `Message::PlaybackPosition { epoch: u64, position: Option<Duration> }`, and route both from the `update` dispatch table to the two handlers in `src/ui/handlers/playback.rs` below. This app does not enable iced's timed-subscription backend (see the `marquee_ticks` note), so add `position_ticks`/`position_tick_stream` beside it, feeding a one-slot channel from a std thread once per `POSITION_POLL_INTERVAL`, and batch that subscription in `init_ui` with `keyboard::listen().filter_map(message_for)` and `marquee_ticks()`. `WinampPlayer::now_playing_time` keeps its signature but passes `self.playback_position` to `views::now_playing_time`; the `view` call site is unchanged, though its comment that all three readouts always borrow must note the combined elapsed case allocates.
- `src/ui/handlers/playback.rs`: the two handlers the table routes to — `position_tick` and `playback_position` — plus the readout reset and the transition counter. `position_tick` captures `player.position_epoch`, clones the service, and returns `Task::perform(async move { service.playback_position() }, move |position| Message::PlaybackPosition { epoch, position })`; `playback_position` stores `position` (including `None`) only when `epoch == player.position_epoch` and no transport transition is outstanding, dropping a reply issued before a transition. Add `fn reset_position_readout(player: &mut WinampPlayer)` that bumps `position_epoch` and sets `playback_position = None`; call it after the song resolves in `track_selected` and at the start of `stop` (which becomes a block returning the existing `transport_into` call). `pending_position_transitions: u64` on `WinampPlayer` counts the transitions in flight: every handler that schedules a backend transport (Play, Pause, Play/Play-Pause, Stop, and a selected track's play) begins one, and the completion that reports that same transition (`TransportSettled` for the transports, `TrackPlayed` for the play) ends it, so the count is nonzero exactly while some transition is in flight. While the count is nonzero the poll issues no query and no reply is stored, so a poll cannot query the previous player under the new epoch and a completion belonging to another transition cannot close that window early. Pause leaves the field alone, so the next poll keeps showing the paused player's frozen position.
- `src/ui/views/tests.rs`: update the `now_playing_time_resolves_known_track_to_cached_label` call sites to pass `None` as the position; extend it (or add `now_playing_time_shows_elapsed_over_total_when_a_position_is_known`) so `now_playing_time(&tracks, Some("song-1"), Some(Duration::from_secs(192)))` reads `"3:12/3:30"` and is `Cow::Owned`; add `format_position_time_formats_elapsed_and_zero` (`0`→`"0:00"`, `192_000`→`"3:12"`, `65_000`→`"1:05"`) and `now_playing_progress_joins_elapsed_and_total` (`(192_000, "12:38")`→`"3:12/12:38"`).
- `src/ui/tests.rs`: keep `now_playing_time_shows_the_played_tracks_duration_after_browsing_to_another_album` asserting the plain total (its calls now use the unchanged `WinampPlayer::now_playing_time` signature); add `now_playing_time_shows_elapsed_over_total_while_a_position_is_known` (reuse `play_song_1_then_browse_to_album_2`, set `player.playback_position = Some(Duration::from_secs(45))`, assert `"0:45/3:30"`), `pause_keeps_the_stored_elapsed_readout` (set a known position, drive `Message::Pause`, then assert the field still holds that position and the bar still reads `"0:45/3:30"`; together with part 2/3's `a_pause_keeps_the_loaded_player_reporting_its_position` this pins that pause freezes rather than clears), `a_position_reply_after_a_seek_updates_the_readout` (set `Some(45)`, drive `Message::SeekForward`, feed `Message::PlaybackPosition` with `Some(50)`, assert `"0:50/3:30"`; the existing `seek_messages_drive_the_audio_backend` pins the backend call), `track_selected_clears_the_elapsed_readout`, `stop_clears_the_elapsed_readout`, `position_replies_store_and_stale_ones_are_dropped` (store a reply at the current epoch, settle the Stop's transition, then drive a reply stamped with the pre-`Stop` epoch and assert it does not overwrite), `position_poll_waits_for_a_stop_to_reach_the_backend` and `position_poll_waits_for_a_selected_tracks_play_to_settle` (the poll issues no query while the reset's transition is outstanding and resumes once its completion lands), `an_unrelated_transport_completion_keeps_the_poll_waiting` (a Pause that settles while a Stop is outstanding keeps the poll waiting), and `position_tick_schedules_a_query` (`assert_message_schedules_work`); the shared `play_song_1_then_browse_to_album_2` fixture now delivers the selection's `TrackPlayed` completion so the transition counter settles.
- `README.md`: extend only the Usage sentence that describes the Now Playing bar's artist and title so it says the length reads as elapsed/total while a preview is loaded and as the plain length otherwise. Do not edit the `<!-- tumwater:status:start -->` block or add archive narrative anywhere; that block is state-only (see `762af8c`), and the README diff must be confined to the Usage paragraph.
- `docs/invariants.md`: in the Now Playing label section, add the time-readout guarantee (elapsed/total only while a loaded preview position is known, the cached total otherwise, `--:--` with no current track), its enforcement by `views::now_playing_time`/`now_playing_progress` and the `PlaybackPosition` handler, and cite the new tests plus part 2/3's audio-seam `position_query_reports_none_when_nothing_is_loaded`, `a_seek_is_reflected_by_the_next_position_query`, and `a_pause_keeps_the_loaded_player_reporting_its_position`.

**Files touched.** `src/apple_music/playback.rs`, `src/apple_music/tests.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes.
- `format_position_time` returns `"0:00"`, `"3:12"`, and `"1:05"` for zero, 192000 ms, and 65000 ms; `now_playing_progress(192_000, "12:38")` returns `"3:12/12:38"`.
- With a known current track and a `Some` position, the bar renders `elapsed/total`; with `None`, the plain total; with no current track or an unknown id, `"--:--"`.
- Pause does not clear the stored position and the next poll still reads the paused player, so the readout keeps the frozen value; `a_position_reply_after_a_seek_updates_the_readout` shows a seek's new position reaching the bar on the next reply; Stop and a new selection clear the readout immediately through `reset_position_readout`. A failed preview start leaves `None`, so the bar falls back to the plain total.
- A `PlaybackPosition` reply stamped with an older `position_epoch` is dropped, so a poll in flight across a Stop or selection cannot restore the previous elapsed value.
- The poll issues no query while a backend transport transition is in flight, and only the completion that reports that transition releases it, so a poll cannot query the previous player under the new epoch.
- A selection whose song has no preview URL stops the backend's previous preview, so the position query answers `None` for the new track and the bar falls back to the plain total.
- The poll is an iced subscription emitting `PositionTick` once per `POSITION_POLL_INTERVAL` (via the app's std-thread tick source, since `iced::time::every` is unavailable), and the position query runs in a task, not in `update`.
- The README diff is confined to the Usage sentence; the status block is byte-for-byte unchanged.
- The change touches only the nine listed files plus the `PLANS.md` move.

### Split the `update` loop into one handler module per concern (landed 2026-10-22; no plan-role entry — rescued manually from a polluted main)

**Goal.** `src/ui/mod.rs` had grown to 1319 lines, with `update` holding every message's behaviour inline and eight helpers shared across arms. Each concern's behaviour now lives in its own module under `src/ui/handlers/` — `playback` (the transport buttons and keys, the track stepping, the play request and its completion, the marquee tick), `mixer` (the volume, the balance, the seek, and the equalizer), `browse` (the hierarchy, the library search, and the fetch results), and `window` (the resolved window id and the title bar's actions) — and `update` is a dispatch table that routes each `Message` variant to its handler and carries no behaviour of its own.

**Approach.**

- `src/ui/mod.rs`: declare `mod handlers;`, drop the imports the moved code took with it (`EqSnapshot`, the `loading::{...}` items, `Ordering`), delete the inline arms and the helpers they shared (`step_track`, `mutate_state`, `mutate_volume`, `mutate_balance`, `seek_preview`, `mutate_equalizer`, `scroll_browse_to_top`, `fetch_at_top`, `with_window_id`, `resize_to_shade`), and rewrite `update` as one delegation per variant.
- `src/ui/handlers/`: one module per concern, each holding its handlers, its shared helpers, and the rationale the inline comments carried, so a change to one concern never reaches into another's code.
- `src/ui/tests.rs`: take `RequestGeneration` and `play_into` from `loading` directly, since `update` no longer re-exports them.
- `docs/invariants.md`: the Browse hierarchy and Browse reply freshness sections now name the handlers in `src/ui/handlers/browse.rs` rather than arms in `src/ui/mod.rs`.

**Files touched.** `src/ui/mod.rs`, `src/ui/handlers/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/handlers/mixer.rs`, `src/ui/handlers/browse.rs`, `src/ui/handlers/window.rs`, `src/ui/tests.rs`, `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes: formatting, clippy with `-D warnings`, rustdoc, and the test suite.
- Every `Message` variant has exactly one dispatch row, and no row holds behaviour.
- The moved logic has one definition: no helper or arm body survives in `src/ui/mod.rs`.
- The existing UI tests pass unchanged apart from the two `loading` imports `src/ui/tests.rs` now takes directly.

### Add the playback-position readout to the Now Playing bar (Playback position, part 2/3: planned 2026-10-21 by the plan role, requires part 1/2 landed, done 2026-10-21)

**Goal.** The audio seam answers a position only while a preview is loaded: `AudioOutput::position` returns `Some(position)` when a player is installed (including `Some(Duration::ZERO)` at the start of a preview) and `None` when no player is loaded or the backend cannot answer. This is the first half of the former part 2/2, split so the seam contract and its Pause/seek behavior are reviewed apart from the Now Playing readout that consumes them. The silent backend keeps the trait's default `None`, so a machine with no audio device shows no elapsed value rather than a frozen `0:00`.

This revision leaves the seam approach as it stands, since no recorded review objection names the code path, and fixes the scope wording that produced a commit body contradicting its own diff: finishing a plan also moves its entry from `PLANS.md` to `PLANS-DONE.md`, so that move is part of this change and must be reported as such rather than denied in RISK or VERIFIED.

**Approach.**

- `src/audio.rs`: revise `AudioOutput::position`'s contract and doc to "`Some` while a player is loaded (including `Duration::ZERO` at its start), `None` when no player is loaded or the backend cannot answer"; the default stays `None`. `SilentOutput` needs no method because the default already answers `None`.
- `src/audio/player.rs`: change `Command::Position` to carry `Sender<Option<Duration>>` and update its doc to say `None` when no player is loaded. In `serve_commands`, reply with `player.as_ref().map(WorkerPlayer::position)`. In `RodioOutput::position`, return `replies.recv_timeout(POSITION_QUERY_TIMEOUT).ok().flatten()`, so a closed worker or timed-out reply is `None`.
- `src/audio/tests.rs`: update `rodio_output_asks_the_worker_for_its_position` to send `Some(Duration::from_secs(3))`; make `position_query_returns_the_loaded_players_position` expect `Some(Duration::from_secs(42))`; rename `position_query_returns_zero_when_nothing_is_loaded` to `position_query_reports_none_when_nothing_is_loaded`, expect `None`, and update its comment from "answers zero" to "reports `None`". The timeout and closed-worker tests already expect `None`.
- `src/audio/tests.rs`: make `RecordingPlayer::seek_by` record the offset and make its `position` account for the recorded offsets via `seek_target(Duration::from_secs(42), ...)`, so the fake models a player whose clock moves on a seek while its no-seek position stays `42s` for the existing test. Add `a_seek_is_reflected_by_the_next_position_query` (Play, `SeekBy(5)`, `Position` replies `Some(47)`) and `a_pause_keeps_the_loaded_player_reporting_its_position` (Play, `Pause`, `Position` still answers `Some(42)`, the player was paused, and it was not stopped). The pause test pins the worker's half of the guarantee: Pause reaches the player and the worker keeps it installed, so the next poll reads the player's own frozen clock; the fake has no advancing clock, so it cannot itself prove rodio's clock stops.
- `src/audio/tests.rs`: add `a_failed_preview_leaves_no_position_to_report`. Its `create_player` factory, built like `recording_players` with a `Mutex<Option<...>>` so it queues only on its first call and stays `Fn`, sends `Command::Play("second")` and then `Command::Position(reply)` through the test's sender; its `fetch` succeeds for `"first"` and fails for `"second"`. The factory drops its sender clone and the test drops its own before `serve_commands`, so the loop ends after the query. The failed second `Play` drops the player, so `replies.recv()` is `None`.
- Completion: move this entry from `PLANS.md` `## Planned` to `PLANS-DONE.md` `## Done` under its part 2/3 heading with a `done <landing date>` stamp, leaving part 3/3 in `## Planned`.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, plus `PLANS.md` and `PLANS-DONE.md` (this entry moved to Done).

**Acceptance criteria.**

- `make check` passes.
- A loaded player makes `Command::Position` answer `Some(player.position())`; no player answers `None`; a closed worker and a timed-out reply answer `None`; the silent backend answers `None`.
- `a_seek_is_reflected_by_the_next_position_query` shows the fake player's reported position moving by the seek (`42s` to `47s`), and `a_pause_keeps_the_loaded_player_reporting_its_position` shows the paused player still installed and reporting its position.
- A failed asynchronous preview start leaves no player, so the following position query answers `None`.
- The implementation changes exactly `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `PLANS.md`, and `PLANS-DONE.md` (this entry moved to `## Done`), and no other file. The commit body's SUMMARY, RISK, and VERIFIED name all five files and agree with `git diff --stat`; none claims `PLANS-DONE.md` is untouched.

### Scroll an overlong Now Playing title across the LCD (planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** The brief's mockup says "The song title scrolls across the display": when the Now Playing bar's combined `artist - title` line is wider than the LCD, it scrolls one character per tick and wraps through a short gap, so the full track name is readable instead of running under the time readout. A line that fits stays still and keeps the existing zero-allocation rendering. This is independent of the playback-position series: it reads only the label and artist the bar already resolves, and touches neither the audio seam nor the time readout.

**Approach.**

- `src/ui/views/now_playing.rs`: add `pub(super) const MARQUEE_WIDTH: usize = 28` — a documented character budget sized to keep the line inside the roughly 550px-wide window beside the `"Now Playing: "` caption and the time — and `const MARQUEE_GAP: usize = 3` (the spaces between repeats). Add `pub(super) fn marquee_text(text: &str, offset: usize, width: usize) -> String`: count characters (not bytes), return `text.to_string()` unchanged when it fits `width`, otherwise walk a cycle of `text` plus `MARQUEE_GAP` spaces for `width` characters starting at `offset % cycle`, substituting a space past the text's end. The pure helper is called only by the overlong branch, so its allocation belongs to the scrolling path.
- `src/ui/views/now_playing.rs`: change `view_now_playing` to take `marquee_offset: usize` as a fourth argument. Compute the display length as `label.chars().count()` plus `artist.chars().count() + 3` when the artist is non-empty. When it fits `MARQUEE_WIDTH`, keep the current structure — the separate artist, `" - "`, and label widgets, all borrowed, plus the `Space` fill and time. When it does not fit, build `artist - label` (or the label alone when the artist is empty) and render it as one `Text` of `marquee_text(&display, marquee_offset, MARQUEE_WIDTH)` in `theme::LCD_GREEN`, keeping the `Space` fill and time at the right edge. Update the doc comment to describe the scrolling branch and the character-budget approximation.
- `src/ui/mod.rs`: add `marquee_offset: usize` to `WinampPlayer`, initialized `0` in `WinampPlayer::new`. Add `Message::MarqueeTick` with a doc comment saying it advances the title window; its `update` arm sets `player.marquee_offset = player.marquee_offset.wrapping_add(1)` and returns `Task::none()`. In the `TrackSelected` arm, set `player.marquee_offset = 0` beside the `known_tracks` entry, so a new song starts at the beginning of its title. Add `const MARQUEE_TICK: std::time::Duration = std::time::Duration::from_millis(250)` near `init_ui` (import `Duration` from `std::time`). Change `init_ui`'s subscription to `iced::Subscription::batch([...])` of the existing `keyboard::listen().filter_map(message_for)` and `marquee_ticks()`, a helper that feeds ticks from one std thread through the futures channel iced re-exports (iced's `time::every` needs a `smol`/`tokio` backend feature the app does not enable, and its default thread-pool backend's `time` module is empty); `marquee_ticks` is a thin wrapper over `marquee_tick_stream()`, whose one-slot channel drops a tick when the UI is behind but stops the thread only when the receiver is gone, so a slow poll cannot end the scrolling. The timer runs unconditionally, and the view ignores the offset whenever the line fits. Pass `player.marquee_offset` to `views::view_now_playing` in `view`.
- `src/ui/views/tests.rs`: add `marquee_text_keeps_a_line_that_fits` (`marquee_text("Opening", 9, 28) == "Opening"`), `marquee_text_windows_from_the_offset` (`"The Sample Band - Opening"`, width 10: offset 0 is `"The Sample"`, offset 2 is `"e Sample B"`), `marquee_text_wraps_through_the_gap_to_the_start` (offset 25 at width 10 is `"   The Sam"`), and `marquee_text_counts_characters_not_bytes` (`marquee_text("Café — Ok", 0, 5) == "Café "`). Add `overlong_now_playing_line_renders_a_new_window_as_the_offset_advances`: lay the row out with iced's null renderer, capture the text widgets through a widget `Operation`, and assert the overlong line contains the window for offsets 0 and 1 (which differ) while a fitting line captures the same separate artist and title at every offset. Extend `now_playing_bar_and_browse_action_buttons_construct` to pass a `marquee_offset` to both `view_now_playing` calls and add one overlong-label construction that exercises the scrolling branch.
- `src/ui/tests.rs`: add `marquee_tick_advances_the_offset` (two `Message::MarqueeTick` updates take `marquee_offset` from 0 to 2), `marquee_ticks_keep_ticking_when_the_receiver_falls_behind` (queue the two messages the channel can hold, sleep through several more tick periods so the sender attempts a send against the full channel, then read a third tick, proving a full channel drops a tick instead of stopping the timer), and `track_selected_resets_the_marquee_offset` (load `stepping_songs()`, set the offset to 7, drive `TrackSelected { epoch: 1, index: 0 }`, assert 0).
- `README.md`: extend only the Usage sentence that describes the Now Playing bar's artist and title so it also says the line scrolls when it is longer than the display. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the Now Playing label section, add the scrolling guarantee: a combined artist/title longer than `MARQUEE_WIDTH` characters is windowed by `views::now_playing::marquee_text`, advanced one character per `Message::MarqueeTick`, reset to the start by `TrackSelected`, and shown whole-and-borrowed when it fits. Cite the marquee tests in `src/ui/views/tests.rs` and the tick/reset tests in `src/ui/tests.rs`.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`, plus the `PLANS.md`/`PLANS-DONE.md` move.

**Acceptance criteria.**

- `make check` passes.
- `marquee_text` returns the input unchanged for a line that fits; for `"The Sample Band - Opening"` at width 10 it returns `"The Sample"` at offset 0, `"e Sample B"` at offset 2, and `"   The Sam"` at offset 25; `marquee_text("Café — Ok", 0, 5)` is `"Café "`, proving character-count indexing.
- A label plus artist within `MARQUEE_WIDTH` renders through the existing separate artist/title widgets and stays still at every offset; a longer one renders the single windowed `Text` and advances with the offset, while the time stays at the right edge. The row-rendering test captures those text widgets and asserts the different windows.
- `Message::MarqueeTick` advances `marquee_offset` by one with no task, `marquee_ticks` emits one per `MARQUEE_TICK` alongside the unchanged key bindings and keeps ticking after a delayed receiver, and a `TrackSelected` resets the offset to 0 so a new song starts at character 0.
- The README diff is confined to the Usage sentence; the status block is byte-for-byte unchanged.
- The change touches only the six listed source/docs files plus the `PLANS.md`/`PLANS-DONE.md` move, and adds no dependency.

### Show each song row's artist and title in every browse list (planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** Every song row in the Songs view, All Songs, and search results reads `<artist> - <title>` when the song carries an artist (for example `The Sample Band - Opening`), as the library mockup's playlist rows do, and reads just the title when the artist is empty. Row numbering, the playing-row marker, the length label, selection messages, and ordering stay exactly as they are. This is independent of the pending playback-position plan: it touches the browse rows, not the Now Playing bar or the audio seam.

**Approach.**

- `src/library.rs`: add an inherent `Song::display_title(&self) -> Cow<'_, str>` (import `std::borrow::Cow`). It returns `Cow::Borrowed(&self.title)` when `self.artist.is_empty()` and `Cow::Owned(format!("{} - {}", self.artist, self.title))` otherwise. This is presentation-only; the serialized model is unchanged. Add a test asserting `sample_song().display_title()` is `"The Sample Band - Opening"`, that a clone with `artist = String::new()` yields the borrowed `"Opening"`, and that the joined case is owned.
- `src/ui/browse.rs`: add a `titles: Vec<String>` cache to `BrowseList<T>`, documented beside `labels` as the per-row primary title cached when the list is stored so a view refresh does not re-join every artist and title. Rename `store_with_labels` to `store_with_row_text(items, title, label)`, taking two closures: the title closure fills `titles` and the label closure fills `labels`, both in row order. `store` clears both caches; `clear` and `fail` drop both with the rows; `replace_rows` leaves both to the caller. Update the module doc and every existing `store_with_labels` reference in docs/comments to the new name.
- `src/ui/mod.rs`: in the `SongsLoaded` and `AllSongsLoaded` arms call `store_with_row_text` with `|song| song.display_title().into_owned()` for the title and the existing `|song| views::format_track_time(song.duration_ms)` for the length. Pass `&player.songs.titles` / `&player.all_songs.titles` to `views::view_songs` / `views::view_all_songs` in `view`.
- `src/ui/views/browse.rs`: change `scrollable_list`'s item title from `&'a str` to `Cow<'a, str>` so a row can borrow a cached title; wrap the artist, album, and playlist row titles in `Cow::Borrowed`. Change `song_row` to take a `cached_title: Option<&'a str>` beside the existing cached length and return the row title as a `Cow`: `Cow::Borrowed(cached_title)` when the cache has one, otherwise `song.display_title()`. Thread a `titles: &'a [String]` slice through `song_list`, `view_songs`, and `view_all_songs`, pairing each song with `titles.get(index).map(String::as_str)` exactly as the length labels are paired now. Update the affected doc comments, including why the cache exists and that an index without a cached title falls back to composing it.
- `src/ui/views/tests.rs`: update every `song_row` call for the new `cached_title` argument and `view_songs`/`view_all_songs` calls for the `titles` slice. Change `song_row_uses_title_and_selects_the_song_by_index` to expect `"The Sample Band - Opening"` from the fallback, add `song_row_prefers_the_lists_cached_title` (a cached value that differs from the song fields must win), and add `song_row_uses_the_title_alone_when_the_artist_is_empty`. Keep the existing length-cache, unknown-duration, current-marker, and construct-every-view tests passing with the new argument order.
- `src/ui/tests.rs`: extend the song-list cache tests to assert `player.songs.titles` and `player.all_songs.titles` are built in row order from `display_title` (for example `["The Sample Band - One", "The Sample Band - Two", "The Sample Band - Three"]`), and that a navigation's `clear` drops the titles together with the length labels. Rename the cache test so its comment names both caches if needed.
- `README.md`: extend the Usage sentence that says song rows show each track's length so it also says song rows read `artist - title` (or the title alone when the source supplies no artist).
- `docs/invariants.md`: update the browse reply-freshness guarantee's "Enforced by" reference from `store_with_labels` to the renamed `store_with_row_text`; the guarantees themselves do not change.

**Files touched.** `src/library.rs`, `src/ui/browse.rs`, `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes.
- `Song::display_title` returns `"The Sample Band - Opening"` for the sample song and the borrowed title `"Opening"` when the artist is empty.
- `SongsLoaded` and `AllSongsLoaded` fill both `titles` and `labels` in row order; `store`, `clear`, and `fail` drop both caches, so a refresh cannot pair a new row with a stale title or length.
- Every song row renders the cached display title when present and `Song::display_title` otherwise; a cached title wins over the song's own fields. The length label, message payload, epoch, numbering, playing marker, and list order are unchanged.
- Songs, All Songs, and search results all render through the same `song_list`, so each shows `<artist> - <title>` and an artist-less song shows only its title.
- The change touches only the eight listed files plus this backlog move.

### Add the library window's Top button and open every browse list at the top (planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** The browse list gains the mockup's `TOP` control: a `Top` button above the list returns the current list to its first row, and opening a browse level — drill-down, flat-list button, search, or Back — starts that newly shown list at the top instead of leaving it at whatever offset the previous list had. This is independent of the playback-position series: it touches no audio code and no Now Playing state.

**Approach.**

- `src/ui/views/browse.rs`: define `pub const BROWSE_LIST_ID: &str = "nostalgia-browse-list";` with a doc comment naming it as the one scrollable's id so the update loop's scroll operation can reach it. In `scrollable_list`, add `.id(BROWSE_LIST_ID)` to the `Scrollable` before its style, leaving the rows, placeholder, and height unchanged. Add `pub fn view_top_button() -> Element<'static, Message>` returning `labeled_button("Top", Message::ScrollBrowseToTop).into()`.
- `src/ui/views/mod.rs`: re-export `BROWSE_LIST_ID` and `view_top_button` from `browse` beside `view_back_button` and `view_retry_button`.
- `src/ui/mod.rs`: add `Message::ScrollBrowseToTop`. Add a private helper `fn scroll_browse_to_top() -> Task<Message>` returning `iced::widget::operation::snap_to(views::BROWSE_LIST_ID, iced::widget::scrollable::RelativeOffset::START)`. Handle the new message by returning that helper. Add a private helper `fn fetch_at_top(task: Task<Message>) -> Task<Message>` returning `Task::batch([task, scroll_browse_to_top()])`, and wrap the `fetch_level(...)` result in the `ArtistSelected`, `AlbumSelected`, `PlaylistSelected`, `ShowAllSongs`, `ShowAllAlbums`, `ShowPlaylists`, and `SearchSubmitted` arms. In `Message::Back`, return `scroll_browse_to_top()` after the existing view change so stepping up also starts the restored list at the top.
- `src/ui/mod.rs` `view`: push `views::view_top_button()` above the browse content for every `CurrentView` — after the Back/Retry buttons and before the Artists-only flat-list buttons and the search box — so the mockup's `TOP` control is available at each browse level.
- `src/ui/views/tests.rs`: extend `now_playing_bar_and_browse_action_buttons_construct` with `let _top = view_top_button();`.
- `src/ui/tests.rs`: add `scroll_browse_to_top_schedules_a_scroll_task`, driving `Message::ScrollBrowseToTop` through `assert_message_schedules_work` and asserting `assert_view` and the shared state are unchanged; add `back_schedules_the_restored_list_to_the_top`, starting in Albums and asserting `Message::Back` both returns to Artists and schedules follow-up work (the scroll task).
- `README.md`: extend the Usage paragraph that describes the browse buttons with one sentence: a `Top` button above every browse list returns it to the first row, and opening a list starts it at the top.

**Files touched.** `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `Message::ScrollBrowseToTop` schedules an iced `snap_to` task for `views::BROWSE_LIST_ID` at `RelativeOffset::START`, and driving it through `update` changes neither `current_view` nor shared state.
- The `Top` button is built into the browse area for all six `CurrentView` variants and is absent in shade mode, which returns before the browse content is assembled.
- Every navigation arm that replaces the visible list batches the snap-to-top task with its fetch, and `Message::Back` schedules it after stepping up, so a list opened after a navigation starts at its first row.
- Scrolling a list manually still works between navigations; the id is stable while the same list is displayed.
- The change touches only the six listed files plus this `PLANS.md` edit; no audio, `apple_music`, or `docs/invariants.md` changes, because the browse hierarchy guarantees are unchanged.

### Add the playback-position readout to the Now Playing bar (Playback position, part 1/2: planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** The audio seam can report where the loaded preview is, so the Now Playing bar can later show elapsed against total. `AudioOutput::position()` answers `Option<Duration>`: `Some` with the loaded player's position (including `Some(Duration::ZERO)` when the worker has no player), or `None` when the backend cannot answer (the silent fallback, a default test stub, or a query the worker cannot reach within the seam's bound). `AppleMusicService::playback_position()` forwards that value. This part changes no UI; it gives part 2/2 a narrow, testable source for the elapsed readout.

**Approach.**

- `src/audio.rs`: import `std::time::Duration` and add a provided trait method after `seek_by`: `fn position(&self) -> Option<Duration> { None }`. Its doc says it returns the loaded preview's position, `Some(Duration::ZERO)` when the backend has no loaded player, and `None` when the backend cannot answer; the default keeps `SilentOutput` and every existing test fake compiling without a position of their own.
- `src/audio/player.rs`: add `Command::Position(std::sync::mpsc::Sender<Duration>)`. Add `fn position(&self) -> Duration` to `WorkerPlayer`, with `RodioPlayer` returning `self.player.get_pos()` (the same source `seek_player` already reads). In `serve_commands`, handle `Command::Position(reply)` by sending `player.as_ref().map_or(Duration::ZERO, WorkerPlayer::position)`; because the loop handles the command in channel order, a position query sent during a preview download is answered after that download installs the new player. In `impl AudioOutput for RodioOutput`, add `position`: send `Command::Position` through the existing `send`, then wait on the reply with `recv_timeout`, returning `None` when the send fails or the wait times out. Define `POSITION_QUERY_TIMEOUT` next to `DEVICE_OPEN_TIMEOUT` (100 ms) and document that a download can hold the worker for up to `preview::PREVIEW_TIMEOUT`, so the query must not block the caller indefinitely.
- `src/apple_music.rs`: add `pub(crate) fn playback_position(&self) -> Option<Duration> { self.audio.position() }` beside `seek_output`, documented as synchronous, touching no shared state, and bounded by the seam's query timeout.
- `src/audio/tests.rs`: give `RecordingPlayer` a `position` implementation returning a fixed non-zero `Duration`, then add `position_query_returns_the_loaded_players_position` (send `Command::Position` after a `Play`, assert the reply equals the fake's position) and `position_query_returns_zero_when_nothing_is_loaded` (send `Command::Position` with no player). Add `rodio_output_asks_the_worker_for_its_position` (a helper thread reads `Command::Position` from the command channel and replies, while the test calls `position()`) and extend `rodio_output_reports_a_closed_worker_for_every_command` with `position() == None`.
- `src/test_support.rs`: override `position` for `RecordingAudio` to return `Some(Duration::from_secs(7))`, documented as a fixed stub position; no other call recording changes.
- `src/apple_music/tests.rs`: add `playback_position_forwards_the_audio_output_position`, building the service with `Arc::new(RecordingAudio::default())` through `with_audio` and asserting the service returns `Some(Duration::from_secs(7))`.
- `docs/invariants.md`: unchanged; this part adds a seam, not a user-visible guarantee.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`.

**Acceptance criteria.**

- `make check` passes.
- With a player loaded, `serve_commands` answers `Command::Position` with that player's `position()`; with no player it answers `Duration::ZERO`.
- `RodioOutput::position` returns the reply the worker sends, and returns `None` when the command channel is closed or no reply arrives within `POSITION_QUERY_TIMEOUT`.
- `AppleMusicService::playback_position` returns exactly the injected backend's `position()` value.
- `SilentOutput` and all existing `AudioOutput` test fakes keep compiling without adding a method, because the trait method has a `None` default.
- The change touches only the six listed files plus this `PLANS.md` edit; no UI, README, or invariant text changes.

### Show the item count beneath artist, album, and playlist lists (planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** Every loaded non-song browse list shows how many rows it holds beneath the rows — `3 artists`, `2 albums`, `1 playlist` — as the library mockup's status area does (`docs/screenshots/library.png`). The song lists keep their existing count-and-time footer and gain nothing new. This is independent of the pending row numbering: it reads only the slice length each view already receives.

**Approach.**

- `src/ui/views/browse.rs`: add a pure `pub(super) fn list_count_summary(count: usize, singular: &str, plural: &str) -> String` returning `"<count> <noun>"`, with `singular` chosen exactly when `count == 1` and `plural` otherwise. It formats any count, including `0`, so the wording is testable without an iced renderer (`format_minutes_seconds` is the shared formatter for the song footer).
- Add a private `fn list_with_count<'a>(list: Element<'a, Message>, count: usize, singular: &str, plural: &str) -> Element<'a, Message>`: when `count` is non-zero, wrap `list` in a `Column::new().push(list).push(Text::new(list_count_summary(...)).size(14))` with `Length::Fill` width and `Length::FillPortion(3)` height, mirroring the song footer in `song_list`; when `count` is zero, return `list` unchanged so a loading, error, or empty placeholder gets no `0 artists` footer.
- Wrap the three non-song list builders at their row-construction sites: `view_artists` passes `artists.len()` with `"artist"`/`"artists"`, `album_list` passes `albums.len()` with `"album"`/`"albums"` (so both `view_albums` and `view_all_albums` gain the footer once), and `view_playlists` passes `playlists.len()` with `"playlist"`/`"playlists"`. Do not wrap inside `scrollable_list` itself: `song_list` already appends its own count-and-time footer, so wrapping there would give every song list a second footer.
- Update the doc comments of `view_artists`, `album_list` (which `view_albums` and `view_all_albums` both delegate to), and `view_playlists` to say a non-empty list appends its item-count footer, and note in `list_with_count` why it skips an empty list.
- `src/ui/views/tests.rs`: import `list_count_summary` and add `list_count_summary_uses_the_singular_and_plural`, asserting `"1 artist"` for a count of one, `"3 artists"` for a count of three, `"1 playlist"`, and `"0 albums"`. The existing `browse_views_construct_over_the_loaded_library` and `browse_views_construct_over_an_empty_list` build the populated and empty branches of every wrapped builder through `construct_every_browse_view`.
- `README.md`: extend the existing Usage sentence — which already says each song list shows its song count and total playing time — to also say artist, album, and playlist lists show their item count beneath the rows. Keep the hunk to that sentence.
- `docs/invariants.md`: the browse-hierarchy and reply-freshness guarantees do not change, and the document makes no claim about footers, so leave it unchanged.

**Files touched.** `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `list_count_summary` returns `"1 artist"` for `(1, "artist", "artists")`, `"3 artists"` for `(3, "artist", "artists")`, `"1 playlist"` for `(1, "playlist", "playlists")`, and `"0 albums"` for `(0, "album", "albums")`.
- A loaded Artists, Albums, All Albums, or Playlists view renders its rows followed by exactly one footer whose count equals the number of rows passed in; an empty, loading, or failed one still renders only its placeholder, with no `0 …` footer.
- Song lists keep the existing `song_list_summary` footer and call no new footer helper, so no song list shows two footers.
- Row order, row labels, selection messages, epochs, and placeholder wording are unchanged, and both construct tests build every browse view without panicking.
- The change touches only the three files listed above (plus the `PLANS.md` edit and backlog moves), and the README hunk is limited to the item-count clause.

### Number every browse-list row (planned 2026-10-21 by the plan role; done 2026-10-21)

**Goal.** Each row in every browse list opens with its 1-based position and a period (`1. The Sample Band`, `2. First Record`), as the classic Winamp playlist editor does; the currently playing row keeps its `▶` marker before the number (`▶ 3. Opening`). This is independent of the planned count/time footer and of the browse data: numbering is derived from each list's row order, so it needs no service or model change.

**Approach.**

- `src/ui/views/browse.rs`: add a pure `pub(super) fn row_prefix(position: usize, is_current: bool) -> String` that returns `"{n}. "` for zero-based `position`, with `"▶ "` before the number when `is_current` is true (so `row_prefix(0, false)` is `"1. "` and `row_prefix(0, true)` is `"▶ 1. "`). It is the one place the marker and number order are composed, so a marker moved after the number cannot slip in untested.
- In `fn scrollable_list`, enumerate the incoming items (`for (position, (title, label, message, is_current)) in items.into_iter().enumerate()`) and build the row's leading label from `row_prefix(position, is_current)` instead of the current conditional `▶ ` row. Keep the title and secondary label as they are, and keep the existing per-row style selection (`current_row_style` for the playing row, `playlist_row_style` otherwise) and the empty-placeholder branch unchanged. Every browse list — Artists, Albums, Songs, All Songs, All Albums, Playlists — renders through `scrollable_list`, so all six gain numbering with no row-builder or call-site change.
- Update `scrollable_list`'s doc comment: it now says each row is prefixed with its 1-based position (and the playing marker), and that the prefix is one owned `String` per row per refresh while the title and secondary label still borrow. Do not cache prefixes in `BrowseList`: the position comes from the iterator and a cached list would need invalidation on every store for no correctness gain.
- `src/ui/views/tests.rs`: add a direct test for the helper, `row_prefix_numbers_rows_and_puts_the_play_marker_first`, asserting `row_prefix(0, false) == "1. "`, `row_prefix(9, false) == "10. "`, `row_prefix(99, false) == "100. "`, and `row_prefix(0, true) == "▶ 1. "`. The existing `browse_views_construct_over_the_loaded_library` and `browse_views_construct_over_an_empty_list` continue to build every numbered view through the shared constructor.
- `README.md`: add one sentence to the Usage paragraph saying every browse list numbers its rows from 1 like a Winamp playlist. Keep the hunk to that sentence.
- `docs/invariants.md` records browse navigation, reply freshness, and the Now Playing label; row numbering changes none of those guarantees, so leave it unchanged.

**Files touched.** `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `row_prefix` returns `"1. "`, `"10. "`, and `"100. "` for positions 0, 9, and 99 with `is_current == false`, and `"▶ 1. "` for position 0 with `is_current == true`.
- Every row built by `scrollable_list` gets the prefix before its title in both the playing and non-playing branches; the playing-row background and style are unchanged, and an empty list still shows only its placeholder with no number.
- Row selection messages, cached length labels, list order, and empty/loading/error wording are unchanged, and the construct tests build every browse view over both the loaded and empty library without panicking.
- The feature itself changes only the three files listed above (the `PLANS.md`/`PLANS-DONE.md` move aside), and the README hunk is limited to the numbering sentence.

### Show each song list's count and total playing time (planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** Every loaded song list shows, beneath its rows, how many songs it holds and their combined playing time — the classic Winamp playlist editor's status line. Today `song_list` in `src/ui/views/browse.rs` builds only the scrollable rows: each row shows its own formatted length (`song_row` via `format_track_time`), but the list never aggregates them. This is independent of the planned Shuffle-On Previous history: it reads only the `[Song]` slice the view already receives.

**Approach.**

- `src/ui/views/browse.rs`: add `pub(super) fn song_list_summary(songs: &[Song]) -> String` returning `"<count> song(s) - <total>"`, with singular `song` for a count of one and plural `songs` otherwise. Count every entry, including one whose `duration_ms` is `0`, and sum the durations with `u64::saturating_add` so a large list cannot overflow the total.
- Add a shared `fn format_minutes_seconds(total_ms: u64) -> String` in `now_playing.rs`: `0` formats as `"0:00"`, otherwise zero-padded `m:ss` with unbounded minutes (`3_665_000` formats as `"61:05"`), mirroring `views::format_track_time`'s arithmetic but not its `0` → `"--:--"` placeholder. Do not change `format_track_time`: `0` there means one track's duration is unknown, while `0` here is a valid list total when every duration is unknown.
- In `fn song_list`, keep the current `scrollable_list(...)` result for the rows; when `songs` is non-empty, return a `Column::new().push(list).push(Text::new(song_list_summary(songs)).size(14))` with `Length::Fill` width and the scrollable's current `Length::FillPortion(3)` height, so the browse panel keeps its share of the window and the summary sits left-aligned below the rows. When `songs` is empty, return the scrollable unchanged so the loading, error, and empty placeholders stay alone instead of gaining a `0 songs` footer. Compute the summary once per view refresh, not once per row, and update `song_list`'s doc comment to say it appends the count/time footer.
- No call-site changes are needed: `view_songs` (album and playlist songs and search results) and `view_all_songs` both build through `song_list`, so every loaded song list gains the footer. Artist, album, and playlist lists keep their current bodies.
- `src/ui/views/tests.rs`: add direct tests for the pure helper — `song_list_summary_counts_songs_and_sums_durations` (210_000 + 45_000 → `"2 songs - 4:15"`), `song_list_summary_uses_the_singular_for_one_song` (210_000 → `"1 song - 3:30"`), `song_list_summary_counts_unknown_durations_without_adding_time` (`[210_000, 0]` → `"2 songs - 3:30"`), and `song_list_summary_keeps_counting_minutes_past_an_hour` (3_665_000 → `"1 song - 61:05"`). The existing `browse_views_construct_over_the_loaded_library` and `browse_views_construct_over_an_empty_list` already exercise both footer branches through the view builders.
- `README.md`: add one sentence to the Usage paragraph saying each song list shows its song count and total playing time beneath the rows. Keep the hunk to that sentence.
- `docs/invariants.md` records browse navigation, reply freshness, and the Now Playing label; this display-only footer changes none of those guarantees, so leave it unchanged.

**Files touched.** `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `song_list_summary` reports the exact count and sum: `[]` → `"0 songs - 0:00"`, one 210_000 ms song → `"1 song - 3:30"`, 210_000 + 45_000 ms → `"2 songs - 4:15"`, and 3_665_000 ms → `"1 song - 61:05"`.
- A song with `duration_ms == 0` still counts toward the song total but contributes no time, so `[210_000, 0]` reads `"2 songs - 3:30"`.
- A non-empty song list built by `view_songs` or `view_all_songs` includes the footer, while an empty list shows only its existing placeholder; the construct tests cover both cases without panicking.
- The implementation diff changes only the three files listed above for the feature itself (the `PLANS.md`/`PLANS-DONE.md` move aside), and the README hunk is limited to the new summary sentence.

### Make Previous retrace the play history when Shuffle is on (planned 2026-10-20 by the plan role, done 2026-10-21)

**Goal.** With Shuffle on, pressing Previous returns to the track actually played before the current one, retracing the play order like classic Winamp, instead of stepping one row backward in the visible list. With Shuffle off, Previous keeps its sequential and Repeat-aware behavior. Today `step_track` in `src/ui/mod.rs` sends only a forward step through `transport::shuffled_track_id`; every backward step goes through `transport::previous_track_id`. The `step_track` doc comment and `previous_with_shuffle_still_steps_backward` in `src/ui/tests.rs` pin that Previous ignores Shuffle because no play history exists.

**Approach.**

- `src/ui/transport.rs`: beside `shuffled_track_id`, add `pub(super) const HISTORY_LIMIT: usize = 100` and `#[derive(Debug, Default)] pub struct PlayHistory` holding `entries: Vec<String>`, `cursor: Option<usize>`, and `pending: Option<PendingJump>`, plus a private `#[derive(Debug)] struct PendingJump { target: usize, from: Option<String>, retry: bool }`.
  - `pub fn record(&mut self, track_id: &str)`: when `pending` names a target entry equal to `track_id`, the history jump landed, so set `cursor` to that target and clear `pending` without pushing, truncating, or trimming — the jump neither duplicates the entry nor drops the forward entries, and a later new track truncates them from the landed cursor. Otherwise clear `pending` first; when the cursor already names `track_id` do nothing, and otherwise truncate entries after the cursor, push `track_id`, point the cursor at the new last entry, then drop the oldest entries past `HISTORY_LIMIT` while moving the cursor down with them.
  - `pub fn previous(&mut self, songs: &[Song], current: Option<&str>) -> Option<String>`:
    - A pending jump stays in effect while `current` is `None`, names the jump's target entry (its play committed but `record` has not run), or names the jump's `from` track (its play has not committed yet). When `current` names anything else, the user selected another track before the jump landed, so clear `pending` and continue from the cursor, which still names the last track actually recorded.
    - A failed play calls `retry_pending`, which keeps the pending jump and marks it for retry: `play_track` leaves `current_track` naming the target when the backend rejects the preview, so the next `previous` returns that same target once more (clearing the retry flag) and only a later press walks before it. A failed play with no pending jump is a no-op.
    - With a jump in effect, scan strictly before its target index, so repeated Previous presses before that play commits keep walking back. Without a jump, take `self.cursor?` and scan strictly before it when its entry is `current`, otherwise up to and including it (`cursor + 1`) when `current` is a newer, unrecorded selection.
    - Find the highest index below the start whose id `songs` contains; on a hit set `pending` to that index (keeping an in-effect jump's original `from`), return its id, and leave the cursor unchanged until the play records. On a miss return `None` leaving the cursor and any in-effect jump unchanged, so `step_track` falls back to `transport::previous_track_id`. Skip entries absent from `songs` so a browse to another list selects a row the visible list can show. Do not add the rejected draft's `at_start` helper: an exhausted history uses the same sequential fallback.
- `src/ui/mod.rs`: add `play_history: transport::PlayHistory` to `WinampPlayer` and initialize it with `Default::default()` in `WinampPlayer::new`. Change `step_track` to take `&mut WinampPlayer`: when `!forward && state.shuffle`, first try `player.play_history.previous(items, state.current_track.as_deref())`, use its id when present, and otherwise fall through to `transport::previous_track_id` as today; the forward Shuffle branch and both sequential branches stay as they are. In the `Message::TrackPlayed` arm, inside the existing current-generation guard, record `state.current_track` only when `state.is_playing` is true, so a failed play never enters the history and a superseded completion is skipped by the generation check; a history jump's completion records the target `pending` already names, which `record` handles without duplicating or truncating. A current-generation completion with `is_playing` false calls `play_history.retry_pending()`: the pending jump stays in place (the backend left `current_track` naming its target), so the next Previous retries that target rather than walking past it. Update the `step_track` doc comment to describe the history-backed Previous behavior instead of "Previous ignores Shuffle".
- `src/ui/tests.rs`: replace `previous_with_shuffle_still_steps_backward` with behavior tests, and drive `Message::TrackPlayed` to exercise the recording path. Cover a shuffled Previous that retraces two recorded plays; two presses before the first jump commits (same `current`) that keep walking back; a failed jump whose target the backend left in `current_track` and whose next Previous retries it; a pending jump superseded by a new committed `current` that then steps from the last actually played track; a shuffled Previous with no usable history that falls back to the sequential row; a history entry absent from the visible list being skipped; and two guard tests. `track_played_ignores_a_failed_completion` and `track_played_ignores_a_superseded_completion` each seed `["song-1", "song-2"]` and report `song-3` without recording it, then assert `previous(&songs, None)` returns `"song-2"` — with a wrongly recorded `song-3` it would return `"song-3"`, so the assertion actually pins the guard instead of passing through `previous`'s current-track branch.
- `src/ui/transport.rs` tests: cover recording order and the repeated-current no-op; recording after a back step dropping the forward entries; the `HISTORY_LIMIT` bound and cursor shift; `previous` returning `None` with no history; walking backward on repeated calls and across a pending jump; a jump target's `record` landing on its entry without duplicating or truncating; a failed jump retrying its target once before walking further back; a superseded jump stepping from the last recorded play rather than skipping it; including the last recorded entry when `current` is a newer selection; skipping ids absent from the supplied `songs`; and returning `None` at the oldest entry without moving the cursor.
- `README.md`: in the Usage paragraph, say that with Shuffle on Previous retraces the tracks played before the current one. Keep the hunk to that sentence.
- `docs/invariants.md` has no transport-stepping section (its headings cover the browse hierarchy, browse reply freshness, and the Now Playing label), so this plan leaves it unchanged.
- When the feature run moves this entry to `PLANS-DONE.md`, write the done entry's approach from the shipped code — including `PendingJump` and `previous(songs, current)` — so it does not repeat the stale draft entry that described a different API.

**Files touched.** `src/ui/transport.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `PlayHistory::record` keeps chronological play order, treats a repeated current id as a no-op, drops forward entries after a back step when a new track is recorded, keeps the forward entries when a pending jump's own target records, and keeps at most `HISTORY_LIMIT` entries with the cursor following the oldest dropped entry.
- `PlayHistory::previous` returns the nearest earlier entry present in `songs`, walks further back on repeated calls including before a jump's play commits, skips entries from a list the player no longer shows, includes the last recorded entry when `current` is a newer unrecorded selection, and returns `None` without moving when no earlier usable entry exists.
- A jump whose play never lands is retried when the backend leaves its target in `current_track`: from history `["song-1", "song-2", "song-3"]`, a Previous from `song-3` selects `song-2`; a failed completion names `song-2` and the next Previous selects `song-2` again before walking on to `song-1`. A jump superseded by a different current track is abandoned: a Previous from `song-3` then a Previous from a new `song-4` selects `song-3`, not `song-1` or `song-2`.
- With Shuffle on, Previous selects the recorded previous track rather than the row before the current one; with no usable history it falls back to `previous_track_id`. With Shuffle off, Previous still follows `previous_track_id` and the shared Repeat flag.
- A current-generation `TrackPlayed` with `is_playing` true records the committed track; one with `is_playing` false marks the pending jump for retry and records nothing, while a superseded generation records nothing. The two guard tests distinguish a recorded `song-3` (`previous(.., None)` would return `"song-3"`) from the unrecorded expected `"song-2"`.
- The new tests named in the approach pass alongside the existing Next/Previous wiring tests, and the README documents the Shuffle-on Previous behavior.
- The implementation diff changes only the four files listed above for the feature itself (the `PLANS.md`/`PLANS-DONE.md` move aside), and the README hunk is limited to the Shuffle-on Previous sentence.

### Seek the playing preview five seconds with the left and right arrow keys (planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** Pressing Left seeks the playing preview back five seconds and pressing Right seeks it forward five seconds, matching classic Winamp's keyboard bindings. Today `shortcuts::shortcut` maps only ArrowUp/ArrowDown (to `Message::VolumeUp`/`VolumeDown`), `Message` has no seek variant, and `AudioOutput` exposes only play/pause/resume/stop/`set_volume`/`set_balance`. `rodio` 0.22's `Player` already provides `get_pos` and `try_seek`, but nothing in `src/audio.rs` calls them, so the left/right keys currently do nothing.

**Approach.**

- **Seek-time channel phase.** rodio 0.22.2's `Source::try_seek` implementations leave the next sample on the same channel as before the seek: `SamplesBuffer::try_seek` offsets the target by the current channel and `SymphoniaDecoder::try_seek` re-advances to the active channel. The wrappers' cursors must stay in that phase. `BalanceSource::try_seek` already only delegates, so leave it that way and say in its doc comment why it must not reset `channel`; `EqSource::try_seek` currently assigns `self.channel = 0` after seeking, so remove that assignment but keep `rebuild_filters()` so a seek still clears the filter ringing.
- `src/audio.rs`: add `fn seek_by(&self, offset_secs: i64) -> Result<(), AppleMusicError>` to `AudioOutput`, documented as a relative move (positive seeks forward, negative backward), a no-op when no player is loaded, and a non-blocking channel send like `set_volume`. Implement it for `SilentOutput` (log and return `Ok`), add `Command::SeekBy(i64)`, and add `fn seek_by(&self, offset_secs: i64)` to `WorkerPlayer`. Add a private pure `fn seek_target(position: Duration, offset_secs: i64) -> Duration` that saturating-adds or saturating-subtracts `Duration::from_secs(offset_secs.unsigned_abs())`, so a backward seek past the start lands at zero. `RodioPlayer::seek_by` reads `self.player.get_pos()`, calls `self.player.try_seek(seek_target(position, offset_secs))`, and logs a `try_seek` error with `eprintln!` without changing any state. `serve_commands` handles `Command::SeekBy` by calling `player.seek_by(offset_secs)` only when a player is installed; the offset is not remembered across players, so a seek while stopped does nothing.
- `src/test_support.rs`: add `AudioCall::SeekBy(i64)`, record it in `RecordingAudio::seek_by`, and return `Ok(())` from `FailingAudio::seek_by`.
- `src/apple_music.rs`: add `pub(crate) fn seek_output(&self, offset_secs: i64) -> Result<(), AppleMusicError>` beside `set_output_volume` and `set_output_balance`, forwarding to `self.audio.seek_by(offset_secs)` and touching no shared state.
- `src/apple_music/tests.rs`: implement `seek_by` for `GatedAudio` and `RejectingAudio`, recording `AudioCall::SeekBy` and succeeding (like their other non-rejected commands). Add `seek_output_forwards_to_the_audio_backend`, asserting one `AudioCall::SeekBy(-5)` and that `AppState` is unchanged.
- `src/ui/mod.rs`: add `SeekBackward` and `SeekForward` variants beside `VolumeUp`/`VolumeDown`, and add a private `seek_preview(player, offset_secs)` helper that calls `player.apple_music_service.seek_output(offset_secs)`, logs a backend error like `mutate_volume`, and returns `Task::none()`. Route the two new arms through it with `-views::SEEK_STEP_SECS` and `views::SEEK_STEP_SECS`.
- `src/ui/views/mod.rs`: add `pub(super) const SEEK_STEP_SECS: i64 = 5;` beside `VOLUME_STEP`, documenting Winamp's five-second seek step.
- `src/ui/shortcuts.rs`: map `Key::Named(key::Named::ArrowLeft)` to `Message::SeekBackward` and ArrowRight to `Message::SeekForward`, updating the `message_for`/`shortcut` doc lists. Extend `message_for_maps_the_arrow_keys` and add both arrows to `shortcut_ignores_chord_modifiers`.
- `src/ui/tests.rs`: add `seek_messages_drive_the_audio_backend`, driving both messages through `update` over a `RecordingAudio` and asserting one `AudioCall::SeekBy(-SEEK_STEP_SECS)` and one `SeekBy(SEEK_STEP_SECS)` with no scheduled task.
- `src/audio.rs` tests: extend `silent_output_reports_success_for_every_command`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` with `seek_by`; add `seek_target_moves_forward_and_backward` (10s +5 → 15s, 10s -5 → 5s) and `seek_target_saturates_at_the_start` (3s -5 → 0); extend the recording `WorkerPlayer` with `seek_by` and `transport_volume_and_balance_commands_drive_the_installed_player` with a `Command::SeekBy(-5)` before its `Stop`, asserting the installed player received it. Change the `SeekableSource` test double's `try_seek` to land on the target frame plus its current channel instead of resetting to a frame boundary, so it models rodio's contract. Rewrite `eq_source_seek_clears_filter_state` to seek from a frame boundary and assert the channel is unchanged, and add `eq_source_seek_keeps_the_channel_cursor` and `balance_source_seek_keeps_the_channel_cursor`, which seek from mid-frame and assert the next sample is still treated as the same channel.
- `README.md`: change the arrow-key sentence to say the up/down arrows nudge the volume and the left/right arrows seek five seconds back or forward.

**Files touched.** `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views/mod.rs`, `src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `seek_target` moves forward and backward by the requested seconds and clamps a backward seek past the start at zero.
- `RodioOutput::seek_by` sends `Command::SeekBy` carrying the same signed offset; `SilentOutput::seek_by` returns `Ok`; a closed worker reports "the audio thread is gone".
- `serve_commands` applies `SeekBy` only to an installed player; the recording player receives `-5` from the extended transport test.
- `AppleMusicService::seek_output` records exactly one `AudioCall::SeekBy` and leaves `AppState` unchanged.
- `message_for` maps ArrowLeft to `Message::SeekBackward` and ArrowRight to `Message::SeekForward`; Control, Alt, and Logo chords map neither.
- Driving both seek messages through `update` records exactly one `AudioCall::SeekBy(-5)` and one `AudioCall::SeekBy(5)` and schedules no task.
- The README documents the left/right five-second seek keys.
- `EqSource::try_seek` clears filter state without changing `channel`, and `BalanceSource::try_seek` leaves `channel` unchanged; a mid-frame seek through either wrapper keeps the next sample on the same channel, and the updated `SeekableSource` models that contract.
- The implementation diff against the pre-change commit changes only the nine files listed above for the feature itself (the move of this entry to `PLANS-DONE.md` aside), and the README hunk is limited to the arrow-key sentence.

### Apply the equalizer curve to the audio output (Equalizer audio, parts 1/2 and 2/2: planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** A preview starts through the equalizer's ten band filters at the curve the panel currently shows. Today `AppState` stores `eq_enabled`, `eq_preamp`, and `eq_bands`, and the equalizer update arms only write them; `AudioOutput` has no equalizer method, so every preview plays flat. This part adds the filter chain and applies the stored curve when a preview starts; part 2 forwards changes made while a preview is already playing.

**Approach.**

- `src/equalizer.rs`: add `pub const BAND_HZ: [f32; BAND_COUNT]` holding the numeric centres matching `BAND_FREQUENCIES` (`60.0` through `16_000.0`, with `1K` as `1_000.0`) and `pub struct BandFilter` with `new(sample_rate)`, `set_curve(enabled, preamp_db, bands_db)`, and `process(sample)`. Build each band as an RBJ peaking biquad at its `BAND_HZ` centre with a fixed `Q = 1.41` (roughly one octave), rebuild the coefficients in `set_curve`, and return the sample unchanged when disabled. When enabled, multiply by the preamp gain `10^(preamp_db / 20)` and run the sample through all ten biquads. Keep `Biquad` private to the module; `BandFilter` is the public type the audio source uses.
- `src/audio.rs`: add `pub struct EqSnapshot { pub enabled: bool, pub preamp_db: f32, pub bands_db: [f32; BAND_COUNT] }` (derive `Debug, Clone, Copy, PartialEq`) and private `EqSettings` holding those values in `AtomicBool`/`AtomicU32`s plus an `AtomicU64` version. Add `EqSource<S>` holding `inner`, `Arc<EqSettings>`, the last seen version, one `BandFilter` per channel from `inner.channels()`, and a channel cursor; on each `next`, rebuild every channel filter when the version changed, then process the sample through the current channel's filter. Delegate `current_span_len`, `channels`, `sample_rate`, `total_duration`, and `try_seek` like `BalanceSource`, resetting the filters on a successful seek. Add `set_equalizer(&self, equalizer: EqSnapshot)` to `AudioOutput`, `Command::SetEqualizer(EqSnapshot)`, and `WorkerPlayer::set_equalizer`; `SilentOutput` logs and returns `Ok`, `RodioPlayer` stores the snapshot in its `Arc<EqSettings>`, and `append` wraps each preview as `BalanceSource::new(EqSource::new(preview, eq), balance)` so equalization runs before the stereo balance. `serve_commands` remembers the snapshot across players and applies it to every newly created player, exactly as it remembers volume and balance.
- `src/test_support.rs`: add `AudioCall::SetEqualizer(EqSnapshot)`, record it in `RecordingAudio::set_equalizer`, and return `Ok(())` from `FailingAudio::set_equalizer`. Extend `assert_preview_calls` so a preview start expects `SetVolume(0.5)`, `SetBalance(0.0)`, `SetEqualizer(default)`, then `Play`.
- `src/apple_music.rs`: add `pub(crate) fn set_output_equalizer(&self, equalizer: EqSnapshot) -> Result<(), AppleMusicError>` forwarding to `self.audio.set_equalizer(equalizer)` and touching no shared state, like `set_output_volume`. In `play_track`, read `state.eq_enabled`, `state.eq_preamp()`, and `state.eq_bands()` under the state lock, then chain `set_volume(volume)`, `set_balance(balance)`, `set_equalizer(snapshot)`, and `play(url)` so the first preview starts at the stored curve rather than flat. Keep the existing rollback on a backend error.
- `src/audio.rs` tests: update `RecordingPlayer` to implement `set_equalizer`; add `a_new_preview_starts_at_the_remembered_equalizer` beside `a_new_preview_starts_at_the_remembered_balance`; extend `silent_output_reports_success_for_every_command`, `audio_with_fallback_falls_back_to_silence_when_open_fails`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` with the new method. Add `eq_source_disabled_passes_samples_through`, `eq_source_enabled_flat_curve_passes_samples_through`, `eq_source_preamp_scales_by_the_decibel_gain`, `eq_source_filters_each_channel_with_its_own_state`, and `eq_source_follows_the_curve_changed_while_playing`.
- `src/equalizer.rs` tests: add `band_frequencies_have_matching_numeric_centres`, `band_filter_boost_and_cut_at_the_center_frequency` (a generated 1 kHz sine through a `+12 dB` and a `-12 dB` 1 kHz band, comparing settled RMS), and `band_filter_preamp_is_a_flat_gain`.
- `src/apple_music/tests.rs`: add `set_output_equalizer_forwards_to_the_audio_backend` and update `play_track_starts_the_preview_through_the_audio_backend` and `assert_preview_calls` callers to expect the equalizer snapshot before `Play`.
- `README.md`: state in the usage paragraph that the equalizer panel's curve is applied to preview playback.

**Files touched.** `src/equalizer.rs`, `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- A disabled `BandFilter` returns each input sample unchanged; an enabled flat curve (zero preamp, zero bands) also passes samples through within floating-point error.
- A `+12 dB` 1 kHz band raises a settled 1 kHz sine's RMS, a `-12 dB` band lowers it, and a `+6.0206 dB` preamp produces approximately twice the input amplitude; each channel of a stereo source keeps its own filter state.
- `EqSource` reports its inner source's channel count, sample rate, span, duration, and seek result, and follows a curve changed through its shared `EqSettings` between samples.
- `RodioOutput::set_equalizer` sends `Command::SetEqualizer` with the same snapshot; `SilentOutput::set_equalizer` returns `Ok`; a closed worker reports "the audio thread is gone".
- `AppleMusicService::set_output_equalizer` records one `AudioCall::SetEqualizer` and leaves `AppState` unchanged; `play_track` records `SetVolume`, `SetBalance`, `SetEqualizer` carrying the state's current curve, then `Play`.

**Part 2/2 (done 2026-10-20).** Toggling the equalizer, dragging the preamp or a band slider, or picking a preset changes the sound of the preview already playing, without restarting it. Part 1 applies the stored curve when a preview starts; before this part the four equalizer update arms still called `mutate_state`, so a change made mid-track never reached the audio backend.

- `src/ui/mod.rs`: added `fn mutate_equalizer(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message>` beside `mutate_volume` and `mutate_balance`: it locks the state, applies the mutation, reads `eq_enabled`, `eq_preamp()`, and `eq_bands()`, builds an `EqSnapshot`, and calls `player.apple_music_service.set_output_equalizer(snapshot)`, logging a backend error with the same `eprintln!` shape as the volume and balance helpers. `ToggleEqualizer`, `EqPreampChange`, `EqBandChange`, and `EqPresetSelected` now route through it instead of `mutate_state`. Because `EqSource` reads its shared `EqSettings` between samples, the running preview picks up the new curve without a new `Play`.
- `src/ui/tests.rs`: added `equalizer_messages_forward_to_the_audio_backend`, driving `ToggleEqualizer`, `EqPreampChange(3.0)`, `EqBandChange(0, 4.0)`, out-of-range and `f32::NAN` band gains, and `EqPresetSelected(rock_preset())` through `update` over a `player_with_audio` built with a `RecordingAudio` fake, asserting the recorded `SetEqualizer` snapshots match the clamped `AppState` values and that no arm schedules a task. Also added `an_equalizer_backend_failure_still_commits_the_state`, which drives `EqBandChange` over a `FailingEqualizerAudio` that rejects `set_equalizer`, asserting the clamped band still lands in `AppState` and the backend saw the attempt.
- `src/test_support.rs`: added `FailingEqualizerAudio`, an `AudioOutput` whose `set_equalizer` records the attempt and returns an error, so the equalizer arm's backend-error branch is reachable from the UI suite.
- `README.md`: extended the equalizer sentence to say the toggles, sliders, and presets change the playing preview's curve live.

**Files touched.** `src/ui/mod.rs`, `src/ui/tests.rs`, `src/test_support.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- Each equalizer message records exactly one `SetEqualizer` call whose enabled flag, preamp, and ten band gains equal the clamped values now in `AppState`; no call restarts the preview.
- A backend error from `set_output_equalizer` is logged and the state mutation still commits, as with volume and balance.
- The README says equalizer changes apply to the playing preview.

### Add the R and S keyboard bindings for Repeat and Shuffle (planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** Pressing R toggles Repeat and pressing S toggles Shuffle from the keyboard, matching classic Winamp, so both transport toggles are reachable without the mouse. Today `shortcuts::shortcut` maps Z/X/C/V/B and the arrow keys only: R and S fall through to `None`, even though `Message::ToggleRepeat`, `Message::ToggleShuffle`, and their `update` arms already exist.

**Approach.**

- `src/ui/shortcuts.rs`: add two arms to `shortcut` beside the existing letter arms — `r` returns `Message::ToggleRepeat` and `s` returns `Message::ToggleShuffle` — using the same `eq_ignore_ascii_case` guard as the other letters, so uppercase and Shift work and the existing Control/Alt/Logo rejection applies. Update the doc comments on `message_for` and `shortcut` to list the two new bindings.
- `README.md`: extend the usage paragraph's classic-key sentence after the V entry with "R toggles Repeat and S toggles Shuffle".
- Tests in `src/ui/shortcuts.rs`: extend the `assert_letter_bindings` helper so the existing `message_for_maps_each_bound_letter` and `shortcut_matches_letters_case_insensitively` tests also assert that `r`/`R` map to `Message::ToggleRepeat` and `s`/`S` map to `Message::ToggleShuffle`; add both keys to the key list in `shortcut_ignores_chord_modifiers`, so a Control, Alt, or Logo chord still maps neither.

**Files touched.** `src/ui/shortcuts.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `message_for` maps a pressed `Key::Character("r")` to `Message::ToggleRepeat` and `Key::Character("s")` to `Message::ToggleShuffle`, with or without Shift, and maps their uppercase forms the same way.
- Control, Alt, and Logo chords over R or S still map to `None`, and the existing unbound-key and key-release behavior is unchanged.
- The README's classic-key sentence lists R and S beside the existing transport and volume keys.

### Back the playlists browse with the Apple Music REST endpoints (Playlists browse, part 2/2: planned 2026-10-20 by the plan role, requires part 1/2 landed, done 2026-10-20)

**Goal.** Point the part-1 playlist queries at the signed-in user's Apple Music library, so a stored session shows that user's real playlists and each playlist's real tracks instead of the sample data, while a session-less service keeps its sample fallback. This is the second half of the playlist browse, the twin of "Back the library search with the Apple Music REST endpoint" (done); land it only once the Playlists view from part 1/2 is the running build.

**Approach.**

- `src/apple_music/rest.rs`: add `Playlist` to the `use crate::library::{Album, Artist, Song}` import and the module doc's model list, then add `pub fn get_all_playlists(&self, session: &MusicKitSession) -> Result<Vec<Playlist>, AppleMusicError>` requesting `{API_BASE}/me/library/playlists` through `self.fetch` and mapping each resource with `Resource::into_playlist`; add `pub fn get_songs_from_playlist(&self, session: &MusicKitSession, playlist_id: &str) -> Result<Vec<Song>, AppleMusicError>` requesting `{API_BASE}/me/library/playlists/{}/tracks` with `encode_path_segment(playlist_id)` and mapping with `Resource::into_song("")` — a library playlist track carries no album id, the same shape `get_all_songs` gives its songs. Add `Resource::into_playlist`, which moves the resource's `id` and `attributes.name` into the model through `required_id(id, "playlist")` and `required_name(…, &id, "playlist")`, exactly as `into_album` does for albums. Document the endpoint, the empty `album_id`, and the id encoding on both methods.
- `src/apple_music.rs`: add `Playlist` to the `use crate::library::{Album, Artist, Song}` import, rewrite `get_playlists` as `self.browse(|rest, session| rest.get_all_playlists(session), sample_all_playlists)` and `get_songs_from_playlist` as `self.browse(move |rest, session| rest.get_songs_from_playlist(session, &id), || lookup(&sample_library().songs_by_playlist, playlist_id))` after the existing `ensure_id_is_valid` guard, keeping the guard's error contract. Add `fn sample_all_playlists() -> Vec<Playlist>` cloning `sample_library().playlists`, and remove the part-1 doc caveat that the queries ignore a stored session.
- `README.md`: remove the "playlist data comes from the sample library until the REST endpoints land" caveat, leaving playlists described alongside the other signed-in browse queries.
- Tests: `src/apple_music/rest/tests.rs` gains `get_all_playlists_maps_the_library_playlists_collection` (pin `/v1/me/library/playlists` and the mapped id/name), `get_all_playlists_follows_a_next_page`, `get_all_playlists_reports_a_transport_error_bare`, `get_all_playlists_rejects_a_nameless_playlist`, `get_songs_from_playlist_maps_the_playlist_tracks_collection` (pin the id-scoped path and empty `album_id`), `get_songs_from_playlist_percent_encodes_the_playlist_id`, and `get_songs_from_playlist_reports_a_transport_error_bare`. `src/apple_music/tests.rs` adds `get_playlists_uses_the_rest_library_when_signed_in` and `get_songs_from_playlist_uses_the_rest_library_when_signed_in` over the stub transport, and keeps the existing no-session sample tests green (updating only the two part-1 tests that pinned the sample-only signed-in behavior).

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/sample_library.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary::get_all_playlists` requests `/v1/me/library/playlists`, maps each resource's id and name, and follows a same-origin `next` page; a nameless playlist is an error naming its id and a transport failure propagates its cause unchanged.
- `RestLibrary::get_songs_from_playlist` requests `/v1/me/library/playlists/{encoded-id}/tracks`, maps songs with an empty `album_id`, percent-encodes a reserved character in the id, and propagates a transport failure.
- With a stored session, `AppleMusicService::get_playlists` and `get_songs_from_playlist` return the REST results; with no session they return the sample playlists and playlist songs in the part-1 order.
- A blank, whitespace-only, or control-character playlist id is still rejected by the service before any REST call is issued.

### Add a Playlists browse level backed by the sample library (Playlists browse, part 1/2: planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** A `Playlists` button on the Artists view opens a flat list of the user's library playlists; pressing a playlist opens its songs in the existing Songs view, and those rows play exactly like any other song row. Back from the songs returns to Playlists, and Back from Playlists returns to Artists. The Playlists level is a new top-level browse sibling of the All Songs and All Albums flat lists. This first half answers the two new service queries from the sample library, the same sample-first split "Add a library search box to the playlist editor" (done) used before its REST sibling landed; the sibling "Back the playlists browse with the Apple Music REST endpoints" (Playlists browse, part 2/2) points the same two methods at the signed-in library. Until that sibling lands, a signed-in service also answers playlists from the sample library — documented in the method docs, not left implicit.

**Approach.**

- `src/library.rs`: add `pub struct Playlist { pub id: String, pub name: String }`, deriving `Debug, Clone, PartialEq, Serialize, Deserialize` like `Artist`, and extend the module doc's model list. Add tests mirroring the `Artist` pins: a hand-written `playlist_payload()` (`{"id":"playlist-1","name":"Morning Drive"}`), serialized-field-name/round-trip, unknown-field tolerance, and `assert_every_field_required`.
- `src/test_support.rs`: add `sample_playlist()` beside `sample_artist`/`sample_album`/`sample_song`, carrying id `playlist-1` and name `Morning Drive`, for the `library` and `apple_music` suites.
- `src/sample_library.rs`: add `pub playlists: Vec<Playlist>` and `pub songs_by_playlist: HashMap<String, Vec<Song>>` to `SampleLibrary`. Build three playlists in render order — `playlist-1` "Morning Drive", `playlist-2` "Focus", `playlist-3` "Sample Favorites" — and index their songs from the existing sample songs, preserving each playlist's named order (`playlist-1` → `song-1`, `song-4`; `playlist-2` → `song-2`, `song-5`; `playlist-3` → `song-1`, `song-2`, `song-3`). Add tests mirroring `albums_are_indexed_by_artist_in_library_order_with_no_empty_groups`: playlists in order, three non-empty groups, and each group's song ids in the documented order. Update the module doc's browse-query list.
- `src/apple_music.rs`: add `pub async fn get_playlists(&self) -> Result<Vec<Playlist>, AppleMusicError>` returning `sample_library().playlists.clone()`, and `pub async fn get_songs_from_playlist(&self, playlist_id: &str) -> Result<Vec<Song>, AppleMusicError>` calling `ensure_id_is_valid(playlist_id, IdKind::Playlist)` then `lookup(&sample_library().songs_by_playlist, playlist_id)`. Add `IdKind::Playlist` and its `"playlist"` `Display` arm. Document on both methods that they answer from the sample library even when a session is stored, and that the REST sibling rewires them through `browse`; the async signatures and id guard already match that end state.
- `src/ui/mod.rs`: add `Playlist` to the `use crate::library::{Album, Artist, Song}` import, `CurrentView::Playlists`, `Message::ShowPlaylists`, `Message::PlaylistsLoaded(Vec<Playlist>)`, `Message::PlaylistsLoadFailed(String)`, and `Message::PlaylistSelected { epoch, index }`; add `playlists: BrowseList<Playlist>` to `WinampPlayer`, built with `BrowseList::new(false)` in `WinampPlayer::new`; add `SongsOrigin::Playlist`. The `ShowPlaylists` arm clears `search_active`, sets `CurrentView::Playlists`, and calls `fetch_level` on `player.playlists` with report `"loading library playlists"` and `|service| async move { service.get_playlists().await }`. The `PlaylistSelected` arm mirrors `AlbumSelected`: resolve through `player.playlists.select(epoch, index)`, clone the id, set `songs_origin = SongsOrigin::Playlist`, switch to `CurrentView::Songs`, clear any active search, and `fetch_level` the songs with `"loading songs from playlist {playlist_id:?}"` and `get_songs_from_playlist`. The `Back` arm's `CurrentView::Songs` match adds `SongsOrigin::Playlist => CurrentView::Playlists`, and `CurrentView::Playlists => CurrentView::Artists`. Store/fail through `player.playlists` in the loaded/failed arms, and map the two new reply messages to `player.playlists.is_current(issued)` in the `BrowseReply` arm.
- `src/ui/views/browse.rs`: add `playlist_row` (name, `"View Songs"`, `Message::PlaylistSelected`, never current) and `view_playlists` built through `scrollable_list` with the `CurrentView::Playlists` placeholder from `browse_placeholder`; add `view_playlists_button` returning `labeled_button("Playlists", Message::ShowPlaylists)`; add `CurrentView::Playlists => "No playlists"` to `empty_list_label` and `Playlists` to `BROWSE_VIEWS` (six entries).
- `src/ui/views/mod.rs`: re-export `view_playlists` and `view_playlists_button`.
- `src/ui/mod.rs`'s `view`: add the `CurrentView::Playlists` browse arm rendering `views::view_playlists(&player.playlists.items, player.playlists.epoch, player.playlists.loading, player.playlists.error.as_deref())`, and push `views::view_playlists_button()` beside the All Songs and All Albums buttons on the Artists view.
- `docs/invariants.md`: add `Playlists → Artists` and `Songs → Playlists when the songs were opened from a playlist` to the Browse hierarchy guarantee, and name `back_from_playlists_returns_to_the_artists_list` and `playlist_selected_in_playlists_loads_its_songs_and_back_returns_to_playlists` under "Pinned by".
- `README.md`: in the usage paragraph, describe the `Playlists` button and its Back path, and state that playlist data comes from the sample library until the REST endpoints land (the sibling plan removes that caveat).
- Tests: `src/apple_music/tests.rs` gains `get_playlists_returns_the_sample_playlists_in_order`, `get_songs_from_playlist_returns_its_sample_songs_in_order`, `get_songs_from_playlist_rejects_a_blank_id` (empty, whitespace-only, control character), and `get_songs_from_playlist_returns_empty_for_an_unknown_playlist`; the session variants pin the documented sample-only behavior so the sibling's change is deliberate. `src/ui/tests.rs` gains `show_playlists_fetches_and_switches_view`, `playlists_loaded_stores_the_list`, `playlist_selected_in_playlists_loads_its_songs_and_back_returns_to_playlists`, `back_from_playlists_returns_to_the_artists_list`, and `show_playlists_clears_an_active_search`, and extends `seed_browse_lists`, `assert_browse_lists_empty`, `each_browse_failure_stores_its_report_in_its_own_list`, and `view_constructs_over_the_apps_full_input_space` to cover `playlists`. `src/ui/views/tests.rs` pins `playlist_row`, `view_playlists_button`, the `empty_list_label` arm, and the extended `BROWSE_VIEWS` loops.

**Files touched.** `src/library.rs`, `src/test_support.rs`, `src/sample_library.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `Playlist` serializes as exactly `{"id", "name"}`, round-trips, tolerates an unknown field, and rejects a payload missing either field.
- The sample library exposes `playlist-1`/`playlist-2`/`playlist-3` in that order with the documented song ids in each `songs_by_playlist` group; no group is empty.
- `AppleMusicService::get_playlists` returns those three playlists in order with no session and with a session; `get_songs_from_playlist("playlist-1")` returns `song-1` then `song-4`; an unknown id returns an empty list; a blank, whitespace-only, or control-character id returns an error naming `playlist`.
- Pressing `Playlists` clears an active search, switches to `CurrentView::Playlists`, and fetches into `playlists`; a loaded reply stores the rows and a failed reply records the report on `playlists` alone.
- Pressing a playlist row fetches that playlist's songs into `songs` and shows `CurrentView::Songs`; Back returns to `CurrentView::Playlists`, while Back from playlist songs reached through any other origin still returns to Albums or All Albums as before.
- A stale playlist press (wrong epoch or out-of-range index) issues no fetch and leaves the view unchanged.
- The Artists view shows the `Playlists` button, and all six browse levels assemble, including empty and failed `playlists`, without panicking.

### Apply the balance slider to the audio output (planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** Moving the balance slider changes the stereo image of the playing preview, and a preview starts at the slider's balance. Today `Message::BalanceChange` only writes `AppState::balance`: `AudioOutput` has no balance method, so `mutate_state` never forwards the value and every preview plays centered. This is the balance twin of "Drive the volume slider through the audio backend" (done), whose approach deliberately left `BalanceChange` alone because rodio's `Player` has no panner. A small `Source` wrapper supplies the missing panner without a new dependency.

**Approach.**

- `src/audio.rs`:
  - Add `set_balance(&self, balance: f32) -> Result<(), AppleMusicError>` to `AudioOutput` (doc: `-1.0` is hard left, `0.0` centered, `1.0` hard right; it affects the current player and is remembered for the next `play`). `SilentOutput::set_balance` logs the value and returns `Ok`.
  - Add `Command::SetBalance(f32)`; `RodioOutput::set_balance` sends it.
  - Add a private `BalanceSource<S: Source>` adapter that wraps the decoded preview and holds the balance as an `Arc<AtomicU32>` of `f32` bits, read on every sample so a slider drag changes the current preview without restarting it. Delegate `current_span_len`, `channels`, `sample_rate`, `total_duration`, and `try_seek` to the inner source. Gain per channel: for a mono source `1.0 - balance.abs()`; otherwise even channel indices use `1.0 - balance.max(0.0)` (left) and odd indices use `1.0 + balance.min(0.0)` (right), so `-1.0` silences the right channel, `+1.0` the left, and `0.0` is unity. Do not use rodio's `ChannelVolume`/`Spatial`: both downmix the input to mono, which would collapse the preview's stereo image.
  - Add a private `RodioPlayer` struct holding the `rodio::Player` and its balance `Arc<AtomicU32>`; implement `WorkerPlayer` for it with `Preview = rodio::Decoder<Cursor<Vec<u8>>>`, appending the preview wrapped in `BalanceSource`, and add `set_balance` to the `WorkerPlayer` trait (the recording fake records it). `run_worker` builds a `RodioPlayer` instead of a bare `rodio::Player`.
  - In `serve_commands`, remember the balance (initial `0.0`) across players, map `Command::SetBalance` to storing it and calling `set_balance` on the current player when one exists, and call `set_balance(balance)` on a newly created player before `append`, as `set_volume` already does. Update the worker tests: the recording fake records `set_balance`, and a test pins that a balance set while stopped applies to the next player and one set while playing reaches the current player.
  - Add unit tests for `BalanceSource` using a two-channel `SamplesBuffer` with distinct left/right constants: `-1.0` keeps left and zeroes right, `+1.0` the reverse, `0.0` is unchanged, and changing the shared atomic between `next()` calls changes the following samples (the live-update contract).
  - Extend `silent_output_reports_success_for_every_command`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` to cover `set_balance`.
- `src/test_support.rs`: add `AudioCall::SetBalance(f32)` and record it in `RecordingAudio::set_balance`; `FailingAudio::set_balance` returns `Ok(())` like its volume twin. Extend `assert_preview_calls` (and its callers) so a preview start expects `SetVolume`, then `SetBalance`, then `Play`.
- `src/apple_music.rs`: add `pub(crate) fn set_output_balance(&self, balance: f32) -> Result<(), AppleMusicError>` forwarding to `self.audio.set_balance(balance)` and touching no shared state, exactly as `set_output_volume` does. In `play_track`, capture `state.balance()` before dropping the lock and apply it with the volume before `play`, so the first preview starts at the slider's balance.
- `src/ui/mod.rs`: add `mutate_balance(player, mutation) -> Task<Message>` beside `mutate_volume`: apply the mutation under one synchronous `blocking_lock`, read back the clamped `state.balance()`, forward it to `player.apple_music_service.set_output_balance(...)`, and log a backend error. Replace the `Message::BalanceChange` arm's `mutate_state` call with it and drop `BalanceChange` from `mutate_state`'s doc list.
- `README.md`: state in the usage paragraph that the balance slider sets the audio backend's stereo balance, beside the existing volume sentence.
- Tests: `src/apple_music/tests.rs` gains `set_output_balance_forwards_to_the_audio_backend` (one `AudioCall::SetBalance`, `AppState::balance` unchanged) and updates `play_track_starts_the_preview_through_the_audio_backend` to expect the balance call. `src/ui/tests.rs` gains `balance_messages_forward_to_the_audio_backend`, driving `Message::BalanceChange` through `update` over a `player_with_audio(RecordingAudio)` and asserting the recorded balance and the clamped `AppState::balance`, including an out-of-range value and `f32::NAN` (which stores and forwards `0.0`).

**Files touched.** `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `BalanceSource` at `-1.0` leaves the left channel unchanged and zeroes the right; at `+1.0` the reverse; at `0.0` both unchanged; a mono source is attenuated by `1.0 - balance.abs()`; mutating the shared balance between `next()` calls changes later samples without rebuilding the source.
- `RodioOutput::set_balance(0.4)` sends `Command::SetBalance(0.4)`; `SilentOutput::set_balance` returns `Ok`; a closed worker reports "the audio thread is gone" from `set_balance`.
- `serve_commands` applies a remembered balance to the player a later `Play` creates and to the current player on `SetBalance`.
- `AppleMusicService::set_output_balance(-0.4)` records one `AudioCall::SetBalance(-0.4)` and leaves `AppState::balance` unchanged; `play_track` records `SetVolume`, then `SetBalance` carrying the state's current balance, then `Play`.
- Driving `Message::BalanceChange(-0.4)` stores `-0.4` in `AppState::balance` and records `SetBalance(-0.4)`; an out-of-range value stores and forwards the clamped end, and `NaN` stores and forwards `0.0`.

### Add docs/invariants.md pinning browse and Now Playing guarantees to their tests (planned 2026-10-20 by the steward role as a gap promotion for `unclear-invariant`, done 2026-10-20)

**Goal.** Create `docs/invariants.md`, the record the validation-gap policy tells every role to read and update but which the repository does not have. Three Fixed entries carry the `unclear-invariant` tag because the guarantee had to be reconstructed before the bug could be confirmed: "Browse view can only move forward — no way back from Albums/Songs, a dead end", "A browse reply is stored after a newer request was issued, so a navigation shows the previous level's rows", and "Now Playing bar shows the track id instead of the song title". The document states each guarantee, names the code that enforces it, and names the tests that pin it, so a later bug in these areas reads the invariant instead of reconstructing it. No behavior changes.

**Approach.**

- Create `docs/invariants.md` with a short header explaining the file: one section per subsystem, each stating the guarantee, the enforcing code, and the tests that fail if it is removed; a change to a guarantee updates its section in the same commit.
- `## Browse hierarchy`: every browse level above Artists has a Back action that steps one level up, Artists is the top level, and Back from an active search leaves the search for the Artists list. Enforced by the `Message::Back` arm in `src/ui/mod.rs` and `views::can_go_back` in `src/ui/views/browse.rs`. Pinned by `back_from_albums_returns_to_artists`, `back_from_songs_returns_to_albums`, `back_from_artists_is_a_noop`, `back_from_all_songs_returns_to_the_artists_list`, `back_from_all_albums_returns_to_the_artists_list`, `back_leaves_an_active_search_for_the_artists_list`, and `back_button_is_available_only_below_the_artist_list`.
- `## Browse reply freshness`: a fetched reply is stored only when no newer request was issued for that list. `RequestGeneration::is_current` drops it on iced's executor thread and `BrowseList::is_current` re-checks at the store on the UI thread; a selection also carries its list's epoch so a press that outlived its list is ignored. Enforced by `RequestGeneration` in `src/ui/loading.rs` and `BrowseList::is_current`, `select`, and `store` in `src/ui/browse.rs`. Pinned by `a_slow_stale_browse_reply_does_not_overwrite_the_newer_list`, `a_slow_stale_browse_failure_does_not_overwrite_the_newer_list_or_error`, `a_reply_superseded_between_the_mapper_and_update_is_not_stored`, `a_stale_songs_reply_superseded_before_update_is_not_stored`, `a_stale_albums_reply_superseded_before_update_is_not_stored`, and `selection_messages_with_a_stale_epoch_or_index_do_nothing`.
- `## Now Playing label`: the bar shows the known current track's human-readable title and artist, never the internal id when the id is in the known-track index; an unknown id falls back to the id, no current track shows "Nothing", and Stop keeps the interrupted track's title. Enforced by `now_playing_label` and `now_playing_artist` in `src/ui/views/now_playing.rs`. Pinned by `now_playing_label_resolves_known_track_to_title`, `now_playing_label_falls_back_to_id_when_track_not_in_tracks`, `now_playing_label_shows_nothing_with_no_current_track`, `now_playing_artist_resolves_known_track_to_artist`, `now_playing_label_keeps_the_track_name_after_browsing_to_another_album`, `now_playing_label_borrows_the_title_and_nothing_literal`, and `now_playing_artist_borrows_the_artist`.
- Each section cites the Fixed entry it came from (title and `BUGS-FIXED.md`) so the reconstruction history stays traceable.

**Files touched.** `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes; the change is documentation only and touches no source.
- `docs/invariants.md` exists with the three sections above, each naming its enforcing code and at least one test.
- Every cited test name is a real `fn` in the named file, checked with `make test-one TEST=<name>` (or by `grep` before running it).
- The browse section records a Back path from every level above Artists; the freshness section records both the executor-thread check and the store-time re-check; the Now Playing section records the unknown-id fallback and the "Nothing" contract.

### Add an All Albums view listing the whole library flat (planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** A new top-level `All Albums` browse view lists every album in the user's library in one flat list, in library order. Pressing an album opens its songs exactly as the per-artist Albums view does, and Back from those songs returns to All Albums rather than to the artist list. It is reached by an `All Albums` button on the Artists view, beside `All Songs`, and Back from All Albums returns to Artists. Today albums are reachable only by drilling Artists → Albums, so there is no whole-library album view. The REST half reads `/v1/me/library/albums` through the same paginated collection helper as the other browse queries; the sample half walks the sample library's artists → albums in library order. Like `get_all_songs`, an all-albums resource has no artist id available in the collection mapping, so `Album::artist_id` is left empty; nothing that opens an all-albums row reads it (`get_songs_from_album` uses only the album id). Independent of the All Songs and search work, so it can land on its own.

**Approach.**

- `src/apple_music/rest.rs`:
  - Add `pub fn get_all_albums(&self, session: &MusicKitSession) -> Result<Vec<Album>, AppleMusicError>` requesting `{API_BASE}/me/library/albums` through `self.fetch`. Map each resource with `Resource::into_album("")` (the same consuming mapping `get_albums_by_artist` uses, which validates the album's id and name and leaves `artist_id` empty); document why the artist id is empty, as `get_all_songs` documents its empty `album_id`.
- `src/apple_music.rs`:
  - Add `pub async fn get_all_albums(&self) -> Result<Vec<Album>, AppleMusicError>` through the existing `browse` helper: `self.browse(|rest, session| rest.get_all_albums(session), sample_all_albums)`.
  - Add a private `fn sample_albums_in_library_order() -> impl Iterator<Item = &'static Album>` walking `sample_library().artists` then `albums_by_artist`, and make `sample_songs_in_library_order` start from that same album walk so the two orders cannot drift. Add `fn sample_all_albums() -> Vec<Album>` from the album walk.
- `src/ui/mod.rs`:
  - Add `CurrentView::AllAlbums`, `Message::ShowAllAlbums`, `Message::AllAlbumsLoaded(Vec<Album>)`, and `Message::AllAlbumsLoadFailed(String)`.
  - Add `all_albums: BrowseList<Album>` to `WinampPlayer`, initialised `BrowseList::new(false)` in `WinampPlayer::new`.
  - Add a `SongsOrigin { Album, AllAlbums }` enum and a `songs_origin: SongsOrigin` field initialised to `SongsOrigin::Album`. The `AlbumSelected` arm must resolve the pressed row against `player.all_albums` when `player.current_view == CurrentView::AllAlbums` and against `player.albums` otherwise, clone the selected album id, then set `songs_origin` from that same view before moving to `CurrentView::Songs`. The `Back` arm's `CurrentView::Songs` match uses `songs_origin` to return to `AllAlbums` or `Albums`; `AllAlbums` on Back returns to `Artists`.
  - `Message::ShowAllAlbums` arm: clear `search_active`, set `current_view = CurrentView::AllAlbums`, and call `fetch_level` on `player.all_albums` with the report `"loading all library albums"`, `|service| async move { service.get_all_albums().await }`, and the two new messages.
  - `AllAlbumsLoaded`/`AllAlbumsLoadFailed` store/fail through `player.all_albums`; the `BrowseReply` arm re-checks `player.all_albums.is_current(issued)` beside the existing lists.
  - `view`'s browse match gains a `CurrentView::AllAlbums` arm rendering `views::view_all_albums(&player.all_albums.items, …)` (no current-track argument, like the Albums arm), and pushes `views::view_all_albums_button()` on the Artists view beside `view_all_songs_button`.
- `src/ui/views/browse.rs`:
  - Add `pub fn view_all_albums(albums, epoch, loading, error)` built by `album_list` from `album_row` with the `CurrentView::AllAlbums` placeholder, and `pub fn view_all_albums_button()` returning `labeled_button("All Albums", Message::ShowAllAlbums)`.
  - Add `CurrentView::AllAlbums => "No albums"` to `empty_list_label` and `CurrentView::AllAlbums` to `BROWSE_VIEWS`.
- Tests:
  - `src/apple_music/rest/tests.rs`: `get_all_albums_maps_the_library_albums_collection` (pin `/v1/me/library/albums`, the mapped id/title, and the empty `artist_id`); `get_all_albums_follows_a_next_page`; `get_all_albums_reports_a_transport_error_bare`; `get_all_albums_rejects_a_nameless_album`.
  - `src/apple_music/tests.rs`: `get_all_albums_returns_every_sample_album_in_library_order` (album-1, album-2, album-3); `get_all_albums_uses_the_rest_library_when_signed_in`.
  - `src/ui/tests.rs`: `show_all_albums_fetches_the_flat_list_and_switches_view`; `all_albums_loaded_stores_the_flat_list`; `back_from_all_albums_returns_to_the_artists_list`; `show_all_albums_clears_an_active_search`; `album_selected_in_all_albums_loads_its_songs_and_back_returns_to_all_albums` (proves the press resolves against `all_albums` and the Back origin). Extend `seed_browse_lists`, `assert_browse_lists_empty`, `each_browse_failure_stores_its_report_in_its_own_list`, and `view_constructs_over_the_apps_full_input_space`'s failure shape to cover `all_albums`.
  - `src/ui/views/tests.rs`'s suite: a `view_all_albums_button` construction check, the `empty_list_label` arm, and the extended `BROWSE_VIEWS` loops.

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary::get_all_albums` requests `/v1/me/library/albums`, maps each resource's id and title, leaves `artist_id` empty, and follows a same-origin `next` page; a transport failure propagates its cause and a nameless album is an error naming its id.
- With no session, `AppleMusicService::get_all_albums` returns every sample-library album in library order (album-1, album-2, album-3); with a session it returns the REST collection.
- Pressing `All Albums` on the Artists view clears the buffer, clears an active search, switches to `CurrentView::AllAlbums`, and fetches; the loaded reply stores rows and a failed reply records the report on `all_albums` alone.
- Back from `CurrentView::AllAlbums` returns to `CurrentView::Artists`.
- Pressing an album row in `CurrentView::AllAlbums` fetches that album's songs into `songs` and shows `CurrentView::Songs`; Back then returns to `CurrentView::AllAlbums`, while Back from songs opened through the per-artist Albums view still returns to `CurrentView::Albums`.
- The Artists view shows the `All Albums` button and the view assembles every browse level, including an empty/failed `all_albums`, without panicking.

### Add an All Songs view listing the whole library flat (found 2026-10-09, done 2026-10-09)

**Goal.** A new top-level `All Songs` browse view lists every song in the user's Apple Music library in one flat list, in library order, and each row plays exactly like a browsed song. It is reached by an `All Songs` button on the Artists view, and Back returns there. Today the only way to reach a song is to drill Artists → Albums → Songs, and the search box returns only query matches, so there is no way to see the library's songs as a whole. The REST half reads the library-songs collection (`/v1/me/library/songs`, which follows `next` pages like the sibling collections) and the sample half lists the whole sample library, through the same signed-in/sample split `browse` already applies. It is independent of the existing search box and can land on its own.

**Approach.**

- `src/apple_music/rest.rs`:
  - Add `pub fn get_all_songs(&self, session: &MusicKitSession) -> Result<Vec<Song>, AppleMusicError>`. Build `{API_BASE}/me/library/songs` and call `self.fetch` — the same paginated collection helper `get_favorite_artists` and `get_albums_by_artist` use, so a library larger than one page is read in full under one `REQUEST_TIMEOUT`. Map each `Resource` exactly as `search_library` does (`Resource::into_song("")`, which moves the id, title, artist, and preview URL out of the parsed resource and validates its id and name) with `album_id: String::new()`, because a library-songs resource carries no album id and nothing that plays a song reads it.
- `src/apple_music.rs`:
  - Add `pub async fn get_all_songs(&self) -> Result<Vec<Song>, AppleMusicError>` answering through the existing `browse` helper: `self.browse(move |rest, session| rest.get_all_songs(session), sample_all_songs)`.
  - Extract the ordered library walk `sample_songs_matching` already spells out into a private `fn sample_songs_in_library_order() -> impl Iterator<Item = &'static Song>` (artists → `albums_by_artist` → `songs_by_album`, in library order), so the new `fn sample_all_songs() -> Vec<Song>` (`sample_songs_in_library_order().cloned().collect()`) and `sample_songs_matching` (the same walk, filtered) cannot drift apart.
- `src/ui/mod.rs`:
  - `Message::TrackSelected` resolves against `all_songs` when `current_view` is `AllSongs` and against `songs` otherwise, and `step_track` steps the same view's list, so a flat row plays — and Next/Previous move through — the flat list rather than the stale browsed songs.
  - Add `CurrentView::AllSongs`, `Message::ShowAllSongs`, `Message::AllSongsLoaded(Vec<Song>)`, and `Message::AllSongsLoadFailed(String)`.
  - Add `all_songs: BrowseList<Song>` to `WinampPlayer`, initialised `BrowseList::new(false)` in `WinampPlayer::new`.
  - `Message::ShowAllSongs` arm: clear `search_active`, set `current_view = CurrentView::AllSongs`, and issue `fetch_level(&mut player.all_songs, …, "loading all library songs".to_string(), |service| async move { service.get_all_songs().await }, Message::AllSongsLoaded, Message::AllSongsLoadFailed)`.
  - `AllSongsLoaded` stores via `player.all_songs.store`; `AllSongsLoadFailed` records via `player.all_songs.fail`; the `BrowseReply` arm re-checks `player.all_songs.is_current(issued)` for both, beside the existing three lists.
  - `Back` arm: `CurrentView::AllSongs => CurrentView::Artists`.
  - `view`'s browse match: `CurrentView::AllSongs` builds `views::view_all_songs` from `player.all_songs`, mirroring the `Songs` arm (including the second lock that hands the current track id to the view).
  - Push `views::view_all_songs_button()` in `view` when `current_view` is `CurrentView::Artists`, beside the Back/Retry buttons.
- `src/ui/views.rs`:
  - Add `pub fn view_all_songs(songs, epoch, loading, error, current_track)` that calls `song_list` from `song_row` with the `CurrentView::AllSongs` placeholder, like `view_songs`.
  - Add `pub fn view_all_songs_button()` returning `labeled_button("All Songs", Message::ShowAllSongs)`.
  - `empty_list_label`: add `CurrentView::AllSongs => "No songs"`.
  - Add `CurrentView::AllSongs` to `BROWSE_VIEWS` so the existing per-view construction and placeholder loops exercise it.
- Tests:
  - `src/apple_music/rest/tests.rs`: `get_all_songs_maps_the_library_songs_collection` (pin the requested URL, the mapped songs, and the empty `album_id`); `get_all_songs_follows_a_next_page`; `get_all_songs_reports_a_transport_error_bare`; `get_all_songs_rejects_a_nameless_song`.
  - `src/apple_music/tests.rs`: `get_all_songs_returns_every_sample_song_in_library_order` (song-1 through song-5); `get_all_songs_reads_the_rest_library_when_signed_in` (store a session over a stub transport and assert the REST songs, not the sample list).
  - `src/ui/tests.rs`: `show_all_songs_fetches_the_flat_list_and_switches_view`; `all_songs_loaded_stores_the_flat_list`; `back_from_all_songs_returns_to_the_artists_list`; `show_all_songs_clears_an_active_search`. Extend `seed_browse_lists` to populate `all_songs` and `assert_browse_lists_empty` to check it, so the shared full-input and failure-shape tests cover the new buffer.
  - `src/ui/views.rs`'s suite: a `view_all_songs_button` construction check and the `empty_list_label` arm, plus the extended `BROWSE_VIEWS` loops.

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views.rs`, `src/ui/tests.rs`.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary::get_all_songs` requests `/v1/me/library/songs`, maps every resource's id, title, artist, duration, and first non-blank preview URL, leaves `album_id` empty, and follows a same-origin `next` page.
- A transport failure propagates as the transport's own cause; a resource without a name is an error naming its id.
- With no session, `AppleMusicService::get_all_songs` returns every sample-library song in library order (song-1, song-2, song-3, song-4, song-5); with a session, it returns the REST collection.
- Pressing `All Songs` on the Artists view clears the buffer, switches to `CurrentView::AllSongs`, and fetches; the loaded reply stores the rows and a failed reply records the report.
- Back from `CurrentView::AllSongs` returns to `CurrentView::Artists`.
- Selecting a row in `CurrentView::AllSongs` plays the flat list's song, and Next/Previous step the flat list rather than the browsed `songs` buffer.

### Back the library search with the Apple Music REST endpoint (found 2026-10-09, done 2026-10-09)

**Goal.** `AppleMusicService::search_songs`, added by the sibling plan "Add a library search box to the playlist editor" (found 2026-10-09), queries the signed-in user's Apple Music library through the REST client and falls back to the sample-library filter when no session is stored — the same signed-in/sample split every browse query already uses. Land this only once the search box is the running build.

**Approach.**

- `src/apple_music/rest.rs`:
  - Add `pub fn search_library(&self, session: &MusicKitSession, query: &str) -> Result<Vec<Song>, AppleMusicError>`. Build `{API_BASE}/me/library/search?term={encode_path_segment(query)}&types=library-songs&limit=25` — Apple documents the library-search `limit` maximum as 25 (default 5), so the request asks for the documented maximum rather than the out-of-contract 100 this entry first named — and parse the search envelope `{"results":{"library-songs":{"data":[...],"next":...}}}` with new private `SearchEnvelope` / `SearchResults` / `SearchCollection` structs (the `library-songs` key renamed in serde). Apple omits `results.library-songs` for a search that matched nothing, so the field is an `Option` and an absent key maps to an empty list rather than a parse error. Map each `Resource` exactly as `get_songs_from_album` does, but with `album_id: String::new()`: a library-songs search result carries no album id, and nothing that plays a song reads `album_id` (only `get_songs_from_album` and the sample-library index use it). Follow the collection's `next` link with the same same-origin, `MAX_PAGES`, `page_context`, and page-notice rules `fetch` applies, so a query matching more than one page is read in full. To share that loop, split `fetch` into a thin wrapper over a new private `fetch_parsed` that takes the page parser as a closure, so the top-level and nested envelopes reuse the same pagination, deadline, and page-notice rules.
- `src/apple_music.rs`: rewrite `search_songs` to answer through the existing `browse` helper — `self.browse(move |rest, session| rest.search_library(session, &query), || sample_songs_matching(&query))` — keeping the blank-query guard from the sibling plan. A signed-in user now searches the real library; a signed-out one keeps the sample filter.
- `README.md`: extend the status and usage paragraphs to say the search box queries the signed-in Apple Music library.
- Tests:
  - `src/apple_music/rest/tests.rs`: `search_library_maps_the_search_envelope_and_percent_encodes_the_term` (assert the requested URL and the mapped songs, with the empty `album_id`); `search_library_returns_no_songs_when_the_results_carry_no_library_songs` (an absent and an empty `library-songs` both return an empty list); `search_library_follows_a_next_page`; `search_library_reports_a_transport_error_bare`; `search_library_rejects_a_nameless_song`.
  - `src/apple_music/tests.rs`: `search_songs_uses_the_rest_library_when_signed_in` — store a session through `authenticate_with` over a stub transport and assert the returned songs come from the REST search envelope, not the sample filter.

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary::search_library` requests `/v1/me/library/search?term=<percent-encoded query>&types=library-songs&limit=25` (a term with a space and a reserved character is encoded), maps the envelope's songs with their id, title, artist, duration, and first non-blank preview URL, and leaves `album_id` empty.
- A no-match search response that omits `results.library-songs` (or carries an empty `data`) returns an empty list rather than a parse error.
- A search response whose `library-songs.next` names a same-origin path is followed, and the songs of both pages are returned.
- A transport failure propagates as the transport's own cause; a resource without a name is an error naming its id.
- With a session stored, `AppleMusicService::search_songs` returns the REST results; with none, it returns the sample-library matches.

### Add a library search box to the playlist editor (found 2026-10-09, done 2026-10-09)

**Goal.** A search box sits above the browse list on every browse screen; typing a query and pressing Enter replaces the playlist-editor list with the matching songs, which play exactly like a browsed song. This is the first half of library search — the UI plus the service seam over the in-memory sample library. The sibling plan "Back the library search with the Apple Music REST endpoint" (found 2026-10-09) points the same `AppleMusicService::search_songs` method at the signed-in Apple Music library; land that one second.

**Approach.**

- `src/apple_music.rs`: add `pub async fn search_songs(&self, query: &str) -> Result<Vec<Song>, AppleMusicError>`. Trim the query and reject a blank one with an `AppleMusicError` (the query guard the id lookups use: a blank search has nothing to match and must not read as an empty library). Answer from the sample library with a new private pure `sample_songs_matching(query)` that keeps every sample-library song whose title or artist contains the query case-insensitively, in library order. (The sample library indexes songs by album in a `HashMap`, so the filter walks the ordered `artists` → `albums_by_artist` → `songs_by_album` chain rather than the map's unspecified iteration order, which would shuffle the matches between calls.)
- `src/ui/mod.rs`:
  - Add `search_query: String` (the box's text, `String::new()`) and `search_active: bool` (`false`) to `WinampPlayer`, initialised in `WinampPlayer::new`.
  - Add `Message::SearchChanged(String)` (stores the text) and `Message::SearchSubmitted(String)`.
  - `SearchSubmitted` trims the query; a blank one is `Task::none()`. Otherwise set `search_query`, set `search_active = true`, set `current_view = CurrentView::Songs`, call `player.songs.clear()`, and issue `fetch_into` with `player.songs.begin_fetch()`, context `format!("searching the library for {query:?}")`, fetch `|service| async move { service.search_songs(&query).await }`, and the existing `Message::SongsLoaded` / `Message::SongsLoadFailed`. Reusing the songs list keeps the `BrowseReply` generation guard, the `SongsLoaded`/`SongsLoadFailed` arms, and `view_songs` unchanged.
  - Clear `search_active` in the `ArtistSelected` and `AlbumSelected` arms. In `Back`, when `search_active` is set, clear it and set `current_view = CurrentView::Artists` (leave the search) instead of the Songs → Albums step, so Back never opens the empty or stale album list the search was issued from.
  - Push `views::view_search_box(&player.search_query)` above `main_content` (after the Back/Retry buttons) so the box is on every browse screen.
- `src/ui/views.rs`: add `pub fn view_search_box(query: &str) -> Element<'_, Message>` — an iced `TextInput` with the `"Search library"` placeholder, `on_input(Message::SearchChanged)`, `on_submit(Message::SearchSubmitted(query.to_string()))` (iced's submit callback carries a value, so the view captures the current box text), and the new `style::chrome_text_input_style`.
- `src/ui/style.rs`: add `chrome_text_input_style(status) -> text_input::Style` using the base-skin face, the light top-left bevel edge (iced's text-input style carries no shadow, so only the top-left half is drawn), and light text, mirroring `chrome_button_style`; focus sinks the face and reverses the edge.
- Tests:
  - `src/apple_music/tests.rs`: `search_songs_matches_the_sample_library_by_title_or_artist` (a title-only match, an artist-only match, case-insensitive, a match spanning two albums in library order, and a non-match absent); `search_songs_rejects_a_blank_query`.
  - `src/ui/tests.rs`: `search_submitted_replaces_the_songs_list_with_the_matches` (drive `Message::SearchSubmitted` through `update` and `drive_task`, then assert `player.songs.items` holds the matches and `current_view == CurrentView::Songs`); `search_submitted_with_a_blank_query_is_a_no_op`; `search_changed_stores_the_box_text`; `back_leaves_an_active_search_for_the_artists_list`; `entering_a_browse_level_clears_the_active_search`.
  - `src/ui/views.rs`'s suite: a `view_search_box` construction test like the other widget-construction tests; `src/ui/style.rs`'s suite: pin the text-input style's face, text, and bevel edges like `chrome_button_style_active_is_raised_chrome`.

**Files touched.** `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views.rs`, `src/ui/style.rs`, `src/ui/tests.rs`.

**Acceptance criteria.**

- `make check` passes.
- `AppleMusicService::search_songs("...")` returns the sample-library songs whose title or artist contains the query case-insensitively; a blank query returns an error naming the query.
- Driving `Message::SearchSubmitted(query)` through `update` and the returned task leaves `current_view == CurrentView::Songs`, stores the matches in `player.songs.items`, and marks the search active; a blank query changes nothing.
- Driving `Message::Back` while a search is active returns to `CurrentView::Artists` and clears the active flag; `Message::ArtistSelected` and `Message::AlbumSelected` also clear it.
- `views::view_search_box` builds an element with the `Search library` placeholder whose input callback is `SearchChanged` and whose submit callback is `SearchSubmitted` carrying the box text.

### Drive the volume slider through the audio backend (found 2026-10-09, done 2026-10-09)

**Goal.** Moving the volume slider, or pressing the up/down arrow keys, changes the loudness of the playing preview, and a preview starts at the slider's volume. Today `Message::VolumeChange`, `VolumeUp`, and `VolumeDown` only write `AppState::volume`; `AudioOutput` has no volume method, so every preview plays at rodio's full-volume default no matter where the slider sits. This is the volume twin of the just-planned transport wiring and the same seam gap: only `play_track` reaches the backend.

**Approach.**

- `src/audio.rs`:
  - Add `set_volume(&self, volume: f32) -> Result<(), AppleMusicError>` to `AudioOutput` (doc: sets the output gain, `1.0` is full volume; it applies to the current player and to the next `play`, and does nothing when nothing is loaded).
  - `SilentOutput::set_volume` logs and returns `Ok`.
  - Add `Command::SetVolume(f32)`; `RodioOutput::set_volume` sends it.
  - In `run_worker`, hold the current gain in a `volume: f32` (initial `1.0`), map `Command::SetVolume(volume)` to storing it and calling `set_volume` on the current `rodio::Player` when one exists, and call `set_volume(volume)` on the player a `Command::Play` creates, so a gain set while stopped still applies to the next track.
  - Extend `silent_output_reports_success_for_every_command`, `audio_with_fallback_falls_back_to_silence_when_open_fails`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` to cover `set_volume`.
- `src/test_support.rs`: add `AudioCall::SetVolume(f32)`; `RecordingAudio::set_volume` records it; `FailingAudio::set_volume` returns `Ok(())` like its `pause`/`stop`, so a `play` failure still surfaces `play`'s own error. Because `f32` is not `Eq`, drop `Eq` from `AudioCall`'s derive (keep `Debug, Clone, PartialEq`); nothing requires `Eq` (the only uses are `assert_eq!` on a `Vec<AudioCall>`).
- `src/apple_music.rs`:
  - Add a synchronous `pub(crate) fn set_output_volume(&self, volume: f32) -> Result<(), AppleMusicError>` that forwards to `self.audio.set_volume(volume)` and touches no shared state. It is synchronous because the UI already stores the clamped value under its own lock (see `ui::mutate_volume`), and `AudioOutput::set_volume` is a non-blocking channel send; an async transition here would let a drag's messages race the state lock.
  - In `play_track`, capture the state's current volume before dropping the lock and apply it before the preview starts: `self.audio.set_volume(volume).and_then(|()| self.audio.play(url))`, keeping the existing `Err` rollback. This makes the first play use the slider's value (the `AppState` default `0.5`) rather than rodio's full-volume default.
- `src/ui/mod.rs`:
  - Add `mutate_volume(player, mutation) -> Task<Message>` beside `mutate_state`: apply `mutation` to `AppState` under one synchronous `blocking_lock`, read back the clamped `state.volume()`, forward that value to `player.apple_music_service.set_output_volume(...)`, and log a backend error. The state write stays synchronous so a drag remains responsive and ordered; the backend forward is a non-blocking channel send, so it runs on the UI thread.
  - Replace the `VolumeChange`, `VolumeUp`, and `VolumeDown` arms with `mutate_volume` calls (still `AppState::set_volume` and `AppState::nudge_volume`), and drop those three names from `mutate_state`'s doc list. `BalanceChange` is unchanged — rodio's `Player` has no panner.
- Tests:
  - `src/apple_music/tests.rs`: add `set_output_volume_forwards_to_the_audio_backend` (one `AudioCall::SetVolume`, `AppState::volume` unchanged). Update `play_track_starts_the_preview_through_the_audio_backend` to expect `[SetVolume(0.5), Play(url)]`.
  - `src/ui/tests.rs`: update `track_selected_plays_the_songs_preview_url` to expect `[SetVolume(0.5), Play(url)]`; add `volume_messages_forward_to_the_audio_backend`, which drives `VolumeChange`, `VolumeUp`, and `VolumeDown` through `update` over a `player_with_audio(RecordingAudio)` and asserts the recorded `SetVolume` values and `AppState::volume` (including a `f32::NAN` `VolumeChange` storing and forwarding the clamped `0.0`).

**Files touched.** `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`.

**Acceptance criteria.**

- `make check` passes.
- `RodioOutput::set_volume(0.4)` sends `Command::SetVolume(0.4)`; `SilentOutput::set_volume` returns `Ok`; a closed worker reports "the audio thread is gone" from `set_volume`.
- `AppleMusicService::set_output_volume(0.3)` records one `AudioCall::SetVolume(0.3)` and leaves `AppState::volume` unchanged.
- `play_track` with a preview records `SetVolume` carrying the state's current volume immediately before `Play`.
- Driving `Message::VolumeChange(0.2)` leaves `AppState::volume` at `0.2` and records `SetVolume(0.2)`; `Message::VolumeUp`/`VolumeDown` record the nudged, clamped value; an out-of-range or `NaN` `VolumeChange` stores and forwards the clamped value.

### Drive the transport's Play/Pause/Stop through the audio backend (found 2026-10-08, done 2026-10-09)

**Goal.** Pressing the transport's Play/Pause, Play, Pause, and Stop controls — and their X, C, and V keys — drives the injected `audio::AudioOutput` as well as `AppState`, so pausing or stopping silences the playing preview and Play resumes a paused preview or restarts a stopped one. Before this, only `AppleMusicService::play_track` reached the backend: the service's `pause` was dead code and there was no `resume` or `stop`, so the preview kept sounding after Stop and Pause. This is the wiring the audio-seam plan (PLANS.md `## Done`, 2026-10-08) explicitly deferred.

**Approach.**

- `src/audio.rs`: add `resume(&self) -> Result<(), AppleMusicError>` to `AudioOutput` (doc: resumes a paused player; no effect when nothing is loaded — rodio's `Player::play` resumes, it does not restart). Implement it for `SilentOutput` (log and `Ok`); add `Command::Resume`; `RodioOutput::resume` sends it; the `run_worker` loop maps `Command::Resume` to `player.play()` on the current `rodio::Player`. Extend `silent_output_reports_success_for_every_command`, `audio_with_fallback_falls_back_to_silence_when_open_fails`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` to cover `resume`.
- `src/test_support.rs`: add `AudioCall::Resume`; `RecordingAudio::resume` records it; `FailingAudio::resume` returns `Ok(())` like its `pause`/`stop`.
- `src/apple_music.rs`: made the service the single owner of each transport transition, reusing the existing `AppState` mutators so none goes dead:
  - Moved `pause` out of the `#[allow(dead_code)]` block, narrowed that attribute to `next_track`/`previous_track`, and made the four transport methods `pub(crate)` so `ui` can call them.
  - Added a `transport: Arc<tokio::sync::Mutex<TransportState>>` field, where `TransportState { loaded, last_url }` records whether the backend holds a paused preview and the most recent preview URL. Every transport method and `play_track` holds this lock across the flag read, the backend call, and the flag write, so two overlapping toggles serialize instead of both reading the same flag.
  - `pause` calls `self.audio.pause()?`, then `self.state.lock().await.pause()`.
  - `resume` resumes a loaded player; when the backend discarded it (after a `stop`) it replays `last_url`, so Play after Stop restarts rather than claiming playback with nothing loaded; when no preview has ever played it logs and leaves the flag alone. On success it calls `self.state.lock().await.play()`.
  - `stop` calls `self.audio.stop()?`, clears `loaded`, then `self.state.lock().await.stop()`.
  - `toggle_play_pause` reads `is_playing` under the transport lock, calls `audio.pause()` (then `AppState::toggle_playing`) or `resume`, all under that one lock.
  - `play_track` also takes the transport lock and records `loaded`/`last_url`, clearing both when the backend rejects the preview.
- `src/ui/loading.rs`: add `transport_into(service, context, action) -> Task<Message>`: clone the service, await `action(service)`, report a backend error through a new pure `transport_failure_report(context, err)`, and map completion to `Message::TransportSettled`. Unlike `play_into` it carries no track or generation — a pause/stop cannot be superseded by a newer play — applies no timeout (the service's transport methods only lock and send a command to the audio worker), and does not prune the title index.
- `src/ui/mod.rs`: add the `Message::TransportSettled` variant with a no-op arm (its purpose is to make iced re-render after the async transition). Replace the four synchronous arms `PlayPause`, `Play`, `Pause`, and `Stop` with `transport_into(...)` calls to `toggle_play_pause`/`resume`/`pause`/`stop`; drop those four from `mutate_state`'s doc list and import `transport_into`.
- Tests:
  - `src/apple_music/tests.rs`: kept the two existing `pause` tests; added `resume_resumes_the_audio_backend`, `stop_stops_the_audio_backend`, `toggle_play_pause_pauses_while_playing`, `toggle_play_pause_resumes_while_paused`, `resume_after_stop_restarts_the_preview`, `resume_without_a_preview_leaves_the_transport_stopped`, and `concurrent_toggles_serialize_on_the_transport_lock` (a gated backend proves the second of two overlapping toggles waits on the transport lock; the test fails when the lock is dropped before the backend call).
  - `src/ui/tests.rs`: converted `play_pause_toggles_is_playing`, `stop_clears_is_playing`, and `play_and_pause_set_the_playback_flag_explicitly` into `#[tokio::test]`s that drive the returned task with `drive_task` before reading state, and updated their stale `blocking_lock` comments. Added `transport_messages_drive_the_audio_backend`, which drives each of the four messages through `update` and `drive_task` over a `player_with_audio(RecordingAudio)` and asserts the recorded calls (`Stop`, `Resume`, `Pause`, and a `Pause` from the toggle).
  - `src/ui/loading.rs`'s suite: added a pure-function test for `transport_failure_report`, like the existing `play_failure_report` test.
- `README.md`: updated the status and usage paragraphs to describe Pause/Stop driving the backend and Play resuming a paused preview or restarting a stopped one.

**Files touched.** `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/loading.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- With a `RecordingAudio`, `AppleMusicService::stop` records one `AudioCall::Stop` and clears `AppState::is_playing` while keeping `current_track`; `resume` after a paused preview records one `AudioCall::Resume` and sets `is_playing`; `toggle_play_pause` records `Pause` from playing and `Resume` from paused.
- `RodioOutput::resume` sends `Command::Resume`, and `SilentOutput::resume` returns `Ok`.
- `resume` after `stop` replays the remembered preview URL and sets `is_playing`; `resume` with no preview ever played records no call and leaves `is_playing` false.
- Two overlapping `toggle_play_pause` calls serialize on the transport lock: the second observes the first's flag write, so the pair ends playing with one `Pause` then one `Resume`.
- Driving `Message::Stop`, `Message::Pause`, `Message::Play`, and `Message::PlayPause` through `update` and the returned task leaves `AppState::is_playing` matching the control (false after Stop/Pause, true after Play when a preview is loaded, toggled after PlayPause when one is) and records the matching `AudioCall`.
- `transport_failure_report` names the context and includes the backend error.

### Document `make test-one` and report a build failure distinctly from a mistyped name (found 2026-10-08, done 2026-10-08)

`make test-one TEST=<name>` (added 2026-10-08) runs only the tests whose
names contain `<name>`, but the README's Development section never mentioned
it, and the target's no-match guard reported a build failure and a mistyped
name through one hedged message: `no test name contains 'X' (or the test build
failed)`.

**Goal.** The README documents the target, and `make test-one` reports a build
failure and a mistyped name distinctly.

**Approach.**

- `Makefile`: capture the `cargo test --list` output and check its exit status
  separately, so a failed build reprints the compiler diagnostics under a
  `make test-one: the test build failed` line, while a successful build with
  no match reports `no test name contains '<name>'`. Update the target's
  comment block to match. The cargo command is the `CARGO` variable so
  `test-one-guard-test` can substitute a stub.
- `README.md`: add `make test-one TEST=<name>` to the individual stages in the
  Development section.
- `Makefile`: `test-one-guard-test` invokes the recipe through `$(MAKE)` with
  an empty `TEST=` (the usage guard), `CARGO=false` (a failed build), and
  `CARGO=true` (a successful build that names no test), asserting each cause
  is reported distinctly, so removing any of the three guards fails
  `make check`.

**Acceptance criteria.**

- `make check` passes.
- `make test-one TEST=zzz_no_such_test` reports the no-match cause rather
  than the build-failure one.
- `make test-one-guard-test` exercises the empty-`TEST` usage guard and both
  the build-failure and no-match branches of the recipe and fails if any
  message regresses.

**Files touched.** `Makefile`, `README.md`.

### Reject a blank preview URL at the playback seam (found 2026-10-08, done 2026-10-08)

`AppleMusicService::play_track` validates its track id — a blank or
control-character id is rejected with a named error before shared state is
touched — but passed `preview_url` straight to the audio backend. A caller
handing `Some("")` (or only whitespace) made the backend attempt a request to
an empty URL, so the user saw a transport error that did not name the actual
defect. The REST layer already maps a blank preview to `None`, but the seam is
public and documented for a real backend to fill, so the guard belongs at the
seam too.

**Goal.** `play_track` rejects a present-but-blank `preview_url` with a named
`AppleMusicError` before committing state, while `None` keeps its documented
"no playable asset" behavior (state committed, no audio).

**Approach.**

- `src/apple_music.rs`: add `ensure_preview_url_is_valid(Option<&str>)`, which
  returns `Ok` for `None` or a non-blank URL and names the offending value
  (quoted with `{url:?}`) for a blank one; call it in `play_track` after
  `ensure_id_is_valid`, before the state lock. Update `play_track`'s doc comment
  and its `# Errors` list.

**Acceptance criteria.**

- `make check` passes.
- A test drives `play_track` with `Some("")` and `Some("   ")` mid-playback and
  sees the named error with shared state untouched.

**Files touched.** `src/apple_music.rs`, `src/apple_music/tests.rs`.

### Show the playing track's artist in the Now Playing bar (found 2026-10-08, done 2026-10-08)

Classic Winamp's main-window marquee reads "Artist - Title", and the Apple
Music browse response already carries each track's `attributes.artistName`.
Nostalgia's `Song` drops it — `rest::Attributes` reads only `name`,
`durationInMillis`, and `previews` — so the Now Playing bar can only name the
title. This carries the artist onto the model and shows it beside the title.
The playlist editor's rows keep their `title + length` layout, and the artist
never reaches the transport, browse navigation, volume, balance, or
equalizer.

**Goal.** `Song` carries `artist: String` — the performing artist's display
name, `""` when the source supplied none. The REST browse client parses
`attributes.artistName` into it; the sample library and test fixtures fill
it. The Now Playing bar renders `<artist> - <title>` when the artist is known
and falls back to the title alone when it is empty. No new dependency.

**Approach.**

- `src/library.rs`: add `pub artist: String` to `Song`, documented as the
  performing artist's display name with `""` meaning the source supplied
  none. Add `"artist": "The Sample Band"` to `song_payload()` and to
  `song_required_fields_payload()`, so `assert_every_field_required` still
  covers every required key and the round-trip test pins the field name.
- `src/apple_music/rest.rs`: add `#[serde(rename = "artistName")]
  artist_name: Option<String>` to `Attributes`, and set `artist:
  resource.artist_name.clone().filter(|name| !name.trim().is_empty())
  .unwrap_or_default()` in the `get_songs_from_album` mapping — an absent or
  blank name maps to `""`, the model's "no artist" sentinel, matching the
  `duration_ms`/`preview_url` treatment of missing source data.
- `src/sample_library.rs`: every `Song` literal gains `artist` — "The Sample
  Band" for the `album-1`/`album-2` songs, "Echo Chamber" for the `album-3`
  song — matching the album's `artist_id`. Add a `songs_carry_the_documented_artists`
  test mirroring `songs_carry_the_documented_durations`, pinning every song's
  artist in library order.
- `src/test_support.rs`: `sample_song`, `stepping_songs`,
  `single_song_album`, and `second_album_songs` gain
  `artist: "The Sample Band".to_string()`.
- The remaining `Song { … }` literals (`src/apple_music/tests.rs`,
  `src/apple_music/rest/tests.rs`, `src/ui/tests.rs`) gain the field.
- `src/apple_music/rest/tests.rs`: the `songs_from_album` fixture carries
  `"artistName": "The Sample Band"` and its expected `Song` carries that
  artist; a companion test pins an absent `artistName` to `""`.
- `src/ui/views.rs`:
  - Add `pub artist: String` to `KnownTrack`, documented beside `title` and
    `duration_ms`.
  - Add `pub fn now_playing_artist<'a>(tracks: &'a HashMap<String,
    KnownTrack>, current_track: Option<&str>) -> Cow<'a, str>`: the known
    track's artist borrowed from the map, or the `""` literal when the track
    is unknown or none is current. It mirrors `now_playing_label`, so the
    artist adds no per-frame `String` allocation.
  - Change `view_now_playing(label: Cow<'_, str>, artist: Cow<'_, str>, time:
    String)` to push the artist and a `" - "` separator before the title only
    when `artist` is non-empty, so an artist-less track still reads `Now
    Playing: <title>`.
  - In the `#[cfg(test)] mod tests`: the `known_tracks` helper takes
    `(id, artist, title)` triples; add value and borrow tests for
    `now_playing_artist`; pass the artist to the `view_now_playing` construct
    call.
- `src/ui/mod.rs`:
  - Add a `WinampPlayer::now_playing_artist` method mirroring
    `now_playing_label`.
  - The `TrackSelected` arm records `artist: song.artist.clone()` in the
    `KnownTrack` entry.
  - `view` resolves the artist in the same state-lock block as the label and
    passes it to `view_now_playing`.

**Files touched.** `src/library.rs`, `src/apple_music/rest.rs`,
`src/apple_music/rest/tests.rs`, `src/sample_library.rs`,
`src/test_support.rs`, plus the `Song` literals and the `KnownTrack` fixture
in `src/apple_music/tests.rs`, `src/ui/tests.rs`, and `src/ui/views.rs`, and
the Now Playing bar wiring in `src/ui/mod.rs`.

**Acceptance criteria.**

- `make check` passes.
- A `get_songs_from_album` stub response carrying `"artistName": "The Sample
  Band"` maps that string to `Song::artist`; a response omitting it (or
  carrying a blank one) maps to `""`.
- Every `sample_library()` song carries its album's artist name, pinned by
  `songs_carry_the_documented_artists`.
- `now_playing_artist` returns the known track's artist, `""` for an unknown
  id, and `""` when no track is current; the known case borrows from the map
  (`Cow::Borrowed`).
- `view_now_playing` builds with a non-empty artist and with an empty one.
- The `now_playing_label` value and borrow tests still pass unchanged, and
  the browse-away regression
  (`now_playing_label_keeps_the_track_name_after_browsing_to_another_album`)
  still passes.

### Bound the audio worker's preview download with a timeout (found 2026-10-08, done 2026-10-08)

The audio seam's worker downloaded the preview with `ureq::get(url)`, whose
default agent leaves every network timeout `None`. A server that accepted the
connection and then stalled blocked the worker thread forever, and because the
worker serves commands one at a time, every later play, pause, and stop queued
behind it. The REST client already bounded its requests; this applies the same
bound to the app's other network call.

**Goal.** A preview download that stalls fails with a logged error within a
fixed bound, so the worker keeps serving later commands, and the production
agent still reuses its pooled connection.

**Approach.**

- `src/audio.rs`: add `PREVIEW_TIMEOUT: Duration = Duration::from_secs(30)`,
  matching the REST client's `REQUEST_TIMEOUT`.
- Add `preview_agent() -> &'static ureq::Agent` (a `OnceLock`, built by
  `agent_with_timeout`) so consecutive plays reuse one pooled agent, and
  `agent_with_timeout(timeout)`, split out so a test can bound a download
  against a stalled loopback server without waiting out the production 30s.
- `download_and_decode(agent, url)` takes the agent and calls `agent.get(url)`;
  the worker passes `preview_agent()`. The download error already logs on the
  worker, so a timeout reads like any other download failure.

**Acceptance criteria.**

- `make check` passes.
- A stalled loopback preview server makes `download_and_decode` return `Err`
  within a short injected bound, asserted from a worker thread so an unbounded
  download fails the test instead of hanging the suite.

**Files touched.** `src/audio.rs`.

### Add the audio-output seam and a rodio backend (found 2026-10-08, done 2026-10-08)

The audio-output decision (QUESTIONS.md `## Answered`, 2026-10-08) scopes
playback to the Apple Music **preview**; this lands the playback half behind an
injectable seam, with `rodio` as the production backend. Rodio 0.22's API
differs from the names first sketched here — the device is a `MixerDeviceSink`
from `DeviceSinkBuilder::open_default_sink()`, playback goes through
`rodio::Player`, and the preview decodes with `Decoder::new_mp4` — so the
approach below names the API that landed. The transport's Pause and Stop
buttons still update only UI state; wiring them to the service's `pause`/`stop`
seam is left to a later plan.

**Goal.** `AppleMusicService` plays real audio through an injectable
`AudioOutput` seam; the production implementation uses `rodio`, `play_track`
starts the selected song's preview, `pause` pauses it, and a recording stub
stands in under test. Depends on "Carry the Apple Music preview URL on
`Song`" (the URL must be on `Song` first).

**Approach.**

- `Cargo.toml`: `rodio = { version = "0.22.2", default-features = false,
  features = ["playback", "mp4"] }`. The `mp4` feature already pulls
  `symphonia-isomp4` and `symphonia-aac` (Apple's preview is AAC in M4A), and
  dropping the default features leaves out recording and the unused decoders.
- A new `audio` module (the `AudioOutput` trait, `SilentOutput`, `RodioOutput`, and `audio_with_fallback` symbols), registered by `pub mod audio;` in `src/main.rs`:
  - `pub trait AudioOutput: Send + Sync` with `fn play(&self, url: &str) ->
    Result<(), AppleMusicError>`, `fn pause(&self) -> Result<(),
    AppleMusicError>`, and `fn stop(&self) -> Result<(), AppleMusicError>`.
  - `pub struct SilentOutput`, whose methods log and return `Ok(())`: the
    fallback when no device opens, and the production stand-in for the browse
    tests.
  - `pub struct RodioOutput`, owning a `Mutex<std::sync::mpsc::Sender<Command>>`
    to a worker `std::thread`. Rodio 0.22 opens the device through
    `DeviceSinkBuilder::open_default_sink()` (a `MixerDeviceSink`) and plays
    through `rodio::Player`; keeping both on the worker makes the seam
    `Send + Sync` regardless of the platform handle's thread bounds. The
    sender sits behind a `Mutex` because the standard sender is `Send` but not
    `Sync`. `Command` is `Play(String)`, `Pause`, `Stop`. The worker downloads
    the URL with `ureq`, buffers it, and decodes it with
    `rodio::Decoder::new_mp4(Cursor<Vec<u8>>)`, appending to a fresh `Player`;
    a download/decode failure is logged with `eprintln!` (the trait call
    itself fails only when the worker is gone). `RodioOutput::new() ->
    Result<Self, AppleMusicError>` opens the default device on the worker and
    reports success/failure back through a `std::sync::mpsc` channel, so a
    device-less machine gets `Err` rather than a panic.
  - `pub(crate) fn audio_with_fallback(open: impl FnOnce() ->
    Result<Arc<dyn AudioOutput>, AppleMusicError>) -> Arc<dyn AudioOutput>`:
    returns the opened output, or a `SilentOutput` when `open` fails. Pure and
    injectable so the fallback is testable without a device.
- `src/apple_music.rs`:
  - Add `audio: Arc<dyn audio::AudioOutput>` to `AppleMusicService`.
  - `with_transport` builds `SilentOutput`; add
    `with_audio(state, transport, audio)` for the playback tests; `new(state)`
    builds `RodioOutput` through `audio_with_fallback`, then calls
    `with_audio`.
  - `play_track(&self, track_id: &str, preview_url: Option<&str>, is_current:
    …)`: keep the id guard and the `is_current`-guarded state commit, then call
    `self.audio.play(url)` when `preview_url` is `Some` and propagate an audio
    error. `pause` calls `self.audio.pause()` before clearing `is_playing`.
  - Update the doc comments that call playback a shared-state-only stub.
- `src/ui/mod.rs`: the `TrackSelected` arm passes `song.preview_url.as_deref()`
  to `play_track`.
- Tests: `AudioCall`/`RecordingAudio`/`FailingAudio` live in
  `src/test_support.rs`, shared by the `apple_music` and `ui` suites. The
  `apple_music` suite asserts `play_track(…, Some(url), …)` records one
  `Play(url)` and commits state, `play_track(…, None, …)` records none and
  still commits, a failing backend's error propagates, and `pause` records
  `Pause`. The `ui` suite drives `TrackSelected` end to end and asserts the
  backend saw the selected `Song::preview_url`. The service constructors in
  both suites use `with_transport` (silent audio), so no test opens an audio
  device.

**Files touched.** `Cargo.toml`, `README.md`, `src/main.rs`, the new
`src/audio.rs` (the `AudioOutput`, `SilentOutput`, `RodioOutput`, and
`audio_with_fallback` symbols), `src/apple_music.rs`,
`src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/mod.rs`,
`src/ui/tests.rs`.

**Acceptance criteria.**

- `make check` passes, including on a machine with no audio device (the
  `RodioOutput` tests cover `new` returning `Err` without panicking; they do
  not assert a device is present).
- `audio_with_fallback` with a failing opener returns a `SilentOutput` whose
  `play`/`pause`/`stop` return `Ok`.
- With a recording `AudioOutput`, `play_track("song-1",
  Some("https://example.test/preview.m4a"), || true)` records one `Play` with
  that URL and sets `AppState::current_track`/`is_playing`.
- `play_track(…, None, …)` records no `Play` but still commits the state
  transition.
- `pause` records a `Pause`.
- The UI passes the selected `Song::preview_url` to `play_track`.

### Reject a `make test-one TEST=<name>` that matches no test name (found 2026-10-08, done 2026-10-08)

`make test-one` forwarded its filter to libtest, which exits `0` when the
filter matches zero tests — so a typo (`TEST=clamp_with_nan_fallback`, a
function name rather than a behavior name) printed "running 0 tests" and a
green result, reading as a pass.

**Goal.** A `TEST` that matches no test name fails the target with a message,
while a real name still runs only the matching tests.

**Approach.**

- `Makefile`: `test-one` lists the tests the filter would select
  (`cargo test --locked -- --list "$(TEST)"`) and requires at least one line
  naming a test before the filtered run. The match is anchored to libtest's
  test-name lines (`: test$`) so the trailing summary (`N tests, 0
  benchmarks`) cannot satisfy the guard. The list command's output is
  captured together with its stderr and its exit status checked separately,
  so a build failure is reported as such rather than as a mistyped name. The
  match pattern lives in the `TEST_LIST_HAS_TEST` variable.
- `Makefile`: `test-one-guard-test` feeds that pattern a synthetic list with
  a test line and a summary-only list, asserting the first matches and the
  second does not; it is added to `check`, so a guard that reintroduces the
  summary-line false pass fails the gate.

**Acceptance criteria.**

- `make check` passes.
- `make test-one TEST=<name>` with a real test name runs only the matching
  tests and exits zero.
- `make test-one` with no `TEST` exits non-zero with a usage line.
- `make test-one TEST=benchmarks` (a value that appears only in libtest's
  summary line) and `make test-one TEST=no_such_test` each exit non-zero with
  a "no test name contains" message and run no tests.
- `make test-one-guard-test` passes.

- Carry the Apple Music preview URL on `Song` (found 2026-10-08, done 2026-10-08; commit 8e9faf3)
- Name the page when a paginated Apple Music browse fails (found 2026-10-08, done 2026-10-08; commit 71853ef)
- Show each track's length in the playlist editor (found 2026-10-08, done 2026-10-08; commit f695290)
- Read a later Apple Music error entry when the first names no cause (found 2026-10-08, done 2026-10-08; commit bb1696d)
- Follow a browse collection's next page so a large library is read in full (found 2026-10-08, done 2026-10-08; commit a330d17)
- Show the current track's duration in the Now Playing bar (found 2026-10-08, done 2026-10-08; commit 98aa4b3)
- Log a notice when a browse response has an unread next page (found 2026-10-08, done 2026-10-08; commit 210deb5)
- Add a `make test-one` target for running one focused test (found 2026-10-08, done 2026-10-08; commit 277625f)
- Name the user-token shape defect when the sign-in callback is rejected (found 2026-10-08, done 2026-10-08; commit db297ed)
- Name the envelope, not "invalid JSON", when a valid-JSON body does not match (found 2026-10-08, done 2026-10-08; commit ad94f37)
- Report just the HTTP status when an Apple Music error body's cause is blank (found 2026-10-08, done 2026-10-08; commit 1723afd)
- Add a Winamp Shuffle toggle that randomizes Next (found 2026-10-08, done 2026-10-08; commit 7c299a7)
- Reject a blank resource id from an Apple Music browse response (found 2026-10-08, done 2026-10-08; commit e19867b)
- De-duplicate the query label in a failed browse report (found 2026-10-08, done 2026-10-08; commit 196e68a)
- Add a Winamp balance slider beside the volume slider (found 2026-10-08, done 2026-10-08; commit 8ee7986)
- Add the Winamp title-bar clutter bar and shade button (found 2026-10-08, done 2026-10-08; commit 7bcb23f)
- Answer the browse queries from the Apple Music REST API when signed in (found 2026-10-08, done 2026-10-08; commit 4a4f2e6)
- Add the Apple Music REST browse client (found 2026-10-08, done 2026-10-08; commit c63808d)
- Wire the MusicKit session into `AppleMusicService` (found 2026-10-08, done 2026-10-08; commit cf514ca)
- Add the MusicKit loopback authorization module (found 2026-10-08, done 2026-10-08; commit c15f6b6)
- Add classic Winamp main-window keyboard shortcuts for transport and volume (found 2026-10-08, done 2026-10-08; commit 97f6658)
- Add Winamp window-shade ("roll up") mode (found 2026-10-08, done 2026-10-08; commit 1e46f04)
- Add Winamp equalizer preset curves and a preset pick list (found 2026-10-07, done 2026-10-07; commit f7647ca)
- Add a Winamp custom title bar and drop the OS window frame (found 2026-10-07, done 2026-10-07; commit 81eb991)
- Frame the browse list as a sunken Winamp playlist editor (found 2026-10-07, done 2026-10-07; commit 74bbe47)
- Style the volume and equalizer sliders as sunken Winamp grooves with raised chrome thumbs (found 2026-10-07, done 2026-10-07; commit 4858b11)
- Style the transport buttons as raised Winamp chrome (found 2026-10-07, done 2026-10-07; commit dd0a5c5)

- Add a Winamp two-tone bevel layer and frame the Now Playing and equalizer panels (found 2026-10-06, done 2026-10-06; commit 8984e90)
- Add a Winamp 2.x base-skin palette and apply it as the app theme (done 2026-10-06; commit 9d74276)
- Add the Winamp equalizer panel: on/off, preamp, and ten band sliders (done 2026-10-06; commit 82b9757)
- Add a Repeat toggle to the transport controls (done 2026-10-06; commit d2f72bc)
- Highlight the currently playing song in the Songs browse view (done 2026-10-06; commit 7e6d83b)
- Add a Stop button to the transport controls (done 2026-10-06; commit 8e91e1e)
- Show the song's title in the Now Playing bar, not its raw id (done 2026-10-05; commit 7d29183)
- Make Previous/Next step through the songs of the current album (done 2026-10-05; commit 4c8e223)
- Add a working volume slider to the transport controls (done 2026-10-05; commit 7029804)
- Make the app build on iced 0.14 and render a browsable sample library (done 2026-10-04; commit 5af3f08)
