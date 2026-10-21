# Done plans

Finished plans, newest first, moved here by tumwater from PLANS.md's `## Done`. This file
only grows: search it with grep, or read one entry by its line range. Never read it whole.

## Done

### Scroll an overlong Now Playing title across the LCD (planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** The brief's mockup says "The song title scrolls across the display": when the Now Playing bar's combined `artist - title` line is wider than the LCD, it scrolls one character per tick and wraps through a short gap, so the full track name is readable instead of running under the time readout. A line that fits stays still and keeps the existing zero-allocation rendering. This is independent of the playback-position series: it reads only the label and artist the bar already resolves, and touches neither the audio seam nor the time readout.

**Approach.**

- `src/ui/views/now_playing.rs`: add `pub(super) const MARQUEE_WIDTH: usize = 28` — a documented character budget sized to keep the line inside the roughly 550px-wide window beside the `"Now Playing: "` caption and the time — and `const MARQUEE_GAP: usize = 3` (the spaces between repeats). Add `pub(super) fn marquee_text(text: &str, offset: usize, width: usize) -> String`: count characters (not bytes), return `text.to_string()` unchanged when it fits `width`, otherwise walk a cycle of `text` plus `MARQUEE_GAP` spaces for `width` characters starting at `offset % cycle`, substituting a space past the text's end. The pure helper is called only by the overlong branch, so its allocation belongs to the scrolling path.
- `src/ui/views/now_playing.rs`: change `view_now_playing` to take `marquee_offset: usize` as a fourth argument. Compute the display length as `label.chars().count()` plus `artist.chars().count() + 3` when the artist is non-empty. When it fits `MARQUEE_WIDTH`, keep the current structure — the separate artist, `" - "`, and label widgets, all borrowed, plus the `Space` fill and time. When it does not fit, build `artist - label` (or the label alone when the artist is empty) and render it as one `Text` of `marquee_text(&display, marquee_offset, MARQUEE_WIDTH)` in `theme::LCD_GREEN`, keeping the `Space` fill and time at the right edge. Update the doc comment to describe the scrolling branch and the character-budget approximation.
- `src/ui/mod.rs`: add `marquee_offset: usize` to `WinampPlayer`, initialized `0` in `WinampPlayer::new`. Add `Message::MarqueeTick` with a doc comment saying it advances the title window; its `update` arm sets `player.marquee_offset = player.marquee_offset.wrapping_add(1)` and returns `Task::none()`. In the `TrackSelected` arm, set `player.marquee_offset = 0` beside the `known_tracks` entry, so a new song starts at the beginning of its title. Add `const MARQUEE_TICK: std::time::Duration = std::time::Duration::from_millis(250)` near `init_ui` (import `Duration` from `std::time`). Change `init_ui`'s subscription to `iced::Subscription::batch([...])` of the existing `keyboard::listen().filter_map(message_for)` and `marquee_ticks()`, a helper that feeds ticks from one std thread through the futures channel iced re-exports (iced's `time::every` needs a `smol`/`tokio` backend feature the app does not enable, and its default thread-pool backend's `time` module is empty); `marquee_ticks` is a thin wrapper over `marquee_tick_stream()`, whose one-slot channel drops a tick when the UI is behind but stops the thread only when the receiver is gone, so a slow poll cannot end the scrolling. The timer runs unconditionally, and the view ignores the offset whenever the line fits. Pass `player.marquee_offset` to `views::view_now_playing` in `view`.
- `src/ui/views/tests.rs`: add `marquee_text_keeps_a_line_that_fits` (`marquee_text("Opening", 9, 28) == "Opening"`), `marquee_text_windows_from_the_offset` (`"The Sample Band - Opening"`, width 10: offset 0 is `"The Sample"`, offset 2 is `"e Sample B"`), `marquee_text_wraps_through_the_gap_to_the_start` (offset 25 at width 10 is `"   The Sam"`), and `marquee_text_counts_characters_not_bytes` (`marquee_text("Café — Ok", 0, 5) == "Café "`). Add `overlong_now_playing_line_renders_a_new_window_as_the_offset_advances`: lay the row out with iced's null renderer, capture the text widgets through a widget `Operation`, and assert the overlong line contains the window for offsets 0 and 1 (which differ) while a fitting line captures the same separate artist and title at every offset. Extend `now_playing_bar_and_browse_action_buttons_construct` to pass a `marquee_offset` to both `view_now_playing` calls and add one overlong-label construction that exercises the scrolling branch.
- `src/ui/tests.rs`: add `marquee_tick_advances_the_offset` (two `Message::MarqueeTick` updates take `marquee_offset` from 0 to 2), `marquee_ticks_keep_ticking_when_the_receiver_falls_behind` (queue the two messages the channel can hold, sleep through several more tick periods so the sender attempts a send against the full channel, then read a third tick, proving a full channel drops a tick instead of stopping the timer), and `track_selected_resets_the_marquee_offset` (load `stepping_songs()`, set the offset to 7, drive `TrackSelected { epoch: 1, index: 0 }`, assert 0).
- `README.md`: extend only the Usage sentence that describes the Now Playing bar's artist and title so it also says the line scrolls when it is longer than the display. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the Now Playing label section, add the scrolling guarantee: a combined artist/title longer than `MARQUEE_WIDTH` characters is windowed by `views::now_playing::marquee_text`, advanced one character per `Message::MarqueeTick`, reset to the start by `TrackSelected`, and shown whole-and-borrowed when it fits. Cite the marquee tests in `src/ui/views/tests.rs` and the tick/reset tests in `src/ui/tests.rs`.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`, plus the `PLANS.md`/`PLANS-DONE.md` move.

**Acceptance criteria.**

- `make check` passes.
- `marquee_text` returns the input unchanged for a line that fits; for `"The Sample Band - Opening"` at width 10 it returns `"The Sample"` at offset 0, `"e Sample B"` at offset 2, and `"   The Sam"` at offset 25; `marquee_text("Café — Ok", 0, 5)` is `"Café "`, proving character-count indexing.
- A label plus artist within `MARQUEE_WIDTH` renders through the existing separate artist/title widgets and stays still at every offset; a longer one renders the single windowed `Text` and advances with the offset, while the time stays at the right edge. The row-rendering test captures those text widgets and asserts the different windows.
- `Message::MarqueeTick` advances `marquee_offset` by one with no task, `marquee_ticks` emits one per `MARQUEE_TICK` alongside the unchanged key bindings and keeps ticking after a delayed receiver, and a `TrackSelected` resets the offset to 0 so a new song starts at character 0.
- The README diff is confined to the Usage sentence; the status block is byte-for-byte unchanged.
- The change touches only the six listed source/docs files plus the `PLANS.md`/`PLANS-DONE.md` move, and adds no dependency.

### Show each song row's artist and title in every browse list (planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** Every song row in the Songs view, All Songs, and search results reads `<artist> - <title>` when the song carries an artist (for example `The Sample Band - Opening`), as the library mockup's playlist rows do, and reads just the title when the artist is empty. Row numbering, the playing-row marker, the length label, selection messages, and ordering stay exactly as they are. This is independent of the pending playback-position plan: it touches the browse rows, not the Now Playing bar or the audio seam.

**Approach.**

- `src/library.rs`: add an inherent `Song::display_title(&self) -> Cow<'_, str>` (import `std::borrow::Cow`). It returns `Cow::Borrowed(&self.title)` when `self.artist.is_empty()` and `Cow::Owned(format!("{} - {}", self.artist, self.title))` otherwise. This is presentation-only; the serialized model is unchanged. Add a test asserting `sample_song().display_title()` is `"The Sample Band - Opening"`, that a clone with `artist = String::new()` yields the borrowed `"Opening"`, and that the joined case is owned.
- `src/ui/browse.rs`: add a `titles: Vec<String>` cache to `BrowseList<T>`, documented beside `labels` as the per-row primary title cached when the list is stored so a view refresh does not re-join every artist and title. Rename `store_with_labels` to `store_with_row_text(items, title, label)`, taking two closures: the title closure fills `titles` and the label closure fills `labels`, both in row order. `store` clears both caches; `clear` and `fail` drop both with the rows; `replace_rows` leaves both to the caller. Update the module doc and every existing `store_with_labels` reference in docs/comments to the new name.
- `src/ui/mod.rs`: in the `SongsLoaded` and `AllSongsLoaded` arms call `store_with_row_text` with `|song| song.display_title().into_owned()` for the title and the existing `|song| views::format_track_time(song.duration_ms)` for the length. Pass `&player.songs.titles` / `&player.all_songs.titles` to `views::view_songs` / `views::view_all_songs` in `view`.
- `src/ui/views/browse.rs`: change `scrollable_list`'s item title from `&'a str` to `Cow<'a, str>` so a row can borrow a cached title; wrap the artist, album, and playlist row titles in `Cow::Borrowed`. Change `song_row` to take a `cached_title: Option<&'a str>` beside the existing cached length and return the row title as a `Cow`: `Cow::Borrowed(cached_title)` when the cache has one, otherwise `song.display_title()`. Thread a `titles: &'a [String]` slice through `song_list`, `view_songs`, and `view_all_songs`, pairing each song with `titles.get(index).map(String::as_str)` exactly as the length labels are paired now. Update the affected doc comments, including why the cache exists and that an index without a cached title falls back to composing it.
- `src/ui/views/tests.rs`: update every `song_row` call for the new `cached_title` argument and `view_songs`/`view_all_songs` calls for the `titles` slice. Change `song_row_uses_title_and_selects_the_song_by_index` to expect `"The Sample Band - Opening"` from the fallback, add `song_row_prefers_the_lists_cached_title` (a cached value that differs from the song fields must win), and add `song_row_uses_the_title_alone_when_the_artist_is_empty`. Keep the existing length-cache, unknown-duration, current-marker, and construct-every-view tests passing with the new argument order.
- `src/ui/tests.rs`: extend the song-list cache tests to assert `player.songs.titles` and `player.all_songs.titles` are built in row order from `display_title` (for example `["The Sample Band - One", "The Sample Band - Two", "The Sample Band - Three"]`), and that a navigation's `clear` drops the titles together with the length labels. Rename the cache test so its comment names both caches if needed.
- `README.md`: extend the Usage sentence that says song rows show each track's length so it also says song rows read `artist - title` (or the title alone when the source supplies no artist).
- `docs/invariants.md`: update the browse reply-freshness guarantee's "Enforced by" reference from `store_with_labels` to the renamed `store_with_row_text`; the guarantees themselves do not change.

**Files touched.** `src/library.rs`, `src/ui/browse.rs`, `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes.
- `Song::display_title` returns `"The Sample Band - Opening"` for the sample song and the borrowed title `"Opening"` when the artist is empty.
- `SongsLoaded` and `AllSongsLoaded` fill both `titles` and `labels` in row order; `store`, `clear`, and `fail` drop both caches, so a refresh cannot pair a new row with a stale title or length.
- Every song row renders the cached display title when present and `Song::display_title` otherwise; a cached title wins over the song's own fields. The length label, message payload, epoch, numbering, playing marker, and list order are unchanged.
- Songs, All Songs, and search results all render through the same `song_list`, so each shows `<artist> - <title>` and an artist-less song shows only its title.
- The change touches only the eight listed files plus this backlog move.

### Add the library window's Top button and open every browse list at the top (planned 2026-10-21 by the plan role, done 2026-10-21)

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

### Add the playback-position readout to the Now Playing bar (Playback position, part 1/2: planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** The audio seam can report where the loaded preview is, so the Now Playing bar can later show elapsed against total. `AudioOutput::position()` answers `Option<Duration>`: `Some` with the loaded player's position (including `Some(Duration::ZERO)` when the worker has no player), or `None` when the backend cannot answer (the silent fallback, a default test stub, or a query the worker cannot reach within the seam's bound). `AppleMusicService::playback_position()` forwards that value. This part changes no UI; it gives part 2/2 a narrow, testable source for the elapsed readout.

**Approach.**

- `src/audio.rs`: import `std::time::Duration` and add a provided trait method after `seek_by`: `fn position(&self) -> Option<Duration> { None }`. Its doc says it returns the loaded preview's position, `Some(Duration::ZERO)` when the backend has no loaded player, and `None` when the backend cannot answer; the default keeps `SilentOutput` and every existing test fake compiling without a position of their own.
- `src/audio/player.rs`: add `Command::Position(std::sync::mpsc::Sender<Duration>)`. Add `fn position(&self) -> Duration` to `WorkerPlayer`, with `RodioPlayer` returning `self.player.get_pos()` (the same source `seek_player` already reads). In `serve_commands`, handle `Command::Position(reply)` by sending `player.as_ref().map_or(Duration::ZERO, WorkerPlayer::position)`; because the loop handles the command in channel order, a position query sent during a preview download is answered after that download installs the new player. In `impl AudioOutput for RodioOutput`, add `position`: send `Command::Position` through the existing `send`, then wait on the reply with `recv_timeout`, returning `None` when the send fails or the wait times out. Define `POSITION_QUERY_TIMEOUT` next to `DEVICE_OPEN_TIMEOUT` (100 ms) and document that a download can hold the worker for up to `preview::PREVIEW_TIMEOUT`, so the query must not block the caller indefinitely.
- `src/apple_music.rs`: add `pub(crate) fn playback_position(&self) -> Option<Duration> { self.audio.position() }` beside `seek_output`, documented as synchronous, touching no shared state, and bounded by the seam's query timeout.
- `src/audio/tests.rs`: give `RecordingPlayer` a `position` implementation returning a fixed non-zero `Duration`, then add `position_query_returns_the_loaded_players_position` (send `Command::Position` after a `Play`, assert the reply equals the fake's position) and `position_query_returns_zero_when_nothing_is_loaded` (send `Command::Position` with no player). Add `rodio_output_asks_the_worker_for_its_position` (a helper thread reads `Command::Position` from the command channel and replies, while the test calls `position()`) and extend `rodio_output_reports_a_closed_worker_for_every_command` with `position() == None`.
- `src/test_support.rs`: override `position` for `RecordingAudio` to return `Some(Duration::from_secs(7))`, documented as a fixed stub position; no other call recording changes.
- `src/apple_music/tests.rs`: add `playback_position_forwards_the_audio_output_position`, building the service with `Arc::new(RecordingAudio::default())` through `with_audio` and asserting the service returns `Some(Duration::from_secs(7))`.
- `docs/invariants.md`: unchanged; this part adds a seam, not a user-visible guarantee.

**Files touched.** `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`.

**Acceptance criteria.**

- `make check` passes.
- With a player loaded, `serve_commands` answers `Command::Position` with that player's `position()`; with no player it answers `Duration::ZERO`.
- `RodioOutput::position` returns the reply the worker sends, and returns `None` when the command channel is closed or no reply arrives within `POSITION_QUERY_TIMEOUT`.
- `AppleMusicService::playback_position` returns exactly the injected backend's `position()` value.
- `SilentOutput` and all existing `AudioOutput` test fakes keep compiling without adding a method, because the trait method has a `None` default.
- The change touches only the six listed files plus this `PLANS.md` edit; no UI, README, or invariant text changes.

### Show the item count beneath artist, album, and playlist lists (planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** Every loaded non-song browse list shows how many rows it holds beneath the rows — `3 artists`, `2 albums`, `1 playlist` — as the library mockup's status area does (`docs/screenshots/library.png`). The song lists keep their existing count-and-time footer and gain nothing new. This is independent of the pending row numbering: it reads only the slice length each view already receives.

**Approach.**

- `src/ui/views/browse.rs`: add a pure `pub(super) fn list_count_summary(count: usize, singular: &str, plural: &str) -> String` returning `"<count> <noun>"`, with `singular` chosen exactly when `count == 1` and `plural` otherwise. It formats any count, including `0`, so the wording is testable without an iced renderer (`format_minutes_seconds` is the shared formatter for the song footer).
- Add a private `fn list_with_count<'a>(list: Element<'a, Message>, count: usize, singular: &str, plural: &str) -> Element<'a, Message>`: when `count` is non-zero, wrap `list` in a `Column::new().push(list).push(Text::new(list_count_summary(...)).size(14))` with `Length::Fill` width and `Length::FillPortion(3)` height, mirroring the song footer in `song_list`; when `count` is zero, return `list` unchanged so a loading, error, or empty placeholder gets no `0 artists` footer.
- Wrap the three non-song list builders at their row-construction sites: `view_artists` passes `artists.len()` with `"artist"`/`"artists"`, `album_list` passes `albums.len()` with `"album"`/`"albums"` (so both `view_albums` and `view_all_albums` gain the footer once), and `view_playlists` passes `playlists.len()` with `"playlist"`/`"playlists"`. Do not wrap inside `scrollable_list` itself: `song_list` already appends its own count-and-time footer, so wrapping there would give every song list a second footer.
- Update the doc comments of `view_artists`, `album_list` (which `view_albums` and `view_all_albums` both delegate to), and `view_playlists` to say a non-empty list appends its item-count footer, and note in `list_with_count` why it skips an empty list.
- `src/ui/views/tests.rs`: import `list_count_summary` and add `list_count_summary_uses_the_singular_and_plural`, asserting `"1 artist"` for a count of one, `"3 artists"` for a count of three, `"1 playlist"`, and `"0 albums"`. The existing `browse_views_construct_over_the_loaded_library` and `browse_views_construct_over_an_empty_list` build the populated and empty branches of every wrapped builder through `construct_every_browse_view`.
- `README.md`: extend the existing Usage sentence — which already says each song list shows its song count and total playing time — to also say artist, album, and playlist lists show their item count beneath the rows. Keep the hunk to that sentence.
- `docs/invariants.md`: the browse-hierarchy and reply-freshness guarantees do not change, and the document makes no claim about footers, so leave it unchanged.

