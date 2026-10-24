//! Tests for the audio seam, its source adapters, the worker loop, and the
//! guarded preview download.

use super::balance::BalanceSource;
use super::equalizer::{EqSettings, EqSource};
use super::meter::{MeterSource, SPECTRUM_WINDOW, SpectrumMeter, spectrum_bands};
use super::player::format::{duration_for_source, format_for_source, preview_bitrate_kbps};
use super::player::rodio_player::RodioPlayer;
use super::player::seek::{SeekablePlayer, clamp_seek, seek_player, seek_target};
use super::player::worker::{Command, WorkerPlayer, await_device_ready, serve_commands};
use super::player::{DEVICE_OPEN_TIMEOUT, WORKER_QUERY_TIMEOUT};
use super::preview::{
    MAX_PREVIEW_BYTES, PREVIEW_TIMEOUT, PublicAddressResolver, download_and_decode, preview_agent,
    preview_agent_with_timeout,
};
use super::*;
use crate::equalizer::BAND_COUNT;
use crate::http::{agent_with_timeout, config_with_timeout};
use crate::test_support::{
    AudioCall, PREVIEW_URL, RecordingAudio, gzip_empty_stored_blocks, gzip_zeros,
    loopback_listener, read_some_request, record_first_request, serve_one_response,
    serve_one_response_bytes,
};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rodio::source::SeekError;
use rodio::{ChannelCount, Sample, SampleRate, Source};
use ureq::unversioned::resolver::Resolver;
use ureq::unversioned::transport::NextTimeout;

/// Runs every fallible [`AudioOutput`] command once, paired with the method
/// name for failure messages, so the "every command" tests cover the same
/// seam methods and cannot drift apart as the trait grows.
fn every_command(output: &dyn AudioOutput) -> Vec<(&'static str, Result<(), AppleMusicError>)> {
    vec![
        ("play", output.play(PREVIEW_URL)),
        ("pause", output.pause()),
        ("resume", output.resume()),
        ("stop", output.stop()),
        ("set_volume", output.set_volume(0.4)),
        ("set_balance", output.set_balance(-0.4)),
        ("seek_by", output.seek_by(-5)),
        ("seek_to", output.seek_to(Duration::from_secs(10))),
        ("set_equalizer", output.set_equalizer(EqSnapshot::default())),
    ]
}

#[test]
fn silent_output_reports_success_for_every_command() {
    for (command, result) in every_command(&SilentOutput) {
        assert!(result.is_ok(), "SilentOutput::{command} must succeed");
    }
}

// The silent fallback cannot report a position: the trait's default answers
// `None`, which the Now Playing readout treats as "no elapsed value" and
// falls back to the plain track length. `Some(Duration::ZERO)` would instead
// show a frozen `0:00` beside the total on a machine with no audio device.
// Pin both the direct `SilentOutput` and the fallback object
// `audio_with_fallback` returns, since that is what the service holds when
// the device open fails.
#[test]
fn silent_output_reports_no_playback_position() {
    assert_eq!(SilentOutput.position(), None);

    let output = audio_with_fallback(|| Err(AppleMusicError::new("no device")));
    assert_eq!(output.position(), None);
}

// The silent fallback also has no preview format to report, so the LCD's
// status row omits the prefix and leaves both channel labels dim rather than
// inventing a sample rate.
#[test]
fn silent_output_reports_no_playback_format() {
    assert_eq!(SilentOutput.playback_format(), None);

    let output = audio_with_fallback(|| Err(AppleMusicError::new("no device")));
    assert_eq!(output.playback_format(), None);
}

// The silent fallback has no samples to analyze, so the visualizer keeps every
// bar at zero rather than inventing a spectrum.
#[test]
fn silent_output_reports_no_spectrum() {
    assert_eq!(SilentOutput.spectrum(), None);

    let output = audio_with_fallback(|| Err(AppleMusicError::new("no device")));
    assert_eq!(output.spectrum(), None);
}

// The silent fallback plays nothing to the end, so the UI must never advance
// the transport off it.
#[test]
fn silent_output_reports_an_unfinished_preview() {
    assert!(!SilentOutput.is_finished());

    let output = audio_with_fallback(|| Err(AppleMusicError::new("no device")));
    assert!(!output.is_finished());
}

/// Builds a full-scale sine at `frequency`, `len` samples long.
fn sine_samples(frequency: f32, sample_rate: u32, len: usize) -> Vec<f32> {
    (0..len)
        .map(|index| (std::f32::consts::TAU * frequency * index as f32 / sample_rate as f32).sin())
        .collect()
}

// A 1 kHz tone must win the band centered on 1 kHz, with every other band
// lower, and silence must read all zeros.
#[test]
fn spectrum_bands_peak_at_the_matching_band_for_a_sine() {
    let bands = spectrum_bands(&sine_samples(1_000.0, 44_100, SPECTRUM_WINDOW), 44_100);
    let peak = bands[4];
    assert!(peak > 0.9, "the 1K band must read the tone, got {peak}");
    for (index, band) in bands.iter().enumerate() {
        if index != 4 {
            assert!(
                *band < peak,
                "band {index} ({band}) must stay below the 1K band ({peak})"
            );
        }
    }

    assert_eq!(
        spectrum_bands(&[0.0; SPECTRUM_WINDOW], 44_100),
        [0.0; BAND_COUNT]
    );
}

// A band at or above Nyquist has no representable tone, so it reads zero even
// while a lower tone the source does carry is still detected.
#[test]
fn spectrum_bands_zero_a_band_at_or_above_nyquist() {
    for sample_rate in [22_050, 32_000] {
        let bands = spectrum_bands(
            &sine_samples(1_000.0, sample_rate, SPECTRUM_WINDOW),
            sample_rate,
        );
        assert_eq!(
            bands[9], 0.0,
            "the 16K band is at or above Nyquist at {sample_rate} Hz"
        );
        assert!(
            bands[4] > 0.0,
            "the 1K band must still detect a tone below Nyquist at {sample_rate} Hz"
        );
    }
}

// The meter adapter must not alter the audio: every sample it records is also
// returned, it delegates its source's format and length, and a seek clears the
// window so pre-seek audio is not analyzed at the new position.
#[test]
fn meter_source_passes_samples_through_and_records_their_spectrum() {
    let sine = sine_samples(1_000.0, 44_100, SPECTRUM_WINDOW);
    let buffer = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(1).unwrap(),
        SampleRate::new(44_100).unwrap(),
        sine.clone(),
    );
    let expected_channels = buffer.channels().get();
    let expected_rate = buffer.sample_rate().get();
    let expected_duration = buffer.total_duration();
    let expected_hint = buffer.size_hint();
    let meter = Arc::new(SpectrumMeter::default());
    let mut source = MeterSource::new(buffer, Arc::clone(&meter));

    assert_eq!(source.channels().get(), expected_channels);
    assert_eq!(source.sample_rate().get(), expected_rate);
    assert_eq!(source.total_duration(), expected_duration);
    assert_eq!(source.size_hint(), expected_hint);

    let pulled: Vec<f32> = source.by_ref().collect();
    assert_eq!(pulled, sine, "the meter must return every sample unchanged");
    assert!(
        meter.take().bands[4] > 0.9,
        "the recorded samples must peak at the sine's band"
    );

    source.try_seek(Duration::ZERO).unwrap();
    assert_eq!(
        meter.take().bands,
        [0.0; BAND_COUNT],
        "a seek must clear the recorded window"
    );
}

// The meter ring keeps only the newest `SPECTRUM_WINDOW` mono samples, in
// play order: once more samples arrive than fit, the oldest are overwritten
// and `chronological` must start at the first sample still held rather than
// serve the ring's storage order. `spectrum_bands` analyzes this order, so a
// full ring must follow the live signal instead of a rotated window.
#[test]
fn meter_window_keeps_the_newest_samples_in_play_order() {
    let total = SPECTRUM_WINDOW + 5;
    let samples: Vec<f32> = (0..total).map(|index| index as f32).collect();
    let buffer = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(1).unwrap(),
        SampleRate::new(44_100).unwrap(),
        samples.clone(),
    );
    let meter = Arc::new(SpectrumMeter::default());
    let mut source = MeterSource::new(buffer, Arc::clone(&meter));

    assert_eq!(source.by_ref().count(), total);

    assert_eq!(
        meter.recorded(),
        samples[total - SPECTRUM_WINDOW..].to_vec(),
        "a full ring must serve the newest window in play order"
    );
}

// A seek must not reset the partial downmix frame: rodio leaves the next
// sample on the same channel as before the seek, so the frame spanning the
// seek stays in phase instead of pairing a right sample with the next left.
#[test]
fn meter_source_seek_keeps_the_downmix_phase() {
    let samples = vec![0.5, 0.25, 0.9, 0.1];
    let meter = Arc::new(SpectrumMeter::default());
    let mut source = MeterSource::new(SeekableSource::new(samples), Arc::clone(&meter));

    assert_eq!(source.next(), Some(0.5), "the first sample opens a frame");
    source.try_seek(Duration::ZERO).unwrap();
    assert_eq!(
        source.by_ref().collect::<Vec<_>>(),
        vec![0.25, 0.9, 0.1],
        "the seek must leave the next sample on the right channel"
    );
    assert_eq!(
        meter.recorded(),
        vec![0.375, 0.5],
        "the partial frame must stay in phase across the seek"
    );
}

// The status row's values come from the decoded source itself: a mono and a
// stereo preview must report their own channel counts and sample rates, and a
// source whose length is known also reports the bitrate computed from the
// encoded byte count.
#[test]
fn format_for_source_reads_sample_rate_and_channels() {
    let mono = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(1).unwrap(),
        SampleRate::new(44_100).unwrap(),
        vec![0.0_f32; 4],
    );
    assert_eq!(
        format_for_source(&mono, 0),
        PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 1,
            bitrate_kbps: None,
        }
    );

    let stereo = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(2).unwrap(),
        SampleRate::new(48_000).unwrap(),
        vec![0.0_f32; 4],
    );
    assert_eq!(
        format_for_source(&stereo, 0),
        PlaybackFormat {
            sample_rate_hz: 48_000,
            channels: 2,
            bitrate_kbps: None,
        }
    );

    // Two hertz, two channels, and 120 samples last 30 seconds; 960,000
    // encoded bytes over that span is 256 kbit/s.
    let known_length = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(2).unwrap(),
        SampleRate::new(2).unwrap(),
        vec![0.0_f32; 120],
    );
    assert_eq!(
        format_for_source(&known_length, 960_000).bitrate_kbps,
        Some(256)
    );
}

