//! The browse views — the artist → album → song drill-down, the flat All
//! Songs and All Albums lists, and the Playlists list: the per-level view
//! builders, the search box, and the Back, Open, Top, Sort, All Songs, All
//! Albums, Playlists, and Retry buttons above them. The scrollable list, its
//! row mappings, and the footer summaries live in the [`rows`] submodule.
//!
//! Every builder is pure — it turns `&[Artist]` / `&[Album]` / `&[Song]` /
//! `&[Playlist]` (or the rows they map to) into an `Element` and knows nothing
//! about `WinampPlayer` or the update loop — so the browse UI is testable
//! without an iced renderer.

use iced::{
    Element,
    widget::{Row, TextInput},
};

use crate::library::{Album, Artist, Playlist, Song};
use crate::ui::{CurrentView, Message, style};

use super::labeled_button;

pub(super) mod rows;

use rows::{
    album_list, artist_row, browse_placeholder, list_with_count, playlist_row, scrollable_list,
    song_list,
};

pub use rows::{BROWSE_LIST_ID, BROWSE_LIST_PADDING, BROWSE_ROW_HEIGHT};

/// The Artists browse view: one row per artist, in the order given, each
/// selecting its row with [`Message::SelectRow`] carrying the artist's index
/// in the list and the list's `epoch`; pressing the selected row again opens
/// the artist. `selected` paints that row's blue bar without adding the `▶`
/// playing marker. `loading` and `error` select the placeholder wording (see
/// [`browse_placeholder`]); a fetch failure wins. Built by [`scrollable_list`]
/// from the [`artist_row`] mapping; a non-empty list also appends its
/// item-count footer ([`list_with_count`]).
pub fn view_artists<'a>(
    artists: &'a [Artist],
    epoch: u64,
    selected: Option<usize>,
    loading: bool,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    list_with_count(
        scrollable_list(
            artists.iter().map(artist_row),
            epoch,
            selected,
            browse_placeholder(&CurrentView::Artists, loading, error, false),
        ),
        artists.len(),
        "artist",
        "artists",
    )
}

/// The Albums browse view: one row per album, in the order given, each
/// selecting its row with [`Message::SelectRow`] (a second press opens it).
/// `titles` and `labels` hold the display titles and formatted running times
/// the loaded list cached in the same order as `albums` (see
/// `BrowseList::store_with_row_text`); an index without a cached title or
/// label falls back to composing that album's title or formatting its
/// duration. `selected` paints that row's blue bar. `loading` and `error`
/// select the placeholder wording (see [`browse_placeholder`]); a fetch
/// failure wins. Built by [`album_list`] from the [`rows::album_row`] mapping.
pub fn view_albums<'a>(
    albums: &'a [Album],
    titles: &'a [String],
    labels: &'a [String],
    epoch: u64,
    selected: Option<usize>,
    loading: bool,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    album_list(
        albums,
        titles,
        labels,
        epoch,
        selected,
        browse_placeholder(&CurrentView::Albums, loading, error, false),
    )
}

/// The Songs browse view: one row per song, in the order given, each
/// selecting its row with [`Message::SelectRow`] (a second press plays it);
/// the row whose id is `current_track` is marked as playing and `selected`
/// paints the selected row's blue bar. `titles` and `labels` hold the display
/// titles and formatted lengths the loaded list cached in the same order as
/// `songs` (see `BrowseList::store_with_row_text`); an index without a cached
/// title or label falls back to composing that song's title or formatting its
/// duration. `loading`, `error`, and `search_active` select the placeholder
/// wording (see [`browse_placeholder`]); a fetch failure wins, and a
/// loaded-empty search reads as "no matches" rather than as an empty album.
/// Built by [`song_list`] from the [`rows::song_row`] mapping.
#[allow(clippy::too_many_arguments)]
pub fn view_songs<'a>(
    songs: &'a [Song],
    titles: &'a [String],
    labels: &'a [String],
    epoch: u64,
    selected: Option<usize>,
    loading: bool,
    error: Option<&'a str>,
    current_track: Option<&str>,
    search_active: bool,
) -> Element<'a, Message> {
    song_list(
        songs,
        titles,
        labels,
        epoch,
        selected,
        current_track,
        browse_placeholder(&CurrentView::Songs, loading, error, search_active),
    )
}

