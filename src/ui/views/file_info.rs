//! The transient File info panel: a modal overlay over the normal-mode screen naming the current
//! track's title, artist, album, year, genre, track number, disc number, composer, content rating,
//! length, and — while the loaded preview reports one — its format.

use iced::{
    Element,
    widget::{Column, Text},
};

use crate::audio::PlaybackFormat;
use crate::ui::{Message, theme};

use super::{
    KnownTrack, PANEL_TEXT_SIZE, labeled_button,
    modal::modal_overlay,
    now_playing::{channel_mode_label, format_playback_prefix},
};

/// The label before the track's title.
const TITLE_LABEL: &str = "Title: ";

/// The label before the track's artist.
const ARTIST_LABEL: &str = "Artist: ";

/// The label before the track's album.
const ALBUM_LABEL: &str = "Album: ";

/// The label before the track's release year.
const YEAR_LABEL: &str = "Year: ";

/// The label before the track's primary genre.
const GENRE_LABEL: &str = "Genre: ";

/// The label before the track's 1-based album position.
const TRACK_NUMBER_LABEL: &str = "Track #: ";

/// The label before the track's disc number.
const DISC_NUMBER_LABEL: &str = "Disc #: ";

/// The label before the track's composer.
const COMPOSER_LABEL: &str = "Composer: ";
/// The label before the track's content rating.
const CONTENT_RATING_LABEL: &str = "Rating: ";

/// The label before the track's formatted length.
const LENGTH_LABEL: &str = "Length: ";

/// The label before the loaded preview's format.
const FORMAT_LABEL: &str = "Format: ";

/// Stacks the transient File info panel for `track` over `base`.
///
/// The panel is modal (see [`modal_overlay`]): `Close` and the backdrop each publish
/// [`Message::ToggleFileInfo`]. It lists the title, the artist, the album, the release year, the
/// genre, the track number, the disc number, the composer, and the content rating (each left out
/// when the source supplied none), the length, which is `--:--` for an unknown duration, and —
/// after the length and only while the loaded preview reports one (`format` is `Some`) — the format
/// line built from the LCD status row's prefix and channel label, for example
/// `Format: 256 KBPS  44 KHZ STEREO`. No state or playback is touched here.
///
/// `track` is taken by value, not by reference, because one caller — the File info branch of
/// `ui::view::overlay` — resolves an ephemeral selected-row entry on the spot; the element it
/// returns borrows only `base`, so an owned entry built in its body lives exactly as long as the
/// panel needs it. The current-track fallback path clones the recorded entry in, and the view
/// tests clone their fixtures in.
pub fn view_file_info_overlay<'a>(
    base: Element<'a, Message>,
    track: KnownTrack,
    format: Option<PlaybackFormat>,
) -> Element<'a, Message> {
    let mut content = Column::new()
        .spacing(10)
        .align_x(iced::alignment::Horizontal::Center)
        .push(
            Text::new("FILE INFO")
                .size(PANEL_TEXT_SIZE)
                .color(theme::TITLE_TAPE),
        )
        .push(Text::new(format!("{TITLE_LABEL}{}", track.title)).color(theme::TEXT));
    if !track.artist.is_empty() {
        content =
            content.push(Text::new(format!("{ARTIST_LABEL}{}", track.artist)).color(theme::TEXT));
    }
    if !track.album.is_empty() {
        content =
            content.push(Text::new(format!("{ALBUM_LABEL}{}", track.album)).color(theme::TEXT));
    }
    if let Some(year) = track.year {
        content = content.push(Text::new(format!("{YEAR_LABEL}{year}")).color(theme::TEXT));
    }
    if !track.genre.is_empty() {
        content =
            content.push(Text::new(format!("{GENRE_LABEL}{}", track.genre)).color(theme::TEXT));
    }
    if let Some(track_number) = track.track_number {
        content = content
            .push(Text::new(format!("{TRACK_NUMBER_LABEL}{track_number}")).color(theme::TEXT));
    }
    if let Some(disc_number) = track.disc_number {
        content =
            content.push(Text::new(format!("{DISC_NUMBER_LABEL}{disc_number}")).color(theme::TEXT));
    }
    if !track.composer.is_empty() {
        content = content
            .push(Text::new(format!("{COMPOSER_LABEL}{}", track.composer)).color(theme::TEXT));
    }
    if let Some(rating) = track.content_rating {
        content = content.push(
            Text::new(format!("{CONTENT_RATING_LABEL}{}", rating.label())).color(theme::TEXT),
        );
    }
    content = content.push(Text::new(format!("{LENGTH_LABEL}{}", track.time)).color(theme::TEXT));
    if let Some(format) = format {
        content = content.push(
            Text::new(format!(
                "{FORMAT_LABEL}{} {}",
                format_playback_prefix(format),
                channel_mode_label(format.channels)
            ))
            .color(theme::TEXT),
        );
    }
    content = content.push(labeled_button("Close", Message::ToggleFileInfo));
    modal_overlay(base, content, Message::ToggleFileInfo)
}
