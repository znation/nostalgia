# Plans

## Planned

### Extract balance and equalizer wheel-routing tests from the mixer module (planned 2026-11-17 by the plan role)

**Goal.** Give root-view slider-wheel event routing a focused test home,
reducing the responsibilities of `src/ui/tests/mixer_tests.rs`. Checked
2026-11-17: the two tests below remain there; the completed wheel-message
extraction explicitly left event-routing tests in place.

**Approach and files.** Move these complete tests, including attached comments
and attributes, into a new `src/ui/tests/slider_wheel_routing_tests.rs` with
`use super::*;`:

- `wheel_over_the_balance_slider_nudges_the_balance_not_the_volume`
- `wheel_over_the_preamp_and_every_band_nudges_that_slider_not_the_volume`

Register the sibling in `src/ui/tests/mod.rs`, remove the original definitions,
and update both tests' attribution in `docs/invariants.md`'s `Slider wheel`
section. Keep the existing shared `captured_root_view_container` and
`root_wheel_messages` fixtures unchanged. Leave message-handler tests in
`slider_wheel_tests.rs` and production code in place. No visible or control
behavior changes are intended; usage and screenshots need no update.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::slider_wheel_routing_tests`, with no copy under
  `ui::tests::mixer_tests`.
- A positive or negative root-view wheel roll over the balance slider emits
  exactly one `BalanceWheel` with direction `1.0` or `-1.0`, respectively,
  and emits no `VolumeWheel`.
- With the EQ panel visible, a positive root-view roll over the preamp emits
  exactly one `EqPreampWheel(1.0)`; the corresponding roll over each band
  emits exactly one `EqBandWheel` for that band's index with direction `1.0`.
  These tested rolls emit no `VolumeWheel`.
- The diff preserves the moved tests' inputs and assertions and contains only
  test relocation, module registration, and invariant attribution changes;
  `make check` passes.

## Done

### Extract preset-name validation tests from the mixer module (planned 2026-11-17 by the plan role)

**Goal.** Give Save preset name-refusal coverage a focused home, reducing the
responsibilities of `src/ui/tests/mixer_tests.rs`. Checked 2026-11-17: the two
tests below remain there, and the archived preset-deletion extraction covers
different tests.

**Approach and files.** Move these complete tests, preserving their comments,
attributes, inputs, and assertions, into a new
`src/ui/tests/save_preset_validation_tests.rs` with `use super::*;`:

- `save_preset_submitted_refuses_empty_builtin_and_colon_names`
- `save_preset_submitted_refuses_a_name_with_a_line_break`

Register the module in `src/ui/tests/mod.rs` and remove the old definitions.
Update `docs/invariants.md`'s `Save preset panel` test attribution to the new
module, including the existing line-break test that the section currently
does not list. Use the existing `test_player` fixture without changing it.
Leave save/replace, custom-choice, deletion, and production code in place.
This extraction is independent of the queued balance-detent extraction.
No visible behavior changes are intended; usage and screenshots need no update.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::save_preset_validation_tests`, with no copy under
  `ui::tests::mixer_tests`.
- With the Save preset panel open, submitting an empty name, `rock`, or
  `A:name` keeps it open, saves no custom preset, and shows the corresponding
  existing empty-name, built-in-name, or colon-name note.
- Submitting `Fuzz\nBass` or `Fuzz\rBass` keeps the panel open, saves no
  custom preset, and shows the existing line-break note; editing the name
  between those submissions clears the previous note.
- The diff preserves the moved test inputs and assertions and contains only
  test relocation, module registration, and invariant attribution changes;
  `make check` passes.
