//! Pure text resolution for the browse rows: the empty and loading placeholder
//! wording, the playlist number prefix, the artist/album/playlist/song row
//! labels, and the item-count and playing-time footers. Everything here is
//! data in, string (or row tuple) out, so it is testable without an iced
//! renderer; the widgets that draw these values live in [`super`].

use std::borrow::Cow;

use crate::library::{Album, Artist, Playlist, Song};
use crate::ui::CurrentView;
use crate::ui::views::{format_minutes_seconds, format_track_time};

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
        CurrentView::RecentlyAdded => "No recently added albums",
        CurrentView::Playlists => "No playlists",
    }
}

/// The placeholder a browse view shows when its list has no rows, choosing
/// between a fetch failure, "still loading", and "loaded, but empty".
///
/// `error` is the list's most recent fetch-failure report (see
/// `crate::ui::browse::BrowseList::fail`/`crate::ui::browse::BrowseList::clear`):
/// when present it wins, so a backend failure is shown rather than misread as
/// an empty library.
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

/// The browse row an artist becomes: its name, its album count (``"1 album"``,
/// ``"2 albums"``, or no label when it has none), and the current-track flag.
/// Artists are never the currently playing row, so the flag is always false.
/// The press message is [`crate::ui::Message::SelectRow`], built by [`super::scrollable_list`]
/// from the row's position and its list's epoch, so building a row never
/// clones the artist's id — the update loop resolves the index back to an id
/// only when the row is actually opened.
///
/// `cached_label` is the album-count label the loaded list stored for this row
/// (see `BrowseList::store_with_labels`), borrowed so a refresh of a large
/// artist list does not format and allocate the same count again. `None` — a
/// list stored without a cache, as a test may do — falls back to composing the
/// label with [`album_count_label`], so the rendered row is unchanged.
pub(in crate::ui::views) fn artist_row<'a>(
    artist: &'a Artist,
    cached_label: Option<&'a str>,
) -> (Cow<'a, str>, Cow<'a, str>, bool) {
    let label = match cached_label {
        Some(label) => Cow::Borrowed(label),
        None => album_count_label(artist.album_count),
    };
    (Cow::Borrowed(artist.name.as_str()), label, false)
}

/// The secondary label an artist row shows for its album count: no label for
/// `0` (the mockup's artist with no albums shows no count), `"1 album"` for
/// one, and `"N albums"` otherwise. The empty label borrows so the common
/// no-albums row allocates nothing.
pub(in crate::ui::views) fn album_count_label(count: usize) -> Cow<'static, str> {
    match count {
        0 => Cow::Borrowed(""),
        1 => Cow::Borrowed("1 album"),
        count => Cow::Owned(format!("{count} albums")),
    }
}

/// The browse row an album becomes: its display title (`<artist> - <title>`,
/// or the title alone when the artist is empty; see [`Album::display_title`]),
/// its total running time ([`format_track_time`], or `--:--` when the source
/// supplied no songs), and the current-track flag. Albums are never the
/// currently playing row, so the flag is always false. [`super::scrollable_list`]
/// supplies the press message from the row's position and list epoch.
///
/// `cached_title` and `cached_label` are the title and running time the loaded
/// list stored for this row (see `BrowseList::store_with_row_text`), borrowed
/// so a refresh of a large album library does not re-join every artist and
/// title and reformat every duration. `None` — a list stored without caches,
/// as a test may do — falls back to composing the title and formatting the
/// duration for this row, so the rendered row is unchanged.
pub(in crate::ui::views) fn album_row<'a>(
    album: &'a Album,
    cached_title: Option<&'a str>,
    cached_label: Option<&'a str>,
) -> (Cow<'a, str>, Cow<'a, str>, bool) {
    let title = match cached_title {
        Some(title) => Cow::Borrowed(title),
        None => album.display_title(),
    };
    let label = match cached_label {
        Some(label) => Cow::Borrowed(label),
        None => Cow::Owned(format_track_time(album.duration_ms)),
    };
    (title, label, false)
}

/// The browse row a playlist becomes: its name, the "View Songs" hint, and
/// the current-track flag. Playlists are never the currently playing row, so
/// the flag is always false. [`super::scrollable_list`] supplies the press message
/// from the row's position and list epoch.
pub(in crate::ui::views) fn playlist_row<'a>(
    playlist: &'a Playlist,
) -> (Cow<'a, str>, Cow<'a, str>, bool) {
    (
        Cow::Borrowed(playlist.name.as_str()),
        Cow::Borrowed("View Songs"),
        false,
    )
}

/// The browse row a song becomes: its display title (`<artist> - <title>`,
/// or the title alone when the artist is empty; see [`Song::display_title`]),
/// the track's formatted length ([`format_track_time`]) as the secondary
/// label, and whether it is the currently playing track (true only when
/// `current_track` names this song). The flag feeds [`super::scrollable_list`]'s `▶`
/// prefix and white text, so the song list reads as a playlist; the press message
/// is that builder's [`crate::ui::Message::SelectRow`], not this row's.
///
/// `cached_title` and `cached_label` are the title and length the loaded list
/// stored for this row (see `BrowseList::store_with_row_text`), borrowed so a
/// refresh of a large library does not compose and allocate a fresh `String`
/// per track. `None` — a list stored without caches, as a test may do — falls
/// back to composing the title and formatting the duration for this row, so
/// the rendered row is unchanged.
pub(in crate::ui::views) fn song_row<'a>(
    song: &'a Song,
    cached_title: Option<&'a str>,
    cached_label: Option<&'a str>,
    current_track: Option<&str>,
) -> (Cow<'a, str>, Cow<'a, str>, bool) {
    let title = match cached_title {
        Some(title) => Cow::Borrowed(title),
        None => song.display_title(),
    };
    let label = match cached_label {
        Some(label) => Cow::Borrowed(label),
        None => Cow::Owned(format_track_time(song.duration_ms)),
    };
    (title, label, Some(song.id.as_str()) == current_track)
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

/// The count-and-time footer beneath a loaded song list: `"<count> song(s) -
/// <total>"`, or `"<count> song(s) - <selected>/<total>"` while `selected`
/// indexes a song in the list (an index past the list falls back to the plain
/// form), with the singular `song` for a
/// count of one and the total from [`format_minutes_seconds`] (whose zero total
/// is a real `"0:00"`, not the per-track `"--:--"` placeholder). Every entry
/// counts, including one whose `duration_ms` is `0` (no known length): a total
/// covering such an entry, and an unknown-length selection, get a trailing `+`
/// like Winamp's (`3:30+`, `0:00+`). The durations are summed
/// with `u64::saturating_add` so a large list cannot overflow the total. Pure so
/// the wording and arithmetic are testable without an iced renderer.
pub(in crate::ui::views) fn song_list_summary(songs: &[Song], selected: Option<usize>) -> String {
    let total_ms = songs
        .iter()
        .fold(0u64, |total, song| total.saturating_add(song.duration_ms));
    let noun = if songs.len() == 1 { "song" } else { "songs" };
    let mark = |known: bool, ms: u64| {
        let time = format_minutes_seconds(ms);
        if known { time } else { format!("{time}+") }
    };
    let total = mark(songs.iter().all(|song| song.duration_ms != 0), total_ms);
    match selected.and_then(|index| songs.get(index)) {
        Some(song) => format!(
            "{} {} - {}/{}",
            songs.len(),
            noun,
            mark(song.duration_ms != 0, song.duration_ms),
            total
        ),
        None => format!("{} {} - {}", songs.len(), noun, total),
    }
}
