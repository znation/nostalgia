//! NaN-safe float clamping shared by the two shared-state float rules.
//!
//! `state::clamp_volume` and `equalizer::clamp_gain` both apply the same rule
//! to a slider value: clamp an in-range value to its bounds, but map a NaN to
//! a caller-chosen neutral value instead of letting `f32::clamp` pass it
//! through. The rule lives here once, so each domain wrapper only names its
//! own range and fallback.

/// Clamps `value` to `[min, max]`, mapping NaN to `nan_fallback`.
///
/// `f32::clamp` passes NaN through unchanged, so a NaN would otherwise be
/// stored as-is in shared state; `nan_fallback` is the safe, neutral value
/// for the caller's domain. Non-finite values that *are* comparable, `+inf`
/// and `-inf`, clamp to the nearer bound like any other out-of-range value.
///
/// `#[must_use]` guards the contract that a clamped value must be stored:
/// the function's entire purpose is its returned value, so a caller that
/// drops it has silently done nothing, leaving the unclamped (possibly NaN)
/// value in shared state with no error. Making the result `#[must_use]`
/// turns that silent no-op into a compile error.
#[must_use]
pub fn clamp_with_nan_fallback(value: f32, min: f32, max: f32, nan_fallback: f32) -> f32 {
    if value.is_nan() {
        nan_fallback
    } else {
        value.clamp(min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::clamp_with_nan_fallback;

    // A range and fallback distinct from any caller's, so these tests pin the
    // helper's generic behaviour rather than one domain's numbers.
    const MIN: f32 = -5.0;
    const MAX: f32 = 5.0;
    const FALLBACK: f32 = -1.0;

    #[test]
    fn caps_at_the_upper_bound() {
        assert_eq!(clamp_with_nan_fallback(99.0, MIN, MAX, FALLBACK), MAX);
    }

    #[test]
    fn floors_at_the_lower_bound() {
        assert_eq!(clamp_with_nan_fallback(-99.0, MIN, MAX, FALLBACK), MIN);
    }

    #[test]
    fn passes_through_in_range() {
        assert_eq!(clamp_with_nan_fallback(MIN, MIN, MAX, FALLBACK), MIN);
        assert_eq!(clamp_with_nan_fallback(0.0, MIN, MAX, FALLBACK), 0.0);
        assert_eq!(clamp_with_nan_fallback(2.5, MIN, MAX, FALLBACK), 2.5);
        assert_eq!(clamp_with_nan_fallback(MAX, MIN, MAX, FALLBACK), MAX);
    }

    #[test]
    fn maps_nan_to_the_caller_fallback() {
        // `f32::clamp` passes NaN through unchanged, so the helper must
        // handle it explicitly or a non-finite value lands in shared state.
        assert_eq!(
            clamp_with_nan_fallback(f32::NAN, MIN, MAX, FALLBACK),
            FALLBACK
        );
    }

    #[test]
    fn caps_positive_infinity_at_the_upper_bound() {
        // `+inf` is non-finite but comparable (it exceeds every value), so it
        // clamps to the upper bound — not the fallback reserved for the
        // incomparable NaN. Pinned so a refactor of the non-finite handling
        // can't silently change this branch.
        assert_eq!(
            clamp_with_nan_fallback(f32::INFINITY, MIN, MAX, FALLBACK),
            MAX
        );
    }

    #[test]
    fn floors_negative_infinity_at_the_lower_bound() {
        // The `-inf` twin of `+inf`: comparable and below every value, so it
        // clamps to the lower bound.
        assert_eq!(
            clamp_with_nan_fallback(f32::NEG_INFINITY, MIN, MAX, FALLBACK),
            MIN
        );
    }
}
