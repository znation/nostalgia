//! The playback-position poll: the one-second tick that makes the UI query
//! the loaded preview's position, format, length, and finished state.
//!
//! Like the other timers, it feeds iced from the shared std-thread tick source
//! in [`super::ticks`], because iced's timed subscription is gated behind
//! backend features the app does not enable.

use std::{io, time::Duration};

use super::ticks;
use crate::ui::Message;

/// How often the Now Playing bar polls the audio backend for the loaded
/// preview's position, format, length, and finished state, refreshing the
/// large elapsed readout, the LCD status row, and the seek bar, and advancing
/// when the preview reaches its end. The audio backend owns playback, so this
/// bounds only how often the UI asks; it does not affect the audio itself.
pub(in crate::ui) const POSITION_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// A subscription that emits [`Message::PositionTick`] once per [`POSITION_POLL_INTERVAL`], using
/// the same std-thread tick source as [`super::marquee::marquee_ticks`] because iced's timed
/// subscription is not enabled. Each tick's `update` arm schedules the actual backend queries —
/// position, format, length, and finished state — as tasks, so the UI thread never blocks on the
/// audio backend.
pub(in crate::ui) fn position_ticks() -> iced::Subscription<Message> {
    iced::Subscription::run(position_tick_stream)
}

/// The tick stream behind [`position_ticks`], split out so a test can poll it.
/// Like [`super::marquee::marquee_tick_stream`], the one-slot channel drops a tick when the UI
/// has not polled the previous one yet, and only a disconnected receiver ends
/// the thread.
pub(in crate::ui) fn position_tick_stream() -> impl iced::futures::Stream<Item = Message> {
    position_tick_stream_with(spawn_position_thread)
}

/// [`position_tick_stream`] with an injectable thread spawner, so a test can
/// make the spawn fail and prove the stream reports it instead of panicking.
///
/// The timer thread starts through the fallible [`spawn_position_thread`]:
/// `std::thread::spawn` panics when the OS refuses a thread, and iced builds
/// this stream on its main thread while it updates the subscriptions, so such
/// a panic would unwind the event loop and take the whole player down. Unlike
/// the marquee timer, this subscription is part of the set whenever a position
/// can refresh, so that panic could hit any playback start. A refused spawn is reported
/// instead, and the tick stream then ends — the sender goes down with the
/// thread that never started — so the elapsed readout stops updating and the
/// app keeps running.
pub(in crate::ui) fn position_tick_stream_with(
    spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> io::Result<()>,
) -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::StreamExt;

    ticks::tick_stream(
        POSITION_POLL_INTERVAL,
        spawn,
        position_thread_failure_report,
    )
    .map(|()| Message::PositionTick)
}

/// The production spawner for the position poll thread: the named [`POSITION_THREAD_NAME`] thread,
/// started through [`crate::threads::spawn_named`]. That shared spawner uses the fallible
/// `std::thread::Builder::spawn` form, as do the audio worker, the REST worker, the sign-in thread,
/// the timeout timer, the marquee timer, and the spectrum poll timer.
pub(in crate::ui) fn spawn_position_thread(work: Box<dyn FnOnce() + Send>) -> io::Result<()> {
    crate::threads::spawn_named(POSITION_THREAD_NAME, work)
}

/// The name [`spawn_position_thread`] gives the position poll thread, so a
/// panic in it is attributed to `nostalgia-position-poll` rather than
/// `<unnamed>`, like the audio worker's `nostalgia-audio`, the REST worker's
/// `nostalgia-rest`, and the marquee's `nostalgia-marquee`.
const POSITION_THREAD_NAME: &str = "nostalgia-position-poll";

/// The report for a position poll thread that could not start: it names the
/// cause and what the user loses — the live elapsed time — so a readout that
/// never advances is diagnosable instead of mysterious. Pure, so the report
/// contract is testable without capturing stderr.
pub(in crate::ui) fn position_thread_failure_report(error: &io::Error) -> String {
    format!(
        "the position poll thread could not start: {error}; the Now Playing elapsed time will not update"
    )
}
