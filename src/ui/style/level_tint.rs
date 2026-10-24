//! Level-dependent groove tints for the volume and balance sliders.

use iced::Color;

use super::{mix_channel, theme};

fn mix(start: Color, end: Color, t: f32) -> Color {
    Color::from_rgba(
        mix_channel(start.r, end.r, t),
        mix_channel(start.g, end.g, t),
        mix_channel(start.b, end.b, t),
        1.0,
    )
}

/// The volume groove tint for `volume`: green ([`theme::EQ_GAIN_LOW`]) when
/// silent, the mockup's orange ([`theme::VOLUME_SLIDER_FILL`]) at half volume,
/// and red ([`theme::EQ_GAIN_HIGH`]) at full volume, interpolated in between.
///
/// `volume` passes through [`crate::state::clamp_volume`] first, so an
/// out-of-range value lands on an end stop and a NaN maps to silence. Alpha is
/// `1.0`.
pub fn volume_level_color(volume: f32) -> Color {
    let v = crate::state::clamp_volume(volume);
    if v <= 0.5 {
        mix(theme::EQ_GAIN_LOW, theme::VOLUME_SLIDER_FILL, v / 0.5)
    } else {
        mix(
            theme::VOLUME_SLIDER_FILL,
            theme::EQ_GAIN_HIGH,
            (v - 0.5) / 0.5,
        )
    }
}

/// The balance groove tint for `balance`: the mockup's green
/// ([`theme::BALANCE_SLIDER_FILL`]) at center, reddening to
/// [`theme::EQ_GAIN_HIGH`] at either extreme, symmetric left and right.
///
/// `balance` passes through [`crate::state::clamp_balance`] first, so a NaN
/// maps to center. Alpha is `1.0`.
pub fn balance_level_color(balance: f32) -> Color {
    let t = crate::state::clamp_balance(balance).abs();
    mix(theme::BALANCE_SLIDER_FILL, theme::EQ_GAIN_HIGH, t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn between(c: f32, a: f32, b: f32) -> bool {
        c >= a.min(b) && c <= a.max(b)
    }

    #[test]
    fn volume_color_hits_its_three_stops_exactly() {
        assert_eq!(volume_level_color(0.0), theme::EQ_GAIN_LOW);
        assert_eq!(volume_level_color(0.5), theme::VOLUME_SLIDER_FILL);
        assert_eq!(volume_level_color(1.0), theme::EQ_GAIN_HIGH);
    }

    #[test]
    fn volume_color_clamps_and_maps_nan_to_silence() {
        assert_eq!(volume_level_color(f32::NAN), theme::EQ_GAIN_LOW);
        assert_eq!(volume_level_color(-1.0), theme::EQ_GAIN_LOW);
        assert_eq!(volume_level_color(2.0), theme::EQ_GAIN_HIGH);
    }

    #[test]
    fn volume_color_between_stops_lies_between_neighbours() {
        let q = volume_level_color(0.25);
        let (lo, mid) = (theme::EQ_GAIN_LOW, theme::VOLUME_SLIDER_FILL);
        assert!(
            between(q.r, lo.r, mid.r) && between(q.g, lo.g, mid.g) && between(q.b, lo.b, mid.b)
        );
        let q = volume_level_color(0.75);
        let hi = theme::EQ_GAIN_HIGH;
        assert!(
            between(q.r, mid.r, hi.r) && between(q.g, mid.g, hi.g) && between(q.b, mid.b, hi.b)
        );
    }

    #[test]
    fn balance_color_is_green_at_center_and_red_at_the_sides() {
        assert_eq!(balance_level_color(0.0), theme::BALANCE_SLIDER_FILL);
        assert_eq!(balance_level_color(f32::NAN), theme::BALANCE_SLIDER_FILL);
        assert_eq!(balance_level_color(-1.0), theme::EQ_GAIN_HIGH);
        assert_eq!(balance_level_color(1.0), theme::EQ_GAIN_HIGH);
    }

    #[test]
    fn balance_color_is_symmetric() {
        assert_eq!(balance_level_color(-0.4), balance_level_color(0.4));
    }

    #[test]
    fn level_colors_are_opaque() {
        for v in [0.0, 0.1, 0.5, 0.9, 1.0] {
            assert_eq!(volume_level_color(v).a, 1.0);
            assert_eq!(balance_level_color(v).a, 1.0);
        }
    }
}
