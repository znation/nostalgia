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
> Recommend the plan loop keep scheduling the open equalizer and seek plans.

## Planned

### Add the playback-position readout to the Now Playing bar (Playback position, part 2/2: planned 2026-10-21 by the plan role, requires part 1/2 landed)

**Goal.** The Now Playing bar's right-hand readout shows the current track's elapsed and total time as `"<elapsed>/<total>"` (for example `0:45/3:12`) whenever the audio backend has a loaded preview, and falls back to the existing plain total (`"--:--"` when no track is current) when it does not. Pause freezes the elapsed value, a seek steps it, Stop and a new selection clear it, and a play whose preview fails to install — including the asynchronous download/decode failure `handle_play` reports on the audio worker — leaves the plain total rather than a frozen `0:00`.

This revision replaces the rejected `position_tracking`/`reset_position` state machine. Rather than infer from shared `is_playing` whether audio is loaded, it makes the audio backend authoritative: `AudioOutput::position` answers `None` when no preview is loaded, so the UI keeps no playback guard and a selected track with no preview URL never turns one on.

**Approach.**

- `src/audio.rs`: revise the `AudioOutput::position` contract and doc: `Some(position)` while a player is loaded (including `Some(Duration::ZERO)` for a preview at its start), `None` when no player is loaded or the backend cannot answer. The default stays `None`. Part 1's `Some(Duration::ZERO)` for "no player" made a stopped worker indistinguishable from a loaded preview at zero — the ambiguity the rejected UI guard tried to patch — so part 2 removes it at the seam.
- `src/audio/player.rs`: change `Command::Position` to carry `Sender<Option<Duration>>` and update its doc (`None` when none is loaded). In `serve_commands`, send `player.as_ref().map(WorkerPlayer::position)` so no player answers `None`. In `impl AudioOutput for RodioOutput::position`, return `replies.recv_timeout(POSITION_QUERY_TIMEOUT).ok().flatten()`, so a closed worker or a timed-out reply is `None`. `handle_play`'s failure path and `Command::Stop` already drop the player, so a failed asynchronous start and a Stop both make later queries answer `None` without touching shared state.
- `src/audio/tests.rs`: rename `position_query_returns_zero_when_nothing_is_loaded` to `position_query_reports_none_when_nothing_is_loaded` and assert `None`; change `position_query_returns_the_loaded_players_position` to expect `Some(Duration::from_secs(42))`; change the helper thread in `rodio_output_asks_the_worker_for_its_position` to reply `Some(Duration::from_secs(3))`. The timeout, closed-worker, and silent-output tests already expect `None` and stay unchanged.
- `src/apple_music.rs`: update `playback_position`'s doc to say it reports the loaded preview's position and `None` when no preview is loaded or the backend cannot answer, and remove its `#[allow(dead_code)]` now that the UI calls it.
- `src/ui/views/now_playing.rs`: add `pub fn format_position_time(position_ms: u64) -> String`, formatting zero-padded `m:ss` through the existing `format_minutes_seconds` but returning `"0:00"` for zero (elapsed zero is valid, unlike an unknown length). Add `pub fn now_playing_progress(position_ms: u64, total: &str) -> String` returning `format!("{}/{}", format_position_time(position_ms), total)`. Change `now_playing_time` to take `position: Option<Duration>`: when `current_track` is in `tracks` and `position` is `Some`, return `Cow::Owned(now_playing_progress(...))` with the duration converted saturating to milliseconds; otherwise keep today's borrowed total or `"--:--"` fallback. Update its doc comment.
- `src/ui/mod.rs`: add `playback_position: Option<Duration>` and `position_epoch: u64` to `WinampPlayer`, initialized `None`/`0` in `new`, and define `POSITION_POLL_INTERVAL` (`Duration::from_secs(1)`) with a doc note that it bounds the readout refresh, not the audio. Add `Message::PositionTick` and `Message::PlaybackPosition { epoch: u64, position: Option<Duration> }`. In `init_ui`, batch `iced::time::every(POSITION_POLL_INTERVAL).map(|_| Message::PositionTick)` with the existing `keyboard::listen().filter_map(message_for)`. The `PositionTick` arm captures `player.position_epoch`, clones the service, and returns `Task::perform(async move { service.playback_position() }, move |position| Message::PlaybackPosition { epoch, position })`; the `PlaybackPosition` arm stores `position` (including `None`) only when `epoch == player.position_epoch`, dropping a reply issued before a transition. Add `fn reset_position_readout(player: &mut WinampPlayer)` that bumps `position_epoch` and sets `playback_position = None`; call it after the song resolves in `TrackSelected` and at the start of the `Stop` arm (which becomes a block returning the existing `transport_into`). Pause leaves the field alone. `WinampPlayer::now_playing_time` passes `self.playback_position` to `views::now_playing_time`; the `view` call site is unchanged, but its comment that all three readouts always borrow must note the combined elapsed case allocates.
- `src/ui/views/tests.rs`: update the `now_playing_time_resolves_known_track_to_cached_label` call sites to pass `None`; extend it (or add `now_playing_time_shows_elapsed_over_total_when_a_position_is_known`) so `now_playing_time(&tracks, Some("song-1"), Some(Duration::from_secs(192)))` reads `"3:12/3:30"` and is `Cow::Owned`; add `format_position_time_formats_elapsed_and_zero` (`0`→`"0:00"`, `192_000`→`"3:12"`, `65_000`→`"1:05"`) and `now_playing_progress_joins_elapsed_and_total` (`(192_000, "12:38")`→`"3:12/12:38"`).
- `src/ui/tests.rs`: update `now_playing_time_shows_the_played_tracks_duration_after_browsing_to_another_album` to pass `None`; add `now_playing_time_shows_elapsed_over_total_while_a_position_is_known` (reuse `play_song_1_then_browse_to_album_2`, set `player.playback_position = Some(Duration::from_secs(45))`, assert `"0:45/3:30"`), `track_selected_clears_the_elapsed_readout`, `stop_clears_the_elapsed_readout`, `position_replies_store_and_stale_ones_are_dropped` (store a reply at the current epoch, then drive a reply stamped with the pre-`Stop` epoch and assert it does not overwrite), and `position_tick_schedules_a_query` (`assert_message_schedules_work`).
- `README.md`: extend the Usage sentence that describes the Now Playing bar's artist and title so it says the length reads as elapsed/total while a preview is loaded and as the plain length otherwise.
- `docs/invariants.md`: in the Now Playing label section, add the time-readout guarantee (elapsed/total only while a loaded preview position is known, the cached total otherwise, `--:--` with no current track), its enforcement by `views::now_playing_time`/`now_playing_progress` and the `PlaybackPosition` arm, and cite the new tests plus the audio-seam `position_query_reports_none_when_nothing_is_loaded`.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/apple_music.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes.
- A loaded player makes `Command::Position` answer `Some(player.position())`; no player answers `None`; a closed worker or a timed-out reply is `None`; the silent backend stays `None`. A failed asynchronous start drops the player in `handle_play`, so the next query answers `None`; Stop does the same through `Command::Stop`.
- `format_position_time` returns `"0:00"`, `"3:12"`, and `"1:05"` for zero, 192000 ms, and 65000 ms; `now_playing_progress(192_000, "12:38")` returns `"3:12/12:38"`.
- With a known current track and a `Some` position, the bar renders `elapsed/total`; with `None`, the plain total; with no current track or an unknown id, `"--:--"`.
- Pause keeps the frozen position because the player stays installed and the poll reads it; a left/right seek is reflected by the next poll; Stop and a new selection clear the readout immediately through `reset_position_readout`.
- A `PlaybackPosition` reply stamped with an older `position_epoch` is dropped, so a poll in flight across a Stop or selection cannot restore the previous elapsed value.
- The poll runs once per `POSITION_POLL_INTERVAL` in an iced task, not in `update`.
- The change touches only the ten listed files plus this `PLANS.md` edit.

