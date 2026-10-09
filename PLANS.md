# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; updated 2026-10-07).** The initial prompt
> names two commitments — a "very close if not pixel-perfect" classic Winamp UI
> and Apple Music as the library. The first is now moving: the base-skin
> palette landed ("Add a Winamp 2.x base-skin palette and apply it as the app
> theme", done 2026-10-06), and the reusable bevel layer followed ("Add a
> Winamp two-tone bevel layer and frame the Now Playing and equalizer panels",
> done 2026-10-06), framing those two panels through `bevel::lcd_well` and
> `bevel::raised_panel` in `src/ui/bevel.rs`. `## Planned` is no longer empty:
> it holds "Style the transport buttons as raised Winamp chrome", which
> restyles the five transport buttons and the browse Back button — all built
> through `labeled_button`, still a default iced `Button`. Still unplanned is
> the rest of widget-level fidelity: slider chrome (the bevel entry and the
> transport plan both defer it), a custom title bar, and playlist chrome. The
> library side is still all stub: `AppleMusicService` answers every browse
> query (`get_favorite_artists`, `get_albums_by_artist`,
> `get_songs_from_album`) from `sample_library()`, `play_track` only mutates
> shared state, and the token field is unset — and no PLANS.md entry plans the
> real integration. Risk: the longer Apple Music stays stubbed the more UI
> grows around the sample library's shape, while the remaining chrome work is
> easier to sequence now that the bevel layer exists. Recommend the plan loop
> keep scheduling the remaining fidelity plans (slider chrome, title bar,
> playlist chrome) and put the Apple Music integration plan on the near
> horizon.

## Planned

_None yet._

## Done

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

### Carry the Apple Music preview URL on `Song` (found 2026-10-08, done 2026-10-08)

The audio-output decision (QUESTIONS.md `## Answered`, 2026-10-08) scopes
playback to the Apple Music **preview** asset, so the model must carry its URL
before any backend can play it. Apple Music's resource attributes include a
`previews` array whose first entry's `url` is a short M4A; the browse client
currently reads only `name` and `durationInMillis` (the `Attributes` struct in
`src/apple_music/rest.rs`), and `Song` (`src/library.rs`) has no URL field.
This adds the field and parses it; it is the data half of "Add the audio-output
seam and a rodio backend", which depends on it. No audio dependency lands
here.

**Goal.** `Song` carries `preview_url: Option<String>` — the first non-blank
`attributes.previews[].url` from the browse response, `None` when the source
supplied none. The REST client fills it; the sample library and test fixtures
set `None`.

**Approach.**

- `src/library.rs`: add `pub preview_url: Option<String>` to `Song`, documented
  as the preview asset URL (`None` when the source supplied none), with
  `#[serde(default)]` so a payload without the field still deserializes. Update
  the `Song` wire-fixture tests: the full JSON payload gains `"preview_url"`,
  the round-trip and field-name assertions cover it, and the missing-field
  probe still omits a required key.
- `src/apple_music/rest.rs`: add `previews: Option<Vec<Preview>>` to
  `Attributes` and a private `struct Preview { url: Option<String> }`; add
  `Resource::preview_url(&self) -> Option<String>` returning the first
  preview's non-blank `url` (compared trimmed, returned untouched) and `None`
  when `previews` is absent, empty, or all blank. Set
  `preview_url: resource.preview_url()` in `get_songs_from_album`.
- `src/sample_library.rs`: every `Song { … }` literal gains
  `preview_url: None`.
- `src/test_support.rs`: `sample_song`, `stepping_songs`, `single_song_album`,
  and `second_album_songs` gain `preview_url: None`.
- Every other `Song { … }` literal (`src/apple_music/tests.rs`,
  `src/apple_music/rest/tests.rs`, `src/ui/tests.rs`, `src/library.rs`) gains
  the field.

**Files touched.** `src/library.rs`, `src/apple_music/rest.rs`,
`src/apple_music/rest/tests.rs`, `src/sample_library.rs`,
`src/test_support.rs`, plus the `Song` literals in `src/apple_music/tests.rs`
and `src/ui/tests.rs`.

**Acceptance criteria.**

- `make check` passes.
- A `get_songs_from_album` stub response carrying
  `"previews": [{"url": "https://example.test/preview.m4a"}]` maps that URL to
  the song's `preview_url`.
- A response with no `previews`, an empty `previews` array, or a blank first
  `url` maps to `preview_url == None`.
