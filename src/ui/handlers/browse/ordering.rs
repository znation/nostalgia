//! The browse row-order edits: Sort, Reverse (Ctrl+R), Randomize
//! (Ctrl+Shift+R), the Delete selected-row removal, and the Alt+Up/Down
//! selected-row move. Each is one-shot and local to the list the browse view
//! shows: it bumps that list's epoch (a press rendered before the edit is
//! rejected) and leaves every other list, the current view, the search state,
//! and playback alone. Returning to an edited parent restores that list's
//! library order, and reopening a list fetches it afresh, so no edit mode
//! persists — except the removal, which is destructive: a refetch or
//! reopening rebuilds the removed row from the library.

use iced::Task;

use crate::ui::browse::BrowseList;
use crate::ui::{CurrentView, Message, WinampPlayer};

use super::scroll;

/// list alphabetically by each row's rendered text, case-insensitively.
///
/// The reorder is one-shot and local to the visible list: it bumps that list's
/// epoch (so a press rendered before the sort is rejected) but leaves every
/// other list, the current view, the search state, and playback alone. Returning
/// to a sorted parent restores that list's library order, and reopening a list
/// fetches it afresh, so no sort mode persists.
pub(in crate::ui) fn sort_browse(player: &mut WinampPlayer) -> Task<Message> {
    match player.current_view {
        CurrentView::Artists => player.artists.sort_rows_by(|artist| artist.name.clone()),
        CurrentView::Albums => player
            .albums
            .sort_rows_by(|album| album.display_title().into_owned()),
        CurrentView::Songs => player
            .songs
            .sort_rows_by(|song| song.display_title().into_owned()),
        CurrentView::AllSongs => player
            .all_songs
            .sort_rows_by(|song| song.display_title().into_owned()),
        CurrentView::AllAlbums => player
            .all_albums
            .sort_rows_by(|album| album.display_title().into_owned()),
        CurrentView::RecentlyAdded => player
            .recently_added
            .sort_rows_by(|album| album.display_title().into_owned()),
        CurrentView::RecentlyPlayed => player
            .recently_played
            .sort_rows_by(|song| song.display_title().into_owned()),
        CurrentView::Playlists => player
            .playlists
            .sort_rows_by(|playlist| playlist.name.clone()),
        CurrentView::Genres => player.genres.sort_rows_by(|genre| genre.name.clone()),
        CurrentView::Years => player.years.sort_rows_by(|year| year.name.clone()),
        CurrentView::Composers => player
            .composers
            .sort_rows_by(|composer| composer.name.clone()),
    }
    Task::none()
}

/// Ctrl+R was pressed: reverse the currently showing list's order.
///
/// Like [`sort_browse`] the reorder is one-shot and local to the visible list:
/// it bumps that list's epoch and clears its selection, and leaves every other
/// list, the current view, the search state, and playback alone. Returning to
/// a reversed parent restores its library order.
pub(in crate::ui) fn reverse_browse(player: &mut WinampPlayer) -> Task<Message> {
    match player.current_view {
        CurrentView::Artists => player.artists.reverse_rows(),
        CurrentView::Albums => player.albums.reverse_rows(),
        CurrentView::Songs => player.songs.reverse_rows(),
        CurrentView::AllSongs => player.all_songs.reverse_rows(),
        CurrentView::AllAlbums => player.all_albums.reverse_rows(),
        CurrentView::RecentlyAdded => player.recently_added.reverse_rows(),
        CurrentView::RecentlyPlayed => player.recently_played.reverse_rows(),
        CurrentView::Playlists => player.playlists.reverse_rows(),
        CurrentView::Genres => player.genres.reverse_rows(),
        CurrentView::Years => player.years.reverse_rows(),
        CurrentView::Composers => player.composers.reverse_rows(),
    }
    Task::none()
}

