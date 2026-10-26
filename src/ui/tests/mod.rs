use super::browse::BrowseList;
use super::loading::{RequestGeneration, fetch_into, play_into};
use super::timers::*;
use super::views::{BROWSE_LIST_ID, BROWSE_PAGE_HEIGHT, BROWSE_SEARCH_ID, BROWSE_VIEWS};
use super::*;
use crate::audio::{PlaybackFormat, Spectrum};
use crate::equalizer::{BAND_COUNT, GAIN_MAX_DB, GAIN_MIN_DB};
use crate::library::{Album, Artist, Playlist, Song};
use crate::sample_library::sample_library;
use crate::settings::Settings;
use crate::settings::file_store::SettingsWriter;
use crate::state::TransportMode;
use crate::test_support::{
    AudioCall, FailingAudio, FailingCommand, PREVIEW_URL, RecordingAudio, StubTransport,
    assert_ids, assert_preview_calls, recording_spectrum, rock_preset, sample_album, sample_artist,
    sample_playlist, sample_song, second_album_songs, sign_in_session, stepping_songs,
};

use std::sync::atomic::AtomicU64;
use std::time::Duration;

/// A fresh player over its own shared state, so a test can inspect the
/// same `AppState` the player mutates.
fn test_player() -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let state = Arc::new(Mutex::new(AppState::default()));
    // Over a stub transport and the silent audio backend, so a UI test never
    // touches the network or an audio device; no session is stored, so the
    // sample library answers the browse queries.
    let service =
        AppleMusicService::with_transport(state.clone(), Box::new(StubTransport::returning("")));
    (WinampPlayer::new(state.clone(), service), state)
}

/// A fresh player over an injected playback backend and its own shared state,
/// so a test can observe what the UI hands the service's audio seam.
fn player_with_audio(
    audio: Arc<dyn crate::audio::AudioOutput>,
) -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let state = Arc::new(Mutex::new(AppState::default()));
    let service =
        AppleMusicService::with_audio(state.clone(), Box::new(StubTransport::returning("")), audio);
    (WinampPlayer::new(state.clone(), service), state)
}

/// A fresh player over `recording`, already playing `song-1`'s
/// [`PREVIEW_URL`] — the precondition the transport-test setups build before
/// driving a Play/Pause/Stop message. The player construction, the preview
/// load, and the URL live here once; a test that asserts on the recording
/// keeps its own `Arc` clone.
async fn player_playing_preview(
    recording: Arc<RecordingAudio>,
) -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let (player, state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
    player
        .apple_music_service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap();
    (player, state)
}

/// A fresh request generation over its own counter — the starting
/// generation the `fetch_into` and `play_into` tests issue before driving
/// their task, so no two tests share a counter.
fn test_generation() -> RequestGeneration {
    RequestGeneration::issue(&Arc::new(AtomicU64::new(0)))
}

/// A fresh player over the shared three-song stepping album
/// ([`stepping_songs`]) with `current` recorded as the shared state's
/// current track — the starting shape the Next/Previous wiring tests step
/// from. Nine tests build this same `test_player`-plus-fixture setup; they
/// differ in the starting track (or its absence) and, for the two Shuffle
/// tests, the shared Shuffle flag, so it lives here once.
fn player_stepping_from(current: Option<&str>) -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let (mut player, state) = test_player();
    player.songs.items = stepping_songs();
    state.blocking_lock().current_track = current.map(str::to_string);
    (player, state)
}

/// A player over `recording`, showing the stepping album with `current`
/// playing — the precondition the preview-end tests drive a finished reply
/// from. The songs carry no preview URL, so a committed play stops the
/// previous backend rather than starting a preview, keeping these tests off
/// the network.
fn player_playing_stepping_song(
    recording: &Arc<RecordingAudio>,
    current: &str,
) -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let (mut player, state) =
        player_with_audio(Arc::clone(recording) as Arc<dyn crate::audio::AudioOutput>);
    player.songs.items = stepping_songs();
    {
        let mut state = state.blocking_lock();
        state.current_track = Some(current.to_string());
        state.play();
    }
    (player, state)
}

/// Drives a finished reply at the player's current epoch and returns the
/// message the handler schedules, as iced would deliver it.
fn drive_playback_finished(player: &mut WinampPlayer) -> Message {
    let epoch = player.position_epoch;
    let task = update(
        player,
        Message::PlaybackFinished {
            epoch,
            finished: true,
        },
    );
    futures::executor::block_on(task_output(task, "preview end"))
}

/// Feeds a committed play of `track_id` through `Message::TrackPlayed`, as the
/// playback task would after a successful `TrackSelected`: the shared state
/// names the track and is playing, and the completion carries the current play
/// generation, so the arm records the track in the player's play history.
fn record_a_play(player: &mut WinampPlayer, state: &Arc<Mutex<AppState>>, track_id: &str) {
    {
        let mut state = state.blocking_lock();
        state.current_track = Some(track_id.to_string());
        state.is_playing = true;
    }
    let generation = player
        .plays_generation
        .load(std::sync::atomic::Ordering::SeqCst);
    let _ = update(
        player,
        Message::TrackPlayed {
            generation,
            committed: true,
        },
    );
}

/// A fresh player showing the Albums view: it seeds the one
/// [`sample_artist`] and drives the `ArtistSelected` press that artist's
/// first row emits (index 0 of epoch 0), the step
/// `artist_selected_flips_to_albums_view` and the back-navigation tests that
/// start from Albums share. The fetch task the arm schedules is
/// dropped — those tests pin the view transition, not the fetch, which
/// `artist_selected_fetches_the_artists_albums_into_the_player` covers with
/// its own `update` call.
fn player_in_albums() -> WinampPlayer {
    let (mut player, _state) = test_player();
    player.artists.items = vec![sample_artist()];
    let _ = update(&mut player, Message::ArtistSelected { epoch: 0, index: 0 });
    player
}

