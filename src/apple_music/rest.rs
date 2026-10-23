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
//! the API's response shapes and their mapping onto the model, and this module
//! holds the queries, pagination, and URL construction that tie them together.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use super::AppleMusicError;
use crate::library::{Album, Artist, Playlist, Song};
use crate::music_kit_auth::MusicKitSession;

mod transport;
mod wire;

pub use transport::{HttpTransport, UreqTransport};
use wire::{Resource, parse_collection, parse_search_envelope};

/// The Apple Music REST API origin, without a version path. A collection's
/// `next` link is resolved against this origin (see [`absolute_next_url`]), so
/// a paginated follow stays on the same host as the first-page query.
const API_ORIGIN: &str = "https://api.music.apple.com";

/// The Apple Music REST API root. The library endpoints are storefront-free,
/// so no storefront is fetched.
const API_BASE: &str = "https://api.music.apple.com/v1";

/// The most collection pages [`RestLibrary::fetch`] reads for one query.
///
/// Apple caps a collection page at 100 items and links the next in `next`, so
/// the client follows that link to read a large library in full. The bound
/// stops a server that returns an endless chain of new `next` links from
/// looping forever; a `next` link that resolves to a page already read stops
/// the follow sooner (see [`RestLibrary::fetch`]). When the bound is reached,
/// `fetch` logs [`page_bound_notice`] and returns the rows read so far.
const MAX_PAGES: usize = 10;

/// The page size [`RestLibrary::search_library`] asks for. Apple documents the
/// library-search `limit` maximum as 25 (default 5), so the request asks for
/// the documented maximum: a larger value is out of contract and can fail the
/// whole search.
const SEARCH_PAGE_LIMIT: usize = 25;

/// The end-to-end bound on one REST call, from DNS lookup through reading the
/// response body. `ureq` defaults every network timeout to `None`, so without
/// this a server that accepts the connection and then stalls leaves the
/// calling thread — spawned by `off_thread` and detached from the UI's own
/// 30-second fetch timeout — blocked forever. [`RestLibrary::fetch`] gives its
/// whole paginated fetch this one budget, handing each page's request only the
/// time left, so a slow multi-page collection cannot keep that detached thread
/// alive for up to [`MAX_PAGES`] times this bound while the UI has already
/// stopped waiting.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

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

    /// Fetches `url` and parses the collection envelope, following the
    /// response's `next` link (up to [`MAX_PAGES`] pages) so a collection
    /// larger than one 100-item page is read in full. A `next` link that does
    /// not name a same-origin path is not followed (see [`absolute_next_url`]).
    /// Each page's resources are passed to `map` as that page is read, so the
    /// caller's validation runs per page rather than after the whole
    /// collection is accumulated. The whole follow shares one
    /// [`REQUEST_TIMEOUT`] budget — each page is requested with only the time
    /// left — so a slow collection cannot outrun the UI's fetch timeout by a
    /// multiple of the page bound.
    ///
    /// A transport failure propagates as the transport's own cause, a parse
    /// failure names the parse, and a resource `map` rejects (a blank id or a
    /// missing name) reports `map`'s cause. None names the query: the browse
    /// methods leave that to the UI, which already labels the fetch it issued,
    /// so the user-facing report names the query once rather than in both
    /// layers. A failure on a page after the first — transport, parse, or
    /// `map` — additionally names the page (see [`page_context`]), because the
    /// UI cannot know which page the failure happened on.
    fn fetch<T>(
        &self,
        url: &str,
        session: &MusicKitSession,
        map: impl Fn(Resource) -> Result<T, AppleMusicError>,
    ) -> Result<Vec<T>, AppleMusicError> {
        self.fetch_parsed(url, session, parse_collection, map)
    }

    /// Fetches `url` and follows its collection's `next` link (up to
    /// [`MAX_PAGES`] pages), parsing each page with `parse` and mapping each
    /// resource with `map`.
    ///
    /// `parse` turns one page's body into its resources and `next` link, so a
    /// caller whose collection is nested inside a larger response (the search
    /// envelope) reuses the same pagination, deadline, and page-notice rules
    /// as a top-level collection. See [`RestLibrary::fetch`] for the
    /// pagination and error-reporting contract.
    fn fetch_parsed<T>(
        &self,
        url: &str,
        session: &MusicKitSession,
        parse: impl Fn(&str) -> Result<(Vec<Resource>, Option<String>), AppleMusicError>,
        map: impl Fn(Resource) -> Result<T, AppleMusicError>,
    ) -> Result<Vec<T>, AppleMusicError> {
        let mut items = Vec::new();
        let mut next_url = url.to_string();
        // The pages already requested, so a `next` link that points back to
        // one of them stops the follow instead of appending that page's rows
        // again (a server that repeats a link, or a two-page cycle, would
        // otherwise return duplicated data until `MAX_PAGES` was reached).
        let mut visited_urls = HashSet::new();
        visited_urls.insert(next_url.clone());
        // One deadline for the whole follow, not one per page: a server that
        // answers each page just inside the request bound cannot stretch a
        // ten-page fetch into ten times the UI's wait.
        let deadline = Instant::now() + REQUEST_TIMEOUT;
        for page in 1..=MAX_PAGES {
            let body = self
                .transport
                .get_within(&next_url, session, deadline)
                .map_err(|error| AppleMusicError::new(page_context(&error.to_string(), page)))?;
            let (data, next) = parse(&body)
                .map_err(|error| AppleMusicError::new(page_context(&error.to_string(), page)))?;
            for resource in data {
                items.push(map(resource).map_err(|error| {
                    AppleMusicError::new(page_context(&error.to_string(), page))
                })?);
            }
            let Some(next) = next.as_deref() else {
                break;
            };
            match absolute_next_url(next) {
                // The bound is reached: report the unread page and stop.
                Some(_) if page >= MAX_PAGES => {
                    eprintln!("{}", page_bound_notice(next));
                    break;
                }
                // A link back to a page already read is a malformed
                // pagination chain: stop rather than append that page's rows
                // again until the page bound is reached.
                Some(resolved) if !visited_urls.insert(resolved.clone()) => {
                    eprintln!("{}", repeated_next_notice(next));
                    break;
                }
                // Another unread page exists and the bound allows reading it.
                Some(resolved) => next_url = resolved,
                // The link is not a same-origin path, so following it could
                // send the session's tokens to another host.
                None => {
                    eprintln!("{}", unfollowable_next_notice(next));
                    break;
                }
            }
        }
        Ok(items)
    }
}

