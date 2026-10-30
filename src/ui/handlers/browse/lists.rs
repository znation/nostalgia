//! The Artist-view lists and the steps between them: the flat and grouped
//! lists the artists view's buttons open (All Songs, All Albums, Recently
//! Played, Recently Added, Playlists, Genres, Years, Composers), Back's step
//! back up the hierarchy, the Reload button, and the two launch hooks that
//! land on a list (`restore_view` at boot, `session_ready` after a startup
//! sign-in).
//!
//! Like every replaced list, these leave any active search behind (`switch_view`)
//! and clear before fetching, so a press during a fetch cannot resolve against
//! stale rows.

use iced::Task;

use crate::settings::library_view::LibraryView;
use crate::ui::composers::group_by_composer;
use crate::ui::genres::group_by_genre;
use crate::ui::loading::fetch_level;
use crate::ui::years::group_by_year;
use crate::ui::{CurrentView, Message, SongsOrigin, WinampPlayer};

use super::super::playback::resolve_current_track;
use super::{fetch, fetch_at_top, scroll, switch_view};

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
                SongsOrigin::UserPlaylist => CurrentView::Playlists,
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
/// Played view. While no `MusicKit` session is stored and something played this
/// run, the songs the run committed stand in for the fetch (see
/// `WinampPlayer::played_songs`); with nothing played yet the signed-out
/// list still fetches the service seam's sample stand-in. Like All Songs,
/// the list hangs off the artists list: leave an active search behind and
/// let Back return to Artists.
pub(in crate::ui) fn show_recently_played(player: &mut WinampPlayer) -> Task<Message> {
    switch_view(player, CurrentView::RecentlyPlayed);
    if player.apple_music_service.session().is_none() && !player.played_songs.is_empty() {
        // The run's played rows replace the showing list immediately, with
        // the same transition a fetched reply uses: clear the stale rows
        // first, then store with the row text cached, so the epoch bump
        // drops any press rendered from the old list and no fetch stays in
        // flight for the media keys to play into.
        player.recently_played.clear();
        fetch::store_songs(
            &mut player.recently_played,
            player.played_songs.newest_first(),
        );
        return Task::none();
    }
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
        LibraryView::Composers => show_composers(player),
    }
}

/// list showing now, so both replace their sample-library rows with the
/// signed-in library. Albums, Songs, and search results stay as they are.
///
/// A saved Apple Music id for the restored last-played track can only be
/// looked up with a session, so the sign-in also issues
/// `resolve_current_track`: with a stored session the lookup at last reaches
/// the signed-in library and the bar switches from the id to the title. Its
/// own guard makes every later re-issue free, and a sample-library id was
/// already resolved at boot.
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
        | CurrentView::Composers
            if !player.search_active =>
        {
            let view = player.library_view();
            restore_view(player, view)
        }
        _ => Task::none(),
    };
    Task::batch([artists, flat, resolve_current_track(player)])
}
