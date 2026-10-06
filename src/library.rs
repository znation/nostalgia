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
    /// The artist's stable identifier; an [`Album::artist_id`] refers to it.
    pub id: String,
    /// The display name the Artists view renders.
    pub name: String,
}

/// An album by an [`Artist`], linked to it by [`Album::artist_id`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Album {
    /// The album's stable identifier; a [`Song::album_id`] refers to it.
    pub id: String,
    /// The display title the Albums view renders.
    pub title: String,
    /// The [`Artist::id`] of the artist this album belongs to.
    pub artist_id: String,
}

/// A song on an [`Album`], linked to it by [`Song::album_id`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Song {
    /// The song's stable identifier; the shared `AppState` records it as the
    /// current track when the song plays.
    pub id: String,
    /// The display title the Songs view and the Now Playing bar render.
    pub title: String,
    /// The [`Album::id`] of the album this song belongs to.
    pub album_id: String,
}

#[cfg(test)]
mod tests {
    use super::{Album, Artist, Song};
    use crate::test_support::{
        assert_every_field_required, assert_serializes_as, assert_unknown_fields_tolerated,
        sample_album, sample_artist, sample_song,
    };
    use serde_json::json;

    /// The real Apple Music API will hand these types to the app as JSON, so
    /// the round-trip (out and back through `serde_json`) is the contract that
    /// lets a live service replace the stub without touching the model or UI.
    #[test]
    fn artist_serializes_field_names_and_round_trips() {
        // Field names are serialized as-is (no renames): the wire contract a
        // real Apple Music payload must satisfy.
        assert_serializes_as(
            sample_artist(),
            json!({ "id": "artist-1", "name": "The Sample Band" }),
        );
    }

    #[test]
    fn album_serializes_field_names_and_round_trips() {
        // As with Artist: field names are serialized as-is (no renames), the
        // wire contract a real Apple Music payload must satisfy. The
        // round-trip alone passes for *any* field names, so the exact JSON
        // shape is pinned before it.
        assert_serializes_as(
            sample_album(),
            json!({ "id": "album-1", "title": "First Record", "artist_id": "artist-1" }),
        );
    }

    #[test]
    fn song_serializes_field_names_and_round_trips() {
        // The Song twin of the Album and Artist field-name pins.
        assert_serializes_as(
            sample_song(),
            json!({ "id": "song-1", "title": "Opening", "album_id": "album-1" }),
        );
    }

    #[test]
    fn deserialization_ignores_unknown_fields() {
        // Real Apple Music payloads carry more than the model's fields; serde's
        // default must tolerate the extras rather than failing the whole parse.
        assert_unknown_fields_tolerated::<Artist>(
            json!({
                "id": "artist-1",
                "name": "The Sample Band",
                "genres": ["rock"]
            }),
            sample_artist(),
        );
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
        assert_unknown_fields_tolerated::<Album>(
            json!({
                "id": "album-1",
                "title": "First Record",
                "artist_id": "artist-1",
                "release_date": "2026-01-01"
            }),
            sample_album(),
        );
    }

    #[test]
    fn song_deserialization_ignores_unknown_fields() {
        assert_unknown_fields_tolerated::<Song>(
            json!({
                "id": "song-1",
                "title": "Opening",
                "album_id": "album-1",
                "duration_ms": 210_000
            }),
            sample_song(),
        );
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
