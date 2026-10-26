//! The crate-wide named-thread spawner.
//!
//! Every background worker the player starts — the audio worker, the sign-in
//! and REST threads, the marquee, position, and spectrum timers, the request
//! timeout timer in [`crate::timeout`], the browser-opener reaper, and the
//! settings writer — goes through [`spawn_named`] or [`spawn_named_joined`],
//! so each gets a name for panic attribution and each takes the fallible
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
    spawn_named_joined(name, work).map(|_handle| ())
}

/// [`spawn_named`] for a worker whose caller must wait for it to finish, such
/// as the settings writer, which flushes its last snapshot when it is joined.
/// The returned handle detaches the thread if it is dropped without a
/// [`std::thread::JoinHandle::join`].
///
/// # Errors
///
/// Returns the [`std::io::Error`] from [`std::thread::Builder::spawn`] when the
/// OS cannot start the thread.
pub fn spawn_named_joined(
    name: &str,
    work: impl FnOnce() + Send + 'static,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name(name.to_string())
        .spawn(work)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::{spawn_named, spawn_named_joined};

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

    #[test]
    fn spawn_named_joined_returns_a_joinable_handle() {
        let handle = spawn_named_joined("nostalgia-joined-thread-test", || {})
            .expect("the test thread should start");

        handle.join().expect("the test thread should finish");
    }
}
