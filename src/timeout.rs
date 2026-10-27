//! The crate-wide bounded-wait seam: races a future against a timer thread.
//!
//! Awaiting a service future with no bound can hang the caller forever. These
//! helpers return `None` when the bound elapses first, so the caller can report
//! a timeout instead of waiting on a backend that never answers. The race is
//! generic over the future and knows nothing about the UI or the Apple Music
//! seam; it lives beside `threads` because the timer it starts is one of that
//! module's named workers.

use std::future::Future;
use std::time::Duration;

/// Awaits `future`, returning `None` when `timeout` elapses first.
///
/// A service call with no bound would hang its caller indefinitely; this races
/// the future against a timer and reports which one finished. The timer is a
/// detached thread that waits on a channel with `recv_timeout`, so it exits as
/// soon as the future completes instead of sleeping the full `timeout`, and no
/// runtime timer is required — the future may run under iced's executor, which
/// has none. `tokio::select!` only polls the two futures, so it works outside
/// a tokio runtime too.
///
/// The timer thread is started through [`spawn_timer`]'s fallible spawn, so a
/// thread-creation failure is reported rather than aborting the app (see
/// [`with_timeout_with`]).
pub(crate) async fn with_timeout<F: Future>(timeout: Duration, future: F) -> Option<F::Output> {
    with_timeout_with(timeout, future, spawn_timer).await
}

/// The name [`spawn_timer`] gives the timeout thread. Naming it, like the
/// audio worker's `nostalgia-audio` and the REST worker's `nostalgia-rest`,
/// attributes a panic there to `nostalgia-timer` rather than `<unnamed>`.
const TIMER_THREAD_NAME: &str = "nostalgia-timer";

/// Starts the detached thread that fires `fired` only when `finished` has not
/// reported within `timeout`, returning the spawn result.
///
/// `crate::threads::spawn_named` starts the thread with the fallible
/// `std::thread::Builder::spawn` form; a thread-creation failure (thread
/// exhaustion, say) is returned to [`with_timeout_with`] instead of aborting
/// the app. The thread waits on `finished` with `recv_timeout`, so it exits as
/// soon as the future completes instead of sleeping the full `timeout`.
fn spawn_timer(
    timeout: Duration,
    finished: std::sync::mpsc::Receiver<()>,
    fired: tokio::sync::oneshot::Sender<()>,
) -> std::io::Result<()> {
    crate::threads::spawn_named(TIMER_THREAD_NAME, move || {
        if finished.recv_timeout(timeout).is_err() {
            let _ = fired.send(());
        }
    })
}

/// [`with_timeout`] with an injectable timer spawner, so a test can drive the
/// thread-creation-failure branch without making the OS refuse a thread.
///
/// When the timer cannot start there is no bound to race the future against;
/// the wait is reported as the timeout the bound promises rather than hanging
/// on a future that may never complete, and the cause is logged so the report
/// is not mistaken for a slow backend.
async fn with_timeout_with<F: Future>(
    timeout: Duration,
    future: F,
    spawn_timer: impl FnOnce(
        Duration,
        std::sync::mpsc::Receiver<()>,
        tokio::sync::oneshot::Sender<()>,
    ) -> std::io::Result<()>,
) -> Option<F::Output> {
    let (fired, timer) = tokio::sync::oneshot::channel::<()>();
    let (done, finished) = std::sync::mpsc::channel::<()>();
    if let Err(error) = spawn_timer(timeout, finished, fired) {
        eprintln!("{}", timeout_timer_failure_report(&error));
        return None;
    }
    tokio::select! {
        biased;
        output = future => {
            let _ = done.send(());
            Some(output)
        }
        _ = timer => None,
    }
}

/// The report for a timeout timer thread that could not start: it names the
/// cause and states the fallback, so a wait reported as timed out is not
/// mistaken for a slow backend. Pure, so the report contract is testable
/// without capturing stderr.
pub(super) fn timeout_timer_failure_report(error: &std::io::Error) -> String {
    format!("the timeout timer thread could not start: {error}; treating the wait as timed out")
}

#[cfg(test)]
mod tests {
    use super::*;

    // The completing half: a future that resolves must yield its output, not
    // the timeout's `None`.
    #[test]
    fn with_timeout_returns_the_output_when_the_future_completes() {
        let output = futures::executor::block_on(with_timeout(Duration::from_secs(1), async { 7 }));
        assert_eq!(output, Some(7));
    }

    // The failing half: a future that never resolves must yield `None` rather
    // than hang the caller, which is what leaves a caller on "Loading…"
    // forever without the bound.
    #[test]
    fn with_timeout_returns_none_when_the_future_never_completes() {
        let output = futures::executor::block_on(with_timeout(
            Duration::from_millis(10),
            std::future::pending::<()>(),
        ));
        assert_eq!(output, None);
    }

    // A timer thread that cannot start must not abort the executor the way
    // `std::thread::spawn` would, nor leave the caller unbounded: with no
    // timer to race, the wait is reported as the timeout the bound promises.
    // The spawner is injected so the OS need not refuse a real thread.
    #[test]
    fn with_timeout_reports_a_timeout_when_the_timer_thread_cannot_start() {
        let output = futures::executor::block_on(with_timeout_with(
            Duration::from_secs(1),
            async { 7 },
            |_timeout, _finished, _fired| Err(std::io::Error::other("no threads left")),
        ));
        assert_eq!(output, None);
    }

    // The timer thread must be told the future finished, so it exits at once
    // instead of sleeping the full bound. The injected spawner keeps the
    // receiver so the test can read the completion signal.
    #[test]
    fn with_timeout_signals_the_timer_when_the_future_completes() {
        let mut kept = None;
        let output = futures::executor::block_on(with_timeout_with(
            Duration::from_secs(60),
            async { 7 },
            |_timeout, finished, fired| {
                kept = Some((finished, fired));
                Ok(())
            },
        ));
        assert_eq!(output, Some(7));
        let (finished, _fired) = kept.expect("the spawner ran");
        assert_eq!(finished.try_recv(), Ok(()));
    }

    // A timeout must not signal completion: the timer's `finished` channel
    // is closed, not fed, when the bound wins.
    #[test]
    fn with_timeout_does_not_signal_completion_when_the_bound_wins() {
        let mut kept = None;
        let output = futures::executor::block_on(with_timeout_with(
            Duration::from_millis(1),
            std::future::pending::<()>(),
            |_timeout, finished, fired| {
                kept = Some(finished);
                let _ = fired.send(());
                Ok(())
            },
        ));
        assert_eq!(output, None);
        let finished = kept.expect("the spawner ran");
        assert_eq!(
            finished.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Disconnected)
        );
    }

    // When the future is ready and the timer has already fired, the future's
    // output wins (the select is biased toward it).
    #[test]
    fn with_timeout_prefers_a_ready_future_over_a_fired_timer() {
        let output = futures::executor::block_on(with_timeout_with(
            Duration::from_millis(1),
            async { 7 },
            |_timeout, _finished, fired| {
                let _ = fired.send(());
                Ok(())
            },
        ));
        assert_eq!(output, Some(7));
    }

    // A refused timer thread's report must name the cause as well as the
    // fallback: without the cause a log reader cannot tell thread exhaustion
    // from an internal fault, and the wait's `None` would look like a slow
    // backend. Pinned as text so the cause cannot be dropped again.
    #[test]
    fn timeout_timer_failure_report_names_the_cause_and_the_fallback() {
        let error = std::io::Error::other("no threads left");
        assert_eq!(
            timeout_timer_failure_report(&error),
            "the timeout timer thread could not start: no threads left; treating the wait as timed out"
        );
    }
}
