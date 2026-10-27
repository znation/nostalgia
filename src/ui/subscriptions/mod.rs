//! The iced subscriptions the UI runs, and the event filters behind them.
//!
//! [`subscriptions`] builds the active set from the player's state: the always-on keyboard and
//! mixer listeners, the Escape listener while a transient panel is open, and the marquee,
//! playback-position, and spectrum timers while each has something to refresh. Each timer lives in
//! its own module — [`marquee`], [`position`], and [`spectrum`] — fed from the shared std-thread
//! tick source in [`super::ticks`], because iced's timed subscription is gated behind backend
//! features the app does not enable; each only schedules the actual backend query from its `update`
//! arm, so the UI thread never waits on the audio backend. The event filters that turn iced events
//! into messages live in [`listeners`].

mod listeners;
mod marquee;
mod position;
mod spectrum;

use iced::keyboard;

use super::{
    Message,
    shortcuts::{message_for, message_for_jump_file},
};

use marquee::marquee_ticks;
use position::position_ticks;
use spectrum::spectrum_ticks;

pub(super) use listeners::{escape_key_message, mixer_key_message, mixer_wheel_message};

// The timer internals are exercised by `ui::tests`, so only a test build needs
// them reachable through this module; `subscriptions` calls the tick
// subscriptions through the imports above.
#[cfg(test)]
pub(super) use marquee::{
    MARQUEE_TICK, marquee_thread_failure_report, marquee_tick_stream, marquee_tick_stream_with,
    spawn_marquee_thread,
};
#[cfg(test)]
pub(super) use position::{
    position_thread_failure_report, position_tick_stream, position_tick_stream_with,
    spawn_position_thread,
};
#[cfg(test)]
pub(super) use spectrum::{
    spawn_spectrum_thread, spectrum_thread_failure_report, spectrum_tick_stream,
    spectrum_tick_stream_with,
};

/// The subscriptions the UI runs: the keyboard listener, plus the marquee
/// timer while the Now Playing line is too long to fit the LCD's character
/// budget, plus the position poll while there is a position to refresh, plus
/// the spectrum poll while playback is running.
///
/// The marquee timer exists to advance the Now Playing title window, and the
/// view ignores that window while the line fits — so ticking for a fitting line
/// would advance nothing. Every message makes iced rebuild the whole visible
/// screen (the library panel included while it shows), so an always-on timer
/// would rebuild the UI four times a second to redraw a bar that cannot change.
/// The timer is therefore subscribed only while the line needs to scroll, and
/// dropped again once it fits.
///
/// The position poll has the same shape: each tick and its replies rebuild the same whole screen,
/// and a stopped player's view clears its current track's time readout (except the `--:--`
/// placeholder), so there is nothing to refresh. It is subscribed while playback is running or the
/// backend still holds a preview (a paused player's position a seek can still move, even before the
/// first position reply); a stopped player leaves it out.
///
/// The spectrum poll runs only while playback is running: a paused or stopped
/// player keeps the last bars the view stored, so ticking would rebuild the
/// whole screen for an unchanged strip.
///
/// Up and Down choose a Jump to File match only while that panel is open;
/// otherwise they keep their normal volume bindings, so the keyboard mapper
/// switches with `jump_file_open`.
///
/// The Escape listener joins only while a transient panel is open, so Escape
/// outside a panel rebuilds no screen. It uses the event-with-status form
/// because the panel boxes hold focus and the focused `TextInput` captures
/// Escape, which `keyboard::listen`'s ignored-only feed would drop.
pub(super) fn subscriptions(
    marquee_needed: bool,
    position_needed: bool,
    spectrum_needed: bool,
    mixer_readout_showing: bool,
    jump_file_open: bool,
    transient_panel_open: bool,
) -> Vec<iced::Subscription<Message>> {
    let keyboard_listener = if jump_file_open {
        keyboard::listen().filter_map(message_for_jump_file)
    } else {
        keyboard::listen().filter_map(message_for)
    };
    let mut subscriptions = vec![
        keyboard_listener,
        iced::event::listen_with(mixer_key_message),
    ];
    if mixer_readout_showing {
        subscriptions.push(iced::event::listen_with(mixer_wheel_message));
    }
    if marquee_needed {
        subscriptions.push(marquee_ticks());
    }
    if position_needed {
        subscriptions.push(position_ticks());
    }
    if spectrum_needed {
        subscriptions.push(spectrum_ticks());
    }
    if transient_panel_open {
        subscriptions.push(iced::event::listen_with(escape_key_message));
    }
    subscriptions
}
