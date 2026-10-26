# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the README's Usage section: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. Record any new drift here rather than appending
> another note.

## Planned

### Open the File info panel from the LCD strip's `I`, like Winamp (planned 2026-10-26 by the plan role)

**Goal.** Winamp's main-window indicator strip has a live `I` cell that opens File info for the
current track. Nostalgia draws the same `O A I D V` strip and already has the File info panel via
Alt+3, but `lcd_indicator_message` leaves `I` inert along with `O` and `D`. Publish the existing
`Message::ToggleFileInfo` from the strip's `I` cell, so the current-track gate, panel closing,
About exclusivity, and shade restore all come from the existing handler. No new message, state,
shortcut, or persistence behavior.

**Approach.**

- `src/ui/views/now_playing.rs`:
  - In `lcd_indicator_message`, add `"I" => Some(Message::ToggleFileInfo)` beside the existing
    `"A"` and `"V"` arms, leaving `O` and `D` to the `_ => None` arm.
  - Update the doc comments on `LCD_INDICATOR_LABELS`, `lcd_indicator_message`, and
    `lcd_indicator_strip`: `A`, `I`, and `V` are live controls, `O` and `D` stay decorative, and
    `I` opens the same File info panel as Alt+3.
- `src/ui/views/tests/now_playing.rs`:
  - Rename `lcd_indicator_message_arms_a_and_v_and_leaves_o_i_d_inert` to
    `lcd_indicator_message_arms_a_i_and_v_and_leaves_o_d_inert`. Assert `A` maps to
    `Message::ToggleAlwaysOnTop`, `V` to `Message::ToggleVisualizer`, and `I` to
    `Message::ToggleFileInfo`; keep `O` and `D` inert in the loop.
  - Rename `lcd_indicator_strip_publishes_always_on_top_on_a_and_visualizer_on_v` to
    `lcd_indicator_strip_publishes_always_on_top_on_a_visualizer_on_v_and_file_info_on_i`.
    Drive a real left press at `A`, `I`, `V`, and `O` as it does now: `I` must publish
    `Message::ToggleFileInfo`, `O` must publish nothing, and the `A`/`V` assertions stay
    unchanged.
  - Leave `lcd_indicator_strip_draws_each_label_in_order_at_a_fixed_width` unchanged; the five
    letters and the fixed width do not change.
- `src/ui/handlers/window.rs`: the `toggle_file_info` doc comment starts "Alt+3: flip …"; say
  Alt+3 or the LCD strip's `I`, and leave the handler body alone.
- `src/ui/views/file_info.rs`: the module doc says the panel names the track without naming a
  route, so no change is needed; this plan adds no album or format line.
- `docs/invariants.md`:
  - In `## Now Playing label`, change the indicator-strip bullet and the `Enforced by` sentence
    that say `A` and `V` are live while `O`, `I`, and `D` are decorative: `A`, `I`, and `V`
    publish messages through `lcd_indicator_message`, `I` publishes `Message::ToggleFileInfo`,
    and only `O` and `D` stay decorative.
  - Replace the old test names in that section's `Pinned by` list with the two renamed tests.
  - In `## File info panel`, add a bullet to the Guarantee: pressing the LCD strip's `I`
    publishes the same `Message::ToggleFileInfo` as Alt+3 and goes through the same
    `handlers::window::toggle_file_info` gate, so it does not open without a known current
    track. Add `views::now_playing::lcd_indicator_message` to `Enforced by` and the renamed
    strip-press test to `Pinned by`.
- `README.md` (Usage, outside the brief markers): in the opening sentence about the
  `O A I D V` strip, say `A` toggles always-on-top, `I` opens the File info panel, and `V` hides
  and restores the visualizer, while only `O` and `D` stay decorative. Leave the later Alt+3
  sentence for the waiting album plan, which already edits it.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests/now_playing.rs`,
`src/ui/handlers/window.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**

- `lcd_indicator_message("I")` returns `Some(Message::ToggleFileInfo)`; `"A"` and `"V"` keep
  their existing messages, and `"O"` and `"D"` still return `None`.
- A left press on the laid-out strip's `I` publishes `Message::ToggleFileInfo`; the `A` and `V`
  presses publish their existing messages, and `O` publishes nothing.
- The panel still opens only while `current_known_track()` resolves: the existing
  `toggle_file_info_needs_a_known_current_track`, About-exclusivity, shade-restore, and
  overlay tests pass unchanged.
- No other strip letter, LCD label, playback, settings, or persistence behavior changes.
- `make check` passes, including `cargo doc` with no broken intra-doc link.

## Done
