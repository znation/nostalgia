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

### Page through the Jump to File matches and keep the highlight in view (planned 2026-10-26 by the plan role)

**Goal.** The Jump to File panel's Up and Down move the selection bar, but Page Up and Page Down
are not its keys: while the panel is open they still scroll the browse list hidden behind the
modal, and moving the highlight past the panel's fixed-height list leaves the highlighted row
off-screen. Make Page Up and Page Down move the highlight one page of visible matches at a time,
clamped at the ends, and make every highlight move (Up, Down, and the two page keys) scroll the
match list only as far as needed to show the row. The keys keep their browse-scrolling behavior
while the panel is closed, and Home and End stay the focused filter box's cursor keys, so they are
not rebound. This is independent of the waiting slider-wheel plan: it adds no wheel or mixer
message and touches none of that plan's files.

**Approach.**

- `src/ui/views/jump_file.rs`: add the list's stable widget id `pub const JUMP_FILE_LIST_ID: &str
  = "nostalgia-jump-file-list";` beside `JUMP_FILE_INPUT_ID`. Add the row geometry the handler's
  reveal needs as constants: `JUMP_FILE_ROW_HEIGHT` (32.0, leaving the 18 px label its vertical
  button padding inside), `JUMP_FILE_ROW_SPACING` (2.0), `JUMP_FILE_LIST_PADDING` (4.0),
  `JUMP_FILE_LIST_HEIGHT` (180.0), and the page step `JUMP_FILE_PAGE_ROWS` (5, the rows that fit
  the list). Use those constants in the view: `Column::new().spacing(..).padding(..)`, each match
  button `.height(Length::Fixed(JUMP_FILE_ROW_HEIGHT))`, and the `Scrollable`
  `.id(JUMP_FILE_LIST_ID).height(Length::Fixed(JUMP_FILE_LIST_HEIGHT))`. The row height is
  explicit so the reveal arithmetic matches what is drawn; five 32 px rows plus four 2 px gaps
  and the 8 px padding fill 176 of the 180 px list.
- `src/ui/views/mod.rs`: re-export `JUMP_FILE_LIST_ID`, `JUMP_FILE_PAGE_ROWS`,
  `JUMP_FILE_ROW_HEIGHT`, `JUMP_FILE_ROW_SPACING`, and `JUMP_FILE_LIST_PADDING` beside
  `JUMP_FILE_INPUT_ID`, so the handler and the geometry test use the view's own numbers.
- `src/ui/handlers/browse/scroll.rs`: move the arithmetic in `reveal_row_offset` into a new
  `pub(in crate::ui) fn reveal_offset(row_top: f32, row_height: f32, scroll_top: f32,
  viewport_height: f32) -> Option<f32>` and make `reveal_row_offset` call it with
  `BROWSE_ROW_HEIGHT` and the browse padding. Its behavior and existing tests stay exactly as they
  are; the Jump to File handler reuses the same minimal-reveal rule.
- `src/ui/handlers/jump.rs`: change `move_jump_file_selection` to take a signed `step: isize`,
  compute the highlighted match's position as it does today (a `None` selection starts from the
  first match), clamp `position as isize + step` to `0..=matches.len() - 1`, store that match's
  source index, and return `reveal_jump_file_row(next)`. `jump_file_selection_up`/`down` pass
  `-1`/`1`; new `jump_file_selection_page_up`/`page_down` pass `-JUMP_FILE_PAGE_ROWS`/
  `JUMP_FILE_PAGE_ROWS`. A closed panel, no showing list, or no matches keeps its current
  no-op/clear path and schedules no scroll. Add `reveal_jump_file_row(position)` as a private
  operation over `views::JUMP_FILE_LIST_ID`, mirroring `reveal_browse_row`: it computes `row_top =
  views::JUMP_FILE_LIST_PADDING + position as f32 * (views::JUMP_FILE_ROW_HEIGHT +
  views::JUMP_FILE_ROW_SPACING)` and calls `state.scroll_to` only when `reveal_offset(row_top,
  views::JUMP_FILE_ROW_HEIGHT, translation.y, bounds.height)` returns a target. Factor `row_top`
  into a small pure `jump_file_row_top(position: usize) -> f32` so a test pins the geometry.
- `src/ui/message.rs`: add `JumpFileSelectionPageUp` and `JumpFileSelectionPageDown` after the
  Up/Down variants, documenting that each moves the highlight one `JUMP_FILE_PAGE_ROWS` page while
  the panel is open, clamps, and requests only the minimal reveal.
- `src/ui/mod.rs`: dispatch both variants to the new handlers beside `JumpFileSelectionUp`/`Down`.
- `src/ui/shortcuts.rs`: in `message_for_jump_file`, map `Key::Named(PageUp)` and
  `Key::Named(PageDown)` with Control, Alt, and Logo released (Shift allowed, like Up/Down) to the
  new messages before falling through to `message_for`, so while the panel is open they move its
  highlight instead of scrolling the browse list behind it; with the panel closed they keep their
  browse mappings. Extend the mapper's doc comment to name them and note Home/End stay with the
  focused filter box.
- `src/ui/shortcuts.rs` tests: extend `message_for_jump_file_maps_up_and_down_and_falls_back` (or
  add a sibling) to assert Page Up/Down map to the page messages, Shift is allowed, and
  Control/Alt/Logo chords still reach the normal mapper's unbound result.
- `src/ui/tests/jump_file_tests.rs`: update the Up/Down walk to expect the reveal task
  (`assert_message_schedules_work`) while the panel is open and no work while it is closed. Add
  `page_up_and_down_move_the_highlight_by_one_page_and_clamp`: load a twelve-song list (clones of
  `stepping_songs()[0]` with distinct ids and titles), open the panel, and assert Page Down from
  the default first match stores source index 5, a second Page Down stores 10, a third clamps at
  11, Page Up returns to 6 and then 1, and each move schedules its reveal. Add
  `jump_file_row_top_starts_at_the_padding_and_steps_by_one_row_and_gap` beside the handler's
  other tests.
- `src/ui/views/tests/jump_file.rs`: add `jump_file_page_step_fits_the_visible_list`, asserting
  `JUMP_FILE_PAGE_ROWS` rows plus their gaps and the list padding fit `JUMP_FILE_LIST_HEIGHT`
  while one more row does not, so the page step cannot drift from the drawn list.
- `docs/usage.md`: in the Jump to File sentence, add that Page Up and Page Down move the selection
  bar one page of visible matches at a time, clamped like Up and Down, and that moving it scrolls
  the panel's list only as far as needed to keep the row in view.
- `docs/invariants.md` `## Jump to File panel`: add the page-key mapping, the clamped page step,
  the minimal-reveal rule and its row geometry to the guarantee bullets, and name
  `shortcuts::message_for_jump_file`, `handlers::jump::move_jump_file_selection`,
  `reveal_jump_file_row`, and `handlers::browse::scroll::reveal_offset` as enforcers; add the new
  tests to **Pinned by**.

