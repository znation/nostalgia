//! The text the LCD shows in place of the title while a mixer slider is
//! being dragged, as in classic Winamp: `Volume: 72%`, `Balance: Center`,
//! `Balance: 31% Left`.

/// The mixer slider currently held down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerDrag {
    Volume,
    Balance,
}

/// The readout for `drag` at the given clamped `volume` (0.0 to 1.0) and
/// `balance` (-1.0 hard left to 1.0 hard right). A balance that rounds to
/// zero reads `Center`, never `-0%`.
pub fn mixer_readout_text(drag: MixerDrag, volume: f32, balance: f32) -> String {
    match drag {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_readout_rounds_to_whole_percent() {
        assert_eq!(
            mixer_readout_text(MixerDrag::Volume, 0.72, 0.0),
            "Volume: 72%"
        );
        assert_eq!(
            mixer_readout_text(MixerDrag::Volume, 0.0, 0.0),
            "Volume: 0%"
        );
        assert_eq!(
            mixer_readout_text(MixerDrag::Volume, 1.0, 0.0),
            "Volume: 100%"
        );
    }

    #[test]
    fn balance_readout_names_the_side() {
        let text = |b| mixer_readout_text(MixerDrag::Balance, 0.5, b);
        assert_eq!(text(0.0), "Balance: Center");
        assert_eq!(text(-0.004), "Balance: Center");
        assert_eq!(text(-0.31), "Balance: 31% Left");
        assert_eq!(text(1.0), "Balance: 100% Right");
    }
}