// 960,000 bytes is 7,680,000 bits; over 30 seconds that is 256 kbit/s, while
// a zero-length preview has no span to divide by.
#[test]
fn preview_bitrate_kbps_computes_encoded_bits_over_the_preview_length() {
    assert_eq!(
        preview_bitrate_kbps(960_000, Duration::from_secs(30)),
        Some(256)
    );
    assert_eq!(preview_bitrate_kbps(960_000, Duration::ZERO), None);
}

// The silent fallback has no preview length to report, so the seek bar stays
// blank rather than inventing a scale.
#[test]
fn silent_output_reports_no_playback_duration() {
    assert_eq!(SilentOutput.duration(), None);

    let output = audio_with_fallback(|| Err(AppleMusicError::new("no device")));
    assert_eq!(output.duration(), None);
}

// The seek bar's scale is the decoded source's own length, which a
// `SamplesBuffer` reports from its sample count, channel count, and rate.
#[test]
fn duration_for_source_reads_a_samples_buffers_length() {
    let source = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(2).unwrap(),
        SampleRate::new(2).unwrap(),
        vec![0.0_f32; 6],
    );

    assert_eq!(
        duration_for_source(&source),
        Some(Duration::from_millis(1500))
    );
}

// An absolute seek is clamped to the loaded preview's own length, and passes
// through unchanged when the backend reports no length to clamp against.
#[test]
fn clamp_seek_clamps_to_the_preview_length_and_passes_through_without_one() {
    assert_eq!(
        clamp_seek(Duration::from_secs(45), Some(Duration::from_secs(30))),
        Duration::from_secs(30)
    );
    assert_eq!(
        clamp_seek(Duration::from_secs(45), None),
        Duration::from_secs(45)
    );
}

#[test]
fn seek_target_moves_forward_and_backward() {
    assert_eq!(
        seek_target(Duration::from_secs(10), 5),
        Duration::from_secs(15)
    );
    assert_eq!(
        seek_target(Duration::from_secs(10), -5),
        Duration::from_secs(5)
    );
}

#[test]
fn seek_target_saturates_at_the_start() {
    assert_eq!(seek_target(Duration::from_secs(3), -5), Duration::ZERO);
}

// A preview's position carries its sub-second part (rodio reports the
// decoder's exact position), so a five-second step must move by exactly
// five seconds rather than snapping to or dropping the whole-second part.
#[test]
fn seek_target_preserves_the_sub_second_position() {
    let position = Duration::from_millis(10_250);
    assert_eq!(seek_target(position, 5), Duration::from_millis(15_250));
    assert_eq!(seek_target(position, -5), Duration::from_millis(5_250));
}

// The forward twin of `seek_target_saturates_at_the_start`: the doc comment
// promises saturation at `Duration::MAX` instead of overflowing, so a
// position close enough to the end to overflow stays there.
#[test]
fn seek_target_saturates_at_the_end() {
    assert_eq!(
        seek_target(Duration::MAX - Duration::from_secs(1), 5),
        Duration::MAX
    );
}

// `i64::MIN` cannot be negated in an `i64`, so the offset's magnitude is
// taken with `unsigned_abs`; a regression to `-(offset)` would panic in
// debug builds on this boundary instead of seeking to the start.
#[test]
fn seek_target_handles_the_largest_negative_offset() {
    assert_eq!(
        seek_target(Duration::from_secs(10), i64::MIN),
        Duration::ZERO
    );
}

/// A [`SeekablePlayer`] that reports a fixed position and records the
/// absolute target each seek asks for, so a test can pin the composition
/// in [`seek_player`] without a `rodio::Player` or an output device.
#[derive(Default)]
struct RecordingSeekPlayer {
    position: Duration,
    targets: Mutex<Vec<Duration>>,
}

impl SeekablePlayer for RecordingSeekPlayer {
    fn position(&self) -> Duration {
        self.position
    }

    fn seek_to(&self, position: Duration) -> Result<(), SeekError> {
        self.targets.lock().unwrap().push(position);
        Ok(())
    }
}

#[test]
fn seek_player_seeks_relative_to_the_players_position() {
    let player = RecordingSeekPlayer {
        position: Duration::from_secs(10),
        ..Default::default()
    };

    seek_player(&player, -5);
    seek_player(&player, 5);

    assert_eq!(
        *player.targets.lock().unwrap(),
        vec![Duration::from_secs(5), Duration::from_secs(15)]
    );
}

#[test]
fn balance_source_centered_leaves_samples_unchanged() {
    let channels = ChannelCount::new(2).unwrap();
    let rate = SampleRate::new(44_100).unwrap();
    let raw = rodio::buffer::SamplesBuffer::new(channels, rate, vec![0.5_f32, -0.25]);
    let balance = Arc::new(AtomicU32::new(0.0_f32.to_bits()));
    let mut source = BalanceSource::new(raw, balance);

    assert_eq!(source.channels(), channels);
    assert_eq!(source.sample_rate(), rate);
    assert_eq!(source.current_span_len(), Some(2));
    assert!(source.total_duration().is_some());
    assert_eq!(source.by_ref().collect::<Vec<_>>(), vec![0.5, -0.25]);
}

#[test]
fn balance_source_hard_left_silences_the_right_channel() {
    let raw = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(2).unwrap(),
        SampleRate::new(44_100).unwrap(),
        vec![0.5_f32, 0.25],
    );
    let balance = Arc::new(AtomicU32::new((-1.0_f32).to_bits()));
    let source = BalanceSource::new(raw, balance);

    assert_eq!(source.collect::<Vec<_>>(), vec![0.5, 0.0]);
}

#[test]
fn balance_source_hard_right_silences_the_left_channel() {
    let raw = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(2).unwrap(),
        SampleRate::new(44_100).unwrap(),
        vec![0.5_f32, 0.25],
    );
    let balance = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
    let source = BalanceSource::new(raw, balance);

    assert_eq!(source.collect::<Vec<_>>(), vec![0.0, 0.25]);
}

#[test]
fn balance_source_partial_right_attenuates_the_left_channel() {
    let raw = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(2).unwrap(),
        SampleRate::new(44_100).unwrap(),
        vec![0.5_f32, 0.25],
    );
    let balance = Arc::new(AtomicU32::new(0.5_f32.to_bits()));
    let source = BalanceSource::new(raw, balance);

    assert_eq!(source.collect::<Vec<_>>(), vec![0.25, 0.25]);
}

#[test]
fn balance_source_partial_left_attenuates_the_right_channel() {
    let raw = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(2).unwrap(),
        SampleRate::new(44_100).unwrap(),
        vec![0.5_f32, 0.25],
    );
    let balance = Arc::new(AtomicU32::new((-0.5_f32).to_bits()));
    let source = BalanceSource::new(raw, balance);

    assert_eq!(source.collect::<Vec<_>>(), vec![0.5, 0.125]);
}

#[test]
fn balance_source_attenuates_a_mono_source() {
    let raw = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(1).unwrap(),
        SampleRate::new(44_100).unwrap(),
        vec![0.5_f32, -0.5],
    );
    let balance = Arc::new(AtomicU32::new(0.5_f32.to_bits()));
    let source = BalanceSource::new(raw, balance);

    assert_eq!(source.collect::<Vec<_>>(), vec![0.25, -0.25]);
}

#[test]
fn balance_source_reads_the_shared_balance_on_every_sample() {
    let raw = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(2).unwrap(),
        SampleRate::new(44_100).unwrap(),
        vec![1.0_f32; 4],
    );
    let balance = Arc::new(AtomicU32::new(0.0_f32.to_bits()));
    let mut source = BalanceSource::new(raw, Arc::clone(&balance));

    assert_eq!(source.next(), Some(1.0));
    balance.store((-1.0_f32).to_bits(), Ordering::Relaxed);
    assert_eq!(source.next(), Some(0.0));
    assert_eq!(source.next(), Some(1.0));
    assert_eq!(source.next(), Some(0.0));
}

fn stereo_source(samples: Vec<f32>) -> rodio::buffer::SamplesBuffer {
    rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(2).unwrap(),
        SampleRate::new(44_100).unwrap(),
        samples,
    )
}

/// A stereo source whose `try_seek` rewinds cleanly from any sample
/// offset. `SamplesBuffer` subtracts the current channel from the target
/// position, so seeking it to zero after an odd number of samples would
/// underflow; this source lets a test leave the channel cursor mid-frame.
struct SeekableSource {
    samples: Vec<f32>,
    position: usize,
}

impl SeekableSource {
    fn new(samples: Vec<f32>) -> Self {
        Self {
            samples,
            position: 0,
        }
    }
}

impl Iterator for SeekableSource {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = *self.samples.get(self.position)?;
        self.position += 1;
        Some(sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.samples.len() - self.position;
        (remaining, Some(remaining))
    }
}

impl Source for SeekableSource {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.samples.len())
    }

    fn channels(&self) -> ChannelCount {
        ChannelCount::new(2).unwrap()
    }

    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(44_100).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            self.samples.len() as f64 / (2.0 * 44_100.0),
        ))
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        // rodio seeks to a frame but leaves the next sample on the same
        // channel as before the seek, so land on the target frame plus
        // the current channel instead of a frame boundary.
        let frame = (pos.as_secs_f64() * 44_100.0) as usize;
        let channel = self.position % 2;
        self.position = (frame * 2 + channel).min(self.samples.len());
        Ok(())
    }
}

