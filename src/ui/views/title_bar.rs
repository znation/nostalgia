//! The custom Winamp title bar: the app name band, the clutter bar's
//! always-on-top toggle, and the shade, minimize, and close buttons.

use iced::{
    Element, Length,
    widget::{Button, Container, MouseArea, Row, Text, button},
};

use crate::ui::{Message, bevel, style, theme};

use super::fixed_width_button;

/// The custom title bar's app name.
const TITLE_BAR_TEXT: &str = "NOSTALGIA";

/// The custom title bar's height in px: short, like the classic Winamp title
/// bar, rather than the OS frame's.
pub const TITLE_BAR_HEIGHT: f32 = 24.0;

/// Each title-bar button's pinned width, so its glyph cannot resize it and
/// reflow the drag region beside it.
const TITLE_BAR_BUTTON_WIDTH: f32 = 22.0;

/// The always-on-top clutter toggle's pinned width. It is narrower than the
/// window buttons: the A slot is a small square in the classic skin, and it
/// sits at the left edge of the bar rather than beside the window buttons.
const CLUTTER_BUTTON_WIDTH: f32 = 18.0;

/// The custom Winamp title bar: the app name on a raised
/// [`theme::TITLE_BLUE`] band, draggable to move the window, with the clutter
/// bar's always-on-top toggle at the left and shade, minimize, and close
/// buttons on the right. Double-clicking the band toggles the classic roll-up
/// (shade) mode via [`Message::ToggleWindowShade`].
///
/// The whole band except the buttons is a [`MouseArea`], so a press anywhere
/// on it emits [`Message::WindowDragged`] and the update loop begins the OS
/// window drag, while a double-click emits [`Message::ToggleWindowShade`].
/// The A toggle is sunken while `always_on_top` holds and raised otherwise;
/// the shade, minimize, and close buttons are [`fixed_width_button`]s — the
/// same raised chrome as the transport row — pinned so their glyphs cannot
/// resize them. The bar paints [`style::title_bar_style`] and wraps it in
/// [`bevel::raised_panel`], so it carries the base skin's raised bevel.
pub fn view_title_bar(always_on_top: bool) -> Element<'static, Message> {
    let drag_region = MouseArea::new(
        Container::new(Text::new(TITLE_BAR_TEXT).size(14).color(theme::TEXT))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .padding([0, 6]),
    )
    .on_press(Message::WindowDragged)
    .on_double_click(Message::ToggleWindowShade);

    let clutter_toggle = Button::new(Text::new("A"))
        .on_press(Message::ToggleAlwaysOnTop)
        .width(Length::Fixed(CLUTTER_BUTTON_WIDTH))
        .style(move |_theme, status| {
            if always_on_top {
                style::chrome_button_style(button::Status::Pressed)
            } else {
                style::chrome_button_style(status)
            }
        });

    bevel::raised_panel(
        Container::new(
            Row::new()
                .push(clutter_toggle)
                .push(drag_region)
                .push(fixed_width_button(
                    "▭",
                    TITLE_BAR_BUTTON_WIDTH,
                    Message::ToggleWindowShade,
                ))
                .push(fixed_width_button(
                    "–",
                    TITLE_BAR_BUTTON_WIDTH,
                    Message::MinimizeWindow,
                ))
                .push(fixed_width_button(
                    "✕",
                    TITLE_BAR_BUTTON_WIDTH,
                    Message::CloseWindow,
                )),
        )
        .width(Length::Fill)
        .height(Length::Fixed(TITLE_BAR_HEIGHT))
        .style(|_theme| style::title_bar_style()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // The title bar's buttons must stay pinned, for the same reason the
    // transport buttons were pinned after the label-reflow bug: a glyph whose
    // metrics change would otherwise resize its button and shift the drag
    // region or the buttons after it. These widths are private to this module
    // and no other test reads them, so a zeroed or widened button could pass
    // the suite while the bar reflowed. Pin the widths; the 22 > 18 split is
    // the documented narrow A slot.
    #[test]
    fn title_bar_buttons_keep_their_pinned_widths() {
        assert_eq!(TITLE_BAR_BUTTON_WIDTH, 22.0);
        assert_eq!(CLUTTER_BUTTON_WIDTH, 18.0);
    }
}
