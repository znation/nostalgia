//! The shared compact transport row: Previous, Play, Pause, Stop, Next, the elapsed-over-total time
//! readout, and a volume slider.
//!
//! The window-shade strip and the library footer both build this one row, so their glyphs, order,
//! fixed widths, messages, and slider spec cannot drift apart. It is presentation only: every
//! control emits the same [`Message`] as the full-size transport row, and the slider writes
//! [`Message::VolumeChange`]. While playing or paused the time readout publishes
//! [`Message::ToggleTimeDisplay`] under the LCD's own
//! [`super::now_playing::elapsed_readout_clickable`] guard; while stopped it stays inert. While
//! paused the readout blinks in step with the LCD's large readout (see [`compact_readout_color`]).

use std::borrow::Cow;
use std::time::Duration;

use iced::{
    Color, Element, Length,
    alignment::Vertical,
    mouse::Interaction,
    widget::{Container, MouseArea, Row, Space, Text},
};

use crate::state::TransportMode;
use crate::ui::{Message, bevel, style, theme};

use super::{
    VOLUME_MAX, VOLUME_MIN, VOLUME_STEP, chrome_slider, fixed_width_button, spacer,
    transport::transport_glyphs,
};

/// The compact row's fixed height in px. The shade strip and the library footer both occupy exactly
/// this height.
pub(crate) const MINI_TRANSPORT_HEIGHT: f32 = 28.0;

/// The compact row's volume slider width in px: narrow enough to leave the time readout and five
/// buttons room.
pub(super) const MINI_VOLUME_WIDTH: f32 = 80.0;

/// The id of the compact row's volume-slider container. Only one strip is in the tree at a time
/// (the library footer in normal mode, the shade strip in shade mode), so the shared id never
/// collides. An explicit [`Id`] lets a test find the slider's laid-out bounds and pin its
/// right-edge placement.
///
/// [`Id`]: iced::advanced::widget::Id
pub(crate) const MINI_VOLUME_ID: &str = "mini-transport-volume";

/// The id of the compact row's seek-slider container. The shade strip is the only strip that draws
/// one (the library footer builds none), so the id never collides; a test finds the slider's
/// laid-out bounds through it.
///
/// [`Id`]: iced::advanced::widget::Id
pub(crate) const MINI_SEEK_ID: &str = "mini-transport-seek";

/// The compact row's five chrome buttons, in row order: Previous, Play, Pause, Stop, and Next. It
/// drops the tooltip words from [`transport_glyphs`] so both compact strips and the full transport
/// row share one glyph, order, width, and message spec.
///
/// Returned as plain `(label, width, message)` tuples rather than widgets, so the labels, messages,
/// and widths are testable without an iced renderer.
pub(super) fn mini_transport_controls() -> [(&'static str, f32, Message); 5] {
    transport_glyphs().map(|(glyph, _word, width, message)| (glyph, width, message))
}

/// The compact row's volume slider parameters, in `chrome_slider` order: the shared volume range
/// and step, [`MINI_VOLUME_WIDTH`], and [`Message::VolumeChange`]. Kept pure so a test can pin the
/// range and callback the row builds.
pub(super) fn mini_volume_spec() -> (f32, f32, f32, f32, fn(f32) -> Message) {
    (
        VOLUME_MIN,
        VOLUME_MAX,
        VOLUME_STEP,
        MINI_VOLUME_WIDTH,
        Message::VolumeChange,
    )
}

/// The compact row's readout: the elapsed (or remaining) time over the track's total, such as
/// `00:53/3:30`.
///
/// `elapsed` is passed through unchanged when `total` is the `--:--` placeholder, so the empty and
/// unknown-track states keep their single placeholder instead of drawing `--:--/--:--`.
pub(super) fn mini_transport_readout<'a>(
    elapsed: Cow<'a, str>,
    total: Cow<'a, str>,
) -> Cow<'a, str> {
    if total == "--:--" {
        elapsed
    } else {
        Cow::Owned(format!("{elapsed}/{total}"))
    }
}

/// The compact readout's color: fully transparent in the hidden phase of the paused blink, else
/// [`theme::LCD_GREEN`].
///
/// The hidden phase keeps the text in the layout and only hides it, so the strip's controls do not
/// move. `visible` is [`WinampPlayer::blink_visible`], the same phase the LCD's large readout uses.
///
/// [`WinampPlayer::blink_visible`]: crate::ui::WinampPlayer::blink_visible
pub(super) fn compact_readout_color(mode: TransportMode, visible: bool) -> Color {
    if mode == TransportMode::Paused && !visible {
        Color::from_rgba(0.0, 0.0, 0.0, 0.0)
    } else {
        theme::LCD_GREEN
    }
}

