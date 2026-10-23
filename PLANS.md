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

### Handle the end of a preview (Preview end, part 2/2: planned 2026-10-23 by the plan role, requires part 1/2 landed)

**Goal.** Make the end of a preview follow the transport's own rules instead of leaving the UI claiming playback against a silent sink. When a loaded preview finishes while playing, behave exactly like Next: step forward through the showing list with Shuffle choosing a non-current song and Repeat wrapping at the end. With Repeat off on the last song, take the existing Stop path so playback becomes stopped, the current song stays queued for Play, and the elapsed readout, status row, visualizer, and seek bar clear together. Pause and Stop never advance. Reuse the one-second position poll; add no timer or subscription.

**Approach.**

- `src/ui/message.rs`: add `PlaybackFinished { epoch: u64, finished: bool }`, documented as the position poll's completion reply stamped with the epoch that issued the query.
- `src/ui/handlers/playback.rs`: in `position_tick`, schedule a fourth task from `service.preview_finished()` with the same epoch, mapping to `Message::PlaybackFinished`; update the tick's doc to name the finished query. Leave the early return while `pending_position_transitions > 0` in place, so a transition in flight suppresses the query too.
- In the same module, extract the forward half of `step_track`'s selection into a private pure helper, for example `fn forward_track_id(items: &[Song], current: Option<&str>, shuffle: bool, repeat: bool) -> Option<String>`: it returns `transport::shuffled_track_id(items, current, transport::shuffle_roll())` when `shuffle` is on and `transport::next_track_id(items, current, repeat)` otherwise. `step_track` calls it under its existing state lock for `forward` (using the same list the current view draws from: `all_songs` for `CurrentView::AllSongs`, `songs` otherwise) and keeps its existing backward branch and `TrackSelected` index resolution, so the Next button and keyboard behavior do not change. `playback_finished` reads the same list and flags under one lock, calls the helper, and resolves the returned id to its index before releasing the lock; taking arguments rather than a player keeps it from trying to lock recursively.
- Add `pub(in crate::ui) fn playback_finished(player: &mut WinampPlayer, epoch: u64, finished: bool) -> Task<Message>`. It returns `Task::none()` unless `finished` is true, no transition is pending, the epoch is current, and `state.is_playing` is true. Otherwise it calls `forward_track_id`; on `Some(id)` it resolves the id to its position in that same list and schedules `Message::TrackSelected { epoch: list_epoch, index }`; on `None`, or when the id no longer appears in the list, it calls [`stop`] and returns its task, so the Stop path clears the readout and keeps `current_track`. Do not record the ended track in `PlayHistory`; a natural end is a forward step, not a Previous.
- `src/ui/mod.rs`: route `Message::PlaybackFinished` to `playback_finished`.
- `src/ui/timers.rs`: update the position-poll module docs that enumerate the queries (position, format, length) to include the finished state; the interval stays `POSITION_POLL_INTERVAL`.
- `src/ui/tests.rs`: add `position_tick_queries_preview_finished` (the reply carries the tick's epoch and `RecordingAudio::set_finished`'s value), `preview_end_advances_to_the_next_song` (seed `stepping_songs` with `song-1` current, drive the finished reply and then its task, assert the next song's play commits), `preview_end_wraps_to_the_first_song_with_repeat_on`, `preview_end_at_the_last_song_stops_when_repeat_is_off` (assert `is_playing` false, `current_track` still the last song, the `AudioCall::Stop` recorded, and the position/format/duration/spectrum cleared), `preview_end_with_shuffle_on_picks_the_other_song` (a two-song list makes the pick deterministic), `preview_end_while_paused_or_stopped_does_not_advance`, and `preview_end_with_a_stale_epoch_or_pending_transition_is_dropped`. Drive `playback_finished` through `update` and settle the scheduled play or stop task with the existing helpers.
- `README.md`: in the Usage paragraph describing preview playback, add one sentence: when a preview reaches its end, the player advances to the next song of the showing list (Shuffle picks another, Repeat wraps), and with Repeat off on the last song it stops and keeps that song queued for Play.
- `docs/invariants.md`: extend `## Preview end` with the UI behavior: the position poll reports the finished state, a finished preview while playing advances under the same list/Shuffle/Repeat rules as Next, Repeat off on the last song takes the Stop path, and Pause, Stop, a stale epoch, or a pending transition never advance. Name `handlers::playback::playback_finished`, `handlers::playback::forward_track_id`/`step_track`, and `Message::PlaybackFinished` as the enforcers and list the new tests.

**Files touched.** `src/ui/message.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/timers.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A finished preview while playing advances to the song Next would select from the showing list: sequential when Shuffle and Repeat are off, the first song when Repeat is on and the last is current, and a non-current song when Shuffle is on.
- A finished preview on the last song with Repeat off takes the Stop path: `is_playing` becomes false, `current_track` still names that song, and the elapsed readout, status row, visualizer bars, and seek bar are cleared.
- A finished reply is ignored while paused or stopped, when `pending_position_transitions > 0`, or when its epoch is stale, so a reply in flight across a transport change cannot advance.
- The finished state is queried on the existing one-second position poll; no new timer, subscription, or dependency is added, and the Next/Previous buttons and keys keep their current behavior.
- `README.md` and `docs/invariants.md` describe the behavior, and `make check` passes.
### Clear the stopped Now Playing LCD readout (planned 2026-10-23 by the plan role)

**Goal.** Match `docs/screenshots/transport.png`'s stopped state: "Time, bitrate and spectrum clear and the seek thumb hides; the current song stays queued for Play." The status row, spectrum, and seek strip already clear through `reset_position_readout`, but the large elapsed readout still draws `00:00` in green because `elapsed_readout_color` returns `theme::LCD_GREEN` for `TransportMode::Stopped`. Draw the stopped readout transparent, keeping its text and bounds so the title and total do not move, and make it non-interactive while stopped so a click cannot silently switch the persisted time mode. This is independent of the queued Preview end plans: it reads the `TransportMode` that already exists and changes only the Now Playing view.

**Approach.**

- `src/ui/views/now_playing.rs`: extend `elapsed_readout_color` so `TransportMode::Stopped` is fully transparent in both blink phases as well as in `TransportMode::Paused` with `visible == false`; `TransportMode::Playing`, and `TransportMode::Paused` with `visible == true`, stay `theme::LCD_GREEN`. Update its doc comment to say the stopped readout is cleared rather than shown, and that drawing transparent keeps its bounds so the title, total, and status row stay in place.
- In the same module, add `pub(super) fn elapsed_readout_clickable(mode: TransportMode) -> bool`, returning false only for `TransportMode::Stopped` and true for `Playing`/`Paused`. In `now_playing_row`, build the elapsed `MouseArea` from the existing `lcd_elapsed_text(elapsed).color(elapsed_readout_color(...))` but attach `.on_press(Message::ToggleTimeDisplay).interaction(Interaction::Pointer)` only when `elapsed_readout_clickable(mode)` is true; otherwise use `.interaction(Interaction::None)` with no message. Keep the widget in the row so its layout slot and the text's bounds are unchanged.
- `src/ui/views/tests.rs`: extend and rename `elapsed_readout_color_blinks_only_while_paused` to `elapsed_readout_color_blinks_when_paused_and_clears_when_stopped`, asserting transparent for `(Paused, false)` and both `(Stopped, false)` and `(Stopped, true)`, and `theme::LCD_GREEN` for `(Playing, false)`, `(Playing, true)`, and `(Paused, true)`. Add `elapsed_readout_is_clickable_only_while_not_stopped` asserting true for Playing/Paused and false for Stopped. Update the comment on `now_playing_row_keeps_the_elapsed_readout_in_place_across_transport_modes` to say the stopped case still lays out the transparent text; its existing equal-x assertion then pins that stopping does not move the readout.
- `src/ui/tests.rs`: correct the comments and assertion messages in `position_tick_blinks_the_elapsed_readout_only_while_paused` that currently claim a stopped tick "shows the readout": the tick still holds `blink_visible` true while stopped, but the view hides the stopped readout regardless of that flag. Do not change the assertions.
- `README.md`: in the Usage paragraph, say the large readout is shown while playing or paused and blank (though it keeps its space) while stopped, and change "it stays steady while playing or stopped" to "it stays steady while playing and clears while stopped". Also scope the click sentence to while the readout is shown. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, update the time-readout bullet to state that while the transport is stopped the large readout draws transparent in both time modes and both blink phases, keeping its bounds and clickability off, so stopped shows no elapsed or remaining time; the stored value is still the `00:00` Stop clears it to, and Play/Pause draw it again. Update the transport-glyph bullet's "playing and stopped always show the readout" sentence to playing only, name `views::now_playing::elapsed_readout_color` and `elapsed_readout_clickable` as the enforcers, and add the new test names. Update the position-poll bullet's reason for leaving a stopped player out of the subscription to say the view clears its readout while stopped rather than that it already shows `00:00`.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `elapsed_readout_color` returns the fully transparent color for `TransportMode::Stopped` in both blink phases, and for `TransportMode::Paused` only when `visible` is false; playing and visible paused stay `theme::LCD_GREEN`.
- While stopped, `now_playing_row` still draws the elapsed text at the same bounds as in the playing and paused states, so the title, total, and status row do not move; the captured stopped row keeps the same elapsed fragment as the other modes.
- `elapsed_readout_clickable` is false only for `Stopped`, and the stopped readout attaches no `ToggleTimeDisplay` message, so clicking it cannot change the persisted `TimeDisplay`; playing and paused readouts still toggle it.
- Stop, a new selection, Pause, the position poll, the marquee, the status row, the spectrum, and the seek strip keep their current behavior; no state field, timer, subscription, or dependency is added, and the shade strip is deliberately unchanged.
- `README.md` and `docs/invariants.md` describe the stopped readout, and `make check` passes.

## Done
