//! The Apple Music REST browse client: turns a `MusicKit` session plus the
//! Apple Music REST API into the model's [`Artist`], [`Album`], [`Playlist`],
//! and [`Song`] lists.
//!
//! The client is synchronous and network-isolated behind [`HttpTransport`], so
//! its tests drive an in-memory stub instead of a live endpoint. The service
//! (`crate::apple_music`) decides when to call it, from the browse, search,
//! and playlist queries, once a `MusicKit` session is stored.
//!
//! `transport` holds the HTTP seam and its `ureq` implementation, `wire` holds
//! the API's response shapes and the parsing that decodes them, `wire::mapping`
//! translates a decoded resource onto the model, `paging` holds the paginated
//! fetch and the URL helpers, and this module holds the query methods that tie
//! them together.

use std::collections::HashMap;

use super::AppleMusicError;
use crate::library::{Album, Artist, Playlist, Song};
use crate::music_kit_auth::MusicKitSession;

mod paging;
mod transport;
mod wire;

use paging::encode_path_segment;
pub use transport::{HttpTransport, UreqTransport};
use wire::{Resource, parse_search_envelope};

/// The Apple Music REST API root. The library endpoints are storefront-free,
/// so no storefront is fetched.
const API_BASE: &str = "https://api.music.apple.com/v1";

/// The page size [`RestLibrary::search_library`] asks for. Apple documents the
/// library-search `limit` maximum as 25 (default 5), so the request asks for
/// the documented maximum: a larger value is out of contract and can fail the
/// whole search.
const SEARCH_PAGE_LIMIT: usize = 25;

/// A library browse client: issues the library browse and search queries
/// over [`HttpTransport`] and maps each response onto the model.
pub struct RestLibrary {
    transport: Box<dyn HttpTransport>,
}

impl RestLibrary {
    /// Builds a client over `transport`.
    #[must_use]
    pub fn new(transport: Box<dyn HttpTransport>) -> Self {
        Self { transport }
    }

    /// All favorite artists in the signed-in user's library.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails, the response is
    /// not valid JSON, or a resource carries a blank id or no name.
    pub fn get_favorite_artists(
        &self,
        session: &MusicKitSession,
    ) -> Result<Vec<Artist>, AppleMusicError> {
        let url = format!("{API_BASE}/me/library/artists");
        self.fetch(&url, session, Resource::into_artist)
    }

    /// Albums by `artist_id` in the signed-in user's library.
    ///
    /// The query already scopes the results to `artist_id`, so every mapped
    /// [`Album`] carries it as its `artist_id`. The id is percent-encoded as
    /// one path segment, so an id carrying a reserved character cannot split
    /// the path or start a query.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails, the response is
    /// not valid JSON, or a resource carries a blank id or no name.
    pub fn get_albums_by_artist(
        &self,
        session: &MusicKitSession,
        artist_id: &str,
    ) -> Result<Vec<Album>, AppleMusicError> {
        let url = format!(
            "{API_BASE}/me/library/artists/{}/albums",
            encode_path_segment(artist_id)
        );
        self.fetch(&url, session, |resource| resource.into_album(artist_id))
    }

    /// Songs on `album_id` in the signed-in user's library.
    ///
    /// The query already scopes the results to `album_id`, so every mapped
    /// [`Song`] carries it as its `album_id`. The id is percent-encoded as one
    /// path segment, so an id carrying a reserved character cannot split the
    /// path or start a query.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails, the response is
    /// not valid JSON, or a resource carries a blank id or no name.
    pub fn get_songs_from_album(
        &self,
        session: &MusicKitSession,
        album_id: &str,
    ) -> Result<Vec<Song>, AppleMusicError> {
        let url = format!(
            "{API_BASE}/me/library/albums/{}/tracks",
            encode_path_segment(album_id)
        );
        self.fetch(&url, session, |resource| resource.into_song(album_id))
    }

    /// Every song in the signed-in user's library, in library order.
    ///
    /// The whole library is read through the same paginated collection
    /// helper the artist and album queries use, so a library larger than one
    /// page is read in full. A library song carries no album id, so every
    /// mapped [`Song`] leaves `album_id` empty.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails, the response is
    /// not valid JSON, or a resource carries a blank id or no name.
    pub fn get_all_songs(&self, session: &MusicKitSession) -> Result<Vec<Song>, AppleMusicError> {
        let url = format!("{API_BASE}/me/library/songs");
        self.fetch(&url, session, |resource| resource.into_song(""))
    }

