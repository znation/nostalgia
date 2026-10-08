//! The service-to-message adapter: mapping `AppleMusicService` results onto
//! the UI's `Message`s and reporting failures.
//!
//! A library fetch or playback call returns a `Result`; the `update` loop
//! needs a `Message`. These helpers are the seam between the two: they map a
//! successful result to its `*Loaded`/`TrackPlayed` message, map a failed
//! fetch to its `*LoadFailed` message, and format the stderr report. The
//! fetch helper also carries each browse request's [`RequestGeneration`] so a
//! reply a newer request has superseded is dropped rather than stored. They
//! know the `AppleMusicService` seam and the `Message` type, not the
//! `WinampPlayer` event loop, so they live in their own module, mirroring
//! `views` and `transport`.

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use iced::Task;

use crate::apple_music::AppleMusicService;

use super::Message;

/// Runs a playback future through iced's runtime, bounding it with
/// [`with_timeout`] and mapping it to the `TrackPlayed` completion message.
///
/// The playback twin of [`fetch_into`]: a play has no payload to load, only a
/// completion, so success maps straight to `TrackPlayed` while an error is
/// handed to `report_error` first so a failed play is never dropped silently —
/// a backend that rejects a track would otherwise look like the button did
/// nothing. A backend that never answers is the failure `timeout` exists to
/// bound: without it the play's task would stay pending forever and never emit
/// its completion, so the title recorded for it would linger until some later
/// play pruned it. A timeout is reported with [`play_timeout_report`] and still
/// emits `TrackPlayed`, so the completion path is never stranded. `generation`
/// is the play's place in the playback stream; the completion carries it so
/// `update` can skip pruning the title index on a play a newer selection has
/// superseded. `play` is injected — rather than calling the service directly —
/// so a test can hang it and prove the bound.
pub(super) fn play_into<E, Fut>(
    service: &AppleMusicService,
    track_id: String,
    generation: RequestGeneration,
    timeout: Duration,
    play: impl FnOnce(AppleMusicService, String, RequestGeneration) -> Fut + Send + 'static,
    report_error: impl FnOnce(&E) + Send + 'static,
) -> Task<Message>
where
    E: Send + 'static,
    Fut: Future<Output = Result<(), E>> + Send + 'static,
{
    let service = service.clone();
    let play_generation = generation.issued();
    let report_id = track_id.clone();
    Task::perform(
        async move { with_timeout(timeout, play(service, track_id, generation)).await },
        move |result| {
            match result {
                Some(Ok(())) => {}
                Some(Err(err)) => report_error(&err),
                None => eprintln!("{}", play_timeout_report(&report_id, timeout)),
            }
            Message::TrackPlayed {
                generation: play_generation,
            }
        },
    )
}

/// Formats the browse-fetch failure report: names the fetch that failed
/// (e.g. "loading albums for artist \"artist-1\"") and includes the underlying
/// error. The play path names the offending track; this names the offending
/// query, so a failed browse tells the user which fetch failed and what it
/// was fetching. The same report is both written to stderr and shown in the
/// browse panel (via the `*LoadFailed` message). Kept as a pure function so
/// the report contract is testable without capturing stderr. The error is
/// formatted with `Display`, not `Debug`, so a real backend's error reads as
/// its human-readable cause (see `AppleMusicError`'s `Display` impl) rather
/// than a struct dump.
fn fetch_failure_report<E: std::fmt::Display>(context: &str, err: &E) -> String {
    format!("music-library fetch failed ({context}): {err}")
}

/// Formats the playback failure report: names the offending track and
/// includes the underlying error. The playback twin of
/// [`fetch_failure_report`], and pure for the same reason — the play path's
/// stderr report is testable without capturing stderr. The error is formatted
/// with `Display` so a real backend's failure reads as its human-readable
/// cause rather than a struct dump.
pub(super) fn play_failure_report<E: std::fmt::Display>(track_id: &str, err: &E) -> String {
    format!("failed to play track {track_id:?}: {err}")
}

