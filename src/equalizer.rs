//! The equalizer's data model: the ten Winamp frequency bands, the shared
//! gain range they move within, and the pure gain clamp the shared state
//! applies. Kept in its own module beside `library` and `state` so both the
//! state and the UI can depend on it without a `state` ← `ui` cycle. The
//! per-channel [`BandFilter`] that applies these bands lives in `filter`.

use crate::clamp;

mod filter;

pub use filter::BandFilter;

/// The center frequencies of Winamp's ten equalizer bands, low to high, as
/// the short display labels printed under each slider (Winamp shows the
/// hundreds-of-hertz bands in full and abbreviates the kilohertz ones).
pub const BAND_FREQUENCIES: [&str; 10] = [
    "60", "170", "310", "600", "1K", "3K", "6K", "12K", "14K", "16K",
];

/// The number of equalizer bands. Derived from [`BAND_FREQUENCIES`] so the
/// band array in the shared state and the sliders the UI builds can never
/// drift out of step with the label list.
pub const BAND_COUNT: usize = BAND_FREQUENCIES.len();

/// The numeric center frequency of each band, in hertz and in
/// [`BAND_FREQUENCIES`] order, matching the labels the panel shows.
///
/// [`BandFilter`] builds one peaking biquad per entry, so changing a label
/// without this table would leave the panel and the sound out of step.
pub const BAND_HZ: [f32; BAND_COUNT] = [
    60.0, 170.0, 310.0, 600.0, 1_000.0, 3_000.0, 6_000.0, 12_000.0, 14_000.0, 16_000.0,
];

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

/// A named curve the player itself saved: a display name plus the same
/// preamp-and-band shape as [`Preset`].
///
/// `Preset`'s own `name` is a `&'static str`, so a runtime-named curve gets
/// its own type rather than a compile-time-name copy. `from_curve` clamps
/// every gain at construction, so a stored curve — built from the shared
/// state or parsed back from the settings file — can never leave
/// [`GAIN_MIN_DB`]..=[`GAIN_MAX_DB`]. `Display` and `PartialEq` let iced's
/// `PickList` render the name and the view code compare choices, like
/// [`Preset`]; `Clone` because a `String` name cannot be `Copy`.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomPreset {
    /// The curve's display name, shown in the equalizer's pick list.
    pub name: String,
    /// The preamp gain in decibels, applied ahead of the bands.
    pub preamp: f32,
    /// The per-band gains in decibels, low frequency to high.
    pub bands: [f32; BAND_COUNT],
}

impl CustomPreset {
    /// Builds a custom preset named `name` with [`clamp_gain`] applied over
    /// the preamp and every band, so a curve built from a hand-edited settings
    /// line or a NaN slider value still stores only in-range gains.
    #[must_use]
    pub fn from_curve(name: String, preamp: f32, bands: [f32; BAND_COUNT]) -> Self {
        Self {
            name,
            preamp: clamp_gain(preamp),
            bands: bands.map(clamp_gain),
        }
    }
}

impl std::fmt::Display for CustomPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

/// The preset pick list's top row: a save trigger, not a curve. Selecting it
/// opens the transient Save preset naming panel instead of applying anything
/// (see `crate::ui::handlers::mixer::eq_preset_selected`; `handlers` is a
/// private `ui` module, so this is a plain path, not a link).
pub const SAVE_PRESET_LABEL: &str = "Save preset\u{2026}";

/// One equalizer pick-list item: the save row, a built-in curve, or a
/// name the user saved.
///
/// `Display` renders each variant's row label — [`SAVE_PRESET_LABEL`] for
/// `Save`, the curve's own name otherwise — and `PartialEq + Clone` match
/// iced's `PickList` item requirements; no `Copy`, because [`CustomPreset`]
/// holds a `String`. The `Preset`-taking state path
/// ([`crate::state::AppState::apply_eq_preset`] and the mixer handlers') is
/// unchanged: only the pick list's items and its message carry the choice.
#[derive(Debug, Clone, PartialEq)]
pub enum PresetChoice {
    /// The pick list's top row, which opens the Save preset naming panel.
    Save,
    /// One of the built-in [`PRESETS`] curves.
    BuiltIn(Preset),
    /// One of the player's saved [`CustomPreset`] curves.
    Custom(CustomPreset),
}

impl std::fmt::Display for PresetChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Save => f.write_str(SAVE_PRESET_LABEL),
            Self::BuiltIn(preset) => write!(f, "{preset}"),
            Self::Custom(preset) => write!(f, "{preset}"),
        }
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

/// The preset named after `genre`, or `None` when the trimmed genre is blank
/// or matches no preset name.
///
/// Winamp's AUTO button loads the preset whose name matches the playing
/// track's primary genre. The comparison ignores surrounding whitespace and
/// ASCII case, so a source's `" rock "` selects the `Rock` curve while an
/// unmatched genre such as `Alternative` leaves the current curve alone.
#[must_use]
pub fn preset_for_genre(genre: &str) -> Option<Preset> {
    let genre = genre.trim();
    if genre.is_empty() {
        return None;
    }
    PRESETS
        .iter()
        .find(|preset| preset.name.eq_ignore_ascii_case(genre))
        .copied()
}

