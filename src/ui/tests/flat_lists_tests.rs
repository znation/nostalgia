use super::*;

/// Asserts a navigation button that hangs off the Artists list — All Songs,
/// All Albums, or Playlists — opens `view` with an active search left behind:
/// none of those lists is built from a search's results. Used by the three
/// `show_*_clears_an_active_search` tests.
pub(super) fn assert_navigation_clears_the_search(message: Message, view: CurrentView) {
    let (mut player, _state) = test_player();
    player.search_active = true;
    player.current_view = CurrentView::Songs;

    let _ = update(&mut player, message);

    assert!(!player.search_active);
    assert_view(&player, view);
}

/// Asserts Back from `view` — one of the views that hangs off the Artists list
/// — returns to Artists. Used by the three `back_from_*_returns_to_the_artists_list`
/// tests.
pub(super) fn assert_back_returns_to_artists(view: CurrentView) {
    let (mut player, _state) = test_player();
    player.current_view = view;

    let _ = update(&mut player, Message::Back);

    assert_view(&player, CurrentView::Artists);
}

/// Asserts that opening row 0 of a Genres, Years, or Composers list shows the
/// group's first and third sample songs under the Songs view with `origin`, and
/// that Back steps to `view` and then to Artists. `select` builds the row-press
/// message from the list's epoch and a row index.
pub(super) fn assert_group_opens_its_songs_and_back_unwinds(
    mut player: WinampPlayer,
    epoch: u64,
    select: fn(u64, usize) -> Message,
    view: CurrentView,
    origin: SongsOrigin,
) {
    let _ = update(&mut player, select(epoch, 0));

    assert_view(&player, CurrentView::Songs);
    assert_eq!(player.songs_origin, origin);
    let ids: Vec<&str> = player
        .songs
        .items
        .iter()
        .map(|song| song.id.as_str())
        .collect();
    assert_eq!(ids, ["song-1", "song-3"]);

    let _ = update(&mut player, Message::Back);
    assert_view(&player, view);
    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Artists);
}

/// Asserts that a Show<Playlists|Genres|Years|Composers> press fetches that
/// list from the seeded library (whose fetch run is `load_call`), stores the
/// loaded rows under the view's own buffer, and switches to `view`, leaving
/// the browsed songs alone and any active search behind. `buffer` names the
/// list, `key` extracts a row's stored text, and `expected` lists those texts
/// in library order. Used by the four `show_*_fetches_and_switches_view`
/// tests; the flat All Songs and All Albums fetches seed more state first and
/// keep their own bodies.
pub(super) async fn assert_show_fetches_the_list_and_switches_view<T, Buffer, Key>(
    show: Message,
    load_call: &str,
    buffer: Buffer,
    key: Key,
    expected: &[&str],
    view: CurrentView,
) where
    Buffer: Fn(&mut WinampPlayer) -> &mut Vec<T>,
    Key: Fn(&T) -> &str,
{
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let task = update(&mut player, show);
    drive_fetch_and_assert_loaded(&mut player, task, load_call, buffer, key, expected).await;

    assert_view(&player, view);
    assert!(!player.search_active);
    assert_eq!(player.songs.items.len(), 3);
}

