use super::*;

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

    let artists = library.get_favorite_artists(&rest_session()).unwrap();

    assert_eq!(
        artists,
        vec![
            sample_artist(),
            Artist {
                id: "artist-2".to_string(),
                name: "Second Act".to_string(),
                album_count: 0,
            },
        ]
    );
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/artists");
}

#[test]
fn albums_by_artist_map_library_json_and_set_the_artist_id() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"album-1","attributes":{"name":"First Record","artistName":"The Sample Band"}}]}"#,
    );
    let library = library_over(&stub);

    let albums = library
        .get_albums_by_artist(&rest_session(), "artist-9")
        .unwrap();

    assert_eq!(
        albums,
        vec![Album {
            artist_id: "artist-9".to_string(),
            ..sample_album()
        }]
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/artists/artist-9/albums",
    );
}

#[test]
fn songs_from_album_map_library_json_and_set_the_album_id() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","durationInMillis":210000}}]}"#,
    );
    let library = library_over(&stub);

    let songs = library
        .get_songs_from_album(&rest_session(), "album-9")
        .unwrap();

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

// A track the API returns without `attributes.durationInMillis` (an older or
// partial resource) must map to the model's `0` "no duration supplied"
// sentinel rather than failing the whole response, so the Now Playing bar
// shows `--:--` for it instead of dropping the song.
#[test]
fn songs_from_album_maps_a_missing_duration_to_zero() {
    assert_eq!(
        song_from_attributes(r#"{"name":"Opening"}"#),
        Song {
            id: "song-1".to_string(),
            title: "Opening".to_string(),
            artist: String::new(),
            album: String::new(),
            year: None,
            genre: String::new(),
            track_number: None,
            album_id: "album-9".to_string(),
            duration_ms: 0,
            preview_url: None,
        }
    );
}

// A negative, fractional, or above-`u64` duration is as unusable as an absent
// one. Like a missing duration it must map to the model's `0` sentinel instead
// of failing the whole album response, which would hide every song on the
// album.
#[test]
fn songs_from_album_maps_an_unusable_duration_to_zero() {
    for attributes in [
        r#"{"name":"Opening","durationInMillis":-1}"#,
        r#"{"name":"Opening","durationInMillis":1234.5}"#,
        r#"{"name":"Opening","durationInMillis":18446744073709551616}"#,
    ] {
        assert_eq!(song_from_attributes(attributes).duration_ms, 0);
    }
}

// A track the API returns without `attributes.artistName` (or with a blank
// one) must map to the model's `""` "no artist supplied" sentinel rather than
// failing the whole response, so the Now Playing bar falls back to the title
// alone instead of dropping the song or rendering a blank artist.
#[test]
fn songs_from_album_maps_a_missing_or_blank_artist_to_empty() {
    for attributes in [
        r#"{"name":"Opening"}"#,
        r#"{"name":"Opening","artistName":""}"#,
        r#"{"name":"Opening","artistName":"   "}"#,
    ] {
        assert_eq!(song_from_attributes(attributes).artist, "");
    }
}

// The album name is optional on a song: `attributes.albumName` maps onto
// `Song::album`, and a missing or blank name uses the same `""` "source
// supplied none" sentinel as the artist, so the File info panel can omit the
// album line.
#[test]
fn songs_from_album_maps_the_album_name() {
    assert_eq!(
        song_from_attributes(r#"{"name":"Opening","albumName":"First Record"}"#).album,
        "First Record"
    );
}

#[test]
fn songs_from_album_maps_a_missing_or_blank_album_name_to_empty() {
    for attributes in [
        r#"{"name":"Opening"}"#,
        r#"{"name":"Opening","albumName":""}"#,
        r#"{"name":"Opening","albumName":"   "}"#,
    ] {
        assert_eq!(song_from_attributes(attributes).album, "");
    }
}

// `attributes.releaseDate` maps onto `Song::year`, keeping only its leading
// four-digit year. Apple documents `YYYY-MM-DD` but also returns `YYYY` and
// `YYYY-MM`, so all three shapes name the same year, and surrounding
// whitespace is trimmed before the year is read.
#[test]
fn release_date_maps_to_the_song_year() {
    for date in ["2001-06-05", "2001", "2001-06", " 2001-06-05 "] {
        assert_eq!(
            song_from_attributes(&format!(r#"{{"name":"Opening","releaseDate":"{date}"}}"#)).year,
            Some(2001)
        );
    }
}

// A missing release date, or one with no usable leading year, maps to the
// `None` "source supplied none" sentinel so the File info panel omits the
// year line rather than drawing a bogus year.
#[test]
fn release_date_absent_or_malformed_maps_to_no_year() {
    for attributes in [
        r#"{"name":"Opening"}"#,
        r#"{"name":"Opening","releaseDate":""}"#,
        r#"{"name":"Opening","releaseDate":"   "}"#,
        r#"{"name":"Opening","releaseDate":"20"}"#,
        r#"{"name":"Opening","releaseDate":"20x1-06-05"}"#,
        r#"{"name":"Opening","releaseDate":"not-a-date"}"#,
    ] {
        assert_eq!(song_from_attributes(attributes).year, None);
    }
}

/// The [`Song`] `get_songs_from_album` maps from one resource carrying
/// `attributes` — the id (`song-1`) and album (`album-9`) are fixed, so a
/// caller varies only the attribute subset it probes. The duration, artist,
/// and preview tests each assert one mapped field of the same resource, so
/// this builder holds the JSON envelope and the fetch in one place.
fn song_from_attributes(attributes: &str) -> Song {
    let stub = StubTransport::returning(&format!(
        r#"{{"data":[{{"id":"song-1","attributes":{attributes}}}]}}"#
    ));
    let library = library_over(&stub);
    library
        .get_songs_from_album(&rest_session(), "album-9")
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
}

// The audio-output decision (QUESTIONS.md `## Answered`, 2026-10-08) scopes
// playback to the Apple Music preview asset, so the browse client must carry
// the preview's URL onto `Song`. The cases below differ only in the resource's
// `attributes`, so they build it through `song_from_attributes` and read the
// mapped `preview_url` here.
fn preview_url_from(attributes: &str) -> Option<String> {
    song_from_attributes(attributes).preview_url
}

// The primary genre is optional on a song: `attributes.genreNames` maps onto
// `Song::genre`, taking the first non-blank entry Apple lists so the umbrella
// genre doesn't win, and a missing or all-blank array uses the same `""`
// "source supplied none" sentinel as the artist.
#[test]
fn songs_from_album_maps_the_genre() {
    assert_eq!(
        song_from_attributes(r#"{"name":"Opening","genreNames":["Rock","Music"]}"#).genre,
        "Rock"
    );
    // A non-blank entry keeps its trimmed form, not the surrounding whitespace.
    assert_eq!(
        song_from_attributes(r#"{"name":"Opening","genreNames":[" Rock "]}"#).genre,
        "Rock"
    );
}

#[test]
fn songs_from_album_maps_a_missing_or_blank_genre_to_empty() {
    for attributes in [
        r#"{"name":"Opening"}"#,
        r#"{"name":"Opening","genreNames":[]}"#,
        r#"{"name":"Opening","genreNames":[""]}"#,
        r#"{"name":"Opening","genreNames":["  "]}"#,
    ] {
        assert_eq!(song_from_attributes(attributes).genre, "");
    }
    // A blank entry is skipped rather than ending the search, so a later real
    // genre still reaches the model.
    assert_eq!(
        song_from_attributes(r#"{"name":"Opening","genreNames":["","Rock"]}"#).genre,
        "Rock"
    );
}

#[test]
fn songs_from_album_maps_the_track_number() {
    assert_eq!(
        song_from_attributes(r#"{"name":"Opening","trackNumber":3}"#).track_number,
        Some(3)
    );
}

// A missing, null, or zero track number maps to the `None` "source supplied
// none" sentinel so the File info panel omits the track-number line rather
// than drawing a bogus position.
#[test]
fn songs_from_album_maps_a_missing_or_zero_track_number_to_none() {
    for attributes in [
        r#"{"name":"Opening"}"#,
        r#"{"name":"Opening","trackNumber":null}"#,
        r#"{"name":"Opening","trackNumber":0}"#,
    ] {
        assert_eq!(song_from_attributes(attributes).track_number, None);
    }
}

// A negative track number is just as unusable as zero, and `Song` documents
// every non-positive number as `None`. It must not make the whole album fetch
// fail: one bad position would hide every song on the album.
#[test]
fn songs_from_album_maps_a_negative_track_number_to_none() {
    assert_eq!(
        song_from_attributes(r#"{"name":"Opening","trackNumber":-1}"#).track_number,
        None
    );
}

// A track number too large for the model's `u32` is just as unusable as a
// non-positive one, and must not fail the whole album fetch. The last value is
// outside the `i64` range, so a signed wire field would reject it too.
#[test]
fn songs_from_album_maps_an_oversized_track_number_to_none() {
    for attributes in [
        r#"{"name":"Opening","trackNumber":4294967296}"#,
        r#"{"name":"Opening","trackNumber":18446744073709551616}"#,
    ] {
        assert_eq!(song_from_attributes(attributes).track_number, None);
    }
}

#[test]
fn songs_from_album_maps_the_first_preview_url() {
    assert_eq!(
        preview_url_from(&format!(
            r#"{{"name":"Opening","previews":[{{"url":"{PREVIEW_URL}"}}]}}"#
        )),
        Some(PREVIEW_URL.to_string())
    );
}

#[test]
fn songs_from_album_maps_no_preview_when_previews_is_absent() {
    assert_eq!(preview_url_from(r#"{"name":"Opening"}"#), None);
}

#[test]
fn songs_from_album_maps_no_preview_when_previews_is_empty() {
    assert_eq!(
        preview_url_from(r#"{"name":"Opening","previews":[]}"#),
        None
    );
}

#[test]
fn songs_from_album_maps_no_preview_when_every_url_is_blank() {
    // A preview with no `url` key and one with a whitespace-only url are both
    // "blank": neither can play, so the resource maps to `None`.
    assert_eq!(
        preview_url_from(r#"{"name":"Opening","previews":[{"url":""},{}]}"#),
        None
    );
}

#[test]
fn songs_from_album_skips_a_blank_preview_url_for_the_next() {
    assert_eq!(
        preview_url_from(
            r#"{"name":"Opening","previews":[{"url":"   "},{"url":"https://example.test/second.m4a"}]}"#
        ),
        Some("https://example.test/second.m4a".to_string())
    );
}

#[test]
fn songs_from_album_keeps_a_padded_preview_url_verbatim() {
    // `Resource::into_song` compares each candidate trimmed, so a padded
    // URL counts as non-blank and is selected rather than skipped, but it is
    // returned exactly as the resource carried it. This is the reachable twin
    // of the whitespace-only skip above: whitespace-only is skipped, padded is
    // taken as-is (the padding is not normalized away).
    assert_eq!(
        preview_url_from(
            r#"{"name":"Opening","previews":[{"url":"  https://example.test/padded.m4a\t"}]}"#
        ),
        Some("  https://example.test/padded.m4a\t".to_string())
    );
}

#[test]
fn albums_by_artist_percent_encodes_the_id() {
    let stub = StubTransport::returning(EMPTY_DATA_BODY);
    let library = library_over(&stub);

    library
        .get_albums_by_artist(&rest_session(), "a/b c?d#e")
        .unwrap();

    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/artists/a%2Fb%20c%3Fd%23e/albums",
    );
}

#[test]
fn songs_from_album_percent_encodes_the_id() {
    let stub = StubTransport::returning(EMPTY_DATA_BODY);
    let library = library_over(&stub);

    library
        .get_songs_from_album(&rest_session(), "caf\u{e9}")
        .unwrap();

    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/albums/caf%C3%A9/tracks",
    );
}

// The library search wraps its songs in `results.library-songs`, so the
// client must reach one level deeper than a browse collection. The requested
// URL pins the endpoint, the percent-encoded term, the `library-songs` type,
// and the documented page-size maximum; the mapped song pins the fields the
// model reads, with the empty `album_id` a search result carries.
#[test]
fn search_library_maps_the_search_envelope_and_percent_encodes_the_term() {
    let stub = StubTransport::returning(
        r#"{"results":{"library-songs":{"data":[{"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","durationInMillis":210000,"previews":[{"url":"https://example.test/preview.m4a"}]}}]}}}"#,
    );
    let library = library_over(&stub);

    let songs = library
        .search_library(&rest_session(), "open ing&x")
        .unwrap();

    assert_eq!(
        songs,
        vec![Song {
            id: "song-1".to_string(),
            title: "Opening".to_string(),
            artist: "The Sample Band".to_string(),
            album: String::new(),
            year: None,
            genre: String::new(),
            track_number: None,
            album_id: String::new(),
            duration_ms: 210_000,
            preview_url: Some(PREVIEW_URL.to_string()),
        }]
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/search?term=open%20ing%26x&types=library-songs&limit=25",
    );
}

// Apple omits `results.library-songs` entirely for a search that matched
// nothing, so an absent key is a valid empty result rather than a parse
// failure; an explicit empty `data` array is the other no-match shape. Both
// must return an empty list, not an error.
#[test]
fn search_library_returns_no_songs_when_the_results_carry_no_library_songs() {
    for body in [
        r#"{"results":{}}"#,
        r#"{"results":{"library-songs":{"data":[]}}}"#,
    ] {
        let stub = StubTransport::returning(body);
        let library = library_over(&stub);

        let songs = library
            .search_library(&rest_session(), "nothing matches")
            .unwrap();

        assert!(songs.is_empty(), "{body}");
    }
}

// Like a browse collection, a search response can be valid JSON and still
// not match the `{ "results": { ... } }` envelope the client expects. The
// report must name the search envelope, not the collection one, so a
// malformed search response points at the shape that failed; a regression
// that passed `"collection"` from `parse_search_envelope` would still produce
// a plausible-looking message for every other test.
#[test]
fn valid_json_that_is_not_the_search_envelope_names_the_search_envelope() {
    for body in [r#"{"data":[]}"#, r#"{"results":[]}"#] {
        let error = error_over(body, |library| library.search_library(&rest_session(), "x"));

        assert!(
            error.starts_with("response did not match the Apple Music search envelope:"),
            "{body}: {error}"
        );
    }
}

/// A two-page query is read in full: `first` answers the initial request and
/// its same-origin `next` link is followed to `second`. The search and the
/// three flat-list pagination tests all run this scaffold, differing only in
/// the response bodies, the followed URL, and the expected ids.
fn assert_follows_next_page<T>(
    first: &str,
    second: &str,
    next_url: &str,
    query: impl FnOnce(&RestLibrary) -> Result<Vec<T>, AppleMusicError>,
    id: impl Fn(&T) -> &str,
    expected: &[&str],
) {
    let stub = StubTransport::returning_bodies(&[first, second]);
    let library = library_over(&stub);

    let items = query(&library).unwrap();

    assert_ids(&items, id, expected);
    let calls = stub.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].0, next_url);
}

// A search matching more than one page is read in full: the client follows
// the same-origin `next` link with the same pagination rules as a browse
// collection.
#[test]
fn search_library_follows_a_next_page() {
    assert_follows_next_page(
        r#"{"results":{"library-songs":{"data":[{"id":"song-1","attributes":{"name":"Opening"}}],"next":"/v1/me/library/search?term=x&types=library-songs&limit=25&offset=25"}}}"#,
        r#"{"results":{"library-songs":{"data":[{"id":"song-2","attributes":{"name":"Second"}}]}}}"#,
        "https://api.music.apple.com/v1/me/library/search?term=x&types=library-songs&limit=25&offset=25",
        |library| library.search_library(&rest_session(), "x"),
        |song| song.id.as_str(),
        &["song-1", "song-2"],
    );
}

#[test]
fn search_library_reports_a_transport_error_bare() {
    let error = transport_error_over(|library| library.search_library(&rest_session(), "x"));

    assert_eq!(error, "network down");
}

#[test]
fn search_library_rejects_a_nameless_song() {
    let error = error_over(
        r#"{"results":{"library-songs":{"data":[{"id":"song-7","attributes":{}}]}}}"#,
        |library| library.search_library(&rest_session(), "x"),
    );

    assert!(error.contains("without a name"), "{error}");
    assert!(error.contains("song-7"), "{error}");
}

// The flat whole-library list reads the library-songs collection directly:
// the requested URL pins the endpoint, and the mapped song pins the fields
// the model reads, with the empty `album_id` a library song carries.
#[test]
fn get_all_songs_maps_the_library_songs_collection() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","durationInMillis":210000,"previews":[{"url":"https://example.test/preview.m4a"}]}}]}"#,
    );
    let library = library_over(&stub);

    let songs = library.get_all_songs(&rest_session()).unwrap();

    assert_eq!(
        songs,
        vec![Song {
            id: "song-1".to_string(),
            title: "Opening".to_string(),
            artist: "The Sample Band".to_string(),
            album: String::new(),
            year: None,
            genre: String::new(),
            track_number: None,
            album_id: String::new(),
            duration_ms: 210_000,
            preview_url: Some(PREVIEW_URL.to_string()),
        }]
    );
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/songs");
}

// The album running times are grouped from the whole-library songs collection
// by exact `(albumName, artistName)`, summing each album's songs. A resource
// without a non-blank album name is skipped, an album missing from the map is
// left to the caller's `0`, a blank artist maps to the empty sentinel, and a
// missing duration counts as `0`. The URL pins the songs endpoint.
#[test]
fn get_album_durations_groups_and_sums_library_songs() {
    let stub = StubTransport::returning(
        r#"{"data":[
            {"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","albumName":"First Record","durationInMillis":210000}},
            {"id":"song-2","attributes":{"name":"Middle","artistName":"The Sample Band","albumName":"First Record","durationInMillis":240000}},
            {"id":"song-3","attributes":{"name":"B-side","artistName":"The Sample Band","albumName":"Second Record","durationInMillis":95000}},
            {"id":"song-4","attributes":{"name":"Orphan","artistName":"Nobody","durationInMillis":1000}},
            {"id":"song-5","attributes":{"name":"Blank","artistName":"Nobody","albumName":"   ","durationInMillis":1000}},
            {"id":"song-6","attributes":{"name":"Headliner","artistName":"   ","albumName":"Debut","durationInMillis":200000}},
            {"id":"song-7","attributes":{"name":"Unknown","artistName":"The Sample Band","albumName":"Unknown Record"}}
        ]}"#,
    );
    let library = library_over(&stub);

    let durations = library.get_album_durations(&rest_session()).unwrap();

    assert_eq!(
        durations,
        std::collections::HashMap::from([
            (
                ("First Record".to_string(), "The Sample Band".to_string()),
                450_000,
            ),
            (
                ("Second Record".to_string(), "The Sample Band".to_string()),
                95_000,
            ),
            (("Debut".to_string(), String::new()), 200_000),
            (
                ("Unknown Record".to_string(), "The Sample Band".to_string()),
                0,
            ),
        ])
    );
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/songs");
}

// The album running-time request reads the same duration field. An unusable
// value must count as zero for its album instead of failing the whole songs
// response, which would also break both album lists.
#[test]
fn get_album_durations_counts_an_unusable_duration_as_zero() {
    let stub = StubTransport::returning(
        r#"{"data":[
            {"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","albumName":"First Record","durationInMillis":-1}},
            {"id":"song-2","attributes":{"name":"Middle","artistName":"The Sample Band","albumName":"First Record","durationInMillis":1234.5}},
            {"id":"song-3","attributes":{"name":"Closing","artistName":"The Sample Band","albumName":"First Record","durationInMillis":18446744073709551616}}
        ]}"#,
    );
    let library = library_over(&stub);

    let durations = library.get_album_durations(&rest_session()).unwrap();

    assert_eq!(
        durations,
        std::collections::HashMap::from([(
            ("First Record".to_string(), "The Sample Band".to_string()),
            0,
        )])
    );
}

// A library larger than one page is read in full: the flat list follows the
// same-origin `next` link with the same pagination rules as a browse
// collection.
#[test]
fn get_all_songs_follows_a_next_page() {
    assert_follows_next_page(
        r#"{"data":[{"id":"song-1","attributes":{"name":"Opening"}}],"next":"/v1/me/library/songs?offset=100"}"#,
        r#"{"data":[{"id":"song-2","attributes":{"name":"Second"}}]}"#,
        "https://api.music.apple.com/v1/me/library/songs?offset=100",
        |library| library.get_all_songs(&rest_session()),
        |song| song.id.as_str(),
        &["song-1", "song-2"],
    );
}

#[test]
fn get_all_songs_reports_a_transport_error_bare() {
    let error = transport_error_over(|library| library.get_all_songs(&rest_session()));

    assert_eq!(error, "network down");
}

#[test]
fn get_all_songs_rejects_a_nameless_song() {
    let error = error_over(r#"{"data":[{"id":"song-7","attributes":{}}]}"#, |library| {
        library.get_all_songs(&rest_session())
    });

    assert!(error.contains("without a name"), "{error}");
    assert!(error.contains("song-7"), "{error}");
}

