use super::*;

#[test]
fn apple_music_error_displays_its_message() {
    // The seam's error type formats as the cause itself, so a reported
    // failure reads as the message, not as a struct dump.
    let error = AppleMusicError::new("track not found");
    assert_eq!(error.to_string(), "track not found");
}

#[test]
fn apple_music_error_new_accepts_an_owned_message() {
    // `new` takes `impl Into<String>`, so a backend can pass a formatted
    // `String` as readily as a `&str`; both must format identically.
    let error = AppleMusicError::new(format!("album {} not found", "album-1"));
    assert_eq!(error.to_string(), "album album-1 not found");
}

#[test]
fn ensure_id_is_valid_rejects_empty_and_whitespace_ids() {
    // An id names a real Apple Music resource, so a blank one is a caller
    // bug. Both an empty id and a whitespace-only id are blank; the
    // message quotes the value so the whitespace case is visible rather
    // than reading as an empty string.
    assert_eq!(
        ensure_id_is_valid("", IdKind::Track)
            .unwrap_err()
            .to_string(),
        "track id must not be blank (got \"\")"
    );
    assert_eq!(
        ensure_id_is_valid("\t", IdKind::Artist)
            .unwrap_err()
            .to_string(),
        "artist id must not be blank (got \"\\t\")"
    );
    // A nameable id passes through untouched, including one with an
    // internal space; only a wholly blank id is rejected.
    assert!(ensure_id_is_valid("song-1", IdKind::Track).is_ok());
    assert!(ensure_id_is_valid("a b", IdKind::Album).is_ok());
}

#[test]
fn ensure_id_is_valid_rejects_terminal_control_characters() {
    // `play_track` writes the track id to the terminal, so an id carrying
    // an escape sequence (`\u{1b}`) or other control byte would let a
    // hostile library entry manipulate the log stream. The guard rejects
    // it and quotes the value with `{id:?}`, so the error message itself
    // cannot carry the raw control bytes.
    let escape = "evil\u{1b}]0;pwnd\u{7}";
    let error = ensure_id_is_valid(escape, IdKind::Track)
        .unwrap_err()
        .to_string();
    assert_eq!(
        error,
        "track id must not contain control characters (got \"evil\\u{1b}]0;pwnd\\u{7}\")"
    );
    assert!(!error.contains('\u{1b}'));
    assert!(!error.contains('\u{7}'));
    // A newline is a control character too, so an id cannot forge a
    // second log line; a plain id still passes.
    assert!(ensure_id_is_valid("song\n1", IdKind::Track).is_err());
    assert!(ensure_id_is_valid("song-1", IdKind::Track).is_ok());
}

// The success log line is the second terminal sink for a track id, and the
// guard does not close it: `char::is_control` is true only for the Cc
// category, so a Unicode format character such as the right-to-left
// override (`\u{202e}`, category Cf) passes `ensure_id_is_valid`. The sink
// must therefore escape the id itself; `play_log_line` formats with
// `Debug`, which renders both the control and the format character as
// `\u{...}`. The helper is pure so this contract is testable without
// capturing stdout.
#[test]
fn play_log_line_escapes_control_and_format_characters() {
    let line = play_log_line("evil\u{1b}]0;pwnd\u{7}");
    assert_eq!(line, "Playing track: \"evil\\u{1b}]0;pwnd\\u{7}\"");
    assert!(!line.contains('\u{1b}'));
    assert!(!line.contains('\u{7}'));

    let line = play_log_line("a\u{202e}b");
    assert_eq!(line, "Playing track: \"a\\u{202e}b\"");
    assert!(!line.contains('\u{202e}'));
}

// `preview_url_problem` extracts and normalizes the host before it checks it:
// it drops a `user:pass@` prefix, a `:port` suffix, an IPv6 `[`/`]` pair, a
// trailing root dot, and case. Each of those spellings can hide an internal
// host that only the normalized form exposes, so a regression that dropped one
// normalization would leave the host looking like an ordinary name and pass it
// through. Pin each disguised spelling and the reason it is refused.
#[test]
fn preview_url_problem_unwraps_a_disguised_internal_host() {
    for (url, reason) in [
        (
            "https://user@127.0.0.1/preview.m4a",
            "must not target an internal address",
        ),
        (
            "https://user:pass@10.0.0.5/preview.m4a",
            "must not target an internal address",
        ),
        (
            "https://127.0.0.1:443/preview.m4a",
            "must not target an internal address",
        ),
        (
            "https://[::1]:443/preview.m4a",
            "must not target an internal address",
        ),
        (
            "https://[fec0::1]:443/preview.m4a",
            "must not target an internal address",
        ),
        (
            "https://127.0.0.1./preview.m4a",
            "must not target an internal address",
        ),
        (
            "https://localhost:8080/preview.m4a",
            "must not target the local host",
        ),
        (
            "https://localhost./preview.m4a",
            "must not target the local host",
        ),
        (
            "https://LOCALHOST/preview.m4a",
            "must not target the local host",
        ),
    ] {
        assert_eq!(
            preview_url_problem(url),
            Some(reason),
            "unexpected reason for {url:?}"
        );
    }
}

