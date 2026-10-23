//! The Now Playing bar: the current track's resolved elapsed time, artist,
//! title, and length on the LCD well, plus the pure helpers that resolve and
//! format them.

use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

use iced::{
    Color, Element, Length,
    alignment::Vertical,
    mouse::Interaction,
    widget::{Column, Container, MouseArea, Row, Space, Text, container},
};

use crate::audio::{PlaybackFormat, Spectrum};
use crate::equalizer::BAND_COUNT;
use crate::library::display_title_for;
use crate::state::TransportMode;
use crate::ui::{Message, bevel, theme};

/// The LCD's character budget for the combined artist/title line: sized to
/// keep an overlong line inside the roughly 550px-wide window beside the
/// large elapsed readout, the `"Now Playing: "` caption, and the total at
/// the right edge. A line that fits renders still and borrowed; a longer one
/// scrolls through [`marquee_text`].
pub(super) const MARQUEE_WIDTH: usize = 22;

/// The spaces between the end of a scrolling line and its next repeat.
const MARQUEE_GAP: usize = 3;

/// The LCD's large elapsed-readout size in pixels, matching the mockup's big
/// left-hand time readout.
const LCD_ELAPSED_SIZE: f32 = 34.0;

/// The transport glyph's size in pixels, small enough to sit beside the large
/// elapsed readout.
const LCD_TRANSPORT_SIZE: f32 = 22.0;

/// The fixed width of the transport glyph's slot in pixels. Keeping the slot
/// fixed leaves the elapsed readout at the same x position as the glyph
/// changes between `▶`, `II`, and `■`.
const LCD_TRANSPORT_WIDTH: f32 = 16.0;

/// The fixed height of the LCD's status row, in pixels. The row keeps this
/// height whether or not a preview reports a format, so loading one does not
/// make the bar jump.
const LCD_STATUS_HEIGHT: f32 = 16.0;

/// The status text's size in pixels: small enough to sit quietly beneath the
/// title line.
const LCD_STATUS_SIZE: f32 = 12.0;

/// The fixed height of the LCD's visualizer row, in pixels, and the maximum
/// height of one bar. The row keeps this height whether or not a preview
/// reports a spectrum, so loading one does not move the title or status text.
const LCD_VISUALIZER_HEIGHT: f32 = 24.0;

/// The fixed width of one visualizer bar, in pixels, so the ten bars keep
/// their places as their heights change.
const LCD_VISUALIZER_BAR_WIDTH: f32 = 8.0;

/// The gap between adjacent visualizer bars, in pixels.
const LCD_VISUALIZER_BAR_GAP: f32 = 3.0;

/// The classic indicator letters down the LCD's left edge, in the mockup's
/// order. [`lcd_indicator_strip`] draws them; the strip is decorative and
/// emits no message.
pub(super) const LCD_INDICATOR_LABELS: [&str; 5] = ["O", "A", "I", "D", "V"];

/// The fixed width of the decorative indicator strip, in pixels, so the
/// content beside it keeps its bounds: the title starts one strip-width
/// further right, while the right-edge total stays at the edge because the
/// content column fills the remaining width.
pub(super) const LCD_INDICATOR_WIDTH: f32 = 12.0;

/// The indicator letters' size in pixels: small enough to read as a column
/// beside the large elapsed readout.
const LCD_INDICATOR_SIZE: f32 = 10.0;

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
/// Which time the Now Playing LCD's large left-hand readout shows. The mode
/// is presentation-only: it changes nothing about playback or the right-edge
/// total, and the player keeps it across Pause, Stop, Next/Previous, and a new
/// selection until the readout is clicked again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeDisplay {
    /// The elapsed playback position, the default (`00:53`).
    #[default]
    Elapsed,
    /// The time left in the track (`-02:37`).
    Remaining,
}

