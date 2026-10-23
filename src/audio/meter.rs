//! The preview spectrum meter: a mono ring of the samples the worker pulls,
//! a pure ten-band Goertzel analysis, and the source adapter that feeds it.
//!
//! The LCD's visualizer draws the band amplitudes this module reports. The
//! meter is observational only: [`MeterSource`] hands every
//! sample to its inner source unchanged, so the equalizer, balance, and
//! output hear exactly what they would without it.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rodio::source::SeekError;
use rodio::{Sample, Source};

use crate::equalizer::{BAND_COUNT, BAND_HZ};

/// The number of mono samples the meter analyzes, about 46 ms at 44.1 kHz.
///
/// A fixed window keeps the analysis length (and so the Goertzel cost) bounded
/// no matter how long the preview runs, and the ring only analyzes the newest
/// audio, so the bars follow the live signal rather than the whole track.
pub const SPECTRUM_WINDOW: usize = 2048;

/// The loaded preview's live spectrum: one normalized amplitude per
/// [`crate::equalizer::BAND_HZ`] band, low to high.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spectrum {
    /// The per-band amplitudes, each in `0.0..=1.0` and in
    /// [`crate::equalizer::BAND_HZ`] order.
    pub bands: [f32; BAND_COUNT],
}

/// The shared window the audio thread writes and the UI poll reads.
///
/// The ring is behind a `Mutex` and the sample rate in an atomic: the audio
/// thread pushes one sample at a time, while a query copies the ring under
/// the lock and runs the Goertzel analysis after releasing it. A poisoned
/// lock is recovered rather than panicking, matching the rest of the audio
/// seam.
#[derive(Debug)]
pub(super) struct SpectrumMeter {
    window: Mutex<SpectrumWindow>,
    sample_rate: AtomicU32,
}

impl Default for SpectrumMeter {
    fn default() -> Self {
        Self {
            window: Mutex::new(SpectrumWindow::default()),
            sample_rate: AtomicU32::new(0),
        }
    }
}

impl SpectrumMeter {
    /// Records the rate the next [`spectrum_bands`] calls analyze at.
    pub(super) fn set_sample_rate(&self, sample_rate: u32) {
        self.sample_rate.store(sample_rate, Ordering::Relaxed);
    }

    /// Adds one interleaved sample, downmixing every complete frame to mono
    /// and overwriting the oldest mono sample once the ring is full.
    pub(super) fn push(&self, sample: f32, channels: u16) {
        if channels == 0 {
            return;
        }
        self.window
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(sample, channels);
    }

    /// Copies the window in chronological order, then analyzes it outside the
    /// lock so the audio thread never waits on the Goertzel work.
    pub(super) fn take(&self) -> Spectrum {
        let sample_rate = self.sample_rate.load(Ordering::Relaxed);
        let samples = self
            .window
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .chronological();
        Spectrum {
            bands: spectrum_bands(&samples, sample_rate),
        }
    }

    /// Empties the ring, so audio from before a seek is not analyzed with
    /// the position the seek landed on, while keeping the current interleaved
    /// frame's partial downmix: rodio leaves the next sample on the same
    /// channel after a seek, so resetting it would shift the frame's phase.
    pub(super) fn clear(&self) {
        self.window
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    /// Copies the recorded mono window in chronological order, for tests.
    #[cfg(test)]
    pub(super) fn recorded(&self) -> Vec<f32> {
        self.window
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .chronological()
    }
}

/// The ring plus the partial frame being downmixed into the next mono sample.
#[derive(Debug)]
struct SpectrumWindow {
    /// The mono samples, oldest overwritten once `len` reaches its capacity.
    samples: [f32; SPECTRUM_WINDOW],
    /// The slot the next mono sample overwrites.
    next: usize,
    /// How many slots hold audio: the window length once the ring has wrapped.
    len: usize,
    /// The running sum of the current interleaved frame's channels.
    downmix_sum: f32,
    /// How many of the current frame's channels have been summed.
    downmix_len: u16,
}

impl Default for SpectrumWindow {
    fn default() -> Self {
        Self {
            samples: [0.0; SPECTRUM_WINDOW],
            next: 0,
            len: 0,
            downmix_sum: 0.0,
            downmix_len: 0,
        }
    }
}

impl SpectrumWindow {
    fn push(&mut self, sample: f32, channels: u16) {
        self.downmix_sum += sample;
        self.downmix_len += 1;
        if self.downmix_len < channels {
            return;
        }
        let mono = self.downmix_sum / f32::from(channels);
        self.samples[self.next] = mono;
        self.next = (self.next + 1) % SPECTRUM_WINDOW;
        self.len = (self.len + 1).min(SPECTRUM_WINDOW);
        self.downmix_sum = 0.0;
        self.downmix_len = 0;
    }

