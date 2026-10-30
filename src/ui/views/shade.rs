//! The rolled-up shade bar: the single 14px bar the window shrinks to in
//! shade mode — the title bar's own chrome with the compact transport drawn
//! on it, no second strip row.
//!
//! The bar draws the compact transport row's controls — the shared
//! [`super::mini_transport::mini_transport_controls`] button spec, the
//! elapsed-over-total readout, a preview seek slider, and a volume slider —
//! between the app bar's `M` button and its shade, minimize, and close
//! buttons, so every control stays reachable in the base skin's 14px height
//! (275×14 in `winamp-2.05-windowshade.gif`). Every button emits the same
//! [`Message`] its full-size twin does, and the volume slider writes
//! [`Message::VolumeChange`]. The bar replaces the title bar outright while
//! shaded, so the chrome controls keep their exact behavior: the `M` button
//! toggles the About panel, the window buttons keep their glyphs and widths,
//! and the bar between the readout and the volume slider is a
//! [`MouseArea`] drag region that moves the window and double-clicks back to
//! the full window — a roll-up restore, like the base skin. The seek slider
//! emits the Now Playing bar's own
//! [`Message::SeekPreviewChanged`]/[`Message::SeekPreviewReleased`] pair
//! while the backend reports a position and a preview length. The readout
//! and the seek slider's wheel rolls keep their own handlers, so the
//! outer [`MouseArea`]'s volume-wheel roll (see `view::shade`, the screen
//! assembly) reaches only the blank chrome around them.

use std::borrow::Cow;
use std::time::Duration;

use iced::{
    Element, Length,
    alignment::Vertical,
    mouse::Interaction,
    widget::{Button, Container, MouseArea, Row, Space, Text},
};

use crate::state::TransportMode;
use crate::ui::{Message, bevel, style, theme};

use super::mini_transport::{
    MINI_SEEK_ID, MINI_VOLUME_ID, MINI_VOLUME_WIDTH, compact_readout_color,
    mini_transport_controls, mini_transport_readout, mini_volume_slider,
};
use super::{
    seek_bar, spacer,
    title_bar::{
        TITLE_BAR_BUTTON_GLYPH_SIZE, TITLE_BAR_BUTTON_WIDTH, TITLE_BAR_HEIGHT,
        TITLE_BAR_LEFT_BUTTON_WIDTH, title_bar_glyph_button,
    },
};

/// The whole rolled-up window's height: the base skin's single 14px shade
/// bar — the title bar's 12px row framed by the root view's two 1px
/// window-edge rings. The view and `handlers::window::resize_to_shade` both
/// read it, so the window they resize to cannot drift from the tree it
/// shows.
pub(crate) const SHADED_HEIGHT: f32 = TITLE_BAR_HEIGHT + 2.0;

/// The bar row's horizontal padding in px: the same 6px inset the shared
/// compact row and the title bar's band both use.
const SHADE_ROW_PAD: f32 = 6.0;

/// The percentage right-edge inset of the bar's volume slider: the padding
/// either side of the window buttons plus the window buttons themselves.
/// Test-visible so the slider's right-edge pin cannot drift from the buttons
/// it leaves room for.
#[cfg(test)]
pub(crate) const SHADE_VOLUME_RIGHT_INSET: f32 = 2.0 * SHADE_ROW_PAD + 3.0 * TITLE_BAR_BUTTON_WIDTH;

/// The shade readout's size in pixels: a multiple of 8, so the letterform
/// face draws it crisp, and the twin of the app bar's 8px label — the
/// largest on-grid text inside the 12px row.
const SHADE_READOUT_SIZE: f32 = 8.0;

/// The shade bar's `M` button, twin of the title bar's: sunken while the
/// About panel holds, raised otherwise (the panel still opens only after the
/// window restores — see `view::shade`).
fn shade_menu_button(about_visible: bool) -> Button<'static, Message> {
    Button::new(
        Text::new("M")
            .font(theme::ui_font())
            .size(TITLE_BAR_BUTTON_GLYPH_SIZE),
    )
    .on_press(Message::ToggleAbout)
    .width(Length::Fixed(TITLE_BAR_LEFT_BUTTON_WIDTH))
    .style(move |_theme, status| style::toggle_button_style(about_visible, status))
}

