//! The browse message handlers: the artist → album → song hierarchy, the flat
//! whole-library lists, and the Back, Open, Top, Sort, and Reload navigation
//! over every level. The initial load and the fetch results that fill the
//! lists live in the [`fetch`] submodule, the list's scroll operations in
//! [`scroll`], and the library search and its Clear button in [`search`].
//!
//! Every fetch clears the list it refills before it goes out, so a press
//! during the fetch cannot resolve against the previous level's rows, and
//! every stored reply re-checks the request generation that issued it, so a
//! superseded reply cannot overwrite a newer list.

use iced::Task;

use crate::library::Song;
use crate::settings::library_view::LibraryView;
use crate::ui::browse::BrowseList;
use crate::ui::composers::group_by_composer;
use crate::ui::genres::group_by_genre;
use crate::ui::loading::fetch_level;
use crate::ui::years::group_by_year;
use crate::ui::{CurrentView, Message, SongsOrigin, WinampPlayer};

pub(in crate::ui) mod fetch;
pub(in crate::ui) mod scroll;
pub(in crate::ui) mod search;

/// Switches the browse view from a navigation action: shows `view`, leaves any
/// active search behind, and closes Jump to File. A replaced list's rows must
/// not stay reachable through Back or the panel, and the search's songs are
/// about to be replaced, so neither can still be treated as current.
fn switch_view(player: &mut WinampPlayer, view: CurrentView) {
    player.search_active = false;
    player.current_view = view;
    player.jump_file_visible = false;
}

/// The context a browse fetch's failure report carries: the action, the row's
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
fn show_group_songs(
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
                SongsOrigin::RecentlyAdded => CurrentView::RecentlyAdded,
                SongsOrigin::Playlist => CurrentView::Playlists,
                SongsOrigin::Genre => CurrentView::Genres,
                SongsOrigin::Year => CurrentView::Years,
                SongsOrigin::Composer => CurrentView::Composers,
            },
            CurrentView::Albums => CurrentView::Artists,
            CurrentView::AllSongs => CurrentView::Artists,
            CurrentView::AllAlbums => CurrentView::Artists,
            CurrentView::RecentlyAdded => CurrentView::Artists,
            CurrentView::RecentlyPlayed => CurrentView::Artists,
            CurrentView::Playlists => CurrentView::Artists,
            CurrentView::Genres => CurrentView::Artists,
            CurrentView::Years => CurrentView::Artists,
            CurrentView::Composers => CurrentView::Artists,
            CurrentView::Artists => CurrentView::Artists,
        };
    }
    if stepping_up {
        player.jump_file_visible = false;
        // A parent list stays in memory while a child is open, so Back does
        // not refetch it. Sort is one-shot, so returning to it undoes any
        // sort and shows the library's order again.
        match player.current_view {
            CurrentView::Artists => player.artists.restore_library_order(),
            CurrentView::Albums => player.albums.restore_library_order(),
            CurrentView::AllAlbums => player.all_albums.restore_library_order(),
            CurrentView::RecentlyAdded => player.recently_added.restore_library_order(),
            CurrentView::Playlists => player.playlists.restore_library_order(),
            CurrentView::Genres => player.genres.restore_library_order(),
            CurrentView::Years => player.years.restore_library_order(),
            CurrentView::Composers => player.composers.restore_library_order(),
            // Back never lands on a song list, and both are refetched when
            // reopened, so neither can be the list being restored.
            CurrentView::Songs | CurrentView::AllSongs | CurrentView::RecentlyPlayed => {}
        }
        scroll::scroll_browse_to_top()
    } else {
        Task::none()
    }
}

