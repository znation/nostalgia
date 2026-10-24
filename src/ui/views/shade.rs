//! The compact transport strip shown while the window is rolled up in shade
//! mode: Previous, Play/Pause, Stop, Next, the elapsed-over-total readout, a
//! preview seek slider, and a volume slider.
//!
//! The strip draws the shared
//! [`super::mini_transport::mini_transport_strip_with_seek`] on its blue
//! title-bar chrome, so it is a twin of the library footer and the full
//! transport row's controls, not a second playback path: every button emits
//! the same [`Message`] its full-size twin does, and the volume slider writes
//! [`Message::VolumeChange`]. The seek slider emits the Now Playing bar's own
//! [`Message::SeekPreviewChanged`]/[`Message::SeekPreviewReleased`] pair while
//! the backend reports a position and a preview length. It reads the same
//! playback state and time string the Now Playing bar does, and while playing
//! or paused its readout publishes [`Message::ToggleTimeDisplay`], so the
//! elapsed/remaining toggle carries into shade mode even though the LCD is
//! hidden.

use std::borrow::Cow;
use std::time::Duration;

use iced::{Element, widget::MouseArea};

use crate::state::TransportMode;
use crate::ui::{Message, bevel, style};

use super::{
    mini_transport::{MINI_TRANSPORT_HEIGHT, mini_transport_strip_with_seek},
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
/// A double-click on the strip's blank chrome publishes
/// [`Message::ToggleWindowShade`], restoring the window exactly as the title
/// bar's double-click and the shade button do, so the rolled-up window
/// restores from a double-click on either its title bar or its blank strip
/// chrome. `MouseArea` forwards events to its content
/// first and stops when that content captures them, so the strip's buttons,
/// time readout, seek slider, and volume slider keep their own presses,
/// drags, toggles, and wheel behavior, and a double-click on them does not
/// unshade. The stopped readout is inert but still swallows the double-click
/// through its own no-op handler, so it too cannot restore by accident. The
/// chrome and [`SHADE_STRIP_HEIGHT`] geometry are unchanged.
///
/// `mode` chooses the Play/Pause glyph and, while playing or paused, makes the
/// time readout publish [`Message::ToggleTimeDisplay`], so the shade strip is
/// the only time toggle available while the LCD is hidden; a stopped readout
/// stays inert. `volume` is the slider's value, and dragging it emits
/// [`Message::VolumeChange`]. `elapsed` and `total` are the same strings
/// [`WinampPlayer::now_playing_elapsed`] and [`WinampPlayer::now_playing_time`]
/// give the Now Playing bar, so its elapsed/remaining choice and the track's
/// total are shown here too. When `position` and a non-zero `duration` are both
/// known, a seek slider over `duration` sits between the readout and the volume
/// slider; a held drag previews its target through the readout, which
/// `WinampPlayer::readout_position` already folds the drag into. Otherwise the
/// seek slider blanks and the row's fill spacer pushes the volume slider to
/// the right edge, keeping the buttons and time readout left-aligned.
///
/// [`WinampPlayer::now_playing_elapsed`]: crate::ui::WinampPlayer::now_playing_elapsed
/// [`WinampPlayer::now_playing_time`]: crate::ui::WinampPlayer::now_playing_time
/// [`WinampPlayer::readout_position`]: crate::ui::WinampPlayer::readout_position
pub fn view_shade_strip<'a>(
    mode: TransportMode,
    volume: f32,
    elapsed: Cow<'a, str>,
    total: Cow<'a, str>,
    position: Option<Duration>,
    duration: Option<Duration>,
) -> Element<'a, Message> {
    MouseArea::new(bevel::raised_panel(
        mini_transport_strip_with_seek(
            mode,
            volume,
            elapsed,
            total,
            position,
            duration,
            SHADE_STRIP_HEIGHT,
        )
        .style(|_theme| style::title_bar_style()),
    ))
    .on_double_click(Message::ToggleWindowShade)
    .into()
}
