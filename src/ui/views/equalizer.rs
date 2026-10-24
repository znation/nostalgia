//! The equalizer panel: the on/off button, the preset pick list, the response
//! curve, and one row of a vertical preamp slider, a dB scale, and the
//! [`BAND_COUNT`] vertical band sliders.

use iced::{
    Element, Length,
    widget::{Column, Container, PickList, Row, Space, Text, VerticalSlider, container, slider},
};

use crate::equalizer::{
    BAND_COUNT, BAND_FREQUENCIES, GAIN_MAX_DB, GAIN_MIN_DB, PRESETS, Preset, clamp_gain,
};
use crate::ui::{Message, bevel, style, theme};

use super::{EQ_STEP, labeled_button, spacer};

/// The horizontal gap between adjacent equalizer band columns. Without it the
/// labels sit edge to edge, so `170`, `310`, `600`, and `1K` read as one
/// number in the rendered panel.
const BAND_ROW_SPACING: f32 = 6.0;

/// The height of every equalizer slider's rail, in pixels. The preamp, the dB
/// scale, and the band sliders share it so their labels land on one row.
pub(super) const EQ_SLIDER_HEIGHT: f32 = 100.0;

/// The response curve's horizontal resolution: one green segment per column,
/// so the line reads as a smooth trace over the ten band sliders.
pub(super) const RESPONSE_CURVE_COLUMNS: usize = 64;

/// The fixed height of the response curve's plot, in pixels. The panel keeps
/// it whether the curve is flat or shaped, so the band sliders below never
/// move.
pub(super) const RESPONSE_CURVE_HEIGHT: f32 = 24.0;

/// The height of one response-curve segment, in pixels. Adjacent segments
/// touch, so the dots read as one continuous line.
pub(super) const RESPONSE_CURVE_LINE_HEIGHT: f32 = 2.0;

/// The equalizer on/off button's label: "EQ: On" while the equalizer is
/// engaged, "EQ: Off" when it is off. Pure so the label logic is testable
/// without an iced `Element`, like
/// [`repeat_label`](super::transport::repeat_label).
pub(super) fn eq_enabled_label(enabled: bool) -> &'static str {
    if enabled { "EQ: On" } else { "EQ: Off" }
}

/// Resolves the response curve's segment offsets from the stored band gains.
///
/// The result holds exactly `columns` values, each the segment's height above
/// the plot's bottom: `0.0` at [`GAIN_MIN_DB`], `0.5` at `0.0`, and `1.0` at
/// [`GAIN_MAX_DB`]. Column 0 sits on the first band and the last column on the
/// last band; every intermediate column linearly interpolates between the two
/// adjacent bands, so the line follows the same gains the sliders and
/// `BandFilter` use. Each band passes through [`clamp_gain`] first, so a NaN or
/// out-of-range stored gain cannot leave the plot.
///
/// `columns == 0` returns an empty vector, and `columns == 1` returns the first
/// band's offset rather than dividing by zero. Pure so the shape is testable
/// without an iced `Element`.
pub(super) fn response_curve_offsets(bands: &[f32; BAND_COUNT], columns: usize) -> Vec<f32> {
    if columns == 0 {
        return Vec::new();
    }

    let offsets: Vec<f32> = bands
        .iter()
        .map(|gain| (clamp_gain(*gain) - GAIN_MIN_DB) / (GAIN_MAX_DB - GAIN_MIN_DB))
        .collect();
    if columns == 1 {
        return vec![offsets[0]];
    }

    let band_span = (BAND_COUNT - 1) as f32;
    let column_step = band_span / (columns - 1) as f32;
    (0..columns)
        .map(|column| {
            let position = column as f32 * column_step;
            let lower = (position.floor() as usize).min(BAND_COUNT - 1);
            if lower + 1 == BAND_COUNT {
                return offsets[lower];
            }
            let fraction = position - lower as f32;
            offsets[lower] + fraction * (offsets[lower + 1] - offsets[lower])
        })
        .collect()
}

