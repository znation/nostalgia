//! The Now Playing seek slider: a chrome groove beneath the LCD whose thumb
//! follows the loaded preview's playback position and which seeks when the
//! user drags and releases it.

use std::time::Duration;

use iced::{
    Element, Length,
    widget::{Container, Slider, Space},
};

use crate::ui::{Message, style};

/// The seek strip's fixed height. The slider and the blank stand-in both fill
/// it, so loading a preview does not jump the layout below.
pub(super) const SEEK_BAR_HEIGHT: f32 = 16.0;

/// Resolves the seek bar's `(thumb seconds, length seconds)` from the
/// backend-reported position and preview length.
///
/// A missing position answers `None` even when the length is known: the bar
/// only appears once the backend reports a position to place its thumb. The
/// thumb is clamped into `0..=length`, and a missing or zero preview length
/// also answers `None` because there is no scale to seek along. The view
/// draws either case as a blank strip.
pub fn seek_bar_value(
    position: Option<Duration>,
    duration: Option<Duration>,
) -> Option<(f32, f32)> {
    let position = position?;
    let length = duration
        .filter(|duration| !duration.is_zero())?
        .as_secs_f32();
    let thumb = position.as_secs_f32().clamp(0.0, length);
    Some((thumb, length))
}

/// Builds the chrome seek slider over the preview's own length, or `None`
/// when [`seek_bar_value`] cannot place a thumb (see its rules).
///
/// Both seek controls build from this one constructor, so the Now Playing
/// strip and the rolled-up shade strip share a scale, step, and message pair:
/// dragging emits [`Message::SeekPreviewChanged`] and releasing emits
/// [`Message::SeekPreviewReleased`], so only the release reaches the backend.
pub(super) fn seek_slider(
    position: Option<Duration>,
    duration: Option<Duration>,
) -> Option<Slider<'static, f32, Message>> {
    let (thumb, length) = seek_bar_value(position, duration)?;
    Some(
        Slider::new(0.0..=length, thumb, Message::SeekPreviewChanged)
            .step(1.0)
            .on_release(Message::SeekPreviewReleased)
            .width(Length::Fill)
            .style(|_theme, status| style::chrome_slider_style(status)),
    )
}

/// Builds the seek strip beneath the LCD: a full-width chrome slider over the
/// preview's own length while one is reported, or an empty fixed-height strip
/// otherwise.
///
/// Dragging emits [`Message::SeekPreviewChanged`] and releasing emits
/// [`Message::SeekPreviewReleased`], so only the release reaches the backend.
pub fn view_seek_bar(
    position: Option<Duration>,
    duration: Option<Duration>,
) -> Element<'static, Message> {
    let content: Element<'static, Message> = match seek_slider(position, duration) {
        Some(slider) => slider.into(),
        None => Space::new().into(),
    };
    Container::new(content)
        .height(Length::Fixed(SEEK_BAR_HEIGHT))
        .into()
}
