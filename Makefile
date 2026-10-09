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

.PHONY: check test test-one fmt lint fix docs run clean

## The full landing gate: formatting, lints, docs, then tests.
check: fmt lint docs test

## Runs the test suite against the committed lockfile.
test:
	cargo test --locked

## Runs only the tests whose names contain TEST, e.g.
## `make test-one TEST=shuffled_pick`.
##
## A focused alternative to the full `test` target while iterating on one
## behavior. Cargo's test harness treats its first positional argument as a
## substring filter, so the target forwards TEST after `--` (which separates
## cargo's own options from the test binary's). The guard rejects an empty
## TEST with a usage line rather than silently running the whole suite, which
## is what `cargo test` would do with no filter.
test-one:
	@test -n "$(TEST)" || { echo "usage: make test-one TEST=<name>"; exit 2; }
	cargo test --locked -- $(TEST)

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
