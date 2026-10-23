//! The equalizer's data model: the ten Winamp frequency bands, the shared
//! gain range they move within, and the pure gain clamp the shared state
//! applies. Kept in its own module beside `library` and `state` so both the
//! state and the UI can depend on it without a `state` ← `ui` cycle.

use crate::clamp;

/// The center frequencies of Winamp's ten equalizer bands, low to high, as
/// the short display labels printed under each slider (Winamp shows the
/// hundreds-of-hertz bands in full and abbreviates the kilohertz ones).
pub const BAND_FREQUENCIES: [&str; 10] = [
    "60", "170", "310", "600", "1K", "3K", "6K", "12K", "14K", "16K",
];

/// The number of equalizer bands. Derived from [`BAND_FREQUENCIES`] so the
/// band array in the shared state and the sliders the UI builds can never
/// drift out of step with the label list.
pub const BAND_COUNT: usize = BAND_FREQUENCIES.len();

/// The numeric center frequency of each band, in hertz and in
/// [`BAND_FREQUENCIES`] order, matching the labels the panel shows.
///
/// [`BandFilter`] builds one peaking biquad per entry, so changing a label
/// without this table would leave the panel and the sound out of step.
pub const BAND_HZ: [f32; BAND_COUNT] = [
    60.0, 170.0, 310.0, 600.0, 1_000.0, 3_000.0, 6_000.0, 12_000.0, 14_000.0, 16_000.0,
];

/// The Q (bandwidth) of every peaking filter: roughly one octave, the width
/// Winamp's ten-band equalizer uses.
const BAND_Q: f32 = 1.41;

/// The lowest band gain in decibels, the bottom of every Winamp equalizer
/// slider's range.
pub const GAIN_MIN_DB: f32 = -12.0;

/// The highest band gain in decibels, the top of every Winamp equalizer
/// slider's range.
pub const GAIN_MAX_DB: f32 = 12.0;

/// A named equalizer curve: a preamp gain plus one gain per
/// [`BAND_COUNT`] band, in [`BAND_FREQUENCIES`] order.
///
/// `Copy` so a selection can be read out of shared state and passed to the
/// view by value, and `Display` so iced's `PickList` can render the name in
/// its selected label and its menu. The stored gains are the raw Winamp 2.x
/// curve values; [`crate::state::AppState::apply_eq_preset`] still runs them
/// through [`clamp_gain`] before storing, so a preset can never push shared
/// state out of range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preset {
    /// The preset's display name, shown in the equalizer's pick list.
    pub name: &'static str,
    /// The preamp gain in decibels, applied ahead of the bands.
    pub preamp: f32,
    /// The per-band gains in decibels, low frequency to high.
    pub bands: [f32; BAND_COUNT],
}

impl std::fmt::Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}

