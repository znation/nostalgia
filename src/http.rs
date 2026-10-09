//! The shared blocking HTTP agent for the app's outbound `ureq` calls.
//!
//! Both the Apple Music REST client and the audio preview downloader send
//! their requests through an agent built here, so the two share one set of
//! transport choices instead of each pinning its own copy.

use std::time::Duration;

/// Builds a `ureq` agent with `timeout` as its end-to-end global bound, no
/// redirect following, and the status-as-error shortcut disabled.
///
/// `ureq` defaults every network timeout to `None`, so without the global
/// bound a server that accepts a connection and then stalls would block the
/// calling thread forever.
///
/// `max_redirects(0)` keeps a request on the single URL its caller built and
/// validated: the REST client sends the `Music-User-Token` credential in a
/// custom header that `ureq` does not strip across a redirect, and the audio
/// downloader validates its preview URL's host once before the fetch, so
/// following a `Location` elsewhere would defeat either check.
///
/// `http_status_as_error(false)` leaves a 4xx/5xx response as `Ok`, so the
/// caller can read its body and report the server's own error detail instead
/// of `ureq`'s bare status error.
pub(crate) fn agent_with_timeout(timeout: Duration) -> ureq::Agent {
    ureq::Agent::new_with_config(config_with_timeout(timeout))
}

/// The [`ureq::Agent`] configuration [`agent_with_timeout`] applies, separated
/// out so a caller that needs a custom resolver — the audio preview fetch,
/// which must validate the addresses a hostname resolves to — can build its
/// agent on the same transport choices instead of duplicating them.
pub(crate) fn config_with_timeout(timeout: Duration) -> ureq::config::Config {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
}
