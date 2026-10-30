//! The equalizer panel: the on/off button, the preset pick list, the response curve with its dim 0
//! dB and band guides, and one row of a vertical preamp slider, a dB scale, and the [`BAND_COUNT`]
//! vertical band sliders.

use iced::{
    Element, Length,
    mouse::{Interaction, ScrollDelta},
    widget::{
        Column, Container, MouseArea, PickList, Row, Space, Text, VerticalSlider, container, slider,
    },
};

use crate::equalizer::{
    BAND_COUNT, BAND_FREQUENCIES, CustomPreset, GAIN_MAX_DB, GAIN_MIN_DB, PRESETS, Preset,
    PresetChoice, clamp_gain,
};
use crate::ui::{Message, bevel, style, theme};

use iced::widget::Button;

use super::{EQ_STEP, fixed_width_toggle_button, spacer, toggle_button};

/// The horizontal gap between adjacent equalizer band columns. Without it the labels sit edge to
/// edge, so `170`, `310`, `600`, and `1K` read as one number in the rendered panel.
const BAND_ROW_SPACING: f32 = 6.0;

/// The height of every equalizer slider's rail, in pixels. The preamp, the dB scale, and the band
/// sliders share it so their labels land on one row.
pub(super) const EQ_SLIDER_HEIGHT: f32 = 100.0;

/// The preamp slider's container id, used only by the layout tests to find the laid-out slider and
/// publish a wheel roll over it.
#[cfg(test)]
pub(crate) const EQ_PREAMP_SLIDER_ID: &str = "equalizer-preamp";

/// The container id for equalizer band `index`, the test-only counterpart of
/// [`EQ_PREAMP_SLIDER_ID`].
#[cfg(test)]
pub(crate) fn eq_band_slider_id(index: usize) -> String {
    format!("equalizer-band-{index}")
}

/// The response curve's horizontal resolution: one green segment per column, so the line reads as a
/// smooth trace over the ten band sliders.
pub(super) const RESPONSE_CURVE_COLUMNS: usize = 64;

/// The fixed height of the response curve's plot, in pixels. The panel keeps it whether the curve
/// is flat or shaped, so the band sliders below never move.
pub(super) const RESPONSE_CURVE_HEIGHT: f32 = 24.0;

/// The height of one response-curve segment, in pixels. Adjacent segments touch, so the dots read
/// as one continuous line.
pub(super) const RESPONSE_CURVE_LINE_HEIGHT: f32 = 2.0;

/// The thickness of the response curve's reference guide, in pixels: the size of each dot on the 0
/// dB line and the width of each band marker. It is kept distinct from
/// [`RESPONSE_CURVE_LINE_HEIGHT`] so the trace's height filter still counts only trace segments.
pub(super) const RESPONSE_CURVE_GUIDE_THICKNESS: f32 = 1.0;

/// The equalizer on/off button's label: "EQ: On" while the equalizer is engaged, "EQ: Off" when it
/// is off. Pure so the label logic is testable without an iced `Element`, like
/// the options menu's row label helpers ([`repeat_label`](super::options::repeat_label)).
pub(super) fn eq_enabled_label(enabled: bool) -> &'static str {
    if enabled { "EQ: On" } else { "EQ: Off" }
}

/// The equalizer AUTO button's label: "AUTO: On" while genre-matched presets are enabled, "AUTO:
/// Off" when they are not. Pure so the label logic is testable without an iced `Element`.
pub(super) fn auto_label(auto: bool) -> &'static str {
    if auto { "AUTO: On" } else { "AUTO: Off" }
}

/// The AUTO button's fixed width, pinned wide enough for `AUTO: Off` so swapping the label cannot
/// reflow the preset pick list beside it.
pub(super) const AUTO_BUTTON_WIDTH: f32 = 96.0;

