# Bugs

Known bugs, found by any loop or the review gate. Each bug: symptom, how to
reproduce, suspected cause. Move fixed bugs to Fixed.

Each fix records its **Validation gap** in its Fixed entry. When that gap is
`unclear-invariant`, the reconstructed guarantee belongs in `docs/invariants.md`
(see its "How to use" note) as a named section with its enforcing code and tests,
not only in the Fixed entry.

## Open

### The largest test modules are flat files, so a failure's fixtures are far from the test (found by the steward role 2026-10-24)

**Symptom.** One test module is still a single flat file with no inner `mod` splits: `src/ui/tests.rs`
(9,261 lines). A test there can sit thousands of lines from the fixture it uses, so finding that fixture
means grepping or scanning the whole file, and a full read costs most of a role's reading budget.
Concurrent worktrees also keep adding tests to the same file, so their merges collide there. This is a
maintainability risk, not a runtime defect: nothing fails because of it today.

**How to reproduce.** `wc -l src/ui/tests.rs` reports that length and `grep -c '^\s*mod ' src/ui/tests.rs`
reports 0, so it is not split by concern (re-measured 2026-10-24). The other large test modules are
already `tests/` directories (see Progress).

**Suspected cause.** Each `tests.rs` grew by appending tests for every feature that landed in its module;
nothing divides it into submodules the way the production code is divided, so the file keeps growing
without a boundary that prompts a split.

**Direction.** When a bug is next hard to locate inside one of these files, split that module by concern
(for example browse, playback, settings, and layout tests) into sibling submodules or a `tests/`
directory, keeping shared fixtures in `test_support`. Split the file that is being worked in rather than
all four at once, and move tests without changing their assertions.

**Progress.** `src/audio/tests.rs`, `src/apple_music/rest/tests.rs`, and `src/music_kit_auth/tests.rs` are now split into `tests/` directories by concern (the last into `bootstrap`, `tokens`, `authorize`, `http_request`, `server`, and `opener`, with shared helpers in `mod.rs`), tests moved without changing assertions. Also `src/apple_music/tests.rs`, now `tests/` (`validation`, `browse_sample`, `browse_rest`, `play_track`, `audio_forwarding`, `transport`, `threads_and_session`, shared helpers in `mod.rs`). Also `src/ui/views/tests.rs`, now `tests/` (`browse`, `now_playing`, `lcd_display`, `transport`, `equalizer`, `shade`, `chrome`, `seek_wheel`, shared helpers in `mod.rs`). Still flat: `src/ui/tests.rs`; split it when next worked in.

## Fixed
