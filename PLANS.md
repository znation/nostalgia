# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

## Planned

### Add the Winamp equalizer panel: on/off, preamp, and ten band sliders

Found by plan 2026-10-06.

**Goal.** The equalizer is the largest classic Winamp UI element the app still
lacks — the README's own mockup (`docs/screenshots/equalizer.png`) shows it
docked under the main window, but no equalizer code exists (`grep -ri equaliz
src` finds nothing). Land the equalizer's core controls as a panel in the
existing single window: an EQ on/off toggle, a preamp slider, and ten vertical
band sliders (60 Hz–16 kHz, ±12 dB) whose values live in the shared `AppState`,
mirroring how `volume` and `repeat` already work. Out of scope (later plans):
preset curves, a separate/docked OS window, window-shade mode, and any real
audio processing (the Apple Music stub has no audio pipeline yet).

**Approach.**

- New file `src/equalizer.rs`, the equalizer data model — a sibling of
  `library.rs` and `state.rs` so both the shared state and the UI can depend on
  it without a `state` ← `ui` cycle:
  - `pub const BAND_FREQUENCIES: [&str; 10] = ["60", "170", "310", "600", "1K", "3K", "6K", "12K", "14K", "16K"];`
  - `pub const BAND_COUNT: usize = BAND_FREQUENCIES.len();`
  - `pub const GAIN_MIN_DB: f32 = -12.0;` and `pub const GAIN_MAX_DB: f32 = 12.0;`
  - `#[must_use] pub fn clamp_gain(gain: f32) -> f32` — `clamp` to the range,
    mapping NaN to `0.0`, exactly as `state::clamp_volume` does for volume.
  - A `#[cfg(test)] mod tests` pinning `clamp_gain` at both bounds, in range,
    and NaN, plus `BAND_FREQUENCIES.len() == BAND_COUNT`.
- `src/main.rs`: add `mod equalizer;` beside the other `mod` declarations.
- `src/state.rs`:
  - Add to `AppState`: `pub eq_enabled: bool`, `pub eq_preamp: f32`,
    `pub eq_bands: [f32; equalizer::BAND_COUNT]`.
  - Extend the manual `Default` impl: `eq_enabled: false`, `eq_preamp: 0.0`,
    `eq_bands: [0.0; equalizer::BAND_COUNT]` (flat), and update the
    "Initial state" doc comment.
  - Add `toggle_equalizer(&mut self)` (flips `eq_enabled`, like
    `toggle_repeat`), `set_eq_preamp(&mut self, gain: f32)` (stores
    `equalizer::clamp_gain(gain)`), and `set_eq_band(&mut self, band: usize,
    gain: f32)` (clamps and stores into `eq_bands[band]`, ignoring an
    out-of-range `band` via `get_mut`).
  - Tests: default eq is off/flat; `toggle_equalizer` flips only `eq_enabled`;
    both setters clamp an out-of-range value and `set_eq_band` ignores an
    out-of-range band index; none of the eq mutators change
    `current_track`/`volume` (reuse the existing
    `assert_keeps_track_and_volume` helper).
- `src/ui/views.rs`:
  - Import `VerticalSlider` from `iced::widget` and `BAND_COUNT` /
    `BAND_FREQUENCIES` / `GAIN_MIN_DB` / `GAIN_MAX_DB` from `crate::equalizer`.
  - `fn eq_enabled_label(enabled: bool) -> &'static str` returning `"EQ: On"`
    / `"EQ: Off"`, mirroring `repeat_label`.
  - `pub fn view_equalizer(enabled: bool, preamp: f32, bands: &[f32; BAND_COUNT]) -> Element<'static, Message>`
    — a `Column` of:
    - a header `Row` with `labeled_button(eq_enabled_label(enabled), Message::ToggleEqualizer)`;
    - a preamp `Row` of `Text::new("Preamp")` and
      `Slider::new(GAIN_MIN_DB..=GAIN_MAX_DB, preamp, Message::EqPreampChange).step(1.0).width(Length::Fixed(150.0))`;
    - a bands `Row` of `BAND_COUNT` `Column`s, each a
      `VerticalSlider::new(GAIN_MIN_DB..=GAIN_MAX_DB, bands[i], move |gain| Message::EqBandChange(i, gain)).step(1.0).height(Length::Fixed(100.0))`
      above `Text::new(BAND_FREQUENCIES[i]).size(12)`.
  - Tests: `eq_enabled_label` mirrors the flag; `view_equalizer` constructs for
    enabled/disabled and both gain endpoints (mirroring the existing
    transport-controls construction test).