impl TimeDisplay {
    /// The other mode: [`Self::Elapsed`] becomes [`Self::Remaining`] and back.
    pub fn toggled(self) -> Self {
        match self {
            Self::Elapsed => Self::Remaining,
            Self::Remaining => Self::Elapsed,
        }
    }
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
    /// The track's formatted length ([`super::format_track_time`]), cached when the
    /// track is recorded so a per-frame bar refresh borrows it instead of
    /// formatting the same duration again.
    pub time: String,
    /// The track's raw length in milliseconds, the value the remaining
    /// readout subtracts the playback position from; `0` means the source
    /// supplied no duration.
    pub duration_ms: u64,
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

/// Looks up the [`KnownTrack`] for `current_track` in `tracks`, returning
/// `None` when no track is current or the id is unknown. Shared by
/// [`now_playing_label`], [`now_playing_artist`], [`now_playing_elapsed`], and
/// [`now_playing_time`] so the four stay in sync on the lookup.
pub(super) fn known_track_for<'a>(
    tracks: &'a HashMap<String, KnownTrack>,
    current_track: Option<&str>,
) -> Option<&'a KnownTrack> {
    current_track.and_then(|id| tracks.get(id))
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
    known_track_for(tracks, current_track)
        .map(|track| Cow::Borrowed(track.artist.as_str()))
        .unwrap_or(Cow::Borrowed(""))
}

/// The Now Playing bar's right-edge length readout for `current_track`: the
/// plain total cached in [`KnownTrack::time`], or `"--:--"` when the track is
/// unknown or none is current. A reported playback position never changes
/// this readout; it drives [`now_playing_elapsed`] instead. The known case
/// borrows from `tracks`, so a per-frame bar refresh formats and allocates
/// nothing. Mirrors [`now_playing_label`], [`now_playing_artist`], and
/// [`now_playing_elapsed`]. Pure data → `Cow` so the resolution is testable
/// without an iced renderer.
pub fn now_playing_time<'a>(
    tracks: &'a HashMap<String, KnownTrack>,
    current_track: Option<&str>,
) -> Cow<'a, str> {
    known_track_for(tracks, current_track)
        .map(|track| Cow::Borrowed(track.time.as_str()))
        .unwrap_or(Cow::Borrowed("--:--"))
}

/// The Now Playing bar's large left-hand time readout for `current_track`,
/// as chosen by `display`: elapsed `MM:SS` (the default) or the remaining
/// time with a leading `-`. In [`TimeDisplay::Elapsed`] the backend-reported
/// position formats as zero-padded `MM:SS` when there is one (`"00:53"`),
/// `"00:00"` when the track is known but no position is reported (no preview
/// loaded, a failed start, or a backend that cannot answer), or `"--:--"`
/// when the track is unknown or none is current. In [`TimeDisplay::Remaining`]
/// a known track with a length reads `-MM:SS` counting down from that length,
/// treating a missing position as zero so a stopped track shows its full
/// length; a missing position past the length saturates at `"-00:00"`, and a
/// known track whose source supplied no length (`duration_ms == 0`) or an
/// unknown/absent track reads the `"--:--"` placeholder. Mirrors
/// [`now_playing_label`], [`now_playing_artist`], and [`now_playing_time`].
/// The placeholder cases borrow static strings, while a reported position
/// allocates its formatted `String`. Pure data → `Cow` so the resolution is
/// testable without an iced renderer.
pub fn now_playing_elapsed<'a>(
    tracks: &'a HashMap<String, KnownTrack>,
    current_track: Option<&str>,
    position: Option<Duration>,
    display: TimeDisplay,
) -> Cow<'a, str> {
    let Some(track) = known_track_for(tracks, current_track) else {
        return Cow::Borrowed("--:--");
    };
    match display {
        TimeDisplay::Elapsed => match position {
            Some(position) => Cow::Owned(format_lcd_elapsed(duration_millis(position))),
            None => Cow::Borrowed("00:00"),
        },
        TimeDisplay::Remaining => {
            if track.duration_ms == 0 {
                return Cow::Borrowed("--:--");
            }
            let position_ms = position.map(duration_millis).unwrap_or(0);
            Cow::Owned(format_lcd_remaining(
                track.duration_ms.saturating_sub(position_ms),
            ))
        }
    }
}

/// `duration` in milliseconds, saturating at [`u64::MAX`] rather than wrapping
/// for a position that large (which no real preview reaches).
fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Formats an elapsed playback position in milliseconds as the LCD's
/// zero-padded `MM:SS` form, so `0` reads `"00:00"`, `53_000` reads `"00:53"`,
/// and `192_000` reads `"03:12"`. Minutes are unbounded, so an hour or more
/// reads `"60:00"`/`"61:05"` rather than rolling over.
pub fn format_lcd_elapsed(position_ms: u64) -> String {
    let total_seconds = position_ms / 1000;
    format!("{:02}:{:02}", total_seconds / 60, total_seconds % 60)
}

