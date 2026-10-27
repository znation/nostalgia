use super::*;

#[test]
fn validate_developer_token_accepts_a_three_segment_jwt() {
    assert!(validate_developer_token(SAMPLE_DEVELOPER_TOKEN).is_ok());
}

#[test]
fn validate_developer_token_rejects_blank_and_malformed_values() {
    for token in ["", " ", "a", "a.b", "a.b.c.d", "a..c", "a.b.c\"", "a.b.\\c"] {
        assert!(
            validate_developer_token(token).is_err(),
            "expected {token:?} to be rejected"
        );
    }
}

#[test]
fn validate_developer_token_names_the_specific_defect() {
    // Every malformed token used to get the same "must be a three-segment
    // base64url JWT" message, which does not say which part is wrong. Each
    // defect now gets its own phrase, so a user with, say, a pasted payload
    // can tell the segment count is the problem rather than the alphabet.
    let cases = [
        ("a.b", "exactly three dot-separated segments"),
        ("a.b.c.d", "exactly three dot-separated segments"),
        ("a..c", "one of its segments is empty"),
        ("a.b.c\"", "outside the base64url alphabet"),
        ("a.b.\\c", "outside the base64url alphabet"),
    ];
    for (token, expected) in cases {
        let error = validate_developer_token(token).expect_err("a malformed token is rejected");
        assert!(
            error.to_string().contains(expected),
            "expected {token:?} to report {expected:?}, got {error}"
        );
    }
}

#[test]
fn validate_developer_token_does_not_echo_the_token() {
    // The developer token is a secret, so the defect phrase must never quote
    // the offending value into the message that `sign_in` logs to stderr.
    let secret = "topsecret.payload.sig!";
    let error = validate_developer_token(secret).expect_err("a malformed token is rejected");
    assert!(!error.to_string().contains("topsecret"), "got {error}");
}

#[test]
fn percent_decode_decodes_form_escapes_and_leaves_bad_ones_literal() {
    // The callback body is form-urlencoded by the browser's `fetch`, so a
    // `+` is a space and `%XX` is the decoded byte (upper- or lowercase hex).
    assert_eq!(percent_decode("a+b"), "a b");
    assert_eq!(percent_decode("a%20b"), "a b");
    assert_eq!(percent_decode("%2E"), ".");
    assert_eq!(percent_decode("%2e"), ".");
    // Multi-byte UTF-8 arrives as several escapes and decodes as one char.
    assert_eq!(percent_decode("%C3%A9"), "\u{e9}");
    // A non-hex, incomplete, or trailing escape stays literal rather than
    // being dropped or panicking.
    assert_eq!(percent_decode("%zz"), "%zz");
    assert_eq!(percent_decode("100%"), "100%");
    assert_eq!(percent_decode("%2"), "%2");
}

// The decoder runs on the token callback's body and the page query, both
// chosen by a local client, so its edges must neither drop bytes nor decode
// twice: an escape that ends the input still decodes, an encoded `+` stays a
// plus rather than becoming a space, one pass never re-decodes its own output,
// and bytes that are not UTF-8 become U+FFFD instead of panicking.
#[test]
fn percent_decode_handles_a_final_escape_a_literal_plus_and_invalid_utf8() {
    assert_eq!(percent_decode("a%41"), "aA");
    assert_eq!(percent_decode("%41"), "A");
    assert_eq!(percent_decode("a%2Bb"), "a+b");
    assert_eq!(percent_decode("%252B"), "%2B");
    assert_eq!(percent_decode("%%41"), "%A");
    assert_eq!(percent_decode("%FF"), "\u{fffd}");
}

// A host that merely starts with the loopback address, or carries an empty
// port, must not pass: only the exact name (after stripping one port) does.
#[test]
fn is_loopback_host_rejects_lookalike_names() {
    assert!(!is_loopback_host(Some("127.0.0.1.evil.example")));
    assert!(!is_loopback_host(Some("127.0.0.10")));
    assert!(!is_loopback_host(Some("")));
    assert!(!is_loopback_host(Some("localhost:51234")));
}

// The `Host` header is the DNS-rebinding guard: the server is bound to
// `127.0.0.1`, so only that address — with or without the port the browser
// appends — may read the sign-in page, which embeds the developer token.
// A missing `Host` (a hand-written or HTTP/1.0 request) must be refused
// exactly like a foreign one, and the port must be stripped before the
// comparison so a foreign host carrying a port is not mistaken for the
// loopback address. The flow tests below cover a missing `Host` and a
// port-bearing loopback `Host`, but never a foreign host carrying a port, so
// this pins that branch directly.
#[test]
fn is_loopback_host_accepts_only_the_bound_loopback_address() {
    assert!(is_loopback_host(Some("127.0.0.1")));
    assert!(is_loopback_host(Some("127.0.0.1:51234")));
    assert!(!is_loopback_host(Some("evil.example")));
    assert!(!is_loopback_host(Some("evil.example:51234")));
    assert!(!is_loopback_host(None));
}

