use super::browse::BrowseList;
use super::loading::{RequestGeneration, fetch_into, play_into};
use super::timers::*;
use super::views::{BROWSE_LIST_ID, BROWSE_VIEWS};
use super::*;
use crate::audio::{PlaybackFormat, Spectrum};
use crate::equalizer::{BAND_COUNT, GAIN_MAX_DB, GAIN_MIN_DB};
use crate::library::{Album, Artist, Playlist, Song};
use crate::sample_library::sample_library;
use crate::settings::{Settings, SettingsWriter};
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

#[test]
fn a_changed_setting_is_saved_once_and_unchanged_updates_are_not() {
    let (mut player, state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(&mut player, Message::VolumeChange(0.8));
    let expected = Settings::from_state(&state.blocking_lock());
    assert_eq!(store.snapshots(), vec![expected]);
    assert_eq!(state.blocking_lock().volume(), 0.8);

    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 1);
}

#[test]
fn repeat_and_equalizer_changes_reach_the_saved_snapshot() {
    let (mut player, state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(&mut player, Message::ToggleRepeat);
    assert_eq!(store.snapshots().len(), 1);
    assert_eq!(
        store.snapshots()[0],
        Settings::from_state(&state.blocking_lock())
    );

    let _ = update(&mut player, Message::EqPresetSelected(rock_preset()));
    let current = {
        let state = state.blocking_lock();
        assert!(state.repeat);
        assert_eq!(state.eq_preset(), Some(rock_preset()));
        Settings::from_state(&state)
    };
    assert_eq!(store.snapshots().len(), 2);
    assert_eq!(store.snapshots()[1], current);
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

/// The browse-list scroll operations a task emitted: a relative snap-to-top
/// and an absolute scroll-to-row are captured separately, so a test pins
/// which one an arm schedules (and that it schedules no other).
#[derive(Default)]
struct BrowseScrollEffects {
    snapped: Option<iced::advanced::widget::operation::scrollable::RelativeOffset<Option<f32>>>,
    scrolled: Option<iced::advanced::widget::operation::scrollable::AbsoluteOffset<Option<f32>>>,
}

impl BrowseScrollEffects {
    fn any(&self) -> bool {
        self.snapped.is_some() || self.scrolled.is_some()
    }
}

/// Applies an iced widget operation to a capture under `id` and returns the
/// browse scroll effects it produced. An operation aimed at another id must
/// produce none, so each caller checks the unrelated id too.
fn browse_scroll_effects_by(
    operation: &mut dyn iced::advanced::widget::Operation<()>,
    id: &iced::advanced::widget::Id,
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
            _offset: AbsoluteOffset,
            _bounds: iced::Rectangle,
            _content_bounds: iced::Rectangle,
        ) {
        }
    }

    let mut capture = Capture::default();
    operation.scrollable(
        Some(id),
        iced::Rectangle::default(),
        iced::Rectangle::default(),
        iced::Vector::default(),
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
        let matching = browse_scroll_effects_by(&mut operation, &Id::new(BROWSE_LIST_ID));
        let unrelated = browse_scroll_effects_by(&mut operation, &Id::new("not-the-browse-list"));
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
/// operation snaps the browse list to its first row. This distinguishes the
/// real scroll task from any other follow-up work a navigation arm might
/// schedule, and pins that the same arm does not also scroll to a row.
async fn assert_task_snaps_browse_list_to_top(task: Task<Message>, what: &str) {
    use iced::advanced::widget::operation::scrollable::RelativeOffset;

    let effects = browse_scroll_effects(task, what).await;
    assert_eq!(
        effects.snapped,
        Some(RelativeOffset::START.into()),
        "{what} must snap the browse list to its first row"
    );
    assert_eq!(
        effects.scrolled, None,
        "{what} must not scroll the browse list to a row"
    );
}

/// Drives `task` until it runs an iced scroll operation and asserts that
/// operation scrolls the browse list to absolute row offset `expected`.
async fn assert_task_scrolls_browse_list_to(task: Task<Message>, what: &str, expected: f32) {
    use iced::advanced::widget::operation::scrollable::AbsoluteOffset;

    let effects = browse_scroll_effects(task, what).await;
    assert_eq!(
        effects.scrolled,
        Some(AbsoluteOffset {
            x: None,
            y: Some(expected),
        }),
        "{what} must scroll the browse list to row offset {expected}"
    );
    assert_eq!(
        effects.snapped, None,
        "{what} must not snap the browse list to the top"
    );
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

#[test]
fn await_or_timeout_returns_the_output_when_the_future_completes() {
    let output = futures::executor::block_on(await_or_timeout(
        "quick",
        std::time::Duration::from_secs(1),
        async { 7 },
    ));
    assert_eq!(output, 7);
}

// A task that never completes is the failure `await_or_timeout` exists to
// bound: without the timeout this test would hang forever, taking the whole
// suite with it. The panic names the awaited task, so the hang is diagnosable.
#[test]
#[should_panic(expected = "hung task did not complete within")]
fn await_or_timeout_fails_loudly_when_the_future_never_completes() {
    futures::executor::block_on(await_or_timeout(
        "hung",
        std::time::Duration::from_millis(10),
        futures::future::pending::<()>(),
    ));
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

// The repeat, shuffle, and equalizer toggle arms share one shape: read the
// flag, update, read it again. `assert_toggles_shared_state` owns that shape,
// so each test only names its message and the flag it flips. All three use
// `blocking_lock`, which panics inside an async runtime, so they stay plain
// tests (no `#[tokio::test]`). The Play/Pause arm is asynchronous now (it
// drives the audio backend), so it has its own `#[tokio::test]` below.
#[tokio::test]
async fn play_pause_toggles_is_playing() {
    // Load a preview so the toggle has audio to pause and resume.
    let (mut player, state) = player_playing_preview(Arc::new(RecordingAudio::default())).await;
    assert!(state.lock().await.is_playing);

    let task = update(&mut player, Message::PlayPause);
    assert_transport_settled(task, "pause toggle").await;
    assert!(!state.lock().await.is_playing);

    let task = update(&mut player, Message::PlayPause);
    assert_transport_settled(task, "resume toggle").await;
    assert!(state.lock().await.is_playing);
}

#[test]
fn toggle_repeat_flips_shared_state() {
    assert_toggles_shared_state(Message::ToggleRepeat, |state| state.repeat);
}

#[test]
fn shuffle_toggle_flips_the_shuffle_flag() {
    assert_toggles_shared_state(Message::ToggleShuffle, |state| state.shuffle);
}

#[test]
fn toggle_equalizer_flips_shared_state() {
    assert_toggles_shared_state(Message::ToggleEqualizer, |state| state.eq_enabled);
}

#[test]
fn eq_preamp_change_clamps_value_before_storing() {
    assert_message_clamps(
        Message::EqPreampChange(99.0),
        |state| state.eq_preamp(),
        GAIN_MAX_DB,
    );
    assert_message_clamps(
        Message::EqPreampChange(-99.0),
        |state| state.eq_preamp(),
        GAIN_MIN_DB,
    );
}

#[test]
fn eq_band_change_clamps_value_before_storing() {
    assert_message_clamps(
        Message::EqBandChange(0, 99.0),
        |state| state.eq_bands()[0],
        GAIN_MAX_DB,
    );
    assert_message_clamps(
        Message::EqBandChange(1, -99.0),
        |state| state.eq_bands()[1],
        GAIN_MIN_DB,
    );
}

// The EqPresetSelected arm writes a whole curve and records the selection,
// which the one-value slider-arm tests above do not cover. Drive a preset
// through `update` and read all three shared-state fields back.
#[test]
fn eq_preset_selected_applies_the_curve_and_selection() {
    let (mut player, state) = test_player();
    let preset = rock_preset();

    let _ = update(&mut player, Message::EqPresetSelected(preset));

    let state = state.blocking_lock();
    assert_eq!(state.eq_preamp(), preset.preamp);
    assert_eq!(state.eq_bands(), preset.bands);
    assert_eq!(state.eq_preset(), Some(preset));
}

// `Message::Stop` drives the audio backend through a `Task`, so it needs the
// async runtime and `drive_task`.
#[tokio::test]
async fn stop_clears_is_playing() {
    let (mut player, state) = player_playing_preview(Arc::new(RecordingAudio::default())).await;
    assert!(state.lock().await.is_playing);

    let task = update(&mut player, Message::Stop);
    assert_transport_settled(task, "stop").await;
    assert!(!state.lock().await.is_playing);
}

#[tokio::test]
async fn play_and_pause_set_the_playback_flag_explicitly() {
    let (mut player, state) = player_playing_preview(Arc::new(RecordingAudio::default())).await;

    // X plays even when already playing, unlike the Play/Pause toggle.
    let task = update(&mut player, Message::Play);
    assert_transport_settled(task, "play").await;
    assert!(state.lock().await.is_playing);
    let task = update(&mut player, Message::Play);
    assert_transport_settled(task, "play again").await;
    assert!(state.lock().await.is_playing);

    // C pauses even when already paused.
    let task = update(&mut player, Message::Pause);
    assert_transport_settled(task, "pause").await;
    assert!(!state.lock().await.is_playing);
    let task = update(&mut player, Message::Pause);
    assert_transport_settled(task, "pause again").await;
    assert!(!state.lock().await.is_playing);
}

// The four transport arms must reach the injected backend, not just flip the
// shared flag. Each message is driven over its own recording backend and the
// call it records is pinned: Play resumes a paused preview, Pause pauses, Stop
// stops, and PlayPause pauses while playing. A regression that left an arm on
// the synchronous `AppState` mutator would flip the flag but record no call.
#[tokio::test]
async fn transport_messages_drive_the_audio_backend() {
    // Play (the X key): resume a paused preview.
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_preview(Arc::clone(&recording)).await;
    player.apple_music_service.pause().await.unwrap();
    assert_transport_settled(update(&mut player, Message::Play), "play").await;
    assert_preview_calls(&recording.calls(), &[AudioCall::Pause, AudioCall::Resume]);
    assert!(state.lock().await.is_playing);

    // Pause (the C key).
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_preview(Arc::clone(&recording)).await;
    assert_transport_settled(update(&mut player, Message::Pause), "pause").await;
    assert_preview_calls(&recording.calls(), &[AudioCall::Pause]);
    assert!(!state.lock().await.is_playing);

    // Stop.
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_preview(Arc::clone(&recording)).await;
    assert_transport_settled(update(&mut player, Message::Stop), "stop").await;
    assert_preview_calls(&recording.calls(), &[AudioCall::Stop]);
    assert!(!state.lock().await.is_playing);

    // PlayPause while playing: pauses.
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_preview(Arc::clone(&recording)).await;
    assert_transport_settled(update(&mut player, Message::PlayPause), "toggle").await;
    assert_preview_calls(&recording.calls(), &[AudioCall::Pause]);
    assert!(!state.lock().await.is_playing);
}

// Every transport arm shares `transport_into`, whose error branch logs the
// backend's rejection and then still emits `TransportSettled` so the view
// re-renders. No other UI test reaches that branch: the recording fake
// succeeds at every command. The service calls the backend before it writes
// shared state, so a rejected command must also leave the playing flag alone.
// `attempts` proves each arm reached a failing backend rather than skipping
// it, which a settled-and-unchanged assertion alone could not tell apart.
#[tokio::test]
async fn rejected_transport_commands_still_settle_and_keep_playing_state() {
    let audio = Arc::new(FailingAudio::failing_transport());
    let (mut player, state) =
        player_with_audio(Arc::clone(&audio) as Arc<dyn crate::audio::AudioOutput>);
    player
        .apple_music_service
        .play_track("song-1", Some(PREVIEW_URL), || true)
        .await
        .unwrap();
    assert!(state.lock().await.is_playing);

    for (message, what) in [
        (Message::Pause, "rejected pause"),
        (Message::Stop, "rejected stop"),
        (Message::Play, "rejected resume"),
        (Message::PlayPause, "rejected toggle"),
    ] {
        assert_transport_settled(update(&mut player, message), what).await;
        assert!(
            state.lock().await.is_playing,
            "{what} must leave the playing flag set"
        );
    }

    assert_eq!(audio.attempts(FailingCommand::Transport), 4);
}

#[test]
fn volume_up_and_down_nudge_by_the_slider_step() {
    let (mut player, state) = test_player();

    let start = state.blocking_lock().volume();
    assert_message_schedules_no_work(&mut player, Message::VolumeUp);
    let raised = state.blocking_lock().volume();
    assert!((raised - (start + views::VOLUME_STEP)).abs() < f32::EPSILON);

    assert_message_schedules_no_work(&mut player, Message::VolumeDown);
    let lowered = state.blocking_lock().volume();
    assert!((lowered - start).abs() < f32::EPSILON);
}

#[test]
fn volume_keys_clamp_at_the_ends() {
    let (mut player, state) = test_player();

    for _ in 0..200 {
        assert_message_schedules_no_work(&mut player, Message::VolumeUp);
    }
    assert_eq!(state.blocking_lock().volume(), 1.0);

    for _ in 0..200 {
        assert_message_schedules_no_work(&mut player, Message::VolumeDown);
    }
    assert_eq!(state.blocking_lock().volume(), 0.0);
}

#[test]
fn volume_wheel_nudges_by_one_slider_step_and_ignores_zero() {
    let (mut player, state) = test_player();

    let start = state.blocking_lock().volume();
    assert_message_schedules_no_work(&mut player, Message::VolumeWheel(1.0));
    let raised = state.blocking_lock().volume();
    assert!((raised - (start + views::VOLUME_STEP)).abs() < f32::EPSILON);

    assert_message_schedules_no_work(&mut player, Message::VolumeWheel(-1.0));
    let lowered = state.blocking_lock().volume();
    assert!((lowered - start).abs() < f32::EPSILON);

    assert_message_schedules_no_work(&mut player, Message::VolumeWheel(0.0));
    assert!((state.blocking_lock().volume() - start).abs() < f32::EPSILON);
}

#[test]
fn volume_wheel_clamps_at_the_ends() {
    let (mut player, state) = test_player();

    for _ in 0..200 {
        assert_message_schedules_no_work(&mut player, Message::VolumeWheel(1.0));
    }
    assert_eq!(state.blocking_lock().volume(), 1.0);

    for _ in 0..200 {
        assert_message_schedules_no_work(&mut player, Message::VolumeWheel(-1.0));
    }
    assert_eq!(state.blocking_lock().volume(), 0.0);
}

#[test]
fn volume_change_clamps_value_before_storing() {
    // Out-of-range slider values are clamped by the update arm.
    assert_message_clamps(Message::VolumeChange(1.5), |state| state.volume(), 1.0);
    assert_message_clamps(Message::VolumeChange(-0.2), |state| state.volume(), 0.0);

    // An in-range value is stored as-is.
    assert_message_clamps(Message::VolumeChange(0.3), |state| state.volume(), 0.3);
}

#[test]
fn balance_change_clamps_value_before_storing() {
    // Out-of-range slider values are clamped by the update arm.
    assert_message_clamps(Message::BalanceChange(1.5), |state| state.balance(), 1.0);
    assert_message_clamps(Message::BalanceChange(-2.0), |state| state.balance(), -1.0);

    // An in-range value is stored as-is.
    assert_message_clamps(Message::BalanceChange(-0.4), |state| state.balance(), -0.4);
}

#[test]
fn artist_selected_flips_to_albums_view() {
    let player = player_in_albums();

    assert_view(&player, CurrentView::Albums);
}

#[test]
fn album_selected_flips_to_songs_view() {
    let (mut player, _state) = test_player();
    player.albums.items = vec![sample_album()];

    let _ = update(&mut player, Message::AlbumSelected { epoch: 0, index: 0 });

    assert_view(&player, CurrentView::Songs);
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

#[test]
fn artist_selected_clears_the_previous_artists_albums() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![sample_artist()];
    let _ = update(&mut player, Message::AlbumsLoaded(vec![sample_album()]));
    player.albums.error = Some("music-library fetch failed: boom".to_string());

    assert_entering_a_level_clears_the_child_list(
        &mut player,
        |player| &mut player.albums,
        Message::ArtistSelected { epoch: 0, index: 0 },
        Message::AlbumsLoaded(vec![sample_album()]),
        |epoch| Message::AlbumSelected { epoch, index: 0 },
    );

    assert_view(&player, CurrentView::Albums);
}

// The songs twin: stepping into an album clears the previous album's songs
// rather than leaving them to be shown or pressed while the new album's
// fetch is in flight.
#[test]
fn album_selected_clears_the_previous_albums_songs() {
    let (mut player, _state) = test_player();
    player.albums.items = vec![sample_album()];
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.songs.error = Some("music-library fetch failed: boom".to_string());

    assert_entering_a_level_clears_the_child_list(
        &mut player,
        |player| &mut player.songs,
        Message::AlbumSelected { epoch: 0, index: 0 },
        Message::SongsLoaded(second_album_songs()),
        |epoch| Message::TrackSelected { epoch, index: 0 },
    );
}

// The selection messages carry a row index and the epoch of the list that
// rendered the row. An index past the end of the current buffer can only
// come from a stale message, so every selection arm must no-op rather than
// panic. And because a replaced list bumps its epoch, an index that is in
// range of the *new* list but was rendered against the old one must also
// no-op rather than select the wrong entry — the bug an unguarded index
// would introduce during the fetch window. The first half runs against
// empty buffers (index 0 is out of range for all three); the second half
// replaces a loaded list with a same-length one so the stale index stays in
// range and only the epoch guard can reject it.
#[test]
fn selection_messages_with_a_stale_epoch_or_index_do_nothing() {
    let (mut player, state) = test_player();

    assert_message_schedules_no_work(&mut player, Message::ArtistSelected { epoch: 0, index: 0 });
    assert_message_schedules_no_work(&mut player, Message::AlbumSelected { epoch: 0, index: 0 });
    assert_message_schedules_no_work(&mut player, Message::TrackSelected { epoch: 0, index: 0 });

    assert_view(&player, CurrentView::Artists);
    assert!(state.blocking_lock().current_track.is_none());

    // Render a row from the first loaded list (epoch 1), then replace that
    // list with a same-length one (epoch 2): the stale press names index 0,
    // which is in range of the new list, so only the epoch guard stops it
    // from playing the wrong song.
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));
    assert_message_schedules_no_work(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    assert!(state.blocking_lock().current_track.is_none());
}

// A browse row's first press only selects it: the view, the fetched lists,
// and the playing track must all stay as they were, and no task may be
// scheduled, so a click cannot drill in or start playback by itself.
#[test]
fn a_single_row_press_selects_without_opening() {
    let (mut player, _state) = test_player();
    seed_browse_lists(&mut player);
    player.artists.loading = false;

    assert_message_schedules_no_work(&mut player, Message::SelectRow { epoch: 0, index: 0 });

    assert_view(&player, CurrentView::Artists);
    assert_eq!(player.artists.selected(), Some(0));
}

// The second press on the row that is already selected is what a
// double-click delivers: it dispatches the existing ArtistSelected action,
// which steps to Albums exactly as the old single press did.
#[test]
fn a_second_press_on_the_selected_row_opens_it() {
    let (mut player, _state) = test_player();
    seed_browse_lists(&mut player);
    player.artists.loading = false;
    let epoch = player.artists.epoch;

    assert_message_schedules_no_work(&mut player, Message::SelectRow { epoch, index: 0 });
    let task = update(&mut player, Message::SelectRow { epoch, index: 0 });
    let opened = futures::executor::block_on(task_output(task, "second row press"));
    match &opened {
        Message::ArtistSelected { epoch: e, index: 0 } if *e == epoch => {}
        other => panic!("the second press must dispatch the artist open: {other:?}"),
    }

    let _ = update(&mut player, opened);
    assert_view(&player, CurrentView::Albums);
}

// The Open button opens the selected song through the existing TrackSelected
// path, so the pressed row's index resolves to the same playback the old
// single press started.
#[tokio::test]
async fn open_selected_opens_the_selected_song() {
    let (mut player, state) = test_player();
    seed_browse_lists(&mut player);
    player.current_view = CurrentView::Songs;
    player.songs.loading = false;
    let epoch = player.songs.epoch;
    assert_message_schedules_no_work(&mut player, Message::SelectRow { epoch, index: 0 });

    let task = update(&mut player, Message::OpenSelected);
    let opened = futures::executor::block_on(task_output(task, "open selected song"));
    match &opened {
        Message::TrackSelected { epoch: e, index: 0 } if *e == epoch => {}
        other => panic!("Open must dispatch the selected song: {other:?}"),
    }
    drive_task(
        update(&mut player, opened),
        "playback",
        |message| match message {
            Message::TrackPlayed { committed, .. } => assert!(committed),
            other => panic!("unexpected playback task output: {other:?}"),
        },
    )
    .await;

    let state = state.lock().await;
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(state.is_playing);
}

#[test]
fn open_selected_with_nothing_selected_is_a_noop() {
    let (mut player, _state) = test_player();
    seed_browse_lists(&mut player);
    player.current_view = CurrentView::Albums;

    assert_message_schedules_no_work(&mut player, Message::OpenSelected);

    assert_view(&player, CurrentView::Albums);
    assert_eq!(player.albums.selected(), None);
}

#[test]
fn replacing_or_sorting_rows_clears_the_selection() {
    let (mut player, _state) = test_player();
    let mut second = sample_artist();
    second.id = "artist-2".to_string();
    second.name = "Zzz".to_string();
    player.artists.items = vec![sample_artist(), second];
    player.artists.loading = false;
    let epoch = player.artists.epoch;
    let _ = update(&mut player, Message::SelectRow { epoch, index: 0 });
    assert_eq!(player.artists.selected(), Some(0));

    let _ = update(&mut player, Message::SortBrowse);
    assert_eq!(player.artists.selected(), None);

    let epoch = player.artists.epoch;
    let _ = update(&mut player, Message::SelectRow { epoch, index: 0 });
    assert_eq!(player.artists.selected(), Some(0));
    let _ = update(&mut player, Message::ArtistsLoaded(vec![sample_artist()]));
    assert_eq!(player.artists.selected(), None);
}

// The browse hierarchy must be navigable back up (Songs → Albums →
// Artists), not just down: without `Message::Back` a session is a one-way
// corridor that ends stuck at the bottom of the hierarchy. These arms use
// only field assignment (no `blocking_lock`), so they stay plain tests
// like the view-flip tests above.
#[test]
fn back_from_albums_returns_to_artists() {
    let mut player = player_in_albums();

    assert_view(&player, CurrentView::Albums);

    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Artists);
}

#[test]
fn back_from_songs_returns_to_albums() {
    let mut player = player_in_albums();
    // `ArtistSelected` clears the albums buffer, so feed the fetch reply the
    // iced loop would before stepping into the album.
    let _ = update(&mut player, Message::AlbumsLoaded(vec![sample_album()]));
    let _ = update(&mut player, Message::AlbumSelected { epoch: 1, index: 0 });

    assert_view(&player, CurrentView::Songs);

    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Albums);
}

#[test]
fn back_from_artists_is_a_noop() {
    let (mut player, _state) = test_player();
    assert_view(&player, CurrentView::Artists);

    assert_message_schedules_no_work(&mut player, Message::Back);

    assert_view(&player, CurrentView::Artists);
}

#[tokio::test]
async fn back_schedules_the_restored_list_to_the_top() {
    let mut player = player_in_albums();

    assert_task_snaps_browse_list_to_top(update(&mut player, Message::Back), "back").await;

    assert_view(&player, CurrentView::Artists);
}

// The seven arms that open a fresh browse list must each batch the
// snap-to-top scroll with their fetch; dropping one leaves that navigation
// (or search) showing the previous list's scroll offset.
#[tokio::test]
async fn every_browse_navigation_batches_a_snap_to_top_task() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![sample_artist()];
    assert_task_snaps_browse_list_to_top(
        update(&mut player, Message::ArtistSelected { epoch: 0, index: 0 }),
        "artist selection",
    )
    .await;

    let (mut player, _state) = test_player();
    player.albums.items = vec![sample_album()];
    assert_task_snaps_browse_list_to_top(
        update(&mut player, Message::AlbumSelected { epoch: 0, index: 0 }),
        "album selection",
    )
    .await;

    let (mut player, _state) = test_player();
    player.playlists.items = vec![sample_playlist()];
    assert_task_snaps_browse_list_to_top(
        update(
            &mut player,
            Message::PlaylistSelected { epoch: 0, index: 0 },
        ),
        "playlist selection",
    )
    .await;

    let (mut player, _state) = test_player();
    assert_task_snaps_browse_list_to_top(update(&mut player, Message::ShowAllSongs), "all songs")
        .await;

    let (mut player, _state) = test_player();
    assert_task_snaps_browse_list_to_top(update(&mut player, Message::ShowAllAlbums), "all albums")
        .await;

    let (mut player, _state) = test_player();
    assert_task_snaps_browse_list_to_top(update(&mut player, Message::ShowPlaylists), "playlists")
        .await;

    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SearchChanged("echo".to_string()));
    assert_task_snaps_browse_list_to_top(update(&mut player, Message::SearchSubmitted), "search")
        .await;
}

