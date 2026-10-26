//! The transient File info panel: a modal overlay over the normal-mode screen
//! naming the current track's title, artist, album, and length.

use iced::{
    Element,
    widget::{Column, Text},
};

use crate::ui::{Message, theme};

use super::{KnownTrack, labeled_button, modal::modal_overlay};

/// The label before the track's title.
const TITLE_LABEL: &str = "Title: ";

/// The label before the track's artist.
const ARTIST_LABEL: &str = "Artist: ";

/// The label before the track's album.
const ALBUM_LABEL: &str = "Album: ";

/// The label before the track's formatted length.
const LENGTH_LABEL: &str = "Length: ";

/// Stacks the transient File info panel for `track` over `base`.
///
/// The panel is modal (see [`modal_overlay`]): `Close` and the backdrop each
/// publish [`Message::ToggleFileInfo`]. It lists the title, the artist and the
/// album (each left out when the source supplied none), and the length, which
/// is `--:--` for an unknown duration. No state or playback is touched here.
pub fn view_file_info_overlay<'a>(
    base: Element<'a, Message>,
    track: &'a KnownTrack,
) -> Element<'a, Message> {
    let mut content = Column::new()
        .spacing(10)
        .align_x(iced::alignment::Horizontal::Center)
        .push(Text::new("FILE INFO").size(18).color(theme::TITLE_TAPE))
        .push(Text::new(format!("{TITLE_LABEL}{}", track.title)).color(theme::TEXT));
    if !track.artist.is_empty() {
        content =
            content.push(Text::new(format!("{ARTIST_LABEL}{}", track.artist)).color(theme::TEXT));
    }
    if !track.album.is_empty() {
        content =
            content.push(Text::new(format!("{ALBUM_LABEL}{}", track.album)).color(theme::TEXT));
    }
    content = content
        .push(Text::new(format!("{LENGTH_LABEL}{}", track.time)).color(theme::TEXT))
        .push(labeled_button("Close", Message::ToggleFileInfo));
    modal_overlay(base, content, Message::ToggleFileInfo)
}
