//! The spectrum poll: the quarter-second tick that makes the UI query the
//! loaded preview's live ten-band spectrum.
//!
//! Like the other timers, it feeds iced from the shared std-thread tick source
//! in [`crate::ui::ticks`], because iced's timed subscription is gated behind
//! backend features the app does not enable.

use std::{io, time::Duration};

use crate::ui::{Message, ticks};

/// How often the Now Playing LCD polls the audio backend for the loaded
/// preview's live spectrum, refreshing the visualizer's bars. The visualizer
/// updates about four times a second while playback runs; the backend owns the
/// signal, so this bounds only how often the UI asks.
pub(in crate::ui) const SPECTRUM_TICK: Duration = Duration::from_millis(250);

/// A subscription that emits [`Message::SpectrumTick`] once per
/// [`SPECTRUM_TICK`], using the same std-thread tick source as the other
/// timers because iced's timed subscription is not enabled. Each tick's
/// `update` arm schedules the actual backend query as a task, so the UI thread
/// never blocks on the audio backend.
pub(in crate::ui) fn spectrum_ticks() -> iced::Subscription<Message> {
    iced::Subscription::run(spectrum_tick_stream)
}

/// The tick stream behind [`spectrum_ticks`], split out so a test can poll it.
/// Like the other tick streams, the one-slot channel drops a tick when the UI
/// has not polled the previous one yet, and only a disconnected receiver ends
/// the thread.
pub(in crate::ui) fn spectrum_tick_stream() -> impl iced::futures::Stream<Item = Message> {
    spectrum_tick_stream_with(spawn_spectrum_thread)
}

/// [`spectrum_tick_stream`] with an injectable thread spawner, so a test can
/// make the spawn fail and prove the stream reports it instead of panicking.
/// A refused spawn ends the stream — the sender goes down with the thread that
/// never started — so the visualizer simply stops updating and the app keeps
/// running.
pub(in crate::ui) fn spectrum_tick_stream_with(
    spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> io::Result<()>,
) -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::StreamExt;

    ticks::tick_stream(SPECTRUM_TICK, spawn, spectrum_thread_failure_report)
        .map(|()| Message::SpectrumTick)
}

/// The production spawner for the spectrum poll thread: the named
/// [`SPECTRUM_THREAD_NAME`] thread, started through
/// [`crate::threads::spawn_named`].
pub(in crate::ui) fn spawn_spectrum_thread(work: Box<dyn FnOnce() + Send>) -> io::Result<()> {
    crate::threads::spawn_named(SPECTRUM_THREAD_NAME, work)
}

/// The name [`spawn_spectrum_thread`] gives the spectrum poll thread, so a
/// panic in it is attributed to `nostalgia-spectrum-poll` rather than
/// `<unnamed>`.
const SPECTRUM_THREAD_NAME: &str = "nostalgia-spectrum-poll";

/// The report for a spectrum poll thread that could not start: it names the
/// cause and what the user loses — the live spectrum — so a visualizer that
/// never moves is diagnosable instead of mysterious. Pure, so the report
/// contract is testable without capturing stderr.
pub(in crate::ui) fn spectrum_thread_failure_report(error: &io::Error) -> String {
    format!(
        "the spectrum timer thread could not start: {error}; the Now Playing visualizer will not update"
    )
}
