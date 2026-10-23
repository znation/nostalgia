//! The equalizer panel: the on/off button, the preset pick list, the preamp
//! slider, and the [`BAND_COUNT`] vertical band sliders.

use iced::{
    Element, Length,
    widget::{Column, PickList, Row, Text, VerticalSlider},
};

use crate::equalizer::{BAND_COUNT, BAND_FREQUENCIES, GAIN_MAX_DB, GAIN_MIN_DB, PRESETS, Preset};
use crate::ui::{Message, bevel, style};

use super::{EQ_STEP, chrome_slider, labeled_button, spacer};

/// The horizontal gap between adjacent equalizer band columns. Without it the
/// labels sit edge to edge, so `170`, `310`, `600`, and `1K` read as one
/// number in the rendered panel.
const BAND_ROW_SPACING: f32 = 6.0;

/// The equalizer on/off button's label: "EQ: On" while the equalizer is
/// engaged, "EQ: Off" when it is off. Pure so the label logic is testable
/// without an iced `Element`, like
/// [`repeat_label`](super::transport::repeat_label).
pub(super) fn eq_enabled_label(enabled: bool) -> &'static str {
    if enabled { "EQ: On" } else { "EQ: Off" }
}

/// The equalizer panel: an on/off button, a preset pick list, a preamp slider,
/// and a row of the [`BAND_COUNT`] vertical band sliders labelled from
/// [`BAND_FREQUENCIES`].
///
/// `enabled` is the shared EQ on/off flag shown on the button and toggled by
/// pressing it; `preset` is the applied curve the pick list shows (or `None`,
/// drawn as the "(none)" custom-curve placeholder), and picking one emits
/// [`Message::EqPresetSelected`]; `preamp` and `bands` are the stored gains the
/// sliders start from. Every slider spans `GAIN_MIN_DB..=GAIN_MAX_DB` in 1 dB steps and
/// emits its own change message, so dragging one routes a clamped gain back
/// into shared state. The band row is built by index so each slider's closure
/// captures its own band number — the vertical twin of the volume slider in
/// [`view_transport_controls`](super::transport::view_transport_controls).
pub fn view_equalizer(
    enabled: bool,
    preamp: f32,
    bands: &[f32; BAND_COUNT],
    preset: Option<Preset>,
) -> Element<'static, Message> {
    let mut band_row = Row::new().spacing(BAND_ROW_SPACING);
    for (index, frequency) in BAND_FREQUENCIES.iter().enumerate() {
        band_row = band_row.push(
            Column::new()
                .push(
                    VerticalSlider::new(GAIN_MIN_DB..=GAIN_MAX_DB, bands[index], move |gain| {
                        Message::EqBandChange(index, gain)
                    })
                    .step(EQ_STEP)
                    .height(Length::Fixed(100.0))
                    .style(|_theme, status| style::chrome_slider_style(status)),
                )
                .push(Text::new(*frequency).size(12)),
        );
    }

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
            .push(Row::new().push(Text::new("Preamp")).push(chrome_slider(
                GAIN_MIN_DB,
                GAIN_MAX_DB,
                preamp,
                EQ_STEP,
                150.0,
                Message::EqPreampChange,
            )))
            .push(band_row),
    )
}
