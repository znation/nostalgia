//! The Apple Music REST browse client: turns a `MusicKit` session plus the
//! Apple Music REST API into the model's [`Artist`], [`Album`], and [`Song`]
//! lists.
//!
//! The client is synchronous and network-isolated behind [`HttpTransport`], so
//! its tests drive an in-memory stub instead of a live endpoint. It changes no
//! app behavior on its own: the service that decides when to call it is the
//! sibling wiring change.

use std::sync::OnceLock;
use std::time::Duration;

use serde::Deserialize;

use super::AppleMusicError;
use crate::library::{Album, Artist, Song};
use crate::music_kit_auth::MusicKitSession;

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
/// stops a server that returns an endless `next` chain — or a link back to a
/// page already read — from looping forever; when it is reached, `fetch` logs
/// [`page_bound_notice`] and returns the rows read so far.
const MAX_PAGES: usize = 10;

/// The end-to-end bound on one REST call, from DNS lookup through reading the
/// response body. `ureq` defaults every network timeout to `None`, so without
/// this a server that accepts the connection and then stalls leaves the
/// calling thread — spawned by `off_thread` and detached from the UI's own
/// 30-second fetch timeout — blocked forever. Matches that fetch bound so the
/// thread exits about when the UI stops waiting on it.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

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
    /// error, a non-success status, or a body that cannot be read. The message
    /// is the bare cause (the HTTP status and Apple's detail, or the underlying
    /// error); [`RestLibrary`] passes it through unchanged and leaves the query
    /// context to the UI, so the user-facing report names the query once.
    fn get(&self, url: &str, session: &MusicKitSession) -> Result<String, AppleMusicError>;
}

/// The production [`HttpTransport`], backed by the blocking `ureq` client.
///
/// Every request runs on one process-wide [`ureq::Agent`]: a fresh agent per
/// request (`ureq::get`) would open a new TCP connection and TLS session each
/// time, while the shared agent reuses the pooled connection across the
/// artist → album → song browse requests. The shared agent carries
/// [`REQUEST_TIMEOUT`] as its global timeout, so a stalled server cannot block
/// a request forever, and follows no redirects, so the `Music-User-Token`
/// credential is only ever sent to the URL this client built (see
/// [`agent_with_timeout`]).
///
/// The agent also disables `ureq`'s status-as-error shortcut
/// (`http_status_as_error(false)`), so a 4xx/5xx response reaches
/// [`UreqTransport::get`] with its body intact and the Apple Music error
/// detail can be surfaced instead of a bare status code.
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl UreqTransport {
    /// Builds the transport over the process-wide pooled [`ureq::Agent`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            agent: shared_agent().clone(),
        }
    }

    /// [`new`](Self::new) with an explicit global timeout, so a test can use a
    /// short bound against a stalled loopback server instead of waiting out
    /// the production 30 seconds.
    #[cfg(test)]
    fn with_timeout(timeout: Duration) -> Self {
        Self {
            agent: agent_with_timeout(timeout),
        }
    }
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// The shared [`ureq::Agent`], built once on first use. `ureq` keeps idle
/// connections per host for 15 seconds by default, so consecutive browse
/// queries reuse the same TLS connection. Its global timeout is
/// [`REQUEST_TIMEOUT`], so every request through it is bounded end to end.
fn shared_agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| agent_with_timeout(REQUEST_TIMEOUT))
}

/// Builds an API agent with `timeout` as its global bound and no redirect
/// following.
///
/// The API requests carry the user token in the custom `Music-User-Token`
/// header. `ureq` follows up to ten redirects by default, and on a redirect it
/// strips only `Authorization`, `Cookie`, and `Content-Length` — a custom
/// header survives, so a `Location` naming another host would re-send the user
/// token there. The library endpoints answer with a JSON body, so a redirect is
/// the unexpected response it looks like, and refusing to follow one keeps the
/// credential on the single URL [`RestLibrary`] constructed.
fn agent_with_timeout(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .into()
}

