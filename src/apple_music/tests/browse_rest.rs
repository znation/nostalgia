use super::*;

// The Playlists view routes through the REST client when a session is stored,
// so a signed-in user sees their real playlists rather than the sample ones.
// The stub returns a playlist whose id is not a sample id, so the result can
// only have come from the REST collection; the URL pins the endpoint.
#[tokio::test]
async fn get_playlists_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"rest-playlist","attributes":{"name":"Remote Mix"}}]}"#,
    );
    let service = signed_in_service(&stub);

    let playlists = service.get_playlists().await.unwrap();

    assert_eq!(
        playlists,
        vec![Playlist {
            id: "rest-playlist".to_string(),
            name: "Remote Mix".to_string(),
        }]
    );
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/playlists");
}

// A signed-in playlists load has no sample-library fallback: it answers from
// the sample library only when no session is stored, so a failed request must
// surface the error for the UI's Retry path. The sample library has playlists,
// so a fallback on failure would return them instead of the error.
#[tokio::test]
async fn get_playlists_fails_when_the_playlists_request_fails() {
    let stub = StubTransport::failing("playlists down");
    let service = signed_in_service(&stub);

    let error = service.get_playlists().await.unwrap_err();

    assert_eq!(error.to_string(), "playlists down");
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/playlists");
}

// Opening a playlist routes through the REST client when a session is stored,
// so a signed-in user browses the real playlist's tracks. The stub returns a
// song whose id is not a sample id, so the result can only have come from the
// REST collection; the URL pins the id-scoped endpoint.
#[tokio::test]
async fn get_songs_from_playlist_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning(&rest_song_data());
    let service = signed_in_service(&stub);

    let songs = service.get_songs_from_playlist("playlist-9").await.unwrap();

    assert_eq!(songs, vec![rest_song()]);
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/playlists/playlist-9/tracks",
    );
}

// A signed-in playlist load has no sample-library fallback: it answers from
// the sample library only when no session is stored, so a failed tracks
// request must surface the error for the UI's Retry path. The id is a sample
// playlist's, so a fallback on failure would have sample songs to return
// instead of the error.
#[tokio::test]
async fn get_songs_from_playlist_fails_when_the_tracks_request_fails() {
    let stub = StubTransport::failing("playlist tracks down");
    let service = signed_in_service(&stub);

    let error = service
        .get_songs_from_playlist("playlist-1")
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "playlist tracks down");
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/playlists/playlist-1/tracks",
    );
}

#[tokio::test]
async fn get_favorite_artists_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}]}"#,
        EMPTY_DATA_BODY,
    ]);
    let service = signed_in_service(&stub);

    let artists = service.get_favorite_artists().await.unwrap();

    assert_eq!(artists, vec![sample_artist()]);
    assert_call_urls(
        &stub,
        &[
            "https://api.music.apple.com/v1/me/library/artists",
            "https://api.music.apple.com/v1/me/library/albums",
        ],
    );
}

#[tokio::test]
async fn get_favorite_artists_counts_albums_by_artist_name_from_the_rest_library() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[
            {"id":"artist-1","attributes":{"name":"The Sample Band"}},
            {"id":"artist-2","attributes":{"name":"Echo Chamber"}}
        ]}"#,
        r#"{"data":[
            {"id":"album-1","attributes":{"name":"First Record","artistName":"The Sample Band"}},
            {"id":"album-2","attributes":{"name":"Second Record","artistName":"The Sample Band"}},
            {"id":"album-3","attributes":{"name":"Debut","artistName":"Echo Chamber"}},
            {"id":"album-4","attributes":{"name":"Unknown Artist Record"}}
        ]}"#,
    ]);
    let service = signed_in_service(&stub);

    let artists = service.get_favorite_artists().await.unwrap();

    assert_eq!(
        artists,
        vec![
            Artist {
                id: "artist-1".to_string(),
                name: "The Sample Band".to_string(),
                album_count: 2,
            },
            Artist {
                id: "artist-2".to_string(),
                name: "Echo Chamber".to_string(),
                album_count: 1,
            },
        ]
    );
    assert_call_urls(
        &stub,
        &[
            "https://api.music.apple.com/v1/me/library/artists",
            "https://api.music.apple.com/v1/me/library/albums",
        ],
    );
}

