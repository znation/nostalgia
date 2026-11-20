# Plans

## Planned

### Extract the seek-preview drag and release tests into a focused module (planned 2026-11-20 by the plan role)

**Goal.** Give the seek bar's drag-store and release-commit coverage a focused
home, shrinking `src/ui/tests/readouts_tests.rs` (1259 lines, the largest test
module after `support.rs`). Checked 2026-11-20: the five tests below are still
defined there as one contiguous block; `seek_drag_tests.rs` holds different
tests (the seek readout text, the no-duration text, the marquee pause) and
`seek_wheel_tests.rs` holds the wheel seeks; no finished or pending plan covers
this extraction, and it shares no file with the pending save-panel extraction.

**Approach and files.** Move these complete tests, preserving their comments,
inputs, and assertions, into a new `src/ui/tests/seek_preview_tests.rs` with
`use super::*;`:

- `seek_preview_changed_tracks_the_drag_without_touching_the_backend`
- `seek_preview_released_seeks_once_to_the_dragged_position`
- `seek_preview_released_without_a_drag_does_nothing`
- `seek_preview_released_invalidates_an_in_flight_position_reply`
- `seek_preview_released_backend_failure_leaves_the_position_alone`

Register the sibling in `src/ui/tests/mod.rs` beside the other seek modules and
remove the old definitions. Update `docs/invariants.md`'s `## Now Playing
label` Pinned-by list: the five tests currently rely on that section's
`readouts_tests.rs` default, so after the move each line names the test at
`(`src/ui/tests/seek_preview_tests.rs`)`. Every helper the moved tests use
already lives in `src/ui/tests/support.rs` as `pub(super)`
(`player_with_recording`, `player_with_failing_audio`,
`assert_message_schedules_no_work`) and needs no change, and the moved tests
need no new fixture. This is one cohesive test relocation; it changes no player
behavior and requires no usage or screenshot update.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::seek_preview_tests`, with no copy under
  `ui::tests::readouts_tests`.
- With the recording audio, `SeekPreviewChanged(12.5)` stores
  `Some(Duration::from_millis(12_500))` in `seek_preview` and makes no audio
  call; `SeekPreviewReleased` after a drag issues exactly one
  `SeekTo(Duration::from_secs(12))`, stores the optimistic position, clears
  `seek_preview`, and bumps `position_epoch`, while a release without a stored
  drag makes no call and changes neither position nor epoch.
- After a release, a `PlaybackPosition` reply carrying the pre-release epoch is
  dropped (the dragged position stands), and with a failing seek backend the
  release still reaches the backend once and leaves `seek_preview`, position,
  and epoch unchanged while a later same-epoch position reply is accepted
  again.
- The diff contains only test relocation, module registration, and
  invariant-attribution updates; no moved test's inputs or assertions change.

## Done

_None yet._
