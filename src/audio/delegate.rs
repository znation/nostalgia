//! Shared [`rodio::Source`] forwarding for the audio sample adapters.

/// Forwards the [`rodio::Source`] methods the sample adapters leave unchanged.
///
/// [`super::balance::BalanceSource`], [`super::equalizer::EqSource`], and
/// [`super::meter::MeterSource`] each wrap an inner source and only change its
/// samples or its seek follow-up, so their four read-only `Source` methods are
/// identical. `try_seek` is deliberately not forwarded here: each adapter does
/// its own follow-up work after the inner seek (keep the channel cursor,
/// rebuild the filter state, or clear the meter).
macro_rules! forward_source_reads {
    () => {
        fn current_span_len(&self) -> Option<usize> {
            self.inner.current_span_len()
        }

        fn channels(&self) -> rodio::ChannelCount {
            self.inner.channels()
        }

        fn sample_rate(&self) -> rodio::SampleRate {
            self.inner.sample_rate()
        }

        fn total_duration(&self) -> Option<std::time::Duration> {
            self.inner.total_duration()
        }
    };
}

/// Forwards [`Iterator::size_hint`] to the wrapped source.
///
/// The three sample adapters neither buffer nor filter their items, so the
/// inner source's hint is exact for them; each `Iterator` impl calls this
/// beside its own `next`.
macro_rules! forward_size_hint {
    () => {
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
    };
}

pub(super) use forward_size_hint;
pub(super) use forward_source_reads;
