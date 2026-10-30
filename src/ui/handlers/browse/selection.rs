//! The browse row presses: the artist, album, playlist, flat row, and
//! in-memory group (Genre, Year, Composer) selections that step down the
//! library's listing hierarchy, plus the shared `show_group_songs` transition.
//! Two of the same presses do one thing: `select_row` only decides whether the
//! row is already the selected one, so a second press opens it.
//!
//! The fetches these handlers issue and the `*Loaded`/`*LoadFailed` replies
//! that fill the lists live in [`super::fetch`]; the flat Artist-view lists
//! (All Songs, All Albums, Recently Played, Recently Added, Playlists, Genres,
//! Years, Composers) in [`super::lists`]; and the row-order edits (Sort,
//! Reverse, Randomize, the Alt+Up/Down move) in [`super::ordering`].

use iced::Task;

use crate::library::Song;
use crate::ui::browse::BrowseList;
use crate::ui::loading::fetch_level;
use crate::ui::{CurrentView, Message, SAVED_PLAYLIST_ROW_ID, SongsOrigin, WinampPlayer};

use super::super::save_list::user_playlist_selected;
use super::{fetch, fetch_at_top, scroll, switch_view};

/// human-readable name, and the id the request used. The id alone told the user
/// nothing when the browse panel showed the report; the name says which row
/// they opened, while the id keeps the exact query in the log. Both are
/// formatted with `Debug`, so a control character in a remote-supplied name or
/// id is escaped instead of reaching the panel or terminal raw.
fn fetch_context(action: &str, name: &str, id: &str) -> String {
    format!("{action} {name:?} (id {id:?})")
}

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
    let Some(artist) = player.artists.select(epoch, index) else {
        return Task::none();
    };
    let context = fetch_context("loading albums for artist", &artist.name, &artist.id);
    let artist_id = artist.id.clone();
    switch_view(player, CurrentView::Albums);
    // Drop the previous artist's albums before the new fetch lands:
    // otherwise the Albums view renders them, and a press during the
    // fetch resolves against the wrong artist's list.
    fetch_at_top(fetch_level(
        &mut player.albums,
        &player.apple_music_service,
        context,
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
    let from_recently_added = player.current_view == CurrentView::RecentlyAdded;
    let selected = if from_all_albums {
        player.all_albums.select(epoch, index)
    } else if from_recently_added {
        player.recently_added.select(epoch, index)
    } else {
        player.albums.select(epoch, index)
    };
    let Some(album) = selected else {
        return Task::none();
    };
    let context = fetch_context(
        "loading songs from album",
        &album.display_title(),
        &album.id,
    );
    let album_id = album.id.clone();
    player.songs_origin = if from_all_albums {
        SongsOrigin::AllAlbums
    } else if from_recently_added {
        SongsOrigin::RecentlyAdded
    } else {
        SongsOrigin::Album
    };
    switch_view(player, CurrentView::Songs);
    // The songs twin of the artist handler above: clear the previous
    // album's songs so they cannot be shown or pressed while the new
    // album's fetch is in flight.
    fetch_at_top(fetch_level(
        &mut player.songs,
        &player.apple_music_service,
        context,
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
        CurrentView::RecentlyAdded => player.recently_added.select_row(epoch, index),
        CurrentView::RecentlyPlayed => player.recently_played.select_row(epoch, index),
        CurrentView::Playlists => player.playlists.select_row(epoch, index),
        CurrentView::Genres => player.genres.select_row(epoch, index),
        CurrentView::Years => player.years.select_row(epoch, index),
        CurrentView::Composers => player.composers.select_row(epoch, index),
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
        CurrentView::RecentlyAdded => player
            .recently_added
            .selected()
            .map(|index| (player.recently_added.epoch, index)),
        CurrentView::RecentlyPlayed => player
            .recently_played
            .selected()
            .map(|index| (player.recently_played.epoch, index)),
        CurrentView::Playlists => player
            .playlists
            .selected()
            .map(|index| (player.playlists.epoch, index)),
        CurrentView::Genres => player
            .genres
            .selected()
            .map(|index| (player.genres.epoch, index)),
        CurrentView::Years => player
            .years
            .selected()
            .map(|index| (player.years.epoch, index)),
        CurrentView::Composers => player
            .composers
            .selected()
            .map(|index| (player.composers.epoch, index)),
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
        CurrentView::Albums | CurrentView::AllAlbums | CurrentView::RecentlyAdded => {
            Task::done(Message::AlbumSelected { epoch, index })
        }
        CurrentView::Songs | CurrentView::AllSongs | CurrentView::RecentlyPlayed => {
            Task::done(Message::TrackSelected { epoch, index })
        }
        CurrentView::Playlists => Task::done(Message::PlaylistSelected { epoch, index }),
        CurrentView::Genres => Task::done(Message::GenreSelected { epoch, index }),
        CurrentView::Years => Task::done(Message::YearSelected { epoch, index }),
        CurrentView::Composers => Task::done(Message::ComposerSelected { epoch, index }),
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
    let Some(playlist) = player.playlists.select(epoch, index) else {
        return Task::none();
    };
    if playlist.id == SAVED_PLAYLIST_ROW_ID {
        // A saved song list's sentinel row: open the saved list the row
        // names instead of querying the server for the sentinel id.
        let name = playlist.name.clone();
        return user_playlist_selected(player, &name);
    }
    let context = fetch_context("loading songs from playlist", &playlist.name, &playlist.id);
    let playlist_id = playlist.id.clone();
    player.songs_origin = SongsOrigin::Playlist;
    switch_view(player, CurrentView::Songs);
    // Clear the previous list before the fetch lands, as the album handler
    // does, so a press during the fetch cannot resolve against stale
    // rows.
    fetch_at_top(fetch_level(
        &mut player.songs,
        &player.apple_music_service,
        context,
        move |service| async move { service.get_songs_from_playlist(&playlist_id).await },
        Message::SongsLoaded,
        Message::SongsLoadFailed,
    ))
}

/// Show an in-memory song group (a genre's or a year's) in the shared songs
/// buffer and record its origin for Back. Starting a songs fetch generation
/// makes an in-flight album or playlist reply stale.
pub(in crate::ui) fn show_group_songs(
    player: &mut WinampPlayer,
    songs: Vec<Song>,
    origin: SongsOrigin,
) -> Task<Message> {
    player.songs.begin_fetch();
    fetch::store_songs(&mut player.songs, songs);
    player.songs_origin = origin;
    switch_view(player, CurrentView::Songs);
    scroll::scroll_browse_to_top()
}

/// A genre's row was opened: show the genre's songs, already grouped in
/// memory, in the shared songs buffer and record the origin for Back. A stale
/// epoch or out-of-range index is rejected like the playlist twin's. Starting
/// a songs fetch generation makes an in-flight album or playlist reply stale.
pub(in crate::ui) fn genre_selected(
    player: &mut WinampPlayer,
    epoch: u64,
    index: usize,
) -> Task<Message> {
    let Some(genre) = player.genres.select(epoch, index) else {
        return Task::none();
    };
    let songs = genre.songs.clone();
    show_group_songs(player, songs, SongsOrigin::Genre)
}

/// A year's row was opened: the Years twin of [`genre_selected`], showing the
/// year's songs in the shared songs buffer with `SongsOrigin::Year` for Back.
pub(in crate::ui) fn year_selected(
    player: &mut WinampPlayer,
    epoch: u64,
    index: usize,
) -> Task<Message> {
    let Some(year) = player.years.select(epoch, index) else {
        return Task::none();
    };
    let songs = year.songs.clone();
    show_group_songs(player, songs, SongsOrigin::Year)
}

/// A composer's row was opened: the Composers twin of [`year_selected`],
/// showing the composer's songs in the shared songs buffer with
/// `SongsOrigin::Composer` for Back.
pub(in crate::ui) fn composer_selected(
    player: &mut WinampPlayer,
    epoch: u64,
    index: usize,
) -> Task<Message> {
    let Some(composer) = player.composers.select(epoch, index) else {
        return Task::none();
    };
    let songs = composer.songs.clone();
    show_group_songs(player, songs, SongsOrigin::Composer)
}

/// Ctrl+Up or Ctrl+Down was pressed: step the showing list's selection bar
/// one row up or down (`down`), Winamp's playlist editor's selection keys,
/// beside [`super::ordering::move_selected_row`]'s Alt row move. The rows
/// stay where they are and no epoch is bumped — a selection step names the
/// showing list itself, so there is no stale index to reject — and every
/// list, view, search state, and playback state the showing one is left
/// alone. A step that changes the selection scrolls the list only as far as
/// needed to show it, like the row moves; a no-op step (nothing selected, an
/// edge repeat, or a loading or failed list) schedules no task.
pub(in crate::ui) fn move_browse_selection(player: &mut WinampPlayer, down: bool) -> Task<Message> {
    let moved_to = match player.current_view {
        CurrentView::Artists => step_in(&mut player.artists, down),
        CurrentView::Albums => step_in(&mut player.albums, down),
        CurrentView::Songs => step_in(&mut player.songs, down),
        CurrentView::AllSongs => step_in(&mut player.all_songs, down),
        CurrentView::AllAlbums => step_in(&mut player.all_albums, down),
        CurrentView::RecentlyAdded => step_in(&mut player.recently_added, down),
        CurrentView::RecentlyPlayed => step_in(&mut player.recently_played, down),
        CurrentView::Playlists => step_in(&mut player.playlists, down),
        CurrentView::Genres => step_in(&mut player.genres, down),
        CurrentView::Years => step_in(&mut player.years, down),
        CurrentView::Composers => step_in(&mut player.composers, down),
    };
    moved_to.map_or_else(Task::none, scroll::reveal_browse_row)
}

/// Steps `list`'s selection one row and returns its new index, or `None`
/// when the selection did not change (`BrowseList::move_selection`'s
/// no-op rules).
fn step_in<T: Clone>(list: &mut BrowseList<T>, down: bool) -> Option<usize> {
    list.move_selection(down)
}
