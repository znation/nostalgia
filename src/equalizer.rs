//! The equalizer's data model: the ten Winamp frequency bands, the shared
//! gain range they move within, and the pure gain clamp the shared state
//! applies. Kept in its own module beside `library` and `state` so both the
//! state and the UI can depend on it without a `state` ← `ui` cycle.

use crate::clamp;

/// The centre frequencies of Winamp's ten equalizer bands, low to high, as
/// the short display labels printed under each slider (Winamp shows the
/// hundreds-of-hertz bands in full and abbreviates the kilohertz ones).
pub const BAND_FREQUENCIES: [&str; 10] = [
    "60", "170", "310", "600", "1K", "3K", "6K", "12K", "14K", "16K",
];

/// The number of equalizer bands. Derived from [`BAND_FREQUENCIES`] so the
/// band array in the shared state and the sliders the UI builds can never
/// drift out of step with the label list.
pub const BAND_COUNT: usize = BAND_FREQUENCIES.len();

/// The lowest band gain in decibels, the bottom of every Winamp equalizer
/// slider's range.
pub const GAIN_MIN_DB: f32 = -12.0;

/// The highest band gain in decibels, the top of every Winamp equalizer
/// slider's range.
pub const GAIN_MAX_DB: f32 = 12.0;

/// A named equalizer curve: a preamp gain plus one gain per
/// [`BAND_COUNT`] band, in [`BAND_FREQUENCIES`] order.
///
/// `Copy` so a selection can be read out of shared state and passed to the
/// view by value, and `Display` so iced's `PickList` can render the name in
/// its selected label and its menu. The stored gains are the raw Winamp 2.x
/// curve values; [`crate::state::AppState::apply_eq_preset`] still runs them
/// through [`clamp_gain`] before storing, so a preset can never push shared
/// state out of range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preset {
    /// The preset's display name, shown in the equalizer's pick list.
    pub name: &'static str,
    /// The preamp gain in decibels, applied ahead of the bands.
    pub preamp: f32,
    /// The per-band gains in decibels, low frequency to high.
    pub bands: [f32; BAND_COUNT],
}

impl std::fmt::Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}

/// The Winamp 2.x equalizer preset curves, in base-skin order.
///
/// The classic Winamp preset list, from Flat through Vocal, with the
/// documented curve values (Winamp's preset data stores every preamp at
/// `0.0` and puts the character in the ten band gains). This is the source
/// list the equalizer panel's pick list offers; selecting one applies the
/// whole curve through [`crate::state::AppState::apply_eq_preset`].
pub const PRESETS: [Preset; 19] = [
    Preset {
        name: "Flat",
        preamp: 0.0,
        bands: [0.0; BAND_COUNT],
    },
    Preset {
        name: "Classical",
        preamp: 0.0,
        bands: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -4.4, -4.4, -4.4, -5.6],
    },
    Preset {
        name: "Club",
        preamp: 0.0,
        bands: [0.0, 0.0, 8.0, 5.6, 5.6, 5.6, 3.1, 0.0, 0.0, 0.0],
    },
    Preset {
        name: "Dance",
        preamp: 0.0,
        bands: [5.6, 7.5, 4.4, 0.0, 0.0, -5.6, -7.5, -7.5, 0.0, 0.0],
    },
    Preset {
        name: "Full Bass",
        preamp: 0.0,
        bands: [8.7, 8.7, 8.7, 5.6, 1.9, -4.4, -7.5, -8.1, -8.7, -8.7],
    },
    Preset {
        name: "Full Bass & Treble",
        preamp: 0.0,
        bands: [7.5, 5.6, 0.0, -7.5, -4.4, 1.9, 8.7, 8.7, 8.7, 8.7],
    },
    Preset {
        name: "Full Treble",
        preamp: 0.0,
        bands: [-9.4, -9.4, -9.4, -4.4, 1.9, 8.7, 9.4, 9.4, 9.4, 10.0],
    },
    Preset {
        name: "Headphones",
        preamp: 0.0,
        bands: [4.4, 7.5, 5.6, 0.0, -3.1, -1.9, 1.9, 5.6, 8.1, 8.7],
    },
    Preset {
        name: "Large Hall",
        preamp: 0.0,
        bands: [8.7, 8.7, 5.6, 5.6, 0.0, -5.6, -5.6, -5.6, 0.0, 0.0],
    },
    Preset {
        name: "Live",
        preamp: 0.0,
        bands: [-4.4, -1.9, 0.0, 3.1, 5.6, 5.6, 5.6, 3.1, 1.9, 1.9],
    },
    Preset {
        name: "Party",
        preamp: 0.0,
        bands: [7.5, 7.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 7.5, 7.5],
    },
    Preset {
        name: "Pop",
        preamp: 0.0,
        bands: [-1.9, 1.9, 4.4, 5.6, 5.6, 0.0, -1.9, -1.9, -1.9, -1.9],
    },
    Preset {
        name: "Reggae",
        preamp: 0.0,
        bands: [0.0, 0.0, 0.0, -4.4, 0.0, 4.4, 4.4, 0.0, 0.0, 0.0],
    },
    Preset {
        name: "Rock",
        preamp: 0.0,
        bands: [8.1, 4.4, -5.6, -7.5, -3.1, 4.4, 8.7, 11.2, 11.2, 11.2],
    },
    Preset {
        name: "Ska",
        preamp: 0.0,
        bands: [-2.5, -4.4, -2.5, 0.0, 0.0, 0.0, 0.0, -2.5, -4.4, -5.6],
    },
    Preset {
        name: "Soft",
        preamp: 0.0,
        bands: [4.4, 1.9, 0.0, -1.9, -1.9, 0.0, 4.4, 8.1, 8.1, 8.1],
    },
    Preset {
        name: "Soft Rock",
        preamp: 0.0,
        bands: [4.4, 4.4, 1.9, -1.9, -3.1, -1.9, 1.9, 5.6, 8.1, 8.1],
    },
    Preset {
        name: "Techno",
        preamp: 0.0,
        bands: [4.4, 1.9, 0.0, -1.9, -4.4, -1.9, 4.4, 8.1, 8.1, 8.1],
    },
    Preset {
        name: "Vocal",
        preamp: 0.0,
        bands: [-1.9, -3.1, -3.1, 1.9, 5.6, 5.6, 5.6, 3.1, 0.0, 0.0],
    },
];

