//! The browse message handlers: the artist → album → song hierarchy, the
//! flat whole-library lists, and the Back, Open, Top, Sort, and Reload
//! navigation over every level. The row presses live in [`selection`], the
//! Artist-view lists and Back in [`lists`], and the row-order edits in
//! [`ordering`]; the initial load and the fetch results that fill the lists
//! in [`fetch`], the list's scroll operations in [`scroll`], and the library
//! search and its Clear button in [`search`]. The two transitions every
//! replaced list goes through live here: the switch that leaves the search
//! behind, and the scroll that follows each replaced list.

use iced::Task;

use crate::ui::{CurrentView, Message, WinampPlayer};

pub(in crate::ui) mod fetch;
pub(in crate::ui) mod lists;
pub(in crate::ui) mod ordering;
pub(in crate::ui) mod scroll;
pub(in crate::ui) mod search;
pub(in crate::ui) mod selection;

/// active search behind, and closes Jump to File. A replaced list's rows must
/// not stay reachable through Back or the panel, and the search's songs are
/// about to be replaced, so neither can still be treated as current.
pub(super) fn switch_view(player: &mut WinampPlayer, view: CurrentView) {
    player.search_active = false;
    player.current_view = view;
    player.jump_file_visible = false;
}

/// Navigation replaces the visible list, so leaving the new level at the
/// previous list's scroll offset would hide its first rows; batching the
/// scroll with the fetch keeps the two one operation from the update loop.
pub(super) fn fetch_at_top(task: Task<Message>) -> Task<Message> {
    Task::batch([task, scroll::scroll_browse_to_top()])
}