// The disguised-host table above only reaches the host checks: every row has
// a lowercase `https` scheme and a non-empty host. Pin the earlier refusal
// arms too — no `://`, a cleartext scheme, and an authority that yields an
// empty or non-name host — so a dropped early return cannot fall through to
// the host parse and accept the URL.
#[test]
fn preview_url_problem_rejects_a_url_without_a_usable_https_host() {
    for (url, reason) in [
        ("preview.m4a", "must be an absolute https URL"),
        ("https:/preview.m4a", "must be an absolute https URL"),
        (
            "http://audio.example/preview.m4a",
            "must use the https scheme",
        ),
        ("https:///preview.m4a", "must name a host"),
        ("https://:443/preview.m4a", "must name a host"),
        ("https://user@/preview.m4a", "must name a host"),
        ("https://-/preview.m4a", "must name a host"),
        ("https://1.2.3.4.5/preview.m4a", "must name a host"),
    ] {
        assert_eq!(
            preview_url_problem(url),
            Some(reason),
            "unexpected reason for {url:?}"
        );
    }
}

// The scheme match ignores ASCII case and the authority ends at the first
// `/`, `?`, or `#`, so these valid shapes must still be accepted rather than
// refused by a stricter regression in either step.
#[test]
fn preview_url_problem_accepts_uppercase_https_and_a_query_only_authority() {
    for url in [
        "HTTPS://audio.example/preview.m4a",
        "https://audio.example?token=x",
        "https://audio.example#part",
    ] {
        assert_eq!(
            preview_url_problem(url),
            None,
            "unexpected refusal for {url:?}"
        );
    }
}

// `ip_is_internal` is the shared SSRF guard: `preview_url_problem` applies it
// to an address literal a preview URL spells, and `audio::preview::PublicAddressResolver`
// applies it to every address a preview hostname resolves to. The URL-level
// tests above only reach the branches their sample literals happen to hit, so
// a dropped range check (broadcast, documentation, CGNAT, multicast, a
// reserved block, a v6 form) would be
// invisible. This table pins each branch on both sides: every internal form
// must report `true`, and public v4/v6 addresses — including the addresses
// bordering `100.64.0.0/10`, `198.18.0.0/15`, the IETF protocol-assignment
// and 6to4 relay blocks, the multicast range, and a public v4-mapped v6
// address — must report
// `false`, so the guard cannot pass by rejecting everything.
#[test]
fn ip_is_internal_refuses_every_internal_range_and_allows_public_addresses() {
    for (text, expected) in [
        // IPv4 internal: private, loopback, link-local, unspecified,
        // broadcast, documentation, 0.0.0.0/8, 100.64.0.0/10, multicast,
        // reserved 240.0.0.0/4, benchmarking 198.18.0.0/15, the IETF
        // protocol-assignment block, and the deprecated 6to4 relay anycast
        // block.
        ("10.0.0.1", true),
        ("172.16.0.1", true),
        ("192.168.0.1", true),
        ("127.0.0.1", true),
        ("169.254.169.254", true),
        ("0.0.0.0", true),
        ("255.255.255.255", true),
        ("192.0.2.1", true),
        ("198.51.100.1", true),
        ("203.0.113.1", true),
        ("0.1.2.3", true),
        ("100.64.0.1", true),
        ("100.127.255.255", true),
        ("224.0.0.1", true),
        ("239.255.255.250", true),
        ("240.0.0.1", true),
        ("198.18.0.1", true),
        ("198.19.255.255", true),
        ("192.0.0.1", true),
        ("192.88.99.1", true),
        // IPv4 public: the addresses bordering the CGNAT, benchmarking,
        // IETF protocol-assignment, and 6to4 relay blocks, the top of the
        // unicast space below multicast, and ordinary public hosts.
        ("100.63.255.255", false),
        ("100.128.0.0", false),
        ("198.17.255.255", false),
        ("198.20.0.0", false),
        ("192.0.1.1", false),
        ("192.88.98.255", false),
        ("192.88.100.0", false),
        ("223.255.255.255", false),
        ("8.8.8.8", false),
        ("93.184.216.34", false),
        // IPv6 internal: loopback, unspecified, multicast, unique-local
        // (fc00::/7), link-local (fe80::/10), and v4-mapped internal
        // addresses.
        ("::1", true),
        ("::", true),
        ("ff02::1", true),
        ("fc00::1", true),
        ("fd12:3456::1", true),
        ("fe80::1", true),
        // Deprecated site-local (`fec0::/10`) addresses are still routed inside
        // a site, so a connection to one reaches an internal host just as a
        // link-local address can.
        ("fec0::1", true),
        ("::ffff:127.0.0.1", true),
        ("::ffff:10.0.0.1", true),
        // IPv6 internal via an IPv4-embedding transition prefix, which
        // `to_ipv4` does not decode: 6to4 (2002::/16), NAT64 (64:ff9b::/96
        // and 64:ff9b:1::/48), and Teredo (2001:0000::/32, server IPv4 in
        // bytes 4..8 and client IPv4 in bytes 12..16, each XORed with
        // 0xFFFFFFFF). Each embeds 127.0.0.1 or 169.254.169.254.
        ("2002:7f00:1::", true),
        ("2002:a9fe:a9fe::", true),
        ("64:ff9b::7f00:1", true),
        ("64:ff9b::a9fe:a9fe", true),
        // NAT64 /48 (`64:ff9b:1::/48`): RFC 6052 splits the IPv4 address
        // around the reserved octet, so the last 32 bits are the suffix, not
        // the IPv4. The first row embeds 127.0.0.1 while its suffix is
        // 8.8.8.8, so a last-32-bits decode would call it public.
        ("64:ff9b:1:7f00:0:100:808:808", true),
        ("64:ff9b:1:a9fe:0:a9fe:0:0", true),
        // Teredo with an internal client (127.0.0.1) and a public server.
        ("2001:0:f7f7:f7f7:0:0:80ff:fffe", true),
        // Teredo with an internal server (127.0.0.1) and a public client.
        ("2001:0:80ff:fffe:0:0:f7f7:f7f7", true),
        // IPv6 public: a public v4-mapped address, ordinary public v6, a
        // 6to4 address embedding 8.8.8.8, a NAT64 address embedding 8.8.8.8,
        // and a Teredo address whose server and client are both public.
        ("::ffff:8.8.8.8", false),
        ("2001:4860:4860::8888", false),
        ("2606:4700:4700::1111", false),
        ("2002:808:808::", false),
        ("64:ff9b::808:808", false),
        ("64:ff9b:1:808:0:808:808:808", false),
        ("2001:0:f7f7:f7f7:0:0:f7f7:fbfb", false),
    ] {
        let ip: std::net::IpAddr = text
            .parse()
            .unwrap_or_else(|_| panic!("bad test address {text}"));
        assert_eq!(
            ip_is_internal(ip),
            expected,
            "ip_is_internal({text}) should be {expected}"
        );
    }
}

