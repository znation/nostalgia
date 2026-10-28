use super::*;

const URL: &str = "https://api.music.apple.com/v1/me/library/recently-added";

// The endpoint mixes resource types: only the `library-albums` resource maps, with the empty
// artist id the library collections carry, while a library playlist and a catalog album are
// dropped.
#[test]
fn get_recently_added_albums_keeps_only_library_albums() {
    let stub = StubTransport::returning(
        r#"{"data":[
            {"id":"p.1","type":"library-playlists","attributes":{"name":"Morning Drive"}},
            {"id":"album-1","type":"library-albums","attributes":{"name":"First Record","artistName":"The Sample Band"}},
            {"id":"1234","type":"albums","attributes":{"name":"Catalog Record"}}
        ]}"#,
    );
    let library = library_over(&stub);

    let albums = library.get_recently_added_albums(&rest_session()).unwrap();

    assert_eq!(
        albums,
        vec![Album {
            id: "album-1".to_string(),
            title: "First Record".to_string(),
            artist_id: String::new(),
            artist: "The Sample Band".to_string(),
            duration_ms: 0,
        }]
    );
    assert_single_call(&stub, URL);
}

#[test]
fn get_recently_added_albums_drops_a_resource_without_a_type() {
    let stub =
        StubTransport::returning(r#"{"data":[{"id":"album-1","attributes":{"name":"Untyped"}}]}"#);
    let library = library_over(&stub);

    let albums = library.get_recently_added_albums(&rest_session()).unwrap();

    assert!(albums.is_empty());
}

#[test]
fn get_recently_added_albums_keeps_apples_order_and_follows_a_next_page() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[{"id":"album-2","type":"library-albums","attributes":{"name":"Newer"}}],"next":"/v1/me/library/recently-added?offset=10"}"#,
        r#"{"data":[{"id":"album-1","type":"library-albums","attributes":{"name":"Older"}}]}"#,
    ]);
    let library = library_over(&stub);

    let albums = library.get_recently_added_albums(&rest_session()).unwrap();

    assert_ids(&albums, |album| album.id.as_str(), &["album-2", "album-1"]);
    let calls = stub.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, URL);
    assert_eq!(
        calls[1].0,
        "https://api.music.apple.com/v1/me/library/recently-added?offset=10"
    );
}

#[test]
fn get_recently_added_albums_reports_a_transport_error_bare() {
    let error = transport_error_over(|library| library.get_recently_added_albums(&rest_session()));

    assert_eq!(error, "network down");
}

#[test]
fn get_recently_added_albums_rejects_a_nameless_library_album() {
    let error = error_over(
        r#"{"data":[{"id":"album-7","type":"library-albums","attributes":{}}]}"#,
        |library| library.get_recently_added_albums(&rest_session()),
    );

    assert!(error.contains("without a name"), "{error}");
    assert!(error.contains("album-7"), "{error}");
}