// The flat whole-library album list reads the library-albums collection
// directly: the requested URL pins the endpoint, and the mapped album pins
// the fields the model reads, with the empty `artist_id` a library album
// carries in this collection response.
#[test]
fn get_all_albums_maps_the_library_albums_collection() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"album-1","attributes":{"name":"First Record","artistName":"The Sample Band"}}]}"#,
    );
    let library = library_over(&stub);

    let albums = library.get_all_albums(&rest_session()).unwrap();

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
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/albums");
}

// An album resource without `attributes.artistName` must map to the model's
// `""` "no artist supplied" sentinel rather than dropping the album, so a
// library album that carries no artist still shows its bare title.
#[test]
fn get_all_albums_leaves_a_missing_album_artist_empty() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"album-1","attributes":{"name":"First Record"}}]}"#,
    );
    let library = library_over(&stub);

    let albums = library.get_all_albums(&rest_session()).unwrap();

    assert_eq!(albums[0].artist, "");
}

// A whitespace-only artist name is as useless as a missing one, so it must map
// to the same empty sentinel instead of rendering a blank prefix.
#[test]
fn get_all_albums_leaves_a_blank_album_artist_empty() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"album-1","attributes":{"name":"First Record","artistName":"   "}}]}"#,
    );
    let library = library_over(&stub);

    let albums = library.get_all_albums(&rest_session()).unwrap();

    assert_eq!(albums[0].artist, "");
}

