//! Browse row and list construction: the row coloring, the widget id and row
//! geometry, the scrollable list that lays out numbered rows, and the list
//! builders with their count and playing-time footers.
//!
//! These are the widget pieces the public browse views in the parent module
//! assemble. The pure wording, row-label, and summary resolution they draw
//! lives in the [`text`] submodule; this module knows nothing about
//! `WinampPlayer` or the update loop.

use std::borrow::Cow;

use iced::{
    Background, Element, Length,
    widget::{Button, Column, Row, Scrollable, Text, button, container, text::Wrapping},
};

use crate::library::{Album, Song};
use crate::ui::{CurrentView, Message, style, theme};

use super::super::spacer;

mod lazy;
pub(in crate::ui::views) mod text;

// Shared with the Jump to File panel's match list, so both heavy row lists use
// the same build-on-key-change cache.
pub(in crate::ui::views) use self::lazy::LazyRow;

use self::text::{
    album_list_summary, album_row, list_count_summary, row_prefix, song_list_summary, song_row,
};

/// The style for a browse row, colored by whether it is playing or selected.
///
/// Starts from the flat [`style::playlist_row_style`], so an ordinary row is
/// green on the near-black well and still lifts and sinks under the cursor.
/// A selected row draws the blue [`theme::ROW_HIGHLIGHT`] bar in every
/// hover/press state; a playing or selected row draws white
/// [`theme::PLAYING_ROW_TEXT`]. A row that is both playing and selected keeps
/// the bar and stays identifiable by its `▶` prefix. Pure, so the colors —
/// which iced's `Element` API gives no way to read back from the built
/// `Button`, and whose closure runs only at render time — are testable.
pub(in crate::ui::views) fn browse_row_style(
    status: button::Status,
    is_current: bool,
    is_selected: bool,
) -> button::Style {
    let mut row = style::playlist_row_style(status);
    if is_selected {
        row.background = Some(Background::Color(theme::ROW_HIGHLIGHT));
    }
    if is_current || is_selected {
        row.text_color = theme::PLAYING_ROW_TEXT;
    }
    row
}

/// The one browse [`Scrollable`]'s widget id: the update loop's snap-to-top
/// operation targets it, so every view assembled by [`scrollable_list`] can be
/// returned to its first row by [`Message::ScrollBrowseToTop`], its
/// scroll-to-row operation targets it to bring a stepped-to row into view, and
/// its page-scroll operation targets it to move one keyboard page at a time.
pub const BROWSE_LIST_ID: &str = "nostalgia-browse-list";

/// The padding [`scrollable_list`] puts around its rows. Row `i`'s top sits
/// `BROWSE_LIST_PADDING + i * BROWSE_ROW_HEIGHT` from the list's top, so the
/// transport's scroll-to-playing-row operation includes this padding in its
/// absolute offset.
pub const BROWSE_LIST_PADDING: f32 = 20.0;

/// The vertical pitch between consecutive laid-out browse rows: the row
/// button's padding plus its 18px text height. [`scrollable_list`] stacks the
/// rows in a plain [`Column`], flattens line breaks, and clips each title to
/// its column, so each row is exactly this tall. Measured by
/// `browse_row_height_matches_the_laid_out_row_pitch`, which lays out the root
/// view with an overlong title and checks consecutive row prefixes land
/// exactly this far apart, so a styling change cannot leave a stale constant
/// behind.
pub const BROWSE_ROW_HEIGHT: f32 = 33.4;

/// The distance one keyboard page scrolls the browse list: eight rows of the
/// same pitch as [`BROWSE_ROW_HEIGHT`]. `Page Up` and `Page Down` scroll by
/// this amount rather than by the viewport height, so a page step is stable
/// across window sizes; iced clamps the result to the list's bounds.
pub const BROWSE_PAGE_HEIGHT: f32 = 8.0 * BROWSE_ROW_HEIGHT;

/// Flattens the line breaks in a browse row's `title` to spaces. cosmic-text
/// starts a new `BufferLine` at `\n`, `\r\n`, or a lone `\r` even with
/// wrapping disabled, so a title carrying one would make its row taller than
/// [`BROWSE_ROW_HEIGHT`]; the scroll-to-row offset counts on every row having
/// exactly that pitch.
fn one_line_title<'a>(title: Cow<'a, str>) -> Cow<'a, str> {
    if title.contains('\n') || title.contains('\r') {
        Cow::Owned(
            title
                .chars()
                .map(|character| match character {
                    '\n' | '\r' => ' ',
                    other => other,
                })
                .collect(),
        )
    } else {
        title
    }
}