/// Builds the equalizer's dB scale: `+12 DB` at the rail's top, `0 DB` in the
/// middle, and `-12 DB` at its bottom. The column is exactly
/// [`EQ_SLIDER_HEIGHT`] tall and its two fill `Space`s split the leftover height
/// evenly, so the middle label centers on the rail beside it.
pub(super) fn db_scale_column() -> Column<'static, Message> {
    Column::new()
        .width(Length::Shrink)
        .height(Length::Fixed(EQ_SLIDER_HEIGHT))
        .push(Text::new("+12 DB").size(12))
        .push(Space::new().height(Length::Fill))
        .push(Text::new("0 DB").size(12))
        .push(Space::new().height(Length::Fill))
        .push(Text::new("-12 DB").size(12))
}

/// Builds one equalizer slider column: a [`VerticalSlider`] over a gain, sized
/// to [`EQ_SLIDER_HEIGHT`] and stepped by [`EQ_STEP`], with its `label`
/// beneath it at the band labels' text size. The preamp column and each of the
/// ten band columns share this shape; only the slider's value, change
/// message, and paint style differ, so the construction lives here once.
///
/// `style` receives the slider's interaction status and returns the chrome
/// style to paint, so the preamp and each band pass their own gain tint
/// through [`style::band_slider_style`].
fn eq_slider_column<'a>(
    value: f32,
    label: &'a str,
    on_change: impl Fn(f32) -> Message + 'a,
    style: impl Fn(slider::Status) -> slider::Style + 'a,
) -> Column<'a, Message> {
    Column::new()
        .push(
            VerticalSlider::new(GAIN_MIN_DB..=GAIN_MAX_DB, value, on_change)
                .step(EQ_STEP)
                .height(Length::Fixed(EQ_SLIDER_HEIGHT))
                .style(move |_theme, status| style(status)),
        )
        .push(Text::new(label).size(12))
}

/// Builds the equalizer's preamp column: a [`VerticalSlider`] over the
/// `Preamp` label, matching a band column's shape so it sits in the band row.
/// The slider spans `GAIN_MIN_DB..=GAIN_MAX_DB` in [`EQ_STEP`] increments and
/// takes the same gain tint as every band slider, from the stored preamp.
pub(super) fn preamp_column(preamp: f32) -> Column<'static, Message> {
    eq_slider_column(preamp, "Preamp", Message::EqPreampChange, move |status| {
        preamp_slider_style(preamp, status)
    })
}

/// The preamp slider's paint style: exactly the band tint for the preamp's
/// gain, so the preamp groove matches a band slider at the same gain.
pub(super) fn preamp_slider_style(preamp: f32, status: slider::Status) -> slider::Style {
    style::band_slider_style(preamp, status)
}

/// Builds the response curve row: one green line segment per
/// [`RESPONSE_CURVE_COLUMNS`] column, placed at the height
/// [`response_curve_offsets`] gives for the stored band gains. The row keeps
/// the fixed [`RESPONSE_CURVE_HEIGHT`] for every curve, flat included, so the
/// band sliders below keep their places. The preamp is not plotted: it is a
/// uniform gain offset, not a bend in the curve. Generic over the theme and
/// renderer so the tests can lay it out with iced's null renderer.
pub(super) fn response_curve_row<'a, Theme, Renderer>(
    bands: &[f32; BAND_COUNT],
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::container::Catalog + 'a,
    <Theme as iced::widget::container::Catalog>::Class<'a>:
        From<iced::widget::container::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::Renderer + 'a,
{
    let line_span = RESPONSE_CURVE_HEIGHT - RESPONSE_CURVE_LINE_HEIGHT;
    let mut row = Row::new().width(Length::Fill).height(Length::Fill);
    for offset in response_curve_offsets(bands, RESPONSE_CURVE_COLUMNS) {
        let above = (1.0 - offset) * line_span;
        let below = offset * line_span;
        let segment = Container::new(Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(RESPONSE_CURVE_LINE_HEIGHT))
            .style(|_theme| container::Style {
                background: Some(theme::LCD_GREEN.into()),
                ..container::Style::default()
            });
        row = row.push(
            Container::new(
                Column::new()
                    .width(Length::Fill)
                    .push(Space::new().height(Length::Fixed(above)))
                    .push(segment)
                    .push(Space::new().height(Length::Fixed(below))),
            )
            .width(Length::Fill),
        );
    }

    Container::new(row)
        .width(Length::Fill)
        .height(Length::Fixed(RESPONSE_CURVE_HEIGHT))
        .style(|_theme| container::Style {
            background: Some(theme::LCD_BACKGROUND.into()),
            ..container::Style::default()
        })
        .into()
}