// A library larger than one page is read in full: the flat album list follows
// the same-origin `next` link with the same pagination rules as the other
// browse collections.
#[test]
fn get_all_albums_follows_a_next_page() {
    assert_follows_next_page(
        r#"{"data":[{"id":"album-1","attributes":{"name":"Opening"}}],"next":"/v1/me/library/albums?offset=100"}"#,
        r#"{"data":[{"id":"album-2","attributes":{"name":"Second"}}]}"#,
        "https://api.music.apple.com/v1/me/library/albums?offset=100",
        |library| library.get_all_albums(&rest_session()),
        |album| album.id.as_str(),
        &["album-1", "album-2"],
    );
}

#[test]
fn get_all_albums_reports_a_transport_error_bare() {
    let error = transport_error_over(|library| library.get_all_albums(&rest_session()));

    assert_eq!(error, "network down");
}

#[test]
fn get_all_albums_rejects_a_nameless_album() {
    let error = error_over(
        r#"{"data":[{"id":"album-7","attributes":{}}]}"#,
        |library| library.get_all_albums(&rest_session()),
    );

    assert!(error.contains("without a name"), "{error}");
    assert!(error.contains("album-7"), "{error}");
}

#[test]
fn get_all_playlists_maps_the_library_playlists_collection() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"playlist-1","attributes":{"name":"Morning Drive"}}]}"#,
    );
    let library = library_over(&stub);

    let playlists = library.get_all_playlists(&rest_session()).unwrap();

    assert_eq!(playlists, vec![sample_playlist()]);
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/playlists");
}