/// A fresh player with a resolved window id — the precondition the title-bar
/// action and window-shade tests need before driving a message, since
/// [`test_player`] leaves `window_id` as `None`. The id is unique per call,
/// matching the one `iced::window::latest()` resolves at boot.
fn player_with_window_id() -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let (mut player, state) = test_player();
    player.window_id = Some(iced::window::Id::unique());
    (player, state)
}

/// Seeds `player`'s browse buffers with the shared [`sample_artist`],
/// [`sample_album`], [`sample_song`], and [`sample_playlist`] fixtures — the
/// "loaded" browse shape. The failure-arm test and the two view-construction
/// tests all build this same six-list setup before exercising their own
/// contract, so it lives here once.
fn seed_browse_lists(player: &mut WinampPlayer) {
    player.artists.items = vec![sample_artist()];
    player.albums.items = vec![sample_album()];
    player.songs.items = vec![sample_song()];
    player.all_songs.items = vec![sample_song()];
    player.all_albums.items = vec![sample_album()];
    player.playlists.items = vec![sample_playlist()];
}

/// Asserts the player has no rows in any of its browse buffers. The
/// failure-arm test (which clears them all) and the startup test (which
/// begins with them all empty) both pin this same "nothing loaded" shape, so
/// the field check lives here once.
fn assert_browse_lists_empty(player: &WinampPlayer) {
    assert!(player.artists.items.is_empty());
    assert!(player.albums.items.is_empty());
    assert!(player.songs.items.is_empty());
    assert!(player.all_songs.items.is_empty());
    assert!(player.all_albums.items.is_empty());
    assert!(player.playlists.items.is_empty());
}

/// Asserts the player is showing `expected`. The tests pin the current
/// view at each navigation step — `ArtistSelected`, `AlbumSelected`,
/// `PlaylistSelected`, the flat-list `Show*` arms, and `Back` — and at
/// startup, so the same view-equality check lives here once instead of
/// spelling out a `matches!` at every site.
fn assert_view(player: &WinampPlayer, expected: CurrentView) {
    assert_eq!(player.current_view, expected);
}

/// Asserts `message` flips the shared-state flag `read_flag` from false to
/// true and back: the `PlayPause`, `ToggleRepeat`, `ToggleShuffle`, and
/// `ToggleEqualizer` update
/// arms all do the same one-flag flip, differing only in which flag they
/// read, so the lock-read-update sequence lives here once. The arms use
/// `blocking_lock`, which panics inside an async runtime, so this stays a
/// plain (non-async) helper.
fn assert_toggles_shared_state(message: Message, read_flag: impl Fn(&AppState) -> bool) {
    let (mut player, state) = test_player();
    assert!(!read_flag(&state.blocking_lock()));

    let _ = update(&mut player, message.clone());
    assert!(read_flag(&state.blocking_lock()));

    let _ = update(&mut player, message);
    assert!(!read_flag(&state.blocking_lock()));
}

/// Drives a slider-change message through `update` and asserts the clamped
/// value lands in shared state. The `VolumeChange`, `BalanceChange`,
/// `EqPreampChange`, and `EqBandChange` arms all do the same one-value write —
/// the setter clamps the slider's value before storing it — so the
/// lock-read-assert sequence lives here once and each call only names its
/// message, the field read back, and the clamped value. The arms use
/// `blocking_lock`, which panics inside an async runtime, so this stays a
/// plain (non-async) helper, like [`assert_toggles_shared_state`].
fn assert_message_clamps(message: Message, read: impl Fn(&AppState) -> f32, expected: f32) {
    let (mut player, state) = test_player();
    let _ = update(&mut player, message);
    assert_eq!(read(&state.blocking_lock()), expected);
}

/// An in-memory [`SettingsWriter`] that records every saved snapshot, so the
/// update loop's save-on-change behavior is asserted without disk I/O. Cloning
/// the store shares the recording, letting a test keep a handle after attaching
/// a boxed clone to the player.
#[derive(Clone, Default)]
struct MemorySettingsStore {
    saved: Arc<std::sync::Mutex<Vec<Settings>>>,
}

impl MemorySettingsStore {
    /// The snapshots saved so far, in save order.
    fn snapshots(&self) -> Vec<Settings> {
        self.saved.lock().unwrap().clone()
    }
}

impl SettingsWriter for MemorySettingsStore {
    fn save(&self, settings: &Settings) {
        self.saved.lock().unwrap().push(settings.clone());
    }
}

/// The longest a driven `Task` is allowed to take before its test fails.
///
/// A task that never yields its completion message — a hung service future,
/// say — would otherwise block `drive_task` forever and stall the whole test
/// run with no indication of which task hung. This bound turns that stall
/// into a named failure.
const TASK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Awaits `future`, panicking if it does not resolve within `timeout`.
///
/// `drive_task` uses this to bound its wait on a task's completion message.
/// The timer is a detached thread that sleeps for `timeout` and wakes the
/// wait through a oneshot channel, so the bound works both under
/// `futures::executor::block_on` and inside a `#[tokio::test]` runtime
/// without depending on a runtime timer. `what` names the awaited task in the
/// panic message so a hang is diagnosable.
async fn await_or_timeout<F: std::future::Future>(
    what: &str,
    timeout: std::time::Duration,
    future: F,
) -> F::Output {
    let (timer, fired) = futures::channel::oneshot::channel::<()>();
    let _handle = std::thread::spawn(move || {
        std::thread::sleep(timeout);
        let _ = timer.send(());
    });
    futures::pin_mut!(future);
    match futures::future::select(future, fired).await {
        futures::future::Either::Left((output, _)) => output,
        futures::future::Either::Right(_) => {
            panic!("{what} task did not complete within {timeout:?}")
        }
    }
}

