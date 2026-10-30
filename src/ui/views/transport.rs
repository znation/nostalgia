//! The transport row's widgets: the Previous, Play, Pause, Stop, Next, Repeat, and Shuffle buttons,
//! the volume slider, the balance slider, the EQ and PL panel toggles, and the options-menu button,
//! plus the pure glyph, word, and button-label helpers they use.

use iced::{
    Element, Length,
    widget::{Button, Container, MouseArea, Row, Text, Tooltip, container, tooltip},
};

use crate::ui::{Message, style, theme};

use super::{
    BALANCE_MAX, BALANCE_MIN, BALANCE_STEP, VOLUME_MAX, VOLUME_MIN, VOLUME_STEP, chrome_slider,
    fixed_width_button, fixed_width_toggle_button, spacer,
};

/// The five transport glyph buttons' shared spec, in row order: Previous, Play, Pause, Stop, and
/// Next. Each tuple is `(glyph, tooltip word, fixed width, message)`.
///
/// The main row and both compact strips (the shade strip and the library footer) build from this
/// one list, so their glyphs, order, widths, and messages cannot drift apart. The spec is
/// playback-independent: Play always resumes and Pause always pauses, like the X and C keys.
/// Previous and Next send the button messages, which jump to the list's edge while Control is held;
/// Stop sends its button message, which fades out while Shift is held.
pub(super) fn transport_glyphs() -> [(&'static str, &'static str, f32, Message); 5] {
    [
        ("◀◀", "Previous", 34.0, Message::PreviousButton),
        ("▶", "Play", 30.0, Message::Play),
        ("II", "Pause", 30.0, Message::Pause),
        ("■", "Stop", 30.0, Message::StopButton),
        ("▶▶", "Next", 34.0, Message::NextButton),
    ]
}

/// The glyph buttons' hover-tooltip word size in pixels: a multiple of 8, so
/// the letterform face draws each word crisp (the split's on-grid mapping,
/// 12 → 16). Consequently the toggle labels out on the face keep iced's own
/// 16.
const TOOLTIP_WORD_SIZE: f32 = 16.0;

/// The transport row's seven chrome buttons, in row order: Previous, Play, Pause, Stop, Next,
/// Repeat, and Shuffle. `repeat` and `shuffle` light their button's LED instead of changing its
/// label, so the other labels and tooltips are static literals.
///
/// The five glyph buttons come from [`transport_glyphs`], each wrapped in a hover tooltip carrying
/// its word. Every button is pinned to a fixed width, so neither toggle's state change (an LED
/// color, not a word) can resize it and reflow every widget after it in the row. Returned as
/// `Element`s so the width each button requests is observable in a test.
pub(super) fn transport_buttons(repeat: bool, shuffle: bool) -> [Element<'static, Message>; 7] {
    // The widths, in pixels, of the two text toggles' widest labels: Repeat
    // must fit "Repeat: Off" and Shuffle must fit "Shuffle: Off". They are
    // the face widths measured from the running window (see the qa
    // reproduction), so at rest each button keeps the width its label
    // already had. The glyph widths live in `transport_glyphs`.
    let [previous, play, pause, stop, next] =
        transport_glyphs().map(|(glyph, word, width, message)| {
            let button = fixed_width_button(glyph, width, theme::shape_font(), message);
            Element::from(Tooltip::new(
                button,
                // Tooltip words draw in the app face at the split's on-grid
                // word size: 12 left the system sans and is off the 8px grid.
                Text::new(word)
                    .size(TOOLTIP_WORD_SIZE)
                    .font(theme::ui_font()),
                tooltip::Position::Top,
            ))
        });

    [
        previous,
        play,
        pause,
        stop,
        next,
        repeat_toggle(repeat).into(),
        shuffle_toggle(shuffle).into(),
    ]
}

/// The Repeat button's fixed width, in pixels: the loop glyph with the LED
/// square and its gap inside the button's default side padding (8+6+10+20),
/// pinned so neither the glyph nor the LED's color change can reflow the row.
const REPEAT_WIDTH: f32 = 44.0;

/// The Shuffle button's fixed width, in pixels, pinned like [`REPEAT_WIDTH`]: enough for its LED
/// square and engraved label.
const SHUFFLE_WIDTH: f32 = 92.0;

/// The Repeat button's face glyph after its LED square: a repeat loop icon
/// like the base skin's repeat glyph (`docs/screenshots/player.png`'s repeat
/// button paints a glyph like it beside its LED square, with no word on the
/// face). `DejaVu Sans` draws `↻` with a real advance — its higher-codepoint
/// cousin `⟳` measured no advance and drew nothing.
const REPEAT_GLYPH: &str = "↻";

