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

### Restart the current preview with Previous after three seconds (planned 2026-10-24 by the plan role)

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

### Open an About panel from the title bar's M button (planned 2026-10-24 by the plan role)

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
`src/ui/views/tests.rs`, `src/ui/message.rs`, `src/ui/player.rs`,
`src/ui/handlers/window.rs`, `src/ui/mod.rs`, `src/ui/view.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

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

## Done
