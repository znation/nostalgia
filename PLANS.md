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

**Goal.** The Now Playing bar's right-hand readout shows the current track's elapsed and total time as `"<elapsed>/<total>"` (for example `0:45/3:12`) while a preview is loaded, and keeps the existing `"--:--"` when no track is current. The elapsed value is the position part 1 reads from the audio backend, so pause freezes it, a seek steps it, and Stop or a new track resets it.

**Approach.**

- `src/ui/views/now_playing.rs`: add `pub fn format_position_time(position_ms: u64) -> String`, formatting zero-padded `m:ss` like `format_track_time` but returning `"0:00"` for zero (elapsed zero is valid, unlike an unknown track length). Add `pub fn now_playing_progress(position_ms: u64, total: &str) -> String` returning `format!("{}/{}", format_position_time(position_ms), total)`, where `total` is the cached length string from `KnownTrack::time`. Both are pure and take no iced renderer.
- `src/ui/mod.rs`: add a `playback_position: Option<Duration>` field to `WinampPlayer`, initialized to `None` in `new`. Add `Message::PositionTick` and `Message::PlaybackPosition(Option<Duration>)`. Merge a 1-second `iced::time::every(POSITION_POLL_INTERVAL).map(|_| Message::PositionTick)` into the existing keyboard subscription; define `POSITION_POLL_INTERVAL` with a doc note that it bounds the readout's refresh, not the audio. The `PositionTick` arm clones the service and returns `Task::perform(async move { service.playback_position() }, Message::PlaybackPosition)`, keeping the seam's bounded blocking query off the UI thread. The `PlaybackPosition(position)` arm stores `position` when it is `Some` and ignores `None`, so a query delayed by a preview download leaves the previous readout in place instead of flickering. Set `playback_position = Some(Duration::ZERO)` in `TrackSelected` so a newly selected track starts at `0:00`, and set the same value in the `Stop` arm so Stop resets the readout immediately; Pause leaves the frozen polled value in place. In `TrackPlayed`, when the shared state reports nothing is playing (a failed stream start), set `playback_position = None` so the bar falls back to the plain total.
- `WinampPlayer::now_playing_time`: when `playback_position` is `Some` and the current track is in `known_tracks`, return `Cow::Owned(now_playing_progress(position_ms, &track.time))`; otherwise keep the existing borrowed total or `"--:--"` fallback. Update its doc comment to describe the combined playing readout and note the playing case allocates the combined string once per view build.
- `src/ui/views/tests.rs`: add `format_position_time_formats_elapsed_and_zero`, asserting `"0:00"` for `0`, `"3:12"` for `192_000`, and `"1:05"` for `65_000`, and `now_playing_progress_joins_elapsed_and_total`, asserting `"3:12/12:38"` for `(192_000, "12:38")`.
- `src/ui/tests.rs`: add `now_playing_time_shows_elapsed_over_total_when_a_position_is_known` (record a `KnownTrack`, set `playback_position`, assert the combined string) and `now_playing_time_falls_back_to_total_and_nothing` (a known track with no position still reads its total; no current track still reads `"--:--"`).
- `README.md`: extend the Usage sentence about the Now Playing bar's length so it says the readout shows the elapsed and total time while playing. Keep the hunk to that sentence.
- `docs/invariants.md`: extend the Now Playing label section with the progress guarantee and cite the new pure formatter/progress tests plus the two `now_playing_time` tests.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes.
- `format_position_time` returns `"0:00"`, `"3:12"`, and `"1:05"` for zero, 192000 ms, and 65000 ms; `now_playing_progress(192_000, "12:38")` returns `"3:12/12:38"`.
- With a position known and a current known track, the Now Playing bar renders `elapsed/total`; with the position unknown, it renders the existing total string; with no current track it renders `"--:--"`.
- Pause freezes the readout because the poll reads the backend's paused position, a left/right seek is reflected by the next poll, and Stop or a new selection resets it to `0:00`.
- A `None` reply leaves the last readout unchanged, and the position query runs in the iced task rather than blocking `update`.
- The change touches only the six listed files plus this `PLANS.md` edit.

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
