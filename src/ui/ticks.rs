//! The std-thread tick source shared by the UI's two timers.
//!
//! iced's timed subscription is gated behind backend features the app does not
//! enable (its default thread-pool backend has no `time::every`), so the Now
//! Playing marquee and the position poll each feed a one-slot futures channel
//! from a sleeping std thread. The stream, its drop-when-full send, and the
//! fallible-spawn reporting live here once.

use std::io;
use std::time::Duration;

use iced::futures::{Stream, channel::mpsc};

/// A stream of `()` once every `interval`, produced by one detached thread.
///
/// The one-slot channel deliberately drops a tick when the UI has not polled
/// the previous one yet; that keeps memory bounded without stopping the timer.
/// Only a disconnected receiver — the subscription ended — ends the thread.
///
/// The thread starts through `spawn`, a fallible spawner: `std::thread::spawn`
/// panics when the OS refuses a thread, and iced builds these streams on its
/// main thread while it updates the subscriptions, so such a panic would
/// unwind the event loop and take the whole player down over a timer. A
/// refused spawn is reported through `on_spawn_failure` instead, and the
/// stream then ends — the sender goes down with the thread that never started
/// — so the caller's readout freezes and the app keeps running.
pub(super) fn tick_stream(
    interval: Duration,
    spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> io::Result<()>,
    on_spawn_failure: impl FnOnce(&io::Error) -> String,
) -> impl Stream<Item = ()> {
    let (mut sender, receiver) = mpsc::channel(1);
    let work: Box<dyn FnOnce() + Send> = Box::new(move || {
        loop {
            std::thread::sleep(interval);
            if let Err(error) = sender.try_send(())
                && error.is_disconnected()
            {
                break;
            }
        }
    });
    if let Err(error) = spawn(work) {
        eprintln!("{}", on_spawn_failure(&error));
    }
    receiver
}
