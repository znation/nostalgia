//! Opens the sign-in bootstrap page in the system browser and reaps the
//! short-lived opener process.

use std::io;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

/// Opens `path` in the system's default browser.
///
/// `path` is the owner-only bootstrap page (`bootstrap::BootstrapPage`) wrote, not the
/// sign-in URL: the URL carries the `state` nonce, and a command-line argument
/// is world-readable on Unix, so only the local file path may reach the opener.
///
/// The opener is a short-lived launcher, so it is reaped on a detached thread
/// (see [`spawn_opener`]) rather than left as a zombie for the rest of the
/// app's life.
///
/// # Errors
///
/// Returns the [`std::io::Error`] from spawning the platform's opener command;
/// the message names the command (e.g. `xdg-open`), so a missing opener is
/// diagnosable rather than a bare "No such file or directory".
// `apple_music::threads::browser_sign_in` passes this to [`authorize`]; it is
// public so the wiring in `apple_music` can supply the real opener.
pub fn open_in_browser(path: &str) -> io::Result<()> {
    if cfg!(target_os = "macos") {
        spawn_opener("open", &[path])
    } else if cfg!(target_os = "windows") {
        spawn_opener("cmd", &["/C", "start", "", path])
    } else {
        spawn_opener("xdg-open", &[path])
    }
}

/// Spawns `program` with `args`, naming it in any spawn error so a missing
/// opener reads as "`xdg-open`: No such file or directory" rather than a bare
/// OS error.
///
/// The opener exits as soon as it has handed the page to the browser, but a
/// [`std::process::Child`] dropped without a wait leaves a zombie for the rest
/// of the app's life, one per sign-in. Waiting here would block the sign-in
/// flow before it starts accepting callbacks, so [`reap_in_background`] waits
/// on its own thread and the flow continues immediately.
pub(super) fn spawn_opener(program: &str, args: &[&str]) -> io::Result<()> {
    let child = Command::new(program)
        .args(args)
        .spawn()
        .map_err(|error| io::Error::new(error.kind(), format!("{program}: {error}")))?;
    reap_in_background(child);
    Ok(())
}

/// How long the browser opener is given to exit before it is killed and reaped.
///
/// The opener is a short-lived launcher: it hands the page to the browser and
/// exits within milliseconds. A launcher still running after this bound is hung
/// (a wedged desktop session, say), and waiting on it forever would leak its
/// reaper thread — and the child process — for the life of the app.
pub(super) const OPENER_REAP_TIMEOUT: Duration = Duration::from_secs(30);

/// How often [`reap_with_timeout`] checks whether the opener has exited.
///
/// `std::process::Child::wait` blocks with no bound, so the wait is a poll:
/// this is the sleep between checks, short enough that a normal opener is
/// reaped promptly and long enough not to spin.
pub(super) const OPENER_REAP_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Waits on `child` on a detached thread so it is reaped without blocking the
/// caller. A failed wait is reported, and so is a nonzero exit: an opener that
/// cannot hand the page to a browser (a missing opener, or no browser
/// installed) exits nonzero, and without this line that failure stays invisible
/// until the flow reports its generic timeout minutes later. An opener that
/// never exits at all is killed and reaped once [`OPENER_REAP_TIMEOUT`]
/// elapses, and that is reported too. The status is only reported, not turned
/// into a flow failure: the opener has already run, and a browser that did open
/// still delivers the callback.
///
/// The reaper thread is started through [`spawn_reaper`], whose fallible spawn
/// keeps a thread-creation failure from panicking the sign-in thread; the child
/// is then killed and reaped synchronously and the failure is reported, so a
/// failed reaper cannot leak the opener as a zombie.
pub(super) fn reap_in_background(child: std::process::Child) {
    reap_with(child, |report| eprintln!("{report}"));
}

/// [`reap_in_background`] with an injectable reporter, so a test can observe
/// the report a nonzero opener exit produces without capturing stderr.
///
/// The wait runs on its own thread (see [`reap_in_background`]); `report` runs
/// on that thread and receives the one line to log, or nothing for a
/// successful exit.
pub(super) fn reap_with(child: std::process::Child, report: impl FnOnce(String) + Send + 'static) {
    reap_with_timeout(child, OPENER_REAP_TIMEOUT, report);
}

/// [`reap_with`] with an injectable deadline, so a test can prove a hung opener
/// is killed and reaped without waiting out [`OPENER_REAP_TIMEOUT`].
///
/// The child is polled with `try_wait` because `Child::wait` has no timeout: a
/// launcher that never exits would otherwise block this thread forever. On the
/// deadline the child is killed and then waited on, so it is reaped rather than
/// left as a zombie (a killed child still needs a `wait` to be reaped) and no
/// process is left running after the flow.
pub(super) fn reap_with_timeout(
    child: std::process::Child,
    timeout: Duration,
    report: impl FnOnce(String) + Send + 'static,
) {
    reap_with_timeout_and_spawn(child, timeout, report, spawn_reaper);
}

/// The name the browser-opener reaper thread gets, so a panic there is
/// attributed to `nostalgia-opener-reaper` rather than `<unnamed>`.
pub(super) const REAPER_THREAD_NAME: &str = "nostalgia-opener-reaper";

