//! The transport row's widgets: the Play/Pause, Stop, Previous, Next, Repeat,
//! and Shuffle buttons, the volume slider, and the balance slider, plus the
//! pure button-label helpers they use.

use iced::{Element, widget::Row};

use crate::ui::{Message, theme};

use super::{
    BALANCE_MAX, BALANCE_MIN, BALANCE_STEP, VOLUME_MAX, VOLUME_MIN, VOLUME_STEP, chrome_slider,
    fixed_width_button, spacer,
};

/// The Play/Pause button's label: "Pause" while playing, "Play" when
/// stopped. Pure so the label logic is testable without an iced `Element`.
pub(super) fn play_pause_label(is_playing: bool) -> &'static str {
    if is_playing { "Pause" } else { "Play" }
}

/// The Repeat button's label: "Repeat: On" while Repeat is on, "Repeat:
/// Off" when it is off. Pure so the label logic is testable without an
/// iced `Element`, like [`play_pause_label`].
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

/// The transport row's six chrome buttons, in row order: Play/Pause, Stop,
/// Previous, Next, Repeat, and Shuffle. `is_playing`, `repeat`, and `shuffle`
/// resolve the three labels that change with state; the other three are static
/// literals.
///
/// Each button is pinned to the fixed face width of its widest label, so a
/// label swap — Play for Pause, Repeat: Off for Repeat: On, or Shuffle: Off
/// for Shuffle: On — cannot resize the button and reflow every widget after it
/// in the row. Returned as `Element`s so the width each button requests is
/// observable in a test.
pub(super) fn transport_buttons(
    is_playing: bool,
    repeat: bool,
    shuffle: bool,
) -> [Element<'static, Message>; 6] {
    // The widths, in pixels, of each button's widest label: Play/Pause must
    // fit "Pause", Repeat must fit "Repeat: Off", and Shuffle must fit
    // "Shuffle: Off". They are the face widths measured from the running
    // window (see the qa reproduction), so at rest each button keeps the
    // width its label already had.
    const PLAY_PAUSE_WIDTH: f32 = 65.0;
    const STOP_WIDTH: f32 = 54.0;
    const PREVIOUS_WIDTH: f32 = 84.0;
    const NEXT_WIDTH: f32 = 54.0;
    const REPEAT_WIDTH: f32 = 104.0;
    const SHUFFLE_WIDTH: f32 = 112.0;

    [
        fixed_width_button(
            play_pause_label(is_playing),
            PLAY_PAUSE_WIDTH,
            Message::PlayPause,
        )
        .into(),
        fixed_width_button("Stop", STOP_WIDTH, Message::Stop).into(),
        fixed_width_button("Previous", PREVIOUS_WIDTH, Message::PreviousTrack).into(),
        fixed_width_button("Next", NEXT_WIDTH, Message::NextTrack).into(),
        fixed_width_button(repeat_label(repeat), REPEAT_WIDTH, Message::ToggleRepeat).into(),
        fixed_width_button(
            shuffle_label(shuffle),
            SHUFFLE_WIDTH,
            Message::ToggleShuffle,
        )
        .into(),
    ]
}

/// The transport row: the Play/Pause, Stop, Previous, Next, Repeat, and
/// Shuffle buttons, the volume slider, and the balance slider. `volume` and
/// `balance` are the sliders' current values; dragging them emits
/// `Message::VolumeChange` and `Message::BalanceChange` respectively.
/// `repeat` is the shared Repeat flag and `shuffle` the shared Shuffle flag,
/// each shown on its button and toggled by pressing it. The Stop label is
/// static — Stop is always pressable, even when already stopped, as in Winamp
/// — so no `play_pause_label`-style helper is needed.
pub fn view_transport_controls(
    is_playing: bool,
    volume: f32,
    balance: f32,
    repeat: bool,
    shuffle: bool,
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
    .into()
}