// The nonce is the only thing a local process must guess to fetch the page
// that embeds the developer token: the loopback `Host` check passes for any
// local client, so the page route refuses a request whose `state` is not this
// flow's nonce. `random_nonce` is therefore a security boundary, and no flow
// test pins it — they all read the nonce back out of the opener URL and
// compare it to itself, so a constant (or otherwise trivially guessable)
// nonce would clear the whole suite while exposing the token to any local
// process. Pin that each call is a fresh 16-hex-digit value.
#[test]
fn random_nonce_is_a_fresh_sixteen_hex_digit_value() {
    let first = random_nonce();
    let second = random_nonce();

    assert_eq!(
        first.len(),
        16,
        "the nonce should be 16 hex digits, got {first:?}"
    );
    assert!(
        first.chars().all(|character| character.is_ascii_hexdigit()),
        "the nonce should be hex, got {first:?}"
    );
    assert_ne!(
        first, second,
        "each flow's nonce must be fresh, not a constant"
    );
}

// Both nonce checks use `nonce_matches`, whose constant-time comparison is
// what keeps a guess's reply time from depending on how many leading bytes it
// got right. A prefix of the nonce, a superstring, an empty value, and a value
// with one byte changed must all be rejected; only the exact nonce matches.
// The timing property itself is not observable from a unit test, so this pins
// the equivalence the constant-time fold must preserve.
#[test]
fn nonce_matches_accepts_only_the_exact_nonce() {
    let nonce = "0123456789abcdef";

    assert!(nonce_matches(nonce, nonce));
    assert!(!nonce_matches("", nonce));
    assert!(!nonce_matches(&nonce[..nonce.len() - 1], nonce));
    assert!(!nonce_matches(&format!("{nonce}0"), nonce));
    assert!(!nonce_matches("0123456789abcdee", nonce));
    for index in 0..nonce.len() {
        let mut guess = nonce.as_bytes().to_vec();
        guess[index] ^= 1;
        let guess = String::from_utf8(guess).expect("flipping one ASCII byte keeps UTF-8");
        assert!(
            !nonce_matches(&guess, nonce),
            "a nonce differing at byte {index} must not match"
        );
    }
}

// The page route's gate reads the nonce out of the query string: the `state`
// field may sit anywhere among others and arrive percent-encoded, a repeated
// field passes if any copy matches, and a missing query, a missing field, a
// wrong value, or a similarly named key is refused.
#[test]
fn page_query_carries_nonce_accepts_only_a_matching_state_field() {
    let nonce = "0123456789abcdef";

    assert!(page_query_carries_nonce(
        Some("state=0123456789abcdef"),
        nonce
    ));
    assert!(page_query_carries_nonce(
        Some("a=1&state=0123456789abcdef&b=2"),
        nonce
    ));
    assert!(page_query_carries_nonce(
        Some("state=%30123456789abcdef"),
        nonce
    ));
    assert!(page_query_carries_nonce(
        Some("state=wrong&state=0123456789abcdef"),
        nonce
    ));
    assert!(page_query_carries_nonce(
        Some("junk&state=0123456789abcdef"),
        nonce
    ));

    assert!(!page_query_carries_nonce(None, nonce));
    assert!(!page_query_carries_nonce(Some(""), nonce));
    assert!(!page_query_carries_nonce(Some("state="), nonce));
    assert!(!page_query_carries_nonce(Some("state"), nonce));
    assert!(!page_query_carries_nonce(
        Some("state=0123456789abcdee"),
        nonce
    ));
    assert!(!page_query_carries_nonce(
        Some("State=0123456789abcdef"),
        nonce
    ));
    assert!(!page_query_carries_nonce(
        Some("xstate=0123456789abcdef"),
        nonce
    ));
    assert!(!page_query_carries_nonce(
        Some("other=0123456789abcdef"),
        nonce
    ));
}

#[test]
fn debug_redacts_both_tokens() {
    let session = MusicKitSession {
        developer_token: SAMPLE_DEVELOPER_TOKEN.to_string(),
        user_token: SAMPLE_USER_TOKEN.to_string(),
    };
    let debug = format!("{session:?}");
    assert!(!debug.contains(SAMPLE_DEVELOPER_TOKEN));
    assert!(!debug.contains(SAMPLE_USER_TOKEN));
    assert!(debug.contains("<redacted>"));
}
