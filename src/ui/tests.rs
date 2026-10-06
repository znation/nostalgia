use super::*;
use crate::library::{
    sample_album, sample_artist, sample_song, second_album_songs, stepping_songs,
};

/// A fresh player over its own shared state, so a test can inspect the
/// same `AppState` the player mutates.
fn test_player() -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let state = Arc::new(Mutex::new(AppState::default()));
    (WinampPlayer::new(state.clone()), state)
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

/// Drives the task a browse arm schedules, feeds the fetched `*Loaded`
/// message back through the update loop, and asserts the list lands in
/// the player's matching buffer in library order. The three fetch tests
/// — loading artists, albums by artist, and songs from album — all repeat
/// this drive-then-feed-then-assert flow; only the message variant, the
/// buffer, the id key, and the expected ids differ, so they come in as
/// parameters and the flow lives here once.
///
/// `clippy::too_many_arguments` is allowed: the eight parameters are the
/// natural vocabulary of the three fetch tests — each one is supplied by
/// every call site, and bundling them into a struct would only add a
/// construction site per test — so the arity is the shape of the flow,
/// not a readability smell.
#[allow(clippy::too_many_arguments)]
async fn drive_fetch_and_assert_loaded<T, Extract, Buffer, Key>(
    player: &mut WinampPlayer,
    task: Task<Message>,
    what: &str,
    extract: Extract,
    loaded: impl Fn(Vec<T>) -> Message,
    buffer: Buffer,
    key: Key,
    expected_ids: &[&str],
) where
    T: 'static,
    Extract: Fn(&Message) -> Option<Vec<T>>,
    Buffer: Fn(&mut WinampPlayer) -> &mut Vec<T>,
    Key: Fn(&T) -> &str,
{
    let mut fetched: Option<Vec<T>> = None;
    drive_task(task, what, |message| {
        fetched = Some(
            extract(&message)
                .unwrap_or_else(|| panic!("unexpected {what} task output: {message:?}")),
        );
    })
    .await;

    let items = fetched.expect("browse task must yield a *Loaded message");
    let _ = update(player, loaded(items));
    let ids: Vec<&str> = buffer(player).iter().map(key).collect();
    assert_eq!(ids, expected_ids);
}

// `Message::PlayPause` uses `blocking_lock`, which panics inside an async
// runtime, so this stays a plain test (no `#[tokio::test]`).
#[test]
fn play_pause_toggles_is_playing() {
    let (mut player, state) = test_player();

    let _ = update(&mut player, Message::PlayPause);
    assert!(state.blocking_lock().is_playing);

    let _ = update(&mut player, Message::PlayPause);
    assert!(!state.blocking_lock().is_playing);
}

// `Message::ToggleRepeat` uses `blocking_lock`, which panics inside an
// async runtime, so this stays a plain test (no `#[tokio::test]`), like
// `play_pause_toggles_is_playing`.
#[test]
fn toggle_repeat_flips_shared_state() {
    let (mut player, state) = test_player();
    assert!(!state.blocking_lock().repeat);

    let _ = update(&mut player, Message::ToggleRepeat);
    assert!(state.blocking_lock().repeat);

    let _ = update(&mut player, Message::ToggleRepeat);
    assert!(!state.blocking_lock().repeat);
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
    let (mut player, state) = test_player();
    // Default volume is 0.5 (see `AppState::default`).
    assert_eq!(state.blocking_lock().volume, 0.5);

    // Out-of-range slider values are clamped by the update arm.
    let _ = update(&mut player, Message::VolumeChange(1.5));
    assert_eq!(state.blocking_lock().volume, 1.0);

    let _ = update(&mut player, Message::VolumeChange(-0.2));
    assert_eq!(state.blocking_lock().volume, 0.0);

    // An in-range value is stored as-is.
    let _ = update(&mut player, Message::VolumeChange(0.3));
    assert_eq!(state.blocking_lock().volume, 0.3);
}

#[test]
fn artist_selected_flips_to_albums_view() {
    let (mut player, _state) = test_player();

    let _ = update(&mut player, Message::ArtistSelected("artist-1".to_string()));

    assert!(matches!(player.current_view, CurrentView::Albums));
}

#[test]
fn album_selected_flips_to_songs_view() {
    let (mut player, _state) = test_player();

    let _ = update(&mut player, Message::AlbumSelected("album-3".to_string()));

    assert!(matches!(player.current_view, CurrentView::Songs));
}

