//! Slider chrome styles: the base groove-and-thumb style plus the tinted variants built on it.
//!
//! [`chrome_slider_style`] is the sunken-groove / raised-thumb base;
//! [`level_slider_style`] and [`band_slider_style`] repaint only its rail fill
//! (the equalizer's gain color comes from [`band_gain_color`]). The bevel
//! faces and 1px border helper they share live in the parent `style` module.

use iced::{Background, Color, widget::slider};

use super::super::theme;
use super::{chrome_face, mix_channel, square_border};

/// The sunken-groove / raised-thumb chrome style for a slider.
///
/// The base skin's slider is a dark recess with a blocky chrome thumb. iced's [`slider::Rail`]
/// paints a single-color [`Border`](iced::Border) and no shadow, so the sunken edge is the dark 1px
/// outline around a near-black groove; the [`slider::Handle`] carries the light 1px border that
/// reads as raised chrome, the same light-border trick
/// [`chrome_button_style`](super::chrome_button_style) uses. The rail is uniform
/// (`LCD_BACKGROUND` on both sides of the handle) because the thumb, not a fill color, marks the
/// value. The handle face comes from [`chrome_face`], the same ladder
/// [`chrome_button_style`](super::chrome_button_style) uses. Pure, so the color rule is testable
/// without building a widget.
pub fn chrome_slider_style(status: slider::Status) -> slider::Style {
    let face = chrome_face(
        matches!(status, slider::Status::Hovered),
        matches!(status, slider::Status::Dragged),
    );

    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(theme::LCD_BACKGROUND),
                Background::Color(theme::LCD_BACKGROUND),
            ),
            width: 4.0,
            border: square_border(theme::PANEL_EDGE_DARK),
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Rectangle {
                width: 8,
                border_radius: 0.0.into(),
            },
            background: Background::Color(face),
            border_width: 1.0,
            border_color: theme::PANEL_EDGE_LIGHT,
        },
    }
}

/// The equalizer band tint for `gain`: green at [`GAIN_MIN_DB`](crate::equalizer::GAIN_MIN_DB),
/// yellow at `0.0`, and red at [`GAIN_MAX_DB`](crate::equalizer::GAIN_MAX_DB), interpolated in
/// between.
///
/// `gain` passes through [`clamp_gain`](crate::equalizer::clamp_gain) first, so
/// an out-of-range value lands on the matching end stop and a NaN maps to the
/// flat `0.0` (yellow) instead of producing an invalid color. Alpha stays
/// `1.0`, so the tint is opaque on the dark groove. Pure, so the color rule is
/// testable without building a widget.
pub fn band_gain_color(gain: f32) -> Color {
    let gain = crate::equalizer::clamp_gain(gain);
    if gain <= 0.0 {
        let t = (gain - crate::equalizer::GAIN_MIN_DB) / (0.0 - crate::equalizer::GAIN_MIN_DB);
        Color::from_rgba(
            mix_channel(theme::EQ_GAIN_LOW.r, theme::EQ_GAIN_MID.r, t),
            mix_channel(theme::EQ_GAIN_LOW.g, theme::EQ_GAIN_MID.g, t),
            mix_channel(theme::EQ_GAIN_LOW.b, theme::EQ_GAIN_MID.b, t),
            1.0,
        )
    } else {
        let t = gain / crate::equalizer::GAIN_MAX_DB;
        Color::from_rgba(
            mix_channel(theme::EQ_GAIN_MID.r, theme::EQ_GAIN_HIGH.r, t),
            mix_channel(theme::EQ_GAIN_MID.g, theme::EQ_GAIN_HIGH.g, t),
            mix_channel(theme::EQ_GAIN_MID.b, theme::EQ_GAIN_HIGH.b, t),
            1.0,
        )
    }
}

