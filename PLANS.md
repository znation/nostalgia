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

### Open the selected browse row with Enter (planned 2026-10-23 by the plan role)

**Goal.** Complete the browse selection flow with the keyboard: after a click selects a row,
pressing Enter should open it exactly as a second click or the `Open` button does. The
select-then-open plan already added `Message::OpenSelected` and
`handlers::browse::open_selected`, which resolves the showing list's selected row and dispatches
the existing artist/album/song/playlist action, but `shortcuts::shortcut` binds only the letters
and arrow keys, so Enter currently does nothing. This adds that one binding. It adds no message,
handler, fetch, or playback path, and it leaves the search box's Enter behavior alone.

**Approach.**

- `src/ui/shortcuts.rs`: in `shortcut`, add
`Key::Named(key::Named::Enter) => Some(Message::OpenSelected)` beside the arrow-key arms. Update
the `message_for` and `shortcut` docs to list Enter as the browse-row activation key. Enter
shares the existing Control/Alt/Logo rejection, so a chord still maps to nothing; a plain or
Shift-held Enter maps.
- `src/ui/shortcuts.rs` tests: add `message_for_maps_enter_to_open_selected`, using the existing
`key_pressed` helper and asserting `Message::OpenSelected`. Add `Key::Named(key::Named::Enter)`
to the key list in `shortcut_ignores_chord_modifiers`. The existing release/non-press test already
proves a key release maps to nothing.
- `src/ui/tests.rs`: add `enter_opens_the_selected_song`, building on
`open_selected_opens_the_selected_song`'s fixture. Seed the browse lists, show `CurrentView::Songs`,
select index 0 through `Message::SelectRow`, map a plain Enter key press through
`shortcuts::message_for`, and assert it is `Message::OpenSelected`; drive that message through
`update` and assert the same `TrackSelected` output and committed `song-1` playback the Open
button test asserts. This pins the key-to-action wiring end to end without introducing another
open path.
- `README.md`: in the Usage sentence describing browse selection, add Enter alongside the
second click and the `Open` button as a way to open the selected row. Do not edit the
`<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Browse selection`, add Enter as another activation route for the
selected row, with no selection being a no-op, and note that the search box's own Enter is
captured by `TextInput::on_submit` and remains `Message::SearchSubmitted` (the keyboard
subscription receives only ignored events). Name `shortcuts::message_for`, `Message::OpenSelected`,
and `handlers::browse::open_selected` as the enforcers, and add
`message_for_maps_enter_to_open_selected` and `enter_opens_the_selected_song` to `Pinned by`.

**Files touched.** `src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- A plain or Shift-held Enter key press maps to `Message::OpenSelected`; a key release,
modifier-only event, or Control/Alt/Logo chord still maps to nothing.
- With a row selected on any browse level, Enter opens it through the existing `open_selected`
path, producing the same action as the `Open` button; with no selected row it is a no-op that
schedules no fetch or playback task.
- Enter while the search box is focused remains a search submit, and the search box's widget and
message wiring are unchanged.
- README and `docs/invariants.md` describe Enter as an activation route, and `make check` passes.

## Done
