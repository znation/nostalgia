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

### Right-click the player to open the options menu, like Winamp's main-window menu (planned 2026-10-27 by the plan role)

**Goal.** In Winamp 2.x, right-clicking anywhere on the main window (or its rolled-up shade
strip) pops up the main menu, and the playlist editor keeps its own right-click menu. Our closest
analog to that menu is the `OPTIONS` overlay, which today opens only from the LCD's `O`, Alt+O,
or the transport row's lightning button. A right-press on the player panels should toggle it
too. The library section stays out of this area.

**Approach.** The player panels already sit inside one outer `MouseArea` that carries the volume
wheel: `MouseArea::new(player_panels).on_scroll(views::volume_wheel_message)` in
`src/ui/view/normal.rs` `view_normal`, and the title bar plus shade strip
`MouseArea::new(...).on_scroll(views::volume_wheel_message)` in `src/ui/view/shade.rs`
`view_shade`. Add `.on_right_press(Message::ToggleOptions)` to both wrappers. No new message or
handler is needed: `handlers::window::toggle_options` already closes the other transient panels,
resets `options_selected`, and restores a shaded window before the menu draws. A second
right-press does not close the menu: while it is open, `view_options_overlay` maps the base to
`Message::Ignored`, and the backdrop (`modal_overlay`) reacts only to a left press. Close, a
left click on the backdrop, Escape, or Alt+O still close it.
iced 0.14's `MouseArea::update` (iced_widget-0.14.2 `src/mouse_area.rs`) lets the content handle
the event first and then publishes `on_right_press` for `ButtonPressed(Right)`. The nested
widgets (buttons, sliders, the preset pick list, and the inner `MouseArea`s for the LCD title
drag, elapsed readout, visualizer, `O A I D V` letters, title-bar drag region, and shade-strip
double-click) react only to the left button, so a right-press over any of them reaches the outer
wrapper. The library column is pushed outside that wrapper in `view_normal`, so a right-press over
the browse list or library footer publishes nothing new. Update the comment above the wrapper in
`view_normal` so it covers the right-press as well as the wheel.

**Files touched.**
- `src/ui/view/normal.rs`, `src/ui/view/shade.rs`: the two `.on_right_press` calls and comments.
- `src/ui/tests/mod.rs`: add a helper `root_right_press_messages(player, bounds) -> Vec<Message>`
  beside `root_wheel_messages`. It uses `layout_capture::publish_at` on `view(player)` with
  `ButtonPressed(Right)` then `ButtonReleased(Right)` at the bounds' center.
- `src/ui/tests/panels_tests.rs` (or `window_tests.rs`, whichever holds the options tests; keep
  each test file focused): add
  - `right_press_on_the_player_panels_toggles_options`: with `test_player()`, a right-press at
    `captured_root_view_container(&player, views::SEEK_STRIP_ID)` and at
    `views::BALANCE_SLIDER_ID` each publishes exactly one `Message::ToggleOptions`.
  - `right_press_on_the_library_does_not_toggle_options`: a right-press at
    `views::BROWSE_LIST_ID` and at the library footer's `views::MINI_VOLUME_ID`
    (`captured_root_view_volume_slider`) publishes no `ToggleOptions`.
  - `right_press_on_the_shade_strip_toggles_options`: with `player.shaded = true`, a right-press at
    the strip's `views::MINI_VOLUME_ID` bounds publishes one `ToggleOptions`. Confirm first that
    `mini_transport_strip_with_seek` tags the shade strip's volume slider with that id. If it
    does not, press at the centre of the laid-out root view instead, as
    `shade_strip_double_click_restores_the_window` does.
  - Also run `update` with the published message on the shaded player and assert that
    `options_visible` is set and `shaded` is cleared. The existing
    `toggle_options_while_shaded_restores_the_window_first` already pins that handler, so a
    published-message check is enough.
