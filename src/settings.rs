//! The user's mixer and equalizer settings, persisted across launches.
//!
//! [`Settings`] is a small snapshot of the values the player keeps while it
//! runs: the volume, balance, Repeat and Shuffle flags, the equalizer's on/off
//! state, preamp, band gains, and selected preset, the Now Playing readout's
//! elapsed/remaining mode, the visualizer's shown/hidden choice, and the
//! always-on-top flag. All but the last three live in [`AppState`]; the readout
//! mode, visualizer choice, and always-on-top flag live on the UI's
//! `WinampPlayer`, so the player sets them on its snapshot before saving and
//! restores them after loading. The file-backed store and the writer seam
//! live in [`file_store`].
//!
//! [`file_store::FileStore`] loads the snapshot from a line-based text file
//! at startup and rewrites it whenever the UI changes one of those values.
//! Playback position, the browse level, the search box, the current track,
//! and window placement (other than the saved always-on-top level) are
//! deliberately not persisted.

pub mod file_store;

use crate::equalizer::{self, Preset};
use crate::state::AppState;

/// The persisted values the next launch restores: the mixer and equalizer
/// subset of [`AppState`] plus the UI-owned readout mode, visualizer choice,
/// and always-on-top flag.
///
/// The fields are private; build a state-backed snapshot with
/// [`Settings::from_state`] and restore it with [`Settings::apply_to`].
/// `time_display_remaining`, `visualizer_visible`, and `always_on_top` are not
/// held in [`AppState`]: `from_state` leaves them elapsed, shown, and unpinned,
/// and the player overwrites them from its own values before saving. `Default`
/// mirrors [`AppState::default`] plus elapsed, shown, and unpinned, so if one
/// starting value changes the other must change with it.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    volume: f32,
    balance: f32,
    repeat: bool,
    shuffle: bool,
    eq_enabled: bool,
    eq_preamp: f32,
    eq_bands: [f32; equalizer::BAND_COUNT],
    eq_preset: Option<Preset>,
    time_display_remaining: bool,
    visualizer_visible: bool,
    always_on_top: bool,
}

impl Default for Settings {
    /// The same starting values as [`AppState::default`]: half volume, a
    /// centered balance, both toggles off, and the equalizer off with a flat
    /// curve and no preset, plus the elapsed readout mode, the shown
    /// visualizer, and an unpinned window. A missing or unreadable settings
    /// file starts here.
    fn default() -> Self {
        Self {
            volume: 0.5,
            balance: 0.0,
            repeat: false,
            shuffle: false,
            eq_enabled: false,
            eq_preamp: 0.0,
            eq_bands: [0.0; equalizer::BAND_COUNT],
            eq_preset: None,
            time_display_remaining: false,
            visualizer_visible: true,
            always_on_top: false,
        }
    }
}

impl Settings {
    /// Build the persisted snapshot of `state`, reading its public flags and
    /// its getters for the private clamped values.
    #[must_use]
    pub fn from_state(state: &AppState) -> Self {
        Self {
            volume: state.volume(),
            balance: state.balance(),
            repeat: state.repeat,
            shuffle: state.shuffle,
            eq_enabled: state.eq_enabled,
            eq_preamp: state.eq_preamp(),
            eq_bands: state.eq_bands(),
            eq_preset: state.eq_preset(),
            time_display_remaining: false,
            visualizer_visible: true,
            always_on_top: false,
        }
    }

    /// Whether the Now Playing readout shows the track's remaining time
    /// rather than its elapsed time. The player owns this value, so it reads
    /// it back when saving and restores it from the loaded snapshot at boot.
    #[must_use]
    pub fn time_display_remaining(&self) -> bool {
        self.time_display_remaining
    }

    /// Set whether the Now Playing readout shows remaining time. The player
    /// calls this on its fresh snapshot before saving; [`Self::apply_to`]
    /// deliberately leaves the UI-owned mode alone.
    pub fn set_time_display_remaining(&mut self, remaining: bool) {
        self.time_display_remaining = remaining;
    }

