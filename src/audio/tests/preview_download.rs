use super::*;

// A preview URL is server-controlled data (it arrives in the Apple Music API
// reply), and the preview agent allows any public host by design, so a hostile
// preview server can answer the download. The 10 MiB body cap must count the
// bytes the decoder produces, not the bytes on the wire: `ureq`'s
// `read_to_vec` limiter sits under its gzip decoder, so a small
// `Content-Encoding: gzip` body expands past the cap. This body is a fixed
// Huffman stream that decodes to just over the cap; a wire-byte cap reads it
// and the over-cap bytes reach the decoder (the pre-fix result is a decode
// failure on the zeros), while a decoded-byte cap rejects it.
#[test]
fn a_compressed_preview_larger_than_the_cap_is_rejected() {
    let cap = usize::try_from(MAX_PREVIEW_BYTES).expect("the cap fits a usize");
    let body = gzip_zeros(cap / 258 + 1);
    assert!(
        body.len() < cap,
        "the compressed body must stay under the wire cap, got {}",
        body.len()
    );
    let address = serve_one_response_bytes(&gzip_preview_response(&body));
    let agent = agent_with_timeout(Duration::from_secs(10));
    let url = format!("http://{address}/bomb.m4a");

    let error = download_and_decode(&agent, &url)
        .err()
        .expect("an over-cap preview must not reach the decoder");
    assert!(
        error.to_string().contains("larger than the 10 MiB cap"),
        "unexpected error: {error}"
    );
}

// The decoded-byte cap alone would leave the download willing to read an
// arbitrarily large compressed body that decodes to little: an empty stored
// block costs five wire bytes and produces no output. This body puts more than
// the cap on the wire while decoding to zero bytes, so only `ureq`'s
// wire-byte limit can reject it; without that limit the read succeeds and the
// failure moves to the decoder.
#[test]
fn a_preview_with_more_wire_bytes_than_the_cap_is_rejected() {
    let cap = usize::try_from(MAX_PREVIEW_BYTES).expect("the cap fits a usize");
    let body = gzip_empty_stored_blocks(cap / 5 + 1);
    assert!(
        body.len() > cap,
        "the compressed body must exceed the wire cap, got {}",
        body.len()
    );
    let address = serve_one_response_bytes(&gzip_preview_response(&body));
    let agent = agent_with_timeout(Duration::from_secs(10));
    let url = format!("http://{address}/bomb.m4a");

    let error = download_and_decode(&agent, &url)
        .err()
        .expect("an over-cap wire body must not reach the decoder");
    assert!(
        error.to_string().contains("reading the preview failed"),
        "unexpected error: {error}"
    );
}

// A preview body of exactly the cap is legal: `ureq`'s wire limit must sit
// one byte above the cap so the read past an exactly-at-cap body reaches EOF
// instead of tripping the limit on its last byte. A limit of exactly the cap
// rejects this body even though nothing exceeds the cap. The bytes are not a
// valid MP4, so an accepted read surfaces as a decode failure; before the fix
// it surfaced as a read failure from the wire limit.
#[test]
fn a_preview_of_exactly_the_cap_reaches_the_decoder() {
    let cap = usize::try_from(MAX_PREVIEW_BYTES).expect("the cap fits a usize");
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: audio/mp4\r\nContent-Length: {cap}\r\nConnection: close\r\n\r\n"
    )
    .into_bytes();
    response.resize(response.len() + cap, 0);
    let address = serve_one_response_bytes(&response);
    let agent = agent_with_timeout(Duration::from_secs(10));
    let url = format!("http://{address}/preview.m4a");

    let error = download_and_decode(&agent, &url)
        .err()
        .expect("zero bytes are not a decodable preview");
    assert!(
        error.to_string().contains("decoding the preview failed"),
        "an exactly-at-cap preview must be read whole before decoding, got: {error}"
    );
}

