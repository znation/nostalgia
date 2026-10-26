use super::*;

#[test]
fn authorize_times_out_without_a_callback_and_names_the_bound() {
    let opener = |_url: &str| Ok(());

    let error = authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_millis(120))
        .expect_err("no callback times out");
    // The report names the bound that expired, like the fetch and play
    // timeout reports in `ui::loading`, so the message says how long the
    // flow waited, not just that it gave up.
    assert_eq!(
        error.to_string(),
        "timed out waiting for the Apple Music sign-in callback after 120ms"
    );
}

#[test]
fn authorize_deadline_bounds_a_stalled_connection() {
    // A client that connects and sends a partial request, then stalls,
    // must not outlive the flow's deadline: `AUTH_TIMEOUT` bounds the
    // whole flow, not just the accept loop. Before the read timeout was
    // capped by the deadline, this waited out `CONNECTION_READ_TIMEOUT`
    // (five seconds) despite the much shorter injected deadline.
    let (opener, handles) = background(|port, _state| {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        // A header that never completes: no `\r\n\r\n`, then hold.
        stream
            .write_all(b"POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\n")
            .expect("write a partial request");
        // Block until the server gives up and closes the connection.
        let mut sink = [0u8; 64];
        let _ = stream.read(&mut sink);
    });

    let start = Instant::now();
    let error = authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_millis(150))
        .expect_err("a stalled connection still times out");
    let elapsed = start.elapsed();
    join_all(&handles);

    assert!(error.to_string().contains("timed out"));
    assert!(
        elapsed < Duration::from_secs(1),
        "the flow waited {elapsed:?} for a stalled connection"
    );
}

#[test]
fn authorize_serves_the_callback_after_a_stalled_connection_is_dropped() {
    // A local client can reach the loopback port without the nonce and open a
    // connection that never completes its request. The accepted connections
    // are read together, so that stalled connection does not hold the accept
    // loop for the whole flow: the per-connection budget drops it, and the
    // real browser callback on a later connection is then served. Before the
    // budget, the stalled connection held the loop past the flow deadline and
    // the callback was starved, so this test failed with a timeout.
    let (opener, handles) = background(|port, state| {
        let mut stalled = TcpStream::connect(("127.0.0.1", port)).expect("connect the stall");
        // A header that never completes: no `\r\n\r\n`.
        stalled
            .write_all(b"POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\n")
            .expect("write a partial request");
        // Let the server accept the stalled connection before the callback
        // arrives, so the test exercises the accept loop with a stalled
        // connection pending.
        thread::sleep(Duration::from_millis(50));
        let response = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
        assert!(
            response.starts_with("HTTP/1.1 200"),
            "the callback should be served once the stalled connection is dropped, got: {response}"
        );
        drop(stalled);
    });

    let session = authorize_with_bounds(
        SAMPLE_DEVELOPER_TOKEN,
        &opener,
        Duration::from_millis(500),
        Duration::from_millis(100),
    )
    .expect("a stalled connection does not starve the callback");
    join_all(&handles);

    assert_eq!(session.user_token, SAMPLE_USER_TOKEN);
}

#[test]
fn a_stalled_connection_is_dropped_when_its_budget_is_spent() {
    // Each connection is bounded by the per-connection budget from the moment
    // it is accepted, so a client that opens a connection and never finishes
    // its request is dropped when that budget is spent — long before the
    // flow's own deadline. The elapsed time is what pins the budget: without
    // the deadline check the connection is held until the flow gives up, and
    // the stalled client then sees it close at the flow's deadline instead.
    const BUDGET: Duration = Duration::from_millis(100);
    const FLOW: Duration = Duration::from_millis(600);
    let (opener, handles) = background(|port, _state| {
        let mut stalled = TcpStream::connect(("127.0.0.1", port)).expect("connect the stall");
        stalled
            .write_all(b"POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\n")
            .expect("write a partial request");
        // The read ends when the server drops the connection, which it does
        // when the connection's budget is spent.
        let started = Instant::now();
        let mut sink = [0u8; 64];
        let read = stalled.read(&mut sink);
        let elapsed = started.elapsed();
        assert!(
            matches!(read, Ok(0) | Err(_)),
            "the stalled connection should be closed, read {read:?}"
        );
        assert!(
            elapsed >= BUDGET / 2,
            "the stalled connection was dropped after {elapsed:?}, before its {BUDGET:?} budget was spent"
        );
        assert!(
            elapsed < FLOW - Duration::from_millis(150),
            "the stalled connection was held for {elapsed:?}, past its {BUDGET:?} budget"
        );
    });

    let error = authorize_with_bounds(SAMPLE_DEVELOPER_TOKEN, &opener, FLOW, BUDGET)
        .expect_err("no callback arrives");
    join_all(&handles);

    assert!(error.to_string().contains("timed out"));
}

