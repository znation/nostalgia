//! The Apple Music REST wire format: response envelopes, resource shapes,
//! and the parsing that decodes them.
//!
//! [`mapping`] turns a decoded [`Resource`] into a library model value; this
//! module owns only the shapes the API sends and the reports for a body that
//! fails to decode.

use serde::Deserialize;

use super::super::AppleMusicError;

mod mapping;

/// Describes why parsing an Apple Music collection response failed.
///
/// `serde_json` reports two distinct failures through the same error type: a
/// body that is not JSON at all, and a body that parses as JSON but does not
/// match the envelope (a missing `data` array, say). Calling both "not valid
/// JSON" misdescribes the second — the bytes are valid JSON; the envelope is
/// what does not match — so the message names the defect `serde_json`
/// classified: a `Data` error is a shape mismatch and every other category
/// keeps the invalid-JSON wording. `Category::Io` cannot arise from a `from_str`
/// parse of an in-memory body, so it shares that wording rather than going
/// unreported. `envelope` names the shape being parsed (`"collection"` or
/// `"search"`), so the report says which envelope did not match.
fn describe_parse_failure(error: &serde_json::Error, envelope: &str) -> String {
    match error.classify() {
        serde_json::error::Category::Data => {
            format!("response did not match the Apple Music {envelope} envelope: {error}")
        }
        _ => format!("response was not valid JSON: {error}"),
    }
}

/// Parses one page of a top-level `{ "data": [...], "next": ... }` collection
/// response.
pub(super) fn parse_collection(
    body: &str,
) -> Result<(Vec<Resource>, Option<String>), AppleMusicError> {
    let Envelope { data, next } = serde_json::from_str(body)
        .map_err(|error| AppleMusicError::new(describe_parse_failure(&error, "collection")))?;
    Ok((data, next))
}

/// Parses one page of an Apple Music search response: the songs collection
/// nested under `results.library-songs`, or no songs when the query matched
/// none.
///
/// Apple omits `results.library-songs` entirely for a search that matched
/// nothing, so an absent key is a valid empty page rather than a parse failure.
/// Any other shape that does not match the search envelope is reported by
/// [`describe_parse_failure`].
pub(super) fn parse_search_envelope(
    body: &str,
) -> Result<(Vec<Resource>, Option<String>), AppleMusicError> {
    let SearchEnvelope { results } = serde_json::from_str(body)
        .map_err(|error| AppleMusicError::new(describe_parse_failure(&error, "search")))?;
    Ok(match results.library_songs {
        Some(collection) => (collection.data, collection.next),
        None => (Vec::new(), None),
    })
}
/// The `{ "data": [ ... ] }` envelope every Apple Music collection response carries. Apple caps a
/// page at 100 items and links the next page in `next`;
/// [`RestLibrary::fetch`](super::RestLibrary::fetch) follows that link (up to
/// [`MAX_PAGES`](super::paging::MAX_PAGES) pages) so a larger library is read in full rather than
/// truncated to its first page.
#[derive(Deserialize)]
struct Envelope<T> {
    data: Vec<T>,
    /// The API's link to the next page of a paginated collection, present only
    /// when there is one. [`RestLibrary::fetch`](super::RestLibrary::fetch)
    /// follows it, up to [`MAX_PAGES`](super::paging::MAX_PAGES) pages, when it names a
    /// same-origin path.
    next: Option<String>,
}

/// The `{ "results": { ... } }` envelope an Apple Music search response carries.
/// [`RestLibrary::search_library`](super::RestLibrary::search_library) parses the songs collection
/// nested inside it through [`parse_search_envelope`].
#[derive(Deserialize)]
struct SearchEnvelope {
    results: SearchResults,
}

/// The `results` object of a search response. Apple carries a `library-songs`
/// collection only when the query matched something; a no-match search omits
/// the key, so it is optional and maps to no songs.
#[derive(Deserialize)]
struct SearchResults {
    /// The matching library songs, or `None` when the search matched none.
    #[serde(rename = "library-songs")]
    library_songs: Option<SearchCollection>,
}

/// The `library-songs` collection of a search response: the same `{ "data": [...], "next": ... }`
/// shape as a top-level browse collection.
#[derive(Deserialize)]
struct SearchCollection {
    data: Vec<Resource>,
    /// The API's link to the next page of matching songs, present only when
    /// there is one.
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
/// terminal raw, matching how [`crate::apple_music::playback::play::play_log_line`] escapes a
/// library id. An ordinary message (letters, digits, spaces) passes through
/// unchanged.
pub(super) fn api_error_cause(body: &str) -> Option<String> {
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
pub(super) struct Resource {
    /// The resource's stable identifier.
    id: String,
    /// Apple's resource type (`library-albums`, `library-playlists`, …); absent when the API
    /// omits it.
    #[serde(rename = "type")]
    kind: Option<String>,
    /// The resource's attributes; absent when the API omits them.
    attributes: Option<Attributes>,
}

/// The subset of a resource's attributes the browse client reads.
#[derive(Deserialize)]
struct Attributes {
    /// The display name; absent when the API omits it.
    name: Option<String>,
    /// The performing artist's display name; absent when the API omits it.
    #[serde(rename = "artistName")]
    artist_name: Option<String>,
    /// The title of the album the resource belongs to; absent when the API
    /// omits it.
    #[serde(rename = "albumName")]
    album_name: Option<String>,
    /// The genre names the API lists for the resource; absent when the API
    /// omits them. The mapping module's `primary_genre` reduces them to the one
    /// the model keeps.
    #[serde(rename = "genreNames")]
    genre_names: Option<Vec<String>>,
    /// The resource's release date, normally `YYYY-MM-DD`; absent when the
    /// API omits it. The mapping module's `release_year` reduces it to the year
    /// the model keeps.
    #[serde(rename = "releaseDate")]
    release_date: Option<String>,
    /// The track's 1-based position on its album; absent when the API omits
    /// it. Parsed as a JSON number so an out-of-`u32` value, including an
    /// integer beyond `i64`, maps to the model's `None` instead of failing the
    /// whole response.
    #[serde(rename = "trackNumber")]
    track_number: Option<serde_json::Number>,
    /// The track's length in milliseconds; absent when the API omits it.
    /// Parsed as a JSON number so a negative value — or an integer beyond
    /// `u64` — maps to the model's `0` "no duration supplied" sentinel
    /// instead of failing the whole response.
    #[serde(rename = "durationInMillis")]
    duration_in_millis: Option<serde_json::Number>,
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
