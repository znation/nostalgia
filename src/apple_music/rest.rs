//! The Apple Music REST browse client: turns a `MusicKit` session plus the
//! Apple Music REST API into the model's [`Artist`], [`Album`], and [`Song`]
//! lists.
//!
//! The client is synchronous and network-isolated behind [`HttpTransport`], so
//! its tests drive an in-memory stub instead of a live endpoint. It changes no
//! app behavior on its own: the service that decides when to call it is the
//! sibling wiring change.

use serde::Deserialize;

use super::AppleMusicError;
use crate::library::{Album, Artist, Song};
use crate::music_kit_auth::MusicKitSession;

/// The Apple Music REST API root. The library endpoints are storefront-free,
/// so no storefront is fetched.
const API_BASE: &str = "https://api.music.apple.com/v1";

/// The narrow seam over the blocking HTTP client, so [`RestLibrary`]'s tests
/// run without a network.
///
/// [`UreqTransport`] is the production implementation; a test installs a stub
/// that returns a canned body and records the request it saw.
pub trait HttpTransport: Send + Sync {
    /// Fetches `url` with `session`'s developer and user tokens and returns the
    /// response body as text.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails — a transport
    /// error, a non-success status, or a body that cannot be read.
    fn get(&self, url: &str, session: &MusicKitSession) -> Result<String, AppleMusicError>;
}

/// The production [`HttpTransport`], backed by the blocking `ureq` client.
pub struct UreqTransport;

impl HttpTransport for UreqTransport {
    fn get(&self, url: &str, session: &MusicKitSession) -> Result<String, AppleMusicError> {
        let authorization = format!("Bearer {}", session.developer_token);
        let mut response = ureq::get(url)
            .header("Authorization", authorization.as_str())
            .header("Music-User-Token", session.user_token.as_str())
            .call()
            .map_err(|error| AppleMusicError::new(format!("request to {url} failed: {error}")))?;

        response.body_mut().read_to_string().map_err(|error| {
            AppleMusicError::new(format!("reading response from {url} failed: {error}"))
        })
    }
}

/// A library browse client: issues the three library queries over
/// [`HttpTransport`] and maps each response onto the model.
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
    /// not valid JSON, or a resource carries no name.
    pub fn get_favorite_artists(
        &self,
        session: &MusicKitSession,
    ) -> Result<Vec<Artist>, AppleMusicError> {
        let url = format!("{API_BASE}/me/library/artists");
        let resources = self.fetch(&url, session, "favorite artists")?;
        resources
            .into_iter()
            .map(|resource| {
                let name = resource.required_name("artist", "favorite artists")?;
                Ok(Artist {
                    id: resource.id,
                    name,
                })
            })
            .collect()
    }

    /// Albums by `artist_id` in the signed-in user's library.
    ///
    /// The query already scopes the results to `artist_id`, so every mapped
    /// [`Album`] carries it as its `artist_id`.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails, the response is
    /// not valid JSON, or a resource carries no name.
    pub fn get_albums_by_artist(
        &self,
        session: &MusicKitSession,
        artist_id: &str,
    ) -> Result<Vec<Album>, AppleMusicError> {
        let url = format!("{API_BASE}/me/library/artists/{artist_id}/albums");
        let resources = self.fetch(&url, session, "albums by artist")?;
        resources
            .into_iter()
            .map(|resource| {
                let title = resource.required_name("album", "albums by artist")?;
                Ok(Album {
                    id: resource.id,
                    title,
                    artist_id: artist_id.to_string(),
                })
            })
            .collect()
    }

    /// Songs on `album_id` in the signed-in user's library.
    ///
    /// The query already scopes the results to `album_id`, so every mapped
    /// [`Song`] carries it as its `album_id`.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the request fails, the response is
    /// not valid JSON, or a resource carries no name.
    pub fn get_songs_from_album(
        &self,
        session: &MusicKitSession,
        album_id: &str,
    ) -> Result<Vec<Song>, AppleMusicError> {
        let url = format!("{API_BASE}/me/library/albums/{album_id}/tracks");
        let resources = self.fetch(&url, session, "songs from album")?;
        resources
            .into_iter()
            .map(|resource| {
                let title = resource.required_name("song", "songs from album")?;
                Ok(Song {
                    id: resource.id,
                    title,
                    album_id: album_id.to_string(),
                })
            })
            .collect()
    }

    /// Fetches `url` and parses the collection envelope, naming `query` in
    /// every failure so the caller can tell which browse request broke.
    fn fetch(
        &self,
        url: &str,
        session: &MusicKitSession,
        query: &str,
    ) -> Result<Vec<Resource>, AppleMusicError> {
        let body = self
            .transport
            .get(url, session)
            .map_err(|error| AppleMusicError::new(format!("{query} request failed: {error}")))?;
        let envelope: Envelope<Resource> = serde_json::from_str(&body).map_err(|error| {
            AppleMusicError::new(format!("{query} response was not valid JSON: {error}"))
        })?;
        Ok(envelope.data)
    }
}

/// The `{ "data": [ ... ] }` envelope every Apple Music collection response
/// carries. Only the first page is read: the API's `next` link is ignored, and
/// Apple caps a page at 100 items.
#[derive(Deserialize)]
struct Envelope<T> {
    data: Vec<T>,
}

/// One entry in a collection response: its stable id plus the attributes the
/// model reads a name from.
#[derive(Deserialize)]
struct Resource {
    /// The resource's stable identifier.
    id: String,
    /// The resource's attributes; absent when the API omits them.
    attributes: Option<Attributes>,
}

/// The subset of a resource's attributes the browse client reads.
#[derive(Deserialize)]
struct Attributes {
    /// The display name; absent when the API omits it.
    name: Option<String>,
}

impl Resource {
    /// The resource's non-blank name, or an [`AppleMusicError`] naming `kind`,
    /// `query`, and this resource's id.
    ///
    /// A resource whose `attributes.name` is absent or only whitespace is
    /// rejected rather than mapped to a blank row the UI would render empty.
    fn required_name(&self, kind: &str, query: &str) -> Result<String, AppleMusicError> {
        match self
            .attributes
            .as_ref()
            .and_then(|attributes| attributes.name.as_deref())
        {
            Some(name) if !name.trim().is_empty() => Ok(name.to_string()),
            _ => Err(AppleMusicError::new(format!(
                "{query} response carried {kind} {:?} without a name",
                self.id
            ))),
        }
    }
}

#[cfg(test)]
mod tests;