impl HttpTransport for UreqTransport {
    fn get(&self, url: &str, session: &MusicKitSession) -> Result<String, AppleMusicError> {
        let authorization = format!("Bearer {}", session.developer_token);
        let mut response = self
            .agent
            .get(url)
            .header("Authorization", authorization.as_str())
            .header("Music-User-Token", session.user_token.as_str())
            .call()
            .map_err(|error| AppleMusicError::new(error.to_string()))?;

        let status = response.status();
        let body = response.body_mut().read_to_string().map_err(|error| {
            AppleMusicError::new(format!("reading the response body failed: {error}"))
        })?;

        // `http_status_as_error(false)` leaves a non-2xx response as `Ok`, so
        // the status is checked here where the body is still available:
        // Apple's error body names the cause ("Invalid developer token",
        // say), which `ureq`'s bare `Error::StatusCode` would otherwise
        // discard.
        if status.is_success() {
            Ok(body)
        } else {
            Err(AppleMusicError::new(match api_error_cause(&body) {
                Some(cause) => format!("HTTP {status}: {cause}"),
                None => format!("HTTP {status}"),
            }))
        }
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
    /// not valid JSON, or a resource carries a blank id or no name.
    pub fn get_favorite_artists(
        &self,
        session: &MusicKitSession,
    ) -> Result<Vec<Artist>, AppleMusicError> {
        let url = format!("{API_BASE}/me/library/artists");
        let resources = self.fetch(&url, session)?;
        resources
            .into_iter()
            .map(|resource| {
                let id = resource.required_id("artist")?;
                let name = resource.required_name("artist")?;
                Ok(Artist { id, name })
            })
            .collect()
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
        let resources = self.fetch(&url, session)?;
        resources
            .into_iter()
            .map(|resource| {
                let id = resource.required_id("album")?;
                let title = resource.required_name("album")?;
                Ok(Album {
                    id,
                    title,
                    artist_id: artist_id.to_string(),
                })
            })
            .collect()
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
        let resources = self.fetch(&url, session)?;
        resources
            .into_iter()
            .map(|resource| {
                let id = resource.required_id("song")?;
                let title = resource.required_name("song")?;
                Ok(Song {
                    id,
                    title,
                    album_id: album_id.to_string(),
                    duration_ms: resource.duration_ms(),
                    preview_url: resource.preview_url(),
                })
            })
            .collect()
    }

    /// Fetches `url` and parses the collection envelope, following the
    /// response's `next` link (up to [`MAX_PAGES`] pages) so a collection
    /// larger than one 100-item page is read in full. A `next` link that does
    /// not name a same-origin path is not followed (see [`absolute_next_url`]).
    ///
    /// A transport failure propagates as the transport's own cause and a parse
    /// failure names the parse. Neither names the query: the browse methods
    /// leave that to the UI, which already labels the fetch it issued, so the
    /// user-facing report names the query once rather than in both layers. A
    /// failure on a page after the first additionally names the page (see
    /// [`page_context`]), because the UI cannot know which page the failure
    /// happened on.
    fn fetch(
        &self,
        url: &str,
        session: &MusicKitSession,
    ) -> Result<Vec<Resource>, AppleMusicError> {
        let mut resources = Vec::new();
        let mut next_url = url.to_string();
        for page in 1..=MAX_PAGES {
            let body = self
                .transport
                .get(&next_url, session)
                .map_err(|error| AppleMusicError::new(page_context(&error.to_string(), page)))?;
            let envelope: Envelope<Resource> = serde_json::from_str(&body).map_err(|error| {
                AppleMusicError::new(page_context(&describe_parse_failure(&error), page))
            })?;
            resources.extend(envelope.data);
            let Some(next) = envelope.next.as_deref() else {
                break;
            };
            match absolute_next_url(next) {
                // Another page exists and the bound allows reading it.
                Some(resolved) if page < MAX_PAGES => next_url = resolved,
                // The bound is reached: report the unread page and stop.
                Some(_) => {
                    eprintln!("{}", page_bound_notice(next));
                    break;
                }
                // The link is not a same-origin path, so following it could
                // send the session's tokens to another host.
                None => {
                    eprintln!("{}", unfollowable_next_notice(next));
                    break;
                }
            }
        }
        Ok(resources)
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

/// Describes why parsing an Apple Music collection response failed.
///
/// `serde_json` reports two distinct failures through the same error type: a
/// body that is not JSON at all, and a body that parses as JSON but does not
/// match the `{ "data": [ ... ] }` envelope (a missing `data` array, say).
/// Calling both "not valid JSON" misdescribes the second — the bytes are valid
/// JSON; the envelope is what does not match — so the message names the defect
/// `serde_json` classified: a `Data` error is a shape mismatch and every other
/// category keeps the invalid-JSON wording. `Category::Io` cannot arise from a
/// `from_str` parse of an in-memory body, so it shares that wording rather
/// than going unreported.
fn describe_parse_failure(error: &serde_json::Error) -> String {
    match error.classify() {
        serde_json::error::Category::Data => {
            format!("response did not match the Apple Music collection envelope: {error}")
        }
        _ => format!("response was not valid JSON: {error}"),
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
/// [`crate::apple_music::play_log_line`] formats an id — to escape a control
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

/// Percent-encodes `segment` for use as a single URL path segment.
///
/// Every byte outside the RFC 3986 unreserved set (`A`-`Z`, `a`-`z`, `0`-`9`,
/// `-`, `.`, `_`, `~`) becomes `%XX`, so an id carrying a space, `/`, `?`,
/// `#`, or a non-ASCII character cannot split the path or start a query. An
/// unreserved byte is never encoded, so a normal Apple Music id (letters,
/// digits, and dots) passes through unchanged.
fn encode_path_segment(segment: &str) -> String {
    let mut encoded = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// The `{ "data": [ ... ] }` envelope every Apple Music collection response
/// carries. Apple caps a page at 100 items and links the next page in `next`;
/// [`RestLibrary::fetch`] follows that link (up to [`MAX_PAGES`] pages) so a
/// larger library is read in full rather than truncated to its first page.
#[derive(Deserialize)]
struct Envelope<T> {
    data: Vec<T>,
    /// The API's link to the next page of a paginated collection, present only
    /// when there is one. [`RestLibrary::fetch`] follows it, up to
    /// [`MAX_PAGES`] pages, when it names a same-origin path.
    next: Option<String>,
}

/// The `{ "errors": [ ... ] }` envelope an Apple Music error response carries,
/// whose entries name the cause that a bare status code drops.
#[derive(Deserialize)]
struct ErrorEnvelope {
    errors: Vec<ApiError>,
}

/// One entry in an Apple Music error response: the longer human-readable
/// `detail` and the shorter `title`, either of which the API may omit.
#[derive(Deserialize)]
struct ApiError {
    title: Option<String>,
    detail: Option<String>,
}

/// The human-readable cause in an Apple Music error body: the first entry's
/// non-blank `detail`, falling back to that entry's non-blank `title`, then the
/// same for each later entry in order; `None` when `body` is not the documented
/// envelope or no entry carries a cause. The API may return several entries,
/// and one whose `detail`/`title` is absent or blank (an entry carrying only a
/// `status`, say) names no cause, so the search continues to the next entry
/// rather than reporting the status alone and hiding a later entry's cause. A
/// field that is present but blank (empty or only whitespace) is treated as
/// absent, so a cause is never empty and the status message does not end in a
/// dangling `": "`.
///
/// The cause is an Apple Music reply — data outside this program's control —
/// and it is carried into [`AppleMusicError`], whose `Display` reaches the
/// browse panel and, through `ui::loading`'s failure reports, the terminal.
/// Escaping it with [`str::escape_debug`] renders a control character
/// (`\u{1b}`) or a Unicode format character such as the right-to-left override
/// `\u{202e}` as a visible `\u{..}` sequence instead of letting it reach the
/// terminal raw, matching how [`crate::apple_music::play_log_line`] escapes a
/// library id. An ordinary message (letters, digits, spaces) passes through
/// unchanged.
fn api_error_cause(body: &str) -> Option<String> {
    let envelope: ErrorEnvelope = serde_json::from_str(body).ok()?;
    envelope
        .errors
        .into_iter()
        .find_map(|error| {
            [error.detail, error.title]
                .into_iter()
                .flatten()
                .find(|cause| !cause.trim().is_empty())
        })
        .map(|cause| cause.escape_debug().to_string())
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
    /// The track's length in milliseconds; absent when the API omits it.
    #[serde(rename = "durationInMillis")]
    duration_in_millis: Option<u64>,
    /// The preview assets the API offers for the resource; absent when the
    /// API omits them.
    previews: Option<Vec<Preview>>,
}

/// One entry in a resource's `attributes.previews` array: the short preview
/// asset the audio output plays.
#[derive(Deserialize)]
struct Preview {
    /// The preview asset's URL; absent when the API omits it.
    url: Option<String>,
}

impl Resource {
    /// The resource's non-blank id, or an [`AppleMusicError`] naming `kind`.
    ///
    /// The id names the resource: the model links rows together by it, and the
    /// seam later uses it to browse or play. A blank id (empty or only
    /// whitespace) can name nothing, so the response is rejected rather than
    /// mapped to a row the UI would render but that errors when pressed — the
    /// same reason [`Self::required_name`] rejects a blank name.
    fn required_id(&self, kind: &str) -> Result<String, AppleMusicError> {
        if self.id.trim().is_empty() {
            return Err(AppleMusicError::new(format!(
                "response carried {kind} with a blank id (got {:?})",
                self.id
            )));
        }
        Ok(self.id.clone())
    }

    /// The resource's length in milliseconds, or `0` when the API omitted
    /// `attributes.durationInMillis`. `0` is [`Song::duration_ms`]'s "source
    /// supplied no duration" sentinel.
    fn duration_ms(&self) -> u64 {
        self.attributes
            .as_ref()
            .and_then(|attributes| attributes.duration_in_millis)
            .unwrap_or(0)
    }

    /// The first non-blank URL in `attributes.previews`, or `None` when
    /// `previews` is absent, empty, or carries only blank URLs.
    ///
    /// A blank URL (empty or only whitespace) can play nothing, so it is
    /// skipped in favour of a later preview; the URL is compared trimmed but
    /// returned exactly as the resource carried it.
    fn preview_url(&self) -> Option<String> {
        self.attributes
            .as_ref()
            .and_then(|attributes| attributes.previews.as_ref())
            .and_then(|previews| {
                previews.iter().find_map(|preview| {
                    preview
                        .url
                        .as_deref()
                        .filter(|url| !url.trim().is_empty())
                        .map(str::to_string)
                })
            })
    }

    /// The resource's non-blank name, or an [`AppleMusicError`] naming `kind`
    /// and this resource's id.
    ///
    /// A resource whose `attributes.name` is absent or only whitespace is
    /// rejected rather than mapped to a blank row the UI would render empty.
    fn required_name(&self, kind: &str) -> Result<String, AppleMusicError> {
        match self
            .attributes
            .as_ref()
            .and_then(|attributes| attributes.name.as_deref())
        {
            Some(name) if !name.trim().is_empty() => Ok(name.to_string()),
            _ => Err(AppleMusicError::new(format!(
                "response carried {kind} {:?} without a name",
                self.id
            ))),
        }
    }
}

#[cfg(test)]
mod tests;