/// The equalizer panel: an on/off button, a preset pick list, a response
/// curve above the [`BAND_COUNT`] band sliders, and to their left the vertical
/// preamp slider and the `+12 DB` / `0 DB` / `-12 DB` scale. The curve starts at
/// the first band, so its column 0 sits on that band rather than over the
/// preamp.
///
/// `enabled` is the shared EQ on/off flag shown on the button and toggled by
/// pressing it; `preset` is the applied curve the pick list shows (or `None`,
/// drawn as the "(none)" custom-curve placeholder), and picking one emits
/// [`Message::EqPresetSelected`]; `preamp` and `bands` are the stored gains the
/// sliders start from. Every slider spans `GAIN_MIN_DB..=GAIN_MAX_DB` in 1 dB steps and
/// emits its own change message, so dragging one routes a clamped gain back
/// into shared state. The response curve traces the stored `bands` whether the
/// EQ is on or off, and each band slider's groove is tinted by that band's own
/// gain through [`style::band_slider_style`]. The band row is built by index so
/// each slider's closure captures its own band number — the vertical twin of
/// the volume slider in
/// [`view_transport_controls`](super::transport::view_transport_controls).
pub fn view_equalizer(
    enabled: bool,
    preamp: f32,
    bands: &[f32; BAND_COUNT],
    preset: Option<Preset>,
) -> Element<'static, Message> {
    let mut bands_row = Row::new().spacing(BAND_ROW_SPACING);
    for (index, frequency) in BAND_FREQUENCIES.iter().enumerate() {
        let gain = bands[index];
        bands_row = bands_row.push(eq_slider_column(
            gain,
            frequency,
            move |gain| Message::EqBandChange(index, gain),
            move |status| style::band_slider_style(gain, status),
        ));
    }

    let preamp_and_scale = Row::new()
        .spacing(BAND_ROW_SPACING)
        .push(preamp_column(preamp))
        .push(db_scale_column());

    // The curve spans only the band area: the left column's curve-height
    // spacer keeps the preamp and scale in the band row while the right
    // column starts both the curve and the bands at the first band's x, so the
    // curve's column 0 sits on the first band instead of over the preamp.
    let sliders = Row::new()
        .spacing(BAND_ROW_SPACING)
        .push(
            Column::new()
                .push(Space::new().height(Length::Fixed(RESPONSE_CURVE_HEIGHT)))
                .push(preamp_and_scale),
        )
        .push(
            Column::new()
                .push(response_curve_row(bands))
                .push(bands_row),
        );

    bevel::raised_panel(
        Column::new()
            .push(
                Row::new()
                    .push(labeled_button(
                        eq_enabled_label(enabled),
                        Message::ToggleEqualizer,
                    ))
                    .push(spacer(8.0))
                    .push(
                        // `PickList` stores its options by value, so passing the
                        // `[Preset; 19]` array copies ~1.2 KB into the widget on
                        // every rebuild; passing the slice borrows the static
                        // array instead. The pick list only ever reads the options
                        // through `Borrow<[Preset]>`, so the slice is equivalent.
                        PickList::new(PRESETS.as_slice(), preset, Message::EqPresetSelected)
                            .placeholder("(none)")
                            .text_size(12)
                            .style(|_theme, status| style::chrome_pick_list_style(status))
                            .menu_style(|_theme| style::preset_menu_style()),
                    ),
            )
            .push(sliders),
    )
}