/// Drives an iced `Task` to completion and hands its single `Output`
/// action to `check`. `update` only schedules work as a `Task`, so a test
/// that wants to observe the resulting message — `TrackPlayed` after
/// playback, a `*Loaded` message after a fetch, a `TrackSelected` after a
/// step — must run the task itself the way the iced runtime would. The
/// playback, fetch, and (through `assert_track_selected`) stepping tests
/// do exactly that, so the stream plumbing and the "exactly one `Output`"
/// assertion live here once instead of at each call site. A batched task — a
/// browse fetch plus its snap-to-top scroll operation — yields the widget
/// effect before the fetch's output, so non-output actions are skipped. The
/// wait is bounded by [`TASK_TIMEOUT`], so a task that hangs fails this test
/// by name rather than stalling the suite.
async fn drive_task(task: Task<Message>, what: &str, mut check: impl FnMut(Message)) {
    use futures::StreamExt;

    let mut stream = iced_runtime::task::into_stream(task).expect("task must schedule a stream");
    loop {
        let action = await_or_timeout(what, TASK_TIMEOUT, stream.next())
            .await
            .unwrap_or_else(|| panic!("{what} task ended without a completion message"));
        if let iced_runtime::Action::Output(message) = action {
            check(message);
            return;
        }
    }
}

/// Drives `task` to completion and returns its single output message — the
/// message the iced runtime would deliver. [`drive_task`] hands a task's
/// output to a callback for inspection; the tests that must feed that output
/// back through `update` (the two slow-stale browse tests, the superseded
/// mapper-to-update test, the two play-completion tests, and
/// [`drive_fetch_and_assert_loaded`]) instead need the message itself, so
/// the capture-and-return plumbing lives here once. `what` names the task
/// in `drive_task`'s timeout panic. Callers on a plain thread reach this
/// through `futures::executor::block_on`, the same way the stepping tests
/// drive `drive_task`.
async fn task_output(task: Task<Message>, what: &str) -> Message {
    let mut output: Option<Message> = None;
    drive_task(task, what, |message| output = Some(message)).await;
    output.expect("drive_task must pass the output to its callback")
}

/// Drives `task` to its end on the current thread and returns every output
/// message it emitted.
///
/// [`task_output`] stops at the task's single output; the tick tests instead
/// need all of a batched task's replies — a position query plus whichever of
/// the format, length, and finished queries still apply — so the
/// drain-and-collect loop lives here once. A task that schedules no stream
/// fails by name, and each wait is bounded by [`TASK_TIMEOUT`].
fn task_outputs(task: Task<Message>, what: &str) -> Vec<Message> {
    use futures::StreamExt;

    let mut stream = iced_runtime::task::into_stream(task)
        .unwrap_or_else(|| panic!("{what} must schedule a task"));
    let mut replies = Vec::new();
    while let Some(action) =
        futures::executor::block_on(await_or_timeout(what, TASK_TIMEOUT, stream.next()))
    {
        if let iced_runtime::Action::Output(message) = action {
            replies.push(message);
        }
    }
    replies
}

/// The browse-list scroll operations a task emitted: a relative snap-to-top,
/// an absolute scroll-to-row, and a relative page scroll are captured
/// separately, so a test pins which one an arm schedules (and that it
/// schedules no other).
#[derive(Default)]
struct BrowseScrollEffects {
    snapped: Option<iced::advanced::widget::operation::scrollable::RelativeOffset<Option<f32>>>,
    scrolled: Option<iced::advanced::widget::operation::scrollable::AbsoluteOffset<Option<f32>>>,
    scrolled_by: Option<iced::advanced::widget::operation::scrollable::AbsoluteOffset>,
}

impl BrowseScrollEffects {
    fn any(&self) -> bool {
        self.snapped.is_some() || self.scrolled.is_some() || self.scrolled_by.is_some()
    }
}

/// Applies an iced widget operation to a capture under `id` and returns the
/// browse scroll effects it produced. An operation aimed at another id must
/// produce none, so each caller checks the unrelated id too.
fn browse_scroll_effects_by(
    operation: &mut dyn iced::advanced::widget::Operation<()>,
    id: &iced::advanced::widget::Id,
    bounds: iced::Rectangle,
    translation: iced::Vector,
) -> BrowseScrollEffects {
    use iced::advanced::widget::operation::scrollable::{
        AbsoluteOffset, RelativeOffset, Scrollable as ScrollableOperation,
    };

    #[derive(Default)]
    struct Capture {
        effects: BrowseScrollEffects,
    }

    impl ScrollableOperation for Capture {
        fn snap_to(&mut self, offset: RelativeOffset<Option<f32>>) {
            self.effects.snapped = Some(offset);
        }

        fn scroll_to(&mut self, offset: AbsoluteOffset<Option<f32>>) {
            self.effects.scrolled = Some(offset);
        }

        fn scroll_by(
            &mut self,
            offset: AbsoluteOffset,
            _bounds: iced::Rectangle,
            _content_bounds: iced::Rectangle,
        ) {
            self.effects.scrolled_by = Some(offset);
        }
    }

    let mut capture = Capture::default();
    operation.scrollable(
        Some(id),
        bounds,
        iced::Rectangle::default(),
        translation,
        &mut capture,
    );
    capture.effects
}

/// Drives `task` until it runs a browse-list scroll operation, returning the
/// effects it captured, or until the task ends with none. A batched browse
/// fetch (or a batched stepping selection) and its scroll effect are separate
/// stream actions, so the operation is captured under the browse list's id and
/// under an unrelated id: only the first may produce an effect. The task may
/// emit non-widget actions first, so they are skipped rather than treated as
/// the scroll.
async fn browse_scroll_effects(task: Task<Message>, what: &str) -> BrowseScrollEffects {
    browse_scroll_effects_in(
        task,
        what,
        iced::Rectangle::default(),
        iced::Vector::default(),
    )
    .await
}

/// Like [`browse_scroll_effects`], but the list reports a viewport of
/// `viewport_height` scrolled to `scroll_top`, for operations that read them.
async fn browse_scroll_effects_at(
    task: Task<Message>,
    what: &str,
    viewport_height: f32,
    scroll_top: f32,
) -> BrowseScrollEffects {
    let bounds = iced::Rectangle {
        height: viewport_height,
        ..iced::Rectangle::default()
    };
    browse_scroll_effects_in(task, what, bounds, iced::Vector::new(0.0, scroll_top)).await
}

