//! The Now Playing marquee timer: the tick that advances the title window
//! while the line is too long to fit the LCD.
//!
//! Like the other timers, it feeds iced from the shared std-thread tick source
//! in [`crate::ui::ticks`], because iced's timed subscription is gated behind
//! backend features the app does not enable.

use std::{io, time::Duration};

use crate::ui::{Message, ticks};

/// How often the Now Playing title window advances one character while an
/// overlong line scrolls. The timer runs only while the line is too long to
/// fit the LCD's character budget (see [`super::subscriptions`]).
pub(in crate::ui) const MARQUEE_TICK: Duration = Duration::from_millis(250);

/// A subscription that emits [`Message::MarqueeTick`] once per [`MARQUEE_TICK`]. iced's timed
/// subscription is gated behind `smol`/`tokio` backend features the app does not enable (its
/// default thread-pool backend has no `time::every`), so this feeds the ticks from one std thread
/// through the futures channel iced re-exports. Dropping the subscription drops the receiver, and
/// the thread exits on its next send.
pub(super) fn marquee_ticks() -> iced::Subscription<Message> {
    iced::Subscription::run(marquee_tick_stream)
}

/// The tick stream behind [`marquee_ticks`], split out so a test can poll it.
/// The one-slot channel deliberately drops a tick when the UI has not polled
/// the previous one yet; that keeps memory bounded without stopping the timer.
/// Only a disconnected receiver — the subscription ended — ends the thread.
pub(in crate::ui) fn marquee_tick_stream() -> impl iced::futures::Stream<Item = Message> {
    marquee_tick_stream_with(spawn_marquee_thread)
}

/// [`marquee_tick_stream`] with an injectable thread spawner, so a test can
/// make the spawn fail and prove the stream reports it instead of panicking.
///
/// The timer thread starts through the fallible [`spawn_marquee_thread`]:
/// `std::thread::spawn` panics when the OS refuses a thread, and iced builds
/// this stream on its main thread while it updates the subscriptions, so such
/// a panic would unwind the event loop and take the whole player down over a
/// cosmetic timer. A refused spawn is reported instead, and the tick stream
/// then ends — the sender goes down with the thread that never started — so
/// the Now Playing title simply stops scrolling and the app keeps running.
pub(in crate::ui) fn marquee_tick_stream_with(
    spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> io::Result<()>,
) -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::StreamExt;

    ticks::tick_stream(MARQUEE_TICK, spawn, marquee_thread_failure_report)
        .map(|()| Message::MarqueeTick)
}

/// The production spawner for the marquee timer thread: the named [`MARQUEE_THREAD_NAME`] thread,
/// started through [`crate::threads::spawn_named`]. That shared spawner uses the fallible
/// `std::thread::Builder::spawn` form, as do the audio worker, the REST worker, the sign-in thread,
/// and the timeout timer.
pub(in crate::ui) fn spawn_marquee_thread(work: Box<dyn FnOnce() + Send>) -> io::Result<()> {
    crate::threads::spawn_named(MARQUEE_THREAD_NAME, work)
}

/// The name the marquee timer thread gets, so a panic in it is attributed to
/// `nostalgia-marquee` rather than `<unnamed>`, like the audio worker's
/// `nostalgia-audio` and the REST worker's `nostalgia-rest`.
const MARQUEE_THREAD_NAME: &str = "nostalgia-marquee";

/// The report for a marquee timer thread that could not start: it names the
/// cause and what the user loses — the scrolling title — so a Now Playing line
/// that never moves is diagnosable instead of mysterious. Pure, so the report
/// contract is testable without capturing stderr.
pub(in crate::ui) fn marquee_thread_failure_report(error: &io::Error) -> String {
    format!(
        "the marquee timer thread could not start: {error}; the Now Playing title will not scroll"
    )
}