/// The replay path through the worker's fetch: the first play of a URL runs
/// the load (download plus decode) and the second decodes the cached body
/// without loading it again.
#[test]
fn preview_cache_replays_the_same_url_without_loading_it_again() {
    fn decode_len(body: Arc<[u8]>) -> Result<usize, AppleMusicError> {
        Ok(body.len())
    }

    let cache = PreviewCache::default();
    let loads = std::cell::Cell::new(0);
    let load = || {
        loads.set(loads.get() + 1);
        Ok::<_, AppleMusicError>((3, Arc::<[u8]>::from(&[1u8, 2, 3][..])))
    };
    assert_eq!(cache.load_or_decode("a", load, decode_len).unwrap(), 3);
    let load = || {
        loads.set(loads.get() + 1);
        Ok::<_, AppleMusicError>((3, Arc::<[u8]>::from(&[1u8, 2, 3][..])))
    };
    assert_eq!(cache.load_or_decode("a", load, decode_len).unwrap(), 3);
    assert_eq!(loads.get(), 1);
}

/// The cache is single-entry: a different URL replaces the body, and a load
/// that fails (a download or decode error) is not cached, so the next play of
/// that URL tries again.
#[test]
fn preview_cache_keeps_only_the_last_url_and_never_caches_a_failed_load() {
    fn decode_len(body: Arc<[u8]>) -> Result<usize, AppleMusicError> {
        Ok(body.len())
    }

    let cache = PreviewCache::default();
    let loads = std::cell::Cell::new(0);
    let load = || {
        loads.set(loads.get() + 1);
        Ok::<_, AppleMusicError>((1, Arc::<[u8]>::from(&[1u8][..])))
    };
    assert_eq!(cache.load_or_decode("a", load, decode_len).unwrap(), 1);
    let load = || {
        loads.set(loads.get() + 1);
        Ok::<_, AppleMusicError>((2, Arc::<[u8]>::from(&[2u8, 3][..])))
    };
    assert_eq!(cache.load_or_decode("b", load, decode_len).unwrap(), 2);
    // The older URL was replaced, so replaying it loads again.
    let load = || {
        loads.set(loads.get() + 1);
        Ok::<_, AppleMusicError>((1, Arc::<[u8]>::from(&[1u8][..])))
    };
    assert_eq!(cache.load_or_decode("a", load, decode_len).unwrap(), 1);
    assert_eq!(loads.get(), 3);

    let cache = PreviewCache::default();
    let loads = std::cell::Cell::new(0);
    let fail = || {
        loads.set(loads.get() + 1);
        Err::<(usize, Arc<[u8]>), _>(AppleMusicError::new("download failed"))
    };
    assert!(cache.load_or_decode("a", fail, decode_len).is_err());
    let fail = || {
        loads.set(loads.get() + 1);
        Err::<(usize, Arc<[u8]>), _>(AppleMusicError::new("download failed"))
    };
    assert!(cache.load_or_decode("a", fail, decode_len).is_err());
    assert_eq!(loads.get(), 2);
}

// `ureq` defaults every network timeout to `None`, so a preview server
// that accepts the connection and never responds used to block the audio
// worker forever, and every later play/pause/stop command queued behind
// it. The bound is asserted from a worker thread with `recv_timeout`, so
// an unbounded download fails this test at 5s instead of hanging the
// suite.
#[test]
fn a_stalled_preview_download_is_bounded_by_the_timeout() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let agent = agent_with_timeout(Duration::from_millis(200));
    let url = format!("http://{address}/stalled");
    let (sender, receiver) = mpsc::channel();
    let started = Instant::now();
    std::thread::spawn(move || {
        let _ = sender.send(download_and_decode(&agent, &url));
    });
    let result = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("the download must return within its bound");
    assert!(result.is_err(), "a stalled preview download must error");
    // A connect failure would also return `Err`, instantly; require that
    // the call actually waited for the bound, so this test only passes
    // because the stalled response was timed out.
    assert!(
        started.elapsed() >= Duration::from_millis(100),
        "returned before the 200ms bound: {:?}",
        started.elapsed()
    );
}

