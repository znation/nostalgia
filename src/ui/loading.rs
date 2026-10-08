//! The service-to-message adapter: mapping `AppleMusicService` results onto
//! the UI's `Message`s and reporting failures.
//!
//! A library fetch or playback call returns a `Result`; the `update` loop
//! needs a `Message`. These helpers are the seam between the two: they map a
//! successful result to its `*Loaded`/`TrackPlayed` message, fall back to an
//! empty list on error, and format the stderr report. The fetch helper also
//! carries each browse request's [`RequestGeneration`] so a reply a newer
//! request has superseded is dropped rather than stored. They know the
//! `AppleMusicService` seam and the `Message` type, not the `WinampPlayer`
//! event loop, so they live in their own module, mirroring `views` and
//! `transport`.

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::Task;

use crate::apple_music::AppleMusicService;

use super::Message;

/// Maps a library-fetch `Result` to its matching `*Loaded` message: the
/// fetched items on success, an empty list on error — after handing the
/// error to `report_error`, so a failed fetch is never dropped silently.
/// Shared by the artists, albums, and songs load paths so the error fallback
/// (and its reporting) stays identical in all three. `report_error` is
/// injected rather than hardcoded so the reporting contract is testable
/// without capturing stderr.
fn loaded_or_empty<T, E>(
    result: Result<Vec<T>, E>,
    loaded: impl Fn(Vec<T>) -> Message,
    report_error: impl FnOnce(&E),
) -> Message {
    match result {
        Ok(items) => loaded(items),
        Err(err) => {
            report_error(&err);
            loaded(Vec::new())
        }
    }
}

/// Maps a playback `Result` to the `TrackPlayed` completion message, handing
/// any error to `report_error` first so a failed play is never dropped
/// silently — a backend that rejects a track would otherwise look like the
/// button did nothing. The playback twin of [`loaded_or_empty`], which
/// cannot serve here: it maps `Result<Vec<T>, _>` onto a `*Loaded` message,
/// while a play has no payload to load, only a completion. `report_error`
/// is injected rather than hardcoded so the reporting contract is testable
/// without capturing stderr. `generation` is the play's place in the playback
/// stream; the completion carries it so `update` can skip pruning the title
/// index on a play a newer selection has superseded.
pub(super) fn played_or_reported<E>(
    result: Result<(), E>,
    generation: u64,
    report_error: impl FnOnce(&E),
) -> Message {
    if let Err(err) = result {
        report_error(&err);
    }
    Message::TrackPlayed { generation }
}

/// Formats the browse-fetch failure report: names the fetch that failed
/// (e.g. "loading albums for artist \"artist-1\""), states the empty-list
/// fallback, and includes the underlying error. The play path names the
/// offending track; this names the offending query, so a failed browse tells
/// the user which fetch failed and what it was fetching. Kept as a pure
/// function so the report contract is testable without capturing stderr.
/// The error is formatted with `Display`, not `Debug`, so a real backend's
/// error reads as its human-readable cause (see `AppleMusicError`'s `Display`
/// impl) rather than a struct dump.
fn fetch_failure_report<E: std::fmt::Display>(context: &str, err: &E) -> String {
    format!("music-library fetch failed ({context}); showing an empty list: {err}")
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
    pub(super) fn issue(latest: &Arc<AtomicU64>) -> Self {
        Self {
            issued: latest.fetch_add(1, Ordering::SeqCst) + 1,
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

/// Runs a library-fetch future through iced's runtime, mapping its `Result`
/// onto the matching `*Loaded` message (empty list on error, via
/// [`loaded_or_empty`], with the error reported to stderr via
/// [`fetch_failure_report`]). `context` names the fetch — "loading favorite
/// artists", "loading albums for artist \"artist-1\"", or "loading songs from
/// album \"album-1\"" — so a failed browse reports *which* query failed and
/// what it was fetching, not just that a fetch failed. `generation` names the
/// request's place in its list's stream: when a newer request for the same
/// list has been issued by the time this fetch completes, the reply is
/// superseded and becomes [`Message::Ignored`] instead of overwriting the
/// newer list. Shared by the artists, albums, and songs load arms so none of
/// them repeats the clone-the-service-then-`Task::perform` boilerplate.
pub(super) fn fetch_into<T, E, Fut>(
    service: &AppleMusicService,
    context: String,
    generation: RequestGeneration,
    fetch: impl FnOnce(AppleMusicService) -> Fut + Send + 'static,
    loaded: impl Fn(Vec<T>) -> Message + Send + 'static,
) -> Task<Message>
where
    T: Send + 'static,
    E: std::fmt::Display + Send + 'static,
    Fut: Future<Output = Result<Vec<T>, E>> + Send + 'static,
{
    let service = service.clone();
    Task::perform(async move { fetch(service).await }, move |result| {
        if !generation.is_current() {
            return Message::Ignored;
        }
        loaded_or_empty(result, loaded, |err| {
            eprintln!("{}", fetch_failure_report(&context, err))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Album;
    use crate::test_support::sample_album;

    #[test]
    fn loaded_or_empty_maps_ok_and_err_to_loaded() {
        let albums = vec![sample_album()];

        let ok_message =
            loaded_or_empty::<Album, String>(Ok(albums.clone()), Message::AlbumsLoaded, |_| {
                unreachable!("ok path must not report an error")
            });
        assert!(matches!(ok_message, Message::AlbumsLoaded(v) if v == albums));

        let err_message = loaded_or_empty::<Album, String>(
            Err("boom".to_string()),
            Message::AlbumsLoaded,
            |_| {},
        );
        assert!(matches!(err_message, Message::AlbumsLoaded(v) if v.is_empty()));
    }

    #[test]
    fn loaded_or_empty_reports_the_error_before_falling_back_to_empty() {
        // A failed browse fetch must not vanish silently: the load path
        // reports the error (to stderr in production; here to a recording
        // closure) and still falls back to an empty list so the UI stays
        // usable. `loaded_or_empty` takes the reporter as a parameter so this
        // contract is testable without capturing stderr.
        let mut reported: Option<String> = None;
        let message = loaded_or_empty::<Album, String>(
            Err("boom".to_string()),
            Message::AlbumsLoaded,
            |err| reported = Some(err.clone()),
        );

        assert_eq!(reported.as_deref(), Some("boom"));
        assert!(matches!(message, Message::AlbumsLoaded(v) if v.is_empty()));
    }

    #[test]
    fn played_or_reported_reports_a_failed_play_and_still_completes() {
        // A failed play must not vanish silently: the playback path reports
        // the error (to stderr in production; here to a recording closure)
        // and still emits the `TrackPlayed` completion so the UI's handoff
        // stays intact. `played_or_reported` takes the reporter as a
        // parameter so this contract is testable without capturing stderr.
        let mut reported: Option<String> = None;
        let message = played_or_reported::<String>(Err("boom".to_string()), 7, |err| {
            reported = Some(err.clone());
        });

        assert_eq!(reported.as_deref(), Some("boom"));
        assert!(matches!(message, Message::TrackPlayed { generation: 7 }));

        // The Ok path completes with the same message and reports nothing.
        let mut reported_ok: Option<String> = None;
        let ok_message = played_or_reported::<String>(Ok(()), 8, |err| {
            reported_ok = Some(err.clone());
        });
        assert!(reported_ok.is_none());
        assert!(matches!(ok_message, Message::TrackPlayed { generation: 8 }));
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
            "music-library fetch failed (loading albums for artist \"artist-1\"); showing an empty list: boom"
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
}
