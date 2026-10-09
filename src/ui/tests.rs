use super::views::BROWSE_VIEWS;
use super::*;
use crate::equalizer::{GAIN_MAX_DB, GAIN_MIN_DB};
use crate::music_kit_auth::MusicKitSession;
use crate::sample_library::sample_library;
use crate::test_support::{
    AudioCall, RecordingAudio, StubTransport, assert_ids, rock_preset, sample_album, sample_artist,
    sample_song, second_album_songs, stepping_songs,
};

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

/// A fresh request generation over its own counter — the starting
/// generation the `fetch_into` and `play_into` tests issue before driving
/// their task, so no two tests share a counter.
fn test_generation() -> RequestGeneration {
    RequestGeneration::issue(&Arc::new(AtomicU64::new(0)))
}

/// A fresh player over the shared three-song stepping album
/// ([`stepping_songs`]) with `current` recorded as the shared state's
/// current track — the starting shape the Next/Previous wiring tests step
/// from. Seven tests build this same `test_player`-plus-fixture setup; only
/// the starting track differs (or is absent), so it lives here once.
fn player_stepping_from(current: Option<&str>) -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let (mut player, state) = test_player();
    player.songs.items = stepping_songs();
    state.blocking_lock().current_track = current.map(str::to_string);
    (player, state)
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

/// Seeds `player`'s three browse buffers with one each of the shared
/// [`sample_artist`], [`sample_album`], and [`sample_song`] fixtures — the
/// "loaded" browse shape. The failure-arm test and the view-construction test
/// both build this same three-buffer setup before exercising their own
/// contract, so it lives here once.
fn seed_browse_lists(player: &mut WinampPlayer) {
    player.artists.items = vec![sample_artist()];
    player.albums.items = vec![sample_album()];
    player.songs.items = vec![sample_song()];
}

/// Asserts the player has no rows in any of its three browse buffers. The
/// failure-arm test (which clears all three) and the startup test (which
/// begins with all three empty) both pin this same "nothing loaded" shape, so
/// the three-field check lives here once.
fn assert_browse_lists_empty(player: &WinampPlayer) {
    assert!(player.artists.items.is_empty());
    assert!(player.albums.items.is_empty());
    assert!(player.songs.items.is_empty());
}

/// Asserts the player is showing `expected`. The tests pin the current
/// view at each navigation step — `ArtistSelected`, `AlbumSelected`, and
/// `Back` — and at startup, so the same view-equality check lives here
/// once instead of spelling out a `matches!` at every site.
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
/// assertion live here once instead of at each call site. The wait is bounded
/// by [`TASK_TIMEOUT`], so a task that hangs fails this test by name rather
/// than stalling the suite.
async fn drive_task(task: Task<Message>, what: &str, mut check: impl FnMut(Message)) {
    use futures::StreamExt;

    let mut stream = iced_runtime::task::into_stream(task).expect("task must schedule a stream");
    let action = await_or_timeout(what, TASK_TIMEOUT, stream.next())
        .await
        .expect("task must yield a completion message");
    match action {
        iced_runtime::Action::Output(message) => check(message),
        other => panic!("unexpected {what} task output: {other:?}"),
    }
}