/// The Artists view's All Songs button: fetch the whole library flat and
/// show it under the All Songs view. The flat list hangs off the artists
/// list, so an active search's results are left behind and Back returns to
/// Artists.
pub(in crate::ui) fn show_all_songs(player: &mut WinampPlayer) -> Task<Message> {
    switch_view(player, CurrentView::AllSongs);
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
    switch_view(player, CurrentView::AllAlbums);
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

/// The Artists view's Recently Played button: fetch the user's recently
/// played songs, most recent first, and show them flat under the Recently
/// Played view. Like All Songs, the list hangs off the artists list: leave an
/// active search behind and let Back return to Artists.
pub(in crate::ui) fn show_recently_played(player: &mut WinampPlayer) -> Task<Message> {
    switch_view(player, CurrentView::RecentlyPlayed);
    // Clear the previous list before the new fetch lands, so a press
    // during the fetch cannot resolve against stale rows.
    fetch_at_top(fetch_level(
        &mut player.recently_played,
        &player.apple_music_service,
        "loading recently played songs".to_string(),
        |service| async move { service.get_recently_played_songs().await },
        Message::RecentlyPlayedLoaded,
        Message::RecentlyPlayedLoadFailed,
    ))
}

/// The Artists view's Recently Added button: fetch the library's recently
/// added albums, newest first, and show them under the Recently Added view.
/// Like All Albums, the list hangs off the artists list: leave an active
/// search behind and let Back return to Artists.
pub(in crate::ui) fn show_recently_added(player: &mut WinampPlayer) -> Task<Message> {
    switch_view(player, CurrentView::RecentlyAdded);
    // Clear the previous list before the new fetch lands, so a press
    // during the fetch cannot resolve against stale rows.
    fetch_at_top(fetch_level(
        &mut player.recently_added,
        &player.apple_music_service,
        "loading recently added albums".to_string(),
        |service| async move { service.get_recently_added_albums().await },
        Message::RecentlyAddedLoaded,
        Message::RecentlyAddedLoadFailed,
    ))
}

/// The Artists view's Playlists button: fetch the library's playlists and
/// show them under the Playlists view. Like the flat lists, Playlists hangs
/// off the artists list: leave an active search behind and let Back return
/// to Artists.
pub(in crate::ui) fn show_playlists(player: &mut WinampPlayer) -> Task<Message> {
    switch_view(player, CurrentView::Playlists);
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

/// The Artists view's Years button: group the library's songs by release year
/// and show them under the Years view, hanging off the artists list like Genres.
pub(in crate::ui) fn show_years(player: &mut WinampPlayer) -> Task<Message> {
    switch_view(player, CurrentView::Years);
    fetch_at_top(fetch_level(
        &mut player.years,
        &player.apple_music_service,
        "loading library years".to_string(),
        |service| async move { service.get_all_songs().await.map(group_by_year) },
        Message::YearsLoaded,
        Message::YearsLoadFailed,
    ))
}

/// The Artists view's Composers button: group the library's songs by composer
/// and show them under the Composers view, hanging off the artists list like Years.
pub(in crate::ui) fn show_composers(player: &mut WinampPlayer) -> Task<Message> {
    switch_view(player, CurrentView::Composers);
    fetch_at_top(fetch_level(
        &mut player.composers,
        &player.apple_music_service,
        "loading library composers".to_string(),
        |service| async move { service.get_all_songs().await.map(group_by_composer) },
        Message::ComposersLoaded,
        Message::ComposersLoadFailed,
    ))
}

/// The Artists view's Genres button: group the library's songs by genre and
/// show them under the Genres view. Like Playlists, Genres hangs off the
/// artists list: leave an active search behind and let Back return to Artists.
pub(in crate::ui) fn show_genres(player: &mut WinampPlayer) -> Task<Message> {
    switch_view(player, CurrentView::Genres);
    fetch_at_top(fetch_level(
        &mut player.genres,
        &player.apple_music_service,
        "loading library genres".to_string(),
        |service| async move { service.get_all_songs().await.map(group_by_genre) },
        Message::GenresLoaded,
        Message::GenresLoadFailed,
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

/// The Reload button was pressed: refetch the top-level artists list and show
/// it under the Artists view. Clearing an active search and returning to
/// Artists make the reloaded list the one the user sees; the search box's
/// typed query, playback, and the other browse buffers stay as they were.
/// The fetch itself is the same clear-then-issue path Retry and boot use, so
/// any previous artists failure is dropped while the new request is in
/// flight. Snap the reloaded list to its first row like any replaced list.
pub(in crate::ui) fn reload_library(player: &mut WinampPlayer) -> Task<Message> {
    switch_view(player, CurrentView::Artists);
    fetch_at_top(fetch::load_artists(player))
}

/// Runs a browse fetch and then snaps the list it fills to the top.
/// Navigation replaces the visible list, so leaving the new level at the
/// previous list's scroll offset would hide its first rows; batching the
/// scroll with the fetch keeps the two one operation from the update loop.
pub(super) fn fetch_at_top(task: Task<Message>) -> Task<Message> {
    Task::batch([task, scroll::scroll_browse_to_top()])
}

/// Open the saved top-level browse list at launch: the matching `show_*`
/// handler, or nothing for Artists, which `boot` already loads.
pub(in crate::ui) fn restore_view(player: &mut WinampPlayer, view: LibraryView) -> Task<Message> {
    match view {
        LibraryView::Artists => Task::none(),
        LibraryView::AllSongs => show_all_songs(player),
        LibraryView::AllAlbums => show_all_albums(player),
        LibraryView::RecentlyAdded => show_recently_added(player),
        LibraryView::RecentlyPlayed => show_recently_played(player),
        LibraryView::Playlists => show_playlists(player),
        LibraryView::Genres => show_genres(player),
        LibraryView::Years => show_years(player),
    }
}

/// The startup sign-in stored a session: refetch the artists list, and a flat
/// list showing now, so both replace their sample-library rows with the
/// signed-in library. Albums, Songs, and search results stay as they are.
pub(in crate::ui) fn session_ready(player: &mut WinampPlayer) -> Task<Message> {
    let artists = fetch::load_artists(player);
    let flat = match player.current_view {
        CurrentView::AllSongs
        | CurrentView::AllAlbums
        | CurrentView::RecentlyAdded
        | CurrentView::RecentlyPlayed
        | CurrentView::Playlists
        | CurrentView::Genres
        | CurrentView::Years
            if !player.search_active =>
        {
            let view = player.library_view();
            restore_view(player, view)
        }
        _ => Task::none(),
    };
    Task::batch([artists, flat])
}
