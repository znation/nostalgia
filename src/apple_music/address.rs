//! Host address classification: legacy numeric IPv4 spellings and internal-address test.

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
/// closes the gap [`preview_url_problem`](super::validation::preview_url_problem) leaves for a name it never resolves.
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
/// calling this. Several transition forms embed an IPv4 address it does not
/// decode, so a preview hostname that resolves to one of them would otherwise
/// route to an internal address through the transition instead of being
/// refused: 6to4 (`2002::/16`, the IPv4 in bytes 2..6), NAT64
/// (`64:ff9b::/96`, the IPv4 in the last 32 bits, and `64:ff9b:1::/48`, whose
/// RFC 6052 `/48` layout splits the IPv4 address around the reserved octet),
/// Teredo (`2001:0000::/32`, whose server IPv4 in bytes 4..8 is stored in plain
/// form and whose client IPv4 in bytes 12..16 is stored as the bitwise
/// complement of the address; a connection goes to the server and then to the
/// client, so an internal either way is refused), IPv4-translated
/// (`::ffff:0:0:0/96`, the IPv4 in the last 32 bits), and ISATAP (an address
/// outside those fixed prefixes whose interface identifier is `0:5efe` or
/// `200:5efe` followed by the IPv4 in the last 32 bits, which tunnels to that
/// IPv4 address).
fn v6_embeds_internal_ipv4(v6: std::net::Ipv6Addr) -> bool {
    let octets = v6.octets();
    let internal = |a: u8, b: u8, c: u8, d: u8| {
        ip_is_internal(std::net::IpAddr::V4(std::net::Ipv4Addr::new(a, b, c, d)))
    };
    // IPv4-translated: `::ffff:0:0:0/96`, distinct from the IPv4-mapped form
    // `to_ipv4` decodes. The IPv4 is the last 32 bits.
    if octets[..8].iter().all(|&byte| byte == 0)
        && octets[8] == 0xff
        && octets[9] == 0xff
        && octets[10] == 0
        && octets[11] == 0
    {
        return internal(octets[12], octets[13], octets[14], octets[15]);
    }
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
    // ISATAP: the fixed interface-identifier marker sits in bytes 10..12,
    // with either the `0:5efe` or the u/l-bit-set `200:5efe` high half, and
    // the IPv4 is the last 32 bits. Checking it last keeps each prefix's own
    // embedding authoritative: an address inside 6to4, NAT64, or Teredo can
    // carry the same marker with internal-looking last 32 bits while the IPv4
    // its prefix embeds is public.
    if octets[10] == 0x5e
        && octets[11] == 0xfe
        && (octets[8..10] == [0x00, 0x00] || octets[8..10] == [0x02, 0x00])
        && internal(octets[12], octets[13], octets[14], octets[15])
    {
        return true;
    }
    false
}