**Files touched.** `src/ui/views/jump_file.rs`, `src/ui/views/mod.rs`,
`src/ui/handlers/browse/scroll.rs`, `src/ui/handlers/jump.rs`, `src/ui/message.rs`,
`src/ui/mod.rs`, `src/ui/shortcuts.rs`, `src/ui/tests/jump_file_tests.rs`,
`src/ui/views/tests/jump_file.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- While the Jump to File panel is open, Page Up and Page Down move the selection bar five matches
  at a time, clamped at the first and last match; Enter or `Jump` then plays the new highlight,
  and a closed panel or an empty match list leaves the keys' browse behavior alone.
- Every highlight move (Up, Down, Page Up, Page Down) scrolls the match list only as far as needed
  to show the row and does not move it when the row is already fully visible; the move changes no
  playback, filter, view, or panel-visibility state beyond the reveal task.
- The explicit row geometry matches the drawn list: a page of rows fits the panel's fixed list
  height and one more row does not.
- The shortcut, handler, and view tests pass, and `make check` passes.

### Light the LCD indicator letters for the modes that are on, like Winamp (planned 2026-10-26 by the plan role)

**Goal.** The LCD's left-edge `O A I D V` strip draws all five letters in the same bright
`theme::LCD_GREEN`, so it cannot tell you that Always on top is engaged, the File info panel is
open, or the visualizer is showing, and its decorative `O` and `D` read as live as the rest.
Make each live letter report its mode: `A` lit while always-on-top is on, `I` lit while the File
info panel is open, and `V` lit while the LCD visualizer is shown; draw the inactive letters,
including the decorative `O` and `D`, in `theme::LCD_GREEN_DIM`. The strip then reports its three
live modes at a glance instead of drawing every letter the same. The letters keep their order,
fixed width, and press messages; this is presentation plus the state the view already holds, with
no new toggle or playback behavior. The plan is independent of the waiting Jump to File paging
plan and touches none of its files.

**Approach.**

- `src/ui/views/now_playing/indicator.rs`: add `#[derive(Debug, Clone, Copy, Default, PartialEq,
  Eq)] pub struct IndicatorModes { pub always_on_top: bool, pub file_info_visible: bool, pub
  visualizer_visible: bool }`, the three flags the strip lights. Add the pure `pub fn
  indicator_label_color(label: &str, modes: IndicatorModes) -> Color`: `A` is
  `theme::LCD_GREEN` when `always_on_top`, `I` when `file_info_visible`, and `V` when
  `visualizer_visible`; every other case, including `O` and `D`, is `theme::LCD_GREEN_DIM`. Change
  `lcd_indicator_strip` to take `modes: IndicatorModes` and color each letter through the helper,
  keeping the same `MouseArea`/message wiring. Update the module and function docs to say the live
  letters light only while their mode holds and `O`/`D` stay dim.
- `src/ui/views/now_playing.rs`: re-export `IndicatorModes` and `lcd_indicator_strip` from the
  `indicator` module, add a `modes: IndicatorModes` parameter to `view_now_playing` and
  `now_playing_content`, and pass it to `lcd_indicator_strip`. Update both docs to describe the
  modes parameter beside the already-resolved readouts; the function keeps its
  `#![allow(clippy::too_many_arguments)]`.