// A signed-in account with an empty library is an ordinary empty result, not
// the sample fallback: the sample library answers only when no session is
// stored, so a regression that treated "no artists came back" as "fall back
// to the samples" would show a signed-in user music they do not own. The
// second call pins that the albums collection is still read for the count,
// and both empty bodies prove the load answers with an empty list rather
// than an error.
#[tokio::test]
async fn get_favorite_artists_returns_no_artists_for_an_empty_signed_in_library() {
    let stub = StubTransport::returning_bodies(&[EMPTY_DATA_BODY, EMPTY_DATA_BODY]);
    let service = signed_in_service(&stub);

    let artists = service.get_favorite_artists().await.unwrap();

    assert!(
        artists.is_empty(),
        "an empty signed-in library must not fall back to the sample artists: {artists:?}"
    );
    assert_call_urls(
        &stub,
        &[
            "https://api.music.apple.com/v1/me/library/artists",
            "https://api.music.apple.com/v1/me/library/albums",
        ],
    );
}

// The empty signed-in library is an ordinary empty result for the whole-library
// queries too, not the sample fallback: a regression that treated an empty
// collection as "no session" would show a signed-in user playlists they do not
// own. The pinned URL proves the answer came from the signed-in collection.
#[tokio::test]
async fn get_playlists_returns_no_playlists_for_an_empty_signed_in_library() {
    let stub = StubTransport::returning(EMPTY_DATA_BODY);
    let service = signed_in_service(&stub);

    let playlists = service.get_playlists().await.unwrap();

    assert!(
        playlists.is_empty(),
        "an empty signed-in library must not fall back to the sample playlists: {playlists:?}"
    );
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/playlists");
}

// The All Songs twin: the sample library has songs, so an empty signed-in
// collection that fell back would fill the flat view with music the user does
// not own. The URL pins the signed-in songs collection as the source.
#[tokio::test]
async fn get_all_songs_returns_no_songs_for_an_empty_signed_in_library() {
    let stub = StubTransport::returning(EMPTY_DATA_BODY);
    let service = signed_in_service(&stub);

    let songs = service.get_all_songs().await.unwrap();

    assert!(
        songs.is_empty(),
        "an empty signed-in library must not fall back to the sample songs: {songs:?}"
    );
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/songs");
}

// The All Albums twin: the load reads the album collection and then the songs
// collection (for the running times), so both empty bodies prove an empty
// signed-in library stays empty rather than falling back to the sample albums.
// Both calls are pinned so a regression cannot skip the durations request.
#[tokio::test]
async fn get_all_albums_returns_no_albums_for_an_empty_signed_in_library() {
    let stub = StubTransport::returning_bodies(&[EMPTY_DATA_BODY, EMPTY_DATA_BODY]);
    let service = signed_in_service(&stub);

    let albums = service.get_all_albums().await.unwrap();

    assert!(
        albums.is_empty(),
        "an empty signed-in library must not fall back to the sample albums: {albums:?}"
    );
    assert_call_urls(
        &stub,
        &[
            "https://api.music.apple.com/v1/me/library/albums",
            "https://api.music.apple.com/v1/me/library/songs",
        ],
    );
}

// The per-artist Albums view reads its albums and then the whole-library
// songs collection for the running times. An empty signed-in album result is
// an ordinary empty result, not the sample fallback: `artist-1` has two sample
// albums, so a regression that fell back on an empty result would return them.
// The second call pins that the durations request still runs for an empty
// album list, matching the documented load.
#[tokio::test]
async fn get_albums_by_artist_returns_no_albums_for_an_empty_signed_in_result() {
    let stub = StubTransport::returning_bodies(&[
        EMPTY_DATA_BODY,
        r#"{"data":[{"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","albumName":"First Record","durationInMillis":210000}}]}"#,
    ]);
    let service = signed_in_service(&stub);

    let albums = service.get_albums_by_artist("artist-1").await.unwrap();

    assert!(
        albums.is_empty(),
        "an empty signed-in album result must not fall back to the sample albums: {albums:?}"
    );
    assert_call_urls(
        &stub,
        &[
            "https://api.music.apple.com/v1/me/library/artists/artist-1/albums",
            "https://api.music.apple.com/v1/me/library/songs",
        ],
    );
}

