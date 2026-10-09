# Developer convenience targets for the nostalgia crate.
#
# The project's landing gate is exactly: `cargo fmt --check` clean,
# `cargo clippy --all-targets -- -D warnings` clean, `cargo doc` clean
# (rustdoc warnings denied), and the test suite green.
# `make check` runs that whole gate in one command; the granular targets
# let a contributor run just the part they changed.
#
# Every cargo invocation that reads the dependency graph passes `--locked`,
# so the committed Cargo.lock is authoritative: if Cargo.toml and the lock
# have drifted apart, the gate fails instead of silently rewriting the lock
# mid-check and passing on a dependency set nobody committed. (`cargo fmt`
# is the exception — it resolves no dependencies and takes no `--locked`.)

.PHONY: check test test-one test-one-guard-test fmt lint fix docs run clean

## The full landing gate: formatting, lints, docs, then tests.
check: fmt lint docs test test-one-guard-test

## Runs the test suite against the committed lockfile.
test:
	cargo test --locked

## Runs only the tests whose names contain TEST, e.g.
## `make test-one TEST=shuffled_pick`.
##
## Matches only the lines of libtest's `--list` output that name a test. The
## trailing summary line (`N tests, 0 benchmarks`) is not a test name, so a
## TEST value that appears only there (for example `benchmarks`) does not
## satisfy the guard. Shared with `test-one-guard-test`, which exercises it.
TEST_LIST_HAS_TEST = grep -q ': test$$'

## The cargo command `test-one` invokes. It is a variable so
## `test-one-guard-test` can substitute a stub and exercise the target's
## branches without running a real build.
CARGO ?= cargo

## A focused alternative to the full `test` target while iterating on one
## behavior. Cargo's test harness treats its first positional argument as a
## substring filter, so the target forwards TEST after `--` (which separates
## cargo's own options from the test binary's). Two guards keep a mistyped
## invocation from reading as success: an empty TEST is rejected with a usage
## line rather than silently running the whole suite (which is what
## `cargo test` does with no filter), and a TEST that matches no test name is
## rejected rather than running nothing — libtest exits `0` on zero matches,
## so without the second guard a typo would report a pass. The match is
## checked against the harness's `--list` output filtered to the lines that
## name a test, so the trailing summary (`N tests, 0 benchmarks`) cannot
## satisfy the guard; only a real test name can. The list command's output is
## captured and its exit status checked separately, so a build failure reprints
## the compiler diagnostics and then a `make test-one: the test build failed`
## line rather than being reported as a mistyped name.
test-one:
	@test -n "$(TEST)" || { echo "usage: make test-one TEST=<name>"; exit 2; }
	@listed=$$($(CARGO) test --locked -- --list "$(TEST)" 2>&1); status=$$?; \
		if [ $$status -ne 0 ]; then printf '%s\n' "$$listed"; echo "make test-one: the test build failed"; exit 1; fi; \
		printf '%s\n' "$$listed" | $(TEST_LIST_HAS_TEST) || { echo "no test name contains '$(TEST)'"; exit 1; }
	$(CARGO) test --locked -- $(TEST)

## Exercises the `test-one` guards. The first two lines test the match pattern
## against synthetic `--list` output: a line naming a test matches, and the
## trailing summary alone does not, using the same `TEST_LIST_HAS_TEST` variable
## as the target. The last two invoke the recipe itself through `$(MAKE)` with
## `CARGO` stubbed to a command that fails (`false`) and one that prints no test
## names (`true`), asserting each cause is reported distinctly. No real build
## runs, so it is cheap enough to run inside `make check`.
test-one-guard-test:
	@printf 'a::b: test\n\n1 test, 0 benchmarks\n' | $(TEST_LIST_HAS_TEST) || { echo "test-one guard rejected a real test name"; exit 1; }
	@printf '0 tests, 0 benchmarks\n' | $(TEST_LIST_HAS_TEST) && { echo "test-one guard accepted a summary-only list"; exit 1; } || true
	@out=$$($(MAKE) --no-print-directory test-one TEST=anything CARGO=false 2>&1); st=$$?; \
		if [ $$st -eq 0 ] || ! printf '%s\n' "$$out" | grep -q 'make test-one: the test build failed' || printf '%s\n' "$$out" | grep -q 'no test name contains'; then \
			echo "test-one did not report a failed build distinctly"; exit 1; fi
	@out=$$($(MAKE) --no-print-directory test-one TEST=anything CARGO=true 2>&1); st=$$?; \
		if [ $$st -eq 0 ] || ! printf '%s\n' "$$out" | grep -q "no test name contains 'anything'" || printf '%s\n' "$$out" | grep -q 'test build failed'; then \
			echo "test-one did not report a mistyped name distinctly"; exit 1; fi

## Checks formatting without editing files (fails on any diff).
fmt:
	cargo fmt --check

## Lints every target with clippy, failing on any warning.
##
## `-D warnings` turns the clippy run into the "clippy clean" the landing
## gate means: without it, a tree that introduces a warning still passes
## `make check`, misreporting a change as merge-ready.
lint:
	cargo clippy --locked --all-targets -- -D warnings

## Applies formatting and clippy's machine-applicable fixes in place.
##
## The writing twin of `fmt`/`lint`: a contributor runs it mid-change to
## apply what those read-only targets would otherwise report. The two
## `--allow-*` flags let it rewrite a tree with uncommitted or staged edits,
## which is the point of running it before committing. `cargo fmt` runs last
## so the tree is formatted after any clippy rewrite, and the lockfile flag
## keeps the committed Cargo.lock authoritative as in the other cargo
## targets. It does not fail on a lint clippy cannot fix automatically; run
## `make lint` or `make check` after it to see what remains.
fix:
	cargo clippy --locked --fix --allow-dirty --allow-staged --all-targets
	cargo fmt

## Builds the docs, failing on any rustdoc warning.
##
## `-D warnings` is what turns a broken intra-doc link or bare URL into a
## failed run instead of a printed warning. Cargo already passes
## `--document-private-items` for this binary crate (`cargo doc -v` shows it
## in the rustdoc invocation), so rustdoc documents and checks the crate's
## private items too; no extra flag is needed.
docs:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps

## Builds and runs the player window.
run:
	cargo run --locked

## Removes build artifacts.
clean:
	cargo clean
