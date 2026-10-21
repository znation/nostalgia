//! The artist → album → song browse views: the scrollable list, its per-level
//! row mappings, the search box, and the Back, All Songs, All Albums, and
//! Retry buttons above it.
//!
//! Every builder is pure — it turns `&[Artist]` / `&[Album]` / `&[Song]` (or
//! the rows they map to) into an `Element` and knows nothing about
//! `WinampPlayer` or the update loop — so the browse UI is testable without an
//! iced renderer.

use std::borrow::Cow;

use iced::{
    Background, Element, Length,
    widget::{Button, Column, Row, Scrollable, Text, TextInput, button},
};

use crate::library::{Album, Artist, Playlist, Song};
use crate::ui::{CurrentView, Message, style, theme};

use super::{labeled_button, now_playing::format_track_time, spacer};

/// The placeholder a browse view shows when its list has no rows: one wording
/// per browse level, so an empty Artists, Albums, Songs, All Songs, All
/// Albums, or Playlists panel names the list rather than showing a blank
/// panel. Pure so the per-level wording is testable without an iced renderer,
/// like the button label helpers below.
pub(super) fn empty_list_label(view: &CurrentView) -> &'static str {
    match view {
        CurrentView::Artists => "No artists",
        CurrentView::Albums => "No albums",
        CurrentView::Songs => "No songs",
        CurrentView::AllSongs => "No songs",
        CurrentView::AllAlbums => "No albums",
        CurrentView::Playlists => "No playlists",
    }
}

/// The placeholder a browse view shows when its list has no rows, choosing
/// between a fetch failure, "still loading", and "loaded, but empty".
///
/// `error` is the list's most recent fetch-failure report (see
/// `crate::ui::browse::BrowseList::fail`/`crate::ui::browse::BrowseList::clear`): when present it wins, so a
/// backend failure is shown rather than misread as an empty library.
/// Otherwise `loading` is the list's load state: true means the reply the
/// navigation just scheduled is still in flight, so the panel reads as
/// loading rather than as an empty library; false means a reply landed —
/// possibly an empty one — so [`empty_list_label`] names the list.
///
/// `search_active` marks the Songs list as holding a library search's results
/// rather than a browsed album's songs. A search that matched nothing then
/// reads as "no matches" instead of the album-level [`empty_list_label`]
/// wording, which would blame the album the search was issued from. The update
/// loop's `SearchSubmitted` arm is the only arm that sets it true, and it
/// always shows the Songs view, so the flag is only ever true with
/// `view == Songs`. Pure, like
/// [`empty_list_label`], so every wording is testable without an iced
/// renderer.
pub(super) fn browse_placeholder<'a>(
    view: &CurrentView,
    loading: bool,
    error: Option<&'a str>,
    search_active: bool,
) -> &'a str {
    if let Some(error) = error {
        error
    } else if loading {
        "Loading…"
    } else if search_active {
        "No songs match your search"
    } else {
        empty_list_label(view)
    }
}

/// The style for a browse row that is currently playing: the flat
/// [`style::playlist_row_style`] with its background forced to
/// [`theme::PLAYING_ROW_HIGHLIGHT`], so the playing row reads as a
/// selection bar in every hover/press state instead of following the row's
/// face ladder under the cursor. Pure, so the highlight — which iced's
/// `Element` API gives no way to read back from the built `Button`, and whose
/// closure runs only at render time — is testable.
pub(super) fn current_row_style(status: button::Status) -> button::Style {
    let mut row = style::playlist_row_style(status);
    row.background = Some(Background::Color(theme::PLAYING_ROW_HIGHLIGHT));
    row
}