// The Top button only asks iced to snap the browse scrollable back to its
// first row; it must leave the view and the shared playback state alone.
#[test]
fn scroll_browse_to_top_schedules_a_scroll_task() {
    let (mut player, state) = test_player();

    futures::executor::block_on(assert_task_snaps_browse_list_to_top(
        update(&mut player, Message::ScrollBrowseToTop),
        "top",
    ));

    assert_view(&player, CurrentView::Artists);
    assert!(state.blocking_lock().current_track.is_none());
    assert!(!state.blocking_lock().is_playing);
}

// The All Songs button fetches the whole library flat and shows it under its
// own view: the sample path returns every song in library order, and the
// buffer is the dedicated `all_songs` list rather than the browsed `songs`.
#[tokio::test]
async fn show_all_songs_fetches_the_flat_list_and_switches_view() {
    let (mut player, _state) = test_player();
    // Seed a browsed album so the flat fetch visibly leaves the `songs`
    // buffer alone: All Songs owns its own list.
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let task = update(&mut player, Message::ShowAllSongs);
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load all library songs",
        |player| &mut player.all_songs.items,
        |song| song.id.as_str(),
        &["song-1", "song-2", "song-3", "song-4", "song-5"],
    )
    .await;

    assert_view(&player, CurrentView::AllSongs);
    assert!(!player.search_active);
    assert_eq!(player.songs.items.len(), 3);
}

// The flat list hangs off the artists list, not off a search's results, so
// opening it clears an active search.
#[test]
fn show_all_songs_clears_an_active_search() {
    let (mut player, _state) = test_player();
    player.search_active = true;
    player.current_view = CurrentView::Songs;

    let _ = update(&mut player, Message::ShowAllSongs);

    assert!(!player.search_active);
    assert_view(&player, CurrentView::AllSongs);
}

#[test]
fn back_from_all_songs_returns_to_the_artists_list() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::AllSongs;

    let _ = update(&mut player, Message::Back);

    assert_view(&player, CurrentView::Artists);
}

// The All Albums button fetches every library album flat and shows it under
// its own view: the sample path returns the albums in library order, and the
// buffer is the dedicated `all_albums` list rather than the browsed `albums`.
#[tokio::test]
async fn show_all_albums_fetches_the_flat_list_and_switches_view() {
    let (mut player, _state) = test_player();
    // Seed a browsed artist's albums so the flat fetch visibly leaves the
    // `albums` buffer alone: All Albums owns its own list.
    let _ = update(&mut player, Message::AlbumsLoaded(vec![sample_album()]));

    let task = update(&mut player, Message::ShowAllAlbums);
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load all library albums",
        |player| &mut player.all_albums.items,
        |album| album.id.as_str(),
        &["album-1", "album-2", "album-3"],
    )
    .await;

    assert_view(&player, CurrentView::AllAlbums);
    assert!(!player.search_active);
    assert_eq!(player.albums.items.len(), 1);
}

// The Playlists button fetches the library's playlists and shows them under
// their own view, leaving the other browse buffers alone.
#[tokio::test]
async fn show_playlists_fetches_and_switches_view() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let task = update(&mut player, Message::ShowPlaylists);
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load library playlists",
        |player| &mut player.playlists.items,
        |playlist| playlist.id.as_str(),
        &["playlist-1", "playlist-2", "playlist-3"],
    )
    .await;

    assert_view(&player, CurrentView::Playlists);
    assert!(!player.search_active);
    assert_eq!(player.songs.items.len(), 3);
}

#[test]
fn playlists_loaded_stores_the_list() {
    let (mut player, _state) = test_player();

    let _ = update(
        &mut player,
        Message::PlaylistsLoaded(vec![sample_playlist()]),
    );

    assert_eq!(player.playlists.items, vec![sample_playlist()]);
    assert!(!player.playlists.loading);
}

// The Playlists list hangs off the artists list, not off a search's results,
// so opening it clears an active search.
#[test]
fn show_playlists_clears_an_active_search() {
    let (mut player, _state) = test_player();
    player.search_active = true;
    player.current_view = CurrentView::Songs;

    let _ = update(&mut player, Message::ShowPlaylists);

    assert!(!player.search_active);
    assert_view(&player, CurrentView::Playlists);
}

#[test]
fn back_from_playlists_returns_to_the_artists_list() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Playlists;

    let _ = update(&mut player, Message::Back);

    assert_view(&player, CurrentView::Artists);
}

// A playlist row press resolves against `playlists`, loads that playlist's
// songs into the shared `songs` buffer, and records the origin so Back returns
// to Playlists. Playlist 1's named order (song-1 then song-4) differs from
// album order, so the exact ids pin the playlist query rather than an album
// lookup.
#[tokio::test]
async fn playlist_selected_in_playlists_loads_its_songs_and_back_returns_to_playlists() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::PlaylistsLoaded(vec![sample_playlist()]),
    );
    player.current_view = CurrentView::Playlists;

    let task = update(
        &mut player,
        Message::PlaylistSelected { epoch: 1, index: 0 },
    );
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load songs from a playlist",
        |player| &mut player.songs.items,
        |song| song.id.as_str(),
        &["song-1", "song-4"],
    )
    .await;

    assert_view(&player, CurrentView::Songs);
    assert_eq!(player.songs_origin, SongsOrigin::Playlist);

    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Playlists);
}

// A stale playlist press names no row in the current list, so it must issue no
// fetch and leave the view and songs buffer untouched.
#[test]
fn a_stale_playlist_press_issues_no_fetch_and_leaves_the_view_unchanged() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::PlaylistsLoaded(vec![sample_playlist()]),
    );
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.current_view = CurrentView::Playlists;

    assert_message_schedules_no_work(
        &mut player,
        Message::PlaylistSelected { epoch: 0, index: 0 },
    );
    assert_view(&player, CurrentView::Playlists);
    assert_eq!(player.songs.items.len(), 3);

    assert_message_schedules_no_work(
        &mut player,
        Message::PlaylistSelected { epoch: 1, index: 9 },
    );
    assert_view(&player, CurrentView::Playlists);
    assert_eq!(player.songs.items.len(), 3);
}

// The flat album list hangs off the artists list, not off a search's
// results, so opening it clears an active search.
#[test]
fn show_all_albums_clears_an_active_search() {
    let (mut player, _state) = test_player();
    player.search_active = true;
    player.current_view = CurrentView::Songs;

    let _ = update(&mut player, Message::ShowAllAlbums);

    assert!(!player.search_active);
    assert_view(&player, CurrentView::AllAlbums);
}

#[test]
fn back_from_all_albums_returns_to_the_artists_list() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::AllAlbums;

    let _ = update(&mut player, Message::Back);

    assert_view(&player, CurrentView::Artists);
}

// A row pressed in the flat All Albums view must resolve against `all_albums`,
// not the per-artist `albums` buffer, and Back from the songs it opens must
// return to the flat list. Both buffers hold a different album at index 0, so
// only routing the press to `all_albums` (and recording the origin) makes the
// right album's songs load and Back land on All Albums.
#[tokio::test]
async fn album_selected_in_all_albums_loads_its_songs_and_back_returns_to_all_albums() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::AlbumsLoaded(vec![sample_album()]));
    let flat_album = Album {
        id: "album-3".to_string(),
        title: "Debut".to_string(),
        artist_id: String::new(),
        artist: "Echo Chamber".to_string(),
        duration_ms: 0,
    };
    let _ = update(&mut player, Message::AllAlbumsLoaded(vec![flat_album]));
    player.current_view = CurrentView::AllAlbums;

    let task = update(&mut player, Message::AlbumSelected { epoch: 1, index: 0 });
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load songs from the flat albums list",
        |player| &mut player.songs.items,
        |song| song.id.as_str(),
        &["song-5"],
    )
    .await;

    assert_view(&player, CurrentView::Songs);
    assert_eq!(player.songs_origin, SongsOrigin::AllAlbums);

    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::AllAlbums);
}

// The library search box's text is stored on every keystroke; submitting is
// what turns it into a fetch. This pins the storing half.
#[test]
fn search_changed_stores_the_box_text() {
    let (mut player, _state) = test_player();

    let _ = update(&mut player, Message::SearchChanged("echo".to_string()));

    assert_eq!(player.search_query, "echo");
    assert!(!player.search_active);
}

// Submitting a query replaces the songs buffer with the library matches and
// shows them under the Songs view — the same buffer the browse hierarchy
// uses, so the playing-row marker and the empty/failure wording keep working.
#[tokio::test]
async fn search_submitted_replaces_the_songs_list_with_the_matches() {
    let (mut player, _state) = test_player();
    // Seed a browsed album's songs so the search visibly replaces them.
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    // The submit resolves the text `SearchChanged` stored, so type it first.
    let _ = update(&mut player, Message::SearchChanged("  echo  ".to_string()));

    let task = update(&mut player, Message::SearchSubmitted);
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "search the library",
        |player| &mut player.songs.items,
        |song| song.id.as_str(),
        &["song-5"],
    )
    .await;

    assert_view(&player, CurrentView::Songs);
    assert!(player.search_active);
    assert_eq!(player.search_query, "echo");
}

// A blank submit has nothing to search; it must not clear the current list,
// flip the view, or mark a search active. The error report `search_songs`
// would produce for the blank query is skipped too.
#[test]
fn search_submitted_with_a_blank_query_is_a_no_op() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.current_view = CurrentView::Songs;
    let _ = update(&mut player, Message::SearchChanged("   ".to_string()));

    assert_message_schedules_no_work(&mut player, Message::SearchSubmitted);

    assert_view(&player, CurrentView::Songs);
    assert!(!player.search_active);
    assert_eq!(player.songs.items.len(), 3);
}

// A search's result list hangs off the artists list, not the album it was
// issued from, so Back leaves the search straight for Artists rather than
// opening the album list the search replaced.
#[test]
fn back_leaves_an_active_search_for_the_artists_list() {
    let (mut player, _state) = test_player();
    player.search_active = true;
    player.current_view = CurrentView::Songs;

    let _ = update(&mut player, Message::Back);

    assert_view(&player, CurrentView::Artists);
    assert!(!player.search_active);
}

// Entering a browse level leaves the search: ArtistSelected and AlbumSelected
// both clear the active flag, so Back from the level they open behaves like an
// ordinary browse again.
#[test]
fn entering_a_browse_level_clears_the_active_search() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![sample_artist()];
    player.search_active = true;

    let _ = update(&mut player, Message::ArtistSelected { epoch: 0, index: 0 });
    assert!(!player.search_active);

    let (mut player, _state) = test_player();
    player.albums.items = vec![sample_album()];
    player.search_active = true;

    let _ = update(&mut player, Message::AlbumSelected { epoch: 0, index: 0 });
    assert!(!player.search_active);
}

// The browse arms do two things — flip the view and fetch the next
// level's list — and the view-flip tests above stop at the first half.
// These pin the fetch half: `ArtistSelected` must load the selected
// artist's albums and `AlbumSelected` the selected album's songs, both
// delivered as a `*Loaded` message that iced feeds back into the browse
// buffer. A regression that fetched the wrong artist's albums (or the
// wrong album's songs) would still flip the view, so the loaded contents
// are asserted, not just the view.

#[tokio::test]
async fn artist_selected_fetches_the_artists_albums_into_the_player() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![sample_artist()];

    let task = update(&mut player, Message::ArtistSelected { epoch: 0, index: 0 });
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load albums",
        |player| &mut player.albums.items,
        |album| album.id.as_str(),
        &["album-1", "album-2"],
    )
    .await;

    // The arm flips to the albums view before the fetched list is fed
    // back through the update loop.
    assert_view(&player, CurrentView::Albums);
}

#[tokio::test]
async fn album_selected_fetches_the_albums_songs_into_the_player() {
    let (mut player, _state) = test_player();
    player.albums.items = vec![Album {
        id: "album-3".to_string(),
        title: "Debut".to_string(),
        artist_id: "artist-2".to_string(),
        artist: "Echo Chamber".to_string(),
        duration_ms: 0,
    }];

    let task = update(&mut player, Message::AlbumSelected { epoch: 0, index: 0 });
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load songs",
        |player| &mut player.songs.items,
        |song| song.id.as_str(),
        &["song-5"],
    )
    .await;

    // As with the artist arm: the view flips to songs, and the fetched
    // list lands in the browse buffer in library order.
    assert_view(&player, CurrentView::Songs);
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

// A browse reply can complete out of order: the user picks artist-1, then
// artist-2, and artist-1's slower fetch lands last. Storing that reply would
// replace artist-2's albums with artist-1's, showing the wrong artist's list.
// The per-list request generation rejects the superseded reply. This test
// holds the first task and drives it after the second has landed, which is
// the completion order a slow backend produces.
#[tokio::test]
async fn a_slow_stale_browse_reply_does_not_overwrite_the_newer_list() {
    let (mut player, _state) = test_player();
    player.artists.items = sample_library().artists.clone();

    // Issue two album fetches: artist-1 (index 0), then artist-2 (index 1).
    // The artists list is not replaced, so both presses carry epoch 0.
    let first = update(&mut player, Message::ArtistSelected { epoch: 0, index: 0 });
    let second = update(&mut player, Message::ArtistSelected { epoch: 0, index: 1 });

    // The newer request's reply lands first; the older one, landing late,
    // must be dropped and leave artist-2's albums in place.
    assert_stale_album_reply_is_dropped(&mut player, second, first, &["album-3"]).await;
}

// The stale-reply test above pins a superseded *success*; this pins the
// failure case. `fetch_into` checks the request generation before it matches
// the result, so a slow backend error is dropped exactly like a slow success:
// without that placement, a stale failure would run `BrowseList::fail` and
// wipe the newer artist's albums and record its error against them, so the
// panel would show the wrong artist's failure. Drive the failing request's
// task after the newer one has landed.
#[tokio::test]
async fn a_slow_stale_browse_failure_does_not_overwrite_the_newer_list_or_error() {
    let (mut player, _state) = test_player();
    // Index 0 is an artist whose blank id the service seam rejects, so its
    // fetch fails; index 1 is a real artist whose fetch succeeds.
    player.artists.items = vec![
        Artist {
            id: String::new(),
            name: "Blank".to_string(),
            album_count: 0,
        },
        sample_artist(),
    ];

    // Issue two album fetches: the failing artist-0, then the valid artist-1.
    // The artists list is not replaced, so both presses carry epoch 0.
    let first = update(&mut player, Message::ArtistSelected { epoch: 0, index: 0 });
    let second = update(&mut player, Message::ArtistSelected { epoch: 0, index: 1 });

    // The newer, successful reply lands first; the older request's failure
    // must be dropped rather than wiping the newer list and recording its
    // error against it.
    assert_stale_album_reply_is_dropped(&mut player, second, first, &["album-1", "album-2"]).await;
}

// The stale-reply tests above hold a reply in its task until after a newer
// request has landed, so `fetch_into`'s generation check on iced's executor
// thread already sees the reply as stale. This covers the narrower window
// that check cannot: the mapper runs while the reply is still current, and a
// newer request is issued on the UI thread before `update` processes the
// stamped reply. Re-checking the generation at store time must drop it, or the
// cleared list is repopulated with the superseded reply and the panel shows
// the previous level's rows.
#[tokio::test]
async fn a_reply_superseded_between_the_mapper_and_update_is_not_stored() {
    let (mut player, _state) = test_player();

    // Issue the first artists fetch and drive it to completion while it is
    // still current, so the mapper stamps its reply as current.
    let generation = player.artists.begin_fetch();
    let task = fetch_into(
        &player.apple_music_service,
        "loading favorite artists".to_string(),
        generation,
        std::time::Duration::from_secs(1),
        |service| async move { service.get_favorite_artists().await },
        Message::ArtistsLoaded,
        Message::ArtistsLoadFailed,
    );
    let reply = task_output(task, "first artists fetch").await;

    // A newer request for the same list is issued before `update` processes
    // the reply.
    let _ = update(&mut player, Message::LoadArtists);

    // The reply the newer request superseded must not be stored: the list
    // stays cleared and loading for the newer fetch.
    let _ = update(&mut player, reply);
    assert!(player.artists.items.is_empty());
    assert!(player.artists.loading);
}

// The songs twin of `a_reply_superseded_between_the_mapper_and_update_is_not_stored`.
// The songs browse level had no superseded-reply test at all, and
// `BrowseReply`'s UI-thread re-check names the songs list in its own match
// arm: without that arm a reply from the album the user browsed away from
// would repopulate the cleared songs buffer with the previous album's songs.
// Issue the first songs fetch and drive it to completion while it is still
// current, issue a newer songs request before `update` processes the reply,
// and assert the stamped reply is dropped.
#[tokio::test]
async fn a_stale_songs_reply_superseded_before_update_is_not_stored() {
    let (mut player, _state) = test_player();
    // Clear the songs buffer and mark it loading, as `AlbumSelected` does
    // before issuing a fetch, so the dropped reply's effect on `loading` is
    // observable below.
    player.songs.clear();

    // Issue the first songs fetch and drive it to completion while it is
    // still current, so the mapper stamps its reply as current rather than
    // dropping it as `Ignored` before it reaches the `BrowseReply` arm.
    let generation = player.songs.begin_fetch();
    let task = fetch_into(
        &player.apple_music_service,
        "loading songs from album \"album-1\"".to_string(),
        generation,
        std::time::Duration::from_secs(1),
        |service| async move { service.get_songs_from_album("album-1").await },
        Message::SongsLoaded,
        Message::SongsLoadFailed,
    );
    let reply = task_output(task, "first songs fetch").await;
    assert!(
        matches!(&reply, Message::BrowseReply { reply, .. } if matches!(reply.as_ref(), Message::SongsLoaded(_))),
        "the driven fetch must produce a stamped songs reply, got {reply:?}"
    );

    // A newer songs request is issued before `update` processes the reply.
    let _ = player.songs.begin_fetch();

    // The reply the newer request superseded must not be stored: the songs
    // buffer stays empty and loading for the newer fetch.
    let _ = update(&mut player, reply);
    assert!(player.songs.items.is_empty());
    assert!(player.songs.loading);
}

// The albums twin of `a_stale_songs_reply_superseded_before_update_is_not_stored`.
// `BrowseReply`'s UI-thread re-check names the albums list in its own match
// arm: without that arm a reply from the artist the user browsed away from
// would repopulate the cleared albums buffer with the previous artist's
// albums. Issue the first albums fetch and drive it to completion while it is
// still current, issue a newer albums request before `update` processes the
// reply, and assert the stamped reply is dropped.
#[tokio::test]
async fn a_stale_albums_reply_superseded_before_update_is_not_stored() {
    let (mut player, _state) = test_player();
    // Clear the albums buffer and mark it loading, as `ArtistSelected` does
    // before issuing a fetch, so the dropped reply's effect on `loading` is
    // observable below.
    player.albums.clear();

    // Issue the first albums fetch and drive it to completion while it is
    // still current, so the mapper stamps its reply as current rather than
    // dropping it as `Ignored` before it reaches the `BrowseReply` arm.
    let generation = player.albums.begin_fetch();
    let task = fetch_into(
        &player.apple_music_service,
        "loading albums for artist \"artist-1\"".to_string(),
        generation,
        std::time::Duration::from_secs(1),
        |service| async move { service.get_albums_by_artist("artist-1").await },
        Message::AlbumsLoaded,
        Message::AlbumsLoadFailed,
    );
    let reply = task_output(task, "first albums fetch").await;
    assert!(
        matches!(&reply, Message::BrowseReply { reply, .. } if matches!(reply.as_ref(), Message::AlbumsLoaded(albums) if !albums.is_empty())),
        "the driven fetch must produce a stamped non-empty albums reply, got {reply:?}"
    );

    // A newer albums request is issued before `update` processes the reply.
    let _ = player.albums.begin_fetch();

    // The reply the newer request superseded must not be stored: the albums
    // buffer stays empty and loading for the newer fetch.
    let _ = update(&mut player, reply);
    assert!(player.albums.items.is_empty());
    assert!(player.albums.loading);
}

// `BrowseReply` names every browse list in its own match arm, but only the
// Artists, Songs, and Albums arms had their UI-thread re-check pinned. The
// three flat lists behind the Artists view's buttons are fetched the same
// way, and pressing Back then the button again issues a newer request before
// the first reply is stored. Without each list's arm the stale reply would
// fall through the `_ => true` catch-all and store the previous rows — or its
// error — over the list the newer request cleared. Pin both reply kinds for
// each flat list: a stale reply leaves the cleared list loading with no rows
// and no error, and that list's own current generation still stores or
// records its reply.
#[test]
fn stale_flat_list_replies_superseded_before_update_are_not_stored() {
    let (mut player, _state) = test_player();

    macro_rules! assert_list_rechecks_its_generation {
        ($list:ident, $loaded:expr, $failed:expr) => {{
            player.$list.clear();
            let stale = player.$list.begin_fetch();
            let current = player.$list.begin_fetch();

            let _ = update(
                &mut player,
                Message::BrowseReply {
                    issued: stale.issued(),
                    reply: Box::new($loaded),
                },
            );
            assert!(
                player.$list.items.is_empty(),
                concat!(stringify!($list), " stored a stale success reply")
            );
            assert!(player.$list.loading);
            assert!(player.$list.error.is_none());

            let _ = update(
                &mut player,
                Message::BrowseReply {
                    issued: stale.issued(),
                    reply: Box::new($failed("stale error".to_string())),
                },
            );
            assert!(
                player.$list.items.is_empty(),
                concat!(stringify!($list), " stored a stale failure reply")
            );
            assert!(player.$list.loading);
            assert!(player.$list.error.is_none());

            let _ = update(
                &mut player,
                Message::BrowseReply {
                    issued: current.issued(),
                    reply: Box::new($loaded),
                },
            );
            assert_eq!(
                player.$list.items.len(),
                1,
                concat!(stringify!($list), " dropped its current success reply")
            );
            assert!(!player.$list.loading);
            assert!(player.$list.error.is_none());

            let _ = update(
                &mut player,
                Message::BrowseReply {
                    issued: current.issued(),
                    reply: Box::new($failed("current error".to_string())),
                },
            );
            assert!(player.$list.items.is_empty());
            assert!(!player.$list.loading);
            assert_eq!(player.$list.error.as_deref(), Some("current error"));
        }};
    }

    assert_list_rechecks_its_generation!(
        all_songs,
        Message::AllSongsLoaded(vec![sample_song()]),
        Message::AllSongsLoadFailed
    );
    assert_list_rechecks_its_generation!(
        all_albums,
        Message::AllAlbumsLoaded(vec![sample_album()]),
        Message::AllAlbumsLoadFailed
    );
    assert_list_rechecks_its_generation!(
        playlists,
        Message::PlaylistsLoaded(vec![sample_playlist()]),
        Message::PlaylistsLoadFailed
    );
}

// A play reply can complete out of order just like a browse reply: the user
// clicks song-1, then song-2, and song-1's slower play lands last. The
// playback counter's guard, evaluated inside `play_track` under the state
// lock, rejects the superseded play so it cannot overwrite the newer track —
// without it the Now Playing bar would name song-1 after the user's last
// click was song-2. This drives the two `TrackSelected` tasks in the reverse
// of their issue order, the completion order a slow backend produces.
#[tokio::test]
async fn a_slow_stale_play_does_not_overwrite_the_newer_track() {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    // Issue two plays: song-1 (index 0), then song-2 (index 1). The songs
    // list is not replaced, so both presses carry epoch 1.
    let first = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    let second = update(&mut player, Message::TrackSelected { epoch: 1, index: 1 });

    // The newer play's reply lands first.
    drive_task(second, "newer play", |_| {}).await;
    assert_eq!(state.lock().await.current_track.as_deref(), Some("song-2"));

    // The older play's reply lands late and must be dropped, leaving the
    // newer track playing.
    drive_task(first, "stale play", |_| {}).await;
    assert_eq!(state.lock().await.current_track.as_deref(), Some("song-2"));
}

// The UI must hand the selected song's preview URL to the service, so the
// audio backend plays the right asset. This drives `TrackSelected` through
// `update` and the returned play task to completion, then asserts the
// recording backend saw exactly that URL — the end-to-end wiring the one-line
// `preview_url.as_deref()` handoff exists for.
#[tokio::test]
async fn track_selected_plays_the_songs_preview_url() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);

    player.songs.items = vec![Song {
        id: "song-1".to_string(),
        title: "One".to_string(),
        artist: "The Sample Band".to_string(),
        album_id: "album-1".to_string(),
        duration_ms: 210_000,
        preview_url: Some(PREVIEW_URL.to_string()),
    }];

    let task = update(&mut player, Message::TrackSelected { epoch: 0, index: 0 });
    drive_task(task, "play", |_| {}).await;

    assert_preview_calls(&recording.calls(), &[]);
}

