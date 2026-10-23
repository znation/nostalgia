//! The music-library data model — `Artist`, `Album`, `Song`, and `Playlist` —
//! shared by the Apple Music service and the UI.
//!
//! The types live outside `apple_music` because both the integration seam
//! (which populates them) and the presentation layer (which renders them)
//! depend on them; the service module alone is not their owner. Keeping them
//! here, beside the shared `state` module, lets the service answer from either
//! the REST library or the sample stub without touching the model or the UI.

use std::borrow::Cow;

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
    /// The performing artist's display name; `""` means the source supplied
    /// no artist. Album rows render `<artist> - <title>` when it is non-empty.
    #[serde(default)]
    pub artist: String,
}

impl Album {
    /// The text an album row renders: `<artist> - <title>` when the source
    /// supplied an artist, and just the title when [`Self::artist`] is empty.
    /// Presentation-only; the serialized model is unchanged. The title is
    /// borrowed in the artist-less case so a view refresh of a large library
    /// does not allocate for it.
    pub fn display_title(&self) -> Cow<'_, str> {
        display_title_for(&self.artist, &self.title)
    }
}

/// A user-created playlist in the library.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    /// The playlist's stable identifier; the browse query that loads its
    /// songs takes it.
    pub id: String,
    /// The display name the Playlists view renders.
    pub name: String,
}

/// A song on an [`Album`], linked to it by [`Song::album_id`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Song {
    /// The song's stable identifier; the shared `AppState` records it as the
    /// current track when the song plays.
    pub id: String,
    /// The track's title. The Songs view combines it with [`Self::artist`]
    /// through [`Self::display_title`] for its rows, and the Now Playing bar
    /// renders it beside the artist rather than on its own.
    pub title: String,
    /// The performing artist's display name; `""` means the source supplied
    /// no artist. Browse song rows and the Now Playing bar render
    /// `<artist> - <title>` when it is non-empty.
    pub artist: String,
    /// The [`Album::id`] of the album this song belongs to.
    pub album_id: String,
    /// The track's length in milliseconds; `0` means the source supplied no
    /// duration.
    pub duration_ms: u64,
    /// The preview asset URL the audio output plays; `None` when the source
    /// supplied none. `#[serde(default)]` keeps a payload without the field
    /// deserializable, as an older or partial response may omit it.
    #[serde(default)]
    pub preview_url: Option<String>,
}

impl Song {
    /// The text a browse row renders for this song: `<artist> - <title>` when
    /// the source supplied an artist, and just the title when
    /// [`Self::artist`] is empty. Presentation-only; the serialized model is
    /// unchanged. The title is borrowed in the artist-less case so a view
    /// refresh of a large library does not allocate for it.
    pub fn display_title(&self) -> Cow<'_, str> {
        display_title_for(&self.artist, &self.title)
    }
}

/// Joins an artist and title into the browse row's display text: the title alone
/// when the artist is empty, else `<artist> - <title>`. Shared by
/// [`Album::display_title`], [`Song::display_title`], and the Now Playing
/// marquee so they stay in sync.
pub fn display_title_for<'a>(artist: &'a str, title: &'a str) -> Cow<'a, str> {
    if artist.is_empty() {
        Cow::Borrowed(title)
    } else {
        Cow::Owned(format!("{} - {}", artist, title))
    }
}

#[cfg(test)]
mod tests {
    use super::{Album, Artist, Playlist, Song};
    use crate::test_support::{
        PREVIEW_URL, assert_every_field_required, assert_serializes_as,
        assert_unknown_fields_tolerated, sample_album, sample_artist, sample_playlist, sample_song,
    };
    use serde_json::json;

    /// The full, valid JSON wire payload for an [`Artist`]: the hand-written
    /// shape a real Apple Music response would carry. Used both to pin the
    /// serialized field names (`artist_serializes_field_names_and_round_trips`)
    /// and as the full payload the missing-field probe omits one key from.
    /// Hand-written rather than derived from [`sample_artist`], so the
    /// serialized field names stay pinned: a `#[serde(rename)]` would change
    /// `to_value(sample_artist())` but not this literal.
    fn artist_payload() -> serde_json::Value {
        json!({ "id": "artist-1", "name": "The Sample Band" })
    }

