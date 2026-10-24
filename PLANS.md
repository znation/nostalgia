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

### Name the current track in the window title (planned 2026-10-23 by the plan role)

**Goal.** Classic Winamp names the playing track in its window title, but this app still passes
iced's static `"nostalgia"` to `.title`, so the desktop environment always shows the app name.
Build the title from the same `known_tracks` index the Now Playing LCD uses, so selecting a track
(Next/Previous and the preview-end advance included) names it in the title, Stop keeps the
interrupted track's name as the LCD does, and no known track restores the plain `nostalgia`.
This is presentation-only: it adds no message, handler, playback path, dependency, or window
geometry.

**Approach.**

- `src/ui/player.rs`: add a private `const APP_TITLE: &str = "nostalgia";` near the struct and
  `pub(in crate::ui) fn window_title(&self) -> String`. Lock the shared state, read `current_track`,
  and look the id up in `self.known_tracks`; no current track or an unknown id returns
  `APP_TITLE.to_string()`. For a known track, compose
  `"<number><artist - ><title> - <APP_TITLE>"` in the LCD's order: append `KnownTrack::number`,
  then `artist` plus `" - "` only when the artist is non-empty, then `title`, then `" - "` and
  `APP_TITLE`. Document that this is the iced window title, not the custom title bar's fixed
  `NOSTALGIA`, and that Stop keeps the interrupted track.
- `src/ui/mod.rs`: replace `.title("nostalgia")` in `init_ui` with
  `.title(|player: &WinampPlayer| player.window_title())`, and mention the track-following window
  title in the `init_ui` doc.
- `src/ui/tests.rs`: add `window_title_names_the_current_track` beside the other Now Playing
  resolution tests. Seed `player.known_tracks["song-1"]` with number `"1. "`, artist
  `"The Sample Band"`, and title `"Opening"` (the shape the marquee test already builds), set
  `state.current_track`, and assert `player.window_title()` is
  `"1. The Sample Band - Opening - nostalgia"`. Then clear the number
  (`"The Sample Band - Opening - nostalgia"`), clear the artist (`"1. Opening - nostalgia"`), set
  `current_track` to an unknown id (`"nostalgia"`), and set it to `None` (`"nostalgia"`). Finally
  restore the known track and call `AppState::stop()`, asserting the title still names it.
- `README.md`: in the Usage paragraph about the title bar, add one sentence that the window title
  (the name the desktop environment shows for the app) names the current track as
  `<number>Artist - Title - nostalgia`, and is just `nostalgia` when no known track is current.
  Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, add a bullet stating that the window title
  mirrors a known current track through `WinampPlayer::window_title`, using the same index and
  number prefix as the LCD, keeps the interrupted track across Stop, and falls back to
  `nostalgia` for no current track or an unknown id; name the new test in that section's
  `Pinned by` list.

**Files touched.** `src/ui/player.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- The application window title reads `1. The Sample Band - Opening - nostalgia` for that known
  current track, omits the number or artist when empty, stays on the interrupted track across
  Stop, and is the plain `nostalgia` with no current track or an unknown id.
- The title resolves `AppState::current_track` against `WinampPlayer::known_tracks`, so a track
  selected through the browse rows, Next/Previous, or the preview-end advance updates it; no new
  message, handler, dependency, or window geometry is added.
- README and `docs/invariants.md` describe the title, and `make check` passes.

## Done