// The per-album Songs view's empty signed-in result is an ordinary empty
// result, not the sample fallback: `album-1` has three sample songs, so a
// regression that treated "no tracks came back" as "fall back to the samples"
// would show a signed-in user music they do not own. The pinned URL proves the
// answer came from the album's tracks endpoint.
#[tokio::test]
async fn get_songs_from_album_returns_no_songs_for_an_empty_signed_in_result() {
    let stub = StubTransport::returning(EMPTY_DATA_BODY);
    let service = signed_in_service(&stub);

    let songs = service.get_songs_from_album("album-1").await.unwrap();

    assert!(
        songs.is_empty(),
        "an empty signed-in album result must not fall back to the sample songs: {songs:?}"
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/albums/album-1/tracks",
    );
}

// The per-playlist twin of the album test above: `playlist-1` has two sample
// songs, so an empty signed-in playlist result that fell back would return
// them. The URL pins the playlist's tracks endpoint as the source.
#[tokio::test]
async fn get_songs_from_playlist_returns_no_songs_for_an_empty_signed_in_result() {
    let stub = StubTransport::returning(EMPTY_DATA_BODY);
    let service = signed_in_service(&stub);

    let songs = service.get_songs_from_playlist("playlist-1").await.unwrap();

    assert!(
        songs.is_empty(),
        "an empty signed-in playlist result must not fall back to the sample songs: {songs:?}"
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/playlists/playlist-1/tracks",
    );
}

#[tokio::test]
async fn get_albums_by_artist_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[{"id":"album-1","attributes":{"name":"First Record","artistName":"The Sample Band"}}]}"#,
        r#"{"data":[{"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","albumName":"First Record","durationInMillis":210000}}]}"#,
    ]);
    let service = signed_in_service(&stub);

    let albums = service.get_albums_by_artist("artist-9").await.unwrap();

    assert_eq!(
        albums,
        vec![Album {
            artist_id: "artist-9".to_string(),
            duration_ms: 210_000,
            ..sample_album()
        }]
    );
    assert_call_urls(
        &stub,
        &[
            "https://api.music.apple.com/v1/me/library/artists/artist-9/albums",
            "https://api.music.apple.com/v1/me/library/songs",
        ],
    );
}

// The documented guarantee is that a failure in *either* signed-in request
// fails the whole album load. The durations request is pinned below; this
// covers the artist's album request itself. The songs body is left
// successful, so a regression that swallowed the album error (say with
// `unwrap_or_default`) would return an empty album list instead of an error,
// and the single recorded call pins that the songs request is not attempted
// after the first failure.
#[tokio::test]
async fn get_albums_by_artist_fails_when_the_albums_request_fails() {
    let stub = StubTransport::returning_results(&[
        Err(AppleMusicError::new("albums down")),
        Ok(
            r#"{"data":[{"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","albumName":"First Record","durationInMillis":210000}}]}"#
                .to_string(),
        ),
    ]);
    let service = signed_in_service(&stub);

    let error = service.get_albums_by_artist("artist-9").await.unwrap_err();

    assert_eq!(error.to_string(), "albums down");
    let calls = stub.calls();
    assert_eq!(
        calls.len(),
        1,
        "the songs request must not follow a failed album request"
    );
    assert_eq!(
        calls[0].0,
        "https://api.music.apple.com/v1/me/library/artists/artist-9/albums"
    );
}