    /// The [`Album`] twin of [`artist_payload`]: the full, valid wire payload
    /// the album round-trip and missing-field tests share.
    fn album_payload() -> serde_json::Value {
        json!({ "id": "album-1", "title": "First Record", "artist_id": "artist-1", "artist": "The Sample Band" })
    }

    /// The [`Playlist`] twin of [`artist_payload`]: the full, valid wire
    /// payload the playlist field-name and missing-field tests share.
    fn playlist_payload() -> serde_json::Value {
        json!({ "id": "playlist-1", "name": "Morning Drive" })
    }

    /// The [`Song`] twin of [`artist_payload`]: the full, valid wire payload
    /// the song field-name and round-trip test pins. It carries the optional
    /// `preview_url` so the serialized field name is pinned too; the payload
    /// the missing-field probe walks is [`song_required_fields_payload`],
    /// because an optional field is legitimately omittable.
    fn song_payload() -> serde_json::Value {
        json!({
            "id": "song-1",
            "title": "Opening",
            "artist": "The Sample Band",
            "album_id": "album-1",
            "duration_ms": 210_000,
            "preview_url": null
        })
    }

    /// The [`Song`] payload carrying every required field and omitting the
    /// optional `preview_url`, so [`assert_every_field_required`] can remove
    /// each key and see the parse fail. Passing [`song_payload`] instead would
    /// clear the probe for `preview_url`: a `#[serde(default)]` field is
    /// allowed to be missing.
    fn song_required_fields_payload() -> serde_json::Value {
        json!({ "id": "song-1", "title": "Opening", "artist": "The Sample Band", "album_id": "album-1", "duration_ms": 210_000 })
    }

    /// The REST client maps Apple Music JSON into `Artist`, `Album`, `Song`,
    /// and `Playlist` through its own private `Resource` types (the
    /// `into_artist`, `into_album`, `into_song`, and `into_playlist` methods),
    /// so the round-trip (out and back through `serde_json`) pins this
    /// model's serialized field names.
    #[test]
    fn artist_serializes_field_names_and_round_trips() {
        // Field names are serialized as-is (no renames): the wire contract a
        // real Apple Music payload must satisfy.
        assert_serializes_as(sample_artist(), artist_payload());
    }

    #[test]
    fn album_serializes_field_names_and_round_trips() {
        // As with Artist: field names are serialized as-is (no renames), the
        // wire contract a real Apple Music payload must satisfy. The
        // round-trip alone passes for *any* field names, so the exact JSON
        // shape is pinned before it.
        assert_serializes_as(sample_album(), album_payload());
    }

    #[test]
    fn song_serializes_field_names_and_round_trips() {
        // The Song twin of the Album and Artist field-name pins.
        assert_serializes_as(sample_song(), song_payload());
    }