#[test]
fn eq_source_disabled_passes_samples_through() {
    let raw = stereo_source(vec![0.5, -0.25, 0.75, -0.5]);
    let mut source = EqSource::new(raw, Arc::new(EqSettings::default()));

    assert_eq!(source.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(source.sample_rate(), SampleRate::new(44_100).unwrap());
    assert_eq!(source.current_span_len(), Some(4));
    assert!(source.total_duration().is_some());
    assert_eq!(
        source.by_ref().collect::<Vec<_>>(),
        vec![0.5, -0.25, 0.75, -0.5]
    );
}

#[test]
fn eq_source_reports_inner_seek_result() {
    let raw = stereo_source(vec![0.5, -0.25]);
    let mut source = EqSource::new(raw, Arc::new(EqSettings::default()));
    let mut unwrapped = stereo_source(vec![0.5, -0.25]);
    let expected = unwrapped.try_seek(Duration::from_secs(1));

    assert_eq!(
        source.try_seek(Duration::from_secs(1)).is_ok(),
        expected.is_ok()
    );
}

#[test]
fn eq_source_seek_clears_filter_state() {
    let mut bands = [0.0; crate::equalizer::BAND_COUNT];
    bands[4] = 12.0;
    let settings = Arc::new(EqSettings::default());
    settings.store(EqSnapshot {
        enabled: true,
        preamp_db: 0.0,
        bands_db: bands,
    });
    // An impulse followed by silence leaves the left channel's filter
    // ringing, so a seek that kept the filter state would replay the
    // impulse differently from a freshly built source. Seeking from a
    // frame boundary (two samples) must keep the channel cursor there,
    // since rodio leaves the next sample on the same channel.
    let samples = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let mut source = EqSource::new(SeekableSource::new(samples.clone()), Arc::clone(&settings));
    let before: Vec<_> = source.by_ref().take(2).collect();
    assert!(
        before.iter().any(|sample| *sample != 0.0),
        "the impulse should leave the filter ringing: {before:?}"
    );
    assert_eq!(
        source.channel, 0,
        "the cursor should be at a frame boundary"
    );

    source.try_seek(Duration::ZERO).unwrap();
    assert_eq!(source.channel, 0, "the seek should keep the channel cursor");
    let after_seek: Vec<_> = source.collect();
    let fresh: Vec<_> = EqSource::new(SeekableSource::new(samples), settings).collect();

    assert_eq!(after_seek.len(), fresh.len());
    for (index, (replayed, expected)) in after_seek.iter().zip(&fresh).enumerate() {
        assert!(
            (replayed - expected).abs() < 1e-6,
            "sample {index} was {replayed}, expected {expected}"
        );
    }
}

#[test]
fn eq_source_seek_keeps_the_channel_cursor() {
    let samples = vec![0.25, -0.5, 0.75, -1.0];
    let mut source = EqSource::new(
        SeekableSource::new(samples.clone()),
        Arc::new(EqSettings::default()),
    );
    source.by_ref().take(3).for_each(drop);
    assert_eq!(source.channel, 1, "the cursor should be mid-frame");

    source.try_seek(Duration::ZERO).unwrap();

    assert_eq!(source.channel, 1, "the seek must keep the channel cursor");
    assert_eq!(
        source.next(),
        Some(samples[1]),
        "the next sample must still be the right channel's"
    );
}

#[test]
fn balance_source_seek_keeps_the_channel_cursor() {
    let samples = vec![0.5, 0.25, 0.5, 0.25];
    let balance = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
    let mut source = BalanceSource::new(SeekableSource::new(samples), balance);
    source.by_ref().take(3).for_each(drop);
    assert_eq!(source.channel, 1, "the cursor should be mid-frame");

    source.try_seek(Duration::ZERO).unwrap();

    assert_eq!(source.channel, 1, "the seek must keep the channel cursor");
    // Hard right leaves the odd (right) channel at unity and silences the
    // even one, so a reset cursor would turn this sample into zero.
    assert_eq!(source.next(), Some(0.25));
}

#[test]
fn eq_source_enabled_flat_curve_passes_samples_through() {
    let settings = Arc::new(EqSettings::default());
    settings.store(EqSnapshot {
        enabled: true,
        preamp_db: 0.0,
        bands_db: [0.0; crate::equalizer::BAND_COUNT],
    });
    let raw = stereo_source(vec![0.5, -0.25, 0.75, -0.5]);
    let mut source = EqSource::new(raw, settings);

    for (output, input) in source.by_ref().zip([0.5, -0.25, 0.75, -0.5]) {
        assert!(
            (output - input).abs() < 1e-6,
            "flat curve changed {input} to {output}"
        );
    }
}

#[test]
fn eq_source_preamp_scales_by_the_decibel_gain() {
    let settings = Arc::new(EqSettings::default());
    settings.store(EqSnapshot {
        enabled: true,
        preamp_db: 6.020_6,
        bands_db: [0.0; crate::equalizer::BAND_COUNT],
    });
    let raw = stereo_source(vec![0.25, -0.25]);
    let mut source = EqSource::new(raw, settings);

    assert!((source.next().unwrap() - 0.5).abs() < 1e-4);
    assert!((source.next().unwrap() + 0.5).abs() < 1e-4);
}

#[test]
fn eq_source_filters_each_channel_with_its_own_state() {
    let mut bands = [0.0; crate::equalizer::BAND_COUNT];
    bands[4] = 12.0;
    let settings = Arc::new(EqSettings::default());
    settings.store(EqSnapshot {
        enabled: true,
        preamp_db: 0.0,
        bands_db: bands,
    });
    // An impulse in the left channel followed by silence. A filter shared
    // between channels would ring into the right channel's samples; one
    // filter per channel keeps the right channel exactly silent.
    let raw = stereo_source(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    let samples: Vec<_> = EqSource::new(raw, settings).collect();

    assert!(
        samples[0] != 0.0,
        "the left impulse should drive its filter"
    );
    assert!(
        samples
            .iter()
            .skip(1)
            .step_by(2)
            .all(|sample| *sample == 0.0),
        "the right channel must not see the left filter's state: {samples:?}"
    );
}

#[test]
fn eq_source_follows_the_curve_changed_while_playing() {
    let settings = Arc::new(EqSettings::default());
    let raw = stereo_source(vec![1.0; 4]);
    let mut source = EqSource::new(raw, Arc::clone(&settings));

    assert_eq!(source.next(), Some(1.0));
    assert_eq!(source.next(), Some(1.0));
    settings.store(EqSnapshot {
        enabled: true,
        preamp_db: 6.020_6,
        bands_db: [0.0; crate::equalizer::BAND_COUNT],
    });
    assert!((source.next().unwrap() - 2.0).abs() < 1e-4);
    assert!((source.next().unwrap() - 2.0).abs() < 1e-4);
}

#[test]
fn audio_with_fallback_returns_the_opened_output() {
    let recording = Arc::new(RecordingAudio::default());
    let opened = Arc::clone(&recording) as Arc<dyn AudioOutput>;

    let output = audio_with_fallback(|| Ok(opened));
    output.play(PREVIEW_URL).unwrap();

    assert_eq!(
        recording.calls(),
        vec![AudioCall::Play(PREVIEW_URL.to_string())]
    );
}

// A preview URL is server-controlled data (it arrives in the Apple Music API
// reply), and the preview agent allows any public host by design, so a hostile
// preview server can answer the download. The 10 MiB body cap must count the
// bytes the decoder produces, not the bytes on the wire: `ureq`'s
// `read_to_vec` limiter sits under its gzip decoder, so a small
// `Content-Encoding: gzip` body expands past the cap. This body is a fixed
// Huffman stream that decodes to just over the cap; a wire-byte cap reads it
// and the over-cap bytes reach the decoder (the pre-fix result is a decode
// failure on the zeros), while a decoded-byte cap rejects it.
#[test]
fn a_compressed_preview_larger_than_the_cap_is_rejected() {
    let cap = usize::try_from(MAX_PREVIEW_BYTES).expect("the cap fits a usize");
    let body = gzip_zeros(cap / 258 + 1);
    assert!(
        body.len() < cap,
        "the compressed body must stay under the wire cap, got {}",
        body.len()
    );
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: audio/mp4\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(&body);
    let address = serve_one_response_bytes(&response);
    let agent = agent_with_timeout(Duration::from_secs(10));
    let url = format!("http://{address}/bomb.m4a");

    let error = download_and_decode(&agent, &url)
        .err()
        .expect("an over-cap preview must not reach the decoder");
    assert!(
        error.to_string().contains("larger than the 10 MiB cap"),
        "unexpected error: {error}"
    );
}

// The decoded-byte cap alone would leave the download willing to read an
// arbitrarily large compressed body that decodes to little: an empty stored
// block costs five wire bytes and produces no output. This body puts more than
// the cap on the wire while decoding to zero bytes, so only `ureq`'s
// wire-byte limit can reject it; without that limit the read succeeds and the
// failure moves to the decoder.
#[test]
fn a_preview_with_more_wire_bytes_than_the_cap_is_rejected() {
    let cap = usize::try_from(MAX_PREVIEW_BYTES).expect("the cap fits a usize");
    let body = gzip_empty_stored_blocks(cap / 5 + 1);
    assert!(
        body.len() > cap,
        "the compressed body must exceed the wire cap, got {}",
        body.len()
    );
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: audio/mp4\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(&body);
    let address = serve_one_response_bytes(&response);
    let agent = agent_with_timeout(Duration::from_secs(10));
    let url = format!("http://{address}/bomb.m4a");

    let error = download_and_decode(&agent, &url)
        .err()
        .expect("an over-cap wire body must not reach the decoder");
    assert!(
        error.to_string().contains("reading the preview failed"),
        "unexpected error: {error}"
    );
}

#[test]
fn audio_with_fallback_falls_back_to_silence_when_open_fails() {
    let output = audio_with_fallback(|| Err(AppleMusicError::new("no device")));

    for (command, result) in every_command(output.as_ref()) {
        assert!(
            result.is_ok(),
            "the silent fallback's {command} must succeed"
        );
    }
}

// `ureq` defaults every network timeout to `None`, so a preview server
// that accepts the connection and never responds used to block the audio
// worker forever, and every later play/pause/stop command queued behind
// it. The bound is asserted from a worker thread with `recv_timeout`, so
// an unbounded download fails this test at 5s instead of hanging the
// suite.
#[test]
fn a_stalled_preview_download_is_bounded_by_the_timeout() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let agent = agent_with_timeout(Duration::from_millis(200));
    let url = format!("http://{address}/stalled");
    let (sender, receiver) = mpsc::channel();
    let started = Instant::now();
    std::thread::spawn(move || {
        let _ = sender.send(download_and_decode(&agent, &url));
    });
    let result = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("the download must return within its bound");
    assert!(result.is_err(), "a stalled preview download must error");
    // A connect failure would also return `Err`, instantly; require that
    // the call actually waited for the bound, so this test only passes
    // because the stalled response was timed out.
    assert!(
        started.elapsed() >= Duration::from_millis(100),
        "returned before the 200ms bound: {:?}",
        started.elapsed()
    );
}

#[test]
fn rodio_output_new_reports_the_device_result_without_panicking() {
    // A machine with an output device gets `Ok`; one without gets `Err`.
    // Either outcome is acceptable here — this pins only that opening the
    // device is reported through the `Result` rather than a panic.
    let _ = RodioOutput::new();
}

// `open_default_sink` talks to the platform audio daemon and can block
// indefinitely when that daemon is wedged. `RodioOutput::new` runs on the
// caller's thread (the binary builds the service before it starts the UI),
// so the wait for the worker's report must be bounded or the whole app
// stalls at startup. A sender that stays alive but never sends stands in
// for the hung open: the wait must return within the bound, not hang the
// test.
#[test]
fn a_device_open_that_never_reports_is_bounded_by_the_timeout() {
    let (_sender, receiver) = mpsc::channel();
    let started = Instant::now();
    let error = await_device_ready(receiver, Duration::from_millis(50))
        .expect_err("a device open that never reports must not wait forever");
    assert!(
        started.elapsed() >= Duration::from_millis(50),
        "returned before the 50ms bound: {:?}",
        started.elapsed()
    );
    assert_eq!(
        error.to_string(),
        "opening the default audio device timed out after 50ms"
    );
}

// The reporting half: a worker that reports the device opened must yield
// `Ok`, and one that reports an open failure must carry that failure
// through unchanged rather than turning it into a timeout.
#[test]
fn await_device_ready_carries_the_workers_report() {
    let (sender, receiver) = mpsc::channel();
    sender.send(Ok(())).unwrap();
    assert!(await_device_ready(receiver, Duration::from_secs(1)).is_ok());

    let (sender, receiver) = mpsc::channel();
    sender.send(Err(AppleMusicError::new("no device"))).unwrap();
    assert_eq!(
        await_device_ready(receiver, Duration::from_secs(1))
            .unwrap_err()
            .to_string(),
        "no device"
    );
}

// A worker whose thread ended before reporting (it panicked) must keep its
// distinct error, not be misreported as a hung device open.
#[test]
fn await_device_ready_reports_a_worker_that_ended_early() {
    let (sender, receiver) = mpsc::channel::<Result<(), AppleMusicError>>();
    drop(sender);
    assert_eq!(
        await_device_ready(receiver, Duration::from_secs(1))
            .unwrap_err()
            .to_string(),
        "the audio thread ended before opening a device"
    );
}

#[test]
fn rodio_output_maps_each_seam_method_to_its_worker_command() {
    // Hold the receiver open so the sends succeed, then read back the
    // command each public seam method put on the worker channel.
    let (commands, receiver) = std::sync::mpsc::channel();
    let output = RodioOutput {
        commands: Mutex::new(commands),
    };

    for (command, result) in every_command(&output) {
        assert!(
            result.is_ok(),
            "RodioOutput::{command} must reach the worker"
        );
    }

    match receiver.try_recv() {
        Ok(Command::Play(url)) => assert_eq!(url, PREVIEW_URL),
        _ => panic!("play did not send Command::Play"),
    }
    assert!(matches!(receiver.try_recv(), Ok(Command::Pause)));
    assert!(matches!(receiver.try_recv(), Ok(Command::Resume)));
    assert!(matches!(receiver.try_recv(), Ok(Command::Stop)));
    assert!(matches!(receiver.try_recv(), Ok(Command::SetVolume(v)) if v == 0.4));
    assert!(matches!(receiver.try_recv(), Ok(Command::SetBalance(v)) if v == -0.4));
    assert!(matches!(receiver.try_recv(), Ok(Command::SeekBy(v)) if v == -5));
    assert!(matches!(receiver.try_recv(), Ok(Command::SeekTo(v)) if v == Duration::from_secs(10)));
    assert!(
        matches!(receiver.try_recv(), Ok(Command::SetEqualizer(v)) if v == EqSnapshot::default())
    );
}

// A position query crosses to the worker and returns the reply it sends,
// rather than reading anything on the caller's thread.
#[test]
fn rodio_output_asks_the_worker_for_its_position() {
    let (commands, receiver) = std::sync::mpsc::channel();
    let output = RodioOutput {
        commands: Mutex::new(commands),
    };
    let worker = std::thread::spawn(move || match receiver.recv() {
        Ok(Command::Position(reply)) => {
            reply.send(Some(Duration::from_secs(3))).unwrap();
        }
        _ => panic!("position did not send Command::Position"),
    });

    assert_eq!(output.position(), Some(Duration::from_secs(3)));
    worker.join().unwrap();
}

// A format query crosses to the worker and returns the reply it sends, just
// like the position query.
#[test]
fn rodio_output_asks_the_worker_for_its_format() {
    let (commands, receiver) = std::sync::mpsc::channel();
    let output = RodioOutput {
        commands: Mutex::new(commands),
    };
    let format = PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    };
    let worker = std::thread::spawn(move || match receiver.recv() {
        Ok(Command::PlaybackFormat(reply)) => {
            reply.send(Some(format)).unwrap();
        }
        _ => panic!("format did not send Command::PlaybackFormat"),
    });

    assert_eq!(output.playback_format(), Some(format));
    worker.join().unwrap();
}

