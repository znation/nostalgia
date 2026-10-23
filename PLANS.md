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
> (both 2026-10-20); the 2026-10-22 note below carries the current queue.

> **Steward drift note (2026-10-22 by the steward role): the built Now
> Playing bar and browse rows do not match the brief's mockups, and the two
> queued plans settle that difference in opposite directions — the
> playback-position plan adds a readout the mockup shows nowhere, while the
> album-row plan matches the mockup's `<artist> - <title>` rows but leaves the
> running time it draws out of scope.**
>
> The brief promises a "very close if not pixel-perfect" Winamp, and the plan
> role has been citing `docs/screenshots/` as the spec — the landed plans for
> the scrolling title, the artist-prefixed rows, the `TOP` button and the list
> counts each name a mockup feature. The main window's mockup
> (`docs/screenshots/player.png`) draws the classic LCD: a vertical `O C I D V`
> strip at the left, a large elapsed-time readout (`00:53`) over a green
> visualizer, a song-title box reading `<ARTIST> - <TITLE> (<length>)`, and a
> `256 KBPS 44 KHZ` / `MONO STEREO` status row. The build renders one row
> instead: `now_playing_row` (`src/ui/views/now_playing.rs`) pushes a
> `Now Playing: ` caption, the artist and title, a fill, and the track length
> at the right edge of the LCD well, and `view` (`src/ui/mod.rs`) stacks that
> bar above the transport row — no elapsed-time readout, no `O C I D V` strip,
> no status row.
>
> The first entry under `## Planned`, "Add the playback-position readout to
> the Now Playing bar (Playback position, part 3/3)", widens that right-edge
> slot to `<elapsed>/<total>`, a value the mockup shows nowhere: its title
> line carries the length in parentheses and the elapsed time in the big left
> readout. Recommend the plan role schedule that elapsed-time display and
> settle where part 3/3's readout belongs before the queue drains.
>
> The browse rows drift the same way: `docs/screenshots/library.png` draws
> each artist row with its album count at the right (`2 albums`) and each
> album row with its running time (`12:38`), while the build shows the
> `View Albums` and `View Songs` hints (`src/ui/views/browse.rs`) and the
> total count beneath the rows. The queued album-row plan, "Show each album
> row's artist beside its title", puts album running times out of scope
> because an album query does not supply its songs' durations, so the
> mockup's per-album time would need a songs fetch per row — a decision for
> the plan role, not a detail to drop.

## Planned

_None yet._

### Toggle the Now Playing elapsed readout between elapsed and remaining time (planned 2026-10-23 by the plan role)

**Goal.** Make the large left-hand LCD readout behave like Winamp's: clicking it switches from the elapsed time (`00:53`) to the time remaining (`-02:37`), and clicking again returns to elapsed. The mode is presentation-only, survives Pause, Stop, Next/Previous, and a new selection until clicked again, and changes nothing about playback, the right-edge total, or the marquee. This is independent of the queued Sort plan: it touches the Now Playing machinery, not the browse lists.

**Approach.**

- `src/ui/views/now_playing.rs`:
  - Add `pub enum TimeDisplay { Elapsed, Remaining }` (derive `Debug, Clone, Copy, PartialEq, Eq, Default`, with `Elapsed` as `#[default]`) and a `pub fn toggled(self) -> Self` beside it (public so `handlers::playback` can flip it; `pub(super)` would be confined to `views`). Re-export `TimeDisplay` from `src/ui/views/mod.rs` like `KnownTrack`.
  - Add `duration_ms: u64` to `KnownTrack` and document it as the raw length the remaining readout subtracts from (the existing `time` field stays the formatted total for the right edge). `src/ui/handlers/playback.rs`'s `track_selected` records `duration_ms: song.duration_ms` in the same entry it already builds.
  - Add `pub fn format_lcd_remaining(remaining_ms: u64) -> String`, formatting the shared core of `format_lcd_elapsed` with a leading `-`: `0` → `"-00:00"`, `157_000` → `"-02:37"`, `3_600_000` → `"-60:00"`.
  - Change `now_playing_elapsed` to take the display mode as its last argument. Unknown/absent track still returns borrowed `"--:--"`. In `TimeDisplay::Elapsed` keep today's behavior exactly (`00:00` with no reported position). In `TimeDisplay::Remaining`, return borrowed `"--:--"` when the known track's `duration_ms` is `0` (no length to count down from), otherwise `Cow::Owned(format_lcd_remaining(duration_ms.saturating_sub(position_ms)))`, treating a missing position as zero (so a stopped track shows its full length remaining). Position milliseconds come from the existing `duration_millis` helper. Document the mode's persistence and the saturating subtraction.
  - Wrap the elapsed fragment in `now_playing_row` in `iced::widget::MouseArea::new(lcd_elapsed_text(elapsed)).on_press(Message::ToggleTimeDisplay).interaction(iced::mouse::Interaction::Pointer)`, keeping it the row's first child so the existing captured-text tests still see it first. The helper's existing `text::Renderer` and theme bounds already satisfy `MouseArea`, and `view_now_playing` keeps its signature.
