//! The Now Playing bar: the current track's resolved artist, title, and
//! length on the LCD well, plus the pure helpers that resolve and format them.

use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

use iced::{
    Element, Length,
    widget::{Row, Space, Text},
};

use crate::library::display_title_for;
use crate::ui::{Message, bevel, theme};

/// The LCD's character budget for the combined artist/title line: sized to
/// keep an overlong line inside the roughly 550px-wide window beside the
/// `"Now Playing: "` caption and the time readout. A line that fits renders
/// still and borrowed; a longer one scrolls through [`marquee_text`].
pub(super) const MARQUEE_WIDTH: usize = 28;

/// The spaces between the end of a scrolling line and its next repeat.
const MARQUEE_GAP: usize = 3;

/// The number of characters the Now Playing line occupies: the label's
/// characters, plus the artist's and the three characters of the `" - "`
/// separator that joins them.
///
/// The bar shows the line whole while it fits [`MARQUEE_WIDTH`] and windows it
/// when it does not, so this is the one measure both the view and the marquee
/// timer read — the timer runs only while the line is too long, and the view
/// windows exactly the lines the timer scrolls.
pub(super) fn display_len(label: &str, artist: &str) -> usize {
    label.chars().count()
        + if artist.is_empty() {
            0
        } else {
            artist.chars().count() + 3
        }
}

/// Whether the Now Playing line is too long for the LCD's character budget
/// and so has to scroll: the combined artist and title exceed
/// [`MARQUEE_WIDTH`] characters.
///
/// The marquee timer runs only while this holds. A line that fits is shown
/// whole, so ticking for it would advance an offset the view ignores — and
/// every tick makes iced rebuild the whole screen, browse list included.
pub fn needs_marquee(label: &str, artist: &str) -> bool {
    display_len(label, artist) > MARQUEE_WIDTH
}

/// Returns the `width`-character window of `text` starting at
/// `offset % cycle`, where `cycle` is the line plus [`MARQUEE_GAP`] trailing
/// spaces. A line that fits `width` is returned unchanged, so the caller's
/// common case stays still; an overlong line wraps through the gap back to
/// its start, with a space for each position past the text's end. Counts
/// characters, not bytes, so a non-ASCII title windows on character
/// boundaries.
pub(super) fn marquee_text(text: &str, offset: usize, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        return text.to_string();
    }
    let cycle = chars.len() + MARQUEE_GAP;
    let start = offset % cycle;
    (0..width)
        .map(|position| {
            let index = (start + position) % cycle;
            chars.get(index).copied().unwrap_or(' ')
        })
        .collect()
}
/// A track the Now Playing bar knows about: the title and artist it renders
/// and the formatted length it shows beside them. Recorded once when a track
/// is played, so the bar keeps naming and timing the playing track after the
/// user browses to a different album.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownTrack {
    /// The track's title, without the artist (the bar renders
    /// [`Self::artist`] separately).
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
        Some(id) => known_track_for(tracks, Some(id))
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
/// Looks up the [`KnownTrack`] for `current_track` in `tracks`, returning
/// `None` when no track is current or the id is unknown. Shared by
/// [`now_playing_label`], [`now_playing_artist`], and [`now_playing_time`] so
/// the three stay in sync on the lookup.
pub(super) fn known_track_for<'a>(
    tracks: &'a HashMap<String, KnownTrack>,
    current_track: Option<&str>,
) -> Option<&'a KnownTrack> {
    current_track.and_then(|id| tracks.get(id))
}

pub fn now_playing_artist<'a>(
    tracks: &'a HashMap<String, KnownTrack>,
    current_track: Option<&str>,
) -> Cow<'a, str> {
    known_track_for(tracks, current_track)
        .map(|track| Cow::Borrowed(track.artist.as_str()))
        .unwrap_or(Cow::Borrowed(""))
}

/// The Now Playing bar's length readout for `current_track`: the elapsed time
/// followed by the label cached in [`KnownTrack::time`] when the audio backend
/// reports a position for a known track (`"0:45/3:30"`), that cached label
/// alone when it does not, or `"--:--"` when the track is unknown or none is
/// current. The plain known case borrows from `tracks`, so a per-frame bar
/// refresh formats and allocates nothing there; the combined elapsed/total
/// case allocates its `String`. Mirrors [`now_playing_label`] and
/// [`now_playing_artist`]. Pure data → `Cow` so the resolution is testable
/// without an iced renderer.
pub fn now_playing_time<'a>(
    tracks: &'a HashMap<String, KnownTrack>,
    current_track: Option<&str>,
    position: Option<Duration>,
) -> Cow<'a, str> {
    let Some(track) = known_track_for(tracks, current_track) else {
        return Cow::Borrowed("--:--");
    };
    match position {
        Some(position) => Cow::Owned(now_playing_progress(duration_millis(position), &track.time)),
        None => Cow::Borrowed(track.time.as_str()),
    }
}