- `src/ui/views/mod.rs`: add `IndicatorModes` to the `pub use now_playing::{...}` re-export list.
- `src/ui/view.rs`: pass `views::IndicatorModes { always_on_top: player.always_on_top,
  file_info_visible: player.file_info_visible, visualizer_visible: player.visualizer_visible }`
  to `view_now_playing`.
- Tests: update the existing `view_now_playing`/`now_playing_content` call sites with an
  `IndicatorModes` argument: `captured_now_playing_texts` in `src/ui/views/tests/mod.rs`,
  `captured_content_with_spectrum` and `now_playing_content_draws_the_indicator_strip_beside_the_title`
  in `src/ui/views/tests/lcd_display.rs`/`src/ui/views/tests/now_playing.rs`, and
  `now_playing_bar_and_browse_action_buttons_construct` in `src/ui/views/tests/browse.rs` pass
  `IndicatorModes::default()`. In `src/ui/views/tests/now_playing.rs`, add
  `indicator_label_color_lights_only_the_active_mode` pinning all five labels under each single
  mode plus all-off and all-on (and that `O`/`D` never light), and
  `lcd_indicator_strip_lights_the_active_letters_and_dims_the_rest`, which lays the strip out with
  `layout_capture::capture_text_and_pixels` for all-off and all-on and asserts the lit letters'
  `max_green_in` clear the dim ones' by more than 100, mirroring
  `now_playing_status_row_lights_the_reported_channel_mode`. Keep the existing order, width, and
  press tests; let `captured_indicator_texts` pass `IndicatorModes::default()`.
- `docs/usage.md`: in the LCD strip sentence, add that each live letter is lit while its mode is on
  (`A` for always-on-top, `I` while File info is open, `V` while the visualizer is shown) and dim
  otherwise, and that `O` and `D` stay dim.
- `docs/invariants.md` `## Now Playing label`: extend the indicator-strip bullet with the
  lit/dim rule and its `LCD_GREEN`/`LCD_GREEN_DIM` colors, name
  `views::now_playing::indicator::indicator_label_color` and `IndicatorModes` in **Enforced by**,
  and add the two new tests to **Pinned by**.

**Files touched.** `src/ui/views/now_playing/indicator.rs`, `src/ui/views/now_playing.rs`,
`src/ui/views/mod.rs`, `src/ui/view.rs`, `src/ui/views/tests/mod.rs`,
`src/ui/views/tests/lcd_display.rs`, `src/ui/views/tests/now_playing.rs`,
`src/ui/views/tests/browse.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- In every playback state the strip draws `O`, `A`, `I`, `D`, `V` in that order at its fixed
  width, and presses still publish `ToggleAlwaysOnTop` for `A`, `ToggleFileInfo` for `I`,
  `ToggleVisualizer` for `V`, and nothing for `O` and `D`.
- `A` draws `theme::LCD_GREEN` exactly while `always_on_top` holds, `I` exactly while
  `file_info_visible` holds, and `V` exactly while `visualizer_visible` holds; every other letter
  and every inactive live letter draws `theme::LCD_GREEN_DIM`.
- The rendered strip shows the lit letters brighter than the dim ones, while the text order and
  bounds are unchanged.
- The project's check passes.

## Done