/// The Winamp 2.x equalizer preset curves, in base-skin order.
///
/// The classic Winamp preset list, from Flat through Vocal, with the
/// documented curve values (Winamp's preset data stores every preamp at
/// `0.0` and puts the character in the ten band gains). This is the source
/// list the equalizer panel's pick list offers; selecting one applies the
/// whole curve through [`crate::state::AppState::apply_eq_preset`].
pub const PRESETS: [Preset; 19] = [
    Preset {
        name: "Flat",
        preamp: 0.0,
        bands: [0.0; BAND_COUNT],
    },
    Preset {
        name: "Classical",
        preamp: 0.0,
        bands: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -4.4, -4.4, -4.4, -5.6],
    },
    Preset {
        name: "Club",
        preamp: 0.0,
        bands: [0.0, 0.0, 8.0, 5.6, 5.6, 5.6, 3.1, 0.0, 0.0, 0.0],
    },
    Preset {
        name: "Dance",
        preamp: 0.0,
        bands: [5.6, 7.5, 4.4, 0.0, 0.0, -5.6, -7.5, -7.5, 0.0, 0.0],
    },
    Preset {
        name: "Full Bass",
        preamp: 0.0,
        bands: [8.7, 8.7, 8.7, 5.6, 1.9, -4.4, -7.5, -8.1, -8.7, -8.7],
    },
    Preset {
        name: "Full Bass & Treble",
        preamp: 0.0,
        bands: [7.5, 5.6, 0.0, -7.5, -4.4, 1.9, 8.7, 8.7, 8.7, 8.7],
    },
    Preset {
        name: "Full Treble",
        preamp: 0.0,
        bands: [-9.4, -9.4, -9.4, -4.4, 1.9, 8.7, 9.4, 9.4, 9.4, 10.0],
    },
    Preset {
        name: "Headphones",
        preamp: 0.0,
        bands: [4.4, 7.5, 5.6, 0.0, -3.1, -1.9, 1.9, 5.6, 8.1, 8.7],
    },
    Preset {
        name: "Large Hall",
        preamp: 0.0,
        bands: [8.7, 8.7, 5.6, 5.6, 0.0, -5.6, -5.6, -5.6, 0.0, 0.0],
    },
    Preset {
        name: "Live",
        preamp: 0.0,
        bands: [-4.4, -1.9, 0.0, 3.1, 5.6, 5.6, 5.6, 3.1, 1.9, 1.9],
    },
    Preset {
        name: "Party",
        preamp: 0.0,
        bands: [7.5, 7.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 7.5, 7.5],
    },
    Preset {
        name: "Pop",
        preamp: 0.0,
        bands: [-1.9, 1.9, 4.4, 5.6, 5.6, 0.0, -1.9, -1.9, -1.9, -1.9],
    },
    Preset {
        name: "Reggae",
        preamp: 0.0,
        bands: [0.0, 0.0, 0.0, -4.4, 0.0, 4.4, 4.4, 0.0, 0.0, 0.0],
    },
    Preset {
        name: "Rock",
        preamp: 0.0,
        bands: [8.1, 4.4, -5.6, -7.5, -3.1, 4.4, 8.7, 11.2, 11.2, 11.2],
    },
    Preset {
        name: "Ska",
        preamp: 0.0,
        bands: [-2.5, -4.4, -2.5, 0.0, 0.0, 0.0, 0.0, -2.5, -4.4, -5.6],
    },
    Preset {
        name: "Soft",
        preamp: 0.0,
        bands: [4.4, 1.9, 0.0, -1.9, -1.9, 0.0, 4.4, 8.1, 8.1, 8.1],
    },
    Preset {
        name: "Soft Rock",
        preamp: 0.0,
        bands: [4.4, 4.4, 1.9, -1.9, -3.1, -1.9, 1.9, 5.6, 8.1, 8.1],
    },
    Preset {
        name: "Techno",
        preamp: 0.0,
        bands: [4.4, 1.9, 0.0, -1.9, -4.4, -1.9, 4.4, 8.1, 8.1, 8.1],
    },
    Preset {
        name: "Vocal",
        preamp: 0.0,
        bands: [-1.9, -3.1, -3.1, 1.9, 5.6, 5.6, 5.6, 3.1, 0.0, 0.0],
    },
];

/// Clamps an equalizer gain to the valid `[GAIN_MIN_DB, GAIN_MAX_DB]` range,
/// mapping a NaN to `0.0` (flat).
///
/// iced's sliders can emit a value outside the range (a drag beyond the ends,
/// or a stale in-flight change), and the UI must never store an unclamped
/// gain in `AppState`. The gains are shared state, so the rule lives beside
/// the band constants as a pure function the `AppState` gain setters call
/// before storing, exactly as `state::clamp_volume` does for volume. The
/// NaN-and-bounds behavior itself is
/// [`crate::clamp::clamp_with_nan_fallback`], shared with
/// `state::clamp_volume`; this wrapper only names the gain range and its flat
/// fallback.
#[must_use]
pub fn clamp_gain(gain: f32) -> f32 {
    clamp::clamp_with_nan_fallback(gain, GAIN_MIN_DB, GAIN_MAX_DB, 0.0)
}

/// One channel's ten-band equalizer: an RBJ peaking biquad per band plus the
/// preamp gain, rebuilt whenever the curve changes.
///
/// The audio output keeps one filter per channel, so each channel's filter
/// state stays independent. A disabled filter returns every sample unchanged;
/// an enabled flat curve is also a pass-through within floating-point error.
pub struct BandFilter {
    sample_rate: u32,
    enabled: bool,
    preamp_gain: f32,
    bands: [Biquad; BAND_COUNT],
}

