# Plans

## Planned

### Pin docs/invariants.md's Pinned-by entries to the tests that define them (planned 2026-11-20 by the steward role)

**Goal.** Every fix whose validation gap is `unclear-invariant` — the largest
non-`none` tag in the archive: `grep -sohE 'gap:[*]{0,2} ?[a-z-]+' BUGS.md
BUGS-FIXED.md | sed -E 's/^gap:[*]{0,2} ?//' | sort | uniq -c` reads 14
`unclear-invariant` of 26 non-`none` entries, ahead of `no-observability` 4,
`real-run-needed` 4, `no-repro` 3, `no-fake` 1 — closes by recording the
reconstructed guarantee in `docs/invariants.md` in the same commit (four
carriers: "A persist before a relaunch's sign-in lands erases the saved
`music_kit_user_token=` line from the settings file", "A song tagged literally
“Unknown” shows a second “Unknown” row in Genres or Composers", "Preview
download rejects a body of exactly the 10 MiB cap", "Shuffle's Previous skips a
play paused before its completion"). That record now spans 61 sections and 61
`**Pinned by.**` lists — 501 bulleted backticked test names plus several
sections' inline lists — and its whole value is those lists staying true: a
renamed, moved, or deleted test leaves a section asserting a pin that no longer
exists, so the next bug in that area reads a stale guarantee instead of
skipping the reconstruction, which is the exact cost the tag measures. Nothing
reads the file: no `src/` code, test, or `Makefile` target references
`docs/invariants.md` (`grep -rn invariants src Makefile` is empty), and
`src/repo_hygiene.rs`'s one scan test covers backup artifacts only. The manual
burden is visible in the live queue: both currently planned test extractions
spend acceptance-criteria lines keeping Pinned-by attribution right.

**Approach.**
- Normalize the Pinned-by conventions first, list by list (a docs-only change,
  each resolution verified by grep against `src/`): every `**Pinned by.**`
  list states its default module once right after the heading (or uses
  per-line paths throughout), every entry then carries or inherits exactly one
  resolvable path, and the file's "How to use" note documents the convention.
  Today the lists mix bare names with no stated default at all (the
  `## Browse hierarchy` list's first ~24 entries), per-line `(in `path`)`
  parentheticals, and group `(all in `path`)` notes, so a bare name's home is
  only findable by searching.
- Then add the check as a `src/repo_hygiene.rs` scan test (standard library
  only, the same read-and-walk shape as
  `no_backup_artifacts_ship_in_the_repository`): parse each `**Pinned by.**`
  list in `docs/invariants.md` and assert every backticked name resolves to a
  test function defined in the file its convention assigns it — failing on a
  name no file defines and on a name its explicit path does not define. The
  scan reads repository text only; it never runs the suite.
- Out of scope: the `**Enforced by.**` prose and each section's guarantee
  wording (not machine-checkable without re-deriving the guarantee), any
  player behavior, `docs/usage.md`, and screenshots. This plan changes no
  player code and requires no usage or screenshot update.

**Acceptance criteria.**
- The scan test passes on the normalized file and demonstrably fails when one
  pinned name is misspelled and when one explicit path points at a file that
  does not define it — the change proves both failure modes.
- Every `**Pinned by.**` list states its resolution convention, and the
  "How to use" note names it, so the next test-extraction plan edits paths
  without guessing a default.
- `grep -rn invariants src` then finds the scan module; the scan is standard
  library only and adds no dependency.
- The diff touches only `docs/invariants.md` and the scan test module; the
  landing commit cites this entry by its exact title.

### Extract the position-poll tests into a focused module (planned 2026-11-20 by the plan role)

**Goal.** Continue shrinking `src/ui/tests/readouts_tests.rs` (869 lines
after the file-info merge) by giving the position poll's transition-gating
and reply-storage tests their own home — the follow-up the waiting
position-tick extraction defers. Checked 2026-11-20: the four tests
`position_poll_waits_for_a_stop_to_reach_the_backend`,
`position_poll_waits_for_a_selected_tracks_play_to_settle`,
`an_unrelated_transport_completion_keeps_the_poll_waiting`, and
`position_replies_store_and_stale_ones_are_dropped` form one contiguous
block (from the `// A Stop resets the readout before the backend stop is
scheduled` comment above the first test through the last test's closing
brace), and the block defines no helpers, so nothing else can move with it;
its neighbors `stop_clears_the_elapsed_readout` (before) and
`playback_format_reply_stores_the_loaded_previews_format` (after) keep
compiling after the removal. Every helper the block calls
(`play_song_1_then_browse_to_album_2`, `update`,
`assert_message_schedules_work`, `assert_message_schedules_no_work`) is
`pub(super)` in `src/ui/tests/support.rs` or reachable via `super::*`, and
`stepping_songs` comes from `crate::test_support`. Grep on PLANS-DONE.md
and PLANS.md finds no finished or pending plan covering a
`position_poll_tests` module. It shares `readouts_tests.rs`, `mod.rs`, and
docs/invariants.md with the waiting position-tick extraction, but touches
a disjoint region of the test file (its block starts at `fn tick_replies`,
after this block) and different Pinned-by lines in the same `## Now
Playing label` run, so the two merge independently in either order.

**Approach.**
- Create `src/ui/tests/position_poll_tests.rs` starting with
  `use super::*;` and move into it, unchanged, the four tests above with
  their preceding doc comments.
- Register `mod position_poll_tests;` in `src/ui/tests/mod.rs`
  alphabetically among the test modules, after `panels_tests` and before
  `position_tick_tests` once that module lands (before `randomize_tests`
  until then).
- docs/invariants.md, `## Now Playing label`: the four moved tests sit in
  one consecutive Pinned-by run, so the first of them
  (`position_replies_store_and_stale_ones_are_dropped`) gets the explicit
  `(`src/ui/tests/position_poll_tests.rs`)` and the run covers the rest —
  per the plan-role lesson on parenthetical runs. That retag makes the
  kept run that follows inherit the new module path until the next
  explicit parenthetical (`a_paused_preview_needs_the_position_poll...`),
  so in the same change give every kept line in that stretch an explicit
  parenthetical naming its test's real defining home, found by grepping
  each test's `fn` in src/: if the position-tick extraction has already
  landed, those lines already carry explicit parentheticals naming
  `position_tick_tests.rs`/`subscriptions_tests.rs` and need no edit; if
  it has not, the three `position_tick_*` lines name
  `src/ui/tests/readouts_tests.rs` and the five
  subscriptions-side lines (`position_tick_stream_emits_position_ticks`,
  `a_refused_position_thread_ends_the_tick_stream_instead_of_panicking`,
  `a_refused_position_thread_reports_the_cause_and_the_loss`,
  `the_position_thread_is_named_nostalgia_position_poll`,
  `the_position_timer_runs_only_while_there_is_a_position_to_refresh`)
  name `src/ui/tests/subscriptions_tests.rs` (all five homes checked
  2026-11-20). This keeps the merge self-consistent in either order. Also
  normalize the doubled `((...))` parentheses on
  `now_playing_label_keeps_the_track_name_after_browsing_to_another_album`
  in the same section to single.
- `a_paused_preview_needs_the_position_poll_before_its_first_reply` stays
  in `readouts_tests.rs`: it is not contiguous with the block, and the
  waiting position-tick plan already normalizes its doubled
  parenthetical.
- No production code changes.

**Files touched.** `src/ui/tests/readouts_tests.rs`,
`src/ui/tests/position_poll_tests.rs` (new), `src/ui/tests/mod.rs`,
docs/invariants.md.

**Acceptance criteria.**
- `make test-one TEST=position_poll` runs both `position_poll_*` tests
  under `ui::tests::position_poll_tests` with no copy under
  `ui::tests::readouts_tests`; grep finds `fn
  an_unrelated_transport_completion_keeps_the_poll_waiting` and `fn
  position_replies_store_and_stale_ones_are_dropped` only in
  `position_poll_tests.rs`.
- `readouts_tests.rs` defines none of the four moved tests, and its kept
  neighbors around the removed block (`stop_clears_the_elapsed_readout`,
  `playback_format_reply_stores_the_loaded_previews_format`) still
  compile and pass.
- Every Pinned-by line naming one of the four moved tests resolves to
  `src/ui/tests/position_poll_tests.rs`, and every Pinned-by line in the
  corrected run that follows names its test's real defining file, checked
  by grepping each test's `fn` in src/; the doubled `((...))` on
  `now_playing_label_keeps_the_track_name_after_browsing_to_another_album`
  is single after the change.
- The diff contains only test relocation, module registration, and
  invariant-attribution updates; no moved test's inputs or assertions
  change.

## Done
