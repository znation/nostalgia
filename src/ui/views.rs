//! Widget construction for the library browser.
//!
//! Pure functions: each turns plain data (`&[Artist]`, `&[Album]`,
//! `&[Song]`) into an `Element` and knows nothing about the player's
//! state or update loop. Keeping them free of the `WinampPlayer` struct
//! means the view layer can be reworked (or tested) independently of how
//! the app is booted.

use iced::{
    Element, Length,
    widget::{Button, Column, Row, Scrollable, Space, Text},
};

use crate::library::{Album, Artist, Song};

use super::Message;

/// Builds a scrollable list where each item is a button showing a title
/// followed by a secondary label, emitting the given message on press.
/// Shared by the artists, albums, and songs views.
fn scrollable_list(
    items: impl IntoIterator<Item = (String, &'static str, Message)>,
) -> Element<'static, Message> {
    let mut column = Column::new().padding(20);

    for (title, label, message) in items {
        column = column.push(
            Button::new(
                Row::new()
                    .push(Text::new(title).size(18))
                    .push(Space::new().width(Length::Fixed(10.0)))
                    .push(Text::new(label).size(14)),
            )
            .on_press(message),
        );
    }

    Scrollable::new(column)
        .width(Length::Fill)
        .height(Length::FillPortion(3))
        .into()
}

/// The browse row an artist becomes: its name, the "View Albums" hint, and
/// the message emitted when it is pressed. Kept out of `view_artists` so the
/// title/label/selection contract is testable without an iced renderer.
fn artist_row(artist: &Artist) -> (String, &'static str, Message) {
    (
        artist.name.clone(),
        "View Albums",
        Message::ArtistSelected(artist.id.clone()),
    )
}

/// The browse row an album becomes: its title, the "View Songs" hint, and
/// the message emitted when it is pressed.
fn album_row(album: &Album) -> (String, &'static str, Message) {
    (
        album.title.clone(),
        "View Songs",
        Message::AlbumSelected(album.id.clone()),
    )
}

/// The browse row a song becomes: its title, the "Play" hint, and the
/// message emitted when it is pressed.
fn song_row(song: &Song) -> (String, &'static str, Message) {
    (
        song.title.clone(),
        "Play",
        Message::TrackSelected(song.id.clone()),
    )
}

pub fn view_artists(artists: &[Artist]) -> Element<'_, Message> {
    scrollable_list(artists.iter().map(artist_row))
}

pub fn view_albums(albums: &[Album]) -> Element<'_, Message> {
    scrollable_list(albums.iter().map(album_row))
}

pub fn view_songs(songs: &[Song]) -> Element<'_, Message> {
    scrollable_list(songs.iter().map(song_row))
}

#[cfg(test)]
mod tests {
    use super::{Message, album_row, artist_row, song_row};
    use crate::library::{Album, Artist, Song};

    fn sample_artist() -> Artist {
        Artist {
            id: "artist-1".to_string(),
            name: "The Sample Band".to_string(),
        }
    }

    fn sample_album() -> Album {
        Album {
            id: "album-1".to_string(),
            title: "First Record".to_string(),
            artist_id: "artist-1".to_string(),
        }
    }

    fn sample_song() -> Song {
        Song {
            id: "song-1".to_string(),
            title: "Opening".to_string(),
            album_id: "album-1".to_string(),
        }
    }

    // Each row maps one library entry to the (title, secondary label, press
    // message) tuple that `scrollable_list` renders as a button. Wrong label
    // text or a swapped selection message would silently break the browse UI,
    // so the mapping is the contract these tests pin down.

    #[test]
    fn artist_row_uses_name_and_selects_the_artist() {
        let (title, label, message) = artist_row(&sample_artist());
        assert_eq!(title, "The Sample Band");
        assert_eq!(label, "View Albums");
        assert!(matches!(message, Message::ArtistSelected(id) if id == "artist-1"));
    }

    #[test]
    fn album_row_uses_title_and_selects_the_album() {
        let (title, label, message) = album_row(&sample_album());
        assert_eq!(title, "First Record");
        assert_eq!(label, "View Songs");
        assert!(matches!(message, Message::AlbumSelected(id) if id == "album-1"));
    }

    #[test]
    fn song_row_uses_title_and_selects_the_song() {
        let (title, label, message) = song_row(&sample_song());
        assert_eq!(title, "Opening");
        assert_eq!(label, "Play");
        assert!(matches!(message, Message::TrackSelected(id) if id == "song-1"));
    }
}