- Every `sample_library()` song has `preview_url == None`.
- A `Song` JSON payload without `preview_url` still deserializes.

### Name the page when a paginated Apple Music browse fails (found 2026-10-08, done 2026-10-08)

A browse fetch follows a collection's `next` link (up to `MAX_PAGES`), but a
failure on a later page read the same as a first-page failure. The UI report
names the query ("loading favorite artists") and the underlying cause, but not
which page failed, so a user with a multi-page library could not tell where the
request broke.

**Goal.** A transport or parse failure while following a `next` link names the
page it happened on, while a first-page failure keeps its bare cause (the query
the UI already labels).

**Approach.** `src/apple_music/rest.rs`: add a `page_context` helper and apply
it to both the transport and parse error paths in `fetch`. The first page is
returned unchanged; later pages get a ` (page N)` suffix.
`src/apple_music/rest/tests.rs`: pin the suffix with a two-page stub whose
second body is not JSON, and pin the first-page boundary with a direct
`page_context` unit test.

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`.

**Acceptance criteria.** `make check` passes; a page-2 parse failure reads
`response was not valid JSON: ... (page 2)`, and a first-page failure still
reads as the bare cause every existing transport/parse test asserts.

### Show each track's length in the playlist editor (found 2026-10-08, done 2026-10-08)

Classic Winamp's playlist editor lists every track with its length beside the
title. Nostalgia's Songs browse list — the sunken playlist editor framed by
"Frame the browse list as a sunken Winamp playlist editor" — renders each row
as `[title] [secondary label]`, and `song_row` fills that label with the
static "Play" hint. The last landing added `Song::duration_ms` and
`views::format_track_time`, but they reach only the Now Playing bar, so the
playlist still shows no lengths. This replaces the song row's "Play" hint
with the track's formatted length, so the list reads as a Winamp playlist; the
artist and album rows keep their "View Albums"/"View Songs" hints. A row
index column, right-aligning the length at the row edge, and per-frame
allocation avoidance stay out of scope: the shared row builder hugs its
content (so right-alignment would reflow every row), and a formatted number is
owned text.

**Goal.** Each Songs browse row shows its track's length as `m:ss` (`--:--`
when `duration_ms` is `0`) in the secondary-label slot, while the artist and
album rows keep their existing hints. No change to the model, the service
seam, navigation, transport, or the other panels.

**Approach.**

- `src/ui/views.rs`:
  - Widen the browse row tuple's second field from `&'static str` to
    `Cow<'a, str>` in `scrollable_list`, `browse_view`, `artist_row`,
    `album_row`, and `song_row`. `scrollable_list` already builds
    `Text::new(label)`; `iced_core`'s `IntoFragment` is implemented for
    `Cow<'a, str>`, so the widget call is unchanged. Add the `'a` lifetime to
    `artist_row` and `album_row`, which already borrow their title from the
    `&Artist`/`&Album` argument.
  - `artist_row` and `album_row` return `Cow::Borrowed("View Albums")` /
    `Cow::Borrowed("View Songs")`.
  - `song_row` returns `Cow::Owned(format_track_time(song.duration_ms))` in
    place of the `"Play"` literal.
  - Update the doc comments on `scrollable_list`, `song_row`, and the row
    tuple: the secondary label is an owned-or-borrowed `Cow`, and a song row's
    label is the track's length. Record the deliberate change that `song_row`
    now allocates one small `String` per song row per frame — the same
    per-frame allocation `now_playing_time` already makes — while the title
    still borrows.
  - In the `#[cfg(test)] mod tests`: change
    `song_row_uses_title_and_selects_the_song_by_index` to assert the label is
    the formatted duration of a `sample_song` (`3:30`), add
    `song_row_shows_an_unknown_duration_as_blank_time` for a `duration_ms == 0`
    song asserting `--:--`, and keep the artist/album row assertions (a `Cow`
    compares equal to a `&str`).

**Files touched.** `src/ui/views.rs` (including its `#[cfg(test)] mod tests`).

**Acceptance criteria.**

- `make check` passes.
- `song_row` for a song with `duration_ms = 210_000` yields the label `3:30`,
  and for `duration_ms = 0` yields `--:--`.
- `artist_row` still yields `View Albums` and `album_row` still yields
  `View Songs`.
- The Songs browse view still builds without panicking over the sample
  library: the existing `browse_views_construct_over_the_loaded_library` and
  `view_constructs_over_the_apps_full_input_space` tests pass.