/// Drives `task` to completion and returns its single output message — the
/// message the iced runtime would deliver. [`drive_task`] hands a task's
/// output to a callback for inspection; the tests that must feed that output
/// back through `update` (the two stale-reply tests, the two play-completion
/// tests, and [`drive_fetch_and_assert_loaded`]) instead need the message
/// itself, so the capture-and-return plumbing lives here once. `what` names
/// the task in `drive_task`'s timeout panic. Callers on a plain thread reach
/// this through `futures::executor::block_on`, the same way the stepping
/// tests drive `drive_task`.
async fn task_output(task: Task<Message>, what: &str) -> Message {
    let mut output: Option<Message> = None;
    drive_task(task, what, |message| output = Some(message)).await;
    output.expect("drive_task must pass the output to its callback")
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
/// the player's matching buffer in library order. The three fetch tests
/// — loading artists, albums by artist, and songs from album — all repeat
/// this drive-then-feed-then-assert flow; only the buffer, the id key, and
/// the expected ids differ, so they come in as parameters and the flow
/// lives here once.
///
/// The task's own output message is fed straight back, rather than a
/// reconstructed copy: each fetch test used to pass a match-and-clone
/// closure naming its `*Loaded` variant plus that variant's constructor,
/// and this helper now takes neither — the task's message serves as both.
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

// The three toggle arms share one shape: read the flag, update, read it
// again. `assert_toggles_shared_state` owns that shape, so each test only
// names its message and the flag it flips. All three use `blocking_lock`,
// which panics inside an async runtime, so they stay plain tests (no
// `#[tokio::test]`).
#[test]
fn play_pause_toggles_is_playing() {
    assert_toggles_shared_state(Message::PlayPause, |state| state.is_playing);
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

// `Message::Stop` uses `blocking_lock`, which panics inside an async
// runtime, so this stays a plain test (no `#[tokio::test]`).
#[test]
fn stop_clears_is_playing() {
    let (mut player, state) = test_player();

    let _ = update(&mut player, Message::PlayPause);
    assert!(state.blocking_lock().is_playing);

    let _ = update(&mut player, Message::Stop);
    assert!(!state.blocking_lock().is_playing);
}

#[test]
fn play_and_pause_set_the_playback_flag_explicitly() {
    let (mut player, state) = test_player();

    // X plays even when already playing, unlike the Play/Pause toggle.
    assert_message_schedules_no_work(&mut player, Message::Play);
    assert!(state.blocking_lock().is_playing);
    assert_message_schedules_no_work(&mut player, Message::Play);
    assert!(state.blocking_lock().is_playing);

    // C pauses even when already paused.
    assert_message_schedules_no_work(&mut player, Message::Pause);
    assert!(!state.blocking_lock().is_playing);
    assert_message_schedules_no_work(&mut player, Message::Pause);
    assert!(!state.blocking_lock().is_playing);
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

    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Artists);
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
        preview_url: Some("https://example.test/preview.m4a".to_string()),
    }];

    let task = update(&mut player, Message::TrackSelected { epoch: 0, index: 0 });
    drive_task(task, "play", |_| {}).await;

    assert_eq!(
        recording.calls(),
        vec![AudioCall::Play(
            "https://example.test/preview.m4a".to_string()
        )]
    );
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
            .authenticate_with("dev-token", &|_| {
                Ok(MusicKitSession {
                    developer_token: "dev-token".to_string(),
                    user_token: "user-token".to_string(),
                })
            })
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
    drive_task(task, "playback", |message| {
        assert!(matches!(message, Message::TrackPlayed { .. }));
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

    let task = update(&mut player, Message::TrackPlayed { generation: 0 });

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
/// completion carrying `expected_generation`, as the iced runtime would
/// deliver the play task's completion. The two `play_into` tests both drive a
/// play to completion and pin the same contract — the completion carries the
/// generation the play was issued as, so `update` can tell a superseded play's
/// completion from the current one — so the drive-and-assert block lives here
/// once. Async, unlike [`assert_track_selected`], because the `play_into`
/// tests run inside a tokio runtime.
async fn assert_track_played(task: Task<Message>, expected_generation: u64) {
    drive_task(task, "play", |message| match message {
        Message::TrackPlayed { generation } => assert_eq!(generation, expected_generation),
        other => panic!("unexpected play task output: {other:?}"),
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
/// toggles, and a recorded shade measurement — each used to repeat the same
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

// Previous ignores Shuffle — the app keeps no play history to walk back — so
// with the flag on it still steps one song backward, exactly as with it off.
#[test]
fn previous_with_shuffle_still_steps_backward() {
    let (mut player, state) = player_stepping_from(Some("song-2"));
    state.blocking_lock().shuffle = true;

    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, 0, 0);
}

/// Feeds a `*Loaded` message built from `items` back through `update` and
/// asserts the list lands in `buffer` unchanged and that the matching
/// `loading` flag is cleared — a `*Loaded` reply reaches `BrowseList::store`,
/// which clears the flag (see `views::browse_placeholder`), so the reply must
/// leave that list not-loading. The three
/// `*_loaded_populates_list` tests — artists, albums, songs — each used
/// to repeat the same update-then-compare flow, differing only in the
/// fixture, the `*Loaded` message variant, the buffer it fills, and the
/// loading flag it clears; the message constructor, the buffer, and the
/// flag reader come in as parameters so the flow lives here once and each
/// test only names its fixture and target.
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
// `AlbumsLoadFailed` leaves the other two unpinned — a copy-paste that routed
// a songs failure into `artists.error`, say, would still compile and pass the
// suite. This drives all three and asserts each report lands in its own
// list's error field with the other two untouched, and that each failure
// clears its own buffer and loading flag.
#[test]
fn each_browse_failure_stores_its_report_in_its_own_list() {
    let (mut player, _state) = test_player();
    seed_browse_lists(&mut player);
    player.artists.loading = true;
    player.albums.loading = true;
    player.songs.loading = true;

    let artists_report = "music-library fetch failed (loading favorite artists): artists boom";
    let albums_report =
        "music-library fetch failed (loading albums for artist \"artist-1\"): albums boom";
    let songs_report =
        "music-library fetch failed (loading songs from album \"album-1\"): songs boom";

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

    assert_browse_lists_empty(&player);
    assert!(!player.artists.loading);
    assert!(!player.albums.loading);
    assert!(!player.songs.loading);
    assert_eq!(player.artists.error.as_deref(), Some(artists_report));
    assert_eq!(player.albums.error.as_deref(), Some(albums_report));
    assert_eq!(player.songs.error.as_deref(), Some(songs_report));
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
fn track_selected_records_the_played_tracks_title_and_duration() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 1 });

    let track = player.known_tracks.get("song-2").expect("song-2 recorded");
    assert_eq!(track.title, "Two");
    assert_eq!(track.duration_ms, 125_000);
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
    let (mut player, state) = test_player();

    // Play song-1 (title "One") from album-1 — recording its title in
    // `known_tracks` — then browse to album-2's songs, the flow that used
    // to leave the bar showing "song-1".
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    state.blocking_lock().current_track = Some("song-1".to_string());
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));

    let current_track = state.blocking_lock().current_track.clone();
    assert_eq!(player.now_playing_label(current_track.as_deref()), "One");
}

