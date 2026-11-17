# Plans

## Planned

### Move `src/ui/tests/mod.rs`'s shared test helpers to `src/ui/tests/support.rs` (planned 2026-10-30 by the plan role)

**Goal.** `src/ui/tests/mod.rs` measures 1451 lines and holds zero tests: after its import
preamble the whole span is the shared helper block (48 top-level fns — `test_player`,
`player_with_audio`, `player_stepping_from`, `drive_playback_finished`,
`captured_root_view_text_bounds`, `root_scrollable_ids`, …), followed by the 34 sibling
`mod *_tests;` declarations. `wc -l` puts it at the codebase's second-largest source file,
and every sibling test file reads its helpers through `super`. PRINCIPLES.md pins "each file
small enough to read in one sitting," and the sibling-file pattern is settled in this same
tree (`src/audio/tests/`, `src/apple_music/tests/`, `src/ui/views/tests/`); the settings
test-move plan above applies it to `src/settings.rs`. This entry moves the helper block to
a `support.rs` sibling so `src/ui/tests/mod.rs` is a short hub again. Files are disjoint
from the settings move above, so the two run independently and in either order.

**Ground truth verified this run (2026-10-30).**

- `src/ui/tests/mod.rs` carries 0 `#[test]` fns (`grep -c '#[test]'` = 0) and 48 top-level
  `fn` helpers, all `fn`/`pub`-less, all with doc comments; the block runs from after the
  import preamble to just before the `mod` declarations, whose last entry is `mod
  years_tests;`. No top-level consts or type aliases sit in the block.
- The 34 sibling files (the 34 `*_tests.rs` files besides `mod.rs` in `src/ui/tests/`) each
  import only via `use super::*;`; nothing outside `src/ui/tests/` names `ui::tests::`
  helpers; `docs/invariants.md` and `docs/usage.md` name no helper path.
- A grep of PLANS.md, PLANS-DONE.md, BUGS.md, BUGS-FIXED.md shows no entry covering a
  `ui/tests` helper split (the PLANS-DONE hits name `removal_tests.rs`, composer
tests, and view tests — different work).
- The import mechanics were proven with a standalone `rustc --edition 2021 -D warnings`
  scratch build mirroring this exact layout: `mod.rs` keeps its `use` preamble verbatim;
  `mod support; use support::*;` after the preamble; `support.rs` opens with
  `use super::*;` and its helpers marked `pub(super) fn`; a sibling using `use super::*;`
  resolved every helper and every name the helpers themselves pull through `mod.rs`'s
  imports — clean, with no unused-import and no dead-code diagnostic.

**Approach.**

1. Create `src/ui/tests/support.rs` holding the moved block verbatim: `use super::*;` as
   its preamble, then every helper with its doc comment, each `fn` becoming `pub(super)
   fn` (that is the point of the `pub(super)` marker: `use support::*;` in `mod.rs` can
   pick the helpers up, and the siblings keep reaching them through their untouched
   `use super::*;`).
2. `src/ui/tests/mod.rs`: delete the moved block; insert `mod support;` and
   `use support::*;` directly after the preamble, before the first `mod *_tests;` line.
   The preamble stays exactly as it is — the moved helpers resolve their names through it
   (as verified in the scratch build), and converting it is not needed.
3. No sibling file and no non-test source file is edited; no helper body changes.

Nothing visible to the player changes, so no `winamp-screenshots/` comparison and no
`docs/usage.md` change applies. No new test ships with it — this changes no behavior;
every helper compiles and is called exactly as before.

**Acceptance criteria.**

- `src/ui/tests/mod.rs` reads in one sitting: its preamble, `mod support;`,
  `use support::*;`, and the 34 sibling `mod` declarations — no top-level `fn` remains.
- `src/ui/tests/support.rs` holds only the moved helpers with `use super::*;` as its
  preamble; no sibling `*_tests.rs` file differs from before this change.
- The test suite runs under the same names with the same helpers under `make check`, and
  `make check` passes.

**Size.** One new file (a verbatim move of the helper block with `pub(super)` added per
`fn`), and one deletion plus two added lines in `src/ui/tests/mod.rs`. No behavior change,
so no new tests; out of scope: any split of the topic files (`screen_tests.rs` at 1488,
`readouts_tests.rs`, `mixer_tests.rs`, …), any helper body or signature change, any change
to `crate::test_support`, and the settings test-move above (independent files).

### Extract the title-drag tests from the mixer test module (planned 2026-11-17 by the plan role)

**Goal.** Give the hand-scrolled Now Playing title its own focused test module rather
than keeping its tests among mixer controls. Checked 2026-11-17:
`src/ui/tests/mixer_tests.rs` is 1,372 lines; the three title-drag tests below form a
contiguous group between the mixer-marquee test and the custom-preset tests.
`title_drag_tests.rs` does not exist. Searches of Planned and the finished-plan
archive found no extraction entry; the original hand-scroll plan offered that file
as a possible home, but the tests currently reside in `mixer_tests.rs`.

**Approach and files touched.** Move these complete test functions, including their
attributes and assertions, verbatim from `src/ui/tests/mixer_tests.rs` into a new
`src/ui/tests/title_drag_tests.rs` with `use super::*;`:

- `the_marquee_pauses_while_the_title_is_dragged_and_resumes_from_there`
- `dragging_the_title_right_wraps_backward_and_a_second_drag_starts_from_there`
- `a_new_selection_clears_a_title_drag`

Register `mod title_drag_tests;` in `src/ui/tests/mod.rs`. The existing shared
`test_player` and `seed_current_title` helpers already reach sibling modules through
the test hub; leave their bodies and visibility alone. Leave
`the_marquee_pauses_during_a_mixer_drag` in the mixer module. This is independent
of the queued shared-helper move: preserve whatever helper imports exist when this
plan is implemented.

**Acceptance criteria.**

- The new file contains the three named tests with their original bodies and leaf
  names; the mixer file no longer contains those definitions. Its neighboring
  mixer-marquee and custom-preset tests remain in place.
- The registered sibling compiles and runs under `make check`. Its tests still
  assert that press pauses the marquee, the first move anchors, a 30 px left move
  advances three characters, release resumes at that offset, rightward movement
  wraps backward, a second drag anchors afresh, and selecting a track clears a drag.
- No production behavior or shared helper changes. The `Now Playing label`
  guarantees and named-test citations in `docs/invariants.md` remain accurate;
  update a path citation only if the implementation finds one naming a moved test.
- `make check` passes after the move. No new behavioral tests, usage update, or
  reference screenshot comparison is needed for this test-only reorganization.

**Size.** Three test files including the hub, roughly eighty moved lines. Excludes
other mixer tests, subscription tests, helper extraction, and production title-drag
changes; there is no independently necessary second deliverable in this move.

## Done