**Files touched.** `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `list_count_summary` returns `"1 artist"` for `(1, "artist", "artists")`, `"3 artists"` for `(3, "artist", "artists")`, `"1 playlist"` for `(1, "playlist", "playlists")`, and `"0 albums"` for `(0, "album", "albums")`.
- A loaded Artists, Albums, All Albums, or Playlists view renders its rows followed by exactly one footer whose count equals the number of rows passed in; an empty, loading, or failed one still renders only its placeholder, with no `0 …` footer.
- Song lists keep the existing `song_list_summary` footer and call no new footer helper, so no song list shows two footers.
- Row order, row labels, selection messages, epochs, and placeholder wording are unchanged, and both construct tests build every browse view without panicking.
- The change touches only the three files listed above (plus the `PLANS.md` edit and backlog moves), and the README hunk is limited to the item-count clause.

### Number every browse-list row (planned 2026-10-21 by the plan role; done 2026-10-21)

**Goal.** Each row in every browse list opens with its 1-based position and a period (`1. The Sample Band`, `2. First Record`), as the classic Winamp playlist editor does; the currently playing row keeps its `▶` marker before the number (`▶ 3. Opening`). This is independent of the planned count/time footer and of the browse data: numbering is derived from each list's row order, so it needs no service or model change.

**Approach.**

- `src/ui/views/browse.rs`: add a pure `pub(super) fn row_prefix(position: usize, is_current: bool) -> String` that returns `"{n}. "` for zero-based `position`, with `"▶ "` before the number when `is_current` is true (so `row_prefix(0, false)` is `"1. "` and `row_prefix(0, true)` is `"▶ 1. "`). It is the one place the marker and number order are composed, so a marker moved after the number cannot slip in untested.
- In `fn scrollable_list`, enumerate the incoming items (`for (position, (title, label, message, is_current)) in items.into_iter().enumerate()`) and build the row's leading label from `row_prefix(position, is_current)` instead of the current conditional `▶ ` row. Keep the title and secondary label as they are, and keep the existing per-row style selection (`current_row_style` for the playing row, `playlist_row_style` otherwise) and the empty-placeholder branch unchanged. Every browse list — Artists, Albums, Songs, All Songs, All Albums, Playlists — renders through `scrollable_list`, so all six gain numbering with no row-builder or call-site change.
- Update `scrollable_list`'s doc comment: it now says each row is prefixed with its 1-based position (and the playing marker), and that the prefix is one owned `String` per row per refresh while the title and secondary label still borrow. Do not cache prefixes in `BrowseList`: the position comes from the iterator and a cached list would need invalidation on every store for no correctness gain.
- `src/ui/views/tests.rs`: add a direct test for the helper, `row_prefix_numbers_rows_and_puts_the_play_marker_first`, asserting `row_prefix(0, false) == "1. "`, `row_prefix(9, false) == "10. "`, `row_prefix(99, false) == "100. "`, and `row_prefix(0, true) == "▶ 1. "`. The existing `browse_views_construct_over_the_loaded_library` and `browse_views_construct_over_an_empty_list` continue to build every numbered view through the shared constructor.
- `README.md`: add one sentence to the Usage paragraph saying every browse list numbers its rows from 1 like a Winamp playlist. Keep the hunk to that sentence.
- `docs/invariants.md` records browse navigation, reply freshness, and the Now Playing label; row numbering changes none of those guarantees, so leave it unchanged.

**Files touched.** `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `row_prefix` returns `"1. "`, `"10. "`, and `"100. "` for positions 0, 9, and 99 with `is_current == false`, and `"▶ 1. "` for position 0 with `is_current == true`.
- Every row built by `scrollable_list` gets the prefix before its title in both the playing and non-playing branches; the playing-row background and style are unchanged, and an empty list still shows only its placeholder with no number.
- Row selection messages, cached length labels, list order, and empty/loading/error wording are unchanged, and the construct tests build every browse view over both the loaded and empty library without panicking.
- The feature itself changes only the three files listed above (the `PLANS.md`/`PLANS-DONE.md` move aside), and the README hunk is limited to the numbering sentence.

### Show each song list's count and total playing time (planned 2026-10-21 by the plan role, done 2026-10-21)

**Goal.** Every loaded song list shows, beneath its rows, how many songs it holds and their combined playing time — the classic Winamp playlist editor's status line. Today `song_list` in `src/ui/views/browse.rs` builds only the scrollable rows: each row shows its own formatted length (`song_row` via `format_track_time`), but the list never aggregates them. This is independent of the planned Shuffle-On Previous history: it reads only the `[Song]` slice the view already receives.

**Approach.**

- `src/ui/views/browse.rs`: add `pub(super) fn song_list_summary(songs: &[Song]) -> String` returning `"<count> song(s) - <total>"`, with singular `song` for a count of one and plural `songs` otherwise. Count every entry, including one whose `duration_ms` is `0`, and sum the durations with `u64::saturating_add` so a large list cannot overflow the total.
- Add a shared `fn format_minutes_seconds(total_ms: u64) -> String` in `now_playing.rs`: `0` formats as `"0:00"`, otherwise zero-padded `m:ss` with unbounded minutes (`3_665_000` formats as `"61:05"`), mirroring `views::format_track_time`'s arithmetic but not its `0` → `"--:--"` placeholder. Do not change `format_track_time`: `0` there means one track's duration is unknown, while `0` here is a valid list total when every duration is unknown.
- In `fn song_list`, keep the current `scrollable_list(...)` result for the rows; when `songs` is non-empty, return a `Column::new().push(list).push(Text::new(song_list_summary(songs)).size(14))` with `Length::Fill` width and the scrollable's current `Length::FillPortion(3)` height, so the browse panel keeps its share of the window and the summary sits left-aligned below the rows. When `songs` is empty, return the scrollable unchanged so the loading, error, and empty placeholders stay alone instead of gaining a `0 songs` footer. Compute the summary once per view refresh, not once per row, and update `song_list`'s doc comment to say it appends the count/time footer.
- No call-site changes are needed: `view_songs` (album and playlist songs and search results) and `view_all_songs` both build through `song_list`, so every loaded song list gains the footer. Artist, album, and playlist lists keep their current bodies.
- `src/ui/views/tests.rs`: add direct tests for the pure helper — `song_list_summary_counts_songs_and_sums_durations` (210_000 + 45_000 → `"2 songs - 4:15"`), `song_list_summary_uses_the_singular_for_one_song` (210_000 → `"1 song - 3:30"`), `song_list_summary_counts_unknown_durations_without_adding_time` (`[210_000, 0]` → `"2 songs - 3:30"`), and `song_list_summary_keeps_counting_minutes_past_an_hour` (3_665_000 → `"1 song - 61:05"`). The existing `browse_views_construct_over_the_loaded_library` and `browse_views_construct_over_an_empty_list` already exercise both footer branches through the view builders.
- `README.md`: add one sentence to the Usage paragraph saying each song list shows its song count and total playing time beneath the rows. Keep the hunk to that sentence.
- `docs/invariants.md` records browse navigation, reply freshness, and the Now Playing label; this display-only footer changes none of those guarantees, so leave it unchanged.

