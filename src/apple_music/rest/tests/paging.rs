use super::*;

// The page-bound and unfollowable-link notices are pure, like the other
// report formatters, so the wording and the escaping of server-controlled
// text are testable without capturing stderr. Pin each notice's exact wording,
// including the production `MAX_PAGES` in the bound message.
#[test]
fn page_bound_notice_names_the_unread_page_and_the_bound() {
    assert_eq!(
        page_bound_notice("/v1/me/library/artists?offset=1000"),
        "Apple Music returned a next page \"/v1/me/library/artists?offset=1000\" after 10 pages; stopping at the page limit"
    );
}

#[test]
fn unfollowable_next_notice_names_the_unfollowed_page() {
    assert_eq!(
        unfollowable_next_notice("https://evil.example/steal"),
        "Apple Music returned a next page \"https://evil.example/steal\" that is not a same-origin path; not following it"
    );
}

#[test]
fn repeated_next_notice_names_the_already_read_page() {
    assert_eq!(
        repeated_next_notice("/v1/me/library/artists"),
        "Apple Music returned a next page \"/v1/me/library/artists\" that was already read; not following it"
    );
}

// `next` is server-controlled and reaches the terminal, so a control character
// or a Unicode format character must be escaped rather than emitted raw, as
// `api_error_cause` escapes an API error detail. Pin every pagination notice:
// `\u{1b}` (a control character) and `\u{202e}` (the right-to-left override,
// which `char::is_control` does not classify as a control).
#[test]
fn pagination_notices_escape_control_and_format_characters() {
    for notice in [
        page_bound_notice("/v1/evil\u{1b}\u{202e}"),
        unfollowable_next_notice("/v1/evil\u{1b}\u{202e}"),
        repeated_next_notice("/v1/evil\u{1b}\u{202e}"),
    ] {
        assert!(!notice.contains('\u{1b}'), "{notice:?}");
        assert!(!notice.contains('\u{202e}'), "{notice:?}");
        assert!(notice.contains("\\u{1b}"), "{notice:?}");
        assert!(notice.contains("\\u{202e}"), "{notice:?}");
    }
}

// `absolute_next_url` is the credential-isolation rule: only a same-origin
// path may be followed, so a hostile `next` cannot redirect the session's
// tokens to another host. A relative path is resolved against the API origin;
// an absolute URL, a scheme-relative `//host` target, and a bare relative
// target all yield `None`.
#[test]
fn absolute_next_url_follows_only_a_same_origin_path() {
    assert_eq!(
        absolute_next_url("/v1/me/library/artists?offset=100"),
        Some("https://api.music.apple.com/v1/me/library/artists?offset=100".to_string())
    );
    for next in [
        "https://evil.example/steal",
        "//evil.example/steal",
        "me/library/artists?offset=100",
        "",
    ] {
        assert_eq!(absolute_next_url(next), None, "{next:?}");
    }
}

// `API_BASE` must stay the origin plus the version path, or a followed `next`
// and a first-page query would disagree about where the API lives.
#[test]
fn api_base_is_the_origin_plus_the_version_path() {
    assert_eq!(API_BASE, format!("{API_ORIGIN}/v1"));
}

// A collection over one page links the next; the client follows it and returns
// both pages' rows in order, so a library larger than 100 items is read in
// full. Two recorded calls, the second to the resolved next URL, prove the
// link was followed exactly once.
#[test]
fn a_next_page_is_followed_and_both_pages_are_returned_in_order() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}],"next":"/v1/me/library/artists?offset=1"}"#,
        r#"{"data":[{"id":"artist-2","attributes":{"name":"Echo Chamber"}}]}"#,
    ]);
    let library = library_over(&stub);

    let artists = library.get_favorite_artists(&rest_session()).unwrap();

    assert_eq!(
        artists,
        vec![
            sample_artist(),
            Artist {
                id: "artist-2".to_string(),
                name: "Echo Chamber".to_string(),
                album_count: 0,
            },
        ]
    );
    let calls = stub.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[0].0,
        "https://api.music.apple.com/v1/me/library/artists"
    );
    assert_eq!(
        calls[1].0,
        "https://api.music.apple.com/v1/me/library/artists?offset=1"
    );
}

