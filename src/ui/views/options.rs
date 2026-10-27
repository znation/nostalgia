//! The transient options menu: a modal overlay listing the app's existing
//! on/off choices plus an About row, each applying that choice and closing the
//! menu like a Winamp menu item. Its highlight is layout-only and unpersisted.

use iced::{
    Element,
    widget::{Column, Text},
};

use crate::ui::{Message, OptionsRow, style, theme};

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

/// Resolve a row's label from the current toggle state.
pub(super) fn options_row_label(
    row: OptionsRow,
    always_on_top: bool,
    time_remaining: bool,
    visualizer_visible: bool,
    eq_panel_visible: bool,
    library_panel_visible: bool,
) -> &'static str {
    match row {
        OptionsRow::AlwaysOnTop => always_on_top_label(always_on_top),
        OptionsRow::TimeDisplay => time_label(time_remaining),
        OptionsRow::Visualizer => visualizer_label(visualizer_visible),
        OptionsRow::EqualizerPanel => equalizer_label(eq_panel_visible),
        OptionsRow::LibraryPanel => library_label(library_panel_visible),
        OptionsRow::About => "About Nostalgia",
        OptionsRow::Close => "Close",
    }
}

pub(super) fn options_row_style(
    selected: bool,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut chrome = style::chrome_button_style(status);
    if selected {
        chrome.background = Some(theme::ROW_HIGHLIGHT.into());
        chrome.text_color = theme::PLAYING_ROW_TEXT;
    }
    chrome
}

/// Stacks the transient options menu over `base` through [`modal_overlay`].
/// The backdrop and Close publish [`Message::ToggleOptions`]; toggle rows
/// publish [`Message::OptionsChoice`] and About publishes [`Message::ToggleAbout`].
/// Each row closes the menu through its existing handler. `selected` highlights
/// the keyboard row without changing the labels or mouse messages.
pub fn view_options_overlay<'a>(
    base: Element<'a, Message>,
    always_on_top: bool,
    time_remaining: bool,
    visualizer_visible: bool,
    eq_panel_visible: bool,
    library_panel_visible: bool,
    selected: usize,
) -> Element<'a, Message> {
    let mut content = Column::new()
        .spacing(8)
        .align_x(iced::alignment::Horizontal::Center)
        .push(Text::new("OPTIONS").size(18).color(theme::TITLE_TAPE));
    for (index, row) in OptionsRow::ALL.into_iter().enumerate() {
        content = content.push(
            labeled_button(
                options_row_label(
                    row,
                    always_on_top,
                    time_remaining,
                    visualizer_visible,
                    eq_panel_visible,
                    library_panel_visible,
                ),
                row.message(),
            )
            .style(move |_, status| options_row_style(index == selected, status)),
        );
    }

    // The stack forwards keyboard events to its base, including a focused
    // search input's Enter. Covered controls must not act behind this menu.
    modal_overlay(
        base.map(|_| Message::Ignored),
        content,
        Message::ToggleOptions,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_row_style_highlights_selection_in_each_status() {
        use iced::widget::button::Status;
        for status in [
            Status::Active,
            Status::Hovered,
            Status::Pressed,
            Status::Disabled,
        ] {
            assert_eq!(
                options_row_style(false, status),
                style::chrome_button_style(status)
            );
            let selected = options_row_style(true, status);
            assert_eq!(selected.background, Some(theme::ROW_HIGHLIGHT.into()));
            assert_eq!(selected.text_color, theme::PLAYING_ROW_TEXT);
        }
    }

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