    /// Whether the LCD's spectrum visualizer draws its bars. The player owns
    /// this value, so it reads it back when saving and restores it from the
    /// loaded snapshot at boot. The stored spectrum and its poll are separate
    /// and are not persisted.
    #[must_use]
    pub fn visualizer_visible(&self) -> bool {
        self.visualizer_visible
    }

    /// Set whether the LCD's spectrum visualizer draws its bars. The player
    /// calls this on its fresh snapshot before saving; [`Self::apply_to`]
    /// deliberately leaves the UI-owned flag alone.
    pub fn set_visualizer_visible(&mut self, visible: bool) {
        self.visualizer_visible = visible;
    }

    /// Whether the window is pinned above other windows. The player owns this
    /// value, so it reads it back when saving and restores it from the loaded
    /// snapshot at boot.
    #[must_use]
    pub fn always_on_top(&self) -> bool {
        self.always_on_top
    }

    /// Set whether the window is pinned above other windows. The player calls
    /// this on its fresh snapshot before saving; [`Self::apply_to`]
    /// deliberately leaves the UI-owned flag alone.
    pub fn set_always_on_top(&mut self, always_on_top: bool) {
        self.always_on_top = always_on_top;
    }

    /// Restore this snapshot into `state`.
    ///
    /// The private values go through the state's clamped setters
    /// (`set_volume`, `set_balance`, `set_eq_preamp`, and `set_eq_band`), so a
    /// hand-edited file cannot push the state out of range. An applied preset
    /// is restored last through `apply_eq_preset`, after the per-band writes
    /// have cleared the selection. The UI-owned readout mode, visualizer
    /// choice, and always-on-top flag are deliberately left alone.
    pub fn apply_to(&self, state: &mut AppState) {
        state.set_volume(self.volume);
        state.set_balance(self.balance);
        state.repeat = self.repeat;
        state.shuffle = self.shuffle;
        state.eq_enabled = self.eq_enabled;
        state.set_eq_preamp(self.eq_preamp);
        for (band, gain) in self.eq_bands.iter().enumerate() {
            state.set_eq_band(band, *gain);
        }
        if let Some(preset) = self.eq_preset {
            state.apply_eq_preset(preset);
        }
    }

    /// Render the snapshot as the line-based `key=value` text the settings
    /// file holds. Floats use their shortest round-tripping `Display` form,
    /// and a custom curve stores an empty `eq_preset` value.
    fn to_text(&self) -> String {
        let mut text = String::new();
        text.push_str(&format!("volume={}\n", self.volume));
        text.push_str(&format!("balance={}\n", self.balance));
        text.push_str(&format!("repeat={}\n", self.repeat));
        text.push_str(&format!("shuffle={}\n", self.shuffle));
        text.push_str(&format!("eq_enabled={}\n", self.eq_enabled));
        text.push_str(&format!("eq_preamp={}\n", self.eq_preamp));
        for (band, gain) in self.eq_bands.iter().enumerate() {
            text.push_str(&format!("eq_band_{band}={gain}\n"));
        }
        text.push_str(&format!(
            "eq_preset={}\n",
            self.eq_preset.map_or("", |preset| preset.name)
        ));
        text.push_str(&format!(
            "time_display={}\n",
            if self.time_display_remaining {
                "remaining"
            } else {
                "elapsed"
            }
        ));
        text.push_str(&format!("visualizer_visible={}\n", self.visualizer_visible));
        text.push_str(&format!("always_on_top={}\n", self.always_on_top));
        text
    }

