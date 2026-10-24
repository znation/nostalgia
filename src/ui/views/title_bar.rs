//! The custom Winamp title bar and the mockups' decorative section bands: the
//! gold title tape, the centered label between two tapes, the clutter bar's
//! always-on-top toggle, and the shade, minimize, and close buttons.

use iced::{
    Color, Element, Length,
    widget::{Button, Column, Container, MouseArea, Row, Space, Text, container},
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
pub(super) const TITLE_BAR_BUTTON_WIDTH: f32 = 22.0;

/// The always-on-top clutter toggle's pinned width. It is narrower than the
/// window buttons: the A slot is a small square in the classic skin, and it
/// sits at the left edge of the bar rather than beside the window buttons.
pub(super) const CLUTTER_BUTTON_WIDTH: f32 = 18.0;

/// A 1px full-width strip in `color`, the tape's top light edge and bottom
/// shadow.
fn colored_strip(color: Color) -> Element<'static, Message> {
    Container::new(Space::new())
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(move |_theme| container::Style {
            background: Some(color.into()),
            ..container::Style::default()
        })
        .into()
}

/// An 8px-tall gold title tape: a light top rule, the [`theme::TITLE_TAPE`]
/// face, and a [`theme::TITLE_TAPE_SHADOW`] bottom rule. It fills the width it
/// is given and is vertically centered by the row that holds it, so a band's
/// two tapes split the spare width around the label.
fn title_tape() -> Element<'static, Message> {
    Column::new()
        .width(Length::Fill)
        .height(Length::Fixed(8.0))
        .push(colored_strip(theme::TEXT))
        .push(
            Container::new(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(theme::TITLE_TAPE.into()),
                    ..container::Style::default()
                }),
        )
        .push(colored_strip(theme::TITLE_TAPE_SHADOW))
        .into()
}

/// The tape/label/tape row shared by the app bar's drag region and the
/// decorative section bands. The two fill-width tapes split the spare width
/// equally, so the label sits at the row's exact center even though the app
/// bar's drag region is narrower than the window.
fn title_band_content(label: &'static str) -> Row<'static, Message> {
    Row::new()
        .push(title_tape())
        .push(Text::new(label).size(14).color(theme::TEXT))
        .push(title_tape())
        .spacing(6)
        .padding([0, 6])
        .align_y(iced::alignment::Vertical::Center)
        .width(Length::Fill)
        .height(Length::Fill)
}

/// A decorative section band in the mockups' title chrome: a raised
/// [`theme::TITLE_BLUE`] panel carrying `label` centered between two gold
/// tapes at [`TITLE_BAR_HEIGHT`].
///
/// It deliberately has no buttons: this app is one real window whose controls
/// live in the app bar, so a second set here would be clickable-looking dead
/// chrome. It emits no messages and paints [`style::title_bar_style`].
pub fn view_title_band(label: &'static str) -> Element<'static, Message> {
    bevel::raised_panel(
        Container::new(title_band_content(label))
            .width(Length::Fill)
            .height(Length::Fixed(TITLE_BAR_HEIGHT))
            .style(|_theme| style::title_bar_style()),
    )
}

/// The custom Winamp title bar: the app name centered between gold tapes on a
/// raised [`theme::TITLE_BLUE`] band, draggable to move the window, with the
/// clutter bar's always-on-top toggle at the left and shade, minimize, and
/// close buttons on the right. Double-clicking the band toggles the classic
/// roll-up (shade) mode via [`Message::ToggleWindowShade`].
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
        Container::new(title_band_content(TITLE_BAR_TEXT))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::WindowDragged)
    .on_double_click(Message::ToggleWindowShade);

    let clutter_toggle = Button::new(Text::new("A"))
        .on_press(Message::ToggleAlwaysOnTop)
        .width(Length::Fixed(CLUTTER_BUTTON_WIDTH))
        .style(move |_theme, status| style::toggle_button_style(always_on_top, status));

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
