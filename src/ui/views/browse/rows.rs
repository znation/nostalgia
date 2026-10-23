//! Browse row mapping and list construction: the placeholder wording, the row
//! prefix and current-row highlight, the scrollable list, the per-level row
//! mappings, and the count and playing-time footers.
//!
//! These are the pure pieces the public browse views in the parent module
//! assemble. They turn `&[Artist]` / `&[Album]` / `&[Song]` / `&[Playlist]` or
//! the rows they map to into widgets, and know nothing about `WinampPlayer`
//! or the update loop.

use std::borrow::Cow;

use iced::{
    Background, Element, Length,
    widget::{Button, Column, Row, Scrollable, Text, button},
};

use crate::library::{Album, Artist, Playlist, Song};
use crate::ui::{CurrentView, Message, style, theme};

use super::super::{format_minutes_seconds, format_track_time, spacer};

/// The placeholder a browse view shows when its list has no rows: one wording
/// per browse level, so an empty Artists, Albums, Songs, All Songs, All
/// Albums, or Playlists panel names the list rather than showing a blank
/// panel. Pure so the per-level wording is testable without an iced renderer,
/// like the button label helpers below.
pub(in crate::ui::views) fn empty_list_label(view: &CurrentView) -> &'static str {
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
pub(in crate::ui::views) fn browse_placeholder<'a>(
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
pub(in crate::ui::views) fn current_row_style(status: button::Status) -> button::Style {
    let mut row = style::playlist_row_style(status);
    row.background = Some(Background::Color(theme::PLAYING_ROW_HIGHLIGHT));
    row
}

/// The prefix the browse list puts before a row's title: its 1-based
/// position and a period, with a `▶ ` marker first when the row is the
/// currently playing track. Pure, so the marker/number order is testable
/// without an iced renderer and the composition lives in one place.
pub(in crate::ui::views) fn row_prefix(position: usize, is_current: bool) -> String {
    let number = position + 1;
    if is_current {
        format!("▶ {number}. ")
    } else {
        format!("{number}. ")
    }
}

/// The one browse [`Scrollable`]'s widget id: the update loop's snap-to-top
/// operation targets it, so every view assembled by [`scrollable_list`] can be
/// returned to its first row by [`Message::ScrollBrowseToTop`].
pub const BROWSE_LIST_ID: &str = "nostalgia-browse-list";

/// Builds a scrollable list where each item is a button showing its 1-based
/// position, a title, and a secondary label, emitting the given message on
/// press. Shared by every browse list view. The last tuple element marks the
/// currently playing row: its prefix carries a `▶` marker first and it gets a
/// highlighted background so the list reads as a playlist. Only `song_row`
/// ever sets the flag to true.
///
/// When `items` yields nothing, the list renders `empty_label` in place of a
/// blank panel. The caller picks the wording via [`browse_placeholder`],
/// which distinguishes a fetch failure, a buffer no reply has populated yet
/// (still loading), and one that loaded empty and names the list.
///
/// The primary title and the secondary label are both [`Cow`]s: a song row's
/// title and length are borrowed from the loaded list's caches in the common
/// case and only composed on the fallback path, while the artist, album, and
/// playlist rows borrow their names and static hints. This builder runs on
/// every view refresh, so borrowing avoids a `String` allocation per row per
/// frame. The row prefix is one owned `String` per row per refresh,
/// since it depends on the row's position; it is not cached in `BrowseList`
/// because that would need invalidating on every store for no correctness
/// gain.
pub(in crate::ui::views) fn scrollable_list<'a>(
    items: impl IntoIterator<Item = (Cow<'a, str>, Cow<'a, str>, Message, bool)>,
    empty_label: &'a str,
) -> Element<'a, Message> {
    let mut column = Column::new().padding(20);
    let mut is_empty = true;

    for (position, (title, label, message, is_current)) in items.into_iter().enumerate() {
        is_empty = false;
        let button = Button::new(
            Row::new()
                .push(Text::new(row_prefix(position, is_current)).size(18))
                .push(Text::new(title).size(18))
                .push(spacer(10.0))
                .push(Text::new(label).size(14)),
        )
        .on_press(message)
        .style(|_theme, status| style::playlist_row_style(status));
        // The current row keeps the playlist row's text and hover/press face
        // and only swaps the background for the highlight color.
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
        .id(BROWSE_LIST_ID)
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
pub(in crate::ui::views) fn artist_row<'a>(
    epoch: u64,
    index: usize,
    artist: &'a Artist,
) -> (Cow<'a, str>, Cow<'a, str>, Message, bool) {
    (
        Cow::Borrowed(artist.name.as_str()),
        Cow::Borrowed("View Albums"),
        Message::ArtistSelected { epoch, index },
        false,
    )
}