- `src/ui/mod.rs`:
  - Add `Message::{ToggleEqualizer, EqPreampChange(f32), EqBandChange(usize, f32)}`.
  - `update` arms: `ToggleEqualizer => mutate_state(player, AppState::toggle_equalizer)`;
    `EqPreampChange(gain) => mutate_state(player, |s| s.set_eq_preamp(gain))`;
    `EqBandChange(band, gain) => mutate_state(player, |s| s.set_eq_band(band, gain))`.
  - In `view`, read `eq_enabled`/`eq_preamp`/`eq_bands` in the existing
    state-lock block and push
    `views::view_equalizer(eq_enabled, eq_preamp, &eq_bands)` after
    `views::view_transport_controls(..)`.
- `src/ui/tests.rs`: add update tests — `Message::ToggleEqualizer` flips the
  shared flag; `EqPreampChange`/`EqBandChange` store clamped values (e.g.
  `EqBandChange(0, 99.0)` stores `12.0`).
- `README.md`: refresh the Status and Usage lines to describe the equalizer
  panel; leave the `tumwater:prompt` block untouched.

**Files touched.** `src/equalizer.rs` (new), `src/main.rs`, `src/state.rs`,
`src/ui/views.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes (`cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`).
- The new `equalizer`, `state`, and `views` unit tests and the `ui` update
  tests listed above pass.
- No `dead_code`/unused warnings: every new constant, field, and method is read
  by the UI or its tests.
- `cargo run`: below the transport row the window shows an "EQ: Off" button, a
  Preamp slider, and ten band sliders labelled `60`…`16K`; pressing the button
  changes its label to "EQ: On", and dragging any slider moves it (manual check
  — build + tests are the primary gate).


## Done

### Add a Repeat toggle to the transport controls (done 2026-10-06)

Found by plan 2026-10-06.

**Goal.** Match Winamp's Repeat control: a Repeat button in the transport row that switches Previous/Next through the current album's songs between wrap-around (Repeat on) and stop-at-the-edge (Repeat off) stepping. Today `transport::next_track_id` and `transport::previous_track_id` always wrap at the album's ends, so the current default is indistinguishable from Repeat-on; this feature makes wrapping explicitly opt-in and starts Repeat off, as Winamp does.

**Approach.**
- `src/state.rs`: add `pub repeat: bool` to `AppState` (initialised `false` in `impl Default`), plus a `pub fn toggle_repeat(&mut self)` mirroring `toggle_playing`. Update `default_state_is_stopped_at_half_volume` to assert `!state.repeat`; add `toggle_repeat_flips_only_the_repeat_flag` (flips both ways, leaves `current_track`, `is_playing`, `volume` untouched). The `stop_clears_playing_flag_and_keeps_current_track` test builds `AppState` with `..Default::default()`, so it needs no change.
- `src/ui/transport.rs`: add a `repeat: bool` parameter to `stepped_track_id`, `next_track_id`, and `previous_track_id`. With `repeat == false`, a step that would cross the boundary stays on the edge instead of wrapping — Next from the last song returns the last song's id, Previous from the first returns the first's — so the button re-lands on the current track rather than moving or doing nothing. With `repeat == true`, and in every no-current / empty-list / single-song case, behavior is unchanged: the no-current branch still lands on the forward/backward edge. Update the two wrap tests (`next_wraps_from_last_to_first`, `previous_wraps_from_first_to_last`) to pass `true`; give the other existing transport tests the new argument (any value where the expectation does not depend on repeat); add `next_stays_on_last_without_repeat` and `previous_stays_on_first_without_repeat`.
- `src/ui/views.rs`: `view_transport_controls` gains a `repeat: bool` parameter and pushes a Repeat button (via the existing `labeled_button`) after the Next button; add `fn repeat_label(repeat: bool) -> &'static str` beside `play_pause_label`, returning "Repeat: Off"/"Repeat: On", and pin it in `repeat_label_mirrors_repeat_state`. Extend `transport_controls_construct_for_both_play_states_and_volume_endpoints` to also loop over `[false, true]` for repeat.
- `src/ui/mod.rs`: add `Message::ToggleRepeat`; its update arm routes to `mutate_state(player, AppState::toggle_repeat)`. `step_track`'s `step` parameter becomes `fn(&[Song], Option<&str>, bool) -> Option<String>` and the Next/Previous arms pass `state.repeat` (read under the existing lock), so the arm wiring keeps the shared flag in step. `view` reads `repeat` in its lock block and passes it to `views::view_transport_controls`. Add wiring tests: `toggle_repeat_flips_shared_state` (drive `Message::ToggleRepeat` through `update`, like `play_pause_toggles_is_playing`) and a stepping-wiring test driving `Message::NextTrack` from the last song — wraps when `state.repeat` is true, stays on the last song when false.