// A preview URL that answers with a non-2xx status must be reported as a
// status failure, before its body reaches the decoder. The agent disables
// `ureq`'s status-as-error shortcut (`http_status_as_error(false)`), so
// this 404 arrives as an `Ok` response and `download_and_decode`'s own
// status check runs. The body is deliberately non-empty: were that check
// removed, these bytes would go to `new_mp4` and the function would report
// a decode failure, so pinning the exact status message keeps the check
// ahead of the decoder.
#[test]
fn a_preview_download_with_an_error_status_is_reported() {
    let body = "<html>captive portal, not a preview</html>";
    let address = serve_one_response(&format!(
        "HTTP/1.1 404 Not Found\r\nContent-Type: audio/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ));
    let agent = agent_with_timeout(Duration::from_secs(5));
    let url = format!("http://{address}/missing.m4a");

    let error = download_and_decode(&agent, &url)
        .err()
        .expect("a 404 preview must not decode");
    assert_eq!(
        error.to_string(),
        "the preview download returned HTTP 404 Not Found"
    );
}

// A preview URL's host is validated before it reaches this backend, but a
// hostile or compromised API reply can point a public preview URL at a
// server that answers 302 to an internal address, which the one-time host
// check cannot see. `ureq` follows redirects by default, so without this
// guard the fetch would reach an address the preview validation refused.
// Pin that the preview agent refuses to follow one, mirroring the REST
// agent.
#[test]
fn a_redirecting_preview_is_not_followed() {
    let (foreign, foreign_addr) = loopback_listener();

    // The redirecting server answers the one request with a 302 pointing
    // at the "foreign" server, exactly what a hostile or compromised API
    // reply could return.
    let redirector_addr = serve_one_response(&format!(
        "HTTP/1.1 302 Found\r\nLocation: http://{foreign_addr}/preview.m4a\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    ));

    // The foreign server records whether any redirected request arrived;
    // the shared recorder polls for one second, so "no request" is a
    // bounded, observable result rather than a hang.
    let foreign_thread = record_first_request(foreign);

    let agent = agent_with_timeout(Duration::from_secs(2));
    let url = format!("http://{redirector_addr}/preview.m4a");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(download_and_decode(&agent, &url));
    });
    let result = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("the redirecting preview must return within its bound");
    let error = result.err().expect("a 302 preview must not decode");
    assert_eq!(
        error.to_string(),
        "the preview download returned HTTP 302 Found"
    );

    let followed = foreign_thread.join().unwrap().request;
    assert!(
        followed.is_none(),
        "the preview fetch reached the redirect target: {followed:?}"
    );
}

// A response whose headers arrive but whose body cannot be read whole (a
// dropped connection mid-download) must be reported as a read failure,
// before its partial bytes reach the decoder. This is the preview
// downloader's read-error arm (the REST transport has the twin test);
// without it a truncated body would surface as a decode failure, naming
// the wrong cause, or a partial decode.
#[test]
fn a_truncated_preview_body_reports_a_read_error() {
    // Declare 100 bytes but send only four, then close the connection.
    let address = serve_one_response(
        "HTTP/1.1 200 OK\r\nContent-Type: audio/mp4\r\nContent-Length: 100\r\nConnection: close\r\n\r\nJUNK",
    );
    let agent = agent_with_timeout(Duration::from_secs(5));
    let url = format!("http://{address}/truncated.m4a");

    let error = download_and_decode(&agent, &url)
        .err()
        .expect("a truncated preview must not reach the decoder");
    assert!(
        error.to_string().starts_with("reading the preview failed:"),
        "unexpected error: {error}"
    );
}

// A 2xx response whose body is not an MP4/AAC preview (a captive portal's
// HTML) must be reported as a decode failure, not panic the worker or play
// noise.
#[test]
fn a_preview_download_that_does_not_decode_is_reported() {
    let address = serve_one_response(
        "HTTP/1.1 200 OK\r\nContent-Type: audio/mp4\r\nContent-Length: 4\r\nConnection: close\r\n\r\nJUNK",
    );
    let agent = agent_with_timeout(Duration::from_secs(5));
    let url = format!("http://{address}/not-a-preview.m4a");

    let error = download_and_decode(&agent, &url)
        .err()
        .expect("garbage bytes must not decode as an MP4 preview");
    assert!(
        error.to_string().contains("decoding the preview failed"),
        "unexpected error: {error}"
    );
}

// `preview_url_problem` rejects internal address *literals*, but a preview
// URL may name a hostname that resolves to an internal address, and the
// fetch resolves that name only at connect time. `preview_agent`'s resolver
// must refuse such a name before any connection: `localhost` resolves to
// `127.0.0.1`/`::1`, so the guarded agent reports a resolution failure
// (`HostNotFound`) instead of connecting. The unguarded agent is checked in
// the same test so the result cannot come from `localhost` merely failing
// to resolve: it reaches the connect stage and fails there instead.
#[test]
fn the_preview_agent_refuses_a_hostname_that_resolves_to_loopback() {
    let (listener, address) = loopback_listener();
    // Accept the unguarded fetch's connection and close it without a
    // response, so that fetch fails at the protocol level rather than on
    // resolution. The guarded fetch never connects, so this thread may
    // wait on `accept` until the test process ends.
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = read_some_request(&mut stream);
        }
    });
    let url = format!("http://localhost:{}/preview.m4a", address.port());

    let guarded = preview_agent_with_timeout(Duration::from_secs(2));
    let error = guarded
        .get(&url)
        .call()
        .expect_err("a loopback-resolving hostname must not connect");
    assert!(
        matches!(error, ureq::Error::HostNotFound),
        "the guarded resolver must refuse the name, got: {error:?}"
    );

    let unguarded = agent_with_timeout(Duration::from_secs(2));
    let error = unguarded
        .get(&url)
        .call()
        .expect_err("the unguarded fetch must fail on the closed connection");
    assert!(
        !matches!(error, ureq::Error::HostNotFound),
        "`localhost` must resolve for the guarded case to prove anything, got: {error:?}"
    );
}