// A spectrum query crosses to the worker and returns the reply it sends, just
// like the position, format, and duration queries.
#[test]
fn rodio_output_asks_the_worker_for_its_spectrum() {
    let (commands, receiver) = std::sync::mpsc::channel();
    let output = RodioOutput {
        commands: Mutex::new(commands),
    };
    let spectrum = test_spectrum();
    let worker = std::thread::spawn(move || match receiver.recv() {
        Ok(Command::Spectrum(reply)) => {
            reply.send(Some(spectrum)).unwrap();
        }
        _ => panic!("spectrum did not send Command::Spectrum"),
    });

    assert_eq!(output.spectrum(), Some(spectrum));
    worker.join().unwrap();
}

// A finished query crosses to the worker and returns the reply it sends, just
// like the position, format, spectrum, and duration queries.
#[test]
fn rodio_output_asks_the_worker_for_its_finished_state() {
    let (commands, receiver) = std::sync::mpsc::channel();
    let output = RodioOutput {
        commands: Mutex::new(commands),
    };
    let worker = std::thread::spawn(move || match receiver.recv() {
        Ok(Command::Finished(reply)) => {
            reply.send(Some(true)).unwrap();
        }
        _ => panic!("finished did not send Command::Finished"),
    });

    assert!(output.is_finished());
    worker.join().unwrap();
}

// A worker that accepts the finished query but does not answer must not hold
// the caller past the seam's bound: the query reports not finished instead of
// waiting for the reply.
#[test]
fn rodio_output_reports_none_when_the_finished_reply_times_out() {
    let (commands, receiver) = std::sync::mpsc::channel();
    let output = RodioOutput {
        commands: Mutex::new(commands),
    };
    let worker = std::thread::spawn(move || match receiver.recv() {
        Ok(Command::Finished(reply)) => {
            // Keep the reply channel alive well past the seam's bound, as a
            // worker busy installing a preview would.
            std::thread::sleep(WORKER_QUERY_TIMEOUT * 10);
            drop(reply);
        }
        _ => panic!("finished did not send Command::Finished"),
    });

    let started = Instant::now();
    assert!(!output.is_finished());
    assert!(
        started.elapsed() < WORKER_QUERY_TIMEOUT * 5,
        "a missing finished reply must time out instead of blocking"
    );
    worker.join().unwrap();
}

// A worker that accepts the query but does not answer must not hold the
// caller past the seam's bound: the query reports `None` instead of waiting
// for the reply.
#[test]
fn rodio_output_reports_none_when_the_position_reply_times_out() {
    let (commands, receiver) = std::sync::mpsc::channel();
    let output = RodioOutput {
        commands: Mutex::new(commands),
    };
    let worker = std::thread::spawn(move || match receiver.recv() {
        Ok(Command::Position(reply)) => {
            // Keep the reply channel alive well past the seam's bound, as a
            // worker busy installing a preview would.
            std::thread::sleep(WORKER_QUERY_TIMEOUT * 10);
            drop(reply);
        }
        _ => panic!("position did not send Command::Position"),
    });

    let started = Instant::now();
    assert_eq!(output.position(), None);
    assert!(
        started.elapsed() < WORKER_QUERY_TIMEOUT * 5,
        "a missing position reply must time out instead of blocking"
    );
    worker.join().unwrap();
}

// A duration query crosses to the worker and returns the reply it sends,
// just like the position and format queries.
#[test]
fn rodio_output_asks_the_worker_for_its_duration() {
    let (commands, receiver) = std::sync::mpsc::channel();
    let output = RodioOutput {
        commands: Mutex::new(commands),
    };
    let worker = std::thread::spawn(move || match receiver.recv() {
        Ok(Command::Duration(reply)) => {
            reply.send(Some(Duration::from_secs(30))).unwrap();
        }
        _ => panic!("duration did not send Command::Duration"),
    });

    assert_eq!(output.duration(), Some(Duration::from_secs(30)));
    worker.join().unwrap();
}

// A worker that accepts the duration query but does not answer must not hold
// the caller past the seam's bound: the query reports `None` instead.
#[test]
fn rodio_output_reports_none_when_the_duration_reply_times_out() {
    let (commands, receiver) = std::sync::mpsc::channel();
    let output = RodioOutput {
        commands: Mutex::new(commands),
    };
    let worker = std::thread::spawn(move || match receiver.recv() {
        Ok(Command::Duration(reply)) => {
            std::thread::sleep(WORKER_QUERY_TIMEOUT * 10);
            drop(reply);
        }
        _ => panic!("duration did not send Command::Duration"),
    });

    let started = Instant::now();
    assert_eq!(output.duration(), None);
    assert!(
        started.elapsed() < WORKER_QUERY_TIMEOUT * 5,
        "a missing duration reply must time out instead of blocking"
    );
    worker.join().unwrap();
}

// Both bounds above are production behavior, but every test that exercises
// them passes its own timeout or compares against the constant itself, so a
// change to either would clear the suite while production applied the new
// value. `DEVICE_OPEN_TIMEOUT` is the longest startup waits for the platform
// audio daemon before falling back to silence: an unbounded or very long value
// stalls the app at launch, and a near-zero one abandons a healthy open.
// `WORKER_QUERY_TIMEOUT` is how long a worker query — the position, the
// format, or the preview's length — blocks its caller: a long value freezes
// the UI thread each poll, and zero would abandon every query. Pin the
// documented durations.
#[test]
fn audio_seam_bounds_are_the_documented_durations() {
    assert_eq!(DEVICE_OPEN_TIMEOUT, Duration::from_secs(5));
    assert_eq!(WORKER_QUERY_TIMEOUT, Duration::from_millis(100));
}

