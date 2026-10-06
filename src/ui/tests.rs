use super::*;
use crate::equalizer::{GAIN_MAX_DB, GAIN_MIN_DB};
use crate::test_support::{
    assert_ids, sample_album, sample_artist, sample_song, second_album_songs, stepping_songs,
};

/// A fresh player over its own shared state, so a test can inspect the
/// same `AppState` the player mutates.
fn test_player() -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let state = Arc::new(Mutex::new(AppState::default()));
    (WinampPlayer::new(state.clone()), state)
}

/// A fresh player over the shared three-song stepping album
/// ([`stepping_songs`]) with `current` set as the playing track — the
/// starting shape the Next/Previous wiring tests step from. Seven tests
/// build this same `test_player`-plus-fixture setup; only the starting
/// track differs (or is absent), so it lives here once.
fn player_stepping_from(current: Option<&str>) -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let (mut player, state) = test_player();
    player.songs = stepping_songs();
    state.blocking_lock().current_track = current.map(str::to_string);
    (player, state)
}

/// Asserts the player is showing `expected`. The tests pin the current
/// view at each navigation step — `ArtistSelected`, `AlbumSelected`, and
/// `Back` — and at startup, so the same view-equality check lives here
/// once instead of spelling out a `matches!` at every site.
fn assert_view(player: &WinampPlayer, expected: CurrentView) {
    assert_eq!(player.current_view, expected);
}

/// Asserts `message` flips the shared-state flag `read_flag` from false to
/// true and back: the PlayPause, ToggleRepeat, and ToggleEqualizer update
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
/// value lands in shared state. The VolumeChange, EqPreampChange, and
/// EqBandChange arms all do the same one-value write — the setter clamps the
/// slider's value before storing it — so the lock-read-assert sequence lives
/// here once and each call only names its message, the field read back, and
/// the clamped value. The arms use `blocking_lock`, which panics inside an
/// async runtime, so this stays a plain (non-async) helper, like
/// [`assert_toggles_shared_state`].
fn assert_message_clamps(message: Message, read: impl Fn(&AppState) -> f32, expected: f32) {
    let (mut player, state) = test_player();
    let _ = update(&mut player, message);
    assert_eq!(read(&state.blocking_lock()), expected);
}

/// Drives an iced `Task` to completion and hands its single `Output`
/// action to `check`. `update` only schedules work as a `Task`, so a test
/// that wants to observe the resulting message — `TrackPlayed` after
/// playback, a `*Loaded` message after a fetch, a `TrackSelected` after a
/// step — must run the task itself the way the iced runtime would. The
/// playback, fetch, and (through `assert_track_selected`) stepping tests
/// do exactly that, so the stream plumbing and the "exactly one `Output`"
/// assertion live here once instead of at each call site.
async fn drive_task(task: Task<Message>, what: &str, mut check: impl FnMut(Message)) {
    use futures::StreamExt;

    let mut stream = iced_runtime::task::into_stream(task).expect("task must schedule a stream");
    let action = stream
        .next()
        .await
        .expect("task must yield a completion message");
    match action {
        iced_runtime::Action::Output(message) => check(message),
        other => panic!("unexpected {what} task output: {other:?}"),
    }
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
    let mut output: Option<Message> = None;
    drive_task(task, what, |message| output = Some(message)).await;

    let message = output.expect("browse task must yield a *Loaded message");
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
fn volume_change_clamps_value_before_storing() {
    // Out-of-range slider values are clamped by the update arm.
    assert_message_clamps(Message::VolumeChange(1.5), |state| state.volume(), 1.0);
    assert_message_clamps(Message::VolumeChange(-0.2), |state| state.volume(), 0.0);

    // An in-range value is stored as-is.
    assert_message_clamps(Message::VolumeChange(0.3), |state| state.volume(), 0.3);
}

#[test]
fn artist_selected_flips_to_albums_view() {
    let (mut player, _state) = test_player();
    player.artists = vec![sample_artist()];

    let _ = update(&mut player, Message::ArtistSelected { epoch: 0, index: 0 });

    assert_view(&player, CurrentView::Albums);
}

#[test]
fn album_selected_flips_to_songs_view() {
    let (mut player, _state) = test_player();
    player.albums = vec![sample_album()];

    let _ = update(&mut player, Message::AlbumSelected { epoch: 0, index: 0 });

    assert_view(&player, CurrentView::Songs);
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

    assert_no_task(update(
        &mut player,
        Message::ArtistSelected { epoch: 0, index: 0 },
    ));
    assert_no_task(update(
        &mut player,
        Message::AlbumSelected { epoch: 0, index: 0 },
    ));
    assert_no_task(update(
        &mut player,
        Message::TrackSelected { epoch: 0, index: 0 },
    ));

    assert_view(&player, CurrentView::Artists);
    assert!(state.blocking_lock().current_track.is_none());

    // Render a row from the first loaded list (epoch 1), then replace that
    // list with a same-length one (epoch 2): the stale press names index 0,
    // which is in range of the new list, so only the epoch guard stops it
    // from playing the wrong song.
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));
    assert_no_task(update(
        &mut player,
        Message::TrackSelected { epoch: 1, index: 0 },
    ));
    assert!(state.blocking_lock().current_track.is_none());
}

