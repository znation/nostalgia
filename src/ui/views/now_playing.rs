//! The Now Playing bar's LCD widget construction: the decorative indicator
//! strip, the transport glyph and elapsed readout, the marquee line, and the
//! status and visualizer rows. The strings they draw are resolved in [`text`];
//! this module only lays them out.

use std::borrow::Cow;

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

pub(super) mod text;

pub use text::{
    KnownTrack, TimeDisplay, format_playback_status, needs_marquee, now_playing_artist,
    now_playing_elapsed, now_playing_label, now_playing_time,
};
pub(in crate::ui::views) use text::{MARQUEE_WIDTH, display_len, marquee_text};

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

/// The large elapsed readout's color: fully transparent while the transport
/// is stopped for a current track and while the paused readout's blink phase
/// is hidden, and [`theme::LCD_GREEN`] in every other mode or phase. The
/// `--:--` placeholder stays green even while stopped: it reports that there
/// is no track or no known time, not a time to clear. Drawing the stopped
/// readout transparent rather than removing it keeps its bounds, so the
/// title, total, and status row stay in place. Pure so the exact colors are
/// testable without a renderer.
pub(super) fn elapsed_readout_color(mode: TransportMode, visible: bool, elapsed: &str) -> Color {
    match (mode, visible) {
        (TransportMode::Stopped, _) if elapsed != "--:--" => Color::from_rgba(0.0, 0.0, 0.0, 0.0),
        (TransportMode::Paused, false) => Color::from_rgba(0.0, 0.0, 0.0, 0.0),
        _ => theme::LCD_GREEN,
    }
}

/// Whether the large elapsed readout responds to a click in `mode`: it does
/// while playing or paused, and not while stopped, where the readout clears
/// any time (except the `--:--` placeholder) and a click must not silently
/// switch the persisted time mode. Pure so the exact gating is testable
/// without a renderer.
pub(super) fn elapsed_readout_clickable(mode: TransportMode) -> bool {
    !matches!(mode, TransportMode::Stopped)
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
        .push({
            let color = elapsed_readout_color(mode, elapsed_visible, elapsed.as_ref());
            let readout = MouseArea::new(lcd_elapsed_text(elapsed).color(color));
            if elapsed_readout_clickable(mode) {
                readout
                    .on_press(Message::ToggleTimeDisplay)
                    .interaction(Interaction::Pointer)
            } else {
                readout.interaction(Interaction::None)
            }
        })
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
