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

/// The source index the Jump to File panel highlights: `selected` when it is
/// still one of the `matches`, otherwise the first match. `None` for an empty
/// match list.
///
/// The view and the submit handler both call this one function, so the row
/// drawn with the selection bar and the row Enter or `Jump` plays cannot
/// drift. A selection that outlived a changed filter falls back to the first
/// match, the same as `None`.
pub(crate) fn jump_file_highlighted(matches: &[usize], selected: Option<usize>) -> Option<usize> {
    selected
        .filter(|index| matches.contains(index))
        .or_else(|| matches.first().copied())
}

/// The label a Jump to File row draws for the source `index`: its 1-based
/// position in the showing list, then a period, then its cached title.
///
/// `index` is the same 0-based source index [`jump_file_matches`] returns and
/// a row press dispatches, so a filtered list keeps the number its source list
/// shows instead of renumbering the matches.
pub(crate) fn jump_file_row_label(index: usize, title: &str) -> String {
    format!("{}. {title}", index + 1)
}

/// The style for a Jump to File row, colored by whether it is highlighted.
///
/// `highlighted` marks the row Enter or `Jump` plays: it takes the browse
/// selection style — the blue [`theme::ROW_HIGHLIGHT`] bar behind white
/// [`theme::PLAYING_ROW_TEXT`] — while every other row keeps the flat green
/// playlist-row colors. Pure, like `super::browse::rows::browse_row_style`, so
/// the colors are testable without a renderer.
pub(crate) fn jump_file_row_style(
    highlighted: bool,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    super::browse::rows::browse_row_style(status, false, highlighted)
}

/// Stacks the transient Jump to File panel over `base`.
///
/// The panel is modal (see [`modal_overlay`]): `Close` and the backdrop each
/// publish [`Message::ToggleJumpFile`], while `Jump` and Enter in the box
/// publish [`Message::JumpFileSubmitted`]. Each matching row publishes
/// [`Message::JumpFileRowSelected`] with the `epoch` of the list that supplied
/// `titles` and its source index. Every row is numbered with its source
/// position by [`jump_file_row_label`]; the row [`jump_file_highlighted`]
/// resolves from `selected` — the first match until the user moves it — is
/// the row Enter or `Jump` plays and carries the browse selection style
/// through [`jump_file_row_style`]. An empty list reads `No songs`; a list
/// with no match reads `No matching songs`. No state or playback is touched
/// here.
pub fn view_jump_file_overlay<'a>(
    base: Element<'a, Message>,
    query: &'a str,
    titles: &'a [String],
    epoch: u64,
    selected: Option<usize>,
) -> Element<'a, Message> {
    let matches = jump_file_matches(titles, query);
    let highlighted = jump_file_highlighted(&matches, selected);
    let has_matches = highlighted.is_some();
    let mut rows = Column::new().spacing(2).padding(4);
    for index in matches {
        let label = jump_file_row_label(index, &titles[index]);
        rows = rows.push(
            Button::new(Text::new(label).size(18))
                .width(Length::Fill)
                .on_press(Message::JumpFileRowSelected { epoch, index })
                .style(move |_theme, status| {
                    jump_file_row_style(highlighted == Some(index), status)
                }),
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