    /// The summed duration of each album in the signed-in user's library,
    /// keyed by its exact `(album title, artist display name)` pair.
    ///
    /// This reads the whole-library songs collection through the same
    /// paginated helper as [`RestLibrary::get_all_songs`], because a library
    /// song carries its album's title in `attributes.albumName` rather than an
    /// album id, and it groups the songs by that title and their
    /// `attributes.artistName` (the `""` sentinel for a missing or blank
    /// artist). Durations are summed with `saturating_add`. A resource without
    /// a non-blank `albumName` cannot be attributed to an album and is skipped
    /// rather than failing the request; an album with no matching song is
    /// simply absent from the map, which callers read as `0`.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails or the response is
    /// not valid JSON.
    pub fn get_album_durations(
        &self,
        session: &MusicKitSession,
    ) -> Result<HashMap<(String, String), u64>, AppleMusicError> {
        let url = format!("{API_BASE}/me/library/songs");
        let songs = self.fetch(&url, session, |resource| Ok(resource.into_album_duration()))?;
        let mut durations: HashMap<(String, String), u64> = HashMap::new();
        for song in songs.into_iter().flatten() {
            let (album, artist, duration_ms) = song;
            let total = durations.entry((album, artist)).or_insert(0);
            *total = total.saturating_add(duration_ms);
        }
        Ok(durations)
    }

    /// Every album in the signed-in user's library, in library order.
    ///
    /// The whole library is read through the same paginated collection
    /// helper the artist and song queries use, so a library larger than one
    /// page is read in full. A library album carries no artist id in this
    /// collection response, so every mapped [`Album`] leaves `artist_id`
    /// empty — the same shape [`RestLibrary::get_all_songs`] gives its songs'
    /// `album_id`. The resource is mapped with [`Resource::into_album`], which
    /// moves its id and title into the model. Nothing that opens an all-albums
    /// row reads the artist id (`get_songs_from_album` uses only the album id).
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails, the response is
    /// not valid JSON, or a resource carries a blank id or no name.
    pub fn get_all_albums(&self, session: &MusicKitSession) -> Result<Vec<Album>, AppleMusicError> {
        let url = format!("{API_BASE}/me/library/albums");
        self.fetch(&url, session, |resource| resource.into_album(""))
    }

    /// The signed-in user's recently added library albums, newest first as Apple orders them.
    ///
    /// The `recently-added` endpoint mixes resource types, so only resources typed
    /// `library-albums` are mapped (with [`Resource::into_album`], leaving the artist id empty as
    /// [`RestLibrary::get_all_albums`] does); library playlists, catalog albums, stations and
    /// untyped resources are dropped. No `limit` is sent, because the endpoint's page size
    /// differs from the other collections'; later pages are followed through `next`.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails, the response is
    /// not valid JSON, or a library album carries a blank id or no name.
    pub fn get_recently_added_albums(
        &self,
        session: &MusicKitSession,
    ) -> Result<Vec<Album>, AppleMusicError> {
        let url = format!("{API_BASE}/me/library/recently-added");
        let albums = self.fetch(&url, session, |resource| {
            if resource.is_library_album() {
                resource.into_album("").map(Some)
            } else {
                Ok(None)
            }
        })?;
        Ok(albums.into_iter().flatten().collect())
    }

    /// Every playlist in the signed-in user's library, in library order.
    ///
    /// The whole library is read through the same paginated collection
    /// helper the other browse queries use, so a library larger than one page
    /// is read in full. The resource is mapped with
    /// [`Resource::into_playlist`], which moves its id and name into the
    /// model.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails, the response is
    /// not valid JSON, or a resource carries a blank id or no name.
    pub fn get_all_playlists(
        &self,
        session: &MusicKitSession,
    ) -> Result<Vec<Playlist>, AppleMusicError> {
        let url = format!("{API_BASE}/me/library/playlists");
        self.fetch(&url, session, Resource::into_playlist)
    }

    /// Songs in `playlist_id` in the signed-in user's library, in the
    /// playlist's named order.
    ///
    /// The id is percent-encoded as one path segment, so an id carrying a
    /// reserved character cannot split the path or start a query. A library
    /// playlist track carries no album id, so every mapped [`Song`] leaves
    /// `album_id` empty — the same shape [`RestLibrary::get_all_songs`] gives
    /// its songs.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails, the response is
    /// not valid JSON, or a resource carries a blank id or no name.
    pub fn get_songs_from_playlist(
        &self,
        session: &MusicKitSession,
        playlist_id: &str,
    ) -> Result<Vec<Song>, AppleMusicError> {
        let url = format!(
            "{API_BASE}/me/library/playlists/{}/tracks",
            encode_path_segment(playlist_id)
        );
        self.fetch(&url, session, |resource| resource.into_song(""))
    }

    /// Songs in the signed-in user's library matching `query`.
    ///
    /// Apple's library search returns its songs nested under
    /// `results.library-songs`; a query that matches nothing omits that key and
    /// maps to an empty list. The term is percent-encoded as one query value,
    /// so a space, `&`, or `#` in the query cannot split the parameter or start
    /// a fragment. A search result carries no album id, so every mapped
    /// [`Song`] leaves `album_id` empty.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails, the response is
    /// not valid JSON, or a resource carries a blank id or no name.
    pub fn search_library(
        &self,
        session: &MusicKitSession,
        query: &str,
    ) -> Result<Vec<Song>, AppleMusicError> {
        let url = format!(
            "{API_BASE}/me/library/search?term={}&types=library-songs&limit={SEARCH_PAGE_LIMIT}",
            encode_path_segment(query)
        );
        self.fetch_parsed(&url, session, parse_search_envelope, |resource| {
            resource.into_song("")
        })
    }
}

#[cfg(test)]
mod tests;
