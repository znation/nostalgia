//! The transient About panel: a modal overlay over the normal-mode screen,
//! naming the app, its package version, and the mockup's one-line
//! description.

use iced::{
    Element,
    widget::{Column, Text},
};

use crate::ui::{Message, theme};

use super::{labeled_button, modal::modal_overlay};

/// The About panel's version line, prefixed so it reads as a version rather
/// than a bare number.
const ABOUT_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

/// The mockup's one-line description of the app.
const ABOUT_TAGLINE: &str =
    "The classic Winamp look, rebuilt in Rust — with Apple Music as the library.";

/// Stacks the transient About panel over `base`.
///
/// The panel is modal (see [`modal_overlay`]): the backdrop closes it on any
/// press, so only its [`Close`](Message::ToggleAbout) button dismisses it from
/// inside. The panel names the app, shows [`ABOUT_VERSION`] from the package,
/// and carries [`ABOUT_TAGLINE`]; no state or playback is touched here.
pub fn view_about_overlay<'a>(base: Element<'a, Message>) -> Element<'a, Message> {
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

    modal_overlay(base, content, Message::ToggleAbout)
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
