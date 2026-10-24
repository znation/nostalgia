//! The settings persistence seam: the [`SettingsWriter`] trait the UI holds
//! behind a box, and [`FileStore`], the production writer that reads and
//! writes the line-based settings file.

use std::ffi::OsString;
use std::path::PathBuf;

use super::Settings;

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

    fn varied_settings() -> Settings {
        let mut settings = Settings {
            volume: 0.8,
            balance: -0.35,
            repeat: true,
            shuffle: true,
            eq_enabled: true,
            eq_preamp: 3.5,
            time_display_remaining: true,
            visualizer_visible: false,
            always_on_top: true,
            ..Settings::default()
        };
        settings.eq_bands[0] = -4.0;
        settings.eq_bands[9] = 7.5;
        settings
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

        let settings = varied_settings();

        store.save(&settings);
        assert_eq!(store.load(), Some(settings));

        std::fs::remove_dir_all(&root).unwrap();
    }

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nostalgia-settings-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    // With no resolvable location, persistence is disabled: loading yields
    // nothing and saving is a no-op rather than an error or a panic.
    #[test]
    fn file_store_without_a_path_disables_load_and_save() {
        let store = FileStore { path: None };

        assert_eq!(store.load(), None);
        store.save(&varied_settings());
        assert_eq!(store.load(), None);
    }

    // A failed write is logged rather than panicking, so the UI's update loop
    // (which saves on every settings change) survives an unwritable location.
    // The path is a directory, which `fs::write` cannot replace.
    #[test]
    fn file_store_recovers_from_a_failed_save() {
        let root = temporary_root("failed-save");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.ini");
        std::fs::create_dir(&path).unwrap();
        let store = FileStore::at(path.clone());

        store.save(&varied_settings());

        assert!(path.is_dir());
        assert_eq!(store.load(), None);
        std::fs::remove_dir_all(&root).unwrap();
    }

    // A parent that is an existing regular file cannot be turned into a
    // directory, so the creation attempt is reported and the file it blocked
    // is left untouched.
    #[test]
    fn file_store_recovers_when_the_settings_directory_cannot_be_created() {
        let root = temporary_root("failed-parent");
        std::fs::create_dir_all(&root).unwrap();
        let parent = root.join("blocked");
        std::fs::write(&parent, "not a directory").unwrap();
        let path = parent.join("settings.ini");
        let store = FileStore::at(path.clone());

        store.save(&varied_settings());

        assert!(!path.exists());
        assert_eq!(std::fs::read_to_string(&parent).unwrap(), "not a directory");
        assert_eq!(store.load(), None);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