/// One full-width compact row: the five fixed-width buttons separated by a 6px gap, the
/// elapsed-over-total time readout, a fill spacer, and the volume slider pinned to the row's right
/// edge.
///
/// `mode` decides whether the readout is a click target, and while paused `elapsed_visible` is the
/// blink phase that hides it (see [`compact_readout_color`]); while playing or paused it publishes
/// [`Message::ToggleTimeDisplay`], and a stopped readout stays inert but still captures the press,
/// so an ancestor double-click target (the shade strip's restore) never sees one. `volume` is the
/// slider's value; `elapsed` and `total` are the same strings [`WinampPlayer::now_playing_elapsed`]
/// and [`WinampPlayer::now_playing_time`] give the Now Playing bar, so the elapsed/remaining choice
/// and the track's total are shown here too. When the total is unknown the readout stays the plain
/// `elapsed` string (see [`mini_transport_readout`]).
///
/// [`WinampPlayer::now_playing_elapsed`]: crate::ui::WinampPlayer::now_playing_elapsed
/// [`WinampPlayer::now_playing_time`]: crate::ui::WinampPlayer::now_playing_time
pub(super) fn mini_transport_row<'a>(
    mode: TransportMode,
    elapsed_visible: bool,
    volume: f32,
    elapsed: Cow<'a, str>,
    total: Cow<'a, str>,
) -> Row<'a, Message> {
    mini_transport_row_with_optional_seek(mode, elapsed_visible, volume, elapsed, total, None)
}

/// The shade strip's compact row: the same buttons, time readout, and volume slider as
/// [`mini_transport_row`], with a chrome seek slider between the readout and the volume slider
/// while the backend reports both a playback position and a non-zero preview length.
///
/// The seek slider comes from [`super::seek_bar::seek_slider`], so it shares the Now Playing seek
/// bar's scale, step, and drag/release messages; dragging it previews the target in the readout the
/// caller already derives from [`WinampPlayer::readout_position`]. With no position or no usable
/// length it draws nothing and the row keeps the fill space that pins the volume slider to the
/// right edge. The library footer builds [`mini_transport_row`], so it never gains a seek slider.
///
/// [`WinampPlayer::readout_position`]: crate::ui::WinampPlayer::readout_position
pub(super) fn mini_transport_row_with_seek<'a>(
    mode: TransportMode,
    elapsed_visible: bool,
    volume: f32,
    elapsed: Cow<'a, str>,
    total: Cow<'a, str>,
    position: Option<Duration>,
    duration: Option<Duration>,
) -> Row<'a, Message> {
    let seek: Option<Element<'a, Message>> =
        super::seek_bar::seek_slider(position, duration).map(|slider| {
            Container::new(MouseArea::new(slider).on_scroll(super::seek_bar::seek_wheel_message))
                .id(MINI_SEEK_ID)
                .width(Length::Fill)
                .into()
        });
    mini_transport_row_with_optional_seek(mode, elapsed_visible, volume, elapsed, total, seek)
}

/// Builds the compact row's shared leading controls — the five buttons and the elapsed-over-total
/// readout — then the optional seek slider or the fill spacer, and finally the right-edge volume
/// slider.
fn mini_transport_row_with_optional_seek<'a>(
    mode: TransportMode,
    elapsed_visible: bool,
    volume: f32,
    elapsed: Cow<'a, str>,
    total: Cow<'a, str>,
    seek: Option<Element<'a, Message>>,
) -> Row<'a, Message> {
    let mut row: Row<'a, Message> = Row::new().width(Length::Fill).align_y(Vertical::Center);
    for (label, width, message) in mini_transport_controls() {
        // The button is `'static`; shorten it to the row's lifetime so the
        // borrowed time readout can share the row.
        let button: Element<'static, Message> = fixed_width_button(label, width, message).into();
        let button: Element<'a, Message> = button;
        row = row.push(button).push(spacer(6.0));
    }
    let readout = Text::new(mini_transport_readout(elapsed, total))
        .size(14)
        .color(compact_readout_color(mode, elapsed_visible));
    let readout: Element<'a, Message> = if super::now_playing::elapsed_readout_clickable(mode) {
        MouseArea::new(readout)
            .on_press(Message::ToggleTimeDisplay)
            .interaction(Interaction::Pointer)
            .into()
    } else {
        // The stopped readout publishes nothing, but `MouseArea` only
        // captures a press when it has a handler, and the shade strip wraps
        // this row in its own double-click restore. A no-op double-click
        // handler swallows the press here so a double-click on the readout
        // cannot reach that restore.
        MouseArea::new(readout)
            .on_double_click(Message::Ignored)
            .into()
    };
    row = row.push(readout);
    match seek {
        Some(seek) => row = row.push(seek).push(spacer(6.0)),
        None => row = row.push(Space::new().width(Length::Fill)),
    }
    row.push(
        Container::new(mini_volume_slider(volume))
            .id(MINI_VOLUME_ID)
            .width(Length::Fixed(MINI_VOLUME_WIDTH)),
    )
}

