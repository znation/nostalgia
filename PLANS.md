# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the usage reference: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. The README's detailed Usage reference moved to
> `docs/usage.md` on 2026-10-26, so plans that change usage prose edit that file, not the README.
> Record any new drift here rather than appending another note.

## Planned

### Show `Custom` in the equalizer preset pick list for a hand-tuned curve (planned 2026-10-26 by the plan role)

**Goal.** Classic Winamp's equalizer preset pick list names the applied preset, and reads
`Custom` once a slider has made the curve the user's own. Nostalgia's pick list instead shows a
lighter `(none)` placeholder whenever `AppState::eq_preset` is `None`, which is also the state
every hand-tuned curve reaches (`set_eq_preamp`, `set_eq_band`, and `set_all_eq_bands` clear the
preset). Show `Custom` as the closed label for that state, in the same text color as a selected
preset, so the pick list reads as a value rather than an empty control. State, handlers, preset
application, and audio forwarding are untouched.

**Approach.**

- `src/ui/views/equalizer.rs`: add a documented `pub(super) const CUSTOM_PRESET_LABEL: &str =
  "Custom";` beside `eq_enabled_label`, and pass it to `PickList::placeholder` in
  `view_equalizer` in place of `"(none)"`. Update `view_equalizer`'s doc comment (the `None`
  branch now draws the `Custom` label) and keep the comment explaining that the slice borrow
  avoids copying `PRESETS` on every rebuild.
- `src/ui/style.rs`: in `chrome_pick_list_style`, change `placeholder_color` to
  [`theme::TEXT`] and rewrite the doc comment: the placeholder now names a real `Custom` curve,
  so it is drawn like the selected preset rather than as an empty box; the hover/opened face and
  border rules stay as they are. Update the assertion in
  `chrome_pick_list_style_sinks_the_face_when_opened` to expect `theme::TEXT`.
- `src/ui/views/tests/equalizer.rs`: add
  `equalizer_pick_list_shows_custom_for_a_hand_tuned_curve`, which lays out
  `view_equalizer(false, 0.0, &[0.0; BAND_COUNT], None)` through
  `layout_capture::capture_text` and asserts the captured texts contain `CUSTOM_PRESET_LABEL`,
  then lays out the same panel with `Some(PRESETS[0])` and asserts it draws that preset's name
  and no `CUSTOM_PRESET_LABEL`. Import `CUSTOM_PRESET_LABEL` in the existing
  `super::equalizer::{…}` list in `src/ui/views/tests/mod.rs`.
- `docs/usage.md`: in the equalizer paragraph, say the preset pick list names the applied
  preset and reads `Custom` once a slider move makes the curve your own.
- `docs/invariants.md`: in `## UI layout`, add a bullet stating the preset pick list shows the
  applied preset's name, or `Custom` when `AppState::eq_preset` is `None`, in `theme::TEXT`;
  name `view_equalizer`/`CUSTOM_PRESET_LABEL` and `chrome_pick_list_style` in `Enforced by`, and
  add the new test to `Pinned by`. Leave the equalizer band-row, tint, and scale guarantees
  unchanged.

**Files touched.** `src/ui/views/equalizer.rs`, `src/ui/views/tests/equalizer.rs`,
`src/ui/views/tests/mod.rs`, `src/ui/style.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- With no applied preset (`eq_preset == None`), the equalizer's closed pick list draws `Custom`;
  with `Some(PRESETS[0])` applied it draws `Flat` and not `Custom` (headless layout capture).
- `chrome_pick_list_style` returns `theme::TEXT` for `placeholder_color`; its updated style test
  passes.
- `AppState::eq_preset` still becomes `None` on a hand-tuned preamp or band change and `Some` on
  a preset pick, picking a preset still emits `Message::EqPresetSelected`, and the saved settings
  format is unchanged.
- `make check` passes.

## Done