// The resolver's allow branch: a preview URL that names a public address
// literal must pass through to the connector. `preview_url_problem`
// accepts such a URL, so a resolver that refused every address — an
// inverted internal-address check, say — would fail every legitimate
// preview literal, and the refusal test above (which only pins the
// internal case) would stay green. An IP literal is resolved without a
// DNS lookup, so this stays offline.
#[test]
fn the_preview_resolver_allows_a_public_address_literal() {
    let uri: ureq::http::Uri = "https://93.184.216.34/preview.m4a".parse().unwrap();
    let config = config_with_timeout(Duration::from_secs(1));
    let timeout = NextTimeout {
        after: ureq::unversioned::transport::time::Duration::from_secs(1),
        reason: ureq::Timeout::Resolve,
    };

    let addresses = PublicAddressResolver::default()
        .resolve(&uri, &config, timeout)
        .expect("a public address literal must not be refused");

    assert!(
        addresses
            .iter()
            .any(|address| address.ip().to_string() == "93.184.216.34"),
        "the resolver must return the public address it resolved: {addresses:?}"
    );
}

// Exercise the connect-time guard with literals, without DNS or a connection.
// Public counterparts prevent a blanket rejection of IPv6 from passing.
#[test]
fn the_preview_resolver_checks_ipv6_transition_literals() {
    use ureq::unversioned::resolver::DefaultResolver;

    let config = config_with_timeout(Duration::from_secs(1));
    for (host, internal) in [
        ("::ffff:0:10.0.0.1", true),
        ("::ffff:0:8.8.8.8", false),
        ("2001:4860::5efe:127.0.0.1", true),
        ("2001:4860::5efe:8.8.8.8", false),
        ("2002:0a00:0001::1", true),
        ("2002:0808:0808::1", false),
    ] {
        let uri: ureq::http::Uri = format!("https://[{host}]:8443/preview.m4a")
            .parse()
            .unwrap();
        let timeout = NextTimeout {
            after: ureq::unversioned::transport::time::Duration::from_secs(1),
            reason: ureq::Timeout::Resolve,
        };
        let expected = format!("[{host}]:8443")
            .parse::<std::net::SocketAddr>()
            .unwrap();
        let unguarded = DefaultResolver::default()
            .resolve(&uri, &config, timeout)
            .expect("the literal must resolve before the guard is applied");
        assert!(unguarded.contains(&expected));

        let guarded = PublicAddressResolver::default().resolve(&uri, &config, timeout);
        if internal {
            assert!(
                matches!(guarded, Err(ureq::Error::HostNotFound)),
                "the resolver must reject {host}, got {guarded:?}"
            );
        } else {
            let addresses = guarded.expect("a public transition literal must resolve");
            assert!(
                addresses.contains(&expected),
                "the resolver must preserve the public address and port for {host}"
            );
        }
    }
}

