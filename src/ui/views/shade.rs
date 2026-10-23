//! The compact transport strip shown while the window is rolled up in shade
//! mode: Previous, Play/Pause, Stop, Next, the current time readout, and a
//! volume slider.
//!
//! The strip is a twin of the full transport row's controls, not a second
//! playback path: every button emits the same [`Message`] its full-size twin
//! does, and the slider writes [`Message::VolumeChange`]. It reads the same
//! playback state and time string the Now Playing bar does, so the
//! elapsed/remaining toggle carries into shade mode.

use std::borrow::Cow;

use iced::{
    Element, Length,
    alignment::Vertical,
    widget::{Container, Row, Space, Text},
};

use crate::ui::{Message, bevel, style, theme};

use super::{
    VOLUME_MAX, VOLUME_MIN, VOLUME_STEP, chrome_slider, fixed_width_button, spacer,
    title_bar::TITLE_BAR_HEIGHT, transport::transport_glyphs,
};

/// The compact strip's height in px, below the title bar in shade mode.
pub(crate) const SHADE_STRIP_HEIGHT: f32 = 28.0;

/// The whole rolled-up window's height: the title bar plus the strip. The
/// view and `handlers::window::resize_to_shade` both read it, so the window
/// they resize to cannot drift from the tree it shows.
pub(crate) const SHADED_HEIGHT: f32 = TITLE_BAR_HEIGHT + SHADE_STRIP_HEIGHT;

/// The id of the strip's volume-slider container. An explicit [`Id`] lets a
/// test find the slider's laid-out bounds and pin its right-edge placement.
///
/// [`Id`]: iced::advanced::widget::Id
pub(super) const SHADE_VOLUME_ID: &str = "shade-volume";

/// The strip's volume slider width in px: narrow enough to leave the time
/// readout and four buttons room in the rolled-up row.
pub(super) const SHADE_VOLUME_WIDTH: f32 = 80.0;

/// The strip's four chrome buttons, in row order: Previous, Play/Pause, Stop,
/// and Next. It drops the tooltip words from [`transport_glyphs`] so the strip
/// and the full transport row share one glyph, order, width, and message spec
/// and cannot drift apart.
///
/// Returned as plain `(label, width, message)` tuples rather than widgets, so
/// the labels, messages, and widths are testable without an iced renderer.
pub(super) fn shade_controls(is_playing: bool) -> [(&'static str, f32, Message); 4] {
    transport_glyphs(is_playing).map(|(glyph, _word, width, message)| (glyph, width, message))
}

/// The shade-mode strip: the four compact transport buttons, the current
/// time readout, and a volume slider, on the title bar's blue chrome.
///
/// `is_playing` chooses the Play/Pause glyph; `volume` is the slider's value,
/// and dragging it emits [`Message::VolumeChange`]. `elapsed` is the same
/// string [`WinampPlayer::now_playing_elapsed`] gives the Now Playing bar, so
/// its elapsed/remaining choice is shown here too. The slider is pushed to
/// the right edge by a fill spacer, keeping the buttons and time readout
/// left-aligned.
///
/// [`WinampPlayer::now_playing_elapsed`]: crate::ui::WinampPlayer::now_playing_elapsed
pub fn view_shade_strip<'a>(
    is_playing: bool,
    volume: f32,
    elapsed: Cow<'a, str>,
) -> Element<'a, Message> {
    let mut row: Row<'a, Message> = Row::new().width(Length::Fill).align_y(Vertical::Center);
    for (label, width, message) in shade_controls(is_playing) {
        // The button is `'static`; shorten it to the row's lifetime so the
        // borrowed time readout can share the row.
        let button: Element<'static, Message> = fixed_width_button(label, width, message).into();
        let button: Element<'a, Message> = button;
        row = row.push(button).push(spacer(6.0));
    }
    row = row
        .push(Text::new(elapsed).size(14).color(theme::LCD_GREEN))
        .push(Space::new().width(Length::Fill))
        .push(
            Container::new(shade_volume_slider(volume))
                .id(SHADE_VOLUME_ID)
                .width(Length::Fixed(SHADE_VOLUME_WIDTH)),
        );

    bevel::raised_panel(
        Container::new(row)
            .width(Length::Fill)
            .height(Length::Fixed(SHADE_STRIP_HEIGHT))
            .align_y(Vertical::Center)
            .padding([0, 6])
            .style(|_theme| style::title_bar_style()),
    )
}

/// The strip's volume slider: the shared volume range and step, pinned to
/// [`SHADE_VOLUME_WIDTH`], emitting [`Message::VolumeChange`] like the full
/// transport row's volume slider and painting the same
/// [`theme::VOLUME_SLIDER_FILL`] groove. Returned as a widget only; the test
/// pins its spec through [`shade_volume_spec`].
fn shade_volume_slider(volume: f32) -> Element<'static, Message> {
    let (min, max, step, width, on_change) = shade_volume_spec();
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

/// The strip's volume slider parameters, in `chrome_slider` order: the shared
/// volume range and step, [`SHADE_VOLUME_WIDTH`], and [`Message::VolumeChange`].
/// Kept pure so a test can pin the range and callback the strip builds.
pub(super) fn shade_volume_spec() -> (f32, f32, f32, f32, fn(f32) -> Message) {
    (
        VOLUME_MIN,
        VOLUME_MAX,
        VOLUME_STEP,
        SHADE_VOLUME_WIDTH,
        Message::VolumeChange,
    )
}