impl BandFilter {
    /// Builds a disabled, flat filter for a source at `sample_rate` hertz.
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        let mut filter = Self {
            sample_rate,
            enabled: false,
            preamp_gain: 1.0,
            bands: [Biquad::flat(); BAND_COUNT],
        };
        filter.set_curve(false, 0.0, &[0.0; BAND_COUNT]);
        filter
    }

    /// Rebuilds every band's coefficients for `enabled`, the preamp gain in
    /// decibels, and the ten band gains in decibels, then clears the filter
    /// state so a changed curve starts from silence rather than carrying the
    /// old curve's ringing.
    ///
    /// A band whose center is at or above the source's Nyquist frequency
    /// (half its sample rate) is left as a pass-through: a peaking section
    /// built above Nyquist has poles outside the unit circle and would blow
    /// up the output. Sources below 32 kHz therefore lose the bands above
    /// their usable range rather than becoming unstable.
    pub fn set_curve(&mut self, enabled: bool, preamp_db: f32, bands_db: &[f32; BAND_COUNT]) {
        self.enabled = enabled;
        self.preamp_gain = 10.0_f32.powf(preamp_db / 20.0);
        let rate = self.sample_rate as f32;
        let nyquist = rate / 2.0;
        for (filter, (frequency, gain_db)) in self
            .bands
            .iter_mut()
            .zip(BAND_HZ.iter().zip(bands_db.iter()))
        {
            *filter = if *frequency >= nyquist {
                Biquad::flat()
            } else {
                Biquad::peaking(rate, *frequency, BAND_Q, *gain_db)
            };
        }
    }

    /// Applies the preamp and every band filter to one sample. A disabled
    /// filter returns `sample` unchanged.
    #[must_use]
    pub fn process(&mut self, sample: f32) -> f32 {
        if !self.enabled {
            return sample;
        }
        let mut output = sample * self.preamp_gain;
        for filter in &mut self.bands {
            output = filter.process(output);
        }
        output
    }
}