/// Builds a scrollable list where each item is a button showing its 1-based
/// position, a title, and a secondary label pinned to the row's right edge.
/// The tuple's final element marks the currently playing row: its prefix
/// carries a `▶` marker first and its text is white, without a background bar
/// of its own, so the list reads as a playlist. Only the song row mapping ever sets the
/// flag to true.
///
/// Every row button emits [`Message::SelectRow`] with the list's `epoch` and
/// the row's position: a single press only selects the row (drawing the blue
/// [`theme::ROW_HIGHLIGHT`] bar with white text when `selected` names
/// it), and a second press on the selected row or the Open button dispatches
/// the existing open action. The `▶` prefix stays tied to `is_current`, so a
/// row that is both playing and selected keeps the bar and is still
/// identifiable as the playing one.
///
/// A title carrying a line break is flattened to one line, and a title too
/// long for its column is clipped there, so it can neither grow the row past
/// [`BROWSE_ROW_HEIGHT`] nor draw over the right-aligned label.
///
/// When `items` yields nothing, the list renders `empty_label` in place of a
/// blank panel. The caller picks the wording via [`text::browse_placeholder`],
/// which distinguishes a fetch failure, a buffer no reply has populated yet
/// (still loading), and one that loaded empty and names the list.
///
/// The primary title and the secondary label are both [`Cow`]s: a song or
/// album row's title and label are borrowed from the loaded list's caches in
/// the common case and only composed on the fallback path, while a playlist
/// row borrows its name and static hint. This builder runs on every view
/// refresh, so borrowing avoids a `String` allocation per row per refresh.
/// Each row's built button — including its prefix and the owned copies of its
/// title and label — is cached by [`RowKey`] and rebuilt only when that key
/// changes, so a playback poll reuses the row instead of reconstructing it.
/// The key does not rescan the row's text: the list's epoch is the text's
/// revision, because [`crate::ui::browse::BrowseList`] bumps it whenever it
/// stores or reorders rows with their cached titles and labels. Hashing every
/// title and label again each refresh would read the whole list's text for a
/// key that has not changed.
pub(in crate::ui::views) fn scrollable_list<'a>(
    items: impl IntoIterator<Item = (Cow<'a, str>, Cow<'a, str>, bool)>,
    list: &CurrentView,
    epoch: u64,
    selected: Option<usize>,
    empty_label: &'a str,
) -> Element<'a, Message> {
    let mut column = Column::new().padding(BROWSE_LIST_PADDING);
    let mut is_empty = true;

    for (position, (title, label, is_current)) in items.into_iter().enumerate() {
        is_empty = false;
        let is_selected = selected == Some(position);
        // The built row is cached across views by its dependency: the list
        // browse level, epoch, the row's position and flags. The epoch is the row text's
        // revision too — `BrowseList` bumps it on every store and reorder,
        // together with the cached titles and labels — so this key does not
        // rescan the text each refresh.
        let key = RowKey {
            list: std::mem::discriminant(list),
            epoch,
            position,
            is_current,
            is_selected,
        };
        let row = LazyRow::new(key, move |key| {
            row_button(
                title.to_string(),
                label.to_string(),
                key.epoch,
                key.position,
                key.is_current,
                key.is_selected,
            )
        });
        // Unselected rows are green, the playing row is white without a bar,
        // and the selected row draws the blue bar with white text; the prefix
        // already distinguishes the current track.
        column = column.push(row);
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

/// The cache key for one browse row: what changes the widget it builds. The
/// browse level identifies the list, and its epoch revises its cached titles and labels;
/// `BrowseList` bumps it on every store and reorder, so a text change always
/// rebuilds the affected rows without rescanning their text here.
#[derive(Clone, Copy, Hash)]
struct RowKey {
    list: std::mem::Discriminant<CurrentView>,
    epoch: u64,
    position: usize,
    is_current: bool,
    is_selected: bool,
}

// Counts browse row builds on the current test thread, so a test can prove
// the real list reuses cached rows instead of rebuilding them.
#[cfg(test)]
thread_local! {
    static ROW_BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many browse rows the current test thread has built. Test-only.
#[cfg(test)]
pub(in crate::ui::views) fn row_build_count() -> usize {
    ROW_BUILDS.with(std::cell::Cell::get)
}

/// Builds one browse row button from its owned text and flags. `LazyRow`
/// calls this only when the row's [`RowKey`] changes, so the strings are owned
/// by the cached element and the row can outlive the view that first built it.
fn row_button(
    title: String,
    label: String,
    epoch: u64,
    position: usize,
    is_current: bool,
    is_selected: bool,
) -> Element<'static, Message> {
    #[cfg(test)]
    ROW_BUILDS.with(|count| count.set(count.get() + 1));

    Button::new(
        Row::new()
            .width(Length::Fill)
            .push(Text::new(row_prefix(position, is_current)).size(18))
            // A wrapped title would make its row taller than
            // BROWSE_ROW_HEIGHT and break the absolute scroll offset.
            // The fill container clips a title wider than its column so
            // it cannot draw under the right-aligned label.
            .push(
                container(
                    Text::new(one_line_title(Cow::Owned(title)))
                        .size(18)
                        .wrapping(Wrapping::None),
                )
                .width(Length::Fill)
                .clip(true),
            )
            .push(spacer(10.0))
            .push(Text::new(label).size(14)),
    )
    .width(Length::Fill)
    .on_press(Message::SelectRow {
        epoch,
        index: position,
    })
    .style(move |_theme, status| browse_row_style(status, is_current, is_selected))
    .into()
}

/// Builds an album list: the shared body of [`super::view_albums`] and
/// [`super::view_all_albums`], mapping each album to an [`album_row`] by its
/// position in `albums` and pairing it with the cached title and running time
/// at the same index (see [`album_row`]). `epoch` and `selected` reach
/// [`scrollable_list`], so every row press selects and the selected row draws
/// the blue bar. `placeholder` is the caller's [`text::browse_placeholder`] wording
/// for an empty list. A non-empty list also appends its count-and-time footer
/// ([`album_list_summary`]).
pub(in crate::ui::views) fn album_list<'a>(
    albums: &'a [Album],
    titles: &'a [String],
    labels: &'a [String],
    list: &CurrentView,
    epoch: u64,
    selected: Option<usize>,
    placeholder: &'a str,
) -> Element<'a, Message> {
    let rows = scrollable_list(
        albums.iter().enumerate().map(|(index, album)| {
            album_row(
                album,
                titles.get(index).map(String::as_str),
                labels.get(index).map(String::as_str),
            )
        }),
        list,
        epoch,
        selected,
        placeholder,
    );
    if albums.is_empty() {
        return rows;
    }
    with_footer(rows, album_list_summary(albums))
}