// The volume arms must reach the injected backend, not just write shared
// state: a slider drag or arrow key that only moved the `AppState` value would
// leave the playing preview at its old gain. This drives each arm through
// `update` over a recording backend and pins the clamped value it forwarded,
// including a `NaN` `VolumeChange` that stores and forwards the `0.0` clamp.
#[test]
fn volume_messages_forward_to_the_audio_backend() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);

    assert_message_schedules_no_work(&mut player, Message::VolumeChange(0.2));
    assert_eq!(state.blocking_lock().volume(), 0.2);

    assert_message_schedules_no_work(&mut player, Message::VolumeUp);
    assert_eq!(state.blocking_lock().volume(), 0.2 + views::VOLUME_STEP);

    assert_message_schedules_no_work(&mut player, Message::VolumeDown);
    assert_eq!(state.blocking_lock().volume(), 0.2);

    assert_message_schedules_no_work(&mut player, Message::VolumeChange(f32::NAN));
    assert_eq!(state.blocking_lock().volume(), 0.0);

    assert_eq!(
        recording.calls(),
        vec![
            AudioCall::SetVolume(0.2),
            AudioCall::SetVolume(0.2 + views::VOLUME_STEP),
            AudioCall::SetVolume(0.2),
            AudioCall::SetVolume(0.0),
        ]
    );
}

// The left and right arrow arms must reach the injected backend: a seek that
// only changed UI state would leave the playing preview at its old position.
// This drives both arms through `update` over a recording backend and pins
// the signed five-second offsets they forwarded. The expected offsets are
// literals rather than `views::SEEK_STEP_SECS`: the README promises the
// arrows seek five seconds, and an assertion written in terms of the same
// constant the arms read would still pass if that documented step changed.
#[test]
fn seek_messages_drive_the_audio_backend() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);

    assert_message_schedules_no_work(&mut player, Message::SeekBackward);
    assert_message_schedules_no_work(&mut player, Message::SeekForward);

    assert_eq!(
        recording.calls(),
        vec![AudioCall::SeekBy(-5), AudioCall::SeekBy(5)]
    );
    assert_eq!(*state.blocking_lock(), AppState::default());
}

// A rejection from the seek backend must only be logged: the seek arm still
// schedules no work and leaves shared state alone. The fake counts its
// rejected calls, proving the message reached the failing backend instead of
// skipping the forward.
#[test]
fn a_seek_backend_failure_still_schedules_no_work() {
    let audio = Arc::new(FailingAudio::failing_seek());
    let (mut player, state) =
        player_with_audio(Arc::clone(&audio) as Arc<dyn crate::audio::AudioOutput>);

    assert_message_schedules_no_work(&mut player, Message::SeekForward);

    assert_eq!(audio.attempts(FailingCommand::Seek), 1);
    assert_eq!(*state.blocking_lock(), AppState::default());
}

// The balance arm must reach the injected backend, not just write shared
// state: a slider drag that only moved the `AppState` value would leave the
// playing preview centered. This drives `BalanceChange` through `update` over
// a recording backend and pins the clamped value it forwarded, including an
// out-of-range value and a `NaN` that stores and forwards the center.
#[test]
fn balance_messages_forward_to_the_audio_backend() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);

    assert_message_schedules_no_work(&mut player, Message::BalanceChange(-0.4));
    assert_eq!(state.blocking_lock().balance(), -0.4);

    assert_message_schedules_no_work(&mut player, Message::BalanceChange(1.5));
    assert_eq!(state.blocking_lock().balance(), 1.0);

    assert_message_schedules_no_work(&mut player, Message::BalanceChange(f32::NAN));
    assert_eq!(state.blocking_lock().balance(), 0.0);

    assert_eq!(
        recording.calls(),
        vec![
            AudioCall::SetBalance(-0.4),
            AudioCall::SetBalance(1.0),
            AudioCall::SetBalance(0.0),
        ]
    );
}

// A backend rejection must not roll back the volume mutation: the state write
// happens before the forward, so the slider and arrow keys keep the user's
// choice and only log the failure. The fake counts the rejected calls, proving
// the message reached the failing backend rather than skipping the forward.
#[test]
fn a_volume_backend_failure_still_commits_the_state() {
    let audio = Arc::new(FailingAudio::failing_volume_and_balance());
    let (mut player, state) =
        player_with_audio(Arc::clone(&audio) as Arc<dyn crate::audio::AudioOutput>);

    assert_message_schedules_no_work(&mut player, Message::VolumeChange(0.3));

    assert_eq!(state.blocking_lock().volume(), 0.3);
    assert_eq!(audio.attempts(FailingCommand::Volume), 1);
    assert_eq!(audio.attempts(FailingCommand::Balance), 0);
}

// The balance arm has the same commit-then-forward contract as volume and the
// equalizer: a rejected backend call leaves the centered image the user chose
// in shared state and only logs the failure.
#[test]
fn a_balance_backend_failure_still_commits_the_state() {
    let audio = Arc::new(FailingAudio::failing_volume_and_balance());
    let (mut player, state) =
        player_with_audio(Arc::clone(&audio) as Arc<dyn crate::audio::AudioOutput>);

    assert_message_schedules_no_work(&mut player, Message::BalanceChange(-0.4));

    assert_eq!(state.blocking_lock().balance(), -0.4);
    assert_eq!(audio.attempts(FailingCommand::Balance), 1);
    assert_eq!(audio.attempts(FailingCommand::Volume), 0);
}

// The equalizer arms must reach the injected backend, not just write shared
// state: a toggle, slider drag, or preset pick that only moved `AppState`
// would leave the playing preview at its old curve. This drives every arm
// through `update` over a recording backend and pins each forwarded snapshot
// against the clamped values that same message left in shared state,
// including an out-of-range band gain and a `NaN` one that stores and forwards
// the flat clamp. Every arm must schedule no work: the running source reads
// the new curve without a new `Play`.
#[test]
fn equalizer_messages_forward_to_the_audio_backend() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);

    fn snapshot(state: &AppState) -> crate::audio::EqSnapshot {
        crate::audio::EqSnapshot {
            enabled: state.eq_enabled,
            preamp_db: state.eq_preamp(),
            bands_db: state.eq_bands(),
        }
    }

    let mut expected = Vec::new();

    assert_message_schedules_no_work(&mut player, Message::ToggleEqualizer);
    expected.push(snapshot(&state.blocking_lock()));

    assert_message_schedules_no_work(&mut player, Message::EqPreampChange(3.0));
    expected.push(snapshot(&state.blocking_lock()));

    assert_message_schedules_no_work(&mut player, Message::EqBandChange(0, 4.0));
    expected.push(snapshot(&state.blocking_lock()));

    assert_message_schedules_no_work(&mut player, Message::EqBandChange(1, 99.0));
    expected.push(snapshot(&state.blocking_lock()));

    assert_message_schedules_no_work(&mut player, Message::EqBandChange(2, f32::NAN));
    expected.push(snapshot(&state.blocking_lock()));

    assert_message_schedules_no_work(&mut player, Message::EqPresetSelected(rock_preset()));
    expected.push(snapshot(&state.blocking_lock()));

    // The values the messages stored are the clamped ones, so the forwarded
    // curve and the shared state cannot drift apart.
    assert!(expected[0].enabled);
    assert_eq!(expected[1].preamp_db, 3.0);
    assert_eq!(expected[2].bands_db[0], 4.0);
    assert_eq!(expected[3].bands_db[1], GAIN_MAX_DB);
    assert_eq!(expected[4].bands_db[2], 0.0);
    assert_eq!(expected[5].preamp_db, rock_preset().preamp);
    assert_eq!(expected[5].bands_db, rock_preset().bands);

    assert_eq!(
        recording.calls(),
        expected
            .into_iter()
            .map(AudioCall::SetEqualizer)
            .collect::<Vec<_>>()
    );
}

// A backend rejection must not roll back the equalizer mutation: the state
// write happens before the forward, exactly as in the volume and balance
// helpers, so the panel keeps the user's choice and only logs the failure.
// The fake counts the rejected calls, proving the message reached the failing
// backend rather than skipping the forward.
#[test]
fn an_equalizer_backend_failure_still_commits_the_state() {
    let audio = Arc::new(FailingAudio::failing_equalizer());
    let (mut player, state) =
        player_with_audio(Arc::clone(&audio) as Arc<dyn crate::audio::AudioOutput>);

    assert_message_schedules_no_work(&mut player, Message::EqBandChange(0, 4.0));

    assert_eq!(state.blocking_lock().eq_bands()[0], 4.0);
    assert_eq!(audio.attempts(FailingCommand::Equalizer), 1);
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

#[tokio::test]
async fn boot_schedules_loading_the_artist_list() {
    let state = Arc::new(Mutex::new(AppState::default()));
    let service =
        AppleMusicService::with_transport(state.clone(), Box::new(StubTransport::returning("")));

    let (player, task) = boot(state, service);
    assert_view(&player, CurrentView::Artists);

    // The batched task carries both the artists fetch and the window-id query,
    // and iced runs both after the window opens; the fetch's `LoadArtists`
    // output is the first the stream yields.
    await_boot_artist_loads(task, 1).await;
}

// The startup sign-in runs on a background thread, so the first artists fetch
// can land before the session exists and answer from the sample library. The
// boot task must watch for the session and re-issue `LoadArtists` when it
// arrives, or a signed-in user keeps seeing the sample artists for the rest of
// the process. A regression that dropped that watch would exhaust the boot
// stream after the initial fetch, so [`await_boot_artist_loads`] would time
// out waiting for a second `LoadArtists`.
#[tokio::test]
async fn boot_reloads_artists_after_the_startup_sign_in_stores_a_session() {
    let state = Arc::new(Mutex::new(AppState::default()));
    let service = AppleMusicService::new(state.clone());
    let sign_in_service = service.clone();

    let (player, task) = boot(state, service);
    assert_view(&player, CurrentView::Artists);

    // Store the session from a background thread, as `init_service` does, so
    // the boot task observes a session that arrives after it started waiting.
    let sign_in = std::thread::spawn(move || {
        sign_in_service
            .authenticate_with("dev-token", &|_| Ok(sign_in_session()))
            .expect("the stub sign-in must store its session");
    });

    // The batched task carries the initial fetch, the session wait, and the
    // window-id query; drive until the wait's second `LoadArtists` lands.
    await_boot_artist_loads(task, 2).await;
    sign_in.join().expect("the sign-in thread must not panic");
}

#[tokio::test]
async fn load_artists_fetches_favorite_artists_into_the_player() {
    let (mut player, _state) = test_player();

    let task = update(&mut player, Message::LoadArtists);
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load artists",
        |player| &mut player.artists.items,
        |artist| artist.id.as_str(),
        &["artist-1", "artist-2", "artist-3"],
    )
    .await;
}

// A failed artists fetch is a dead end: `boot` issues `LoadArtists` once and
// no navigation re-issues it, so the Artists view's Retry button sends
// `LoadArtists` again. The arm must clear the stored failure before the
// re-fetch, or the view would keep showing the old error (and never
// "Loading…") while the retry is in flight. Pin the clear alongside the
// empty buffer and the loading flag.
#[test]
fn load_artists_clears_a_prior_failure_before_refetching() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![sample_artist()];
    player.artists.error =
        Some("music-library fetch failed (loading favorite artists): boom".to_string());
    player.artists.loading = false;

    let _task = update(&mut player, Message::LoadArtists);

    assert!(player.artists.items.is_empty());
    assert!(player.artists.loading);
    assert!(player.artists.error.is_none());
}

// `Message::TrackSelected` wraps playback in a `Task`; the arm itself only
// schedules it, so the real behavior lives in the returned task. Drive that
// task to completion (as the iced runtime would) and assert the shared
// state it mutates through the service.
#[tokio::test]
async fn track_selected_starts_playback_of_the_selected_track() {
    let (mut player, state) = test_player();
    player.songs.items = vec![sample_song()];

    let task = update(&mut player, Message::TrackSelected { epoch: 0, index: 0 });
    drive_task(task, "playback", |message| match message {
        Message::TrackPlayed { committed, .. } => assert!(committed),
        other => panic!("unexpected playback task output: {other:?}"),
    })
    .await;

    let state = state.lock().await;
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(state.is_playing);
}

// `Message::TrackPlayed` is the other half of the `TrackSelected` handoff:
// the playback task reports the track started, and `update`'s `TrackPlayed`
// arm receives that completion. The test above only *produces* the message
// (it drives the task and asserts its output); nothing feeds `TrackPlayed`
// back through `update`, so this arm — the one update branch the suite
// never reaches — could drift (e.g. scheduling a follow-up task or touching
// shared state) with no test catching it. Pin the arm's contract: it
// schedules no work and leaves the shared state untouched, pruning only the
// player-local title index. `blocking_lock` panics inside an async runtime,
// so this stays a plain test.
#[test]
fn track_played_handoff_schedules_no_work_and_leaves_shared_state_untouched() {
    let (mut player, state) = test_player();

    // A track is mid-playback when the completion handoff arrives.
    {
        let mut state = state.blocking_lock();
        state.current_track = Some("song-1".to_string());
        state.is_playing = true;
        state.set_volume(0.7);
    }

    let task = update(
        &mut player,
        Message::TrackPlayed {
            generation: 0,
            committed: true,
        },
    );

    // The arm schedules no follow-up work...
    assert_no_task(task);

    // ...and leaves the shared state exactly as it was.
    let state = state.blocking_lock();
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(state.is_playing);
    assert_eq!(state.volume(), 0.7);
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

#[test]
fn window_id_resolved_stores_the_window_id() {
    let (mut player, _state) = test_player();
    assert_eq!(player.window_id, None);

    let id = iced::window::Id::unique();
    let _ = update(&mut player, Message::WindowIdResolved(Some(id)));
    assert_eq!(player.window_id, Some(id));

    let _ = update(&mut player, Message::WindowIdResolved(None));
    assert_eq!(player.window_id, None);
}

/// The three custom-title-bar window actions that are complete no-ops without
/// a window id, in the order the title bar renders them: the draggable band,
/// minimize, and close. The clutter and shade toggles also touch the window,
/// but each flips a player flag before its window call, so they keep their own
/// tests. The two tests below drive this set against the two window-id states
/// — no work before `boot`'s query resolves, work after — so a new no-op
/// window action is added here once rather than to both loops.
const TITLE_BAR_WINDOW_ACTIONS: [Message; 3] = [
    Message::WindowDragged,
    Message::MinimizeWindow,
    Message::CloseWindow,
];

#[test]
fn title_bar_window_actions_are_noops_without_a_window_id() {
    let (mut player, _state) = test_player();
    assert_eq!(player.window_id, None);

    for message in TITLE_BAR_WINDOW_ACTIONS {
        assert_message_schedules_no_work(&mut player, message);
    }
}

#[test]
fn title_bar_window_actions_schedule_work_with_a_window_id() {
    let (mut player, _state) = player_with_window_id();

    for message in TITLE_BAR_WINDOW_ACTIONS {
        assert_message_schedules_work(&mut player, message);
    }
}

#[test]
fn toggle_always_on_top_flips_the_flag_and_sets_the_window_level() {
    let (mut player, _state) = player_with_window_id();
    assert!(!player.always_on_top);

    assert_message_schedules_work(&mut player, Message::ToggleAlwaysOnTop);
    assert!(player.always_on_top);

    assert_message_schedules_work(&mut player, Message::ToggleAlwaysOnTop);
    assert!(!player.always_on_top);
}

#[test]
fn toggle_always_on_top_without_a_window_id_still_flips_the_flag() {
    let (mut player, _state) = test_player();
    assert_eq!(player.window_id, None);

    assert_message_schedules_no_work(&mut player, Message::ToggleAlwaysOnTop);
    assert!(player.always_on_top);
}

#[test]
fn toggle_window_shade_flips_the_flag_and_measures_the_window() {
    let (mut player, _state) = player_with_window_id();
    assert!(!player.shaded);

    assert_message_schedules_work(&mut player, Message::ToggleWindowShade);
    assert!(player.shaded);

    // Once the pre-shade size is known, unshading resizes back to it.
    player.unshaded_size = Some(iced::Size::new(800.0, 600.0));
    assert_message_schedules_work(&mut player, Message::ToggleWindowShade);
    assert!(!player.shaded);
}

#[test]
fn window_shade_measured_stores_the_size_and_resizes() {
    let (mut player, _state) = player_with_window_id();
    player.shaded = true;

    assert_message_schedules_work(
        &mut player,
        Message::WindowShadeMeasured(iced::Size::new(800.0, 600.0)),
    );

    assert_eq!(player.unshaded_size, Some(iced::Size::new(800.0, 600.0)));
}

#[test]
fn window_shade_measurement_after_unshade_is_ignored() {
    let (mut player, _state) = player_with_window_id();

    // Shade, then unshade before the deferred measurement lands.
    let _ = update(&mut player, Message::ToggleWindowShade);
    assert!(player.shaded);
    let _ = update(&mut player, Message::ToggleWindowShade);
    assert!(!player.shaded);

    // The stale measurement must neither record the size nor clamp the
    // restored window back down to the strip.
    assert_message_schedules_no_work(
        &mut player,
        Message::WindowShadeMeasured(iced::Size::new(800.0, 600.0)),
    );
    assert_eq!(player.unshaded_size, None);
}

#[test]
fn a_late_measurement_cannot_overwrite_the_captured_size() {
    let (mut player, _state) = player_with_window_id();

    // The first shade captures the real pre-shade size.
    let _ = update(&mut player, Message::ToggleWindowShade);
    let _ = update(
        &mut player,
        Message::WindowShadeMeasured(iced::Size::new(800.0, 600.0)),
    );
    assert_eq!(player.unshaded_size, Some(iced::Size::new(800.0, 600.0)));

    // Roll down, then up again: the known size rolls straight up without a
    // fresh measurement that could read the strip a prior shade left.
    let _ = update(&mut player, Message::ToggleWindowShade);
    assert_message_schedules_work(&mut player, Message::ToggleWindowShade);

    // A measurement from the first shade arriving now must not overwrite the
    // captured size with the 24 px strip.
    assert_message_schedules_no_work(
        &mut player,
        Message::WindowShadeMeasured(iced::Size::new(800.0, 24.0)),
    );
    assert_eq!(player.unshaded_size, Some(iced::Size::new(800.0, 600.0)));
}

#[test]
fn toggle_window_shade_without_a_window_id_still_flips_the_flag() {
    let (mut player, _state) = test_player();
    assert_eq!(player.window_id, None);

    assert_message_schedules_no_work(&mut player, Message::ToggleWindowShade);
    assert!(player.shaded);
}

#[test]
fn next_track_steps_to_the_following_song() {
    let (mut player, _state) = player_stepping_from(Some("song-1"));

    let task = update(&mut player, Message::NextTrack);

    assert_track_selected(task, 0, 1);
}

#[test]
fn previous_track_steps_to_the_preceding_song() {
    let (mut player, _state) = player_stepping_from(Some("song-2"));

    let task = update(&mut player, Message::PreviousTrack);

    assert_track_selected(task, 0, 0);
}

#[test]
fn next_track_with_no_current_track_starts_at_the_first_song() {
    let (mut player, _state) = player_stepping_from(None);

    let task = update(&mut player, Message::NextTrack);

    assert_track_selected(task, 0, 0);
}

// `next_track_with_no_current_track_starts_at_the_first_song` pins the
// Next half of the no-current case; this pins the Previous half — pressing
// Previous before anything has played must land on the last song, not do
// nothing. The transport arithmetic for a `None` current is pinned in
// `transport.rs` (`previous_without_current_starts_at_last`); this pins
// the `PreviousTrack` arm's wiring over the same case, which no other
// test drives (the only other `previous` wiring tests set a current track
// or load no songs).
#[test]
fn previous_track_with_no_current_track_starts_at_the_last_song() {
    let (mut player, _state) = player_stepping_from(None);

    let task = update(&mut player, Message::PreviousTrack);

    assert_track_selected(task, 0, 2);
}

// Browsing to a different album replaces `player.songs.items` with the new
// album's list while the shared `current_track` still names a song from
// the album just left — the same browse-away state the now-playing-label
// regression (`now_playing_label_keeps_the_track_name_after_browsing_to_another_album`)
// is about. Pressing Next or Previous must then step within the newly
// browsed album's songs — Next onto its first, Previous onto its last —
// not do nothing because the playing track is no longer in the buffer.
// `transport.rs` pins the arithmetic for an unknown current
// (`next_with_unknown_current_starts_at_first` and
// `previous_with_unknown_current_starts_at_last`), but no arm-level test
// drives this exact browse-away-then-step flow: the other stepping wiring
// tests set a current track that IS in `player.songs.items` or load no songs, so
// a guard in `step_track` that bailed on a current unknown to this album
// would clear every existing test.
#[test]
fn stepping_after_browsing_away_steps_within_the_new_albums_songs() {
    let (mut player, _state) = player_stepping_from(Some("song-4"));
    // Played song-4 (an album-2 song) from the library, then browsed to
    // album-1's songs (now the current buffer): the current track names a
    // song that is not in the buffer, exactly as after a browse-away.

    let next = update(&mut player, Message::NextTrack);
    assert_track_selected(next, 0, 0);

    // The buttons only schedule a step — neither mutates the current
    // track — so Previous still sees the same browse-away state.
    let previous = update(&mut player, Message::PreviousTrack);
    assert_track_selected(previous, 0, 2);
}

#[test]
fn next_track_with_no_songs_loaded_does_nothing() {
    let (mut player, _state) = test_player();

    assert_message_schedules_no_work(&mut player, Message::NextTrack);
}

#[test]
fn previous_track_with_no_songs_loaded_does_nothing() {
    let (mut player, _state) = test_player();

    assert_message_schedules_no_work(&mut player, Message::PreviousTrack);
}

// Next and Previous can step the playing row outside the visible list, so
// while a song list is showing each step batches a scroll to the landed row's
// laid-out offset with the selection it already scheduled. From song-2, Next
// lands on song-3 (index 2) and Previous on song-1 (index 0).
#[test]
fn next_and_previous_scroll_the_showing_song_list_to_the_landed_row() {
    let (mut player, _state) = player_stepping_from(Some("song-2"));
    player.current_view = CurrentView::Songs;

    futures::executor::block_on(assert_task_scrolls_browse_list_to(
        update(&mut player, Message::NextTrack),
        "next",
        views::BROWSE_LIST_PADDING + 2.0 * views::BROWSE_ROW_HEIGHT,
    ));
    futures::executor::block_on(assert_task_scrolls_browse_list_to(
        update(&mut player, Message::PreviousTrack),
        "previous",
        views::BROWSE_LIST_PADDING,
    ));
}

// A step only follows the playing row when a song list is showing: the other
// browse levels carry no playing marker, and their lists keep their offset.
#[test]
fn stepping_while_another_browse_level_shows_does_not_scroll() {
    let (mut player, _state) = player_stepping_from(Some("song-1"));
    assert_view(&player, CurrentView::Artists);

    futures::executor::block_on(assert_task_scrolls_nothing(
        update(&mut player, Message::NextTrack),
        "next from artists",
    ));
}

// A pressed row is already in view, so the direct-press path deliberately
// leaves the list where it is.
#[test]
fn a_row_press_does_not_scroll_the_list() {
    let (mut player, _state) = player_stepping_from(Some("song-1"));
    player.current_view = CurrentView::Songs;

    futures::executor::block_on(assert_task_scrolls_nothing(
        update(&mut player, Message::TrackSelected { epoch: 0, index: 1 }),
        "row press",
    ));
}

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

// The Next arm forwards the shared Repeat flag to the transport helper:
// from the last song, Next wraps to the first when Repeat is on and
// re-lands on the last when it is off. The arithmetic is pinned in
// `transport.rs` (`next_wraps_from_last_to_first` and
// `next_stays_on_last_without_repeat`); this pins the arm's wiring over
// the shared flag, which no other test drives.
#[test]
fn next_track_follows_the_shared_repeat_flag_at_the_albums_end() {
    assert_repeat_wraps_at_the_edge(Message::NextTrack, "song-3", 2, 0);
}

// The Previous arm forwards the shared Repeat flag to the transport
// helper, the mirror of the Next test above: from the first song,
// Previous wraps to the last when Repeat is on and re-lands on the first
// when it is off. The arithmetic is pinned in `transport.rs`
// (`previous_wraps_from_first_to_last` and
// `previous_stays_on_first_without_repeat`); this pins the Previous arm's
// wiring over the shared flag, which no other test drives — the other
// Previous wiring tests set a current track with Repeat off, start from
// no current track, or load no songs, so an arm that dropped the shared
// flag (or passed a stale one) would clear every existing test and only
// fail here.
#[test]
fn previous_track_follows_the_shared_repeat_flag_at_the_albums_start() {
    assert_repeat_wraps_at_the_edge(Message::PreviousTrack, "song-1", 0, 2);
}

// With Shuffle on, the Next arm must land somewhere other than the current
// song. `shuffled_track_id`'s arithmetic is pinned in `transport.rs`; this
// pins the arm's wiring over the shared Shuffle flag and that the picked id
// resolves back to an index. The roll is random, so the test asserts only the
// invariant that matters — the index is not the current song's (index 1).
#[test]
fn next_with_shuffle_never_reselects_the_current_track() {
    let (mut player, state) = player_stepping_from(Some("song-2"));
    state.blocking_lock().shuffle = true;

    let task = update(&mut player, Message::NextTrack);
    futures::executor::block_on(drive_task(
        task,
        "shuffled stepping",
        |message| match message {
            Message::TrackSelected { index, .. } => {
                assert_ne!(index, 1, "shuffle re-selected the current song");
            }
            other => panic!("unexpected shuffled stepping task output: {other:?}"),
        },
    ));
}

// With Shuffle on, Previous retraces the recorded play order: the history
// here is song-1, song-3, song-2, so the first press must land on song-3
// (index 2), not the sequential song-1 (index 0), and the second press —
// after the jump's play commits — continues back to song-1.
#[test]
fn previous_with_shuffle_retraces_recorded_plays() {
    let (mut player, state) = player_stepping_from(None);
    state.blocking_lock().shuffle = true;
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-3");
    record_a_play(&mut player, &state, "song-2");

    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, 0, 2);

    record_a_play(&mut player, &state, "song-3");
    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, 0, 0);
}

