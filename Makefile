# Developer convenience targets for the nostalgia crate.
#
# The project's landing gate is exactly: `cargo fmt --check` clean,
# `cargo clippy --all-targets -- -D warnings` clean, `cargo doc` clean
# (rustdoc warnings denied), and the test suite green.
# `make check` runs that whole gate in one command; the granular targets
# let a contributor run just the part they changed.

.PHONY: check test fmt lint docs run clean

## The full landing gate: formatting, lints, docs, then tests.
check: fmt lint docs test

## Runs the test suite.
test:
	cargo test

## Checks formatting without editing files (fails on any diff).
fmt:
	cargo fmt --check

## Lints every target with clippy, failing on any warning.
##
## `-D warnings` turns the clippy run into the "clippy clean" the landing
## gate means: without it, a tree that introduces a warning still passes
## `make check`, misreporting a change as merge-ready.
lint:
	cargo clippy --all-targets -- -D warnings

## Builds the docs, failing on any rustdoc warning.
##
## `-D warnings` is what turns a broken intra-doc link or bare URL into a
## failed run instead of a printed warning. rustdoc only checks the doc
## comments of the items it documents, and without `--document-private-items`
## that is the public surface: a broken link in a private item's docs is not
## reported. Passing the flag both renders private items into the generated
## HTML and extends the link check to them.
docs:
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

## Builds and runs the player window.
run:
	cargo run

## Removes build artifacts.
clean:
	cargo clean