/// Appends a loaded list's footer line beneath it, filling the browse panel.
/// The shared layout of the item-count footer ([`list_with_count`]) and the
/// count-and-total footer of the Artists list ([`super::view_artists`]) and the
/// count-and-time footers of the album ([`album_list_summary`]), song
/// ([`song_list_summary`]), and Genres, Years, and Composers
/// ([`crate::ui::song_groups::group_list_summary`]) lists. Callers skip
/// this for an empty list, whose loading, error, or empty placeholder must
/// stay alone beneath the rows it replaces.
pub(in crate::ui::views) fn with_footer<'a>(
    list: Element<'a, Message>,
    footer: String,
) -> Element<'a, Message> {
    Column::new()
        .push(list)
        .push(Text::new(footer).size(14))
        .width(Length::Fill)
        .height(Length::FillPortion(3))
        .into()
}

/// Wraps a playlist list in a footer naming how many rows it holds; the other
/// lists use their count-and-total or count-and-time footers instead. A zero count
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

/// Builds a song list: the shared body of [`super::view_songs`] and
/// [`super::view_all_songs`], mapping each song to a [`song_row`] by its
/// position in `songs` and pairing it with the cached title and length at the
/// same index (see [`song_row`]). `epoch` and `selected` reach
/// [`scrollable_list`], so every row press selects and the selected row draws
/// the blue bar. `placeholder` is the caller's [`text::browse_placeholder`]
/// wording for an empty list; it differs between the two callers because only a
/// search reads as "no matches". A non-empty list also appends the count-and-time
/// footer ([`song_list_summary`], which includes the selected row's length)
/// beneath its rows; an empty list returns only the scrollable so its loading,
/// error, or empty placeholder stays alone.
#[allow(clippy::too_many_arguments)]
pub(in crate::ui::views) fn song_list<'a>(
    songs: &'a [Song],
    titles: &'a [String],
    labels: &'a [String],
    list: &CurrentView,
    epoch: u64,
    selected: Option<usize>,
    current_track: Option<&str>,
    placeholder: &'a str,
) -> Element<'a, Message> {
    let list = scrollable_list(
        songs.iter().enumerate().map(|(index, song)| {
            song_row(
                song,
                titles.get(index).map(String::as_str),
                labels.get(index).map(String::as_str),
                current_track,
            )
        }),
        list,
        epoch,
        selected,
        placeholder,
    );
    if songs.is_empty() {
        return list;
    }
    with_footer(list, song_list_summary(songs, selected))
}