/// The sequence position of one browse request within its list's request
/// stream, paired with the shared counter naming the list's latest request.
///
/// A slow fetch can finish after a newer request for the same list has
/// already landed; storing that older reply would replace the newer list
/// with stale data — the wrong artist's albums, say. The counter is bumped
/// when a request is issued, so [`Self::is_current`] lets the completion
/// tell whether it is still the newest request and its result may be stored.
/// The counter is an `Arc<AtomicU64>` because the completion runs on iced's
/// executor while the player's UI thread issues later requests against the
/// same counter.
pub(super) struct RequestGeneration {
    issued: u64,
    latest: Arc<AtomicU64>,
}

impl RequestGeneration {
    /// Records a new request as the latest for its list and returns the
    /// generation that request's completion must still match to be stored.
    ///
    /// The `+ 1` wraps rather than overflowing: `fetch_add` already wraps the
    /// counter at `u64::MAX` back to `0`, so a plain `+ 1` on the value it
    /// returns would panic in a debug build at that boundary. Wrapping keeps
    /// `issued` equal to the counter `fetch_add` just stored (and matches
    /// `browse::BrowseList::store`'s epoch bump). Reaching the boundary takes
    /// `2^64` requests, but the two counters should not disagree on how they
    /// step.
    pub(super) fn issue(latest: &Arc<AtomicU64>) -> Self {
        Self {
            issued: latest.fetch_add(1, Ordering::SeqCst).wrapping_add(1),
            latest: Arc::clone(latest),
        }
    }

    /// The generation this request was issued as. Carried through to a play's
    /// completion message so `update` can compare it against the shared
    /// counter and recognize a superseded completion.
    pub(super) fn issued(&self) -> u64 {
        self.issued
    }

    /// Whether this request is still the latest issued for its list. A later
    /// request for the same list makes an earlier completion stale.
    pub(super) fn is_current(&self) -> bool {
        self.issued == self.latest.load(Ordering::SeqCst)
    }
}

/// The longest a browse fetch may run before the UI gives up on it.
///
/// The `loading` flag a navigation sets (see `browse::BrowseList::clear`) only
/// clears when the reply lands, so a backend that never answers would leave
/// the browse panel stuck on "Loading…" forever with no way back. Bounding
/// the wait turns that silent hang into a reported failure and the ordinary
/// empty-list fallback, exactly as a returned error does.
pub(super) const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// The longest a playback call may run before the UI gives up on it.
///
/// The play path has no loading flag to strand, but a backend that never
/// answers would leave the play's task pending forever, so its `TrackPlayed`
/// completion never fires and the title recorded for it lingers in the
/// player's title index until some later play prunes it. Bounding the wait
/// turns that silent hang into a reported failure and still delivers the
/// completion, exactly as a returned error does.
pub(super) const PLAY_TIMEOUT: Duration = Duration::from_secs(30);

/// Awaits `future`, returning `None` when `timeout` elapses first.
///
/// A service fetch with no bound would hang the browse panel indefinitely
/// (see [`FETCH_TIMEOUT`]); this races the future against a timer and reports
/// which one finished. The timer is a detached thread that waits on a channel
/// with `recv_timeout`, so it exits as soon as the future completes instead
/// of sleeping the full `timeout`, and no runtime timer is required — the
/// future runs under iced's executor, which has none. `tokio::select!` only
/// polls the two futures, so it works outside a tokio runtime too.
pub(super) async fn with_timeout<F: Future>(timeout: Duration, future: F) -> Option<F::Output> {
    let (fired, timer) = tokio::sync::oneshot::channel::<()>();
    let (done, finished) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        // Wake early when the request finishes; fire the timer only if
        // `timeout` elapses first. A completed request leaves no thread
        // sleeping for the full bound.
        if finished.recv_timeout(timeout).is_err() {
            let _ = fired.send(());
        }
    });
    tokio::select! {
        biased;
        output = future => {
            let _ = done.send(());
            Some(output)
        }
        _ = timer => None,
    }
}

