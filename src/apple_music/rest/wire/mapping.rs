//! Mapping decoded Apple Music resources onto the library model: each
//! resource becomes an [`Artist`], [`Album`], [`Playlist`], or [`Song`], and
//! the fallbacks below turn an absent or unusable wire field into the model's
//! "source supplied none" sentinel instead of failing the whole response.
//!
//! The wire shapes themselves, and the parsing that decodes them, live in the
//! parent [`wire`](super) module.

use super::super::AppleMusicError;
use super::Resource;
use crate::library::{Album, Artist, ContentRating, Playlist, Song};

impl Resource {
    /// Whether Apple typed this resource `library-albums`, the only type
    /// [`Resource::into_album`] maps in a mixed-type collection.
    pub(in crate::apple_music::rest) fn is_library_album(&self) -> bool {
        self.kind.as_deref() == Some("library-albums")
    }

    /// Whether Apple typed this resource `songs` or `library-songs`, the two types
    /// `RestLibrary::get_recently_played_songs` maps.
    pub(in crate::apple_music::rest) fn is_song(&self) -> bool {
        matches!(self.kind.as_deref(), Some("songs" | "library-songs"))
    }

    /// Maps this resource to an [`Artist`], moving its id and name into the
    /// model instead of cloning them out of the parsed response.
    ///
    /// The id and name are required (see [`required_id`] and [`required_name`]): a blank id can
    /// name nothing, and a nameless resource would render as an empty row.
    pub(in crate::apple_music::rest) fn into_artist(self) -> Result<Artist, AppleMusicError> {
        let Resource { id, attributes, .. } = self;
        let id = required_id(id, "artist")?;
        let name = required_name(
            attributes.and_then(|attributes| attributes.name),
            &id,
            "artist",
        )?;
        Ok(Artist {
            id,
            name,
            album_count: 0,
        })
    }

    /// Maps this resource to an [`Album`] linked to `artist_id`, moving its id
    /// and title into the model (see [`Self::into_artist`]).
    ///
    /// The per-artist album query scopes its results to `artist_id`, so it
    /// passes the id it queried; the all-albums collection carries no artist
    /// id, so that caller passes `""` (see
    /// [`RestLibrary::get_all_albums`](super::super::RestLibrary::get_all_albums)).
    /// Either way the mapped album carries the argument as its `artist_id`.
    /// Its id and name are required, like an artist's.
    pub(in crate::apple_music::rest) fn into_album(
        self,
        artist_id: &str,
    ) -> Result<Album, AppleMusicError> {
        let Resource { id, attributes, .. } = self;
        let id = required_id(id, "album")?;
        let (title, artist_name, release_date) = match attributes {
            Some(attributes) => (
                attributes.name,
                attributes.artist_name,
                attributes.release_date,
            ),
            None => (None, None, None),
        };
        let title = required_name(title, &id, "album")?;
        let artist = artist_or_empty(artist_name);
        Ok(Album {
            id,
            title,
            artist_id: artist_id.to_string(),
            artist,
            duration_ms: 0,
            year: release_date.as_deref().and_then(release_year),
        })
    }

    /// Maps this resource to the `(album title, artist display name, duration)`
    /// triple the whole-library duration map groups by, or `None` when it
    /// carries no non-blank `albumName`.
    ///
    /// A resource without an album name cannot be attributed to an album, so
    /// the caller skips it rather than failing the whole songs request. The
    /// artist uses [`Album::artist`]'s `""` "source supplied no artist"
    /// sentinel, and a missing duration is `0`, matching [`Self::into_song`].
    pub(in crate::apple_music::rest) fn into_album_duration(self) -> Option<(String, String, u64)> {
        let attributes = self.attributes?;
        let album_name = attributes
            .album_name
            .filter(|name| !name.trim().is_empty())?;
        let artist = artist_or_empty(attributes.artist_name);
        Some((
            album_name,
            artist,
            duration_millis(attributes.duration_in_millis),
        ))
    }