### Add the library window's Top button and open every browse list at the top (planned 2026-10-21 by the plan role)

**Goal.** The browse list gains the mockup's `TOP` control: a `Top` button above the list returns the current list to its first row, and opening a browse level — drill-down, flat-list button, search, or Back — starts that newly shown list at the top instead of leaving it at whatever offset the previous list had. This is independent of the playback-position series: it touches no audio code and no Now Playing state.

**Approach.**

- `src/ui/views/browse.rs`: define `pub const BROWSE_LIST_ID: &str = "nostalgia-browse-list";` with a doc comment naming it as the one scrollable's id so the update loop's scroll operation can reach it. In `scrollable_list`, add `.id(BROWSE_LIST_ID)` to the `Scrollable` before its style, leaving the rows, placeholder, and height unchanged. Add `pub fn view_top_button() -> Element<'static, Message>` returning `labeled_button("Top", Message::ScrollBrowseToTop).into()`.
- `src/ui/views/mod.rs`: re-export `BROWSE_LIST_ID` and `view_top_button` from `browse` beside `view_back_button` and `view_retry_button`.
- `src/ui/mod.rs`: add `Message::ScrollBrowseToTop`. Add a private helper `fn scroll_browse_to_top() -> Task<Message>` returning `iced::widget::operation::snap_to(views::BROWSE_LIST_ID, iced::widget::scrollable::RelativeOffset::START)`. Handle the new message by returning that helper. Add a private helper `fn fetch_at_top(task: Task<Message>) -> Task<Message>` returning `Task::batch([task, scroll_browse_to_top()])`, and wrap the `fetch_level(...)` result in the `ArtistSelected`, `AlbumSelected`, `PlaylistSelected`, `ShowAllSongs`, `ShowAllAlbums`, `ShowPlaylists`, and `SearchSubmitted` arms. In `Message::Back`, return `scroll_browse_to_top()` after the existing view change so stepping up also starts the restored list at the top.
- `src/ui/mod.rs` `view`: push `views::view_top_button()` above the browse content for every `CurrentView` — after the Back/Retry buttons and before the Artists-only flat-list buttons and the search box — so the mockup's `TOP` control is available at each browse level.
- `src/ui/views/tests.rs`: extend `now_playing_bar_and_browse_action_buttons_construct` with `let _top = view_top_button();`.
- `src/ui/tests.rs`: add `scroll_browse_to_top_schedules_a_scroll_task`, driving `Message::ScrollBrowseToTop` through `assert_message_schedules_work` and asserting `assert_view` and the shared state are unchanged; add `back_schedules_the_restored_list_to_the_top`, starting in Albums and asserting `Message::Back` both returns to Artists and schedules follow-up work (the scroll task).
- `README.md`: extend the Usage paragraph that describes the browse buttons with one sentence: a `Top` button above every browse list returns it to the first row, and opening a list starts it at the top.

**Files touched.** `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `Message::ScrollBrowseToTop` schedules an iced `snap_to` task for `views::BROWSE_LIST_ID` at `RelativeOffset::START`, and driving it through `update` changes neither `current_view` nor shared state.
- The `Top` button is built into the browse area for all six `CurrentView` variants and is absent in shade mode, which returns before the browse content is assembled.
- Every navigation arm that replaces the visible list batches the snap-to-top task with its fetch, and `Message::Back` schedules it after stepping up, so a list opened after a navigation starts at its first row.
- Scrolling a list manually still works between navigations; the id is stable while the same list is displayed.
- The change touches only the six listed files plus this `PLANS.md` edit; no audio, `apple_music`, or `docs/invariants.md` changes, because the browse hierarchy guarantees are unchanged.

## Done
