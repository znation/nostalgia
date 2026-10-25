//! The settings persistence seam: the [`SettingsWriter`] trait the UI holds
//! behind a box, and [`FileStore`], the production writer that reads and
//! writes the line-based settings file.

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

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
        sweep_stale_temporaries(path);
        // Read bytes and decode lossily: one corrupt byte must spoil only its
        // own line, not make `read_to_string` reject the whole file and reset
        // every other setting.
        match std::fs::read(path) {
            Ok(bytes) => Some(Settings::from_text(&String::from_utf8_lossy(&bytes))),
            Err(error) => {
                if let Some(message) = read_failure_message(path, &error) {
                    eprintln!("{message}");
                }
                None
            }
        }
    }
}

/// The diagnostic for a settings file that exists but cannot be read, or
/// `None` for a missing file, which is the normal first-run state.
fn read_failure_message(path: &Path, error: &std::io::Error) -> Option<String> {
    (error.kind() != std::io::ErrorKind::NotFound).then(|| {
        format!(
            "failed to read the settings file {}: {error}; using defaults",
            path.display()
        )
    })
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
        if let Err(error) = write_atomically(path, &settings.to_text()) {
            eprintln!(
                "failed to write the settings file {}: {error}",
                path.display()
            );
        }
    }
}

/// The hidden-sibling name prefix [`temporary_sibling`] stages under.
fn temporary_prefix(path: &Path) -> Option<String> {
    let file_name = path.file_name()?;
    Some(format!(".{}.tmp-", file_name.to_string_lossy()))
}

/// How long a temp file must sit untouched before a load treats it as debris
/// from a killed save rather than a save still in progress.
const STALE_TEMP_AGE: Duration = Duration::from_secs(600);

/// Remove sibling temp files left by a save whose process died before it could
/// rename or clean up. A fresh temp may belong to an in-flight save in another
/// process, so only files older than [`STALE_TEMP_AGE`] are removed.
fn sweep_stale_temporaries(path: &Path) {
    sweep_temporaries_older_than(path, STALE_TEMP_AGE);
}