**Files touched.** `src/state.rs`, `src/ui/transport.rs`, `src/ui/views.rs`, `src/ui/mod.rs` (each including its `#[cfg(test)]` module).

**Acceptance criteria.**
- `AppState::default().repeat` is `false`; `toggle_repeat` flips it and nothing else.
- `next_track_id`/`previous_track_id` with `repeat = false` stop at the album's edge (return the current edge song's id); with `repeat = true`, and in every no-current/empty/single-song case, they behave exactly as before the change.
- The transport row renders a Repeat button whose label tracks the shared `repeat` flag, and pressing it flips that flag in shared state.
- `make check` is green: `cargo fmt --check`, `cargo clippy --all-targets`, and the full `cargo test` suite pass.


### Highlight the currently playing song in the Songs browse view (done 2026-10-06)

Found by plan 2026-10-06.

**Goal.** Winamp's playlist editor draws a highlighted bar on the track that is
currently playing, so you can see where in the list the music is and watch it
move as you step. This player has no such cue: the Songs view renders every row
identically, so after clicking a song (or pressing Next/Previous) there is no
way to tell which song in the album is current. Mark the row whose id equals the
shared `current_track` — a `▶` prefix on its title and a highlighted button
background — so the album list reads as a playlist. The marker is pure data
derived state (current id vs. the displayed songs), so it stays in the view
layer like the rest of `views.rs` and never touches the service seam.

**Approach.** All changes are in the view layer; `AppleMusicService`, `state`,
`library`, and the transport stepping arithmetic are untouched.

- `src/ui/views.rs`:
  - Extend the browse-row tuple from `(String, &'static str, Message)` to
    `(String, &'static str, Message, bool)` — the new bool is whether this row
    is the currently playing track. `scrollable_list` reads it: when true, it
    prefixes the title with `"▶ "` and gives the button a highlighted
    background (a `Button::style` closure returning a
    `iced::widget::button::Style` with a `Background::Color`); when false, the
    plain button as today. `artist_row` and `album_row` return `false` — the
    marker only ever applies inside an album's song list.
  - `song_row(song: &Song, current_track: Option<&str>)` computes the flag as
    `Some(&song.id) == current_track`.
  - `view_songs(songs: &[Song], current_track: Option<&str>)` takes the current
    track id and passes it to `song_row`. `view_artists`/`view_albums`
    signatures are unchanged.
  - Tests: update `song_row_uses_title_and_selects_the_song` (and the
    `artist_row`/`album_row` tests) for the 4-tuple, and add a
    `song_row_marks_the_current_track` test pinning the flag for `Some(id)`
    matching the song, a different `Some`, and `None`. Update the two
    `browse_views_construct_*` construction tests for the new `view_songs`
    signature and add a construction case passing a current track, following
    the existing pattern (iced `Element`s are not introspectable, so the
    row-tuple flag is the testable contract and construction tests cover
    rendering without panicking).
