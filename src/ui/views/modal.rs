//! The shared modal chrome behind the transient panels: a dimming backdrop
//! over the base screen and a centered raised panel.

use iced::{
    Color, Element, Length,
    widget::{Column, Container, MouseArea, Space, Stack, container},
};

use crate::ui::{Message, bevel, theme};

/// The translucent scrim that dims the base screen behind the panel.
const BACKDROP: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.55);

/// A modal panel's widest allowed width, so it stays a compact dialog on a
/// wide window.
const PANEL_MAX_WIDTH: f32 = 420.0;

/// Stacks a transient panel holding `content` over `base`.
///
/// The panel is modal: a full-window [`MouseArea`] backdrop publishes `close`
/// on any press, while the centered [`bevel::raised_panel`] swallows presses on
/// its own chrome with [`Message::Ignored`] (both also capture the mouse wheel
/// the same way, so a scroll never reaches the base beneath). It reuses the
/// existing raised-chrome bevel, window face, and button styles; no state or
/// playback is touched here.
pub(super) fn modal_overlay<'a>(
    base: Element<'a, Message>,
    content: Column<'a, Message>,
    close: Message,
) -> Element<'a, Message> {
    let backdrop = MouseArea::new(
        Container::new(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(BACKDROP.into()),
                ..container::Style::default()
            }),
    )
    .on_press(close)
    .on_scroll(|_| Message::Ignored);

    let panel =
        bevel::raised_panel(
            Container::new(content)
                .padding(16)
                .style(|_theme| container::Style {
                    background: Some(theme::WINDOW_BACKGROUND.into()),
                    ..container::Style::default()
                }),
        );
    let panel = Container::new(
        MouseArea::new(panel)
            .on_press(Message::Ignored)
            .on_scroll(|_| Message::Ignored),
    )
    .width(Length::Fill)
    .max_width(PANEL_MAX_WIDTH);

    Stack::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(base)
        .push(backdrop)
        .push(Container::new(panel).center(Length::Fill))
        .into()
}