// A library larger than one page is read in full: the flat playlist list
// follows the same-origin `next` link with the same pagination rules as the
// other browse collections.
#[test]
fn get_all_playlists_follows_a_next_page() {
    assert_follows_next_page(
        r#"{"data":[{"id":"playlist-1","attributes":{"name":"Morning Drive"}}],"next":"/v1/me/library/playlists?offset=100"}"#,
        r#"{"data":[{"id":"playlist-2","attributes":{"name":"Focus"}}]}"#,
        "https://api.music.apple.com/v1/me/library/playlists?offset=100",
        |library| library.get_all_playlists(&rest_session()),
        |playlist| playlist.id.as_str(),
        &["playlist-1", "playlist-2"],
    );
}

#[test]
fn get_all_playlists_reports_a_transport_error_bare() {
    let error = transport_error_over(|library| library.get_all_playlists(&rest_session()));

    assert_eq!(error, "network down");
}

#[test]
fn get_all_playlists_rejects_a_nameless_playlist() {
    let error = error_over(
        r#"{"data":[{"id":"playlist-7","attributes":{}}]}"#,
        |library| library.get_all_playlists(&rest_session()),
    );

    assert!(error.contains("without a name"), "{error}");
    assert!(error.contains("playlist-7"), "{error}");
}

