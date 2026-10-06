# Developer convenience targets for the nostalgia crate.
#
# The project's landing gate is exactly: `cargo fmt --check` clean,
# `cargo clippy --all-targets` clean, and the test suite green.
# `make check` runs that whole gate in one command; the granular targets
# let a contributor run just the part they changed.

.PHONY: check test fmt lint run clean

## The full landing gate: formatting, lints, then tests.
check: fmt lint test

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

## Builds and runs the player window.
run:
	cargo run

## Removes build artifacts.
clean:
	cargo clean