/// The on-grid size the equalizer's dB-scale and band/preamp labels hand iced,
/// in pixels: the out-of-grid 12 they replaced, rebased onto the 8-px grid the
/// vendored faces shape crisply (only multiples of 8 are crisp), drawn in
/// [`theme::ui_font`].
const EQ_TEXT_SIZE: f32 = 16.0;

/// The on-grid size of the preset pick list's closed label and items, the 12
/// it replaced — [`theme::ui_font`] again.
const PICK_LIST_TEXT_SIZE: f32 = 16.0;

/// The equalizer on/off button: sunken while `enabled`, ordinary while off.
pub(super) fn eq_toggle(enabled: bool) -> Button<'static, Message> {
    toggle_button(
        eq_enabled_label(enabled),
        theme::ui_font(),
        Message::ToggleEqualizer,
        enabled,
    )
}

/// The AUTO button: sunken while `auto`, ordinary while off, pinned to [`AUTO_BUTTON_WIDTH`].
pub(super) fn auto_toggle(auto: bool) -> Button<'static, Message> {
    fixed_width_toggle_button(
        auto_label(auto),
        AUTO_BUTTON_WIDTH,
        theme::ui_font(),
        Message::ToggleEqAuto,
        auto,
    )
}

/// The preset pick list's label when no named preset is applied. Classic Winamp names a hand-tuned
/// curve `Custom`, so the closed control names the curve instead of showing an empty placeholder
/// while `AppState::eq_preset` is `None`, including a fresh launch with a flat curve.
pub(super) const CUSTOM_PRESET_LABEL: &str = "Custom";

/// The equalizer pick list's closed label: an applied preset's name, or [`CUSTOM_PRESET_LABEL`] for
/// a hand-tuned curve. `view_equalizer` passes it as the placeholder, which iced draws only while
/// no preset is selected.
pub(super) fn preset_pick_list_label(preset: Option<Preset>) -> &'static str {
    preset.map_or(CUSTOM_PRESET_LABEL, |preset| preset.name)
}

/// Builds the pick list's items for one refresh: the `Save preset…` row
/// first, then the built-ins in [`PRESETS`] order, then `customs` in stored
/// order. Rebuilt every refresh — [`PRESETS`] is static, but a saved custom's
/// row appears the moment the naming panel saves it — and pure so the order
/// is testable without an iced `Element`. The closed label still resolves
/// through [`preset_pick_list_label`] unchanged: it names only built-ins, so
/// an applied custom keeps reading [`CUSTOM_PRESET_LABEL`].
pub(super) fn preset_choices(customs: &[CustomPreset]) -> Vec<PresetChoice> {
    let mut choices = Vec::with_capacity(PRESETS.len() + customs.len() + 1);
    choices.push(PresetChoice::Save);
    choices.extend(PRESETS.iter().copied().map(PresetChoice::BuiltIn));
    choices.extend(customs.iter().cloned().map(PresetChoice::Custom));
    choices
}

/// Resolves the response curve's segment offsets from the stored band gains.
///
/// The result holds exactly `columns` values, each the segment's height above the plot's bottom:
/// `0.0` at [`GAIN_MIN_DB`], `0.5` at `0.0`, and `1.0` at [`GAIN_MAX_DB`]. Column 0 sits on the
/// first band and the last column on the last band; each span between adjacent bands is eased with
/// the cubic smoothstep `f * f * (3 - 2f)`, so the line meets every band at a zero slope instead of
/// leaving a corner. The curve still passes exactly through each band's gain, so it follows the
/// same gains the sliders and `BandFilter` use. A flat, all-minimum, or all-maximum curve is
/// unchanged. Each band passes through [`clamp_gain`] first, so a NaN or out-of-range stored gain
/// cannot leave the plot; the smoothstep is monotone and the endpoint offsets are clamped, so no
/// eased value leaves `0.0..=1.0` either.
///
/// `columns == 0` returns an empty vector, and `columns == 1` returns the first band's offset
/// rather than dividing by zero. Pure so the shape is testable without an iced `Element`.
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
            let eased = fraction * fraction * (3.0 - 2.0 * fraction);
            offsets[lower] + eased * (offsets[lower + 1] - offsets[lower])
        })
        .collect()
}

