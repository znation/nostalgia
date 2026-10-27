//! The user's mixer and equalizer settings, persisted across launches.
//!
//! [`Settings`] is a small snapshot of the values the player keeps while it
//! runs: the volume, balance, Repeat and Shuffle flags, the equalizer's on/off
//! and AUTO states, preamp, band gains, and selected preset, the Now Playing readout's
//! elapsed/remaining mode, the visualizer's shown/hidden choice, the equalizer
//! and library panels' shown/hidden choices, the always-on-top flag, and the
//! window-shade mode. All but the last six live in [`AppState`]; the readout
//! mode, visualizer choice, panel choices, always-on-top flag, and shade mode
//! live on the UI's `WinampPlayer`, so the player sets them on its snapshot
//! before saving and restores them after loading. The file-backed store and the
//! writer seam live in [`file_store`], and [`writer::BackgroundWriter`] runs the
//! blocking write on a worker thread in production.
//!
//! [`file_store::FileStore`] loads the snapshot from a line-based text file
//! at startup and rewrites it whenever the UI changes one of those values.
//! Playback position, the browse level, the search box, the current track,
//! and the window's size and position are deliberately not persisted.

pub mod file_store;
pub mod writer;

use crate::equalizer::{self, Preset};
use crate::state::AppState;

/// The persisted values the next launch restores: the mixer and equalizer
/// subset of [`AppState`] plus the UI-owned readout mode, visualizer choice,
/// equalizer and library panel choices, always-on-top flag, and shade mode.
///
/// The fields are private; build a state-backed snapshot with
/// [`Settings::from_state`] and restore it with [`Settings::apply_to`].
/// `time_display_remaining`, `visualizer_visible`, `eq_panel_visible`,
/// `library_panel_visible`, `always_on_top`, and `shade` are not held in
/// [`AppState`]: `from_state` leaves them elapsed, shown (all three), unpinned,
/// and unshaded, and the player overwrites them from its own values before
/// saving. `Default` mirrors [`AppState::default`] plus elapsed, shown,
/// unpinned, and unshaded, so if one starting value changes the other must
/// change with it.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    volume: f32,
    balance: f32,
    repeat: bool,
    shuffle: bool,
    eq_enabled: bool,
    eq_auto: bool,
    eq_preamp: f32,
    eq_bands: [f32; equalizer::BAND_COUNT],
    eq_preset: Option<Preset>,
    time_display_remaining: bool,
    visualizer_visible: bool,
    eq_panel_visible: bool,
    library_panel_visible: bool,
    always_on_top: bool,
    shade: bool,
}