/// The Shuffle button's face label after its LED square: the base skin's
/// engraved word (`docs/screenshots/player.png`'s shuffle button reads
/// `SHUFFLE` beside its LED square).
const SHUFFLE_LABEL: &str = "SHUFFLE";

/// The LED square's edge, in pixels: the mockups' small on/off marker square,
/// filling [`theme::LED_ON`] while its flag is on and [`theme::LED_OFF`] while
/// off (`docs/screenshots/player.png` reads (16,20,15) on the resting
/// Shuffle button's LED and `docs/screenshots/transport.png` reads (0,232,0)
/// on the repeat LED in its Repeat-on panel).
const LED_SIZE: f32 = 8.0;

/// The gap between a toggle's LED square and the label beside it.
const LED_GAP: f32 = 6.0;

/// The Repeat button: raised chrome whose face carries a LED square lit while
/// `repeat` is on and the loop glyph beside it, publishing
/// [`Message::ToggleRepeat`]. The base skin marks the state on the LED and
/// keeps the chrome raised at rest in both states; pressing sinks it only
/// while the mouse holds the button down.
pub(super) fn repeat_toggle(repeat: bool) -> Button<'static, Message> {
    led_toggle(REPEAT_GLYPH, REPEAT_WIDTH, Message::ToggleRepeat, repeat)
}

/// The Shuffle button: raised chrome whose face carries a LED square lit
/// while `shuffle` is on and the engraved `SHUFFLE` label beside it, like
/// [`repeat_toggle`] but for [`Message::ToggleShuffle`].
pub(super) fn shuffle_toggle(shuffle: bool) -> Button<'static, Message> {
    led_toggle(
        SHUFFLE_LABEL,
        SHUFFLE_WIDTH,
        Message::ToggleShuffle,
        shuffle,
    )
}

/// One of the transport row's LED-marked toggles: a fixed-width chrome button
/// whose face carries a small square LED lit while `on` and the `label`
/// beside it. Repeat and Shuffle both build through this one builder, so
/// their marker squares and chrome cannot drift apart.
fn led_toggle(
    label: &'static str,
    width: f32,
    message: Message,
    on: bool,
) -> Button<'static, Message> {
    let face: Element<'static, Message> = Row::new()
        .align_y(iced::Alignment::Center)
        .push(led_square(on))
        .push(iced::widget::Space::new().width(Length::Fixed(LED_GAP)))
        .push(Text::new(label).font(theme::ui_font()))
        .into();
    Button::new(face)
        .on_press(message)
        .width(Length::Fixed(width))
        .style(|_theme, status| style::chrome_button_style(status))
}

/// The on/off indicator square on a transport toggle's face: a solid fill
/// block, [`theme::LED_ON`] while `on` and [`theme::LED_OFF`] off (see the
/// constants' sampled mockup readings). A filled container rather than a text
/// glyph, so the square renders as exact pixels like the buttons' bevel fills.
fn led_square(on: bool) -> Element<'static, Message> {
    Container::new(iced::widget::Space::new().width(Length::Fixed(LED_SIZE)))
        .width(Length::Fixed(LED_SIZE))
        .height(Length::Fixed(LED_SIZE))
        .style(move |_theme| container::Style {
            background: Some(iced::Background::Color(if on {
                theme::LED_ON
            } else {
                theme::LED_OFF
            })),
            ..container::Style::default()
        })
        .into()
}

/// The equalizer-panel toggle's fixed width, in pixels: enough for its `EQ` label, pinned so the
/// panel's visibility cannot reflow the row.
pub(super) const EQ_PANEL_BUTTON_WIDTH: f32 = 30.0;

/// The player row's EQ toggle spec: `(label, fixed width, message)`. Pressing it shows or hides the
/// equalizer panel without changing the equalizer's on/off state, curve, or preset.
pub(super) fn eq_panel_toggle() -> (&'static str, f32, Message) {
    ("EQ", EQ_PANEL_BUTTON_WIDTH, Message::ToggleEqualizerPanel)
}

/// The library-section toggle's fixed width, in pixels: enough for its `PL` label, pinned so the
/// section's visibility cannot reflow the row.
pub(super) const LIBRARY_PANEL_BUTTON_WIDTH: f32 = 30.0;

/// The player row's PL toggle spec: `(label, fixed width, message)`. Pressing it shows or hides the
/// normal-mode library section without changing any playback, equalizer, browse, search, or
/// selection state.
pub(super) fn library_panel_toggle() -> (&'static str, f32, Message) {
    (
        "PL",
        LIBRARY_PANEL_BUTTON_WIDTH,
        Message::ToggleLibraryPanel,
    )
}