    /// Parse `text` into a snapshot, starting from [`Settings::default`].
    ///
    /// Unknown keys, malformed numbers, booleans other than `true`/`false`,
    /// out-of-range band indexes, and unrecognized preset names are ignored,
    /// so one bad line falls back to its default without failing the load.
    fn from_text(text: &str) -> Self {
        let mut settings = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            match key {
                "volume" => parse_into(&mut settings.volume, value),
                "balance" => parse_into(&mut settings.balance, value),
                "repeat" => parse_into(&mut settings.repeat, value),
                "shuffle" => parse_into(&mut settings.shuffle, value),
                "eq_enabled" => parse_into(&mut settings.eq_enabled, value),
                "eq_preamp" => parse_into(&mut settings.eq_preamp, value),
                "eq_preset" => {
                    if value.is_empty() {
                        settings.eq_preset = None;
                    } else if let Some(preset) = equalizer::PRESETS
                        .iter()
                        .find(|preset| preset.name == value)
                    {
                        settings.eq_preset = Some(*preset);
                    }
                }
                // Only the two written words are recognized; any other value
                // leaves the default elapsed mode.
                "time_display" => match value {
                    "remaining" => settings.time_display_remaining = true,
                    "elapsed" => settings.time_display_remaining = false,
                    _ => {}
                },
                // A malformed value leaves the default shown visualizer.
                "visualizer_visible" => {
                    parse_into(&mut settings.visualizer_visible, value);
                }
                // A malformed value leaves the default unpinned window.
                "always_on_top" => parse_into(&mut settings.always_on_top, value),
                _ => {
                    if let Some(index) = key
                        .strip_prefix("eq_band_")
                        .and_then(|index| index.parse::<usize>().ok())
                        && let Some(slot) = settings.eq_bands.get_mut(index)
                        && let Ok(gain) = value.parse()
                    {
                        *slot = gain;
                    }
                }
            }
        }
        settings
    }
}

