//! The Now Playing bar: the current track's resolved artist, title, and
//! length on the LCD well, plus the pure helpers that resolve and format them.

use std::borrow::Cow;
use std::collections::HashMap;

use iced::{
    Element, Length,
    widget::{Row, Space, Text},
};

use crate::ui::{Message, bevel, theme};

/// A track the Now Playing bar knows about: the title and artist it renders
/// and the formatted length it shows beside them. Recorded once when a track
/// is played, so the bar keeps naming and timing the playing track after the
/// user browses to a different album.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownTrack {
    /// The track's display title.
    pub title: String,
    /// The performing artist's display name; `""` means the source supplied
    /// no artist, and the Now Playing bar falls back to the title alone.
    pub artist: String,
    /// The track's formatted length ([`format_track_time`]), cached when the
    /// track is recorded so a per-frame bar refresh borrows it instead of
    /// formatting the same duration again.
    pub time: String,
}

/// The Now Playing bar label: the title of `current_track` when `tracks`
/// knows it; the raw id when it doesn't; "Nothing" when no track is current
/// (Stop keeps the interrupted track's title). `tracks` is
/// the player's known id→track index, so this per-frame resolution is a single
/// map get rather than a scan of every song the player has ever loaded. The
/// result borrows the title from `tracks` (or the `"Nothing"` literal) in the
/// common cases, so a per-frame refresh allocates only for the rare unknown-id
/// fallback instead of cloning the title on every view. Pure data → `Cow` so
/// the title resolution is testable without an iced renderer.
pub fn now_playing_label<'a>(
    tracks: &'a HashMap<String, KnownTrack>,
    current_track: Option<&str>,
) -> Cow<'a, str> {
    match current_track {
        Some(id) => tracks
            .get(id)
            .map(|track| Cow::Borrowed(track.title.as_str()))
            .unwrap_or_else(|| Cow::Owned(id.to_string())),
        None => Cow::Borrowed("Nothing"),
    }
}

/// The Now Playing bar's artist for `current_track`: the artist of the track
/// `tracks` knows, or the `""` literal when the track is unknown or none is
/// current. Mirrors [`now_playing_label`], so resolving the artist adds no
/// per-frame `String` allocation — the known case borrows from `tracks`.
/// Pure data → `Cow` so the artist resolution is testable without an iced
/// renderer.
pub fn now_playing_artist<'a>(
    tracks: &'a HashMap<String, KnownTrack>,
    current_track: Option<&str>,
) -> Cow<'a, str> {
    current_track
        .and_then(|id| tracks.get(id))
        .map(|track| Cow::Borrowed(track.artist.as_str()))
        .unwrap_or(Cow::Borrowed(""))
}

/// The Now Playing bar's length readout for `current_track`: the label cached
/// in [`KnownTrack::time`] when the track was recorded, or `"--:--"` when the
/// track is unknown or none is current. The known case borrows from `tracks`,
/// so a per-frame bar refresh formats and allocates nothing; mirrors
/// [`now_playing_label`] and [`now_playing_artist`]. Pure data → `Cow` so the
/// resolution is testable without an iced renderer.
pub fn now_playing_time<'a>(
    tracks: &'a HashMap<String, KnownTrack>,
    current_track: Option<&str>,
) -> Cow<'a, str> {
    current_track
        .and_then(|id| tracks.get(id))
        .map(|track| Cow::Borrowed(track.time.as_str()))
        .unwrap_or(Cow::Borrowed("--:--"))
}

/// The Now Playing bar's length readout for `duration_ms`: `"--:--"` when the
/// source supplied no duration (`0`), else zero-padded `m:ss` (so a track
/// under a minute reads `0:45` and one over an hour keeps counting minutes
/// rather than rolling into hours). Pure so the formatting is testable without
/// an iced renderer.
pub fn format_track_time(duration_ms: u64) -> String {
    if duration_ms == 0 {
        return "--:--".to_string();
    }
    let total_seconds = duration_ms / 1000;
    format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
}

/// The Now Playing bar: the "Now Playing:" caption followed by the current
/// track's resolved artist and title (or "Nothing" when nothing is current),
/// with the track's length ([`format_track_time`]) laid at the right edge of
/// the LCD well. The artist (from [`now_playing_artist`]) is pushed before the
/// title with a `" - "` separator only when it is non-empty, so an artist-less
/// track still reads `Now Playing: <title>`. Takes the already resolved
/// label, artist, and time string so this per-frame widget build does no song
/// lookup or formatting itself. All three text values are [`Cow`]s: the
/// borrowed cases keep the strings owned by the caller's index (no
/// allocation), while the owned fallback is moved into the widget, so the
/// built element never borrows a temporary.
pub fn view_now_playing<'a>(
    label: Cow<'a, str>,
    artist: Cow<'a, str>,
    time: Cow<'a, str>,
) -> Element<'a, Message> {
    let mut row = Row::new().push(Text::new("Now Playing: ").size(20));
    if !artist.is_empty() {
        row = row
            .push(Text::new(artist).size(20).color(theme::LCD_GREEN))
            .push(Text::new(" - ").size(20).color(theme::LCD_GREEN));
    }
    bevel::lcd_well(
        row.push(Text::new(label).size(20).color(theme::LCD_GREEN))
            .push(Space::new().width(Length::Fill))
            .push(Text::new(time).size(20).color(theme::LCD_GREEN)),
    )
}
