//! The silent stand-in for the audio-output seam: [`SilentOutput`] and the
//! [`audio_with_fallback`] opener that substitutes it when no device opens.

use std::sync::Arc;
use std::time::Duration;

use super::{AudioOutput, EqSnapshot};
use crate::music_error::AppleMusicError;

/// An [`AudioOutput`] that produces no sound: it logs each command and reports
/// success.
///
/// This is the stand-in when no output device opens (see
/// [`audio_with_fallback`]) and the backend the browse-only tests use, so the
/// service can be driven end to end without touching an audio device.
#[derive(Debug, Default, Clone, Copy)]
pub struct SilentOutput;

impl AudioOutput for SilentOutput {
    fn play(&self, url: &str) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; not playing {url:?}");
        Ok(())
    }

    fn pause(&self) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; nothing to pause");
        Ok(())
    }

    fn resume(&self) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; nothing to resume");
        Ok(())
    }

    fn stop(&self) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; nothing to stop");
        Ok(())
    }

    fn set_volume(&self, volume: f32) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; volume set to {volume}");
        Ok(())
    }

    fn set_balance(&self, balance: f32) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; balance set to {balance}");
        Ok(())
    }

    fn seek_by(&self, offset_secs: i64) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; nothing to seek by {offset_secs}s");
        Ok(())
    }

    fn seek_to(&self, position: Duration) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; nothing to seek to {position:?}");
        Ok(())
    }

    fn set_equalizer(&self, equalizer: EqSnapshot) -> Result<(), AppleMusicError> {
        println!("Audio output is silent; equalizer set to {equalizer:?}");
        Ok(())
    }
}

/// Runs `open` and returns its output, or a [`SilentOutput`] when it fails.
///
/// The service builds its [`super::RodioOutput`] through this so a machine with no
/// audio device still starts and browses; the failed opener is logged once.
/// Pure and injectable — the opener is a parameter — so the fallback is
/// testable without a device.
pub(crate) fn audio_with_fallback(
    open: impl FnOnce() -> Result<Arc<dyn AudioOutput>, AppleMusicError>,
) -> Arc<dyn AudioOutput> {
    match open() {
        Ok(output) => output,
        Err(error) => {
            eprintln!("audio output unavailable, falling back to silence: {error}");
            Arc::new(SilentOutput)
        }
    }
}
