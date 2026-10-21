//! Preview download and its SSRF guard: the process-wide `ureq` agent, the
//! resolver that refuses internal addresses, and the MP4/AAC decode.

use std::io::Cursor;
use std::sync::OnceLock;
use std::time::Duration;

use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};

use crate::apple_music::ip_is_internal;
use crate::http::config_builder_with_timeout;
use crate::music_error::AppleMusicError;

/// The end-to-end bound on one preview download, from DNS lookup through
/// reading the response body. `ureq` defaults every network timeout to `None`,
/// so without this a server that accepts the connection and then stalls would
/// block the audio worker forever; because the worker serves commands one at a
/// time, every later play, pause, stop, and volume command would queue behind
/// it. Matches the REST client's `REQUEST_TIMEOUT` so both of the app's network
/// calls share one bound.
pub(super) const PREVIEW_TIMEOUT: Duration = Duration::from_secs(30);

/// The process-wide [`ureq::Agent`] the audio worker downloads previews
/// through, so consecutive plays reuse its pooled connection. It carries
/// [`PREVIEW_TIMEOUT`] as its global timeout, so a stalled server cannot block
/// the worker forever, refuses to follow a redirect, so a preview URL that
/// passed `preview_url_problem`'s one-time host check cannot be bounced to an
/// address that check refused, and resolves through [`PublicAddressResolver`],
/// so a hostname the URL names is refused when it resolves to an internal
/// address rather than only when the URL spells one as a literal. It is built
/// with no proxy, so that resolver always sees the preview host.
pub(super) fn preview_agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| preview_agent_with_timeout(PREVIEW_TIMEOUT))
}

/// [`preview_agent`]'s builder, with an injectable bound so a test can build
/// the same guarded agent without waiting out [`PREVIEW_TIMEOUT`].
///
/// The proxy is disabled so `ureq` resolves the preview host itself. With an
/// environment proxy configured (`HTTPS_PROXY`/`ALL_PROXY`), `ureq` sends the
/// target hostname to the proxy and never consults [`PublicAddressResolver`],
/// which would bypass the guard; a proxy that resolves the target differently
/// (DNS rebinding) would also defeat a local pre-check.
pub(super) fn preview_agent_with_timeout(timeout: Duration) -> ureq::Agent {
    ureq::Agent::with_parts(
        config_builder_with_timeout(timeout).proxy(None).build(),
        DefaultConnector::default(),
        PublicAddressResolver::default(),
    )
}

/// A [`Resolver`] that refuses to resolve a host to any internal address, so
/// the preview fetch cannot reach the local machine or a private network even
/// when the preview URL names a public-looking hostname.
///
/// `preview_url_problem` in [`crate::apple_music`] rejects internal *address
/// literals* before the fetch, but a hostname it accepts (a cloud metadata
/// name such as `metadata.google.internal`, an `/etc/hosts` alias such as
/// `ip6-localhost`, or an attacker-controlled name) is resolved by `ureq`'s
/// own resolver at connect time, after that check. Wrapping the default
/// resolver here validates every address it returns and errors with
/// [`ureq::Error::HostNotFound`] when any of them is internal, so the
/// connection is made only to an address the guard approved — closing the
/// resolve-after-check gap, not just the literal case. The preview agent
/// disables the proxy so this resolver always runs; a CONNECT proxy would
/// resolve the target itself and never call it.
#[derive(Debug, Default)]
pub(super) struct PublicAddressResolver {
    inner: DefaultResolver,
}

impl Resolver for PublicAddressResolver {
    fn resolve(
        &self,
        uri: &ureq::http::Uri,
        config: &ureq::config::Config,
        timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let addresses = self.inner.resolve(uri, config, timeout)?;
        if addresses.iter().any(|address| ip_is_internal(address.ip())) {
            return Err(ureq::Error::HostNotFound);
        }
        Ok(addresses)
    }
}

/// Downloads `url` through `agent` and decodes it as an `MP4`/`AAC` preview.
///
/// The Apple Music preview is an `M4A` (AAC in an `MP4` container), so the
/// decoder is hinted with that format. The whole asset is buffered in memory
/// before decoding because the preview is short and the decoder needs a
/// `Seek` source; `ureq` caps that buffer at its 10 MiB body limit.
///
/// # Errors
///
/// Returns an [`AppleMusicError`] when the download fails (including its
/// timeout), the response is not a success status, the body cannot be read, or
/// the bytes do not decode.
pub(super) fn download_and_decode(
    agent: &ureq::Agent,
    url: &str,
) -> Result<rodio::Decoder<Cursor<Vec<u8>>>, AppleMusicError> {
    let mut response = agent.get(url).call().map_err(|error| {
        AppleMusicError::new(format!("downloading the preview failed: {error}"))
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppleMusicError::new(format!(
            "the preview download returned HTTP {status}"
        )));
    }
    let bytes = response
        .body_mut()
        .read_to_vec()
        .map_err(|error| AppleMusicError::new(format!("reading the preview failed: {error}")))?;
    rodio::Decoder::new_mp4(Cursor::new(bytes))
        .map_err(|error| AppleMusicError::new(format!("decoding the preview failed: {error}")))
}