/// Ctrl+Shift+R was pressed: shuffle the currently showing list's order.
///
/// Like [`reverse_browse`] the reorder is one-shot and local to the visible
/// list, and returning to a shuffled parent restores its library order. One
/// [`crate::ui::transport::shuffle_roll`] seeds the permutation.
pub(in crate::ui) fn randomize_browse(player: &mut WinampPlayer) -> Task<Message> {
    let seed = crate::ui::transport::shuffle_roll();
    match player.current_view {
        CurrentView::Artists => player.artists.shuffle_rows(seed),
        CurrentView::Albums => player.albums.shuffle_rows(seed),
        CurrentView::Songs => player.songs.shuffle_rows(seed),
        CurrentView::AllSongs => player.all_songs.shuffle_rows(seed),
        CurrentView::AllAlbums => player.all_albums.shuffle_rows(seed),
        CurrentView::RecentlyAdded => player.recently_added.shuffle_rows(seed),
        CurrentView::RecentlyPlayed => player.recently_played.shuffle_rows(seed),
        CurrentView::Playlists => player.playlists.shuffle_rows(seed),
        CurrentView::Genres => player.genres.shuffle_rows(seed),
        CurrentView::Years => player.years.shuffle_rows(seed),
        CurrentView::Composers => player.composers.shuffle_rows(seed),
    }
    Task::none()
}

/// Delete was pressed: remove the showing songs list's selected row.
///
/// Like [`reverse_browse`] the edit is one-shot and local to the visible list:
/// it bumps that list's epoch and clears its selection, and leaves every other
/// list, the current view, the search state, playback, and the played history
/// alone (the playing track is not named by any browse-list index). Unlike
/// [`randomize_browse`] it is restricted to `CurrentView::Songs`: the songs
/// view is Winamp's playlist-shaped artifact the row edits compose on, and a
/// deletion on the artists, albums, or the other song lists would cut a row
/// the user did not compose — so every other view is a no-op there, like
/// Sort is a reorder rather than a deletion outside the songs buffer.
/// Nothing persists; a refetch or reopening the list
/// rebuilds the rows from the library.
pub(in crate::ui) fn remove_selected(player: &mut WinampPlayer) -> Task<Message> {
    if player.current_view == CurrentView::Songs {
        player.songs.remove_selected_row();
    }
    Task::none()
}

/// Ctrl+Shift+Delete was pressed: clear the showing songs list.
///
/// Winamp's playlist editor's "Remove all" beside [`remove_selected`]'s
/// "Remove selected": like [`remove_selected`] the edit is one-shot and local
/// to the visible list, bumps its epoch, and is restricted to
/// `CurrentView::Songs` so the artists, albums, and the other song lists can
/// never be emptied, and playback, the played history, and the saved
/// playlists are left alone (the playing track is not named by any
/// browse-list index). Nothing persists; a refetch or reopening the list
/// rebuilds the rows.
pub(in crate::ui) fn remove_all(player: &mut WinampPlayer) -> Task<Message> {
    if player.current_view == CurrentView::Songs {
        player.songs.remove_all_rows();
    }
    Task::none()
}

/// Alt+Up or Alt+Down was pressed: move the showing list's selected row one
/// place up or down (`down`). Like [`reverse_browse`] the reorder is one-shot
/// and local to the visible list, and returning to a moved parent restores its
/// library order; the row stays selected so repeated presses keep moving it.
/// A move that changes the selection scrolls the list only as far as needed
/// to show the moved row; a no-op move schedules no task.
pub(in crate::ui) fn move_selected_row(player: &mut WinampPlayer, down: bool) -> Task<Message> {
    let moved_to = match player.current_view {
        CurrentView::Artists => move_in(&mut player.artists, down),
        CurrentView::Albums => move_in(&mut player.albums, down),
        CurrentView::Songs => move_in(&mut player.songs, down),
        CurrentView::AllSongs => move_in(&mut player.all_songs, down),
        CurrentView::AllAlbums => move_in(&mut player.all_albums, down),
        CurrentView::RecentlyAdded => move_in(&mut player.recently_added, down),
        CurrentView::RecentlyPlayed => move_in(&mut player.recently_played, down),
        CurrentView::Playlists => move_in(&mut player.playlists, down),
        CurrentView::Genres => move_in(&mut player.genres, down),
        CurrentView::Years => move_in(&mut player.years, down),
        CurrentView::Composers => move_in(&mut player.composers, down),
    };
    moved_to.map_or_else(Task::none, scroll::reveal_browse_row)
}

/// Moves `list`'s selected row and returns its new index, or `None` when the
/// selection did not change (nothing selected, an edge, or a loading or
/// failed list).
fn move_in<T: Clone>(list: &mut BrowseList<T>, down: bool) -> Option<usize> {
    let before = list.selected();
    list.move_selected(down);
    let after = list.selected();
    if after == before { None } else { after }
}