/// The options-menu button's fixed width, in pixels, pinned like the other panel toggles so a glyph
/// change cannot reflow the row.
pub(super) const OPTIONS_BUTTON_WIDTH: f32 = 30.0;

/// The options button (`≡`, the shape face's substitute for the lightning
/// `⚡` the split's part 2/4 swapped out when it re-based the chrome's
/// glyphs) at the end of the player row: opens the same transient options menu as the
/// LCD strip's `O` through [`Message::ToggleOptions`].
pub(super) fn options_button() -> Element<'static, Message> {
    // `⚡` is uncovered by both embedded faces (see part 1/4's probe in
    // `PLANS.md`); `≡` is the closest covered substitute and draws crisp in
    // Tiny5.
    fixed_width_button(
        "≡",
        OPTIONS_BUTTON_WIDTH,
        theme::shape_font(),
        Message::ToggleOptions,
    )
    .into()
}

/// One of the transport row's panel toggles, built from its spec and pinned to the spec's width.
/// `open` sinks the chrome while the panel it controls is shown (see
/// [`style::toggle_button_style`]), the same on/off rule as the title bar's `M` toggle (the LCD
/// strip's `A` is a lit letter, not sunken chrome). The EQ and PL toggles both build through here,
/// so neither can drift from that rule.
fn panel_toggle_button(
    (label, width, message): (&'static str, f32, Message),
    open: bool,
) -> Button<'static, Message> {
    fixed_width_toggle_button(label, width, theme::ui_font(), message, open)
}

/// The balance slider's container id, used only by the layout tests to find the laid-out slider and
/// publish a wheel roll over it, like `SEEK_BAR_ID`.
#[cfg(test)]
pub(crate) const BALANCE_SLIDER_ID: &str = "transport-balance";

/// The transport row: the Previous, Play, Pause, Stop, Next, Repeat, and Shuffle buttons, the
/// volume slider, the balance slider, the fixed-width `EQ` and `PL` panel toggles, and the
/// `≡` options-menu button. `volume` and `balance` are the sliders' current values; dragging
/// them emits `Message::VolumeChange` and `Message::BalanceChange` respectively. `repeat` is the
/// shared Repeat flag and `shuffle` the shared Shuffle flag, each shown on its button by the square
/// LED and toggled by pressing it: pressing a toggle swaps the lit and dark LED square, not the
/// label or the chrome. The five glyph buttons carry word tooltips. `eq_panel_visible` sinks the EQ button while the equalizer section is drawn and leaves
/// its chrome raised while the section is hidden; `library_panel_visible` does the same for the PL
/// button and the library section. The options button publishes [`Message::ToggleOptions`] and is
/// separated from PL by the row's usual gap.
pub fn view_transport_controls(
    volume: f32,
    balance: f32,
    repeat: bool,
    shuffle: bool,
    eq_panel_visible: bool,
    library_panel_visible: bool,
) -> Element<'static, Message> {
    let mut row = Row::new();
    for button in transport_buttons(repeat, shuffle) {
        row = row.push(button).push(spacer(20.0));
    }
    row = row.push(chrome_slider(
        VOLUME_MIN,
        VOLUME_MAX,
        volume,
        VOLUME_STEP,
        100.0,
        style::volume_level_color(volume),
        Message::VolumeChange,
    ));
    // The balance slider's own wheel handling wraps the slider inside the
    // player-panel area, so a plain roll moves the balance instead of the
    // volume while the slider's own Ctrl+wheel step still runs first.
    let balance_slider = MouseArea::new(chrome_slider(
        BALANCE_MIN,
        BALANCE_MAX,
        balance,
        BALANCE_STEP,
        100.0,
        style::balance_level_color(balance),
        Message::BalanceChange,
    ))
    .on_scroll(super::balance_wheel_message);
    #[cfg(test)]
    let balance_slider = Container::new(balance_slider).id(BALANCE_SLIDER_ID);
    row = row
        .push(balance_slider)
        .push(spacer(20.0))
        .push(panel_toggle_button(eq_panel_toggle(), eq_panel_visible))
        .push(panel_toggle_button(
            library_panel_toggle(),
            library_panel_visible,
        ))
        .push(spacer(20.0))
        .push(options_button());
    row.into()
}

#[cfg(test)]
mod size_pins {
    // Part 2/4 of the pixel-font split re-based every size the transport row
    // hands iced onto the 8px grid — only multiples of 8 keep the embedded
    // faces crisp — so pin the tooltip word's size on the grid.
    use super::TOOLTIP_WORD_SIZE;

    #[test]
    fn transport_text_sizes_stay_on_the_pixel_grid() {
        assert_eq!(TOOLTIP_WORD_SIZE, 16.0);
        assert_eq!(TOOLTIP_WORD_SIZE % 8.0, 0.0);
    }
}
