//! Form-urlencoded parsing and the request guards the sign-in server applies:
//! the loopback `Host` check, the constant-time state-nonce comparison, and the
//! percent-decoding both the page query and the token callback use.

/// Splits `application/x-www-form-urlencoded` text into its `name=value`
/// pairs.
///
/// Both the GET page request's query string and the POST callback's body use
/// this syntax, so the `&`-separated pair scan lives here once; the caller
/// decodes the value it wants with [`percent_decode`] and applies its own
/// duplicate-field rule. A pair without `=` names no field and is skipped.
pub(super) fn form_pairs(form: &str) -> impl Iterator<Item = (&str, &str)> {
    form.split('&').filter_map(|pair| pair.split_once('='))
}

/// Whether the page request's query carries this flow's `state` nonce.
///
/// The browser is opened at `/?state=<nonce>`, so the legitimate page request
/// already presents the per-flow secret. Requiring it here means a local
/// client that reaches the loopback port without knowing the nonce is refused
/// the sign-in page and the developer token it embeds.
pub(in crate::music_kit_auth) fn page_query_carries_nonce(
    query: Option<&str>,
    nonce: &str,
) -> bool {
    query.is_some_and(|query| {
        form_pairs(query)
            .any(|(key, value)| key == "state" && nonce_matches(&percent_decode(value), nonce))
    })
}

/// Whether `candidate` equals this flow's `state` nonce, compared in constant
/// time.
///
/// Both checks of the nonce — the page query and the callback body — compare a
/// value any local client can choose against the one secret that gates the
/// developer token. Ordinary string equality stops at the first differing
/// byte, so a client that times the server's replies could learn how many
/// leading bytes of a guess matched and recover the nonce one byte at a time.
/// Here every byte of both values is folded into the same accumulator, and a
/// length difference is folded into it too rather than returning early, so a
/// reply's timing does not depend on where a guess first differs. The nonce's
/// own length is fixed and public (see `random_nonce`), so the loop length
/// depends only on values the client already knows.
pub(in crate::music_kit_auth) fn nonce_matches(candidate: &str, nonce: &str) -> bool {
    let candidate = candidate.as_bytes();
    let nonce = nonce.as_bytes();
    let mut difference = u8::from(candidate.len() != nonce.len());
    for index in 0..candidate.len().max(nonce.len()) {
        let left = candidate.get(index).copied().unwrap_or(0);
        let right = nonce.get(index).copied().unwrap_or(0);
        difference |= left ^ right;
    }
    difference == 0
}

/// Whether the request's `Host` header names the loopback address the server
/// is bound to, with or without a port.
pub(in crate::music_kit_auth) fn is_loopback_host(host: Option<&str>) -> bool {
    let Some(host) = host else {
        return false;
    };
    let name = host.rsplit_once(':').map_or(host, |(name, _port)| name);
    name == "127.0.0.1"
}

/// Decodes a form-urlencoded value (`+` for space, `%XX` escapes).
pub(in crate::music_kit_auth) fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'+' {
            decoded.push(b' ');
            index += 1;
            continue;
        }
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
        {
            decoded.push(high * 16 + low);
            index += 3;
            continue;
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
