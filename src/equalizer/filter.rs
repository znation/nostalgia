//! The equalizer's signal processing: [`BandFilter`], one channel's ten
//! peaking biquads plus the preamp. The band centers it reads live in the
//! parent module's data model.

use super::{BAND_COUNT, BAND_HZ};

/// The Q (bandwidth) of every peaking filter: roughly one octave, the width
/// Winamp's ten-band equalizer uses.
const BAND_Q: f32 = 1.41;

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
    use super::{BAND_COUNT, BandFilter};

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
