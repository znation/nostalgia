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

pub fn view_artists(artists: &[Artist]) -> Element<'_, Message> {
    let mut column = Column::new().padding(20);

    for artist in artists {
        column = column.push(
            Button::new(
                Row::new()
                    .push(Text::new(&artist.name).size(18))
                    .push(Space::new().width(Length::Fixed(10.0)))
                    .push(Text::new("View Albums").size(14)),
            )
            .on_press(Message::ArtistSelected(artist.id.clone())),
        );
    }

    Scrollable::new(column)
        .width(Length::Fill)
        .height(Length::FillPortion(3))
        .into()
}

pub fn view_albums(albums: &[Album]) -> Element<'_, Message> {
    let mut column = Column::new().padding(20);

    for album in albums {
        column = column.push(
            Button::new(
                Row::new()
                    .push(Text::new(&album.title).size(18))
                    .push(Space::new().width(Length::Fixed(10.0)))
                    .push(Text::new("View Songs").size(14)),
            )
            .on_press(Message::AlbumSelected(album.id.clone())),
        );
    }

    Scrollable::new(column)
        .width(Length::Fill)
        .height(Length::FillPortion(3))
        .into()
}

pub fn view_songs(songs: &[Song]) -> Element<'_, Message> {
    let mut column = Column::new().padding(20);

    for song in songs {
        column = column.push(
            Button::new(
                Row::new()
                    .push(Text::new(&song.title).size(18))
                    .push(Space::new().width(Length::Fixed(10.0)))
                    .push(Text::new("Play").size(14)),
            )
            .on_press(Message::TrackSelected(song.id.clone())),
        );
    }

    Scrollable::new(column)
        .width(Length::Fill)
        .height(Length::FillPortion(3))
        .into()
}
