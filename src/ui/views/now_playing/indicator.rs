//! The LCD's left-edge indicator strip: the classic `O A I D V` letters, the
//! modes that light them, and the messages its live `O`, `A`, `I`, and `V`
//! cells publish.

use iced::{
    Color, Length,
    mouse::Interaction,
    widget::{Column, MouseArea, Text},
};

use crate::ui::{Message, theme};

/// The classic indicator letters down the LCD's left edge, in the mockup's
/// order. [`lcd_indicator_strip`] draws them; `O`, `A`, `I`, and `V` are live
/// controls, while `D` stays decorative (pixel-doubled window mode has no
/// implementation here).
pub const LCD_INDICATOR_LABELS: [&str; 5] = ["O", "A", "I", "D", "V"];

/// The modes the indicator strip reports: `O` lights while the options menu
/// is open, `A` while always-on-top holds, `I` while the File info panel is
/// open, and `V` while the LCD visualizer is shown. `D` stays dim whatever
/// these hold.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndicatorModes {
    pub options_visible: bool,
    pub always_on_top: bool,
    pub file_info_visible: bool,
    pub visualizer_visible: bool,
}

/// The color a strip letter draws in `modes`: [`theme::LCD_GREEN`] for the live `O`, `A`, `I`, or
/// `V` exactly while its mode holds, and [`theme::LCD_GREEN_DIM`] for every other case, including
/// the decorative `D` and an inactive live letter. Pure so the exact colors are testable without a
/// renderer.
pub fn indicator_label_color(label: &str, modes: IndicatorModes) -> Color {
    match label {
        "O" if modes.options_visible => theme::LCD_GREEN,
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

/// The indicator letters' size in pixels: a multiple of 8, so the letterform
/// face draws each letter crisp (the split's on-grid mapping, 10 \u2192 8).
const LCD_INDICATOR_SIZE: f32 = 8.0;

/// The message an indicator letter publishes when pressed: `O` opens the
/// options menu through [`Message::ToggleOptions`], `A` toggles always-on-top
/// through the same [`Message::ToggleAlwaysOnTop`] the `A` key uses, `I` opens
/// the same File info panel as Alt+3 through [`Message::ToggleFileInfo`], `V`
/// toggles the LCD visualizer, and `D` is decorative and publishes nothing.
/// Pure so the mapping is testable without a renderer.
pub fn lcd_indicator_message(label: &str) -> Option<Message> {
    match label {
        "O" => Some(Message::ToggleOptions),
        "A" => Some(Message::ToggleAlwaysOnTop),
        "I" => Some(Message::ToggleFileInfo),
        "V" => Some(Message::ToggleVisualizer),
        _ => None,
    }
}

/// Builds the indicator strip at the LCD's left edge: one small letter per line, in
/// [`LCD_INDICATOR_LABELS`] order, in a column of fixed [`LCD_INDICATOR_WIDTH`]. `O`, `A`, `I`, and
/// `V` are mouse areas that publish [`Message::ToggleOptions`], [`Message::ToggleAlwaysOnTop`],
/// [`Message::ToggleFileInfo`], and [`Message::ToggleVisualizer`]; `D` stays plain, decorative
/// text. Each letter draws in [`indicator_label_color`], so the live letters light only while their
/// mode holds while `D` stays dim. The strip is drawn in every playback state, and its fixed width
/// pushes the content beside it one strip-width right.
pub fn lcd_indicator_strip<'a, Theme, Renderer>(
    modes: IndicatorModes,
) -> Column<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + 'a,
    Theme::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer + 'a,
    <Renderer as iced::advanced::text::Renderer>::Font: From<iced::Font>,
{
    LCD_INDICATOR_LABELS.into_iter().fold(
        Column::new().width(Length::Fixed(LCD_INDICATOR_WIDTH)),
        |column, label| {
            let text = Text::new(label)
                .size(LCD_INDICATOR_SIZE)
                .font(theme::ui_font())
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

#[cfg(test)]
mod size_pins {
    // Part 2/4 of the pixel-font split re-based the indicator letters onto
    // the 8px grid (10 -> 8; only multiples of 8 keep the embedded faces
    // crisp), so pin it.
    use super::LCD_INDICATOR_SIZE;

    #[test]
    fn indicator_text_sizes_stay_on_the_pixel_grid() {
        assert_eq!(LCD_INDICATOR_SIZE, 8.0);
        assert_eq!(LCD_INDICATOR_SIZE % 8.0, 0.0);
    }
}