// The worker fetches through `preview_agent`, but the redirect test above
// and the non-success-status test below build their agents with
// `agent_with_timeout` directly. A regression that built `preview_agent`
// from `ureq`'s default configuration — no global timeout, redirects
// followed, status-as-error on — would leave those tests green while the
// production fetch could stall the worker forever, follow a redirect past
// the one-time host check, or discard Apple's error body. Pin the three
// shared transport choices on the production agent itself.
#[test]
fn the_preview_agent_carries_the_documented_transport_settings() {
    assert_eq!(PREVIEW_TIMEOUT, Duration::from_secs(30));
    let config = preview_agent().config();
    assert_eq!(config.timeouts().global, Some(PREVIEW_TIMEOUT));
    assert_eq!(config.max_redirects(), 0);
    assert!(!config.http_status_as_error());
}

// An HTTP(S) CONNECT proxy resolves the target host itself, so with
// `HTTPS_PROXY` (or `ALL_PROXY`) set, `ureq` sends the preview hostname to
// the proxy and never consults `preview_agent`'s resolver — the SSRF guard
// is bypassed. `preview_agent` therefore disables the proxy. The assertion
// runs in a child process of this same test binary: the proxy variables
// are process-global, and setting them here would race every other test
// that builds an agent. The parent forwards them only to the child.
#[test]
fn the_preview_agent_ignores_a_configured_proxy() {
    let child =
        std::process::Command::new(std::env::current_exe().expect("the test executable path"))
            .args([
                "--exact",
                "audio::tests::preview_download::with_a_proxy_set_the_preview_agent_stays_direct",
            ])
            .env("HTTPS_PROXY", "http://127.0.0.1:9")
            .env("ALL_PROXY", "http://127.0.0.1:9")
            .output()
            .expect("run the child test process");
    let stdout = String::from_utf8_lossy(&child.stdout);
    assert!(
        child.status.success() && stdout.contains("1 passed"),
        "the child test must run and pass; stdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&child.stderr)
    );
}

// Runs only in the child process `the_preview_agent_ignores_a_configured_proxy`
// spawns with the proxy variables set: the ordinary suite leaves them unset,
// and this returns without asserting rather than racing to set them.
#[test]
fn with_a_proxy_set_the_preview_agent_stays_direct() {
    if std::env::var_os("HTTPS_PROXY").is_none() && std::env::var_os("ALL_PROXY").is_none() {
        return;
    }
    let timeout = Duration::from_secs(1);
    // The REST agent inherits the environment proxy; this proves the
    // variables are visible to `ureq` in this process, so the preview
    // assertion below is not vacuously true.
    assert!(
        agent_with_timeout(timeout).config().proxy().is_some(),
        "the environment proxy must reach the REST agent's config"
    );
    assert!(
        preview_agent_with_timeout(timeout)
            .config()
            .proxy()
            .is_none(),
        "the preview fetch must be direct even when a proxy is configured"
    );
}