// The per-artist albums load reads the whole-library songs collection after
// the artist's album list so it can fill in each row's running time. A
// failure there must fail the whole load (the Retry path) instead of
// quietly leaving every duration at zero, which the row would render as the
// `--:--` placeholder. The All Albums twin is already pinned; this covers
// the same guarantee on the Albums view's own query.
#[tokio::test]
async fn get_albums_by_artist_fails_when_the_album_durations_request_fails() {
    let stub = StubTransport::returning_results(&[
        Ok(
            r#"{"data":[{"id":"album-1","attributes":{"name":"First Record","artistName":"The Sample Band"}}]}"#
                .to_string(),
        ),
        Err(AppleMusicError::new("songs down")),
    ]);
    let service = signed_in_service(&stub);

    let error = service.get_albums_by_artist("artist-9").await.unwrap_err();

    assert_eq!(error.to_string(), "songs down");
    assert_call_urls(
        &stub,
        &[
            "https://api.music.apple.com/v1/me/library/artists/artist-9/albums",
            "https://api.music.apple.com/v1/me/library/songs",
        ],
    );
}

#[tokio::test]
async fn get_songs_from_album_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","durationInMillis":210000}}]}"#,
    );
    let service = signed_in_service(&stub);

    let songs = service.get_songs_from_album("album-9").await.unwrap();

    assert_eq!(
        songs,
        vec![Song {
            album: String::new(),
            album_id: "album-9".to_string(),
            ..sample_song()
        }]
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/albums/album-9/tracks",
    );
}

// A signed-in album load has no sample-library fallback: it answers from the
// sample library only when no session is stored, so a failed tracks request
// must surface the error for the UI's Retry path. The id is a sample album's,
// so a fallback on failure would have sample songs to return instead of the
// error.
#[tokio::test]
async fn get_songs_from_album_fails_when_the_tracks_request_fails() {
    let stub = StubTransport::failing("tracks down");
    let service = signed_in_service(&stub);

    let error = service.get_songs_from_album("album-1").await.unwrap_err();

    assert_eq!(error.to_string(), "tracks down");
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/albums/album-1/tracks",
    );
}

// The search box's backend routes through the REST client when a session is
// stored, so a signed-in user searches the real library rather than the
// sample one. The stub returns a song whose id is not a sample id, so the
// result can only have come from the REST search envelope; the URL pins the
// search endpoint, the term, and the documented page-size maximum.
#[tokio::test]
async fn search_songs_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning(&rest_search_song_data());
    let service = signed_in_service(&stub);

    let songs = service.search_songs("remote").await.unwrap();

    assert_eq!(songs, vec![rest_song()]);
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/search?term=remote&types=library-songs&limit=25",
    );
}

// A signed-in search has no sample-library fallback: it answers from the
// sample library only when no session is stored, so a failed REST search must
// surface the error for the UI's Retry path. The query matches a sample title
// case-insensitively, so a fallback on failure would return that song instead
// of the error. The surrounding whitespace is trimmed from the term the
// request carries, as the signed-out sample path trims it for matching.
#[tokio::test]
async fn search_songs_fails_when_the_rest_search_fails() {
    let stub = StubTransport::failing("search down");
    let service = signed_in_service(&stub);

    let error = service.search_songs("  OPENING  ").await.unwrap_err();

    assert_eq!(error.to_string(), "search down");
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/search?term=OPENING&types=library-songs&limit=25",
    );
}

// A signed-in search that matched nothing is an ordinary empty result, not a
// reason to fall back to the sample library: the term matches a sample title
// case-insensitively, so a regression that treated an empty REST result as
// "fall back" would return "Opening" for a signed-in user. Apple omits the
// `library-songs` key for a no-match search, so this also drives the
// missing-key branch of the search envelope through the public query. The URL
// pins the trimmed term and the search endpoint, proving the answer came from
// the REST search rather than the in-memory samples.
#[tokio::test]
async fn search_songs_returns_no_songs_for_an_empty_signed_in_result() {
    let stub = StubTransport::returning(r#"{"results":{}}"#);
    let service = signed_in_service(&stub);

    let songs = service.search_songs("  opening  ").await.unwrap();

    assert!(
        songs.is_empty(),
        "an empty signed-in search must not fall back to the sample songs: {songs:?}"
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/search?term=opening&types=library-songs&limit=25",
    );
}

// The All Songs view routes through the REST client when a session is stored,
// so a signed-in user sees the real library rather than the sample one. The
// stub returns a song whose id is not a sample id, so the result can only
// have come from the REST collection; the URL pins the endpoint.
#[tokio::test]
async fn get_all_songs_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning(&rest_song_data());
    let service = signed_in_service(&stub);

    let songs = service.get_all_songs().await.unwrap();

    assert_eq!(songs, vec![rest_song()]);
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/songs");
}

