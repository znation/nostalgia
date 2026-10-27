use super::*;

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

// A failed seek leaves playback where it was, so the meter must not clear the
// window it recorded from the audio still playing. The source above always
// seeks successfully, so nothing reached `try_seek`'s `?` and this pins that
// the clear follows a successful inner seek rather than running first.
#[test]
fn meter_source_failed_seek_keeps_the_recorded_window() {
    let samples = vec![0.5, -0.5, 0.25, -0.25];
    let meter = Arc::new(SpectrumMeter::default());
    let mut source = MeterSource::new(SeekableSource::failing(samples), Arc::clone(&meter));

    let _: Vec<f32> = source.by_ref().take(4).collect();
    let before = meter.recorded();
    assert!(
        !before.is_empty(),
        "the meter must record the samples it pulled"
    );

    assert!(
        source.try_seek(Duration::ZERO).is_err(),
        "a failed inner seek must be reported to the caller"
    );
    assert_eq!(
        meter.recorded(),
        before,
        "a failed seek must leave the recorded window intact"
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

// The doc comment promises the nearest kilobit, not a truncation: 1000 bytes
// (8000 bits) over 3 ms is 2666.67 kbit/s and over 6 ms is 1333.33. A result
// wider than a u32 is reported as unknown rather than wrapped or truncated.
#[test]
fn preview_bitrate_kbps_rounds_to_the_nearest_kilobit_and_rejects_overflow() {
    assert_eq!(
        preview_bitrate_kbps(1_000, Duration::from_millis(3)),
        Some(2_667)
    );
    assert_eq!(
        preview_bitrate_kbps(1_000, Duration::from_millis(6)),
        Some(1_333)
    );
    // 536,870,911 bytes is 4,294,967,288 bits over 1 ms: still a u32.
    assert_eq!(
        preview_bitrate_kbps(536_870_911, Duration::from_millis(1)),
        Some(4_294_967_288)
    );
    // One more byte crosses u32::MAX, so the guard reports no bitrate.
    assert_eq!(
        preview_bitrate_kbps(536_870_912, Duration::from_millis(1)),
        None
    );
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

// A rejected seek's report must name the absolute target as well as the
// backend's cause; the target is what distinguishes a failed forward step
// from a failed backward one or a seek-bar release, and the bare cause alone
// cannot. `15s` is `Duration`'s `Debug` form, which the report uses.
#[test]
fn seek_failure_report_names_the_target_and_the_cause() {
    assert_eq!(
        seek_failure_report(Duration::from_secs(15), &"seek refused"),
        "seeking the preview to 15s failed: seek refused"
    );
}

/// Runs `samples` through a `BalanceSource` set to `balance` and returns the output.
fn balanced(channels: u16, samples: Vec<f32>, balance: f32) -> Vec<f32> {
    let raw = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(channels).unwrap(),
        SampleRate::new(44_100).unwrap(),
        samples,
    );
    let balance = Arc::new(AtomicU32::new(balance.to_bits()));
    BalanceSource::new(raw, balance).collect()
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
    assert_eq!(balanced(2, vec![0.5_f32, 0.25], -1.0_f32), vec![0.5, 0.0]);
}

#[test]
fn balance_source_hard_right_silences_the_left_channel() {
    assert_eq!(balanced(2, vec![0.5_f32, 0.25], 1.0_f32), vec![0.0, 0.25]);
}

#[test]
fn balance_source_partial_right_attenuates_the_left_channel() {
    assert_eq!(balanced(2, vec![0.5_f32, 0.25], 0.5_f32), vec![0.25, 0.25]);
}

#[test]
fn balance_source_partial_left_attenuates_the_right_channel() {
    assert_eq!(balanced(2, vec![0.5_f32, 0.25], -0.5_f32), vec![0.5, 0.125]);
}

#[test]
fn balance_source_attenuates_a_mono_source() {
    assert_eq!(balanced(1, vec![0.5_f32, -0.5], 0.5_f32), vec![0.25, -0.25]);
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

// The filter state is cleared only after a successful inner seek, so a seek
// the source rejects must not hide the audio still playing: the boosted band's
// ringing continues into the next sample instead of the failed seek rebuilding
// the filters. `eq_source_seek_clears_filter_state` covers the success path.
#[test]
fn eq_source_failed_seek_keeps_the_filter_state() {
    let mut bands = [0.0; crate::equalizer::BAND_COUNT];
    bands[4] = 12.0;
    let settings = Arc::new(EqSettings::default());
    settings.store(EqSnapshot {
        enabled: true,
        preamp_db: 0.0,
        bands_db: bands,
    });
    let samples = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let mut source = EqSource::new(SeekableSource::failing(samples), settings);

    // Consume the impulse and the right channel after it, leaving the left
    // channel's boosted filter ringing.
    let _ = source.next();
    let _ = source.next();

    assert!(
        source.try_seek(Duration::ZERO).is_err(),
        "a failed inner seek must be reported to the caller"
    );
    assert_ne!(
        source.next(),
        Some(0.0),
        "a failed seek must not rebuild the filter and drop its ringing state"
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
