//! The music-library data model — `Artist`, `Album`, `Song` — shared by the
//! Apple Music service and the UI.
//!
//! The types live outside `apple_music` because both the integration seam
//! (which populates them) and the presentation layer (which renders them)
//! depend on them; the service module alone is not their owner. Keeping them
//! here, beside the shared `state` module, means the real Apple Music API can
//! replace the stub without touching the model or the UI.

use serde::{Deserialize, Serialize};

/// An artist in the user's library.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
}

/// An album by an [`Artist`], linked to it by [`Album::artist_id`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist_id: String,
}

/// A song on an [`Album`], linked to it by [`Song::album_id`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub album_id: String,
}

/// The three songs on `album-1` that the Previous/Next stepping tests step
/// through, shared by the `ui::transport` and `ui` test suites. Both suites
/// pin the same stepping behavior over the same three-song album and used to
/// build this list independently — a rename or reorder in one fixture would
/// silently diverge from the other — so it lives here once, next to `Song`,
/// and is compiled only for tests.
#[cfg(test)]
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
/// other — so it lives here once, next to [`stepping_songs`] and `Song`, and
/// is compiled only for tests.
#[cfg(test)]
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
#[cfg(test)]
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
#[cfg(test)]
pub(crate) fn sample_artist() -> Artist {
    Artist {
        id: "artist-1".to_string(),
        name: "The Sample Band".to_string(),
    }
}

#[cfg(test)]
pub(crate) fn sample_album() -> Album {
    Album {
        id: "album-1".to_string(),
        title: "First Record".to_string(),
        artist_id: "artist-1".to_string(),
    }
}

#[cfg(test)]
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
#[cfg(test)]
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

/// Asserts that `value` survives an out-and-back trip through `serde_json`
/// unchanged: serialize it, deserialize the result, and compare. The three
/// model types below and `apple_music`'s auth-token test each pin this
/// contract, so the serialize-then-deserialize-then-compare chain lives here
/// once. Each round-trip test asserts the exact serialized field names first,
/// then calls this; a `#[serde(rename)]` would pass a round-trip alone but
/// fails that field-name pin.
#[cfg(test)]
pub(crate) fn assert_round_trips<T>(value: T)
where
    T: PartialEq + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned,
{
    let back: T = serde_json::from_value(serde_json::to_value(&value).unwrap()).unwrap();
    assert_eq!(back, value);
}

#[cfg(test)]
mod tests {
    use super::{
        Album, Artist, Song, assert_every_field_required, assert_round_trips, sample_album,
        sample_artist, sample_song,
    };
    use serde_json::json;

    /// The real Apple Music API will hand these types to the app as JSON, so
    /// the round-trip (out and back through `serde_json`) is the contract that
    /// lets a live service replace the stub without touching the model or UI.
    #[test]
    fn artist_serializes_field_names_and_round_trips() {
        let artist = sample_artist();

        // Field names are serialized as-is (no renames): the wire contract a
        // real Apple Music payload must satisfy.
        let value = serde_json::to_value(&artist).unwrap();
        assert_eq!(
            value,
            json!({ "id": "artist-1", "name": "The Sample Band" })
        );

        assert_round_trips(artist);
    }

    #[test]
    fn album_serializes_field_names_and_round_trips() {
        let album = sample_album();

        // As with Artist: field names are serialized as-is (no renames), the
        // wire contract a real Apple Music payload must satisfy. The
        // round-trip alone passes for *any* field names, so the exact JSON
        // shape is pinned before it.
        let value = serde_json::to_value(&album).unwrap();
        assert_eq!(
            value,
            json!({ "id": "album-1", "title": "First Record", "artist_id": "artist-1" })
        );

        assert_round_trips(album);
    }

    #[test]
    fn song_serializes_field_names_and_round_trips() {
        let song = sample_song();

        // The Song twin of the Album and Artist field-name pins.
        let value = serde_json::to_value(&song).unwrap();
        assert_eq!(
            value,
            json!({ "id": "song-1", "title": "Opening", "album_id": "album-1" })
        );

        assert_round_trips(song);
    }

    #[test]
    fn deserialization_ignores_unknown_fields() {
        // Real Apple Music payloads carry more than the model's fields; serde's
        // default must tolerate the extras rather than failing the whole parse.
        let artist: Artist = serde_json::from_value(json!({
            "id": "artist-1",
            "name": "The Sample Band",
            "genres": ["rock"]
        }))
        .unwrap();

        assert_eq!(artist, sample_artist());
    }

    // The unknown-field tolerance is per-type, not global: the test above pins
    // it for `Artist` only, so `Album`'s and `Song`'s serde "ignore an unknown
    // field" arms are never exercised. Real Apple Music payloads carry more
    // than the model's fields, so a `#[serde(deny_unknown_fields)]` added to
    // either type would fail the whole parse while clearing every existing
    // test (none deserialize these types with an extra field). Pin the same
    // tolerance for both, as the missing-field tests below do per-type.
    #[test]
    fn album_deserialization_ignores_unknown_fields() {
        let album: Album = serde_json::from_value(json!({
            "id": "album-1",
            "title": "First Record",
            "artist_id": "artist-1",
            "release_date": "2026-01-01"
        }))
        .unwrap();

        assert_eq!(album, sample_album());
    }

    #[test]
    fn song_deserialization_ignores_unknown_fields() {
        let song: Song = serde_json::from_value(json!({
            "id": "song-1",
            "title": "Opening",
            "album_id": "album-1",
            "duration_ms": 210_000
        }))
        .unwrap();

        assert_eq!(song, sample_song());
    }

    #[test]
    fn deserialization_rejects_missing_required_fields() {
        // A payload missing any required field must error, not silently yield
        // a half-populated model the UI would render as blank data.
        assert_every_field_required::<Artist>(
            json!({ "id": "artist-1", "name": "The Sample Band" }),
        );
    }

    // The missing-field contract holds for every model type, not just Artist:
    // `Album` and `Song` only have round-trip tests, which pass for *any*
    // deserialization that yields some value, so a regression that made one of
    // their fields optional (e.g. a stray `#[serde(default)]` added to tolerate
    // a payload variant) would clear every existing test while silently
    // rendering blank data. Each gets the same every-field probe as Artist.
    #[test]
    fn album_deserialization_rejects_missing_required_fields() {
        assert_every_field_required::<Album>(
            json!({ "id": "album-1", "title": "First Record", "artist_id": "artist-1" }),
        );
    }

    #[test]
    fn song_deserialization_rejects_missing_required_fields() {
        assert_every_field_required::<Song>(
            json!({ "id": "song-1", "title": "Opening", "album_id": "album-1" }),
        );
    }
}