// A signed-in All Songs load has no sample-library fallback: it answers from
// the sample library only when no session is stored, so a failed collection
// request must surface the error for the UI's Retry path. The sample library
// has songs, so a fallback on failure would return them instead of the error.
#[tokio::test]
async fn get_all_songs_fails_when_the_songs_request_fails() {
    let stub = StubTransport::failing("songs down");
    let service = signed_in_service(&stub);

    let error = service.get_all_songs().await.unwrap_err();

    assert_eq!(error.to_string(), "songs down");
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/songs");
}

// The All Albums view routes through the REST client when a session is
// stored, so a signed-in user sees the real library rather than the sample
// one. The stub returns an album whose id is not a sample id, so the result
// can only have come from the REST collection; the URL pins the endpoint.
#[tokio::test]
async fn get_all_albums_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[{"id":"rest-album","attributes":{"name":"Remote Record","artistName":"The Sample Band"}}]}"#,
        r#"{"data":[
            {"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","albumName":"Remote Record","durationInMillis":210000}},
            {"id":"song-2","attributes":{"name":"Middle","artistName":"The Sample Band","albumName":"Remote Record","durationInMillis":1000}}
        ]}"#,
    ]);
    let service = signed_in_service(&stub);

    let albums = service.get_all_albums().await.unwrap();

    assert_eq!(
        albums,
        vec![Album {
            id: "rest-album".to_string(),
            title: "Remote Record".to_string(),
            artist_id: String::new(),
            artist: "The Sample Band".to_string(),
            duration_ms: 211_000,
            year: None,
        }]
    );
    assert_call_urls(
        &stub,
        &[
            "https://api.music.apple.com/v1/me/library/albums",
            "https://api.music.apple.com/v1/me/library/songs",
        ],
    );
}

// The signed-in all-albums load reads the songs collection after the album
// list so it can total each album's running time. A failure there must fail
// the whole load (the Retry path) instead of quietly leaving every duration
// at zero.
#[tokio::test]
async fn get_all_albums_fails_when_the_album_durations_request_fails() {
    let stub = StubTransport::returning_results(&[
        Ok(
            r#"{"data":[{"id":"rest-album","attributes":{"name":"Remote Record","artistName":"The Sample Band"}}]}"#
                .to_string(),
        ),
        Err(AppleMusicError::new("songs down")),
    ]);
    let service = signed_in_service(&stub);

    let error = service.get_all_albums().await.unwrap_err();

    assert_eq!(error.to_string(), "songs down");
    let calls = stub.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[1].0,
        "https://api.music.apple.com/v1/me/library/songs"
    );
}

// The documented guarantee is that a failure in *either* signed-in request
// fails the whole album load. The durations request is pinned above; this
// covers the albums collection itself. The songs body is left successful, so
// a regression that swallowed the album error (say with `unwrap_or_default`)
// would return an empty album list instead of an error, and the single
// recorded call pins that the songs request is not attempted after the first
// failure.
#[tokio::test]
async fn get_all_albums_fails_when_the_albums_request_fails() {
    let stub = StubTransport::returning_results(&[
        Err(AppleMusicError::new("albums down")),
        Ok(
            r#"{"data":[{"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","albumName":"Remote Record","durationInMillis":210000}}]}"#
                .to_string(),
        ),
    ]);
    let service = signed_in_service(&stub);

    let error = service.get_all_albums().await.unwrap_err();

    assert_eq!(error.to_string(), "albums down");
    let calls = stub.calls();
    assert_eq!(
        calls.len(),
        1,
        "the songs request must not follow a failed albums request"
    );
    assert_eq!(
        calls[0].0,
        "https://api.music.apple.com/v1/me/library/albums"
    );
}

