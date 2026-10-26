//! The transient Jump to File panel: a modal overlay over the showing song
//! list with a filter box and one row per matching title.

use iced::{
    Element, Length,
    widget::{Button, Column, Scrollable, Text, TextInput},
};

use crate::ui::{Message, style, theme};

use super::{labeled_button, modal::modal_overlay};

/// The Jump to File box's widget id, so `J` can focus it on opening.
pub const JUMP_FILE_INPUT_ID: &str = "nostalgia-jump-file";

/// The source indices of the `titles` matching `query`, in list order.
///
/// The query is trimmed and compared case-insensitively as a substring; an
/// empty query matches every index. The view and the submit handler both call
/// this one function, so the rows shown and the row Enter plays cannot drift.
pub(crate) fn jump_file_matches(titles: &[String], query: &str) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    titles
        .iter()
        .enumerate()
        .filter(|(_, title)| query.is_empty() || title.to_lowercase().contains(&query))
        .map(|(index, _)| index)
        .collect()
}

/// Stacks the transient Jump to File panel over `base`.
///
/// The panel is modal (see [`modal_overlay`]): `Close` and the backdrop each
/// publish [`Message::ToggleJumpFile`], while `Jump` and Enter in the box
/// publish [`Message::JumpFileSubmitted`]. Each matching row publishes
/// [`Message::JumpFileRowSelected`] with the `epoch` of the list that supplied
/// `titles` and its source index. An empty list reads `No songs`; a list with
/// no match reads `No matching songs`. No state or playback is touched here.
pub fn view_jump_file_overlay<'a>(
    base: Element<'a, Message>,
    query: &'a str,
    titles: &'a [String],
    epoch: u64,
) -> Element<'a, Message> {
    let matches = jump_file_matches(titles, query);
    let has_matches = !matches.is_empty();
    let mut rows = Column::new().spacing(2).padding(4);
    for index in matches {
        rows = rows.push(
            Button::new(Text::new(titles[index].as_str()).size(18))
                .width(Length::Fill)
                .on_press(Message::JumpFileRowSelected { epoch, index })
                .style(|_theme, status| style::playlist_row_style(status)),
        );
    }
    if titles.is_empty() {
        rows = rows.push(Text::new("No songs").size(18).color(theme::TEXT));
    } else if !has_matches {
        rows = rows.push(Text::new("No matching songs").size(18).color(theme::TEXT));
    }

    let input = TextInput::new("Filter songs", query)
        .id(JUMP_FILE_INPUT_ID)
        .on_input(Message::JumpFileChanged)
        .on_submit(Message::JumpFileSubmitted)
        .style(|_theme, status| style::chrome_text_input_style(status));
    let content = Column::new()
        .spacing(10)
        .align_x(iced::alignment::Horizontal::Center)
        .push(Text::new("JUMP TO FILE").size(18).color(theme::TITLE_TAPE))
        .push(input)
        .push(
            Scrollable::new(rows)
                .width(Length::Fill)
                .height(Length::Fixed(180.0))
                .style(|_theme, _status| style::playlist_scrollable_style()),
        )
        .push(
            iced::widget::Row::new()
                .spacing(10)
                .push(labeled_button("Jump", Message::JumpFileSubmitted))
                .push(labeled_button("Close", Message::ToggleJumpFile)),
        );
    modal_overlay(base, content, Message::ToggleJumpFile)
}
