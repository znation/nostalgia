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

#[cfg(test)]
mod tests {
    use super::{Album, Artist, Song};
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
        let artist = Artist {
            id: "artist-1".to_string(),
            name: "The Sample Band".to_string(),
        };

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
        let album = Album {
            id: "album-1".to_string(),
            title: "First Record".to_string(),
            artist_id: "artist-1".to_string(),
        };

        assert_round_trips(album);
    }

    #[test]
    fn song_round_trips_through_json() {
        let song = Song {
            id: "song-1".to_string(),
            title: "Opening".to_string(),
            album_id: "album-1".to_string(),
        };

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

        assert_eq!(
            artist,
            Artist {
                id: "artist-1".to_string(),
                name: "The Sample Band".to_string(),
            }
        );
    }

    #[test]
    fn deserialization_rejects_missing_required_fields() {
        // A payload missing a required field must error, not silently yield a
        // half-populated model the UI would render as blank data.
        let result: Result<Artist, _> = serde_json::from_value(json!({ "id": "artist-1" }));
        assert!(result.is_err());
    }
}
