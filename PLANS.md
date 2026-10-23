# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; updated 2026-10-20).** The initial prompt
> names two commitments — a "very close if not pixel-perfect" classic Winamp UI
> and Apple Music as the library. The first is now moving: the base-skin
> palette landed ("Add a Winamp 2.x base-skin palette and apply it as the app
> theme", done 2026-10-06), and the reusable bevel layer followed ("Add a
> Winamp two-tone bevel layer and frame the Now Playing and equalizer panels",
> done 2026-10-06), framing those two panels through `bevel::lcd_well` and
> `bevel::raised_panel` in `src/ui/bevel.rs`. The transport-button chrome plan
> landed ("Style the transport buttons as raised Winamp chrome", done
> 2026-10-07), and both transport toggles are reachable: the Repeat toggle
> landed 2026-10-06 and the Shuffle toggle 2026-10-08, and their R and S
> keyboard bindings followed ("Add the R and S keyboard bindings for Repeat
> and Shuffle", done 2026-10-20). The rest of widget-level fidelity has
> landed too: slider chrome ("Style the volume and
> equalizer sliders as sunken Winamp grooves with raised chrome thumbs", done
> 2026-10-07), a custom title bar ("Add a Winamp custom title bar and drop the
> OS window frame", done 2026-10-07), and playlist chrome ("Frame the browse
> list as a sunken Winamp playlist editor", done 2026-10-07). The library side
> is real:
> `AppleMusicService` answers its browse and search queries through the REST
> client once a `MusicKit` session is stored, falling back to
> `sample_library()` otherwise — playlist queries included — and `play_track`
> plays the selected preview through the audio seam. Risk: the Winamp chrome
> arrived piecemeal, so later changes still need one consistent vocabulary.
> The equalizer-audio and seek plans this note pointed at have since landed
> (both 2026-10-20); the 2026-10-22 note below records drift that has since
> been resolved.

> **Steward drift note (2026-10-22 by the steward role; resolved 2026-10-23
> by the steward role).** That note recorded the built Now Playing bar and
> browse rows drifting from the brief's mockups — no elapsed readout, no
> `O A I D V` strip, no status row, album rows without running time, and a
> queued playback-position plan that put an `<elapsed>/<total>` readout at the
> LCD's right edge. Those drifts have since landed: "Add the classic elapsed
> readout to the Now Playing LCD and restore the plain total at its right
> edge", "Draw the classic O A I D V strip at the LCD's left edge", "Show the
> loaded preview's bitrate in the LCD status row", "Show each album row's
> running time", and "Show each artist row's album count" are done, and the
> playback-position plan is no longer queued. No current brief-vs-build drift
> is known. The LCD `MONO`/`STEREO` pair has since landed (done 2026-10-23),
> which brought the status row to `docs/screenshots/player.png` and
> `docs/screenshots/transport.png`, and the volume and balance slider fill
> has since landed (done 2026-10-23) with the orange and green grooves those
> mockups draw, and the playlist-number title prefix landed too (done
> 2026-10-23), matching `docs/screenshots/player.png`, and the transport
> glyph row landed on the same day. No planned entry remains.

## Planned

_None yet._

### Keep the playing row in view when the transport steps (planned 2026-10-23 by the plan role)

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

## Done