fn sweep_temporaries_older_than(path: &Path, max_age: Duration) {
    // A bare file name such as `settings.ini` lives in the current directory.
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let Some(prefix) = temporary_prefix(path) else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        if !entry.file_name().to_string_lossy().starts_with(&prefix) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        let Ok(age) = now.duration_since(modified) else {
            continue;
        };
        if age >= max_age {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// A sibling of `path` to stage a replacement in. The process id and a
/// per-process counter keep concurrent saves (and concurrent processes) from
/// staging into the same file.
fn temporary_sibling(path: &Path) -> PathBuf {
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    let mut name = OsString::from(".");
    if let Some(file_name) = path.file_name() {
        name.push(file_name);
    }
    name.push(format!(
        ".tmp-{}-{}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    path.with_file_name(name)
}

/// Replace `path` with `text` by writing a sibling temp file, syncing it, and
/// renaming it over the target, so a process killed mid-write leaves the
/// previous file intact instead of a truncated one. A failed write, sync, or
/// rename removes the temp file and reports the error.
fn write_atomically(path: &Path, text: &str) -> io::Result<()> {
    write_atomically_syncing(path, text, File::sync_all)
}

/// [`write_atomically`] with the flush step injected. The sync runs after the
/// bytes are written and before the rename: without it a delayed-allocation
/// filesystem could report ENOSPC during writeback after the rename already
/// replaced the previous file, and a crash could persist the rename without
/// the new contents.
fn write_atomically_syncing(
    path: &Path,
    text: &str,
    sync: impl FnOnce(&File) -> io::Result<()>,
) -> io::Result<()> {
    write_atomically_with(path, |temporary| {
        let mut file = File::create(temporary)?;
        file.write_all(text.as_bytes())?;
        sync(&file)
    })
}

fn write_atomically_with(
    path: &Path,
    write_temporary: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<()> {
    let temporary = temporary_sibling(path);
    let result = write_temporary(&temporary).and_then(|()| std::fs::rename(&temporary, path));
    if result.is_err() {
        // Best effort: do not mask the write error with a cleanup error.
        let _ = std::fs::remove_file(&temporary);
    }
    result
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

    #[test]
    fn read_failure_message_names_the_path_except_for_a_missing_file() {
        let path = Path::new("/cfg/settings.ini");
        let missing = std::io::Error::from(std::io::ErrorKind::NotFound);
        assert_eq!(read_failure_message(path, &missing), None);

        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        let message = read_failure_message(path, &denied).unwrap();
        assert!(message.contains("/cfg/settings.ini"), "{message}");
        assert!(message.contains("using defaults"), "{message}");
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

    // A byte that is not UTF-8 spoils only its own line; the other settings
    // still load instead of the whole file falling back to defaults.
    #[test]
    fn file_store_load_keeps_valid_lines_around_invalid_utf8() {
        let root = temporary_root("invalid-utf8");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.ini");
        let mut bytes = b"volume=0.25\nbalance=\xff\xfe\nrepeat=true\n".to_vec();
        bytes.extend_from_slice(b"shuffle=true\n");
        std::fs::write(&path, bytes).unwrap();

        let loaded = FileStore::at(path).load().expect("file is readable");

        assert!((loaded.volume - 0.25).abs() < f32::EPSILON);
        assert!(loaded.repeat && loaded.shuffle);
        assert!(loaded.balance.abs() < f32::EPSILON);

        std::fs::remove_dir_all(&root).unwrap();
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

    // A save killed mid-write must not truncate the previous file: the
    // replacement is staged aside, and a failure removes the staging file.
    #[test]
    fn a_failed_half_write_leaves_the_previous_settings_file_intact() {
        let root = temporary_root("half-write");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.ini");
        std::fs::write(&path, "volume=0.25\n").unwrap();

        let error = write_atomically_with(&path, |temporary| {
            // Model a disk filling up after the first few bytes.
            std::fs::write(temporary, "volume=0.9\nre")?;
            Err(io::Error::other("disk full"))
        })
        .unwrap_err();

        assert_eq!(error.to_string(), "disk full");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "volume=0.25\n");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }

    // The staged bytes are synced before the rename replaces the old file, and
    // a failing sync (a delayed-allocation ENOSPC) keeps the old file intact.
    #[test]
    fn staged_settings_are_synced_before_the_rename_and_a_failed_sync_keeps_the_old_file() {
        let root = temporary_root("sync-first");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.ini");
        std::fs::write(&path, "volume=0.25\n").unwrap();

        let mut synced_with_old_target = false;
        write_atomically_syncing(&path, "volume=0.9\n", |file| {
            synced_with_old_target = std::fs::read_to_string(&path).unwrap() == "volume=0.25\n";
            file.sync_all()
        })
        .unwrap();
        assert!(synced_with_old_target);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "volume=0.9\n");

        let error = write_atomically_syncing(&path, "volume=0.1\n", |_| {
            Err(io::Error::other("no space left"))
        })
        .unwrap_err();
        assert_eq!(error.to_string(), "no space left");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "volume=0.9\n");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }

    // Through the store: when the temp file cannot be staged (here because the
    // directory is read-only), an existing settings file must stay untouched
    // rather than being rewritten in place.
    #[cfg(unix)]
    #[test]
    fn file_store_keeps_the_previous_file_when_the_replacement_cannot_be_staged() {
        use std::os::unix::fs::PermissionsExt;

        let root = temporary_root("read-only-dir");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.ini");
        std::fs::write(&path, "volume=0.25\n").unwrap();
        let set_mode = |mode: u32| {
            let mut permissions = std::fs::metadata(&root).unwrap().permissions();
            permissions.set_mode(mode);
            std::fs::set_permissions(&root, permissions).unwrap();
        };
        set_mode(0o555);
        if std::fs::write(root.join("probe"), b"").is_ok() {
            // Elevated privileges ignore the directory mode; nothing to test.
            std::fs::remove_file(root.join("probe")).ok();
            set_mode(0o755);
            std::fs::remove_dir_all(&root).unwrap();
            return;
        }

        let store = FileStore::at(path.clone());
        store.save(&varied_settings());

        set_mode(0o755);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "volume=0.25\n");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }

    fn age_file(path: &Path, seconds: u64) {
        let file = File::options().write(true).open(path).unwrap();
        file.set_modified(SystemTime::now() - Duration::from_secs(seconds))
            .unwrap();
    }

    // A save killed between staging and rename leaves its hidden sibling
    // behind. The next load sweeps that debris once it is old enough, while
    // leaving a fresh temp alone because it may belong to a save in progress.
    #[test]
    fn file_store_sweeps_a_stale_temp_file_left_by_a_killed_save() {
        let root = temporary_root("stale-temp");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.ini");
        std::fs::write(&path, "volume=0.25\n").unwrap();
        let stale = root.join(".settings.ini.tmp-999-0");
        std::fs::write(&stale, "volume=0.9\nre").unwrap();
        age_file(&stale, 1200);
        let fresh = root.join(".settings.ini.tmp-999-1");
        std::fs::write(&fresh, "volume=0.9\n").unwrap();

        let store = FileStore::at(path);
        assert_eq!(store.load(), Some(Settings::from_text("volume=0.25\n")));

        assert!(!stale.exists());
        assert!(fresh.exists());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
        std::fs::remove_dir_all(&root).unwrap();
    }

    // `NOSTALGIA_SETTINGS_PATH=settings.ini` has an empty parent: the sibling
    // temp lives in the current directory, so the sweep must look there too.
    #[test]
    fn the_sweep_covers_a_bare_file_name_in_the_current_directory() {
        let name = format!("nostalgia-bare-{}.ini", std::process::id());
        let stale = PathBuf::from(format!(".{name}.tmp-999-0"));
        let fresh = PathBuf::from(format!(".{name}.tmp-999-1"));
        std::fs::write(&stale, "x").unwrap();
        age_file(&stale, 1200);
        std::fs::write(&fresh, "x").unwrap();

        sweep_stale_temporaries(Path::new(&name));

        let stale_gone = !stale.exists();
        let fresh_kept = fresh.exists();
        std::fs::remove_file(&stale).ok();
        std::fs::remove_file(&fresh).ok();
        assert!(stale_gone);
        assert!(fresh_kept);
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