// The jump's play has not committed yet, so `current` still names song-3;
// two Previous presses must keep walking back from the pending target rather
// than both resolving against the same current track.
#[test]
fn previous_with_shuffle_walks_back_before_the_jump_commits() {
    let (mut player, state) = player_stepping_from(None);
    state.blocking_lock().shuffle = true;
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");
    record_a_play(&mut player, &state, "song-3");

    let first = update(&mut player, Message::PreviousTrack);
    assert_track_selected(first, 0, 1);
    let second = update(&mut player, Message::PreviousTrack);
    assert_track_selected(second, 0, 0);
}

// With no recorded history there is nothing to retrace, so a shuffled
// Previous falls back to the sequential row before the current song.
#[test]
fn previous_with_shuffle_falls_back_to_the_row_before_the_current_song() {
    let (mut player, state) = player_stepping_from(Some("song-2"));
    state.blocking_lock().shuffle = true;

    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, 0, 0);
}

// A recorded track the visible list no longer shows is skipped: the history
// is song-1, song-3, song-2, but song-3 was removed from the browsed songs, so
// Previous lands on song-1 (index 0) rather than scheduling a hidden row.
#[test]
fn previous_with_shuffle_skips_history_entries_outside_the_visible_list() {
    let (mut player, state) = player_stepping_from(None);
    state.blocking_lock().shuffle = true;
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-3");
    record_a_play(&mut player, &state, "song-2");
    player.songs.items.retain(|song| song.id != "song-3");

    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, 0, 0);
}

// A history jump whose play fails must be retried, not left behind: the
// backend rolls `is_playing` back but leaves the jump target in
// `current_track`, so the failed completion names song-2 and `TrackPlayed`
// marks the jump for retry. The next shuffled Previous must select song-2
// again instead of walking past it to song-1.
#[test]
fn a_failed_history_jump_retries_the_target() {
    let (mut player, state) = player_stepping_from(None);
    state.blocking_lock().shuffle = true;
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");
    record_a_play(&mut player, &state, "song-3");

    let jump = update(&mut player, Message::PreviousTrack);
    assert_track_selected(jump, 0, 1);

    {
        let mut state = state.blocking_lock();
        state.current_track = Some("song-2".to_string());
        state.is_playing = false;
    }
    let generation = player
        .plays_generation
        .load(std::sync::atomic::Ordering::SeqCst);
    let _ = update(
        &mut player,
        Message::TrackPlayed {
            generation,
            committed: false,
        },
    );

    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, 0, 1);
}

// A new committed play supersedes the pending jump; from song-4 the next
// Previous steps from the last actually recorded track, song-3 (index 2),
// rather than from the abandoned jump's target.
#[test]
fn previous_with_shuffle_resumes_from_the_last_recorded_play_after_a_new_track() {
    let (mut player, state) = player_stepping_from(None);
    state.blocking_lock().shuffle = true;
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");
    record_a_play(&mut player, &state, "song-3");

    let jump = update(&mut player, Message::PreviousTrack);
    assert_track_selected(jump, 0, 1);
    record_a_play(&mut player, &state, "song-4");

    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, 0, 2);
}

// A failed play reports a current track but is not playing, so `TrackPlayed`
// must not record it. With history song-1, song-2 and the failed song-3 left
// out, `previous(.., None)` returns song-2; a wrongly recorded song-3 would
// return song-3 instead.
#[test]
fn track_played_ignores_a_failed_completion() {
    let (mut player, state) = player_stepping_from(None);
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");
    {
        let mut state = state.blocking_lock();
        state.current_track = Some("song-3".to_string());
        state.is_playing = false;
    }

    let _ = update(
        &mut player,
        Message::TrackPlayed {
            generation: 0,
            committed: false,
        },
    );

    assert_eq!(
        player.play_history.previous(&stepping_songs(), None),
        Some("song-2".to_string())
    );
}

// A play can commit and then be paused before its completion reaches
// `update`: the Pause's `TransportSettled` may be processed first, leaving
// `is_playing` false although the play committed and the preview is merely
// paused. `TrackPlayed` must still record that play, or Shuffle's Previous
// retraces past it. The completion carries the play's own outcome for exactly
// this case; the shared flags cannot stand in, because Pause clears
// `is_playing`. Without the carried outcome this test sees history song-1,
// song-2 only and the paused song-3 is treated as a failed play.
#[test]
fn track_played_records_a_play_paused_before_its_completion() {
    let (mut player, state) = player_stepping_from(None);
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");
    {
        let mut state = state.blocking_lock();
        state.current_track = Some("song-3".to_string());
        state.pause();
    }

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

    record_a_play(&mut player, &state, "song-4");
    assert_eq!(
        player
            .play_history
            .previous(&stepping_songs(), Some("song-4")),
        Some("song-3".to_string())
    );
}

// A completion from an older generation belongs to a superseded play and must
// not record its track. The same song-2 assertion distinguishes the expected
// history from one that wrongly recorded song-3.
#[test]
fn track_played_ignores_a_superseded_completion() {
    let (mut player, state) = player_stepping_from(None);
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");
    player
        .plays_generation
        .store(1, std::sync::atomic::Ordering::SeqCst);
    {
        let mut state = state.blocking_lock();
        state.current_track = Some("song-3".to_string());
        state.is_playing = true;
    }

    let _ = update(
        &mut player,
        Message::TrackPlayed {
            generation: 0,
            committed: true,
        },
    );

    assert_eq!(
        player.play_history.previous(&stepping_songs(), None),
        Some("song-2".to_string())
    );
}

/// Feeds a `*Loaded` message built from `items` back through `update` and
/// asserts the list lands in `buffer` unchanged and that the matching
/// `loading` flag is cleared — a `*Loaded` reply reaches `BrowseList::store`,
/// which clears the flag (see `views::browse::rows::browse_placeholder`), so the reply must
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

#[test]
fn artists_loaded_populates_list() {
    let (mut player, _state) = test_player();
    let artists = vec![sample_artist()];

    assert_loaded_populates_list(
        &mut player,
        artists,
        Message::ArtistsLoaded,
        |player| &mut player.artists.items,
        |player| player.artists.loading,
    );
}

#[test]
fn albums_loaded_populates_list() {
    let (mut player, _state) = test_player();
    let albums = vec![sample_album()];

    assert_loaded_populates_list(
        &mut player,
        albums,
        Message::AlbumsLoaded,
        |player| &mut player.albums.items,
        |player| player.albums.loading,
    );
}

#[test]
fn songs_loaded_populates_list() {
    let (mut player, _state) = test_player();
    let songs = vec![sample_song()];

    assert_loaded_populates_list(
        &mut player,
        songs,
        Message::SongsLoaded,
        |player| &mut player.songs.items,
        |player| player.songs.loading,
    );
}

#[test]
fn all_songs_loaded_stores_the_flat_list() {
    let (mut player, _state) = test_player();
    let songs = vec![sample_song()];

    assert_loaded_populates_list(
        &mut player,
        songs,
        Message::AllSongsLoaded,
        |player| &mut player.all_songs.items,
        |player| player.all_songs.loading,
    );

    // The flat list caches each row's display title and formatted length when
    // it is stored, in row order, so a view refresh borrows them instead of
    // re-joining every artist and title and formatting every duration again
    // (see `BrowseList::store_with_row_text`).
    let _ = update(&mut player, Message::AllSongsLoaded(stepping_songs()));
    assert_eq!(
        player.all_songs.titles,
        [
            "The Sample Band - One",
            "The Sample Band - Two",
            "The Sample Band - Three"
        ]
    );
    assert_eq!(player.all_songs.labels, ["3:30", "2:05", "1:35"]);
}

#[test]
fn all_albums_loaded_stores_the_flat_list() {
    let (mut player, _state) = test_player();
    let albums = vec![sample_album()];

    assert_loaded_populates_list(
        &mut player,
        albums,
        Message::AllAlbumsLoaded,
        |player| &mut player.all_albums.items,
        |player| player.all_albums.loading,
    );
}

// Selecting a row in the flat All Songs view must play the song the flat
// list rendered, not the same index in the browsed `songs` buffer. Both
// lists use `Message::TrackSelected` and, after one store each, carry the
// same epoch — so only routing the press to `all_songs` while the All Songs
// view is current makes the flat list's song play. Without that route the
// press resolves against `songs` and plays the wrong song.
#[tokio::test]
async fn track_selected_in_all_songs_plays_the_flat_lists_song() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
    // Both buffers have a song at index 0 and both epochs land on 1:
    // `songs` gets album-1's "song-1", the flat list gets album-2's
    // "song-4".
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let mut flat = second_album_songs();
    flat[0].preview_url = Some(PREVIEW_URL.to_string());
    let _ = update(&mut player, Message::AllSongsLoaded(flat));
    player.current_view = CurrentView::AllSongs;

    let task = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    drive_task(task, "flat-list play", |_| {}).await;

    assert_eq!(state.lock().await.current_track.as_deref(), Some("song-4"));
    assert_preview_calls(&recording.calls(), &[]);
}

#[test]
fn sort_reorders_artists_case_insensitively() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![
        Artist {
            id: "banana".to_string(),
            name: "banana".to_string(),
            album_count: 0,
        },
        Artist {
            id: "apple".to_string(),
            name: "Apple".to_string(),
            album_count: 0,
        },
        Artist {
            id: "cherry".to_string(),
            name: "cherry".to_string(),
            album_count: 0,
        },
    ];
    // A list with rows has finished its fetch; the startup list is still
    // marked loading until a reply lands.
    player.artists.loading = false;

    let _ = update(&mut player, Message::SortBrowse);

    let names: Vec<&str> = player
        .artists
        .items
        .iter()
        .map(|artist| artist.name.as_str())
        .collect();
    assert_eq!(names, ["Apple", "banana", "cherry"]);
}

// The documented sort is stable: rows whose rendered text compares equal
// (ignoring case) keep the library order. Every other sort test uses distinct
// keys, so a regression to an unstable sort could reorder these equal-key rows
// and leave them green. Enough rows are used that an unstable sort cannot keep
// them in place by luck on a short list. The same rows also pin the saved
// permutation when keys collide: Back must still restore their stored order.
#[test]
fn sort_is_stable_for_equal_keys_and_back_restores_them() {
    const EQUAL_ROWS: usize = 32;
    let same_rows = || {
        (0..EQUAL_ROWS).map(|index| Artist {
            id: format!("same-{index:02}"),
            name: "Same".to_string(),
            album_count: 0,
        })
    };
    let (mut player, _state) = test_player();
    player.artists.items = std::iter::once(Artist {
        id: "zulu".to_string(),
        name: "Zulu".to_string(),
        album_count: 0,
    })
    .chain(same_rows())
    .chain(std::iter::once(Artist {
        id: "alpha".to_string(),
        name: "alpha".to_string(),
        album_count: 0,
    }))
    .collect();
    player.artists.loading = false;

    let _ = update(&mut player, Message::SortBrowse);

    let sorted_ids: Vec<String> = player
        .artists
        .items
        .iter()
        .map(|artist| artist.id.clone())
        .collect();
    let mut expected_sorted = vec!["alpha".to_string()];
    expected_sorted.extend((0..EQUAL_ROWS).map(|index| format!("same-{index:02}")));
    expected_sorted.push("zulu".to_string());
    assert_eq!(
        sorted_ids, expected_sorted,
        "equal keys must keep library order"
    );

    player.current_view = CurrentView::Albums;
    let _ = update(&mut player, Message::Back);

    let restored_ids: Vec<String> = player
        .artists
        .items
        .iter()
        .map(|artist| artist.id.clone())
        .collect();
    let mut expected_restored = vec!["zulu".to_string()];
    expected_restored.extend((0..EQUAL_ROWS).map(|index| format!("same-{index:02}")));
    expected_restored.push("alpha".to_string());
    assert_eq!(restored_ids, expected_restored);
}

#[test]
fn sort_reorders_albums_by_their_display_title() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Albums;
    // The display titles are "Zulu - Alpha" and "alpha - Zulu", so sorting
    // by the rendered text puts the second row first; sorting by `title`
    // alone would leave this order unchanged.
    player.albums.items = vec![
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
    ];

    let _ = update(&mut player, Message::SortBrowse);

    let ids: Vec<&str> = player
        .albums
        .items
        .iter()
        .map(|album| album.id.as_str())
        .collect();
    assert_eq!(ids, ["alpha", "zulu"]);
}

#[test]
fn sort_reorders_playlists_by_name() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Playlists;
    player.playlists.items = vec![
        Playlist {
            id: "banana".to_string(),
            name: "banana".to_string(),
        },
        Playlist {
            id: "apple".to_string(),
            name: "Apple".to_string(),
        },
        Playlist {
            id: "cherry".to_string(),
            name: "cherry".to_string(),
        },
    ];

    let _ = update(&mut player, Message::SortBrowse);

    let names: Vec<&str> = player
        .playlists
        .items
        .iter()
        .map(|playlist| playlist.name.as_str())
        .collect();
    assert_eq!(names, ["Apple", "banana", "cherry"]);
}

#[test]
fn sort_reorders_songs_and_keeps_their_cached_text_aligned() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Songs;
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let _ = update(&mut player, Message::SortBrowse);

    let ids: Vec<&str> = player
        .songs
        .items
        .iter()
        .map(|song| song.id.as_str())
        .collect();
    assert_eq!(ids, ["song-1", "song-3", "song-2"]);
    assert_eq!(
        player.songs.titles,
        [
            "The Sample Band - One",
            "The Sample Band - Three",
            "The Sample Band - Two"
        ]
    );
    assert_eq!(player.songs.labels, ["3:30", "1:35", "2:05"]);
}

#[test]
fn sort_reorders_all_songs_and_keeps_their_cached_text_aligned() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::AllSongs;
    let _ = update(&mut player, Message::AllSongsLoaded(stepping_songs()));

    let _ = update(&mut player, Message::SortBrowse);

    let ids: Vec<&str> = player
        .all_songs
        .items
        .iter()
        .map(|song| song.id.as_str())
        .collect();
    assert_eq!(ids, ["song-1", "song-3", "song-2"]);
    assert_eq!(
        player.all_songs.titles,
        [
            "The Sample Band - One",
            "The Sample Band - Three",
            "The Sample Band - Two"
        ]
    );
    assert_eq!(player.all_songs.labels, ["3:30", "1:35", "2:05"]);
}

#[test]
fn sort_reorders_all_albums_by_their_display_title() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::AllAlbums;
    player.all_albums.items = vec![
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
    ];

    let _ = update(&mut player, Message::SortBrowse);

    let ids: Vec<&str> = player
        .all_albums
        .items
        .iter()
        .map(|album| album.id.as_str())
        .collect();
    assert_eq!(ids, ["alpha", "zulu"]);
}

#[test]
fn sort_bumps_the_epoch_so_a_pre_sort_press_is_rejected() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![
        Artist {
            id: "banana".to_string(),
            name: "banana".to_string(),
            album_count: 0,
        },
        Artist {
            id: "apple".to_string(),
            name: "Apple".to_string(),
            album_count: 0,
        },
    ];
    // A list with rows has finished its fetch; the startup list is still
    // marked loading until a reply lands.
    player.artists.loading = false;
    let old_epoch = player.artists.epoch;

    let _ = update(&mut player, Message::SortBrowse);

    assert_eq!(player.artists.epoch, old_epoch.wrapping_add(1));
    let _ = update(
        &mut player,
        Message::ArtistSelected {
            epoch: old_epoch,
            index: 0,
        },
    );
    assert_view(&player, CurrentView::Artists);

    let new_epoch = player.artists.epoch;
    let _ = update(
        &mut player,
        Message::ArtistSelected {
            epoch: new_epoch,
            index: 0,
        },
    );
    assert_view(&player, CurrentView::Albums);
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

#[test]
fn sort_with_an_empty_or_loading_list_is_a_noop() {
    assert_sort_is_a_noop_on_an_unloaded_view(CurrentView::Artists);
    assert_sort_is_a_noop_on_an_unloaded_view(CurrentView::Albums);
    assert_sort_is_a_noop_on_an_unloaded_view(CurrentView::Songs);
    assert_sort_is_a_noop_on_an_unloaded_view(CurrentView::AllSongs);
    assert_sort_is_a_noop_on_an_unloaded_view(CurrentView::AllAlbums);
    assert_sort_is_a_noop_on_an_unloaded_view(CurrentView::Playlists);

    // A loading list is a no-op even if its buffer still holds rows: the
    // documented guarantee is about the list's state, not just its emptiness,
    // so a future transition that keeps rows while a fetch is in flight must
    // not let Sort reorder them or bump the epoch. (The current transitions
    // empty the rows in `clear` before setting `loading`, so this state is
    // constructed here to pin that rule.)
    let (mut player, _state) = test_player();
    player.artists.items = vec![
        Artist {
            id: "banana".to_string(),
            name: "banana".to_string(),
            album_count: 0,
        },
        Artist {
            id: "apple".to_string(),
            name: "Apple".to_string(),
            album_count: 0,
        },
    ];
    player.artists.loading = true;
    let epoch = player.artists.epoch;

    let _ = update(&mut player, Message::SortBrowse);

    let names: Vec<&str> = player
        .artists
        .items
        .iter()
        .map(|artist| artist.name.as_str())
        .collect();
    assert_eq!(names, ["banana", "Apple"]);
    assert_eq!(player.artists.epoch, epoch);
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

// The "Browse row order" invariant says a sort leaves every other list, the
// current view, the search state, and playback alone. Each test above seeds
// only the list it sorts, so a handler that also sorted another list (or
// cleared a search or playback) could pass them all. Seed every list and both
// kinds of state, then sort each view in turn and check only its own list's
// rows and epoch moved.
#[test]
fn sort_leaves_other_lists_view_search_and_playback_untouched() {
    let views = [
        CurrentView::Artists,
        CurrentView::Albums,
        CurrentView::Songs,
        CurrentView::AllSongs,
        CurrentView::AllAlbums,
        CurrentView::Playlists,
    ];
    for view in views {
        let target = match &view {
            CurrentView::Artists => 0,
            CurrentView::Albums => 1,
            CurrentView::Songs => 2,
            CurrentView::AllSongs => 3,
            CurrentView::AllAlbums => 4,
            CurrentView::Playlists => 5,
        };
        let expected_sorted = match &view {
            CurrentView::Artists => vec!["alpha", "bravo", "Zulu"],
            CurrentView::Albums | CurrentView::AllAlbums => vec!["alpha", "zulu"],
            CurrentView::Songs | CurrentView::AllSongs => vec!["song-1", "song-3", "song-2"],
            CurrentView::Playlists => vec!["alpha", "zulu"],
        };
        let view_name = format!("{view:?}");

        let (mut player, state) = test_player();
        seed_lists_for_sort_locality(&mut player);
        player.current_view = view;
        player.search_query = "echo".to_string();
        player.search_active = true;
        {
            let mut shared = state.blocking_lock();
            shared.current_track = Some("song-2".to_string());
            shared.is_playing = true;
            shared.repeat = true;
            shared.shuffle = true;
            shared.set_volume(0.3);
            shared.set_balance(-0.4);
            shared.eq_enabled = true;
        }
        let state_before = format!("{:?}", *state.blocking_lock());
        let ids_before = browse_list_ids(&player);
        let epochs_before = browse_list_epochs(&player);
        let view_before = std::mem::discriminant(&player.current_view);

        let _ = update(&mut player, Message::SortBrowse);

        let ids_after = browse_list_ids(&player);
        let epochs_after = browse_list_epochs(&player);
        assert_eq!(ids_after[target], expected_sorted, "sorting {view_name}");
        assert_eq!(
            epochs_after[target],
            epochs_before[target].wrapping_add(1),
            "sorting {view_name} must bump its own epoch"
        );
        for index in 0..ids_after.len() {
            if index == target {
                continue;
            }
            assert_eq!(
                ids_after[index], ids_before[index],
                "sorting {view_name} touched list {index}'s rows"
            );
            assert_eq!(
                epochs_after[index], epochs_before[index],
                "sorting {view_name} bumped list {index}'s epoch"
            );
        }
        assert_eq!(
            std::mem::discriminant(&player.current_view),
            view_before,
            "sorting {view_name} changed the view"
        );
        assert_eq!(player.search_query, "echo", "sorting {view_name}");
        assert!(
            player.search_active,
            "sorting {view_name} cleared the search"
        );
        assert_eq!(
            format!("{:?}", *state.blocking_lock()),
            state_before,
            "sorting {view_name} changed playback state"
        );
    }
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

// The reorder is one-shot: leaving a sorted parent list and returning with
// Back must show the library's order again (README, "reopening the list",
// and the "Browse row order" invariant). Back does not refetch the parent, so
// the list has to restore the order it had before Sort.
#[test]
fn back_restores_a_sorted_artists_list_to_library_order() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![
        Artist {
            id: "zulu".to_string(),
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
    player.artists.loading = false;
    player.current_view = CurrentView::Artists;

    let _ = update(&mut player, Message::SortBrowse);
    assert_eq!(artist_names(&player), ["alpha", "bravo", "Zulu"]);
    // A second Sort must not lose the library order: the saved permutation
    // composes with the second reorder instead of being overwritten.
    let _ = update(&mut player, Message::SortBrowse);

    // Opening an artist replaces the albums list, not the artists one.
    player.current_view = CurrentView::Albums;
    let epoch = player.artists.epoch;
    let _ = update(&mut player, Message::Back);

    assert_eq!(artist_names(&player), ["Zulu", "alpha", "bravo"]);
    assert_eq!(player.artists.epoch, epoch.wrapping_add(1));
}

// A fresh fetch that replaces a sorted parent list must drop the saved sort
// permutation: the replacement's rows are already in library order, so Back
// must not reapply the old sort's inverse to them. The sign-in re-issuing
// `LoadArtists` is the production path that can replace the list the user
// already sorted while a child list is open.
#[test]
fn a_fresh_artists_reply_drops_the_saved_sort_permutation() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![
        Artist {
            id: "zulu".to_string(),
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
    player.artists.loading = false;
    let _ = update(&mut player, Message::SortBrowse);
    assert_eq!(artist_names(&player), ["alpha", "bravo", "Zulu"]);

    // The refresh lands with a different same-length order while the child
    // list is showing, so the saved permutation no longer describes these
    // rows.
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![
            Artist {
                id: "bravo".to_string(),
                name: "bravo".to_string(),
                album_count: 0,
            },
            Artist {
                id: "zulu".to_string(),
                name: "Zulu".to_string(),
                album_count: 0,
            },
            Artist {
                id: "alpha".to_string(),
                name: "alpha".to_string(),
                album_count: 0,
            },
        ]),
    );
    assert_eq!(artist_names(&player), ["bravo", "Zulu", "alpha"]);

    player.current_view = CurrentView::Albums;
    let epoch = player.artists.epoch;
    let _ = update(&mut player, Message::Back);

    assert_eq!(
        artist_names(&player),
        ["bravo", "Zulu", "alpha"],
        "a fresh reply's library order must survive Back"
    );
    assert_eq!(
        player.artists.epoch, epoch,
        "restoring a list with no saved sort must be a no-op"
    );
}

// The same one-shot rule holds for the other in-memory parent list: songs
// opened from an album return to that album's list, whose library order Back
// must restore after a Sort.
#[test]
fn back_from_songs_restores_a_sorted_albums_list_to_library_order() {
    let mut player = player_in_albums();
    let _ = update(
        &mut player,
        Message::AlbumsLoaded(vec![
            Album {
                id: "zulu".to_string(),
                title: "Zulu".to_string(),
                artist_id: "artist-1".to_string(),
                artist: "Zulu".to_string(),
                duration_ms: 0,
            },
            Album {
                id: "alpha".to_string(),
                title: "alpha".to_string(),
                artist_id: "artist-1".to_string(),
                artist: "alpha".to_string(),
                duration_ms: 0,
            },
        ]),
    );
    let _ = update(&mut player, Message::SortBrowse);
    assert_eq!(player.albums.items[0].id, "alpha");

    let epoch = player.albums.epoch;
    let _ = update(&mut player, Message::AlbumSelected { epoch, index: 0 });
    assert_view(&player, CurrentView::Songs);
    let _ = update(&mut player, Message::Back);

    let ids: Vec<&str> = player
        .albums
        .items
        .iter()
        .map(|album| album.id.as_str())
        .collect();
    assert_eq!(ids, ["zulu", "alpha"]);
}

// The flat All Albums list is the other parent Back can return to after a
// Sort: opening an album leaves it in memory, so Back must restore its
// library order rather than leave the sorted order showing.
#[test]
fn back_from_all_albums_songs_restores_a_sorted_flat_list_to_library_order() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::AllAlbums;
    player.all_albums.loading = false;
    player.all_albums.items = vec![
        Album {
            id: "zulu".to_string(),
            title: "Zulu".to_string(),
            artist_id: "artist-1".to_string(),
            artist: "Zulu".to_string(),
            duration_ms: 0,
        },
        Album {
            id: "alpha".to_string(),
            title: "alpha".to_string(),
            artist_id: "artist-1".to_string(),
            artist: "alpha".to_string(),
            duration_ms: 0,
        },
    ];

    let _ = update(&mut player, Message::SortBrowse);
    let sorted: Vec<&str> = player
        .all_albums
        .items
        .iter()
        .map(|album| album.id.as_str())
        .collect();
    assert_eq!(sorted, ["alpha", "zulu"]);

    let epoch = player.all_albums.epoch;
    let _ = update(&mut player, Message::AlbumSelected { epoch, index: 0 });
    assert_view(&player, CurrentView::Songs);
    assert_eq!(player.songs_origin, SongsOrigin::AllAlbums);

    let _ = update(&mut player, Message::Back);

    let restored: Vec<&str> = player
        .all_albums
        .items
        .iter()
        .map(|album| album.id.as_str())
        .collect();
    assert_view(&player, CurrentView::AllAlbums);
    assert_eq!(restored, ["zulu", "alpha"]);
}

// Playlists is the remaining in-memory parent Back restores: a sorted list
// whose playlist was opened must come back in library order.
#[test]
fn back_from_a_playlists_songs_restores_a_sorted_list_to_library_order() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Playlists;
    player.playlists.loading = false;
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

    let _ = update(&mut player, Message::SortBrowse);
    let sorted: Vec<&str> = player
        .playlists
        .items
        .iter()
        .map(|playlist| playlist.id.as_str())
        .collect();
    assert_eq!(sorted, ["alpha", "zulu"]);

    let epoch = player.playlists.epoch;
    let _ = update(&mut player, Message::PlaylistSelected { epoch, index: 0 });
    assert_view(&player, CurrentView::Songs);
    assert_eq!(player.songs_origin, SongsOrigin::Playlist);

    let _ = update(&mut player, Message::Back);

    let restored: Vec<&str> = player
        .playlists
        .items
        .iter()
        .map(|playlist| playlist.id.as_str())
        .collect();
    assert_view(&player, CurrentView::Playlists);
    assert_eq!(restored, ["zulu", "alpha"]);
}