#[test]
fn rodio_output_reports_a_closed_worker_for_every_command() {
    // Dropping the receiver stands in for a worker thread that has ended,
    // so every seam method must surface the closed channel as an error
    // rather than panicking.
    let (commands, receiver) = std::sync::mpsc::channel();
    drop(receiver);
    let output = RodioOutput {
        commands: Mutex::new(commands),
    };

    for (command, result) in every_command(&output) {
        assert_eq!(
            result.map_err(|error| error.to_string()),
            Err("the audio thread is gone".to_string()),
            "RodioOutput::{command} must report the closed worker"
        );
    }
    assert!(
        output.position().is_none(),
        "a closed worker cannot answer a position query"
    );
    assert!(
        output.playback_format().is_none(),
        "a closed worker cannot answer a format query"
    );
    assert!(
        output.spectrum().is_none(),
        "a closed worker cannot answer a spectrum query"
    );
    assert!(
        !output.is_finished(),
        "a closed worker cannot report a finished preview"
    );
}

// A preview URL that answers with a non-2xx status must be reported as a
// status failure, before its body reaches the decoder. The agent disables
// `ureq`'s status-as-error shortcut (`http_status_as_error(false)`), so
// this 404 arrives as an `Ok` response and `download_and_decode`'s own
// status check runs. The body is deliberately non-empty: were that check
// removed, these bytes would go to `new_mp4` and the function would report
// a decode failure, so pinning the exact status message keeps the check
// ahead of the decoder.
#[test]
fn a_preview_download_with_an_error_status_is_reported() {
    let body = "<html>captive portal, not a preview</html>";
    let address = serve_one_response(&format!(
        "HTTP/1.1 404 Not Found\r\nContent-Type: audio/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ));
    let agent = agent_with_timeout(Duration::from_secs(5));
    let url = format!("http://{address}/missing.m4a");

    let error = download_and_decode(&agent, &url)
        .err()
        .expect("a 404 preview must not decode");
    assert_eq!(
        error.to_string(),
        "the preview download returned HTTP 404 Not Found"
    );
}

// A preview URL's host is validated before it reaches this backend, but a
// hostile or compromised API reply can point a public preview URL at a
// server that answers 302 to an internal address, which the one-time host
// check cannot see. `ureq` follows redirects by default, so without this
// guard the fetch would reach an address the preview validation refused.
// Pin that the preview agent refuses to follow one, mirroring the REST
// agent.
#[test]
fn a_redirecting_preview_is_not_followed() {
    let (foreign, foreign_addr) = loopback_listener();

    // The redirecting server answers the one request with a 302 pointing
    // at the "foreign" server, exactly what a hostile or compromised API
    // reply could return.
    let redirector_addr = serve_one_response(&format!(
        "HTTP/1.1 302 Found\r\nLocation: http://{foreign_addr}/preview.m4a\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    ));

    // The foreign server records whether any redirected request arrived;
    // the shared recorder polls for one second, so "no request" is a
    // bounded, observable result rather than a hang.
    let foreign_thread = record_first_request(foreign);

    let agent = agent_with_timeout(Duration::from_secs(2));
    let url = format!("http://{redirector_addr}/preview.m4a");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(download_and_decode(&agent, &url));
    });
    let result = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("the redirecting preview must return within its bound");
    let error = result.err().expect("a 302 preview must not decode");
    assert_eq!(
        error.to_string(),
        "the preview download returned HTTP 302 Found"
    );

    let followed = foreign_thread.join().unwrap();
    assert!(
        followed.is_none(),
        "the preview fetch reached the redirect target: {followed:?}"
    );
}

// A response whose headers arrive but whose body cannot be read whole (a
// dropped connection mid-download) must be reported as a read failure,
// before its partial bytes reach the decoder. This is the preview
// downloader's read-error arm (the REST transport has the twin test);
// without it a truncated body would surface as a decode failure, naming
// the wrong cause, or a partial decode.
#[test]
fn a_truncated_preview_body_reports_a_read_error() {
    // Declare 100 bytes but send only four, then close the connection.
    let address = serve_one_response(
        "HTTP/1.1 200 OK\r\nContent-Type: audio/mp4\r\nContent-Length: 100\r\nConnection: close\r\n\r\nJUNK",
    );
    let agent = agent_with_timeout(Duration::from_secs(5));
    let url = format!("http://{address}/truncated.m4a");

    let error = download_and_decode(&agent, &url)
        .err()
        .expect("a truncated preview must not reach the decoder");
    assert!(
        error.to_string().starts_with("reading the preview failed:"),
        "unexpected error: {error}"
    );
}

// A 2xx response whose body is not an MP4/AAC preview (a captive portal's
// HTML) must be reported as a decode failure, not panic the worker or play
// noise.
#[test]
fn a_preview_download_that_does_not_decode_is_reported() {
    let address = serve_one_response(
        "HTTP/1.1 200 OK\r\nContent-Type: audio/mp4\r\nContent-Length: 4\r\nConnection: close\r\n\r\nJUNK",
    );
    let agent = agent_with_timeout(Duration::from_secs(5));
    let url = format!("http://{address}/not-a-preview.m4a");

    let error = download_and_decode(&agent, &url)
        .err()
        .expect("garbage bytes must not decode as an MP4 preview");
    assert!(
        error.to_string().contains("decoding the preview failed"),
        "unexpected error: {error}"
    );
}

// `preview_url_problem` rejects internal address *literals*, but a preview
// URL may name a hostname that resolves to an internal address, and the
// fetch resolves that name only at connect time. `preview_agent`'s resolver
// must refuse such a name before any connection: `localhost` resolves to
// `127.0.0.1`/`::1`, so the guarded agent reports a resolution failure
// (`HostNotFound`) instead of connecting. The unguarded agent is checked in
// the same test so the result cannot come from `localhost` merely failing
// to resolve: it reaches the connect stage and fails there instead.
#[test]
fn the_preview_agent_refuses_a_hostname_that_resolves_to_loopback() {
    let (listener, address) = loopback_listener();
    // Accept the unguarded fetch's connection and close it without a
    // response, so that fetch fails at the protocol level rather than on
    // resolution. The guarded fetch never connects, so this thread may
    // wait on `accept` until the test process ends.
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = read_some_request(&mut stream);
        }
    });
    let url = format!("http://localhost:{}/preview.m4a", address.port());

    let guarded = preview_agent_with_timeout(Duration::from_secs(2));
    let error = guarded
        .get(&url)
        .call()
        .expect_err("a loopback-resolving hostname must not connect");
    assert!(
        matches!(error, ureq::Error::HostNotFound),
        "the guarded resolver must refuse the name, got: {error:?}"
    );

    let unguarded = agent_with_timeout(Duration::from_secs(2));
    let error = unguarded
        .get(&url)
        .call()
        .expect_err("the unguarded fetch must fail on the closed connection");
    assert!(
        !matches!(error, ureq::Error::HostNotFound),
        "`localhost` must resolve for the guarded case to prove anything, got: {error:?}"
    );
}

// The resolver's allow branch: a preview URL that names a public address
// literal must pass through to the connector. `preview_url_problem`
// accepts such a URL, so a resolver that refused every address — an
// inverted internal-address check, say — would fail every legitimate
// preview literal, and the refusal test above (which only pins the
// internal case) would stay green. An IP literal is resolved without a
// DNS lookup, so this stays offline.
#[test]
fn the_preview_resolver_allows_a_public_address_literal() {
    let uri: ureq::http::Uri = "https://93.184.216.34/preview.m4a".parse().unwrap();
    let config = config_with_timeout(Duration::from_secs(1));
    let timeout = NextTimeout {
        after: ureq::unversioned::transport::time::Duration::from_secs(1),
        reason: ureq::Timeout::Resolve,
    };

    let addresses = PublicAddressResolver::default()
        .resolve(&uri, &config, timeout)
        .expect("a public address literal must not be refused");

    assert!(
        addresses
            .iter()
            .any(|address| address.ip().to_string() == "93.184.216.34"),
        "the resolver must return the public address it resolved: {addresses:?}"
    );
}

// The worker fetches through `preview_agent`, but the redirect test above
// and the non-success-status test below build their agents with
// `agent_with_timeout` directly. A regression that built `preview_agent`
// from `ureq`'s default configuration — no global timeout, redirects
// followed, status-as-error on — would leave those tests green while the
// production fetch could stall the worker forever, follow a redirect past
// the one-time host check, or discard Apple's error body. Pin the three
// shared transport choices on the production agent itself.
#[test]
fn the_preview_agent_carries_the_documented_transport_settings() {
    assert_eq!(PREVIEW_TIMEOUT, Duration::from_secs(30));
    let config = preview_agent().config();
    assert_eq!(config.timeouts().global, Some(PREVIEW_TIMEOUT));
    assert_eq!(config.max_redirects(), 0);
    assert!(!config.http_status_as_error());
}

// An HTTP(S) CONNECT proxy resolves the target host itself, so with
// `HTTPS_PROXY` (or `ALL_PROXY`) set, `ureq` sends the preview hostname to
// the proxy and never consults `preview_agent`'s resolver — the SSRF guard
// is bypassed. `preview_agent` therefore disables the proxy. The assertion
// runs in a child process of this same test binary: the proxy variables
// are process-global, and setting them here would race every other test
// that builds an agent. The parent forwards them only to the child.
#[test]
fn the_preview_agent_ignores_a_configured_proxy() {
    let child =
        std::process::Command::new(std::env::current_exe().expect("the test executable path"))
            .args([
                "--exact",
                "audio::tests::with_a_proxy_set_the_preview_agent_stays_direct",
            ])
            .env("HTTPS_PROXY", "http://127.0.0.1:9")
            .env("ALL_PROXY", "http://127.0.0.1:9")
            .output()
            .expect("run the child test process");
    let stdout = String::from_utf8_lossy(&child.stdout);
    assert!(
        child.status.success() && stdout.contains("1 passed"),
        "the child test must run and pass; stdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&child.stderr)
    );
}

