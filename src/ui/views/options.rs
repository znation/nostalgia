//! The transient options menu: a modal overlay listing the app's existing
//! on/off choices plus an About row, each applying that choice and closing the
//! menu like a Winamp menu item. The menu holds no state of its own and is not
//! persisted.

use iced::{
    Element,
    widget::{Column, Text},
};

use crate::ui::{Message, OptionsChoice, theme};

use super::{labeled_button, modal::modal_overlay};

/// The always-on-top row's label: `On` while the window is pinned above other
/// windows, `Off` otherwise. Pure so the wording is testable without a
/// renderer, like the other row labels.
pub(super) fn always_on_top_label(always_on_top: bool) -> &'static str {
    if always_on_top {
        "Always on Top: On"
    } else {
        "Always on Top: Off"
    }
}

/// The time row's label: `Remaining` while the large readout counts down,
/// `Elapsed` otherwise.
pub(super) fn time_label(time_remaining: bool) -> &'static str {
    if time_remaining {
        "Time: Remaining"
    } else {
        "Time: Elapsed"
    }
}

/// The visualizer row's label: `Shown` while the LCD draws the spectrum bars,
/// `Hidden` otherwise.
pub(super) fn visualizer_label(visualizer_visible: bool) -> &'static str {
    if visualizer_visible {
        "Visualizer: Shown"
    } else {
        "Visualizer: Hidden"
    }
}

/// The equalizer row's label: `Shown` while the equalizer section is drawn,
/// `Hidden` otherwise.
pub(super) fn equalizer_label(eq_panel_visible: bool) -> &'static str {
    if eq_panel_visible {
        "Equalizer: Shown"
    } else {
        "Equalizer: Hidden"
    }
}

/// The library row's label: `Shown` while the library section is drawn,
/// `Hidden` otherwise.
pub(super) fn library_label(library_panel_visible: bool) -> &'static str {
    if library_panel_visible {
        "Library: Shown"
    } else {
        "Library: Hidden"
    }
}

/// Stacks the transient options menu over `base`.
///
/// The panel is modal (see [`modal_overlay`]): the backdrop publishes
/// [`Message::ToggleOptions`] on any press, and the `Close` row publishes it
/// too, so either dismisses the menu. Each of the five choice rows publishes
/// [`Message::OptionsChoice`], whose handler closes the menu before applying
/// that choice; `About Nostalgia` publishes [`Message::ToggleAbout`], whose
/// opener closes this menu through `WinampPlayer::close_transient_panels`. So
/// every row dismisses the menu. Every label reflects the passed state on each
/// rebuild; no state or playback is touched here.
pub fn view_options_overlay<'a>(
    base: Element<'a, Message>,
    always_on_top: bool,
    time_remaining: bool,
    visualizer_visible: bool,
    eq_panel_visible: bool,
    library_panel_visible: bool,
) -> Element<'a, Message> {
    let content = Column::new()
        .spacing(8)
        .align_x(iced::alignment::Horizontal::Center)
        .push(Text::new("OPTIONS").size(18).color(theme::TITLE_TAPE))
        .push(labeled_button(
            always_on_top_label(always_on_top),
            Message::OptionsChoice(OptionsChoice::AlwaysOnTop),
        ))
        .push(labeled_button(
            time_label(time_remaining),
            Message::OptionsChoice(OptionsChoice::TimeDisplay),
        ))
        .push(labeled_button(
            visualizer_label(visualizer_visible),
            Message::OptionsChoice(OptionsChoice::Visualizer),
        ))
        .push(labeled_button(
            equalizer_label(eq_panel_visible),
            Message::OptionsChoice(OptionsChoice::EqualizerPanel),
        ))
        .push(labeled_button(
            library_label(library_panel_visible),
            Message::OptionsChoice(OptionsChoice::LibraryPanel),
        ))
        .push(labeled_button("About Nostalgia", Message::ToggleAbout))
        .push(labeled_button("Close", Message::ToggleOptions));

    modal_overlay(base, content, Message::ToggleOptions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_on_top_label_mirrors_the_flag() {
        assert_eq!(always_on_top_label(true), "Always on Top: On");
        assert_eq!(always_on_top_label(false), "Always on Top: Off");
    }

    #[test]
    fn time_label_names_the_readout_mode() {
        assert_eq!(time_label(true), "Time: Remaining");
        assert_eq!(time_label(false), "Time: Elapsed");
    }

    #[test]
    fn visualizer_label_mirrors_the_flag() {
        assert_eq!(visualizer_label(true), "Visualizer: Shown");
        assert_eq!(visualizer_label(false), "Visualizer: Hidden");
    }

    #[test]
    fn equalizer_label_mirrors_the_flag() {
        assert_eq!(equalizer_label(true), "Equalizer: Shown");
        assert_eq!(equalizer_label(false), "Equalizer: Hidden");
    }

    #[test]
    fn library_label_mirrors_the_flag() {
        assert_eq!(library_label(true), "Library: Shown");
        assert_eq!(library_label(false), "Library: Hidden");
    }
}
