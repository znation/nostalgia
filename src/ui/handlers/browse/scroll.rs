//! The browse list's scroll operations: the Top/Home and End/End-key snaps,
//! the Page Up and Page Down steps, and the row and playing-row scrolls.
//!
//! Each handler is a pure `iced` scroll task against the list's stable widget
//! id. They touch only the list's scroll offset — never its rows, selection,
//! view, search state, or playback — so the navigation in the parent [`super`]
//! module and the keyboard shortcuts can share them without changing browse
//! state.

use iced::Task;

use crate::ui::views;
use crate::ui::{CurrentView, Message, WinampPlayer};

/// Snap the showing browse list to `offset` through its stable widget id.
/// The list carries `views::BROWSE_LIST_ID` whichever level is showing, so
/// one operation serves the Top button, the Home and End keys, and every
/// navigation that opens a fresh list.
fn snap_browse_list(offset: iced::widget::scrollable::RelativeOffset) -> Task<Message> {
    iced::widget::operation::snap_to(views::BROWSE_LIST_ID, offset)
}

/// The browse view's Top button and the Home key: snap the current list back
/// to its first row. This touches only the list's scroll offset; it changes
/// no rows, selection, view, search state, or playback.
pub(in crate::ui) fn scroll_browse_to_top() -> Task<Message> {
    snap_browse_list(iced::widget::scrollable::RelativeOffset::START)
}

/// The End key's counterpart to [`scroll_browse_to_top`]: snap the current
/// list to its last row. Like Home it touches only the list's scroll offset;
/// it changes no rows, selection, view, search state, or playback.
pub(in crate::ui) fn scroll_browse_to_bottom() -> Task<Message> {
    snap_browse_list(iced::widget::scrollable::RelativeOffset::END)
}

/// Scroll the showing browse list by `delta` vertical pixels through its
/// stable widget id. Unlike the absolute [`scroll_browse_to_row`], the step is
/// relative to the list's current offset. It touches only the scroll offset
/// and has no target in shade mode, where no browse list is built.
fn scroll_browse_by(delta: f32) -> Task<Message> {
    iced::widget::operation::scroll_by(
        views::BROWSE_LIST_ID,
        iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: delta },
    )
}

/// The Page Up key: scroll the current list back by one
/// [`views::BROWSE_PAGE_HEIGHT`], relative to where it sits now. Like Home and
/// End it changes no rows, selection, view, search state, or playback, and
/// iced clamps a page that would pass the list's top.
pub(in crate::ui) fn scroll_browse_page_up() -> Task<Message> {
    scroll_browse_by(-views::BROWSE_PAGE_HEIGHT)
}

/// The Page Down key's counterpart to [`scroll_browse_page_up`]: scroll the
/// current list forward by one [`views::BROWSE_PAGE_HEIGHT`], relative to
/// where it sits now. It likewise touches only the scroll offset, and iced
/// clamps a page that would pass the list's end.
pub(in crate::ui) fn scroll_browse_page_down() -> Task<Message> {
    scroll_browse_by(views::BROWSE_PAGE_HEIGHT)
}

/// Scroll the browse list so its row `index` starts at the viewport's top
/// edge. The absolute offset is the list's top padding plus the row's
/// laid-out pitch times its position; iced clamps it to the list's content,
/// so a row near the end scrolls only as far as the list allows and a short
/// list simply stays put.
pub(in crate::ui) fn scroll_browse_to_row(index: usize) -> Task<Message> {
    iced::widget::operation::scroll_to(
        views::BROWSE_LIST_ID,
        iced::widget::scrollable::AbsoluteOffset {
            x: None,
            y: Some(views::BROWSE_LIST_PADDING + index as f32 * views::BROWSE_ROW_HEIGHT),
        },
    )
}

/// Keep the stepped-to playing row visible: scroll the browse list to `index`
/// only while a song list (`Songs` or `AllSongs`) is showing, since those are
/// the levels whose rows carry the `▶` playing marker. Another browse level
/// leaves the list where it is, and so does a direct row press — the pressed
/// row is already in view — which deliberately does not call this.
pub(in crate::ui) fn scroll_browse_to_playing_row(
    player: &WinampPlayer,
    index: usize,
) -> Task<Message> {
    match player.current_view {
        CurrentView::Songs | CurrentView::AllSongs => scroll_browse_to_row(index),
        _ => Task::none(),
    }
}