/// Builds the equalizer's dB scale: `+12 DB` at the rail's top, `0 DB` in the middle, and `-12 DB`
/// at its bottom. The column is exactly [`EQ_SLIDER_HEIGHT`] tall and its two fill `Space`s split
/// the leftover height evenly, so the middle label centers on the rail beside it. Pressing a label
/// sends [`Message::EqBandsSetAll`] with its gain, as in Winamp.
pub(super) fn db_scale_column() -> Column<'static, Message> {
    Column::new()
        .width(Length::Shrink)
        .height(Length::Fixed(EQ_SLIDER_HEIGHT))
        .push(scale_label("+12 DB", GAIN_MAX_DB))
        .push(Space::new().height(Length::Fill))
        .push(scale_label("0 DB", 0.0))
        .push(Space::new().height(Length::Fill))
        .push(scale_label("-12 DB", GAIN_MIN_DB))
}

/// One clickable dB scale label that sets every band to `gain`.
fn scale_label(label: &'static str, gain: f32) -> MouseArea<'static, Message> {
    MouseArea::new(Text::new(label).size(EQ_TEXT_SIZE).font(theme::ui_font()))
        .on_press(Message::EqBandsSetAll(gain))
        .interaction(Interaction::Pointer)
}

/// Builds one equalizer slider column: a [`VerticalSlider`] over a gain, sized to
/// [`EQ_SLIDER_HEIGHT`] and stepped by [`EQ_STEP`], with its `label` beneath it at the band labels'
/// text size. The preamp column and each of the ten band columns share this shape; only the
/// slider's value, change message, and paint style differ, so the construction lives here once.
/// Releasing the slider sends [`Message::MixerReleased`], ending the LCD's gain readout.
///
/// `style` receives the slider's interaction status and returns the chrome style to paint, so the
/// preamp and each band pass their own gain tint through [`style::band_slider_style`]. `on_scroll`
/// receives a plain wheel roll over the slider and returns that slider's wheel message; the
/// slider's own Ctrl+wheel step still runs first and keeps the wheel when it captures it.
fn eq_slider_column<'a>(
    value: f32,
    label: &'a str,
    on_change: impl Fn(f32) -> Message + 'a,
    on_scroll: impl Fn(ScrollDelta) -> Message + 'a,
    style: impl Fn(slider::Status) -> slider::Style + 'a,
) -> Column<'a, Message> {
    Column::new()
        .push(
            MouseArea::new(
                VerticalSlider::new(GAIN_MIN_DB..=GAIN_MAX_DB, value, on_change)
                    .step(EQ_STEP)
                    .on_release(Message::MixerReleased)
                    .height(Length::Fixed(EQ_SLIDER_HEIGHT))
                    .style(move |_theme, status| style(status)),
            )
            .on_scroll(on_scroll),
        )
        .push(Text::new(label).size(EQ_TEXT_SIZE).font(theme::ui_font()))
}

/// Builds the equalizer's preamp column: a [`VerticalSlider`] over the `Preamp` label, matching a
/// band column's shape so it sits in the band row. The slider spans `GAIN_MIN_DB..=GAIN_MAX_DB` in
/// [`EQ_STEP`] increments and takes the same gain tint as every band slider, from the stored
/// preamp.
pub(super) fn preamp_column(preamp: f32) -> Column<'static, Message> {
    eq_slider_column(
        preamp,
        "Preamp",
        Message::EqPreampChange,
        super::eq_preamp_wheel_message,
        move |status| preamp_slider_style(preamp, status),
    )
}

/// The preamp slider's paint style: exactly the band tint for the preamp's gain, so the preamp
/// groove matches a band slider at the same gain.
pub(super) fn preamp_slider_style(preamp: f32, status: slider::Status) -> slider::Style {
    style::band_slider_style(preamp, status)
}