// The browse hierarchy must be navigable back up (Songs → Albums →
// Artists), not just down: without `Message::Back` a session is a one-way
// corridor that ends stuck at the bottom of the hierarchy. These arms use
// only field assignment (no `blocking_lock`), so they stay plain tests
// like the view-flip tests above.
#[test]
fn back_from_albums_returns_to_artists() {
    let (mut player, _state) = test_player();

    let _ = update(&mut player, Message::ArtistSelected("artist-1".to_string()));

    assert!(matches!(player.current_view, CurrentView::Albums));

    let _ = update(&mut player, Message::Back);
    assert!(matches!(player.current_view, CurrentView::Artists));
}

#[test]
fn back_from_songs_returns_to_albums() {
    let (mut player, _state) = test_player();

    let _ = update(&mut player, Message::ArtistSelected("artist-1".to_string()));
    let _ = update(&mut player, Message::AlbumSelected("album-1".to_string()));

    assert!(matches!(player.current_view, CurrentView::Songs));

    let _ = update(&mut player, Message::Back);
    assert!(matches!(player.current_view, CurrentView::Albums));
}

#[test]
fn back_from_artists_is_a_noop() {
    let (mut player, _state) = test_player();
    assert!(matches!(player.current_view, CurrentView::Artists));

    let _ = update(&mut player, Message::Back);
    assert!(matches!(player.current_view, CurrentView::Artists));
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

    let task = update(&mut player, Message::ArtistSelected("artist-1".to_string()));
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load albums",
        |message| match message {
            Message::AlbumsLoaded(albums) => Some(albums.clone()),
            _ => None,
        },
        Message::AlbumsLoaded,
        |player| &mut player.albums,
        |album| album.id.as_str(),
        &["album-1", "album-2"],
    )
    .await;

    // The arm flips to the albums view before the fetched list is fed
    // back through the update loop.
    assert!(matches!(player.current_view, CurrentView::Albums));
}