- `src/ui/mod.rs` `view()`: the current-track id must reach `view_songs`. The
    Songs arm acquires the shared-state lock briefly and passes
    `state.current_track.as_deref()` — a second short `blocking_lock` rather
    than a per-frame `Option<String>` clone, consistent with the earlier
    commit that removed the per-frame `current_track` clone (4d7a585). The
    label tuple block above stays as is.

**Files touched.** `src/ui/views.rs`, `src/ui/mod.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `song_row_marks_the_current_track`
  row-flag test and the updated construction tests.
- `cargo fmt --check` and `cargo clippy --all-targets` are clean.
- `cargo run`: in a Songs view, clicking a song marks it (`▶` + highlighted
  background); Next/Previous move the marker through the list; browsing to
  another album shows no marker (its songs do not contain the current track);
  stepping works with the marker following along. Build + tests are the
  primary gate; the visual check confirms the marker renders.

Note: the acceptance criteria require `cargo fmt --check` clean, and the only
line failing it was a pre-existing over-long `assert_ids` call in
`src/apple_music.rs` (added by c9cb1cb) — this change reformats that one call
so the gate passes; no other line in the integration seam is touched.

### Add a Stop button to the transport controls (done 2026-10-06)

Found by plan 2026-10-06.

**Goal.** The classic Winamp transport is Play, Pause, Stop, Previous, Next; this
player renders Play/Pause, Previous, and Next (plus the volume slider) but no
Stop — a Winamp-clone transport without Stop is missing one of its five core
buttons. Land a Stop button that halts playback: it clears `is_playing` while
leaving `current_track` in place, so the Now Playing bar keeps showing the
interrupted track's title (matching Winamp, where Stop stops the music and the
title stays in the display).

**Approach.** Stop is a transport control, so — following the Previous/Next
precedent, which moved transport sequencing out of the service (the transport
plan's note: "the service models the library API, not the current playlist") —
it is handled in the UI against shared state, not as a new
`AppleMusicService` method. The service's private `pause`/`next_track`/
`previous_track` stubs stay untouched.

- `src/state.rs`: add `pub fn stop(&mut self)` on `AppState` that sets
  `self.is_playing = false` and changes nothing else — the sibling of
  `toggle_playing` (whose doc comment says the Play/Pause button is its only
  caller), keeping the stop semantics beside the field they mutate rather than
  inlined at the call site. Add a `#[cfg(test)]` test
  `stop_clears_playing_flag_and_keeps_current_track` mirroring
  `toggle_playing_flips_only_the_playback_flag`: after `stop()`, `is_playing`
  is false and `current_track` and `volume` are unchanged.
- `src/ui/mod.rs`:
  - Add a `Message::Stop` variant to the `Message` enum.
  - In `update`, add a `Message::Stop` arm calling
    `player.state.blocking_lock().stop()` and returning `Task::none()` —
    synchronous, exactly like the `Message::PlayPause` arm. (The stub has no
    playback position yet, so Stop's only observable effect is the cleared
    playing flag — identical to Pause today. The distinction is the seam:
    once real playback lands, Stop also resets the track position while Pause
    keeps it. Not a design question for this plan — just record it in the
    arm's comment.)
  - In the `#[cfg(test)]` module, add a plain test
    `stop_clears_is_playing` mirroring `play_pause_toggles_is_playing`: after
    `Message::Stop` the shared state's `is_playing` is false (it stays a plain
    test because the arm uses `blocking_lock`, which panics inside an async
    runtime).