    /// Maps this resource to a [`Playlist`], moving its id and name into the
    /// model (see [`Self::into_artist`]).
    ///
    /// A playlist's id and name are required, like an artist's: a blank id
    /// can name nothing and a nameless playlist would render as an empty row.
    pub(in crate::apple_music::rest) fn into_playlist(self) -> Result<Playlist, AppleMusicError> {
        let Resource { id, attributes, .. } = self;
        let id = required_id(id, "playlist")?;
        let name = required_name(
            attributes.and_then(|attributes| attributes.name),
            &id,
            "playlist",
        )?;
        Ok(Playlist { id, name })
    }

    /// Maps this resource to a [`Song`] linked to `album_id`, moving its id,
    /// title, artist, album name, primary genre, track number, disc number, and
    /// preview URL into the model instead of cloning them.
    ///
    /// The all-songs, playlist-track, and search collections carry no album id, so those callers
    /// pass `""`; only an album's tracks pass a real id (see
    /// [`RestLibrary::get_all_songs`](super::super::RestLibrary::get_all_songs),
    /// [`get_songs_from_playlist`](super::super::RestLibrary::get_songs_from_playlist), and
    /// [`RestLibrary::search_library`](super::super::RestLibrary::search_library)). The id and
    /// title are required (see [`required_id`] and [`required_name`]); the artist, album name,
    /// genre, track number, disc number, composer, content rating, duration, and preview URL fall
    /// back to their "source supplied none" sentinels. The whole-library path maps up to
    /// [`MAX_PAGES`](super::super::paging::MAX_PAGES) pages of songs, so consuming the parsed
    /// resource avoids re-allocating every string it already owns.
    pub(in crate::apple_music::rest) fn into_song(
        self,
        album_id: &str,
    ) -> Result<Song, AppleMusicError> {
        let Resource { id, attributes, .. } = self;
        let id = required_id(id, "song")?;
        let (
            title,
            artist,
            album,
            genre_names,
            release_date,
            track_number,
            disc_number,
            composer_name,
            content_rating,
            duration_ms,
            previews,
        ) = match attributes {
            Some(attributes) => (
                attributes.name,
                attributes.artist_name,
                attributes.album_name,
                attributes.genre_names,
                attributes.release_date,
                attributes.track_number,
                attributes.disc_number,
                attributes.composer_name,
                attributes.content_rating,
                attributes.duration_in_millis,
                attributes.previews,
            ),
            None => (
                None, None, None, None, None, None, None, None, None, None, None,
            ),
        };
        let title = required_name(title, &id, "song")?;
        let artist = artist_or_empty(artist);
        // An absent or whitespace-only album name is the same "source supplied
        // none" sentinel the artist uses, so the File info panel can leave the
        // album line out.
        let album = album
            .filter(|album| !album.trim().is_empty())
            .unwrap_or_default();
        let genre = primary_genre(genre_names);
        // Like the album, an absent or whitespace-only composer is the
        // "source supplied none" sentinel.
        let composer = composer_name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_default();
        // A blank URL (empty or only whitespace) can play nothing, so skip it
        // in favor of a later preview; a non-blank URL is returned exactly as
        // the resource carried it.
        let preview_url = previews.and_then(|previews| {
            previews
                .into_iter()
                .find_map(|preview| preview.url.filter(|url| !url.trim().is_empty()))
        });
        Ok(Song {
            id,
            title,
            artist,
            album,
            year: release_date.as_deref().and_then(release_year),
            genre,
            track_number: positive_position(track_number),
            disc_number: positive_position(disc_number),
            composer,
            content_rating: content_rating_from(content_rating),
            album_id: album_id.to_string(),
            duration_ms: duration_millis(duration_ms),
            preview_url,
        })
    }
}

/// Maps the raw `contentRating` to a [`ContentRating`], ignoring ASCII case and surrounding
/// whitespace; an absent, blank, or unknown value is `None`.
fn content_rating_from(raw: Option<String>) -> Option<ContentRating> {
    let raw = raw?;
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case("explicit") {
        Some(ContentRating::Explicit)
    } else if raw.eq_ignore_ascii_case("clean") {
        Some(ContentRating::Clean)
    } else {
        None
    }
}

