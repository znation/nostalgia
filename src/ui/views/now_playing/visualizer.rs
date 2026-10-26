//! The LCD's visualizer row: the stored spectrum's clamped bar heights, the
//! fixed-width bars that draw them, and the flag that hides the bars while the
//! spectrum poll keeps running.

use iced::{
    Length,
    alignment::Vertical,
    mouse::Interaction,
    widget::{Container, MouseArea, Row, Space, container},
};

use crate::audio::Spectrum;
use crate::clamp::clamp_with_nan_fallback;
use crate::equalizer::BAND_COUNT;
use crate::ui::{Message, theme};

/// The fixed height of the LCD's visualizer row, in pixels, and the maximum
/// height of one bar. The row keeps this height whether or not a preview
/// reports a spectrum, so loading one does not move the title or status text.
const LCD_VISUALIZER_HEIGHT: f32 = 24.0;

/// The fixed width of one visualizer bar, in pixels, so the ten bars keep
/// their places as their heights change.
const LCD_VISUALIZER_BAR_WIDTH: f32 = 8.0;

/// The gap between adjacent visualizer bars, in pixels.
const LCD_VISUALIZER_BAR_GAP: f32 = 3.0;

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

/// Builds the LCD's visualizer row: exactly [`BAND_COUNT`] bottom-aligned green
/// bars, left to right in band order, inside a row of fixed
/// [`LCD_VISUALIZER_HEIGHT`]. Every band keeps a fixed-width slot even when its
/// height is zero, so a real spectrum with silent bands still draws all ten
/// bars. A missing spectrum draws the same row with zero-height bars, so the
/// title and status text keep their places in every playback state. Generic
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
    );
    MouseArea::new(row)
        .on_press(Message::ToggleVisualizer)
        .interaction(Interaction::Pointer)
}
