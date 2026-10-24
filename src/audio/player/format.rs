//! Playback values read from a decoded source: its sample rate, channel
//! count, average encoded bitrate, and own length.

use std::time::Duration;

use super::super::PlaybackFormat;

/// Reads `source`'s sample rate and channel count into a [`PlaybackFormat`],
/// the format the Now Playing LCD's status row shows, and computes its average
/// bitrate from `encoded_bytes` and the source's own length when it reports
/// one.
pub(in crate::audio) fn format_for_source(
    source: &impl rodio::Source,
    encoded_bytes: u64,
) -> PlaybackFormat {
    PlaybackFormat {
        sample_rate_hz: source.sample_rate().get(),
        channels: source.channels().get(),
        bitrate_kbps: source
            .total_duration()
            .and_then(|duration| preview_bitrate_kbps(encoded_bytes, duration)),
    }
}

/// Computes the average encoded bitrate in kilobits per second for an
/// `encoded_bytes`-byte body spanning `duration`.
///
/// One bit per millisecond is one kilobit per second, so the byte count is
/// multiplied by eight and divided by the duration in milliseconds, rounded to
/// the nearest kilobit. Returns `None` when the duration is zero (nothing to
/// divide by) or the result does not fit a `u32`.
pub(in crate::audio) fn preview_bitrate_kbps(
    encoded_bytes: u64,
    duration: Duration,
) -> Option<u32> {
    let millis = duration.as_millis();
    if millis == 0 {
        return None;
    }
    let bits = u128::from(encoded_bytes) * 8;
    let kbps = (bits + millis / 2) / millis;
    u32::try_from(kbps).ok()
}

/// Reads `source`'s own length, the span the seek bar draws.
pub(in crate::audio) fn duration_for_source(source: &impl rodio::Source) -> Option<Duration> {
    source.total_duration()
}
