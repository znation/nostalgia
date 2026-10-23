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
> at the right edge of the LCD well, and `view` (`src/ui/view.rs`) stacks that
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

### Preview a seek drag's target time in the Now Playing readout (planned 2026-10-23 by the plan role)

**Goal.** While the user drags the seek slider beneath the Now Playing LCD, show the time the thumb points at in the large left-hand readout, so seeking a long preview is not blind. Today `WinampPlayer::now_playing_elapsed` in `src/ui/player.rs` reads only `playback_position`, while the slider thumb reads `seek_preview.or(playback_position)` inline in `src/ui/view.rs`; a drag therefore moves the thumb but leaves the readout on the backend's last position. Releasing already seeks to the drag and records that position, so the readout lands where the thumb pointed with no playback change. It touches only the Now Playing readout and the seek bar's shared position source.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) fn readout_position(&self) -> Option<Duration>` returning `self.seek_preview.or(self.playback_position)`. Document it as the one position the seek thumb and the large readout both show: an in-progress drag wins, otherwise the backend-reported position. Use it in `WinampPlayer::now_playing_elapsed` in place of `self.playback_position`, so elapsed mode previews the drag and remaining mode counts down from it through the existing `views::now_playing_elapsed` paths. Extend the `seek_preview` field's doc to say the held drag also previews in the readout.
- `src/ui/view.rs`: pass `player.readout_position()` to `views::view_seek_bar` instead of the inline `player.seek_preview.or(player.playback_position)`, so the thumb and readout cannot drift apart.
- Leave the right-edge total, marquee, paused blink, position/format/duration/spectrum tick batches, and the seek-on-release path unchanged. `reset_position_readout` still clears `seek_preview` on Stop and a new selection, so the readout returns to the cleared state then.
- `src/ui/tests.rs`: add `now_playing_elapsed_previews_the_seek_drag` beside `now_playing_elapsed_shows_the_position_while_it_is_known`. Build a player with `play_song_1_then_browse_to_album_2()` (song 1 is 3:30), set `playback_position = Some(Duration::from_secs(45))`, then send `Message::SeekPreviewChanged(12.0)`; assert `player.readout_position()` is `Some(Duration::from_secs(12))`, the elapsed readout is `"00:12"`, and `player.now_playing_time(...)` is still `"3:30"`. Set `player.time_display = views::TimeDisplay::Remaining` and assert `"-03:18"`. Set `player.seek_preview = None` and assert the readout falls back to `"00:45"` in elapsed mode (and `"-02:45"` in remaining mode), proving the drag is a preview, not a replacement.
- `README.md`: in the Usage sentence about the seek slider, replace "dragging moves only the thumb, and releasing seeks to that position" with wording that dragging moves the thumb and previews that position in the large elapsed readout (counting down from it in remaining-time mode), and releasing seeks to it. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the `## Now Playing label` seek-slider bullet, add that the large readout follows the in-progress drag in both time modes while the right-edge total does not, and that the thumb and readout share `WinampPlayer::readout_position`; add the new test to that bullet's Pinned-by list.

**Files touched.** `src/ui/player.rs`, `src/ui/view.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- While `seek_preview` holds a drag, the large readout shows that time in elapsed mode and counts down from it in remaining mode; the right-edge total is unchanged.
- The seek slider's thumb and the large readout both resolve their position through `WinampPlayer::readout_position`, so an in-progress drag wins over `playback_position` and clearing the drag falls back to the reported position.
- A release still sends exactly one `seek_output_to` with the dragged position and records it; Stop and a new selection still clear the drag and readout through `reset_position_readout`. No marquee, blink, tick, format, duration, spectrum, or playback behavior changes.
- `README.md` and `docs/invariants.md` describe the preview, `make check` passes, and the new test covers the drag, the remaining-mode countdown, and the fallback.

## Done