// A playlist's tracks are read from the id-scoped collection: the requested
// URL pins the endpoint, and the mapped song pins the fields the model reads,
// with the empty `album_id` a library playlist track carries.
#[test]
fn get_songs_from_playlist_maps_the_playlist_tracks_collection() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"song-1","attributes":{"name":"Opening","artistName":"The Sample Band","durationInMillis":210000,"previews":[{"url":"https://example.test/preview.m4a"}]}}]}"#,
    );
    let library = library_over(&stub);

    let songs = library
        .get_songs_from_playlist(&rest_session(), "playlist-9")
        .unwrap();

    assert_eq!(
        songs,
        vec![Song {
            id: "song-1".to_string(),
            title: "Opening".to_string(),
            artist: "The Sample Band".to_string(),
            album: String::new(),
            year: None,
            genre: String::new(),
            track_number: None,
            album_id: String::new(),
            duration_ms: 210_000,
            preview_url: Some(PREVIEW_URL.to_string()),
        }]
    );
    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/playlists/playlist-9/tracks",
    );
}

#[test]
fn get_songs_from_playlist_percent_encodes_the_playlist_id() {
    let stub = StubTransport::returning(EMPTY_DATA_BODY);
    let library = library_over(&stub);

    library
        .get_songs_from_playlist(&rest_session(), "a/b c?d#e")
        .unwrap();

    assert_single_call(
        &stub,
        "https://api.music.apple.com/v1/me/library/playlists/a%2Fb%20c%3Fd%23e/tracks",
    );
}

