//! The browse message handlers: the artist → album → song hierarchy, the flat
//! whole-library lists, and the Back, Open, Top, and Sort navigation over every
//! level, plus the library search. The initial load and the fetch results that
//! fill the lists live in the [`fetch`] submodule.
//!
//! Every fetch clears the list it refills before it goes out, so a press
//! during the fetch cannot resolve against the previous level's rows, and
//! every stored reply re-checks the request generation that issued it, so a
//! superseded reply cannot overwrite a newer list.

use iced::Task;

use crate::ui::loading::fetch_level;
use crate::ui::views;
use crate::ui::{CurrentView, Message, SongsOrigin, WinampPlayer};

pub(in crate::ui) mod fetch;

/// An artist's row was pressed: step down to the Albums level and fetch that
/// artist's albums. As with `TrackSelected`, the row message carries the
/// artist's index and its list's epoch. A mismatched epoch is a stale press
/// from a replaced list; an out-of-range index has no artist to browse.
/// `artists.select` rejects both.
pub(in crate::ui) fn artist_selected(
    player: &mut WinampPlayer,
    epoch: u64,
    index: usize,
) -> Task<Message> {
    let Some(artist_id) = player
        .artists
        .select(epoch, index)
        .map(|artist| artist.id.clone())
    else {
        return Task::none();
    };
    player.current_view = CurrentView::Albums;
    // Entering a browse level leaves any active search: the songs list
    // the search filled is about to be replaced, so Back must not
    // treat it as a searched list any more.
    player.search_active = false;
    // Drop the previous artist's albums before the new fetch lands:
    // otherwise the Albums view renders them, and a press during the
    // fetch resolves against the wrong artist's list.
    fetch_at_top(fetch_level(
        &mut player.albums,
        &player.apple_music_service,
        format!("loading albums for artist {artist_id:?}"),
        move |service| async move { service.get_albums_by_artist(&artist_id).await },
        Message::AlbumsLoaded,
        Message::AlbumsLoadFailed,
    ))
}

/// An album's row was pressed: step down to the Songs level and fetch that
/// album's songs. The album twin of the artist handler above. The Songs view
/// can be opened from the per-artist Albums list or from the flat All
/// Albums list, so resolve the press against the list that view rendered;
/// `select` rejects the same stale-epoch and out-of-range presses either
/// way. The origin is recorded from the same view, so Back later returns to
/// the level the row came from.
pub(in crate::ui) fn album_selected(
    player: &mut WinampPlayer,
    epoch: u64,
    index: usize,
) -> Task<Message> {
    let from_all_albums = player.current_view == CurrentView::AllAlbums;
    let selected = if from_all_albums {
        player.all_albums.select(epoch, index)
    } else {
        player.albums.select(epoch, index)
    };
    let Some(album_id) = selected.map(|album| album.id.clone()) else {
        return Task::none();
    };
    player.songs_origin = if from_all_albums {
        SongsOrigin::AllAlbums
    } else {
        SongsOrigin::Album
    };
    player.current_view = CurrentView::Songs;
    player.search_active = false;
    // The songs twin of the artist handler above: clear the previous
    // album's songs so they cannot be shown or pressed while the new
    // album's fetch is in flight.
    fetch_at_top(fetch_level(
        &mut player.songs,
        &player.apple_music_service,
        format!("loading songs from album {album_id:?}"),
        move |service| async move { service.get_songs_from_album(&album_id).await },
        Message::SongsLoaded,
        Message::SongsLoadFailed,
    ))
}

/// A browse row was pressed once. Select it in the showing list; only a
/// second press on the row that is already selected opens it, through
/// [`open_row`]. A stale epoch or out-of-range index is rejected by
/// `BrowseList::select_row` and leaves the selection alone, and a first press
/// schedules no fetch, playback, or view change.
pub(in crate::ui) fn select_row(
    player: &mut WinampPlayer,
    epoch: u64,
    index: usize,
) -> Task<Message> {
    let already_selected = match player.current_view {
        CurrentView::Artists => player.artists.select_row(epoch, index),
        CurrentView::Albums => player.albums.select_row(epoch, index),
        CurrentView::Songs => player.songs.select_row(epoch, index),
        CurrentView::AllSongs => player.all_songs.select_row(epoch, index),
        CurrentView::AllAlbums => player.all_albums.select_row(epoch, index),
        CurrentView::Playlists => player.playlists.select_row(epoch, index),
    };
    if already_selected == Some(true) {
        open_row(player, epoch, index)
    } else {
        Task::none()
    }
}