/// The readout: the elapsed (or remaining) time over the track's total, at
/// [`SHADE_READOUT_SIZE`], on the pixel-grid letterform face.
///
/// While playing or paused it publishes [`Message::ToggleTimeDisplay`], so
/// the elapsed/remaining toggle carries into shade mode even though the LCD
/// is hidden; while stopped it stays inert but still swallows
/// double-clicks, so it cannot reach the restore.
///
/// [`WinampPlayer::now_playing_elapsed`]: crate::ui::WinampPlayer::now_playing_elapsed
/// [`WinampPlayer::now_playing_time`]: crate::ui::WinampPlayer::now_playing_time
fn shade_readout<'a>(
    mode: TransportMode,
    elapsed_visible: bool,
    elapsed: Cow<'a, str>,
    total: Cow<'a, str>,
) -> Element<'a, Message> {
    let readout = Text::new(mini_transport_readout(elapsed, total))
        .size(SHADE_READOUT_SIZE)
        .font(theme::ui_font())
        .color(compact_readout_color(mode, elapsed_visible));
    if mode != TransportMode::Stopped {
        MouseArea::new(readout)
            .on_press(Message::ToggleTimeDisplay)
            .interaction(Interaction::Pointer)
            .into()
    } else {
        // `MouseArea` only captures a press when it has a handler, and the
        // restore lives in the bar's own drag region: a no-op double-click
        // handler swallows the press here so a double-click on the inert
        // readout cannot restore by accident.
        MouseArea::new(readout)
            .on_double_click(Message::Ignored)
            .into()
    }
}

/// The drag region between the readout and the volume slider: a
/// [`MouseArea`] that moves the window on press and rolls the window back
/// open on double-click, with the seek slider riding inside it while the
/// backend reports a playback position and a non-zero preview length.
///
/// A pressed slider captures the button, so dragging the seek slider never
/// drags the window and a double-click on it previews instead of restoring;
/// the inner [`MouseArea`] keeps the wheel roll on the seek path (see
/// [`seek_bar::seek_wheel_message`]). Without a seek slider the region's
/// blank filler is the chrome the base skin puts its marquee on.
fn shade_drag_region<'a>(
    position: Option<Duration>,
    duration: Option<Duration>,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = match seek_bar::seek_slider(position, duration) {
        Some(slider) => MouseArea::new(
            Container::new(MouseArea::new(slider).on_scroll(seek_bar::seek_wheel_message))
                .id(MINI_SEEK_ID)
                .width(Length::Fill),
        )
        .into(),
        // The restore's single-click area: fill the row's spare width, so
        // the volume slider stays pinned right after the window buttons.
        None => Space::new().width(Length::Fill).height(Length::Fill).into(),
    };
    MouseArea::new(content)
        .on_press(Message::WindowDragged)
        .on_double_click(Message::ToggleWindowShade)
        .into()
}

/// The shade bar's volume slider container: the shared compact spec at
/// [`MINI_VOLUME_WIDTH`], ending at the `SHADE_VOLUME_RIGHT_INSET` pinned
/// by the tests before the window's right edge.
fn shade_volume<'a>(volume: f32) -> Container<'a, Message> {
    Container::new(mini_volume_slider(volume))
        .id(MINI_VOLUME_ID)
        .width(Length::Fixed(MINI_VOLUME_WIDTH))
}