// Runs only in the child process `the_preview_agent_ignores_a_configured_proxy`
// spawns with the proxy variables set: the ordinary suite leaves them unset,
// and this returns without asserting rather than racing to set them.
#[test]
fn with_a_proxy_set_the_preview_agent_stays_direct() {
    if std::env::var_os("HTTPS_PROXY").is_none() && std::env::var_os("ALL_PROXY").is_none() {
        return;
    }
    let timeout = Duration::from_secs(1);
    // The REST agent inherits the environment proxy; this proves the
    // variables are visible to `ureq` in this process, so the preview
    // assertion below is not vacuously true.
    assert!(
        agent_with_timeout(timeout).config().proxy().is_some(),
        "the environment proxy must reach the REST agent's config"
    );
    assert!(
        preview_agent_with_timeout(timeout)
            .config()
            .proxy()
            .is_none(),
        "the preview fetch must be direct even when a proxy is configured"
    );
}

/// A distinctive spectrum for the recording player's query tests: one
/// nonzero band, so a reply can be compared without float rounding.
fn test_spectrum() -> Spectrum {
    let mut bands = [0.0; BAND_COUNT];
    bands[4] = 0.75;
    Spectrum { bands }
}

/// A recording [`WorkerPlayer`] for the command-loop tests: it records the
/// previews appended, the calls made and their order, the pause/resume
/// calls, and whether it was stopped, and remembers its gain. Its position
/// starts at 42 seconds and moves by every recorded seek, so a test can pin
/// that a seek reaches the next position query.
#[derive(Clone)]
struct RecordingPlayer {
    appended: Arc<AtomicUsize>,
    calls: Arc<Mutex<Vec<&'static str>>>,
    paused: Arc<AtomicUsize>,
    resumed: Arc<AtomicUsize>,
    stopped: Arc<AtomicBool>,
    volume: Arc<Mutex<f32>>,
    balance: Arc<Mutex<f32>>,
    seeks: Arc<Mutex<Vec<i64>>>,
    absolute_seeks: Arc<Mutex<Vec<Duration>>>,
    position: Arc<Mutex<Duration>>,
    duration: Arc<Mutex<Option<Duration>>>,
    spectrum: Arc<Mutex<Option<Spectrum>>>,
    finished: Arc<AtomicBool>,
    equalizer: Arc<Mutex<EqSnapshot>>,
}

impl Default for RecordingPlayer {
    fn default() -> Self {
        Self {
            appended: Arc::default(),
            calls: Arc::default(),
            paused: Arc::default(),
            resumed: Arc::default(),
            stopped: Arc::default(),
            volume: Arc::default(),
            balance: Arc::default(),
            seeks: Arc::default(),
            absolute_seeks: Arc::default(),
            position: Arc::new(Mutex::new(Duration::from_secs(42))),
            duration: Arc::new(Mutex::new(Some(Duration::from_secs(30)))),
            spectrum: Arc::new(Mutex::new(Some(test_spectrum()))),
            finished: Arc::default(),
            equalizer: Arc::default(),
        }
    }
}

impl WorkerPlayer for RecordingPlayer {
    type Preview = ();

    fn set_volume(&self, volume: f32) {
        self.calls.lock().unwrap().push("volume");
        *self.volume.lock().unwrap() = volume;
    }

    fn set_balance(&self, balance: f32) {
        *self.balance.lock().unwrap() = balance;
    }

    fn seek_by(&self, offset_secs: i64) {
        self.seeks.lock().unwrap().push(offset_secs);
        let mut position = self.position.lock().unwrap();
        *position = seek_target(*position, offset_secs);
    }

    fn seek_to(&self, position: Duration) {
        self.absolute_seeks.lock().unwrap().push(position);
        *self.position.lock().unwrap() = position;
    }

    fn position(&self) -> Duration {
        *self.position.lock().unwrap()
    }

    fn duration(&self) -> Option<Duration> {
        *self.duration.lock().unwrap()
    }

    fn playback_format(&self) -> Option<PlaybackFormat> {
        Some(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: Some(256),
        })
    }

    fn spectrum(&self) -> Option<Spectrum> {
        *self.spectrum.lock().unwrap()
    }

    fn is_finished(&self) -> bool {
        self.finished.load(Ordering::SeqCst)
    }

    fn set_equalizer(&self, equalizer: EqSnapshot) {
        *self.equalizer.lock().unwrap() = equalizer;
    }

    fn append(&self, _preview: ()) {
        self.calls.lock().unwrap().push("append");
        self.appended.fetch_add(1, Ordering::SeqCst);
    }

    fn pause(&self) {
        self.calls.lock().unwrap().push("pause");
        self.paused.fetch_add(1, Ordering::SeqCst);
    }

    fn play(&self) {
        self.calls.lock().unwrap().push("resume");
        self.resumed.fetch_add(1, Ordering::SeqCst);
    }

    fn stop(&self) {
        self.calls.lock().unwrap().push("stop");
        self.stopped.store(true, Ordering::SeqCst);
    }
}

/// Builds the recording player factory the [`serve_commands`] tests drive:
/// every call records the fresh [`RecordingPlayer`] it returns in creation
/// order, so a test can assert which players the loop installed and what
/// happened to them. Returns the shared recording list alongside the
/// factory to pass to [`serve_commands`].
///
/// `queue_on_first` is sent through `sender` from the factory's first call,
/// once the first player is recorded. A test queues a later selection this
/// way so it is not already waiting when the loop drains the channel after
/// a download: an already-queued selection would supersede the preview the
/// test means to replace, coalescing the two commands into one.
fn recording_players(
    sender: &Sender<Command>,
    queue_on_first: Command,
) -> (
    Arc<Mutex<Vec<RecordingPlayer>>>,
    impl Fn() -> RecordingPlayer + use<>,
) {
    let players: Arc<Mutex<Vec<RecordingPlayer>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&players);
    let queued = Arc::new(Mutex::new(Some((sender.clone(), queue_on_first))));
    let create_player = move || {
        let player = RecordingPlayer::default();
        recorded.lock().unwrap().push(player.clone());
        if let Some((sender, command)) = queued.lock().unwrap().take() {
            sender.send(command).unwrap();
        }
        player
    };
    (players, create_player)
}

/// Builds the recording player factory for tests that queue no command from
/// inside the factory: every call records the fresh [`RecordingPlayer`] it
/// returns in creation order, so a test can assert what happened to it.
/// Returns the shared recording list alongside the factory to pass to
/// [`serve_commands`].
///
/// Unlike [`recording_players`], the factory holds no sender clone, so the
/// test can drop its own sender and let the worker loop end.
fn recording_players_without_queue() -> (
    Arc<Mutex<Vec<RecordingPlayer>>>,
    impl Fn() -> RecordingPlayer + use<>,
) {
    let players: Arc<Mutex<Vec<RecordingPlayer>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&players);
    let create_player = move || {
        let player = RecordingPlayer::default();
        recorded.lock().unwrap().push(player.clone());
        player
    };
    (players, create_player)
}

/// Runs one [`serve_commands`] loop whose first preview download (for
/// `"first"`) queues `supersede` from inside the fetch, so the command
/// lands while that download is still in flight. Returns the players the
/// loop installed, in creation order, and the URLs whose downloads ran.
///
/// The queued command is sent from inside the fetch, and its sender clone
/// is dropped with the send, so the channel closes and the worker loop
/// ends. The two supersession tests share this setup; only the command
/// (a newer Play or a Stop) and their assertions differ.
fn run_with_superseding_command(supersede: Command) -> (Vec<RecordingPlayer>, Vec<String>) {
    let (sender, receiver) = mpsc::channel();
    let fetched: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let (players, create_player) = recording_players_without_queue();
    let queued = Arc::new(Mutex::new(Some((sender.clone(), supersede))));
    let seen = Arc::clone(&fetched);
    let fetch = move |url: &str| -> Result<(), AppleMusicError> {
        seen.lock().unwrap().push(url.to_string());
        if url == "first"
            && let Some((sender, command)) = queued.lock().unwrap().take()
        {
            sender.send(command).unwrap();
        }
        Ok(())
    };

    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap().clone();
    let fetched = fetched.lock().unwrap().clone();
    (players, fetched)
}

// `play_track` commits the new track and returns `Ok` as soon as the
// command is queued, so the worker's later download failure is invisible
// to the caller. If that failure left the previous player running, the old
// preview would stay audible under the new track's name. The first Play
// installs a player and the second's download fails; the loop must stop
// the first player and install none. The second selection is queued from
// inside `create_player`, once the first player is installed, so the two
// do not coalesce into one.
#[test]
fn a_failed_preview_stops_the_previous_player() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players(&sender, Command::Play("second".to_string()));
    let fetches = Arc::new(AtomicUsize::new(0));

    let fetch = move |_url: &str| {
        if fetches.fetch_add(1, Ordering::SeqCst) == 0 {
            Ok(())
        } else {
            Err(AppleMusicError::new("the preview download failed"))
        }
    };

    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(
        players.len(),
        1,
        "a failed download must not install a player"
    );
    assert_eq!(players[0].appended.load(Ordering::SeqCst), 1);
    assert!(
        players[0].stopped.load(Ordering::SeqCst),
        "the failed preview must stop the previous player"
    );
}

// A successful Play installs a fresh player for the new track (rather than
// queueing it behind the old one) and starts it at the remembered gain, so
// a volume set while a previous track played carries to the next.
// The second selection is queued from inside `create_player`, once the
// first player has been installed, so the two downloads do not coalesce
// into one: this pins the replacement itself, not the supersession path.
#[test]
fn a_new_preview_replaces_the_previous_player_at_the_remembered_volume() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players(&sender, Command::Play("second".to_string()));
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };

    sender.send(Command::SetVolume(0.4)).unwrap();
    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players.len(), 2, "each successful Play installs a player");
    assert_eq!(*players[0].volume.lock().unwrap(), 0.4);
    assert_eq!(*players[1].volume.lock().unwrap(), 0.4);
}

