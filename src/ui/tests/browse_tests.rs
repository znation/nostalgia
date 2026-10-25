use super::*;

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

// The Open button opens whatever row the showing list has selected, not only
// a song: the README promises it drills into an artist, album, or playlist
// exactly as a second row press does. `open_selected` resolves the selection
// against the current view and `open_row` maps that view to its selection
// message, so pin every arm the song-only test above leaves out. A dropped or
// swapped arm would make Open silently do nothing (or open the wrong level)
// on that browse screen.
#[test]
fn open_selected_opens_the_selected_row_on_every_browse_level() {
    for view in [
        CurrentView::Artists,
        CurrentView::Albums,
        CurrentView::AllSongs,
        CurrentView::AllAlbums,
        CurrentView::Playlists,
    ] {
        let (mut player, _state) = test_player();
        seed_browse_lists(&mut player);
        player.current_view = view;
        let epoch = match &player.current_view {
            CurrentView::Artists => player.artists.epoch,
            CurrentView::Albums => player.albums.epoch,
            CurrentView::AllSongs => player.all_songs.epoch,
            CurrentView::AllAlbums => player.all_albums.epoch,
            CurrentView::Playlists => player.playlists.epoch,
            CurrentView::Songs => unreachable!("the Songs level has its own Open test"),
        };

        assert_message_schedules_no_work(&mut player, Message::SelectRow { epoch, index: 0 });
        let task = update(&mut player, Message::OpenSelected);
        let opened = futures::executor::block_on(task_output(task, "open the selected row"));

        let expected = match &player.current_view {
            CurrentView::Artists => {
                matches!(&opened, Message::ArtistSelected { epoch: e, index: 0 } if *e == epoch)
            }
            CurrentView::Albums | CurrentView::AllAlbums => {
                matches!(&opened, Message::AlbumSelected { epoch: e, index: 0 } if *e == epoch)
            }
            CurrentView::AllSongs => {
                matches!(&opened, Message::TrackSelected { epoch: e, index: 0 } if *e == epoch)
            }
            CurrentView::Playlists => {
                matches!(&opened, Message::PlaylistSelected { epoch: e, index: 0 } if *e == epoch)
            }
            CurrentView::Songs => unreachable!("the Songs level has its own Open test"),
        };
        assert!(
            expected,
            "Open from {:?} must dispatch the selected row, got {opened:?}",
            player.current_view
        );
    }
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

    // The artists list caches each row's album-count label when it is stored,
    // in row order, so a view refresh borrows it instead of formatting the
    // same count again. The title cache stays empty because the name is
    // already borrowed from the row.
    let mut artist = sample_artist();
    artist.album_count = 2;
    let _ = update(&mut player, Message::ArtistsLoaded(vec![artist]));
    assert_eq!(
        player.artists.labels,
        ["2 albums"],
        "the artist row's album count is cached at store time"
    );
    assert!(
        player.artists.titles.is_empty(),
        "the artists list caches only labels, not a duplicate of every name"
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

    // The list caches each row's display title and formatted running time
    // when it is stored, in row order, so a view refresh borrows them instead
    // of re-joining every artist and title and formatting every duration
    // again (see `BrowseList::store_with_row_text`).
    let mut album = sample_album();
    album.duration_ms = 630_000;
    let _ = update(&mut player, Message::AlbumsLoaded(vec![album]));
    assert_eq!(
        player.albums.titles,
        ["The Sample Band - First Record"],
        "the album row's display title is cached at store time"
    );
    assert_eq!(
        player.albums.labels,
        ["10:30"],
        "the album row's running time is cached at store time"
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
