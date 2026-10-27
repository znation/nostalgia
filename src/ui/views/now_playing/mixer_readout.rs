//! The text the LCD shows in place of the title while a mixer slider is being dragged, as in
//! classic Winamp: `Volume: 72%`, `Balance: Center`, `Balance: 31% Left`. An equalizer slider reads
//! `EQ: 60HZ +3 DB` for a band and `Preamp: -12 DB` for the preamp. A seek-slider drag reads
//! `Seek to: 01:23/03:30 (39%)`.

use std::time::Duration;

use super::text::format_lcd_elapsed;
use crate::equalizer::{BAND_COUNT, BAND_FREQUENCIES};

/// The mixer slider currently held down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerDrag {
    Volume,
    Balance,
    EqPreamp,
    EqBand(usize),
}

/// The readout for `drag` at the given clamped `volume` (0.0 to 1.0) and
/// `balance` (-1.0 hard left to 1.0 hard right). A balance that rounds to
/// zero reads `Center`, never `-0%`. Equalizer gains round to whole decibels
/// with an explicit sign; an out-of-range band reads as empty text.
pub fn mixer_readout_text(
    drag: MixerDrag,
    volume: f32,
    balance: f32,
    preamp: f32,
    bands: &[f32; BAND_COUNT],
) -> String {
    match drag {
        MixerDrag::EqPreamp => format!("Preamp: {:+} DB", preamp.round() as i32),
        MixerDrag::EqBand(i) => match (BAND_FREQUENCIES.get(i), bands.get(i)) {
            (Some(label), Some(gain)) => {
                format!("EQ: {label}HZ {:+} DB", gain.round() as i32)
            }
            _ => String::new(),
        },
        MixerDrag::Volume => format!("Volume: {}%", (volume * 100.0).round() as u32),
        MixerDrag::Balance => {
            let pct = (balance * 100.0).round() as i32;
            match pct {
                0 => "Balance: Center".to_string(),
                p if p < 0 => format!("Balance: {}% Left", -p),
                p => format!("Balance: {p}% Right"),
            }
        }
    }
}

/// The readout for a seek drag at `target` in a preview of `length`:
/// `Seek to: 00:12/00:30 (41%)`. The target is clamped to the length and the
/// percentage is the floored share of it, `0` for a zero length.
pub fn seek_readout_text(target: Duration, length: Duration) -> String {
    let target_ms = target.min(length).as_millis() as u64;
    let length_ms = length.as_millis() as u64;
    let pct = (target_ms * 100)
        .checked_div(length_ms)
        .unwrap_or(0)
        .min(100);
    format!(
        "Seek to: {}/{} ({pct}%)",
        format_lcd_elapsed(target_ms),
        format_lcd_elapsed(length_ms)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seek_readout_shows_target_length_and_floored_percent() {
        let secs = Duration::from_secs;
        let len = secs(30);
        assert_eq!(
            seek_readout_text(Duration::from_millis(12_500), len),
            "Seek to: 00:12/00:30 (41%)"
        );
        assert_eq!(
            seek_readout_text(Duration::ZERO, len),
            "Seek to: 00:00/00:30 (0%)"
        );
        assert_eq!(seek_readout_text(len, len), "Seek to: 00:30/00:30 (100%)");
        assert_eq!(
            seek_readout_text(secs(45), len),
            "Seek to: 00:30/00:30 (100%)"
        );
        assert_eq!(
            seek_readout_text(secs(5), Duration::ZERO),
            "Seek to: 00:00/00:00 (0%)"
        );
    }

    #[test]
    fn volume_readout_rounds_to_whole_percent() {
        assert_eq!(
            mixer_readout_text(MixerDrag::Volume, 0.72, 0.0, 0.0, &[0.0; BAND_COUNT]),
            "Volume: 72%"
        );
        assert_eq!(
            mixer_readout_text(MixerDrag::Volume, 0.0, 0.0, 0.0, &[0.0; BAND_COUNT]),
            "Volume: 0%"
        );
        assert_eq!(
            mixer_readout_text(MixerDrag::Volume, 1.0, 0.0, 0.0, &[0.0; BAND_COUNT]),
            "Volume: 100%"
        );
    }

    #[test]
    fn balance_readout_names_the_side() {
        let text = |b| mixer_readout_text(MixerDrag::Balance, 0.5, b, 0.0, &[0.0; BAND_COUNT]);
        assert_eq!(text(0.0), "Balance: Center");
        assert_eq!(text(-0.004), "Balance: Center");
        assert_eq!(text(-0.31), "Balance: 31% Left");
        assert_eq!(text(1.0), "Balance: 100% Right");
    }

    #[test]
    fn eq_readouts_show_the_band_label_and_signed_gain() {
        let mut bands = [0.0; BAND_COUNT];
        bands[0] = 3.0;
        bands[4] = -0.4;
        bands[9] = 12.0;
        let text = |d, p| mixer_readout_text(d, 0.5, 0.0, p, &bands);
        assert_eq!(text(MixerDrag::EqPreamp, -12.0), "Preamp: -12 DB");
        assert_eq!(text(MixerDrag::EqPreamp, 0.0), "Preamp: +0 DB");
        assert_eq!(text(MixerDrag::EqBand(0), 0.0), "EQ: 60HZ +3 DB");
        assert_eq!(text(MixerDrag::EqBand(4), 0.0), "EQ: 1KHZ +0 DB");
        assert_eq!(text(MixerDrag::EqBand(9), 0.0), "EQ: 16KHZ +12 DB");
        assert_eq!(text(MixerDrag::EqBand(BAND_COUNT), 0.0), "");
    }
}