- `src/ui/views.rs`: in `view_transport_controls`, insert a Stop button
  between the Play/Pause button and the Previous button, in Winamp's order:
  `Button::new(Text::new("Stop")).on_press(Message::Stop)`, separated by the
  existing `spacer(20.0)`. The label is static (Stop is always pressable, even
  when stopped, as in Winamp), so no `play_pause_label`-style helper is
  needed; `view_transport_controls`'s signature stays unchanged.

**Files touched.** `src/state.rs`, `src/ui/mod.rs`, `src/ui/views.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `stop_clears_playing_flag_and_keeps_current_track`
  (state.rs) and `stop_clears_is_playing` (ui/mod.rs) tests.
- `cargo fmt --check` passes.
- `Message::Stop` is wired on both ends: `grep -n 'Message::Stop' src` shows
  the emitter (the Stop button's `on_press` in `view_transport_controls`) and
  the handler (the `update` arm).
- `cargo run`: with a song playing, Stop flips the Play/Pause button back to
  "Play" (is_playing cleared) while the Now Playing bar keeps the interrupted
  track's title; pressing Play resumes it (manual check — build + tests are the
  primary gate).

### Show the song's title in the Now Playing bar, not its raw id (done 2026-10-05)

Found by plan 2026-10-05.

**Goal.** `AppState.current_track` stores a track *id*, and `WinampPlayer::view`
renders it verbatim — after playing "Opening" the bar reads
"Now Playing: song-1", not the title a Winamp-style player should show.
Resolve the id against the songs currently loaded in `WinampPlayer.songs`
(the same list the songs view renders and the Previous/Next plan sequences)
and display the matching `Song.title`, falling back to the id when the track
isn't in the loaded songs and to "Nothing" when nothing is current. Keep the
resolution pure so it is testable without the UI.

**Approach.**

- `src/ui/mod.rs`: add a private
  `fn now_playing_label(songs: &[Song], current_track: Option<&str>) -> String`
  beside `WinampPlayer::view` (the formatting is a view-time concern, so the
  helper stays with the player UI rather than in `views.rs`, whose stated
  contract is data → `Element`). Semantics: `current_track` is `None` →
  `"Nothing"`; `current_track` is `Some(id)` whose id matches a `Song` in
  `songs` → that song's `title` (cloned); `Some(id)` with no match → the raw
  `id` itself (a track is current, so "Nothing" would lie). Add a
  `#[cfg(test)] mod tests` covering: no current track → "Nothing"; current id
  found in `songs` → its title; current id not in `songs` → the id; empty
  `songs` with a current id → the id. The function is pure data → `String`, so
  it is testable without the UI.
- In `WinampPlayer::view`, replace the existing
  ```rust
  let now_playing = state
      .current_track
      .clone()
      .unwrap_or_else(|| "Nothing".to_string());
  ```
  with a call that reads the id while the `blocking_lock` guard is held and
  delegates the formatting to the helper:
  ```rust
  let now_playing = now_playing_label(&player.songs, state.current_track.as_deref());
  ```
  No `Message` change and no `AppState` change: `current_track` stays the
  id (the Previous/Next plan reads it as an id via
  `current_track.as_deref()`), and the title lookup is a view-time concern.
  A track played from album A stays resolvable after navigating to an album
  B's songs only if its id is still in `player.songs` (which isn't cleared on
  navigation) — the id fallback covers any miss; sourcing titles more
  robustly is a later plan.