// Songs view draws from `all_songs`, so Next there moves through the flat
// rows, not the browsed `songs` buffer. `songs` holds a single song while the
// flat list holds the three-song stepping album, so a Next that stepped
// `songs` could not yield the flat list's second row (epoch 1, index 1).
#[test]
fn next_track_in_all_songs_steps_the_flat_list() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));
    let _ = update(&mut player, Message::AllSongsLoaded(stepping_songs()));
    player.current_view = CurrentView::AllSongs;
    player.state.blocking_lock().current_track = Some("song-1".to_string());

    let task = update(&mut player, Message::NextTrack);
    assert_track_selected(task, 1, 1);
}

// A failed browse fetch must not read as an empty library: the failure's
// report is stored for the list (so the view can show it), the list is marked
// not-loading, and the buffer stays empty. A later successful reply clears the
// stored error, so the panel returns to the list's rows.
#[test]
fn albums_load_failed_stores_the_error_until_the_next_load() {
    let (mut player, _state) = test_player();
    player.albums.items = vec![sample_album()];
    player.albums.loading = true;

    let report = "music-library fetch failed (loading albums for artist \"artist-1\"): boom";
    let _ = update(&mut player, Message::AlbumsLoadFailed(report.to_string()));

    assert!(player.albums.items.is_empty());
    assert!(!player.albums.loading);
    assert_eq!(player.albums.error.as_deref(), Some(report));

    let _ = update(&mut player, Message::AlbumsLoaded(vec![sample_album()]));

    assert!(player.albums.error.is_none());
}

// The Artists and Songs failure arms are distinct `update` arms from the
// Albums one (each handles its own message variant), so driving only
// `AlbumsLoadFailed` leaves the others unpinned — a copy-paste that routed
// a songs failure into `artists.error`, say, would still compile and pass the
// suite. This drives every browse failure and asserts each report lands in
// its own list's error field with the others untouched, and that each failure
// clears its own buffer and loading flag.
#[test]
fn each_browse_failure_stores_its_report_in_its_own_list() {
    let (mut player, _state) = test_player();
    seed_browse_lists(&mut player);
    player.artists.loading = true;
    player.albums.loading = true;
    player.songs.loading = true;
    player.all_songs.loading = true;
    player.all_albums.loading = true;
    player.playlists.loading = true;

    let artists_report = "music-library fetch failed (loading favorite artists): artists boom";
    let albums_report =
        "music-library fetch failed (loading albums for artist \"artist-1\"): albums boom";
    let songs_report =
        "music-library fetch failed (loading songs from album \"album-1\"): songs boom";
    let all_songs_report = "music-library fetch failed (loading all library songs): all songs boom";
    let all_albums_report =
        "music-library fetch failed (loading all library albums): all albums boom";
    let playlists_report = "music-library fetch failed (loading library playlists): playlists boom";

    let _ = update(
        &mut player,
        Message::ArtistsLoadFailed(artists_report.to_string()),
    );
    let _ = update(
        &mut player,
        Message::AlbumsLoadFailed(albums_report.to_string()),
    );
    let _ = update(
        &mut player,
        Message::SongsLoadFailed(songs_report.to_string()),
    );
    let _ = update(
        &mut player,
        Message::AllSongsLoadFailed(all_songs_report.to_string()),
    );
    let _ = update(
        &mut player,
        Message::AllAlbumsLoadFailed(all_albums_report.to_string()),
    );
    let _ = update(
        &mut player,
        Message::PlaylistsLoadFailed(playlists_report.to_string()),
    );

    assert_browse_lists_empty(&player);
    assert!(!player.artists.loading);
    assert!(!player.albums.loading);
    assert!(!player.songs.loading);
    assert!(!player.all_songs.loading);
    assert!(!player.all_albums.loading);
    assert!(!player.playlists.loading);
    assert_eq!(player.artists.error.as_deref(), Some(artists_report));
    assert_eq!(player.albums.error.as_deref(), Some(albums_report));
    assert_eq!(player.songs.error.as_deref(), Some(songs_report));
    assert_eq!(player.all_songs.error.as_deref(), Some(all_songs_report));
    assert_eq!(player.all_albums.error.as_deref(), Some(all_albums_report));
    assert_eq!(player.playlists.error.as_deref(), Some(playlists_report));
}

// `BrowseList::clear` runs on every navigation step and must reset all three
// pieces of the entered list's state: the stale rows, the loading flag (so
// the panel shows "Loading…"), and any earlier fetch failure (so the retry
// does not keep showing the old error). This pins the error reset in
// particular — a regression that dropped `*error = None` would leave the
// previous failure on screen for the whole retry.
#[test]
fn browse_list_clear_clears_the_buffer_loading_and_error() {
    let mut list = BrowseList::new(false);
    list.items = vec![sample_album()];
    list.error = Some("music-library fetch failed: boom".to_string());

    list.clear();

    assert!(list.items.is_empty());
    assert!(list.loading);
    assert!(list.error.is_none());
}

// The `known_tracks` index backs the Now Playing bar's title and time
// lookup, and it is populated only when a track is played — not when an album
// is loaded. Folding a browsed album's songs into the index would make it grow
// with every album the user visits while the bar only ever reads the playing
// track's entry, so the browse path must leave the index untouched and
// `TrackSelected` must record the played song's title and duration.
#[test]
fn songs_loaded_does_not_populate_the_known_tracks_index() {
    let (mut player, _state) = test_player();

    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    assert!(player.known_tracks.is_empty());
}

#[test]
fn track_selected_records_the_played_tracks_title_and_time() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 1 });

    let track = player.known_tracks.get("song-2").expect("song-2 recorded");
    assert_eq!(track.title, "Two");
    assert_eq!(track.number, "2. ");
    assert_eq!(track.time, "2:05");
}

// The number is the 1-based position of the row that selected the track, so
// the first row records `"1. "` and the second `"2. "`.
#[test]
fn track_selected_records_the_selected_rows_number() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    assert_eq!(
        player
            .known_tracks
            .get("song-1")
            .map(|track| track.number.as_str()),
        Some("1. ")
    );

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 1 });
    assert_eq!(
        player
            .known_tracks
            .get("song-2")
            .map(|track| track.number.as_str()),
        Some("2. ")
    );
}

// Re-playing the same id from a row in another list refreshes its cached
// number (and the rest of its fields) instead of keeping the first play's
// number. song-1 is row 1 of `stepping_songs` and row 2 of the second list.
#[test]
fn replaying_a_track_from_another_row_updates_its_number() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    assert_eq!(
        player
            .known_tracks
            .get("song-1")
            .map(|track| track.number.as_str()),
        Some("1. ")
    );

    let mut second_list = second_album_songs();
    second_list.push(sample_song());
    let _ = update(&mut player, Message::SongsLoaded(second_list));
    let _ = update(&mut player, Message::TrackSelected { epoch: 2, index: 1 });

    let track = player.known_tracks.get("song-1").expect("song-1 recorded");
    assert_eq!(track.number, "2. ");
    assert_eq!(track.title, "Opening");
}

// The marquee timer advances the title window by one character without
// scheduling any work. The timer itself runs only while the line is too long
// to fit the LCD (see `the_marquee_timer_runs_only_while_the_line_needs_to_scroll`),
// so every tick scrolls the bar.
#[test]
fn marquee_tick_advances_the_offset() {
    let (mut player, _state) = test_player();

    assert_message_schedules_no_work(&mut player, Message::MarqueeTick);
    assert_eq!(player.marquee_offset, 1);
    assert_message_schedules_no_work(&mut player, Message::MarqueeTick);
    assert_eq!(player.marquee_offset, 2);
}

// Every message makes iced rebuild the whole screen, browse list included, so
// the marquee timer must not run for a line that fits the LCD whole: it would
// rebuild the UI four times a second to redraw a bar that cannot change. The
// keyboard listener always runs, so the subscription set is the listener alone
// until the line needs to scroll.
#[test]
fn the_marquee_timer_runs_only_while_the_line_needs_to_scroll() {
    assert_eq!(subscriptions(false, false, false).len(), 1);
    assert_eq!(subscriptions(true, false, false).len(), 2);
}

// The position poll has the same cost as the marquee: every tick and reply
// rebuilds the whole screen, browse list included. A stopped player's view
// clears its current track's time readout (except the `--:--` placeholder), so
// the poll must leave the subscription set then, and join it again only while
// playback is running or the backend still holds a preview whose position a
// seek can move.
#[test]
fn the_position_timer_runs_only_while_there_is_a_position_to_refresh() {
    assert_eq!(subscriptions(false, true, false).len(), 2);
    assert_eq!(subscriptions(true, true, false).len(), 3);

    let (mut player, state) = test_player();
    assert!(!player.needs_position_poll());

    state.blocking_lock().play();
    assert!(player.needs_position_poll());

    player.playback_position = Some(Duration::from_secs(45));
    state.blocking_lock().pause();
    assert!(player.needs_position_poll());

    player.playback_position = None;
    state.blocking_lock().stop();
    assert!(!player.needs_position_poll());
}

// The spectrum poll costs the same as the others: it is subscribed only while
// playback runs, since a paused or stopped player keeps the last bars the view
// already stored.
#[test]
fn the_spectrum_timer_runs_only_while_playback_runs() {
    assert_eq!(subscriptions(false, false, false).len(), 1);
    assert_eq!(subscriptions(false, false, true).len(), 2);
    assert_eq!(subscriptions(true, true, true).len(), 4);

    let (player, state) = test_player();
    assert!(!player.needs_spectrum_poll());

    state.blocking_lock().play();
    assert!(player.needs_spectrum_poll());

    state.blocking_lock().pause();
    assert!(!player.needs_spectrum_poll());

    state.blocking_lock().play();
    assert!(player.needs_spectrum_poll());

    state.blocking_lock().stop();
    assert!(!player.needs_spectrum_poll());
}

// A preview can be paused before its first position reply lands: the backend
// still holds it and a seek can move it, so the poll must stay subscribed even
// though `playback_position` is still `None`.
#[tokio::test]
async fn a_paused_preview_needs_the_position_poll_before_its_first_reply() {
    let (player, state) = player_playing_preview(Arc::new(RecordingAudio::default())).await;
    assert!(player.playback_position.is_none());

    state.lock().await.pause();
    assert!(!state.lock().await.is_playing);
    assert!(player.needs_position_poll());
}

// The timer is subscribed from the player's own view of the bar: the same
// number, label, and artist the view renders, measured the same way. A line
// that fits leaves the timer unsubscribed; a line past the LCD's character
// budget needs it, and the playlist number prefix counts towards that budget.
#[test]
fn the_player_knows_when_the_now_playing_line_needs_the_marquee() {
    let (mut player, state) = test_player();

    // Nothing is current: the bar reads "Nothing", which fits.
    assert!(!player.needs_marquee());

    // A known track with a short title fits — until its row number joins the
    // line. "One" (3) plus "The Sample Band" (15) and " - " (3) is 21
    // characters; the 3-character `"1. "` prefix pushes it past the
    // 22-character budget.
    player.known_tracks.insert(
        "song-1".to_string(),
        views::KnownTrack {
            title: "One".to_string(),
            artist: "The Sample Band".to_string(),
            number: String::new(),
            time: "3:30".to_string(),
            duration_ms: 210_000,
        },
    );
    state.blocking_lock().current_track = Some("song-1".to_string());
    assert!(!player.needs_marquee());
    player
        .known_tracks
        .get_mut("song-1")
        .expect("seeded")
        .number = "1. ".to_string();
    assert!(player.needs_marquee());

    // A title that pushes the line past the LCD's budget needs the marquee.
    player.known_tracks.insert(
        "song-2".to_string(),
        views::KnownTrack {
            title: "A Title Too Long For The LCD".to_string(),
            artist: "The Sample Band".to_string(),
            number: "2. ".to_string(),
            time: "3:30".to_string(),
            duration_ms: 210_000,
        },
    );
    state.blocking_lock().current_track = Some("song-2".to_string());
    assert!(player.needs_marquee());
}

// A slow UI must not kill the timer: the futures channel holds
// `buffer + senders` messages — two for its one sender — so after the receiver
// lets two ticks queue, the next send finds it full. The sender must drop that
// tick and keep running rather than stop forever. Sleeping through several
// tick periods without polling first forces those full-channel sends; the
// third tick read below can then only arrive if the thread survived them.
#[test]
fn marquee_ticks_keep_ticking_when_the_receiver_falls_behind() {
    use futures::StreamExt;

    let mut ticks = marquee_tick_stream();
    // Queue both messages the channel can hold, then leave the sender several
    // ticks to attempt more sends while the channel is full.
    std::thread::sleep(MARQUEE_TICK * 6);

    let first = futures::executor::block_on(await_or_timeout(
        "first marquee tick",
        TASK_TIMEOUT,
        ticks.next(),
    ));
    assert!(matches!(first, Some(Message::MarqueeTick)));
    let second = futures::executor::block_on(await_or_timeout(
        "second marquee tick",
        TASK_TIMEOUT,
        ticks.next(),
    ));
    assert!(matches!(second, Some(Message::MarqueeTick)));
    let third = futures::executor::block_on(await_or_timeout(
        "marquee tick after a full channel",
        TASK_TIMEOUT,
        ticks.next(),
    ));
    assert!(
        matches!(third, Some(Message::MarqueeTick)),
        "the timer stopped after a full channel"
    );
}

// A refused timer thread must not take the player down: `std::thread::spawn`
// panics when the OS refuses a thread, and iced builds the subscription's
// stream on its main thread while it updates the subscriptions, so that panic
// would unwind the event loop and kill the app over a cosmetic timer. The
// fallible spawn reports the refusal and ends the tick stream instead, so the
// app keeps running with a Now Playing title that no longer scrolls.
#[test]
fn a_refused_marquee_thread_ends_the_tick_stream_instead_of_panicking() {
    use futures::StreamExt;

    let mut ticks = marquee_tick_stream_with(|_work| {
        Err(std::io::Error::other("the OS refused another thread"))
    });

    let next = futures::executor::block_on(await_or_timeout(
        "the marquee stream after a refused thread",
        TASK_TIMEOUT,
        ticks.next(),
    ));
    assert!(
        next.is_none(),
        "a refused marquee thread must end the tick stream, not tick or panic"
    );
}

// The refusal is reported, not swallowed: the report names the cause and
// says what the user loses, so a Now Playing title that never scrolls is
// diagnosable instead of mysterious.
#[test]
fn a_refused_marquee_thread_reports_the_cause_and_the_loss() {
    let report = marquee_thread_failure_report(&std::io::Error::other("no threads left"));

    assert!(
        report.contains("no threads left"),
        "the report must name the cause, got: {report}"
    );
    assert!(
        report.contains("will not scroll"),
        "the report must say what the user loses, got: {report}"
    );
}

// The timer thread carries a name, like the audio and REST workers, so a
// panic in it is attributable. The name is spelled out here rather than read
// from the production constant, so renaming the constant fails this test.
#[test]
fn the_marquee_thread_is_named_nostalgia_marquee() {
    let (sender, receiver) = std::sync::mpsc::channel();

    spawn_marquee_thread(Box::new(move || {
        let _ = sender.send(std::thread::current().name().map(str::to_string));
    }))
    .expect("the marquee thread starts");

    let name = receiver
        .recv_timeout(TASK_TIMEOUT)
        .expect("the marquee thread reports its name");
    assert_eq!(name.as_deref(), Some("nostalgia-marquee"));
}

// A newly selected song must start its title at character zero rather than
// inheriting the previous song's scroll position.
#[test]
fn track_selected_resets_the_marquee_offset() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.marquee_offset = 7;

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });

    assert_eq!(player.marquee_offset, 0);
}

// The recording above only covers a `TrackSelected` whose index still names
// a song in the loaded `songs`, so the arm's stale-press guards are never
// entered. A stale click reaches them: the row was rendered from the album
// the user just browsed away from, so its message carries that list's epoch
// and an index into it, and by the time the click is processed `SongsLoaded`
// has replaced `songs` and bumped the epoch. The guards must leave
// `known_tracks` exactly as it was — an already-recorded title survives the
// stale click, and a press whose index names no song adds no entry (the
// index stays proportional to songs actually played). A regression that
// recorded a title before the guards would either clobber a known title or
// grow the index with stale entries.
#[test]
fn track_selected_stale_press_leaves_known_tracks_untouched() {
    let (mut player, _state) = test_player();

    // Play song-1 (title "One", index 0) while album-1 is loaded, recording
    // its title. `SongsLoaded` leaves the buffer's epoch at 1.
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });

    // Browse to album-2, replacing `songs` and bumping the epoch to 2, then
    // deliver the stale click from the old list (epoch 1) and one whose
    // index names no song in the new one.
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    let _ = update(&mut player, Message::TrackSelected { epoch: 2, index: 9 });

    // The stale clicks changed nothing: the known title survived and no entry
    // was added.
    assert_eq!(
        player
            .known_tracks
            .get("song-1")
            .map(|track| track.title.as_str()),
        Some("One")
    );
    assert_eq!(player.known_tracks.len(), 1);

    // The Now Playing bar still names the playing track rather than its id.
    assert_eq!(player.now_playing_label(Some("song-1")), "One");
}

// A play completion can reach `update` after the user has already selected a
// newer track but before that newer play commits. Pruning the title index on
// that stale completion would delete the pending track's title, and once the
// pending play committed the bar would fall back to the raw id for good. The
// completion carries its play's generation, so `update` prunes only when it is
// still the latest play; a superseded completion leaves the index alone.
#[test]
fn a_stale_play_completion_does_not_prune_a_pending_selection() {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    // song-1 plays and commits, but its completion has not reached `update`.
    let first = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    let first_done = futures::executor::block_on(task_output(first, "first play"));
    assert_eq!(
        state.blocking_lock().current_track.as_deref(),
        Some("song-1")
    );

    // The user switches to song-3 before song-1's completion is processed.
    let second = update(&mut player, Message::TrackSelected { epoch: 1, index: 2 });

    // The stale completion must not prune song-3's still-pending title.
    let _ = update(&mut player, first_done);
    assert_eq!(
        player
            .known_tracks
            .get("song-3")
            .map(|track| track.title.as_str()),
        Some("Three")
    );

    // song-3's own completion is the latest, so it prunes the index to it.
    let second_done = futures::executor::block_on(task_output(second, "second play"));
    assert_eq!(
        state.blocking_lock().current_track.as_deref(),
        Some("song-3")
    );
    let _ = update(&mut player, second_done);

    assert_eq!(player.known_tracks.len(), 1);
    assert_eq!(player.now_playing_label(Some("song-3")), "Three");
}

// When the latest play fails before committing a track, `current_track` is
// still `None`, so no entry the bar can read should remain. The completion
// must clear the index rather than leave the failed selection's title behind
// — the `None` case is exactly where an unbounded map would otherwise start.
#[test]
fn a_failed_play_completion_clears_the_index_when_nothing_committed() {
    let (mut player, state) = test_player();
    // A song with a blank id is rejected by the service seam, so the play
    // fails without committing `current_track`.
    player.songs.items = vec![Song {
        id: String::new(),
        title: "Blank".to_string(),
        artist: "The Sample Band".to_string(),
        album_id: "album-1".to_string(),
        duration_ms: 0,
        preview_url: None,
    }];

    let task = update(&mut player, Message::TrackSelected { epoch: 0, index: 0 });
    assert_eq!(player.known_tracks.len(), 1);

    let done = futures::executor::block_on(task_output(task, "failed play"));

    let _ = update(&mut player, done);

    assert_eq!(state.blocking_lock().current_track, None);
    assert!(player.known_tracks.is_empty());
}

// Browsing to a new album must *replace* the Songs view's buffer, not
// append to it: after loading album-1's songs and then browsing to
// album-2, `player.songs.items` holds only album-2's song — otherwise the
// Songs view would render stale rows from every album visited. The other
// `*Loaded` tests load only into an empty buffer, and the browse-away
// label test (`now_playing_label_keeps_the_track_name_after_browsing_to_another_album`)
// asserts the label but never the buffer, so an
// append-instead-of-replace regression in `BrowseList::store` would clear every
// existing test and only fail here.
#[test]
fn songs_loaded_replaces_the_previous_albums_songs() {
    let (mut player, _state) = test_player();

    // Load album-1's three songs, then browse to album-2 (one song).
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));

    assert_ids(&player.songs.items, |song| song.id.as_str(), &["song-4"]);
}

// The songs list caches each row's display title and formatted length when it
// is stored, so a view refresh of a large library borrows them instead of
// recomposing every row. Pin that both caches are built in row order and
// replaced with the rows, and dropped when a navigation clears the list, so a
// refresh can never show a stale title or length beside a new row.
#[test]
fn songs_loaded_caches_each_rows_display_title_and_length() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    assert_eq!(
        player.songs.titles,
        [
            "The Sample Band - One",
            "The Sample Band - Two",
            "The Sample Band - Three"
        ]
    );
    assert_eq!(player.songs.labels, ["3:30", "2:05", "1:35"]);

    // Navigate into an album: the fetch starts by clearing the previous
    // level's rows, and both caches must be dropped with them.
    player.albums.items = vec![sample_album()];
    let _ = update(&mut player, Message::AlbumSelected { epoch: 0, index: 0 });
    assert!(player.songs.titles.is_empty());
    assert!(player.songs.labels.is_empty());

    // The next load caches the new list's title and length only.
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));
    assert_eq!(player.songs.titles, ["The Sample Band - B-side"]);
    assert_eq!(player.songs.labels, ["1:35"]);
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

// The Now Playing bar must keep naming the playing track, not its raw id,
// after the user browses to a different album. `view` renders the bar's
// label through `WinampPlayer::now_playing_label`, so asserting that same
// resolution after a browse-away pins the actual bar path: if the label
// were resolved against `songs` (the currently-browsed album's list, which
// album-2's `SongsLoaded` just replaced) instead of the `known_tracks`
// entry recorded when song-1 was played, song-1 would no longer be "known"
// and the label would fall back to the raw id "song-1", failing this test.
#[test]
fn now_playing_label_keeps_the_track_name_after_browsing_to_another_album() {
    let (player, current_track) = play_song_1_then_browse_to_album_2();

    assert_eq!(player.now_playing_label(current_track.as_deref()), "One");
}

// The Now Playing bar's time readout comes from the same `known_tracks` entry
// as its label: a played track's duration must reach
// `WinampPlayer::now_playing_time`, and — like the title — survive a later
// browse to a different album that replaces `songs`. An unknown track (or no
// current track) reads the `--:--` placeholder.
#[test]
fn now_playing_time_shows_the_played_tracks_duration_after_browsing_to_another_album() {
    let (player, current_track) = play_song_1_then_browse_to_album_2();

    assert_eq!(player.now_playing_time(None), "--:--");
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");
    assert_eq!(player.now_playing_time(Some("no-such-song")), "--:--");
}