    fn chronological(&self) -> Vec<f32> {
        let mut ordered = Vec::with_capacity(self.len);
        if self.len < SPECTRUM_WINDOW {
            ordered.extend_from_slice(&self.samples[..self.len]);
        } else {
            ordered.extend_from_slice(&self.samples[self.next..]);
            ordered.extend_from_slice(&self.samples[..self.next]);
        }
        ordered
    }

    fn clear(&mut self) {
        self.next = 0;
        self.len = 0;
        // Keep `downmix_sum`/`downmix_len`: rodio's seek leaves the next
        // sample on the same channel, so the partial frame stays in phase.
    }
}

/// A [`Source`] adapter that records every sample into `meter` and returns it
/// unchanged, so the spectrum reflects the audio the worker actually pulls.
pub(super) struct MeterSource<S> {
    inner: S,
    meter: Arc<SpectrumMeter>,
}

impl<S: Source> MeterSource<S> {
    /// Wraps `inner`, recording its sample rate for the analysis.
    pub(super) fn new(inner: S, meter: Arc<SpectrumMeter>) -> Self {
        meter.set_sample_rate(inner.sample_rate().get());
        Self { inner, meter }
    }
}

impl<S: Source> Iterator for MeterSource<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
        self.meter.push(sample, self.inner.channels().get());
        Some(sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<S: Source> Source for MeterSource<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.inner.try_seek(pos)?;
        self.meter.clear();
        Ok(())
    }
}

/// Analyzes a mono window into one normalized amplitude per
/// [`crate::equalizer::BAND_HZ`] band.
///
/// Each band runs a Goertzel magnitude at its center frequency and normalizes
/// a full-scale sine to about `1.0` (`2 * magnitude / window length`). Bands
/// at or above Nyquist have no representable tone and read zero, as does
/// silence and an empty window. Pure, so the frequency mapping is testable
/// without an audio device.
pub(super) fn spectrum_bands(samples: &[f32], sample_rate: u32) -> [f32; BAND_COUNT] {
    let mut bands = [0.0; BAND_COUNT];
    if samples.is_empty() || sample_rate == 0 {
        return bands;
    }
    let nyquist = sample_rate as f32 / 2.0;
    for (band, frequency) in bands.iter_mut().zip(BAND_HZ) {
        if frequency >= nyquist {
            continue;
        }
        *band = goertzel_amplitude(samples, frequency, sample_rate).clamp(0.0, 1.0);
    }
    bands
}

/// The normalized Goertzel magnitude of `frequency` over `samples`.
fn goertzel_amplitude(samples: &[f32], frequency: f32, sample_rate: u32) -> f32 {
    let omega = 2.0 * std::f32::consts::PI * frequency / sample_rate as f32;
    let coefficient = 2.0 * omega.cos();
    let mut previous = 0.0_f32;
    let mut before_previous = 0.0_f32;
    for &sample in samples {
        let current = coefficient * previous - before_previous + sample;
        before_previous = previous;
        previous = current;
    }
    let magnitude = (previous * previous + before_previous * before_previous
        - coefficient * previous * before_previous)
        .max(0.0)
        .sqrt();
    2.0 * magnitude / samples.len() as f32
}
