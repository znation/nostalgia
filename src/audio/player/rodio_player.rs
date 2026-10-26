//! The production [`WorkerPlayer`]: a `rodio::Player` plus the source adapters
//! that meter and equalize each appended preview.
//!
//! The player reads its format and length from the decoded source through
//! [`super::format`] and composes relative and absolute seeks through
//! [`super::seek`]. Its test-only constructors let the source adapters and
//! query gates be exercised without an output device.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use rodio::Source;

use super::super::balance::BalanceSource;
use super::super::equalizer::{EqSettings, EqSnapshot, EqSource};
use super::super::meter::{MeterSource, SpectrumMeter};
use super::super::preview::DecodedPreview;
use super::super::{PlaybackFormat, Spectrum};
use super::format::{duration_for_source, format_for_source};
use super::seek::{clamp_seek, seek_failure_report, seek_player};
use super::worker::WorkerPlayer;

/// Reports `meter`'s live spectrum only once the player holds a preview.
///
/// `loaded` is the player's format cell: it is set only after a preview is
/// appended, so an unloaded player (before its append, or after a stop)
/// reports nothing rather than a window that still holds older audio.
pub(in crate::audio) fn spectrum_for_loaded_player(
    loaded: bool,
    meter: &SpectrumMeter,
) -> Option<Spectrum> {
    loaded.then(|| meter.take())
}

/// The production [`WorkerPlayer`]: a `rodio::Player` plus the balance and
/// equalizer shared with the source adapters wrapping each preview it appends.
pub(in crate::audio) struct RodioPlayer {
    player: rodio::Player,
    balance: Arc<AtomicU32>,
    equalizer: Arc<EqSettings>,
    /// The format of the preview appended to this player, set once because
    /// each played track builds a fresh player.
    format: OnceLock<PlaybackFormat>,
    /// The appended preview's own length, set once beside the format. It can
    /// be absent even with a player loaded when the source cannot report it.
    duration: OnceLock<Option<Duration>>,
    /// The mono window every sample this player pulls is recorded into for
    /// the LCD visualizer.
    spectrum: Arc<SpectrumMeter>,
}

impl RodioPlayer {
    /// Builds a player around an already opened `rodio::Player`, with a
    /// neutral balance, the default equalizer settings, empty format and
    /// length cells, and a fresh spectrum meter.
    pub(in crate::audio) fn new(player: rodio::Player) -> Self {
        Self {
            player,
            balance: Arc::new(AtomicU32::new(0.0_f32.to_bits())),
            equalizer: Arc::new(EqSettings::default()),
            format: OnceLock::new(),
            duration: OnceLock::new(),
            spectrum: Arc::new(SpectrumMeter::default()),
        }
    }

    /// Wraps `source` in the meter and equalizer chain and appends it, setting
    /// the format and length cells from the source itself. Production appends
    /// a decoded preview through here; keeping the chain generic lets a test
    /// drive the same wiring with an in-memory source.
    pub(in crate::audio) fn append_source<S>(&self, source: S, encoded_bytes: u64)
    where
        S: Source<Item = f32> + Send + 'static,
    {
        let _ = self.format.set(format_for_source(&source, encoded_bytes));
        let _ = self.duration.set(duration_for_source(&source));
        let metered = MeterSource::new(source, Arc::clone(&self.spectrum));
        let equalized = EqSource::new(metered, Arc::clone(&self.equalizer));
        self.player
            .append(BalanceSource::new(equalized, Arc::clone(&self.balance)));
    }
}

#[cfg(test)]
impl RodioPlayer {
    /// Builds a player without an output device, sharing `spectrum` with the
    /// caller and returning the queue its appended audio is pulled from. The
    /// format cell stays empty until an append, as it does in production.
    pub(in crate::audio) fn without_device(
        spectrum: Arc<SpectrumMeter>,
    ) -> (Self, rodio::queue::SourcesQueueOutput) {
        let (player, queue) = rodio::Player::new();
        (
            Self {
                player,
                balance: Arc::new(AtomicU32::new(0.0_f32.to_bits())),
                equalizer: Arc::new(EqSettings::default()),
                format: OnceLock::new(),
                duration: OnceLock::new(),
                spectrum,
            },
            queue,
        )
    }

    /// Sets the format cell the way an append does, without needing a decoded
    /// preview.
    pub(in crate::audio) fn mark_loaded_for_test(&self) {
        let _ = self.format.set(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 1,
            bitrate_kbps: None,
        });
    }
}

impl WorkerPlayer for RodioPlayer {
    type Preview = DecodedPreview;

    fn set_volume(&self, volume: f32) {
        self.player.set_volume(volume);
    }

    fn set_balance(&self, balance: f32) {
        self.balance.store(balance.to_bits(), Ordering::Relaxed);
    }

    fn seek_by(&self, offset_secs: i64) {
        seek_player(&self.player, offset_secs);
    }

    fn seek_to(&self, position: Duration) {
        let target = clamp_seek(position, self.duration());
        if let Err(error) = self.player.try_seek(target) {
            eprintln!("{}", seek_failure_report(target, &error));
        }
    }

    fn position(&self) -> Duration {
        self.player.get_pos()
    }

    fn duration(&self) -> Option<Duration> {
        self.duration.get().copied().flatten()
    }

    fn playback_format(&self) -> Option<PlaybackFormat> {
        self.format.get().copied()
    }

    fn spectrum(&self) -> Option<Spectrum> {
        // The format cell is set only after a preview is appended, so a player
        // without one (before its append, or after a stop) reports nothing
        // rather than a window that still holds older audio.
        spectrum_for_loaded_player(self.format.get().is_some(), &self.spectrum)
    }

    fn is_finished(&self) -> bool {
        // `empty` also holds before a preview is ever appended, so the format
        // gate keeps a fresh player from reporting a finished preview.
        self.format.get().is_some() && self.player.empty()
    }

    fn set_equalizer(&self, equalizer: EqSnapshot) {
        self.equalizer.store(equalizer);
    }

    fn append(&self, preview: Self::Preview) {
        // Read the format and length from the decoded source before moving it
        // into the adapter chain; the encoded length comes from the download
        // so the format can report the bitrate. A fresh player per track sets
        // each cell once.
        let DecodedPreview {
            source,
            encoded_bytes,
            ..
        } = preview;
        self.append_source(source, encoded_bytes);
    }

    fn pause(&self) {
        self.player.pause();
    }

    fn play(&self) {
        self.player.play();
    }

    fn stop(&self) {
        self.player.stop();
    }
}
