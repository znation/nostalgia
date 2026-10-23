//! Timer subscriptions for the iced UI: the Now Playing marquee and the
//! playback-position poll.
//!
//! iced's timed subscription is gated behind backend features the app does not
//! enable, so each timer is fed by a named std thread through the shared tick
//! source in [`super::ticks`], which uses the futures channel iced re-exports.
//! The marquee runs only while the title needs to
//! scroll, and the position poll only while there is a position to refresh
//! (see [`subscriptions`]); each only schedules the actual backend query from
//! its `update` arm, so the UI thread never waits on the audio backend.

use iced::keyboard;
use std::{io, time::Duration};

use super::{Message, shortcuts::message_for, ticks};

/// How often the Now Playing title window advances one character while an
/// overlong line scrolls. The timer runs only while the line is too long to
/// fit the LCD's character budget (see [`subscriptions`]).
pub(super) const MARQUEE_TICK: Duration = Duration::from_millis(250);

/// How often the Now Playing bar polls the audio backend for the loaded
/// preview's position, format, and length, refreshing the large elapsed
/// readout, the LCD status row, and the seek bar. The audio backend owns
/// playback, so this bounds only how often the UI asks; it does not affect the
/// audio itself.
pub(super) const POSITION_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// A subscription that emits [`Message::MarqueeTick`] once per
/// [`MARQUEE_TICK`]. iced's timed subscription is gated behind `smol`/`tokio`
/// backend features the app does not enable (its default thread-pool backend
/// has no `time::every`), so this feeds the ticks from one std thread through
/// the futures channel iced re-exports. Dropping the subscription drops the
/// receiver, and the thread exits on its next send.
fn marquee_ticks() -> iced::Subscription<Message> {
    iced::Subscription::run(marquee_tick_stream)
}

/// The tick stream behind [`marquee_ticks`], split out so a test can poll it.
/// The one-slot channel deliberately drops a tick when the UI has not polled
/// the previous one yet; that keeps memory bounded without stopping the timer.
/// Only a disconnected receiver — the subscription ended — ends the thread.
pub(super) fn marquee_tick_stream() -> impl iced::futures::Stream<Item = Message> {
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
pub(super) fn marquee_tick_stream_with(
    spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> io::Result<()>,
) -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::StreamExt;

    ticks::tick_stream(MARQUEE_TICK, spawn, marquee_thread_failure_report)
        .map(|()| Message::MarqueeTick)
}

/// The production spawner for the marquee timer thread: the named
/// [`MARQUEE_THREAD_NAME`] thread, started through
/// [`crate::threads::spawn_named`]. That shared spawner uses the fallible
/// `std::thread::Builder::spawn` form, as do the audio worker, the REST
/// worker, the sign-in thread, and the timeout timer.
pub(super) fn spawn_marquee_thread(work: Box<dyn FnOnce() + Send>) -> io::Result<()> {
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
pub(super) fn marquee_thread_failure_report(error: &io::Error) -> String {
    format!(
        "the marquee timer thread could not start: {error}; the Now Playing title will not scroll"
    )
}

/// The subscriptions the UI runs: the keyboard listener, plus the marquee
/// timer while the Now Playing line is too long to fit the LCD's character
/// budget, plus the position poll while there is a position to refresh.
///
/// The marquee timer exists to advance the Now Playing title window, and the
/// view ignores that window while the line fits — so ticking for a fitting line
/// would advance nothing. Every message makes iced rebuild the whole screen
/// (the browse list's rows included), so an always-on timer would rebuild the
/// UI four times a second to redraw a bar that cannot change. The timer is
/// therefore subscribed only while the line needs to scroll, and dropped again
/// once it fits.
///
/// The position poll has the same shape: each tick and its replies rebuild the
/// same whole screen, and a stopped player's readout is already the `00:00`
/// Stop cleared it to, so there is nothing to refresh. It is subscribed while
/// playback is running or the backend still holds a preview (a paused player's
/// position a seek can still move, even before the first position reply); a
/// stopped player leaves it out.
pub(super) fn subscriptions(
    marquee_needed: bool,
    position_needed: bool,
) -> Vec<iced::Subscription<Message>> {
    let mut subscriptions = vec![keyboard::listen().filter_map(message_for)];
    if marquee_needed {
        subscriptions.push(marquee_ticks());
    }
    if position_needed {
        subscriptions.push(position_ticks());
    }
    subscriptions
}

/// A subscription that emits [`Message::PositionTick`] once per
/// [`POSITION_POLL_INTERVAL`], using the same std-thread tick source as
/// [`marquee_ticks`] because iced's timed subscription is not enabled. Each
/// tick's `update` arm schedules the actual backend queries — position, format,
/// and length — as tasks, so the UI thread never blocks on the audio backend.
pub(super) fn position_ticks() -> iced::Subscription<Message> {
    iced::Subscription::run(position_tick_stream)
}

/// The tick stream behind [`position_ticks`], split out so a test can poll it.
/// Like [`marquee_tick_stream`], the one-slot channel drops a tick when the UI
/// has not polled the previous one yet, and only a disconnected receiver ends
/// the thread.
pub(super) fn position_tick_stream() -> impl iced::futures::Stream<Item = Message> {
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
pub(super) fn position_tick_stream_with(
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

/// The production spawner for the position poll thread: the named
/// [`POSITION_THREAD_NAME`] thread, started through
/// [`crate::threads::spawn_named`]. That shared spawner uses the fallible
/// `std::thread::Builder::spawn` form, as do the audio worker, the REST
/// worker, the sign-in thread, the timeout timer, and the marquee timer.
pub(super) fn spawn_position_thread(work: Box<dyn FnOnce() + Send>) -> io::Result<()> {
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
pub(super) fn position_thread_failure_report(error: &io::Error) -> String {
    format!(
        "the position poll thread could not start: {error}; the Now Playing elapsed time will not update"
    )
}