### Read a later Apple Music error entry when the first names no cause (found 2026-10-08, done 2026-10-08)

Apple's error body can carry several `errors` entries, and the client read only
the first, so an entry carrying just a `status` hid a later entry's `detail` or
`title` and the browse failure report showed a bare `HTTP 401`.

**Goal.** `api_error_cause` returns the first non-blank cause across all
entries, keeping each entry's `detail`-before-`title` preference.

**Approach.** `src/apple_music/rest.rs`: replace the single `next()` entry with
`find_map` over `envelope.errors`, so a causeless entry falls through to the
next. `src/apple_music/rest/tests.rs`: pin the fall-through with a two-entry
body whose first entry has only a `status`.

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`.

**Acceptance criteria.** `make check` passes; a body whose first `errors` entry
names no cause yields the later entry's cause, while every existing
`api_error_cause` test still passes.

### Follow a browse collection's next page so a large library is read in full (found 2026-10-08, done 2026-10-08)

Apple caps a collection page at 100 items and links the next page in the
response's `next` field. The prior change only logged the unread page; the
client still returned the first page, so a library over one page was truncated.
This follows the link (up to a bound) and returns every page's rows.

**Goal.** `RestLibrary::fetch` follows `next` for every browse query, combining
the pages in order, without letting a hostile link redirect the session's
tokens or an endless chain loop forever.

**Approach.**

- `src/apple_music/rest.rs`: add `API_ORIGIN`, `MAX_PAGES` (10), and
  `absolute_next_url`, which resolves only a single-slash same-origin path
  against `API_ORIGIN`; rewrite `fetch` to loop up to `MAX_PAGES`, extending
  the rows and following each `next`, logging `page_bound_notice` at the bound
  and `unfollowable_next_notice` for a non-same-origin link.
- `src/test_support.rs`: add `StubTransport::returning_bodies` so a test can
  serve a sequence of responses (the existing stub repeated one body).
- `src/apple_music/rest/tests.rs`: pin the two notices, `absolute_next_url`,
  `API_BASE`/`API_ORIGIN` agreement, a two-page fetch, the page bound, and that
  a cross-host `next` is not followed.

**Acceptance criteria.**

- `make check` passes.
- A two-page response yields both pages' rows in order with two requests, the
  second to the resolved next URL.
- An endless `next` chain issues exactly `MAX_PAGES` requests.
- A `next` naming another host is not followed.

### Show the current track's duration in the Now Playing bar (found 2026-10-08, done 2026-10-08)

Classic Winamp's main LCD pairs the current track with a time, and its
playlist editor lists each track's length. Nostalgia's `Song` drops the
duration the Apple Music API already returns — `rest::Attributes` reads only
`name` — so no surface can show a time. This adds the duration to the model
and shows the current track's total as `m:ss` in the Now Playing bar, where
the title already survives a browse to another album. Elapsed counting and a
seek bar stay out of scope: they need the real playback clock the open backend
question gates (QUESTIONS.md, "What audio-output backend should Nostalgia use
for real playback?").

**Goal.** `Song` carries `duration_ms`, the REST browse client parses
`attributes.durationInMillis` into it, and the Now Playing bar shows the
current track's length as `m:ss` (`--:--` while unknown), surviving a browse
to another album exactly as the title does. No new dependency and no change to
transport, browse navigation, volume, balance, or the equalizer.

**Approach.**

- `src/library.rs`: add `pub duration_ms: u64` to `Song`, documented as the
  track's length in milliseconds with `0` meaning the source supplied none.
  Add it to `song_payload()`, and change
  `song_deserialization_ignores_unknown_fields`'s extra key from the now-known
  `"duration_ms"` to a still-unknown one (`"genre"`), so that test keeps
  pinning unknown-field tolerance.
- `src/test_support.rs`: give `sample_song`, `stepping_songs`,
  `single_song_album`, and `second_album_songs` durations.
- `src/sample_library.rs`: add `duration_ms` to every `Song { … }` literal.
- `src/apple_music/rest.rs`: add
  `#[serde(rename = "durationInMillis")] duration_in_millis: Option<u64>` to
  `Attributes`; add `Resource::duration_ms(&self) -> u64` returning it or `0`;
  map it in `get_songs_from_album`.
- `src/apple_music/rest/tests.rs` and `src/apple_music/tests.rs`: extend the
  `songs_from_album` stub bodies and expected `Song`s with
  `"durationInMillis"` / `duration_ms`; add a
  `songs_from_album_maps_a_missing_duration_to_zero` case.