async fn browse_scroll_effects_in(
    task: Task<Message>,
    what: &str,
    bounds: iced::Rectangle,
    translation: iced::Vector,
) -> BrowseScrollEffects {
    use futures::StreamExt;
    use iced::advanced::widget::Id;

    let mut stream = iced_runtime::task::into_stream(task)
        .unwrap_or_else(|| panic!("{what} must schedule a task"));
    loop {
        let Some(action) = await_or_timeout(what, TASK_TIMEOUT, stream.next()).await else {
            return BrowseScrollEffects::default();
        };
        let iced_runtime::Action::Widget(mut operation) = action else {
            continue;
        };
        let matching = browse_scroll_effects_by(
            &mut operation,
            &Id::new(BROWSE_LIST_ID),
            bounds,
            translation,
        );
        let unrelated = browse_scroll_effects_by(
            &mut operation,
            &Id::new("not-the-browse-list"),
            bounds,
            translation,
        );
        assert!(
            !unrelated.any(),
            "{what} must scroll only the browse list id"
        );
        if matching.any() {
            return matching;
        }
    }
}

/// Drives `task` until it runs an iced scroll operation and asserts that
/// operation snaps the browse list to `offset`. This distinguishes the real
/// scroll task from any other follow-up work a navigation arm might schedule,
/// and pins that the same arm does not also scroll to a row.
async fn assert_task_snaps_browse_list_to(
    offset: iced::advanced::widget::operation::scrollable::RelativeOffset<Option<f32>>,
    task: Task<Message>,
    what: &str,
) {
    let effects = browse_scroll_effects(task, what).await;
    assert_eq!(
        effects.snapped,
        Some(offset),
        "{what} must snap the browse list to the expected row"
    );
    assert_eq!(
        effects.scrolled, None,
        "{what} must not scroll the browse list to a row"
    );
    assert_eq!(
        effects.scrolled_by, None,
        "{what} must not scroll the browse list by a page"
    );
}

/// Drives `task` until it runs an iced scroll operation and asserts that
/// operation snaps the browse list to its first row.
async fn assert_task_snaps_browse_list_to_top(task: Task<Message>, what: &str) {
    use iced::advanced::widget::operation::scrollable::RelativeOffset;

    assert_task_snaps_browse_list_to(RelativeOffset::START.into(), task, what).await;
}

/// Drives `task` until it runs an iced scroll operation and asserts that
/// operation snaps the browse list to its last row.
async fn assert_task_snaps_browse_list_to_end(task: Task<Message>, what: &str) {
    use iced::advanced::widget::operation::scrollable::RelativeOffset;

    assert_task_snaps_browse_list_to(RelativeOffset::END.into(), task, what).await;
}

/// Drives `task` until it runs an iced scroll operation and asserts that
/// operation reveals the browse row at absolute offset `expected` in a list
/// reporting `viewport_height` scrolled to `scroll_top`.
async fn assert_task_reveals_browse_row(
    task: Task<Message>,
    what: &str,
    viewport_height: f32,
    scroll_top: f32,
    expected: f32,
) {
    use iced::advanced::widget::operation::scrollable::AbsoluteOffset;

    let effects = browse_scroll_effects_at(task, what, viewport_height, scroll_top).await;
    assert_eq!(
        effects.scrolled,
        Some(AbsoluteOffset {
            x: None,
            y: Some(expected),
        }),
        "{what} must scroll only as far as needed to reveal its row"
    );
    assert_eq!(
        effects.snapped, None,
        "{what} must not snap the browse list to the top"
    );
    assert_eq!(
        effects.scrolled_by, None,
        "{what} must not scroll the browse list by a page"
    );
}

/// Drives `task` until it runs an iced scroll operation and asserts that
/// operation scrolls the browse list by `expected_y` pixels from its current
/// offset, without snapping or scrolling to an absolute row.
async fn assert_task_scrolls_browse_list_by(task: Task<Message>, what: &str, expected_y: f32) {
    use iced::advanced::widget::operation::scrollable::AbsoluteOffset;

    let effects = browse_scroll_effects(task, what).await;
    assert_eq!(
        effects.scrolled_by,
        Some(AbsoluteOffset {
            x: 0.0,
            y: expected_y,
        }),
        "{what} must scroll the browse list by {expected_y} pixels"
    );
    assert_eq!(
        effects.snapped, None,
        "{what} must not snap the browse list to the top"
    );
    assert_eq!(
        effects.scrolled, None,
        "{what} must not scroll the browse list to a row"
    );
}

/// Drives `task` until it runs an iced widget operation and returns it, so a
/// test can apply the operation to a laid-out widget tree.
async fn widget_operation(
    task: Task<Message>,
    what: &str,
) -> Box<dyn iced::advanced::widget::Operation> {
    use futures::StreamExt;

    let mut stream = iced_runtime::task::into_stream(task)
        .unwrap_or_else(|| panic!("{what} must schedule a task"));
    loop {
        let Some(action) = await_or_timeout(what, TASK_TIMEOUT, stream.next()).await else {
            panic!("{what} task ended without a widget operation");
        };
        if let iced_runtime::Action::Widget(operation) = action {
            return operation;
        }
    }
}

/// Drives `task` to its end and asserts it never scrolls the browse list,
/// neither to the top nor to a row.
async fn assert_task_scrolls_nothing(task: Task<Message>, what: &str) {
    let effects = browse_scroll_effects(task, what).await;
    assert_eq!(
        effects.scrolled, None,
        "{what} must not scroll the browse list to a row"
    );
    assert_eq!(
        effects.snapped, None,
        "{what} must not snap the browse list to the top"
    );
    assert_eq!(
        effects.scrolled_by, None,
        "{what} must not scroll the browse list by a page"
    );
}

