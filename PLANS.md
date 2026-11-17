# Plans

## Planned

### Extract mixer readout event-filter tests into a focused module (planned 2026-11-17 by the plan role)

**Goal.** Give the navigation-key and wheel dismissal contract a focused test
home, reducing the responsibilities of `src/ui/tests/mixer_tests.rs`.
Checked 2026-11-17: the tests below still live there; the archived volume-input
and slider-wheel extractions cover different message-handling tests.

**Approach and files.** Move these complete tests, preserving comments,
attributes, inputs, and assertions, into a new
`src/ui/tests/mixer_readout_input_tests.rs` with `use super::*;`:

- `a_key_or_wheel_event_ends_a_mixer_readout`
- `only_the_slider_navigation_keys_end_a_mixer_readout`

Register the sibling in `src/ui/tests/mod.rs` and remove the old definitions.
Update `docs/invariants.md`'s `Mixer drag readout` attribution for the first
test and add the second test alongside it. Keep the shared fixtures and
`subscriptions::mixer_key_message` / `subscriptions::mixer_wheel_message`
in `src/ui/subscriptions/listeners.rs` unchanged. Drag rendering, marquee,
volume-step, and slider-wheel tests remain in their existing modules.
This is one cohesive event-filter extraction, with no product decision or
visible behavior change; usage and screenshots need no update.

**Acceptance criteria.**

- Test discovery lists each named test once under
  `ui::tests::mixer_readout_input_tests`, without a copy under
  `ui::tests::mixer_tests`.
- Given a volume drag readout, a captured ArrowUp press or captured wheel
  event produces a dismissal message that clears `mixer_drag` when handled.
- Arrow, Home, End, PageUp, and PageDown presses produce `MixerReleased`;
  their releases, Enter/Space/Tab/Escape presses, typing `a`, and the tested
  unrelated mouse events produce no message from the corresponding filter.
- The diff preserves the moved test inputs and assertions and is limited to
  relocation, module registration, and invariant attribution changes.
  `make check` passes after extraction.

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
