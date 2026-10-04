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

use crate::apple_music::{Album, Artist, Song};

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

pub fn view_artists(artists: &[Artist]) -> Element<'_, Message> {
    scrollable_list(artists.iter().map(|artist| {
        (
            artist.name.clone(),
            "View Albums",
            Message::ArtistSelected(artist.id.clone()),
        )
    }))
}

pub fn view_albums(albums: &[Album]) -> Element<'_, Message> {
    scrollable_list(albums.iter().map(|album| {
        (
            album.title.clone(),
            "View Songs",
            Message::AlbumSelected(album.id.clone()),
        )
    }))
}

pub fn view_songs(songs: &[Song]) -> Element<'_, Message> {
    scrollable_list(songs.iter().map(|song| {
        (
            song.title.clone(),
            "Play",
            Message::TrackSelected(song.id.clone()),
        )
    }))
}