/// Unwraps the [`Message::BrowseReply`] stamp `fetch_into` puts on every
/// browse reply into the `*Loaded`/`*LoadFailed` message it carries. A test
/// inspecting a fetch's mapped message unwraps the stamp first.
fn browse_reply(message: Message) -> Message {
    match message {
        Message::BrowseReply { reply, .. } => *reply,
        other => panic!("expected a browse reply, got {other:?}"),
    }
}

/// Drives the task a browse arm schedules, feeds the resulting `*Loaded`
/// message back through the update loop, and asserts the list lands in
/// the player's matching buffer in library order. The fetch tests
/// — artists, an artist's albums, an album's songs, the flat lists, the
/// playlists list, a playlist's songs, and a library search — all repeat this
/// drive-then-feed-then-assert flow; only the buffer, the id key, and the
/// expected ids differ, so they come in as parameters and the flow lives
/// here once.
///
/// The task's own output message is fed straight back, rather than a
/// reconstructed copy: the fetch tests pass no match-and-clone closure naming
/// their `*Loaded` variant plus that variant's constructor, and this helper
/// takes neither — the task's message serves as both.
/// Feeding the message unchanged also proves the arm's task yields the
/// `*Loaded` variant whose buffer is asserted — a wrong variant would leave
/// the expected buffer empty.
async fn drive_fetch_and_assert_loaded<T, Buffer, Key>(
    player: &mut WinampPlayer,
    task: Task<Message>,
    what: &str,
    buffer: Buffer,
    key: Key,
    expected_ids: &[&str],
) where
    Buffer: Fn(&mut WinampPlayer) -> &mut Vec<T>,
    Key: Fn(&T) -> &str,
{
    let message = task_output(task, what).await;
    let _ = update(player, message);
    assert_ids(buffer(player).as_slice(), key, expected_ids);
}

/// Asserts the "entering a level clears it" contract shared by
/// `artist_selected_clears_the_previous_artists_albums` and
/// `album_selected_clears_the_previous_albums_songs`: the row press empties
/// the child buffer and marks it loading, dropping any error; the reply that
/// replaces the list gets a fresh epoch; and a press still carrying the old
/// list's epoch schedules no work rather than resolving its index against the
/// new list. `child` names the buffer under test, `select` is the row press,
/// `next_reply` the `*Loaded` message that replaces the list, and
/// `stale_press` builds the old-epoch press from the captured epoch. The
/// caller seeds the parent list and the child's first reply before calling.
fn assert_entering_a_level_clears_the_child_list<T>(
    player: &mut WinampPlayer,
    child: impl Fn(&mut WinampPlayer) -> &mut BrowseList<T>,
    select: Message,
    next_reply: Message,
    stale_press: impl Fn(u64) -> Message,
) {
    let previous_epoch = child(player).epoch;

    let _ = update(player, select);

    assert!(child(player).items.is_empty());
    assert!(child(player).loading);
    assert!(child(player).error.is_none());

    // The reply the fetch schedules lands with an epoch this list has never
    // used, and a press from the previous list is rejected rather than
    // resolving its index against the new one.
    let _ = update(player, next_reply);
    assert!(!child(player).loading);
    assert_ne!(child(player).epoch, previous_epoch);
    assert_no_task(update(player, stale_press(previous_epoch)));
}

/// Lays out the real root `view` and records every scrollable's id, so a test
/// can tell whether the stable `BROWSE_LIST_ID` scroll target is present.
fn root_scrollable_ids(player: &WinampPlayer) -> Vec<Option<iced::advanced::widget::Id>> {
    use iced::Rectangle;
    use iced::advanced::Layout;
    use iced::advanced::widget::{
        Id, Tree,
        operation::{Operation, Scrollable},
    };

    let renderer = layout_capture::headless_renderer();
    let mut element = view(player);
    let mut tree = Tree::new(element.as_widget());
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &layout_capture::limits());

    #[derive(Default)]
    struct ScrollCapture(Vec<Option<Id>>);
    impl Operation for ScrollCapture {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn scrollable(
            &mut self,
            id: Option<&Id>,
            _bounds: Rectangle,
            _content_bounds: Rectangle,
            _translation: iced::Vector,
            _state: &mut dyn Scrollable,
        ) {
            self.0.push(id.cloned());
        }
    }

    let mut capture = ScrollCapture::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut capture);
    capture.0
}

/// Asserts the "a superseded browse reply is dropped" contract shared by
/// `a_slow_stale_browse_reply_does_not_overwrite_the_newer_list` and
/// `a_slow_stale_browse_failure_does_not_overwrite_the_newer_list_or_error`:
/// once `newer` has landed and stored its list, driving the earlier `stale`
/// reply — whether it carries a success or a failure — yields
/// `Message::Ignored`, leaves `expected_ids` in the albums buffer, and records
/// no error. The caller seeds the parent artists list and issues both album
/// fetches before calling, passing the later-issued task as `newer` and the
/// earlier one as `stale`.
async fn assert_stale_album_reply_is_dropped(
    player: &mut WinampPlayer,
    newer: Task<Message>,
    stale: Task<Message>,
    expected_ids: &[&str],
) {
    let newer = task_output(newer, "newer album fetch").await;
    let _ = update(player, newer);
    assert_ids(
        &player.albums.items,
        |album| album.id.as_str(),
        expected_ids,
    );

    let stale = task_output(stale, "stale album fetch").await;
    assert!(matches!(stale, Message::Ignored));
    let _ = update(player, stale);
    assert_ids(
        &player.albums.items,
        |album| album.id.as_str(),
        expected_ids,
    );
    assert!(player.albums.error.is_none());
}

// Startup wiring: `boot` hands iced a fresh player plus a batched task that
// loads the artist list and resolves the window id, and `update`'s
// `LoadArtists` arm runs that fetch into the player's `artists` buffer. A
// regression that stopped boot from emitting the fetch — or pointed the arm at
// the wrong fetch — would open the app with an empty browse list, so the
// wiring is pinned end to end.

