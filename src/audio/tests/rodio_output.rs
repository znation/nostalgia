use super::*;

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

// The silent fallback has no preview length to report, so the seek bar stays
// blank rather than inventing a scale.
#[test]
fn silent_output_reports_no_playback_duration() {
    assert_eq!(SilentOutput.duration(), None);

    let output = audio_with_fallback(|| Err(AppleMusicError::new("no device")));
    assert_eq!(output.duration(), None);
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

/// A `RodioOutput` wired to a stand-in worker thread that receives the first
/// command and hands it to `handle`. Join the returned handle after the query.
fn output_with_worker(
    handle: impl FnOnce(Command) + Send + 'static,
) -> (RodioOutput, std::thread::JoinHandle<()>) {
    let (commands, receiver) = std::sync::mpsc::channel();
    let output = RodioOutput {
        commands: Mutex::new(commands),
    };
    let worker = std::thread::spawn(move || {
        handle(receiver.recv().expect("the query sent no command"));
    });
    (output, worker)
}

/// Run a query against a worker that accepts it (via `handle`) but never
/// answers, and assert it returns within the seam's bound. `what` names the
/// query in the failure message.
fn assert_unanswered_query_times_out<T: std::fmt::Debug + PartialEq>(
    what: &str,
    handle: impl FnOnce(Command) + Send + 'static,
    query: impl FnOnce(&RodioOutput) -> T,
    expected: T,
) {
    let (output, worker) = output_with_worker(handle);
    let started = Instant::now();
    assert_eq!(query(&output), expected);
    assert!(
        started.elapsed() < WORKER_QUERY_TIMEOUT * 5,
        "a missing {what} reply must time out instead of blocking"
    );
    worker.join().unwrap();
}

// A position query crosses to the worker and returns the reply it sends,
// rather than reading anything on the caller's thread.
#[test]
fn rodio_output_asks_the_worker_for_its_position() {
    let (output, worker) = output_with_worker(|command| match command {
        Command::Position(reply) => reply.send(Some(Duration::from_secs(3))).unwrap(),
        _ => panic!("position did not send Command::Position"),
    });

    assert_eq!(output.position(), Some(Duration::from_secs(3)));
    worker.join().unwrap();
}

// A format query crosses to the worker and returns the reply it sends, just
// like the position query.
#[test]
fn rodio_output_asks_the_worker_for_its_format() {
    let format = PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    };
    let (output, worker) = output_with_worker(move |command| match command {
        Command::PlaybackFormat(reply) => reply.send(Some(format)).unwrap(),
        _ => panic!("format did not send Command::PlaybackFormat"),
    });

    assert_eq!(output.playback_format(), Some(format));
    worker.join().unwrap();
}

// A spectrum query crosses to the worker and returns the reply it sends, just
// like the position, format, and duration queries.
#[test]
fn rodio_output_asks_the_worker_for_its_spectrum() {
    let spectrum = test_spectrum();
    let (output, worker) = output_with_worker(move |command| match command {
        Command::Spectrum(reply) => reply.send(Some(spectrum)).unwrap(),
        _ => panic!("spectrum did not send Command::Spectrum"),
    });

    assert_eq!(output.spectrum(), Some(spectrum));
    worker.join().unwrap();
}

// A finished query crosses to the worker and returns the reply it sends, just
// like the position, format, spectrum, and duration queries.
#[test]
fn rodio_output_asks_the_worker_for_its_finished_state() {
    let (output, worker) = output_with_worker(|command| match command {
        Command::Finished(reply) => reply.send(Some(true)).unwrap(),
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
    assert_unanswered_query_times_out(
        "finished",
        |command| match command {
            Command::Finished(reply) => {
                // Keep the reply channel alive well past the seam's bound, as
                // a worker busy installing a preview would.
                std::thread::sleep(WORKER_QUERY_TIMEOUT * 10);
                drop(reply);
            }
            _ => panic!("finished did not send Command::Finished"),
        },
        RodioOutput::is_finished,
        false,
    );
}

// A worker that accepts the query but does not answer must not hold the
// caller past the seam's bound: the query reports `None` instead of waiting
// for the reply.
#[test]
fn rodio_output_reports_none_when_the_position_reply_times_out() {
    assert_unanswered_query_times_out(
        "position",
        |command| match command {
            Command::Position(reply) => {
                std::thread::sleep(WORKER_QUERY_TIMEOUT * 10);
                drop(reply);
            }
            _ => panic!("position did not send Command::Position"),
        },
        RodioOutput::position,
        None,
    );
}

// A duration query crosses to the worker and returns the reply it sends,
// just like the position and format queries.
#[test]
fn rodio_output_asks_the_worker_for_its_duration() {
    let (output, worker) = output_with_worker(|command| match command {
        Command::Duration(reply) => reply.send(Some(Duration::from_secs(30))).unwrap(),
        _ => panic!("duration did not send Command::Duration"),
    });

    assert_eq!(output.duration(), Some(Duration::from_secs(30)));
    worker.join().unwrap();
}

// A worker that accepts the duration query but does not answer must not hold
// the caller past the seam's bound: the query reports `None` instead.
#[test]
fn rodio_output_reports_none_when_the_duration_reply_times_out() {
    assert_unanswered_query_times_out(
        "duration",
        |command| match command {
            Command::Duration(reply) => {
                std::thread::sleep(WORKER_QUERY_TIMEOUT * 10);
                drop(reply);
            }
            _ => panic!("duration did not send Command::Duration"),
        },
        RodioOutput::duration,
        None,
    );
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
