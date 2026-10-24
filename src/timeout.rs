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
/// reported within `timeout`, and returns whether the thread started.
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
) -> bool {
    crate::threads::spawn_named(TIMER_THREAD_NAME, move || {
        if finished.recv_timeout(timeout).is_err() {
            let _ = fired.send(());
        }
    })
    .is_ok()
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
    ) -> bool,
) -> Option<F::Output> {
    let (fired, timer) = tokio::sync::oneshot::channel::<()>();
    let (done, finished) = std::sync::mpsc::channel::<()>();
    if !spawn_timer(timeout, finished, fired) {
        eprintln!("the timeout timer thread could not start; treating the wait as timed out");
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
            |_timeout, _finished, _fired| false,
        ));
        assert_eq!(output, None);
    }
}