#[tokio::test]
async fn a_transport_error_propagates_from_a_signed_in_browse_query() {
    let service = signed_in_service(&StubTransport::failing("network down"));

    let error = service.get_favorite_artists().await.unwrap_err();

    assert_eq!(error.to_string(), "network down");
}

// The signed-in artists load reads the albums collection after the artist
// list so it can count them. A failure there must fail the whole load (the
// Retry path) instead of quietly leaving every count at zero.
#[tokio::test]
async fn get_favorite_artists_fails_when_the_albums_request_fails() {
    let stub = StubTransport::returning_results(&[
        Ok(r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}]}"#.to_string()),
        Err(AppleMusicError::new("albums down")),
    ]);
    let service = signed_in_service(&stub);

    let error = service.get_favorite_artists().await.unwrap_err();

    assert_eq!(error.to_string(), "albums down");
    let calls = stub.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[1].0,
        "https://api.music.apple.com/v1/me/library/albums"
    );
}

// The documented guarantee is that a failure in either signed-in request
// fails the whole artists load. The albums request is pinned above; this
// covers the artist collection itself. The albums body is left successful, so
// a regression that swallowed the artist error (say with `unwrap_or_default`)
// would attempt it and return an empty artist list instead of an error. The
// single recorded call also pins that the albums request is not attempted
// after the first failure.
#[tokio::test]
async fn get_favorite_artists_fails_when_the_artists_request_fails() {
    let stub = StubTransport::returning_results(&[
        Err(AppleMusicError::new("artists down")),
        Ok(EMPTY_DATA_BODY.to_string()),
    ]);
    let service = signed_in_service(&stub);

    let error = service.get_favorite_artists().await.unwrap_err();

    assert_eq!(error.to_string(), "artists down");
    let calls = stub.calls();
    assert_eq!(
        calls.len(),
        1,
        "the albums request must not follow a failed artists request"
    );
    assert_eq!(
        calls[0].0,
        "https://api.music.apple.com/v1/me/library/artists"
    );
}

#[tokio::test]
async fn a_signed_out_service_ignores_the_transport_and_returns_sample_data() {
    let stub = StubTransport::failing("must not be called");
    let service = AppleMusicService::with_transport(
        Arc::new(Mutex::new(AppState::default())),
        Box::new(stub.clone()),
    );

    assert_eq!(
        service.get_favorite_artists().await.unwrap(),
        sample_library().artists.clone()
    );
    assert_eq!(
        service.get_albums_by_artist("artist-1").await.unwrap(),
        sample_library().albums_by_artist["artist-1"].clone()
    );
    assert_eq!(
        service.get_songs_from_album("album-1").await.unwrap(),
        sample_library().songs_by_album["album-1"].clone()
    );
    assert!(stub.calls().is_empty());
}

// The documented guarantee is that an artist whose name matches no album
// gets a count of 0. Both artists in the test above match at least one
// album, so this pins the missing-key fallback itself.
#[tokio::test]
async fn get_favorite_artists_gives_an_artist_with_no_matching_album_a_count_of_zero() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[{"id":"artist-1","attributes":{"name":"Mono Tones"}}]}"#,
        r#"{"data":[{"id":"album-1","attributes":{"name":"First Record","artistName":"The Sample Band"}}]}"#,
    ]);
    let service = signed_in_service(&stub);

    let artists = service.get_favorite_artists().await.unwrap();

    assert_eq!(
        artists,
        vec![Artist {
            id: "artist-1".to_string(),
            name: "Mono Tones".to_string(),
            album_count: 0,
        }]
    );
    let calls = stub.calls();
    assert_eq!(calls.len(), 2);
}

