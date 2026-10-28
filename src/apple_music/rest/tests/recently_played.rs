use super::*;

const URL: &str = "https://api.music.apple.com/v1/me/recent/played/tracks";

// The endpoint mixes resource types: catalog and library songs map in order, a music video is
// dropped.
#[test]
fn get_recently_played_songs_keeps_catalog_and_library_songs() {
    let stub = StubTransport::returning(&format!(
        r#"{{"data":[
            {{"id":"1234","type":"songs","attributes":{{"name":"Opening","artistName":"The Sample Band","albumName":"First Record","durationInMillis":1000,"previews":[{{"url":"{PREVIEW_URL}"}}]}}}},
            {{"id":"i.5","type":"library-songs","attributes":{{"name":"Middle"}}}},
            {{"id":"mv.1","type":"music-videos","attributes":{{"name":"Clip"}}}}
        ]}}"#
    ));
    let library = library_over(&stub);

    let songs = library.get_recently_played_songs(&rest_session()).unwrap();

    assert_eq!(songs.len(), 2);
    assert_eq!(songs[0].id, "1234");
    assert_eq!(songs[0].title, "Opening");
    assert_eq!(songs[0].artist, "The Sample Band");
    assert_eq!(songs[0].album, "First Record");
    assert_eq!(songs[0].album_id, "");
    assert_eq!(songs[0].preview_url, Some(PREVIEW_URL.to_string()));
    assert_eq!(songs[1].id, "i.5");
    assert_single_call(&stub, URL);
}

#[test]
fn get_recently_played_songs_drops_a_resource_without_a_type() {
    let stub =
        StubTransport::returning(r#"{"data":[{"id":"s.1","attributes":{"name":"Untyped"}}]}"#);
    let library = library_over(&stub);

    let songs = library.get_recently_played_songs(&rest_session()).unwrap();

    assert!(songs.is_empty());
}

#[test]
fn get_recently_played_songs_keeps_apples_order_and_follows_a_next_page() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[{"id":"s.2","type":"songs","attributes":{"name":"Newer"}}],"next":"/v1/me/recent/played/tracks?offset=10"}"#,
        r#"{"data":[{"id":"s.1","type":"songs","attributes":{"name":"Older"}}]}"#,
    ]);
    let library = library_over(&stub);

    let songs = library.get_recently_played_songs(&rest_session()).unwrap();

    assert_ids(&songs, |song| song.id.as_str(), &["s.2", "s.1"]);
    let calls = stub.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, URL);
    assert_eq!(
        calls[1].0,
        "https://api.music.apple.com/v1/me/recent/played/tracks?offset=10"
    );
}