/// The browse view's Open button: open the row the showing list has selected
/// through the same message a second press dispatches. Nothing selected means
/// nothing to open, so the press is a no-op that schedules no task.
pub(in crate::ui) fn open_selected(player: &mut WinampPlayer) -> Task<Message> {
    let selection = match player.current_view {
        CurrentView::Artists => player
            .artists
            .selected()
            .map(|index| (player.artists.epoch, index)),
        CurrentView::Albums => player
            .albums
            .selected()
            .map(|index| (player.albums.epoch, index)),
        CurrentView::Songs => player
            .songs
            .selected()
            .map(|index| (player.songs.epoch, index)),
        CurrentView::AllSongs => player
            .all_songs
            .selected()
            .map(|index| (player.all_songs.epoch, index)),
        CurrentView::AllAlbums => player
            .all_albums
            .selected()
            .map(|index| (player.all_albums.epoch, index)),
        CurrentView::Playlists => player
            .playlists
            .selected()
            .map(|index| (player.playlists.epoch, index)),
    };
    let Some((epoch, index)) = selection else {
        return Task::none();
    };
    open_row(player, epoch, index)
}

/// Dispatches the existing open action for the showing list: the artist,
/// album, track, or playlist selection message the old single row press used
/// to emit directly. Those handlers keep their epoch/index staleness checks,
/// fetch clearing, and playback path, so selecting a row first changes only
/// when the action fires, not what it does.
fn open_row(player: &WinampPlayer, epoch: u64, index: usize) -> Task<Message> {
    match player.current_view {
        CurrentView::Artists => Task::done(Message::ArtistSelected { epoch, index }),
        CurrentView::Albums | CurrentView::AllAlbums => {
            Task::done(Message::AlbumSelected { epoch, index })
        }
        CurrentView::Songs | CurrentView::AllSongs => {
            Task::done(Message::TrackSelected { epoch, index })
        }
        CurrentView::Playlists => Task::done(Message::PlaylistSelected { epoch, index }),
    }
}

/// A playlist's row was pressed: load the playlist's songs into the shared
/// songs buffer and record the origin for Back. The playlist twin of the
/// album handler above: resolve the press against the Playlists list that
/// rendered it, rejecting a stale epoch or out-of-range index.
pub(in crate::ui) fn playlist_selected(
    player: &mut WinampPlayer,
    epoch: u64,
    index: usize,
) -> Task<Message> {
    let Some(playlist_id) = player
        .playlists
        .select(epoch, index)
        .map(|playlist| playlist.id.clone())
    else {
        return Task::none();
    };
    player.songs_origin = SongsOrigin::Playlist;
    player.current_view = CurrentView::Songs;
    player.search_active = false;
    // Clear the previous list before the fetch lands, as the album handler
    // does, so a press during the fetch cannot resolve against stale
    // rows.
    fetch_at_top(fetch_level(
        &mut player.songs,
        &player.apple_music_service,
        format!("loading songs from playlist {playlist_id:?}"),
        move |service| async move { service.get_songs_from_playlist(&playlist_id).await },
        Message::SongsLoaded,
        Message::SongsLoadFailed,
    ))
}

/// Step back up the browse hierarchy: the browse hierarchy is navigable both
/// ways — `ArtistSelected`, `AlbumSelected`, and `PlaylistSelected` step down
/// (Artists → Albums → Songs, or into a playlist's songs), Back steps up
/// again. The Back button is only rendered below the artist list, so this
/// handler mostly fires where a level exists to leave; the explicit
/// top-level case keeps the no-op in the same place as the steps.
pub(in crate::ui) fn back(player: &mut WinampPlayer) -> Task<Message> {
    // Only stepping up replaces the visible list, so Back at the top
    // level stays a true no-op (the invariant docs pin that) while a
    // restored list starts at its first row.
    let stepping_up = player.search_active || player.current_view != CurrentView::Artists;
    // A search result list sits under the artists list, not under the
    // album it was issued from: leaving it returns straight to
    // Artists, and the album list it would otherwise open is empty or
    // stale. A normal browse steps up one level.
    if player.search_active {
        player.search_active = false;
        player.current_view = CurrentView::Artists;
    } else {
        player.current_view = match &player.current_view {
            CurrentView::Songs => match player.songs_origin {
                SongsOrigin::Album => CurrentView::Albums,
                SongsOrigin::AllAlbums => CurrentView::AllAlbums,
                SongsOrigin::Playlist => CurrentView::Playlists,
            },
            CurrentView::Albums => CurrentView::Artists,
            CurrentView::AllSongs => CurrentView::Artists,
            CurrentView::AllAlbums => CurrentView::Artists,
            CurrentView::Playlists => CurrentView::Artists,
            CurrentView::Artists => CurrentView::Artists,
        };
    }
    if stepping_up {
        // A parent list stays in memory while a child is open, so Back does
        // not refetch it. Sort is one-shot, so returning to it undoes any
        // sort and shows the library's order again.
        match player.current_view {
            CurrentView::Artists => player.artists.restore_library_order(),
            CurrentView::Albums => player.albums.restore_library_order(),
            CurrentView::AllAlbums => player.all_albums.restore_library_order(),
            CurrentView::Playlists => player.playlists.restore_library_order(),
            // Back never lands on a song list, and both are refetched when
            // reopened, so neither can be the list being restored.
            CurrentView::Songs | CurrentView::AllSongs => {}
        }
        scroll_browse_to_top()
    } else {
        Task::none()
    }
}

