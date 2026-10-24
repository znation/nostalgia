//! The browse fetch results: the initial artists load and every `*Loaded`,
//! `*LoadFailed`, and `BrowseReply` handler that fills the browse lists.
//!
//! Each handler writes only the list its request named. `browse_reply`
//! re-checks the request generation on the UI thread before dispatching, so a
//! superseded reply cannot overwrite a newer list. The navigation and search
//! that issue these fetches live in the parent [`super`] module.

use iced::Task;

use crate::library::{Album, Artist, Playlist, Song};
use crate::ui::browse::BrowseList;
use crate::ui::loading::fetch_level;
use crate::ui::views;
use crate::ui::{Message, WinampPlayer};

/// The artists level is fetched: `boot` schedules this for the initial load,
/// and again once the startup sign-in stores a session; it is also the Retry
/// button's case — clearing first drops any earlier failure report and shows
/// "Loading…" while the re-fetch is in flight, exactly as the navigation
/// handlers clear their level before re-fetching.
pub(in crate::ui) fn load_artists(player: &mut WinampPlayer) -> Task<Message> {
    fetch_level(
        &mut player.artists,
        &player.apple_music_service,
        "loading favorite artists".to_string(),
        |service| async move { service.get_favorite_artists().await },
        Message::ArtistsLoaded,
        Message::ArtistsLoadFailed,
    )
}

/// The artists fetch landed: store the rows in the artists list.
pub(in crate::ui) fn artists_loaded(
    player: &mut WinampPlayer,
    artists: Vec<Artist>,
) -> Task<Message> {
    player.artists.store(artists);
    Task::none()
}

/// The albums fetch landed: store the rows in the albums list, caching each
/// row's display title and formatted running time (see [`store_albums`]).
pub(in crate::ui) fn albums_loaded(player: &mut WinampPlayer, albums: Vec<Album>) -> Task<Message> {
    store_albums(&mut player.albums, albums);
    Task::none()
}

/// Store a loaded album list, caching each row's display title and formatted
/// running time once (see `BrowseList::store_with_row_text`), the album twin
/// of [`store_songs`]. Without the cache a view refresh re-joins every album's
/// artist and title and reformats every running time, and the flat All Albums
/// list can hold the whole library.
fn store_albums(list: &mut BrowseList<Album>, albums: Vec<Album>) {
    list.store_with_row_text(
        albums,
        |album| album.display_title().into_owned(),
        |album| views::format_track_time(album.duration_ms),
    );
}

/// The songs fetch landed: store the rows in the songs list. Cache each
/// row's display title and formatted length once, when the list loads, so a
/// view refresh borrows them instead of re-joining every artist and title
/// and formatting every row's duration again (see
/// `BrowseList::store_with_row_text`).
pub(in crate::ui) fn songs_loaded(player: &mut WinampPlayer, songs: Vec<Song>) -> Task<Message> {
    store_songs(&mut player.songs, songs);
    Task::none()
}

/// Store a loaded song list, caching each row's display title and formatted
/// length once (see `BrowseList::store_with_row_text`). Shared by
/// [`songs_loaded`] and [`all_songs_loaded`] so the two stay in sync.
fn store_songs(list: &mut BrowseList<Song>, songs: Vec<Song>) {
    list.store_with_row_text(
        songs,
        |song| song.display_title().into_owned(),
        |song| views::format_track_time(song.duration_ms),
    );
}

/// The flat whole-library songs fetch landed: store the rows in the All Songs
/// list.
pub(in crate::ui) fn all_songs_loaded(
    player: &mut WinampPlayer,
    songs: Vec<Song>,
) -> Task<Message> {
    store_songs(&mut player.all_songs, songs);
    Task::none()
}

/// The flat whole-library albums fetch landed: store the rows in the All
/// Albums list, with the same cached row text as the browsed list.
pub(in crate::ui) fn all_albums_loaded(
    player: &mut WinampPlayer,
    albums: Vec<Album>,
) -> Task<Message> {
    store_albums(&mut player.all_albums, albums);
    Task::none()
}