/// The rolled-up window: the base skin's single 14px shade bar.
///
/// A double-click on the bar's blank chrome publishes
/// [`Message::ToggleWindowShade`], restoring the window exactly as the shade
/// button does (and as the title bar's double-click does unshaded). While
/// playing or paused the readout publishes [`Message::ToggleTimeDisplay`],
/// so the elapsed/remaining choice stays switchable while the LCD is hidden;
/// while paused the readout blinks: `elapsed_visible` is the LCD's blink
/// phase and hides it in the off phase (see [`compact_readout_color`]).
/// `volume` is the slider's value, and dragging it emits
/// [`Message::VolumeChange`]. `elapsed` and `total` are the same strings
/// [`WinampPlayer::now_playing_elapsed`] and [`WinampPlayer::now_playing_time`]
/// give the Now Playing bar, so its elapsed/remaining choice and the track's
/// total are shown here too. When `position` and a non-zero `duration` are
/// both known, a seek slider over `duration` sits between the readout and
/// the volume slider; a held drag previews its target through the readout,
/// which `WinampPlayer::readout_position` already folds the drag into.
/// Otherwise that region is blank chrome and nothing else moves.
///
/// [`WinampPlayer::now_playing_elapsed`]: crate::ui::WinampPlayer::now_playing_elapsed
/// [`WinampPlayer::now_playing_time`]: crate::ui::WinampPlayer::now_playing_time
/// [`WinampPlayer::readout_position`]: crate::ui::WinampPlayer::readout_position
// One argument past clippy's limit: the about state rides along with the
// bar's other inputs, the same allowance the pre-fix compact row had for the
// blink phase.
#[allow(clippy::too_many_arguments)]
pub fn view_shade_strip<'a>(
    mode: TransportMode,
    elapsed_visible: bool,
    volume: f32,
    elapsed: Cow<'a, str>,
    total: Cow<'a, str>,
    position: Option<Duration>,
    duration: Option<Duration>,
    about_visible: bool,
) -> Element<'a, Message> {
    let mut row = Row::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(Vertical::Center)
        .padding([0, SHADE_ROW_PAD as u16]);
    // The buttons are `'static`; shortening each to the row's lifetime lets
    // the borrowed time readout share the row (the same coercion the shared
    // compact row makes).
    let menu_button: Element<'static, Message> = shade_menu_button(about_visible).into();
    let menu_button: Element<'a, Message> = menu_button;
    row = row.push(menu_button);
    // The five chrome buttons: the shared compact spec's order, widths, and
    // messages, at the title bar buttons' 8px glyph size, so all five fit
    // the 12px row.
    for (glyph, width, message) in mini_transport_controls() {
        let button: Element<'static, Message> =
            title_bar_glyph_button(glyph, width, theme::shape_font(), message).into();
        let button: Element<'a, Message> = button;
        row = row.push(button).push(spacer(6.0));
    }
    row = row.push(shade_readout(mode, elapsed_visible, elapsed, total));
    row = row.push(shade_drag_region(position, duration));
    row = row.push(shade_volume(volume));
    row = row.push(spacer(6.0));
    for (glyph, message) in [
        // `▭` is uncovered by both embedded faces (see part 1/4's probe in
        // `PLANS.md`); `□` is the closest covered substitute and draws crisp
        // in Tiny5.
        ("□", Message::ToggleWindowShade),
        ("–", Message::MinimizeWindow),
        ("✕", Message::CloseWindow),
    ] {
        let button: Element<'static, Message> =
            title_bar_glyph_button(glyph, TITLE_BAR_BUTTON_WIDTH, theme::shape_font(), message)
                .into();
        let button: Element<'a, Message> = button;
        row = row.push(button);
    }

    bevel::raised_panel(
        Container::new(row)
            .width(Length::Fill)
            .height(Length::Fixed(TITLE_BAR_HEIGHT))
            .style(|_theme| style::title_bar_style()),
    )
}

#[cfg(test)]
mod size_pins {
    // The shade bar sits on the title bar's chrome, so its text must stay on
    // the same 8px pixel grid the title bar's label pins (see
    // `title_bar::tests`) — only multiples of 8 keep the embedded faces crisp.
    use super::SHADE_READOUT_SIZE;

    #[test]
    fn shade_bar_text_sizes_stay_on_the_pixel_grid() {
        assert_eq!(SHADE_READOUT_SIZE, 8.0);
        assert_eq!(SHADE_READOUT_SIZE % 8.0, 0.0);
    }
}
