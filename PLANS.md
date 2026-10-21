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

### Add the playback-position readout to the Now Playing bar (Playback position, part 2/3: planned 2026-10-21 by the plan role, requires part 1/2 landed)

**Goal.** The audio seam answers a position only while a preview is loaded: `AudioOutput::position` returns `Some(position)` when a player is installed (including `Some(Duration::ZERO)` at the start of a preview) and `None` when no player is loaded or the backend cannot answer. This is the first half of the former part 2/2, split so the seam contract and its Pause/seek behavior are reviewed apart from the Now Playing readout that consumes them. The silent backend keeps the trait's default `None`, so a machine with no audio device shows no elapsed value rather than a frozen `0:00`.

**Approach.**

- `src/audio.rs`: revise `AudioOutput::position`'s contract and doc to "`Some` while a player is loaded (including `Duration::ZERO` at its start), `None` when no player is loaded or the backend cannot answer"; the default stays `None`. `SilentOutput` needs no method because the default already answers `None`.
- `src/audio/player.rs`: change `Command::Position` to carry `Sender<Option<Duration>>` and update its doc to say `None` when no player is loaded. In `serve_commands`, reply with `player.as_ref().map(WorkerPlayer::position)`. In `RodioOutput::position`, return `replies.recv_timeout(POSITION_QUERY_TIMEOUT).ok().flatten()`, so a closed worker or timed-out reply is `None`.
- `src/audio/tests.rs`: update `rodio_output_asks_the_worker_for_its_position` to send `Some(Duration::from_secs(3))`; make `position_query_returns_the_loaded_players_position` expect `Some(Duration::from_secs(42))`; rename `position_query_returns_zero_when_nothing_is_loaded` to `position_query_reports_none_when_nothing_is_loaded` and expect `None`. The timeout and closed-worker tests already expect `None`.
- `src/audio/tests.rs`: make `RecordingPlayer::seek_by` record the offset and make its `position` account for the recorded offsets via `seek_target(Duration::from_secs(42), ...)`, so the fake models a player whose clock moves on a seek while its no-seek position stays `42s` for the existing test. Add `a_seek_is_reflected_by_the_next_position_query` (Play, `SeekBy(5)`, `Position` replies `Some(47)`) and `a_pause_keeps_the_loaded_player_reporting_its_position` (Play, `Pause`, `Position` still answers `Some(42)`, the player was paused, and it was not stopped). The pause test pins the worker's half of the guarantee: Pause reaches the player and the worker keeps it installed, so the next poll reads the player's own frozen clock; the fake has no advancing clock, so it cannot itself prove rodio's clock stops.
- `src/audio/tests.rs`: extend `a_failed_preview_stops_the_previous_player` (or add `a_failed_preview_leaves_no_position_to_report`) to send a `Position` query after the failed asynchronous `Play` and assert it answers `None`, so a failed download/decode leaves no loaded player for the readout to freeze.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`.

**Acceptance criteria.**

- `make check` passes.
- A loaded player makes `Command::Position` answer `Some(player.position())`; no player answers `None`; a closed worker and a timed-out reply answer `None`; the silent backend answers `None`.
- `a_seek_is_reflected_by_the_next_position_query` shows the fake player's reported position moving by the seek (`42s` to `47s`), and `a_pause_keeps_the_loaded_player_reporting_its_position` shows the paused player still installed and reporting its position.
- A failed asynchronous preview start leaves no player, so the following position query answers `None`.
- The change touches only the three listed files plus this `PLANS.md` edit.

### Add the playback-position readout to the Now Playing bar (Playback position, part 3/3: planned 2026-10-21 by the plan role, requires part 2/3 landed)

**Goal.** The Now Playing bar's right-hand readout shows the current track's elapsed and total time as `"<elapsed>/<total>"` (for example `0:45/3:12`) whenever the audio backend reports a loaded preview's position, and falls back to the existing plain total (`"--:--"` when no track is current) when it does not. Pause leaves the stored value and the paused player's position in place, a seek is reflected by the next position reply, Stop and a new selection clear the readout, and a play whose preview fails to install leaves the plain total rather than a frozen `0:00`.

This is the second half of the former part 2/2 and consumes part 2/3's authoritative `AudioOutput::position`: the backend answers `None` when no preview is loaded, so the UI keeps no playback guard and a selected track with no preview URL never turns one on.

**Approach.**

- `src/apple_music.rs`: update `playback_position`'s doc to say it reports the loaded preview's position and `None` when no preview is loaded or the backend cannot answer, and remove its `#[allow(dead_code)]` now that the UI calls it.
- `src/ui/views/now_playing.rs`: add `pub fn format_position_time(position_ms: u64) -> String`, calling the existing `format_minutes_seconds` (which already yields `"0:00"` for zero and keeps minutes unbounded). Add `pub fn now_playing_progress(position_ms: u64, total: &str) -> String` returning `format!("{}/{}", format_position_time(position_ms), total)`. Change `now_playing_time` to take `position: Option<Duration>`: when `current_track` is in `tracks` and `position` is `Some`, return `Cow::Owned(now_playing_progress(...))` with the duration converted saturating to milliseconds; otherwise keep today's borrowed `KnownTrack::time` or `"--:--"` fallback. Update its doc comment to say the combined case allocates.
- `src/ui/mod.rs`: add `playback_position: Option<Duration>` and `position_epoch: u64` to `WinampPlayer`, initialized `None`/`0` in `new`, and define `POSITION_POLL_INTERVAL` (`Duration::from_secs(1)`) with a doc note that it bounds the readout refresh, not the audio. Add `Message::PositionTick` and `Message::PlaybackPosition { epoch: u64, position: Option<Duration> }`. In `init_ui`, batch `iced::time::every(POSITION_POLL_INTERVAL).map(|_| Message::PositionTick)` with the existing `keyboard::listen().filter_map(message_for)` (for example with `iced::Subscription::batch`). The `PositionTick` arm captures `player.position_epoch`, clones the service, and returns `Task::perform(async move { service.playback_position() }, move |position| Message::PlaybackPosition { epoch, position })`; the `PlaybackPosition` arm stores `position` (including `None`) only when `epoch == player.position_epoch`, dropping a reply issued before a transition. Add `fn reset_position_readout(player: &mut WinampPlayer)` that bumps `position_epoch` and sets `playback_position = None`; call it after the song resolves in `TrackSelected` and at the start of the `Stop` arm (which becomes a block returning the existing `transport_into`). Pause leaves the field alone, so the next poll keeps showing the paused player's frozen position. `WinampPlayer::now_playing_time` keeps its signature but passes `self.playback_position` to `views::now_playing_time`; the `view` call site is unchanged, though its comment that all three readouts always borrow must note the combined elapsed case allocates.
- `src/ui/views/tests.rs`: update the `now_playing_time_resolves_known_track_to_cached_label` call sites to pass `None` as the position; extend it (or add `now_playing_time_shows_elapsed_over_total_when_a_position_is_known`) so `now_playing_time(&tracks, Some("song-1"), Some(Duration::from_secs(192)))` reads `"3:12/3:30"` and is `Cow::Owned`; add `format_position_time_formats_elapsed_and_zero` (`0`→`"0:00"`, `192_000`→`"3:12"`, `65_000`→`"1:05"`) and `now_playing_progress_joins_elapsed_and_total` (`(192_000, "12:38")`→`"3:12/12:38"`).
- `src/ui/tests.rs`: keep `now_playing_time_shows_the_played_tracks_duration_after_browsing_to_another_album` asserting the plain total (its calls now use the unchanged `WinampPlayer::now_playing_time` signature); add `now_playing_time_shows_elapsed_over_total_while_a_position_is_known` (reuse `play_song_1_then_browse_to_album_2`, set `player.playback_position = Some(Duration::from_secs(45))`, assert `"0:45/3:30"`), `pause_keeps_the_stored_elapsed_readout` (set a known position, drive `Message::Pause`, then assert the field still holds that position and the bar still reads `"0:45/3:30"`; together with part 2/3's `a_pause_keeps_the_loaded_player_reporting_its_position` this pins that pause freezes rather than clears), `a_position_reply_after_a_seek_updates_the_readout` (set `Some(45)`, drive `Message::SeekForward`, feed `Message::PlaybackPosition` with `Some(50)`, assert `"0:50/3:30"`; the existing `seek_messages_drive_the_audio_backend` pins the backend call), `track_selected_clears_the_elapsed_readout`, `stop_clears_the_elapsed_readout`, `position_replies_store_and_stale_ones_are_dropped` (store a reply at the current epoch, then drive a reply stamped with the pre-`Stop` epoch and assert it does not overwrite), and `position_tick_schedules_a_query` (`assert_message_schedules_work`).
- `README.md`: extend only the Usage sentence that describes the Now Playing bar's artist and title so it says the length reads as elapsed/total while a preview is loaded and as the plain length otherwise. Do not edit the `<!-- tumwater:status:start -->` block or add archive narrative anywhere; that block is state-only (see `762af8c`), and the README diff must be confined to the Usage paragraph.
- `docs/invariants.md`: in the Now Playing label section, add the time-readout guarantee (elapsed/total only while a loaded preview position is known, the cached total otherwise, `--:--` with no current track), its enforcement by `views::now_playing_time`/`now_playing_progress` and the `PlaybackPosition` arm, and cite the new tests plus part 2/3's audio-seam `position_query_reports_none_when_nothing_is_loaded`, `a_seek_is_reflected_by_the_next_position_query`, and `a_pause_keeps_the_loaded_player_reporting_its_position`.

**Files touched.** `src/apple_music.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes.
- `format_position_time` returns `"0:00"`, `"3:12"`, and `"1:05"` for zero, 192000 ms, and 65000 ms; `now_playing_progress(192_000, "12:38")` returns `"3:12/12:38"`.
- With a known current track and a `Some` position, the bar renders `elapsed/total`; with `None`, the plain total; with no current track or an unknown id, `"--:--"`.
- Pause does not clear the stored position and the next poll still reads the paused player, so the readout keeps the frozen value; `a_position_reply_after_a_seek_updates_the_readout` shows a seek's new position reaching the bar on the next reply; Stop and a new selection clear the readout immediately through `reset_position_readout`. A failed preview start leaves `None`, so the bar falls back to the plain total.
- A `PlaybackPosition` reply stamped with an older `position_epoch` is dropped, so a poll in flight across a Stop or selection cannot restore the previous elapsed value.
- The poll runs once per `POSITION_POLL_INTERVAL` in an iced task, not in `update`.
- The README diff is confined to the Usage sentence; the status block is byte-for-byte unchanged.
- The change touches only the seven listed files plus this `PLANS.md` edit.

### Show each song row's artist and title in every browse list (planned 2026-10-21 by the plan role)

**Goal.** Every song row in the Songs view, All Songs, and search results reads `<artist> - <title>` when the song carries an artist (for example `The Sample Band - Opening`), as the library mockup's playlist rows do, and reads just the title when the artist is empty. Row numbering, the playing-row marker, the length label, selection messages, and ordering stay exactly as they are. This is independent of the pending playback-position plan: it touches the browse rows, not the Now Playing bar or the audio seam.

**Approach.**

- `src/library.rs`: add an inherent `Song::display_title(&self) -> Cow<'_, str>` (import `std::borrow::Cow`). It returns `Cow::Borrowed(&self.title)` when `self.artist.is_empty()` and `Cow::Owned(format!("{} - {}", self.artist, self.title))` otherwise. This is presentation-only; the serialized model is unchanged. Add a test asserting `sample_song().display_title()` is `"The Sample Band - Opening"`, that a clone with `artist = String::new()` yields the borrowed `"Opening"`, and that the joined case is owned.
- `src/ui/browse.rs`: add a `titles: Vec<String>` cache to `BrowseList<T>`, documented beside `labels` as the per-row primary title cached when the list is stored so a view refresh does not re-join every artist and title. Rename `store_with_labels` to `store_with_row_text(items, title, label)`, taking two closures: the title closure fills `titles` and the label closure fills `labels`, both in row order. `store` clears both caches; `clear` and `fail` drop both with the rows; `replace_rows` leaves both to the caller. Update the module doc and every existing `store_with_labels` reference in docs/comments to the new name.
- `src/ui/mod.rs`: in the `SongsLoaded` and `AllSongsLoaded` arms call `store_with_row_text` with `|song| song.display_title().into_owned()` for the title and the existing `|song| views::format_track_time(song.duration_ms)` for the length. Pass `&player.songs.titles` / `&player.all_songs.titles` to `views::view_songs` / `views::view_all_songs` in `view`.
- `src/ui/views/browse.rs`: change `scrollable_list`'s item title from `&'a str` to `Cow<'a, str>` so a row can borrow a cached title; wrap the artist, album, and playlist row titles in `Cow::Borrowed`. Change `song_row` to take a `cached_title: Option<&'a str>` beside the existing cached length and return the row title as a `Cow`: `Cow::Borrowed(cached_title)` when the cache has one, otherwise `song.display_title()`. Thread a `titles: &'a [String]` slice through `song_list`, `view_songs`, and `view_all_songs`, pairing each song with `titles.get(index).map(String::as_str)` exactly as the length labels are paired now. Update the affected doc comments, including why the cache exists and that an index without a cached title falls back to composing it.
- `src/ui/views/tests.rs`: update every `song_row` call for the new `cached_title` argument and `view_songs`/`view_all_songs` calls for the `titles` slice. Change `song_row_uses_title_and_selects_the_song_by_index` to expect `"The Sample Band - Opening"` from the fallback, add `song_row_prefers_the_lists_cached_title` (a cached value that differs from the song fields must win), and add `song_row_uses_the_title_alone_when_the_artist_is_empty`. Keep the existing length-cache, unknown-duration, current-marker, and construct-every-view tests passing with the new argument order.
- `src/ui/tests.rs`: extend the song-list cache tests to assert `player.songs.titles` and `player.all_songs.titles` are built in row order from `display_title` (for example `["The Sample Band - One", "The Sample Band - Two", "The Sample Band - Three"]`), and that a navigation's `clear` drops the titles together with the length labels. Rename the cache test so its comment names both caches if needed.
- `README.md`: extend the Usage sentence that says song rows show each track's length so it also says song rows read `artist - title` (or the title alone when the source supplies no artist).
- `docs/invariants.md`: the browse-hierarchy, reply-freshness, and Now Playing-label guarantees do not change, so leave it unchanged.

**Files touched.** `src/library.rs`, `src/ui/browse.rs`, `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `Song::display_title` returns `"The Sample Band - Opening"` for the sample song and the borrowed title `"Opening"` when the artist is empty.
- `SongsLoaded` and `AllSongsLoaded` fill both `titles` and `labels` in row order; `store`, `clear`, and `fail` drop both caches, so a refresh cannot pair a new row with a stale title or length.
- Every song row renders the cached display title when present and `Song::display_title` otherwise; a cached title wins over the song's own fields. The length label, message payload, epoch, numbering, playing marker, and list order are unchanged.
- Songs, All Songs, and search results all render through the same `song_list`, so each shows `<artist> - <title>` and an artist-less song shows only its title.
- The change touches only the seven listed files plus this `PLANS.md` edit.

## Done
