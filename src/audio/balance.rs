//! The stereo-balance `Source` adapter.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use rodio::source::SeekError;
use rodio::{Sample, Source};

use super::delegate::{forward_size_hint, forward_source_reads};

/// A [`Source`] adapter that applies a live stereo balance to its samples.
///
/// The balance is read from shared state on every sample, so a slider drag
/// changes the current preview without rebuilding the source. A mono source
/// is attenuated by `1.0 - balance.abs()`; for a multi-channel source the even
/// channel indices stay at unity while the balance leans left, and the odd
/// indices stay at unity while it leans right, so `-1.0` silences the right
/// channel, `+1.0` the left, and `0.0` is unity. This does not use rodio's
/// `ChannelVolume` or `Spatial`, which downmix the input to mono and would
/// collapse the preview's stereo image.
pub(super) struct BalanceSource<S> {
    inner: S,
    balance: Arc<AtomicU32>,
    /// The channel index of the next sample, advanced modulo the source's
    /// channel count.
    pub(super) channel: usize,
}

impl<S> BalanceSource<S> {
    /// Wraps `inner`, applying the live stereo balance stored in `balance`.
    pub(super) fn new(inner: S, balance: Arc<AtomicU32>) -> Self {
        Self {
            inner,
            balance,
            channel: 0,
        }
    }
}

impl<S: Source> Iterator for BalanceSource<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
        let channels = self.inner.channels().get() as usize;
        let balance = f32::from_bits(self.balance.load(Ordering::Relaxed));
        let gain = if channels == 1 {
            1.0 - balance.abs()
        } else if self.channel.is_multiple_of(2) {
            1.0 - balance.max(0.0)
        } else {
            1.0 + balance.min(0.0)
        };
        self.channel = (self.channel + 1) % channels;
        Some(sample * gain)
    }

    forward_size_hint!();
}

impl<S: Source> Source for BalanceSource<S> {
    forward_source_reads!();

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        // rodio's seek implementations leave the next sample on the same
        // channel as before the seek (see `SamplesBuffer::try_seek` and
        // `SymphoniaDecoder::try_seek`), so this wrapper's cursor must stay
        // in that phase rather than resetting to zero.
        self.inner.try_seek(pos)
    }
}