/// The compact row's volume slider, built from [`mini_volume_spec`] and tinting its groove with
/// [`style::volume_level_color`].
fn mini_volume_slider(volume: f32) -> Element<'static, Message> {
    let (min, max, step, width, on_change) = mini_volume_spec();
    chrome_slider(
        min,
        max,
        volume,
        step,
        width,
        style::volume_level_color(volume),
        on_change,
    )
}

/// The compact row in its shared strip geometry: full width, `height` tall, vertically centered,
/// and padded 6px horizontally so the volume slider lands at the same right edge in both places it
/// appears. The caller supplies the height (the library footer's [`MINI_TRANSPORT_HEIGHT`] or the
/// shade strip's own) and wraps the result in its chrome, so the two strips cannot drift apart.
pub(super) fn mini_transport_strip<'a>(
    mode: TransportMode,
    elapsed_visible: bool,
    volume: f32,
    elapsed: Cow<'a, str>,
    total: Cow<'a, str>,
    height: f32,
) -> Container<'a, Message> {
    mini_transport_strip_container(
        mini_transport_row(mode, elapsed_visible, volume, elapsed, total),
        height,
    )
}

/// The shade strip's compact row in the same shared strip geometry as [`mini_transport_strip`],
/// with the seek slider the rolled-up window needs. The library footer calls
/// [`mini_transport_strip`] instead, so it stays seekless, while both views keep their geometry
/// from drifting apart.
// One argument past clippy's limit: the blink phase rides along with the row's other inputs.
#[allow(clippy::too_many_arguments)]
pub(super) fn mini_transport_strip_with_seek<'a>(
    mode: TransportMode,
    elapsed_visible: bool,
    volume: f32,
    elapsed: Cow<'a, str>,
    total: Cow<'a, str>,
    position: Option<Duration>,
    duration: Option<Duration>,
    height: f32,
) -> Container<'a, Message> {
    mini_transport_strip_container(
        mini_transport_row_with_seek(
            mode,
            elapsed_visible,
            volume,
            elapsed,
            total,
            position,
            duration,
        ),
        height,
    )
}

/// Wraps a compact row in the fill-width, fixed-height, centered, 6px-padded container both strips
/// use, so their geometry cannot drift apart.
fn mini_transport_strip_container<'a>(
    row: Row<'a, Message>,
    height: f32,
) -> Container<'a, Message> {
    Container::new(row)
        .width(Length::Fill)
        .height(Length::Fixed(height))
        .align_y(Vertical::Center)
        .padding([0, 6])
}

/// The normal-mode library footer: the shared compact row beneath the browse list, on the window
/// face's raised chrome.
///
/// `elapsed` and `total` are the same time strings the LCD shows, so the footer's readout is the
/// elapsed (or remaining) time over the track's total; `mode` makes that readout toggle the mode
/// while playing or paused and stay inert while stopped. It keeps the row's fixed
/// [`MINI_TRANSPORT_HEIGHT`] and its 6px horizontal padding, so the slider lands at the same right
/// edge as the shade strip's. While paused the readout blinks with `elapsed_visible`, the LCD's
/// blink phase.
pub fn view_library_strip<'a>(
    mode: TransportMode,
    elapsed_visible: bool,
    volume: f32,
    elapsed: Cow<'a, str>,
    total: Cow<'a, str>,
) -> Element<'a, Message> {
    bevel::raised_panel(mini_transport_strip(
        mode,
        elapsed_visible,
        volume,
        elapsed,
        total,
        MINI_TRANSPORT_HEIGHT,
    ))
}
