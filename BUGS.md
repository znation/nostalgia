# Bugs

Known bugs, found by any loop or the review gate. Each bug: symptom, how to
reproduce, suspected cause. Move fixed bugs to Fixed.

Each fix records its **Validation gap** in its Fixed entry. When that gap is
`unclear-invariant`, the reconstructed guarantee belongs in `docs/invariants.md`
(see its "How to use" note) as a named section with its enforcing code and tests,
not only in the Fixed entry.

## Open

### The largest test modules are flat files, so a failure's fixtures are far from the test (found by the steward role 2026-10-24)

**Symptom.** The five largest test modules are single flat files with no inner `mod` splits: `src/ui/tests.rs`
(8,769 lines), `src/ui/views/tests.rs` (3,190), `src/apple_music/tests.rs` (3,185),
`src/music_kit_auth/tests.rs` (1,816), and `src/apple_music/rest/tests.rs` (1,734). A test in
`src/ui/tests.rs` can sit thousands of lines from the fixture it uses, so finding that fixture means
grepping or scanning the whole file, and a full read costs most of a role's reading budget. Concurrent
worktrees also keep adding tests to the same file, so their merges collide there. This is a
maintainability risk, not a runtime defect: nothing fails because of it today.

**How to reproduce.** `wc -l` on the paths above reports those lengths, and
`grep -c '^\s*mod '` reports 0 for each of the five, so none is split by concern.

**Suspected cause.** Each `tests.rs` grew by appending tests for every feature that landed in its module;
nothing divides it into submodules the way the production code is divided, so the file keeps growing
without a boundary that prompts a split.

**Direction.** When a bug is next hard to locate inside one of these files, split that module by concern
(for example browse, playback, settings, and layout tests) into sibling submodules or a `tests/`
directory, keeping shared fixtures in `test_support`. Split the file that is being worked in rather than
all five at once, and move tests without changing their assertions.

## Fixed