/// Formats a remaining time in milliseconds as the LCD's countdown form with
/// a leading `-`, so `0` reads `"-00:00"`, `157_000` reads `"-02:37"`, and
/// `3_600_000` reads `"-60:00"`. Shares [`format_lcd_elapsed`]'s unbounded
/// minutes and zero padding after the sign.
pub fn format_lcd_remaining(remaining_ms: u64) -> String {
    format!("-{}", format_lcd_elapsed(remaining_ms))
}

/// Formats a loaded preview's [`PlaybackFormat`] as the LCD's status row,
/// `"{kbps} KBPS  {khz} KHZ  {mode}"` (for example
/// `256 KBPS  44 KHZ  STEREO`). The rate is truncated to whole kilohertz
/// (`44_100` reads `44`, `48_000` reads `48`), and previews are mono or
/// stereo, so one channel reads `MONO` and any other count reads `STEREO`.
/// The bitrate prefix is omitted when the backend could not compute it, so
/// the row falls back to `44 KHZ  STEREO`. Pure so the formatting is testable
/// without an iced renderer.
pub fn format_playback_status(format: PlaybackFormat) -> String {
    let mode = if format.channels == 1 {
        "MONO"
    } else {
        "STEREO"
    };
    let rate = format!("{} KHZ  {mode}", format.sample_rate_hz / 1000);
    match format.bitrate_kbps {
        Some(kbps) => format!("{kbps} KBPS  {rate}"),
        None => rate,
    }
}

/// The Now Playing bar: a small decorative `O A I D V` strip down the LCD's
/// left edge, then the large elapsed readout, the "Now Playing:" caption
/// followed by the current track's resolved artist and
/// title (or "Nothing" when nothing is current), with the track's plain total
/// length ([`super::format_track_time`]) laid at the right edge of the LCD well. The
/// artist (from [`now_playing_artist`]) is pushed before the title with a
/// `" - "` separator only when it is non-empty, so an artist-less track still
/// reads `Now Playing: <title>`. A line within [`MARQUEE_WIDTH`] characters
/// keeps that borrowed, still layout; a longer one is combined into one string
/// and windowed by [`marquee_text`] at `marquee_offset`, scrolling one
/// character per tick through the gap and back to its start. The character
/// budget is an approximation of the LCD's pixel width beside the elapsed
/// readout, the caption, and the total. Takes the already resolved elapsed,
/// label, artist, and time strings so this per-frame widget build does no song
/// lookup or formatting itself. All four text values are [`Cow`]s: the
/// borrowed cases keep the strings owned by the caller's index (no
/// allocation), while the owned fallbacks are moved into the widget, so the
/// built element never borrows a temporary; the scrolling branch allocates
/// only its window, as the fitting branch allocates nothing.
// The resolved LCD readouts and playback state; grouping them into a wrapper
// would add a concept without making the call site clearer, as in `view_songs`.
#[allow(clippy::too_many_arguments)]
pub fn view_now_playing<'a>(
    elapsed: Cow<'a, str>,
    label: Cow<'a, str>,
    artist: Cow<'a, str>,
    time: Cow<'a, str>,
    marquee_offset: usize,
    format: Option<PlaybackFormat>,
    spectrum: Option<Spectrum>,
    mode: TransportMode,
    elapsed_visible: bool,
) -> Element<'a, Message> {
    bevel::lcd_well(now_playing_content(
        elapsed,
        label,
        artist,
        time,
        marquee_offset,
        format,
        spectrum,
        mode,
        elapsed_visible,
    ))
}

/// Builds the content [`view_now_playing`] wraps in the LCD well: the
/// decorative indicator strip, then the elapsed/title row and status row in a
/// filling column so the total stays at the right edge. Generic over the
/// theme and renderer, unlike the concrete well wrapper, so the tests can lay
/// it out with iced's null renderer and read back the text it would draw.
// Mirrors `view_now_playing`'s arguments (without the well wrapper) so the
// tests can lay out the same content headlessly.
#[allow(clippy::too_many_arguments)]
pub(super) fn now_playing_content<'a, Theme, Renderer>(
    elapsed: Cow<'a, str>,
    label: Cow<'a, str>,
    artist: Cow<'a, str>,
    time: Cow<'a, str>,
    marquee_offset: usize,
    format: Option<PlaybackFormat>,
    spectrum: Option<Spectrum>,
    mode: TransportMode,
    elapsed_visible: bool,
) -> Row<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + iced::widget::container::Catalog + 'a,
    <Theme as iced::widget::text::Catalog>::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    <Theme as iced::widget::container::Catalog>::Class<'a>:
        From<iced::widget::container::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer + 'a,
{
    Row::new().push(lcd_indicator_strip()).push(
        Column::new()
            .width(Length::Fill)
            .push(now_playing_row(
                elapsed,
                label,
                artist,
                time,
                marquee_offset,
                mode,
                elapsed_visible,
            ))
            .push(now_playing_visualizer_row(spectrum))
            .push(now_playing_status_row(format)),
    )
}