// The Now Playing bar's time readout comes from the same `known_tracks` entry
// as its label: a played track's duration must reach
// `WinampPlayer::now_playing_time`, and — like the title — survive a later
// browse to a different album that replaces `songs`. An unknown track (or no
// current track) reads the `--:--` placeholder.
#[test]
fn now_playing_time_shows_the_played_tracks_duration_after_browsing_to_another_album() {
    let (mut player, state) = test_player();

    assert_eq!(player.now_playing_time(None), "--:--");

    // Play song-1 (210_000 ms -> 3:30) from album-1, then browse to
    // album-2's songs, which replaces `songs`.
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    state.blocking_lock().current_track = Some("song-1".to_string());
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));

    let current_track = state.blocking_lock().current_track.clone();
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");
    assert_eq!(player.now_playing_time(Some("no-such-song")), "--:--");
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
    assert_track_played(task, expected).await;
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
    assert_track_played(task, expected).await;

    assert_eq!(received.try_recv().ok().as_deref(), Some("boom"));
}

#[test]
fn new_player_starts_at_artists_with_nothing_selected() {
    let (player, _state) = test_player();

    assert_view(&player, CurrentView::Artists);
    assert_browse_lists_empty(&player);
    // The artists fetch `boot` schedules is in flight, so the panel shows
    // "Loading…" rather than claiming an empty library (see
    // `views::browse_placeholder`); the two lower lists are not loading.
    assert!(player.artists.loading);
    assert!(!player.albums.loading);
    assert!(!player.songs.loading);
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
// buffer, and stacks the Now Playing bar, the transport row, the optional
// Back button (only below the artist list), and the browse list. No other
// test reaches this assembly — the shade test stops at `view`'s title-bar
// early return, the `views.rs` tests stop at the individual builders, and
// the `update` tests stop before the view layer — so a regression
// that made this assembly panic (a bad slider range, a `view_*` builder
// failing over the input the app produces) would take the window down on
// every refresh with no test catching it. iced `Element`s expose no tree
// introspection, so the observable contract — as with the `views.rs`
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

    // The loaded shape: artists, albums, and songs in the browse buffers,
    // rendered for every now-playing resolution a session can reach.
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
    player.artists.error = Some("music-library fetch failed: boom".to_string());
    player.albums.error = Some("music-library fetch failed: boom".to_string());
    player.songs.error = Some("music-library fetch failed: boom".to_string());
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
