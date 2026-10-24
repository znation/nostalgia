//! The compact transport strip shown while the window is rolled up in shade
//! mode: Previous, Play/Pause, Stop, Next, the elapsed-over-total readout, and
//! a volume slider.
//!
//! The strip draws the shared [`super::mini_transport::mini_transport_row`] on its
//! blue title-bar chrome, so it is a twin of the library footer and the full
//! transport row's controls, not a second playback path: every button emits
//! the same [`Message`] its full-size twin does, and the slider writes
//! [`Message::VolumeChange`]. It reads the same playback state and time string
//! the Now Playing bar does, and while playing or paused its readout publishes
//! [`Message::ToggleTimeDisplay`], so the elapsed/remaining toggle carries into
//! shade mode even though the LCD is hidden.

use std::borrow::Cow;

use iced::Element;

use crate::state::TransportMode;
use crate::ui::{Message, bevel, style};

use super::{
    mini_transport::{MINI_TRANSPORT_HEIGHT, mini_transport_strip},
    title_bar::TITLE_BAR_HEIGHT,
};

/// The compact strip's height in px, below the title bar in shade mode. It is
/// the shared compact row's height, so the shade strip and the library footer
/// keep the same geometry.
pub(crate) const SHADE_STRIP_HEIGHT: f32 = MINI_TRANSPORT_HEIGHT;

/// The whole rolled-up window's height: the title bar plus the strip. The
/// view and `handlers::window::resize_to_shade` both read it, so the window
/// they resize to cannot drift from the tree it shows.
pub(crate) const SHADED_HEIGHT: f32 = TITLE_BAR_HEIGHT + SHADE_STRIP_HEIGHT;

/// The shade-mode strip: the shared compact transport row on the title bar's
/// blue chrome.
///
/// `mode` chooses the Play/Pause glyph and, while playing or paused, makes the
/// time readout publish [`Message::ToggleTimeDisplay`], so the shade strip is
/// the only time toggle available while the LCD is hidden; a stopped readout
/// stays inert. `volume` is the slider's value, and dragging it emits
/// [`Message::VolumeChange`]. `elapsed` and `total` are the same strings
/// [`WinampPlayer::now_playing_elapsed`] and [`WinampPlayer::now_playing_time`]
/// give the Now Playing bar, so its elapsed/remaining choice and the track's
/// total are shown here too. The shared row's fill spacer pushes the slider to
/// the right edge, keeping the buttons and time readout left-aligned.
///
/// [`WinampPlayer::now_playing_elapsed`]: crate::ui::WinampPlayer::now_playing_elapsed
/// [`WinampPlayer::now_playing_time`]: crate::ui::WinampPlayer::now_playing_time
pub fn view_shade_strip<'a>(
    mode: TransportMode,
    volume: f32,
    elapsed: Cow<'a, str>,
    total: Cow<'a, str>,
) -> Element<'a, Message> {
    bevel::raised_panel(
        mini_transport_strip(mode, volume, elapsed, total, SHADE_STRIP_HEIGHT)
            .style(|_theme| style::title_bar_style()),
    )
}
