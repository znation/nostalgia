//! The transient Jump to File panel: a modal overlay over the showing song
//! list with a filter box and one row per matching title.

use iced::{
    Element, Length,
    widget::{Button, Column, Scrollable, Text, TextInput},
};

use crate::ui::{Message, style, theme};

use super::browse::rows::LazyRow;
use super::{labeled_button, modal::modal_overlay};

/// The Jump to File box's widget id, so `J` can focus it on opening.
pub const JUMP_FILE_INPUT_ID: &str = "nostalgia-jump-file";

/// The Jump to File match list's widget id, so a highlight move can reveal
/// its row.
pub const JUMP_FILE_LIST_ID: &str = "nostalgia-jump-file-list";

/// A Jump to File row's drawn height, matching the label's vertical button
/// padding so the reveal arithmetic follows the list's real row pitch.
pub const JUMP_FILE_ROW_HEIGHT: f32 = 32.0;

/// The vertical gap between two Jump to File rows.
pub const JUMP_FILE_ROW_SPACING: f32 = 2.0;

/// The match list's padding on every side; its top edge offsets the first
/// row from the scrollable's top.
pub const JUMP_FILE_LIST_PADDING: f32 = 4.0;

/// The match list's fixed height, which a page of matches fits without
/// clipping a row.
pub const JUMP_FILE_LIST_HEIGHT: f32 = 180.0;

/// How many matches Page Up and Page Down move the highlight by: the rows
/// that fit the fixed-height match list.
pub const JUMP_FILE_PAGE_ROWS: isize = 5;

/// The source indices of the `titles` matching `query`, in list order.
///
/// The query is split on whitespace and an index matches when its title
/// contains every word as a case-insensitive substring, in any order; a blank
/// query has no words and matches every index. The view and the submit handler both call
/// this one function, so the rows shown and the row Enter plays cannot drift.
pub(crate) fn jump_file_matches(titles: &[String], query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    let words: Vec<&str> = query.split_whitespace().collect();
    titles
        .iter()
        .enumerate()
        .filter(|(_, title)| {
            let title = title.to_lowercase();
            words.iter().all(|word| title.contains(word))
        })
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

// Counts match-list builds on the current test thread, so a test can prove
// the panel reuses its built rows across view refreshes.
#[cfg(test)]
thread_local! {
    static MATCH_LIST_BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many Jump to File match lists the current test thread has built.
#[cfg(test)]
pub(super) fn match_list_build_count() -> usize {
    MATCH_LIST_BUILDS.with(std::cell::Cell::get)
}

/// The cached match list's dependency: everything that changes the rows it
/// builds. The list reuses its built buttons while these hash the same, so a
/// view rebuild driven by a playback poll or marquee tick does not re-filter
/// the whole song list or rebuild every match row. Production lists bump
/// `epoch` whenever their titles change (see `BrowseList`); `title_count`
/// keeps the empty-list wording honest for any caller that does not.
#[derive(Clone, Copy, Hash)]
struct MatchListKey {
    epoch: u64,
    query: u64,
    title_count: usize,
    selected: Option<usize>,
}

/// A stable hash of the typed query, so an unchanged filter reuses the built
/// rows without comparing (or re-filtering) the whole title list.
fn query_hash(query: &str) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    query.hash(&mut hasher);
    hasher.finish()
}

/// Builds one row per title matching `query`, numbered by its source position,
/// with the row [`jump_file_highlighted`] resolves from `key.selected` drawn
/// highlighted. Called by the row cache only when [`MatchListKey`] changes, so
/// the labels are formatted once per filter or highlight change rather than
/// once per view refresh.
fn match_rows(query: &str, titles: &[String], key: &MatchListKey) -> Column<'static, Message> {
    #[cfg(test)]
    MATCH_LIST_BUILDS.with(|count| count.set(count.get() + 1));

    let matches = jump_file_matches(titles, query);
    let highlighted = jump_file_highlighted(&matches, key.selected);
    let mut rows = Column::new()
        .spacing(JUMP_FILE_ROW_SPACING)
        .padding(JUMP_FILE_LIST_PADDING);
    for index in matches {
        let label = jump_file_row_label(index, &titles[index]);
        rows = rows.push(
            Button::new(Text::new(label).size(18))
                .width(Length::Fill)
                .height(Length::Fixed(JUMP_FILE_ROW_HEIGHT))
                .on_press(Message::JumpFileRowSelected {
                    epoch: key.epoch,
                    index,
                })
                .style(move |_theme, status| {
                    jump_file_row_style(highlighted == Some(index), status)
                }),
        );
    }
    if titles.is_empty() {
        rows = rows.push(Text::new("No songs").size(18).color(theme::TEXT));
    } else if highlighted.is_none() {
        rows = rows.push(Text::new("No matching songs").size(18).color(theme::TEXT));
    }
    rows
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
///
/// The rows are wrapped in a [`LazyRow`] keyed by [`MatchListKey`], so the
/// panel's own rows — the heaviest part of this overlay — are reused across
/// view refreshes whose filter, list epoch, and highlight are unchanged.
pub fn view_jump_file_overlay<'a>(
    base: Element<'a, Message>,
    query: &'a str,
    titles: &'a [String],
    epoch: u64,
    selected: Option<usize>,
) -> Element<'a, Message> {
    let rows = LazyRow::new(
        MatchListKey {
            epoch,
            query: query_hash(query),
            title_count: titles.len(),
            selected,
        },
        move |key| match_rows(query, titles, key),
    );

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
                .id(JUMP_FILE_LIST_ID)
                .width(Length::Fill)
                .height(Length::Fixed(JUMP_FILE_LIST_HEIGHT))
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
