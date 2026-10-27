//! The transient About panel: a modal overlay over the normal-mode screen,
//! naming the app, its package version, the mockup's one-line description,
//! and the library source currently answering browse queries.

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

/// Names the library source answering browse queries: the real Apple Music
/// service once a session is stored, the built-in sample library otherwise.
pub(super) fn library_source_label(signed_in: bool) -> &'static str {
    if signed_in {
        "Library: Apple Music"
    } else {
        "Library: Sample library"
    }
}

/// Stacks the transient About panel over `base`.
///
/// The panel is modal (see [`modal_overlay`]): the backdrop closes it on any
/// press, so only its [`Close`](Message::ToggleAbout) button dismisses it from
/// inside. The panel names the app, shows [`ABOUT_VERSION`] from the package,
/// carries [`ABOUT_TAGLINE`], and names the active library source from
/// `signed_in` (see [`library_source_label`]); no state or playback is touched
/// here.
pub fn view_about_overlay<'a>(base: Element<'a, Message>, signed_in: bool) -> Element<'a, Message> {
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
        .push(Text::new(library_source_label(signed_in)).color(theme::TEXT))
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

    #[test]
    fn library_source_label_names_the_signed_in_and_sample_libraries() {
        assert_eq!(library_source_label(true), "Library: Apple Music");
        assert_eq!(library_source_label(false), "Library: Sample library");
        assert_ne!(
            library_source_label(true),
            library_source_label(false),
            "the two library sources must read differently"
        );
    }
}