// Pause, Resume, SetVolume, SetBalance and Stop drive the player a
// previous Play installed; their `if let Some(player)` arms are the only
// way those commands reach the output. The commands are sent from inside
// the first `create_player` call, once that player is recorded, so they
// arrive in order after the Play installs it. The final Pause proves Stop
// dropped the player instead of leaving it for later commands: it must not
// reach the stopped player.
#[test]
fn transport_volume_and_balance_commands_drive_the_installed_player() {
    let (sender, receiver) = mpsc::channel();
    let players: Arc<Mutex<Vec<RecordingPlayer>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&players);
    let command_sender = Arc::new(Mutex::new(Some(sender.clone())));
    let create_player = move || {
        let player = RecordingPlayer::default();
        recorded.lock().unwrap().push(player.clone());
        if let Some(sender) = command_sender.lock().unwrap().take() {
            sender.send(Command::Pause).unwrap();
            sender.send(Command::Resume).unwrap();
            sender.send(Command::SetVolume(0.6)).unwrap();
            sender.send(Command::SetBalance(0.3)).unwrap();
            sender.send(Command::SeekBy(-5)).unwrap();
            sender.send(Command::Stop).unwrap();
            sender.send(Command::Pause).unwrap();
        }
        player
    };
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };

    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players.len(), 1, "the Play must install one player");
    assert_eq!(players[0].paused.load(Ordering::SeqCst), 1);
    assert_eq!(players[0].resumed.load(Ordering::SeqCst), 1);
    assert_eq!(*players[0].volume.lock().unwrap(), 0.6);
    assert_eq!(*players[0].balance.lock().unwrap(), 0.3);
    assert_eq!(*players[0].seeks.lock().unwrap(), vec![-5]);
    assert!(
        players[0].stopped.load(Ordering::SeqCst),
        "Stop must stop the installed player"
    );
}

// A seek reaches only the player a previous Play installed: with none
// installed yet it must not create one, and after Stop drops the player
// it must not reach the stopped one. The first SeekBy arrives before the
// Play; Stop and the second SeekBy are sent once the player is installed,
// so the recording player's empty seek list proves neither went through.
#[test]
fn a_seek_while_stopped_reaches_no_player() {
    let (sender, receiver) = mpsc::channel();
    let players: Arc<Mutex<Vec<RecordingPlayer>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&players);
    let command_sender = Arc::new(Mutex::new(Some(sender.clone())));
    let create_player = move || {
        let player = RecordingPlayer::default();
        recorded.lock().unwrap().push(player.clone());
        if let Some(sender) = command_sender.lock().unwrap().take() {
            sender.send(Command::Stop).unwrap();
            sender.send(Command::SeekBy(5)).unwrap();
        }
        player
    };
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };

    sender.send(Command::SeekBy(-5)).unwrap();
    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players.len(), 1, "only the Play may create a player");
    assert!(
        players[0].seeks.lock().unwrap().is_empty(),
        "a seek with no installed player must not reach one"
    );
    assert!(
        players[0].stopped.load(Ordering::SeqCst),
        "Stop must drop the installed player"
    );
}

