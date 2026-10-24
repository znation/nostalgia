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

### Show the compact transport readout as elapsed over the track's total (planned 2026-10-23 by the plan role)

**Goal.** Match `docs/screenshots/library.png` and `docs/screenshots/hero.png`, whose compact
footer readout reads `4:05/12:33`: show the current time over the playing track's total in the
shared compact row used by the library footer and the window-shade strip. The left value stays
the same `WinampPlayer::now_playing_elapsed` string (so the elapsed/remaining toggle still applies,
and remaining mode reads `-02:37/3:30`), the right value is the same
`WinampPlayer::now_playing_time` total the LCD's right edge shows, and when that total is the
`--:--` placeholder the readout stays that single string rather than `--:--/--:--`. This changes
only the compact row's text: the LCD, playback, messages, glyphs, volume slider, heights, and
layout are unchanged.

**Approach.**

- `src/ui/views/mini_transport.rs`: add `pub(super) fn mini_transport_readout<'a>(elapsed:
  Cow<'a, str>, total: Cow<'a, str>) -> Cow<'a, str>`. It returns `elapsed` unchanged when
  `total` is `"--:--"` (so a missing or unknown track keeps its current placeholder) and
  otherwise returns `Cow::Owned(format!("{elapsed}/{total}"))`. Give `mini_transport_row` a
  `total: Cow<'a, str>` parameter beside `elapsed`, build the drawn `Text` from this helper, and
  give `view_library_strip` the same parameter. Update the module and function docs to name both
  time strings and the placeholder rule.
- `src/ui/views/shade.rs`: give `view_shade_strip` a `total: Cow<'a, str>` parameter beside
  `elapsed` and pass it to `mini_transport_row`; update its docs to say the readout is
  elapsed-over-total and keeps the shared toggle.
- `src/ui/view.rs`: in the shaded branch, resolve `now_playing_time` from the same state lock
  tuple (using `state.current_track.as_deref()`), and pass it to `view_shade_strip`. In the normal
  branch, clone the already-resolved `now_playing_time` before it moves into `view_now_playing`
  (the way `library_elapsed` is cloned) and pass the clone to `view_library_strip`, so the footer
  and the LCD's right edge read the same total.
- `src/ui/views/tests.rs`: add `mini_transport_readout_appends_the_known_total` covering
  `00:53`/`3:30` → `00:53/3:30`, `-02:37`/`3:30` → `-02:37/3:30`, `00:53`/`--:--` → `00:53`, and
  `--:--`/`--:--` → `--:--`. Update `captured_shade_strip`, `captured_library_strip`, and the
  existing height and geometry tests to pass a total (for example `Cow::Borrowed("3:30")`) and
  assert the combined readout reaches the laid-out row. Keep the glyph order, fixed height, and
  right-edge volume assertions unchanged.
- `src/ui/tests.rs`: add `root_view_footer_shows_elapsed_over_total`, using
  `play_song_1_then_browse_to_album_2` and `captured_root_view_text_bounds` with `shaded = false`
  and then `true`, asserting both trees draw `00:00/3:30` (the known song's cached total) and
  that neither draws `--:--/--:--`. This pins the two `view` call sites, not just the builders.
- `README.md`: in the Usage prose, extend the library-footer and shade-strip sentences to say the
  compact readout shows the elapsed (or remaining) time over the track's total, and falls back to
  the plain `--:--` placeholder when no total is known. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Library footer` and `## Window shade`, replace the bullet that
  calls the compact label "the current time readout" / the same elapsed string with the
  elapsed-over-total rule and its single-placeholder fallback. Name
  `views::mini_transport::mini_transport_readout` as the enforcer, note that both strips take the
  same `now_playing_elapsed` and `now_playing_time` strings the LCD resolves, and add the new
  tests to both `Pinned by` lists.

**Files touched.** `src/ui/views/mini_transport.rs`, `src/ui/views/shade.rs`, `src/ui/view.rs`,
`src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Both the library footer and the shade strip draw `<elapsed>/<total>`, such as `00:53/3:30`, when
  the current track's total is known; remaining mode still prefixes the elapsed value with `-`
  (`-02:37/3:30`), and a known total with no reported position reads `00:00/3:30`.
- When `now_playing_time` returns `--:--` (no current track, or an unknown one), the readout is
  just the elapsed string, so the empty state still reads `--:--` and never `--:--/--:--`.
- The compact row's four glyphs, messages, heights, right-edge `MINI_VOLUME_ID` slider, and
  elapsed/remaining toggle behavior are unchanged, and the LCD's large left readout and
  right-edge total are unchanged.
- The root `view` passes the same total string to the strip that it passes to the LCD, in both
  normal and shaded modes; README and `docs/invariants.md` describe the combined readout, and
  `make check` passes.

## Done