// The browse hierarchy must be navigable back up (Songs → Albums →
// Artists), not just down: without `Message::Back` a session is a one-way
// corridor that ends stuck at the bottom of the hierarchy. These arms use
// only field assignment (no `blocking_lock`), so they stay plain tests
// like the view-flip tests above.
#[test]
fn back_from_albums_returns_to_artists() {
    let (mut player, _state) = test_player();
    player.artists = vec![sample_artist()];

    let _ = update(&mut player, Message::ArtistSelected { epoch: 0, index: 0 });

    assert_view(&player, CurrentView::Albums);

    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Artists);
}

#[test]
fn back_from_songs_returns_to_albums() {
    let (mut player, _state) = test_player();
    player.artists = vec![sample_artist()];
    player.albums = vec![sample_album()];

    let _ = update(&mut player, Message::ArtistSelected { epoch: 0, index: 0 });
    let _ = update(&mut player, Message::AlbumSelected { epoch: 0, index: 0 });

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
    player.artists = vec![sample_artist()];

    let task = update(&mut player, Message::ArtistSelected { epoch: 0, index: 0 });
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load albums",
        |player| &mut player.albums,
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
    player.albums = vec![Album {
        id: "album-3".to_string(),
        title: "Debut".to_string(),
        artist_id: "artist-2".to_string(),
    }];

    let task = update(&mut player, Message::AlbumSelected { epoch: 0, index: 0 });
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load songs",
        |player| &mut player.songs,
        |song| song.id.as_str(),
        &["song-5"],
    )
    .await;

    // As with the artist arm: the view flips to songs, and the fetched
    // list lands in the browse buffer in library order.
    assert_view(&player, CurrentView::Songs);
}

// Startup wiring: `boot` hands iced a fresh player plus the task that
// loads the artist list, and `update`'s `LoadArtists` arm runs that fetch
// into the player's `artists` buffer. A regression that stopped boot from
// emitting the task — or pointed the arm at the wrong fetch — would open
// the app with an empty browse list, so the wiring is pinned end to end.

#[tokio::test]
async fn boot_schedules_loading_the_artist_list() {
    let state = Arc::new(Mutex::new(AppState::default()));

    let (player, task) = boot(state);
    assert_view(&player, CurrentView::Artists);

    drive_task(task, "boot", |message| {
        assert!(matches!(message, Message::LoadArtists));
    })
    .await;
}

#[tokio::test]
async fn load_artists_fetches_favorite_artists_into_the_player() {
    let (mut player, _state) = test_player();

    let task = update(&mut player, Message::LoadArtists);
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load artists",
        |player| &mut player.artists,
        |artist| artist.id.as_str(),
        &["artist-1", "artist-2", "artist-3"],
    )
    .await;
}