/// Builds the response curve row: one green line segment per [`RESPONSE_CURVE_COLUMNS`] column,
/// placed at the height [`response_curve_offsets`] gives for the stored band gains. Behind the
/// trace it draws a dim reference grid: a dotted line on the 0 dB midline and one full-height
/// marker at each band's `i / (BAND_COUNT - 1)` position, from the plot's left edge to its right
/// edge, both in [`theme::LCD_GREEN_DIM`]. The trace draws last, over the guide. The row keeps the
/// fixed [`RESPONSE_CURVE_HEIGHT`] for every curve, flat included, so the band sliders below keep
/// their places. The preamp is not plotted: it is a uniform gain offset, not a bend in the curve.
/// Generic over the theme and renderer so the tests can lay it out with iced's null renderer.
pub(super) fn response_curve_row<'a, Theme, Renderer>(
    bands: &[f32; BAND_COUNT],
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::container::Catalog + 'a,
    <Theme as iced::widget::container::Catalog>::Class<'a>:
        From<iced::widget::container::StyleFn<'a, Theme>>,
    Renderer: iced::advanced::Renderer + 'a,
{
    let guide_style = |_theme: &Theme| container::Style {
        background: Some(theme::LCD_GREEN_DIM.into()),
        ..container::Style::default()
    };

    let mut markers = Row::new().width(Length::Fill).height(Length::Fill);
    for index in 0..BAND_COUNT {
        markers = markers.push(
            Container::new(Space::new())
                .width(Length::Fixed(RESPONSE_CURVE_GUIDE_THICKNESS))
                .height(Length::Fill)
                .style(guide_style),
        );
        if index + 1 < BAND_COUNT {
            markers = markers.push(Space::new().width(Length::Fill));
        }
    }

    let mut zero_guide = Row::new()
        .width(Length::Fill)
        .height(Length::Fixed(RESPONSE_CURVE_GUIDE_THICKNESS));
    for _ in 0..RESPONSE_CURVE_COLUMNS {
        let dot = Container::new(Space::new())
            .width(Length::Fixed(RESPONSE_CURVE_GUIDE_THICKNESS))
            .height(Length::Fixed(RESPONSE_CURVE_GUIDE_THICKNESS))
            .style(guide_style);
        zero_guide = zero_guide.push(Container::new(dot).center_x(Length::Fill));
    }
    let guide_half = (RESPONSE_CURVE_HEIGHT - RESPONSE_CURVE_GUIDE_THICKNESS) / 2.0;
    let zero_guide = Column::new()
        .width(Length::Fill)
        .height(Length::Fixed(RESPONSE_CURVE_HEIGHT))
        .push(Space::new().height(Length::Fixed(guide_half)))
        .push(zero_guide)
        .push(Space::new().height(Length::Fixed(guide_half)));

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

    let stack = iced::widget::stack([markers.into(), zero_guide.into(), row.into()])
        .width(Length::Fill)
        .height(Length::Fixed(RESPONSE_CURVE_HEIGHT));

    Container::new(stack)
        .width(Length::Fill)
        .height(Length::Fixed(RESPONSE_CURVE_HEIGHT))
        .style(|_theme| container::Style {
            background: Some(theme::LCD_BACKGROUND.into()),
            ..container::Style::default()
        })
        .into()
}