- `src/ui/views.rs`: add `pub struct KnownTrack { pub title: String, pub
  duration_ms: u64 }`; change `now_playing_label` to take
  `&HashMap<String, KnownTrack>` and read `.title`; add
  `pub fn format_track_time(duration_ms: u64) -> String` (`"--:--"` for `0`,
  else zero-padded `m:ss`); `view_now_playing` gains a `time: String` laid at
  the right edge of the LCD well (a `Length::Fill` spacer before it).
- `src/ui/mod.rs`: replace `known_titles: HashMap<String, String>` with
  `known_tracks: HashMap<String, KnownTrack>`; the `TrackSelected` arm records
  both fields; the `TrackPlayed` prune is unchanged; add
  `WinampPlayer::now_playing_time` reading the map's `duration_ms`; `view`
  passes the formatted time to `view_now_playing`.
- `src/ui/tests.rs`: rename the `known_titles` references to `known_tracks`
  (reading `.title`); add a test that a played track's duration reaches
  `now_playing_time` and survives browsing to another album.

- `BUGS.md`: update the two Fixed records that describe the live Now
  Playing lookup as `known_titles` (the id→title index renamed to
  `known_tracks`, now carrying title and duration).

**Files touched.** `src/library.rs`, `src/test_support.rs`,
`src/sample_library.rs`, `src/apple_music/rest.rs`,
`src/apple_music/rest/tests.rs`, `src/apple_music/tests.rs`,
`src/ui/views.rs`, `src/ui/mod.rs`, `src/ui/tests.rs`, `BUGS.md`.

**Acceptance criteria.**

- `make check` passes.
- A REST song resource carrying `attributes.durationInMillis` maps to
  `Song::duration_ms`; one without maps to `0`.
- `format_track_time(0)` is `--:--` and `format_track_time(210_000)` is
  `3:30`.
- The Now Playing bar shows `--:--` with no current track and the played
  track's `m:ss` after a play, and that time survives browsing to another
  album.

### Log a notice when a browse response has an unread next page (found 2026-10-08, done 2026-10-08)

Apple caps a collection page at 100 items and links the next page in the
response's `next` field, but `rest::Envelope` dropped that field, so a library
larger than one page was truncated silently: the user saw the first 100
artists (or songs) with nothing to say the rest existed. Following the link is
a pagination feature this change does not attempt; it makes the truncation
visible instead.

**Goal.** When a collection response carries `next`, log a line naming the
unread page, without changing what the browse queries return.

**Approach.**

- `src/apple_music/rest.rs`: add `next: Option<String>` to `Envelope`; in
  `RestLibrary::fetch`, when `next` is `Some`, log `truncation_notice(next)` to
  stderr and still return the first page's `data`. `truncation_notice` is a
  pure function that formats `next` with `Debug`, so a server-controlled
  control character or Unicode format character is escaped before it reaches
  the terminal, matching `api_error_cause` and `play_log_line`.
- `src/apple_music/rest/tests.rs`: pin the notice's wording, its escaping of
  `\u{1b}` and `\u{202e}`, and that a response with `next` still returns only
  the first page and issues one request.

**Acceptance criteria.**

- `make check` passes.
- A response with a `next` field returns the same first page it did before and
  logs the notice; a response without `next` logs nothing (the `if let`
  guard).

### Add a `make test-one` target for running one focused test (found 2026-10-08, done 2026-10-08)

The Makefile's declared entry point for tests was only `make test`, the full
`cargo test --locked` suite. Iterating on one behavior meant either waiting
for every test or remembering the raw `cargo test --locked -- <name>`
incantation, so a single focused test had no first-class way to run through
the project's own tooling.

**Goal.** Run only the tests matching a name, through the Makefile, without
weakening the full-suite `test` target the landing gate uses.

**Approach.**

- `Makefile`: add a `test-one` target that forwards `TEST` to cargo's test
  harness (`cargo test --locked -- $(TEST)`), and a guard that fails with a
  usage line when `TEST` is empty instead of silently running the whole suite.
  `test-one` is added to `.PHONY`; the existing `test` target and the
  `check: fmt lint docs test` chain are unchanged.

**Acceptance criteria.**

- `make check` passes (the full-suite `test` target is unchanged).
- `make test-one TEST=<name>` runs only the matching tests, and
  `make test-one` with no `TEST` exits non-zero with a usage line.

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