// The bar's large left readout follows the backend's reported position (or a
// held seek drag) while it is known and reads 00:00 before one is reported,
// while the right edge keeps showing the plain cached total.
#[test]
fn now_playing_elapsed_shows_the_position_while_it_is_known() {
    let (mut player, current_track) = play_song_1_then_browse_to_album_2();

    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:00"
    );
    player.playback_position = Some(Duration::from_secs(45));
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:45"
    );
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");
}

// A held seek drag previews the time the thumb points at in the large
// readout, counting down from it in remaining mode, while the right-edge total
// stays put. Clearing the drag falls back to the backend's reported position,
// proving the drag is a preview and not a replacement.
#[test]
fn now_playing_elapsed_previews_the_seek_drag() {
    let (mut player, current_track) = play_song_1_then_browse_to_album_2();
    player.playback_position = Some(Duration::from_secs(45));

    let _ = update(&mut player, Message::SeekPreviewChanged(12.0));

    assert_eq!(player.readout_position(), Some(Duration::from_secs(12)));
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:12"
    );
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");

    player.time_display = views::TimeDisplay::Remaining;
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "-03:18"
    );

    player.seek_preview = None;
    assert_eq!(player.readout_position(), Some(Duration::from_secs(45)));
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "-02:45"
    );
    player.time_display = views::TimeDisplay::Elapsed;
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:45"
    );
}

// Pause stops the backend clock but keeps the loaded player, so the stored
// elapsed readout must survive the arm and continue to render.
// `a_pause_keeps_the_loaded_player_reporting_its_position` pins the backend
// side; this pins that the UI does not clear the value on Pause.
#[test]
fn pause_keeps_the_stored_elapsed_readout() {
    let (mut player, current_track) = play_song_1_then_browse_to_album_2();
    player.playback_position = Some(Duration::from_secs(45));

    let _ = update(&mut player, Message::Pause);

    assert_eq!(player.playback_position, Some(Duration::from_secs(45)));
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:45"
    );
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");
}

// A seek moves the backend, and the next poll's reply carries the new
// position, which the bar shows. `seek_messages_drive_the_audio_backend`
// pins the backend call itself.
#[test]
fn a_position_reply_after_a_seek_updates_the_readout() {
    let (mut player, current_track) = play_song_1_then_browse_to_album_2();
    player.playback_position = Some(Duration::from_secs(45));

    let _ = update(&mut player, Message::SeekForward);
    let epoch = player.position_epoch;
    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch,
            position: Some(Duration::from_secs(50)),
        },
    );

    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:50"
    );
}

#[test]
fn track_selected_clears_the_elapsed_readout() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.playback_position = Some(Duration::from_secs(45));

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });

    assert_eq!(player.playback_position, None);
    assert_eq!(player.now_playing_elapsed(Some("song-1")), "00:00");
    assert_eq!(player.now_playing_time(Some("song-1")), "3:30");
}

#[test]
fn track_selected_clears_the_playback_format() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.playback_format = Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: None,
    });

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });

    assert_eq!(player.playback_format, None);
}

// Clicking the large readout switches it to the track's remaining time and
// back. The mode is presentation-only: the right-edge total never changes,
// Pause leaves the remaining value frozen, and a new selection counts down
// from that track's own length. The mode itself survives both transports and
// selections until the next click.
#[test]
fn toggle_time_display_switches_to_remaining_and_persists() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    player.playback_position = Some(Duration::from_secs(53));

    let _ = update(&mut player, Message::ToggleTimeDisplay);
    assert_eq!(player.time_display, views::TimeDisplay::Remaining);
    assert_eq!(player.now_playing_elapsed(Some("song-1")), "-02:37");
    assert_eq!(player.now_playing_time(Some("song-1")), "3:30");

    let _ = update(&mut player, Message::Pause);
    assert_eq!(player.now_playing_elapsed(Some("song-1")), "-02:37");

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 1 });
    assert_eq!(player.time_display, views::TimeDisplay::Remaining);
    assert_eq!(player.now_playing_elapsed(Some("song-2")), "-02:05");
    assert_eq!(player.now_playing_time(Some("song-2")), "2:05");

    let _ = update(&mut player, Message::ToggleTimeDisplay);
    assert_eq!(player.time_display, views::TimeDisplay::Elapsed);
    assert_eq!(player.now_playing_elapsed(Some("song-2")), "00:00");
    assert_eq!(player.now_playing_time(Some("song-2")), "2:05");
}

#[test]
fn toggle_time_display_with_no_track_keeps_the_placeholder() {
    let (mut player, _state) = test_player();

    let _ = update(&mut player, Message::ToggleTimeDisplay);

    assert_eq!(player.time_display, views::TimeDisplay::Remaining);
    assert_eq!(player.now_playing_elapsed(None), "--:--");
}

// The indicator strip's V control hides and shows the LCD bars. It is
// presentation-only: the stored spectrum stays, so showing the bars again
// draws the current ones, the spectrum poll keeps running while they are
// hidden, and no transport or mixer state moves.
#[test]
fn toggle_visualizer_hides_and_shows_the_bars_without_clearing_the_spectrum() {
    let (mut player, state) = test_player();
    let spectrum = recording_spectrum();
    player.spectrum = Some(spectrum);
    state.blocking_lock().play();
    let before = {
        let state = state.blocking_lock();
        (
            state.transport_mode(),
            state.is_playing,
            state.volume(),
            state.balance(),
            state.repeat,
            state.shuffle,
        )
    };
    assert!(player.visualizer_visible);
    assert!(player.needs_spectrum_poll());

    assert_message_schedules_no_work(&mut player, Message::ToggleVisualizer);
    assert!(!player.visualizer_visible);
    assert_eq!(player.spectrum, Some(spectrum));
    assert!(
        player.needs_spectrum_poll(),
        "hiding the bars must leave the spectrum poll running"
    );

    assert_message_schedules_no_work(&mut player, Message::ToggleVisualizer);
    assert!(player.visualizer_visible);
    assert_eq!(player.spectrum, Some(spectrum));
    assert!(player.needs_spectrum_poll());

    let after = {
        let state = state.blocking_lock();
        (
            state.transport_mode(),
            state.is_playing,
            state.volume(),
            state.balance(),
            state.repeat,
            state.shuffle,
        )
    };
    assert_eq!(
        after, before,
        "the visualizer toggle must leave transport and mixer state alone"
    );
}

// The root view must apply the stored flag, not just expose `visible_spectrum`:
// lay the real `view` out with a loaded spectrum and read the pixels in the
// LCD's visualizer row. Showing it paints a bright green bar; hiding it leaves
// that row empty. Passing the raw spectrum through instead of the gated one
// would keep the bars green here.
#[test]
fn hidden_visualizer_draws_no_bars_in_the_root_view() {
    let (mut player, _state) = test_player();
    player.spectrum = Some(recording_spectrum());
    let mut renderer = layout_capture::headless_renderer();

    player.visualizer_visible = true;
    let (shown_text, shown_pixels) = {
        let mut shown = view(&player);
        layout_capture::capture_text_and_pixels(&mut shown, &mut renderer, &iced::Theme::Dark)
    };

    player.visualizer_visible = false;
    let hidden_pixels = {
        let mut hidden = view(&player);
        let (_hidden_text, hidden_pixels) =
            layout_capture::capture_text_and_pixels(&mut hidden, &mut renderer, &iced::Theme::Dark);
        hidden_pixels
    };

    // `MONO` starts just below the 24px visualizer row, and the strip's `O`
    // locates its left edge (the strip itself is 12px wide). Stop 4px above the
    // status text so only the bar row's pixels are read.
    let status = shown_text.find("MONO");
    let strip_left = shown_text.find("O").x;
    let bars = iced::Rectangle {
        x: strip_left + 12.0,
        y: status.y - 24.0,
        width: 200.0,
        height: 20.0,
    };
    let shown_green = shown_pixels.max_green_in(bars);
    let hidden_green = hidden_pixels.max_green_in(bars);
    assert!(
        shown_green > 200,
        "the visible visualizer must draw its green bars: {bars:?} has green {shown_green}"
    );
    assert!(
        hidden_green < 50,
        "hiding the visualizer must leave its row without bars: {bars:?} has green {hidden_green}"
    );
}

#[test]
fn stop_clears_the_elapsed_readout() {
    let (mut player, current_track) = play_song_1_then_browse_to_album_2();
    player.playback_position = Some(Duration::from_secs(45));

    let _ = update(&mut player, Message::Stop);

    assert_eq!(player.playback_position, None);
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:00"
    );
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");
}

// A Stop resets the readout before the backend stop is scheduled, so a poll
// must wait for that transition to complete. Otherwise it could query the old
// player under the new epoch and restore the elapsed value the Stop cleared.
#[test]
fn position_poll_waits_for_a_stop_to_reach_the_backend() {
    let (mut player, current_track) = play_song_1_then_browse_to_album_2();
    player.playback_position = Some(Duration::from_secs(45));

    let _ = update(&mut player, Message::Stop);
    assert_message_schedules_no_work(&mut player, Message::PositionTick);

    let epoch = player.position_epoch;
    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch,
            position: Some(Duration::from_secs(45)),
        },
    );
    assert_eq!(player.playback_position, None);

    let _ = update(&mut player, Message::TransportSettled);
    assert_message_schedules_work(&mut player, Message::PositionTick);
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:00"
    );
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");
}

// A new selection has the same window: the play task has not yet stopped the
// previous preview when `TrackSelected` clears the readout. The matching
// `TrackPlayed` completion reopens the poll only after the play reached the
// backend.
#[test]
fn position_poll_waits_for_a_selected_tracks_play_to_settle() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.playback_position = Some(Duration::from_secs(45));

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    assert_eq!(player.playback_position, None);
    assert_message_schedules_no_work(&mut player, Message::PositionTick);

    let epoch = player.position_epoch;
    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch,
            position: Some(Duration::from_secs(45)),
        },
    );
    assert_eq!(player.playback_position, None);

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
    assert_message_schedules_work(&mut player, Message::PositionTick);
}

// Every transport completion reports one transition, so the count only
// reaches zero once every transition has reached the backend. A Pause that
// settles while a Stop is still in flight must therefore keep the poll
// waiting: the backend still holds the preview the Stop has not discarded.
#[test]
fn an_unrelated_transport_completion_keeps_the_poll_waiting() {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();

    let _ = update(&mut player, Message::Stop);
    let _ = update(&mut player, Message::Pause);
    assert_message_schedules_no_work(&mut player, Message::PositionTick);

    // The pause's completion lands first; the stop is still outstanding.
    let _ = update(&mut player, Message::TransportSettled);
    assert_message_schedules_no_work(&mut player, Message::PositionTick);

    // Now the stop's completion lands and the poll may query again.
    let _ = update(&mut player, Message::TransportSettled);
    assert_message_schedules_work(&mut player, Message::PositionTick);
}

// A reply stores its position at the current epoch, while a reply stamped with
// the epoch from before a Stop is dropped: a poll already in flight must not
// bring the previous track's elapsed value back. The Stop's completion lands
// first, so no transport transition is outstanding and the epoch check is the
// only guard that can drop the reply — a reply at the current epoch is stored,
// which shows the drop above came from the epoch and not from a pending
// transition.
#[test]
fn position_replies_store_and_stale_ones_are_dropped() {
    let (mut player, current_track) = play_song_1_then_browse_to_album_2();
    let current_epoch = player.position_epoch;

    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch: current_epoch,
            position: Some(Duration::from_secs(45)),
        },
    );
    assert_eq!(player.playback_position, Some(Duration::from_secs(45)));

    let _ = update(&mut player, Message::Stop);
    // The stop reaches the backend, so the poll is no longer waiting on a
    // transition.
    let _ = update(&mut player, Message::TransportSettled);
    assert_eq!(player.pending_position_transitions, 0);

    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch: current_epoch,
            position: Some(Duration::from_secs(20)),
        },
    );

    assert_eq!(player.playback_position, None);
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:00"
    );
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");

    // A reply stamped with the epoch the Stop left behind is stored, so the
    // stale reply above was dropped by the epoch check alone.
    let stopped_epoch = player.position_epoch;
    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch: stopped_epoch,
            position: Some(Duration::from_secs(10)),
        },
    );
    assert_eq!(player.playback_position, Some(Duration::from_secs(10)));
}

// A format reply stores exactly what the backend reported, and a later
// `None` reply clears the format (dropping the prefix and dimming both labels)
// instead of keeping the old one.
#[test]
fn playback_format_reply_stores_the_loaded_previews_format() {
    let (mut player, _state) = test_player();
    let epoch = player.position_epoch;
    let format = PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    };

    let _ = update(
        &mut player,
        Message::PlaybackFormat {
            epoch,
            format: Some(format),
        },
    );
    assert_eq!(player.playback_format, Some(format));

    let _ = update(
        &mut player,
        Message::PlaybackFormat {
            epoch,
            format: None,
        },
    );
    assert_eq!(player.playback_format, None);
}

// A format reply is dropped when it was issued before a Stop, or when it
// lands while a transport transition is still reaching the backend: either
// way it could describe the previous player. A reply at the current epoch
// with no transition pending is stored, so the drops above come from the
// guards and not from the message being ignored entirely.
#[test]
fn playback_format_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped() {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();
    let old_epoch = player.position_epoch;

    let _ = update(&mut player, Message::Stop);
    let _ = update(&mut player, Message::TransportSettled);
    let stale = PlaybackFormat {
        sample_rate_hz: 48_000,
        channels: 2,
        bitrate_kbps: None,
    };
    let _ = update(
        &mut player,
        Message::PlaybackFormat {
            epoch: old_epoch,
            format: Some(stale),
        },
    );
    assert_eq!(player.playback_format, None);

    // A pending transition drops even a reply at the current epoch.
    let current_epoch = player.position_epoch;
    let kept = PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 1,
        bitrate_kbps: None,
    };
    player.playback_format = Some(kept);
    let _ = update(&mut player, Message::Pause);
    let _ = update(
        &mut player,
        Message::PlaybackFormat {
            epoch: current_epoch,
            format: Some(stale),
        },
    );
    assert_eq!(player.playback_format, Some(kept));

    // Once the transition settles, the same reply is stored.
    let _ = update(&mut player, Message::TransportSettled);
    let _ = update(
        &mut player,
        Message::PlaybackFormat {
            epoch: current_epoch,
            format: Some(stale),
        },
    );
    assert_eq!(player.playback_format, Some(stale));
}

// Pause keeps the loaded preview's format on the status row, while Stop
// clears it immediately, like the elapsed readout.
#[test]
fn stop_clears_the_playback_format_while_pause_keeps_it() {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();
    let format = PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    };
    player.playback_format = Some(format);

    let _ = update(&mut player, Message::Pause);
    assert_eq!(player.playback_format, Some(format));

    let _ = update(&mut player, Message::Stop);
    assert_eq!(player.playback_format, None);
}

// A spectrum reply stores exactly what the backend reported, and a later
// `None` reply clears the visualizer instead of keeping the old bars.
#[test]
fn spectrum_reply_stores_the_loaded_previews_spectrum() {
    let (mut player, _state) = test_player();
    let epoch = player.position_epoch;
    let spectrum = recording_spectrum();

    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch,
            spectrum: Some(spectrum),
        },
    );
    assert_eq!(player.spectrum, Some(spectrum));

    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch,
            spectrum: None,
        },
    );
    assert_eq!(player.spectrum, None);
}

// A spectrum reply is dropped when it was issued before a Stop, or when it
// lands while a transport transition is still reaching the backend: either
// way it could describe the previous player. A reply at the current epoch
// with no transition pending is stored, so the drops above come from the
// guards and not from the message being ignored entirely.
#[test]
fn spectrum_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped() {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();
    let old_epoch = player.position_epoch;

    let _ = update(&mut player, Message::Stop);
    let _ = update(&mut player, Message::TransportSettled);
    let stale = Spectrum {
        bands: [0.25; BAND_COUNT],
    };
    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch: old_epoch,
            spectrum: Some(stale),
        },
    );
    assert_eq!(player.spectrum, None);

    // A pending transition drops even a reply at the current epoch.
    let current_epoch = player.position_epoch;
    let kept = recording_spectrum();
    player.spectrum = Some(kept);
    let _ = update(&mut player, Message::Pause);
    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch: current_epoch,
            spectrum: Some(stale),
        },
    );
    assert_eq!(player.spectrum, Some(kept));

    // Once the transition settles, the same reply is stored.
    let _ = update(&mut player, Message::TransportSettled);
    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch: current_epoch,
            spectrum: Some(stale),
        },
    );
    assert_eq!(player.spectrum, Some(stale));
}

// Pause keeps the loaded preview's bars frozen, while Stop clears them
// immediately, like the elapsed readout and the status row's format.
#[test]
fn stop_clears_the_spectrum_while_pause_keeps_it() {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();
    let spectrum = recording_spectrum();
    player.spectrum = Some(spectrum);

    let _ = update(&mut player, Message::Pause);
    assert_eq!(player.spectrum, Some(spectrum));

    let _ = update(&mut player, Message::Stop);
    assert_eq!(player.spectrum, None);
}

// A duration reply stores exactly what the backend reported, and a later
// `None` reply clears the seek bar's scale instead of keeping the old length.
#[test]
fn playback_duration_reply_stores_the_loaded_previews_length() {
    let (mut player, _state) = test_player();
    let epoch = player.position_epoch;

    let _ = update(
        &mut player,
        Message::PlaybackDuration {
            epoch,
            duration: Some(Duration::from_secs(30)),
        },
    );
    assert_eq!(player.playback_duration, Some(Duration::from_secs(30)));

    let _ = update(
        &mut player,
        Message::PlaybackDuration {
            epoch,
            duration: None,
        },
    );
    assert_eq!(player.playback_duration, None);
}

// A duration reply is dropped when it was issued before a Stop, or when it
// lands while a transport transition is still reaching the backend: either
// way it could describe the previous player. A reply at the current epoch
// with no transition pending is stored, so the drops above come from the
// guards and not from the message being ignored entirely.
#[test]
fn playback_duration_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped() {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();
    let old_epoch = player.position_epoch;

    let _ = update(&mut player, Message::Stop);
    let _ = update(&mut player, Message::TransportSettled);
    let _ = update(
        &mut player,
        Message::PlaybackDuration {
            epoch: old_epoch,
            duration: Some(Duration::from_secs(5)),
        },
    );
    assert_eq!(player.playback_duration, None);

    // A pending transition drops even a reply at the current epoch.
    let current_epoch = player.position_epoch;
    player.playback_duration = Some(Duration::from_secs(30));
    let _ = update(&mut player, Message::Pause);
    let _ = update(
        &mut player,
        Message::PlaybackDuration {
            epoch: current_epoch,
            duration: Some(Duration::from_secs(5)),
        },
    );
    assert_eq!(player.playback_duration, Some(Duration::from_secs(30)));

    // Once the transition settles, the same reply is stored.
    let _ = update(&mut player, Message::TransportSettled);
    let _ = update(
        &mut player,
        Message::PlaybackDuration {
            epoch: current_epoch,
            duration: Some(Duration::from_secs(5)),
        },
    );
    assert_eq!(player.playback_duration, Some(Duration::from_secs(5)));
}

// Stop and a new selection clear the seek bar's scale and any drag
// immediately, along with the position and format.
#[test]
fn stop_and_track_selection_clear_the_playback_duration() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.playback_duration = Some(Duration::from_secs(30));
    player.seek_preview = Some(Duration::from_secs(5));

    let _ = update(&mut player, Message::Stop);

    assert_eq!(player.playback_duration, None);
    assert_eq!(player.seek_preview, None);

    player.playback_duration = Some(Duration::from_secs(30));
    player.seek_preview = Some(Duration::from_secs(5));
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });

    assert_eq!(player.playback_duration, None);
    assert_eq!(player.seek_preview, None);
}

// Dragging the seek bar stores the thumb value in seconds and sends nothing
// to the backend: only a release seeks.
#[test]
fn seek_preview_changed_tracks_the_drag_without_touching_the_backend() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);

    assert_no_task(update(&mut player, Message::SeekPreviewChanged(12.5)));

    assert_eq!(player.seek_preview, Some(Duration::from_millis(12_500)));
    assert_eq!(recording.calls(), vec![]);
}

// Releasing the seek bar sends exactly one absolute seek, records the dragged
// position optimistically, and bumps the epoch so replies already in flight
// cannot restore the old position.
#[test]
fn seek_preview_released_seeks_once_to_the_dragged_position() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
    player.playback_position = Some(Duration::from_secs(3));
    let epoch = player.position_epoch;
    let _ = update(&mut player, Message::SeekPreviewChanged(12.0));

    assert_message_schedules_no_work(&mut player, Message::SeekPreviewReleased);

    assert_eq!(
        recording.calls(),
        vec![AudioCall::SeekTo(Duration::from_secs(12))]
    );
    assert_eq!(player.playback_position, Some(Duration::from_secs(12)));
    assert_eq!(player.seek_preview, None);
    assert_eq!(player.position_epoch, epoch.wrapping_add(1));
}

// A release with no stored drag does nothing: no backend call, no position
// change, and no epoch bump.
#[test]
fn seek_preview_released_without_a_drag_does_nothing() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
    player.playback_position = Some(Duration::from_secs(3));
    let epoch = player.position_epoch;

    assert_message_schedules_no_work(&mut player, Message::SeekPreviewReleased);

    assert_eq!(recording.calls(), vec![]);
    assert_eq!(player.playback_position, Some(Duration::from_secs(3)));
    assert_eq!(player.position_epoch, epoch);
}

// A successful release bumps the epoch, so a position reply the poll issued
// before the seek is dropped instead of restoring the old thumb position.
#[test]
fn seek_preview_released_invalidates_an_in_flight_position_reply() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
    player.playback_position = Some(Duration::from_secs(3));
    let old_epoch = player.position_epoch;
    let _ = update(&mut player, Message::SeekPreviewChanged(12.0));
    let _ = update(&mut player, Message::SeekPreviewReleased);

    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch: old_epoch,
            position: Some(Duration::from_secs(2)),
        },
    );

    assert_eq!(player.playback_position, Some(Duration::from_secs(12)));
}

// A rejected absolute seek is logged and leaves the optimistic state alone:
// the drag is cleared, the position and epoch stay as they were, and the next
// position poll's reply is still accepted, so it can put the thumb back on
// the player's real position. The fake counts its rejection, proving the
// release reached the failing backend instead of skipping the forward.
#[test]
fn seek_preview_released_backend_failure_leaves_the_position_alone() {
    let audio = Arc::new(FailingAudio::failing_seek());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&audio) as Arc<dyn crate::audio::AudioOutput>);
    player.playback_position = Some(Duration::from_secs(3));
    let epoch = player.position_epoch;
    let _ = update(&mut player, Message::SeekPreviewChanged(12.0));

    assert_message_schedules_no_work(&mut player, Message::SeekPreviewReleased);

    assert_eq!(audio.attempts(FailingCommand::Seek), 1);
    assert_eq!(player.seek_preview, None);
    assert_eq!(player.playback_position, Some(Duration::from_secs(3)));
    assert_eq!(player.position_epoch, epoch);

    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch,
            position: Some(Duration::from_secs(4)),
        },
    );
    assert_eq!(player.playback_position, Some(Duration::from_secs(4)));
}

#[test]
fn position_tick_schedules_a_query() {
    let (mut player, _state) = test_player();
    assert_message_schedules_work(&mut player, Message::PositionTick);
}

// The subscription itself must emit the message its `update` arm expects: a
// stream that never ticks (or yields a different message) would leave the
// readout frozen even though `PositionTick` schedules the query correctly.
// Reading two ticks also pins that the timer keeps running after the first.
// The tick must ask the backend for both values: one query for the elapsed
// position and one for the status row's format. Yield every output the
// batched task emits and check both replies arrive.
#[test]
fn position_tick_queries_position_and_format() {
    use futures::StreamExt;

    let (mut player, _state) = test_player();
    let epoch = player.position_epoch;
    let mut stream = iced_runtime::task::into_stream(update(&mut player, Message::PositionTick))
        .expect("the position tick must schedule a task");
    let mut replies = Vec::new();
    while let Some(action) = futures::executor::block_on(await_or_timeout(
        "position tick",
        TASK_TIMEOUT,
        stream.next(),
    )) {
        if let iced_runtime::Action::Output(message) = action {
            replies.push(message);
        }
    }

    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackPosition { epoch: reply_epoch, .. } if *reply_epoch == epoch
        )),
        "the tick must query the position: {replies:?}"
    );
    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackFormat { epoch: reply_epoch, .. } if *reply_epoch == epoch
        )),
        "the tick must query the format: {replies:?}"
    );
}