/// The usable 1-based position (track or disc number) an optional JSON number carries.
///
/// An API-reported non-positive number is not a usable position, so it maps to the same `None`
/// sentinel an absent number does. A value too large for the model's `u32` — or even outside
/// `i64` — is just as unusable, and must not fail the whole response either.
fn positive_position(number: Option<serde_json::Number>) -> Option<u32> {
    number
        .and_then(|number| number.as_u64())
        .filter(|number| *number > 0)
        .and_then(|number| u32::try_from(number).ok())
}

/// The usable length in milliseconds an optional `durationInMillis` carries.
///
/// An absent duration, a non-positive one, a fractional one, or one too large for `u64` is not a
/// length any track can have, so it maps to `0` — the `Song::duration_ms` and `Album::duration_ms`
/// "source supplied no duration" sentinel. One unusable value therefore never fails a whole
/// collection response. Shared by [`Resource::into_song`] and [`Resource::into_album_duration`] so
/// both drop a useless length the same way.
fn duration_millis(duration: Option<serde_json::Number>) -> u64 {
    duration.and_then(|number| number.as_u64()).unwrap_or(0)
}

/// The release year at the start of an Apple Music `releaseDate`, or `None`
/// when the date is absent or malformed.
///
/// Apple documents `releaseDate` as `YYYY-MM-DD` but also returns `YYYY` and `YYYY-MM`, so only the
/// first four bytes must be ASCII digits; the rest of the date does not affect the year. A blank,
/// short, non-numeric, or `0000`-dated source has no usable year and maps to `None`, matching
/// [`Song::year`]'s "source supplied none" sentinel. The date is trimmed first so surrounding
/// whitespace does not hide a valid year.
fn release_year(date: &str) -> Option<u16> {
    let year = date.trim().get(..4)?;
    match year.parse::<u16>() {
        // `0000` names no real calendar year, so a date carrying it is as
        // malformed as a non-numeric one.
        Ok(0) | Err(_) => None,
        Ok(year) => Some(year),
    }
}

/// The artist display name an optional `artistName` carries, mapping an absent
/// or whitespace-only name to `""` — [`Album::artist`]'s and [`Song::artist`]'s
/// "source supplied no artist" sentinel. Shared by [`Resource::into_album`],
/// [`Resource::into_album_duration`], and [`Resource::into_song`] so all three
/// drop a useless name the same way.
fn artist_or_empty(name: Option<String>) -> String {
    name.filter(|name| !name.trim().is_empty())
        .unwrap_or_default()
}

/// The first non-blank genre name an optional `genreNames` array carries, trimmed, or `""` when the
/// array is absent, empty, or all blank — [`Song::genre`]'s "source supplied none" sentinel. Apple
/// lists an umbrella genre alongside the specific one (`["Rock", "Music"]`), so only the first real
/// entry is kept; skipping blank entries means a stray `""` never hides a later real genre.
fn primary_genre(genres: Option<Vec<String>>) -> String {
    genres
        .into_iter()
        .flatten()
        .find_map(|genre| {
            let genre = genre.trim();
            (!genre.is_empty()).then(|| genre.to_string())
        })
        .unwrap_or_default()
}

/// The non-blank id `id`, or an [`AppleMusicError`] naming `kind`.
///
/// The id names the resource: the model links rows together by it, and the
/// seam later uses it to browse or play. A blank id (empty or only whitespace)
/// can name nothing, so the response is rejected rather than mapped to a row
/// the UI would render but that errors when pressed — the same reason
/// [`required_name`] rejects a blank name.
fn required_id(id: String, kind: &str) -> Result<String, AppleMusicError> {
    if id.trim().is_empty() {
        return Err(AppleMusicError::new(format!(
            "response carried {kind} with a blank id (got {id:?})"
        )));
    }
    Ok(id)
}

/// The non-blank name `name`, or an [`AppleMusicError`] naming `kind` and the
/// resource's `id`.
///
/// A resource whose `attributes.name` is absent or only whitespace is rejected
/// rather than mapped to a blank row the UI would render empty. `id` is only
/// used in the report, so it can be the already-validated id.
fn required_name(name: Option<String>, id: &str, kind: &str) -> Result<String, AppleMusicError> {
    match name {
        Some(name) if !name.trim().is_empty() => Ok(name),
        _ => Err(AppleMusicError::new(format!(
            "response carried {kind} {id:?} without a name"
        ))),
    }
}