// `Message::TrackSelected` wraps playback in a `Task`; the arm itself only
// schedules it, so the real behavior lives in the returned task. Drive that
// task to completion (as the iced runtime would) and assert the shared
// state it mutates through the service.
#[tokio::test]
async fn track_selected_starts_playback_of_the_selected_track() {
    let (mut player, state) = test_player();
    player.songs = vec![sample_song()];

    let task = update(&mut player, Message::TrackSelected { epoch: 0, index: 0 });
    drive_task(task, "playback", |message| {
        assert!(matches!(message, Message::TrackPlayed));
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
// shared state) with no test catching it. Pin the arm's no-op contract: it
// schedules no work and leaves the shared state untouched. `blocking_lock`
// panics inside an async runtime, so this stays a plain test.
#[test]
fn track_played_handoff_is_a_noop() {
    let (mut player, state) = test_player();

    // A track is mid-playback when the completion handoff arrives.
    {
        let mut state = state.blocking_lock();
        state.current_track = Some("song-1".to_string());
        state.is_playing = true;
        state.set_volume(0.7);
    }

    let task = update(&mut player, Message::TrackPlayed);

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

/// Asserts that an update arm scheduled no follow-up work: `update`
/// returns `Task::none()` for a no-op arm, and iced's `Task` represents
/// that as no stream to run. The no-op arms — `TrackPlayed`, and
/// Next/Previous with an empty `songs` buffer — each used to repeat the
/// same `into_stream(task).is_none()` probe, so it lives here once and
/// "this arm schedules nothing" reads as the named contract it is.
fn assert_no_task(task: Task<Message>) {
    assert!(iced_runtime::task::into_stream(task).is_none());
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

// Browsing to a different album replaces `player.songs` with the new
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
// tests set a current track that IS in `player.songs` or load no songs, so
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

    let task = update(&mut player, Message::NextTrack);

    assert_no_task(task);
}

#[test]
fn previous_track_with_no_songs_loaded_does_nothing() {
    let (mut player, _state) = test_player();

    let task = update(&mut player, Message::PreviousTrack);

    assert_no_task(task);
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
// Previous wiring tests set a current track with Repeat off or load no
// songs, so an arm that dropped the shared flag (or passed a stale one)
// would clear every existing test and only fail here.
#[test]
fn previous_track_follows_the_shared_repeat_flag_at_the_albums_start() {
    assert_repeat_wraps_at_the_edge(Message::PreviousTrack, "song-1", 0, 2);
}

/// Feeds a `*Loaded` message built from `items` back through `update` and
/// asserts the list lands in `buffer` unchanged. The three
/// `*_loaded_populates_list` tests — artists, albums, songs — each used
/// to repeat the same update-then-compare flow, differing only in the
/// fixture, the `*Loaded` message variant, and the buffer it fills; the
/// message constructor and the buffer come in as parameters so the flow
/// lives here once and each test only names its fixture and target.
fn assert_store_loaded<T: Clone + PartialEq + std::fmt::Debug>(
    player: &mut WinampPlayer,
    items: Vec<T>,
    loaded: impl Fn(Vec<T>) -> Message,
    buffer: impl Fn(&mut WinampPlayer) -> &mut Vec<T>,
) {
    let _ = update(player, loaded(items.clone()));
    assert_eq!(&*buffer(player), &items);
}

#[test]
fn artists_loaded_populates_list() {
    let (mut player, _state) = test_player();
    let artists = vec![sample_artist()];

    assert_store_loaded(&mut player, artists, Message::ArtistsLoaded, |player| {
        &mut player.artists
    });
}

#[test]
fn albums_loaded_populates_list() {
    let (mut player, _state) = test_player();
    let albums = vec![sample_album()];

    assert_store_loaded(&mut player, albums, Message::AlbumsLoaded, |player| {
        &mut player.albums
    });
}

#[test]
fn songs_loaded_populates_list() {
    let (mut player, _state) = test_player();
    let songs = vec![sample_song()];

    assert_store_loaded(&mut player, songs, Message::SongsLoaded, |player| {
        &mut player.songs
    });
}

// The `known_titles` index backs the Now Playing bar's title lookup, and it
// is populated only when a track is played — not when an album is loaded.
// Folding a browsed album's songs into the index would make it grow with
// every album the user visits while the bar only ever reads the playing
// track's entry, so the browse path must leave the index untouched and
// `TrackSelected` must record the played song's title.
#[test]
fn songs_loaded_does_not_populate_the_known_titles_index() {
    let (mut player, _state) = test_player();

    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    assert!(player.known_titles.is_empty());
}

#[test]
fn track_selected_records_the_played_tracks_title() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 1 });

    assert_eq!(
        player.known_titles.get("song-2").map(String::as_str),
        Some("Two")
    );
}

// The recording above only covers a `TrackSelected` whose id is still in the
// loaded `songs`, so the arm's `if let Some(song) = player.songs.iter().find(..)`
// guard is never entered with `None`. A stale click reaches that branch: the
// row was rendered from the album the user just browsed away from, and its
// click message is processed after `SongsLoaded` replaced `songs`, so the id
// no longer resolves to a song and the title cannot be recovered. The guard
// must then leave `known_titles` exactly as it was — an already-recorded
// title survives the stale click, and an id that was never loaded adds no
// entry (the index stays proportional to songs actually played). A
// regression that fell through to a fallback insert would either clobber a
// known title or grow the index with stale ids.
#[test]
fn track_selected_ignores_an_id_no_longer_in_the_songs_buffer() {
    let (mut player, _state) = test_player();

    // Play song-1 (title "One") while album-1 is loaded, recording its title.
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::TrackSelected("song-1".to_string()));

    // Browse to album-2, replacing `songs`, then deliver the stale click for
    // the just-left song and one for an id never loaded at all.
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));
    let _ = update(&mut player, Message::TrackSelected("song-1".to_string()));
    let _ = update(&mut player, Message::TrackSelected("song-9".to_string()));

    // The stale clicks changed nothing: the known title survived and no entry
    // was added for the id that was never loaded.
    assert_eq!(
        player.known_titles.get("song-1").map(String::as_str),
        Some("One")
    );
    assert!(!player.known_titles.contains_key("song-9"));
    assert_eq!(player.known_titles.len(), 1);

    // The Now Playing bar still names the playing track rather than its id.
    assert_eq!(player.now_playing_label(Some("song-1")), "One");
}