// Pagination exists so a large library is read in full, so a transport
// failure on a *later* page must surface as an error rather than be swallowed
// to return the earlier pages' rows: silently returning a prefix would
// reintroduce the truncation the paging loop replaced. Page 1 succeeds and
// links page 2; the second request fails. The page-named cause and the two
// recorded calls prove `fetch` followed the link once, stopped at the failure,
// and returned no partial rows.
#[test]
fn a_transport_error_on_a_later_page_surfaces_instead_of_truncating() {
    let stub = StubTransport::returning_results(&[
        Ok(r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}],"next":"/v1/me/library/artists?offset=1"}"#.to_string()),
        Err(AppleMusicError::new("connection reset")),
    ]);
    let library = library_over(&stub);

    let error = library
        .get_favorite_artists(&rest_session())
        .unwrap_err()
        .to_string();

    // The UI labels the query ("loading favorite artists") but cannot know the
    // failure was on page 2, so the transport cause carries the page suffix —
    // the transport-path twin of `a_failure_following_a_next_page_names_the_page`.
    assert_eq!(error, "connection reset (page 2)");
    let calls = stub.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[1].0,
        "https://api.music.apple.com/v1/me/library/artists?offset=1"
    );
}

// An endless chain of *new* pages must stop at `MAX_PAGES` rather than loop
// forever. Each page here links a distinct next offset, so the page bound —
// not the repeated-link check — is what ends the follow.
#[test]
fn an_endless_next_chain_stops_at_the_page_bound() {
    let pages: Vec<String> = (1..=MAX_PAGES)
        .map(|page| {
            format!(
                r#"{{"data":[{{"id":"artist-{page}","attributes":{{"name":"Artist {page}"}}}}],"next":"/v1/me/library/artists?offset={page}"}}"#
            )
        })
        .collect();
    let bodies: Vec<&str> = pages.iter().map(String::as_str).collect();
    let stub = StubTransport::returning_bodies(&bodies);
    let library = library_over(&stub);

    let artists = library.get_favorite_artists(&rest_session()).unwrap();

    assert_eq!(artists.len(), MAX_PAGES);
    assert_eq!(stub.calls().len(), MAX_PAGES);
}

// A server whose second page links back to the first has no new data to add:
// following the cycle would append the same rows again up to `MAX_PAGES`,
// silently showing a duplicated list. Stop at the first link to an
// already-read page, after returning each page once.
#[test]
fn a_next_link_back_to_an_already_read_page_stops_at_the_repeat() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[{"id":"artist-1","attributes":{"name":"Artist 1"}}],"next":"/v1/me/library/artists?offset=1"}"#,
        r#"{"data":[{"id":"artist-2","attributes":{"name":"Artist 2"}}],"next":"/v1/me/library/artists"}"#,
    ]);
    let library = library_over(&stub);

    let artists = library.get_favorite_artists(&rest_session()).unwrap();

    assert_eq!(artists.len(), 2);
    assert_eq!(stub.calls().len(), 2);
}

/// A transport that records the deadline each request receives, so a test can
/// prove [`RestLibrary::fetch`] shares one overall budget across pages.
struct DeadlineRecordingTransport {
    deadlines: std::sync::Arc<std::sync::Mutex<Vec<Instant>>>,
    bodies: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<String>>>,
}

impl HttpTransport for DeadlineRecordingTransport {
    fn get(&self, _url: &str, _session: &MusicKitSession) -> Result<String, AppleMusicError> {
        unreachable!("`fetch` must call `get_within`, not `get`")
    }

    fn get_within(
        &self,
        _url: &str,
        _session: &MusicKitSession,
        deadline: Instant,
    ) -> Result<String, AppleMusicError> {
        self.deadlines.lock().unwrap().push(deadline);
        Ok(self.bodies.lock().unwrap().pop_front().unwrap_or_default())
    }
}

// Pagination follows the `next` link, but the UI gives a fetch one
// `FETCH_TIMEOUT`; a fresh request bound per page would let a ten-page
// collection run ten times that. `fetch` must hand every page the same
// deadline, computed once from `REQUEST_TIMEOUT`.
#[test]
fn fetch_shares_one_deadline_across_all_pages() {
    let deadlines = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let bodies = std::sync::Arc::new(std::sync::Mutex::new(std::collections::VecDeque::from(
        vec![
            r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}],"next":"/v1/me/library/artists?offset=1"}"#.to_string(),
            r#"{"data":[{"id":"artist-2","attributes":{"name":"Echo Chamber"}}]}"#.to_string(),
        ],
    )));
    let transport = DeadlineRecordingTransport {
        deadlines: std::sync::Arc::clone(&deadlines),
        bodies,
    };
    let library = RestLibrary::new(Box::new(transport));
    let before = Instant::now();

    let artists = library.get_favorite_artists(&rest_session()).unwrap();
    let after = Instant::now();

    assert_eq!(artists.len(), 2);
    let deadlines = deadlines.lock().unwrap();
    assert_eq!(deadlines.len(), 2);
    assert_eq!(deadlines[0], deadlines[1]);
    assert!(*deadlines.last().unwrap() > before);
    assert!(*deadlines.last().unwrap() <= after + REQUEST_TIMEOUT);
}