/// Clamps an equalizer gain to the valid `[GAIN_MIN_DB, GAIN_MAX_DB]` range,
/// mapping a NaN to `0.0` (flat).
///
/// iced's sliders can emit a value outside the range (a drag beyond the ends,
/// or a stale in-flight change), and the UI must never store an unclamped
/// gain in `AppState`. The gains are shared state, so the rule lives beside
/// the band constants as a pure function the `AppState` gain setters call
/// before storing, exactly as `state::clamp_volume` does for volume. The
/// NaN-and-bounds behavior itself is
/// [`crate::clamp::clamp_with_nan_fallback`], shared with
/// `state::clamp_volume`; this wrapper only names the gain range and its flat
/// fallback.
#[must_use]
pub fn clamp_gain(gain: f32) -> f32 {
    clamp::clamp_with_nan_fallback(gain, GAIN_MIN_DB, GAIN_MAX_DB, 0.0)
}

#[cfg(test)]
mod tests {
    use super::{
        BAND_COUNT, BAND_FREQUENCIES, BAND_HZ, CustomPreset, GAIN_MAX_DB, GAIN_MIN_DB, PRESETS,
        PresetChoice, SAVE_PRESET_LABEL, clamp_gain, preset_for_genre,
    };
    use crate::test_support::rock_preset;

    #[test]
    fn band_frequencies_match_the_band_count() {
        assert_eq!(BAND_FREQUENCIES.len(), BAND_COUNT);
    }

    // The display labels and the filters' numeric centers are separate
    // tables; a typo or reordering in either would leave the panel and the
    // sound out of step while every other test still passed. Pin both in the
    // same low-to-high order.
    #[test]
    fn band_frequencies_have_matching_numeric_centers() {
        assert_eq!(
            BAND_HZ,
            [
                60.0, 170.0, 310.0, 600.0, 1_000.0, 3_000.0, 6_000.0, 12_000.0, 14_000.0, 16_000.0
            ]
        );
        assert_eq!(BAND_HZ.len(), BAND_FREQUENCIES.len());
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

    // `from_curve` is the single constructor for every custom preset, whatever
    // source it came from — the shared state's stored gains or a hand-edited
    // settings line — so a curve leaving it must already be clamped, even for
    // input a slider or a settings file never produced.
    #[test]
    fn from_curve_clamps_the_whole_curve() {
        let mut bands = [0.0; BAND_COUNT];
        bands[0] = 13.0;
        bands[1] = -13.0;
        bands[2] = f32::NAN;
        let preset = CustomPreset::from_curve(String::from("Fuzz"), 99.0, bands);
        assert_eq!(preset.name, "Fuzz");
        assert_eq!(preset.preamp, GAIN_MAX_DB);
        assert_eq!(preset.bands[0], GAIN_MAX_DB);
        assert_eq!(preset.bands[1], GAIN_MIN_DB);
        assert_eq!(preset.bands[2], 0.0);
    }

    // The pick list draws every row through `Display`, so each variant must
    // name its own row: the save sentinel, the built-in's name, or the saved
    // custom's name.
    #[test]
    fn preset_choices_render_their_own_labels() {
        assert_eq!(PresetChoice::Save.to_string(), SAVE_PRESET_LABEL);
        assert_eq!(
            PresetChoice::BuiltIn(PRESETS[0]).to_string(),
            PRESETS[0].name
        );
        let custom = CustomPreset::from_curve(String::from("Very Fuzzed"), 0.0, [0.0; BAND_COUNT]);
        assert_eq!(
            PresetChoice::Custom(custom.clone()).to_string(),
            "Very Fuzzed"
        );
        assert_eq!(
            PresetChoice::Custom(custom.clone()),
            PresetChoice::Custom(custom.clone())
        );
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
    // nothing in the crate exercised `Preset`'s `Display` before this test.
    // That impl is what iced's `PickList` actually renders for the selected
    // preset and for every menu option (see the struct doc), so a `Display`
    // that returned anything but `name` — a debug repr, say — would leave the
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
    fn preset_for_genre_matches_trimmed_case_insensitive_names() {
        let rock = rock_preset();
        assert_eq!(preset_for_genre("Rock"), Some(rock));
        assert_eq!(preset_for_genre("  rOcK  "), Some(rock));
        assert_eq!(
            preset_for_genre("Full Bass & Treble").map(|p| p.name),
            Some("Full Bass & Treble")
        );
        assert_eq!(preset_for_genre(""), None);
        assert_eq!(preset_for_genre("   "), None);
        assert_eq!(preset_for_genre("Alternative"), None);
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