/// Drives a `boot` task's stream until `expected` `LoadArtists` outputs have
/// landed, then returns. `boot` batches the artists fetch with the window-id
/// query, whose runtime `Action::Window` this test runtime does not service,
/// so [`drive_task`]'s single-output drive cannot be used; the wait is
/// bounded by [`TASK_TIMEOUT`]. The two boot tests below both count
/// `LoadArtists` outputs — one for the initial fetch, two once the session
/// wait re-issues it — so the counting loop lives here once.
async fn await_boot_artist_loads(task: Task<Message>, expected: usize) {
    use futures::StreamExt;

    let mut stream = iced_runtime::task::into_stream(task).expect("boot must schedule work");
    let mut artist_loads = 0;
    while artist_loads < expected {
        let action = await_or_timeout("boot", TASK_TIMEOUT, stream.next())
            .await
            .expect("boot task must yield the artists fetch");
        if matches!(action, iced_runtime::Action::Output(Message::LoadArtists)) {
            artist_loads += 1;
        }
    }
}

// The Previous/Next buttons read the player's songs buffer and the shared
// current track, hand the direction to the transport helper, and schedule
// the stepped track as `Message::TrackSelected` (or no task when there is
// nothing to step through). The stepping arithmetic is tested in
// `transport.rs`; these tests pin the arm's wiring — that it reads the
// player's state and forwards the stepped track. The arms use
// `blocking_lock`, which panics inside an async runtime, so each test
// calls `update` on a plain thread and only then drives the returned
// task's stream.
/// Drives `task` to its single output and asserts it is a `TrackSelected`
/// for the given epoch and index into the player's songs, as the iced
/// runtime would deliver the button's task. The stepping tests drive their
/// tasks exactly the way the playback and fetch tests do, so they route
/// through [`drive_task`] instead of repeating the `into_stream`/`next`
/// plumbing; this stays a plain function (rather than `async`) because the
/// stepping arms use `blocking_lock`, which panics inside an async runtime.
fn assert_track_selected(task: Task<Message>, expected_epoch: u64, expected_index: usize) {
    futures::executor::block_on(drive_task(task, "stepping", |message| match message {
        Message::TrackSelected { epoch, index } => {
            assert_eq!(epoch, expected_epoch);
            assert_eq!(index, expected_index);
        }
        other => panic!("unexpected stepping task output: {other:?}"),
    }));
}

/// Drives `task` to its single output and asserts it is a `TrackPlayed`
/// completion carrying `expected_generation` and `expected_committed`, as the
/// iced runtime would deliver the play task's completion. The two `play_into`
/// tests both drive a play to completion and pin the same contract — the
/// completion carries the generation the play was issued as, so `update` can
/// tell a superseded play's completion from the current one, and whether the
/// play itself committed, so a failed play is not recorded — so the
/// drive-and-assert block lives here once. Async, unlike
/// [`assert_track_selected`], because the `play_into` tests run inside a tokio
/// runtime.
async fn assert_track_played(
    task: Task<Message>,
    expected_generation: u64,
    expected_committed: bool,
) {
    drive_task(task, "play", |message| match message {
        Message::TrackPlayed {
            generation,
            committed,
        } => {
            assert_eq!(generation, expected_generation);
            assert_eq!(committed, expected_committed);
        }
        other => panic!("unexpected play task output: {other:?}"),
    })
    .await;
}

/// Drives `task` to its single output and asserts it is
/// `Message::TransportSettled`, the completion the four transport arms emit.
/// The arms all schedule the same payload-less completion, so the
/// drive-and-assert block lives here once and each call site only names the
/// action it drove.
async fn assert_transport_settled(task: Task<Message>, what: &str) {
    drive_task(task, what, |message| {
        assert!(matches!(message, Message::TransportSettled));
    })
    .await;
}

/// Asserts that an update arm scheduled no follow-up work: `update`
/// returns `Task::none()` for an arm with nothing to run, and iced's `Task`
/// represents that as no stream to run. The suite repeats this same
/// `into_stream(task).is_none()` probe — directly for the `TrackPlayed`
/// handoff (which still prunes the player-local title index) and a rejected
/// selection press, and through [`assert_message_schedules_no_work`] for the
/// arms it names — so it lives here once and "this arm schedules nothing"
/// reads as the named contract it is.
fn assert_no_task(task: Task<Message>) {
    assert!(iced_runtime::task::into_stream(task).is_none());
}

/// Drives `message` through `update` and asserts the arm schedules no
/// follow-up work. Some arms are true no-ops — a selection press the
/// epoch/index guard rejects, Next/Previous with an empty `songs` buffer, a
/// title-bar window action before the window id resolves, and a shade
/// measurement that arrives after the shade it belongs to — while others
/// (`Play`, `Pause`, the volume nudges, and the shade and always-on-top
/// toggles whose resize/level call the missing window id skips) still mutate
/// the shared state and only skip the follow-up task. Both kinds repeat the
/// update-then-[`assert_no_task`] sequence, so it lives here once and each
/// call site names only the message it drives.
fn assert_message_schedules_no_work(player: &mut WinampPlayer, message: Message) {
    assert_no_task(update(player, message));
}

/// Drives `message` through `update` and asserts the arm schedules follow-up
/// work. The arms that must run a task once their guard passes — a title-bar
/// window action with a resolved window id, the shade and always-on-top
/// toggles, and a recorded shade measurement — each need the same
/// `into_stream(update(..)).is_some()` probe, so it lives here once and each
/// call site names only the message it drives.
fn assert_message_schedules_work(player: &mut WinampPlayer, message: Message) {
    assert!(
        iced_runtime::task::into_stream(update(player, message)).is_some(),
        "the update arm must schedule follow-up work"
    );
}