**Files touched.** `src/ui/mod.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `now_playing_label` unit tests (no
  current track, title lookup, unknown-id fallback, empty songs list).
- `cargo fmt --check` passes.
- `cargo run`: after playing a song from the songs view, the Now Playing bar
  shows that song's title (e.g. "Opening") instead of its id ("song-1"); at
  boot it still shows "Nothing" (manual check — build + tests are the primary
  gate).

**Update (2026-10-06, organize).** `now_playing_label` (with its unit tests)
and the Now Playing bar / transport row construction moved to
`src/ui/views.rs`, grouped with the other widget builders; the plan's
original placement — the helper beside `view` in `src/ui/mod.rs` — is
superseded.

### Make Previous/Next step through the songs of the current album (done 2026-10-05)

Found by plan 2026-10-04.

**Goal.** The transport row renders Previous and Next buttons, but in
`WinampPlayer::update` the `Message::NextTrack | Message::PreviousTrack` arm
maps to `Task::none()` — pressing them does nothing. Land functional transport:
Previous/Next advance through the songs of the currently loaded album (the
`WinampPlayer.songs` list), reusing the existing `play_track` path so the Now
Playing bar updates. Sequencing is a player concern, so the index logic is a
pure, tested function beside the player, not in `AppleMusicService` (whose
`next_track`/`previous_track` private stubs stay unused and untouched — the
service models the library API, not the current playlist).

**Approach.**

- New file `src/ui/transport.rs`, a pure module mirroring `views.rs`:
  - `pub fn next_track_id(songs: &[Song], current: Option<&str>) -> Option<String>`
    and `pub fn previous_track_id(...) -> Option<String>`.
  - Semantics: empty `songs` → `None` (no-op). `current` is `None` or not
    found in `songs` → first song for next, last song for previous. `current`
    found at index `i` → `songs[(i + 1) % len].id` for next,
    `songs[(i + len - 1) % len].id` for previous (wrap in both directions;
    repeat/shuffle semantics are a later plan).
  - A `#[cfg(test)] mod tests` covering: empty list → `None`; `current` `None`
    → first/last; unknown `current` id → first/last; mid-list advance and
    reverse; wrap from last → first (next) and from first → last (previous).
- `src/ui/mod.rs`:
  - Add `mod transport;` beside `mod views;`.
  - In `update`, split the combined arm: `Message::NextTrack` reads
    `current_track` via `player.state.blocking_lock()`, calls
    `transport::next_track_id(&player.songs, current.as_deref())`, and returns
    `Task::done(Message::TrackSelected(id))` when `Some`, else `Task::none()`.
    `Message::PreviousTrack` does the same with `previous_track_id`.
  - Dispatching `Message::TrackSelected` (rather than calling `play_track`
    directly) reuses the one existing play path — it performs `play_track`,
    which sets `state.current_track`/`is_playing`, and re-renders via
    `TrackPlayed`; the same mechanism `boot` already uses with
    `Task::done(Message::LoadArtists)` feeds the message back into `update`.

**Files touched.** `src/ui/transport.rs` (new), `src/ui/mod.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `next_track_id` / `previous_track_id`
  unit tests (empty list, no current, unknown current, mid-list advance/reverse,
  wrap at both ends).
- `cargo fmt --check` passes.
- `Message::NextTrack` / `Message::PreviousTrack` are no longer dead: the
  `update` arms no longer map to `Task::none()`.
- `cargo run`: after loading an album's songs and playing one, Previous/Next
  move the Now Playing track through that album's songs, wrapping at the ends;
  at the artist/album list (no songs loaded) they do nothing (manual check —
  build + tests are the primary gate).

### Add a working volume slider to the transport controls (done 2026-10-05)

Found by plan 2026-10-04.

**Goal.** The transport row renders Play/Pause, Previous, and Next, and
`AppState` holds a `volume: f32` (default 0.5) that nothing currently reads or
changes — the iced 0.14 port dropped the `VolumeChange` message entirely, so
the volume control must be built from scratch (message variant, `update` arm,
and widget), not re-wired to a handler that no longer exists. Land the classic
Winamp-style volume slider: add iced's `Slider` widget to the transport row in
`WinampPlayer::view` wired to a new `Message::VolumeChange(f32)` variant and
`update` arm that stores the clamped value into `state.volume`, keeping the
clamping logic in a pure, tested helper. The iced 0.14 port has landed (done
2026-10-04), so the crate compiles on iced 0.14 as-is and this plan can build.

**Approach.**

- `src/state.rs`: add `pub fn clamp_volume(volume: f32) -> f32` returning
  `volume.clamp(0.0, 1.0)`, plus a `#[cfg(test)] mod tests` covering the three
  cases: below 0 → 0.0, above 1 → 1.0, in-range value unchanged. Volume is
  shared state, so the helper lives beside `AppState` rather than in the UI
  module.
