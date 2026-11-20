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

## Done