/// The equalizer panel: an on/off button, an AUTO toggle, a preset pick list, a response curve
/// above the [`BAND_COUNT`] band sliders, and to their left the vertical preamp slider and the
/// `+12 DB` / `0 DB` / `-12 DB` scale. The curve starts at the first band, so its column 0 sits on
/// that band rather than over the preamp.
///
/// `enabled` is the shared EQ on/off flag shown on the button and toggled by pressing it; `auto` is
/// the genre-matched preset flag shown on the AUTO button and toggled by pressing it; `preset` is
/// the applied curve the pick list shows (or `None`, drawn as [`CUSTOM_PRESET_LABEL`]), and picking
/// one emits [`Message::EqPresetSelected`]; the pick list's items are rebuilt
/// every refresh into a fresh `Vec<PresetChoice>` — the `Save preset…` row
/// first, then the built-ins in [`PRESETS`] order, then the player's saved
/// customs in stored order (see [`preset_choices`]). `preamp` and `bands` are
/// the stored gains the sliders
/// start from. Every slider spans `GAIN_MIN_DB..=GAIN_MAX_DB` in 1 dB steps and emits its own
/// change message, so dragging one routes a clamped gain back into shared state. The response curve
/// traces the stored `bands` whether the EQ is on or off, and each band slider's groove is tinted
/// by that band's own gain through [`style::band_slider_style`]. The band row is built by index so
/// each slider's closure captures its own band number — the vertical twin of the volume slider in
/// [`view_transport_controls`](super::transport::view_transport_controls).
pub fn view_equalizer(
    enabled: bool,
    auto: bool,
    preamp: f32,
    bands: &[f32; BAND_COUNT],
    preset: Option<Preset>,
    customs: &[CustomPreset],
) -> Element<'static, Message> {
    let mut bands_row = Row::new().spacing(BAND_ROW_SPACING);
    for (index, frequency) in BAND_FREQUENCIES.iter().enumerate() {
        let gain = bands[index];
        let column = eq_slider_column(
            gain,
            frequency,
            move |gain| Message::EqBandChange(index, gain),
            move |delta| super::eq_band_wheel_message(index, delta),
            move |status| style::band_slider_style(gain, status),
        );
        #[cfg(test)]
        let column = Container::new(column).id(eq_band_slider_id(index));
        bands_row = bands_row.push(column);
    }

    let preamp = preamp_column(preamp);
    #[cfg(test)]
    let preamp = Container::new(preamp).id(EQ_PREAMP_SLIDER_ID);
    let preamp_and_scale = Row::new()
        .spacing(BAND_ROW_SPACING)
        .push(preamp)
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
                    .push(eq_toggle(enabled))
                    .push(spacer(8.0))
                    .push(auto_toggle(auto))
                    .push(spacer(8.0))
                    .push(
                        // The items are a fresh `Vec<PresetChoice>` per
                        // refresh (see [`preset_choices`]): the save row, the
                        // built-ins, and the saved customs. The selection stays
                        // the standing applied `Preset` mapped onto the choice
                        // enum, so the closed label resolves through
                        // `preset_pick_list_label` unchanged — an applied
                        // custom selects nothing there and keeps reading
                        // `Custom`.
                        PickList::new(
                            preset_choices(customs),
                            preset.map(PresetChoice::BuiltIn),
                            Message::EqPresetSelected,
                        )
                        .placeholder(preset_pick_list_label(preset))
                        .text_size(PICK_LIST_TEXT_SIZE)
                        .font(theme::ui_font())
                        .style(|_theme, status| style::chrome_pick_list_style(status))
                        .menu_style(|_theme| style::preset_menu_style()),
                    ),
            )
            .push(sliders),
    )
}

/// The equalizer's text sizes are literals iced exposes no way to read back,
/// so these tests pin them to the 8-px grid the vendored pixel faces render
/// sharply — off-grid sizes blend instead (see the pixel-font split in
/// `PLANS.md`).
#[cfg(test)]
mod size_pins {
    use super::{EQ_TEXT_SIZE, PICK_LIST_TEXT_SIZE};

    #[test]
    fn equalizer_text_sizes_stay_on_the_pixel_grid() {
        assert_eq!(EQ_TEXT_SIZE, 16.0);
        assert_eq!(PICK_LIST_TEXT_SIZE, 16.0);
        assert_eq!(EQ_TEXT_SIZE % 8.0, 0.0);
        assert_eq!(PICK_LIST_TEXT_SIZE % 8.0, 0.0);
    }
}
