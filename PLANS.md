# Plans

## Planned

### Extract mixer drag-readout tests into a focused module (planned 2026-11-17 by the plan role)

**Goal.** Give mixer-level LCD replacement and marquee coverage a focused
home, reducing the responsibilities of `src/ui/tests/mixer_tests.rs`.
Checked 2026-11-17: the tests below remain there; the finished event-filter
and seek-drag extractions cover different tests.

**Approach and files.** Move these complete tests, preserving comments,
attributes, inputs, and assertions, into a new
`src/ui/tests/mixer_drag_readout_tests.rs` with `use super::*;`:

- `dragging_volume_shows_the_level_until_release`
- `dragging_an_eq_band_shows_its_gain_until_release`
- `dragging_the_preamp_shows_its_gain`
- `eq_toggle_and_preset_do_not_show_the_readout`
- `an_out_of_range_eq_band_shows_no_readout`
- `dragging_balance_shows_the_side`
- `volume_keys_and_wheel_do_not_show_the_readout`
- `the_marquee_pauses_during_a_mixer_drag`

Register the sibling in `src/ui/tests/mod.rs`, remove the old definitions,
and update their attribution in `docs/invariants.md`'s `Mixer drag readout`
section. Keep existing shared fixtures and production code unchanged.
This is one cohesive test relocation; it changes no player behavior and
requires no usage or screenshot update.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::mixer_drag_readout_tests`, without copies under
  `ui::tests::mixer_tests`.
- With title `Tune`, a volume change to 0.4 or band-zero change to 5 dB
  shows `Volume: 40%` or `EQ: 60HZ +5 DB` instead of the title; release
  restores the title and clears the drag state.
- A preamp change to -3 dB shows `Preamp: -3 DB`; a balance change to
  -0.25 shows `Balance: 25% Left`, with the corresponding drag state.
- Starting without a drag, EQ toggle, built-in preset selection,
  `EqBandChange(BAND_COUNT, 5.0)`, VolumeUp, and VolumeWheel(1.0) leave
  the drag state empty in the existing scenarios.
- For the existing long-title fixture, marquee eligibility is true before
  the volume drag, false during it, and true after release.
- The diff contains test relocation, module registration, and invariant
  attribution changes, preserving the moved inputs and assertions;
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