    #[test]
    fn playlist_serializes_field_names_and_round_trips() {
        // As with Artist and Album: field names are serialized as-is, the
        // wire contract a real Apple Music payload must satisfy.
        assert_serializes_as(sample_playlist(), playlist_payload());
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
    // it for `Artist` only, so `Album`'s, `Song`'s, and `Playlist`'s serde
    // "ignore an unknown field" arms would otherwise go unexercised. Real
    // Apple Music payloads carry more than the model's fields, so a
    // `#[serde(deny_unknown_fields)]` added to any of them would fail the
    // whole parse while clearing every existing test (none deserialize these
    // types with an extra field). Pin the same tolerance for all three, as
    // the missing-field tests below do per-type.
    #[test]
    fn album_deserialization_ignores_unknown_fields() {
        assert_unknown_fields_tolerated::<Album>(
            json!({
                "id": "album-1",
                "title": "First Record",
                "artist_id": "artist-1",
                "artist": "The Sample Band",
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
                "artist": "The Sample Band",
                "album_id": "album-1",
                "duration_ms": 210_000,
                "genre": "rock"
            }),
            sample_song(),
        );
    }

    #[test]
    fn playlist_deserialization_ignores_unknown_fields() {
        assert_unknown_fields_tolerated::<Playlist>(
            json!({
                "id": "playlist-1",
                "name": "Morning Drive",
                "curator_name": "The Sample Band"
            }),
            sample_playlist(),
        );
    }

    #[test]
    fn deserialization_rejects_missing_required_fields() {
        // A payload missing any required field must error, not silently yield
        // a half-populated model the UI would render as blank data.
        assert_every_field_required::<Artist>(artist_payload());
    }

    // The missing-field contract holds for every model type, not just Artist:
    // the other model types' tests all supply every field, so a regression
    // that made one of their fields optional (e.g. a stray `#[serde(default)]`
    // added to tolerate a payload variant) would clear them while silently
    // rendering blank data. `Album`, `Song`, and `Playlist` each get the same
    // every-field probe as Artist.
    /// The [`Album`] payload carrying every required field and omitting the
    /// optional `artist`, so [`assert_every_field_required`] can remove each
    /// key and see the parse fail.
    fn album_required_fields_payload() -> serde_json::Value {
        json!({ "id": "album-1", "title": "First Record", "artist_id": "artist-1" })
    }

    #[test]
    fn album_deserialization_rejects_missing_required_fields() {
        assert_every_field_required::<Album>(album_required_fields_payload());
    }

    #[test]
    fn album_deserialization_defaults_a_missing_artist_to_empty() {
        let album: Album = serde_json::from_value(album_required_fields_payload()).unwrap();
        assert_eq!(album.artist, "");
    }

    #[test]
    fn song_deserialization_rejects_missing_required_fields() {
        assert_every_field_required::<Song>(song_required_fields_payload());
    }

    #[test]
    fn playlist_deserialization_rejects_missing_required_fields() {
        assert_every_field_required::<Playlist>(playlist_payload());
    }

    #[test]
    fn song_deserialization_defaults_a_missing_preview_url_to_none() {
        // The browse response may omit `previews`, and an older or partial
        // payload may omit `preview_url`; the model must still deserialize to
        // the `None` "no preview" state rather than reject the whole song.
        let parsed: Song = serde_json::from_value(song_required_fields_payload())
            .expect("a Song without preview_url must deserialize");
        assert_eq!(parsed.preview_url, None);
    }

    #[test]
    fn album_display_title_joins_the_artist_and_title() {
        assert_eq!(
            sample_album().display_title(),
            "The Sample Band - First Record"
        );
        assert!(matches!(
            sample_album().display_title(),
            std::borrow::Cow::Owned(_)
        ));
    }

    #[test]
    fn album_display_title_borrows_the_title_when_the_artist_is_empty() {
        let mut album = sample_album();
        album.artist = String::new();
        assert_eq!(album.display_title(), "First Record");
        assert!(matches!(
            album.display_title(),
            std::borrow::Cow::Borrowed(_)
        ));
    }

    #[test]
    fn display_title_joins_the_artist_and_title() {
        // The sample song carries both fields, so the row text reads like the
        // mockup's playlist rows rather than a bare title.
        assert_eq!(sample_song().display_title(), "The Sample Band - Opening");
        assert!(matches!(
            sample_song().display_title(),
            std::borrow::Cow::Owned(_)
        ));
    }

    #[test]
    fn display_title_borrows_the_title_when_the_artist_is_empty() {
        // An artist-less song must not render a dangling `" - "` prefix; it
        // renders its title alone, borrowed rather than reallocated.
        let mut song = sample_song();
        song.artist = String::new();
        assert_eq!(song.display_title(), "Opening");
        assert!(matches!(
            song.display_title(),
            std::borrow::Cow::Borrowed(_)
        ));
    }

    #[test]
    fn song_deserializes_a_preview_url() {
        let parsed: Song = serde_json::from_value(json!({
            "id": "song-1",
            "title": "Opening",
            "artist": "The Sample Band",
            "album_id": "album-1",
            "duration_ms": 210_000,
            "preview_url": PREVIEW_URL
        }))
        .unwrap();
        assert_eq!(parsed.preview_url, Some(PREVIEW_URL.to_string()));
    }
}