#[test]
fn get_songs_from_playlist_reports_a_transport_error_bare() {
    let error = transport_error_over(|library| {
        library.get_songs_from_playlist(&rest_session(), "playlist-9")
    });

    assert_eq!(error, "network down");
}

#[test]
fn encode_path_segment_leaves_unreserved_bytes_and_encodes_the_rest() {
    assert_eq!(encode_path_segment("i.abc-123_~"), "i.abc-123_~");
    assert_eq!(encode_path_segment("a b/c?d#e"), "a%20b%2Fc%3Fd%23e");
    assert_eq!(encode_path_segment("caf\u{e9}"), "caf%C3%A9");
}

// The example test above pins a handful of bytes; the RFC 3986 rule covers
// all of them. Sweep the whole ASCII range so every byte's branch — the
// unreserved passthrough, the reserved/control escape, and the uppercase hex
// digits — is pinned, not just the samples a hand-written list happened to
// include. A regression that, say, lowercased the hex or forgot `~` would
// clear the example test's three inputs but fail here.
#[test]
fn encode_path_segment_applies_the_rfc_3986_rule_to_every_ascii_byte() {
    for byte in 0x00u8..=0x7F {
        let input = char::from(byte).to_string();
        let expected = if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
        {
            input.clone()
        } else {
            format!("%{byte:02X}")
        };
        assert_eq!(
            encode_path_segment(&input),
            expected,
            "ASCII byte {byte:#04X} must encode to {expected:?}"
        );
    }
}