/// Asserts that a press on a Genres, Years, or Composers row rendered before
/// the list was replaced (`epoch - 1`), or past its last row, schedules nothing
/// and leaves the `view` showing and the loaded songs alone.
pub(super) fn assert_stale_group_press_does_nothing(
    mut player: WinampPlayer,
    epoch: u64,
    select: fn(u64, usize) -> Message,
    view: CurrentView,
) {
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    assert_message_schedules_no_work(&mut player, select(epoch - 1, 0));
    assert_message_schedules_no_work(&mut player, select(epoch, 9));

    assert_view(&player, view);
    assert_eq!(player.songs.items.len(), 3);
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

#[test]
fn show_all_songs_clears_an_active_search() {
    assert_navigation_clears_the_search(Message::ShowAllSongs, CurrentView::AllSongs);
}

// Reload replaces the artists list with a fresh fetch and returns there, but
// it is a library refresh rather than a reset: the typed search text, the
// other browse buffers, and playback must survive it. The fetch is the same
// clear-then-issue path Retry uses, so the stale rows and error go away and
// the loading placeholder shows until the reply lands.
#[tokio::test]
async fn reload_library_returns_to_artists_and_refetches_them() {
    let (mut player, state) = test_player();
    player.current_view = CurrentView::AllSongs;
    player.search_active = true;
    player.search_query = "echo".to_string();
    player.artists.items = vec![sample_artist()];
    player.artists.error = Some("music-library fetch failed: boom".to_string());
    player.artists.loading = false;
    player.all_songs.items = vec![sample_song()];
    let flat_song_id = player.all_songs.items[0].id.clone();
    {
        let mut shared = state.lock().await;
        shared.current_track = Some("song-1".to_string());
        shared.is_playing = true;
    }

    let task = update(&mut player, Message::ReloadLibrary);

    assert_view(&player, CurrentView::Artists);
    assert!(!player.search_active);
    assert_eq!(player.search_query, "echo");
    assert!(player.artists.items.is_empty());
    assert!(player.artists.loading);
    assert_eq!(player.artists.error, None);
    assert_eq!(player.all_songs.items.len(), 1);
    assert_eq!(player.all_songs.items[0].id, flat_song_id);
    {
        let shared = state.lock().await;
        assert_eq!(shared.current_track.as_deref(), Some("song-1"));
        assert!(shared.is_playing);
    }

    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "reload the library's artists",
        |player| &mut player.artists.items,
        |artist| artist.id.as_str(),
        &["artist-1", "artist-2", "artist-3"],
    )
    .await;
}

