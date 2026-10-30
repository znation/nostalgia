# Plans

## Planned

### Save the equalizer's current curve as a named custom preset, like Winamp's preset save row (planned 2026-10-29 by the plan role)

**Goal.** The equalizer's pick list offers exactly the 19 built-in Winamp curves
(`PRESETS: [Preset; 19]`, `src/equalizer.rs`) and reads `Custom` for a hand-tuned one
(`CUSTOM_PRESET_LABEL`, `src/ui/views/equalizer.rs`), so a curve you like has no name and can
only be recalled by setting every slider again. Winamp's preset popup offers a save row beside
the curve list, so a curve can be named and recalled. Give the player a `Save preset…` row: the
current stored gains are saved under a typed name, the pick list then names it beside the
built-ins, picking it applies it like any preset, and the saved curve survives relaunches in
the settings file. Deleting stays out: saving the same name again replaces the saved curve, as
Winamp's own save does; nothing else is removed.

**Approach.** One custom-curve type beside `Preset`, the pick list gains a source enum, one
transient naming panel mirrors the Jump-to-time panel, and a repeated settings line stores the
saved curves.

- `src/equalizer.rs`, beside `Preset`: `pub struct CustomPreset { pub name: String, pub preamp:
  f32, pub bands: [f32; BAND_COUNT] }` — `Preset.name` is a `&'static str`, so customs get their
  own type rather than a compile-time-name copy — plus `pub fn from_curve(name: String, preamp:
  f32, bands: [f32; BAND_COUNT]) -> CustomPreset` that runs `clamp_gain` over the preamp and
  every band at construction, so a stored curve never leaves range. Add the pick list's item
  enum beside it: `pub enum PresetChoice { Save, BuiltIn(Preset), Custom(CustomPreset) }` with
  `Display` (`BuiltIn` and `Custom` render their preset's name, `Save` renders the sentinel
  `Save preset…`) and `PartialEq + Clone` — no `Copy`: `CustomPreset` holds a `String`. The
  `Preset`-taking call sites (`state.rs`'s `apply_eq_preset` path, `mixer.rs`'s handlers) keep
  taking plain `Preset`s; only the pick list and its message use the choice enum. Unit tests:
  `from_curve` clamps a `-13`/`13`/NaN gain, and `Display` names each variant.
- `src/settings.rs` + `src/settings/text.rs`: a new `eq_custom_presets: Vec<CustomPreset>` field
  with an accessor and `Settings::from_state` plumbing beside the other equalizer fields
  (`eq_enabled`…`eq_preset`), parsed and written in `text.rs`. Line format: repeated
  `eq_custom_preset=<name>:<preamp>,<band 1>,…,<band 10>` lines, written with the same float
  format the `eq_preamp`/`eq_band_*` lines use, and written before `eq_preset=` in
  `to_text`. Parsing (`text.rs`'s per-line match beside `parse_eq_preset`): a line without a
  `:`, a blank name, or one whose field count is not `BAND_COUNT + 1` is skipped like the
  other malformed-number lines, and a valid line keeps the text before the first `:` as the
  name and the `BAND_COUNT + 1` gains after it; `from_curve` clamps out-of-range gains. A saved custom's name is never resolved
  into `eq_preset` — an applied custom keeps today's `Custom` closed label after a relaunch,
  with the curve restored through the existing `eq_preamp`/`eq_band_*` lines and re-picked
  from the list (the closed label names only built-ins today by design; that stays).
  Round-trip tests beside the existing `eq_preset` and `time_display` ones: two saved presets
  survive `to_text` → `from_text` with the name, preamp and every band equal; a malformed line
  yields an empty custom list; unknown keys stay ignored.
- `src/ui/player.rs`: `WinampPlayer` gains `custom_eq_presets: Vec<CustomPreset>` (loaded from
  the saved settings in `adopt_saved_settings` and carried into `Settings::from_state` by
  `persist_settings_if_changed`) and the two transient-panel fields the Jump-to-time panel
  holds (`save_preset_open: bool`, `save_preset_name: String`), reset to closed-and-empty
  beside the Jump fields in `WinampPlayer::new`.
- `src/ui/message.rs`: mirror the Jump-to-time trio — `ToggleSavePreset`,
  `SavePresetNameChanged(String)`, `SavePresetSubmitted` — and widen the preset-pick message to
  `EqPresetSelected(PresetChoice)`.
- `src/ui/handlers/mixer.rs`:
  - `eq_preset_selected` routes on the choice: `PresetChoice::BuiltIn(preset)` keeps today's
    `state.apply_eq_preset(preset)` path verbatim; `PresetChoice::Custom(custom)` applies the
    custom's curve through the same `mutate_equalizer` path (stored gains, selection stays
    `None`); `PresetChoice::Save` opens the naming panel instead and applies nothing.
  - `toggle_save_preset` (opens the panel with an empty box and applies the transient-panel
    dismissal rules `docs/invariants.md`'s `## Transient panel dismissal` pins: it closes
    About/Options/File info/Jump panels and closes on backdrop click, Escape-equivalents, and
    any playback or view change that today closes those panels),
    `save_preset_name_changed`, and `save_preset_submitted`: trim the box; an empty name keeps
    the panel open with an inline `Type a name` note; a name equal case-insensitively to any
    `PRESETS` name shows an inline `Already a preset` note and saves nothing; a name
    containing `:` shows an inline `Use it without a colon` note; on success
    build the `CustomPreset` from the stored curve (`state.eq_preamp()`/`eq_bands()`), replace
    an existing save of the same name case-insensitively or push the new one, persist through
    the settings writer, and close the panel. The saved preset is not applied by saving — the
    closed label reads `Custom` for the identical curve as before, exactly as a hand-tuned
    curve does.
  - The panel's text input and its inline notes mirror the Jump-to-time panel's build
    (`src/ui/views/jump_time.rs`) and its view wiring in `src/ui/view/overlay.rs`.