#[test]
fn transport_error_propagates_the_bare_cause() {
    let error = transport_error_over(|library| library.get_favorite_artists(&rest_session()));

    // The cause is bare: the UI supplies the context naming the query, so
    // this layer does not repeat it.
    assert_eq!(error, "network down");
}

#[test]
fn albums_by_artist_transport_error_propagates_the_bare_cause() {
    let error =
        transport_error_over(|library| library.get_albums_by_artist(&rest_session(), "artist-9"));

    assert_eq!(error, "network down");
}

#[test]
fn songs_from_album_transport_error_propagates_the_bare_cause() {
    let error =
        transport_error_over(|library| library.get_songs_from_album(&rest_session(), "album-9"));

    assert_eq!(error, "network down");
}

#[test]
fn malformed_json_surfaces_the_parse_error() {
    let error = error_over("not json", |library| {
        library.get_albums_by_artist(&rest_session(), "artist-9")
    });

    assert!(error.starts_with("response was not valid JSON:"), "{error}");
}

// A response can be valid JSON and still not be the `{ "data": [ ... ] }`
// envelope the client expects (an unexpected shape, or an `errors` envelope on
// a 2xx the status check never inspected). `serde_json` classifies that as a
// `Data` error, distinct from a syntax error; the report must say the envelope
// does not match rather than mislabeling valid JSON as invalid. Pin the
// distinct wording, and that it still carries `serde_json`'s own detail.
#[test]
fn valid_json_that_is_not_the_collection_envelope_names_the_envelope() {
    let error = error_over(r#"{"artists":[]}"#, |library| {
        library.get_albums_by_artist(&rest_session(), "artist-9")
    });

    assert!(
        error.starts_with("response did not match the Apple Music collection envelope:"),
        "{error}"
    );
}

