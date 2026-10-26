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

### Close the transient panels with Escape, like a Winamp dialog (planned 2026-10-26 by the plan role)

**Goal.** Each transient panel — About, File info, Jump to time, and Jump to File — closes from its
own `Close` button, a backdrop click, or its toggle key, but not from the keyboard's universal
dismiss key. Add one shared route: a plain or Shift-held Escape press closes whichever of those
panels is showing, including while its filter or time box has focus. Escape with Control, Alt, or
Logo held, and every other key, stay as they are; with no panel open Escape stays unbound as it is
today, so a focused `Search library` box keeps it for clearing its own focus. The new route only
clears panel-visibility flags: no query, selection, playback, transport, mixer, browse, or settings
state changes, no work is scheduled, and nothing is persisted.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) fn transient_panel_open(&self) -> bool` beside
  `close_transient_panels`, returning `self.about_visible || self.file_info_visible ||
  self.jump_time_visible || self.jump_file_visible`. Document it beside `close_transient_panels`
  as the read side of the same four-flag set, so the subscription gate and the closer cannot
  drift; the set stays named in this one impl rather than in each caller.
- `src/ui/message.rs`: add a payload-free `EscapePressed` after `JumpFileRowSelected`, documented
  as closing the showing transient panel (About, File info, Jump to time, or Jump to File) by
  clearing only those four visibility flags, changing no other state, and scheduling no work.
- `src/ui/handlers/window.rs`: add `pub(in crate::ui) fn escape_pressed(player: &mut WinampPlayer)
  -> Task<Message>` beside `toggle_about`: call `player.close_transient_panels()` and return
  `Task::none()`. Document that the subscription only delivers it while a panel is open, and that
  with none open it is still a no-op. It is not a toggle, so it never opens a panel.
- `src/ui/timers.rs`:
  - Add `pub(super) fn escape_key_message(event: iced::Event, _status: iced::event::Status,
    _window: iced::window::Id) -> Option<Message>` beside `mixer_key_message`. Match
    `iced::Event::Keyboard(keyboard::Event::KeyPressed { key: keyboard::Key::Named(
    keyboard::key::Named::Escape), modifiers, .. })` and return `Some(Message::EscapePressed)`
    unless `modifiers.control() || modifiers.alt() || modifiers.logo()` holds; every other event,
    and every Escape release, returns `None`. Ignore the status argument deliberately: this
    listener exists so the press is seen even when a focused `TextInput` captures Escape and
    clears its own focus, which `keyboard::listen`'s ignored-only feed would drop.
  - Give `subscriptions` a fifth `transient_panel_open: bool` argument after
    `mixer_readout_showing`, and push `iced::event::listen_with(escape_key_message)` only when it
    holds. Document that the Escape listener joins only while a panel is open, so Escape outside
    a panel rebuilds no screen, and that the event-with-status form is required because the panel
    boxes hold focus.
- `src/ui/mod.rs`: pass `player.transient_panel_open()` as the fifth `subscriptions` argument,
  and route `Message::EscapePressed => handlers::window::escape_pressed(player)` beside the
  `ToggleAbout`/`ToggleFileInfo` arms.
- `src/ui/tests/timers_tests.rs`: append `false` to every existing `subscriptions(...)` call. Add
  `the_escape_listener_runs_only_while_a_transient_panel_is_open`, asserting the baseline
  `subscriptions(false, false, false, false, false)` and
  `subscriptions(false, false, false, false, true)` differ by exactly the Escape listener (3 vs
  2). Add `escape_key_message_maps_escape_but_not_its_chords_or_other_keys`, building a
  `KeyPressed` event as `a_key_or_wheel_event_ends_a_mixer_readout` does
  (`src/ui/tests/mixer_tests.rs`), asserting `Some(Message::EscapePressed)` for a plain and a
  Shift-held press with `Status::Captured` (proving the captured-event seam), and `None` for
  Ctrl, Alt, and Logo chords, an Escape release, and another named key.
- `src/ui/tests/panels_tests.rs`: add
  `escape_closes_each_transient_panel_without_touching_anything_else`. From one `test_player()`,
  drive `Message::SongsLoaded(stepping_songs())`, set `current_view = CurrentView::Songs`, call
  `make_current_track_known`, and set `playback_duration = Some(...)` so all four panels can
  open. Assert `Message::EscapePressed` with no panel open schedules no work and leaves
  `transient_panel_open()` false. Then loop over the four toggles
  (`ToggleAbout`, `ToggleFileInfo`, `ToggleJumpTime`, `ToggleJumpFile`): open it, assert
  `transient_panel_open()`, drive `Message::EscapePressed` with
  `assert_message_schedules_no_work`, and assert `!transient_panel_open()`; snapshot transport
  mode, current track, volume, balance, repeat, shuffle, current view, and the songs row
  selection before the loop and assert them unchanged after it.
- `docs/usage.md`: in the transient-panel sentence, say a plain or Shift-held Escape press closes
  whichever panel is open, like its `Close` button or the backdrop, and that with no panel open
  Escape keeps its existing behavior.
- `docs/invariants.md`: add a `## Transient panel dismissal` section after `## Jump to File panel`
  stating the guarantee above, enforced by `WinampPlayer::transient_panel_open`,
  `timers::escape_key_message`, `Message::EscapePressed`,
  `handlers::window::escape_pressed`, and `WinampPlayer::close_transient_panels`, pinned by the
  three new tests; name `handlers::window::escape_pressed` and `ui::timers` in that section. Add
  one line to each of `## About panel`, `## File info panel`, `## Jump to time panel`, and
  `## Jump to File panel` saying a plain or Shift-held Escape also closes it, cross-referencing
  `## Transient panel dismissal`, and add the new tests to those sections' **Pinned by** lists.

**Files touched.** `src/ui/player.rs`, `src/ui/message.rs`, `src/ui/handlers/window.rs`,
`src/ui/timers.rs`, `src/ui/mod.rs`, `src/ui/tests/timers_tests.rs`,
`src/ui/tests/panels_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- With any one of About, File info, Jump to time, or Jump to File open, a plain or Shift-held
  Escape press closes exactly that panel: all four `*_visible` flags are false afterwards, no
  task is scheduled, and no other state changes.
- The Escape press is delivered even while the panel's own text box has focus, because the
  listener uses `iced::event::listen_with` and ignores the event status; it is subscribed only
  while `WinampPlayer::transient_panel_open()` holds, so Escape outside a panel sends nothing.
- Escape with Control, Alt, or Logo held and Escape key releases map to nothing, and
  `shortcuts::message_for` still returns `None` for a plain Escape press, so every existing
  keyboard and media-key binding is unchanged.
- With no panel open, `Message::EscapePressed` is a no-op, and Escape keeps its current behavior
  (including a focused `Search library` box clearing its own focus).
- Queries, browse selection, playback, transport, mixer, settings, and persisted state are
  untouched; nothing new is saved.
- `docs/usage.md` and `docs/invariants.md` describe the dismissal route, and `make check` passes.

## Done
