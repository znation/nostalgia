//! The LCD's left-edge indicator strip: the classic `O A I D V` letters and
//! the messages its live `A`, `I`, and `V` cells publish.

use iced::{
    Length,
    mouse::Interaction,
    widget::{Column, MouseArea, Text},
};

use crate::ui::{Message, theme};

/// The classic indicator letters down the LCD's left edge, in the mockup's
/// order. [`lcd_indicator_strip`] draws them; `A`, `I`, and `V` are live
/// controls, while `O` and `D` stay decorative (an options menu and
/// pixel-doubled window mode have no implementation here).
pub const LCD_INDICATOR_LABELS: [&str; 5] = ["O", "A", "I", "D", "V"];

/// The fixed width of the indicator strip, in pixels, so the content beside
/// it keeps its bounds: the title starts one strip-width further right, while
/// the right-edge total stays at the edge because the content column fills
/// the remaining width.
pub const LCD_INDICATOR_WIDTH: f32 = 12.0;

/// The indicator letters' size in pixels: small enough to read as a column
/// beside the large elapsed readout.
const LCD_INDICATOR_SIZE: f32 = 10.0;

/// The message an indicator letter publishes when pressed: `A` toggles
/// always-on-top through the same [`Message::ToggleAlwaysOnTop`] the `A` key
/// uses, `I` opens the same File info panel as Alt+3 through
/// [`Message::ToggleFileInfo`], `V` toggles the LCD visualizer, and `O` and
/// `D` are decorative and publish nothing. Pure so the mapping is testable
/// without a renderer.
pub fn lcd_indicator_message(label: &str) -> Option<Message> {
    match label {
        "A" => Some(Message::ToggleAlwaysOnTop),
        "I" => Some(Message::ToggleFileInfo),
        "V" => Some(Message::ToggleVisualizer),
        _ => None,
    }
}

/// Builds the indicator strip at the LCD's left edge: one small green letter
/// per line, in [`LCD_INDICATOR_LABELS`] order, in a column of fixed
/// [`LCD_INDICATOR_WIDTH`]. `A`, `I`, and `V` are mouse areas that publish
/// [`Message::ToggleAlwaysOnTop`], [`Message::ToggleFileInfo`], and
/// [`Message::ToggleVisualizer`]; `O` and `D` stay plain, decorative text.
/// The strip is drawn in every playback state, and its fixed width pushes the
/// content beside it one strip-width right.
pub fn lcd_indicator_strip<'a, Theme, Renderer>() -> Column<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + 'a,
    Theme::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer + 'a,
{
    LCD_INDICATOR_LABELS.into_iter().fold(
        Column::new().width(Length::Fixed(LCD_INDICATOR_WIDTH)),
        |column, label| {
            let text = Text::new(label)
                .size(LCD_INDICATOR_SIZE)
                .color(theme::LCD_GREEN);
            match lcd_indicator_message(label) {
                Some(message) => column.push(
                    MouseArea::new(text)
                        .on_press(message)
                        .interaction(Interaction::Pointer),
                ),
                None => column.push(text),
            }
        },
    )
}