/// Builds a scrollable list where each item is a button showing a title
/// followed by a secondary label, emitting the given message on press.
/// Shared by the artists, albums, and songs views. The last tuple element
/// marks the currently playing row: such a row gets a `▶` prefix and a
/// highlighted background so the list reads as a playlist. Only `song_row`
/// ever sets the flag to true.
///
/// When `items` yields nothing, the list renders `empty_label` in place of a
/// blank panel. The caller picks the wording via [`browse_placeholder`],
/// which distinguishes a fetch failure, a buffer no reply has populated yet
/// (still loading), and one that loaded empty and names the list.
///
/// The rows borrow their titles from the list the caller passes in rather
/// than owning clones: this builder runs on every view refresh, so the
/// borrowed title avoids a `String` allocation per row per frame. The
/// secondary label is a [`Cow`] because a song row's label is the track's
/// formatted length, borrowed from the loaded list's label cache in the
/// common case, while the artist and album rows keep their static hints
/// borrowed. The playing row's `▶` marker is pushed as
/// its own static label beside the borrowed title rather than formatted into
/// an owned one, so the marker adds no `String` allocation per frame either.
fn scrollable_list<'a>(
    items: impl IntoIterator<Item = (&'a str, Cow<'a, str>, Message, bool)>,
    empty_label: &'a str,
) -> Element<'a, Message> {
    let mut column = Column::new().padding(20);
    let mut is_empty = true;

    for (title, label, message, is_current) in items {
        is_empty = false;
        let row = if is_current {
            Row::new().push(Text::new("▶ ").size(18))
        } else {
            Row::new()
        };
        let button = Button::new(
            row.push(Text::new(title).size(18))
                .push(spacer(10.0))
                .push(Text::new(label).size(14)),
        )
        .on_press(message)
        .style(|_theme, status| style::playlist_row_style(status));
        // The current row keeps the playlist row's text and hover/press face
        // and only swaps the background for the highlight colour.
        column = column.push(if is_current {
            button.style(|_theme, status| current_row_style(status))
        } else {
            button
        });
    }

    if is_empty {
        column = column.push(Text::new(empty_label).size(18));
    }

    Scrollable::new(column)
        .width(Length::Fill)
        .height(Length::FillPortion(3))
        .style(|_theme, _status| style::playlist_scrollable_style())
        .into()
}

/// The browse row an artist becomes: its name, the "View Albums" hint, the
/// message emitted when it is pressed, and the current-track flag. `epoch` is
/// the loaded-list epoch and `index` the artist's position in that list; both
/// become the selection message's payload, so building a row never clones the
/// artist's id — the update loop resolves the index back to an id only when
/// the row is actually pressed, and rejects the press if the list has since
/// been replaced. Kept out of `view_artists` so the title/label/selection
/// contract is testable without an iced renderer. Artists are never the
/// currently playing row, so the flag is always false.
pub(super) fn artist_row<'a>(
    epoch: u64,
    index: usize,
    artist: &'a Artist,
) -> (&'a str, Cow<'a, str>, Message, bool) {
    (
        artist.name.as_str(),
        Cow::Borrowed("View Albums"),
        Message::ArtistSelected { epoch, index },
        false,
    )
}

/// The browse row an album becomes: its title, the "View Songs" hint, the
/// message emitted when it is pressed, and the current-track flag. `epoch`
/// and `index` are the loaded-list epoch and the album's position in that
/// list, carried so building a row never clones the album's id (see
/// [`artist_row`]). Albums are never the currently playing row, so the flag
/// is always false.
pub(super) fn album_row<'a>(
    epoch: u64,
    index: usize,
    album: &'a Album,
) -> (&'a str, Cow<'a, str>, Message, bool) {
    (
        album.title.as_str(),
        Cow::Borrowed("View Songs"),
        Message::AlbumSelected { epoch, index },
        false,
    )
}

/// The browse row a playlist becomes: its name, the "View Songs" hint, the
/// message emitted when it is pressed, and the current-track flag. `epoch` and
/// `index` are the loaded-list epoch and the playlist's position in that list,
/// carried so building a row never clones the playlist's id (see
/// [`artist_row`]). Playlists are never the currently playing row, so the flag
/// is always false.
pub(super) fn playlist_row<'a>(
    epoch: u64,
    index: usize,
    playlist: &'a Playlist,
) -> (&'a str, Cow<'a, str>, Message, bool) {
    (
        playlist.name.as_str(),
        Cow::Borrowed("View Songs"),
        Message::PlaylistSelected { epoch, index },
        false,
    )
}

