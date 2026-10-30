# Plans

## Planned

_None yet._

### Add an Autoscroll song name row to the OPTIONS menu, like Winamp's main-menu toggle (planned 2026-10-29 by the plan role)

**Goal.** Winamp 2.x's main-window menu carries an `Autoscroll song name` toggle that stops the
Too-long-titles marquee scroll, and Nostalgia already has both halves of that behavior — the
scrolling Now Playing title (`Message::MarqueeTick` → `handlers::playback::marquee_tick`, advancing
`WinampPlayer.marquee_offset`, subscribed only while `WinampPlayer::needs_marquee` holds) and the
manual drag-by-hand scroll (`player.title_drag`) — but no way to switch the automatic half off, so
a user who prefers a still title cannot stop the wheel. Add the OPTIONS menu row Winamp has, with
the same persistence treatment the menu's other display toggles (`Visualizer: Shown/Hidden`, the
panel rows) get. Out of scope: any change to the marquee's windowing math (`views::now_playing::text`
), the manual title drag, the LCD tick's mixer-flash and fade duties, and any new keyboard shortcut.

**Approach.** The OPTIONS rows are one enum (`src/ui/message.rs`'s `OptionsRow` with its `ALL`
array and `message()` mapping through `OptionsChoice`), drawn with per-row label functions in
`src/ui/views/options.rs`, and dispatched through `options_choice` in `src/ui/handlers/window.rs`;
a new boolean row rides the same rails end to end. The flag is UI-owned like its neighbors:
`WinampPlayer` keeps it (`visualizer_visible` is the model), reads it back into its settings
snapshot before the shared sweep saves, and adopts it in `adopt_saved_settings`, so it survives
relaunches like every other OPTIONS choice.

- `src/ui/message.rs`: add `Message::ToggleAutoscroll` next to `ToggleVisualizer` with a doc
  comment in the toggle-message shape (flips only the marquee flag, changes no playback state,
  schedules no work); add an `Autoscroll` variant to `OptionsChoice` (doc: reuses
  `Message::ToggleAutoscroll`) and to `OptionsRow` between `TimeDisplay` and `Visualizer`, in the
  same position in `OptionsRow::ALL`, with `message()` mapping it there; extend the
  `options_rows_map_to_mouse_messages_in_draw_order` test for the widened arrays.
- `src/ui/handlers/playback.rs`: add `pub(in crate::ui) fn toggle_autoscroll(player) -> Task<Message>`
  beside `toggle_visualizer`, flipping `player.marquee_autoscroll`, with a doc comment carrying the
  flag rules (below).
- `src/ui/handlers/window.rs`: route `OptionsChoice::Autoscroll` to it in `options_choice`'s
  `Visualizer` arm's neighborhood.
- `src/ui/player.rs`: add the UI-owned flag `marquee_autoscroll: bool`, `true` by default, with a
  doc comment stating the rules — while false, `needs_marquee` returns false so the marquee
  subscription stays off and `MarqueeTick` stops advancing `marquee_offset` (the roller coasts to a
  stop; it does not reset); the manual `title_drag` scroll and `mixer_flash_ticks`/`fade_out_ticks`
  duties of the LCD tick are untouched; the flag persists like `visualizer_visible` and turning it
  back on resumes scrolling from the stopped offset. Wire it into the same three places the
  neighboring flags occupy: the constructor default beside `visualizer_visible: true`, the
  pre-sweep snapshot `set_autoscroll(self.marquee_autoscroll)` next to
  `set_visualizer_visible` (~line 575), and the adoption
  `self.marquee_autoscroll = saved.autoscroll()` in `adopt_saved_settings`. Short-circuit
  `needs_marquee` on `!self.marquee_autoscroll` ahead of the drag checks.
- `src/settings.rs`/`src/settings/text.rs`: add the private `autoscroll: bool` field defaulting to
  `true` with `autoscroll()`/`set_autoscroll` like `set_time_display_remaining`; `to_text` writes
  `autoscroll={}` beside the other always-written UI-owned lines; the parse-match arm reads it with
  the same bool parser `visualizer_visible=` uses, so `garbage` falls back to the default instead
  of being dropped from the file; `from_state` leaves it at the default for the player to
  overwrite, like the other UI-owned flags.
- `src/ui/views/options.rs`: add `pub(super) fn autoscroll_label(autoscroll: bool) -> &'static str`
  (`Autoscroll song name: On` / `: Off`) beside `visualizer_label`, pure for testability, and map
  `OptionsRow::Autoscroll` to it in the row-label match; update the module's doc comment to name
  the new row.
- `src/ui/views/tests/chrome.rs` (or the options tests beside `src/ui/views/options.rs`'s unit
  tests): cover the label wording for both states and that the menu lists the new row between
  `Time:` and `Visualizer:` in the drawing order `OptionsRow::ALL` fixes.
- `src/ui/tests/`: a needs-marquee test (autoscroll off makes `needs_marquee` false for the same
  too-long line that scrolls when it is on, and the drag checks ahead of it stay green), a
  `toggle_autoscroll` round-trip, and a settings-adopt round-trip beside the existing
  `settings_tests.rs` cases (`saved_window_position_is_restored_on_boot` shows the shape).
- `src/settings.rs`: extend the round-trip coverage (`to_text_round_trips_every_field`) and add a
  malformed-value fallback test, matching the suite's existing bad-boolean shapes.
- `docs/usage.md`: extend the OPTIONS menu paragraph's row list with `Autoscroll song name: On/Off`
  (after `Time: Remaining/Elapsed`, reflecting the current state), say what it does and does not
  do (stops the marquee tick, leaves hand-dragging alone, resumes the stopped scroll when turned
  on), and add the choice to the settings-file list of saved choices.
- `docs/invariants.md`: extend `## Options panel` with the row's behavior and `## Now Playing
  label` (or `## UI layout`, whichever pins `needs_marquee`) with the autoscroll rule in the same
  change as the behavior.
- Visible change is menu text over existing chrome — compare the widened menu against the
  player-right-click chrome already on disk (`docs/screenshots/`), since no Winamp main-menu
  screenshot exists (`winamp-screenshots/README.md` notes the missing capture) and keep the
  Winamp 2.x palette and bevels untouched.

**Files touched.** `src/ui/message.rs`, `src/ui/handlers/playback.rs`, `src/ui/handlers/window.rs`,
`src/ui/player.rs`, `src/settings.rs`, `src/settings/text.rs`, `src/ui/views/options.rs`,
`src/ui/views/tests/chrome.rs`, `src/ui/tests/`, `docs/usage.md`, `docs/invariants.md`, plus the
`PLANS.md` move.

**Acceptance criteria.**

- `make check` passes.
- The OPTIONS menu lists `Autoscroll song name: On/Off` between `Time:` and `Visualizer:`,
  reflecting the current state; choosing the row toggles the flag and closes the menu like every
  other row.
- With the row Off, a too-long title stops scrolling — `needs_marquee` returns false, the marquee
  subscription stays unsubscribed, and `marquee_offset` stops advancing — while the hand drag
  still scrolls and the mixer flash and fade-out tick duties continue; turning it On resumes the
  scroll from the stopped offset.
- The choice is written to the settings file (`autoscroll=true`/`false`) in the same sweep as the
  other OPTIONS choices and restored at the next launch; a malformed value falls back to On and
  the other lines still load.
- Playback, the seek and volume bars, the visualizer, and the transport are untouched; no
  `MarqueeTick` handling changes beyond the flag's gate.
- `docs/usage.md` and `docs/invariants.md` carry the new behavior in the same change.

## Done
