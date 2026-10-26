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

### Next, Previous, and track jumps scroll only as far as needed to show the landed row, like Winamp (planned 2026-10-25 by the plan role)

**Goal.** In Winamp's playlist editor, stepping tracks leaves the list alone while the new row is
visible, and otherwise scrolls only far enough to bring it into view. Nostalgia does something
different. While a song list shows, Next, Previous, a shuffled retrace, the preview-end advance,
Ctrl+Z/Ctrl+B, and keypad 1/3 all call `scroll_browse_to_playing_row`, and that function snaps the
landed row to the viewport's top through `scroll_browse_to_row`. So each step down a visible list
jerks it upward. Route those steps through the existing minimal-scroll `reveal_browse_row`, which
Alt+Up/Alt+Down already uses, and leave the view gating as it is.

**Approach.**

- `src/ui/handlers/browse/scroll.rs`:
  - `scroll_browse_to_playing_row` should call `reveal_browse_row(index)` in place of
    `scroll_browse_to_row(index)` for `CurrentView::Songs | CurrentView::AllSongs`. Every other
    view still returns `Task::none()`. Update its doc comment to say the scroll goes only as far
    as needed, and not at all when the row is already fully visible.
  - Delete `scroll_browse_to_row`. After this change nothing calls it (grep `src` to confirm),
    and clippy `-D warnings` would reject it as dead code.
  - Retarget the intra-doc links that name it, because `make docs` fails on a broken link. In
    the `scroll_browse_by` doc, contrast with `reveal_browse_row`. In the `reveal_row_offset`
    doc, state the geometry directly: row `index` spans from `BROWSE_LIST_PADDING + index *
    BROWSE_ROW_HEIGHT` for one `BROWSE_ROW_HEIGHT`.
  - Grep `src` and `docs` for any other `scroll_browse_to_row` mention and fix it too.
- `src/ui/tests/stepping_tests.rs`: the current assertions use `assert_task_scrolls_browse_list_to`.
  Its `browse_scroll_effects` reports a zero-height viewport at offset 0, so under reveal the
  expected offsets change. Rewrite these tests to use `browse_scroll_effects_at` from
  `src/ui/tests/mod.rs` with an explicit viewport, in the style of `move_row_tests.rs`:
  - `next_and_previous_scroll_the_showing_song_list_to_the_landed_row` becomes
    `next_and_previous_reveal_the_landed_row_only_when_it_leaves_the_viewport`. From song-2,
    with a one-row viewport (`views::BROWSE_ROW_HEIGHT`) scrolled to song-2's top
    (`PAD + 1*ROW`):
    - Next scrolls to `PAD + 3*ROW - ROW`, which is song-3's bottom minus the viewport.
    - Previous (re-run from a fresh song-2 fixture) scrolls up to `PAD`.
    - Next with a viewport tall enough for all three rows (`10*ROW`, top 0) produces no effect
      (`!effects.any()`).
  - `a_finished_preview_scrolls_the_song_list_to_the_advanced_row`: give it a one-row viewport
    at offset 0. The advance to index 1 then scrolls to `PAD + 2*ROW - ROW`. Add a sibling,
    `a_finished_preview_within_the_viewport_does_not_scroll`, that uses a tall viewport and
    asserts `!effects.any()`.
  - `edge_jumps_scroll_the_showing_song_list`: Ctrl+B (LastTrack) with a one-row viewport at 0
    scrolls to `PAD + (last+1)*ROW - ROW`. Ctrl+Z (FirstTrack) with a one-row viewport
    scrolled to the last row scrolls to `PAD`.
  - `stepping_while_another_browse_level_shows_does_not_scroll` stays unchanged.
- `src/ui/tests/mod.rs`: if `assert_task_scrolls_browse_list_to` has no callers left, delete it
  (dead test code fails clippy `--all-targets`). If its body is useful, a small helper
  `assert_task_reveals_browse_row(task, what, viewport_height, scroll_top, expected)` that wraps
  `browse_scroll_effects_at` may replace it. It must still assert that `snapped` is `None`.
- `docs/invariants.md`, section `## Browse playing-row visibility`:
  - Rewrite the Guarantee. The update loop scrolls the browse list only as far as needed to
    show the landed row fully: to the row's top when the row is above the viewport, to its
    bottom edge when it is below, and not at all when it is already visible. The row geometry
    stays `BROWSE_LIST_PADDING + index * BROWSE_ROW_HEIGHT`.
  - Add keypad 1/3 (`jump_ten_tracks`) to the list of steps.
  - Update **Enforced by** to name `scroll::reveal_browse_row` / `scroll::reveal_row_offset`
    instead of `scroll_browse_to_row` and `scroll_to`.
  - Update **Pinned by** with the renamed and new tests.
- `README.md` (Usage, outside the brief markers): the sentence "scroll that list so the landed
  row stays in view" becomes "scroll that list only as far as needed to keep the landed row in
  view, like Winamp's playlist editor". Do the same for Ctrl+Z/Ctrl+B and keypad 1/3 if they
  are described there.

**Files touched.** `src/ui/handlers/browse/scroll.rs`, `src/ui/tests/stepping_tests.rs`,
`src/ui/tests/mod.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**

- `scroll_browse_to_playing_row` schedules `reveal_browse_row` while `Songs` or `AllSongs` shows,
  and returns `Task::none()` otherwise. `scroll_browse_to_row` no longer exists.
- The rewritten stepping tests above pass:
  - A step to a row below a one-row viewport scrolls to that row's bottom minus the viewport.
  - A step to a row above it scrolls to the row's top.
  - A step to a row already inside a tall viewport produces no browse scroll effect.
  - None of these steps snaps.
- `stepping_while_another_browse_level_shows_does_not_scroll`,
  `a_row_press_does_not_scroll_the_list`, the `move_row_tests.rs` reveal tests, and the inline
  `reveal_row_offset` tests in `scroll.rs` pass unchanged.
- No playback, selection, or `TrackSelected` behavior changes: the stepping tests that assert
  the selected index pass unchanged.
- `make check` passes, including `cargo doc` with no broken intra-doc link.

## Done