/// Starts the detached thread that waits on the opener through
/// [`crate::threads::spawn_named`], which uses the fallible
/// [`thread::Builder::spawn`] so a thread-creation failure is returned to
/// [`reap_with_timeout_and_spawn`] instead of panicking the sign-in thread.
pub(super) fn spawn_reaper(work: Box<dyn FnOnce() + Send>) -> io::Result<()> {
    crate::threads::spawn_named(REAPER_THREAD_NAME, work)
}

/// [`reap_with_timeout`] with an injectable thread spawner, so a test can drive
/// the thread-creation-failure branch without making the OS refuse a thread.
///
/// The child and the reporter are held in a shared slot until the reaper thread
/// takes them. If the spawn fails, the slot still owns both: the child is
/// killed and waited on so the failed reaper cannot leave the opener as a
/// zombie, and then the reporter gets the failure line. The sign-in flow itself
/// continues, because the opener has already been started and may still have
/// delivered the page.
pub(super) fn reap_with_timeout_and_spawn(
    child: std::process::Child,
    timeout: Duration,
    report: impl FnOnce(String) + Send + 'static,
    spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> io::Result<()>,
) {
    let pending = std::sync::Arc::new(std::sync::Mutex::new(Some((child, report))));
    let pending_for_thread = std::sync::Arc::clone(&pending);
    let work = Box::new(move || {
        let Some((mut child, report)) = pending_for_thread
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        else {
            return;
        };
        reap_child(&mut child, timeout, report);
    });
    if let Err(error) = spawn(work) {
        let Some((mut child, report)) = pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        else {
            return;
        };
        kill_and_reap(
            &mut child,
            |child| child.kill(),
            &format!("could not start the browser opener reaper thread: {error}"),
            report,
        );
    }
}

/// Kills `child` and reaps it, reporting the outcome in one line prefixed by
/// `context`.
///
/// A successful kill is followed by a blocking `wait`: the signal was
/// delivered, so the wait reaps the child and cannot be mistaken for the
/// original hang. If `kill` itself fails, the child may still be running and
/// an unbounded `wait` would block this reaper forever without ever reporting
/// why; the failure is therefore reported *before* the wait, then the wait
/// still runs so a child that does exit is reaped rather than left a zombie.
/// `kill` is injected so a test can make it fail against a real child.
pub(super) fn kill_and_reap(
    child: &mut std::process::Child,
    kill: impl FnOnce(&mut std::process::Child) -> io::Result<()>,
    context: &str,
    report: impl FnOnce(String),
) {
    match kill(child) {
        Ok(()) => match child.wait() {
            Ok(_) => report(format!("{context}; killed and reaped it")),
            Err(error) => report(format!("{context}; killed it but reaping failed: {error}")),
        },
        Err(error) => {
            report(format!("{context}; killing it failed: {error}"));
            let _ = child.wait();
        }
    }
}

/// The reaper thread's body: polls `child` until it exits or `timeout` elapses,
/// then reports a nonzero exit, a hang it killed and reaped, or a wait error.
pub(super) fn reap_child(
    child: &mut std::process::Child,
    timeout: Duration,
    report: impl FnOnce(String) + Send + 'static,
) {
    reap_child_with(child, timeout, report, |child| child.try_wait());
}

/// [`reap_child`] with an injectable poll, so a test can make `try_wait` fail
/// and prove the failing poll still cleans up the child.
///
/// A poll error does not mean the child is gone: it can still be running. The
/// reaper's job is to leave no process behind, so the error path kills and
/// waits on the child too (reporting a cleanup failure if that wait also
/// fails) instead of returning on the error and leaking the opener for the
/// rest of the app's life.
pub(super) fn reap_child_with(
    child: &mut std::process::Child,
    timeout: Duration,
    report: impl FnOnce(String) + Send + 'static,
    mut try_wait: impl FnMut(&mut std::process::Child) -> io::Result<Option<std::process::ExitStatus>>,
) {
    let deadline = Instant::now() + timeout;
    loop {
        match try_wait(child) {
            Ok(Some(status)) if status.success() => return,
            Ok(Some(status)) => {
                report(opener_exit_report(&status));
                return;
            }
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(OPENER_REAP_POLL_INTERVAL);
            }
            Ok(None) => {
                kill_and_reap(
                    child,
                    |child| child.kill(),
                    &format!("the browser opener did not exit within {timeout:?}"),
                    report,
                );
                return;
            }
            Err(error) => {
                // A failed poll says nothing about the child's state: it may
                // still be running. Kill and reap it anyway so the error path
                // cannot leave the opener (and its pid) behind.
                kill_and_reap(
                    child,
                    |child| child.kill(),
                    &format!("could not reap the browser opener process: {error}"),
                    report,
                );
                return;
            }
        }
    }
}

/// The line logged when the browser opener exits without success. Names the
/// status so a missing or broken opener is diagnosable, and states the
/// consequence without overclaiming it: the browser may still have opened.
pub(super) fn opener_exit_report(status: &std::process::ExitStatus) -> String {
    format!("the browser opener exited with {status}; the sign-in page may not have opened")
}
