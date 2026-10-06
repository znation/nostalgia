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

#[cfg(test)]
mod tests {
    use super::{Album, Artist, Song, sample_album, sample_artist, sample_song};
    use serde_json::json;
    use std::fmt::Debug;

    /// The model's wire contract: a value survives an out-and-back trip
    /// through `serde_json` unchanged. Every model type is pinned for the
    /// round-trip, so the serialize-then-deserialize-then-compare chain lives
    /// here once and each test only builds its value. (The artist test
    /// additionally asserts the exact serialized field names, which it does
    /// before the round-trip.)
    fn assert_round_trips<T>(value: T)
    where
        T: PartialEq + Debug + serde::Serialize + serde::de::DeserializeOwned,
    {
        let back: T = serde_json::from_value(serde_json::to_value(&value).unwrap()).unwrap();
        assert_eq!(back, value);
    }

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
    fn album_round_trips_through_json() {
        assert_round_trips(sample_album());
    }

    #[test]
    fn song_round_trips_through_json() {
        assert_round_trips(sample_song());
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

    #[test]
    fn deserialization_rejects_missing_required_fields() {
        // A payload missing a required field must error, not silently yield a
        // half-populated model the UI would render as blank data.
        let result: Result<Artist, _> = serde_json::from_value(json!({ "id": "artist-1" }));
        assert!(result.is_err());
    }

    // The missing-field contract holds for every model type, not just Artist:
    // `Album` and `Song` only have round-trip tests, which pass for *any*
    // deserialization that yields some value, so a regression that made one of
    // their fields optional (e.g. a stray `#[serde(default)]` added to tolerate
    // a payload variant) would clear every existing test while silently
    // rendering blank data. Each gets the same explicit probe as Artist.
    #[test]
    fn album_deserialization_rejects_missing_required_fields() {
        let result: Result<Album, _> =
            serde_json::from_value(json!({ "id": "album-1", "title": "First Record" }));
        assert!(result.is_err());
    }

    #[test]
    fn song_deserialization_rejects_missing_required_fields() {
        let result: Result<Song, _> =
            serde_json::from_value(json!({ "id": "song-1", "title": "Opening" }));
        assert!(result.is_err());
    }
}
