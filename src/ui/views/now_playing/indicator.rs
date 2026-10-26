//! The LCD's left-edge indicator strip: the classic `O A I D V` letters, the
//! modes that light them, and the messages its live `A`, `I`, and `V` cells
//! publish.

use iced::{
    Color, Length,
    mouse::Interaction,
    widget::{Column, MouseArea, Text},
};

use crate::ui::{Message, theme};

/// The classic indicator letters down the LCD's left edge, in the mockup's
/// order. [`lcd_indicator_strip`] draws them; `A`, `I`, and `V` are live
/// controls, while `O` and `D` stay decorative (an options menu and
/// pixel-doubled window mode have no implementation here).
pub const LCD_INDICATOR_LABELS: [&str; 5] = ["O", "A", "I", "D", "V"];

/// The modes the indicator strip reports: `A` lights while always-on-top
/// holds, `I` while the File info panel is open, and `V` while the LCD
/// visualizer is shown. The remaining letters stay dim whatever these hold.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndicatorModes {
    pub always_on_top: bool,
    pub file_info_visible: bool,
    pub visualizer_visible: bool,
}

/// The color a strip letter draws in `modes`: [`theme::LCD_GREEN`] for the
/// live `A`, `I`, or `V` exactly while its mode holds, and
/// [`theme::LCD_GREEN_DIM`] for every other case, including the decorative
/// `O` and `D` and an inactive live letter. Pure so the exact colors are
/// testable without a renderer.
pub fn indicator_label_color(label: &str, modes: IndicatorModes) -> Color {
    match label {
        "A" if modes.always_on_top => theme::LCD_GREEN,
        "I" if modes.file_info_visible => theme::LCD_GREEN,
        "V" if modes.visualizer_visible => theme::LCD_GREEN,
        _ => theme::LCD_GREEN_DIM,
    }
}

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

/// Builds the indicator strip at the LCD's left edge: one small letter per
/// line, in [`LCD_INDICATOR_LABELS`] order, in a column of fixed
/// [`LCD_INDICATOR_WIDTH`]. `A`, `I`, and `V` are mouse areas that publish
/// [`Message::ToggleAlwaysOnTop`], [`Message::ToggleFileInfo`], and
/// [`Message::ToggleVisualizer`]; `O` and `D` stay plain, decorative text.
/// Each letter draws in [`indicator_label_color`], so the live letters light
/// only while their mode holds while `O` and `D` stay dim. The strip is drawn
/// in every playback state, and its fixed width pushes the content beside it
/// one strip-width right.
pub fn lcd_indicator_strip<'a, Theme, Renderer>(
    modes: IndicatorModes,
) -> Column<'a, Message, Theme, Renderer>
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
                .color(indicator_label_color(label, modes));
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