- `src/ui/views/equalizer.rs`: the pick list's items become a per-refresh
  `Vec<PresetChoice>` — `[PresetChoice::Save]` first, then the built-ins in `PRESETS` order,
  then `custom_eq_presets` in stored order; the pick list section's doc comment is updated to
  note the items are rebuilt every refresh and now include the saved customs; the
  `selection` argument is the standing `Option<Preset>` mapped
  to `Some(PresetChoice::BuiltIn(...))` and `None` otherwise, so the closed label still reads
  the applied name or `Custom` through `preset_pick_list_label` unchanged. The layout gains no
  chrome — the added rows live in the pick list popup, nothing is drawn outside existing
  widget primitives; compare the panel against `winamp-screenshots/winamp-2.1-main-eq.gif`
  (EQ docked, `ON`/`AUTO` lit) and keep the base skin's panel exactly as it renders today.
- Document it: `docs/usage.md`'s equalizer paragraph — the pick list names saved curves after
  the built-ins, `Save preset…` opens the naming panel, the naming panel's Enter/backdrop
  rules and its
  per-note errors, same-name save replaces, and the relaunch story (saved list survives; an
  applied custom relaunches showing `Custom` with its curve restored);
  `docs/invariants.md` gains a `## Custom equalizer presets` section pinning the guarantee
  (clamped storage, whole-curve save, case-insensitive same-name replace, built-in-name
  refusal, persistence round trip) to the new tests.

**Files touched.** `src/equalizer.rs`, `src/settings.rs`, `src/settings/text.rs`,
`src/ui/player.rs`, `src/ui/message.rs`, `src/ui/handlers/mixer.rs`, `src/ui/views/equalizer.rs`,
`src/ui/view/overlay.rs`, `docs/usage.md`, `docs/invariants.md`, and the tests beside
`src/ui/tests/mixer_tests.rs`, `src/ui/views/tests/equalizer.rs`, and `src/settings.rs`'s
round-trip block.

**Acceptance criteria.**
- Pick an equalizer curve, move a band, choose `Save preset…` from the pick list's top
  entry, type `Very Fuzzed`, press Enter — the pick list then lists `Very Fuzzed` after
  the built-ins, choosing it applies its whole curve, and the closed label reads `Custom`
  while it is applied (built-in selections keep showing their names).
- Typing `Rock` (a built-in name, any case) and pressing Enter shows the panel's `Already a
  preset` inline note and saves nothing; typing an existing custom's name (any case) saves
  over it with the new curve; an empty box keeps the panel open with the `Type a name` note;
  a backdrop click or other transient-panel dismissal closes it, exactly like Jump to time.
- Saving `Very Fuzzed` twice keeps one row whose gains are the second save's curve; the name
  is stored as first saved.
- Quit and relaunch: the pick list still lists `Very Fuzzed` with its saved curve, applying
  it reproduces the saved gains; an applied custom relaunches with the label `Custom` and the
  same curve restored through the `eq_preamp`/`eq_band_*` lines (the saved `eq_preset=` name
  stays unresolvable for a custom, by the label rule pinned above).
- A saved preset line with a blank name, no colon, or the wrong gain count is skipped at
  parse, and the other `eq_custom_preset=` lines still load; today's settings rules for
  malformed lines are unchanged.
- The existing preset, AUTO, and pick-list behaviors are unchanged: built-in picks, the AUTO
  genre match, `+12/0/-12 DB` scale shortcuts, and the existing equalizer tests pass with the
  message widened to `PresetChoice` (existing call sites wrap built-ins in
  `PresetChoice::BuiltIn`).
- `make check` passes.

## Done