#[test]
fn nameless_artist_is_an_error_naming_its_id() {
    let error = error_over(
        r#"{"data":[{"id":"artist-1","attributes":{}}]}"#,
        |library| library.get_favorite_artists(&rest_session()),
    );

    assert_eq!(error, "response carried artist \"artist-1\" without a name");
}

#[test]
fn blank_album_name_is_an_error() {
    let error = error_over(
        r#"{"data":[{"id":"album-1","attributes":{"name":"   "}}]}"#,
        |library| library.get_albums_by_artist(&rest_session(), "artist-9"),
    );

    assert_eq!(error, "response carried album \"album-1\" without a name");
}

#[test]
fn nameless_song_is_an_error() {
    let error = error_over(r#"{"data":[{"id":"song-1"}]}"#, |library| {
        library.get_songs_from_album(&rest_session(), "album-9")
    });

    assert_eq!(error, "response carried song \"song-1\" without a name");
}

// A resource that omits the whole `attributes` object (not just its `name`)
// has no name either. `into_album` takes its explicit `None` arm and the
// artist and playlist maps fold the missing object through `and_then`; every
// one must reject the response as nameless rather than panic on the missing
// object. The song map's missing-attributes case is `nameless_song_is_an_error`
// above.
#[test]
fn a_resource_without_attributes_is_rejected_as_nameless() {
    let artist = error_over(r#"{"data":[{"id":"artist-1"}]}"#, |library| {
        library.get_favorite_artists(&rest_session())
    });
    assert_eq!(
        artist,
        "response carried artist \"artist-1\" without a name"
    );

    let album = error_over(r#"{"data":[{"id":"album-1"}]}"#, |library| {
        library.get_albums_by_artist(&rest_session(), "artist-9")
    });
    assert_eq!(album, "response carried album \"album-1\" without a name");

    let playlist = error_over(r#"{"data":[{"id":"playlist-1"}]}"#, |library| {
        library.get_all_playlists(&rest_session())
    });
    assert_eq!(
        playlist,
        "response carried playlist \"playlist-1\" without a name"
    );
}

// A resource with a blank id and a valid name is the id twin of the
// `nameless_*` cases: an id is how the model links rows together and how the
// seam later resolves a browse, so a blank one can name nothing and must fail
// the response rather than become a row that errors when pressed. Each of the
// three maps calls `required_id`, so each query gets its own probe.
#[test]
fn blank_artist_id_is_an_error() {
    let error = error_over(
        r#"{"data":[{"id":"   ","attributes":{"name":"Ghost"}}]}"#,
        |library| library.get_favorite_artists(&rest_session()),
    );

    assert_eq!(
        error,
        "response carried artist with a blank id (got \"   \")"
    );
}

#[test]
fn blank_album_id_is_an_error() {
    let error = error_over(
        r#"{"data":[{"id":"","attributes":{"name":"Ghost"}}]}"#,
        |library| library.get_albums_by_artist(&rest_session(), "artist-9"),
    );

    assert_eq!(error, "response carried album with a blank id (got \"\")");
}

#[test]
fn blank_song_id_is_an_error() {
    let error = error_over(
        r#"{"data":[{"id":"","attributes":{"name":"Ghost"}}]}"#,
        |library| library.get_songs_from_album(&rest_session(), "album-9"),
    );

    assert_eq!(error, "response carried song with a blank id (got \"\")");
}