/// The chrome slider style with both rail halves tinted by the band's `gain`.
///
/// iced paints `backgrounds.0` from the range start to the thumb and `backgrounds.1` from the thumb
/// to the range end, so both halves must carry [`band_gain_color`]; tinting only one would leave a
/// gray segment whose extent moves with the value. The rail width and border and the whole handle
/// stay exactly [`chrome_slider_style`]'s, so the tint changes only the groove's color. Delegates
/// the rail-fill rule to [`level_slider_style`], so the two styles cannot drift apart. Pure, so the
/// rule is testable without building a widget.
pub fn band_slider_style(gain: f32, status: slider::Status) -> slider::Style {
    level_slider_style(band_gain_color(gain), status)
}

/// The chrome slider style with both rail halves painted `fill`.
///
/// This is the level sliders' style (transport volume and balance, shade volume), and
/// [`band_slider_style`] reuses it with the band's gain color as the fill: iced paints
/// `backgrounds.0` from the range start to the thumb and `backgrounds.1` from the thumb to the
/// range end, so both halves carry the fill and no gray segment remains on either side. The rail
/// width and border and the whole handle stay exactly [`chrome_slider_style`]'s, so the fill
/// changes only the groove's color while hover and drag still swap only the handle face. Pure, so
/// the rule is testable without building a widget.
pub fn level_slider_style(fill: Color, status: slider::Status) -> slider::Style {
    let mut style = chrome_slider_style(status);
    let fill = Background::Color(fill);
    style.rail.backgrounds = (fill, fill);
    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::equalizer::{GAIN_MAX_DB, GAIN_MIN_DB};

    /// Asserts `status`'s chrome slider handle keeps `active`'s shape and 1px
    /// border.
    ///
    /// Interaction statuses change the handle face; they must not change the thumb's geometry or
    /// border. The Hovered and Dragged assertions in
    /// `chrome_slider_style_hovered_and_dragged_only_swap_the_handle_face` each pin that, so the
    /// contract lives here once.
    fn assert_same_handle_geometry(active: &slider::Style, status: &slider::Style) {
        assert_eq!(status.handle.shape, active.handle.shape);
        assert_eq!(status.handle.border_width, active.handle.border_width);
        assert_eq!(status.handle.border_color, active.handle.border_color);
    }

    #[test]
    fn chrome_slider_style_active_is_a_sunken_groove_with_a_raised_thumb() {
        let style = chrome_slider_style(slider::Status::Active);
        assert_eq!(
            style.rail.backgrounds,
            (
                Background::Color(theme::LCD_BACKGROUND),
                Background::Color(theme::LCD_BACKGROUND)
            )
        );
        assert_eq!(style.rail.width, 4.0);
        assert_eq!(style.rail.border.color, theme::PANEL_EDGE_DARK);
        assert_eq!(style.rail.border.width, 1.0);
        // The rail's blocky 1px outline: a Winamp sunken groove reads as chrome
        // only with square corners. The whole-`Rail` equality in the
        // hovered/dragged test pins only that the radius is status-independent,
        // not its literal value, so a shared-code change to a rounded groove
        // would clear every other test while the rail lost its chrome shape.
        assert_eq!(style.rail.border.radius, 0.0.into());
        assert_eq!(
            style.handle.shape,
            slider::HandleShape::Rectangle {
                width: 8,
                border_radius: 0.0.into()
            }
        );
        assert_eq!(
            style.handle.background,
            Background::Color(theme::BUTTON_FACE)
        );
        assert_eq!(style.handle.border_width, 1.0);
        assert_eq!(style.handle.border_color, theme::PANEL_EDGE_LIGHT);
    }

    #[test]
    fn chrome_slider_style_hovered_and_dragged_only_swap_the_handle_face() {
        let active = chrome_slider_style(slider::Status::Active);
        let hovered = chrome_slider_style(slider::Status::Hovered);
        let dragged = chrome_slider_style(slider::Status::Dragged);

        assert_eq!(
            hovered.handle.background,
            Background::Color(theme::BUTTON_FACE_HOVERED)
        );
        assert_eq!(
            dragged.handle.background,
            Background::Color(theme::BUTTON_FACE_PRESSED)
        );
        assert_eq!(hovered.rail, active.rail);
        assert_eq!(dragged.rail, active.rail);
        // The name's "only" is the contract: hovering and dragging swap the
        // handle face and nothing else. The Active test above pins the handle's
        // shape and border for the shared fields; assert the two non-resting
        // statuses keep them unchanged too, or a change that made (say) the
        // dragged thumb's border a status-dependent color would clear every
        // other test while the thumb gained a color no test read.
        assert_same_handle_geometry(&active, &hovered);
        assert_same_handle_geometry(&active, &dragged);
    }

    /// Asserts each RGB channel of `color` lies between `start` and `end`.
    fn assert_between(color: Color, start: Color, end: Color) {
        for (channel, (value, (low, high))) in [
            (color.r, (start.r, end.r)),
            (color.g, (start.g, end.g)),
            (color.b, (start.b, end.b)),
        ]
        .into_iter()
        .enumerate()
        {
            let (low, high) = (low.min(high), low.max(high));
            assert!(
                value >= low && value <= high,
                "channel {channel} = {value} is outside {low}..={high}"
            );
        }
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn band_gain_color_runs_green_through_yellow_to_red() {
        assert_eq!(band_gain_color(GAIN_MIN_DB), theme::EQ_GAIN_LOW);
        assert_eq!(band_gain_color(0.0), theme::EQ_GAIN_MID);
        assert_eq!(band_gain_color(GAIN_MAX_DB), theme::EQ_GAIN_HIGH);

        // The middle of each half sits strictly between its two stops, so the
        // tint sweeps rather than jumping at the named gains.
        assert_between(
            band_gain_color(GAIN_MIN_DB / 2.0),
            theme::EQ_GAIN_LOW,
            theme::EQ_GAIN_MID,
        );
        assert_between(
            band_gain_color(GAIN_MAX_DB / 2.0),
            theme::EQ_GAIN_MID,
            theme::EQ_GAIN_HIGH,
        );
    }

    #[test]
    fn band_gain_color_clamps_and_maps_nan_to_flat() {
        assert_eq!(band_gain_color(GAIN_MIN_DB - 100.0), theme::EQ_GAIN_LOW);
        assert_eq!(band_gain_color(GAIN_MAX_DB + 100.0), theme::EQ_GAIN_HIGH);
        assert_eq!(band_gain_color(f32::NAN), theme::EQ_GAIN_MID);
    }

    /// Asserts `style_for` paints both rail halves `fill` at every slider status while keeping the
    /// chrome slider's rail geometry and thumb.
    fn assert_fills_rail_and_keeps_chrome_thumb(
        fill: Color,
        style_for: impl Fn(slider::Status) -> slider::Style,
    ) {
        let fill = Background::Color(fill);
        for status in [
            slider::Status::Active,
            slider::Status::Hovered,
            slider::Status::Dragged,
        ] {
            let style = style_for(status);
            let chrome = chrome_slider_style(status);
            assert_eq!(style.rail.backgrounds, (fill, fill));
            assert_eq!(style.rail.width, chrome.rail.width);
            assert_eq!(style.rail.border, chrome.rail.border);
            assert_eq!(style.handle, chrome.handle);
        }
    }

    #[test]
    fn band_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb() {
        let gain = 6.0;
        assert_fills_rail_and_keeps_chrome_thumb(band_gain_color(gain), |status| {
            band_slider_style(gain, status)
        });
    }

    #[test]
    fn level_slider_style_paints_both_rail_halves_and_keeps_the_chrome_thumb() {
        for fill in [theme::VOLUME_SLIDER_FILL, theme::BALANCE_SLIDER_FILL] {
            assert_fills_rail_and_keeps_chrome_thumb(fill, |status| {
                level_slider_style(fill, status)
            });
        }
    }
}