// The songs are grouped by the exact `(album title, artist display name)`
// pair, and an album no song names keeps the `0` the row renders as `--:--`.
// Two albums sharing a title but not an artist must not pick up each other's
// running time, and the unmatched one must not fail or borrow the total: the
// map miss is the placeholder path, not an error.
#[tokio::test]
async fn get_all_albums_leaves_an_album_without_matching_songs_at_zero_duration() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[
            {"id":"album-one","attributes":{"name":"Shared Title","artistName":"Artist One"}},
            {"id":"album-two","attributes":{"name":"Shared Title","artistName":"Artist Two"}}
        ]}"#,
        r#"{"data":[
            {"id":"song-1","attributes":{"name":"Only Track","artistName":"Artist One","albumName":"Shared Title","durationInMillis":210000}}
        ]}"#,
    ]);
    let service = signed_in_service(&stub);

    let albums = service.get_all_albums().await.unwrap();

    assert_eq!(albums.len(), 2);
    assert_eq!(albums[0].id, "album-one");
    assert_eq!(
        albums[0].duration_ms, 210_000,
        "an album whose title and artist both match a song takes its total"
    );
    assert_eq!(albums[1].id, "album-two");
    assert_eq!(
        albums[1].duration_ms, 0,
        "a same-title album by another artist has no matching songs and keeps the placeholder duration"
    );
}

// The input guards run before the signed-in branch: a blank or
// control-character id, or a blank search query, is a caller bug that must be
// rejected without spending a request on it. The stub would answer any call,
// so a guard moved behind the session check would return data (or record a
// call) instead of the error.
#[tokio::test]
async fn a_signed_in_service_rejects_bad_input_without_sending_a_request() {
    let stub = StubTransport::returning(r#"{"data":[]}"#);
    let service = signed_in_service(&stub);

    assert!(service.get_albums_by_artist("  ").await.is_err());
    assert!(service.get_albums_by_artist("a\u{7}b").await.is_err());
    assert!(service.get_songs_from_album("").await.is_err());
    assert!(service.get_songs_from_playlist("\n").await.is_err());
    let error = service.search_songs("   ").await.unwrap_err();
    assert!(error.to_string().contains("must not be blank"));

    assert!(stub.calls().is_empty(), "calls: {:?}", stub.calls());
}

// Recently Added reads the `recently-added` endpoint, then the songs collection for running
// times, like All Albums.
#[tokio::test]
async fn get_recently_added_albums_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[{"id":"rest-album","type":"library-albums","attributes":{"name":"Remote Record","artistName":"The Sample Band"}}]}"#,
        r#"{"data":[
            {"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","albumName":"Remote Record","durationInMillis":210000}},
            {"id":"song-2","attributes":{"name":"Middle","artistName":"The Sample Band","albumName":"Remote Record","durationInMillis":1000}}
        ]}"#,
    ]);
    let service = signed_in_service(&stub);

    let albums = service.get_recently_added_albums().await.unwrap();

    assert_eq!(
        albums,
        vec![Album {
            id: "rest-album".to_string(),
            title: "Remote Record".to_string(),
            artist_id: String::new(),
            artist: "The Sample Band".to_string(),
            duration_ms: 211_000,
            year: None,
        }]
    );
    assert_call_urls(
        &stub,
        &[
            "https://api.music.apple.com/v1/me/library/recently-added",
            "https://api.music.apple.com/v1/me/library/songs",
        ],
    );
}

#[tokio::test]
async fn get_recently_added_albums_fails_when_the_album_durations_request_fails() {
    let stub = StubTransport::returning_results(&[
        Ok(
            r#"{"data":[{"id":"rest-album","type":"library-albums","attributes":{"name":"Remote Record"}}]}"#
                .to_string(),
        ),
        Err(AppleMusicError::new("songs down")),
    ]);
    let service = signed_in_service(&stub);

    let error = service.get_recently_added_albums().await.unwrap_err();

    assert_eq!(error.to_string(), "songs down");
}

#[tokio::test]
async fn get_recently_played_songs_uses_the_rest_library_when_signed_in() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"song-1","type":"library-songs","attributes":{"name":"Opening","artistName":"The Sample Band","albumName":"Remote Record","durationInMillis":210000}}]}"#,
    );
    let service = signed_in_service(&stub);

    let songs = service.get_recently_played_songs().await.unwrap();

    assert_eq!(songs.len(), 1);
    assert_eq!(songs[0].title, "Opening");
    assert_call_urls(
        &stub,
        &["https://api.music.apple.com/v1/me/recent/played/tracks"],
    );
}
