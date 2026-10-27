//! The Now Playing bar's LCD widget construction: the LCD well and content
//! column, and the transport, elapsed, and marquee title row. The indicator
//! strip, status row, and visualizer row each build their own widgets in
//! [`indicator`], [`status`], and [`visualizer`]; the strings they draw are
//! resolved in [`text`], and the mixer drag readout in [`mixer_readout`].

use std::borrow::Cow;

use iced::{
    Color, Element, Length,
    mouse::Interaction,
    widget::{Column, Container, MouseArea, Row, Space, Text},
};

use crate::audio::{PlaybackFormat, Spectrum};
use crate::library::display_title_for;
use crate::state::TransportMode;
use crate::ui::{Message, bevel, theme};

pub(super) mod indicator;
pub(super) mod mixer_readout;
pub(super) mod status;
pub(super) mod text;
pub(super) mod visualizer;

pub use indicator::{IndicatorModes, lcd_indicator_strip};
pub use mixer_readout::{MixerDrag, mixer_readout_text, seek_readout_text};
use status::now_playing_status_row;
pub use text::{
    KnownTrack, TimeDisplay, channel_mode_label, dragged_marquee_offset, format_playback_prefix,
    marquee_cycle_len, needs_marquee, now_playing_artist, now_playing_elapsed, now_playing_label,
    now_playing_number, now_playing_time,
};
pub(in crate::ui::views) use text::{MARQUEE_WIDTH, display_len, marquee_text};
use visualizer::now_playing_visualizer_row;
pub use visualizer::{next_spectrum_peaks, visible_spectrum};

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

/// The Now Playing bar: a small `O A I D V` strip down the LCD's left edge (with `O` opening the
/// options menu, `A` toggling always-on-top, `I` opening File info, and `V` the visualizer while
/// `D` stays decorative), then the large elapsed readout, the "Now Playing:" caption followed by
/// the current track's playlist number prefix, resolved artist, and title (or "Nothing" when
/// nothing is current), with the track's plain total length ([`super::format_track_time`]) laid at
/// the right edge of the LCD well. The number (from [`now_playing_number`]) is pushed after the
/// caption only when it is non-empty, then the artist (from [`now_playing_artist`]) before the
/// title with a `" - "` separator only when it is non-empty, so an artist-less track still reads
/// `Now Playing: <title>`. A line within [`MARQUEE_WIDTH`] characters keeps that borrowed, still
/// layout; a longer one is combined into one string and windowed by [`marquee_text`] at
/// `marquee_offset`, scrolling one character per tick through the gap and back to its start, prefix
/// included. The character budget is an approximation of the LCD's pixel width beside the elapsed
/// readout, the caption, and the total. Takes the already resolved elapsed, label, artist, number,
/// and time strings so this per-frame widget build does no song lookup or formatting itself. The
/// five text values are [`Cow`]s: the borrowed cases keep the strings owned by the caller's index
/// (no allocation), while the owned fallbacks are moved into the widget, so the built element never
/// borrows a temporary; the scrolling branch allocates only its window, as the fitting branch
/// allocates nothing. `modes` carries the four flags the indicator strip lights (the options menu,
/// always-on-top, File info, and the visualizer).
// The resolved LCD readouts and playback state; grouping them into a wrapper
// would add a concept without making the call site clearer, as in `view_songs`.
#[allow(clippy::too_many_arguments)]
pub fn view_now_playing<'a>(
    elapsed: Cow<'a, str>,
    label: Cow<'a, str>,
    artist: Cow<'a, str>,
    number: Cow<'a, str>,
    time: Cow<'a, str>,
    marquee_offset: usize,
    format: Option<PlaybackFormat>,
    spectrum: Option<Spectrum>,
    peaks: Option<Spectrum>,
    mode: TransportMode,
    elapsed_visible: bool,
    mixer_readout: Option<String>,
    modes: IndicatorModes,
) -> Element<'a, Message> {
    bevel::lcd_well(now_playing_content(
        elapsed,
        label,
        artist,
        number,
        time,
        marquee_offset,
        format,
        spectrum,
        peaks,
        mode,
        elapsed_visible,
        mixer_readout,
        modes,
    ))
}