/// The All Songs browse view: one row per song in the whole library, in the
/// order given, each selecting its row with [`Message::SelectRow`] (a second
/// press plays it); the row whose id is `current_track` is marked as playing
/// and `selected` paints the selected row's blue bar. `titles` and `labels`
/// are the same cached display-title and formatted-length slices
/// [`view_songs`] takes. It mirrors [`view_songs`] but never shows the search
/// wording, because the flat list is not a search's result. Built by
/// [`song_list`] from the [`rows::song_row`] mapping.
#[allow(clippy::too_many_arguments)]
pub fn view_all_songs<'a>(
    songs: &'a [Song],
    titles: &'a [String],
    labels: &'a [String],
    epoch: u64,
    selected: Option<usize>,
    loading: bool,
    error: Option<&'a str>,
    current_track: Option<&str>,
) -> Element<'a, Message> {
    song_list(
        songs,
        titles,
        labels,
        epoch,
        selected,
        current_track,
        browse_placeholder(&CurrentView::AllSongs, loading, error, false),
    )
}

/// The All Albums browse view: one row per album in the whole library, in the
/// order given, each selecting its row with [`Message::SelectRow`] (a second
/// press opens it); `titles` and `labels` are the same cached display-title
/// and formatted-running-time slices [`view_albums`] takes. `selected` paints
/// that row's blue bar. It mirrors [`view_albums`] — album rows carry no
/// current-track marker — but is its own view so its presses resolve against
/// `all_albums` and Back from the songs it opens returns to this list. It
/// never shows the search wording, because the flat list is not a search's
/// result. Built by [`album_list`] from the [`rows::album_row`] mapping.
pub fn view_all_albums<'a>(
    albums: &'a [Album],
    titles: &'a [String],
    labels: &'a [String],
    epoch: u64,
    selected: Option<usize>,
    loading: bool,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    album_list(
        albums,
        titles,
        labels,
        epoch,
        selected,
        browse_placeholder(&CurrentView::AllAlbums, loading, error, false),
    )
}

/// The Playlists browse view: one row per playlist, in the order given, each
/// selecting its row with [`Message::SelectRow`] (a second press opens it);
/// `selected` paints that row's blue bar. `loading` and `error` select the
/// placeholder wording (see [`browse_placeholder`]); a fetch failure wins.
/// Built by [`scrollable_list`] from the [`playlist_row`] mapping; a non-empty
/// list also appends its item-count footer ([`list_with_count`]).
pub fn view_playlists<'a>(
    playlists: &'a [Playlist],
    epoch: u64,
    selected: Option<usize>,
    loading: bool,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    list_with_count(
        scrollable_list(
            playlists.iter().map(playlist_row),
            epoch,
            selected,
            browse_placeholder(&CurrentView::Playlists, loading, error, false),
        ),
        playlists.len(),
        "playlist",
        "playlists",
    )
}

/// Whether the browse view has a level above it to return to. Every view
/// below the top-level Artists list does — the Back button is shown above
/// their lists — while Artists itself has nothing to go back to.
///
/// `#[must_use]`: a discarded result is a logic bug — the caller that decides
/// whether to show the Back button would silently never show it.
#[must_use]
pub fn can_go_back(view: &CurrentView) -> bool {
    !matches!(view, CurrentView::Artists)
}

/// Whether the Artists view should offer its Retry button. Only the
/// top-level artists fetch is unrecoverable by navigation, so Retry belongs
/// to the Artists view and only while that fetch has failed; every other
/// browse level recovers by navigating back into it. The update loop's
/// `LoadArtists` arm clears the failure and re-issues the fetch.
///
/// `#[must_use]`: a discarded result is a logic bug — the caller that decides
/// whether to show the Retry button would silently never show it.
#[must_use]
pub fn can_retry_artists(view: &CurrentView, error: Option<&str>) -> bool {
    matches!(view, CurrentView::Artists) && error.is_some()
}

/// The browse view's Back button, stepping the hierarchy one level up: Songs
/// returns to the list it was opened from (Albums, All Albums, or Playlists),
/// while a library-search result and every other level return to Artists.
/// Rendered only where [`can_go_back`] is true; the update loop turns the
/// pressed message into the view change.
pub fn view_back_button() -> Element<'static, Message> {
    labeled_button("Back", Message::Back).into()
}

