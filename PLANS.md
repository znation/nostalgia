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

### Alt+3 opens a File info panel for the current track, like Winamp (planned 2026-10-25 by the plan role)

**Goal.** Winamp binds Alt+3 to "File info" for the current song. Nostalgia has no way to see the
current track's details apart from the scrolling LCD line. Bind Alt+3 to toggle a transient,
unpersisted File info panel that names the current track (the same track the Now Playing bar and
the window title name). The panel works like the About panel: a modal overlay over the
normal-mode screen that closes from a `Close` button, from a backdrop click, or from Alt+3 again.

**Approach.**
- `src/ui/views/modal.rs` (new): move the backdrop/panel/`Stack` code out of
  `views::about::view_about_overlay` into
  `pub(super) fn modal_overlay<'a>(base: Element<'a, Message>, content: Column<'a, Message>, close: Message) -> Element<'a, Message>`.
  It keeps the `BACKDROP` scrim, the `PANEL_MAX_WIDTH` cap, `bevel::raised_panel` on
  `theme::WINDOW_BACKGROUND` with 16px padding, a backdrop press publishing `close`, and
  `Message::Ignored` for presses on the panel and for wheel rolls on the panel or backdrop.
  `view_about_overlay` builds its unchanged content `Column` and calls
  `modal_overlay(base, content, Message::ToggleAbout)`. This is a pure move: the existing About
  chrome tests in `src/ui/views/tests/chrome.rs` must pass unedited. Register `mod modal;` in
  `src/ui/views/mod.rs`.
- `src/ui/views/file_info.rs` (new):
  `pub fn view_file_info_overlay<'a>(base: Element<'a, Message>, track: &'a KnownTrack) -> Element<'a, Message>`.
  It draws a `FILE INFO` heading (size 18, `theme::TITLE_TAPE`, like `ABOUT NOSTALGIA`), then
  `Title: <track.title>`, then `Artist: <track.artist>` (left out when `track.artist` is empty),
  then `Length: <track.time>` (already `--:--` for an unknown duration, via
  `views::format_track_time`), and finally `labeled_button("Close", Message::ToggleFileInfo)`. It
  wraps everything with `modal_overlay(.., Message::ToggleFileInfo)`. Export it from
  `views/mod.rs` next to `view_about_overlay`. Name the label prefixes as consts.
- `src/ui/message.rs`: add `ToggleFileInfo` beside `ToggleAbout`, with a comment in the same style.
  `src/ui/mod.rs` `update`: route it to `handlers::window::toggle_file_info`.
- `src/ui/player.rs`: add `pub(in crate::ui) file_info_visible: bool` next to `about_visible`
  (same doc style: closed at launch, not persisted), initialized `false`.
- `src/ui/handlers/window.rs`: add `toggle_file_info(player) -> Task<Message>`, modeled on
  `toggle_about`. When opening and there is no current track known to `player.known_tracks` (look
  up `state.current_track` the same way as `WinampPlayer::window_title`), it does nothing and
  returns `Task::none()`. Otherwise it flips `file_info_visible`. Opening also sets
  `about_visible = false`, and opening while `player.shaded` returns `toggle_window_shade(player)`,
  exactly as About does. In `toggle_about`, opening also sets `file_info_visible = false`, so at
  most one panel shows at a time.
- `src/ui/view.rs` `view`: after the About branch, when `player.file_info_visible && !player.shaded`
  and the current track resolves in `known_tracks`, wrap `base` in `views::view_file_info_overlay`.
  An unresolved track draws the plain base. Do not hold the `state` lock across building the
  element: copy the `current_track` id out first, or reuse a lookup already done in `view`.
- `src/ui/shortcuts.rs`: add `file_info_shortcut(physical_key, location, modifiers)`, matching the
  top-row `3` by physical key (`key::Code::Digit3`, `Location::Standard`) with Alt held and
  Control, Shift, and Logo all released. Model it on `playlist_sort_shortcut` and chain it into
  `message_for`. Mention Alt+3 in `message_for`'s doc comment.

**Files touched.** `src/ui/views/modal.rs` (new), `src/ui/views/file_info.rs` (new),
`src/ui/views/about.rs`, `src/ui/views/mod.rs`, `src/ui/message.rs`, `src/ui/mod.rs`,
`src/ui/player.rs`, `src/ui/handlers/window.rs`, `src/ui/view.rs`, `src/ui/shortcuts.rs`, tests in
`src/ui/views/tests/chrome.rs`, `src/ui/tests/window_tests.rs`, and the inline shortcut tests,
`README.md` (the keys paragraph), and `docs/invariants.md` (a new `## File info panel` section
after `## About panel`).

**Acceptance criteria.**
- Alt+3 (top row, Alt only) maps to `Message::ToggleFileInfo`. These map to nothing: Ctrl+Alt+3,
  Shift+Alt+3, Logo+Alt+3, keypad 3 with Alt (`Location::Numpad`), and a plain `3`. The inline
  shortcut tests pin each case.