// A `next` link that names another host must not be followed: the session's
// developer and user tokens only ever go to the API origin (the same rule as
// `max_redirects(0)`). One recorded call and the first page's row prove the
// link was dropped.
#[test]
fn a_next_link_to_another_host_is_not_followed() {
    let stub = StubTransport::returning(
        r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}],"next":"https://evil.example/steal"}"#,
    );
    let library = library_over(&stub);

    let artists = library.get_favorite_artists(&rest_session()).unwrap();

    assert_eq!(artists.len(), 1);
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/artists");
}

// A failure while following a `next` link must say which page it happened on.
// The UI labels the query ("loading favorite artists"), but only the client
// knows the failure was on page 2 rather than the first page, so without the
// page number the report is ambiguous. The first page's row is returned by the
// stub's first body; the second body is not JSON, so the parse fails on page 2.
#[test]
fn a_failure_following_a_next_page_names_the_page() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}],"next":"/v1/me/library/artists?offset=1"}"#,
        "not json",
    ]);
    let library = library_over(&stub);

    let error = library
        .get_favorite_artists(&rest_session())
        .unwrap_err()
        .to_string();

    assert!(
        error.starts_with("response was not valid JSON:") && error.ends_with("(page 2)"),
        "{error}"
    );
}

// A resource `map` rejects on a later page must name the page, just as a
// transport or parse failure does: the UI labels the query but cannot know
// which page carried the bad resource, so a multi-page browse would otherwise
// report an ambiguous cause. Page 1 maps a valid artist and links page 2;
// page 2's artist has no name.
#[test]
fn a_nameless_resource_on_a_later_page_names_the_page() {
    let stub = StubTransport::returning_bodies(&[
        r#"{"data":[{"id":"artist-1","attributes":{"name":"The Sample Band"}}],"next":"/v1/me/library/artists?offset=1"}"#,
        r#"{"data":[{"id":"artist-2","attributes":{}}]}"#,
    ]);
    let library = library_over(&stub);

    let error = library
        .get_favorite_artists(&rest_session())
        .unwrap_err()
        .to_string();

    assert_eq!(
        error,
        "response carried artist \"artist-2\" without a name (page 2)"
    );
}

// `fetch` maps each page's resources as that page is read, before it follows
// the response's `next` link, so a resource that fails validation on page 1 is
// reported immediately and the linked page is never fetched. Pinning the
// single recorded call proves the link was not followed; a regression that
// accumulated every page before mapping (the shape this refactor replaced)
// would request page 2 first, and its transport error would surface instead of
// the page-1 validation failure. The body deliberately links a page the stub
// would answer with a transport error, so the two failures are distinguishable.
#[test]
fn a_nameless_resource_on_the_first_page_stops_before_the_next_page() {
    let stub = StubTransport::returning_results(&[
        Ok(r#"{"data":[{"id":"artist-1","attributes":{}}],"next":"/v1/me/library/artists?offset=1"}"#.to_string()),
        Err(AppleMusicError::new("connection reset")),
    ]);
    let library = library_over(&stub);

    let error = library
        .get_favorite_artists(&rest_session())
        .unwrap_err()
        .to_string();

    assert_eq!(error, "response carried artist \"artist-1\" without a name");
    assert_single_call(&stub, "https://api.music.apple.com/v1/me/library/artists");
}

// The page suffix is only appended when it adds information: the first page is
// the query the UI already names, so its cause stays bare (the transport and
// parse tests above pin that end to end). A unit test pins the boundary
// directly, so a refactor cannot start appending "(page 1)" to every failure.
#[test]
fn page_context_leaves_the_first_page_message_bare() {
    assert_eq!(page_context("network down", 1), "network down");
    assert_eq!(page_context("network down", 2), "network down (page 2)");
}
