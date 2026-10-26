//! The library search box's handlers: each keystroke, Ctrl+F focus, the Clear
//! button, and the submitted query.
//!
//! The search replaces the shared songs buffer, so it uses the same
//! clear-before-fetch path as the browse navigation in the parent [`super`]
//! module and delegates its active Clear case to [`super::back`].

use iced::Task;

use super::{back, fetch_at_top, switch_view};
use crate::ui::loading::fetch_level;
use crate::ui::views;
use crate::ui::{CurrentView, Message, WinampPlayer};

/// The library search box: each keystroke is stored here and the search box
/// renders it back.
pub(in crate::ui) fn search_changed(player: &mut WinampPlayer, text: String) -> Task<Message> {
    player.search_query = text;
    Task::none()
}

/// Ctrl+F was pressed: move keyboard focus to the library search box through
/// its `views::BROWSE_SEARCH_ID` widget id. The handler mutates no state and
/// submits nothing; typing into the focused box then edits the query exactly
/// as a click does. In shade mode the box is not in the widget tree, so the
/// focus operation finds no matching focusable and the key is a no-op.
pub(in crate::ui) fn focus_search() -> Task<Message> {
    iced::widget::operation::focus(views::BROWSE_SEARCH_ID)
}

/// The Clear button beside the search box: empty the typed query and, when a
/// search is showing, leave it exactly as Back does.
///
/// Reusing [`back`] for the active case is what makes it identical to the
/// Back button: it deactivates the search, switches to Artists, restores the
/// artists list's library order, and snaps the list to its top. The inactive
/// case mutates only the query, so it schedules no fetch or scroll. Neither
/// case refetches or clears any browse rows, and neither touches playback.
pub(in crate::ui) fn clear_search(player: &mut WinampPlayer) -> Task<Message> {
    player.search_query.clear();
    if player.search_active {
        back(player)
    } else {
        Task::none()
    }
}

/// The library search box submitted: take the stored text when Enter is
/// pressed and replace the songs list with the search results.
pub(in crate::ui) fn search_submitted(player: &mut WinampPlayer) -> Task<Message> {
    // A blank query has nothing to match: `search_songs` rejects it,
    // and issuing the fetch would only turn the empty box into an
    // error report. Leave the current list and view alone.
    let query = player.search_query.trim().to_string();
    if query.is_empty() {
        return Task::none();
    }
    player.search_query = query.clone();
    // The search results replace the shared songs list, so switch to it as
    // any navigation does, then mark the list as the search's results.
    switch_view(player, CurrentView::Songs);
    player.search_active = true;
    // The search results reuse the songs buffer, so the previous list's
    // rows are dropped and a press during the fetch cannot resolve
    // against them (the same clear-before-fetch the navigation handlers do).
    fetch_at_top(fetch_level(
        &mut player.songs,
        &player.apple_music_service,
        format!("searching the library for {query:?}"),
        move |service| async move { service.search_songs(&query).await },
        Message::SongsLoaded,
        Message::SongsLoadFailed,
    ))
}
