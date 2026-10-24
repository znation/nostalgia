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

_None yet._

### Drive the transport with the system media keys (planned 2026-10-24 by the plan role)

**Goal.** A focused Nostalgia window answers the classic Winamp letters and arrows, but the
system media keys a keyboard or headset sends are unbound. Bind the transport media keys to the
same `Message`s the on-screen controls use: `MediaPlayPause` to `Message::PlayPause` (the button's
toggle), `MediaPlay` to `Message::Play`, `MediaPause` to `Message::Pause`, `MediaStop` to
`Message::Stop`, `MediaTrackNext` to `Message::NextTrack`, and `MediaTrackPrevious` to
`Message::PreviousTrack`. The keys therefore behave exactly like pressing Play/Pause, Stop,
Next, or Previous, including their existing Shuffle, Repeat, and end-of-list rules; they change
no browse, search, or LCD state beyond what those controls already change.

**Approach.**

- `src/ui/shortcuts.rs`: add the six `Key::Named(key::Named::Media…)` arms to `shortcut` beside
  the existing named-key arms, returning the messages above. Update the `message_for` and
  `shortcut` doc comments to list the media-key transport route along with the classic letters.
  The existing chord guard stays first, so Ctrl/Alt/Logo chords with a media key remain unbound,
  and Shift-held media keys work like the plain keys.
- `src/ui/message.rs`: extend the comments on `Play` and `Pause` to say the explicit media keys
  share the X and C routes, rather than naming only the keyboard letters.
- Tests. `src/ui/shortcuts.rs`: add `message_for_maps_media_keys_to_the_transport`, mapping each
  of the six named keys with `Modifiers::NONE` and `Modifiers::SHIFT` through `message_for` and
  asserting the matching `Message` with `matches!`. Add the six keys to the `shortcut_ignores_chord_modifiers`
  key list so Ctrl/Alt/Logo + media key stay unbound. `src/ui/tests.rs`: add
  `media_play_pause_key_toggles_playback`, modeled on `play_pause_toggles_is_playing`: start from
  `player_playing_preview` with a `RecordingAudio`, map a `Key::Named(key::Named::MediaPlayPause)`
  press through `super::shortcuts::message_for`, assert it is `Message::PlayPause`, drive it
  through `update` with `assert_transport_settled`, and assert the shared `is_playing` flag flips
  each time. Leave the existing `Message::Play`, `Pause`, `Stop`, `NextTrack`, and `PreviousTrack`
  handler tests to pin the behavior each media key reaches.
- `README.md`: in the Usage keyboard sentence, after the F5 clause, say the system media keys
  Play/Pause, Stop, Next track, and Previous track drive the same transport while the window is
  focused. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a short `## Media keys` section after `## Browse keyboard scrolling`
  with the guarantee, `shortcuts::shortcut` as **Enforced by**, the two new test names as
  **Pinned by**, and a **History** line for this addition.

**Files touched.** `src/ui/shortcuts.rs`, `src/ui/message.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- Plain or Shift-held `MediaPlayPause`, `MediaPlay`, `MediaPause`, `MediaStop`, `MediaTrackNext`,
  and `MediaTrackPrevious` map to `PlayPause`, `Play`, `Pause`, `Stop`, `NextTrack`, and
  `PreviousTrack` respectively; Ctrl/Alt/Logo chords with those keys map to nothing.
- Driving the mapped media Play/Pause message through `update` toggles playback exactly like the
  on-screen Play/Pause button, and the remaining media keys reach the same already-tested
  transport handlers as their controls and classic letters.
- The existing classic letter, arrow, and named-key bindings are unchanged.
- The new tests pass, `make check` passes, and README and `docs/invariants.md` describe the
  media-key transport route.

## Done