/// Builds the decorative indicator strip at the LCD's left edge: one small
/// green letter per line, in [`LCD_INDICATOR_LABELS`] order, in a column of
/// fixed [`LCD_INDICATOR_WIDTH`]. It emits no message and is drawn in every
/// playback state, so it cannot toggle anything; its fixed width pushes the
/// content beside it one strip-width right.
pub(super) fn lcd_indicator_strip<'a, Theme, Renderer>() -> Column<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + 'a,
    Theme::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer + 'a,
{
    LCD_INDICATOR_LABELS.into_iter().fold(
        Column::new().width(Length::Fixed(LCD_INDICATOR_WIDTH)),
        |column, label| {
            column.push(
                Text::new(label)
                    .size(LCD_INDICATOR_SIZE)
                    .color(theme::LCD_GREEN),
            )
        },
    )
}

/// The LCD's large elapsed-readout text style: [`LCD_ELAPSED_SIZE`]px in
/// [`theme::LCD_GREEN`], matching the color of the line beside it.
fn lcd_elapsed_text<'a, Theme, Renderer>(
    fragment: impl iced::widget::text::IntoFragment<'a>,
) -> Text<'a, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + 'a,
    Theme::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer,
{
    Text::new(fragment)
        .size(LCD_ELAPSED_SIZE)
        .color(theme::LCD_GREEN)
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

/// Resolves the visualizer's bar heights from the loaded preview's spectrum.
/// A missing spectrum yields all-zero bars, and each reported band is clamped
/// to `0.0..=1.0` before it scales the fixed row height, so the bars stay
/// inside their strip. Pure so the scaling is testable without a renderer.
pub fn visualizer_bar_heights(spectrum: Option<Spectrum>) -> [f32; BAND_COUNT] {
    let bands = spectrum.map_or([0.0; BAND_COUNT], |spectrum| spectrum.bands);
    bands.map(|band| band.clamp(0.0, 1.0) * LCD_VISUALIZER_HEIGHT)
}

/// Builds the LCD's visualizer row: exactly [`BAND_COUNT`] bottom-aligned green
/// bars, left to right in band order, inside a row of fixed
/// [`LCD_VISUALIZER_HEIGHT`]. Every band keeps a fixed-width slot even when its
/// height is zero, so a real spectrum with silent bands still draws all ten
/// bars. A missing spectrum draws the same row with zero-height bars, so the
/// title and status text keep their places in every playback state. Generic
/// over the theme and renderer so the tests can lay it out with iced's null
/// renderer.
pub(super) fn now_playing_visualizer_row<'a, Theme, Renderer>(
    spectrum: Option<Spectrum>,
) -> Row<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + iced::widget::container::Catalog + 'a,
    <Theme as iced::widget::text::Catalog>::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    <Theme as iced::widget::container::Catalog>::Class<'a>:
        From<iced::widget::container::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer + 'a,
{
    visualizer_bar_heights(spectrum).into_iter().fold(
        Row::new()
            .height(Length::Fixed(LCD_VISUALIZER_HEIGHT))
            .spacing(LCD_VISUALIZER_BAR_GAP)
            .align_y(Vertical::Bottom),
        |row, height| {
            // A zero band uses `Length::Shrink` rather than `Fixed(0.0)`:
            // iced's `Row::push` drops a child whose size hint is void (either
            // dimension `Fixed(0.0)`), which would silently remove silent
            // bands and leave fewer than `BAND_COUNT` bars. `Shrink` still
            // lays the bar out at zero height while keeping it in the row.
            let bar_height = if height > 0.0 {
                Length::Fixed(height)
            } else {
                Length::Shrink
            };
            row.push(
                Container::new(Space::new())
                    .width(Length::Fixed(LCD_VISUALIZER_BAR_WIDTH))
                    .height(bar_height)
                    .style(|_theme| container::Style {
                        background: Some(theme::LCD_GREEN.into()),
                        ..container::Style::default()
                    }),
            )
        },
    )
}