impl Default for Settings {
    /// The same starting values as [`AppState::default`]: half volume, a
    /// centered balance, both toggles off, and the equalizer off with AUTO off,
    /// a flat curve, and no preset, plus the elapsed readout mode, the shown
    /// visualizer and both panels, and an unpinned, unshaded window. A missing
    /// or unreadable settings file starts here.
    fn default() -> Self {
        Self {
            volume: 0.5,
            balance: 0.0,
            repeat: false,
            shuffle: false,
            eq_enabled: false,
            eq_auto: false,
            eq_preamp: 0.0,
            eq_bands: [0.0; equalizer::BAND_COUNT],
            eq_preset: None,
            time_display_remaining: false,
            visualizer_visible: true,
            eq_panel_visible: true,
            library_panel_visible: true,
            always_on_top: false,
            shade: false,
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
            eq_auto: state.eq_auto,
            eq_preamp: state.eq_preamp(),
            eq_bands: state.eq_bands(),
            eq_preset: state.eq_preset(),
            time_display_remaining: false,
            visualizer_visible: true,
            eq_panel_visible: true,
            library_panel_visible: true,
            always_on_top: false,
            shade: false,
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

    /// Whether the normal-mode equalizer panel is shown. The player owns this
    /// value, so it reads it back when saving and restores it from the loaded
    /// snapshot at boot. It is a layout choice only; the EQ's own state is
    /// separate.
    #[must_use]
    pub fn eq_panel_visible(&self) -> bool {
        self.eq_panel_visible
    }

    /// Set whether the equalizer panel is shown. The player calls this on its
    /// fresh snapshot before saving; [`Self::apply_to`] deliberately leaves
    /// the UI-owned flag alone.
    pub fn set_eq_panel_visible(&mut self, visible: bool) {
        self.eq_panel_visible = visible;
    }

    /// Whether the normal-mode library section is shown. The player owns this
    /// value, so it reads it back when saving and restores it from the loaded
    /// snapshot at boot.
    #[must_use]
    pub fn library_panel_visible(&self) -> bool {
        self.library_panel_visible
    }

    /// Set whether the library section is shown. The player calls this on its
    /// fresh snapshot before saving; [`Self::apply_to`] deliberately leaves
    /// the UI-owned flag alone.
    pub fn set_library_panel_visible(&mut self, visible: bool) {
        self.library_panel_visible = visible;
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

    /// Whether the window is rolled up into shade mode. The player owns this
    /// value, so it reads it back when saving and restores it from the loaded
    /// snapshot at boot. The window's size and position are separate and are
    /// not persisted.
    #[must_use]
    pub fn shaded(&self) -> bool {
        self.shade
    }

    /// Set whether the window is rolled up into shade mode. The player calls
    /// this on its fresh snapshot before saving; [`Self::apply_to`]
    /// deliberately leaves the UI-owned flag alone.
    pub fn set_shaded(&mut self, shaded: bool) {
        self.shade = shaded;
    }

    /// Restore this snapshot into `state`.
    ///
    /// The private values go through the state's clamped setters
    /// (`set_volume`, `set_balance`, `set_eq_preamp`, and `set_eq_band`), so a
    /// hand-edited file cannot push the state out of range. An applied preset
    /// is restored last through `apply_eq_preset`, after the per-band writes
    /// have cleared the selection. The AUTO flag is a plain restored boolean.
    /// The UI-owned readout mode, visualizer
    /// choice, panel choices, always-on-top flag, and shade mode are
    /// deliberately left alone.
    pub fn apply_to(&self, state: &mut AppState) {
        state.set_volume(self.volume);
        state.set_balance(self.balance);
        state.repeat = self.repeat;
        state.shuffle = self.shuffle;
        state.eq_enabled = self.eq_enabled;
        state.eq_auto = self.eq_auto;
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
        text.push_str(&format!("eq_auto={}\n", self.eq_auto));
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
        text.push_str(&format!("eq_panel_visible={}\n", self.eq_panel_visible));
        text.push_str(&format!(
            "library_panel_visible={}\n",
            self.library_panel_visible
        ));
        text.push_str(&format!("always_on_top={}\n", self.always_on_top));
        text.push_str(&format!("shade={}\n", self.shade));
        text
    }

    /// Parse `text` into a snapshot, starting from [`Settings::default`].
    ///
    /// Unknown keys, malformed numbers, booleans other than `true`/`false`,
    /// out-of-range band indexes, and unrecognized preset names are ignored,
    /// so one bad line falls back to its default without failing the load. A
    /// leading UTF-8 BOM is an encoding marker, not part of the first key, so
    /// it is dropped before the lines are read. A `NaN` is malformed rather
    /// than a number: `f32::from_str` accepts it, but it is not a value the
    /// player can hold, so the field keeps its default.
    fn from_text(text: &str) -> Self {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut settings = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            match key {
                "volume" => parse_number_into(&mut settings.volume, value),
                "balance" => parse_number_into(&mut settings.balance, value),
                "repeat" => parse_into(&mut settings.repeat, value),
                "shuffle" => parse_into(&mut settings.shuffle, value),
                "eq_enabled" => parse_into(&mut settings.eq_enabled, value),
                // A malformed value leaves the default AUTO off.
                "eq_auto" => parse_into(&mut settings.eq_auto, value),
                "eq_preamp" => parse_number_into(&mut settings.eq_preamp, value),
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
                // A malformed value leaves the default shown panel.
                "eq_panel_visible" => parse_into(&mut settings.eq_panel_visible, value),
                "library_panel_visible" => {
                    parse_into(&mut settings.library_panel_visible, value);
                }
                // A malformed value leaves the default unpinned window.
                "always_on_top" => parse_into(&mut settings.always_on_top, value),
                // A malformed value leaves the default unshaded window.
                "shade" => parse_into(&mut settings.shade, value),
                _ => {
                    if let Some(index) = key
                        .strip_prefix("eq_band_")
                        .and_then(|index| index.parse::<usize>().ok())
                        && let Some(slot) = settings.eq_bands.get_mut(index)
                    {
                        parse_number_into(slot, value);
                    }
                }
            }
        }
        settings
    }
}

/// Assign `value`'s parsed form to `slot`, or leave `slot` unchanged when the
/// value is malformed. [`Settings::from_text`]'s boolean fields follow this
/// rule, so a bad line falls back to that field's default without failing the
/// load; the float fields, `eq_band_*`, and `eq_preset` need their own
/// handling for the NaN, index, and preset lookup.
fn parse_into<T: std::str::FromStr>(slot: &mut T, value: &str) {
    if let Ok(parsed) = value.parse() {
        *slot = parsed;
    }
}

/// Assign `value`'s parsed number to `slot` unless it is malformed or `NaN`,
/// leaving `slot` unchanged.
///
/// `f32::from_str` accepts `NaN` and the infinities. An infinity is a value
/// outside every documented range, so [`Settings::apply_to`]'s state setters
/// clamp it like any other out-of-range number. `NaN` is not a number at all,
/// and passing it on would make each field's fallback depend on its state
/// setter's neutral value — volume's is silence, not its default — so it is
/// treated as malformed and the field keeps its default.
fn parse_number_into(slot: &mut f32, value: &str) {
    if let Ok(parsed) = value.parse::<f32>()
        && !parsed.is_nan()
    {
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
        state.eq_auto = true;
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
        settings.set_eq_panel_visible(false);
        settings.set_library_panel_visible(false);
        settings.set_always_on_top(true);
        settings.set_shaded(true);
        assert_eq!(Settings::from_text(&settings.to_text()), settings);
        assert!(Settings::from_text(&settings.to_text()).time_display_remaining());
        assert!(!Settings::from_text(&settings.to_text()).visualizer_visible());
        assert!(!Settings::from_text(&settings.to_text()).eq_panel_visible());
        assert!(!Settings::from_text(&settings.to_text()).library_panel_visible());
        assert!(Settings::from_text(&settings.to_text()).always_on_top());
        assert!(Settings::from_text(&settings.to_text()).shaded());

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
    fn panel_visibility_round_trips_and_ignores_malformed_values() {
        let shown = Settings::default();
        assert!(shown.eq_panel_visible());
        assert!(shown.library_panel_visible());

        let mut eq_hidden = shown.clone();
        eq_hidden.set_eq_panel_visible(false);
        let back = Settings::from_text(&eq_hidden.to_text());
        assert!(!back.eq_panel_visible());
        assert!(back.library_panel_visible());

        let mut library_hidden = shown.clone();
        library_hidden.set_library_panel_visible(false);
        let back = Settings::from_text(&library_hidden.to_text());
        assert!(back.eq_panel_visible());
        assert!(!back.library_panel_visible());

        let unknown = Settings::from_text("eq_panel_visible=maybe\nlibrary_panel_visible=maybe\n");
        assert!(
            unknown.eq_panel_visible(),
            "an unknown value must fall back to shown"
        );
        assert!(unknown.library_panel_visible());
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
    fn from_text_ignores_a_leading_utf8_bom() {
        let settings = Settings::from_text("\u{feff}volume=0.25\nbalance=0.25\n");

        assert_eq!(
            settings.volume, 0.25,
            "a leading BOM is an encoding marker, not part of the first key"
        );
        assert_eq!(settings.balance, 0.25);
    }

    #[test]
    fn shade_round_trips_and_ignores_malformed_values() {
        let unshaded = Settings::default();
        assert!(!unshaded.shaded());
        assert!(!Settings::from_text(&unshaded.to_text()).shaded());
        assert!(
            !Settings::from_text("volume=0.5\n").shaded(),
            "a missing shade line must fall back to unshaded"
        );

        let mut shaded = unshaded.clone();
        shaded.set_shaded(true);
        assert!(shaded.to_text().contains("shade=true"));
        assert!(
            Settings::from_text(&shaded.to_text()).shaded(),
            "a shaded window must survive the round trip"
        );

        assert!(
            !Settings::from_text("shade=false\n").shaded(),
            "an explicit false must stay unshaded"
        );
        assert!(
            !Settings::from_text("shade=\n").shaded(),
            "an empty value must fall back to unshaded"
        );
        assert!(
            !Settings::from_text("shade=yes\n").shaded(),
            "an unknown word must fall back to unshaded"
        );
    }

    #[test]
    fn eq_auto_round_trips_and_ignores_malformed_values() {
        let off = Settings::default();
        assert!(!off.eq_auto);
        assert!(!Settings::from_text(&off.to_text()).eq_auto);
        assert!(
            !Settings::from_text("volume=0.5\n").eq_auto,
            "a missing eq_auto line must fall back to off"
        );

        let mut on = off.clone();
        on.eq_auto = true;
        assert!(on.to_text().contains("eq_auto=true"));
        assert!(
            Settings::from_text(&on.to_text()).eq_auto,
            "an enabled AUTO must survive the round trip"
        );

        assert!(
            !Settings::from_text("eq_auto=false\n").eq_auto,
            "an explicit false must stay off"
        );
        assert!(
            !Settings::from_text("eq_auto=\n").eq_auto,
            "an empty value must fall back to off"
        );
        assert!(
            !Settings::from_text("eq_auto=yes\n").eq_auto,
            "an unknown word must fall back to off"
        );
    }

    #[test]
    fn from_text_ignores_nan_numbers() {
        // `f32::from_str` accepts `NaN`, but it is not a value the player can
        // use: an ignored line must fall back to its default, not to whichever
        // neutral value a later clamp happens to choose (volume's is silence).
        let text = "volume=NaN\n\
                    balance=NaN\n\
                    eq_preamp=NaN\n\
                    eq_band_0=NaN\n";
        assert_eq!(Settings::from_text(text), Settings::default());

        let recovered = Settings::from_text("volume=NaN\nvolume=0.25\n");
        assert_eq!(recovered.volume, 0.25, "a later valid line still applies");
    }

    #[test]
    fn from_text_ignores_malformed_and_unknown_lines() {
        let text = "volume=not-a-number\n\
                    balance=abc\n\
                    repeat=yes\n\
                    shuffle=1\n\
                    eq_enabled=maybe\n\
                    eq_auto=maybe\n\
                    eq_preamp=high\n\
                    eq_band_10=3\n\
                    eq_band_x=2\n\
                    eq_band_0=NaN\n\
                    eq_preset=Unknown\n\
                    time_display=sideways\n\
                    visualizer_visible=maybe\n\
                    eq_panel_visible=maybe\n\
                    library_panel_visible=maybe\n\
                    always_on_top=yes\n\
                    shade=yes\n\
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