/// The three custom-title-bar window actions that are complete no-ops without
/// a window id, in the order the title bar renders them: the draggable band,
/// minimize, and close. The always-on-top, shade, and About toggles also touch
/// the window, but each flips a player flag before its window call, so they
/// keep their own tests. The two tests below drive this set against the two
/// window-id states — no work before `boot`'s query resolves, work after — so a
/// new no-op window action is added here once rather than to both loops.
const TITLE_BAR_WINDOW_ACTIONS: [Message; 3] = [
    Message::WindowDragged,
    Message::MinimizeWindow,
    Message::CloseWindow,
];

/// Drives a step `message` twice from `start` — once with Repeat off (the
/// default) and once with Repeat on — and asserts the index it lands on
/// each time. The Next and Previous repeat-flag tests below pin the same
/// shape: step from the album's edge, re-land on that edge with Repeat off,
/// then wrap to the opposite end with Repeat on. Only the message, the
/// starting song, and the two expected indices differ, so the
/// set-Repeat-then-step sequence lives here once and each test names its
/// direction and edges.
/// The arms use `blocking_lock`, which panics inside an async runtime, so
/// this stays a plain (non-async) helper, like
/// [`assert_toggles_shared_state`].
fn assert_repeat_wraps_at_the_edge(
    message: Message,
    start: &str,
    repeat_off: usize,
    repeat_on: usize,
) {
    let (mut player, state) = player_stepping_from(Some(start));

    // Repeat off (the default): the step re-lands on the edge song.
    let task = update(&mut player, message.clone());
    assert_track_selected(task, 0, repeat_off);

    // Repeat on: the step wraps to the opposite end.
    state.blocking_lock().repeat = true;
    let task = update(&mut player, message);
    assert_track_selected(task, 0, repeat_on);
}

/// Feeds a `*Loaded` message built from `items` back through `update` and
/// asserts the list lands in `buffer` unchanged and that the matching
/// `loading` flag is cleared — a `*Loaded` reply reaches `BrowseList::store`,
/// which clears the flag (see `views::browse::rows::text::browse_placeholder`), so the reply must
/// leave that list not-loading. The six store-a-reply tests — artists,
/// albums, songs, all songs, all albums, and playlists — exercise the same
/// update-then-compare flow, differing only in the fixture, the `*Loaded`
/// message variant, the buffer it fills, and the loading flag it clears; this
/// helper serves the first five, while `playlists_loaded_stores_the_list`
/// repeats the flow directly. The message constructor, the buffer, and the
/// flag reader come in as parameters so the shared flow lives here once and
/// each test only names its fixture and target.
fn assert_loaded_populates_list<T: Clone + PartialEq + std::fmt::Debug>(
    player: &mut WinampPlayer,
    items: Vec<T>,
    loaded: impl Fn(Vec<T>) -> Message,
    buffer: impl Fn(&mut WinampPlayer) -> &mut Vec<T>,
    loading: impl Fn(&WinampPlayer) -> bool,
) {
    let _ = update(player, loaded(items.clone()));
    assert_eq!(&*buffer(player), &items);
    assert!(!loading(player));
}

/// Pressing Sort on `view`'s list before any rows load must leave every list's
/// epoch and the current view alone.
fn assert_sort_is_a_noop_on_an_unloaded_view(view: CurrentView) {
    let (mut player, _state) = test_player();
    let expected_view = std::mem::discriminant(&view);
    player.current_view = view;
    let epochs = [
        player.artists.epoch,
        player.albums.epoch,
        player.songs.epoch,
        player.all_songs.epoch,
        player.all_albums.epoch,
        player.playlists.epoch,
    ];

    let _ = update(&mut player, Message::SortBrowse);

    assert_eq!(
        [
            player.artists.epoch,
            player.albums.epoch,
            player.songs.epoch,
            player.all_songs.epoch,
            player.all_albums.epoch,
            player.playlists.epoch,
        ],
        epochs
    );
    assert_eq!(std::mem::discriminant(&player.current_view), expected_view);
}

/// The ids in each browse list, in the order Artists, Albums, Songs, All
/// Songs, All Albums, Playlists.
fn browse_list_ids(player: &WinampPlayer) -> [Vec<String>; 6] {
    [
        player
            .artists
            .items
            .iter()
            .map(|row| row.id.clone())
            .collect(),
        player
            .albums
            .items
            .iter()
            .map(|row| row.id.clone())
            .collect(),
        player
            .songs
            .items
            .iter()
            .map(|row| row.id.clone())
            .collect(),
        player
            .all_songs
            .items
            .iter()
            .map(|row| row.id.clone())
            .collect(),
        player
            .all_albums
            .items
            .iter()
            .map(|row| row.id.clone())
            .collect(),
        player
            .playlists
            .items
            .iter()
            .map(|row| row.id.clone())
            .collect(),
    ]
}

/// The epochs of the six browse lists, in the same order as
/// [`browse_list_ids`].
fn browse_list_epochs(player: &WinampPlayer) -> [u64; 6] {
    [
        player.artists.epoch,
        player.albums.epoch,
        player.songs.epoch,
        player.all_songs.epoch,
        player.all_albums.epoch,
        player.playlists.epoch,
    ]
}

/// Gives every browse list finished-loading rows in an order Sort would change,
/// so a locality test can tell sorting the current list from touching another.
fn seed_lists_for_sort_locality(player: &mut WinampPlayer) {
    player.artists.items = vec![
        Artist {
            id: "Zulu".to_string(),
            name: "Zulu".to_string(),
            album_count: 0,
        },
        Artist {
            id: "alpha".to_string(),
            name: "alpha".to_string(),
            album_count: 0,
        },
        Artist {
            id: "bravo".to_string(),
            name: "bravo".to_string(),
            album_count: 0,
        },
    ];
    let albums = || {
        vec![
            Album {
                id: "zulu".to_string(),
                title: "Alpha".to_string(),
                artist_id: "artist-1".to_string(),
                artist: "Zulu".to_string(),
                duration_ms: 0,
            },
            Album {
                id: "alpha".to_string(),
                title: "Zulu".to_string(),
                artist_id: "artist-1".to_string(),
                artist: "alpha".to_string(),
                duration_ms: 0,
            },
        ]
    };
    player.albums.items = albums();
    player.songs.items = stepping_songs();
    player.all_songs.items = stepping_songs();
    player.all_albums.items = albums();
    player.playlists.items = vec![
        Playlist {
            id: "zulu".to_string(),
            name: "Zulu".to_string(),
        },
        Playlist {
            id: "alpha".to_string(),
            name: "alpha".to_string(),
        },
    ];
    player.artists.loading = false;
    player.albums.loading = false;
    player.songs.loading = false;
    player.all_songs.loading = false;
    player.all_albums.loading = false;
    player.playlists.loading = false;
}