// The cached fetch the worker runs must not remember a body that failed to
// decode: the second play of the same URL goes back to the network. The
// server answers one connection, so the retry's download error (rather than a
// second decode error from a replayed body) shows the cache stayed empty.
#[test]
fn a_cached_fetch_does_not_replay_a_body_that_failed_to_decode() {
    let address = serve_one_response(
        "HTTP/1.1 200 OK\r\nContent-Type: audio/mp4\r\nContent-Length: 4\r\nConnection: close\r\n\r\nJUNK",
    );
    let agent = agent_with_timeout(Duration::from_secs(5));
    let url = format!("http://{address}/not-a-preview.m4a");
    let cache = PreviewCache::default();

    let first = download_and_decode_cached(&cache, &agent, &url)
        .err()
        .expect("garbage bytes must not decode as an MP4 preview");
    assert!(
        first.to_string().contains("decoding the preview failed"),
        "unexpected error: {first}"
    );

    let second = download_and_decode_cached(&cache, &agent, &url)
        .err()
        .expect("the one-shot server is gone, so the retry must fail");
    assert!(
        second
            .to_string()
            .contains("downloading the preview failed"),
        "the retry should have gone back to the network, got: {second}"
    );
}

/// A short, decodable M4A (AAC-LC, mono, 44.1 kHz): the CC0 1.0
/// `scratch_002.ogg` interface sound from Kenney (<https://kenney.nl/assets/interface-sounds>),
/// encoded to M4A. It gives the success-path tests below a real preview body
/// to decode, which the encrypted and invalid bodies elsewhere cannot.
const DECODABLE_PREVIEW: &[u8] = include_bytes!("fixtures/chew-scratch.m4a");

/// A `200 OK` response carrying `body` as an MP4 preview.
fn preview_response(body: &[u8]) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: audio/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
}

/// A `200 OK` response carrying `body` as a gzip-encoded MP4 preview.
fn gzip_preview_response(body: &[u8]) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: audio/mp4\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
}

// Every other download test stops at a decode error, so the successful
// composition was untested: a decodable body must come back as a source that
// yields samples, with `encoded_bytes` equal to the body's length (the LCD
// bitrate readout divides by it) and with the same body kept for a replay.
#[test]
fn a_downloaded_preview_decodes_and_reports_its_encoded_length() {
    let address = serve_one_response_bytes(&preview_response(DECODABLE_PREVIEW));
    let agent = agent_with_timeout(Duration::from_secs(5));
    let url = format!("http://{address}/preview.m4a");

    let mut preview = download_and_decode(&agent, &url).expect("the fixture is a decodable M4A");
    assert_eq!(
        preview.encoded_bytes,
        u64::try_from(DECODABLE_PREVIEW.len()).unwrap()
    );
    assert_eq!(preview.body.as_ref(), DECODABLE_PREVIEW);
    assert_eq!(preview.source.sample_rate().get(), 44_100);
    assert_eq!(preview.source.channels().get(), 1);
    assert!(
        preview.source.next().is_some(),
        "the fixture must decode to at least one sample"
    );
}

// The cache's replay path through the worker's fetch: the first play downloads
// and decodes the fixture, and the second play of the same URL decodes the
// stored body even though the one-shot server is gone. The generic cache tests
// above use a fake decoder, so this pins the real decode composition and the
// replayed body's encoded length that the worker uses.
#[test]
fn a_cached_preview_replays_the_same_decodable_body_without_the_server() {
    let address = serve_one_response_bytes(&preview_response(DECODABLE_PREVIEW));
    let agent = agent_with_timeout(Duration::from_secs(5));
    let url = format!("http://{address}/preview.m4a");
    let cache = PreviewCache::default();

    let mut first = download_and_decode_cached(&cache, &agent, &url)
        .expect("the first play downloads and decodes the fixture");
    assert_eq!(
        first.encoded_bytes,
        u64::try_from(DECODABLE_PREVIEW.len()).unwrap()
    );
    assert_eq!(first.body.as_ref(), DECODABLE_PREVIEW);
    assert!(first.source.next().is_some());

    let mut second = download_and_decode_cached(&cache, &agent, &url)
        .expect("the replay decodes the cached body after the one-shot server is gone");
    assert_eq!(second.encoded_bytes, first.encoded_bytes);
    assert_eq!(second.body.as_ref(), DECODABLE_PREVIEW);
    assert!(second.source.next().is_some());
}