/// A direct-form-II-transposed biquad section.
#[derive(Debug, Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    /// The identity section, used before [`BandFilter::set_curve`] builds the
    /// real coefficients.
    fn flat() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    /// An RBJ peaking-EQ section centered at `frequency` with bandwidth `q`
    /// and `gain_db` decibels of boost (or cut).
    fn peaking(sample_rate: f32, frequency: f32, q: f32, gain_db: f32) -> Self {
        let amplitude = 10.0_f32.powf(gain_db / 40.0);
        let omega = 2.0 * std::f32::consts::PI * frequency / sample_rate;
        let alpha = omega.sin() / (2.0 * q);
        let cos = omega.cos();
        let a0 = 1.0 + alpha / amplitude;
        Self {
            b0: (1.0 + alpha * amplitude) / a0,
            b1: (-2.0 * cos) / a0,
            b2: (1.0 - alpha * amplitude) / a0,
            a1: (-2.0 * cos) / a0,
            a2: (1.0 - alpha / amplitude) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    /// Filters one sample, updating the section's state.
    fn process(&mut self, sample: f32) -> f32 {
        let output = self.b0 * sample + self.z1;
        self.z1 = self.b1 * sample - self.a1 * output + self.z2;
        self.z2 = self.b2 * sample - self.a2 * output;
        output
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BAND_COUNT, BAND_FREQUENCIES, BAND_HZ, BandFilter, GAIN_MAX_DB, GAIN_MIN_DB, PRESETS,
        clamp_gain,
    };
    use crate::test_support::rock_preset;

    #[test]
    fn band_frequencies_match_the_band_count() {
        assert_eq!(BAND_FREQUENCIES.len(), BAND_COUNT);
    }

    // The display labels and the filters' numeric centers are separate
    // tables; a typo or reordering in either would leave the panel and the
    // sound out of step while every other test still passed. Pin both in the
    // same low-to-high order.
    #[test]
    fn band_frequencies_have_matching_numeric_centers() {
        assert_eq!(
            BAND_HZ,
            [
                60.0, 170.0, 310.0, 600.0, 1_000.0, 3_000.0, 6_000.0, 12_000.0, 14_000.0, 16_000.0
            ]
        );
        assert_eq!(BAND_HZ.len(), BAND_FREQUENCIES.len());
    }

    // `BAND_FREQUENCIES` is the equalizer's user-visible content: `view_equalizer`
    // renders one label under each of the ten sliders, and the base skin shows
    // the hundreds-of-hertz bands in full and abbreviates the kilohertz ones.
    // The length test above compares the array to itself (`BAND_COUNT` is
    // defined as `BAND_FREQUENCIES.len()`), so it can never fail; a typo or a
    // reordered band would pass every test while the panel silently showed the
    // wrong frequency. Pin the exact labels, low to high.
    #[test]
    fn band_frequencies_are_the_winamp_labels_in_order() {
        assert_eq!(
            BAND_FREQUENCIES,
            [
                "60", "170", "310", "600", "1K", "3K", "6K", "12K", "14K", "16K"
            ]
        );
    }

    // `GAIN_MIN_DB`/`GAIN_MAX_DB` are the equalizer's user-visible range:
    // every band slider spans them (`view_equalizer`) and both `clamp_gain`
    // and `AppState` clamp to them. The tests around this one only compare the
    // constants to themselves or to values they themselves define, so editing
    // either literal would pass the whole suite while the panel silently
    // changed range. Pin the documented -12 dB..=+12 dB span and the ordering
    // it promises: min below flat, flat below max, symmetric about 0.
    #[test]
    fn gain_range_is_the_documented_twelve_db_span() {
        assert_eq!(GAIN_MIN_DB, -12.0);
        assert_eq!(GAIN_MAX_DB, 12.0);
        // Bound the constants to locals: `assert!` on a bare constant trips
        // `clippy::assertions_on_constants`.
        let min = GAIN_MIN_DB;
        let max = GAIN_MAX_DB;
        assert!(min < 0.0);
        assert!(0.0 < max);
        assert_eq!(min, -max);
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

    // `PRESETS` is the equalizer's user-visible pick-list content: iced's
    // `PickList` renders each preset's `Display` name and the app shows the
    // selected one in the equalizer header. A duplicate or empty name would
    // make two entries indistinguishable (and the selection ambiguous), so
    // pin both here rather than relying on the exact-curve test below.
    #[test]
    fn presets_have_unique_non_empty_names() {
        for (index, preset) in PRESETS.iter().enumerate() {
            assert!(!preset.name.is_empty(), "preset {index} has an empty name");
            for other in &PRESETS[index + 1..] {
                assert_ne!(preset.name, other.name, "duplicate preset name");
            }
        }
    }

    // The test above reads the `name` field directly, and the
    // `view_equalizer` construction test never lays the widget out, so
    // nothing in the crate exercised `Preset`'s `Display` before this test.
    // That impl is what iced's `PickList` actually renders for the selected
    // preset and for every menu option (see the struct doc), so a `Display`
    // that returned anything but `name` — a debug repr, say — would leave the
    // whole suite green while the pick list showed the wrong label. Pin the
    // rendered label for every preset.
    #[test]
    fn preset_display_renders_the_name() {
        for preset in PRESETS {
            assert_eq!(preset.to_string(), preset.name);
        }
    }

    // The curves are raw Winamp data, but `apply_eq_preset` still runs every
    // gain through `clamp_gain` before storing. A preset with a value outside
    // the slider range would then be silently altered on selection, so pin
    // that every stored gain is already its own clamp.
    #[test]
    fn preset_gains_stay_within_the_clamp_range() {
        for preset in PRESETS {
            assert_eq!(preset.preamp, clamp_gain(preset.preamp), "{}", preset.name);
            for gain in preset.bands {
                assert_eq!(gain, clamp_gain(gain), "{}", preset.name);
            }
        }
    }

    #[test]
    fn flat_preset_is_all_zero() {
        let flat = PRESETS[0];
        assert_eq!(flat.name, "Flat");
        assert_eq!(flat.preamp, 0.0);
        assert_eq!(flat.bands, [0.0; BAND_COUNT]);
    }

    // The flat preset's all-zero shape is pinned above, but the character
    // curves are the point of the table: a transposed or mistyped band would
    // pass the range and uniqueness tests while applying the wrong sound.
    // Pin the classic Rock curve, name and every band, as the representative
    // exact-array check.
    #[test]
    fn rock_preset_pins_the_classic_curve() {
        let rock = rock_preset();
        assert_eq!(rock.preamp, 0.0);
        assert_eq!(
            rock.bands,
            [8.1, 4.4, -5.6, -7.5, -3.1, 4.4, 8.7, 11.2, 11.2, 11.2]
        );
    }

    #[test]
    fn band_filter_disabled_returns_samples_unchanged() {
        let mut filter = BandFilter::new(44_100);
        for sample in [0.0, 0.5, -0.25, 1.0, -1.0] {
            assert_eq!(filter.process(sample), sample);
        }
    }

    // Toggling the equalizer off must bypass the stored curve, not merely
    // skip a flat one: shared state keeps the preamp and band gains while
    // enabled is false, so a filter that applied the gain before checking
    // enabled would keep boosting a preview the user switched off. The test
    // above starts from `new`'s flat preamp, which cannot tell that order
    // apart; this one first runs the curve enabled and then disables it with
    // the same non-zero gains.
    #[test]
    fn band_filter_disabled_bypasses_a_non_zero_stored_curve() {
        let mut filter = BandFilter::new(44_100);
        filter.set_curve(true, 6.020_6, &gain_curve(4, 12.0));
        // Let the enabled filter record state, as it would during playback.
        let _ = filter.process(1.0);

        filter.set_curve(false, 6.020_6, &gain_curve(4, 12.0));
        for sample in [0.0, 0.5, -0.25, 1.0, -1.0] {
            assert_eq!(filter.process(sample), sample);
        }
    }

    #[test]
    fn band_filter_enabled_flat_curve_passes_samples_through() {
        let mut filter = BandFilter::new(44_100);
        filter.set_curve(true, 0.0, &[0.0; BAND_COUNT]);
        for sample in [0.0, 0.5, -0.25, 1.0, -1.0] {
            assert!((filter.process(sample) - sample).abs() < 1e-6);
        }
    }

    // Changing the curve while audio plays must clear the filters' internal
    // state, not just rebuild their coefficients: otherwise the old curve's
    // ringing carries into the new one. Drive an impulse through a boosted
    // band, then switch to a flat curve and prove the next output matches a
    // freshly built flat filter instead of continuing the old ringing.
    #[test]
    fn band_filter_set_curve_clears_the_previous_curves_ringing() {
        let mut filter = BandFilter::new(44_100);
        filter.set_curve(true, 0.0, &gain_curve(4, 12.0));

        let _ = filter.process(1.0);
        assert_ne!(
            filter.process(0.0),
            0.0,
            "the boosted band must leave ringing state to clear"
        );

        filter.set_curve(true, 0.0, &[0.0; BAND_COUNT]);
        let mut fresh = BandFilter::new(44_100);
        fresh.set_curve(true, 0.0, &[0.0; BAND_COUNT]);
        assert_eq!(
            filter.process(0.0),
            fresh.process(0.0),
            "a changed curve must start from silence, not the old curve's ring"
        );
    }

    // The band sliders are useless if the filters ignore their gain, and a
    // sign error would make a boost cut. Drive a 1 kHz sine through the 1 kHz
    // band with +12 dB and -12 dB and compare the settled RMS with the input's.
    #[test]
    fn band_filter_boost_and_cut_at_the_center_frequency() {
        let input_rms = settled_sine_rms(&mut BandFilter::new(44_100), 1_000.0);
        let mut boost = BandFilter::new(44_100);
        boost.set_curve(true, 0.0, &gain_curve(4, 12.0));
        let mut cut = BandFilter::new(44_100);
        cut.set_curve(true, 0.0, &gain_curve(4, -12.0));

        assert!(
            settled_sine_rms(&mut boost, 1_000.0) > input_rms * 2.0,
            "a +12 dB band must raise a centered sine's RMS"
        );
        assert!(
            settled_sine_rms(&mut cut, 1_000.0) < input_rms * 0.5,
            "a -12 dB band must lower a centered sine's RMS"
        );
    }

    // A preview decoded at 22.05 kHz has a Nyquist of 11.025 kHz, below the
    // 12, 14, and 16 kHz bands. Building peaking sections there puts their
    // poles outside the unit circle, so the output diverges. Those bands must
    // become pass-throughs instead, leaving the rest of the curve intact.
    #[test]
    fn band_filter_skips_bands_at_or_above_nyquist() {
        let mut bands = [0.0; BAND_COUNT];
        bands[7] = 12.0;
        bands[8] = 12.0;
        bands[9] = 12.0;
        let mut filter = BandFilter::new(22_050);
        filter.set_curve(true, 0.0, &bands);

        for sample in [0.0, 0.5, -0.25, 1.0, -1.0] {
            let output = filter.process(sample);
            assert!(output.is_finite(), "unstable filter produced {output}");
            assert!((output - sample).abs() < 1e-6);
        }
    }

    #[test]
    fn band_filter_preamp_is_a_flat_gain() {
        let mut filter = BandFilter::new(44_100);
        filter.set_curve(true, 6.020_6, &[0.0; BAND_COUNT]);

        assert!((filter.process(0.25) - 0.5).abs() < 1e-4);
        assert!((filter.process(-0.25) + 0.5).abs() < 1e-4);
    }

    fn gain_curve(band: usize, gain: f32) -> [f32; BAND_COUNT] {
        let mut bands = [0.0; BAND_COUNT];
        bands[band] = gain;
        bands
    }

    /// The RMS of `frequency`'s sine after the filter has settled, skipping the
    /// first half of the samples so the transient does not dominate.
    fn settled_sine_rms(filter: &mut BandFilter, frequency: f32) -> f32 {
        const SAMPLE_RATE: f32 = 44_100.0;
        const TOTAL: usize = 16_384;
        const SETTLE: usize = TOTAL / 2;
        let mut sum = 0.0;
        let mut count = 0;
        for index in 0..TOTAL {
            let sample =
                (2.0 * std::f32::consts::PI * frequency * index as f32 / SAMPLE_RATE).sin();
            let output = filter.process(sample);
            if index >= SETTLE {
                sum += output * output;
                count += 1;
            }
        }
        (sum / count as f32).sqrt()
    }
}
