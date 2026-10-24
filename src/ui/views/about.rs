//! The transient About panel: a modal overlay over the normal-mode screen,
//! naming the app, its package version, and the mockup's one-line
//! description.

use iced::{
    Color, Element, Length,
    widget::{Column, Container, MouseArea, Space, Stack, Text, container},
};

use crate::ui::{Message, bevel, theme};

use super::labeled_button;

/// The About panel's version line, prefixed so it reads as a version rather
/// than a bare number.
const ABOUT_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

/// The mockup's one-line description of the app.
const ABOUT_TAGLINE: &str =
    "The classic Winamp look, rebuilt in Rust — with Apple Music as the library.";

/// The translucent scrim that dims the base screen behind the panel.
const BACKDROP: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.55);

/// The About panel's widest allowed width, so it stays a compact dialog on a
/// wide window.
const PANEL_MAX_WIDTH: f32 = 420.0;

/// Stacks the transient About panel over `base`.
///
/// The panel is modal: a full-window [`MouseArea`] backdrop closes it on any
/// press, while the centered [`bevel::raised_panel`] swallows presses on its
/// own chrome with [`Message::Ignored`] (both also capture the mouse wheel
/// the same way, so a scroll never reaches the base beneath) so only its [`Close`](Message::ToggleAbout)
/// button dismisses it from inside. The panel names the app, shows
/// [`ABOUT_VERSION`] from the package, and carries [`ABOUT_TAGLINE`]. It
/// reuses the existing raised-chrome bevel, window face, and button styles;
/// no state or playback is touched here.
pub fn view_about_overlay<'a>(base: Element<'a, Message>) -> Element<'a, Message> {
    let backdrop = MouseArea::new(
        Container::new(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(BACKDROP.into()),
                ..container::Style::default()
            }),
    )
    .on_press(Message::ToggleAbout)
    .on_scroll(|_| Message::Ignored);

    let content = Column::new()
        .spacing(10)
        .align_x(iced::alignment::Horizontal::Center)
        .push(
            Text::new("ABOUT NOSTALGIA")
                .size(18)
                .color(theme::TITLE_TAPE),
        )
        .push(Text::new(ABOUT_VERSION).color(theme::TEXT))
        .push(Text::new(ABOUT_TAGLINE).color(theme::TEXT))
        .push(labeled_button("Close", Message::ToggleAbout));

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_version_prepends_a_v_to_the_package_version() {
        assert_eq!(ABOUT_VERSION, concat!("v", env!("CARGO_PKG_VERSION")));
        assert!(ABOUT_VERSION.starts_with('v'));
    }
}
