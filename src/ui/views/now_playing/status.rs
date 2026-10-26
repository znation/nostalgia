//! The LCD's status row: the loaded preview's bitrate/sample-rate prefix and
//! the `MONO`/`STEREO` channel labels, with the lit/dim color rule.

use iced::{
    Color, Length,
    widget::{Row, Space, Text},
};

use crate::audio::PlaybackFormat;
use crate::ui::{Message, theme};

use super::text::{channel_mode_label, format_playback_prefix};

/// The fixed height of the LCD's status row, in pixels. The row keeps this
/// height whether or not a preview reports a format, so loading one does not
/// make the bar jump.
const LCD_STATUS_HEIGHT: f32 = 16.0;

/// The status text's size in pixels: small enough to sit quietly beneath the
/// title line.
const LCD_STATUS_SIZE: f32 = 12.0;

/// The LCD status row's channel indicators, in the mockup's left-to-right
/// order: the mono label first, then stereo.
pub const CHANNEL_LABELS: [&str; 2] = ["MONO", "STEREO"];

/// The color [`now_playing_status_row`] draws `label` in: the reported channel
/// mode is lit [`theme::LCD_GREEN`], and the other label (or both, with no
/// reported format) is dim [`theme::LCD_GREEN_DIM`]. Pure so the exact
/// lit/dim mapping is testable without a renderer.
pub fn channel_label_color(channels: Option<u16>, label: &str) -> Color {
    match channels {
        Some(channels) if label == channel_mode_label(channels) => theme::LCD_GREEN,
        _ => theme::LCD_GREEN_DIM,
    }
}

/// Builds the LCD's status row in the mockup's horizontal layout: the loaded
/// preview's bitrate and sample rate at the left, then the `MONO`/`STEREO`
/// channel labels at the right edge, in small text beneath the title line. The
/// reported mode is lit green and the other label is dim; with no reported
/// format the labels stay dim and no prefix is drawn, while the row keeps its
/// fixed height so the bar does not jump when a preview loads. Generic over the
/// theme and renderer so the tests can lay it out with iced's null renderer and
/// read back the text it would draw.
pub fn now_playing_status_row<'a, Theme, Renderer>(
    format: Option<PlaybackFormat>,
) -> Row<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + 'a,
    Theme::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer + 'a,
{
    let mut row = Row::new().height(Length::Fixed(LCD_STATUS_HEIGHT));
    let channels = format.map(|format| format.channels);
    if let Some(format) = format {
        row = row.push(
            Text::new(format_playback_prefix(format))
                .size(LCD_STATUS_SIZE)
                .color(theme::LCD_GREEN),
        );
    }
    row = row.push(Space::new().width(Length::Fill));
    for (index, label) in CHANNEL_LABELS.iter().enumerate() {
        if index > 0 {
            row = row.push(Space::new().width(Length::Fixed(6.0)));
        }
        row = row.push(
            Text::new(*label)
                .size(LCD_STATUS_SIZE)
                .color(channel_label_color(channels, label)),
        );
    }
    row
}