/// The browse row a song becomes: its title, the track's formatted length
/// ([`format_track_time`]) as the secondary label, the message emitted when
/// it is pressed, and whether it is the currently playing track (true only
/// when `current_track` names this song). `epoch` and `index` are the
/// loaded-list epoch and the song's position in that list, carried so
/// building a row never clones the song's id (see [`artist_row`]). The flag
/// feeds `scrollable_list`'s `▶` + highlight marker, so the song list reads
/// as a playlist. `cached_label` is the length label the loaded list stored
/// for this row (see `BrowseList::store_with_labels`), borrowed so a refresh
/// of a large library does not format and allocate a fresh `String` per
/// track; `None` — a list stored without labels, as a test may do — falls
/// back to formatting the duration for this row so the rendered label is
/// unchanged. The title always borrows.
pub(super) fn song_row<'a>(
    epoch: u64,
    index: usize,
    song: &'a Song,
    cached_label: Option<&'a str>,
    current_track: Option<&str>,
) -> (&'a str, Cow<'a, str>, Message, bool) {
    let label = match cached_label {
        Some(label) => Cow::Borrowed(label),
        None => Cow::Owned(format_track_time(song.duration_ms)),
    };
    (
        song.title.as_str(),
        label,
        Message::TrackSelected { epoch, index },
        Some(song.id.as_str()) == current_track,
    )
}

/// Builds an album list: the shared body of [`view_albums`] and
/// [`view_all_albums`], mapping each album to an [`album_row`] by its
/// position in `albums` and carrying `epoch`. `placeholder` is the caller's
/// [`browse_placeholder`] wording for an empty list.
fn album_list<'a>(albums: &'a [Album], epoch: u64, placeholder: &'a str) -> Element<'a, Message> {
    scrollable_list(
        albums
            .iter()
            .enumerate()
            .map(|(index, album)| album_row(epoch, index, album)),
        placeholder,
    )
}

/// Builds a song list: the shared body of [`view_songs`] and
/// [`view_all_songs`], mapping each song to a [`song_row`] by its position in
/// `songs` and pairing it with the cached length label at the same index (see
/// [`song_row`]). `placeholder` is the caller's [`browse_placeholder`]
/// wording for an empty list; it differs between the two callers because only
/// a search reads as "no matches".
fn song_list<'a>(
    songs: &'a [Song],
    labels: &'a [String],
    epoch: u64,
    current_track: Option<&str>,
    placeholder: &'a str,
) -> Element<'a, Message> {
    scrollable_list(
        songs.iter().enumerate().map(|(index, song)| {
            song_row(
                epoch,
                index,
                song,
                labels.get(index).map(String::as_str),
                current_track,
            )
        }),
        placeholder,
    )
}

/// The Artists browse view: one row per artist, in the order given, each
/// emitting [`Message::ArtistSelected`] with the artist's index in the list
/// and the list's `epoch`. `loading` and `error` select the placeholder
/// wording (see [`browse_placeholder`]); a fetch failure wins. Built by
/// [`scrollable_list`] from the [`artist_row`] mapping.
pub fn view_artists<'a>(
    artists: &'a [Artist],
    epoch: u64,
    loading: bool,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    scrollable_list(
        artists
            .iter()
            .enumerate()
            .map(|(index, artist)| artist_row(epoch, index, artist)),
        browse_placeholder(&CurrentView::Artists, loading, error, false),
    )
}

/// The Albums browse view: one row per album, in the order given, each
/// emitting [`Message::AlbumSelected`] with the album's index in the list and
/// the list's `epoch`. `loading` and `error` select the placeholder wording
/// (see [`browse_placeholder`]); a fetch failure wins. Built by
/// [`album_list`] from the [`album_row`] mapping.
pub fn view_albums<'a>(
    albums: &'a [Album],
    epoch: u64,
    loading: bool,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    album_list(
        albums,
        epoch,
        browse_placeholder(&CurrentView::Albums, loading, error, false),
    )
}

