use super::*;
use std::sync::{Arc, Mutex};

/// A session whose tokens are recognizable, so a test can assert the transport
/// saw exactly these credentials.
fn session() -> MusicKitSession {
    MusicKitSession {
        developer_token: "developer-token".to_string(),
        user_token: "user-token".to_string(),
    }
}

/// A transport stub: it records every `(url, session)` it is handed and returns
/// the same canned result to each call. Cloning it shares the recording, so a
/// test keeps a handle after [`RestLibrary::new`] boxes the clone.
#[derive(Clone)]
struct StubTransport {
    result: Result<String, AppleMusicError>,
    calls: Arc<Mutex<Vec<(String, MusicKitSession)>>>,
}

impl StubTransport {
    /// A stub that answers every request with `body`.
    fn returning(body: &str) -> Self {
        Self {
            result: Ok(body.to_string()),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// A stub that fails every request with `message`.
    fn failing(message: &str) -> Self {
        Self {
            result: Err(AppleMusicError::new(message)),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// The requests recorded so far, oldest first.
    fn calls(&self) -> Vec<(String, MusicKitSession)> {
        self.calls.lock().unwrap().clone()
    }
}

impl HttpTransport for StubTransport {
    fn get(&self, url: &str, session: &MusicKitSession) -> Result<String, AppleMusicError> {
        self.calls
            .lock()
            .unwrap()
            .push((url.to_string(), session.clone()));
        self.result.clone()
    }
}

/// A [`RestLibrary`] over `stub`, plus the handle to inspect its calls.
fn library_over(stub: &StubTransport) -> RestLibrary {
    RestLibrary::new(Box::new(stub.clone()))
}

/// Asserts the stub saw exactly one request, to `expected_url` with the
/// expected session.
fn assert_single_call(stub: &StubTransport, expected_url: &str) {
    let calls = stub.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, expected_url);
    assert_eq!(calls[0].1, session());
}

#[test]
fn favorite_artists_map_library_json() {
    // The extra `genres` attribute is real-payload noise: serde must tolerate
    // it rather than fail the whole parse.
    let stub = StubTransport::returning(
        r#"{"data":[
            {"id":"artist-1","attributes":{"name":"The Sample Band","genres":["rock"]}},
            {"id":"artist-2","attributes":{"name":"Second Act"}}
        ]}"#,
    );
    let library = library_over(&stub);

    let artists = library.get_favorite_artists(&session()).unwrap();

    assert_eq!(
        artists,
        vec![
            Artist {
                id: "artist-1".to_string(),
                name: "The Sample Band".to_string(),
            },
            Artist {
                id: "artist-2".to_string(),
                name: "Second Act".to_string(),
            },
        ]
    );
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/artists");
}

#[test]
fn albums_by_artist_map_library_json_and_set_the_artist_id() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"album-1","attributes":{"name":"First Record"}}]}"#,
    );
    let library = library_over(&stub);

    let albums = library
        .get_albums_by_artist(&session(), "artist-9")
        .unwrap();

    assert_eq!(
        albums,
        vec![Album {
            id: "album-1".to_string(),
            title: "First Record".to_string(),
            artist_id: "artist-9".to_string(),
        }]
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/artists/artist-9/albums",
    );
}

#[test]
fn songs_from_album_map_library_json_and_set_the_album_id() {
    let stub =
        StubTransport::returning(r#"{"data":[{"id":"song-1","attributes":{"name":"Opening"}}]}"#);
    let library = library_over(&stub);

    let songs = library.get_songs_from_album(&session(), "album-9").unwrap();

    assert_eq!(
        songs,
        vec![Song {
            id: "song-1".to_string(),
            title: "Opening".to_string(),
            album_id: "album-9".to_string(),
        }]
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/albums/album-9/tracks",
    );
}

#[test]
fn transport_error_names_the_query() {
    let stub = StubTransport::failing("network down");
    let library = library_over(&stub);

    let error = library
        .get_favorite_artists(&session())
        .unwrap_err()
        .to_string();

    assert!(error.contains("favorite artists"), "{error}");
    assert!(error.contains("network down"), "{error}");
}

#[test]
fn malformed_json_names_the_query() {
    let stub = StubTransport::returning("not json");
    let library = library_over(&stub);

    let error = library
        .get_albums_by_artist(&session(), "artist-9")
        .unwrap_err()
        .to_string();

    assert!(error.contains("albums by artist"), "{error}");
    assert!(error.contains("JSON"), "{error}");
}

#[test]
fn nameless_artist_is_an_error_naming_its_id() {
    let stub = StubTransport::returning(r#"{"data":[{"id":"artist-1","attributes":{}}]}"#);
    let library = library_over(&stub);

    let error = library
        .get_favorite_artists(&session())
        .unwrap_err()
        .to_string();

    assert!(error.contains("favorite artists"), "{error}");
    assert!(error.contains("artist-1"), "{error}");
}

#[test]
fn blank_album_name_is_an_error() {
    let stub =
        StubTransport::returning(r#"{"data":[{"id":"album-1","attributes":{"name":"   "}}]}"#);
    let library = library_over(&stub);

    let error = library
        .get_albums_by_artist(&session(), "artist-9")
        .unwrap_err()
        .to_string();

    assert!(error.contains("albums by artist"), "{error}");
    assert!(error.contains("album-1"), "{error}");
}

#[test]
fn nameless_song_is_an_error() {
    let stub = StubTransport::returning(r#"{"data":[{"id":"song-1"}]}"#);
    let library = library_over(&stub);

    let error = library
        .get_songs_from_album(&session(), "album-9")
        .unwrap_err()
        .to_string();

    assert!(error.contains("songs from album"), "{error}");
    assert!(error.contains("song-1"), "{error}");
}
