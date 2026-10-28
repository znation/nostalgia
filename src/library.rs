//! The music-library data model — `Artist`, `Album`, `Song`, and `Playlist` —
//! shared by the Apple Music service and the UI.
//!
//! The types live outside `apple_music` because both the integration seam
//! (which populates them) and the presentation layer (which renders them)
//! depend on them; the service module alone is not their owner. Keeping them
//! here, beside the shared `state` module, lets the service answer from either
//! the REST library or the sample stub without touching the model or the UI.

use std::borrow::Cow;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// An artist in the user's library.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artist {
    /// The artist's stable identifier; an [`Album::artist_id`] refers to it.
    pub id: String,
    /// The display name the Artists view renders.
    pub name: String,
    /// The number of albums the loaded library holds for this artist, and `0`
    /// when it holds none. Derived display data rather than an Apple Music
    /// field, so it is skipped by serde and deserializes to `0`; the service
    /// fills it from the library's albums.
    #[serde(skip)]
    pub album_count: usize,
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
    /// The summed duration of the songs the library reports for this album in
    /// milliseconds, and `0` when it reports none. Derived display data
    /// rather than an Apple Music field, so it is skipped by serde and
    /// deserializes to `0`; the service fills it from the library's songs.
    #[serde(skip)]
    pub duration_ms: u64,
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
    /// The album's title; `""` means the source supplied none. The File info
    /// panel draws an `Album:` line only when it is non-empty.
    /// `#[serde(default)]` keeps a payload without the field deserializable,
    /// as an older or partial response may omit it.
    #[serde(default)]
    pub album: String,
    /// The track's release year, parsed from the source's release date; `None`
    /// when the source supplied none or it was malformed. The File info panel
    /// draws a `Year:` line only while it is `Some`. `#[serde(default)]` keeps
    /// a payload without the field deserializable, as an older or partial
    /// response may omit it.
    #[serde(default)]
    pub year: Option<u16>,
    /// The track's primary genre name; `""` means the source supplied none
    /// (the same sentinel [`Self::album`] uses). The File info panel draws a
    /// `Genre:` line only when it is non-empty. `#[serde(default)]` keeps a
    /// payload without the field deserializable, as an older or partial
    /// response may omit it.
    #[serde(default)]
    pub genre: String,
    /// The track's 1-based position on its album; `None` when the source
    /// supplied none or reported a non-positive number. The File info panel
    /// draws a `Track #:` line only while it is `Some`. `#[serde(default)]`
    /// keeps a payload without the field deserializable, as an older or
    /// partial response may omit it.
    #[serde(default)]
    pub track_number: Option<u32>,
    /// The track's 1-based disc in its album's set; `None` when the source
    /// supplied none or reported a non-positive number. The File info panel
    /// draws a `Disc #:` line only while it is `Some`. `#[serde(default)]`
    /// keeps a payload without the field deserializable, as an older or
    /// partial response may omit it.
    #[serde(default)]
    pub disc_number: Option<u32>,
    /// The track's composer; `""` means the source supplied none (the same
    /// sentinel [`Self::album`] uses). The File info panel draws a `Composer:`
    /// line only when it is non-empty. `#[serde(default)]` keeps a payload
    /// without the field deserializable, as an older or partial response may
    /// omit it.
    #[serde(default)]
    pub composer: String,
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

