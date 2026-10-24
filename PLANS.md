# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; updated 2026-10-20).** The initial prompt
> names two commitments — a "very close if not pixel-perfect" classic Winamp UI
> and Apple Music as the library. The first is now moving: the base-skin
> palette landed ("Add a Winamp 2.x base-skin palette and apply it as the app
> theme", done 2026-10-06), and the reusable bevel layer followed ("Add a
> Winamp two-tone bevel layer and frame the Now Playing and equalizer panels",
> done 2026-10-06), framing those two panels through `bevel::lcd_well` and
> `bevel::raised_panel` in `src/ui/bevel.rs`. The transport-button chrome plan
> landed ("Style the transport buttons as raised Winamp chrome", done
> 2026-10-07), and both transport toggles are reachable: the Repeat toggle
> landed 2026-10-06 and the Shuffle toggle 2026-10-08, and their R and S
> keyboard bindings followed ("Add the R and S keyboard bindings for Repeat
> and Shuffle", done 2026-10-20). The rest of widget-level fidelity has
> landed too: slider chrome ("Style the volume and
> equalizer sliders as sunken Winamp grooves with raised chrome thumbs", done
> 2026-10-07), a custom title bar ("Add a Winamp custom title bar and drop the
> OS window frame", done 2026-10-07), and playlist chrome ("Frame the browse
> list as a sunken Winamp playlist editor", done 2026-10-07). The library side
> is real:
> `AppleMusicService` answers its browse and search queries through the REST
> client once a `MusicKit` session is stored, falling back to
> `sample_library()` otherwise — playlist queries included — and `play_track`
> plays the selected preview through the audio seam. Risk: the Winamp chrome
> arrived piecemeal, so later changes still need one consistent vocabulary.
> The equalizer-audio and seek plans this note pointed at have since landed
> (both 2026-10-20); the 2026-10-22 note below records drift that has since
> been resolved.

> **Steward drift note (2026-10-22 by the steward role; resolved 2026-10-23
> by the steward role).** That note recorded the built Now Playing bar and
> browse rows drifting from the brief's mockups — no elapsed readout, no
> `O A I D V` strip, no status row, album rows without running time, and a
> queued playback-position plan that put an `<elapsed>/<total>` readout at the
> LCD's right edge. Those drifts have since landed: "Add the classic elapsed
> readout to the Now Playing LCD and restore the plain total at its right
> edge", "Draw the classic O A I D V strip at the LCD's left edge", "Show the
> loaded preview's bitrate in the LCD status row", "Show each album row's
> running time", and "Show each artist row's album count" are done, and the
> playback-position plan is no longer queued. No current brief-vs-build drift
> is known. The LCD `MONO`/`STEREO` pair has since landed (done 2026-10-23),
> which brought the status row to `docs/screenshots/player.png` and
> `docs/screenshots/transport.png`, and the volume and balance slider fill
> has since landed (done 2026-10-23) with the orange and green grooves those
> mockups draw, and the playlist-number title prefix landed too (done
> 2026-10-23), matching `docs/screenshots/player.png`, and the transport
> glyph row landed on the same day. The select-then-open plan has since landed
> (done 2026-10-23).

## Planned

### Show and hide the equalizer panel from an EQ button on the player (planned 2026-10-23 by the plan role)

**Goal.** Match `docs/screenshots/equalizer.png`'s "MAIN + EQ" caption — "The EQ button on the
main window opens and closes the equalizer docked beneath it" — and the `EQ` square beside the
balance slider in `docs/screenshots/player.png`, `hero.png`, and `library.png`. Today `view`
always pushes the `NOSTALGIA EQUALIZER` band and `view_equalizer` panel, and the main transport
row has no EQ button, so the panel cannot be closed. This adds a player-row `EQ` toggle for the
panel's visibility. It is a layout-only flag: the equalizer's on/off state, curve, preset, and
audio forwarding are untouched, the window is not resized, and hiding the section hands its
height to the browse list. The `PL` button the mockups also draw is out of scope; the library
section is unchanged.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) eq_panel_visible: bool` to `WinampPlayer`,
documented as whether the equalizer section is drawn; initialize it true in `new()` (open at
launch, like the mockup) and do not add it to settings, since shade and visualizer visibility
are not persisted either.
- `src/ui/message.rs`: add `ToggleEqualizerPanel` beside `ToggleEqualizer`, with a comment saying
it flips only the panel-visibility flag — not the EQ's on/off state or its curve — and schedules
no work.
- `src/ui/mod.rs`: dispatch `Message::ToggleEqualizerPanel` to
`handlers::mixer::toggle_equalizer_panel`.
- `src/ui/handlers/mixer.rs`: add `pub(in crate::ui) fn toggle_equalizer_panel(player: &mut
WinampPlayer) -> Task<Message>` next to `toggle_equalizer`; it flips
`player.eq_panel_visible` and returns `Task::none()`. Its doc distinguishes the audio on/off
button (`toggle_equalizer`) from this layout-only toggle.
- `src/ui/views/transport.rs`: add
`pub(super) fn eq_panel_toggle() -> (&'static str, f32, Message)` returning `("EQ",
EQ_PANEL_BUTTON_WIDTH, Message::ToggleEqualizerPanel)` with `EQ_PANEL_BUTTON_WIDTH` = 30.0, and
`pub(super) fn eq_panel_button_style(open: bool, status: button::Status) -> button::Style`
returning `style::chrome_button_style(button::Status::Pressed)` while open and
`style::chrome_button_style(status)` otherwise (the title bar's A toggle sunken-while-on rule).
Extend `view_transport_controls` with an `eq_panel_visible: bool` parameter and, after the
balance slider and a `spacer(20.0)`, push the fixed-width `EQ` button built from those helpers.
Update the module and function docs to name the added button.
- `src/ui/view.rs`: make `player_panels` mutable, pass `player.eq_panel_visible` to
`view_transport_controls`, and push the `NOSTALGIA EQUALIZER` band and `view_equalizer` only when
`player.eq_panel_visible`. Update the module and `view` docs: the equalizer section is normal-mode
chrome the player's EQ button shows and hides, and hiding it lets the browse list take that
height without an `iced::window::resize`.
- `src/ui/views/tests.rs`: import `eq_panel_button_style` and `eq_panel_toggle`; update
`transport_controls_construct_for_both_play_states_volume_endpoints_and_repeat_states` to pass
both visibility states; add `eq_panel_toggle_targets_the_visibility_flag` pinning the label,
width, and `Message::ToggleEqualizerPanel`, and
`eq_panel_button_style_sinks_while_the_panel_is_open`, asserting the open style's background,
border, and shadow match `style::chrome_button_style(button::Status::Pressed)` for a resting
status while the closed style follows the status passed in.
- `src/ui/tests.rs`: add `toggle_equalizer_panel_hides_and_restores_the_section`: lay out the root,
assert `NOSTALGIA EQUALIZER` and `EQ: On` draw; drive `Message::ToggleEqualizerPanel` through
`update`, assert both are gone while `NOSTALGIA LIBRARY`, the browse list, and the row's `EQ`
button remain and the library band's `y` moved up; toggle back and assert both return at their
original `y`; also assert `AppState::eq_enabled`, `eq_preamp()`, and `eq_bands()` are unchanged
across both toggles.
- `README.md`: in the Usage paragraph's equalizer sentence, say an `EQ` button beside the balance
slider shows and hides the equalizer panel (open at launch), and that hiding it gives the height
to the browse list without changing EQ processing. Do not edit the
`<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## UI layout`, add a bullet: the main transport row's fixed-width `EQ`
toggle is present in every playback state and is sunken while the panel is shown; `view` draws
the `NOSTALGIA EQUALIZER` band and `view_equalizer` exactly when `WinampPlayer::eq_panel_visible`
holds, and hiding them moves the library band and browse list up without resizing the window or
changing EQ state. Name `views::transport::eq_panel_toggle`/`eq_panel_button_style`,
`handlers::mixer::toggle_equalizer_panel`, and `view` as the enforcers, and add the new test
names to `Pinned by`.

**Files touched.** `src/ui/player.rs`, `src/ui/message.rs`, `src/ui/mod.rs`,
`src/ui/handlers/mixer.rs`, `src/ui/views/transport.rs`, `src/ui/view.rs`,
`src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The main transport row draws a fixed-width `EQ` button after the balance slider in every
playback state; pressing it emits `Message::ToggleEqualizerPanel`, and the button is sunken while
the panel is open and follows the normal chrome faces while closed.
- The `NOSTALGIA EQUALIZER` title band and equalizer panel are drawn only while
`eq_panel_visible` is true. Hiding them removes both, keeps `NOSTALGIA LIBRARY`, the browse
controls, and the browse list, and moves the library section up to use the freed height; the
window is not resized.
- Toggling the panel leaves `AppState::eq_enabled`, the preamp, the band gains, and the preset
unchanged and forwards nothing to the audio backend, so hiding the panel does not bypass the
curve.
- The flag starts true at launch and is not read from or written to settings; shade mode still
drops both sections as today, and hiding the panel before shading does not change the shaded
strip.
- README and `docs/invariants.md` describe the toggle and the layout guarantee, and `make check`
passes.

## Done