#[tokio::test]
async fn album_selected_fetches_the_albums_songs_into_the_player() {
    let (mut player, _state) = test_player();

    let task = update(&mut player, Message::AlbumSelected("album-3".to_string()));
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load songs",
        |message| match message {
            Message::SongsLoaded(songs) => Some(songs.clone()),
            _ => None,
        },
        Message::SongsLoaded,
        |player| &mut player.songs,
        |song| song.id.as_str(),
        &["song-5"],
    )
    .await;

    // As with the artist arm: the view flips to songs, and the fetched
    // list lands in the browse buffer in library order.
    assert!(matches!(player.current_view, CurrentView::Songs));
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
    assert!(matches!(player.current_view, CurrentView::Artists));

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
        |message| match message {
            Message::ArtistsLoaded(artists) => Some(artists.clone()),
            _ => None,
        },
        Message::ArtistsLoaded,
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

    let task = update(&mut player, Message::TrackSelected("song-1".to_string()));
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
        state.volume = 0.7;
    }

    let task = update(&mut player, Message::TrackPlayed);

    // The arm schedules no follow-up work...
    assert_no_task(task);

    // ...and leaves the shared state exactly as it was.
    let state = state.blocking_lock();
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(state.is_playing);
    assert_eq!(state.volume, 0.7);
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
/// for the given id, as the iced runtime would deliver the button's task.
/// The stepping tests drive their tasks exactly the way the playback and
/// fetch tests do, so they route through [`drive_task`] instead of
/// repeating the `into_stream`/`next` plumbing; this stays a plain
/// function (rather than `async`) because the stepping arms use
/// `blocking_lock`, which panics inside an async runtime.
fn assert_track_selected(task: Task<Message>, expected: &str) {
    futures::executor::block_on(drive_task(task, "stepping", |message| match message {
        Message::TrackSelected(id) => assert_eq!(id, expected),
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
    let (mut player, state) = test_player();
    player.songs = stepping_songs();
    state.blocking_lock().current_track = Some("song-1".to_string());

    let task = update(&mut player, Message::NextTrack);

    assert_track_selected(task, "song-2");
}

#[test]
fn previous_track_steps_to_the_preceding_song() {
    let (mut player, state) = test_player();
    player.songs = stepping_songs();
    state.blocking_lock().current_track = Some("song-2".to_string());

    let task = update(&mut player, Message::PreviousTrack);

    assert_track_selected(task, "song-1");
}

#[test]
fn next_track_with_no_current_track_starts_at_the_first_song() {
    let (mut player, _state) = test_player();
    player.songs = stepping_songs();

    let task = update(&mut player, Message::NextTrack);

    assert_track_selected(task, "song-1");
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
    let (mut player, _state) = test_player();
    player.songs = stepping_songs();

    let task = update(&mut player, Message::PreviousTrack);

    assert_track_selected(task, "song-3");
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
    let (mut player, state) = test_player();
    player.songs = stepping_songs();
    // Played song-4 (an album-2 song) from the library, then browsed to
    // album-1's songs (now the current buffer): the current track names a
    // song that is not in the buffer, exactly as after a browse-away.
    state.blocking_lock().current_track = Some("song-4".to_string());

    let next = update(&mut player, Message::NextTrack);
    assert_track_selected(next, "song-1");

    // The buttons only schedule a step — neither mutates the current
    // track — so Previous still sees the same browse-away state.
    let previous = update(&mut player, Message::PreviousTrack);
    assert_track_selected(previous, "song-3");
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

// The Next arm forwards the shared Repeat flag to the transport helper:
// from the last song, Next wraps to the first when Repeat is on and
// re-lands on the last when it is off. The arithmetic is pinned in
// `transport.rs` (`next_wraps_from_last_to_first` and
// `next_stays_on_last_without_repeat`); this pins the arm's wiring over
// the shared flag, which no other test drives.
#[test]
fn next_track_follows_the_shared_repeat_flag_at_the_albums_end() {
    let (mut player, state) = test_player();
    player.songs = stepping_songs();
    state.blocking_lock().current_track = Some("song-3".to_string());

    // Repeat off (the default): Next from the last song stays on it.
    let task = update(&mut player, Message::NextTrack);
    assert_track_selected(task, "song-3");

    // Repeat on: Next from the last song wraps to the first.
    state.blocking_lock().repeat = true;
    let task = update(&mut player, Message::NextTrack);
    assert_track_selected(task, "song-1");
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
    let (mut player, state) = test_player();
    player.songs = stepping_songs();
    state.blocking_lock().current_track = Some("song-1".to_string());

    // Repeat off (the default): Previous from the first song stays on it.
    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, "song-1");

    // Repeat on: Previous from the first song wraps to the last.
    state.blocking_lock().repeat = true;
    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, "song-3");
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

// `store_songs` records a loaded song list's id→title pairs into the
// accumulated `known_titles` index so the Now Playing bar keeps naming a
// playing track after a browse away. The index must not grow on a revisit:
// re-loading the same album re-inserts the same ids and titles, so the map
// still holds exactly the album's songs. No other test loads overlapping
// lists: `songs_loaded_populates_list` fills an empty buffer and the
// browse-away test loads two disjoint albums, so the revisit branch (a
// song arriving that the index already holds) is reachable only by loading
// the same album twice — which this does. A regression that grew the index
// per load would pass every other test while duplicating entries here.
#[test]
fn songs_loaded_does_not_grow_the_known_titles_index_on_revisit() {
    let (mut player, _state) = test_player();

    let songs = stepping_songs();
    let _ = update(&mut player, Message::SongsLoaded(songs.clone()));

    // Re-load the same album — e.g. browsing back to it after stepping
    // away — and the index must still hold exactly these three titles.
    let _ = update(&mut player, Message::SongsLoaded(songs));

    assert_eq!(player.known_titles.len(), 3);
    for (id, title) in [("song-1", "One"), ("song-2", "Two"), ("song-3", "Three")] {
        assert_eq!(player.known_titles.get(id).map(String::as_str), Some(title));
    }
}

// Browsing to a new album must *replace* the Songs view's buffer, not
// append to it: after loading album-1's songs and then browsing to
// album-2, `player.songs` holds only album-2's song — otherwise the
// Songs view would render stale rows from every album visited. The other
// `*Loaded` tests load only into an empty buffer, and the browse-away
// label test (`now_playing_label_keeps_the_track_name_after_browsing_to_another_album`)
// asserts the label but never the buffer, so an
// append-instead-of-replace regression in `store_songs` would clear every
// existing test and only fail here.
#[test]
fn songs_loaded_replaces_the_previous_albums_songs() {
    let (mut player, _state) = test_player();

    // Load album-1's three songs, then browse to album-2 (one song).
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));

    let ids: Vec<&str> = player.songs.iter().map(|song| song.id.as_str()).collect();
    assert_eq!(ids, vec!["song-4"]);
}

// The Now Playing bar must keep naming the playing track, not its raw id,
// after the user browses to a different album. `view` renders the bar's
// label through `WinampPlayer::now_playing_label`, so asserting that same
// resolution after a browse-away pins the actual bar path: if the label
// were resolved against `songs` (the currently-browsed album's list, which
// album-2's `SongsLoaded` just replaced) instead of the accumulated
// `known_titles`, song-1 would no longer be "known" and the label would
// fall back to the raw id "song-1", failing this test.
#[test]
fn now_playing_label_keeps_the_track_name_after_browsing_to_another_album() {
    let (mut player, state) = test_player();

    // Play song-1 (title "One") from album-1, then browse to album-2's
    // songs — the flow that used to leave the bar showing "song-1".
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    state.blocking_lock().current_track = Some("song-1".to_string());
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));

    let current_track = state.blocking_lock().current_track.clone();
    assert_eq!(player.now_playing_label(current_track.as_deref()), "One");
}