// A position query with a player installed must answer from that player.
// The Position command follows the Play in the channel, so the worker
// handles it after the Play's download installs the player.
#[test]
fn position_query_returns_the_loaded_players_position() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender.send(Command::Position(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(
        players.lock().unwrap().len(),
        1,
        "the Play installs a player"
    );
    assert_eq!(
        replies.recv().unwrap(),
        Some(Duration::from_secs(42)),
        "the query must answer with the installed player's position"
    );
}

// A seek must move the player, so the next position query reports the new
// position rather than the one before it. SeekBy follows the Play in the
// channel, and the Position query follows the seek.
#[test]
fn a_seek_is_reflected_by_the_next_position_query() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender.send(Command::SeekBy(5)).unwrap();
    sender.send(Command::Position(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(*players.lock().unwrap()[0].seeks.lock().unwrap(), vec![5]);
    assert_eq!(
        replies.recv().unwrap(),
        Some(Duration::from_secs(47)),
        "the query must report the position the seek moved the player to"
    );
}

// An absolute seek must move the loaded player to that exact position, so the
// next position query reports it. SeekTo follows the Play in the channel, and
// the Position query follows the seek.
#[test]
fn seek_to_command_reaches_the_loaded_player() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender
        .send(Command::SeekTo(Duration::from_secs(10)))
        .unwrap();
    sender.send(Command::Position(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(
        *players.lock().unwrap()[0].absolute_seeks.lock().unwrap(),
        vec![Duration::from_secs(10)]
    );
    assert_eq!(
        replies.recv().unwrap(),
        Some(Duration::from_secs(10)),
        "the query must report the position the absolute seek moved to"
    );
}

// A duration query with a player installed answers from that player's own
// preview length.
#[test]
fn duration_query_returns_the_loaded_players_duration() {
    let (sender, receiver) = mpsc::channel();
    let (_, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender.send(Command::Duration(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(
        replies.recv().unwrap(),
        Some(Duration::from_secs(30)),
        "the query must answer with the installed player's duration"
    );
}

// With no player installed, a duration query reports `None` instead of
// creating a player or inventing a length.
#[test]
fn duration_query_reports_none_when_nothing_is_loaded() {
    let (sender, receiver) = mpsc::channel();
    let create_player = || -> RecordingPlayer {
        panic!("a duration query must not create a player");
    };
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Duration(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(replies.recv().unwrap(), None);
}

// Pause must not unload the player: the worker keeps it installed and
// paused, so the next poll reads its own frozen clock instead of reporting
// that no preview is loaded.
#[test]
fn a_pause_keeps_the_loaded_player_reporting_its_position() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender.send(Command::Pause).unwrap();
    sender.send(Command::Position(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players[0].paused.load(Ordering::SeqCst), 1);
    assert!(
        !players[0].stopped.load(Ordering::SeqCst),
        "Pause must not stop the player"
    );
    assert_eq!(
        replies.recv().unwrap(),
        Some(Duration::from_secs(42)),
        "a paused player must still report its position"
    );
}

// With no player installed, a position query reports `None` instead of
// creating a player or inventing a zero position.
#[test]
fn position_query_reports_none_when_nothing_is_loaded() {
    let (sender, receiver) = mpsc::channel();
    let create_player = || -> RecordingPlayer {
        panic!("a position query must not create a player");
    };
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Position(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(replies.recv().unwrap(), None);
}

// A format query with a player installed must answer from that player, the
// same way a position query does.
#[test]
fn playback_format_query_returns_the_loaded_players_format() {
    let (sender, receiver) = mpsc::channel();
    let (_, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender.send(Command::PlaybackFormat(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(
        replies.recv().unwrap(),
        Some(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: Some(256),
        }),
        "the query must answer with the installed player's format"
    );
}

// With no player installed, a format query reports `None` instead of
// inventing a sample rate for a preview that is not loaded.
#[test]
fn playback_format_query_reports_none_when_nothing_is_loaded() {
    let (sender, receiver) = mpsc::channel();
    let create_player = || -> RecordingPlayer {
        panic!("a format query must not create a player");
    };
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::PlaybackFormat(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(replies.recv().unwrap(), None);
}

// The Rodio player's query guard reports its meter only once a preview is
// loaded, so a player that has not appended one cannot answer with an old
// window left behind by a previous preview. This drives the real
// `RodioPlayer::spectrum` against its format cell, not a stand-in for the
// guard.
#[test]
fn loaded_player_reports_its_meter_but_an_unloaded_one_reports_nothing() {
    let meter = Arc::new(SpectrumMeter::default());
    meter.set_sample_rate(44_100);
    for sample in sine_samples(1_000.0, 44_100, SPECTRUM_WINDOW) {
        meter.push(sample, 1);
    }

    let player = RodioPlayer::without_device(Arc::clone(&meter)).0;
    assert_eq!(
        player.spectrum(),
        None,
        "a player that has not appended a preview must not report the window"
    );

    player.mark_loaded_for_test();
    let loaded = player
        .spectrum()
        .expect("a loaded player must report its meter's spectrum");
    assert!(
        loaded.bands[4] > 0.9,
        "the loaded player must report the recorded sine"
    );
}

// The player's append path must feed its meter from the source it actually
// appends: a hand-filled meter would pass the guard test above even if
// production stopped wrapping the preview in `MeterSource`.
#[test]
fn append_records_the_appended_source_through_the_players_meter() {
    let meter = Arc::new(SpectrumMeter::default());
    let (player, mut queue) = RodioPlayer::without_device(Arc::clone(&meter));
    let sine = sine_samples(1_000.0, 44_100, SPECTRUM_WINDOW);
    let buffer = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(1).unwrap(),
        SampleRate::new(44_100).unwrap(),
        sine.clone(),
    );

    player.append_source(buffer, 0);
    assert_eq!(
        player.spectrum(),
        Some(Spectrum {
            bands: [0.0; BAND_COUNT]
        }),
        "the meter stays empty until the appended source is pulled"
    );

    // Pull the appended sound through the player's queue, as the output
    // device would, so the meter records exactly the samples production plays.
    queue.by_ref().take(sine.len()).for_each(drop);

    let spectrum = player
        .spectrum()
        .expect("an appended preview must report a spectrum");
    assert!(
        spectrum.bands[4] > 0.9,
        "the appended sine must reach the player's own meter, got {spectrum:?}"
    );
}

// The production player's finished state must come from rodio's own queue:
// the format cell alone is set by an append, and `empty` alone also holds
// before a preview is ever queued. This drives the real
// `RodioPlayer::is_finished` over a real `rodio::Player`.
#[test]
fn rodio_player_reports_finished_only_after_its_appended_preview_drains() {
    let (player, mut queue) = RodioPlayer::without_device(Arc::new(SpectrumMeter::default()));
    assert!(
        !player.is_finished(),
        "a player that has appended nothing must not report a finished preview"
    );

    let samples = sine_samples(1_000.0, 44_100, 16);
    let total = samples.len();
    let buffer = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(1).unwrap(),
        SampleRate::new(44_100).unwrap(),
        samples,
    );
    player.append_source(buffer, 0);
    assert!(
        !player.is_finished(),
        "a preview that is still queued must not report finished"
    );

    // Pull every sample but leave the source unpolled for its end, so the
    // queue still holds the preview even though its audio is exhausted.
    queue.by_ref().take(total).for_each(drop);
    assert!(
        !player.is_finished(),
        "a preview whose end has not been observed must not report finished"
    );

    // The output device polls once more, which is when rodio observes the
    // source ending and decrements its queued-sound count.
    let _ = queue.next();
    assert!(
        player.is_finished(),
        "a drained appended preview must report finished"
    );
}

// A paused preview has not reached its end, so the production player must not
// report it as finished even though no more samples are being pulled.
#[test]
fn a_paused_preview_reports_unfinished() {
    let (player, _queue) = RodioPlayer::without_device(Arc::new(SpectrumMeter::default()));
    let buffer = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(1).unwrap(),
        SampleRate::new(44_100).unwrap(),
        sine_samples(1_000.0, 44_100, 16),
    );

    player.append_source(buffer, 0);
    player.pause();

    assert!(!player.is_finished());
}

// A spectrum query with a player installed must answer from that player, the
// same way a format query does.
#[test]
fn spectrum_query_returns_the_loaded_players_spectrum() {
    let (sender, receiver) = mpsc::channel();
    let (_, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender.send(Command::Spectrum(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(
        replies.recv().unwrap(),
        Some(test_spectrum()),
        "the query must answer with the installed player's spectrum"
    );
}

// A finished query answers from the installed player, so the UI observes a
// preview's natural end without reading the worker's state itself.
#[test]
fn finished_query_returns_the_loaded_players_finished_state() {
    for finished in [true, false] {
        let (sender, receiver) = mpsc::channel();
        let player = RecordingPlayer::default();
        player.finished.store(finished, Ordering::SeqCst);
        let installed = player.clone();
        let create_player = move || installed.clone();
        let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
        let (reply, replies) = mpsc::channel();

        sender.send(Command::Play("first".to_string())).unwrap();
        sender.send(Command::Finished(reply)).unwrap();
        drop(sender);
        serve_commands(receiver, create_player, fetch);

        assert_eq!(
            replies.recv().unwrap(),
            Some(finished),
            "the query must answer with the installed player's finished state"
        );
    }
}

// With no player installed, a finished query reports `None` instead of
// claiming a preview that is not loaded has ended.
#[test]
fn finished_query_reports_none_when_nothing_is_loaded() {
    let (sender, receiver) = mpsc::channel();
    let create_player = || -> RecordingPlayer {
        panic!("a finished query must not create a player");
    };
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Finished(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(replies.recv().unwrap(), None);
}

// With no player installed, a spectrum query reports `None` instead of
// inventing bars for a preview that is not loaded.
#[test]
fn spectrum_query_reports_none_when_nothing_is_loaded() {
    let (sender, receiver) = mpsc::channel();
    let create_player = || -> RecordingPlayer {
        panic!("a spectrum query must not create a player");
    };
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Spectrum(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(replies.recv().unwrap(), None);
}

// Stop drops the loaded player, so a later spectrum query cannot answer with
// the stopped preview's bars.
#[test]
fn stop_removes_the_player_so_a_spectrum_query_reports_none() {
    let (sender, receiver) = mpsc::channel();
    let (_, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender.send(Command::Stop).unwrap();
    sender.send(Command::Spectrum(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(replies.recv().unwrap(), None);
}

// Stop drops the loaded player, so a later finished query cannot report the
// stopped preview as having reached its end. Stop and the query are sent from
// inside the first `create_player` call, once that player is recorded, so they
// arrive after the Play installs it; were the player still installed, the
// finished query would answer with its state instead of `None`.
#[test]
fn stop_removes_the_player_so_a_finished_query_reports_none() {
    let (sender, receiver) = mpsc::channel();
    let players: Arc<Mutex<Vec<RecordingPlayer>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&players);
    let (reply, replies) = mpsc::channel();
    let queued = Arc::new(Mutex::new(Some((
        sender.clone(),
        vec![Command::Stop, Command::Finished(reply)],
    ))));
    let create_player = move || {
        let player = RecordingPlayer::default();
        recorded.lock().unwrap().push(player.clone());
        if let Some((sender, commands)) = queued.lock().unwrap().take() {
            for command in commands {
                sender.send(command).unwrap();
            }
        }
        player
    };
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };

    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players.len(), 1, "the Play must install one player");
    assert!(
        players[0].stopped.load(Ordering::SeqCst),
        "Stop must stop the installed player"
    );
    assert_eq!(replies.recv().unwrap(), None);
}

// A selection whose preview fails to install leaves no player behind, so
// the following position query reports `None` rather than a stale position
// for audio that is not playing. The second selection and the query are
// queued from inside the first player's creation, and both senders are
// dropped before the loop drains them, so it ends after answering.
#[test]
fn a_failed_preview_leaves_no_position_to_report() {
    let (sender, receiver) = mpsc::channel();
    let (reply, replies) = mpsc::channel();
    let queued = Arc::new(Mutex::new(Some((sender.clone(), reply))));
    let create_player = move || {
        let player = RecordingPlayer::default();
        if let Some((sender, reply)) = queued.lock().unwrap().take() {
            sender.send(Command::Play("second".to_string())).unwrap();
            sender.send(Command::Position(reply)).unwrap();
        }
        player
    };
    let fetches = Arc::new(AtomicUsize::new(0));
    let fetch = move |_url: &str| {
        if fetches.fetch_add(1, Ordering::SeqCst) == 0 {
            Ok(())
        } else {
            Err(AppleMusicError::new("the preview download failed"))
        }
    };

    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(
        replies.recv().unwrap(),
        None,
        "a failed preview must leave no position to report"
    );
}

// A balance set while stopped must carry to the player a later Play
// creates, just as the remembered volume does.
#[test]
fn a_new_preview_starts_at_the_remembered_balance() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players(&sender, Command::Play("second".to_string()));
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };

    sender.send(Command::SetBalance(-0.4)).unwrap();
    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players.len(), 2, "each successful Play installs a player");
    assert_eq!(*players[0].balance.lock().unwrap(), -0.4);
    assert_eq!(*players[1].balance.lock().unwrap(), -0.4);
}

// An equalizer curve set while stopped must carry to the player a later
// Play creates, just as the remembered volume and balance do.
#[test]
fn a_new_preview_starts_at_the_remembered_equalizer() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players(&sender, Command::Play("second".to_string()));
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let equalizer = EqSnapshot {
        enabled: true,
        preamp_db: 3.0,
        bands_db: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0],
    };

    sender.send(Command::SetEqualizer(equalizer)).unwrap();
    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players.len(), 2, "each successful Play installs a player");
    assert_eq!(*players[0].equalizer.lock().unwrap(), equalizer);
    assert_eq!(*players[1].equalizer.lock().unwrap(), equalizer);
}

// An equalizer curve set during playback must reach the player the current
// Play installed, not just the players later Plays create. The command is
// queued from the player factory, so it is handled after the Play installs
// its player.
#[test]
fn an_equalizer_set_while_playing_updates_the_installed_player() {
    let (sender, receiver) = mpsc::channel();
    let equalizer = EqSnapshot {
        enabled: true,
        preamp_db: -2.0,
        bands_db: [0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -3.0],
    };
    let (players, create_player) = recording_players(&sender, Command::SetEqualizer(equalizer));
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };

    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players.len(), 1, "the Play must install one player");
    assert_eq!(
        *players[0].equalizer.lock().unwrap(),
        equalizer,
        "SetEqualizer must update the player already installed"
    );
}

// A Play's download runs on the worker, so commands issued meanwhile wait
// in the channel. If a newer Play arrives during the download, the UI has
// already committed the newer selection, and installing the older preview
// would play the wrong audio under the committed track's name. The worker
// must drop the superseded preview and handle the queued selection.
#[test]
fn a_preview_superseded_during_its_download_is_not_installed() {
    // "first" is still downloading when the user selects "second", so the
    // second Play is sent from inside the first fetch.
    let (players, fetched) = run_with_superseding_command(Command::Play("second".to_string()));

    assert_eq!(
        fetched,
        vec!["first".to_string(), "second".to_string()],
        "the superseded download still ran; only its preview is dropped"
    );
    assert_eq!(
        players.len(),
        1,
        "only the newer selection may install a player"
    );
    assert_eq!(
        players[0].appended.load(Ordering::SeqCst),
        1,
        "only the newer preview may be appended"
    );
}

// Not every command that arrives during a download supersedes it: Pause,
// Resume and SetVolume all act on the player the preview installs. The
// loop drains them into `queued` and handles them only after installing
// the preview, since before that there is no player to receive them. A
// drain that dropped them (or applied them ahead of the install) would
// silently lose the user's volume or transport action, so this pins that
// they reach the new player in the order they arrived.
#[test]
fn commands_queued_during_a_download_apply_to_the_new_player() {
    let (sender, receiver) = mpsc::channel();
    let fetched: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let (players, create_player) = recording_players_without_queue();
    // The commands are sent from inside the fetch, while the preview is
    // still downloading, so they are waiting in the channel when the loop
    // drains it. The sender clone is taken (and so dropped) with the
    // sends, letting the channel close and the worker loop end.
    let fetch_sender = Arc::new(Mutex::new(Some(sender.clone())));
    let seen = Arc::clone(&fetched);
    let fetch = move |url: &str| -> Result<(), AppleMusicError> {
        seen.lock().unwrap().push(url.to_string());
        if let Some(sender) = fetch_sender.lock().unwrap().take() {
            sender.send(Command::SetVolume(0.3)).unwrap();
            sender.send(Command::Pause).unwrap();
            sender.send(Command::Resume).unwrap();
        }
        Ok(())
    };

    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(
        *fetched.lock().unwrap(),
        vec!["first".to_string()],
        "the queued commands must not supersede the download"
    );
    let players = players.lock().unwrap();
    assert_eq!(players.len(), 1, "the download must install one player");
    assert_eq!(players[0].appended.load(Ordering::SeqCst), 1);
    assert_eq!(
        *players[0].volume.lock().unwrap(),
        0.3,
        "the SetVolume queued during the download must reach the new player"
    );
    assert_eq!(
        players[0].paused.load(Ordering::SeqCst),
        1,
        "the Pause queued during the download must reach the new player"
    );
    assert_eq!(
        players[0].resumed.load(Ordering::SeqCst),
        1,
        "the Resume queued during the download must reach the new player"
    );
    assert_eq!(
        *players[0].calls.lock().unwrap(),
        vec!["volume", "append", "volume", "pause", "resume"],
        "the queued commands must reach the new player after the install, in arrival order"
    );
}

// The supersession check above names `Command::Stop` as well as
// `Command::Play`: a Stop that arrives while a Play is downloading means
// the user has already silenced playback, so installing the
// just-downloaded preview would start audio they asked to stop. The queued
// Stop must drop the in-flight preview instead of installing it (it is then
// a no-op, since no player exists). A regression that checked only
// `Command::Play` would install a player and append its preview here, so
// this is the case that pins the Stop half of the check.
#[test]
fn a_preview_superseded_by_stop_during_its_download_is_not_installed() {
    // "first" is still downloading when the user presses Stop, so the Stop
    // is sent from inside the fetch.
    let (players, fetched) = run_with_superseding_command(Command::Stop);

    assert_eq!(
        fetched,
        vec!["first".to_string()],
        "the superseded download still ran; only its preview is dropped"
    );
    assert!(
        players.is_empty(),
        "a preview superseded by Stop must not install a player"
    );
}
