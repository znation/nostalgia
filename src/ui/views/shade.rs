//! The compact transport strip shown while the window is rolled up in shade
//! mode: Previous, Play/Pause, Stop, Next, the current time readout, and a
//! volume slider.
//!
//! The strip draws the shared [`super::mini_transport::mini_transport_row`] on its
//! blue title-bar chrome, so it is a twin of the library footer and the full
//! transport row's controls, not a second playback path: every button emits
//! the same [`Message`] its full-size twin does, and the slider writes
//! [`Message::VolumeChange`]. It reads the same playback state and time string
//! the Now Playing bar does, so the elapsed/remaining toggle carries into
//! shade mode.

use std::borrow::Cow;

use iced::{Element, Length, alignment::Vertical, widget::Container};

use crate::ui::{Message, bevel, style};

use super::{
    mini_transport::{MINI_TRANSPORT_HEIGHT, mini_transport_row},
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
/// `is_playing` chooses the Play/Pause glyph; `volume` is the slider's value,
/// and dragging it emits [`Message::VolumeChange`]. `elapsed` is the same
/// string [`WinampPlayer::now_playing_elapsed`] gives the Now Playing bar, so
/// its elapsed/remaining choice is shown here too. The shared row's fill
/// spacer pushes the slider to the right edge, keeping the buttons and time
/// readout left-aligned.
///
/// [`WinampPlayer::now_playing_elapsed`]: crate::ui::WinampPlayer::now_playing_elapsed
pub fn view_shade_strip<'a>(
    is_playing: bool,
    volume: f32,
    elapsed: Cow<'a, str>,
) -> Element<'a, Message> {
    bevel::raised_panel(
        Container::new(mini_transport_row(is_playing, volume, elapsed))
            .width(Length::Fill)
            .height(Length::Fixed(SHADE_STRIP_HEIGHT))
            .align_y(Vertical::Center)
            .padding([0, 6])
            .style(|_theme| style::title_bar_style()),
    )
}
