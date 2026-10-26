//! Seam validation: resource ids and preview URLs.

use super::AppleMusicError;

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

/// Whether `host` is an IPv4 address written in one of the legacy numeric
/// forms `getaddrinfo` accepts — a single 32-bit value (`2130706433`,
/// `0x7f000001`, `017700000001`), a two-part `a.b`, or a three- or four-part
/// dotted value with hex or octal components (`127.1`, `0x7f.0.0.1`). Such a
/// host is resolved as an address, never looked up as a DNS name, and
/// `IpAddr::parse` does not accept most of these spellings.
pub(super) fn is_numeric_host_form(host: &str) -> bool {
    let mut components = 0;
    for part in host.split('.') {
        components += 1;
        if components > 4 || !is_numeric_component(part) {
            return false;
        }
    }
    true
}

/// Whether `part` is a decimal, `0x`/`0X` hex, or leading-`0` octal integer
/// component of a legacy numeric IPv4 form. An empty part is not a component.
fn is_numeric_component(part: &str) -> bool {
    if let Some(hex) = part.strip_prefix("0x").or_else(|| part.strip_prefix("0X")) {
        return !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    if part.len() > 1 && part.starts_with('0') {
        return part.chars().all(|c| ('0'..='7').contains(&c));
    }
    !part.is_empty() && part.chars().all(|c| c.is_ascii_digit())
}

/// Whether `ip` is an address the preview fetch must not reach.
///
/// IPv4: the local machine, private, loopback, link-local, unspecified,
/// broadcast, documentation, multicast, `0.0.0.0/8`, shared (CGNAT),
/// benchmarking, IETF protocol-assignment, 6to4 relay, and `240.0.0.0/4`
/// reserved networks. IPv6: loopback, unspecified, multicast, unique-local,
/// link-local, deprecated site-local (`fec0::/10`), and a transition address
/// embedding an internal IPv4 address. IPv6 documentation (`2001:db8::/32`)
/// and benchmarking (`2001:2::/48`) addresses are allowed.
///
/// Shared with [`crate::audio`], whose preview agent resolves a preview URL's
/// hostname and rejects any internal address it resolves to — the check that
/// closes the gap [`preview_url_problem`] leaves for a name it never resolves.
pub(crate) fn ip_is_internal(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            let octets = v4.octets();
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_multicast()
                || octets[0] == 0
                || octets[0] >= 240
                || (octets[0] == 100 && (octets[1] & 0xc0) == 64)
                || (octets[0] == 198 && (octets[1] & 0xfe) == 18)
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
                || (octets[0] == 192 && octets[1] == 88 && octets[2] == 99)
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                || (v6.segments()[0] & 0xffc0) == 0xfe80
                // Site-local (`fec0::/10`) was deprecated in favor of
                // unique-local, but it can still be routed inside a site, so
                // a connection to one is not a public destination.
                || (v6.segments()[0] & 0xffc0) == 0xfec0
                || v6
                    .to_ipv4()
                    .is_some_and(|v4| ip_is_internal(std::net::IpAddr::V4(v4)))
                || v6_embeds_internal_ipv4(v6)
        }
    }
}

/// Whether `v6` is an IPv6 transition address that embeds an internal IPv4
/// address.
///
/// `Ipv6Addr::to_ipv4` decodes only the IPv4-mapped (`::ffff:0:0/96`) and
/// IPv4-compatible (`::/96`) forms, which [`ip_is_internal`] checks before
/// calling this. Three transition prefixes embed an IPv4 address it does not
/// decode, so a preview hostname that resolves to one of them would otherwise
/// route to an internal address through the transition instead of being
/// refused: 6to4 (`2002::/16`, the IPv4 in bytes 2..6), NAT64
/// (`64:ff9b::/96`, the IPv4 in the last 32 bits, and `64:ff9b:1::/48`, whose
/// RFC 6052 `/48` layout splits the IPv4 address around the reserved octet), and
/// Teredo (`2001:0000::/32`, whose server IPv4 in bytes 4..8 is stored in plain
/// form and whose client IPv4 in bytes 12..16 is stored as the bitwise
/// complement of the address; a connection goes to the server and then to the
/// client, so an internal either way is refused).
fn v6_embeds_internal_ipv4(v6: std::net::Ipv6Addr) -> bool {
    let octets = v6.octets();
    let internal = |a: u8, b: u8, c: u8, d: u8| {
        ip_is_internal(std::net::IpAddr::V4(std::net::Ipv4Addr::new(a, b, c, d)))
    };
    // 6to4: 2002:VVVV:WWWW::/48.
    if octets[0] == 0x20 && octets[1] == 0x02 {
        return internal(octets[2], octets[3], octets[4], octets[5]);
    }
    // NAT64: 64:ff9b::/96 and 64:ff9b:1::/48. Only those two sub-prefixes are
    // assigned inside 64:ff9b::/32. The /96 form embeds the IPv4 in the last
    // 32 bits; the /48 form uses RFC 6052's /48 layout, with the first 16 IPv4
    // bits in octets 6 and 7 and the last 16 in octets 9 and 10 (octet 8 is
    // the reserved u octet, and octets 11..16 are the suffix).
    if octets[0] == 0x00 && octets[1] == 0x64 && octets[2] == 0xff && octets[3] == 0x9b {
        if octets[4] == 0x00 && octets[5] == 0x01 {
            return internal(octets[6], octets[7], octets[9], octets[10]);
        }
        return internal(octets[12], octets[13], octets[14], octets[15]);
    }
    // Teredo: 2001:0000::/32. The server IPv4 is plain; only the client IPv4
    // is bitwise-complemented (RFC 4380).
    if octets[0] == 0x20 && octets[1] == 0x01 && octets[2] == 0x00 && octets[3] == 0x00 {
        return internal(octets[4], octets[5], octets[6], octets[7])
            || internal(!octets[12], !octets[13], !octets[14], !octets[15]);
    }
    false
}