- In `window_tests.rs`, with a known current track, `ToggleFileInfo` opens and then closes the
  panel and schedules no work. Playback, volume, equalizer, browse, and search state stay
  unchanged, asserted the way `toggle_about_shows_and_hides_the_panel_without_touching_playback`
  asserts them.
- With no current track, or with a current id missing from `known_tracks`, `ToggleFileInfo`
  leaves `file_info_visible` false.
- Opening File info closes About, and opening About closes File info. Opening File info while
  shaded schedules the restore and sets the flag.
- A chrome test builds `view_file_info_overlay` over a blank base and captures its text. The
  heading, `Title: …`, `Artist: …`, `Length: …`, and `Close` are present, and the `Artist:` line
  is absent for an artist-less track. `Close` and a backdrop press each publish one
  `ToggleFileInfo`. A press on the panel's text publishes nothing that closes it.
- The About chrome tests pass unchanged after the `modal_overlay` extraction.
- `view_constructs_when_the_window_is_shaded` (or a sibling test) shows that a set
  `file_info_visible` flag draws no `FILE INFO` text in shade mode.
- README and `docs/invariants.md` describe the key and the panel. `make check` passes.

### Ctrl+W toggles window-shade mode, like Winamp (planned 2026-10-25 by the plan role)

**Goal.** Winamp binds Ctrl+W to "Toggle windowshade mode" for the main window. Nostalgia rolls
the window up only through the title bar's shade button or a double-click on the bar or the
strip, so there is no keyboard route. Bind Ctrl+W (with or without Shift, no Alt or Logo) to the
existing `Message::ToggleWindowShade`, so the key rolls the window up and back exactly as the
shade button does. Plain W stays unbound.

**Approach.**
- `src/ui/shortcuts.rs`, `shortcut`: in the existing `modifiers.control() && !modifiers.alt() &&
  !modifiers.logo()` block (the Ctrl+Z / Ctrl+B / Ctrl+R arms), add an arm for
  `character.eq_ignore_ascii_case("w")` returning `Some(Message::ToggleWindowShade)`. Shift is
  allowed, the same as Ctrl+Z and Ctrl+B. Update the doc comments on `message_for` and `shortcut`
  and the block's inline comment to list Ctrl+W.
- No handler change: `handlers::window::toggle_window_shade` already flips `player.shaded`,
  measures on the first shade, and resizes; without a window id it only flips the flag. The key
  does not touch `about_visible` (or the File info flag, if that plan has merged): the view's
  existing `!player.shaded` guard in `src/ui/view.rs` stops the overlay drawing while shaded, and
  it draws again after an unshade. That matches what the shade button would do and needs no new
  code.
- `iced::keyboard::listen` skips events a focused widget captures. Do not add special handling
  for a focused `Search library` box.

**Tests.**
- `src/ui/shortcuts.rs` tests: add `message_for_maps_ctrl_w_to_toggle_window_shade`. Use
  `assert_maps!` to show that `w` and `W` with `Modifiers::CTRL`, and `w` with
  `Modifiers::CTRL | Modifiers::SHIFT`, map to `Message::ToggleWindowShade`. Then show that `w`
  with `Modifiers::NONE`, `SHIFT`, `ALT`, `LOGO`, `CTRL | ALT`, and `CTRL | LOGO` maps to `None`.
- `src/ui/tests/shortcuts_tests.rs`: add `ctrl_w_rolls_the_window_up_and_back`, following
  `a_toggles_always_on_top_from_the_keyboard`'s pattern. Build a `KeyPressed` event for `w` with
  `Modifiers::CTRL` (physical `key::Code::KeyW`, `Location::Standard`) and map it through
  `super::shortcuts::message_for`. Starting from `player_with_window_id()`, use
  `assert_message_schedules_work` to show that the mapped message sets `player.shaded`. Then
  set `player.unshaded_size = Some(iced::Size::new(800.0, 600.0))` and show that a second mapped
  press clears `player.shaded` and schedules work.

**Files touched.** `src/ui/shortcuts.rs`, `src/ui/tests/shortcuts_tests.rs`, `README.md` (the
keys paragraph in Usage, after the Ctrl+Z/Ctrl+B sentence: "Ctrl+W rolls the window up into
shade mode and back, like the title bar's shade button."), `docs/invariants.md` (`## Window
shade`: add a bullet that Ctrl+W, Control with optional Shift and without Alt or Logo, publishes
`Message::ToggleWindowShade`, pinned by the two new tests. In `## Search focus`, change "every
other Control chord remains the window manager's" so it no longer claims Ctrl+W).

**Acceptance criteria.**
- Ctrl+W and Ctrl+Shift+W map to `Message::ToggleWindowShade`. Plain W, Shift+W, Alt+W,
  Logo+W, Ctrl+Alt+W, and Ctrl+Logo+W map to nothing.
- Pressing Ctrl+W in normal mode shades the window, and pressing it again restores the window,
  through the unchanged `toggle_window_shade`.
- The existing shade, About, and shortcut tests still pass, and `make check` passes.

## Done