// The tick must also ask the backend for the preview's own length, so the
// seek bar has a scale. Drive the batched task's stream to its end and check
// the duration reply arrives beside the position.
#[test]
fn position_tick_queries_position_and_duration() {
    use futures::StreamExt;

    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
    let epoch = player.position_epoch;
    let mut stream = iced_runtime::task::into_stream(update(&mut player, Message::PositionTick))
        .expect("the position tick must schedule a task");
    let mut replies = Vec::new();
    while let Some(action) = futures::executor::block_on(await_or_timeout(
        "position tick",
        TASK_TIMEOUT,
        stream.next(),
    )) {
        if let iced_runtime::Action::Output(message) = action {
            replies.push(message);
        }
    }

    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackPosition { epoch: reply_epoch, .. } if *reply_epoch == epoch
        )),
        "the tick must query the position: {replies:?}"
    );
    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackDuration { epoch: reply_epoch, duration: Some(duration) }
                if *reply_epoch == epoch && *duration == Duration::from_secs(30)
        )),
        "the tick must query the duration: {replies:?}"
    );
}

// The tick must also ask the backend whether the preview has reached its end,
// so the transport can advance off the existing one-second poll instead of a
// new timer. Drive the batched task's stream to its end and check the finished
// reply carries the tick's epoch and the backend's state.
#[test]
fn position_tick_queries_preview_finished() {
    use futures::StreamExt;

    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
    for finished in [true, false] {
        recording.set_finished(finished);
        let epoch = player.position_epoch;
        let mut stream =
            iced_runtime::task::into_stream(update(&mut player, Message::PositionTick))
                .expect("the position tick must schedule a task");
        let mut replies = Vec::new();
        while let Some(action) = futures::executor::block_on(await_or_timeout(
            "position tick",
            TASK_TIMEOUT,
            stream.next(),
        )) {
            if let iced_runtime::Action::Output(message) = action {
                replies.push(message);
            }
        }

        assert!(
            replies.iter().any(|message| matches!(
                message,
                Message::PlaybackFinished { epoch: reply_epoch, finished: reply_finished }
                    if *reply_epoch == epoch && *reply_finished == finished
            )),
            "the tick must query the finished state {finished}: {replies:?}"
        );
    }
}

// A preview that ends while playing must step forward exactly as Next would:
// from song-1, the showing list's next song is song-2. The scheduled selection
// resolves against the same list, and its play commits the new current track.
#[test]
fn preview_end_advances_to_the_next_song() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");

    let selected = drive_playback_finished(&mut player);
    assert!(matches!(
        selected,
        Message::TrackSelected { epoch: 0, index: 1 }
    ));
    let task = update(&mut player, selected);
    futures::executor::block_on(drive_task(task, "preview end play", |_| {}));

    let state = state.blocking_lock();
    assert_eq!(state.current_track.as_deref(), Some("song-2"));
    assert!(state.is_playing);
}

// The All Songs view draws from `all_songs`, so a natural end there must step
// the flat list, not the browsed `songs` buffer, and carry the flat list's
// epoch. `songs` holds only "song-4" while `all_songs` holds the three-song
// stepping album, so a preview end that read `songs` could not select index 1.
#[test]
fn preview_end_in_all_songs_steps_the_flat_list() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
    player.songs.items = second_album_songs();
    let _ = update(&mut player, Message::AllSongsLoaded(stepping_songs()));
    player.current_view = CurrentView::AllSongs;
    let all_songs_epoch = player.all_songs.epoch;

    let selected = drive_playback_finished(&mut player);
    assert!(matches!(
        selected,
        Message::TrackSelected { epoch, index } if epoch == all_songs_epoch && index == 1
    ));
    let task = update(&mut player, selected);
    futures::executor::block_on(drive_task(task, "preview end play", |_| {}));

    let state = state.blocking_lock();
    assert_eq!(state.current_track.as_deref(), Some("song-2"));
    assert!(state.is_playing);
}

// A natural preview end follows the advanced row exactly as Next does: from
// song-1 it scrolls the showing song list to song-2's row.
#[test]
fn a_finished_preview_scrolls_the_song_list_to_the_advanced_row() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) = player_playing_stepping_song(&recording, "song-1");
    player.current_view = CurrentView::Songs;
    let epoch = player.position_epoch;

    futures::executor::block_on(assert_task_scrolls_browse_list_to(
        update(
            &mut player,
            Message::PlaybackFinished {
                epoch,
                finished: true,
            },
        ),
        "preview end",
        views::BROWSE_LIST_PADDING + views::BROWSE_ROW_HEIGHT,
    ));
}

// Repeat wraps a natural end at the last song back to the first, just as the
// Next button does.
#[test]
fn preview_end_wraps_to_the_first_song_with_repeat_on() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-3");
    state.blocking_lock().repeat = true;

    let selected = drive_playback_finished(&mut player);
    assert!(matches!(
        selected,
        Message::TrackSelected { epoch: 0, index: 0 }
    ));
    let task = update(&mut player, selected);
    futures::executor::block_on(drive_task(task, "preview end play", |_| {}));

    assert_eq!(
        state.blocking_lock().current_track.as_deref(),
        Some("song-1")
    );
}

// Repeat wraps a one-song list back to itself: the pick equals the current
// song, which is still a step that replays it rather than the Repeat-off Stop
// path. The three-song fixture above never exercises this equal-id pick.
#[test]
fn preview_end_repeats_a_single_song_with_repeat_on() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
    player.songs.items.truncate(1);
    state.blocking_lock().repeat = true;

    let selected = drive_playback_finished(&mut player);
    assert!(matches!(
        selected,
        Message::TrackSelected { epoch: 0, index: 0 }
    ));
    let task = update(&mut player, selected);
    futures::executor::block_on(drive_task(task, "preview end play", |_| {}));

    let state = state.blocking_lock();
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(state.is_playing);
}

// With Repeat off, a natural end at the last song takes the Stop path: the
// transport becomes stopped, the song stays queued for Play, and the readout,
// status row's format, visualizer, and seek bar clear together.
#[test]
fn preview_end_at_the_last_song_stops_when_repeat_is_off() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-3");
    player.playback_position = Some(Duration::from_secs(95));
    player.playback_format = Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    });
    player.playback_duration = Some(Duration::from_secs(30));
    player.spectrum = Some(recording_spectrum());
    player.seek_preview = Some(Duration::from_secs(5));

    let settled = drive_playback_finished(&mut player);
    assert!(matches!(settled, Message::TransportSettled));

    assert_eq!(recording.calls(), vec![AudioCall::Stop]);
    let state = state.blocking_lock();
    assert!(!state.is_playing);
    assert_eq!(state.current_track.as_deref(), Some("song-3"));
    assert_eq!(player.playback_position, None);
    assert_eq!(player.playback_format, None);
    assert_eq!(player.playback_duration, None);
    assert_eq!(player.spectrum, None);
    assert_eq!(player.seek_preview, None);
}

// Shuffle picks a non-current song on a natural end, as it does for Next. A
// two-song list makes the pick deterministic: song-1 is current, so the only
// candidate is song-2.
#[test]
fn preview_end_with_shuffle_on_picks_the_other_song() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
    player.songs.items.truncate(2);
    state.blocking_lock().shuffle = true;

    let selected = drive_playback_finished(&mut player);
    assert!(matches!(
        selected,
        Message::TrackSelected { epoch: 0, index: 1 }
    ));
    let task = update(&mut player, selected);
    futures::executor::block_on(drive_task(task, "preview end play", |_| {}));

    assert_eq!(
        state.blocking_lock().current_track.as_deref(),
        Some("song-2")
    );
}

// Repeat alone repeats a one-song list, but Shuffle changes the forward step:
// `shuffled_track_id` can only pick the current song, so the advance filters it
// out and stops rather than replaying, whether Repeat is on or off. The
// two-song Shuffle test above never reaches this no-other-song path.
#[test]
fn preview_end_with_shuffle_on_a_single_song_stops() {
    for repeat in [false, true] {
        let recording = Arc::new(RecordingAudio::default());
        let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
        player.songs.items.truncate(1);
        {
            let mut state = state.blocking_lock();
            state.shuffle = true;
            state.repeat = repeat;
        }

        let settled = drive_playback_finished(&mut player);
        assert!(
            matches!(settled, Message::TransportSettled),
            "Shuffle with a single song and Repeat {repeat} must take the Stop path"
        );
        assert_eq!(recording.calls(), vec![AudioCall::Stop]);
        let state = state.blocking_lock();
        assert!(!state.is_playing);
        assert_eq!(state.current_track.as_deref(), Some("song-1"));
    }
}

// Pause and Stop never advance: the finished signal may still arrive from a
// poll issued before the transport changed, and a paused or stopped player has
// nothing to step past.
#[test]
fn preview_end_while_paused_or_stopped_does_not_advance() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");

    state.blocking_lock().pause();
    let epoch = player.position_epoch;
    assert_message_schedules_no_work(
        &mut player,
        Message::PlaybackFinished {
            epoch,
            finished: true,
        },
    );

    state.blocking_lock().stop();
    let epoch = player.position_epoch;
    assert_message_schedules_no_work(
        &mut player,
        Message::PlaybackFinished {
            epoch,
            finished: true,
        },
    );
}

// A finished reply in flight across a transport change must not advance: a
// stale epoch is dropped, and a reply that lands while a transition is still
// reaching the backend is dropped too, so it cannot step from the previous
// track's list. A not-finished reply is likewise a no-op.
#[test]
fn preview_end_with_a_stale_epoch_or_pending_transition_is_dropped() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) = player_playing_stepping_song(&recording, "song-1");

    let stale = player.position_epoch.wrapping_add(1);
    assert_message_schedules_no_work(
        &mut player,
        Message::PlaybackFinished {
            epoch: stale,
            finished: true,
        },
    );

    let epoch = player.position_epoch;
    player.pending_position_transitions = 1;
    assert_message_schedules_no_work(
        &mut player,
        Message::PlaybackFinished {
            epoch,
            finished: true,
        },
    );

    player.pending_position_transitions = 0;
    assert_message_schedules_no_work(
        &mut player,
        Message::PlaybackFinished {
            epoch,
            finished: false,
        },
    );
}

// The paused elapsed readout blinks on the existing position poll: each tick
// flips the phase while the transport is paused, and a tick after Play or Stop
// restores it. A stopped tick still holds the flag true, but the view draws the
// stopped readout transparent regardless of the phase except for the `--:--`
// placeholder. A loaded preview is the paused state Play can resume.
#[test]
fn position_tick_blinks_the_elapsed_readout_only_while_paused() {
    let (mut player, state) = player_with_audio(Arc::new(RecordingAudio::default()));
    futures::executor::block_on(player.apple_music_service.play_track(
        "song-1",
        Some(PREVIEW_URL),
        || true,
    ))
    .unwrap();
    state.blocking_lock().pause();
    assert_eq!(
        state.blocking_lock().transport_mode(),
        TransportMode::Paused
    );
    assert!(player.blink_visible);

    let _ = update(&mut player, Message::PositionTick);
    assert!(
        !player.blink_visible,
        "the first paused tick hides the readout"
    );
    let _ = update(&mut player, Message::PositionTick);
    assert!(
        player.blink_visible,
        "the second paused tick shows it again"
    );

    state.blocking_lock().play();
    let _ = update(&mut player, Message::PositionTick);
    assert!(
        player.blink_visible,
        "a playing tick always shows the readout"
    );

    state.blocking_lock().pause();
    let _ = update(&mut player, Message::PositionTick);
    assert!(!player.blink_visible, "a paused tick resumes the blink");
    state.blocking_lock().stop();
    let _ = update(&mut player, Message::PositionTick);
    assert!(
        player.blink_visible,
        "a stopped tick holds the blink phase visible, though the view clears a real readout"
    );
}

#[test]
fn spectrum_tick_queries_spectrum_and_carries_the_epoch() {
    use futures::StreamExt;

    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
    let epoch = player.position_epoch;
    let mut stream = iced_runtime::task::into_stream(update(&mut player, Message::SpectrumTick))
        .expect("the spectrum tick must schedule a task");
    let mut replies = Vec::new();
    while let Some(action) = futures::executor::block_on(await_or_timeout(
        "spectrum tick",
        TASK_TIMEOUT,
        stream.next(),
    )) {
        if let iced_runtime::Action::Output(message) = action {
            replies.push(message);
        }
    }

    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::Spectrum { epoch: reply_epoch, spectrum: Some(spectrum) }
                if *reply_epoch == epoch && *spectrum == recording_spectrum()
        )),
        "the tick must query the spectrum at its epoch: {replies:?}"
    );
}

#[test]
fn position_tick_stream_emits_position_ticks() {
    use futures::StreamExt;

    let mut ticks = position_tick_stream();
    for what in ["first position tick", "second position tick"] {
        let tick = futures::executor::block_on(await_or_timeout(what, TASK_TIMEOUT, ticks.next()));
        assert!(
            matches!(tick, Some(Message::PositionTick)),
            "{what} did not yield PositionTick: {tick:?}"
        );
    }
}

// A refused poll thread must not take the player down: `std::thread::spawn`
// panics when the OS refuses a thread, and iced builds the subscription's
// stream on its main thread while it updates the subscriptions, so that panic
// would unwind the event loop and kill the app on startup. The fallible spawn
// reports the refusal and ends the tick stream instead, so the app keeps
// running with an elapsed readout that no longer advances.
#[test]
fn a_refused_position_thread_ends_the_tick_stream_instead_of_panicking() {
    use futures::StreamExt;

    let mut ticks = position_tick_stream_with(|_work| {
        Err(std::io::Error::other("the OS refused another thread"))
    });

    let next = futures::executor::block_on(await_or_timeout(
        "the position stream after a refused thread",
        TASK_TIMEOUT,
        ticks.next(),
    ));
    assert!(
        next.is_none(),
        "a refused position thread must end the tick stream, not tick or panic"
    );
}

// The refusal is reported, not swallowed: the report names the cause and
// says what the user loses, so a readout that never advances is diagnosable
// instead of mysterious.
#[test]
fn a_refused_position_thread_reports_the_cause_and_the_loss() {
    let report = position_thread_failure_report(&std::io::Error::other("no threads left"));

    assert!(
        report.contains("no threads left"),
        "the report must name the cause, got: {report}"
    );
    assert!(
        report.contains("will not update"),
        "the report must say what the user loses, got: {report}"
    );
}

// The poll thread carries a name, like the audio and REST workers, so a panic
// in it is attributable. The name is spelled out here rather than read from
// the production constant, so renaming the constant fails this test.
#[test]
fn the_position_thread_is_named_nostalgia_position_poll() {
    let (sender, receiver) = std::sync::mpsc::channel();

    spawn_position_thread(Box::new(move || {
        let _ = sender.send(std::thread::current().name().map(str::to_string));
    }))
    .expect("the position thread starts");

    let name = receiver
        .recv_timeout(TASK_TIMEOUT)
        .expect("the position thread reports its name");
    assert_eq!(name.as_deref(), Some("nostalgia-position-poll"));
}

// The spectrum poll's stream emits the tick its `update` arm turns into a
// backend query, just as the position stream does: without this, the
// visualizer would never refresh even though the subscription is wired.
#[test]
fn spectrum_tick_stream_emits_spectrum_ticks() {
    use futures::StreamExt;

    let mut ticks = spectrum_tick_stream();
    for what in ["first spectrum tick", "second spectrum tick"] {
        let tick = futures::executor::block_on(await_or_timeout(what, TASK_TIMEOUT, ticks.next()));
        assert!(
            matches!(tick, Some(Message::SpectrumTick)),
            "{what} did not yield SpectrumTick: {tick:?}"
        );
    }
}

// A refused spectrum poll thread must not take the player down, for the same
// reason as the other timers: `std::thread::spawn` panics when the OS refuses
// a thread, and iced builds the stream on its main thread. The fallible spawn
// reports the refusal and ends the tick stream, so the app keeps running with
// a visualizer that stops updating.
#[test]
fn a_refused_spectrum_thread_ends_the_tick_stream_instead_of_panicking() {
    use futures::StreamExt;

    let mut ticks = spectrum_tick_stream_with(|_work| {
        Err(std::io::Error::other("the OS refused another thread"))
    });

    let next = futures::executor::block_on(await_or_timeout(
        "the spectrum stream after a refused thread",
        TASK_TIMEOUT,
        ticks.next(),
    ));
    assert!(
        next.is_none(),
        "a refused spectrum thread must end the tick stream, not tick or panic"
    );
}

// The spectrum refusal is reported, not swallowed: the report names the cause
// and says what the user loses, so a visualizer that never moves is
// diagnosable instead of mysterious.
#[test]
fn a_refused_spectrum_thread_reports_the_cause_and_the_loss() {
    let report = spectrum_thread_failure_report(&std::io::Error::other("no threads left"));

    assert!(
        report.contains("no threads left"),
        "the report must name the cause, got: {report}"
    );
    assert!(
        report.contains("will not update"),
        "the report must say what the user loses, got: {report}"
    );
}

// The spectrum poll thread carries a name, like the audio, REST, and other
// timer workers, so a panic in it is attributable. The name is spelled out
// here rather than read from the production constant, so renaming it fails.
#[test]
fn the_spectrum_thread_is_named_nostalgia_spectrum_poll() {
    let (sender, receiver) = std::sync::mpsc::channel();

    spawn_spectrum_thread(Box::new(move || {
        let _ = sender.send(std::thread::current().name().map(str::to_string));
    }))
    .expect("the spectrum thread starts");

    let name = receiver
        .recv_timeout(TASK_TIMEOUT)
        .expect("the spectrum thread reports its name");
    assert_eq!(name.as_deref(), Some("nostalgia-spectrum-poll"));
}

// The Now Playing bar renders the played track's artist before its title
// (`<artist> - <title>`). The artist comes from the same `known_tracks`
// entry as the title and time: `TrackSelected` must record the played song's
// artist, and `WinampPlayer::now_playing_artist` must resolve it — and keep
// resolving it after a later browse to a different album replaces `songs`.
// The view-level `now_playing_artist` tests build their maps by hand, so a
// regression that dropped `artist` from the recorded `KnownTrack` (or
// resolved the bar against the currently-browsed `songs` list) would clear
// every existing test and only fail here.
#[test]
fn now_playing_artist_shows_the_played_tracks_artist_after_browsing_to_another_album() {
    let (player, current_track) = play_song_1_then_browse_to_album_2();

    assert_eq!(player.now_playing_artist(None), "");
    assert_eq!(
        player.now_playing_artist(current_track.as_deref()),
        "The Sample Band"
    );
    assert_eq!(player.now_playing_artist(Some("no-such-song")), "");
}

// The playlist number comes from the same `known_tracks` entry as the artist,
// so it must survive the browse that replaces the visible song list, and be
// absent for an unknown id or no current track.
#[test]
fn now_playing_number_survives_browsing_to_another_album() {
    let (player, current_track) = play_song_1_then_browse_to_album_2();

    assert_eq!(player.now_playing_number(None), "");
    assert_eq!(player.now_playing_number(current_track.as_deref()), "1. ");
    assert_eq!(player.now_playing_number(Some("no-such-song")), "");
}

// `fetch_into` schedules the fetch as an iced `Task`; the arm itself only
// builds it, so the real behavior lives in the returned task. Drive that
// task to completion and assert the mapped `*Loaded` message, as the
// load-path arms would produce it.
#[tokio::test]
async fn fetch_into_schedules_fetch_and_maps_result_to_loaded_message() {
    let (player, _state) = test_player();

    let generation = test_generation();
    let task = fetch_into(
        &player.apple_music_service,
        "loading favorite artists".to_string(),
        generation,
        std::time::Duration::from_secs(1),
        |service| async move { service.get_favorite_artists().await },
        Message::ArtistsLoaded,
        Message::ArtistsLoadFailed,
    );
    drive_task(task, "fetch", |message| {
        assert!(matches!(
            browse_reply(message),
            Message::ArtistsLoaded(artists) if !artists.is_empty()
        ));
    })
    .await;
}

// The success path above proves `fetch_into` delivers a `*Loaded` message
// for a successful fetch; this pins the error path — a fetch that returns
// `Err` must yield the matching `*LoadFailed` message carrying the formatted
// report (which names the failed query and includes the backend's cause),
// not a `*Loaded` message with an empty list that the panel would misread as
// an empty library. `fetch_failure_report` is tested in `loading`'s test
// module, but no test drives a failing fetch *through* `fetch_into` — the real
// service always succeeds, so the error path is reachable only by injecting a
// failing fetch closure, which is exactly what this does. A regression that
// swallowed the error would leave the browse panel claiming the library is
// empty, and only this test would catch it.
#[tokio::test]
async fn fetch_into_maps_a_failed_fetch_to_a_load_failed_message() {
    let (player, _state) = test_player();

    let generation = test_generation();
    let task = fetch_into(
        &player.apple_music_service,
        "loading albums for artist \"artist-1\"".to_string(),
        generation,
        std::time::Duration::from_secs(1),
        |_service| async { Err::<Vec<Album>, String>("boom".to_string()) },
        Message::AlbumsLoaded,
        Message::AlbumsLoadFailed,
    );
    drive_task(task, "failed fetch", |message| {
        assert!(matches!(
            browse_reply(message),
            Message::AlbumsLoadFailed(report)
                if report
                    == "music-library fetch failed (loading albums for artist \"artist-1\"): boom"
        ));
    })
    .await;
}

// A fetch that never answers is the failure `with_timeout` exists to bound:
// without it the browse panel stays on "Loading…" forever, because `loading`
// only clears when the reply lands. The real service always answers, so the
// hang is reachable only by injecting a pending fetch closure; the short
// timeout keeps the test fast. A timeout is reported through the `*LoadFailed`
// path, exactly like a returned error, so the panel names the failed fetch
// (and, for the top-level artists fetch, offers the Retry button) instead of
// misreading the hang as an empty library.
#[tokio::test]
async fn fetch_into_maps_a_hanging_fetch_to_a_failed_message() {
    let (player, _state) = test_player();

    let generation = test_generation();
    let task = fetch_into(
        &player.apple_music_service,
        "loading favorite artists".to_string(),
        generation,
        std::time::Duration::from_millis(50),
        |_service| std::future::pending::<Result<Vec<Artist>, String>>(),
        Message::ArtistsLoaded,
        Message::ArtistsLoadFailed,
    );
    drive_task(task, "hanging fetch", |message| {
        assert!(matches!(
            browse_reply(message),
            Message::ArtistsLoadFailed(report)
                if report
                    == "music-library fetch failed (loading favorite artists); timed out after 50ms"
        ));
    })
    .await;
}

// The play path has the same unbounded-wait failure as the browse path: a
// backend that never answers leaves the play's task pending forever, so its
// `TrackPlayed` completion never fires and `update`'s title-index prune never
// runs. `play_into` bounds the play with its `timeout`; the real service
// always answers, so the hang is reachable only by injecting a pending play
// closure. The short timeout keeps the test fast, and the completion must
// still arrive.
#[tokio::test]
async fn play_into_maps_a_hanging_play_to_a_completed_message() {
    let (player, _state) = test_player();
    let generation = test_generation();
    let expected = generation.issued();

    let task = play_into(
        &player.apple_music_service,
        "song-1".to_string(),
        generation,
        std::time::Duration::from_millis(50),
        |_service, _id, _generation| std::future::pending::<Result<(), String>>(),
        |_err| {},
    );
    assert_track_played(task, expected, false).await;
}

// The play path's error is handed to `play_into`'s injected reporter; the real
// service rejects only a blank id, which the `update`-level test covers, so
// this injects a failing play closure and pins that the reporter sees the
// error while the completion still arrives. A swallowed error would leave a
// rejected play looking like it worked.
#[tokio::test]
async fn play_into_reports_a_failed_play_and_still_completes() {
    let (player, _state) = test_player();
    let generation = test_generation();
    let expected = generation.issued();
    let (reported, received) = std::sync::mpsc::channel();

    let task = play_into(
        &player.apple_music_service,
        "song-1".to_string(),
        generation,
        std::time::Duration::from_secs(1),
        |_service, _id, _generation| async { Err::<(), String>("boom".to_string()) },
        move |err| {
            let _ = reported.send(err.clone());
        },
    );
    assert_track_played(task, expected, false).await;

    assert_eq!(received.try_recv().ok().as_deref(), Some("boom"));
}

