use super::*;

#[test]
fn authorize_returns_a_session_on_a_matching_callback() {
    let observed = Arc::new(Mutex::new(Vec::<String>::new()));
    let observed_for_flow = Arc::clone(&observed);
    let session = authorize_with_flow(move |port, state| {
        let page = request(
            port,
            &format!("GET /?state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
        );
        let response = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
        let mut observed = observed_for_flow.lock().expect("observed lock");
        observed.push(state);
        observed.push(page);
        observed.push(response);
    })
    .expect("authorize succeeds");

    assert_eq!(session.developer_token, SAMPLE_DEVELOPER_TOKEN);
    assert_eq!(session.user_token, SAMPLE_USER_TOKEN);

    let observed = observed.lock().expect("observed lock");
    let state = &observed[0];
    let page = &observed[1];
    assert!(page.contains(SAMPLE_DEVELOPER_TOKEN));
    assert!(page.contains(state));
    assert!(observed[2].starts_with("HTTP/1.1 200"));
}

#[test]
fn authorize_serves_the_page_for_the_browsers_query_request() {
    // The browser opens `/?state=<nonce>`, so the page route must both match
    // `/` (the reader strips the query from the path) and carry the nonce. A
    // regression in either would refuse the page and the sign-in would never
    // start.
    let page = response_to_probe(|_port, state| {
        format!("GET /?state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
    });
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(page.contains(SAMPLE_DEVELOPER_TOKEN));
}

#[test]
fn authorize_marks_the_sign_in_page_no_store() {
    // The page carries the owner-side developer token in its HTML. A 200 GET
    // response without an explicit caching policy may be written to the
    // browser's disk cache, leaving that secret behind after the flow. Mark
    // every response no-store so the token-bearing page is never cached.
    let page = response_to_probe(|_port, state| {
        format!("GET /?state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
    });
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(
        page.contains("\r\nCache-Control: no-store\r\n"),
        "the token-bearing sign-in page must not be cacheable, got: {page}"
    );
}

#[test]
fn authorize_marks_the_sign_in_page_no_referrer() {
    // The page is served at a URL that carries the per-flow `state` nonce, and
    // the page loads Apple's MusicKit script from a third-party origin — so the
    // browser is about to send that third party a request whose `Referer` may
    // carry the nonce. Whether it does is the browser's policy to decide
    // unless the server decides it: a browser configured with `unsafe-url`, or
    // one old enough to default to `no-referrer-when-downgrade` (which sends
    // the full URL on an http -> https request), would hand the nonce over,
    // and a holder of the nonce can fetch the page and read the developer
    // token. The server must set the policy itself.
    let page = response_to_probe(|_port, state| {
        format!("GET /?state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
    });
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(
        page.contains("\r\nReferrer-Policy: no-referrer\r\n"),
        "the sign-in page must not let the browser's referrer policy leak the nonce, got: {page}"
    );
}

// A response larger than the socket send buffer, written to a client that stops
// reading, blocks `write_all` with no timeout — holding the sign-in flow past
// `AUTH_TIMEOUT`. `write_response` must arm a bounded write timeout first (a
// `None` timeout means "block forever"); the read path's deadline cap has its
// own tests below, and this pins the write side.
#[test]
fn write_response_arms_a_bounded_write_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a loopback listener");
    let address = listener.local_addr().expect("read the listener address");
    let _client = TcpStream::connect(address).expect("connect a client");
    let (mut server, _) = listener.accept().expect("accept the client");

    write_response(&mut server, 200, "OK", "text/plain; charset=utf-8", "hello")
        .expect("write the response");

    assert!(
        server
            .write_timeout()
            .expect("read the stream's write timeout")
            .is_some(),
        "write_response must arm a bounded write timeout, not leave it unbounded"
    );
}

#[test]
fn authorize_trims_surrounding_whitespace_from_the_developer_token() {
    // `APPLE_MUSIC_DEVELOPER_TOKEN` commonly arrives with a trailing newline
    // (read from a file) or a stray space; the strict validator would reject
    // it as "not base64url", which does not say why. The flow trims it, so
    // the padded token both succeeds and is stored trimmed.
    let (opener, handles) = background(|port, state| {
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
    });

    let padded = format!("  {SAMPLE_DEVELOPER_TOKEN}\n");
    let session = authorize_with_timeout(&padded, &opener, Duration::from_secs(5))
        .expect("a token with surrounding whitespace is accepted");
    join_all(&handles);

    assert_eq!(session.developer_token, SAMPLE_DEVELOPER_TOKEN);
}

#[test]
fn authorize_rejects_a_malformed_developer_token_without_opening() {
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_opener = Arc::clone(&calls);
    let opener = move |_url: &str| {
        calls_for_opener.fetch_add(1, Ordering::SeqCst);
        Ok(())
    };

    let error = authorize_with_timeout("not-a-jwt", &opener, Duration::from_millis(50))
        .expect_err("a malformed developer token is rejected");

    assert!(error.to_string().contains("developer token"));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn authorize_reports_a_failed_opener_with_its_cause() {
    // A missing opener — `xdg-open` absent on a headless Linux box, say — is
    // a real startup failure, so the flow must surface it at once rather than
    // block until the callback deadline. The opener is called once with the
    // bootstrap path, and its own `io::Error` is named in the returned message.
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_opener = Arc::clone(&calls);
    let opener = move |_page_path: &str| {
        calls_for_opener.fetch_add(1, Ordering::SeqCst);
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "xdg-open: not found",
        ))
    };

    let error = authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
        .expect_err("a failed opener aborts the flow");

    assert!(
        error
            .to_string()
            .contains("could not open the sign-in page"),
        "the message must name the opener step, got: {error}"
    );
    assert!(
        error.to_string().contains("xdg-open: not found"),
        "the message must carry the opener's own error, got: {error}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn authorize_refuses_the_page_without_the_state_nonce() {
    let observed = Arc::new(Mutex::new(Vec::<String>::new()));
    let observed_for_flow = Arc::clone(&observed);
    let (opener, handles) = background(move |port, state| {
        // A local client that does not present the per-flow nonce — here with
        // a guess and with no query at all — must not be served the page,
        // which embeds the developer token. The real browser request carries
        // the nonce, so the flow still completes.
        let guessed = request(
            port,
            "GET /?state=not-the-nonce HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        );
        let absent = request(port, "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        let mut observed = observed_for_flow.lock().expect("observed lock");
        observed.push(guessed);
        observed.push(absent);
        drop(observed);
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
    });

    authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
        .expect("a page request without the nonce does not abort the flow");
    join_all(&handles);

    let observed = observed.lock().expect("observed lock");
    for response in observed.iter() {
        assert!(
            response.starts_with("HTTP/1.1 403"),
            "a page request without the state nonce should be forbidden, got: {response}"
        );
        assert!(
            !response.contains(SAMPLE_DEVELOPER_TOKEN),
            "the sign-in page must not leak the developer token without the nonce"
        );
    }
}

#[test]
fn authorize_ignores_a_callback_with_the_wrong_state() {
    // Any local client can reach `POST /token` without knowing the nonce, so a
    // wrong-state request must not abort the sign-in: it is answered 400 and
    // the real callback still completes on a later connection.
    let wrong = response_to_probe(|_port, _state| token_request("wrong-state", SAMPLE_USER_TOKEN));
    assert!(
        wrong.starts_with("HTTP/1.1 400"),
        "a wrong state should be rejected with 400"
    );
}

#[test]
fn authorize_rejects_a_callback_with_an_empty_user_token() {
    let error = authorize_with_flow(|port, state| {
        let _ = request(port, &token_request(&state, ""));
    })
    .expect_err("an empty user token is rejected");
    assert!(error.to_string().contains("user token"));
}

#[test]
fn authorize_rejects_a_malformed_user_token_naming_the_defect() {
    // A present-but-malformed token gets the same shape-defect wording the
    // developer-token validator uses, so a broken callback is diagnosable
    // rather than reported as a generic "invalid user token". The message
    // must name the defect without echoing the token, which is a secret.
    let error = authorize_with_flow(|port, state| {
        let _ = request(port, &token_request(&state, "aaa.bbb.cc!"));
    })
    .expect_err("a malformed user token is rejected");
    let message = error.to_string();
    assert!(
        message.contains("character outside the base64url alphabet"),
        "the message must name the defect, got: {message}"
    );
    assert!(
        !message.contains("aaa.bbb.cc!"),
        "the message must not echo the user token, got: {message}"
    );
}

#[test]
fn a_malformed_user_token_is_named_in_the_400_body_without_echoing_it() {
    // The 400 body is what the browser displays, so it must name the shape
    // defect the returned error names and must not echo the token, which is a
    // secret. A rejection is terminal, so `response_to_probe` — which expects
    // the flow to finish with a valid callback — cannot observe it; capture
    // the response the browser actually receives instead.
    let observed = Arc::new(Mutex::new(String::new()));
    let observed_for_flow = Arc::clone(&observed);
    let (opener, handles) = background(move |port, state| {
        *observed_for_flow.lock().expect("observed lock") =
            request(port, &token_request(&state, "aaa.bbb.cc!"));
    });
    authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
        .expect_err("a malformed user token is rejected");
    join_all(&handles);

    let response = observed.lock().expect("observed lock").clone();
    assert!(
        response.starts_with("HTTP/1.1 400"),
        "the callback should be answered 400, got: {response}"
    );
    assert!(
        response.contains("character outside the base64url alphabet"),
        "the body must name the defect, got: {response}"
    );
    assert!(
        !response.contains("aaa.bbb.cc!"),
        "the body must not echo the user token, got: {response}"
    );
}

#[test]
fn authorize_rejects_a_callback_without_a_user_token_field() {
    // A callback missing `userToken` altogether is rejected with a message
    // distinct from a malformed one, so the two causes are not conflated.
    let error = authorize_with_flow(|port, state| {
        let body = format!("state={state}");
        let raw = format!(
            "POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let _ = request(port, &raw);
    })
    .expect_err("a callback without a user token is rejected");
    assert!(
        error.to_string().contains("did not carry a user token"),
        "got: {error}"
    );
}

#[test]
fn authorize_ignores_an_unknown_request_before_the_callback() {
    let missing = response_to_probe(|_port, _state| {
        "GET /nope HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n".to_string()
    });
    assert!(missing.starts_with("HTTP/1.1 404"));
}

#[test]
fn authorize_rejects_a_foreign_host_header() {
    // A DNS-rebinding page reaches the loopback socket under its own
    // hostname, so its `Host` header names that hostname, not 127.0.0.1.
    // The response must not carry the sign-in page (and its developer
    // token), and the real callback must still complete the flow.
    let rebound = response_to_probe(|_port, _state| {
        "GET / HTTP/1.1\r\nHost: evil.example\r\n\r\n".to_string()
    });
    assert!(
        rebound.starts_with("HTTP/1.1 403"),
        "a foreign Host should be forbidden, got: {rebound}"
    );
    assert!(
        !rebound.contains(SAMPLE_DEVELOPER_TOKEN),
        "the sign-in page must not leak the developer token to a foreign Host"
    );
}

#[test]
fn authorize_refuses_a_page_request_without_a_host_header() {
    // A hand-written or HTTP/1.0 request can omit `Host` entirely. The probe
    // carries the correct nonce, so only the loopback guard stands between it
    // and the page that embeds the developer token: a guard that treated a
    // missing host as loopback would serve the page here.
    let page = response_to_probe(|_port, state| format!("GET /?state={state} HTTP/1.1\r\n\r\n"));
    assert!(
        page.starts_with("HTTP/1.1 403"),
        "a page request without a Host should be forbidden, got: {page}"
    );
    assert!(
        !page.contains(SAMPLE_DEVELOPER_TOKEN),
        "the sign-in page must not leak the developer token without a Host"
    );
}

#[test]
fn authorize_serves_the_page_for_a_loopback_host_carrying_the_port() {
    // The browser opens `http://127.0.0.1:{port}/?state={state}`, so its `Host`
    // header is the port-bearing form `127.0.0.1:{port}` — not the bare address
    // the other tests send — and the page request also carries the per-flow
    // nonce. The host check must strip the port before comparing, or the
    // legitimate page request is refused as a foreign host and sign-in never
    // starts.
    let page = response_to_probe(|port, state| {
        format!("GET /?state={state} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n")
    });
    assert!(page.starts_with("HTTP/1.1 200"), "got: {page}");
    assert!(page.contains(SAMPLE_DEVELOPER_TOKEN));
}

#[test]
fn authorize_survives_a_connection_that_closes_early() {
    authorize_with_flow(|port, state| {
        // A client that connects and closes without a complete request.
        drop(TcpStream::connect(("127.0.0.1", port)).expect("connect"));
        // Then the real callback.
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
    })
    .expect("an aborted connection does not abort the flow");
}

#[test]
fn authorize_survives_an_oversized_content_length() {
    authorize_with_flow(|port, state| {
        // `Content-Length: usize::MAX` overflows a naive `body_start + len`.
        let oversized = "POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 18446744073709551615\r\n\r\n";
        let _ = request(port, oversized);
        // Then the real callback.
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
    })
    .expect("an oversized Content-Length does not abort the flow");
}