/// Names `page` in a pagination failure, so a request that fails while
/// following a `next` link says which page it happened on.
///
/// The first page is the query the UI already labels, so its message is
/// returned unchanged and the report stays the bare cause. A later page is
/// otherwise indistinguishable from the first in the report — the UI names the
/// query, not the page — so the page number is appended only when it adds
/// information.
fn page_context(message: &str, page: usize) -> String {
    if page == 1 {
        message.to_string()
    } else {
        format!("{message} (page {page})")
    }
}

/// The absolute URL to follow for a collection's `next` link, or `None` when
/// the link is not a same-origin path this client may follow.
///
/// Apple returns `next` as an absolute path (`/v1/me/library/artists?offset=100`),
/// which is resolved against [`API_ORIGIN`]. Only a path beginning with a
/// single `/` is accepted: an absolute URL, a scheme-relative `//host` target,
/// and a bare relative target all yield `None`, so the session's credentials
/// only ever go to [`API_ORIGIN`] — the same credential-isolation rule as
/// `agent_with_timeout`'s `max_redirects(0)`.
fn absolute_next_url(next: &str) -> Option<String> {
    if next.starts_with('/') && !next.starts_with("//") {
        Some(format!("{API_ORIGIN}{next}"))
    } else {
        None
    }
}

/// The notice logged when a collection has more pages than [`MAX_PAGES`], so
/// `fetch` stops with rows unread. `next` is server-controlled text headed for
/// the terminal, so it is formatted with `Debug` — as
/// [`crate::apple_music::playback::play_log_line`] formats an id — to escape a control
/// character (`\u{1b}`) or a Unicode format character such as the
/// right-to-left override `\u{202e}` instead of letting it reach the terminal
/// raw.
fn page_bound_notice(next: &str) -> String {
    format!(
        "Apple Music returned a next page {next:?} after {MAX_PAGES} pages; stopping at the page limit"
    )
}

/// The notice logged when a collection's `next` link is not a same-origin path
/// and is therefore not followed. Escaped with `Debug` for the same reason as
/// [`page_bound_notice`]: the link is server-controlled and reaches the
/// terminal.
fn unfollowable_next_notice(next: &str) -> String {
    format!(
        "Apple Music returned a next page {next:?} that is not a same-origin path; not following it"
    )
}

/// The notice logged when a collection's `next` link resolves to a page this
/// fetch has already read, so following it would append that page's rows
/// again. Escaped with `Debug` for the same reason as [`page_bound_notice`]:
/// the link is server-controlled and reaches the terminal.
fn repeated_next_notice(next: &str) -> String {
    format!("Apple Music returned a next page {next:?} that was already read; not following it")
}

/// Percent-encodes `segment` for use as a single URL path segment.
///
/// Every byte outside the RFC 3986 unreserved set (`A`-`Z`, `a`-`z`, `0`-`9`,
/// `-`, `.`, `_`, `~`) becomes `%XX`, so an id carrying a space, `/`, `?`,
/// `#`, or a non-ASCII character cannot split the path or start a query. An
/// unreserved byte is never encoded, so a normal Apple Music id (letters,
/// digits, and dots) passes through unchanged.
fn encode_path_segment(segment: &str) -> String {
    // The uppercase hex digits RFC 3986 percent-encoding uses.
    const HEX: [u8; 16] = *b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            // Push the `%` and its two hex digits directly; a `format!` here
            // would allocate a throwaway `String` for every encoded byte.
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0F)]));
        }
    }
    encoded
}

#[cfg(test)]
mod tests;
