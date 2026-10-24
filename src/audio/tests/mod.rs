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
    MAX_PREVIEW_BYTES, PREVIEW_TIMEOUT, PreviewCache, PublicAddressResolver, download_and_decode,
    preview_agent, preview_agent_with_timeout,
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

/// Builds a full-scale sine at `frequency`, `len` samples long.
fn sine_samples(frequency: f32, sample_rate: u32, len: usize) -> Vec<f32> {
    (0..len)
        .map(|index| (std::f32::consts::TAU * frequency * index as f32 / sample_rate as f32).sin())
        .collect()
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

mod preview_download;
mod rodio_output;
mod sources;
mod worker;