/// The browse view's Top button: snap the current list back to its first
/// row. The list carries `views::BROWSE_LIST_ID` whichever level is
/// showing, so one operation serves the Top button and every navigation that
/// opens a fresh list.
pub(in crate::ui) fn scroll_browse_to_top() -> Task<Message> {
    iced::widget::operation::snap_to(
        views::BROWSE_LIST_ID,
        iced::widget::scrollable::RelativeOffset::START,
    )
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

/// The Artists view's All Songs button: fetch the whole library flat and
/// show it under the All Songs view. The flat list hangs off the artists
/// list, so an active search's results are left behind and Back returns to
/// Artists.
pub(in crate::ui) fn show_all_songs(player: &mut WinampPlayer) -> Task<Message> {
    player.search_active = false;
    player.current_view = CurrentView::AllSongs;
    // Clear the previous flat list before the new fetch lands, so a
    // press during the fetch cannot resolve against stale rows (the
    // same clear-before-fetch the navigation handlers do).
    fetch_at_top(fetch_level(
        &mut player.all_songs,
        &player.apple_music_service,
        "loading all library songs".to_string(),
        |service| async move { service.get_all_songs().await },
        Message::AllSongsLoaded,
        Message::AllSongsLoadFailed,
    ))
}

/// The Artists view's All Albums button: fetch every library album flat and
/// show it under the All Albums view. Like All Songs, the flat album list
/// hangs off the artists list: leave an active search behind and let Back
/// return to Artists.
pub(in crate::ui) fn show_all_albums(player: &mut WinampPlayer) -> Task<Message> {
    player.search_active = false;
    player.current_view = CurrentView::AllAlbums;
    // Clear the previous flat list before the new fetch lands, so a
    // press during the fetch cannot resolve against stale rows.
    fetch_at_top(fetch_level(
        &mut player.all_albums,
        &player.apple_music_service,
        "loading all library albums".to_string(),
        |service| async move { service.get_all_albums().await },
        Message::AllAlbumsLoaded,
        Message::AllAlbumsLoadFailed,
    ))
}

/// The Artists view's Playlists button: fetch the library's playlists and
/// show them under the Playlists view. Like the flat lists, Playlists hangs
/// off the artists list: leave an active search behind and let Back return
/// to Artists.
pub(in crate::ui) fn show_playlists(player: &mut WinampPlayer) -> Task<Message> {
    player.search_active = false;
    player.current_view = CurrentView::Playlists;
    // Clear the previous list before the new fetch lands, so a press
    // during the fetch cannot resolve against stale rows.
    fetch_at_top(fetch_level(
        &mut player.playlists,
        &player.apple_music_service,
        "loading library playlists".to_string(),
        |service| async move { service.get_playlists().await },
        Message::PlaylistsLoaded,
        Message::PlaylistsLoadFailed,
    ))
}

/// The library search box: each keystroke is stored here and the search box
/// renders it back.
pub(in crate::ui) fn search_changed(player: &mut WinampPlayer, text: String) -> Task<Message> {
    player.search_query = text;
    Task::none()
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
    player.search_active = true;
    player.current_view = CurrentView::Songs;
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

/// The browse view's Sort button was pressed: reorder the currently showing
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
        CurrentView::Playlists => player
            .playlists
            .sort_rows_by(|playlist| playlist.name.clone()),
    }
    Task::none()
}

/// Runs a browse fetch and then snaps the list it fills to the top.
/// Navigation replaces the visible list, so leaving the new level at the
/// previous list's scroll offset would hide its first rows; batching the
/// scroll with the fetch keeps the two one operation from the update loop.
fn fetch_at_top(task: Task<Message>) -> Task<Message> {
    Task::batch([task, scroll_browse_to_top()])
}