- `src/ui/mod.rs`:
  - Extend the `iced::widget` import (`Button, Column, Row, Space, Text`) to
    include `Slider`.
  - In `WinampPlayer::view`, append `Slider::new(0.0..=1.0, state.volume,
    Message::VolumeChange)` with a fixed width (`Length::Fixed(100.0)` or
    similar) to the transport `Row` that currently holds the Play/Pause,
    Previous, and Next buttons.
  - In `WinampPlayer::update`, add a `Message::VolumeChange(volume)` arm — the
    port dropped this message, so no such arm exists today — that stores
    `state::clamp_volume(volume)` into `state.volume`.
  - Use iced 0.14's `Length::Fixed` spelling near the slider (e.g.
    `Length::Fixed(100.0)`): `Length::Pixels` does not exist in iced 0.14
    (the port's Done note records the rename from `Length::Units` to
    `Length::Fixed`).
  - In the `#[cfg(test)] mod tests`, add a `volume_change_clamps_value_before_storing`
    test driving the new `Message::VolumeChange` arm: out-of-range values are
    clamped before landing in `state.volume`, an in-range value is stored as-is
    (every other `update` arm in this module carries such a test).

**Files touched.** `src/state.rs`, `src/ui/mod.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new `clamp_volume` tests (clamp below 0,
  above 1, pass through in-range) and the `volume_change_clamps_value_before_storing`
  update-arm test in `src/ui/mod.rs`.
- `cargo run` shows a volume slider in the transport row; dragging it changes
  the volume (manual check — build + tests are the primary gate).
- The new `Message::VolumeChange` is wired on both ends: a `grep -n
  'VolumeChange' src` shows both the emitter (`Slider` `on_changed` in
  `view`) and the handler in `update`.

### Make the app build on iced 0.14 and render a browsable sample library (done 2026-10-04)

Found by plan 2026-10-04.

**Goal.** The crate does not compile. `src/ui/mod.rs` implements iced's
`Application` trait against the iced ≤0.13 API — `iced_native::Command`,
`async fn update`, `Length::Units`, `WinampPlayer::run(Settings::default())` —
while `Cargo.toml` pins iced 0.14, whose `Program` requires a synchronous
`update` returning `iced::Task` and is booted via the `iced::application(boot,
update, view)` builder. Independently, the UI calls
`AppleMusicService::get_favorite_artists`, `get_albums_by_artist`,
`get_songs_from_album` and imports `Artist`, `Album`, `Song`, none of which
`src/library.rs` defines. Land the artist → album → song library-browse
flow end to end: add the missing model and service methods (backed by an
in-memory sample library; real Apple Music auth/API is a later, larger plan),
port the UI to iced 0.14, and get
`cargo build` + `cargo test` green.

**Approach.**

- `src/library.rs`:
  - Add `pub struct Artist { pub id: String, pub name: String }`,
    `pub struct Album { pub id: String, pub title: String, pub artist_id: String }`,
    `pub struct Song { pub id: String, pub title: String, pub album_id: String }`,
    each deriving `Clone, Debug, PartialEq, Serialize, Deserialize`.
- `src/apple_music.rs`:
  - Add a private builder `fn sample_library() -> SampleLibrary` returning a
    small in-memory library (3 artists, 3 albums, 5 songs) in one struct so
    the browse queries and the tests share the same data.
  - Add three `pub async fn(&self, ...) -> Result<Vec<…>, reqwest::Error>`
    methods on `AppleMusicService`: `get_favorite_artists` (all artists),
    `get_albums_by_artist(&self, artist_id: &str)` (albums whose
    `artist_id` matches), `get_songs_from_album(&self, album_id: &str)`
    (songs whose `album_id` matches); unknown id → empty `Vec`. `play_track`
    is made `pub` so the UI can call it.
  - Derive `Clone` on `AppleMusicService` and `AppleMusicToken` so the UI
    can move a cloned service into a `'static` task future.
  - Make `init_service` a synchronous module-level free function (it never
    awaited anything) and add a `#[cfg(test)] mod tests` covering: sample
    library is non-empty; `get_albums_by_artist` returns only matching
    albums and is empty for an unknown id; `get_songs_from_album` likewise.