/// The browse row an album becomes: its display title (`<artist> - <title>`,
/// or the title alone when the artist is empty; see [`Album::display_title`]),
/// the "View Songs" hint, the message emitted when it is pressed, and the
/// current-track flag. `epoch`
/// and `index` are the loaded-list epoch and the album's position in that
/// list, carried so building a row never clones the album's id (see
/// [`artist_row`]). Albums are never the currently playing row, so the flag
/// is always false.
pub(in crate::ui::views) fn album_row<'a>(
    epoch: u64,
    index: usize,
    album: &'a Album,
) -> (Cow<'a, str>, Cow<'a, str>, Message, bool) {
    (
        album.display_title(),
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
pub(in crate::ui::views) fn playlist_row<'a>(
    epoch: u64,
    index: usize,
    playlist: &'a Playlist,
) -> (Cow<'a, str>, Cow<'a, str>, Message, bool) {
    (
        Cow::Borrowed(playlist.name.as_str()),
        Cow::Borrowed("View Songs"),
        Message::PlaylistSelected { epoch, index },
        false,
    )
}

/// The browse row a song becomes: its display title (`<artist> - <title>`,
/// or the title alone when the artist is empty; see [`Song::display_title`]),
/// the track's formatted length ([`format_track_time`]) as the secondary
/// label, the message emitted when it is pressed, and whether it is the
/// currently playing track (true only when `current_track` names this song).
/// `epoch` and `index` are the loaded-list epoch and the song's position in
/// that list, carried so building a row never clones the song's id (see
/// [`artist_row`]). The flag feeds `scrollable_list`'s `▶` + highlight
/// marker, so the song list reads as a playlist.
///
/// `cached_title` and `cached_label` are the title and length the loaded list
/// stored for this row (see `BrowseList::store_with_row_text`), borrowed so a
/// refresh of a large library does not compose and allocate a fresh `String`
/// per track. `None` — a list stored without caches, as a test may do — falls
/// back to composing the title and formatting the duration for this row, so
/// the rendered row is unchanged.
pub(in crate::ui::views) fn song_row<'a>(
    epoch: u64,
    index: usize,
    song: &'a Song,
    cached_title: Option<&'a str>,
    cached_label: Option<&'a str>,
    current_track: Option<&str>,
) -> (Cow<'a, str>, Cow<'a, str>, Message, bool) {
    let title = match cached_title {
        Some(title) => Cow::Borrowed(title),
        None => song.display_title(),
    };
    let label = match cached_label {
        Some(label) => Cow::Borrowed(label),
        None => Cow::Owned(format_track_time(song.duration_ms)),
    };
    (
        title,
        label,
        Message::TrackSelected { epoch, index },
        Some(song.id.as_str()) == current_track,
    )
}

/// Builds an album list: the shared body of [`super::view_albums`] and
/// [`super::view_all_albums`], mapping each album to an [`album_row`] by its
/// position in `albums` and carrying `epoch`. `placeholder` is the caller's
/// [`browse_placeholder`] wording for an empty list. A non-empty list also
/// appends its item-count footer ([`list_with_count`]).
pub(in crate::ui::views) fn album_list<'a>(
    albums: &'a [Album],
    epoch: u64,
    placeholder: &'a str,
) -> Element<'a, Message> {
    list_with_count(
        scrollable_list(
            albums
                .iter()
                .enumerate()
                .map(|(index, album)| album_row(epoch, index, album)),
            placeholder,
        ),
        albums.len(),
        "album",
        "albums",
    )
}

