//! The user's mixer and equalizer settings, persisted across launches.
//!
//! [`Settings`] is a small snapshot of the values the player keeps in
//! [`AppState`] while it runs: the volume, balance, Repeat and Shuffle flags,
//! and the equalizer's on/off state, preamp, band gains, and selected preset.
//! [`FileStore`] loads that snapshot from a line-based text file at startup
//! and rewrites it whenever the UI changes one of those values. Playback
//! position, the browse level, the search box, the current track, and window
//! placement are deliberately not persisted.

use std::ffi::OsString;
use std::path::PathBuf;

use crate::equalizer::{self, Preset};
use crate::state::AppState;

/// The persisted subset of [`AppState`]: the mixer and equalizer values the
/// next launch restores.
///
/// The fields are private; build a snapshot with [`Settings::from_state`] and
/// restore it with [`Settings::apply_to`]. `Default` mirrors
/// [`AppState::default`], so if one starting value changes the other must
/// change with it.
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
}

impl Default for Settings {
    /// The same starting values as [`AppState::default`]: half volume, a
    /// centered balance, both toggles off, and the equalizer off with a flat
    /// curve and no preset. A missing or unreadable settings file starts here.
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
        }
    }

    /// Restore this snapshot into `state`.
    ///
    /// The private values go through the state's clamped setters
    /// (`set_volume`, `set_balance`, `set_eq_preamp`, and `set_eq_band`), so a
    /// hand-edited file cannot push the state out of range. An applied preset
    /// is restored last through `apply_eq_preset`, after the per-band writes
    /// have cleared the selection.
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
                "volume" => {
                    if let Ok(volume) = value.parse() {
                        settings.volume = volume;
                    }
                }
                "balance" => {
                    if let Ok(balance) = value.parse() {
                        settings.balance = balance;
                    }
                }
                "repeat" => {
                    if let Ok(repeat) = value.parse() {
                        settings.repeat = repeat;
                    }
                }
                "shuffle" => {
                    if let Ok(shuffle) = value.parse() {
                        settings.shuffle = shuffle;
                    }
                }
                "eq_enabled" => {
                    if let Ok(enabled) = value.parse() {
                        settings.eq_enabled = enabled;
                    }
                }
                "eq_preamp" => {
                    if let Ok(preamp) = value.parse() {
                        settings.eq_preamp = preamp;
                    }
                }
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

/// Writes a [`Settings`] snapshot somewhere. The UI's `WinampPlayer` holds one
/// behind a `Box<dyn SettingsWriter>`; production uses [`FileStore`] and the
/// tests use an in-memory recorder.
pub trait SettingsWriter: Send + Sync {
    /// Persist `settings`. A failure is logged by the implementation rather
    /// than reported to the caller, so the UI's update loop never aborts.
    fn save(&self, settings: &Settings);
}

/// The production [`SettingsWriter`]: a text file on disk.
///
/// The path is resolved once at startup by [`FileStore::from_environment`];
/// with no resolvable location persistence is disabled and every operation is
/// a no-op.
#[derive(Debug, Clone)]
pub struct FileStore {
    path: Option<PathBuf>,
}

impl FileStore {
    /// Resolve the settings path from the environment.
    ///
    /// `NOSTALGIA_SETTINGS_PATH` is the cross-platform override (read once at
    /// startup), then `$XDG_CONFIG_HOME/nostalgia/settings.ini`, then
    /// `$HOME/.config/nostalgia/settings.ini`. An empty variable is treated as
    /// unset; when none resolves, persistence is disabled.
    #[must_use]
    pub fn from_environment() -> Self {
        Self {
            path: Self::resolve_path(
                std::env::var_os("NOSTALGIA_SETTINGS_PATH"),
                std::env::var_os("XDG_CONFIG_HOME"),
                std::env::var_os("HOME"),
            ),
        }
    }

    /// The settings file's path: the override, else the XDG config directory,
    /// else the home directory's `.config`, else `None`. Empty values count as
    /// unset.
    #[must_use]
    fn resolve_path(
        override_path: Option<OsString>,
        xdg_config_home: Option<OsString>,
        home: Option<OsString>,
    ) -> Option<PathBuf> {
        let non_empty = |value: Option<OsString>| value.filter(|value| !value.is_empty());
        if let Some(path) = non_empty(override_path) {
            return Some(PathBuf::from(path));
        }
        if let Some(config_home) = non_empty(xdg_config_home) {
            return Some(
                PathBuf::from(config_home)
                    .join("nostalgia")
                    .join("settings.ini"),
            );
        }
        non_empty(home).map(|home| {
            PathBuf::from(home)
                .join(".config")
                .join("nostalgia")
                .join("settings.ini")
        })
    }

    /// A store over an explicit path, for tests and callers that already know
    /// where the file belongs.
    #[must_use]
    pub fn at(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    /// Read and parse the settings file, or `None` when persistence is
    /// disabled, the file is missing, or it cannot be read. Malformed lines
    /// fall back per key through [`Settings::from_text`].
    #[must_use]
    pub fn load(&self) -> Option<Settings> {
        let path = self.path.as_ref()?;
        let text = std::fs::read_to_string(path).ok()?;
        Some(Settings::from_text(&text))
    }
}

impl SettingsWriter for FileStore {
    fn save(&self, settings: &Settings) {
        let Some(path) = self.path.as_ref() else {
            return;
        };
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            eprintln!(
                "failed to create the settings directory {}: {error}",
                parent.display()
            );
            return;
        }
        if let Err(error) = std::fs::write(path, settings.to_text()) {
            eprintln!(
                "failed to write the settings file {}: {error}",
                path.display()
            );
        }
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

        let settings = Settings::from_state(&state);
        assert_eq!(Settings::from_text(&settings.to_text()), settings);

        let custom = Settings {
            eq_preset: None,
            ..settings
        };
        assert_eq!(Settings::from_text(&custom.to_text()), custom);
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

    #[test]
    fn settings_path_resolves_override_then_xdg_then_home_then_none() {
        let override_path = Some(OsString::from("/custom/settings.ini"));
        let xdg = Some(OsString::from("/xdg"));
        let home = Some(OsString::from("/home/user"));

        assert_eq!(
            FileStore::resolve_path(override_path, xdg.clone(), home.clone()),
            Some(PathBuf::from("/custom/settings.ini"))
        );
        assert_eq!(
            FileStore::resolve_path(Some(OsString::new()), xdg, home.clone()),
            Some(PathBuf::from("/xdg/nostalgia/settings.ini"))
        );
        assert_eq!(
            FileStore::resolve_path(
                None,
                Some(OsString::new()),
                Some(OsString::from("/home/user")),
            ),
            Some(PathBuf::from("/home/user/.config/nostalgia/settings.ini"))
        );
        assert_eq!(
            FileStore::resolve_path(None, None, Some(OsString::new())),
            None
        );
    }

    #[test]
    fn file_store_save_then_load_returns_the_same_settings() {
        let root = std::env::temp_dir().join(format!(
            "nostalgia-settings-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("nested").join("settings.ini");
        let store = FileStore::at(path);

        let mut state = state_with_every_field_set();
        let rock = *equalizer::PRESETS
            .iter()
            .find(|preset| preset.name == "Rock")
            .unwrap();
        state.apply_eq_preset(rock);
        let settings = Settings::from_state(&state);

        store.save(&settings);
        assert_eq!(store.load(), Some(settings));

        std::fs::remove_dir_all(&root).unwrap();
    }
}