#[test]
fn loaded_or_empty_maps_ok_and_err_to_loaded() {
    let albums = vec![sample_album()];

    let ok_message =
        loaded_or_empty::<Album, String>(Ok(albums.clone()), Message::AlbumsLoaded, |_| {
            unreachable!("ok path must not report an error")
        });
    assert!(matches!(ok_message, Message::AlbumsLoaded(v) if v == albums));

    let err_message =
        loaded_or_empty::<Album, String>(Err("boom".to_string()), Message::AlbumsLoaded, |_| {});
    assert!(matches!(err_message, Message::AlbumsLoaded(v) if v.is_empty()));
}

#[test]
fn loaded_or_empty_reports_the_error_before_falling_back_to_empty() {
    // A failed browse fetch must not vanish silently: the load path
    // reports the error (to stderr in production; here to a recording
    // closure) and still falls back to an empty list so the UI stays
    // usable. `loaded_or_empty` takes the reporter as a parameter so this
    // contract is testable without capturing stderr.
    let mut reported: Option<String> = None;
    let message =
        loaded_or_empty::<Album, String>(Err("boom".to_string()), Message::AlbumsLoaded, |err| {
            reported = Some(err.clone())
        });

    assert_eq!(reported.as_deref(), Some("boom"));
    assert!(matches!(message, Message::AlbumsLoaded(v) if v.is_empty()));
}

#[test]
fn played_or_reported_reports_a_failed_play_and_still_completes() {
    // A failed play must not vanish silently: the playback path reports
    // the error (to stderr in production; here to a recording closure)
    // and still emits the `TrackPlayed` completion so the UI's handoff
    // stays intact. `played_or_reported` takes the reporter as a
    // parameter so this contract is testable without capturing stderr.
    let mut reported: Option<String> = None;
    let message = played_or_reported::<String>(Err("boom".to_string()), |err| {
        reported = Some(err.clone());
    });

    assert_eq!(reported.as_deref(), Some("boom"));
    assert!(matches!(message, Message::TrackPlayed));

    // The Ok path completes with the same message and reports nothing.
    let mut reported_ok: Option<String> = None;
    let ok_message = played_or_reported::<String>(Ok(()), |err| {
        reported_ok = Some(err.clone());
    });
    assert!(reported_ok.is_none());
    assert!(matches!(ok_message, Message::TrackPlayed));
}

// `fetch_into` schedules the fetch as an iced `Task`; the arm itself only
// builds it, so the real behavior lives in the returned task. Drive that
// task to completion and assert the mapped `*Loaded` message, as the
// load-path arms would produce it.
#[test]
fn fetch_failure_report_names_the_fetch_and_includes_the_error() {
    // The browse error report must identify the failing query (the fetch
    // context plus the underlying error), not just say a fetch failed —
    // otherwise a broken backend would log three identical lines for the
    // artists, albums, and songs paths with no way to tell which query
    // failed. The format is pinned here so the contract can't drift.
    let report = fetch_failure_report("loading albums for artist \"artist-1\"", &"boom");
    assert_eq!(
        report,
        "music-library fetch failed (loading albums for artist \"artist-1\"); showing an empty list: \"boom\""
    );
}

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
// is tested directly above, but no test drives a failing fetch *through*
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

    assert!(matches!(player.current_view, CurrentView::Artists));
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
                            state.volume = volume;
                        }
                        let _screen = view(&player);
                    }
                }
            }
        }
    }
}