#[test]
fn new_player_starts_at_artists_with_nothing_selected() {
    let (player, _state) = test_player();

    assert_view(&player, CurrentView::Artists);
    assert_browse_lists_empty(&player);
    // The artists fetch `boot` schedules is in flight, so the panel shows
    // "Loading…" rather than claiming an empty library (see
    // `views::browse::rows::browse_placeholder`); the lists below Artists are not
    // loading, including the two flat lists.
    assert!(player.artists.loading);
    assert!(!player.albums.loading);
    assert!(!player.songs.loading);
    assert!(!player.all_songs.loading);
    assert!(!player.all_albums.loading);
    assert!(!player.playlists.loading);
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

// `view` is the per-frame assembly: it locks the shared state, resolves
// the now-playing label, matches the current browse view onto its list
// buffer, and stacks the title bar, the Now Playing bar, the seek strip, the
// transport row, the equalizer panel, and the browse controls (Back, Retry,
// Open, Top, Sort, the flat-list buttons, and the search box) above the browse
// list. No other
// test reaches this assembly — the shade test stops at `view`'s title-bar
// early return, the `views` tests stop at the individual builders, and
// the `update` tests stop before the view layer — so a regression
// that made this assembly panic (a bad slider range, a `view_*` builder
// failing over the input the app produces) would take the window down on
// every refresh with no test catching it. iced `Element`s expose no tree
// introspection, so the observable contract — as with the `views`
// builder tests — is that `view` builds its widget tree without panicking
// over the space the app actually produces: every browse view (including
// the pre-load empty buffers), both play states, both repeat states, both
// shuffle states, the
// volume endpoints the update arm can store, and each now-playing
// resolution. `view` uses `blocking_lock`, which panics inside an async
// runtime, so this stays a plain test.
#[test]
fn view_constructs_over_the_apps_full_input_space() {
    let (mut player, state) = test_player();

    // The pre-load shape: nothing fetched, no current track. This is what
    // the window shows before the first `LoadArtists` fetch lands, in
    // every browse view.
    construct_view_in_every_browse_view(&mut player);

    // The loaded shape: every browse buffer populated, rendered for every
    // now-playing resolution a session can reach.
    seed_browse_lists(&mut player);

    let now_playing_options = [None, Some("song-1"), Some("no-such-song")];
    let play_states = [false, true];
    let repeats = [false, true];
    let shuffles = [false, true];
    let volumes = [0.0, 0.5, 1.0];

    for current_view in BROWSE_VIEWS {
        player.current_view = current_view;
        for current_track in now_playing_options {
            for is_playing in play_states {
                for repeat in repeats {
                    for shuffle in shuffles {
                        for volume in volumes {
                            {
                                let mut state = state.blocking_lock();
                                state.current_track = current_track.map(str::to_string);
                                state.is_playing = is_playing;
                                state.repeat = repeat;
                                state.shuffle = shuffle;
                                state.set_volume(volume);
                            }
                            let _screen = view(&player);
                        }
                    }
                }
            }
        }
    }

    // The fetch-failure shape: a list that failed to load is empty and
    // carries an error report, and `view` must render the report rather than
    // an empty-list label. Clearing the buffers matters: `scrollable_list`
    // only shows the placeholder when there are no rows, so with the loaded
    // rows still present the error branch would never render.
    player.artists.items.clear();
    player.albums.items.clear();
    player.songs.items.clear();
    player.all_songs.items.clear();
    player.all_albums.items.clear();
    player.playlists.items.clear();
    player.artists.error = Some("music-library fetch failed: boom".to_string());
    player.albums.error = Some("music-library fetch failed: boom".to_string());
    player.songs.error = Some("music-library fetch failed: boom".to_string());
    player.all_songs.error = Some("music-library fetch failed: boom".to_string());
    player.all_albums.error = Some("music-library fetch failed: boom".to_string());
    player.playlists.error = Some("music-library fetch failed: boom".to_string());
    construct_view_in_every_browse_view(&mut player);
}

#[test]
fn view_constructs_when_the_window_is_shaded() {
    let (mut player, _state) = test_player();
    player.shaded = true;

    // The rolled-up frame builds only the title bar, so it must not panic
    // over either the pre-load empty buffers or the loaded browse lists.
    construct_view_in_every_browse_view(&mut player);

    seed_browse_lists(&mut player);
    construct_view_in_every_browse_view(&mut player);
}

#[test]
fn title_bar_constructs_for_both_always_on_top_states() {
    // iced `Element`s expose no tree introspection, so the observable
    // contract is that the title bar builds for both toggle states — the
    // sunken A slot and the raised one — without panicking.
    let _ = views::view_title_bar(false);
    let _ = views::view_title_bar(true);
}

// `TITLE_BAR_HEIGHT` is the title bar band alone. The rolled-up window is
// that band plus the compact transport strip: `views::SHADED_HEIGHT` owns the
// sum, and both the shaded root view and `resize_to_shade` read it. The shade
// tests above never read the value — they pass a literal 24.0 only as a stale
// measurement argument — so a zero or oversized band could leave the
// rolled-up window blank or push the strip out of view while every test
// stayed green. Pin the user-visible band height.
#[test]
fn title_bar_height_pins_the_shade_strip() {
    assert_eq!(views::TITLE_BAR_HEIGHT, 24.0);
}

// `resize_to_shade` sends `shade_target_size` to the window; pin the target
// itself, because the `Task` it returns is opaque and the shade tests above
// only see that some work was scheduled. A title-bar-only target would still
// schedule work while cutting the compact strip off below the window.
#[test]
fn shade_target_size_is_the_title_bar_plus_the_strip() {
    let target = handlers::window::shade_target_size(800.0);
    assert_eq!(target, iced::Size::new(800.0, views::SHADED_HEIGHT));
    assert_ne!(target.height, views::TITLE_BAR_HEIGHT);
}

/// Lays the root `view` out with the headless tiny-skia renderer and returns
/// every text fragment's absolute bounds plus the resolved root size.
fn captured_root_view_text_bounds(player: &WinampPlayer) -> layout_capture::TextCapture {
    let renderer = layout_capture::headless_renderer();
    let mut element = view(player);
    layout_capture::capture_text(&mut element, &renderer)
}

/// Lays the root `view` out and returns the bounds of the container tagged
/// `views::MINI_VOLUME_ID`, so a test can find the library footer's volume
/// slider in the real laid-out tree.
fn captured_root_view_volume_slider(player: &WinampPlayer) -> Option<iced::Rectangle> {
    use iced::advanced::{
        Layout,
        widget::{Id, Tree, operation::Operation},
    };

    #[derive(Default)]
    struct CapturedSlider(Option<iced::Rectangle>);
    impl Operation for CapturedSlider {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn container(&mut self, id: Option<&Id>, bounds: iced::Rectangle) {
            if id == Some(&Id::new(views::MINI_VOLUME_ID)) {
                self.0 = Some(bounds);
            }
        }
    }

    let renderer = layout_capture::headless_renderer();
    let mut element = view(player);
    let mut tree = Tree::new(element.as_widget());
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &layout_capture::limits());
    let mut captured = CapturedSlider::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut captured);
    captured.0
}

// In shade mode the root view is the title bar over the compact strip, so the
// whole laid-out tree must resolve to `SHADED_HEIGHT` — the same height
// `resize_to_shade` gives the window. A title-bar-only tree would leave the
// controls cut off below the resized window and fail here.
#[test]
fn shaded_root_view_is_the_title_bar_plus_the_strip() {
    let (mut player, _state) = test_player();
    player.shaded = true;

    let capture = captured_root_view_text_bounds(&player);
    assert_eq!(capture.size().height, views::SHADED_HEIGHT);
}

// The strip is shade-mode-only. Both modes now draw a Previous glyph — the
// full transport row has one too — so pin where they land: in normal mode the
// first belongs to the full transport row and a second to the library footer,
// and in shade mode exactly one sits inside the strip between the title bar
// and `SHADED_HEIGHT`.
#[test]
fn shade_strip_draws_only_while_shaded() {
    let (mut player, _state) = test_player();

    let normal = captured_root_view_text_bounds(&player);
    let normal_previous: Vec<iced::Rectangle> = normal
        .fragments()
        .iter()
        .filter(|(text, _)| text == "◀◀")
        .map(|(_, bounds)| *bounds)
        .collect();
    assert_eq!(
        normal_previous.len(),
        2,
        "normal mode must draw the full transport row and the library footer: {:?}",
        normal.fragments()
    );
    assert!(
        normal_previous[0].y >= views::SHADED_HEIGHT,
        "normal mode's first Previous glyph belongs to the full transport row, not the strip: {:?}",
        normal.fragments()
    );
    assert!(
        normal_previous[1].y > normal_previous[0].y,
        "the library footer's Previous glyph must sit below the full transport row: {:?}",
        normal.fragments()
    );

    player.shaded = true;
    let shaded = captured_root_view_text_bounds(&player);
    let shaded_previous: Vec<iced::Rectangle> = shaded
        .fragments()
        .iter()
        .filter(|(text, _)| text == "◀◀")
        .map(|(_, bounds)| *bounds)
        .collect();
    assert_eq!(
        shaded_previous.len(),
        1,
        "shade mode must draw exactly the shade strip: {:?}",
        shaded.fragments()
    );
    assert!(
        shaded_previous[0].y >= views::TITLE_BAR_HEIGHT
            && shaded_previous[0].y < views::SHADED_HEIGHT,
        "shade mode's Previous glyph must sit inside the strip: {:?}",
        shaded.fragments()
    );
}

// In normal mode the library footer sits beneath the browse list, with the
// same four glyphs, time readout, and right-edge volume slider as the shade
// strip. A builder-only test cannot catch the root dropping the footer, so
// lay out the whole window and measure it.
#[test]
fn normal_root_view_draws_the_library_strip_beneath_the_browse_list() {
    let (player, _state) = test_player();
    let capture = captured_root_view_text_bounds(&player);

    let previous: Vec<iced::Rectangle> = capture
        .fragments()
        .iter()
        .filter(|(text, _)| text == "◀◀")
        .map(|(_, bounds)| *bounds)
        .collect();
    assert_eq!(
        previous.len(),
        2,
        "normal mode must draw the full transport row and the library footer: {:?}",
        capture.fragments()
    );
    let footer = previous[1];
    let top = capture.find("Top");
    assert!(
        footer.y > top.y,
        "the library footer must sit below the browse list's Top button: footer {footer:?}, Top {top:?}"
    );
    assert!(
        footer.y + footer.height <= capture.size().height,
        "the library footer must stay inside the window: footer {footer:?} in {:?}",
        capture.size()
    );

    let elapsed = player.now_playing_elapsed(None);
    assert!(
        capture
            .fragments()
            .iter()
            .any(|(text, bounds)| text == elapsed.as_ref() && (bounds.y - footer.y).abs() < 5.0),
        "missing the footer's {elapsed:?} time readout: {:?}",
        capture.fragments()
    );
    assert!(
        captured_root_view_volume_slider(&player).is_some(),
        "the library footer's volume slider must be laid out: {:?}",
        capture.fragments()
    );
}

// The README and the Browse row order invariant both say Sort renders beside
// Top. This lays out the whole root `view` — the same column whose separate
// `.push()` calls once stacked the buttons — and reads the text bounds it
// would draw: Top and Sort must share a row, with Sort at or after Top's
// right edge. A unit test of the nav row alone would not catch the root
// pushing the two buttons itself.
#[test]
fn browse_nav_buttons_place_sort_beside_top() {
    let (player, _state) = test_player();
    let capture = captured_root_view_text_bounds(&player);
    let top = capture.find("Top");
    let sort = capture.find("Sort");

    assert_eq!(
        top.y,
        sort.y,
        "Sort must share Top's row: {:?}",
        capture.fragments()
    );
    assert!(
        sort.x >= top.x + top.width,
        "Sort must render to the right of Top: {:?}",
        capture.fragments()
    );
}

// Open is a browse-list control: it sits above the list it opens from, so the
// root must push it above the first row on every level.
#[test]
fn browse_open_button_sits_above_the_browse_list() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![sample_artist()];
    player.artists.loading = false;

    let capture = captured_root_view_text_bounds(&player);
    let open = capture.find("Open");
    let row = capture.find("The Sample Band");
    assert!(
        open.y < row.y,
        "Open must sit above the browse list's first row: {open:?}, row {row:?}"
    );
}

// The scroll-to-playing-row operation offsets by `BROWSE_LIST_PADDING +
// index * BROWSE_ROW_HEIGHT`, so that constant must equal the pitch the browse
// list actually lays its rows out at. A stale constant would scroll a later
// row out of position. Lay the root view out with the stepping album showing
// — its first title far too long for one line if it wrapped — and measure the
// distance between the first two row prefixes. The same long title must stay
// inside its own column instead of drawing under the right-aligned length.
#[test]
fn browse_row_height_matches_the_laid_out_row_pitch() {
    let (mut player, _state) = player_stepping_from(None);
    player.current_view = CurrentView::Songs;
    player.songs.items[0].title = "A very long title ".repeat(20);
    let long_title = player.songs.items[0].display_title().into_owned();

    let capture = captured_root_view_text_bounds(&player);
    let first = capture.find("1. ");
    let second = capture.find("2. ");
    let title = capture.find(&long_title);
    let label = capture.find("3:30");
    let pitch = second.y - first.y;

    assert!(
        (pitch - views::BROWSE_ROW_HEIGHT).abs() < 0.01,
        "row pitch {pitch} must equal BROWSE_ROW_HEIGHT {}; rows at {first:?} and {second:?}",
        views::BROWSE_ROW_HEIGHT
    );
    assert!(
        title.x + title.width <= label.x,
        "the title must be clipped to its column, not drawn under the label: {title:?} ends after {label:?} begins"
    );
}

// `Wrapping::None` only suppresses soft wrapping: cosmic-text still splits a
// hard line break into another `BufferLine`, which would make the first row
// taller than `BROWSE_ROW_HEIGHT`. The list flattens line breaks to spaces, so
// a title carrying one keeps the same row pitch as any other.
#[test]
fn a_hard_line_break_in_a_title_does_not_change_the_row_pitch() {
    let (mut player, _state) = player_stepping_from(None);
    player.current_view = CurrentView::Songs;
    player.songs.items[0].title = "One\nTwo".to_string();

    let capture = captured_root_view_text_bounds(&player);
    let first = capture.find("1. ");
    let second = capture.find("2. ");
    let pitch = second.y - first.y;

    assert!(
        (pitch - views::BROWSE_ROW_HEIGHT).abs() < 0.01,
        "row pitch {pitch} must equal BROWSE_ROW_HEIGHT {} despite the title's line break; rows at {first:?} and {second:?}",
        views::BROWSE_ROW_HEIGHT
    );
}

// The transport row is rebuilt every frame and the Play/Pause glyph changes
// with playback. A text-sized Play button once widened to Pause and moved
// every control after it; this lays out the whole root `view` in both states
// and asserts the Repeat and Shuffle toggles keep the same absolute x, which
// only holds while the glyph row's faces stay fixed.
#[test]
fn transport_row_does_not_reflow_when_the_play_glyph_changes() {
    let (player, state) = test_player();

    state.blocking_lock().is_playing = false;
    let stopped = captured_root_view_text_bounds(&player);
    let stopped_repeat = stopped.find("Repeat: Off");
    let stopped_shuffle = stopped.find("Shuffle: Off");

    state.blocking_lock().is_playing = true;
    let playing = captured_root_view_text_bounds(&player);
    let playing_repeat = playing.find("Repeat: Off");
    let playing_shuffle = playing.find("Shuffle: Off");

    assert_eq!(
        stopped_repeat.x,
        playing_repeat.x,
        "the Repeat button must not move when Play becomes Pause: {:?}",
        playing.fragments()
    );
    assert_eq!(
        stopped_shuffle.x,
        playing_shuffle.x,
        "the Shuffle button must not move when Play becomes Pause: {:?}",
        playing.fragments()
    );
}

// The main transport row is built in `src/ui/views/transport.rs`, but only the
// root `view` shows whether it reaches the window. Lay out the whole root and
// find the Previous and Next glyphs: they must share one row with Previous
// left of Next, matching the mockup's compact button order.
#[test]
fn root_view_draws_the_transport_glyph_row() {
    let (player, _state) = test_player();
    let capture = captured_root_view_text_bounds(&player);
    let previous = capture.find("◀◀");
    let next = capture.find("▶▶");

    assert_eq!(
        previous.y,
        next.y,
        "Previous and Next must share the transport row: {:?}",
        capture.fragments()
    );
    assert!(
        next.x >= previous.x + previous.width,
        "Next must render to the right of Previous: {:?}",
        capture.fragments()
    );
}

// The root `view` resolves the LCD's transport mode from the shared state, so
// the glyph the user sees follows Play, Pause, and Stop. The unit tests in
// `src/ui/views/tests.rs` pass a mode straight to the row and cannot catch the
// root handing it a stale or hard-coded mode. Lay out the whole root `view`
// after each state change and read the glyphs it would draw.
#[test]
fn root_view_draws_the_glyph_for_the_shared_transport_mode() {
    let (player, state) = test_player();
    // The LCD's glyph is drawn before the transport row's, so the first
    // fragment carrying one of these three glyphs is the LCD's.
    let shown_glyphs = |player: &WinampPlayer| {
        let capture = captured_root_view_text_bounds(player);
        capture
            .fragments()
            .iter()
            .find(|(text, _)| ["▶", "II", "■"].contains(&text.as_str()))
            .map(|(text, _)| vec![text.clone()])
            .unwrap_or_default()
    };

    state.blocking_lock().stop();
    assert_eq!(
        shown_glyphs(&player),
        vec!["■"],
        "a stopped transport must draw the stopped glyph"
    );

    state.blocking_lock().play();
    assert_eq!(
        shown_glyphs(&player),
        vec!["▶"],
        "a playing transport must draw the playing glyph"
    );

    state.blocking_lock().pause();
    assert_eq!(
        shown_glyphs(&player),
        vec!["II"],
        "a paused transport must draw the paused glyph"
    );

    state.blocking_lock().stop();
    assert_eq!(
        shown_glyphs(&player),
        vec!["■"],
        "Stop after Pause must draw the stopped glyph again"
    );
}

// Normal mode introduces the equalizer and browse sections with the mockups'
// title bands. A per-band widget test can pass while the root never pushes the
// bands or pushes them in the wrong place, so lay out the whole root with an
// album's songs open and read the vertical positions. The Songs view draws
// Back above its list, so the library band must come before that button too,
// not after it as a push placed with the Top/Sort group would.
#[test]
fn root_view_draws_the_equalizer_and_library_title_bands_in_order() {
    let (mut player, _state) = player_stepping_from(None);
    player.current_view = CurrentView::Songs;

    let capture = captured_root_view_text_bounds(&player);
    let equalizer = capture.find("NOSTALGIA EQUALIZER");
    let library = capture.find("NOSTALGIA LIBRARY");
    let back = capture.find("Back");
    let first_row = capture.find("1. ");

    assert!(
        equalizer.y < library.y,
        "the equalizer band must sit above the library band: {equalizer:?} vs {library:?}"
    );
    assert!(
        library.y < back.y,
        "the library band must sit above Back: {library:?} vs {back:?}"
    );
    assert!(
        back.y < first_row.y,
        "Back must sit above the first browse row: {back:?} vs {first_row:?}"
    );
}

// The section bands are part of the normal column, not the rolled-up frame:
// shade mode still draws the app bar but neither section label.
#[test]
fn shaded_root_view_draws_no_section_title_bands() {
    let (mut player, _state) = test_player();
    player.shaded = true;

    let capture = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = capture.texts().collect();
    assert!(
        texts.contains(&"NOSTALGIA"),
        "the shaded app bar must still draw the app name: {texts:?}"
    );
    assert!(
        !texts.contains(&"NOSTALGIA EQUALIZER"),
        "shade mode must not draw the equalizer band: {texts:?}"
    );
    assert!(
        !texts.contains(&"NOSTALGIA LIBRARY"),
        "shade mode must not draw the library band: {texts:?}"
    );
}

// The root `view` decides which chrome the current level offers: Back below
// the top of the hierarchy, Retry only for a failed top-level artists fetch,
// and the All Songs/All Albums/Playlists buttons only on Artists. The
// `can_go_back` and `can_retry_artists` unit tests check the predicates, and
// the per-level view tests build single panels, so a root mistake — the wrong
// predicate, a missing `matches!`, or a stale error checked on every level —
// would pass them. Lay out the whole root for each level and read what it
// draws.
#[test]
fn root_view_offers_back_retry_and_flat_list_buttons_only_where_they_belong() {
    let (mut player, _state) = test_player();
    let shows = |player: &WinampPlayer, label: &str| {
        captured_root_view_text_bounds(player)
            .texts()
            .any(|text| text == label)
    };
    let flat_list_buttons = ["All Songs", "All Albums", "Playlists"];

    // Artists with a healthy fetch: the three flat-list entry points, but no
    // Back (nothing above it) and no Retry (nothing failed).
    for label in flat_list_buttons {
        assert!(
            shows(&player, label),
            "Artists must draw the {label:?} button"
        );
    }
    assert!(!shows(&player, "Back"), "Artists must not draw Back");
    assert!(!shows(&player, "Retry"), "Artists must not draw Retry");

    // A failed artists fetch adds Retry without adding Back, and keeps the
    // flat-list entry points.
    player.artists.error = Some("music-library fetch failed: boom".to_string());
    assert!(
        shows(&player, "Retry"),
        "a failed artists fetch must draw Retry"
    );
    assert!(!shows(&player, "Back"), "Retry must not add Back");
    for label in flat_list_buttons {
        assert!(
            shows(&player, label),
            "a failed artists fetch must keep the {label:?} button"
        );
    }

    // Every level below Artists offers Back and drops the flat-list entry
    // points. The stale artists error stays set to prove Retry is gated on the
    // level as well as the error, not on the error alone.
    for view in [
        CurrentView::Albums,
        CurrentView::Songs,
        CurrentView::AllSongs,
        CurrentView::AllAlbums,
        CurrentView::Playlists,
    ] {
        let level = format!("{view:?}");
        player.current_view = view;
        assert!(shows(&player, "Back"), "{level} must draw Back");
        assert!(!shows(&player, "Retry"), "{level} must not draw Retry");
        for label in flat_list_buttons {
            assert!(
                !shows(&player, label),
                "{level} must not draw the {label:?} button"
            );
        }
    }
}

// The root `view` chooses the browse list to draw from `current_view`, passing
// each level its own buffer, cached row text, and current-track id. The
// per-level view tests feed each builder directly, and every root-view test
// above lays out the default Artists level, so an arm wired to the wrong
// buffer — or dropping the song caches or the playing marker — would still
// pass them. Give every buffer distinct row text, labels, and total time, then
// lay out the whole root once per level and read what it actually drew.
#[test]
fn root_view_draws_the_current_browse_levels_own_rows() {
    let (mut player, state) = test_player();

    let mut artist = sample_artist();
    artist.name = "Artist Level Row".to_string();
    let mut album = sample_album();
    album.title = "Album Level Row".to_string();
    let mut all_album = sample_album();
    all_album.title = "All Albums Level Row".to_string();
    let mut song = sample_song();
    song.title = "Songs Item Title".to_string();
    song.duration_ms = 210_000;
    let mut all_song = sample_song();
    all_song.title = "All Songs Item Title".to_string();
    all_song.duration_ms = 95_000;
    let mut playlist = sample_playlist();
    playlist.name = "Playlist Level Row".to_string();

    player.artists.items = vec![artist];
    player.albums.items = vec![album];
    player.songs.items = vec![song.clone()];
    player.songs.titles = vec!["Songs Cached Row".to_string()];
    player.songs.labels = vec!["9:01".to_string()];
    player.all_songs.items = vec![all_song];
    player.all_songs.titles = vec!["All Songs Cached Row".to_string()];
    player.all_songs.labels = vec!["9:02".to_string()];
    player.all_albums.items = vec![all_album];
    player.playlists.items = vec![playlist];
    // Both song lists hold the playing track, so each arm must pass the
    // current id through to mark its own row.
    state.blocking_lock().current_track = Some("song-1".to_string());

    // The expected primary row text, the footer and cached label unique to a
    // song list, and whether this level marks the playing row.
    let cases = [
        (
            CurrentView::Artists,
            "Artist Level Row",
            "1 artist",
            "",
            false,
        ),
        (
            CurrentView::Albums,
            "The Sample Band - Album Level Row",
            "1 album",
            "",
            false,
        ),
        (
            CurrentView::Songs,
            "Songs Cached Row",
            "1 song - 3:30",
            "9:01",
            true,
        ),
        (
            CurrentView::AllSongs,
            "All Songs Cached Row",
            "1 song - 1:35",
            "9:02",
            true,
        ),
        (
            CurrentView::AllAlbums,
            "The Sample Band - All Albums Level Row",
            "1 album",
            "",
            false,
        ),
        (
            CurrentView::Playlists,
            "Playlist Level Row",
            "1 playlist",
            "",
            false,
        ),
    ];
    let row_texts = [
        "Artist Level Row",
        "The Sample Band - Album Level Row",
        "Songs Cached Row",
        "All Songs Cached Row",
        "The Sample Band - All Albums Level Row",
        "Playlist Level Row",
    ];

    for (view, row, footer, label, marked) in cases {
        let level = format!("{view:?}");
        player.current_view = view;
        let capture = captured_root_view_text_bounds(&player);
        let texts: Vec<&str> = capture.texts().collect();

        assert!(
            texts.contains(&row),
            "{level} must draw its own row text {row:?}: {:?}",
            capture.fragments()
        );
        assert!(
            texts.contains(&footer),
            "{level} must draw its own footer {footer:?}: {:?}",
            capture.fragments()
        );
        if !label.is_empty() {
            assert!(
                texts.contains(&label),
                "{level} must pass its cached row label {label:?}: {:?}",
                capture.fragments()
            );
        }
        for other in row_texts {
            if other != row {
                assert!(
                    !texts.contains(&other),
                    "{level} also drew another level's row {other:?}: {:?}",
                    capture.fragments()
                );
            }
        }
        assert_eq!(
            texts.iter().any(|text| text.starts_with("▶ ")),
            marked,
            "{level} marked the playing row: {:?}",
            capture.fragments()
        );
    }
}