/// The playlists fetch landed: store the rows in the playlists list.
pub(in crate::ui) fn playlists_loaded(
    player: &mut WinampPlayer,
    playlists: Vec<Playlist>,
) -> Task<Message> {
    player.playlists.store(playlists);
    Task::none()
}

/// Record a fetch failure on one browse list and leave the UI task-free.
/// The per-list handlers below differ only in which list they name, so the
/// shared body — `BrowseList::fail` plus the empty task — lives here once.
fn fail_list<T>(list: &mut BrowseList<T>, report: String) -> Task<Message> {
    list.fail(report);
    Task::none()
}

/// The artists fetch failed: record the report so the browse panel can show
/// why the list is empty instead of misreading the failure as an empty
/// library.
pub(in crate::ui) fn artists_load_failed(
    player: &mut WinampPlayer,
    report: String,
) -> Task<Message> {
    fail_list(&mut player.artists, report)
}

/// The albums fetch failed: record the report on the albums list.
pub(in crate::ui) fn albums_load_failed(
    player: &mut WinampPlayer,
    report: String,
) -> Task<Message> {
    fail_list(&mut player.albums, report)
}

/// The songs fetch failed: record the report on the songs list.
pub(in crate::ui) fn songs_load_failed(player: &mut WinampPlayer, report: String) -> Task<Message> {
    fail_list(&mut player.songs, report)
}

/// The flat whole-library songs fetch failed: record the report on the All
/// Songs list.
pub(in crate::ui) fn all_songs_load_failed(
    player: &mut WinampPlayer,
    report: String,
) -> Task<Message> {
    fail_list(&mut player.all_songs, report)
}

/// The flat whole-library albums fetch failed: record the report on the All
/// Albums list.
pub(in crate::ui) fn all_albums_load_failed(
    player: &mut WinampPlayer,
    report: String,
) -> Task<Message> {
    fail_list(&mut player.all_albums, report)
}

/// The playlists fetch failed: record the report on the playlists list.
pub(in crate::ui) fn playlists_load_failed(
    player: &mut WinampPlayer,
    report: String,
) -> Task<Message> {
    fail_list(&mut player.playlists, report)
}

/// A browse reply stamped with the generation that produced it. The
/// executor-thread check in `fetch_into` already dropped replies a newer
/// request had superseded when they were mapped. This re-checks the stamp on
/// the UI thread, where the store happens, closing the window between that
/// check and this handler: a navigation there issues a newer request, and
/// the stale reply must not repopulate the list (or record its error) over
/// it.
pub(in crate::ui) fn browse_reply(
    player: &mut WinampPlayer,
    issued: u64,
    reply: Box<Message>,
) -> Task<Message> {
    let current = match reply.as_ref() {
        Message::ArtistsLoaded(_) | Message::ArtistsLoadFailed(_) => {
            player.artists.is_current(issued)
        }
        Message::AlbumsLoaded(_) | Message::AlbumsLoadFailed(_) => player.albums.is_current(issued),
        Message::SongsLoaded(_) | Message::SongsLoadFailed(_) => player.songs.is_current(issued),
        Message::AllSongsLoaded(_) | Message::AllSongsLoadFailed(_) => {
            player.all_songs.is_current(issued)
        }
        Message::AllAlbumsLoaded(_) | Message::AllAlbumsLoadFailed(_) => {
            player.all_albums.is_current(issued)
        }
        Message::PlaylistsLoaded(_) | Message::PlaylistsLoadFailed(_) => {
            player.playlists.is_current(issued)
        }
        _ => true,
    };
    if current {
        crate::ui::update(player, *reply)
    } else {
        Task::none()
    }
}

/// A browse reply that a newer request for the same list has superseded:
/// `fetch_into` emits this instead of the `*Loaded` message so the stale
/// reply never reaches `BrowseList::store`. The handler is a no-op.
pub(in crate::ui) fn ignored(_player: &mut WinampPlayer) -> Task<Message> {
    Task::none()
}