**Files touched.** `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `song_list_summary` reports the exact count and sum: `[]` → `"0 songs - 0:00"`, one 210_000 ms song → `"1 song - 3:30"`, 210_000 + 45_000 ms → `"2 songs - 4:15"`, and 3_665_000 ms → `"1 song - 61:05"`.
- A song with `duration_ms == 0` still counts toward the song total but contributes no time, so `[210_000, 0]` reads `"2 songs - 3:30"`.
- A non-empty song list built by `view_songs` or `view_all_songs` includes the footer, while an empty list shows only its existing placeholder; the construct tests cover both cases without panicking.
- The implementation diff changes only the three files listed above for the feature itself (the `PLANS.md`/`PLANS-DONE.md` move aside), and the README hunk is limited to the new summary sentence.

### Make Previous retrace the play history when Shuffle is on (planned 2026-10-20 by the plan role, done 2026-10-21)

**Goal.** With Shuffle on, pressing Previous returns to the track actually played before the current one, retracing the play order like classic Winamp, instead of stepping one row backward in the visible list. With Shuffle off, Previous keeps its sequential and Repeat-aware behavior. Today `step_track` in `src/ui/mod.rs` sends only a forward step through `transport::shuffled_track_id`; every backward step goes through `transport::previous_track_id`. The `step_track` doc comment and `previous_with_shuffle_still_steps_backward` in `src/ui/tests.rs` pin that Previous ignores Shuffle because no play history exists.

**Approach.**

- `src/ui/transport.rs`: beside `shuffled_track_id`, add `pub(super) const HISTORY_LIMIT: usize = 100` and `#[derive(Debug, Default)] pub struct PlayHistory` holding `entries: Vec<String>`, `cursor: Option<usize>`, and `pending: Option<PendingJump>`, plus a private `#[derive(Debug)] struct PendingJump { target: usize, from: Option<String>, retry: bool }`.
  - `pub fn record(&mut self, track_id: &str)`: when `pending` names a target entry equal to `track_id`, the history jump landed, so set `cursor` to that target and clear `pending` without pushing, truncating, or trimming — the jump neither duplicates the entry nor drops the forward entries, and a later new track truncates them from the landed cursor. Otherwise clear `pending` first; when the cursor already names `track_id` do nothing, and otherwise truncate entries after the cursor, push `track_id`, point the cursor at the new last entry, then drop the oldest entries past `HISTORY_LIMIT` while moving the cursor down with them.
  - `pub fn previous(&mut self, songs: &[Song], current: Option<&str>) -> Option<String>`:
    - A pending jump stays in effect while `current` is `None`, names the jump's target entry (its play committed but `record` has not run), or names the jump's `from` track (its play has not committed yet). When `current` names anything else, the user selected another track before the jump landed, so clear `pending` and continue from the cursor, which still names the last track actually recorded.
    - A failed play calls `retry_pending`, which keeps the pending jump and marks it for retry: `play_track` leaves `current_track` naming the target when the backend rejects the preview, so the next `previous` returns that same target once more (clearing the retry flag) and only a later press walks before it. A failed play with no pending jump is a no-op.
    - With a jump in effect, scan strictly before its target index, so repeated Previous presses before that play commits keep walking back. Without a jump, take `self.cursor?` and scan strictly before it when its entry is `current`, otherwise up to and including it (`cursor + 1`) when `current` is a newer, unrecorded selection.
    - Find the highest index below the start whose id `songs` contains; on a hit set `pending` to that index (keeping an in-effect jump's original `from`), return its id, and leave the cursor unchanged until the play records. On a miss return `None` leaving the cursor and any in-effect jump unchanged, so `step_track` falls back to `transport::previous_track_id`. Skip entries absent from `songs` so a browse to another list selects a row the visible list can show. Do not add the rejected draft's `at_start` helper: an exhausted history uses the same sequential fallback.
- `src/ui/mod.rs`: add `play_history: transport::PlayHistory` to `WinampPlayer` and initialize it with `Default::default()` in `WinampPlayer::new`. Change `step_track` to take `&mut WinampPlayer`: when `!forward && state.shuffle`, first try `player.play_history.previous(items, state.current_track.as_deref())`, use its id when present, and otherwise fall through to `transport::previous_track_id` as today; the forward Shuffle branch and both sequential branches stay as they are. In the `Message::TrackPlayed` arm, inside the existing current-generation guard, record `state.current_track` only when `state.is_playing` is true, so a failed play never enters the history and a superseded completion is skipped by the generation check; a history jump's completion records the target `pending` already names, which `record` handles without duplicating or truncating. A current-generation completion with `is_playing` false calls `play_history.retry_pending()`: the pending jump stays in place (the backend left `current_track` naming its target), so the next Previous retries that target rather than walking past it. Update the `step_track` doc comment to describe the history-backed Previous behavior instead of "Previous ignores Shuffle".
- `src/ui/tests.rs`: replace `previous_with_shuffle_still_steps_backward` with behavior tests, and drive `Message::TrackPlayed` to exercise the recording path. Cover a shuffled Previous that retraces two recorded plays; two presses before the first jump commits (same `current`) that keep walking back; a failed jump whose target the backend left in `current_track` and whose next Previous retries it; a pending jump superseded by a new committed `current` that then steps from the last actually played track; a shuffled Previous with no usable history that falls back to the sequential row; a history entry absent from the visible list being skipped; and two guard tests. `track_played_ignores_a_failed_completion` and `track_played_ignores_a_superseded_completion` each seed `["song-1", "song-2"]` and report `song-3` without recording it, then assert `previous(&songs, None)` returns `"song-2"` — with a wrongly recorded `song-3` it would return `"song-3"`, so the assertion actually pins the guard instead of passing through `previous`'s current-track branch.
- `src/ui/transport.rs` tests: cover recording order and the repeated-current no-op; recording after a back step dropping the forward entries; the `HISTORY_LIMIT` bound and cursor shift; `previous` returning `None` with no history; walking backward on repeated calls and across a pending jump; a jump target's `record` landing on its entry without duplicating or truncating; a failed jump retrying its target once before walking further back; a superseded jump stepping from the last recorded play rather than skipping it; including the last recorded entry when `current` is a newer selection; skipping ids absent from the supplied `songs`; and returning `None` at the oldest entry without moving the cursor.
- `README.md`: in the Usage paragraph, say that with Shuffle on Previous retraces the tracks played before the current one. Keep the hunk to that sentence.
- `docs/invariants.md` has no transport-stepping section (its headings cover the browse hierarchy, browse reply freshness, and the Now Playing label), so this plan leaves it unchanged.
- When the feature run moves this entry to `PLANS-DONE.md`, write the done entry's approach from the shipped code — including `PendingJump` and `previous(songs, current)` — so it does not repeat the stale draft entry that described a different API.

**Files touched.** `src/ui/transport.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `PlayHistory::record` keeps chronological play order, treats a repeated current id as a no-op, drops forward entries after a back step when a new track is recorded, keeps the forward entries when a pending jump's own target records, and keeps at most `HISTORY_LIMIT` entries with the cursor following the oldest dropped entry.
- `PlayHistory::previous` returns the nearest earlier entry present in `songs`, walks further back on repeated calls including before a jump's play commits, skips entries from a list the player no longer shows, includes the last recorded entry when `current` is a newer unrecorded selection, and returns `None` without moving when no earlier usable entry exists.
- A jump whose play never lands is retried when the backend leaves its target in `current_track`: from history `["song-1", "song-2", "song-3"]`, a Previous from `song-3` selects `song-2`; a failed completion names `song-2` and the next Previous selects `song-2` again before walking on to `song-1`. A jump superseded by a different current track is abandoned: a Previous from `song-3` then a Previous from a new `song-4` selects `song-3`, not `song-1` or `song-2`.
- With Shuffle on, Previous selects the recorded previous track rather than the row before the current one; with no usable history it falls back to `previous_track_id`. With Shuffle off, Previous still follows `previous_track_id` and the shared Repeat flag.
- A current-generation `TrackPlayed` with `is_playing` true records the committed track; one with `is_playing` false marks the pending jump for retry and records nothing, while a superseded generation records nothing. The two guard tests distinguish a recorded `song-3` (`previous(.., None)` would return `"song-3"`) from the unrecorded expected `"song-2"`.
- The new tests named in the approach pass alongside the existing Next/Previous wiring tests, and the README documents the Shuffle-on Previous behavior.
- The implementation diff changes only the four files listed above for the feature itself (the `PLANS.md`/`PLANS-DONE.md` move aside), and the README hunk is limited to the Shuffle-on Previous sentence.

### Seek the playing preview five seconds with the left and right arrow keys (planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** Pressing Left seeks the playing preview back five seconds and pressing Right seeks it forward five seconds, matching classic Winamp's keyboard bindings. Today `shortcuts::shortcut` maps only ArrowUp/ArrowDown (to `Message::VolumeUp`/`VolumeDown`), `Message` has no seek variant, and `AudioOutput` exposes only play/pause/resume/stop/`set_volume`/`set_balance`. `rodio` 0.22's `Player` already provides `get_pos` and `try_seek`, but nothing in `src/audio.rs` calls them, so the left/right keys currently do nothing.

**Approach.**

- **Seek-time channel phase.** rodio 0.22.2's `Source::try_seek` implementations leave the next sample on the same channel as before the seek: `SamplesBuffer::try_seek` offsets the target by the current channel and `SymphoniaDecoder::try_seek` re-advances to the active channel. The wrappers' cursors must stay in that phase. `BalanceSource::try_seek` already only delegates, so leave it that way and say in its doc comment why it must not reset `channel`; `EqSource::try_seek` currently assigns `self.channel = 0` after seeking, so remove that assignment but keep `rebuild_filters()` so a seek still clears the filter ringing.
- `src/audio.rs`: add `fn seek_by(&self, offset_secs: i64) -> Result<(), AppleMusicError>` to `AudioOutput`, documented as a relative move (positive seeks forward, negative backward), a no-op when no player is loaded, and a non-blocking channel send like `set_volume`. Implement it for `SilentOutput` (log and return `Ok`), add `Command::SeekBy(i64)`, and add `fn seek_by(&self, offset_secs: i64)` to `WorkerPlayer`. Add a private pure `fn seek_target(position: Duration, offset_secs: i64) -> Duration` that saturating-adds or saturating-subtracts `Duration::from_secs(offset_secs.unsigned_abs())`, so a backward seek past the start lands at zero. `RodioPlayer::seek_by` reads `self.player.get_pos()`, calls `self.player.try_seek(seek_target(position, offset_secs))`, and logs a `try_seek` error with `eprintln!` without changing any state. `serve_commands` handles `Command::SeekBy` by calling `player.seek_by(offset_secs)` only when a player is installed; the offset is not remembered across players, so a seek while stopped does nothing.
- `src/test_support.rs`: add `AudioCall::SeekBy(i64)`, record it in `RecordingAudio::seek_by`, and return `Ok(())` from `FailingAudio::seek_by`.
- `src/apple_music.rs`: add `pub(crate) fn seek_output(&self, offset_secs: i64) -> Result<(), AppleMusicError>` beside `set_output_volume` and `set_output_balance`, forwarding to `self.audio.seek_by(offset_secs)` and touching no shared state.
- `src/apple_music/tests.rs`: implement `seek_by` for `GatedAudio` and `RejectingAudio`, recording `AudioCall::SeekBy` and succeeding (like their other non-rejected commands). Add `seek_output_forwards_to_the_audio_backend`, asserting one `AudioCall::SeekBy(-5)` and that `AppState` is unchanged.
- `src/ui/mod.rs`: add `SeekBackward` and `SeekForward` variants beside `VolumeUp`/`VolumeDown`, and add a private `seek_preview(player, offset_secs)` helper that calls `player.apple_music_service.seek_output(offset_secs)`, logs a backend error like `mutate_volume`, and returns `Task::none()`. Route the two new arms through it with `-views::SEEK_STEP_SECS` and `views::SEEK_STEP_SECS`.
- `src/ui/views/mod.rs`: add `pub(super) const SEEK_STEP_SECS: i64 = 5;` beside `VOLUME_STEP`, documenting Winamp's five-second seek step.
- `src/ui/shortcuts.rs`: map `Key::Named(key::Named::ArrowLeft)` to `Message::SeekBackward` and ArrowRight to `Message::SeekForward`, updating the `message_for`/`shortcut` doc lists. Extend `message_for_maps_the_arrow_keys` and add both arrows to `shortcut_ignores_chord_modifiers`.
- `src/ui/tests.rs`: add `seek_messages_drive_the_audio_backend`, driving both messages through `update` over a `RecordingAudio` and asserting one `AudioCall::SeekBy(-SEEK_STEP_SECS)` and one `SeekBy(SEEK_STEP_SECS)` with no scheduled task.
- `src/audio.rs` tests: extend `silent_output_reports_success_for_every_command`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` with `seek_by`; add `seek_target_moves_forward_and_backward` (10s +5 → 15s, 10s -5 → 5s) and `seek_target_saturates_at_the_start` (3s -5 → 0); extend the recording `WorkerPlayer` with `seek_by` and `transport_volume_and_balance_commands_drive_the_installed_player` with a `Command::SeekBy(-5)` before its `Stop`, asserting the installed player received it. Change the `SeekableSource` test double's `try_seek` to land on the target frame plus its current channel instead of resetting to a frame boundary, so it models rodio's contract. Rewrite `eq_source_seek_clears_filter_state` to seek from a frame boundary and assert the channel is unchanged, and add `eq_source_seek_keeps_the_channel_cursor` and `balance_source_seek_keeps_the_channel_cursor`, which seek from mid-frame and assert the next sample is still treated as the same channel.
- `README.md`: change the arrow-key sentence to say the up/down arrows nudge the volume and the left/right arrows seek five seconds back or forward.

**Files touched.** `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views/mod.rs`, `src/ui/shortcuts.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `seek_target` moves forward and backward by the requested seconds and clamps a backward seek past the start at zero.
- `RodioOutput::seek_by` sends `Command::SeekBy` carrying the same signed offset; `SilentOutput::seek_by` returns `Ok`; a closed worker reports "the audio thread is gone".
- `serve_commands` applies `SeekBy` only to an installed player; the recording player receives `-5` from the extended transport test.
- `AppleMusicService::seek_output` records exactly one `AudioCall::SeekBy` and leaves `AppState` unchanged.
- `message_for` maps ArrowLeft to `Message::SeekBackward` and ArrowRight to `Message::SeekForward`; Control, Alt, and Logo chords map neither.
- Driving both seek messages through `update` records exactly one `AudioCall::SeekBy(-5)` and one `AudioCall::SeekBy(5)` and schedules no task.
- The README documents the left/right five-second seek keys.
- `EqSource::try_seek` clears filter state without changing `channel`, and `BalanceSource::try_seek` leaves `channel` unchanged; a mid-frame seek through either wrapper keeps the next sample on the same channel, and the updated `SeekableSource` models that contract.
- The implementation diff against the pre-change commit changes only the nine files listed above for the feature itself (the move of this entry to `PLANS-DONE.md` aside), and the README hunk is limited to the arrow-key sentence.

### Apply the equalizer curve to the audio output (Equalizer audio, parts 1/2 and 2/2: planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** A preview starts through the equalizer's ten band filters at the curve the panel currently shows. Today `AppState` stores `eq_enabled`, `eq_preamp`, and `eq_bands`, and the equalizer update arms only write them; `AudioOutput` has no equalizer method, so every preview plays flat. This part adds the filter chain and applies the stored curve when a preview starts; part 2 forwards changes made while a preview is already playing.

**Approach.**

- `src/equalizer.rs`: add `pub const BAND_HZ: [f32; BAND_COUNT]` holding the numeric centres matching `BAND_FREQUENCIES` (`60.0` through `16_000.0`, with `1K` as `1_000.0`) and `pub struct BandFilter` with `new(sample_rate)`, `set_curve(enabled, preamp_db, bands_db)`, and `process(sample)`. Build each band as an RBJ peaking biquad at its `BAND_HZ` centre with a fixed `Q = 1.41` (roughly one octave), rebuild the coefficients in `set_curve`, and return the sample unchanged when disabled. When enabled, multiply by the preamp gain `10^(preamp_db / 20)` and run the sample through all ten biquads. Keep `Biquad` private to the module; `BandFilter` is the public type the audio source uses.
- `src/audio.rs`: add `pub struct EqSnapshot { pub enabled: bool, pub preamp_db: f32, pub bands_db: [f32; BAND_COUNT] }` (derive `Debug, Clone, Copy, PartialEq`) and private `EqSettings` holding those values in `AtomicBool`/`AtomicU32`s plus an `AtomicU64` version. Add `EqSource<S>` holding `inner`, `Arc<EqSettings>`, the last seen version, one `BandFilter` per channel from `inner.channels()`, and a channel cursor; on each `next`, rebuild every channel filter when the version changed, then process the sample through the current channel's filter. Delegate `current_span_len`, `channels`, `sample_rate`, `total_duration`, and `try_seek` like `BalanceSource`, resetting the filters on a successful seek. Add `set_equalizer(&self, equalizer: EqSnapshot)` to `AudioOutput`, `Command::SetEqualizer(EqSnapshot)`, and `WorkerPlayer::set_equalizer`; `SilentOutput` logs and returns `Ok`, `RodioPlayer` stores the snapshot in its `Arc<EqSettings>`, and `append` wraps each preview as `BalanceSource::new(EqSource::new(preview, eq), balance)` so equalization runs before the stereo balance. `serve_commands` remembers the snapshot across players and applies it to every newly created player, exactly as it remembers volume and balance.
- `src/test_support.rs`: add `AudioCall::SetEqualizer(EqSnapshot)`, record it in `RecordingAudio::set_equalizer`, and return `Ok(())` from `FailingAudio::set_equalizer`. Extend `assert_preview_calls` so a preview start expects `SetVolume(0.5)`, `SetBalance(0.0)`, `SetEqualizer(default)`, then `Play`.
- `src/apple_music.rs`: add `pub(crate) fn set_output_equalizer(&self, equalizer: EqSnapshot) -> Result<(), AppleMusicError>` forwarding to `self.audio.set_equalizer(equalizer)` and touching no shared state, like `set_output_volume`. In `play_track`, read `state.eq_enabled`, `state.eq_preamp()`, and `state.eq_bands()` under the state lock, then chain `set_volume(volume)`, `set_balance(balance)`, `set_equalizer(snapshot)`, and `play(url)` so the first preview starts at the stored curve rather than flat. Keep the existing rollback on a backend error.
- `src/audio.rs` tests: update `RecordingPlayer` to implement `set_equalizer`; add `a_new_preview_starts_at_the_remembered_equalizer` beside `a_new_preview_starts_at_the_remembered_balance`; extend `silent_output_reports_success_for_every_command`, `audio_with_fallback_falls_back_to_silence_when_open_fails`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` with the new method. Add `eq_source_disabled_passes_samples_through`, `eq_source_enabled_flat_curve_passes_samples_through`, `eq_source_preamp_scales_by_the_decibel_gain`, `eq_source_filters_each_channel_with_its_own_state`, and `eq_source_follows_the_curve_changed_while_playing`.
- `src/equalizer.rs` tests: add `band_frequencies_have_matching_numeric_centres`, `band_filter_boost_and_cut_at_the_center_frequency` (a generated 1 kHz sine through a `+12 dB` and a `-12 dB` 1 kHz band, comparing settled RMS), and `band_filter_preamp_is_a_flat_gain`.
- `src/apple_music/tests.rs`: add `set_output_equalizer_forwards_to_the_audio_backend` and update `play_track_starts_the_preview_through_the_audio_backend` and `assert_preview_calls` callers to expect the equalizer snapshot before `Play`.
- `README.md`: state in the usage paragraph that the equalizer panel's curve is applied to preview playback.

**Files touched.** `src/equalizer.rs`, `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- A disabled `BandFilter` returns each input sample unchanged; an enabled flat curve (zero preamp, zero bands) also passes samples through within floating-point error.
- A `+12 dB` 1 kHz band raises a settled 1 kHz sine's RMS, a `-12 dB` band lowers it, and a `+6.0206 dB` preamp produces approximately twice the input amplitude; each channel of a stereo source keeps its own filter state.
- `EqSource` reports its inner source's channel count, sample rate, span, duration, and seek result, and follows a curve changed through its shared `EqSettings` between samples.
- `RodioOutput::set_equalizer` sends `Command::SetEqualizer` with the same snapshot; `SilentOutput::set_equalizer` returns `Ok`; a closed worker reports "the audio thread is gone".
- `AppleMusicService::set_output_equalizer` records one `AudioCall::SetEqualizer` and leaves `AppState` unchanged; `play_track` records `SetVolume`, `SetBalance`, `SetEqualizer` carrying the state's current curve, then `Play`.

**Part 2/2 (done 2026-10-20).** Toggling the equalizer, dragging the preamp or a band slider, or picking a preset changes the sound of the preview already playing, without restarting it. Part 1 applies the stored curve when a preview starts; before this part the four equalizer update arms still called `mutate_state`, so a change made mid-track never reached the audio backend.

- `src/ui/mod.rs`: added `fn mutate_equalizer(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message>` beside `mutate_volume` and `mutate_balance`: it locks the state, applies the mutation, reads `eq_enabled`, `eq_preamp()`, and `eq_bands()`, builds an `EqSnapshot`, and calls `player.apple_music_service.set_output_equalizer(snapshot)`, logging a backend error with the same `eprintln!` shape as the volume and balance helpers. `ToggleEqualizer`, `EqPreampChange`, `EqBandChange`, and `EqPresetSelected` now route through it instead of `mutate_state`. Because `EqSource` reads its shared `EqSettings` between samples, the running preview picks up the new curve without a new `Play`.
- `src/ui/tests.rs`: added `equalizer_messages_forward_to_the_audio_backend`, driving `ToggleEqualizer`, `EqPreampChange(3.0)`, `EqBandChange(0, 4.0)`, out-of-range and `f32::NAN` band gains, and `EqPresetSelected(rock_preset())` through `update` over a `player_with_audio` built with a `RecordingAudio` fake, asserting the recorded `SetEqualizer` snapshots match the clamped `AppState` values and that no arm schedules a task. Also added `an_equalizer_backend_failure_still_commits_the_state`, which drives `EqBandChange` over a `FailingEqualizerAudio` that rejects `set_equalizer`, asserting the clamped band still lands in `AppState` and the backend saw the attempt.
- `src/test_support.rs`: added `FailingEqualizerAudio`, an `AudioOutput` whose `set_equalizer` records the attempt and returns an error, so the equalizer arm's backend-error branch is reachable from the UI suite.
- `README.md`: extended the equalizer sentence to say the toggles, sliders, and presets change the playing preview's curve live.

**Files touched.** `src/ui/mod.rs`, `src/ui/tests.rs`, `src/test_support.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- Each equalizer message records exactly one `SetEqualizer` call whose enabled flag, preamp, and ten band gains equal the clamped values now in `AppState`; no call restarts the preview.
- A backend error from `set_output_equalizer` is logged and the state mutation still commits, as with volume and balance.
- The README says equalizer changes apply to the playing preview.

### Add the R and S keyboard bindings for Repeat and Shuffle (planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** Pressing R toggles Repeat and pressing S toggles Shuffle from the keyboard, matching classic Winamp, so both transport toggles are reachable without the mouse. Today `shortcuts::shortcut` maps Z/X/C/V/B and the arrow keys only: R and S fall through to `None`, even though `Message::ToggleRepeat`, `Message::ToggleShuffle`, and their `update` arms already exist.

**Approach.**

- `src/ui/shortcuts.rs`: add two arms to `shortcut` beside the existing letter arms — `r` returns `Message::ToggleRepeat` and `s` returns `Message::ToggleShuffle` — using the same `eq_ignore_ascii_case` guard as the other letters, so uppercase and Shift work and the existing Control/Alt/Logo rejection applies. Update the doc comments on `message_for` and `shortcut` to list the two new bindings.
- `README.md`: extend the usage paragraph's classic-key sentence after the V entry with "R toggles Repeat and S toggles Shuffle".
- Tests in `src/ui/shortcuts.rs`: extend the `assert_letter_bindings` helper so the existing `message_for_maps_each_bound_letter` and `shortcut_matches_letters_case_insensitively` tests also assert that `r`/`R` map to `Message::ToggleRepeat` and `s`/`S` map to `Message::ToggleShuffle`; add both keys to the key list in `shortcut_ignores_chord_modifiers`, so a Control, Alt, or Logo chord still maps neither.

**Files touched.** `src/ui/shortcuts.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `message_for` maps a pressed `Key::Character("r")` to `Message::ToggleRepeat` and `Key::Character("s")` to `Message::ToggleShuffle`, with or without Shift, and maps their uppercase forms the same way.
- Control, Alt, and Logo chords over R or S still map to `None`, and the existing unbound-key and key-release behavior is unchanged.
- The README's classic-key sentence lists R and S beside the existing transport and volume keys.

### Back the playlists browse with the Apple Music REST endpoints (Playlists browse, part 2/2: planned 2026-10-20 by the plan role, requires part 1/2 landed, done 2026-10-20)

**Goal.** Point the part-1 playlist queries at the signed-in user's Apple Music library, so a stored session shows that user's real playlists and each playlist's real tracks instead of the sample data, while a session-less service keeps its sample fallback. This is the second half of the playlist browse, the twin of "Back the library search with the Apple Music REST endpoint" (done); land it only once the Playlists view from part 1/2 is the running build.

**Approach.**

- `src/apple_music/rest.rs`: add `Playlist` to the `use crate::library::{Album, Artist, Song}` import and the module doc's model list, then add `pub fn get_all_playlists(&self, session: &MusicKitSession) -> Result<Vec<Playlist>, AppleMusicError>` requesting `{API_BASE}/me/library/playlists` through `self.fetch` and mapping each resource with `Resource::into_playlist`; add `pub fn get_songs_from_playlist(&self, session: &MusicKitSession, playlist_id: &str) -> Result<Vec<Song>, AppleMusicError>` requesting `{API_BASE}/me/library/playlists/{}/tracks` with `encode_path_segment(playlist_id)` and mapping with `Resource::into_song("")` — a library playlist track carries no album id, the same shape `get_all_songs` gives its songs. Add `Resource::into_playlist`, which moves the resource's `id` and `attributes.name` into the model through `required_id(id, "playlist")` and `required_name(…, &id, "playlist")`, exactly as `into_album` does for albums. Document the endpoint, the empty `album_id`, and the id encoding on both methods.
- `src/apple_music.rs`: add `Playlist` to the `use crate::library::{Album, Artist, Song}` import, rewrite `get_playlists` as `self.browse(|rest, session| rest.get_all_playlists(session), sample_all_playlists)` and `get_songs_from_playlist` as `self.browse(move |rest, session| rest.get_songs_from_playlist(session, &id), || lookup(&sample_library().songs_by_playlist, playlist_id))` after the existing `ensure_id_is_valid` guard, keeping the guard's error contract. Add `fn sample_all_playlists() -> Vec<Playlist>` cloning `sample_library().playlists`, and remove the part-1 doc caveat that the queries ignore a stored session.
- `README.md`: remove the "playlist data comes from the sample library until the REST endpoints land" caveat, leaving playlists described alongside the other signed-in browse queries.
- Tests: `src/apple_music/rest/tests.rs` gains `get_all_playlists_maps_the_library_playlists_collection` (pin `/v1/me/library/playlists` and the mapped id/name), `get_all_playlists_follows_a_next_page`, `get_all_playlists_reports_a_transport_error_bare`, `get_all_playlists_rejects_a_nameless_playlist`, `get_songs_from_playlist_maps_the_playlist_tracks_collection` (pin the id-scoped path and empty `album_id`), `get_songs_from_playlist_percent_encodes_the_playlist_id`, and `get_songs_from_playlist_reports_a_transport_error_bare`. `src/apple_music/tests.rs` adds `get_playlists_uses_the_rest_library_when_signed_in` and `get_songs_from_playlist_uses_the_rest_library_when_signed_in` over the stub transport, and keeps the existing no-session sample tests green (updating only the two part-1 tests that pinned the sample-only signed-in behavior).

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/sample_library.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary::get_all_playlists` requests `/v1/me/library/playlists`, maps each resource's id and name, and follows a same-origin `next` page; a nameless playlist is an error naming its id and a transport failure propagates its cause unchanged.
- `RestLibrary::get_songs_from_playlist` requests `/v1/me/library/playlists/{encoded-id}/tracks`, maps songs with an empty `album_id`, percent-encodes a reserved character in the id, and propagates a transport failure.
- With a stored session, `AppleMusicService::get_playlists` and `get_songs_from_playlist` return the REST results; with no session they return the sample playlists and playlist songs in the part-1 order.
- A blank, whitespace-only, or control-character playlist id is still rejected by the service before any REST call is issued.

### Add a Playlists browse level backed by the sample library (Playlists browse, part 1/2: planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** A `Playlists` button on the Artists view opens a flat list of the user's library playlists; pressing a playlist opens its songs in the existing Songs view, and those rows play exactly like any other song row. Back from the songs returns to Playlists, and Back from Playlists returns to Artists. The Playlists level is a new top-level browse sibling of the All Songs and All Albums flat lists. This first half answers the two new service queries from the sample library, the same sample-first split "Add a library search box to the playlist editor" (done) used before its REST sibling landed; the sibling "Back the playlists browse with the Apple Music REST endpoints" (Playlists browse, part 2/2) points the same two methods at the signed-in library. Until that sibling lands, a signed-in service also answers playlists from the sample library — documented in the method docs, not left implicit.

**Approach.**

- `src/library.rs`: add `pub struct Playlist { pub id: String, pub name: String }`, deriving `Debug, Clone, PartialEq, Serialize, Deserialize` like `Artist`, and extend the module doc's model list. Add tests mirroring the `Artist` pins: a hand-written `playlist_payload()` (`{"id":"playlist-1","name":"Morning Drive"}`), serialized-field-name/round-trip, unknown-field tolerance, and `assert_every_field_required`.
- `src/test_support.rs`: add `sample_playlist()` beside `sample_artist`/`sample_album`/`sample_song`, carrying id `playlist-1` and name `Morning Drive`, for the `library` and `apple_music` suites.
- `src/sample_library.rs`: add `pub playlists: Vec<Playlist>` and `pub songs_by_playlist: HashMap<String, Vec<Song>>` to `SampleLibrary`. Build three playlists in render order — `playlist-1` "Morning Drive", `playlist-2` "Focus", `playlist-3` "Sample Favorites" — and index their songs from the existing sample songs, preserving each playlist's named order (`playlist-1` → `song-1`, `song-4`; `playlist-2` → `song-2`, `song-5`; `playlist-3` → `song-1`, `song-2`, `song-3`). Add tests mirroring `albums_are_indexed_by_artist_in_library_order_with_no_empty_groups`: playlists in order, three non-empty groups, and each group's song ids in the documented order. Update the module doc's browse-query list.
- `src/apple_music.rs`: add `pub async fn get_playlists(&self) -> Result<Vec<Playlist>, AppleMusicError>` returning `sample_library().playlists.clone()`, and `pub async fn get_songs_from_playlist(&self, playlist_id: &str) -> Result<Vec<Song>, AppleMusicError>` calling `ensure_id_is_valid(playlist_id, IdKind::Playlist)` then `lookup(&sample_library().songs_by_playlist, playlist_id)`. Add `IdKind::Playlist` and its `"playlist"` `Display` arm. Document on both methods that they answer from the sample library even when a session is stored, and that the REST sibling rewires them through `browse`; the async signatures and id guard already match that end state.
- `src/ui/mod.rs`: add `Playlist` to the `use crate::library::{Album, Artist, Song}` import, `CurrentView::Playlists`, `Message::ShowPlaylists`, `Message::PlaylistsLoaded(Vec<Playlist>)`, `Message::PlaylistsLoadFailed(String)`, and `Message::PlaylistSelected { epoch, index }`; add `playlists: BrowseList<Playlist>` to `WinampPlayer`, built with `BrowseList::new(false)` in `WinampPlayer::new`; add `SongsOrigin::Playlist`. The `ShowPlaylists` arm clears `search_active`, sets `CurrentView::Playlists`, and calls `fetch_level` on `player.playlists` with report `"loading library playlists"` and `|service| async move { service.get_playlists().await }`. The `PlaylistSelected` arm mirrors `AlbumSelected`: resolve through `player.playlists.select(epoch, index)`, clone the id, set `songs_origin = SongsOrigin::Playlist`, switch to `CurrentView::Songs`, clear any active search, and `fetch_level` the songs with `"loading songs from playlist {playlist_id:?}"` and `get_songs_from_playlist`. The `Back` arm's `CurrentView::Songs` match adds `SongsOrigin::Playlist => CurrentView::Playlists`, and `CurrentView::Playlists => CurrentView::Artists`. Store/fail through `player.playlists` in the loaded/failed arms, and map the two new reply messages to `player.playlists.is_current(issued)` in the `BrowseReply` arm.
- `src/ui/views/browse.rs`: add `playlist_row` (name, `"View Songs"`, `Message::PlaylistSelected`, never current) and `view_playlists` built through `scrollable_list` with the `CurrentView::Playlists` placeholder from `browse_placeholder`; add `view_playlists_button` returning `labeled_button("Playlists", Message::ShowPlaylists)`; add `CurrentView::Playlists => "No playlists"` to `empty_list_label` and `Playlists` to `BROWSE_VIEWS` (six entries).
- `src/ui/views/mod.rs`: re-export `view_playlists` and `view_playlists_button`.
- `src/ui/mod.rs`'s `view`: add the `CurrentView::Playlists` browse arm rendering `views::view_playlists(&player.playlists.items, player.playlists.epoch, player.playlists.loading, player.playlists.error.as_deref())`, and push `views::view_playlists_button()` beside the All Songs and All Albums buttons on the Artists view.
- `docs/invariants.md`: add `Playlists → Artists` and `Songs → Playlists when the songs were opened from a playlist` to the Browse hierarchy guarantee, and name `back_from_playlists_returns_to_the_artists_list` and `playlist_selected_in_playlists_loads_its_songs_and_back_returns_to_playlists` under "Pinned by".
- `README.md`: in the usage paragraph, describe the `Playlists` button and its Back path, and state that playlist data comes from the sample library until the REST endpoints land (the sibling plan removes that caveat).
- Tests: `src/apple_music/tests.rs` gains `get_playlists_returns_the_sample_playlists_in_order`, `get_songs_from_playlist_returns_its_sample_songs_in_order`, `get_songs_from_playlist_rejects_a_blank_id` (empty, whitespace-only, control character), and `get_songs_from_playlist_returns_empty_for_an_unknown_playlist`; the session variants pin the documented sample-only behavior so the sibling's change is deliberate. `src/ui/tests.rs` gains `show_playlists_fetches_and_switches_view`, `playlists_loaded_stores_the_list`, `playlist_selected_in_playlists_loads_its_songs_and_back_returns_to_playlists`, `back_from_playlists_returns_to_the_artists_list`, and `show_playlists_clears_an_active_search`, and extends `seed_browse_lists`, `assert_browse_lists_empty`, `each_browse_failure_stores_its_report_in_its_own_list`, and `view_constructs_over_the_apps_full_input_space` to cover `playlists`. `src/ui/views/tests.rs` pins `playlist_row`, `view_playlists_button`, the `empty_list_label` arm, and the extended `BROWSE_VIEWS` loops.

**Files touched.** `src/library.rs`, `src/test_support.rs`, `src/sample_library.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `Playlist` serializes as exactly `{"id", "name"}`, round-trips, tolerates an unknown field, and rejects a payload missing either field.
- The sample library exposes `playlist-1`/`playlist-2`/`playlist-3` in that order with the documented song ids in each `songs_by_playlist` group; no group is empty.
- `AppleMusicService::get_playlists` returns those three playlists in order with no session and with a session; `get_songs_from_playlist("playlist-1")` returns `song-1` then `song-4`; an unknown id returns an empty list; a blank, whitespace-only, or control-character id returns an error naming `playlist`.
- Pressing `Playlists` clears an active search, switches to `CurrentView::Playlists`, and fetches into `playlists`; a loaded reply stores the rows and a failed reply records the report on `playlists` alone.
- Pressing a playlist row fetches that playlist's songs into `songs` and shows `CurrentView::Songs`; Back returns to `CurrentView::Playlists`, while Back from playlist songs reached through any other origin still returns to Albums or All Albums as before.
- A stale playlist press (wrong epoch or out-of-range index) issues no fetch and leaves the view unchanged.
- The Artists view shows the `Playlists` button, and all six browse levels assemble, including empty and failed `playlists`, without panicking.

### Apply the balance slider to the audio output (planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** Moving the balance slider changes the stereo image of the playing preview, and a preview starts at the slider's balance. Today `Message::BalanceChange` only writes `AppState::balance`: `AudioOutput` has no balance method, so `mutate_state` never forwards the value and every preview plays centered. This is the balance twin of "Drive the volume slider through the audio backend" (done), whose approach deliberately left `BalanceChange` alone because rodio's `Player` has no panner. A small `Source` wrapper supplies the missing panner without a new dependency.

**Approach.**

- `src/audio.rs`:
  - Add `set_balance(&self, balance: f32) -> Result<(), AppleMusicError>` to `AudioOutput` (doc: `-1.0` is hard left, `0.0` centered, `1.0` hard right; it affects the current player and is remembered for the next `play`). `SilentOutput::set_balance` logs the value and returns `Ok`.
  - Add `Command::SetBalance(f32)`; `RodioOutput::set_balance` sends it.
  - Add a private `BalanceSource<S: Source>` adapter that wraps the decoded preview and holds the balance as an `Arc<AtomicU32>` of `f32` bits, read on every sample so a slider drag changes the current preview without restarting it. Delegate `current_span_len`, `channels`, `sample_rate`, `total_duration`, and `try_seek` to the inner source. Gain per channel: for a mono source `1.0 - balance.abs()`; otherwise even channel indices use `1.0 - balance.max(0.0)` (left) and odd indices use `1.0 + balance.min(0.0)` (right), so `-1.0` silences the right channel, `+1.0` the left, and `0.0` is unity. Do not use rodio's `ChannelVolume`/`Spatial`: both downmix the input to mono, which would collapse the preview's stereo image.
  - Add a private `RodioPlayer` struct holding the `rodio::Player` and its balance `Arc<AtomicU32>`; implement `WorkerPlayer` for it with `Preview = rodio::Decoder<Cursor<Vec<u8>>>`, appending the preview wrapped in `BalanceSource`, and add `set_balance` to the `WorkerPlayer` trait (the recording fake records it). `run_worker` builds a `RodioPlayer` instead of a bare `rodio::Player`.
  - In `serve_commands`, remember the balance (initial `0.0`) across players, map `Command::SetBalance` to storing it and calling `set_balance` on the current player when one exists, and call `set_balance(balance)` on a newly created player before `append`, as `set_volume` already does. Update the worker tests: the recording fake records `set_balance`, and a test pins that a balance set while stopped applies to the next player and one set while playing reaches the current player.
  - Add unit tests for `BalanceSource` using a two-channel `SamplesBuffer` with distinct left/right constants: `-1.0` keeps left and zeroes right, `+1.0` the reverse, `0.0` is unchanged, and changing the shared atomic between `next()` calls changes the following samples (the live-update contract).
  - Extend `silent_output_reports_success_for_every_command`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` to cover `set_balance`.
- `src/test_support.rs`: add `AudioCall::SetBalance(f32)` and record it in `RecordingAudio::set_balance`; `FailingAudio::set_balance` returns `Ok(())` like its volume twin. Extend `assert_preview_calls` (and its callers) so a preview start expects `SetVolume`, then `SetBalance`, then `Play`.
- `src/apple_music.rs`: add `pub(crate) fn set_output_balance(&self, balance: f32) -> Result<(), AppleMusicError>` forwarding to `self.audio.set_balance(balance)` and touching no shared state, exactly as `set_output_volume` does. In `play_track`, capture `state.balance()` before dropping the lock and apply it with the volume before `play`, so the first preview starts at the slider's balance.
- `src/ui/mod.rs`: add `mutate_balance(player, mutation) -> Task<Message>` beside `mutate_volume`: apply the mutation under one synchronous `blocking_lock`, read back the clamped `state.balance()`, forward it to `player.apple_music_service.set_output_balance(...)`, and log a backend error. Replace the `Message::BalanceChange` arm's `mutate_state` call with it and drop `BalanceChange` from `mutate_state`'s doc list.
- `README.md`: state in the usage paragraph that the balance slider sets the audio backend's stereo balance, beside the existing volume sentence.
- Tests: `src/apple_music/tests.rs` gains `set_output_balance_forwards_to_the_audio_backend` (one `AudioCall::SetBalance`, `AppState::balance` unchanged) and updates `play_track_starts_the_preview_through_the_audio_backend` to expect the balance call. `src/ui/tests.rs` gains `balance_messages_forward_to_the_audio_backend`, driving `Message::BalanceChange` through `update` over a `player_with_audio(RecordingAudio)` and asserting the recorded balance and the clamped `AppState::balance`, including an out-of-range value and `f32::NAN` (which stores and forwards `0.0`).

**Files touched.** `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `BalanceSource` at `-1.0` leaves the left channel unchanged and zeroes the right; at `+1.0` the reverse; at `0.0` both unchanged; a mono source is attenuated by `1.0 - balance.abs()`; mutating the shared balance between `next()` calls changes later samples without rebuilding the source.
- `RodioOutput::set_balance(0.4)` sends `Command::SetBalance(0.4)`; `SilentOutput::set_balance` returns `Ok`; a closed worker reports "the audio thread is gone" from `set_balance`.
- `serve_commands` applies a remembered balance to the player a later `Play` creates and to the current player on `SetBalance`.
- `AppleMusicService::set_output_balance(-0.4)` records one `AudioCall::SetBalance(-0.4)` and leaves `AppState::balance` unchanged; `play_track` records `SetVolume`, then `SetBalance` carrying the state's current balance, then `Play`.
- Driving `Message::BalanceChange(-0.4)` stores `-0.4` in `AppState::balance` and records `SetBalance(-0.4)`; an out-of-range value stores and forwards the clamped end, and `NaN` stores and forwards `0.0`.

### Add docs/invariants.md pinning browse and Now Playing guarantees to their tests (planned 2026-10-20 by the steward role as a gap promotion for `unclear-invariant`, done 2026-10-20)

**Goal.** Create `docs/invariants.md`, the record the validation-gap policy tells every role to read and update but which the repository does not have. Three Fixed entries carry the `unclear-invariant` tag because the guarantee had to be reconstructed before the bug could be confirmed: "Browse view can only move forward — no way back from Albums/Songs, a dead end", "A browse reply is stored after a newer request was issued, so a navigation shows the previous level's rows", and "Now Playing bar shows the track id instead of the song title". The document states each guarantee, names the code that enforces it, and names the tests that pin it, so a later bug in these areas reads the invariant instead of reconstructing it. No behavior changes.

**Approach.**

- Create `docs/invariants.md` with a short header explaining the file: one section per subsystem, each stating the guarantee, the enforcing code, and the tests that fail if it is removed; a change to a guarantee updates its section in the same commit.
- `## Browse hierarchy`: every browse level above Artists has a Back action that steps one level up, Artists is the top level, and Back from an active search leaves the search for the Artists list. Enforced by the `Message::Back` arm in `src/ui/mod.rs` and `views::can_go_back` in `src/ui/views/browse.rs`. Pinned by `back_from_albums_returns_to_artists`, `back_from_songs_returns_to_albums`, `back_from_artists_is_a_noop`, `back_from_all_songs_returns_to_the_artists_list`, `back_from_all_albums_returns_to_the_artists_list`, `back_leaves_an_active_search_for_the_artists_list`, and `back_button_is_available_only_below_the_artist_list`.
- `## Browse reply freshness`: a fetched reply is stored only when no newer request was issued for that list. `RequestGeneration::is_current` drops it on iced's executor thread and `BrowseList::is_current` re-checks at the store on the UI thread; a selection also carries its list's epoch so a press that outlived its list is ignored. Enforced by `RequestGeneration` in `src/ui/loading.rs` and `BrowseList::is_current`, `select`, and `store` in `src/ui/browse.rs`. Pinned by `a_slow_stale_browse_reply_does_not_overwrite_the_newer_list`, `a_slow_stale_browse_failure_does_not_overwrite_the_newer_list_or_error`, `a_reply_superseded_between_the_mapper_and_update_is_not_stored`, `a_stale_songs_reply_superseded_before_update_is_not_stored`, `a_stale_albums_reply_superseded_before_update_is_not_stored`, and `selection_messages_with_a_stale_epoch_or_index_do_nothing`.
- `## Now Playing label`: the bar shows the known current track's human-readable title and artist, never the internal id when the id is in the known-track index; an unknown id falls back to the id, no current track shows "Nothing", and Stop keeps the interrupted track's title. Enforced by `now_playing_label` and `now_playing_artist` in `src/ui/views/now_playing.rs`. Pinned by `now_playing_label_resolves_known_track_to_title`, `now_playing_label_falls_back_to_id_when_track_not_in_tracks`, `now_playing_label_shows_nothing_with_no_current_track`, `now_playing_artist_resolves_known_track_to_artist`, `now_playing_label_keeps_the_track_name_after_browsing_to_another_album`, `now_playing_label_borrows_the_title_and_nothing_literal`, and `now_playing_artist_borrows_the_artist`.
- Each section cites the Fixed entry it came from (title and `BUGS-FIXED.md`) so the reconstruction history stays traceable.

**Files touched.** `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes; the change is documentation only and touches no source.
- `docs/invariants.md` exists with the three sections above, each naming its enforcing code and at least one test.
- Every cited test name is a real `fn` in the named file, checked with `make test-one TEST=<name>` (or by `grep` before running it).
- The browse section records a Back path from every level above Artists; the freshness section records both the executor-thread check and the store-time re-check; the Now Playing section records the unknown-id fallback and the "Nothing" contract.

### Add an All Albums view listing the whole library flat (planned 2026-10-20 by the plan role, done 2026-10-20)

**Goal.** A new top-level `All Albums` browse view lists every album in the user's library in one flat list, in library order. Pressing an album opens its songs exactly as the per-artist Albums view does, and Back from those songs returns to All Albums rather than to the artist list. It is reached by an `All Albums` button on the Artists view, beside `All Songs`, and Back from All Albums returns to Artists. Today albums are reachable only by drilling Artists → Albums, so there is no whole-library album view. The REST half reads `/v1/me/library/albums` through the same paginated collection helper as the other browse queries; the sample half walks the sample library's artists → albums in library order. Like `get_all_songs`, an all-albums resource has no artist id available in the collection mapping, so `Album::artist_id` is left empty; nothing that opens an all-albums row reads it (`get_songs_from_album` uses only the album id). Independent of the All Songs and search work, so it can land on its own.

**Approach.**

- `src/apple_music/rest.rs`:
  - Add `pub fn get_all_albums(&self, session: &MusicKitSession) -> Result<Vec<Album>, AppleMusicError>` requesting `{API_BASE}/me/library/albums` through `self.fetch`. Map each resource with `Resource::into_album("")` (the same consuming mapping `get_albums_by_artist` uses, which validates the album's id and name and leaves `artist_id` empty); document why the artist id is empty, as `get_all_songs` documents its empty `album_id`.
- `src/apple_music.rs`:
  - Add `pub async fn get_all_albums(&self) -> Result<Vec<Album>, AppleMusicError>` through the existing `browse` helper: `self.browse(|rest, session| rest.get_all_albums(session), sample_all_albums)`.
  - Add a private `fn sample_albums_in_library_order() -> impl Iterator<Item = &'static Album>` walking `sample_library().artists` then `albums_by_artist`, and make `sample_songs_in_library_order` start from that same album walk so the two orders cannot drift. Add `fn sample_all_albums() -> Vec<Album>` from the album walk.
- `src/ui/mod.rs`:
  - Add `CurrentView::AllAlbums`, `Message::ShowAllAlbums`, `Message::AllAlbumsLoaded(Vec<Album>)`, and `Message::AllAlbumsLoadFailed(String)`.
  - Add `all_albums: BrowseList<Album>` to `WinampPlayer`, initialised `BrowseList::new(false)` in `WinampPlayer::new`.
  - Add a `SongsOrigin { Album, AllAlbums }` enum and a `songs_origin: SongsOrigin` field initialised to `SongsOrigin::Album`. The `AlbumSelected` arm must resolve the pressed row against `player.all_albums` when `player.current_view == CurrentView::AllAlbums` and against `player.albums` otherwise, clone the selected album id, then set `songs_origin` from that same view before moving to `CurrentView::Songs`. The `Back` arm's `CurrentView::Songs` match uses `songs_origin` to return to `AllAlbums` or `Albums`; `AllAlbums` on Back returns to `Artists`.
  - `Message::ShowAllAlbums` arm: clear `search_active`, set `current_view = CurrentView::AllAlbums`, and call `fetch_level` on `player.all_albums` with the report `"loading all library albums"`, `|service| async move { service.get_all_albums().await }`, and the two new messages.
  - `AllAlbumsLoaded`/`AllAlbumsLoadFailed` store/fail through `player.all_albums`; the `BrowseReply` arm re-checks `player.all_albums.is_current(issued)` beside the existing lists.
  - `view`'s browse match gains a `CurrentView::AllAlbums` arm rendering `views::view_all_albums(&player.all_albums.items, …)` (no current-track argument, like the Albums arm), and pushes `views::view_all_albums_button()` on the Artists view beside `view_all_songs_button`.
- `src/ui/views/browse.rs`:
  - Add `pub fn view_all_albums(albums, epoch, loading, error)` built by `album_list` from `album_row` with the `CurrentView::AllAlbums` placeholder, and `pub fn view_all_albums_button()` returning `labeled_button("All Albums", Message::ShowAllAlbums)`.
  - Add `CurrentView::AllAlbums => "No albums"` to `empty_list_label` and `CurrentView::AllAlbums` to `BROWSE_VIEWS`.
- Tests:
  - `src/apple_music/rest/tests.rs`: `get_all_albums_maps_the_library_albums_collection` (pin `/v1/me/library/albums`, the mapped id/title, and the empty `artist_id`); `get_all_albums_follows_a_next_page`; `get_all_albums_reports_a_transport_error_bare`; `get_all_albums_rejects_a_nameless_album`.
  - `src/apple_music/tests.rs`: `get_all_albums_returns_every_sample_album_in_library_order` (album-1, album-2, album-3); `get_all_albums_uses_the_rest_library_when_signed_in`.
  - `src/ui/tests.rs`: `show_all_albums_fetches_the_flat_list_and_switches_view`; `all_albums_loaded_stores_the_flat_list`; `back_from_all_albums_returns_to_the_artists_list`; `show_all_albums_clears_an_active_search`; `album_selected_in_all_albums_loads_its_songs_and_back_returns_to_all_albums` (proves the press resolves against `all_albums` and the Back origin). Extend `seed_browse_lists`, `assert_browse_lists_empty`, `each_browse_failure_stores_its_report_in_its_own_list`, and `view_constructs_over_the_apps_full_input_space`'s failure shape to cover `all_albums`.
  - `src/ui/views/tests.rs`'s suite: a `view_all_albums_button` construction check, the `empty_list_label` arm, and the extended `BROWSE_VIEWS` loops.

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views/browse.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary::get_all_albums` requests `/v1/me/library/albums`, maps each resource's id and title, leaves `artist_id` empty, and follows a same-origin `next` page; a transport failure propagates its cause and a nameless album is an error naming its id.
- With no session, `AppleMusicService::get_all_albums` returns every sample-library album in library order (album-1, album-2, album-3); with a session it returns the REST collection.
- Pressing `All Albums` on the Artists view clears the buffer, clears an active search, switches to `CurrentView::AllAlbums`, and fetches; the loaded reply stores rows and a failed reply records the report on `all_albums` alone.
- Back from `CurrentView::AllAlbums` returns to `CurrentView::Artists`.
- Pressing an album row in `CurrentView::AllAlbums` fetches that album's songs into `songs` and shows `CurrentView::Songs`; Back then returns to `CurrentView::AllAlbums`, while Back from songs opened through the per-artist Albums view still returns to `CurrentView::Albums`.
- The Artists view shows the `All Albums` button and the view assembles every browse level, including an empty/failed `all_albums`, without panicking.

### Add an All Songs view listing the whole library flat (found 2026-10-09, done 2026-10-09)

**Goal.** A new top-level `All Songs` browse view lists every song in the user's Apple Music library in one flat list, in library order, and each row plays exactly like a browsed song. It is reached by an `All Songs` button on the Artists view, and Back returns there. Today the only way to reach a song is to drill Artists → Albums → Songs, and the search box returns only query matches, so there is no way to see the library's songs as a whole. The REST half reads the library-songs collection (`/v1/me/library/songs`, which follows `next` pages like the sibling collections) and the sample half lists the whole sample library, through the same signed-in/sample split `browse` already applies. It is independent of the existing search box and can land on its own.

**Approach.**

- `src/apple_music/rest.rs`:
  - Add `pub fn get_all_songs(&self, session: &MusicKitSession) -> Result<Vec<Song>, AppleMusicError>`. Build `{API_BASE}/me/library/songs` and call `self.fetch` — the same paginated collection helper `get_favorite_artists` and `get_albums_by_artist` use, so a library larger than one page is read in full under one `REQUEST_TIMEOUT`. Map each `Resource` exactly as `search_library` does (`Resource::into_song("")`, which moves the id, title, artist, and preview URL out of the parsed resource and validates its id and name) with `album_id: String::new()`, because a library-songs resource carries no album id and nothing that plays a song reads it.
- `src/apple_music.rs`:
  - Add `pub async fn get_all_songs(&self) -> Result<Vec<Song>, AppleMusicError>` answering through the existing `browse` helper: `self.browse(move |rest, session| rest.get_all_songs(session), sample_all_songs)`.
  - Extract the ordered library walk `sample_songs_matching` already spells out into a private `fn sample_songs_in_library_order() -> impl Iterator<Item = &'static Song>` (artists → `albums_by_artist` → `songs_by_album`, in library order), so the new `fn sample_all_songs() -> Vec<Song>` (`sample_songs_in_library_order().cloned().collect()`) and `sample_songs_matching` (the same walk, filtered) cannot drift apart.
- `src/ui/mod.rs`:
  - `Message::TrackSelected` resolves against `all_songs` when `current_view` is `AllSongs` and against `songs` otherwise, and `step_track` steps the same view's list, so a flat row plays — and Next/Previous move through — the flat list rather than the stale browsed songs.
  - Add `CurrentView::AllSongs`, `Message::ShowAllSongs`, `Message::AllSongsLoaded(Vec<Song>)`, and `Message::AllSongsLoadFailed(String)`.
  - Add `all_songs: BrowseList<Song>` to `WinampPlayer`, initialised `BrowseList::new(false)` in `WinampPlayer::new`.
  - `Message::ShowAllSongs` arm: clear `search_active`, set `current_view = CurrentView::AllSongs`, and issue `fetch_level(&mut player.all_songs, …, "loading all library songs".to_string(), |service| async move { service.get_all_songs().await }, Message::AllSongsLoaded, Message::AllSongsLoadFailed)`.
  - `AllSongsLoaded` stores via `player.all_songs.store`; `AllSongsLoadFailed` records via `player.all_songs.fail`; the `BrowseReply` arm re-checks `player.all_songs.is_current(issued)` for both, beside the existing three lists.
  - `Back` arm: `CurrentView::AllSongs => CurrentView::Artists`.
  - `view`'s browse match: `CurrentView::AllSongs` builds `views::view_all_songs` from `player.all_songs`, mirroring the `Songs` arm (including the second lock that hands the current track id to the view).
  - Push `views::view_all_songs_button()` in `view` when `current_view` is `CurrentView::Artists`, beside the Back/Retry buttons.
- `src/ui/views.rs`:
  - Add `pub fn view_all_songs(songs, epoch, loading, error, current_track)` that calls `song_list` from `song_row` with the `CurrentView::AllSongs` placeholder, like `view_songs`.
  - Add `pub fn view_all_songs_button()` returning `labeled_button("All Songs", Message::ShowAllSongs)`.
  - `empty_list_label`: add `CurrentView::AllSongs => "No songs"`.
  - Add `CurrentView::AllSongs` to `BROWSE_VIEWS` so the existing per-view construction and placeholder loops exercise it.
- Tests:
  - `src/apple_music/rest/tests.rs`: `get_all_songs_maps_the_library_songs_collection` (pin the requested URL, the mapped songs, and the empty `album_id`); `get_all_songs_follows_a_next_page`; `get_all_songs_reports_a_transport_error_bare`; `get_all_songs_rejects_a_nameless_song`.
  - `src/apple_music/tests.rs`: `get_all_songs_returns_every_sample_song_in_library_order` (song-1 through song-5); `get_all_songs_reads_the_rest_library_when_signed_in` (store a session over a stub transport and assert the REST songs, not the sample list).
  - `src/ui/tests.rs`: `show_all_songs_fetches_the_flat_list_and_switches_view`; `all_songs_loaded_stores_the_flat_list`; `back_from_all_songs_returns_to_the_artists_list`; `show_all_songs_clears_an_active_search`. Extend `seed_browse_lists` to populate `all_songs` and `assert_browse_lists_empty` to check it, so the shared full-input and failure-shape tests cover the new buffer.
  - `src/ui/views.rs`'s suite: a `view_all_songs_button` construction check and the `empty_list_label` arm, plus the extended `BROWSE_VIEWS` loops.

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views.rs`, `src/ui/tests.rs`.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary::get_all_songs` requests `/v1/me/library/songs`, maps every resource's id, title, artist, duration, and first non-blank preview URL, leaves `album_id` empty, and follows a same-origin `next` page.
- A transport failure propagates as the transport's own cause; a resource without a name is an error naming its id.
- With no session, `AppleMusicService::get_all_songs` returns every sample-library song in library order (song-1, song-2, song-3, song-4, song-5); with a session, it returns the REST collection.
- Pressing `All Songs` on the Artists view clears the buffer, switches to `CurrentView::AllSongs`, and fetches; the loaded reply stores the rows and a failed reply records the report.
- Back from `CurrentView::AllSongs` returns to `CurrentView::Artists`.
- Selecting a row in `CurrentView::AllSongs` plays the flat list's song, and Next/Previous step the flat list rather than the browsed `songs` buffer.

### Back the library search with the Apple Music REST endpoint (found 2026-10-09, done 2026-10-09)

**Goal.** `AppleMusicService::search_songs`, added by the sibling plan "Add a library search box to the playlist editor" (found 2026-10-09), queries the signed-in user's Apple Music library through the REST client and falls back to the sample-library filter when no session is stored — the same signed-in/sample split every browse query already uses. Land this only once the search box is the running build.

**Approach.**

- `src/apple_music/rest.rs`:
  - Add `pub fn search_library(&self, session: &MusicKitSession, query: &str) -> Result<Vec<Song>, AppleMusicError>`. Build `{API_BASE}/me/library/search?term={encode_path_segment(query)}&types=library-songs&limit=25` — Apple documents the library-search `limit` maximum as 25 (default 5), so the request asks for the documented maximum rather than the out-of-contract 100 this entry first named — and parse the search envelope `{"results":{"library-songs":{"data":[...],"next":...}}}` with new private `SearchEnvelope` / `SearchResults` / `SearchCollection` structs (the `library-songs` key renamed in serde). Apple omits `results.library-songs` for a search that matched nothing, so the field is an `Option` and an absent key maps to an empty list rather than a parse error. Map each `Resource` exactly as `get_songs_from_album` does, but with `album_id: String::new()`: a library-songs search result carries no album id, and nothing that plays a song reads `album_id` (only `get_songs_from_album` and the sample-library index use it). Follow the collection's `next` link with the same same-origin, `MAX_PAGES`, `page_context`, and page-notice rules `fetch` applies, so a query matching more than one page is read in full. To share that loop, split `fetch` into a thin wrapper over a new private `fetch_parsed` that takes the page parser as a closure, so the top-level and nested envelopes reuse the same pagination, deadline, and page-notice rules.
- `src/apple_music.rs`: rewrite `search_songs` to answer through the existing `browse` helper — `self.browse(move |rest, session| rest.search_library(session, &query), || sample_songs_matching(&query))` — keeping the blank-query guard from the sibling plan. A signed-in user now searches the real library; a signed-out one keeps the sample filter.
- `README.md`: extend the status and usage paragraphs to say the search box queries the signed-in Apple Music library.
- Tests:
  - `src/apple_music/rest/tests.rs`: `search_library_maps_the_search_envelope_and_percent_encodes_the_term` (assert the requested URL and the mapped songs, with the empty `album_id`); `search_library_returns_no_songs_when_the_results_carry_no_library_songs` (an absent and an empty `library-songs` both return an empty list); `search_library_follows_a_next_page`; `search_library_reports_a_transport_error_bare`; `search_library_rejects_a_nameless_song`.
  - `src/apple_music/tests.rs`: `search_songs_uses_the_rest_library_when_signed_in` — store a session through `authenticate_with` over a stub transport and assert the returned songs come from the REST search envelope, not the sample filter.

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `RestLibrary::search_library` requests `/v1/me/library/search?term=<percent-encoded query>&types=library-songs&limit=25` (a term with a space and a reserved character is encoded), maps the envelope's songs with their id, title, artist, duration, and first non-blank preview URL, and leaves `album_id` empty.
- A no-match search response that omits `results.library-songs` (or carries an empty `data`) returns an empty list rather than a parse error.
- A search response whose `library-songs.next` names a same-origin path is followed, and the songs of both pages are returned.
- A transport failure propagates as the transport's own cause; a resource without a name is an error naming its id.
- With a session stored, `AppleMusicService::search_songs` returns the REST results; with none, it returns the sample-library matches.

### Add a library search box to the playlist editor (found 2026-10-09, done 2026-10-09)

**Goal.** A search box sits above the browse list on every browse screen; typing a query and pressing Enter replaces the playlist-editor list with the matching songs, which play exactly like a browsed song. This is the first half of library search — the UI plus the service seam over the in-memory sample library. The sibling plan "Back the library search with the Apple Music REST endpoint" (found 2026-10-09) points the same `AppleMusicService::search_songs` method at the signed-in Apple Music library; land that one second.

**Approach.**

- `src/apple_music.rs`: add `pub async fn search_songs(&self, query: &str) -> Result<Vec<Song>, AppleMusicError>`. Trim the query and reject a blank one with an `AppleMusicError` (the query guard the id lookups use: a blank search has nothing to match and must not read as an empty library). Answer from the sample library with a new private pure `sample_songs_matching(query)` that keeps every sample-library song whose title or artist contains the query case-insensitively, in library order. (The sample library indexes songs by album in a `HashMap`, so the filter walks the ordered `artists` → `albums_by_artist` → `songs_by_album` chain rather than the map's unspecified iteration order, which would shuffle the matches between calls.)
- `src/ui/mod.rs`:
  - Add `search_query: String` (the box's text, `String::new()`) and `search_active: bool` (`false`) to `WinampPlayer`, initialised in `WinampPlayer::new`.
  - Add `Message::SearchChanged(String)` (stores the text) and `Message::SearchSubmitted(String)`.
  - `SearchSubmitted` trims the query; a blank one is `Task::none()`. Otherwise set `search_query`, set `search_active = true`, set `current_view = CurrentView::Songs`, call `player.songs.clear()`, and issue `fetch_into` with `player.songs.begin_fetch()`, context `format!("searching the library for {query:?}")`, fetch `|service| async move { service.search_songs(&query).await }`, and the existing `Message::SongsLoaded` / `Message::SongsLoadFailed`. Reusing the songs list keeps the `BrowseReply` generation guard, the `SongsLoaded`/`SongsLoadFailed` arms, and `view_songs` unchanged.
  - Clear `search_active` in the `ArtistSelected` and `AlbumSelected` arms. In `Back`, when `search_active` is set, clear it and set `current_view = CurrentView::Artists` (leave the search) instead of the Songs → Albums step, so Back never opens the empty or stale album list the search was issued from.
  - Push `views::view_search_box(&player.search_query)` above `main_content` (after the Back/Retry buttons) so the box is on every browse screen.
- `src/ui/views.rs`: add `pub fn view_search_box(query: &str) -> Element<'_, Message>` — an iced `TextInput` with the `"Search library"` placeholder, `on_input(Message::SearchChanged)`, `on_submit(Message::SearchSubmitted(query.to_string()))` (iced's submit callback carries a value, so the view captures the current box text), and the new `style::chrome_text_input_style`.
- `src/ui/style.rs`: add `chrome_text_input_style(status) -> text_input::Style` using the base-skin face, the light top-left bevel edge (iced's text-input style carries no shadow, so only the top-left half is drawn), and light text, mirroring `chrome_button_style`; focus sinks the face and reverses the edge.
- Tests:
  - `src/apple_music/tests.rs`: `search_songs_matches_the_sample_library_by_title_or_artist` (a title-only match, an artist-only match, case-insensitive, a match spanning two albums in library order, and a non-match absent); `search_songs_rejects_a_blank_query`.
  - `src/ui/tests.rs`: `search_submitted_replaces_the_songs_list_with_the_matches` (drive `Message::SearchSubmitted` through `update` and `drive_task`, then assert `player.songs.items` holds the matches and `current_view == CurrentView::Songs`); `search_submitted_with_a_blank_query_is_a_no_op`; `search_changed_stores_the_box_text`; `back_leaves_an_active_search_for_the_artists_list`; `entering_a_browse_level_clears_the_active_search`.
  - `src/ui/views.rs`'s suite: a `view_search_box` construction test like the other widget-construction tests; `src/ui/style.rs`'s suite: pin the text-input style's face, text, and bevel edges like `chrome_button_style_active_is_raised_chrome`.

**Files touched.** `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/views.rs`, `src/ui/style.rs`, `src/ui/tests.rs`.

**Acceptance criteria.**

- `make check` passes.
- `AppleMusicService::search_songs("...")` returns the sample-library songs whose title or artist contains the query case-insensitively; a blank query returns an error naming the query.
- Driving `Message::SearchSubmitted(query)` through `update` and the returned task leaves `current_view == CurrentView::Songs`, stores the matches in `player.songs.items`, and marks the search active; a blank query changes nothing.
- Driving `Message::Back` while a search is active returns to `CurrentView::Artists` and clears the active flag; `Message::ArtistSelected` and `Message::AlbumSelected` also clear it.
- `views::view_search_box` builds an element with the `Search library` placeholder whose input callback is `SearchChanged` and whose submit callback is `SearchSubmitted` carrying the box text.

### Drive the volume slider through the audio backend (found 2026-10-09, done 2026-10-09)

**Goal.** Moving the volume slider, or pressing the up/down arrow keys, changes the loudness of the playing preview, and a preview starts at the slider's volume. Today `Message::VolumeChange`, `VolumeUp`, and `VolumeDown` only write `AppState::volume`; `AudioOutput` has no volume method, so every preview plays at rodio's full-volume default no matter where the slider sits. This is the volume twin of the just-planned transport wiring and the same seam gap: only `play_track` reaches the backend.

**Approach.**

- `src/audio.rs`:
  - Add `set_volume(&self, volume: f32) -> Result<(), AppleMusicError>` to `AudioOutput` (doc: sets the output gain, `1.0` is full volume; it applies to the current player and to the next `play`, and does nothing when nothing is loaded).
  - `SilentOutput::set_volume` logs and returns `Ok`.
  - Add `Command::SetVolume(f32)`; `RodioOutput::set_volume` sends it.
  - In `run_worker`, hold the current gain in a `volume: f32` (initial `1.0`), map `Command::SetVolume(volume)` to storing it and calling `set_volume` on the current `rodio::Player` when one exists, and call `set_volume(volume)` on the player a `Command::Play` creates, so a gain set while stopped still applies to the next track.
  - Extend `silent_output_reports_success_for_every_command`, `audio_with_fallback_falls_back_to_silence_when_open_fails`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` to cover `set_volume`.
- `src/test_support.rs`: add `AudioCall::SetVolume(f32)`; `RecordingAudio::set_volume` records it; `FailingAudio::set_volume` returns `Ok(())` like its `pause`/`stop`, so a `play` failure still surfaces `play`'s own error. Because `f32` is not `Eq`, drop `Eq` from `AudioCall`'s derive (keep `Debug, Clone, PartialEq`); nothing requires `Eq` (the only uses are `assert_eq!` on a `Vec<AudioCall>`).
- `src/apple_music.rs`:
  - Add a synchronous `pub(crate) fn set_output_volume(&self, volume: f32) -> Result<(), AppleMusicError>` that forwards to `self.audio.set_volume(volume)` and touches no shared state. It is synchronous because the UI already stores the clamped value under its own lock (see `ui::mutate_volume`), and `AudioOutput::set_volume` is a non-blocking channel send; an async transition here would let a drag's messages race the state lock.
  - In `play_track`, capture the state's current volume before dropping the lock and apply it before the preview starts: `self.audio.set_volume(volume).and_then(|()| self.audio.play(url))`, keeping the existing `Err` rollback. This makes the first play use the slider's value (the `AppState` default `0.5`) rather than rodio's full-volume default.
- `src/ui/mod.rs`:
  - Add `mutate_volume(player, mutation) -> Task<Message>` beside `mutate_state`: apply `mutation` to `AppState` under one synchronous `blocking_lock`, read back the clamped `state.volume()`, forward that value to `player.apple_music_service.set_output_volume(...)`, and log a backend error. The state write stays synchronous so a drag remains responsive and ordered; the backend forward is a non-blocking channel send, so it runs on the UI thread.
  - Replace the `VolumeChange`, `VolumeUp`, and `VolumeDown` arms with `mutate_volume` calls (still `AppState::set_volume` and `AppState::nudge_volume`), and drop those three names from `mutate_state`'s doc list. `BalanceChange` is unchanged — rodio's `Player` has no panner.
- Tests:
  - `src/apple_music/tests.rs`: add `set_output_volume_forwards_to_the_audio_backend` (one `AudioCall::SetVolume`, `AppState::volume` unchanged). Update `play_track_starts_the_preview_through_the_audio_backend` to expect `[SetVolume(0.5), Play(url)]`.
  - `src/ui/tests.rs`: update `track_selected_plays_the_songs_preview_url` to expect `[SetVolume(0.5), Play(url)]`; add `volume_messages_forward_to_the_audio_backend`, which drives `VolumeChange`, `VolumeUp`, and `VolumeDown` through `update` over a `player_with_audio(RecordingAudio)` and asserts the recorded `SetVolume` values and `AppState::volume` (including a `f32::NAN` `VolumeChange` storing and forwarding the clamped `0.0`).

**Files touched.** `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`.

**Acceptance criteria.**

- `make check` passes.
- `RodioOutput::set_volume(0.4)` sends `Command::SetVolume(0.4)`; `SilentOutput::set_volume` returns `Ok`; a closed worker reports "the audio thread is gone" from `set_volume`.
- `AppleMusicService::set_output_volume(0.3)` records one `AudioCall::SetVolume(0.3)` and leaves `AppState::volume` unchanged.
- `play_track` with a preview records `SetVolume` carrying the state's current volume immediately before `Play`.
- Driving `Message::VolumeChange(0.2)` leaves `AppState::volume` at `0.2` and records `SetVolume(0.2)`; `Message::VolumeUp`/`VolumeDown` record the nudged, clamped value; an out-of-range or `NaN` `VolumeChange` stores and forwards the clamped value.

### Drive the transport's Play/Pause/Stop through the audio backend (found 2026-10-08, done 2026-10-09)

**Goal.** Pressing the transport's Play/Pause, Play, Pause, and Stop controls — and their X, C, and V keys — drives the injected `audio::AudioOutput` as well as `AppState`, so pausing or stopping silences the playing preview and Play resumes a paused preview or restarts a stopped one. Before this, only `AppleMusicService::play_track` reached the backend: the service's `pause` was dead code and there was no `resume` or `stop`, so the preview kept sounding after Stop and Pause. This is the wiring the audio-seam plan (PLANS.md `## Done`, 2026-10-08) explicitly deferred.

**Approach.**

- `src/audio.rs`: add `resume(&self) -> Result<(), AppleMusicError>` to `AudioOutput` (doc: resumes a paused player; no effect when nothing is loaded — rodio's `Player::play` resumes, it does not restart). Implement it for `SilentOutput` (log and `Ok`); add `Command::Resume`; `RodioOutput::resume` sends it; the `run_worker` loop maps `Command::Resume` to `player.play()` on the current `rodio::Player`. Extend `silent_output_reports_success_for_every_command`, `audio_with_fallback_falls_back_to_silence_when_open_fails`, `rodio_output_maps_each_seam_method_to_its_worker_command`, and `rodio_output_reports_a_closed_worker_for_every_command` to cover `resume`.
- `src/test_support.rs`: add `AudioCall::Resume`; `RecordingAudio::resume` records it; `FailingAudio::resume` returns `Ok(())` like its `pause`/`stop`.
- `src/apple_music.rs`: made the service the single owner of each transport transition, reusing the existing `AppState` mutators so none goes dead:
  - Moved `pause` out of the `#[allow(dead_code)]` block, narrowed that attribute to `next_track`/`previous_track`, and made the four transport methods `pub(crate)` so `ui` can call them.
  - Added a `transport: Arc<tokio::sync::Mutex<TransportState>>` field, where `TransportState { loaded, last_url }` records whether the backend holds a paused preview and the most recent preview URL. Every transport method and `play_track` holds this lock across the flag read, the backend call, and the flag write, so two overlapping toggles serialize instead of both reading the same flag.
  - `pause` calls `self.audio.pause()?`, then `self.state.lock().await.pause()`.
  - `resume` resumes a loaded player; when the backend discarded it (after a `stop`) it replays `last_url`, so Play after Stop restarts rather than claiming playback with nothing loaded; when no preview has ever played it logs and leaves the flag alone. On success it calls `self.state.lock().await.play()`.
  - `stop` calls `self.audio.stop()?`, clears `loaded`, then `self.state.lock().await.stop()`.
  - `toggle_play_pause` reads `is_playing` under the transport lock, calls `audio.pause()` (then `AppState::toggle_playing`) or `resume`, all under that one lock.
  - `play_track` also takes the transport lock and records `loaded`/`last_url`, clearing both when the backend rejects the preview.
- `src/ui/loading.rs`: add `transport_into(service, context, action) -> Task<Message>`: clone the service, await `action(service)`, report a backend error through a new pure `transport_failure_report(context, err)`, and map completion to `Message::TransportSettled`. Unlike `play_into` it carries no track or generation — a pause/stop cannot be superseded by a newer play — applies no timeout (the service's transport methods only lock and send a command to the audio worker), and does not prune the title index.
- `src/ui/mod.rs`: add the `Message::TransportSettled` variant with a no-op arm (its purpose is to make iced re-render after the async transition). Replace the four synchronous arms `PlayPause`, `Play`, `Pause`, and `Stop` with `transport_into(...)` calls to `toggle_play_pause`/`resume`/`pause`/`stop`; drop those four from `mutate_state`'s doc list and import `transport_into`.
- Tests:
  - `src/apple_music/tests.rs`: kept the two existing `pause` tests; added `resume_resumes_the_audio_backend`, `stop_stops_the_audio_backend`, `toggle_play_pause_pauses_while_playing`, `toggle_play_pause_resumes_while_paused`, `resume_after_stop_restarts_the_preview`, `resume_without_a_preview_leaves_the_transport_stopped`, and `concurrent_toggles_serialize_on_the_transport_lock` (a gated backend proves the second of two overlapping toggles waits on the transport lock; the test fails when the lock is dropped before the backend call).
  - `src/ui/tests.rs`: converted `play_pause_toggles_is_playing`, `stop_clears_is_playing`, and `play_and_pause_set_the_playback_flag_explicitly` into `#[tokio::test]`s that drive the returned task with `drive_task` before reading state, and updated their stale `blocking_lock` comments. Added `transport_messages_drive_the_audio_backend`, which drives each of the four messages through `update` and `drive_task` over a `player_with_audio(RecordingAudio)` and asserts the recorded calls (`Stop`, `Resume`, `Pause`, and a `Pause` from the toggle).
  - `src/ui/loading.rs`'s suite: added a pure-function test for `transport_failure_report`, like the existing `play_failure_report` test.
- `README.md`: updated the status and usage paragraphs to describe Pause/Stop driving the backend and Play resuming a paused preview or restarting a stopped one.

**Files touched.** `src/audio.rs`, `src/test_support.rs`, `src/apple_music.rs`, `src/apple_music/tests.rs`, `src/ui/loading.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- With a `RecordingAudio`, `AppleMusicService::stop` records one `AudioCall::Stop` and clears `AppState::is_playing` while keeping `current_track`; `resume` after a paused preview records one `AudioCall::Resume` and sets `is_playing`; `toggle_play_pause` records `Pause` from playing and `Resume` from paused.
- `RodioOutput::resume` sends `Command::Resume`, and `SilentOutput::resume` returns `Ok`.
- `resume` after `stop` replays the remembered preview URL and sets `is_playing`; `resume` with no preview ever played records no call and leaves `is_playing` false.
- Two overlapping `toggle_play_pause` calls serialize on the transport lock: the second observes the first's flag write, so the pair ends playing with one `Pause` then one `Resume`.
- Driving `Message::Stop`, `Message::Pause`, `Message::Play`, and `Message::PlayPause` through `update` and the returned task leaves `AppState::is_playing` matching the control (false after Stop/Pause, true after Play when a preview is loaded, toggled after PlayPause when one is) and records the matching `AudioCall`.
- `transport_failure_report` names the context and includes the backend error.

### Document `make test-one` and report a build failure distinctly from a mistyped name (found 2026-10-08, done 2026-10-08)

`make test-one TEST=<name>` (added 2026-10-08) runs only the tests whose
names contain `<name>`, but the README's Development section never mentioned
it, and the target's no-match guard reported a build failure and a mistyped
name through one hedged message: `no test name contains 'X' (or the test build
failed)`.

**Goal.** The README documents the target, and `make test-one` reports a build
failure and a mistyped name distinctly.

**Approach.**

- `Makefile`: capture the `cargo test --list` output and check its exit status
  separately, so a failed build reprints the compiler diagnostics under a
  `make test-one: the test build failed` line, while a successful build with
  no match reports `no test name contains '<name>'`. Update the target's
  comment block to match. The cargo command is the `CARGO` variable so
  `test-one-guard-test` can substitute a stub.
- `README.md`: add `make test-one TEST=<name>` to the individual stages in the
  Development section.
- `Makefile`: `test-one-guard-test` invokes the recipe through `$(MAKE)` with
  an empty `TEST=` (the usage guard), `CARGO=false` (a failed build), and
  `CARGO=true` (a successful build that names no test), asserting each cause
  is reported distinctly, so removing any of the three guards fails
  `make check`.

**Acceptance criteria.**

- `make check` passes.
- `make test-one TEST=zzz_no_such_test` reports the no-match cause rather
  than the build-failure one.
- `make test-one-guard-test` exercises the empty-`TEST` usage guard and both
  the build-failure and no-match branches of the recipe and fails if any
  message regresses.

**Files touched.** `Makefile`, `README.md`.

### Reject a blank preview URL at the playback seam (found 2026-10-08, done 2026-10-08)

`AppleMusicService::play_track` validates its track id — a blank or
control-character id is rejected with a named error before shared state is
touched — but passed `preview_url` straight to the audio backend. A caller
handing `Some("")` (or only whitespace) made the backend attempt a request to
an empty URL, so the user saw a transport error that did not name the actual
defect. The REST layer already maps a blank preview to `None`, but the seam is
public and documented for a real backend to fill, so the guard belongs at the
seam too.

**Goal.** `play_track` rejects a present-but-blank `preview_url` with a named
`AppleMusicError` before committing state, while `None` keeps its documented
"no playable asset" behavior (state committed, no audio).

**Approach.**

- `src/apple_music.rs`: add `ensure_preview_url_is_valid(Option<&str>)`, which
  returns `Ok` for `None` or a non-blank URL and names the offending value
  (quoted with `{url:?}`) for a blank one; call it in `play_track` after
  `ensure_id_is_valid`, before the state lock. Update `play_track`'s doc comment
  and its `# Errors` list.

**Acceptance criteria.**

- `make check` passes.
- A test drives `play_track` with `Some("")` and `Some("   ")` mid-playback and
  sees the named error with shared state untouched.

**Files touched.** `src/apple_music.rs`, `src/apple_music/tests.rs`.

### Show the playing track's artist in the Now Playing bar (found 2026-10-08, done 2026-10-08)

Classic Winamp's main-window marquee reads "Artist - Title", and the Apple
Music browse response already carries each track's `attributes.artistName`.
Nostalgia's `Song` drops it — `rest::Attributes` reads only `name`,
`durationInMillis`, and `previews` — so the Now Playing bar can only name the
title. This carries the artist onto the model and shows it beside the title.
The playlist editor's rows keep their `title + length` layout, and the artist
never reaches the transport, browse navigation, volume, balance, or
equalizer.

**Goal.** `Song` carries `artist: String` — the performing artist's display
name, `""` when the source supplied none. The REST browse client parses
`attributes.artistName` into it; the sample library and test fixtures fill
it. The Now Playing bar renders `<artist> - <title>` when the artist is known
and falls back to the title alone when it is empty. No new dependency.

**Approach.**

- `src/library.rs`: add `pub artist: String` to `Song`, documented as the
  performing artist's display name with `""` meaning the source supplied
  none. Add `"artist": "The Sample Band"` to `song_payload()` and to
  `song_required_fields_payload()`, so `assert_every_field_required` still
  covers every required key and the round-trip test pins the field name.
- `src/apple_music/rest.rs`: add `#[serde(rename = "artistName")]
  artist_name: Option<String>` to `Attributes`, and set `artist:
  resource.artist_name.clone().filter(|name| !name.trim().is_empty())
  .unwrap_or_default()` in the `get_songs_from_album` mapping — an absent or
  blank name maps to `""`, the model's "no artist" sentinel, matching the
  `duration_ms`/`preview_url` treatment of missing source data.
- `src/sample_library.rs`: every `Song` literal gains `artist` — "The Sample
  Band" for the `album-1`/`album-2` songs, "Echo Chamber" for the `album-3`
  song — matching the album's `artist_id`. Add a `songs_carry_the_documented_artists`
  test mirroring `songs_carry_the_documented_durations`, pinning every song's
  artist in library order.
- `src/test_support.rs`: `sample_song`, `stepping_songs`,
  `single_song_album`, and `second_album_songs` gain
  `artist: "The Sample Band".to_string()`.
- The remaining `Song { … }` literals (`src/apple_music/tests.rs`,
  `src/apple_music/rest/tests.rs`, `src/ui/tests.rs`) gain the field.
- `src/apple_music/rest/tests.rs`: the `songs_from_album` fixture carries
  `"artistName": "The Sample Band"` and its expected `Song` carries that
  artist; a companion test pins an absent `artistName` to `""`.
- `src/ui/views.rs`:
  - Add `pub artist: String` to `KnownTrack`, documented beside `title` and
    `duration_ms`.
  - Add `pub fn now_playing_artist<'a>(tracks: &'a HashMap<String,
    KnownTrack>, current_track: Option<&str>) -> Cow<'a, str>`: the known
    track's artist borrowed from the map, or the `""` literal when the track
    is unknown or none is current. It mirrors `now_playing_label`, so the
    artist adds no per-frame `String` allocation.
  - Change `view_now_playing(label: Cow<'_, str>, artist: Cow<'_, str>, time:
    String)` to push the artist and a `" - "` separator before the title only
    when `artist` is non-empty, so an artist-less track still reads `Now
    Playing: <title>`.
  - In the `#[cfg(test)] mod tests`: the `known_tracks` helper takes
    `(id, artist, title)` triples; add value and borrow tests for
    `now_playing_artist`; pass the artist to the `view_now_playing` construct
    call.
- `src/ui/mod.rs`:
  - Add a `WinampPlayer::now_playing_artist` method mirroring
    `now_playing_label`.
  - The `TrackSelected` arm records `artist: song.artist.clone()` in the
    `KnownTrack` entry.
  - `view` resolves the artist in the same state-lock block as the label and
    passes it to `view_now_playing`.

**Files touched.** `src/library.rs`, `src/apple_music/rest.rs`,
`src/apple_music/rest/tests.rs`, `src/sample_library.rs`,
`src/test_support.rs`, plus the `Song` literals and the `KnownTrack` fixture
in `src/apple_music/tests.rs`, `src/ui/tests.rs`, and `src/ui/views.rs`, and
the Now Playing bar wiring in `src/ui/mod.rs`.

**Acceptance criteria.**

- `make check` passes.
- A `get_songs_from_album` stub response carrying `"artistName": "The Sample
  Band"` maps that string to `Song::artist`; a response omitting it (or
  carrying a blank one) maps to `""`.
- Every `sample_library()` song carries its album's artist name, pinned by
  `songs_carry_the_documented_artists`.
- `now_playing_artist` returns the known track's artist, `""` for an unknown
  id, and `""` when no track is current; the known case borrows from the map
  (`Cow::Borrowed`).
- `view_now_playing` builds with a non-empty artist and with an empty one.
- The `now_playing_label` value and borrow tests still pass unchanged, and
  the browse-away regression
  (`now_playing_label_keeps_the_track_name_after_browsing_to_another_album`)
  still passes.

### Bound the audio worker's preview download with a timeout (found 2026-10-08, done 2026-10-08)

The audio seam's worker downloaded the preview with `ureq::get(url)`, whose
default agent leaves every network timeout `None`. A server that accepted the
connection and then stalled blocked the worker thread forever, and because the
worker serves commands one at a time, every later play, pause, and stop queued
behind it. The REST client already bounded its requests; this applies the same
bound to the app's other network call.

**Goal.** A preview download that stalls fails with a logged error within a
fixed bound, so the worker keeps serving later commands, and the production
agent still reuses its pooled connection.

**Approach.**

- `src/audio.rs`: add `PREVIEW_TIMEOUT: Duration = Duration::from_secs(30)`,
  matching the REST client's `REQUEST_TIMEOUT`.
- Add `preview_agent() -> &'static ureq::Agent` (a `OnceLock`, built by
  `agent_with_timeout`) so consecutive plays reuse one pooled agent, and
  `agent_with_timeout(timeout)`, split out so a test can bound a download
  against a stalled loopback server without waiting out the production 30s.
- `download_and_decode(agent, url)` takes the agent and calls `agent.get(url)`;
  the worker passes `preview_agent()`. The download error already logs on the
  worker, so a timeout reads like any other download failure.

**Acceptance criteria.**

- `make check` passes.
- A stalled loopback preview server makes `download_and_decode` return `Err`
  within a short injected bound, asserted from a worker thread so an unbounded
  download fails the test instead of hanging the suite.

**Files touched.** `src/audio.rs`.

### Add the audio-output seam and a rodio backend (found 2026-10-08, done 2026-10-08)

The audio-output decision (QUESTIONS.md `## Answered`, 2026-10-08) scopes
playback to the Apple Music **preview**; this lands the playback half behind an
injectable seam, with `rodio` as the production backend. Rodio 0.22's API
differs from the names first sketched here — the device is a `MixerDeviceSink`
from `DeviceSinkBuilder::open_default_sink()`, playback goes through
`rodio::Player`, and the preview decodes with `Decoder::new_mp4` — so the
approach below names the API that landed. The transport's Pause and Stop
buttons still update only UI state; wiring them to the service's `pause`/`stop`
seam is left to a later plan.

**Goal.** `AppleMusicService` plays real audio through an injectable
`AudioOutput` seam; the production implementation uses `rodio`, `play_track`
starts the selected song's preview, `pause` pauses it, and a recording stub
stands in under test. Depends on "Carry the Apple Music preview URL on
`Song`" (the URL must be on `Song` first).

**Approach.**

- `Cargo.toml`: `rodio = { version = "0.22.2", default-features = false,
  features = ["playback", "mp4"] }`. The `mp4` feature already pulls
  `symphonia-isomp4` and `symphonia-aac` (Apple's preview is AAC in M4A), and
  dropping the default features leaves out recording and the unused decoders.
- A new `audio` module (the `AudioOutput` trait, `SilentOutput`, `RodioOutput`, and `audio_with_fallback` symbols), registered by `pub mod audio;` in `src/main.rs`:
  - `pub trait AudioOutput: Send + Sync` with `fn play(&self, url: &str) ->
    Result<(), AppleMusicError>`, `fn pause(&self) -> Result<(),
    AppleMusicError>`, and `fn stop(&self) -> Result<(), AppleMusicError>`.
  - `pub struct SilentOutput`, whose methods log and return `Ok(())`: the
    fallback when no device opens, and the production stand-in for the browse
    tests.
  - `pub struct RodioOutput`, owning a `Mutex<std::sync::mpsc::Sender<Command>>`
    to a worker `std::thread`. Rodio 0.22 opens the device through
    `DeviceSinkBuilder::open_default_sink()` (a `MixerDeviceSink`) and plays
    through `rodio::Player`; keeping both on the worker makes the seam
    `Send + Sync` regardless of the platform handle's thread bounds. The
    sender sits behind a `Mutex` because the standard sender is `Send` but not
    `Sync`. `Command` is `Play(String)`, `Pause`, `Stop`. The worker downloads
    the URL with `ureq`, buffers it, and decodes it with
    `rodio::Decoder::new_mp4(Cursor<Vec<u8>>)`, appending to a fresh `Player`;
    a download/decode failure is logged with `eprintln!` (the trait call
    itself fails only when the worker is gone). `RodioOutput::new() ->
    Result<Self, AppleMusicError>` opens the default device on the worker and
    reports success/failure back through a `std::sync::mpsc` channel, so a
    device-less machine gets `Err` rather than a panic.
  - `pub(crate) fn audio_with_fallback(open: impl FnOnce() ->
    Result<Arc<dyn AudioOutput>, AppleMusicError>) -> Arc<dyn AudioOutput>`:
    returns the opened output, or a `SilentOutput` when `open` fails. Pure and
    injectable so the fallback is testable without a device.
- `src/apple_music.rs`:
  - Add `audio: Arc<dyn audio::AudioOutput>` to `AppleMusicService`.
  - `with_transport` builds `SilentOutput`; add
    `with_audio(state, transport, audio)` for the playback tests; `new(state)`
    builds `RodioOutput` through `audio_with_fallback`, then calls
    `with_audio`.
  - `play_track(&self, track_id: &str, preview_url: Option<&str>, is_current:
    …)`: keep the id guard and the `is_current`-guarded state commit, then call
    `self.audio.play(url)` when `preview_url` is `Some` and propagate an audio
    error. `pause` calls `self.audio.pause()` before clearing `is_playing`.
  - Update the doc comments that call playback a shared-state-only stub.
- `src/ui/mod.rs`: the `TrackSelected` arm passes `song.preview_url.as_deref()`
  to `play_track`.
- Tests: `AudioCall`/`RecordingAudio`/`FailingAudio` live in
  `src/test_support.rs`, shared by the `apple_music` and `ui` suites. The
  `apple_music` suite asserts `play_track(…, Some(url), …)` records one
  `Play(url)` and commits state, `play_track(…, None, …)` records none and
  still commits, a failing backend's error propagates, and `pause` records
  `Pause`. The `ui` suite drives `TrackSelected` end to end and asserts the
  backend saw the selected `Song::preview_url`. The service constructors in
  both suites use `with_transport` (silent audio), so no test opens an audio
  device.

**Files touched.** `Cargo.toml`, `README.md`, `src/main.rs`, the new
`src/audio.rs` (the `AudioOutput`, `SilentOutput`, `RodioOutput`, and
`audio_with_fallback` symbols), `src/apple_music.rs`,
`src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/mod.rs`,
`src/ui/tests.rs`.

**Acceptance criteria.**

- `make check` passes, including on a machine with no audio device (the
  `RodioOutput` tests cover `new` returning `Err` without panicking; they do
  not assert a device is present).
- `audio_with_fallback` with a failing opener returns a `SilentOutput` whose
  `play`/`pause`/`stop` return `Ok`.
- With a recording `AudioOutput`, `play_track("song-1",
  Some("https://example.test/preview.m4a"), || true)` records one `Play` with
  that URL and sets `AppState::current_track`/`is_playing`.
- `play_track(…, None, …)` records no `Play` but still commits the state
  transition.
- `pause` records a `Pause`.
- The UI passes the selected `Song::preview_url` to `play_track`.

### Reject a `make test-one TEST=<name>` that matches no test name (found 2026-10-08, done 2026-10-08)

`make test-one` forwarded its filter to libtest, which exits `0` when the
filter matches zero tests — so a typo (`TEST=clamp_with_nan_fallback`, a
function name rather than a behavior name) printed "running 0 tests" and a
green result, reading as a pass.

**Goal.** A `TEST` that matches no test name fails the target with a message,
while a real name still runs only the matching tests.

**Approach.**

- `Makefile`: `test-one` lists the tests the filter would select
  (`cargo test --locked -- --list "$(TEST)"`) and requires at least one line
  naming a test before the filtered run. The match is anchored to libtest's
  test-name lines (`: test$`) so the trailing summary (`N tests, 0
  benchmarks`) cannot satisfy the guard. The list command's output is
  captured together with its stderr and its exit status checked separately,
  so a build failure is reported as such rather than as a mistyped name. The
  match pattern lives in the `TEST_LIST_HAS_TEST` variable.
- `Makefile`: `test-one-guard-test` feeds that pattern a synthetic list with
  a test line and a summary-only list, asserting the first matches and the
  second does not; it is added to `check`, so a guard that reintroduces the
  summary-line false pass fails the gate.

**Acceptance criteria.**

- `make check` passes.
- `make test-one TEST=<name>` with a real test name runs only the matching
  tests and exits zero.
- `make test-one` with no `TEST` exits non-zero with a usage line.
- `make test-one TEST=benchmarks` (a value that appears only in libtest's
  summary line) and `make test-one TEST=no_such_test` each exit non-zero with
  a "no test name contains" message and run no tests.
- `make test-one-guard-test` passes.

- Carry the Apple Music preview URL on `Song` (found 2026-10-08, done 2026-10-08; commit 8e9faf3)
- Name the page when a paginated Apple Music browse fails (found 2026-10-08, done 2026-10-08; commit 71853ef)
- Show each track's length in the playlist editor (found 2026-10-08, done 2026-10-08; commit f695290)
- Read a later Apple Music error entry when the first names no cause (found 2026-10-08, done 2026-10-08; commit bb1696d)
- Follow a browse collection's next page so a large library is read in full (found 2026-10-08, done 2026-10-08; commit a330d17)
- Show the current track's duration in the Now Playing bar (found 2026-10-08, done 2026-10-08; commit 98aa4b3)
- Log a notice when a browse response has an unread next page (found 2026-10-08, done 2026-10-08; commit 210deb5)
- Add a `make test-one` target for running one focused test (found 2026-10-08, done 2026-10-08; commit 277625f)
- Name the user-token shape defect when the sign-in callback is rejected (found 2026-10-08, done 2026-10-08; commit db297ed)
- Name the envelope, not "invalid JSON", when a valid-JSON body does not match (found 2026-10-08, done 2026-10-08; commit ad94f37)
- Report just the HTTP status when an Apple Music error body's cause is blank (found 2026-10-08, done 2026-10-08; commit 1723afd)
- Add a Winamp Shuffle toggle that randomizes Next (found 2026-10-08, done 2026-10-08; commit 7c299a7)
- Reject a blank resource id from an Apple Music browse response (found 2026-10-08, done 2026-10-08; commit e19867b)
- De-duplicate the query label in a failed browse report (found 2026-10-08, done 2026-10-08; commit 196e68a)
- Add a Winamp balance slider beside the volume slider (found 2026-10-08, done 2026-10-08; commit 8ee7986)
- Add the Winamp title-bar clutter bar and shade button (found 2026-10-08, done 2026-10-08; commit 7bcb23f)
- Answer the browse queries from the Apple Music REST API when signed in (found 2026-10-08, done 2026-10-08; commit 4a4f2e6)
- Add the Apple Music REST browse client (found 2026-10-08, done 2026-10-08; commit c63808d)
- Wire the MusicKit session into `AppleMusicService` (found 2026-10-08, done 2026-10-08; commit cf514ca)
- Add the MusicKit loopback authorization module (found 2026-10-08, done 2026-10-08; commit c15f6b6)
- Add classic Winamp main-window keyboard shortcuts for transport and volume (found 2026-10-08, done 2026-10-08; commit 97f6658)
- Add Winamp window-shade ("roll up") mode (found 2026-10-08, done 2026-10-08; commit 1e46f04)
- Add Winamp equalizer preset curves and a preset pick list (found 2026-10-07, done 2026-10-07; commit f7647ca)
- Add a Winamp custom title bar and drop the OS window frame (found 2026-10-07, done 2026-10-07; commit 81eb991)
- Frame the browse list as a sunken Winamp playlist editor (found 2026-10-07, done 2026-10-07; commit 74bbe47)
- Style the volume and equalizer sliders as sunken Winamp grooves with raised chrome thumbs (found 2026-10-07, done 2026-10-07; commit 4858b11)
- Style the transport buttons as raised Winamp chrome (found 2026-10-07, done 2026-10-07; commit dd0a5c5)

- Add a Winamp two-tone bevel layer and frame the Now Playing and equalizer panels (found 2026-10-06, done 2026-10-06; commit 8984e90)
- Add a Winamp 2.x base-skin palette and apply it as the app theme (done 2026-10-06; commit 9d74276)
- Add the Winamp equalizer panel: on/off, preamp, and ten band sliders (done 2026-10-06; commit 82b9757)
- Add a Repeat toggle to the transport controls (done 2026-10-06; commit d2f72bc)
- Highlight the currently playing song in the Songs browse view (done 2026-10-06; commit 7e6d83b)
- Add a Stop button to the transport controls (done 2026-10-06; commit 8e91e1e)
- Show the song's title in the Now Playing bar, not its raw id (done 2026-10-05; commit 7d29183)
- Make Previous/Next step through the songs of the current album (done 2026-10-05; commit 4c8e223)
- Add a working volume slider to the transport controls (done 2026-10-05; commit 7029804)
- Make the app build on iced 0.14 and render a browsable sample library (done 2026-10-04; commit 5af3f08)