#[test]
fn back_from_all_songs_returns_to_the_artists_list() {
    assert_back_returns_to_artists(CurrentView::AllSongs);
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
    assert_show_fetches_the_list_and_switches_view(
        Message::ShowPlaylists,
        "load library playlists",
        |player| &mut player.playlists.items,
        |playlist| playlist.id.as_str(),
        &["playlist-1", "playlist-2", "playlist-3"],
        CurrentView::Playlists,
    )
    .await;
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

#[test]
fn show_playlists_clears_an_active_search() {
    assert_navigation_clears_the_search(Message::ShowPlaylists, CurrentView::Playlists);
}

#[test]
fn back_from_playlists_returns_to_the_artists_list() {
    assert_back_returns_to_artists(CurrentView::Playlists);
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

#[test]
fn show_all_albums_clears_an_active_search() {
    assert_navigation_clears_the_search(Message::ShowAllAlbums, CurrentView::AllAlbums);
}

#[test]
fn back_from_all_albums_returns_to_the_artists_list() {
    assert_back_returns_to_artists(CurrentView::AllAlbums);
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
        year: None,
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
fn all_albums_loaded_caches_the_year_and_running_time_labels() {
    // The cached row label must match `album_row_label`, so the sample
    // albums' release years show beside their derived running times.
    let (mut player, _state) = test_player();
    let mut albums: Vec<Album> = sample_library()
        .albums_by_artist
        .values()
        .flatten()
        .cloned()
        .collect();
    albums.sort_by(|a, b| a.id.cmp(&b.id));
    let _ = update(&mut player, Message::AllAlbumsLoaded(albums));
    assert_eq!(
        player.all_albums.labels,
        ["2001  10:30", "2003  1:35", "2005  3:20"]
    );
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

    // The flat list caches each row's display title and formatted running
    // time too, so the whole-library Albums view does not re-join and
    // reformat every row on each refresh.
    let mut album = sample_album();
    album.duration_ms = 95_000;
    let _ = update(&mut player, Message::AllAlbumsLoaded(vec![album]));
    assert_eq!(
        player.all_albums.titles,
        ["The Sample Band - First Record"],
        "the flat album row's display title is cached at store time"
    );
    assert_eq!(
        player.all_albums.labels,
        ["1:35"],
        "the flat album row's running time is cached at store time"
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
    let (recording, mut player, state) = player_with_recording();
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

/// Two zydeco songs (a genre with no EQ preset, so playing one leaves AUTO
/// alone) and one blank-genre song, in library order.
fn genre_songs() -> Vec<Song> {
    let mut songs = stepping_songs();
    songs[0].genre = "Zydeco".to_string();
    songs[1].genre = String::new();
    songs[2].genre = "zydeco".to_string();
    songs
}

/// A player showing the Genres list loaded from [`genre_songs`]: `Zydeco`
/// (songs 0 and 2), then `Unknown` (song 1).
fn player_with_genres() -> WinampPlayer {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::ShowGenres);
    let _ = update(
        &mut player,
        Message::GenresLoaded(group_by_genre(genre_songs())),
    );
    player
}

// The Genres button groups the library's songs by genre and shows them under
// their own view, leaving the other browse buffers alone.
#[tokio::test]
async fn show_genres_fetches_and_switches_view() {
    assert_show_fetches_the_list_and_switches_view(
        Message::ShowGenres,
        "load library genres",
        |player| &mut player.genres.items,
        |genre| genre.name.as_str(),
        &["Alternative", "Electronic", "Rock"],
        CurrentView::Genres,
    )
    .await;
}

#[test]
fn genres_loaded_stores_rows_with_song_count_labels() {
    let player = player_with_genres();

    let names: Vec<&str> = player
        .genres
        .items
        .iter()
        .map(|genre| genre.name.as_str())
        .collect();
    assert_eq!(names, ["Zydeco", "Unknown"]);
    assert_eq!(player.genres.labels, ["2 songs - 5:05", "1 song - 2:05"]);
    assert!(!player.genres.loading);
}

#[test]
fn show_genres_clears_an_active_search() {
    assert_navigation_clears_the_search(Message::ShowGenres, CurrentView::Genres);
}

#[test]
fn back_from_genres_returns_to_the_artists_list() {
    assert_back_returns_to_artists(CurrentView::Genres);
}

// Opening a genre shows exactly its songs under the Songs view, and Back
// steps to Genres and then to Artists.
#[test]
fn genre_selected_shows_that_genres_songs_and_back_returns_to_genres() {
    let player = player_with_genres();
    let epoch = player.genres.epoch;
    assert_group_opens_its_songs_and_back_unwinds(
        player,
        epoch,
        |epoch, index| Message::GenreSelected { epoch, index },
        CurrentView::Genres,
        SongsOrigin::Genre,
    );
}

#[test]
fn a_stale_genre_press_does_nothing() {
    let player = player_with_genres();
    let epoch = player.genres.epoch;
    assert_stale_group_press_does_nothing(
        player,
        epoch,
        |epoch, index| Message::GenreSelected { epoch, index },
        CurrentView::Genres,
    );
}

// A song in an opened genre plays through the ordinary TrackSelected path.
#[tokio::test]
async fn a_track_selected_in_a_genre_plays_it() {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::ShowGenres);
    let _ = update(
        &mut player,
        Message::GenresLoaded(group_by_genre(genre_songs())),
    );
    let epoch = player.genres.epoch;
    let _ = update(&mut player, Message::GenreSelected { epoch, index: 0 });
    let songs_epoch = player.songs.epoch;

    drive_task(
        update(
            &mut player,
            Message::TrackSelected {
                epoch: songs_epoch,
                index: 1,
            },
        ),
        "playback",
        |message| match message {
            Message::TrackPlayed { committed, .. } => assert!(committed),
            other => panic!("unexpected playback task output: {other:?}"),
        },
    )
    .await;

    assert_eq!(state.lock().await.current_track.as_deref(), Some("song-3"));
}

#[test]
fn genres_view_footer_counts_genres_and_totals_their_songs() {
    let player = player_with_genres();
    assert_eq!(player.current_view, CurrentView::Genres);

    let capture = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = capture.texts().collect();
    assert!(texts.contains(&"2 genres - 7:10"), "{texts:?}");
}