/// Assign `value`'s parsed form to `slot`, or leave `slot` unchanged when the
/// value is malformed. [`Settings::from_text`]'s scalar fields all follow this
/// rule, so a bad line falls back to that field's default without failing the
/// load; `eq_band_*` and `eq_preset` need their own handling for the index and
/// preset lookup.
fn parse_into<T: std::str::FromStr>(slot: &mut T, value: &str) {
    if let Ok(parsed) = value.parse() {
        *slot = parsed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_every_field_set() -> AppState {
        let mut state = AppState::default();
        state.set_volume(0.8);
        state.set_balance(-0.35);
        state.repeat = true;
        state.shuffle = true;
        state.eq_enabled = true;
        state.set_eq_preamp(3.5);
        state.set_eq_band(0, -4.0);
        state.set_eq_band(9, 7.5);
        state
    }

    #[test]
    fn to_text_round_trips_every_field() {
        let mut state = state_with_every_field_set();
        let rock = *equalizer::PRESETS
            .iter()
            .find(|preset| preset.name == "Rock")
            .unwrap();
        state.apply_eq_preset(rock);

        let mut settings = Settings::from_state(&state);
        settings.set_time_display_remaining(true);
        settings.set_visualizer_visible(false);
        settings.set_always_on_top(true);
        assert_eq!(Settings::from_text(&settings.to_text()), settings);
        assert!(Settings::from_text(&settings.to_text()).time_display_remaining());
        assert!(!Settings::from_text(&settings.to_text()).visualizer_visible());
        assert!(Settings::from_text(&settings.to_text()).always_on_top());

        let custom = Settings {
            eq_preset: None,
            ..settings
        };
        assert_eq!(Settings::from_text(&custom.to_text()), custom);
    }

    #[test]
    fn time_display_round_trips_both_words_and_ignores_unknown_values() {
        let elapsed = Settings::default();
        assert!(!elapsed.time_display_remaining());
        assert!(!Settings::from_text(&elapsed.to_text()).time_display_remaining());

        let mut remaining = elapsed.clone();
        remaining.set_time_display_remaining(true);
        assert!(remaining.to_text().contains("time_display=remaining"));
        assert!(
            Settings::from_text(&remaining.to_text()).time_display_remaining(),
            "remaining must survive the round trip"
        );

        assert!(
            !Settings::from_text("time_display=sideways\n").time_display_remaining(),
            "an unknown word must fall back to elapsed"
        );
    }

    #[test]
    fn visualizer_visibility_round_trips_and_ignores_malformed_values() {
        let shown = Settings::default();
        assert!(shown.visualizer_visible());
        assert!(Settings::from_text(&shown.to_text()).visualizer_visible());

        let mut hidden = shown.clone();
        hidden.set_visualizer_visible(false);
        assert!(hidden.to_text().contains("visualizer_visible=false"));
        assert!(
            !Settings::from_text(&hidden.to_text()).visualizer_visible(),
            "a hidden visualizer must survive the round trip"
        );

        assert!(
            Settings::from_text("visualizer_visible=true\n").visualizer_visible(),
            "an explicit true must stay shown"
        );
        assert!(
            Settings::from_text("visualizer_visible=maybe\n").visualizer_visible(),
            "an unknown value must fall back to shown"
        );
    }

    #[test]
    fn always_on_top_round_trips_and_ignores_malformed_values() {
        let unpinned = Settings::default();
        assert!(!unpinned.always_on_top());
        assert!(!Settings::from_text(&unpinned.to_text()).always_on_top());

        let mut pinned = unpinned.clone();
        pinned.set_always_on_top(true);
        assert!(pinned.to_text().contains("always_on_top=true"));
        assert!(
            Settings::from_text(&pinned.to_text()).always_on_top(),
            "a pinned window must survive the round trip"
        );

        assert!(
            !Settings::from_text("always_on_top=false\n").always_on_top(),
            "an explicit false must stay unpinned"
        );
        assert!(
            !Settings::from_text("always_on_top=yes\n").always_on_top(),
            "an unknown word must fall back to unpinned"
        );
    }

    #[test]
    fn from_text_ignores_malformed_and_unknown_lines() {
        let text = "volume=not-a-number\n\
                    balance=abc\n\
                    repeat=yes\n\
                    shuffle=1\n\
                    eq_enabled=maybe\n\
                    eq_preamp=high\n\
                    eq_band_10=3\n\
                    eq_band_x=2\n\
                    eq_preset=Unknown\n\
                    time_display=sideways\n\
                    visualizer_visible=maybe\n\
                    always_on_top=yes\n\
                    mystery=1\n";
        assert_eq!(Settings::from_text(text), Settings::default());
    }

    #[test]
    fn apply_to_clamps_out_of_range_values_through_the_state_setters() {
        let settings = Settings {
            volume: 2.0,
            balance: -2.0,
            eq_preamp: 99.0,
            eq_bands: [99.0; equalizer::BAND_COUNT],
            ..Settings::default()
        };
        let mut state = AppState::default();
        settings.apply_to(&mut state);

        assert_eq!(state.volume(), 1.0);
        assert_eq!(state.balance(), -1.0);
        assert_eq!(state.eq_preamp(), equalizer::GAIN_MAX_DB);
        assert!(
            state
                .eq_bands()
                .iter()
                .all(|gain| *gain == equalizer::GAIN_MAX_DB)
        );
    }

    #[test]
    fn apply_to_restores_a_saved_preset() {
        let rock = *equalizer::PRESETS
            .iter()
            .find(|preset| preset.name == "Rock")
            .unwrap();
        let settings = Settings {
            repeat: true,
            eq_enabled: true,
            eq_preset: Some(rock),
            ..Settings::default()
        };
        let mut state = AppState::default();
        settings.apply_to(&mut state);

        assert!(state.repeat);
        assert!(state.eq_enabled);
        assert_eq!(state.eq_preset(), Some(rock));
        assert_eq!(state.eq_preamp(), rock.preamp);
        assert_eq!(state.eq_bands(), rock.bands);
    }
}