- `docs/usage.md`: in the options-menu paragraph ("The LCD's `O` letter, Alt+O, or the lightning
  button at the end of the transport row opens an `OPTIONS` menu …"), add a right-click anywhere on
  the player (title bar, LCD, seek strip, transport row, equalizer, or the rolled-up shade strip),
  but not on the library, like Winamp's main-window menu.
- `docs/invariants.md` `## Options panel`: add the right-press to the Guarantee's opening
  sentence and first bullet, add the two `on_right_press` wrappers in `src/ui/view/normal.rs` and
  `src/ui/view/shade.rs` to Enforced by, and add the three new tests to Pinned by. In
  `## Volume wheel`'s Enforced by, note that the same two wrappers also carry the options
  right-press.

**Acceptance criteria.**
- A right-press on the title bar, Now Playing LCD, seek strip, transport row, or equalizer
  section opens the options menu with its first row highlighted. While the menu is open, a
  right-press on the base does nothing.
- A right-press on the shade strip restores the window and opens the menu.
- A right-press on the browse list, search row, or library footer does not open it.
- Every left-click, wheel, and keyboard behavior is unchanged; the existing view and wheel tests
  still pass.
- The new tests pass, docs/usage.md and docs/invariants.md describe the right-press, and
  `make check` passes.

### Match each typed word separately in Jump to File, like Winamp's Jump to file (planned 2026-10-27 by the plan role)

**Goal.** Winamp's Jump to file matches a title when every space-separated word typed appears
somewhere in it, in any order, so `band two` finds `The Sample Band - Two`. Our Jump to File
panel compares the whole trimmed query as one substring, so the same query matches nothing: the
cached row title is `<artist> - <title>` (`library::display_title_for`), and typing the artist
then the title never matches across the ` - ` join. Make the filter match word by word.

**Approach.** Change only `views::jump_file::jump_file_matches` in `src/ui/views/jump_file.rs`:
lowercase the query, split it with `str::split_whitespace` into words, and keep an index when
its lowercased title `contains` every word (`Iterator::all`). An empty or all-whitespace query
yields no words, and `all` over no words is true, so it still matches every index; the explicit
`query.is_empty()` branch goes away. Lowercase each title once per call, as today. Order stays
list order. The view (`view_jump_file_overlay`) and the handlers in `src/ui/handlers/jump.rs`
(`move_jump_file_selection` and the submit path) already call this one function, so the drawn
rows, highlight, and the row Enter plays change together and need no edits. Update the function's
doc comment to say the query is split on whitespace and each word must appear in the title as a
case-insensitive substring, in any order. No new dependency, state, or message.

**Files touched.**
- `src/ui/views/jump_file.rs`: `jump_file_matches` body and doc comment.
- `src/ui/views/tests/jump_file.rs`: keep `matches_blank_all_case_insensitive_trimmed_in_order`
  as is (every assertion there holds under word matching) and add
  `matches_every_whitespace_separated_word_in_any_order`, over titles such as
  `"The Sample Band - One"`, `"The Sample Band - Two"`, `"Other Act - Two Step"`: `"band two"`
  gives `[1]`; `"two band"` gives `[1]` (any order); `"TWO"` gives `[1, 2]`; `"two  step"`
  (double space, and with a tab) gives `[2]`; `"band zzz"` gives `[]` (every word must match);
  `"- two"` gives `[1, 2]` (a punctuation word is still a plain substring).
- `src/ui/tests/jump_file_tests.rs`: add `submit_matches_the_artist_and_title_words_together`,
  using the existing `player_in_songs` and `submit` helpers over `stepping_songs` (artist
  `The Sample Band`, titles `One`, `Two`, …): open the panel, `submit(&mut player, "band two")`,
  and `assert_track_selected(task, epoch, 1)` with the panel closed, the same shape as
  `submit_plays_the_first_match_and_closes`. Confirm the fixture's titles before choosing the
  query; the query must match exactly one row by two words that do not appear together as one
  substring.
- `docs/usage.md`: in the Jump to File paragraph, change "typing filters its cached row titles
  case-insensitively" to say each typed word must appear in the row's artist-and-title text, in
  any order, case-insensitively.
- `docs/invariants.md` `## Jump to File panel`: rewrite the `jump_file_matches` bullet (splits
  the query on whitespace; an index matches when its title contains every word case-insensitively;
  no words matches every index; list order) and add the two new tests to its Pinned by list.

**Acceptance criteria.**
- In Jump to File over a song list, typing `band two` shows and plays `The Sample Band - Two`;
  word order does not matter; a query with one word that matches nowhere shows no rows.
- A blank or whitespace-only query still shows every row, a single-word query behaves as before,
  and the existing Jump to File tests pass unchanged.
- The new tests pass, docs/usage.md and docs/invariants.md describe word matching, and
  `make check` passes.

## Done
