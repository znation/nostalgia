//! The transport row's widgets: the Play/Pause, Stop, Previous, Next, Repeat,
//! and Shuffle buttons, the volume slider, the balance slider, and the EQ
//! panel toggle, plus the pure glyph, word, and button-label helpers they use.

use iced::{
    Element, Length,
    widget::{Button, Row, Text, Tooltip, button, tooltip},
};

use crate::ui::{Message, style, theme};

use super::{
    BALANCE_MAX, BALANCE_MIN, BALANCE_STEP, VOLUME_MAX, VOLUME_MIN, VOLUME_STEP, chrome_slider,
    fixed_width_button, spacer,
};

/// The Play/Pause button's glyph: `"II"` while playing, `"▶"` otherwise.
/// Pure so the glyph logic is testable without an iced `Element`, and shared
/// with the shade strip so the two transport rows cannot drift.
pub(super) fn play_pause_glyph(is_playing: bool) -> &'static str {
    if is_playing { "II" } else { "▶" }
}

/// The Play/Pause button's word: `"Pause"` while playing, `"Play"`
/// otherwise. It labels the glyph's hover tooltip.
pub(super) fn play_pause_word(is_playing: bool) -> &'static str {
    if is_playing { "Pause" } else { "Play" }
}

/// The Repeat button's label: "Repeat: On" while Repeat is on, "Repeat:
/// Off" when it is off. Pure so the label logic is testable without an
/// iced `Element`, like [`play_pause_glyph`].
pub(super) fn repeat_label(repeat: bool) -> &'static str {
    if repeat { "Repeat: On" } else { "Repeat: Off" }
}

/// The Shuffle button's label: "Shuffle: On" while Shuffle is on, "Shuffle:
/// Off" when it is off. Pure so the label logic is testable without an iced
/// `Element`, like [`repeat_label`].
pub(super) fn shuffle_label(shuffle: bool) -> &'static str {
    if shuffle {
        "Shuffle: On"
    } else {
        "Shuffle: Off"
    }
}

/// The four transport glyph buttons' shared spec, in row order: Previous,
/// Play/Pause, Stop, and Next. Each tuple is `(glyph, tooltip word, fixed
/// width, message)`.
///
/// The main row and the shade strip both build from this one list, so their
/// glyphs, order, widths, and messages cannot drift apart. `is_playing`
/// chooses the Play/Pause glyph and word.
pub(super) fn transport_glyphs(
    is_playing: bool,
) -> [(&'static str, &'static str, f32, Message); 4] {
    [
        ("◀◀", "Previous", 34.0, Message::PreviousTrack),
        (
            play_pause_glyph(is_playing),
            play_pause_word(is_playing),
            30.0,
            Message::PlayPause,
        ),
        ("■", "Stop", 30.0, Message::Stop),
        ("▶▶", "Next", 34.0, Message::NextTrack),
    ]
}

/// The transport row's six chrome buttons, in row order: Previous,
/// Play/Pause, Stop, Next, Repeat, and Shuffle. `is_playing`, `repeat`, and
/// `shuffle` resolve the labels that change with state; the other labels and
/// tooltips are static literals.
///
/// The four glyph buttons come from [`transport_glyphs`], each wrapped in a
/// hover tooltip carrying its word. Every button is pinned to a fixed width,
/// so the Play/Pause glyph swap or either toggle's label swap cannot resize
/// it and reflow every widget after it in the row. Returned as `Element`s so
/// the width each button requests is observable in a test.
pub(super) fn transport_buttons(
    is_playing: bool,
    repeat: bool,
    shuffle: bool,
) -> [Element<'static, Message>; 6] {
    // The widths, in pixels, of the two text toggles' widest labels: Repeat
    // must fit "Repeat: Off" and Shuffle must fit "Shuffle: Off". They are
    // the face widths measured from the running window (see the qa
    // reproduction), so at rest each button keeps the width its label
    // already had. The glyph widths live in `transport_glyphs`.
    const REPEAT_WIDTH: f32 = 104.0;
    const SHUFFLE_WIDTH: f32 = 112.0;

    let [previous, play_pause, stop, next] =
        transport_glyphs(is_playing).map(|(glyph, word, width, message)| {
            let button = fixed_width_button(glyph, width, message);
            Element::from(Tooltip::new(
                button,
                Text::new(word).size(12),
                tooltip::Position::Top,
            ))
        });

    [
        previous,
        play_pause,
        stop,
        next,
        fixed_width_button(repeat_label(repeat), REPEAT_WIDTH, Message::ToggleRepeat).into(),
        fixed_width_button(
            shuffle_label(shuffle),
            SHUFFLE_WIDTH,
            Message::ToggleShuffle,
        )
        .into(),
    ]
}

/// The equalizer-panel toggle's fixed width, in pixels: enough for its `EQ`
/// label, pinned so the panel's visibility cannot reflow the row.
pub(super) const EQ_PANEL_BUTTON_WIDTH: f32 = 30.0;

/// The player row's EQ toggle spec: `(label, fixed width, message)`. Pressing
/// it shows or hides the equalizer panel without changing the equalizer's
/// on/off state, curve, or preset.
pub(super) fn eq_panel_toggle() -> (&'static str, f32, Message) {
    ("EQ", EQ_PANEL_BUTTON_WIDTH, Message::ToggleEqualizerPanel)
}

/// The EQ toggle's chrome: sunken while the equalizer panel is open, so the
/// button reads as on, and the ordinary chrome for `status` while it is
/// hidden. That is the title bar's A toggle rule.
pub(super) fn eq_panel_button_style(open: bool, status: button::Status) -> button::Style {
    if open {
        style::chrome_button_style(button::Status::Pressed)
    } else {
        style::chrome_button_style(status)
    }
}

/// The transport row: the Previous, Play/Pause, Stop, Next, Repeat, and
/// Shuffle buttons, the volume slider, the balance slider, and the fixed-width
/// `EQ` panel toggle. `volume` and `balance` are the sliders' current values;
/// dragging them emits `Message::VolumeChange` and `Message::BalanceChange`
/// respectively. `repeat` is the shared Repeat flag and `shuffle` the shared
/// Shuffle flag, each shown on its button and toggled by pressing it. The four
/// glyph buttons carry word tooltips; Repeat and Shuffle stay text toggles.
/// `eq_panel_visible` sinks the EQ button while the equalizer section is drawn
/// and leaves its chrome raised while the section is hidden.
pub fn view_transport_controls(
    is_playing: bool,
    volume: f32,
    balance: f32,
    repeat: bool,
    shuffle: bool,
    eq_panel_visible: bool,
) -> Element<'static, Message> {
    let mut row = Row::new();
    for button in transport_buttons(is_playing, repeat, shuffle) {
        row = row.push(button).push(spacer(20.0));
    }
    row.push(chrome_slider(
        VOLUME_MIN,
        VOLUME_MAX,
        volume,
        VOLUME_STEP,
        100.0,
        theme::VOLUME_SLIDER_FILL,
        Message::VolumeChange,
    ))
    .push(chrome_slider(
        BALANCE_MIN,
        BALANCE_MAX,
        balance,
        BALANCE_STEP,
        100.0,
        theme::BALANCE_SLIDER_FILL,
        Message::BalanceChange,
    ))
    .push(spacer(20.0))
    .push({
        let (label, width, message) = eq_panel_toggle();
        Button::new(Text::new(label))
            .on_press(message)
            .width(Length::Fixed(width))
            .style(move |_theme, status| eq_panel_button_style(eq_panel_visible, status))
    })
    .into()
}