// Browsing to a new album must *replace* the Songs view's buffer, not
// append to it: after loading album-1's songs and then browsing to
// album-2, `player.songs` holds only album-2's song — otherwise the
// Songs view would render stale rows from every album visited. The other
// `*Loaded` tests load only into an empty buffer, and the browse-away
// label test (`now_playing_label_keeps_the_track_name_after_browsing_to_another_album`)
// asserts the label but never the buffer, so an
// append-instead-of-replace regression in `store_loaded` would clear every
// existing test and only fail here.
#[test]
fn songs_loaded_replaces_the_previous_albums_songs() {
    let (mut player, _state) = test_player();

    // Load album-1's three songs, then browse to album-2 (one song).
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));

    assert_ids(&player.songs, |song| song.id.as_str(), &["song-4"]);
}

// The Now Playing bar must keep naming the playing track, not its raw id,
// after the user browses to a different album. `view` renders the bar's
// label through `WinampPlayer::now_playing_label`, so asserting that same
// resolution after a browse-away pins the actual bar path: if the label
// were resolved against `songs` (the currently-browsed album's list, which
// album-2's `SongsLoaded` just replaced) instead of the `known_titles`
// entry recorded when song-1 was played, song-1 would no longer be "known"
// and the label would fall back to the raw id "song-1", failing this test.
#[test]
fn now_playing_label_keeps_the_track_name_after_browsing_to_another_album() {
    let (mut player, state) = test_player();

    // Play song-1 (title "One") from album-1 — recording its title in
    // `known_titles` — then browse to album-2's songs, the flow that used
    // to leave the bar showing "song-1".
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    state.blocking_lock().current_track = Some("song-1".to_string());
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));

    let current_track = state.blocking_lock().current_track.clone();
    assert_eq!(player.now_playing_label(current_track.as_deref()), "One");
}

// `fetch_into` schedules the fetch as an iced `Task`; the arm itself only
// builds it, so the real behavior lives in the returned task. Drive that
// task to completion and assert the mapped `*Loaded` message, as the
// load-path arms would produce it.
#[tokio::test]
async fn fetch_into_schedules_fetch_and_maps_result_to_loaded_message() {
    let (player, _state) = test_player();

    let task = fetch_into(
        &player.apple_music_service,
        "loading favorite artists".to_string(),
        |service| async move { service.get_favorite_artists().await },
        Message::ArtistsLoaded,
    );
    drive_task(task, "fetch", |message| {
        assert!(matches!(
            message,
            Message::ArtistsLoaded(artists) if !artists.is_empty()
        ));
    })
    .await;
}

