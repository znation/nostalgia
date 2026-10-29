use super::*;

// "All favorite artists" is the service's content contract, not just a
// non-empty list: every artist, in library order. The albums, songs, and
// playlists queries each pin their exact group in order, but the artists
// query only asserted non-empty — so a regression that dropped an artist
// (e.g. a `.take(1)`) or reversed the list would pass every existing test
// while the browse view silently lost or reordered an artist. `assert_ids` is
// the same helper the other browse queries use.
#[tokio::test]
async fn get_favorite_artists_returns_all_artists_in_library_order() {
    let artists = test_service().get_favorite_artists().await.unwrap();
    assert_ids(
        &artists,
        |artist| artist.id.as_str(),
        &["artist-1", "artist-2", "artist-3"],
    );
}

#[tokio::test]
async fn get_albums_by_artist_returns_only_matching_albums() {
    let service = test_service();
    let albums = service.get_albums_by_artist("artist-1").await.unwrap();
    assert!(!albums.is_empty());
    assert!(albums.iter().all(|album| album.artist_id == "artist-1"));
    assert!(albums.iter().all(|album| album.artist_id != "artist-2"));
}

#[tokio::test]
async fn get_albums_by_artist_returns_all_matching_albums_in_library_order() {
    // The lookup table must keep every album of a multi-album artist and
    // preserve the library's original order: a regression that dropped
    // duplicates or reversed a group would still pass the all()-style
    // checks above, so pin the exact group here.
    let albums = test_service()
        .get_albums_by_artist("artist-1")
        .await
        .unwrap();
    assert_ids(&albums, |album| album.id.as_str(), &["album-1", "album-2"]);

    // An existing artist with no albums yields an empty list, distinct
    // from an unknown id (same lookup path, but worth pinning the sample).
    assert!(
        test_service()
            .get_albums_by_artist("artist-3")
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn get_albums_by_artist_unknown_id_is_empty() {
    let albums = test_service()
        .get_albums_by_artist("no-such-artist")
        .await
        .unwrap();
    assert!(albums.is_empty());
}

// A blank id is a caller bug, not an unknown artist: the seam rejects it
// with an error rather than the empty list an unknown id yields, so a
// future caller that drops the id gets a report naming the cause instead
// of a silent "no albums". The whitespace-only case goes through the same
// guard as the empty one.
#[tokio::test]
async fn get_albums_by_artist_rejects_a_blank_artist_id() {
    let service = test_service();
    assert_blank_id_rejected(
        service.get_albums_by_artist(""),
        service.get_albums_by_artist("   "),
        IdKind::Artist,
    )
    .await;
}

// The security twin of the blank-id rejection above: an artist id carrying
// a terminal control character — the shape a hostile Apple Music reply
// could supply — must be rejected rather than looked up. The guard's own
// test pins the check with kind "track"; this pins that the public browse
// query reaches it for artist ids, so a regression that dropped the
// control-character check from this path (or fell back to a blank-only
// guard) cannot ship silently.
#[tokio::test]
async fn get_albums_by_artist_rejects_a_control_character_artist_id() {
    let service = test_service();
    let id = "artist\u{1b}1";
    assert_control_character_id_rejected(service.get_albums_by_artist(id), IdKind::Artist, id)
        .await;
}

#[tokio::test]
async fn get_albums_by_artist_returns_single_album_artists_album() {
    // The empty (artist-3) and multi-album (artist-1) groups are pinned
    // above; artist-2's single-album group is the third branch of the
    // `index_by` grouping — a regression that dropped a one-element group
    // (e.g. only pushing after a second element arrives) would clear the
    // other two tests and silently lose this artist's album.
    let albums = test_service()
        .get_albums_by_artist("artist-2")
        .await
        .unwrap();
    assert_ids(&albums, |album| album.id.as_str(), &["album-3"]);
}

#[tokio::test]
async fn get_songs_from_album_returns_only_matching_songs() {
    let service = test_service();
    let songs = service.get_songs_from_album("album-1").await.unwrap();
    assert!(!songs.is_empty());
    assert!(songs.iter().all(|song| song.album_id == "album-1"));
}

#[tokio::test]
async fn get_songs_from_album_returns_all_songs_in_library_order() {
    // As with albums: a multi-song album must come back whole and in the
    // sample library's order, not a subset or a reversed group.
    let songs = test_service()
        .get_songs_from_album("album-1")
        .await
        .unwrap();
    assert_ids(
        &songs,
        |song| song.id.as_str(),
        &["song-1", "song-2", "song-3"],
    );
}

#[tokio::test]
async fn get_songs_from_album_unknown_id_is_empty() {
    let songs = test_service()
        .get_songs_from_album("no-such-album")
        .await
        .unwrap();
    assert!(songs.is_empty());
}

// The album-query twin of the blank-artist-id rejection above, whitespace
// case included.
#[tokio::test]
async fn get_songs_from_album_rejects_a_blank_album_id() {
    let service = test_service();
    assert_blank_id_rejected(
        service.get_songs_from_album(""),
        service.get_songs_from_album("   "),
        IdKind::Album,
    )
    .await;
}

// The album-query twin of the control-character artist-id rejection above.
#[tokio::test]
async fn get_songs_from_album_rejects_a_control_character_album_id() {
    let service = test_service();
    let id = "album\u{1b}1";
    assert_control_character_id_rejected(service.get_songs_from_album(id), IdKind::Album, id).await;
}

#[tokio::test]
async fn get_songs_from_album_returns_single_song_albums_song() {
    // album-1 (multi-song) and the unknown-id case are pinned above;
    // album-2's single-song group is the third branch — if the grouping
    // dropped one-element groups, this song would silently vanish.
    let songs = test_service()
        .get_songs_from_album("album-2")
        .await
        .unwrap();
    assert_ids(&songs, |song| song.id.as_str(), &["song-4"]);
}

#[tokio::test]
async fn get_playlists_returns_the_sample_playlists_in_order() {
    let playlists = test_service().get_playlists().await.unwrap();
    assert_ids(
        &playlists,
        |playlist| playlist.id.as_str(),
        &["playlist-1", "playlist-2", "playlist-3"],
    );
    assert_eq!(
        playlists
            .iter()
            .map(|playlist| playlist.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Morning Drive", "Focus", "Sample Favorites"]
    );
}

#[tokio::test]
async fn get_songs_from_playlist_returns_its_sample_songs_in_order() {
    // Playlist 1 interleaves album-1 and album-2 songs in its own named
    // order, so the exact ids distinguish the playlist lookup from an album
    // lookup.
    let songs = test_service()
        .get_songs_from_playlist("playlist-1")
        .await
        .unwrap();
    assert_ids(&songs, |song| song.id.as_str(), &["song-1", "song-4"]);
}

#[tokio::test]
async fn get_songs_from_playlist_returns_empty_for_an_unknown_playlist() {
    let songs = test_service()
        .get_songs_from_playlist("no-such-playlist")
        .await
        .unwrap();
    assert!(songs.is_empty());
}

// The playlist-query twin of the blank-album-id rejection above,
// whitespace case included.
#[tokio::test]
async fn get_songs_from_playlist_rejects_a_blank_id() {
    let service = test_service();
    assert_blank_id_rejected(
        service.get_songs_from_playlist(""),
        service.get_songs_from_playlist("   "),
        IdKind::Playlist,
    )
    .await;
}

// The playlist-query control-character twin of the album test above.
#[tokio::test]
async fn get_songs_from_playlist_rejects_a_control_character_id() {
    let service = test_service();
    let id = "playlist\u{1b}1";
    assert_control_character_id_rejected(service.get_songs_from_playlist(id), IdKind::Playlist, id)
        .await;
}

// The library search is the UI search box's backend: it answers with the
// sample songs whose title or artist contains the query, case-insensitively,
// so a user can find a track by either field without matching the exact case.
// The album-title match and the genre/composer match have their own tests.
#[tokio::test]
async fn search_songs_matches_the_sample_library_by_title_or_artist() {
    let service = test_service();

    // A title-only match, case-insensitively, with surrounding whitespace
    // trimmed off the query.
    let by_title = service.search_songs("  OPENING  ").await.unwrap();
    assert_ids(&by_title, |song| song.id.as_str(), &["song-1"]);

    // An artist-only match: "song-5" is the only Echo Chamber track.
    let by_artist = service.search_songs("echo").await.unwrap();
    assert_ids(&by_artist, |song| song.id.as_str(), &["song-5"]);

    // A match spanning two albums of one artist comes back in library order,
    // not the `songs_by_album` map's unspecified iteration order.
    let band = service.search_songs("sample band").await.unwrap();
    assert_ids(
        &band,
        |song| song.id.as_str(),
        &["song-1", "song-2", "song-3", "song-4"],
    );

    // A query matching nothing is an empty list, not an error.
    let none = service.search_songs("no such track").await.unwrap();
    assert!(none.is_empty());
}

// The search also finds a song by the title of the album it belongs to, so
// typing an album name the browse lists show returns that album's songs.
#[tokio::test]
async fn search_songs_matches_the_sample_library_by_album_title() {
    let service = test_service();

    // "song-4" is the only track on "Second Record"; the case and surrounding
    // whitespace are folded and trimmed like the other matches.
    let by_album = service.search_songs("  SECOND RECORD  ").await.unwrap();
    assert_ids(&by_album, |song| song.id.as_str(), &["song-4"]);

    // Every song on "First Record" comes back in library order.
    let first = service.search_songs("first record").await.unwrap();
    assert_ids(
        &first,
        |song| song.id.as_str(),
        &["song-1", "song-2", "song-3"],
    );
}

// The search also finds a song by its genre or composer, so typing a name the
// Genres and Composers lists show returns that entry's songs offline.
#[tokio::test]
async fn search_songs_matches_the_sample_library_by_genre_or_composer() {
    let service = test_service();

    let rock = service.search_songs("  ROCK  ").await.unwrap();
    assert_ids(
        &rock,
        |song| song.id.as_str(),
        &["song-1", "song-2", "song-3"],
    );

    let electronic = service.search_songs("electronic").await.unwrap();
    assert_ids(&electronic, |song| song.id.as_str(), &["song-5"]);

    let composer = service.search_songs("sam ample").await.unwrap();
    assert_ids(
        &composer,
        |song| song.id.as_str(),
        &["song-1", "song-2", "song-3"],
    );

    let alternative = service.search_songs("alternative").await.unwrap();
    assert_ids(&alternative, |song| song.id.as_str(), &["song-4"]);
}

// The search also finds a song by its release year, written as a plain number.
#[tokio::test]
async fn search_songs_matches_the_sample_library_by_release_year() {
    let service = test_service();

    let first = service.search_songs("  2001  ").await.unwrap();
    assert_ids(
        &first,
        |song| song.id.as_str(),
        &["song-1", "song-2", "song-3"],
    );

    let b_side = service.search_songs("2003").await.unwrap();
    assert_ids(&b_side, |song| song.id.as_str(), &["song-4"]);

    let last = service.search_songs("2005").await.unwrap();
    assert_ids(&last, |song| song.id.as_str(), &["song-5"]);

    let decade = service.search_songs("200").await.unwrap();
    assert_ids(
        &decade,
        |song| song.id.as_str(),
        &["song-1", "song-2", "song-3", "song-4", "song-5"],
    );

    assert!(service.search_songs("1999").await.unwrap().is_empty());
}

// The All Songs backend returns the whole sample library, flat and in
// library order: a regression that dropped a song or shuffled the albums
// would leave the flat view incomplete even though every other browse query
// still passed.
#[tokio::test]
async fn get_all_songs_returns_every_sample_song_in_library_order() {
    let songs = test_service().get_all_songs().await.unwrap();

    assert_ids(
        &songs,
        |song| song.id.as_str(),
        &["song-1", "song-2", "song-3", "song-4", "song-5"],
    );
}

// The All Albums backend returns every sample-library album, flat and in
// library order: the ordered artists -> albums walk, not the unordered
// album index. A regression that dropped an album or shuffled the artists
// would leave the flat album view incomplete even though every other browse
// query still passed.
#[tokio::test]
async fn get_all_albums_returns_every_sample_album_in_library_order() {
    let albums = test_service().get_all_albums().await.unwrap();

    assert_ids(
        &albums,
        |album| album.id.as_str(),
        &["album-1", "album-2", "album-3"],
    );
}

// A blank search has nothing to match; returning the whole library (or an
// empty list) would report the caller's empty input as an ordinary result.
// The error names the offending query, whitespace included.
#[tokio::test]
async fn search_songs_rejects_a_blank_query() {
    let service = test_service();

    for query in ["", "   "] {
        let error = service.search_songs(query).await.unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("search query must not be blank"),
            "unexpected message: {message}"
        );
        assert!(
            message.contains(&format!("{query:?}")),
            "the error must name the query: {message}"
        );
    }
}

// With no added dates in the sample library, Recently Added is the library order reversed.
#[tokio::test]
async fn get_recently_added_albums_returns_the_sample_albums_reversed() {
    let service = test_service();
    let mut expected = service.get_all_albums().await.unwrap();
    expected.reverse();

    let albums = service.get_recently_added_albums().await.unwrap();

    assert_eq!(albums, expected);
}

// With no play history in the sample library, Recently Played is the library order reversed.
#[tokio::test]
async fn get_recently_played_songs_returns_the_sample_songs_reversed() {
    let service = test_service();
    let mut expected = service.get_all_songs().await.unwrap();
    expected.reverse();

    let songs = service.get_recently_played_songs().await.unwrap();

    assert_eq!(songs, expected);
}
