use super::*;

/// Waits up to five seconds for `pid` to leave `/proc`, which only happens
/// once the child has been reaped. Panics naming `context` when it remains.
#[cfg(target_os = "linux")]
fn assert_child_is_reaped(pid: u32, context: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::path::Path::new(&format!("/proc/{pid}")).exists() {
        assert!(
            Instant::now() < deadline,
            "child {pid} was still present five seconds after {context}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

// The browser opener is a short-lived launcher. Dropping its `Child` without
// waiting leaves a zombie for the rest of the app's life, one per sign-in;
// `reap_in_background` waits on a detached thread so the child is reaped
// without blocking the caller. A real child plus `/proc` is the only way to
// observe reaping: before the wait was added, the pid stayed a zombie and this
// loop timed out.
#[cfg(target_os = "linux")]
#[test]
fn reap_in_background_reaps_the_child() {
    let child = Command::new("sh")
        .args(["-c", "exit 0"])
        .spawn()
        .expect("spawn a short-lived child");
    let pid = child.id();
    reap_in_background(child);
    assert_child_is_reaped(pid, "reap_in_background");
}

/// Spawns a shell that exits with `exit_code`, reaps it through `reap_with`,
/// and returns the receiver of the one report it emits (or none, for a zero
/// exit). The two tests below share this scaffolding and differ only in the
/// status they inject and the report they expect.
#[cfg(unix)]
fn reap_exit_code(exit_code: i32) -> std::sync::mpsc::Receiver<String> {
    let child = Command::new("sh")
        .arg("-c")
        .arg(format!("exit {exit_code}"))
        .spawn()
        .expect("spawn a short-lived shell");
    let (sender, receiver) = std::sync::mpsc::channel();
    reap_with(child, move |report| {
        let _ = sender.send(report);
    });
    receiver
}

// The opener reaps on a detached thread, so its exit status is the only signal
// the flow has about whether the browser actually opened. A nonzero exit (a
// broken or missing browser) used to be discarded: the sign-in then sat for
// the full `AUTH_TIMEOUT` and reported only a generic timeout, hiding the
// cause. `reap_with`'s injectable reporter makes the status observable, and a
// real nonzero-exiting process is the fault injected.
#[cfg(unix)]
#[test]
fn a_nonzero_opener_exit_is_reported() {
    let report = reap_exit_code(3)
        .recv_timeout(Duration::from_secs(10))
        .expect("a nonzero opener exit must be reported");
    assert!(
        report.contains("opener"),
        "the report must name the opener, got: {report}"
    );
    assert!(
        report.contains('3'),
        "the report must name the exit status, got: {report}"
    );
}

// The twin of the test above: a successful opener exit must stay quiet, so the
// new report cannot fire on every sign-in and turn a working browser into a
// logged failure. The child exits zero and the reporter must never receive a
// line.
#[cfg(unix)]
#[test]
fn a_successful_opener_exit_is_not_reported() {
    assert!(
        reap_exit_code(0)
            .recv_timeout(Duration::from_millis(500))
            .is_err(),
        "a successful opener exit must not be reported"
    );
}

// An opener that never exits is the fault a bare `Child::wait` cannot bound:
// the reaper thread blocks forever and the child process outlives the sign-in.
// The injected fault is a real long-running child plus a short deadline; the
// report must name the timeout and the pid must be reaped (a killed child still
// needs a `wait`, and `/proc` only disappears once it has one).
#[cfg(target_os = "linux")]
#[test]
fn a_hung_opener_is_killed_and_reported() {
    let child = Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("spawn a long-running child");
    let pid = child.id();
    let (sender, receiver) = std::sync::mpsc::channel();
    reap_with_timeout(child, Duration::from_millis(200), move |report| {
        let _ = sender.send(report);
    });
    let report = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("a hung opener must be reported");
    assert!(
        report.contains("did not exit"),
        "the report must name the hang, got: {report}"
    );
    assert_child_is_reaped(pid, "the reap timeout");
}

// If the reaper thread cannot be started, the child it was meant to wait on
// would otherwise be dropped without a wait and stay a zombie, and the old
// plain `thread::spawn` would panic the sign-in thread instead. The injected
// fault is a failing spawner; the failure must be reported and the real child
// must be killed and reaped synchronously.
#[cfg(target_os = "linux")]
#[test]
fn a_reaper_thread_spawn_failure_kills_the_opener_and_is_reported() {
    let child = Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("spawn a long-running child");
    let pid = child.id();
    let (sender, receiver) = std::sync::mpsc::channel();
    reap_with_timeout_and_spawn(
        child,
        Duration::from_secs(30),
        move |report| {
            let _ = sender.send(report);
        },
        |_work| Err(io::Error::other("the OS refused a thread")),
    );
    let report = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("a failed reaper spawn must be reported");
    assert!(
        report.contains("reaper thread"),
        "the report must name the reaper thread, got: {report}"
    );
    assert!(
        report.contains("the OS refused a thread"),
        "the report must carry the spawn cause, got: {report}"
    );
    assert_child_is_reaped(pid, "the reaper spawn failed");
}

// A failed `try_wait` does not prove the child exited: the reaper used to log
// the wait error and return, leaving a still-running opener (and its pid)
// behind for the rest of the app's life. The injected fault is a poll that
// always errors against a real long-running child; the fix must kill and reap
// it before reporting, and the report must still carry the poll's cause.
#[cfg(target_os = "linux")]
#[test]
fn a_failed_reap_poll_kills_the_opener_and_reports_the_cause() {
    let mut child = Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("spawn a long-running child");
    let pid = child.id();
    let (sender, receiver) = std::sync::mpsc::channel();
    reap_child_with(
        &mut child,
        Duration::from_secs(30),
        move |report| {
            let _ = sender.send(report);
        },
        |_child| Err(io::Error::other("the wait failed")),
    );
    let report = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("a failed reap poll must be reported");
    assert!(
        report.contains("could not reap"),
        "the report must name the failed reap, got: {report}"
    );
    assert!(
        report.contains("the wait failed"),
        "the report must carry the poll's cause, got: {report}"
    );
    assert!(
        report.contains("killed and reaped it"),
        "the failed poll must still kill and reap the child, got: {report}"
    );
    assert_child_is_reaped(pid, "the failed reap poll");
}

// A kill that fails must not hide the reason behind an unbounded wait: the
// reaper used to discard the kill error and then call `wait`, so a child that
// could not be signaled kept the thread — and the report — blocked until the
// child happened to exit. The injected fault is a refused kill against a real
// child that outlives the assertion; the report must arrive while that child
// is still running, then the follow-up wait reaps it once it exits on its own.
#[cfg(target_os = "linux")]
#[test]
fn a_failed_opener_kill_is_reported_before_waiting_for_the_child() {
    let mut child = Command::new("sh")
        .args(["-c", "sleep 2"])
        .spawn()
        .expect("spawn a child that outlives the report");
    let (sender, receiver) = std::sync::mpsc::channel();
    let reaper = thread::spawn(move || {
        kill_and_reap(
            &mut child,
            |_child| Err(io::Error::other("the kill was refused")),
            "the browser opener did not exit within 50ms",
            move |report| {
                let _ = sender.send(report);
            },
        );
    });
    let report = receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("a failed kill must be reported without waiting for the child");
    assert!(
        report.contains("did not exit"),
        "the report must name the timeout, got: {report}"
    );
    assert!(
        report.contains("the kill was refused"),
        "the report must carry the kill failure, got: {report}"
    );
    reaper.join().expect("the reaper thread finished");
}

// The reaper runs on a named thread, so a panic in it is attributed to
// `nostalgia-opener-reaper` rather than `<unnamed>`. Pin the name on the
// production spawner, since the failure test above injects its own spawner
// and would not notice a rename. The name is spelled out here rather than
// read back from `REAPER_THREAD_NAME`, so a rename of the constant fails
// this test instead of renaming the thread and its assertion together.
#[test]
fn spawn_reaper_names_its_work_thread() {
    let (sender, receiver) = std::sync::mpsc::channel();
    spawn_reaper(Box::new(move || {
        let _ = sender.send(std::thread::current().name().map(str::to_string));
    }))
    .expect("start the reaper thread");

    assert_eq!(
        receiver
            .recv()
            .expect("the reaper reports its name")
            .as_deref(),
        Some("nostalgia-opener-reaper")
    );
}

// `AUTH_TIMEOUT` is production behavior — how long `authorize` waits for
// the browser callback — but `authorize` is its only reader and the
// timeout tests above inject their own deadline, so a changed constant
// would clear the suite. Pin the documented five minutes.
#[test]
fn auth_timeout_is_the_documented_five_minutes() {
    assert_eq!(AUTH_TIMEOUT, Duration::from_secs(300));
}

// A missing opener is the most likely spawn failure on a minimal Linux
// box, and a bare "No such file or directory" does not say which command
// is missing. `spawn_opener` names the program in the error, so the
// `authorize` message ("could not open the sign-in page: ...") says what
// to install. The probe program cannot exist, so this fails without
// starting a process on every platform.
#[test]
fn spawn_opener_names_the_missing_program_in_its_error() {
    let error = spawn_opener("nostalgia-no-such-opener", &[]).unwrap_err();
    assert!(
        error.to_string().contains("nostalgia-no-such-opener"),
        "the error should name the program, got {error:?}"
    );
}