/// Formats the browse-fetch timeout report: names the fetch that timed out
/// and states the empty-list fallback, mirroring [`fetch_failure_report`] so
/// a hung backend is as diagnosable from the log as one that returned an
/// error. Pure, like the other report formatters, so the contract is
/// testable without capturing stderr.
fn fetch_timeout_report(context: &str, timeout: Duration) -> String {
    format!(
        "music-library fetch failed ({context}); timed out after {timeout:?}, showing an empty list"
    )
}

/// Formats the playback timeout report: names the track that was being played
/// and states that no track change was committed, mirroring
/// [`fetch_timeout_report`] so a hung backend is as diagnosable from the log as
/// one that returned an error. Pure, like the other report formatters, so the
/// contract is testable without capturing stderr.
fn play_timeout_report(track_id: &str, timeout: Duration) -> String {
    format!(
        "playback failed (playing track {track_id:?}); timed out after {timeout:?}, leaving the current track unchanged"
    )
}

/// Runs a library-fetch future through iced's runtime, mapping its `Result`
/// onto the matching message: the `*Loaded` message built by `loaded` on
/// success, or the `*LoadFailed` message built by `failed` on error (after
/// the error is reported to stderr via [`fetch_failure_report`], whose
/// report is the string handed to `failed`). `context` names the fetch —
/// "loading favorite artists", "loading albums for artist \"artist-1\"", or
/// "loading songs from album \"album-1\"" — so a failed browse reports *which*
/// query failed and what it was fetching, not just that a fetch failed.
/// `generation` names the request's place in its list's stream: when a newer
/// request for the same list has been issued by the time this fetch completes,
/// the reply is superseded and becomes [`Message::Ignored`] instead of
/// overwriting the newer list. `timeout` bounds the wait via [`with_timeout`],
/// so a backend that never answers falls back to the empty list instead of
/// leaving the panel on "Loading…". Shared by the artists, albums, and songs
/// load arms so none of them repeats the clone-the-service-then-`Task::perform`
/// boilerplate.
pub(super) fn fetch_into<T, E, Fut>(
    service: &AppleMusicService,
    context: String,
    generation: RequestGeneration,
    timeout: Duration,
    fetch: impl FnOnce(AppleMusicService) -> Fut + Send + 'static,
    loaded: impl Fn(Vec<T>) -> Message + Send + 'static,
    failed: impl Fn(String) -> Message + Send + 'static,
) -> Task<Message>
where
    T: Send + 'static,
    E: std::fmt::Display + Send + 'static,
    Fut: Future<Output = Result<Vec<T>, E>> + Send + 'static,
{
    let service = service.clone();
    Task::perform(
        async move { with_timeout(timeout, fetch(service)).await },
        move |result| {
            if !generation.is_current() {
                return Message::Ignored;
            }
            match result {
                Some(Ok(items)) => loaded(items),
                Some(Err(err)) => {
                    let report = fetch_failure_report(&context, &err);
                    eprintln!("{report}");
                    failed(report)
                }
                None => {
                    eprintln!("{}", fetch_timeout_report(&context, timeout));
                    loaded(Vec::new())
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // The playback twin of `fetch_timeout_report`'s test: a hung play's report
    // must name the track and the bound that expired, so it is as diagnosable
    // from the log as a returned error. Pinned so the wording can't drift.
    #[test]
    fn play_timeout_report_names_the_track_and_the_bound() {
        // Format with the production `PLAY_TIMEOUT`, not a test-local
        // literal: the report the app logs must describe the bound the app
        // actually applies, so the expected wording below pins the constant
        // too. A test-local 30s would pass even if the production constant
        // changed, leaving the log naming a bound nothing enforces.
        let report = play_timeout_report("song-1", PLAY_TIMEOUT);
        assert_eq!(
            report,
            "playback failed (playing track \"song-1\"); timed out after 30s, leaving the current track unchanged"
        );
    }

    #[test]
    fn fetch_failure_report_names_the_fetch_and_includes_the_error() {
        // The browse error report must identify the failing query (the fetch
        // context plus the underlying error), not just say a fetch failed —
        // otherwise a broken backend would log three identical lines for the
        // artists, albums, and songs paths with no way to tell which query
        // failed. The format is pinned here so the contract can't drift.
        let report = fetch_failure_report("loading albums for artist \"artist-1\"", &"boom");
        assert_eq!(
            report,
            "music-library fetch failed (loading albums for artist \"artist-1\"): boom"
        );
    }

    #[test]
    fn play_failure_report_names_the_track_and_includes_the_error() {
        // The playback error report is the sibling of the browse one: it must
        // name the offending track and include the underlying error, so a
        // rejected play is diagnosable from the log. `Display` formatting is
        // pinned here too — `&str` renders bare, so a regression to `Debug`
        // would quote it as `"boom"`.
        let report = play_failure_report("track-1", &"boom");
        assert_eq!(report, "failed to play track \"track-1\": boom");
    }

    // `with_timeout` is the bound `fetch_into` puts on a backend reply. The
    // completing half: a future that resolves must yield its output, not the
    // timeout's `None`.
    #[test]
    fn with_timeout_returns_the_output_when_the_future_completes() {
        let output = futures::executor::block_on(with_timeout(Duration::from_secs(1), async { 7 }));
        assert_eq!(output, Some(7));
    }

    // The failing half: a future that never resolves must yield `None` rather
    // than hang the caller, which is what leaves the browse panel on
    // "Loading…" forever without the bound.
    #[test]
    fn with_timeout_returns_none_when_the_future_never_completes() {
        let output = futures::executor::block_on(with_timeout(
            Duration::from_millis(10),
            std::future::pending::<()>(),
        ));
        assert_eq!(output, None);
    }

    // The timeout report names the fetch and the bound that expired, so a
    // hung backend is as diagnosable from the log as one that returned an
    // error. Pinned so the wording can't drift.
    #[test]
    fn fetch_timeout_report_names_the_fetch_and_the_bound() {
        // The playback report's twin: format with the production
        // `FETCH_TIMEOUT` so the wording pins the bound the browse path
        // actually applies.
        let report = fetch_timeout_report("loading favorite artists", FETCH_TIMEOUT);
        assert_eq!(
            report,
            "music-library fetch failed (loading favorite artists); timed out after 30s, showing an empty list"
        );
    }

    // `FETCH_TIMEOUT` and `PLAY_TIMEOUT` are production behavior — how long a
    // hung backend may run before the UI gives up and falls back (an empty
    // list, or the current track left unchanged) — and every fetch and play
    // in `ui` passes one of them. Nothing read them: the report tests above
    // used to pass their own `Duration::from_secs(30)`, so a change to either
    // constant (a zero bound would abandon every fetch and play instantly)
    // would clear the whole suite while production applied the new bound.
    // Pin the documented 30 seconds for both.
    #[test]
    fn fetch_and_play_timeouts_are_the_documented_thirty_seconds() {
        assert_eq!(FETCH_TIMEOUT, Duration::from_secs(30));
        assert_eq!(PLAY_TIMEOUT, Duration::from_secs(30));
    }

    // The counter wraps at `u64::MAX` instead of overflowing. `fetch_add`
    // already wraps the atomic back to `0`, so the `+ 1` that derives `issued`
    // from the value it returns must wrap too, or a debug build would panic at
    // the boundary and leave `issued` disagreeing with the counter. Seeded at
    // `u64::MAX` because that is the only value the next `fetch_add` wraps
    // from, and pinning `is_current` keeps the wrapped generation usable.
    #[test]
    fn issue_wraps_the_generation_counter_at_the_maximum() {
        let latest = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(u64::MAX));
        let generation = RequestGeneration::issue(&latest);
        assert_eq!(generation.issued(), 0);
        assert!(generation.is_current());
    }
}
