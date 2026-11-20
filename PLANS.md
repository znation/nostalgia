# Plans

## Planned

### Extract the spectrum-reply tests into a focused module (planned 2026-11-20 by the plan role)

**Goal.** Give the LCD visualizer's spectrum-reply handling its own test home,
shrinking `src/ui/tests/readouts_tests.rs` (1259 lines, the largest test module
after `support.rs`). Checked 2026-11-20: the four tests below are still defined
there as one contiguous block; `panels_tests.rs`, `stepping_tests.rs`, and
`subscriptions_tests.rs` only *use* `recording_spectrum` and the stored
spectrum, they define no spectrum-reply tests; no finished or pending plan
covers this extraction, and it shares no file with the pending seek-preview
extraction beyond a one-line `src/ui/tests/mod.rs` registration.

**Approach and files.** Move these complete tests, preserving their comments,
inputs, and assertions, into a new `src/ui/tests/spectrum_tests.rs` with
`use super::*;` (the shape of the recently landed
`src/ui/tests/save_preset_panel_tests.rs`):

- `spectrum_reply_stores_the_bars_and_lets_a_quieter_band_fall`
- `a_stale_spectrum_reply_leaves_the_peaks_alone`
- `spectrum_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`
- `stop_clears_the_spectrum_while_pause_keeps_it`

Also move the shared assertion helper
`assert_reply_dropped_while_stale_or_pending` (currently a private `fn` beside
the format tests in `readouts_tests.rs`) into `src/ui/tests/support.rs` as
`pub(super)`, mirroring `assert_curve_applied` there; the tests that stay in
`readouts_tests.rs` keep calling it through the existing `use super::*;`
chain, so their bodies do not change. Register the sibling in
`src/ui/tests/mod.rs` beside `readouts_tests` and remove the old definitions.
Every other helper the moved tests use already exists: `recording_spectrum`
comes from `crate::test_support` via `mod.rs`'s import, `test_player` and
`play_song_1_then_browse_to_album_2` are `pub(super)` in `support.rs`, `update`
and `Message` reach the new module through the `use super::*;` chain, and
`BAND_COUNT` is imported in `mod.rs` — no new fixture is needed. Update
`docs/invariants.md`'s `## Now Playing label` Pinned-by list: the three list
lines naming `spectrum_reply_stores_the_bars_and_lets_a_quieter_band_fall`,
`spectrum_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped`, and
`stop_clears_the_spectrum_while_pause_keeps_it` currently inherit that
section's `readouts_tests.rs` default, so after the move each of those lines
gains an explicit `(`src/ui/tests/spectrum_tests.rs`)`; the list already
attributes `spectrum_tick_queries_spectrum_and_carries_the_epoch` and the
subscriptions tests to their own files and needs no other edit. This is one
cohesive test relocation; it changes no player behavior and requires no usage
or screenshot update.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::spectrum_tests`, with no copy under
  `ui::tests::readouts_tests`; `assert_reply_dropped_while_stale_or_pending`
  is defined only in `src/ui/tests/support.rs`.
- With the recording audio, a `Message::Spectrum` reply at the current epoch
  with the sample spectrum stores it in both `player.spectrum` and
  `player.spectrum_peaks`; a follow-up reply whose bands are all lower keeps
  the higher held value in each peak slot while the bars take the new reply;
  and a `None` reply clears both fields.
- A `Spectrum` reply stamped with a stale epoch or arriving while a transport
  transition is pending is dropped, a reply at the current epoch without a
  pending transition is stored, and a stale `None` reply leaves stored peaks
  untouched.
- `Message::Pause` keeps both the bars and the peaks; `Message::Stop` clears
  both.
- The diff contains only test relocation, the helper's move to `support.rs`,
  module registration, and invariant-attribution updates; no moved test's
  inputs or assertions change.

## Done

_None yet._

## Done