/// The Songs browse view: one row per song, in the order given, each emitting
/// [`Message::TrackSelected`] with the song's index in the list and the
/// list's `epoch`; the row whose id is `current_track` is marked as playing.
/// `labels` holds the formatted lengths the loaded list cached in the same
/// order as `songs` (see `BrowseList::store_with_labels`); an index without a
/// cached label falls back to formatting that song's duration. `loading`,
/// `error`, and `search_active` select the placeholder wording (see
/// [`browse_placeholder`]); a fetch failure wins, and a loaded-empty search
/// reads as "no matches" rather than as an empty album. Built by
/// [`song_list`] from the [`song_row`] mapping.
pub fn view_songs<'a>(
    songs: &'a [Song],
    labels: &'a [String],
    epoch: u64,
    loading: bool,
    error: Option<&'a str>,
    current_track: Option<&str>,
    search_active: bool,
) -> Element<'a, Message> {
    song_list(
        songs,
        labels,
        epoch,
        current_track,
        browse_placeholder(&CurrentView::Songs, loading, error, search_active),
    )
}

/// The All Songs browse view: one row per song in the whole library, in the
/// order given, each emitting [`Message::TrackSelected`] with the song's
/// index in the list and the list's `epoch`; the row whose id is
/// `current_track` is marked as playing. `labels` is the same cached
/// formatted-length slice [`view_songs`] takes. It mirrors [`view_songs`] but
/// never shows the search wording, because the flat list is not a search's
/// result. Built by [`song_list`] from the [`song_row`] mapping.
pub fn view_all_songs<'a>(
    songs: &'a [Song],
    labels: &'a [String],
    epoch: u64,
    loading: bool,
    error: Option<&'a str>,
    current_track: Option<&str>,
) -> Element<'a, Message> {
    song_list(
        songs,
        labels,
        epoch,
        current_track,
        browse_placeholder(&CurrentView::AllSongs, loading, error, false),
    )
}

/// The All Albums browse view: one row per album in the whole library, in the
/// order given, each emitting [`Message::AlbumSelected`] with the album's
/// index in the list and the list's `epoch`. It mirrors [`view_albums`] —
/// album rows carry no current-track marker — but is its own view so its
/// presses resolve against `all_albums` and Back returns to the flat list. It
/// never shows the search wording, because the flat list is not a search's
/// result. Built by [`album_list`] from the [`album_row`] mapping.
pub fn view_all_albums<'a>(
    albums: &'a [Album],
    epoch: u64,
    loading: bool,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    album_list(
        albums,
        epoch,
        browse_placeholder(&CurrentView::AllAlbums, loading, error, false),
    )
}

/// The Playlists browse view: one row per playlist, in the order given, each
/// emitting [`Message::PlaylistSelected`] with the playlist's index in the
/// list and the list's `epoch`. `loading` and `error` select the placeholder
/// wording (see [`browse_placeholder`]); a fetch failure wins. Built by
/// [`scrollable_list`] from the [`playlist_row`] mapping.
pub fn view_playlists<'a>(
    playlists: &'a [Playlist],
    epoch: u64,
    loading: bool,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    scrollable_list(
        playlists
            .iter()
            .enumerate()
            .map(|(index, playlist)| playlist_row(epoch, index, playlist)),
        browse_placeholder(&CurrentView::Playlists, loading, error, false),
    )
}

/// Whether the browse view has a level above it to return to. The Albums,
/// Songs, and Playlists views do — the Back button is shown above their
/// lists — while the top-level Artists list has nothing to go back to.
///
/// `#[must_use]`: a discarded result is a logic bug — the caller that decides
/// whether to show the Back button would silently never show it.
#[must_use]
pub fn can_go_back(view: &CurrentView) -> bool {
    !matches!(view, CurrentView::Artists)
}

/// Whether the Artists view should offer its Retry button. Only the
/// top-level artists fetch is unrecoverable by navigation, so Retry belongs
/// to the Artists view and only while that fetch has failed; the Albums and
/// Songs levels recover by navigating back into them. The update loop's
/// `LoadArtists` arm clears the failure and re-issues the fetch.
///
/// `#[must_use]`: a discarded result is a logic bug — the caller that decides
/// whether to show the Retry button would silently never show it.
#[must_use]
pub fn can_retry_artists(view: &CurrentView, error: Option<&str>) -> bool {
    matches!(view, CurrentView::Artists) && error.is_some()
}

/// The browse view's Back button, stepping the hierarchy one level up (Songs
/// → Albums → Artists). Rendered only where [`can_go_back`] is true; the
/// update loop turns the pressed message into the view change.
pub fn view_back_button() -> Element<'static, Message> {
    labeled_button("Back", Message::Back).into()
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