/// Counts the albums each artist display name owns, skipping albums whose
/// [`Album::artist`] is empty (the source supplied no artist). The returned
/// map borrows its keys from `albums`, so callers can match an
/// [`Artist::name`] against it without allocating. This is the display-name
/// link a signed-in artists load uses because `get_all_albums` deliberately
/// leaves [`Album::artist_id`] empty.
pub fn album_counts_by_artist(albums: &[Album]) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    for album in albums {
        if !album.artist.is_empty() {
            *counts.entry(album.artist.as_str()).or_insert(0) += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::{Album, Artist, Playlist, Song, album_counts_by_artist};
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
            "album": "First Record",
            "year": null,
            "genre": "",
            "track_number": null,
            "disc_number": null,
            "composer": "",
            "album_id": "album-1",
            "duration_ms": 210_000,
            "preview_url": null
        })
    }

    /// The [`Song`] payload carrying every required field and omitting the
    /// optional `preview_url`, `album`, `year`, `genre`, and `track_number`, so
    /// [`assert_every_field_required`] can remove each key and see the parse
    /// fail. Passing [`song_payload`] instead would clear the probe for those
    /// fields: a `#[serde(default)]` field is allowed to be missing.
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
    fn artist_album_count_is_not_serialized_and_defaults_to_zero() {
        // `album_count` is derived display data, not an Apple Music field: a
        // nonzero count must not leak into the wire payload, and a payload
        // without the field must deserialize to the `0` no-albums default.
        let mut artist = sample_artist();
        artist.album_count = 3;
        assert_eq!(serde_json::to_value(&artist).unwrap(), artist_payload());
        let parsed: Artist = serde_json::from_value(artist_payload()).unwrap();
        assert_eq!(parsed.album_count, 0);
    }

    #[test]
    fn album_counts_by_artist_counts_non_empty_names() {
        let mut first = sample_album();
        first.artist = "The Sample Band".to_string();
        let mut second = first.clone();
        second.id = "album-2".to_string();
        let mut third = first.clone();
        third.id = "album-3".to_string();
        third.artist = "Echo Chamber".to_string();
        let mut nameless = first.clone();
        nameless.id = "album-4".to_string();
        nameless.artist = String::new();

        let albums = vec![first, second, third, nameless];
        let counts = album_counts_by_artist(&albums);

        assert_eq!(counts.get("The Sample Band"), Some(&2));
        assert_eq!(counts.get("Echo Chamber"), Some(&1));
        assert_eq!(counts.get("Mono Tones"), None);
        assert_eq!(counts.len(), 2);
    }

    #[test]
    fn album_duration_is_not_serialized_and_defaults_to_zero() {
        // `duration_ms` is derived display data, not an Apple Music field: a
        // nonzero total must not leak into the wire payload, and a payload
        // without the field must deserialize to the `0` no-songs default.
        let mut album = sample_album();
        album.duration_ms = 630_000;
        assert_eq!(serde_json::to_value(&album).unwrap(), album_payload());
        let parsed: Album = serde_json::from_value(album_payload()).unwrap();
        assert_eq!(parsed.duration_ms, 0);
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
                "album": "First Record",
                "album_id": "album-1",
                "duration_ms": 210_000,
                "isrc": "US-ABC-01-00001"
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
    fn song_deserialization_defaults_a_missing_album_to_empty() {
        // An older or partial payload may omit `album`; the model must still
        // deserialize to the `""` "source supplied no album" sentinel.
        let parsed: Song = serde_json::from_value(song_required_fields_payload())
            .expect("a Song without album must deserialize");
        assert_eq!(parsed.album, "");
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
    fn song_round_trips_its_year() {
        // The release year is part of the serialized model, not derived
        // display data: a `Some(2001)` year must survive a round trip and
        // serialize under the `year` key.
        let mut song = sample_song();
        song.year = Some(2001);
        let payload = serde_json::to_value(&song).unwrap();
        assert_eq!(payload["year"], json!(2001));
        let parsed: Song = serde_json::from_value(payload).unwrap();
        assert_eq!(parsed.year, Some(2001));
    }

    #[test]
    fn song_year_defaults_to_none_when_the_payload_omits_it() {
        // An older or partial response without `year` must parse, mapping to
        // the `None` "source supplied none" sentinel.
        let parsed: Song = serde_json::from_value(song_required_fields_payload()).unwrap();
        assert_eq!(parsed.year, None);
    }

    #[test]
    fn song_round_trips_its_genre() {
        // The primary genre is part of the serialized model, not derived
        // display data: a `"Rock"` genre must survive a round trip and
        // serialize under the `genre` key.
        let mut song = sample_song();
        song.genre = "Rock".to_string();
        let payload = serde_json::to_value(&song).unwrap();
        assert_eq!(payload["genre"], json!("Rock"));
        let parsed: Song = serde_json::from_value(payload).unwrap();
        assert_eq!(parsed.genre, "Rock");
    }

    #[test]
    fn song_genre_defaults_to_empty_when_the_payload_omits_it() {
        // An older or partial response without `genre` must parse, mapping to
        // the `""` "source supplied none" sentinel.
        let parsed: Song = serde_json::from_value(song_required_fields_payload()).unwrap();
        assert_eq!(parsed.genre, "");
    }

    #[test]
    fn song_round_trips_its_track_number() {
        // The album position is part of the serialized model, not derived
        // display data: a `Some(3)` number must survive a round trip and
        // serialize under the `track_number` key.
        let mut song = sample_song();
        song.track_number = Some(3);
        let payload = serde_json::to_value(&song).unwrap();
        assert_eq!(payload["track_number"], json!(3));
        let parsed: Song = serde_json::from_value(payload).unwrap();
        assert_eq!(parsed.track_number, Some(3));
    }

    #[test]
    fn song_track_number_defaults_to_none_when_the_payload_omits_it() {
        // An older or partial response without `track_number` must parse,
        // mapping to the `None` "source supplied none" sentinel.
        let parsed: Song = serde_json::from_value(song_required_fields_payload()).unwrap();
        assert_eq!(parsed.track_number, None);
    }

    #[test]
    fn song_round_trips_its_disc_number() {
        // The disc is part of the serialized model: a `Some(2)` number must
        // survive a round trip and serialize under the `disc_number` key.
        let mut song = sample_song();
        song.disc_number = Some(2);
        let payload = serde_json::to_value(&song).unwrap();
        assert_eq!(payload["disc_number"], json!(2));
        let parsed: Song = serde_json::from_value(payload).unwrap();
        assert_eq!(parsed.disc_number, Some(2));
    }

    #[test]
    fn song_disc_number_defaults_to_none_when_the_payload_omits_it() {
        // An older or partial response without `disc_number` must parse,
        // mapping to the `None` "source supplied none" sentinel.
        let parsed: Song = serde_json::from_value(song_required_fields_payload()).unwrap();
        assert_eq!(parsed.disc_number, None);
    }

    #[test]
    fn song_round_trips_its_composer() {
        // The composer is part of the serialized model: it must survive a
        // round trip and serialize under the `composer` key.
        let mut song = sample_song();
        song.composer = "Jane Doe".to_string();
        let payload = serde_json::to_value(&song).unwrap();
        assert_eq!(payload["composer"], json!("Jane Doe"));
        let parsed: Song = serde_json::from_value(payload).unwrap();
        assert_eq!(parsed.composer, "Jane Doe");
    }

    #[test]
    fn song_composer_defaults_to_empty_when_the_payload_omits_it() {
        // An older or partial response without `composer` must parse, mapping
        // to the `""` "source supplied none" sentinel.
        let parsed: Song = serde_json::from_value(song_required_fields_payload()).unwrap();
        assert_eq!(parsed.composer, "");
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
