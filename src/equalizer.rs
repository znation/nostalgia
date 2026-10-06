//! The equalizer's data model: the ten Winamp frequency bands, the shared
//! gain range they move within, and the pure gain clamp the shared state
//! applies. Kept in its own module beside `library` and `state` so both the
//! state and the UI can depend on it without a `state` ← `ui` cycle.

use crate::clamp;

/// The centre frequencies of Winamp's ten equalizer bands, low to high, as
/// the short display labels printed under each slider (Winamp shows the
/// hundreds-of-hertz bands in full and abbreviates the kilohertz ones).
pub const BAND_FREQUENCIES: [&str; 10] = [
    "60", "170", "310", "600", "1K", "3K", "6K", "12K", "14K", "16K",
];

/// The number of equalizer bands. Derived from [`BAND_FREQUENCIES`] so the
/// band array in the shared state and the sliders the UI builds can never
/// drift out of step with the label list.
pub const BAND_COUNT: usize = BAND_FREQUENCIES.len();

/// The lowest band gain in decibels, the bottom of every Winamp equalizer
/// slider's range.
pub const GAIN_MIN_DB: f32 = -12.0;

/// The highest band gain in decibels, the top of every Winamp equalizer
/// slider's range.
pub const GAIN_MAX_DB: f32 = 12.0;

/// Clamps an equalizer gain to the valid `[GAIN_MIN_DB, GAIN_MAX_DB]` range,
/// mapping a NaN to `0.0` (flat).
///
/// iced's sliders can emit a value outside the range (a drag beyond the ends,
/// or a stale in-flight change), and the UI must never store an unclamped
/// gain in `AppState`. The gains are shared state, so the rule lives beside
/// the band constants as a pure function the `AppState` gain setters call
/// before storing, exactly as `state::clamp_volume` does for volume. The
/// NaN-and-bounds behaviour itself is
/// [`crate::clamp::clamp_with_nan_fallback`], shared with
/// `state::clamp_volume`; this wrapper only names the gain range and its flat
/// fallback.
#[must_use]
pub fn clamp_gain(gain: f32) -> f32 {
    clamp::clamp_with_nan_fallback(gain, GAIN_MIN_DB, GAIN_MAX_DB, 0.0)
}

#[cfg(test)]
mod tests {
    use super::{BAND_COUNT, BAND_FREQUENCIES, GAIN_MAX_DB, GAIN_MIN_DB, clamp_gain};

    #[test]
    fn band_frequencies_match_the_band_count() {
        assert_eq!(BAND_FREQUENCIES.len(), BAND_COUNT);
    }

    #[test]
    fn clamp_gain_caps_at_the_upper_bound() {
        assert_eq!(clamp_gain(99.0), GAIN_MAX_DB);
    }

    #[test]
    fn clamp_gain_floors_at_the_lower_bound() {
        assert_eq!(clamp_gain(-99.0), GAIN_MIN_DB);
    }

    #[test]
    fn clamp_gain_passes_through_in_range() {
        assert_eq!(clamp_gain(GAIN_MIN_DB), GAIN_MIN_DB);
        assert_eq!(clamp_gain(0.0), 0.0);
        assert_eq!(clamp_gain(3.5), 3.5);
        assert_eq!(clamp_gain(GAIN_MAX_DB), GAIN_MAX_DB);
    }

    #[test]
    fn clamp_gain_treats_nan_as_flat() {
        // The shared clamp handles the NaN branch; this pins that
        // `clamp_gain` passes the flat fallback.
        assert_eq!(clamp_gain(f32::NAN), 0.0);
    }
}
