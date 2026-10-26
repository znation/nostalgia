//! The LCD's visualizer row: the stored spectrum's clamped bar heights, the
//! fixed-width bars that draw them, and the flag that hides the bars while the
//! spectrum poll keeps running.

use iced::{
    Color, Length,
    alignment::Vertical,
    mouse::Interaction,
    widget::{Column, Container, MouseArea, Row, Space, container},
};

use crate::audio::Spectrum;
use crate::clamp::clamp_with_nan_fallback;
use crate::equalizer::BAND_COUNT;
use crate::ui::Message;

/// The fixed height of the LCD's visualizer row, in pixels, and the maximum
/// height of one bar. The row keeps this height whether or not a preview
/// reports a spectrum, so loading one does not move the title or status text.
const LCD_VISUALIZER_HEIGHT: f32 = 24.0;

/// The fixed width of one visualizer bar, in pixels, so the ten bars keep
/// their places as their heights change.
const LCD_VISUALIZER_BAR_WIDTH: f32 = 8.0;

/// The gap between adjacent visualizer bars, in pixels.
const LCD_VISUALIZER_BAR_GAP: f32 = 3.0;

/// The height of one ramp step, in pixels. The steps are absolute rows above
/// the rail rather than a gradient stretched over each bar, so a bar's color
/// follows its height: a short bar stops at the green low steps, and only a
/// full-height bar reaches the orange peak.
const LCD_VISUALIZER_RAMP_STEP: f32 = 2.0;

/// The analyzer's color ramp, bottom to top: dark green at the rail through
/// yellow to orange at the peaks, sampled from the mockup's 24 px analyzer.
const LCD_VISUALIZER_RAMP: [Color; 12] = [
    Color::from_rgb8(24, 132, 8),
    Color::from_rgb8(41, 148, 0),
    Color::from_rgb8(49, 156, 8),
    Color::from_rgb8(57, 181, 16),
    Color::from_rgb8(50, 190, 16),
    Color::from_rgb8(41, 206, 16),
    Color::from_rgb8(148, 222, 33),
    Color::from_rgb8(189, 222, 41),
    Color::from_rgb8(214, 181, 33),
    Color::from_rgb8(222, 165, 24),
    Color::from_rgb8(198, 123, 8),
    Color::from_rgb8(214, 115, 0),
];

/// The spectrum the LCD should draw: the stored bars only while the visualizer
/// is visible, otherwise nothing. Hiding changes only what the LCD draws —
/// `WinampPlayer::spectrum` and the spectrum poll keep running, so turning the
/// visualizer back on shows the current bars instead of waiting for a new
/// reply.
pub fn visible_spectrum(spectrum: Option<Spectrum>, visualizer_visible: bool) -> Option<Spectrum> {
    if visualizer_visible { spectrum } else { None }
}

/// Resolves the visualizer's bar heights from the loaded preview's spectrum.
/// A missing spectrum yields all-zero bars, and each reported band is clamped
/// to `0.0..=1.0` before it scales the fixed row height, so the bars stay
/// inside their strip. Pure so the scaling is testable without a renderer.
pub fn visualizer_bar_heights(spectrum: Option<Spectrum>) -> [f32; BAND_COUNT] {
    let bands = spectrum.map_or([0.0; BAND_COUNT], |spectrum| spectrum.bands);
    bands.map(|band| clamp_with_nan_fallback(band, 0.0, 1.0, 0.0) * LCD_VISUALIZER_HEIGHT)
}

/// Splits a bar `height` into the ramp steps it reaches, bottom first: each
/// entry is the segment's visible height and its color. A step is one
/// [`LCD_VISUALIZER_RAMP_STEP`]-tall absolute row above the rail, so a taller
/// bar adds the next color rather than stretching the ramp over itself. The
/// height clamps to `0.0..=LCD_VISUALIZER_HEIGHT` with NaN mapped to zero, and
/// the returned heights sum to the clamped height so a bar keeps its measured
/// size. Zero-height steps are omitted.
pub fn visualizer_bar_segments(height: f32) -> Vec<(f32, Color)> {
    let height = clamp_with_nan_fallback(height, 0.0, LCD_VISUALIZER_HEIGHT, 0.0);
    LCD_VISUALIZER_RAMP
        .iter()
        .enumerate()
        .filter_map(|(index, color)| {
            let shown = (height - index as f32 * LCD_VISUALIZER_RAMP_STEP)
                .clamp(0.0, LCD_VISUALIZER_RAMP_STEP);
            (shown > 0.0).then_some((shown, *color))
        })
        .collect()
}

/// Builds the LCD's visualizer row: [`BAND_COUNT`] bottom-aligned bars, left to
/// right in band order, inside a row of fixed [`LCD_VISUALIZER_HEIGHT`]. Each
/// bar is shaded from dark green at the rail up its earned ramp steps to orange
/// at its top. Every band keeps a fixed-width slot even when its height is
/// zero, so a real spectrum with silent bands still draws all ten bars. A
/// missing spectrum draws the same row with zero-height bars, so the title and
/// status text keep their places in every playback state. Generic
/// over the theme and renderer so the tests can lay it out with iced's null
/// renderer. The row is a mouse area: a left press publishes
/// [`Message::ToggleVisualizer`] in every playback state, whether the bars are
/// shown or hidden, exactly like the indicator strip's `V`.
pub fn now_playing_visualizer_row<'a, Theme, Renderer>(
    spectrum: Option<Spectrum>,
) -> MouseArea<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + iced::widget::container::Catalog + 'a,
    <Theme as iced::widget::text::Catalog>::Class<'a>: From<iced::widget::text::StyleFn<'a, Theme>>,
    <Theme as iced::widget::container::Catalog>::Class<'a>:
        From<iced::widget::container::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::text::Renderer + 'a,
{
    let row = visualizer_bar_heights(spectrum).into_iter().fold(
        Row::new()
            .height(Length::Fixed(LCD_VISUALIZER_HEIGHT))
            .spacing(LCD_VISUALIZER_BAR_GAP)
            .align_y(Vertical::Bottom),
        |row, height| {
            // The bar's ramp segments stack from the top down, above a spacer
            // that fills the part of the strip its height does not reach. A
            // silent bar keeps its fixed-width slot through the column alone:
            // it draws no segment, but the column still has a nonzero fixed
            // size, so `Row::push` does not drop it.
            let mut column = Column::new()
                .width(Length::Fixed(LCD_VISUALIZER_BAR_WIDTH))
                .height(Length::Fixed(LCD_VISUALIZER_HEIGHT));
            let segments = visualizer_bar_segments(height);
            let spacer_height = LCD_VISUALIZER_HEIGHT - height;
            if spacer_height > 0.0 {
                column = column.push(Space::new().height(Length::Fixed(spacer_height)));
            }
            for (segment_height, color) in segments.into_iter().rev() {
                column = column.push(
                    Container::new(Space::new())
                        .width(Length::Fill)
                        .height(Length::Fixed(segment_height))
                        .style(move |_theme| container::Style {
                            background: Some(color.into()),
                            ..container::Style::default()
                        }),
                );
            }
            row.push(column)
        },
    );
    MouseArea::new(row)
        .on_press(Message::ToggleVisualizer)
        .interaction(Interaction::Pointer)
}
