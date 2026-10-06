//! The equalizer's data model: the ten Winamp frequency bands, the shared
//! gain range they move within, and the pure gain clamp both the shared state
//! and the UI apply. Kept in its own module beside `library` and `state` so
//! both the state and the UI can depend on it without a `state` ← `ui` cycle.

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

/// Clamps an equalizer gain to the valid `[GAIN_MIN_DB, GAIN_MAX_DB]` range.
///
/// iced's sliders can emit a value outside the range (a drag beyond the ends,
/// or a stale in-flight change), and the UI must never store an unclamped
/// gain in `AppState`. The gains are shared state, so the rule lives beside
/// the band constants as a pure function the UI's `update` arms call before
/// storing, exactly as `state::clamp_volume` does for volume.
///
/// `f32::clamp` passes NaN through unchanged, so a NaN gain is mapped to
/// `0.0` (flat) rather than being stored as-is — the safe, neutral outcome
/// for a value that is neither in range nor comparable to it. Non-finite
/// values that *are* comparable, `+inf` and `-inf`, clamp to the nearer bound
/// like any other out-of-range value: `+inf` to `GAIN_MAX_DB`, `-inf` to
/// `GAIN_MIN_DB`.
///
/// `#[must_use]` guards the contract that a clamped value must be stored:
/// the function's entire purpose is its returned value, so a caller that
/// drops it has silently done nothing, leaving the unclamped (possibly NaN)
/// gain in shared state with no error. Making the result `#[must_use]` turns
/// that silent no-op into a compile error, the same way the `f32::clamp` NaN
/// hole is caught by the function itself.
#[must_use]
pub fn clamp_gain(gain: f32) -> f32 {
    if gain.is_nan() {
        0.0
    } else {
        gain.clamp(GAIN_MIN_DB, GAIN_MAX_DB)
    }
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
        // `f32::clamp` passes NaN through unchanged, so it must be handled
        // explicitly or a non-finite value lands in shared state.
        assert_eq!(clamp_gain(f32::NAN), 0.0);
    }

    #[test]
    fn clamp_gain_caps_positive_infinity_at_the_upper_bound() {
        // `+inf` is non-finite but comparable (it exceeds every gain), so
        // `f32::clamp` maps it to the upper bound — not the flat value
        // reserved for the incomparable NaN. Pinned so a refactor of the
        // non-finite handling can't silently change this branch.
        assert_eq!(clamp_gain(f32::INFINITY), GAIN_MAX_DB);
    }

    #[test]
    fn clamp_gain_floors_negative_infinity_at_the_lower_bound() {
        // The `-inf` twin of `+inf`: comparable and below every gain, so it
        // clamps to the lower bound.
        assert_eq!(clamp_gain(f32::NEG_INFINITY), GAIN_MIN_DB);
    }
}
