//! The shared compact transport row: Previous, Play/Pause, Stop, Next, the
//! elapsed-over-total time readout, and a volume slider.
//!
//! The window-shade strip and the library footer both build this one row, so
//! their glyphs, order, fixed widths, messages, and slider spec cannot drift
//! apart. It is presentation only: every control emits the same [`Message`] as
//! the full-size transport row, and the slider writes [`Message::VolumeChange`].

use std::borrow::Cow;

use iced::{
    Element, Length,
    alignment::Vertical,
    widget::{Container, Row, Space, Text},
};

use crate::ui::{Message, bevel, theme};

use super::{
    VOLUME_MAX, VOLUME_MIN, VOLUME_STEP, chrome_slider, fixed_width_button, spacer,
    transport::transport_glyphs,
};

/// The compact row's fixed height in px. The shade strip and the library
/// footer both occupy exactly this height.
pub(crate) const MINI_TRANSPORT_HEIGHT: f32 = 28.0;

/// The compact row's volume slider width in px: narrow enough to leave the
/// time readout and four buttons room.
pub(super) const MINI_VOLUME_WIDTH: f32 = 80.0;

/// The id of the compact row's volume-slider container. Only one strip is in
/// the tree at a time (the library footer in normal mode, the shade strip in
/// shade mode), so the shared id never collides. An explicit [`Id`] lets a
/// test find the slider's laid-out bounds and pin its right-edge placement.
///
/// [`Id`]: iced::advanced::widget::Id
pub(crate) const MINI_VOLUME_ID: &str = "mini-transport-volume";

/// The compact row's four chrome buttons, in row order: Previous, Play/Pause,
/// Stop, and Next. It drops the tooltip words from [`transport_glyphs`] so
/// both compact strips and the full transport row share one glyph, order,
/// width, and message spec.
///
/// Returned as plain `(label, width, message)` tuples rather than widgets, so
/// the labels, messages, and widths are testable without an iced renderer.
pub(super) fn mini_transport_controls(is_playing: bool) -> [(&'static str, f32, Message); 4] {
    transport_glyphs(is_playing).map(|(glyph, _word, width, message)| (glyph, width, message))
}

/// The compact row's volume slider parameters, in `chrome_slider` order: the
/// shared volume range and step, [`MINI_VOLUME_WIDTH`], and
/// [`Message::VolumeChange`]. Kept pure so a test can pin the range and
/// callback the row builds.
pub(super) fn mini_volume_spec() -> (f32, f32, f32, f32, fn(f32) -> Message) {
    (
        VOLUME_MIN,
        VOLUME_MAX,
        VOLUME_STEP,
        MINI_VOLUME_WIDTH,
        Message::VolumeChange,
    )
}

/// The compact row's readout: the elapsed (or remaining) time over the
/// track's total, such as `00:53/3:30`.
///
/// `elapsed` is passed through unchanged when `total` is the `--:--`
/// placeholder, so the empty and unknown-track states keep their single
/// placeholder instead of drawing `--:--/--:--`.
pub(super) fn mini_transport_readout<'a>(
    elapsed: Cow<'a, str>,
    total: Cow<'a, str>,
) -> Cow<'a, str> {
    if total == "--:--" {
        elapsed
    } else {
        Cow::Owned(format!("{elapsed}/{total}"))
    }
}

/// One full-width compact row: the four fixed-width buttons separated by a
/// 6px gap, the elapsed-over-total time readout, a fill spacer, and the volume
/// slider pinned to the row's right edge.
///
/// `is_playing` chooses the Play/Pause glyph; `volume` is the slider's value;
/// `elapsed` and `total` are the same strings
/// [`WinampPlayer::now_playing_elapsed`] and [`WinampPlayer::now_playing_time`]
/// give the Now Playing bar, so the elapsed/remaining choice and the track's
/// total are shown here too. When the total is unknown the readout stays the
/// plain `elapsed` string (see [`mini_transport_readout`]).
///
/// [`WinampPlayer::now_playing_elapsed`]: crate::ui::WinampPlayer::now_playing_elapsed
/// [`WinampPlayer::now_playing_time`]: crate::ui::WinampPlayer::now_playing_time
pub(super) fn mini_transport_row<'a>(
    is_playing: bool,
    volume: f32,
    elapsed: Cow<'a, str>,
    total: Cow<'a, str>,
) -> Row<'a, Message> {
    let mut row: Row<'a, Message> = Row::new().width(Length::Fill).align_y(Vertical::Center);
    for (label, width, message) in mini_transport_controls(is_playing) {
        // The button is `'static`; shorten it to the row's lifetime so the
        // borrowed time readout can share the row.
        let button: Element<'static, Message> = fixed_width_button(label, width, message).into();
        let button: Element<'a, Message> = button;
        row = row.push(button).push(spacer(6.0));
    }
    row.push(
        Text::new(mini_transport_readout(elapsed, total))
            .size(14)
            .color(theme::LCD_GREEN),
    )
    .push(Space::new().width(Length::Fill))
    .push(
        Container::new(mini_volume_slider(volume))
            .id(MINI_VOLUME_ID)
            .width(Length::Fixed(MINI_VOLUME_WIDTH)),
    )
}

/// The compact row's volume slider, built from [`mini_volume_spec`] and
/// painting the shared [`theme::VOLUME_SLIDER_FILL`] groove.
fn mini_volume_slider(volume: f32) -> Element<'static, Message> {
    let (min, max, step, width, on_change) = mini_volume_spec();
    chrome_slider(
        min,
        max,
        volume,
        step,
        width,
        theme::VOLUME_SLIDER_FILL,
        on_change,
    )
}

/// The normal-mode library footer: the shared compact row beneath the browse
/// list, on the window face's raised chrome.
///
/// `elapsed` and `total` are the same time strings the LCD shows, so the
/// footer's readout is the elapsed (or remaining) time over the track's total.
/// It keeps the row's fixed [`MINI_TRANSPORT_HEIGHT`] and its 6px horizontal
/// padding, so the slider lands at the same right edge as the shade strip's.
pub fn view_library_strip<'a>(
    is_playing: bool,
    volume: f32,
    elapsed: Cow<'a, str>,
    total: Cow<'a, str>,
) -> Element<'a, Message> {
    bevel::raised_panel(
        Container::new(mini_transport_row(is_playing, volume, elapsed, total))
            .width(Length::Fill)
            .height(Length::Fixed(MINI_TRANSPORT_HEIGHT))
            .align_y(Vertical::Center)
            .padding([0, 6]),
    )
}