// `preview_url_problem` lowercases the host before it calls
// `is_numeric_host_form`, so the helper never sees the uppercase `0X` prefix
// in production; that case below pins the helper's documented spelling as a
// defensive contract, not a separate guard on the URL path. The URL-level
// rejection test above exercises several legacy forms end-to-end, including
// the uppercase spelling. This table pins each parser branch directly,
// including the shapes that must stay names, so a regression in a component
// spelling fails here rather than only at a real fetch.
#[test]
fn is_numeric_host_form_recognizes_legacy_spellings_and_leaves_names_alone() {
    for host in [
        // One 32-bit decimal, hex (either prefix case), or octal component.
        "2130706433",
        "0x7f000001",
        "0X7F000001",
        "017700000001",
        // Two, three, and four dotted components, decimal and mixed radix.
        "127.1",
        "127.0.1",
        "1.2.3.4",
        "0x7f.0.0.1",
        "0177.0.0.1",
    ] {
        assert!(
            is_numeric_host_form(host),
            "{host:?} is a numeric host form"
        );
    }

    for host in [
        // Ordinary DNS names, including the preview CDN's.
        "localhost",
        "example.com",
        "audio-ssl.itunes.apple.com",
        // Shapes that start like a numeric form but are not one: an empty
        // `0x` body, a non-hex digit, an empty component, five components,
        // invalid octal, a letter, a sign, and the empty host.
        "0x",
        "0x7g",
        "1..2",
        "1.2.3.4.5",
        "09",
        "123abc",
        "-1",
        "",
    ] {
        assert!(
            !is_numeric_host_form(host),
            "{host:?} is not a numeric host form"
        );
    }
}

// A scoped IPv6 literal (`[fe80::1%lo]`, or RFC 6874's `%25` spelling) names
// a link-local address, but `IpAddr::parse` cannot read the zone suffix, so the
// host would otherwise fall through to the "ordinary name" branch and skip the
// internal-address check. The guard must refuse it like any other link-local
// literal.
#[test]
fn preview_url_problem_rejects_a_scoped_ipv6_literal() {
    for url in [
        "https://[fe80::1%lo]/preview.m4a",
        "https://[fe80::1%25lo]/preview.m4a",
        "https://[::1%25lo]/preview.m4a",
    ] {
        assert_eq!(
            preview_url_problem(url),
            Some("must not carry a zone identifier"),
            "unexpected reason for {url:?}"
        );
    }
}