/// Builds the LCD's status row: the loaded preview's bitrate, sample rate, and
/// channel mode, right-aligned in a small green text beneath the title line.
/// With no reported format it draws no text but keeps the row's fixed height,
/// so the bar does not jump when a preview loads. Generic over the theme and
/// renderer so the tests can lay it out with iced's null renderer and read
/// back the text it would draw.
pub(super) fn now_playing_status_row<'a, Theme, Renderer>(
    format: Option<PlaybackFormat>,
) -> Row<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + 'a,
    Theme::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer + 'a,
{
    let mut row = Row::new()
        .height(Length::Fixed(LCD_STATUS_HEIGHT))
        .push(Space::new().width(Length::Fill));
    if let Some(format) = format {
        row = row.push(
            Text::new(format_playback_status(format))
                .size(LCD_STATUS_SIZE)
                .color(theme::LCD_GREEN),
        );
    }
    row
}

/// The transport glyph [`now_playing_row`] draws for `mode`: a green `▶`
/// while playing, a green `II` while paused, and a red `■` while stopped.
/// Pure so the exact glyphs are testable without a renderer.
pub(super) fn transport_glyph(mode: TransportMode) -> &'static str {
    match mode {
        TransportMode::Playing => "▶",
        TransportMode::Paused => "II",
        TransportMode::Stopped => "■",
    }
}

/// The transport glyph's color: [`theme::LCD_GREEN`] while playing or paused,
/// and [`theme::LCD_RED`] while stopped. Pure so the exact colors are testable
/// without a renderer.
pub(super) fn transport_glyph_color(mode: TransportMode) -> Color {
    match mode {
        TransportMode::Playing | TransportMode::Paused => theme::LCD_GREEN,
        TransportMode::Stopped => theme::LCD_RED,
    }
}

/// The large elapsed readout's color: fully transparent while the paused
/// readout's blink phase is hidden, and [`theme::LCD_GREEN`] in every other
/// mode or phase. Drawing it transparent rather than removing it keeps the
/// text's bounds, so a blink does not move the title or total. Pure so the
/// exact colors are testable without a renderer.
pub(super) fn elapsed_readout_color(mode: TransportMode, visible: bool) -> Color {
    match (mode, visible) {
        (TransportMode::Paused, false) => Color::from_rgba(0.0, 0.0, 0.0, 0.0),
        _ => theme::LCD_GREEN,
    }
}

/// Builds the LCD row before the bevel wraps it, generic over the theme and
/// renderer so the tests can lay it out with iced's null renderer and read
/// back the text it would draw. The transport glyph leads the row in a fixed
/// [`LCD_TRANSPORT_WIDTH`] slot, so the elapsed readout does not move as the
/// glyph changes.
pub(super) fn now_playing_row<'a, Theme, Renderer>(
    elapsed: Cow<'a, str>,
    label: Cow<'a, str>,
    artist: Cow<'a, str>,
    time: Cow<'a, str>,
    marquee_offset: usize,
    mode: TransportMode,
    elapsed_visible: bool,
) -> Row<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + iced::widget::container::Catalog + 'a,
    <Theme as iced::widget::text::Catalog>::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    <Theme as iced::widget::container::Catalog>::Class<'a>:
        From<iced::widget::container::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer + 'a,
{
    let line_len = display_len(&label, &artist);
    let mut row = Row::new()
        .push(
            Container::new(
                Text::new(transport_glyph(mode))
                    .size(LCD_TRANSPORT_SIZE)
                    .color(transport_glyph_color(mode)),
            )
            .width(Length::Fixed(LCD_TRANSPORT_WIDTH)),
        )
        .push(
            MouseArea::new(
                lcd_elapsed_text(elapsed).color(elapsed_readout_color(mode, elapsed_visible)),
            )
            .on_press(Message::ToggleTimeDisplay)
            .interaction(Interaction::Pointer),
        )
        .push(Space::new().width(Length::Fixed(12.0)))
        .push(Text::new("Now Playing: ").size(20));
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
