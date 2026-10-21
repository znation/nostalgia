//! The equalizer `Source` adapter and the live curve it reads.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

use rodio::source::SeekError;
use rodio::{ChannelCount, Sample, SampleRate, Source};

use crate::equalizer::{BAND_COUNT, BandFilter};

/// The equalizer curve the output should apply to a preview.
///
/// This is a plain copyable snapshot of `AppState`'s equalizer fields so the
/// audio seam can carry a curve across its worker channel without depending
/// on the state module.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EqSnapshot {
    /// Whether the equalizer is switched on.
    pub enabled: bool,
    /// The preamp gain in decibels.
    pub preamp_db: f32,
    /// The ten band gains in decibels, low frequency to high.
    pub bands_db: [f32; BAND_COUNT],
}

/// The live equalizer curve shared between the production player and the
/// [`EqSource`]s wrapping its previews.
///
/// The fields are atomics so the worker can store a new curve while the audio
/// thread is reading the current one. `version` changes on every store; a
/// source rebuilds its coefficients only when it sees a new version.
pub(super) struct EqSettings {
    enabled: AtomicBool,
    preamp_db: AtomicU32,
    bands_db: [AtomicU32; BAND_COUNT],
    version: AtomicU64,
}

impl Default for EqSettings {
    fn default() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            preamp_db: AtomicU32::new(0.0_f32.to_bits()),
            bands_db: std::array::from_fn(|_| AtomicU32::new(0.0_f32.to_bits())),
            version: AtomicU64::new(0),
        }
    }
}

impl EqSettings {
    /// Stores `snapshot` and bumps the version so a running source rebuilds
    /// its coefficients before filtering the next sample.
    pub(super) fn store(&self, snapshot: EqSnapshot) {
        self.enabled.store(snapshot.enabled, Ordering::Relaxed);
        self.preamp_db
            .store(snapshot.preamp_db.to_bits(), Ordering::Relaxed);
        for (slot, gain) in self.bands_db.iter().zip(snapshot.bands_db) {
            slot.store(gain.to_bits(), Ordering::Relaxed);
        }
        self.version.fetch_add(1, Ordering::Release);
    }

    /// Reads the current curve. The caller pairs this with [`Self::version`]
    /// to know whether it is new.
    fn snapshot(&self) -> EqSnapshot {
        EqSnapshot {
            enabled: self.enabled.load(Ordering::Relaxed),
            preamp_db: f32::from_bits(self.preamp_db.load(Ordering::Relaxed)),
            bands_db: std::array::from_fn(|index| {
                f32::from_bits(self.bands_db[index].load(Ordering::Relaxed))
            }),
        }
    }

    /// The current curve's version, loaded with the acquire half of the
    /// store's release, so the fields read after a changed version are the
    /// ones that version wrote.
    fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }
}

/// A [`Source`] adapter that applies a live equalizer curve to its samples.
///
/// Each channel gets its own [`BandFilter`], so stereo channels keep separate
/// filter state. The curve is read from shared [`EqSettings`] on every
/// sample; when its version changes the filters are rebuilt, so a slider move
/// changes the preview already playing without restarting it. This sits
/// inside the [`super::balance::BalanceSource`] wrapper, so equalization runs
/// before the stereo balance.
pub(super) struct EqSource<S> {
    inner: S,
    settings: Arc<EqSettings>,
    /// The settings version the current coefficients were built from.
    last_version: u64,
    filters: Vec<BandFilter>,
    /// The channel index of the next sample, advanced modulo the source's
    /// channel count.
    pub(super) channel: usize,
}

impl<S: Source> EqSource<S> {
    /// Wraps `inner` with the curve in `settings`, building one [`BandFilter`]
    /// per output channel and applying the current curve.
    pub(super) fn new(inner: S, settings: Arc<EqSettings>) -> Self {
        let channels = inner.channels().get() as usize;
        let sample_rate = inner.sample_rate().get();
        let last_version = settings.version();
        let mut source = Self {
            inner,
            last_version,
            settings,
            filters: (0..channels)
                .map(|_| BandFilter::new(sample_rate))
                .collect(),
            channel: 0,
        };
        source.rebuild_filters();
        source
    }

    /// Rebuilds every channel filter from the current curve, clearing their
    /// state.
    fn rebuild_filters(&mut self) {
        let snapshot = self.settings.snapshot();
        for filter in &mut self.filters {
            filter.set_curve(snapshot.enabled, snapshot.preamp_db, &snapshot.bands_db);
        }
    }
}

impl<S: Source> Iterator for EqSource<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
        let version = self.settings.version();
        if version != self.last_version {
            self.last_version = version;
            self.rebuild_filters();
        }
        let channels = self.inner.channels().get() as usize;
        let filtered = self
            .filters
            .get_mut(self.channel)
            .map_or(sample, |filter| filter.process(sample));
        self.channel = (self.channel + 1) % channels;
        Some(filtered)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<S: Source> Source for EqSource<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.inner.try_seek(pos)?;
        // rodio's seek leaves the next sample on the same channel, so keep
        // `channel` in phase; only the filter state is cleared, so a seek
        // does not replay ringing from the old position.
        self.rebuild_filters();
        Ok(())
    }
}