// The success path above proves `fetch_into` delivers a `*Loaded` message
// for a successful fetch; this pins the error path — a fetch that returns
// `Err` must still yield the matching `*Loaded` message with an empty
// list, so the browse view falls back to an empty list instead of waiting
// forever on a list that never arrives. `loaded_or_empty`'s error branch
// is tested in `loading`'s test module, but no test drives a failing fetch *through*
// `fetch_into` — the real service always succeeds, so the error path is
// reachable only by injecting a failing fetch closure, which is exactly
// what this does. A regression that swallowed the error (or failed to
// emit any message) would leave the browse view stuck, and only this
// test would catch it.
#[tokio::test]
async fn fetch_into_maps_a_failed_fetch_to_an_empty_loaded_message() {
    let (player, _state) = test_player();

    let task = fetch_into(
        &player.apple_music_service,
        "loading albums for artist \"artist-1\"".to_string(),
        |_service| async { Err::<Vec<Album>, String>("boom".to_string()) },
        Message::AlbumsLoaded,
    );
    drive_task(task, "failed fetch", |message| {
        assert!(matches!(
            message,
            Message::AlbumsLoaded(albums) if albums.is_empty()
        ));
    })
    .await;
}

#[test]
fn new_player_starts_at_artists_with_nothing_selected() {
    let (player, _state) = test_player();

    assert_view(&player, CurrentView::Artists);
    assert!(player.artists.is_empty());
    assert!(player.albums.is_empty());
    assert!(player.songs.is_empty());
}

// `view` is the per-frame assembly: it locks the shared state, resolves
// the now-playing label, matches the current browse view onto its list
// buffer, and stacks the Now Playing bar, the transport row, the optional
// Back button (only below the artist list), and the browse list. No other
// test reaches it — the `views.rs` tests stop at the individual builders
// and the `update` tests stop before the view layer — so a regression
// that made this assembly panic (a bad slider range, a `view_*` builder
// failing over the input the app produces) would take the window down on
// every refresh with no test catching it. iced `Element`s expose no tree
// introspection, so the observable contract — as with the `views.rs`
// builder tests — is that `view` builds its widget tree without panicking
// over the space the app actually produces: every browse view (including
// the pre-load empty buffers), both play states, both repeat states, the
// volume endpoints the update arm can store, and each now-playing
// resolution. `view` uses `blocking_lock`, which panics inside an async
// runtime, so this stays a plain test.
#[test]
fn view_constructs_over_the_apps_full_input_space() {
    let (mut player, state) = test_player();

    // The pre-load shape: nothing fetched, no current track. This is what
    // the window shows before the first `LoadArtists` fetch lands, in
    // every browse view.
    for current_view in [
        CurrentView::Artists,
        CurrentView::Albums,
        CurrentView::Songs,
    ] {
        player.current_view = current_view;
        let _empty = view(&player);
    }

    // The loaded shape: artists, albums, and songs in the browse buffers,
    // rendered for every now-playing resolution a session can reach.
    player.artists = vec![sample_artist()];
    player.albums = vec![sample_album()];
    player.songs = vec![sample_song()];

    let now_playing_options = [None, Some("song-1"), Some("no-such-song")];
    let play_states = [false, true];
    let repeats = [false, true];
    let volumes = [0.0, 0.5, 1.0];

    for current_view in [
        CurrentView::Artists,
        CurrentView::Albums,
        CurrentView::Songs,
    ] {
        player.current_view = current_view;
        for current_track in now_playing_options {
            for is_playing in play_states {
                for repeat in repeats {
                    for volume in volumes {
                        {
                            let mut state = state.blocking_lock();
                            state.current_track = current_track.map(str::to_string);
                            state.is_playing = is_playing;
                            state.repeat = repeat;
                            state.set_volume(volume);
                        }
                        let _screen = view(&player);
                    }
                }
            }
        }
    }
}