#[test]
fn authorize_serves_the_callback_after_repeated_stalled_connections() {
    // One stalled connection is bounded by the per-connection budget, but the
    // cost of each stall used to be *serialized* behind the previous one:
    // connections were served one at a time, so ten stalled connections cost
    // ten budgets and the browser's callback, queued behind them, was never
    // served. The pending connections are read together now, so the callback
    // is served as soon as it arrives however many stalls are held open.
    const STALLS: usize = 10;
    let (opener, handles) = background(|port, state| {
        let mut stalled = Vec::new();
        for _ in 0..STALLS {
            let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect the stall");
            // A header that never completes: no `\r\n\r\n`.
            stream
                .write_all(b"POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\n")
                .expect("write a partial request");
            stalled.push(stream);
        }
        // Let the server accept the stalled connections before the callback
        // arrives, so the callback is queued behind them.
        thread::sleep(Duration::from_millis(50));
        let response = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
        assert!(
            response.starts_with("HTTP/1.1 200"),
            "the callback should be served after the stalls, got: {response}"
        );
        drop(stalled);
    });

    let started = Instant::now();
    let session = authorize_with_bounds(
        SAMPLE_DEVELOPER_TOKEN,
        &opener,
        Duration::from_millis(600),
        Duration::from_millis(100),
    )
    .expect("repeated stalled connections do not starve the callback");
    let elapsed = started.elapsed();
    join_all(&handles);

    assert_eq!(session.user_token, SAMPLE_USER_TOKEN);
    // Ten stalls at a hundred milliseconds each are a second of serialized
    // waiting, past the six-hundred-millisecond flow deadline.
    assert!(
        elapsed < Duration::from_millis(500),
        "the flow waited {elapsed:?} for the callback behind the stalls"
    );
}

// The server reads a bounded number of connections at once, so a flood of
// connections cannot grow the server's share of the process's file descriptors
// without bound. The cap is behavior, not a comment: the pending list never
// grows past `MAX_PENDING_CONNECTIONS`, and a connection already accepted is
// never displaced by a newer one — the extra connections wait in the kernel's
// accept queue until a pending connection is served or its budget is spent.
#[test]
fn the_pending_list_is_capped_without_displacing_an_accepted_connection() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind the listener");
    let port = listener
        .local_addr()
        .expect("the listener has an address")
        .port();
    listener.set_nonblocking(true).expect("poll the listener");
    // One client more than the pending list can hold.
    let mut clients = Vec::new();
    for _ in 0..=MAX_PENDING_CONNECTIONS {
        clients.push(TcpStream::connect(("127.0.0.1", port)).expect("connect a client"));
    }
    // The client's ephemeral port tells the connections apart.
    let ports: Vec<u16> = clients
        .iter()
        .map(|client| {
            client
                .local_addr()
                .expect("the client has an address")
                .port()
        })
        .collect();

    let mut pending = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    // Take on every client that fits; each pass takes on at most
    // `MAX_ACCEPTS_PER_PASS`.
    while pending.len() < MAX_PENDING_CONNECTIONS {
        accept_pending(&listener, &mut pending, deadline, Duration::from_secs(5))
            .expect("accept the waiting connections");
    }
    // The list is full, so this pass takes on nothing and the extra client
    // stays in the kernel's accept queue.
    accept_pending(&listener, &mut pending, deadline, Duration::from_secs(5))
        .expect("a full pending list accepts no more connections");

    assert_eq!(
        pending.len(),
        MAX_PENDING_CONNECTIONS,
        "the pending list must not grow past its cap"
    );
    // The oldest connection was not displaced: the first pending connection
    // is still the first client.
    let oldest_kept = pending[0]
        .reader
        .stream()
        .peer_addr()
        .expect("the pending connection has a peer")
        .port();
    assert_eq!(
        oldest_kept, ports[0],
        "an accepted connection must not be displaced by a newer one"
    );

    // Free one slot the way serving that connection does, then the extra
    // client that was waiting in the kernel is taken on.
    pending.remove(0);
    accept_pending(&listener, &mut pending, deadline, Duration::from_secs(5))
        .expect("accept the queued connection once a slot is free");
    let pending_ports: Vec<u16> = pending
        .iter_mut()
        .map(|connection| {
            connection
                .reader
                .stream()
                .peer_addr()
                .expect("the pending connection has a peer")
                .port()
        })
        .collect();
    assert_eq!(pending.len(), MAX_PENDING_CONNECTIONS);
    assert!(
        pending_ports.contains(&ports[MAX_PENDING_CONNECTIONS]),
        "the extra connection must be accepted after a slot frees"
    );
}

