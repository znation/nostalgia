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

### Add the playback-position readout to the Now Playing bar (Playback position, part 3/3: planned 2026-10-21 by the plan role, requires part 2/3 landed)

**Goal.** The Now Playing bar's right-hand readout shows the current track's elapsed and total time as `"<elapsed>/<total>"` (for example `0:45/3:12`) whenever the audio backend reports a loaded preview's position, and falls back to the existing plain total (`"--:--"` when no track is current) when it does not. Pause leaves the stored value and the paused player's position in place, a seek is reflected by the next position reply, Stop and a new selection clear the readout, and a play whose preview fails to install leaves the plain total rather than a frozen `0:00`.

This is the second half of the former part 2/2 and consumes part 2/3's authoritative `AudioOutput::position`: the backend answers `None` when no preview is loaded, so the UI keeps no playback guard and a selected track with no preview URL never turns one on.

**Approach.**

- `src/apple_music/playback.rs`: update `playback_position`'s doc to say it reports the loaded preview's position and `None` when no preview is loaded or the backend cannot answer, and remove its `#[allow(dead_code)]` now that the UI calls it.
- `src/ui/views/now_playing.rs`: add `pub fn format_position_time(position_ms: u64) -> String`, calling the existing `format_minutes_seconds` (which already yields `"0:00"` for zero and keeps minutes unbounded). Add `pub fn now_playing_progress(position_ms: u64, total: &str) -> String` returning `format!("{}/{}", format_position_time(position_ms), total)`. Change `now_playing_time` to take `position: Option<Duration>`: when `current_track` is in `tracks` and `position` is `Some`, return `Cow::Owned(now_playing_progress(...))` with the duration converted saturating to milliseconds; otherwise keep today's borrowed `KnownTrack::time` or `"--:--"` fallback. Update its doc comment to say the combined case allocates.
- `src/ui/mod.rs`: add `playback_position: Option<Duration>` and `position_epoch: u64` to `WinampPlayer`, initialized `None`/`0` in `new`, and define `POSITION_POLL_INTERVAL` (`Duration::from_secs(1)`) with a doc note that it bounds the readout refresh, not the audio. Add `Message::PositionTick` and `Message::PlaybackPosition { epoch: u64, position: Option<Duration> }`, and route both from the `update` dispatch table to the two handlers added in `src/ui/handlers/playback.rs` below. In `init_ui`, batch `iced::time::every(POSITION_POLL_INTERVAL).map(|_| Message::PositionTick)` with the existing `keyboard::listen().filter_map(message_for)` (for example with `iced::Subscription::batch`). Pause leaves the field alone, so the next poll keeps showing the paused player's frozen position. `WinampPlayer::now_playing_time` keeps its signature but passes `self.playback_position` to `views::now_playing_time`; the `view` call site is unchanged, though its comment that all three readouts always borrow must note the combined elapsed case allocates.
- `src/ui/handlers/playback.rs`: the two handlers the table routes to — `position_tick` and `playback_position` — plus the readout reset. `position_tick` captures `player.position_epoch`, clones the service, and returns `Task::perform(async move { service.playback_position() }, move |position| Message::PlaybackPosition { epoch, position })`; `playback_position` stores `position` (including `None`) only when `epoch == player.position_epoch`, dropping a reply issued before a transition. Add `fn reset_position_readout(player: &mut WinampPlayer)` that bumps `position_epoch` and sets `playback_position = None`; call it after the song resolves in `track_selected` and at the start of `stop` (which becomes a block returning the existing `transport_into` call).
- `src/ui/views/tests.rs`: update the `now_playing_time_resolves_known_track_to_cached_label` call sites to pass `None` as the position; extend it (or add `now_playing_time_shows_elapsed_over_total_when_a_position_is_known`) so `now_playing_time(&tracks, Some("song-1"), Some(Duration::from_secs(192)))` reads `"3:12/3:30"` and is `Cow::Owned`; add `format_position_time_formats_elapsed_and_zero` (`0`→`"0:00"`, `192_000`→`"3:12"`, `65_000`→`"1:05"`) and `now_playing_progress_joins_elapsed_and_total` (`(192_000, "12:38")`→`"3:12/12:38"`).
- `src/ui/tests.rs`: keep `now_playing_time_shows_the_played_tracks_duration_after_browsing_to_another_album` asserting the plain total (its calls now use the unchanged `WinampPlayer::now_playing_time` signature); add `now_playing_time_shows_elapsed_over_total_while_a_position_is_known` (reuse `play_song_1_then_browse_to_album_2`, set `player.playback_position = Some(Duration::from_secs(45))`, assert `"0:45/3:30"`), `pause_keeps_the_stored_elapsed_readout` (set a known position, drive `Message::Pause`, then assert the field still holds that position and the bar still reads `"0:45/3:30"`; together with part 2/3's `a_pause_keeps_the_loaded_player_reporting_its_position` this pins that pause freezes rather than clears), `a_position_reply_after_a_seek_updates_the_readout` (set `Some(45)`, drive `Message::SeekForward`, feed `Message::PlaybackPosition` with `Some(50)`, assert `"0:50/3:30"`; the existing `seek_messages_drive_the_audio_backend` pins the backend call), `track_selected_clears_the_elapsed_readout`, `stop_clears_the_elapsed_readout`, `position_replies_store_and_stale_ones_are_dropped` (store a reply at the current epoch, then drive a reply stamped with the pre-`Stop` epoch and assert it does not overwrite), and `position_tick_schedules_a_query` (`assert_message_schedules_work`).
- `README.md`: extend only the Usage sentence that describes the Now Playing bar's artist and title so it says the length reads as elapsed/total while a preview is loaded and as the plain length otherwise. Do not edit the `<!-- tumwater:status:start -->` block or add archive narrative anywhere; that block is state-only (see `762af8c`), and the README diff must be confined to the Usage paragraph.
- `docs/invariants.md`: in the Now Playing label section, add the time-readout guarantee (elapsed/total only while a loaded preview position is known, the cached total otherwise, `--:--` with no current track), its enforcement by `views::now_playing_time`/`now_playing_progress` and the `PlaybackPosition` handler, and cite the new tests plus part 2/3's audio-seam `position_query_reports_none_when_nothing_is_loaded`, `a_seek_is_reflected_by_the_next_position_query`, and `a_pause_keeps_the_loaded_player_reporting_its_position`.

**Files touched.** `src/apple_music/playback.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/mod.rs`, `src/ui/handlers/playback.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `make check` passes.
- `format_position_time` returns `"0:00"`, `"3:12"`, and `"1:05"` for zero, 192000 ms, and 65000 ms; `now_playing_progress(192_000, "12:38")` returns `"3:12/12:38"`.
- With a known current track and a `Some` position, the bar renders `elapsed/total`; with `None`, the plain total; with no current track or an unknown id, `"--:--"`.
- Pause does not clear the stored position and the next poll still reads the paused player, so the readout keeps the frozen value; `a_position_reply_after_a_seek_updates_the_readout` shows a seek's new position reaching the bar on the next reply; Stop and a new selection clear the readout immediately through `reset_position_readout`. A failed preview start leaves `None`, so the bar falls back to the plain total.
- A `PlaybackPosition` reply stamped with an older `position_epoch` is dropped, so a poll in flight across a Stop or selection cannot restore the previous elapsed value.
- The poll runs once per `POSITION_POLL_INTERVAL` in an iced task, not in `update`.
- The README diff is confined to the Usage sentence; the status block is byte-for-byte unchanged.
- The change touches only the eight listed files plus this `PLANS.md` edit.

### Show each album row's artist beside its title (planned 2026-10-21 by the plan role)

**Goal.** Every album row reads `<artist> - <title>` when the album carries an artist and just the title when it does not, so the per-artist Albums view and the flat All Albums view match the library mockup's album rows (`docs/screenshots/library.png`). The secondary `View Songs` hint, row numbering, the playing-row marker, selection messages, and ordering stay exactly as they are. This is independent of the playback-position series: it touches the album model, the REST album mapping, and browse rows, not the audio seam or the Now Playing bar. Album row running times are out of scope because an album query does not supply its songs' durations.

**Approach.**

- `src/library.rs`: add `pub artist: String` to `Album`, with `#[serde(default)]` and a doc comment saying `""` means the source supplied no artist (the same sentinel as `Song::artist`). Add `pub fn display_title(&self) -> Cow<'_, str>` mirroring `Song::display_title`: borrow the title when the artist is empty, otherwise return an owned `"<artist> - <title>"`. In the tests, add `"artist": "The Sample Band"` to `album_payload` and to the payload in `album_deserialization_ignores_unknown_fields` (which compares against `sample_album`); add an `album_required_fields_payload` that omits the optional artist and point `album_deserialization_rejects_missing_required_fields` at it; add `album_deserialization_defaults_a_missing_artist_to_empty`; and add `album_display_title_joins_the_artist_and_title` plus `album_display_title_borrows_the_title_when_the_artist_is_empty`, mirroring the two `display_title` song tests.
- `src/test_support.rs`: give `sample_album`'s new field `artist: "The Sample Band"`.
- `src/sample_library.rs`: fill the three sample `Album` literals with their owner's name — `"The Sample Band"` for `album-1` and `album-2`, `"Echo Chamber"` for `album-3`.
- `src/apple_music/rest.rs`: in `Resource::into_album`, take `attributes.artist_name` and store it in `Album.artist`, filtering out a blank or whitespace-only name to `""` exactly as `Resource::into_song` does; document that a resource without an artist name maps to the empty sentinel. Both `get_albums_by_artist` and `get_all_albums` already map through this function, so no query changes are needed.
- `src/apple_music/rest/tests.rs`: add `"artistName":"The Sample Band"` to the album stubs in `albums_by_artist_map_library_json_and_set_the_artist_id` and `get_all_albums_maps_the_library_albums_collection`, and update their expected `Album` values (including the one spelled out in `get_all_albums_maps_the_library_albums_collection`) with the mapped artist. Add `get_all_albums_leaves_a_missing_album_artist_empty`, using a stub whose album attributes omit `artistName`, and assert `artist` is `""`.
- `src/apple_music/tests.rs`: update the two signed-in browse expectations that construct an `Album` in `get_albums_by_artist_uses_the_rest_library_when_signed_in` and `get_all_albums_uses_the_rest_library_when_signed_in`; supply `artistName` in their stubs and assert the mapped names so the service-level path is pinned too.
- `src/ui/tests.rs`: add the new field to the `flat_album` literal in `album_selected_in_all_albums_loads_its_songs_and_back_returns_to_all_albums` and the `album-3` literal in `album_selected_fetches_the_albums_songs_into_the_player` (use the sample library's `"Echo Chamber"`); other `Album` values come from `sample_album`.
- `src/ui/views/browse.rs`: change `album_row`'s title from `Cow::Borrowed(album.title.as_str())` to `album.display_title()`, and update its doc to say an album with an artist reads `<artist> - <title>` while an artist-less one reads the title alone.
- `src/ui/views/tests.rs`: update `album_row_uses_title_and_selects_the_album_by_index` to expect `"The Sample Band - First Record"`; add `album_row_shows_the_title_alone_when_the_artist_is_empty`, blanking `sample_album().artist` and asserting the title is the bare `"First Record"` and borrows.
- `README.md`: extend the Usage sentence that describes browse rows so it also says album rows read `<artist> - <title>`, falling back to the title when the source supplies no artist. Keep the diff to the Usage paragraph; do not touch the `<!-- tumwater:status:start -->` block.
- Completion: move this entry from `PLANS.md` `## Planned` to `PLANS-DONE.md` `## Done` with a `done <landing date>` stamp.

**Files touched.** `src/library.rs`, `src/test_support.rs`, `src/sample_library.rs`, `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music/tests.rs`, `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, plus the `PLANS.md`/`PLANS-DONE.md` move on completion.

**Acceptance criteria.**

- `make check` passes.
- `Album::display_title` returns `"The Sample Band - First Record"` for the sample album, borrows the title alone for an album whose artist is empty, and never emits a dangling `" - "` prefix.
- `album_row` returns the combined artist/title for both the per-artist Albums list and the All Albums list, since both build through `album_row`, and returns the bare title when the artist is empty; the secondary label and selection message are unchanged.
- `Resource::into_album` maps a non-blank `artistName` into `Album.artist`, and a missing or blank one leaves `""`; the two signed-in browse tests and the two REST mapping tests show the name reaching the album returned to the UI.
- `Album` round-trips with the new field, still tolerates unknown fields, and its missing-required-field probe still rejects a payload without an id, title, or artist id while a missing artist defaults to `""`.
- The README change is confined to the Usage paragraph, and the status block is byte-for-byte unchanged.
- The implementation changes only the ten listed files plus the `PLANS.md`/`PLANS-DONE.md` move, and the commit body names them consistently with `git diff --stat`.


### Retire the `unclear-invariant` validation gap by pinning subsystem contracts in docs/invariants.md (planned 2026-10-23 by the steward role)

**Goal.** Three Fixed entries (the stale browse reply store-time invariant, the bidirectional browse contract, and the Now Playing title contract) each reconstructed a guarantee that nothing had written down, recorded as `gap: unclear-invariant`. Promote that recurring tag into infrastructure: make `docs/invariants.md` the single, discoverable home for every subsystem contract that a bug has had to reconstruct, so the next bug in those areas finds the invariant instead of rebuilding it, and so the tag stops recurring.

**Approach.**

- Audit `docs/invariants.md` against the three Fixed entries cited below and ensure each reconstructed guarantee is stated as a named, test-cited section (Browse hierarchy, Now Playing label, stale reply store-time window), adding any that are missing.
- Add a short "How to use" note at the top of `docs/invariants.md` pointing readers (and the bugfix role) at it before reconstructing an invariant, and cross-reference it from the BUGS.md header's Validation gap description.
- No code change; this is a docs-only plan whose acceptance is `make check` plus the three cited guarantees each having a section.

**Cited entries.** `BUGS-FIXED.md`: "A browse reply is stored after a newer request was issued …" (unclear-invariant, store-time invariant), "Now Playing bar shows the track id instead of the song title" (unclear-invariant, Now Playing contract), and the bidirectional-browse entry (unclear-invariant, browse hierarchy).

**Files touched.** `docs/invariants.md`, `BUGS.md`.

**Acceptance criteria.**

- `make check` passes.
- Every `unclear-invariant` Fixed entry cited above has a matching section in `docs/invariants.md` naming the enforcing code and the tests that fail if it is removed.
- The BUGS.md header cross-references `docs/invariants.md` for the `unclear-invariant` tag.


## Done