/// `duration` in milliseconds, saturating at [`u64::MAX`] rather than wrapping
/// for a position that large (which no real preview reaches).
fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Formats an elapsed playback position in milliseconds as the
/// [`format_minutes_seconds`] `m:ss` form, so `0` reads `"0:00"` and `192_000`
/// reads `"3:12"`.
pub fn format_position_time(position_ms: u64) -> String {
    format_minutes_seconds(position_ms)
}

/// Joins an elapsed playback position with the track's formatted total as
/// `"<elapsed>/<total>"`, so a 192-second position and a `"12:38"` total read
/// `"3:12/12:38"`.
pub fn now_playing_progress(position_ms: u64, total: &str) -> String {
    format!("{}/{}", format_position_time(position_ms), total)
}

/// Formats `duration_ms` as zero-padded `m:ss`, with unbounded minutes so an
/// hour or more reads as `60:00`/`61:05` rather than rolling over. This is the
/// shared core of [`format_track_time`], whose zero case is the per-track
/// `"--:--"` placeholder, and the browse song list's total-playing-time
/// footer, whose zero total is a real `"0:00"`.
pub(super) fn format_minutes_seconds(duration_ms: u64) -> String {
    let total_seconds = duration_ms / 1000;
    format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
}

/// The Now Playing bar's length readout for `duration_ms`: `"--:--"` when the
/// source supplied no duration (`0`), else the shared
/// [`format_minutes_seconds`] form (so a track under a minute reads `0:45` and
/// one over an hour keeps counting minutes rather than rolling into hours).
/// Pure so the formatting is testable without an iced renderer.
pub fn format_track_time(duration_ms: u64) -> String {
    if duration_ms == 0 {
        return "--:--".to_string();
    }
    format_minutes_seconds(duration_ms)
}

/// The Now Playing bar: the "Now Playing:" caption followed by the current
/// track's resolved artist and title (or "Nothing" when nothing is current),
/// with the track's length ([`format_track_time`]) laid at the right edge of
/// the LCD well. The artist (from [`now_playing_artist`]) is pushed before the
/// title with a `" - "` separator only when it is non-empty, so an artist-less
/// track still reads `Now Playing: <title>`. A line within
/// [`MARQUEE_WIDTH`] characters keeps that borrowed, still layout; a longer
/// one is combined into one string and windowed by [`marquee_text`] at
/// `marquee_offset`, scrolling one character per tick through the gap and
/// back to its start. The character budget is an approximation of the LCD's
/// pixel width beside the caption and the time. Takes the already resolved
/// label, artist, and time string so this per-frame widget build does no song
/// lookup or formatting itself. All three text values are [`Cow`]s: the
/// borrowed cases keep the strings owned by the caller's index (no
/// allocation), while the owned fallback is moved into the widget, so the
/// built element never borrows a temporary; the scrolling branch allocates
/// only its window, as the fitting branch allocates nothing.
pub fn view_now_playing<'a>(
    label: Cow<'a, str>,
    artist: Cow<'a, str>,
    time: Cow<'a, str>,
    marquee_offset: usize,
) -> Element<'a, Message> {
    bevel::lcd_well(now_playing_row(label, artist, time, marquee_offset))
}

/// The LCD's text style: 20px in [`theme::LCD_GREEN`], shared by every green
/// fragment of the Now Playing line so no branch drifts from another.
fn lcd_text<'a, Theme, Renderer>(
    fragment: impl iced::widget::text::IntoFragment<'a>,
) -> Text<'a, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + 'a,
    Theme::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer,
{
    Text::new(fragment).size(20).color(theme::LCD_GREEN)
}

/// Builds the LCD row before the bevel wraps it, generic over the theme and
/// renderer so the tests can lay it out with iced's null renderer and read
/// back the text it would draw.
pub(super) fn now_playing_row<'a, Theme, Renderer>(
    label: Cow<'a, str>,
    artist: Cow<'a, str>,
    time: Cow<'a, str>,
    marquee_offset: usize,
) -> Row<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + 'a,
    Theme::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer + 'a,
{
    let line_len = display_len(&label, &artist);
    let mut row = Row::new().push(Text::new("Now Playing: ").size(20));
    if line_len <= MARQUEE_WIDTH {
        if !artist.is_empty() {
            row = row.push(lcd_text(artist)).push(lcd_text(" - "));
        }
        row = row.push(lcd_text(label));
    } else {
        let display = display_title_for(&artist, &label).into_owned();
        row = row.push(lcd_text(marquee_text(
            &display,
            marquee_offset,
            MARQUEE_WIDTH,
        )));
    }
    row.push(Space::new().width(Length::Fill))
        .push(lcd_text(time))
}