// One pass of the accept loop takes on a bounded number of connections, so a
// flood cannot fill the pending list in one pass: the browser's callback is
// read on a later pass.
#[test]
fn one_pass_takes_on_a_bounded_number_of_connections() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind the listener");
    let port = listener
        .local_addr()
        .expect("the listener has an address")
        .port();
    listener.set_nonblocking(true).expect("poll the listener");
    let mut clients = Vec::new();
    for _ in 0..2 * MAX_ACCEPTS_PER_PASS {
        clients.push(TcpStream::connect(("127.0.0.1", port)).expect("connect a client"));
    }

    let mut pending = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    accept_pending(&listener, &mut pending, deadline, Duration::from_secs(5))
        .expect("accept the waiting connections");

    assert_eq!(
        pending.len(),
        MAX_ACCEPTS_PER_PASS,
        "one pass must take on at most {MAX_ACCEPTS_PER_PASS} connections"
    );
    accept_pending(&listener, &mut pending, deadline, Duration::from_secs(5))
        .expect("accept the waiting connections");
    assert_eq!(
        pending.len(),
        2 * MAX_ACCEPTS_PER_PASS,
        "the next pass must take on the connections the first one left"
    );
}

// A connection's budget is a hard bound, not a limit on how long it may stall
// before finishing: one whose complete request arrived after its deadline was
// spent must still be dropped, never routed. The callback here carries the
// right nonce and a malformed user token, so serving it would end the flow
// with `Rejected`; an expired connection must answer `Continue` with the
// pending list emptied instead.
#[test]
fn an_expired_connection_is_dropped_even_when_its_request_has_arrived() {
    const NONCE: &str = "0123456789abcdef";
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind the listener");
    let port = listener
        .local_addr()
        .expect("the listener has an address")
        .port();
    listener.set_nonblocking(true).expect("poll the listener");
    let mut client = TcpStream::connect(("127.0.0.1", port)).expect("connect a client");
    let body = format!("state={NONCE}&userToken=bad");
    let request = format!(
        "POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    client
        .write_all(request.as_bytes())
        .expect("write a complete callback");

    let mut pending = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    // A zero per-connection budget makes this connection's deadline its
    // acceptance instant, so the sleep below puts it past that bound.
    accept_pending(&listener, &mut pending, deadline, Duration::ZERO)
        .expect("accept the waiting connection");
    assert_eq!(pending.len(), 1, "the connection must be taken on first");
    thread::sleep(Duration::from_millis(1));

    let outcome = serve_pending(&mut pending, SAMPLE_DEVELOPER_TOKEN, NONCE);

    assert!(
        matches!(outcome, Connection::Continue),
        "an expired connection must not be routed, so its malformed callback cannot end the flow"
    );
    assert!(
        pending.is_empty(),
        "the expired connection must be dropped from the pending list"
    );
}

// A transient `accept` failure does not end the sign-in: a signal can
// interrupt the call, and a client that connects and resets before it is
// accepted makes `accept` report `ConnectionAborted` on some platforms and
// `ConnectionReset` on Windows. Any used to be returned as a fatal error, so
// one local client could abort a sign-in the user was still completing. For
// each kind, the injected accept call fails once and then reports an empty
// queue, proving the failure is retried rather than returned.
#[test]
fn a_transient_accept_failure_is_retried_instead_of_ending_the_flow() {
    for kind in [
        io::ErrorKind::Interrupted,
        io::ErrorKind::ConnectionAborted,
        io::ErrorKind::ConnectionReset,
    ] {
        let mut pending = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut calls = 0;
        let result = accept_pending_with(
            || {
                calls += 1;
                if calls == 1 {
                    Err(io::Error::from(kind))
                } else {
                    Err(io::Error::from(io::ErrorKind::WouldBlock))
                }
            },
            &mut pending,
            deadline,
            Duration::from_secs(5),
        );

        assert!(
            result.is_ok(),
            "an {kind:?} accept failure must not end the flow, got: {result:?}"
        );
        assert_eq!(
            calls, 2,
            "the accept call must be retried after an {kind:?} failure"
        );
        assert!(
            pending.is_empty(),
            "a failed accept must not add a connection to the pending list"
        );
    }
}