/// Clamps an equalizer gain to the valid `[GAIN_MIN_DB, GAIN_MAX_DB]` range,
/// mapping a NaN to `0.0` (flat).
///
/// iced's sliders can emit a value outside the range (a drag beyond the ends,
/// or a stale in-flight change), and the UI must never store an unclamped
/// gain in `AppState`. The gains are shared state, so the rule lives beside
/// the band constants as a pure function the `AppState` gain setters call
/// before storing, exactly as `state::clamp_volume` does for volume. The
/// NaN-and-bounds behaviour itself is
/// [`crate::clamp::clamp_with_nan_fallback`], shared with
/// `state::clamp_volume`; this wrapper only names the gain range and its flat
/// fallback.
#[must_use]
pub fn clamp_gain(gain: f32) -> f32 {
    clamp::clamp_with_nan_fallback(gain, GAIN_MIN_DB, GAIN_MAX_DB, 0.0)
}

#[cfg(test)]
mod tests {
    use super::{BAND_COUNT, BAND_FREQUENCIES, GAIN_MAX_DB, GAIN_MIN_DB, PRESETS, clamp_gain};
    use crate::test_support::rock_preset;

    #[test]
    fn band_frequencies_match_the_band_count() {
        assert_eq!(BAND_FREQUENCIES.len(), BAND_COUNT);
    }

    // `BAND_FREQUENCIES` is the equalizer's user-visible content: `view_equalizer`
    // renders one label under each of the ten sliders, and the base skin shows
    // the hundreds-of-hertz bands in full and abbreviates the kilohertz ones.
    // The length test above compares the array to itself (`BAND_COUNT` is
    // defined as `BAND_FREQUENCIES.len()`), so it can never fail; a typo or a
    // reordered band would pass every test while the panel silently showed the
    // wrong frequency. Pin the exact labels, low to high.
    #[test]
    fn band_frequencies_are_the_winamp_labels_in_order() {
        assert_eq!(
            BAND_FREQUENCIES,
            [
                "60", "170", "310", "600", "1K", "3K", "6K", "12K", "14K", "16K"
            ]
        );
    }

