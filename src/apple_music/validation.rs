//! Seam validation: resource ids and preview URLs.

use super::AppleMusicError;
use super::address::{ip_is_internal, is_numeric_host_form};

/// The resource an id names, so the seam's validation error can label it.
///
/// [`ensure_id_is_valid`] serves the four id-taking queries; naming the id
/// kinds as a closed enum (rather than passing a `&str`) makes a misspelled
/// label impossible to compile and keeps the four kinds the only ones the
/// guard accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IdKind {
    Track,
    Artist,
    Album,
    Playlist,
}

impl std::fmt::Display for IdKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Track => "track",
            Self::Artist => "artist",
            Self::Album => "album",
            Self::Playlist => "playlist",
        })
    }
}

/// Validates an id at the seam: `Ok` when `id` names a resource, `Err` naming
/// the offending value when it is blank or carries a terminal control
/// character.
///
/// Every id in the library names a real Apple Music resource, so a blank id
/// is a caller bug rather than the ordinary "unknown id" case the browse
/// queries answer with an empty list. `kind` names the id in the message
/// (see [`IdKind`]). A whitespace-only id is rejected too:
/// like an empty one it can name nothing, and letting it through would record
/// a blank-looking Now Playing entry or report the bug as an ordinary empty
/// result. A control character (`\u{1b}`, `\n`, and the rest) is rejected
/// because `play_track` writes the id to the terminal log; without this check
/// a hostile id from the library could inject escape sequences there. The
/// message quotes the value with `{id:?}`, so both a whitespace-only id and a
/// control-character id read as escaped text rather than as invisible bytes.
pub(super) fn ensure_id_is_valid(id: &str, kind: IdKind) -> Result<(), AppleMusicError> {
    if id.trim().is_empty() {
        return Err(AppleMusicError::new(format!(
            "{kind} id must not be blank (got {id:?})"
        )));
    }
    if id.chars().any(char::is_control) {
        return Err(AppleMusicError::new(format!(
            "{kind} id must not contain control characters (got {id:?})"
        )));
    }
    Ok(())
}

/// Validates a preview URL at the seam: `Ok` for `None` or an `https` URL
/// that [`preview_url_problem`] accepts, `Err` naming the offending value
/// otherwise.
///
/// `None` is [`crate::apple_music::AppleMusicService::play_track`]'s documented "the track has no
/// playable asset" case, so it passes and the state transition is still
/// committed without audio. A blank `Some` (empty or only whitespace) is a
/// caller bug: [`crate::apple_music::rest`]'s `Resource::into_song` already
/// maps a blank preview to `None`, and handing a blank string on to the audio
/// backend would surface as a transport error that does not name the actual
/// defect. A non-blank URL is then held to the host rule of
/// [`preview_url_problem`], because the audio backend hands it to a network
/// fetch. The message quotes the value with `{url:?}`, so a whitespace-only or
/// control-character URL reads as escaped text rather than as invisible bytes.
pub(super) fn ensure_preview_url_is_valid(
    preview_url: Option<&str>,
) -> Result<(), AppleMusicError> {
    let Some(url) = preview_url else {
        return Ok(());
    };
    if url.trim().is_empty() {
        return Err(AppleMusicError::new(format!(
            "preview URL must not be blank (got {url:?})"
        )));
    }
    if let Some(problem) = preview_url_problem(url) {
        return Err(AppleMusicError::new(format!(
            "preview URL {problem} (got {url:?})"
        )));
    }
    Ok(())
}

/// Describes why `url` is not a preview URL the audio backend may fetch, or
/// `None` when it is.
///
/// A preview URL arrives in the Apple Music API response
/// (`attributes.previews[].url`) and is handed to a network fetch, so it is
/// untrusted input at a network sink. The guard requires the absolute
/// `https` scheme and rejects a host that is the local machine or a private,
/// loopback, link-local, unspecified, multicast, or reserved address literal,
/// as well as the
/// legacy numeric address forms (`2130706433`, `0x7f000001`, `0177.0.0.1`,
/// `127.1`) that `getaddrinfo` resolves as an address instead of a name. This
/// closes the address-literal SSRF shapes a hostile or compromised API reply
/// can carry. A public DNS name that resolves to an internal address is
/// refused at the fetch instead: the audio agent resolves the host and refuses
/// an internal address (see [`crate::audio`]), so the name this pure guard
/// cannot see is checked before the connection. A redirect to an internal
/// address is refused at the audio agent, which follows no redirects (see
/// [`crate::audio`]). An IPv6 literal carrying a zone identifier (`[fe80::1%lo]`,
/// or RFC 6874's encoded `[fe80::1%25lo]`) is refused too: it scopes the
/// literal to a local interface, and `IpAddr::parse` cannot read the suffix, so
/// without this check it would fall through as an ordinary name instead of
/// reaching the internal-address test. A value carrying a control character is
/// rejected too, matching
/// [`ensure_id_is_valid`]: a control character is never valid in a URL. The
/// host may be any public name — Apple's preview CDN can move without a code
/// change — so the check names what it refuses rather than pinning a host
/// allowlist.
pub(super) fn preview_url_problem(url: &str) -> Option<&'static str> {
    if url.chars().any(char::is_control) {
        return Some("must not contain control characters");
    }
    let Some((scheme, rest)) = url.split_once("://") else {
        return Some("must be an absolute https URL");
    };
    if !scheme.eq_ignore_ascii_case("https") {
        return Some("must use the https scheme");
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    let host = if let Some(inner) = host_port.strip_prefix('[') {
        inner.split(']').next().unwrap_or("")
    } else {
        host_port.split(':').next().unwrap_or("")
    };
    if host.is_empty() {
        return Some("must name a host");
    }
    // A zone identifier scopes an IPv6 literal to a local interface. It is not
    // part of a DNS name, and `IpAddr::parse` below cannot read it, so a scoped
    // link-local literal would otherwise be treated as an ordinary host name.
    if host.contains('%') {
        return Some("must not carry a zone identifier");
    }
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if host == "localhost" || host.ends_with(".localhost") {
        return Some("must not target the local host");
    }
    match host.parse::<std::net::IpAddr>() {
        Ok(ip) if ip_is_internal(ip) => Some("must not target an internal address"),
        Ok(_) => None,
        // `IpAddr::parse` accepts only the canonical forms, but `getaddrinfo`
        // (which ureq's resolver uses) also accepts the legacy numeric forms
        // `2130706433`, `0x7f000001`, `0177.0.0.1`, and `127.1`, resolving each
        // to an address without a DNS lookup. Reject those here so they cannot
        // bypass the address-literal check above.
        Err(_) if is_numeric_host_form(&host) => Some("must not be a numeric address"),
        // A host with no ASCII letter is neither an `IpAddr` nor a numeric
        // form, so it is not a name `getaddrinfo` can look up; reject it too.
        Err(_) if !host.chars().any(|c| c.is_ascii_alphabetic()) => Some("must name a host"),
        _ => None,
    }
}