- `src/ui/mod.rs`:
  - Drop the `iced_native`, `executor`, and `Settings` imports and the
    `Application` trait impl; boot with
    `iced::application(boot, update, view).title("nostalgia").run()`, where
    `update` is synchronous and returns `Task<Message>` and `view` returns
    `Element<'_, Message>`.
  - Replace the three `Command::perform(...)` load sites (`LoadArtists`,
    `ArtistSelected`, `AlbumSelected`) with `Task::perform`, moving a
    `service.clone()` and the id `String`s into `async move { ... }` blocks
    so the future satisfies `Task::perform`'s `'static + Send` bound.
    `TrackSelected` does the same around `play_track`, emitting a no-op
    `TrackPlayed` message so the Now Playing bar re-renders.
  - Replace `Length::Units(20)` with `Length::Fixed(20.0)` and
    `Space::with_width(...)` with `Space::new().width(...)` (both renamed in
    iced 0.14; `Length::Pixels` does not exist).
  - `view` copies the current track and play label out of the shared state
    into owned values (the `Text` widgets own their strings, so no borrow of
    the temporary `MutexGuard` escapes `view`); drop the `VolumeChange`
    message (no widget sends it).
  - Keep the `Message` enum and the `view_artists` / `view_albums` /
    `view_songs` helpers, which live in `src/ui/views.rs` as free functions
    taking `&[Artist]` / `&[Album]` / `&[Song]`.
- `src/main.rs`: `main` is now a plain synchronous `fn main() -> iced::Result`
  (no `#[tokio::main]`): iced drives its event loop synchronously on the
  calling thread, and `update`/`view` use `blocking_lock` on the shared state,
  which panics inside a tokio runtime. `ui::init_ui(state)` is no longer
  awaited.
- `Cargo.toml`: drop the `iced_native` dependency; enable tokio `macros` and
  `rt` features for the `#[tokio::test]` runtime (only `sync` was enabled).

**Files touched.** `Cargo.toml`, `Cargo.lock`, `src/library.rs`,
`src/apple_music.rs`, `src/ui/mod.rs`, `src/ui/views.rs`, `src/main.rs`.

**Acceptance criteria.**
- `cargo build` succeeds.
- `cargo test` passes, including the new sample-library tests (non-empty
  library; correct filtering by `artist_id` / `album_id`; empty result for
  unknown ids).
- `cargo fmt --check` passes.
- `cargo run` starts without panicking and, with a display, shows the sample
  artists; selecting an artist lists that artist's albums, and selecting an
  album lists its songs (manual check — the build + tests are the primary
  gate).

**Update (2026-10-06, organize).** `SampleLibrary` (with its `index_by`
lookup builder, the `SAMPLE_LIBRARY` cache, and the `sample_library()`
accessor) moved out of `src/apple_music.rs` into a new `src/sample_library.rs`
module, so the in-memory stub data is separate from the Apple Music service
seam — `src/apple_music.rs` now holds only the service. The plan's original
placement — the builder inside `src/apple_music.rs` — is superseded.
