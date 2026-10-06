//! Test-only fixtures and assertions shared across the crate's unit tests.
//!
//! These are the sample `Artist`/`Album`/`Song` values and the serde-contract
//! helpers that the `library`, `apple_music`, and `ui` test suites share. They
//! live in one named module rather than inside `library` so the data model
//! module stays only the model; every item here is compiled only for tests.

use crate::library::{Album, Artist, Song};

/// The three songs on `album-1` that the Previous/Next stepping tests step
/// through, shared by the `ui::transport` and `ui` test suites. Both suites
/// pin the same stepping behavior over the same three-song album and used to
/// build this list independently — a rename or reorder in one fixture would
/// silently diverge from the other — so it lives here once, next to the other
/// shared fixtures, and is compiled only for tests.
pub(crate) fn stepping_songs() -> Vec<Song> {
    vec![
        Song {
            id: "song-1".to_string(),
            title: "One".to_string(),
            album_id: "album-1".to_string(),
        },
        Song {
            id: "song-2".to_string(),
            title: "Two".to_string(),
            album_id: "album-1".to_string(),
        },
        Song {
            id: "song-3".to_string(),
            title: "Three".to_string(),
            album_id: "album-1".to_string(),
        },
    ]
}

/// The one-song album the single-song stepping tests step through, shared by
/// the two `ui::transport` tests that pin the single-song edge cases (Next and
/// Previous on a one-element list). Both used to build this same list
/// independently — a rename in one fixture would silently diverge from the
/// other — so it lives here once, next to [`stepping_songs`], and is compiled
/// only for tests.
pub(crate) fn single_song_album() -> Vec<Song> {
    vec![Song {
        id: "song-1".to_string(),
        title: "Only".to_string(),
        album_id: "album-1".to_string(),
    }]
}

/// The one-song album the browse-away tests land on after leaving `album-1`:
/// album-2's "song-4" / "B-side".
/// `songs_loaded_replaces_the_previous_albums_songs` and
/// `now_playing_label_keeps_the_track_name_after_browsing_to_another_album`
/// both built this same list inline to stand in for the album browsed to — a
/// retitle or id change in one copy would silently diverge from the other — so
/// it lives here once, next to [`single_song_album`], and is compiled only for
/// tests.
pub(crate) fn second_album_songs() -> Vec<Song> {
    vec![Song {
        id: "song-4".to_string(),
        title: "B-side".to_string(),
        album_id: "album-2".to_string(),
    }]
}

/// A single representative artist, album, and song, shared by the `library`,
/// `ui::views`, and `ui` test suites. Each suite used to build these same
/// objects independently — a retitle or id change in one fixture would
/// silently diverge from the others — so they live here once, next to
/// [`stepping_songs`], and each test only names which one it wants.
pub(crate) fn sample_artist() -> Artist {
    Artist {
        id: "artist-1".to_string(),
        name: "The Sample Band".to_string(),
    }
}

pub(crate) fn sample_album() -> Album {
    Album {
        id: "album-1".to_string(),
        title: "First Record".to_string(),
        artist_id: "artist-1".to_string(),
    }
}

pub(crate) fn sample_song() -> Song {
    Song {
        id: "song-1".to_string(),
        title: "Opening".to_string(),
        album_id: "album-1".to_string(),
    }
}

/// Asserts that `T` requires every field `payload` declares: for each key,
/// removing that one key must fail deserialization, because a payload missing
/// a required field must error rather than silently yield a half-populated
/// value the UI would render as blank data. The model types below and
/// `apple_music`'s auth-token test each hand in a full, valid payload, so the
/// walk-every-key loop lives here once. Probing every key matters: a single
/// named field per type leaves the others unpinned, and a `#[serde(default)]`
/// added to a field no probe omitted would clear the suite while blanking
/// that field.
pub(crate) fn assert_every_field_required<T>(payload: serde_json::Value)
where
    T: serde::de::DeserializeOwned,
{
    let object = payload
        .as_object()
        .expect("a full payload must be a JSON object");
    assert!(
        !object.is_empty(),
        "a full payload must declare at least one field"
    );
    for key in object.keys() {
        let mut without = object.clone();
        without.remove(key);
        assert!(
            serde_json::from_value::<T>(serde_json::Value::Object(without)).is_err(),
            "a payload missing required field {key:?} must be rejected"
        );
    }
}

/// Asserts that `value` serializes to exactly `expected` and then survives an
/// out-and-back trip through `serde_json` unchanged. The library model types
/// and `apple_music`'s auth-token test each pin the same two halves — the
/// exact serialized field names (a `#[serde(rename)]` would pass a round-trip
/// alone but fails this pin), then the round trip via [`assert_round_trips`] —
/// so the serialize-then-compare-then-round-trip sequence lives here once and
/// no caller can pin one half without the other.
pub(crate) fn assert_serializes_as<T>(value: T, expected: serde_json::Value)
where
    T: PartialEq + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned,
{
    assert_eq!(serde_json::to_value(&value).unwrap(), expected);
    assert_round_trips(value);
}

/// Asserts that `value` survives an out-and-back trip through `serde_json`
/// unchanged: serialize it, deserialize the result, and compare. The
/// round-trip half of [`assert_serializes_as`], kept separate so the chain has
/// one named implementation. Private to this module: [`assert_serializes_as`]
/// is the only entry point, so no caller can pin the round trip alone.
fn assert_round_trips<T>(value: T)
where
    T: PartialEq + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned,
{
    let back: T = serde_json::from_value(serde_json::to_value(&value).unwrap()).unwrap();
    assert_eq!(back, value);
}

/// Asserts that `payload` deserializes as `T` to exactly `expected`, proving
/// serde's default tolerance of fields beyond `T`'s declared set: a real
/// Apple Music payload carries more than the model's fields, so an unknown
/// field must be ignored rather than failing the whole parse. The three model
/// types below and `apple_music`'s auth-token test each probe this contract,
/// so the `from_value`-then-`assert_eq` chain lives here once.
pub(crate) fn assert_unknown_fields_tolerated<T>(payload: serde_json::Value, expected: T)
where
    T: PartialEq + std::fmt::Debug + serde::de::DeserializeOwned,
{
    let parsed: T = serde_json::from_value(payload).expect("unknown fields must be tolerated");
    assert_eq!(parsed, expected);
}

/// Asserts that `items` yield exactly the expected ids, in order.
///
/// The browse-query tests in the `apple_music` and `ui` suites each fetch a
/// list and pin its exact contents by mapping the list to its ids and
/// comparing; only the id projection and the expected ids differ, so the
/// map-to-ids-then-compare chain lives here once and each call site only
/// names its list and expectation. Comparing the ids *in order* is what
/// catches a dropped or reordered element that a bare non-empty assertion
/// would miss. The `sample_library` suite's `assert_id_label_pairs` is a
/// distinct helper — it pins each item's secondary label alongside its id,
/// not the id alone — so it stays local to that suite.
pub(crate) fn assert_ids<T>(items: &[T], id: impl Fn(&T) -> &str, expected: &[&str]) {
    let ids: Vec<&str> = items.iter().map(id).collect();
    assert_eq!(ids, expected);
}
