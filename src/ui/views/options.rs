//! The transient options menu: a modal overlay listing the app's existing
//! on/off choices plus Jump to time, Jump to file, and About rows, each applying that choice and closing the
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

/// The stop-after-current row's label: `On` while the one-shot is armed,
/// `Off` otherwise.
pub(super) fn stop_after_current_label(stop_after_current: bool) -> &'static str {
    if stop_after_current {
        "Stop after current: On"
    } else {
        "Stop after current: Off"
    }
}

/// The repeat row's label: `On` while Repeat is enabled, `Off` otherwise.
pub(super) fn repeat_label(repeat: bool) -> &'static str {
    if repeat { "Repeat: On" } else { "Repeat: Off" }
}

/// The shuffle row's label: `On` while Shuffle is enabled, `Off` otherwise.
pub(super) fn shuffle_label(shuffle: bool) -> &'static str {
    if shuffle {
        "Shuffle: On"
    } else {
        "Shuffle: Off"
    }
}

/// The current toggle state the options menu's labels are resolved from.
#[derive(Debug, Clone, Copy)]
pub struct OptionsState {
    /// The window is pinned above other windows.
    pub always_on_top: bool,
    /// The large readout counts down instead of up.
    pub time_remaining: bool,
    /// The LCD draws the spectrum bars.
    pub visualizer_visible: bool,
    /// The equalizer section is drawn.
    pub eq_panel_visible: bool,
    /// The library section is drawn.
    pub library_panel_visible: bool,
    /// Playback stops at the end of the current song.
    pub stop_after_current: bool,
    /// Repeat is enabled.
    pub repeat: bool,
    /// Shuffle is enabled.
    pub shuffle: bool,
}

/// Resolve a row's label from the current toggle state.
pub(super) fn options_row_label(row: OptionsRow, state: OptionsState) -> &'static str {
    match row {
        OptionsRow::AlwaysOnTop => always_on_top_label(state.always_on_top),
        OptionsRow::TimeDisplay => time_label(state.time_remaining),
        OptionsRow::Visualizer => visualizer_label(state.visualizer_visible),
        OptionsRow::EqualizerPanel => equalizer_label(state.eq_panel_visible),
        OptionsRow::LibraryPanel => library_label(state.library_panel_visible),
        OptionsRow::StopAfterCurrent => stop_after_current_label(state.stop_after_current),
        OptionsRow::Repeat => repeat_label(state.repeat),
        OptionsRow::Shuffle => shuffle_label(state.shuffle),
        OptionsRow::JumpTime => "Jump to time",
        OptionsRow::JumpFile => "Jump to file",
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
/// publish [`Message::OptionsChoice`] (so do the Jump rows) and About publishes [`Message::ToggleAbout`].
/// Each row closes the menu through its existing handler. `selected` highlights
/// the keyboard row without changing the labels or mouse messages.
pub fn view_options_overlay<'a>(
    base: Element<'a, Message>,
    state: OptionsState,
    selected: usize,
) -> Element<'a, Message> {
    let mut content = Column::new()
        .spacing(8)
        .align_x(iced::alignment::Horizontal::Center)
        .push(Text::new("OPTIONS").size(18).color(theme::TITLE_TAPE));
    for (index, row) in OptionsRow::ALL.into_iter().enumerate() {
        content = content.push(
            labeled_button(options_row_label(row, state), row.message())
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
    fn repeat_and_shuffle_labels_mirror_their_flags() {
        assert_eq!(repeat_label(true), "Repeat: On");
        assert_eq!(repeat_label(false), "Repeat: Off");
        assert_eq!(shuffle_label(true), "Shuffle: On");
        assert_eq!(shuffle_label(false), "Shuffle: Off");
    }

    #[test]
    fn stop_after_current_label_mirrors_the_flag() {
        assert_eq!(stop_after_current_label(true), "Stop after current: On");
        assert_eq!(stop_after_current_label(false), "Stop after current: Off");
    }

    #[test]
    fn library_label_mirrors_the_flag() {
        assert_eq!(library_label(true), "Library: Shown");
        assert_eq!(library_label(false), "Library: Hidden");
    }
}