/// The item-count footer for a loaded artist, album, or playlist list:
/// `"<count> <noun>"`, with `singular` chosen exactly when `count == 1` and
/// `plural` otherwise. Unlike [`song_list_summary`] it has no time total and
/// formats any count, including `0`, so the wording is testable without an
/// iced renderer.
pub(in crate::ui::views) fn list_count_summary(
    count: usize,
    singular: &str,
    plural: &str,
) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("{count} {noun}")
}

/// Appends a loaded list's footer line beneath it, filling the browse panel.
/// The shared layout of the item-count footer ([`list_with_count`]) and the
/// song list's count-and-time footer ([`song_list_summary`]). Callers skip
/// this for an empty list, whose loading, error, or empty placeholder must
/// stay alone beneath the rows it replaces.
fn with_footer<'a>(list: Element<'a, Message>, footer: String) -> Element<'a, Message> {
    Column::new()
        .push(list)
        .push(Text::new(footer).size(14))
        .width(Length::Fill)
        .height(Length::FillPortion(3))
        .into()
}

/// Wraps a non-song browse list in a footer naming how many rows it holds,
/// matching the song list's count-and-time footer above it. A zero count
/// returns `list` unchanged: the list is then showing its loading, error, or
/// empty placeholder, and a `0 artists` footer beneath that wording would
/// describe rows the panel does not have.
pub(in crate::ui::views) fn list_with_count<'a>(
    list: Element<'a, Message>,
    count: usize,
    singular: &str,
    plural: &str,
) -> Element<'a, Message> {
    if count == 0 {
        return list;
    }
    with_footer(list, list_count_summary(count, singular, plural))
}

/// The count-and-time footer beneath a loaded song list: `"<count> song(s) -
/// <total>"`, with the singular `song` for a count of one and the total from
/// [`format_minutes_seconds`] (whose zero total is a real `"0:00"`, not the
/// per-track `"--:--"` placeholder). Every entry counts, including one whose
/// `duration_ms` is `0`; the durations are summed with `u64::saturating_add`
/// so a large list cannot overflow the total. Pure so the wording and
/// arithmetic are testable without an iced renderer.
pub(in crate::ui::views) fn song_list_summary(songs: &[Song]) -> String {
    let total_ms = songs
        .iter()
        .fold(0u64, |total, song| total.saturating_add(song.duration_ms));
    let noun = if songs.len() == 1 { "song" } else { "songs" };
    format!(
        "{} {} - {}",
        songs.len(),
        noun,
        format_minutes_seconds(total_ms)
    )
}

/// Builds a song list: the shared body of [`super::view_songs`] and
/// [`super::view_all_songs`], mapping each song to a [`song_row`] by its position in
/// `songs` and pairing it with the cached title and length at the same index
/// (see [`song_row`]). `placeholder` is the caller's [`browse_placeholder`]
/// wording for an empty list; it differs between the two callers because only
/// a search reads as "no matches". A non-empty list also appends the
/// count-and-time footer ([`song_list_summary`]) beneath its rows; an empty
/// list returns only the scrollable so its loading, error, or empty
/// placeholder stays alone.
pub(in crate::ui::views) fn song_list<'a>(
    songs: &'a [Song],
    titles: &'a [String],
    labels: &'a [String],
    epoch: u64,
    current_track: Option<&str>,
    placeholder: &'a str,
) -> Element<'a, Message> {
    let list = scrollable_list(
        songs.iter().enumerate().map(|(index, song)| {
            song_row(
                epoch,
                index,
                song,
                titles.get(index).map(String::as_str),
                labels.get(index).map(String::as_str),
                current_track,
            )
        }),
        placeholder,
    );
    if songs.is_empty() {
        return list;
    }
    with_footer(list, song_list_summary(songs))
}
