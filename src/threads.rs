//! The crate-wide named-thread spawner.
//!
//! Every background worker the player starts — the audio worker, the sign-in
//! and REST threads, the marquee, position, and spectrum timers, the request
//! timeout timer, and the browser-opener reaper — goes through [`spawn_named`], so
//! each gets a name for panic attribution and each takes the fallible
//! [`std::thread::Builder`] form rather than [`std::thread::spawn`], which
//! panics when the OS refuses a thread.

/// Spawns `work` on a new detached thread named `name`.
///
/// Uses [`std::thread::Builder`]'s fallible `spawn`, so a refused thread is
/// returned as an [`std::io::Error`] instead of panicking the caller. The
/// handle is dropped because every caller starts a detached worker; `name` is
/// the `nostalgia-*` name a panic report attributes the thread to.
///
/// # Errors
///
/// Returns the [`std::io::Error`] from [`std::thread::Builder::spawn`] when the
/// OS cannot start the thread.
pub fn spawn_named(name: &str, work: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name(name.to_string())
        .spawn(work)
        .map(|_handle| ())
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::spawn_named;

    #[test]
    fn spawn_named_runs_the_work_on_a_thread_with_that_name() {
        let (sender, receiver) = mpsc::channel();
        spawn_named("nostalgia-named-thread-test", move || {
            let _ = sender.send(std::thread::current().name().map(str::to_string));
        })
        .expect("the test thread should start");

        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
            Some("nostalgia-named-thread-test".to_string())
        );
    }
}