/// Builds the content [`view_now_playing`] wraps in the LCD well: the
/// indicator strip lit by `modes`, then the elapsed/title row and status row
/// in a filling column so the total stays at the right edge. Generic over the
/// theme and renderer, unlike the concrete well wrapper, so the tests can lay
/// it out with iced's null renderer and read back the text it would draw.
// Mirrors `view_now_playing`'s arguments (without the well wrapper) so the
// tests can lay out the same content headlessly.
#[allow(clippy::too_many_arguments)]
pub(super) fn now_playing_content<'a, Theme, Renderer>(
    elapsed: Cow<'a, str>,
    label: Cow<'a, str>,
    artist: Cow<'a, str>,
    number: Cow<'a, str>,
    time: Cow<'a, str>,
    marquee_offset: usize,
    format: Option<PlaybackFormat>,
    spectrum: Option<Spectrum>,
    peaks: Option<Spectrum>,
    mode: TransportMode,
    elapsed_visible: bool,
    mixer_readout: Option<String>,
    modes: IndicatorModes,
) -> Row<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + iced::widget::container::Catalog + 'a,
    <Theme as iced::widget::text::Catalog>::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    <Theme as iced::widget::container::Catalog>::Class<'a>:
        From<iced::widget::container::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer + 'a,
{
    Row::new().push(lcd_indicator_strip(modes)).push(
        Column::new()
            .width(Length::Fill)
            .push(now_playing_row(
                elapsed,
                label,
                artist,
                number,
                time,
                marquee_offset,
                mode,
                elapsed_visible,
                mixer_readout,
            ))
            .push(now_playing_visualizer_row(spectrum, peaks))
            .push(now_playing_status_row(format)),
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
// Mirrors `now_playing_content`'s arguments (without the status and visualizer
// rows) so the row can be laid out headlessly on its own.
#[allow(clippy::too_many_arguments)]
pub(super) fn now_playing_row<'a, Theme, Renderer>(
    elapsed: Cow<'a, str>,
    label: Cow<'a, str>,
    artist: Cow<'a, str>,
    number: Cow<'a, str>,
    time: Cow<'a, str>,
    marquee_offset: usize,
    mode: TransportMode,
    elapsed_visible: bool,
    mixer_readout: Option<String>,
) -> Row<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + iced::widget::container::Catalog + 'a,
    <Theme as iced::widget::text::Catalog>::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    <Theme as iced::widget::container::Catalog>::Class<'a>:
        From<iced::widget::container::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer + 'a,
{
    let line_len = display_len(&label, &artist, &number);
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
        .push(Space::new().width(Length::Fixed(12.0)));
    if let Some(text) = mixer_readout {
        row = row.push(lcd_text(text));
    } else {
        row = row.push(Text::new("Now Playing: ").size(20));
        row = push_title(row, label, artist, number, marquee_offset, line_len);
    }
    row.push(Space::new().width(Length::Fill))
        .push(lcd_text(time))
}

/// Pushes the title portion of the LCD row: the whole number, artist, and
/// label when they fit, else the marquee window at `marquee_offset`.
fn push_title<'a, Theme, Renderer>(
    mut row: Row<'a, Message, Theme, Renderer>,
    label: Cow<'a, str>,
    artist: Cow<'a, str>,
    number: Cow<'a, str>,
    marquee_offset: usize,
    line_len: usize,
) -> Row<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + 'a,
    <Theme as iced::widget::text::Catalog>::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer + 'a,
{
    if line_len <= MARQUEE_WIDTH {
        if !number.is_empty() {
            row = row.push(lcd_text(number));
        }
        if !artist.is_empty() {
            row = row.push(lcd_text(artist)).push(lcd_text(" - "));
        }
        row = row.push(lcd_text(label));
    } else {
        let display = format!("{number}{}", display_title_for(&artist, &label));
        row = row.push(
            MouseArea::new(lcd_text(marquee_text(
                &display,
                marquee_offset,
                MARQUEE_WIDTH,
            )))
            .on_press(Message::TitleDragPressed)
            .interaction(Interaction::Grab),
        );
    }
    row
}