- `src/ui/mod.rs`:
  - Add `Message::ToggleTimeDisplay` next to `PlaybackPosition` with a doc comment saying it only flips the LCD's time mode, and dispatch it to `handlers::playback::toggle_time_display` beside the position arms (not among the browse arms, so the Sort plan's message edits stay separate).
  - Add `time_display: TimeDisplay` to `WinampPlayer`, initialized `TimeDisplay::Elapsed` in `new` (or via `Default`), documented as presentation-only and preserved across tracks. Pass it to `views::now_playing_elapsed` in `WinampPlayer::now_playing_elapsed`; the accessor keeps its one-argument signature.
- `src/ui/handlers/playback.rs`: add `pub(in crate::ui) fn toggle_time_display(player: &mut WinampPlayer) -> Task<Message>` that assigns `player.time_display = player.time_display.toggled()` and returns `Task::none()`. It touches no playback state and clears no position.
- Tests. `src/ui/views/tests.rs`: give the `known_tracks` helper `duration_ms: 0`; update its existing `now_playing_elapsed` calls with `TimeDisplay::Elapsed`; add `format_lcd_remaining_prefixes_a_minus_and_pads` and `now_playing_elapsed_remaining_counts_down_from_the_total` (known 210_000 ms track with a 53 s position → `-02:37`, no position → `-03:30`, `duration_ms` 0 → `--:--`, unknown id → `--:--`, a position past the total → `-00:00`). `src/ui/tests.rs`: add `duration_ms` to the two hand-built `KnownTrack` literals, then add `toggle_time_display_switches_to_remaining_and_persists` beside the elapsed tests: play song 1 (210 s), set `playback_position` to 53 s, send `Message::ToggleTimeDisplay`, assert `-02:37` and an unchanged right-edge total; send `Message::Pause` and assert the remaining value survives; select song 2 and assert the new track counts down from its own total (`-02:05`); toggle back and assert elapsed mode again (`00:00` after the selection). Add `toggle_time_display_with_no_track_keeps_the_placeholder` (a fresh player toggles and still reads `--:--`).
- `README.md`: in the Usage paragraph describing the Now Playing bar, say clicking the large elapsed readout switches it to the track's remaining time (`-02:37`) and back, and that the choice stays until clicked again. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: extend the `## Now Playing label` split-readout bullet with the toggle: the left readout shows elapsed by default, clicking it shows `-MM:SS` remaining (saturating at `-00:00`, `--:--` when the track's length is unknown), the right edge keeps the plain total in both modes, and the mode persists across Pause, Stop, Next/Previous, and a new selection. Name `TimeDisplay`, `views::now_playing_elapsed`, and `format_lcd_remaining` as the enforcers, and add the new tests to the Pinned-by list.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/mod.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The large left readout still shows elapsed `MM:SS` by default, unchanged for every existing position case.
- Pressing the readout flips it to `-MM:SS` remaining and pressing it again flips back; no other control, playback call, track selection, marquee offset, or right-edge total changes.
- Remaining is `total - position`, saturating at `-00:00`; a known track with `duration_ms == 0` and any unknown/absent track read `--:--`.
- The chosen mode survives Pause, Stop, Next/Previous, and selecting another track, and applies to that track's own length.
- The README and `docs/invariants.md` describe the toggle in the same change, and `make check` passes (fmt, clippy, rustdoc, tests).

## Done