    // `GAIN_MIN_DB`/`GAIN_MAX_DB` are the equalizer's user-visible range:
    // every band slider spans them (`view_equalizer`) and both `clamp_gain`
    // and `AppState` clamp to them. The tests around this one only compare the
    // constants to themselves or to values they themselves define, so editing
    // either literal would pass the whole suite while the panel silently
    // changed range. Pin the documented -12 dB..=+12 dB span and the ordering
    // it promises: min below flat, flat below max, symmetric about 0.
    #[test]
    fn gain_range_is_the_documented_twelve_db_span() {
        assert_eq!(GAIN_MIN_DB, -12.0);
        assert_eq!(GAIN_MAX_DB, 12.0);
        // Bound the constants to locals: `assert!` on a bare constant trips
        // `clippy::assertions_on_constants`.
        let min = GAIN_MIN_DB;
        let max = GAIN_MAX_DB;
        assert!(min < 0.0);
        assert!(0.0 < max);
        assert_eq!(min, -max);
    }

    #[test]
    fn clamp_gain_caps_at_the_upper_bound() {
        assert_eq!(clamp_gain(99.0), GAIN_MAX_DB);
    }

    #[test]
    fn clamp_gain_floors_at_the_lower_bound() {
        assert_eq!(clamp_gain(-99.0), GAIN_MIN_DB);
    }

    #[test]
    fn clamp_gain_passes_through_in_range() {
        assert_eq!(clamp_gain(GAIN_MIN_DB), GAIN_MIN_DB);
        assert_eq!(clamp_gain(0.0), 0.0);
        assert_eq!(clamp_gain(3.5), 3.5);
        assert_eq!(clamp_gain(GAIN_MAX_DB), GAIN_MAX_DB);
    }

    #[test]
    fn clamp_gain_treats_nan_as_flat() {
        // The shared clamp handles the NaN branch; this pins that
        // `clamp_gain` passes the flat fallback.
        assert_eq!(clamp_gain(f32::NAN), 0.0);
    }

    // `PRESETS` is the equalizer's user-visible pick-list content: iced's
    // `PickList` renders each preset's `Display` name and the app shows the
    // selected one in the equalizer header. A duplicate or empty name would
    // make two entries indistinguishable (and the selection ambiguous), so
    // pin both here rather than relying on the exact-curve test below.
    #[test]
    fn presets_have_unique_non_empty_names() {
        for (index, preset) in PRESETS.iter().enumerate() {
            assert!(!preset.name.is_empty(), "preset {index} has an empty name");
            for other in &PRESETS[index + 1..] {
                assert_ne!(preset.name, other.name, "duplicate preset name");
            }
        }
    }

    // The test above reads the `name` field directly, and the
    // `view_equalizer` construction test never lays the widget out, so
    // nothing in the crate ever calls `Preset`'s `Display`. That impl is
    // what iced's `PickList` actually renders for the selected preset and
    // for every menu option (see the struct doc), so a `Display` that
    // returned anything but `name` — a debug repr, say — would leave the
    // whole suite green while the pick list showed the wrong label. Pin the
    // rendered label for every preset.
    #[test]
    fn preset_display_renders_the_name() {
        for preset in PRESETS {
            assert_eq!(preset.to_string(), preset.name);
        }
    }

    // The curves are raw Winamp data, but `apply_eq_preset` still runs every
    // gain through `clamp_gain` before storing. A preset with a value outside
    // the slider range would then be silently altered on selection, so pin
    // that every stored gain is already its own clamp.
    #[test]
    fn preset_gains_stay_within_the_clamp_range() {
        for preset in PRESETS {
            assert_eq!(preset.preamp, clamp_gain(preset.preamp), "{}", preset.name);
            for gain in preset.bands {
                assert_eq!(gain, clamp_gain(gain), "{}", preset.name);
            }
        }
    }

    #[test]
    fn flat_preset_is_all_zero() {
        let flat = PRESETS[0];
        assert_eq!(flat.name, "Flat");
        assert_eq!(flat.preamp, 0.0);
        assert_eq!(flat.bands, [0.0; BAND_COUNT]);
    }

    // The flat preset's all-zero shape is pinned above, but the character
    // curves are the point of the table: a transposed or mistyped band would
    // pass the range and uniqueness tests while applying the wrong sound.
    // Pin the classic Rock curve, name and every band, as the representative
    // exact-array check.
    #[test]
    fn rock_preset_pins_the_classic_curve() {
        let rock = rock_preset();
        assert_eq!(rock.preamp, 0.0);
        assert_eq!(
            rock.bands,
            [8.1, 4.4, -5.6, -7.5, -3.1, 4.4, 8.7, 11.2, 11.2, 11.2]
        );
    }
}