/// The names the artists list shows, in row order.
fn artist_names(player: &WinampPlayer) -> Vec<&str> {
    player
        .artists
        .items
        .iter()
        .map(|artist| artist.name.as_str())
        .collect()
}

/// Plays song-1 (title "One", artist "The Sample Band") from the shared
/// [`stepping_songs`] album — recording it in `known_tracks` — then browses to
/// album-2's songs ([`second_album_songs`]), which replaces `songs`. Returns
/// the player and the shared state's current track, so a Now Playing bar test
/// can assert the bar still names the played track after the browse-away that
/// used to leave it showing the raw id. The three bar tests below share this
/// exact play-then-browse flow, so it lives here once.
fn play_song_1_then_browse_to_album_2() -> (WinampPlayer, Option<String>) {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    state.blocking_lock().current_track = Some("song-1".to_string());
    // Complete the play the selection started, as the playback task does once
    // its backend call returns; this also lets the position poll resume after
    // the selection's reset.
    let generation = player
        .plays_generation
        .load(std::sync::atomic::Ordering::SeqCst);
    let _ = update(
        &mut player,
        Message::TrackPlayed {
            generation,
            committed: true,
        },
    );
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));
    let current_track = state.blocking_lock().current_track.clone();
    (player, current_track)
}

/// Builds `view` once per browse level — the pre-load, loaded, and failed
/// shapes all need the per-frame assembly driven at every level, and iced
/// `Element`s expose no tree introspection, so the observable contract is
/// that `view` builds its tree without panicking over the app's input
/// space. Shared by the two construction tests below so a new browse level
/// is exercised in both at once.
fn construct_view_in_every_browse_view(player: &mut WinampPlayer) {
    for current_view in BROWSE_VIEWS {
        player.current_view = current_view;
        let _ = view(player);
    }
}

/// Lays the root `view` out with the headless tiny-skia renderer and returns
/// every text fragment's absolute bounds plus the resolved root size.
fn captured_root_view_text_bounds(player: &WinampPlayer) -> layout_capture::TextCapture {
    let renderer = layout_capture::headless_renderer();
    let mut element = view(player);
    layout_capture::capture_text(&mut element, &renderer)
}

/// Lays the root `view` out and returns the bounds of the container tagged
/// `id`, so a test can find a compact strip's slider in the real laid-out
/// tree.
fn captured_root_view_container(player: &WinampPlayer, target_id: &str) -> Option<iced::Rectangle> {
    use iced::advanced::{
        Layout,
        widget::{Id, Tree, operation::Operation},
    };

    struct CapturedContainer {
        target_id: Id,
        bounds: Option<iced::Rectangle>,
    }
    impl Operation for CapturedContainer {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn container(&mut self, id: Option<&Id>, bounds: iced::Rectangle) {
            if id == Some(&self.target_id) {
                self.bounds = Some(bounds);
            }
        }
    }

    let renderer = layout_capture::headless_renderer();
    let mut element = view(player);
    let mut tree = Tree::new(element.as_widget());
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &layout_capture::limits());
    let mut captured = CapturedContainer {
        target_id: Id::from(target_id.to_string()),
        bounds: None,
    };
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut captured);
    captured.bounds
}

/// Lays the root `view` out and returns the bounds of the container tagged
/// `views::MINI_VOLUME_ID`, so a test can find the library footer's volume
/// slider in the real laid-out tree.
fn captured_root_view_volume_slider(player: &WinampPlayer) -> Option<iced::Rectangle> {
    captured_root_view_container(player, views::MINI_VOLUME_ID)
}

/// Rolls the wheel at the center of `bounds` in the root view of `player` and
/// returns what the view published.
fn root_wheel_messages(player: &WinampPlayer, bounds: iced::Rectangle, y: f32) -> Vec<Message> {
    let mut renderer = layout_capture::headless_renderer();
    layout_capture::publish_at(
        view(player),
        &mut renderer,
        &[iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
            delta: iced::mouse::ScrollDelta::Lines { x: 0.0, y },
        })],
        iced::Point::new(bounds.center_x(), bounds.center_y()),
    )
}

/// Seeds a known current track titled `title` into `player` and `state`.
fn seed_current_title(player: &mut WinampPlayer, state: &Arc<Mutex<AppState>>, title: &str) {
    player.known_tracks.insert(
        "song-1".to_string(),
        views::KnownTrack {
            title: title.to_string(),
            artist: String::new(),
            album: String::new(),
            number: String::new(),
            time: "3:30".to_string(),
            duration_ms: 210_000,
        },
    );
    state.blocking_lock().current_track = Some("song-1".to_string());
}

fn root_view_draws(player: &WinampPlayer, fragment: &str) -> bool {
    captured_root_view_text_bounds(player)
        .texts()
        .any(|text| text.contains(fragment))
}

mod browse_tests;
mod flat_lists_tests;
mod jump_tests;
mod jump_time_tests;
mod loading_tests;
mod mixer_tests;
mod move_row_tests;
mod panels_tests;
mod randomize_tests;
mod readouts_tests;
mod reverse_tests;
mod scroll_tests;
mod search_tests;
mod settings_tests;
mod shortcuts_tests;
mod sort_tests;
mod stepping_tests;
mod timers_tests;
mod transport_tests;
mod view_tests;
mod window_tests;