/// The browse view's Top button, returning the current list to its first row.
/// Rendered above every browse level; the update loop turns the pressed
/// message into a snap-to-top operation on [`BROWSE_LIST_ID`].
pub fn view_top_button() -> Element<'static, Message> {
    labeled_button("Top", Message::ScrollBrowseToTop).into()
}

/// The browse view's Sort button, reordering the showing list in place by its
/// rows' rendered text. Rendered above every browse level beside Top; the
/// update loop's `SortBrowse` arm reorders that level's list and bumps its
/// epoch.
pub fn view_sort_button() -> Element<'static, Message> {
    labeled_button("Sort", Message::SortBrowse).into()
}

/// The browse view's Open button: open the row selected in the showing list
/// through the same artist/album/track/playlist action a second press
/// dispatches. Rendered above every browse level; with nothing selected the
/// update loop's `OpenSelected` arm is a no-op.
pub fn view_open_button() -> Element<'static, Message> {
    labeled_button("Open", Message::OpenSelected).into()
}

/// The Top and Sort buttons as one row, so Sort renders beside Top rather than
/// on the row below it, as the README and the Browse row order invariant
/// promise. Rendered above every browse level.
pub fn view_browse_nav_buttons() -> Element<'static, Message> {
    Row::new()
        .push(view_top_button())
        .push(view_sort_button())
        .into()
}

/// The Artists view's All Songs button, opening the flat whole-library list.
/// Rendered only on the Artists view; the update loop's `ShowAllSongs` arm
/// switches to `CurrentView::AllSongs` and issues the fetch.
pub fn view_all_songs_button() -> Element<'static, Message> {
    labeled_button("All Songs", Message::ShowAllSongs).into()
}

/// The Artists view's All Albums button, opening the flat whole-library album
/// list. Rendered only on the Artists view, beside the All Songs button; the
/// update loop's `ShowAllAlbums` arm switches to `CurrentView::AllAlbums` and
/// issues the fetch.
pub fn view_all_albums_button() -> Element<'static, Message> {
    labeled_button("All Albums", Message::ShowAllAlbums).into()
}

/// The Artists view's Playlists button, opening the library's playlist list.
/// Rendered only on the Artists view, beside the All Songs and All Albums
/// buttons; the update loop's `ShowPlaylists` arm switches to
/// `CurrentView::Playlists` and issues the fetch.
pub fn view_playlists_button() -> Element<'static, Message> {
    labeled_button("Playlists", Message::ShowPlaylists).into()
}

/// The Artists view's Retry button, re-running the top-level artists fetch
/// after it failed. Rendered only where [`can_retry_artists`] is true; the
/// update loop's `LoadArtists` arm clears the failure and re-issues the
/// fetch, so the button turns the otherwise unrecoverable failed startup
/// fetch — no navigation re-issues it — into a retry.
pub fn view_retry_button() -> Element<'static, Message> {
    labeled_button("Retry", Message::LoadArtists).into()
}

/// The library search box shown above the browse list: an iced text input with
/// the `Search library` placeholder. Typing emits [`Message::SearchChanged`]
/// so the player stores the box's text; pressing Enter emits the payload-free
/// [`Message::SearchSubmitted`], which the update loop resolves against that
/// stored text into a library search. Rendered on every browse screen, so a
/// search can be started from any level of the hierarchy. The submit message
/// owns no copy of the query, so this per-view builder allocates nothing for
/// the submit path; the text input itself still builds its own value.
pub fn view_search_box(query: &str) -> Element<'_, Message> {
    TextInput::new("Search library", query)
        .on_input(Message::SearchChanged)
        .on_submit(Message::SearchSubmitted)
        .style(|_theme, status| style::chrome_text_input_style(status))
        .into()
}

/// Every browse view the UI can assemble, in hierarchy order. The per-view
/// tests and the full-input test in `ui::tests` all drive each level, so
/// naming the list once means a new view is added here rather than to each
/// loop's literal.
#[cfg(test)]
pub const BROWSE_VIEWS: [CurrentView; 6] = [
    CurrentView::Artists,
    CurrentView::Albums,
    CurrentView::Songs,
    CurrentView::AllSongs,
    CurrentView::AllAlbums,
    CurrentView::Playlists,
];
