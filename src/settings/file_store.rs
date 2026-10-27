//! The settings persistence seam: the [`SettingsWriter`] trait the UI holds
//! behind a box, and [`FileStore`], the production writer that reads and
//! writes the line-based settings file.

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
    /// Set when [`Self::load`] found the file but could not read it, so its
    /// values are unknown. Saves then refuse to replace it rather than write
    /// defaults over a file the user never saw. Shared with clones: the
    /// production writer runs on a clone of the store `main` loaded from.
    read_failed: Arc<AtomicBool>,
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
            read_failed: Arc::new(AtomicBool::new(false)),
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
        Self {
            path: Some(path),
            read_failed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Read and parse the settings file, or `None` when persistence is
    /// disabled, the file is missing, or it cannot be read. Malformed lines
    /// fall back per key through [`Settings::from_text`].
    ///
    /// A file that exists but cannot be read is not the defaults: its values
    /// are unknown, so the store records the failure and later [`save`] calls
    /// refuse to replace it (see [`Self::save`]). A missing file is the normal
    /// first run and records nothing.
    ///
    /// [`save`]: SettingsWriter::save
    #[must_use]
    pub fn load(&self) -> Option<Settings> {
        self.load_with(read_settings_file)
    }

    /// [`Self::load`] with the byte reader injected, so a test can make the
    /// read fail without relying on file permissions (which differ for a
    /// privileged test runner).
    fn load_with(&self, read: impl FnOnce(&Path) -> io::Result<Vec<u8>>) -> Option<Settings> {
        let path = self.path.as_ref()?;
        sweep_stale_temporaries(path);
        // Read bytes and decode lossily: one corrupt byte must spoil only its
        // own line, not make `read_to_string` reject the whole file and reset
        // every other setting.
        match read(path) {
            Ok(bytes) => Some(Settings::from_text(&String::from_utf8_lossy(&bytes))),
            Err(error) => {
                if let Some(message) = read_failure_message(path, &error) {
                    // The file's values are unknown, so replacing it with a
                    // snapshot built from defaults would lose them.
                    self.read_failed.store(true, Ordering::Relaxed);
                    eprintln!("{message}");
                }
                None
            }
        }
    }
}

/// The largest settings file [`FileStore::load`] will read.
///
/// The file holds a fixed set of short lines, so a legitimate one is a few
/// hundred bytes. A much larger file is not this app's settings: an
/// accidentally overridden path, a corrupted file, or a file whose reported
/// length is meaningless must not be read into memory on the startup thread.
/// Oversized input is treated like an unreadable file, so the load falls back
/// to defaults and later saves leave it alone.
const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;

/// Reads the settings file without trusting its path or length.
///
/// `std::fs::read` opens whatever the path names: a FIFO or device can block
/// the caller forever, and a huge file can exhaust memory. [`FileStore::load`]
/// runs on the startup thread before the window opens, so either fault leaves
/// the player unable to start. On Unix a nonblocking open prevents a FIFO
/// from waiting for a writer, even if the path or its symlink changes. Metadata
/// is checked on the opened handle before reading. The read bound also covers
/// unreliable lengths (`/proc` files report zero).
fn read_settings_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut options = File::options();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the settings path is not a regular file",
        ));
    }
    read_settings_bytes(file, metadata.len())
}

/// Check the reported length and bound the actual read independently of it.
fn read_settings_bytes(reader: impl Read, reported_len: u64) -> io::Result<Vec<u8>> {
    if reported_len > MAX_SETTINGS_BYTES {
        return Err(oversized_settings_error());
    }
    let mut bytes = Vec::new();
    reader
        .take(MAX_SETTINGS_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_SETTINGS_BYTES {
        return Err(oversized_settings_error());
    }
    Ok(bytes)
}

/// The error for a settings file larger than [`MAX_SETTINGS_BYTES`], naming
/// the cap so the stderr line explains why the file was not read.
fn oversized_settings_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "the settings file is larger than {} KiB",
            MAX_SETTINGS_BYTES / 1024
        ),
    )
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

/// The diagnostic for a save refused because the file could not be read at
/// startup: names the file, why the save stopped, and the way to recover, so a
/// settings file that is never updated for the rest of the run is explained
/// instead of silent.
fn unreadable_settings_save_report(path: &Path) -> String {
    format!(
        "refusing to overwrite the settings file {}: it could not be read at startup, so its values are unknown; repair or remove it and restart to save settings again",
        path.display()
    )
}

impl SettingsWriter for FileStore {
    fn save(&self, settings: &Settings) {
        let Some(path) = self.path.as_ref() else {
            return;
        };
        // A save writes the whole snapshot the app has, which on a failed load
        // is built from defaults. Replacing a file whose contents could not be
        // read would destroy values the user still has, so refuse it and name
        // both the reason and how to recover.
        if self.read_failed.load(Ordering::Relaxed) {
            eprintln!("{}", unreadable_settings_save_report(path));
            return;
        }
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
    // Saves stage beside the file a symlink points at, so sweep there.
    let path = &resolve_symlink(path);
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

/// The file a symlink at `path` points at; a dangling link or a missing file
/// keeps the path as given. Save and sweep both use it so they agree on the
/// directory that holds temp files.
fn resolve_symlink(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
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

/// Replace `path` (or, for a symlink, the file it points at) with `text` by
/// writing a sibling temp file of that file, syncing it, and
/// renaming it over the target, so a process killed mid-write leaves the
/// previous file intact instead of a truncated one. A failed write, sync, or
/// rename removes the temp file and reports the error. The containing
/// directory is synced after the rename so the new name survives a power loss;
/// a failure there is reported although the new file is already in place.
fn write_atomically(path: &Path, text: &str) -> io::Result<()> {
    write_atomically_syncing(path, text, File::sync_all, sync_parent_directory)
}

/// Flush the directory entry of `path` (the rename) to disk. Directories
/// cannot be opened for syncing on non-Unix platforms, where this does nothing.
fn sync_parent_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        File::open(parent)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

/// [`write_atomically`] with the flush steps injected. The file sync runs after the
/// bytes are written and before the rename: without it a delayed-allocation
/// filesystem could report ENOSPC during writeback after the rename already
/// replaced the previous file, and a crash could persist the rename without
/// the new contents. `sync_directory` runs only after a successful rename.
fn write_atomically_syncing(
    path: &Path,
    text: &str,
    sync: impl FnOnce(&File) -> io::Result<()>,
    sync_directory: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<()> {
    // Resolve once so the directory synced is the one the rename happened in.
    let path = resolve_symlink(path);
    write_atomically_with(&path, |temporary| {
        let mut file = File::create(temporary)?;
        file.write_all(text.as_bytes())?;
        sync(&file)
    })?;
    sync_directory(&path)
}

fn write_atomically_with(
    path: &Path,
    write_temporary: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<()> {
    // Rename over the file a symlink points at, not over the link itself; a
    // dangling link or a missing file keeps the path as given.
    let resolved = resolve_symlink(path);
    let path = resolved.as_path();
    let temporary = temporary_sibling(path);
    let result = write_temporary(&temporary)
        .and_then(|()| {
            // Keep the existing file's mode, as writing in place would have.
            match std::fs::metadata(path) {
                Ok(existing) => std::fs::set_permissions(&temporary, existing.permissions()),
                Err(_) => Ok(()),
            }
        })
        .and_then(|()| std::fs::rename(&temporary, path));
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

    #[test]
    fn unreadable_settings_save_report_names_the_file_and_the_recovery() {
        let report = unreadable_settings_save_report(Path::new("/cfg/settings.ini"));
        assert!(report.contains("/cfg/settings.ini"), "{report}");
        assert!(report.contains("refusing to overwrite"), "{report}");
        assert!(report.contains("could not be read at startup"), "{report}");
        assert!(report.contains("restart"), "{report}");
    }

    // A settings file that exists but cannot be read holds values the app never
    // saw; the snapshot it would save is built from defaults, so writing it
    // would destroy the user's settings. The store records the failed load and
    // the later save leaves the file (and stages nothing) alone.
    #[test]
    fn a_settings_file_that_could_not_be_read_is_not_overwritten() {
        let root = temporary_root("unreadable");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.ini");
        std::fs::write(&path, "volume=0.25\n").unwrap();
        let store = FileStore::at(path.clone());

        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        assert_eq!(store.load_with(|_| Err(denied)), None);

        store.save(&varied_settings());

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "volume=0.25\n",
            "the refused save must leave the unreadable file's bytes intact"
        );
        assert_eq!(
            std::fs::read_dir(&root).unwrap().count(),
            1,
            "the refused save must not stage a temp file beside it"
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    // Production loads through `main`'s store and hands the writer a clone. The
    // failed read is recorded in storage shared by every clone, so the clone
    // the background writer owns must refuse too; a per-value flag would let
    // the writer replace the still-intact file with the defaults snapshot.
    #[test]
    fn a_writer_clone_shares_the_failed_read_and_refuses_to_overwrite() {
        use crate::settings::writer::BackgroundWriter;

        let root = temporary_root("unreadable-clone");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.ini");
        std::fs::write(&path, "volume=0.25\n").unwrap();
        let store = FileStore::at(path.clone());

        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        assert_eq!(store.load_with(|_| Err(denied)), None);

        let writer = BackgroundWriter::new(store.clone());
        writer.save(&varied_settings());
        drop(writer);

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "volume=0.25\n",
            "a writer clone must leave the file its original could not read intact"
        );
        assert_eq!(
            std::fs::read_dir(&root).unwrap().count(),
            1,
            "the refused save must not stage a temp file beside the file"
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    // A settings path can be overridden to something that is not this app's
    // settings. Reading it with no bound can block forever (a FIFO or device)
    // or exhaust memory (a huge file) on the startup thread, so an oversized
    // file is refused like an unreadable one: the load falls back to defaults
    // and a later save leaves the file untouched.
    #[test]
    fn an_oversized_settings_file_is_refused_without_being_read_or_overwritten() {
        let root = temporary_root("oversized");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.ini");
        std::fs::File::create(&path)
            .unwrap()
            .set_len(MAX_SETTINGS_BYTES + 1)
            .unwrap();
        let store = FileStore::at(path.clone());

        assert_eq!(store.load(), None);

        store.save(&varied_settings());

        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            MAX_SETTINGS_BYTES + 1,
            "the refused save must leave the oversized file unchanged"
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn understated_settings_lengths_still_enforce_the_read_cap() {
        for length in [MAX_SETTINGS_BYTES - 1, MAX_SETTINGS_BYTES] {
            let mut reader = io::Cursor::new(vec![b'x'; length as usize]);
            assert_eq!(
                read_settings_bytes(&mut reader, 0).unwrap().len(),
                length as usize
            );
            assert_eq!(reader.position(), length);
        }

        let mut reader = io::Cursor::new(vec![b'x'; (MAX_SETTINGS_BYTES + 100) as usize]);
        assert_eq!(
            read_settings_bytes(&mut reader, 0).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(reader.position(), MAX_SETTINGS_BYTES + 1);
    }

    // Exercise the open itself with no FIFO writer. A blocking open would
    // hang, so run this case in a child with a deadline enforced by the parent.
    #[cfg(unix)]
    #[test]
    fn a_fifo_settings_path_does_not_wait_for_a_writer() {
        const CHILD: &str = "NOSTALGIA_FIFO_TEST_CHILD";
        if let Some(path) = std::env::var_os(CHILD) {
            assert_eq!(
                read_settings_file(Path::new(&path)).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
            return;
        }
        let root = temporary_root("fifo-open");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.ini");
        rustix::fs::mkfifoat(
            rustix::fs::CWD,
            &path,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        )
        .unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "settings::file_store::tests::a_fifo_settings_path_does_not_wait_for_a_writer",
            ])
            .env(CHILD, &path)
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break Some(status);
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                break None;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        std::fs::remove_dir_all(&root).unwrap();
        assert!(
            status.is_some_and(|status| status.success()),
            "FIFO open blocked or failed"
        );
    }

    // Non-regular handles are refused before reading, even when the device
    // would return an empty file.
    #[cfg(unix)]
    #[test]
    fn a_non_regular_settings_path_is_treated_as_unreadable() {
        let store = FileStore::at(PathBuf::from("/dev/null"));

        assert_eq!(store.load(), None);

        store.save(&varied_settings());

        assert_eq!(store.load(), None);
    }

    // A missing file is the normal first run, not a failure: the first save must
    // still create the file rather than being refused for the whole run.
    #[test]
    fn a_missing_settings_file_still_allows_the_first_save() {
        let root = temporary_root("missing-first-save");
        let path = root.join("settings.ini");
        let store = FileStore::at(path.clone());

        assert_eq!(store.load(), None);

        let settings = varied_settings();
        store.save(&settings);

        assert_eq!(store.load(), Some(settings));

        std::fs::remove_dir_all(&root).unwrap();
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
        let root = temporary_root("round-trip");
        let path = root.join("nested").join("settings.ini");
        let store = FileStore::at(path);

        let settings = varied_settings();

        store.save(&settings);
        assert_eq!(store.load(), Some(settings));

        std::fs::remove_dir_all(&root).unwrap();
    }

    // A save path with no directory component (`settings.ini`, say) belongs to
    // the process's current directory, whose `Path::parent` is the empty path.
    // The store must skip creating that empty parent and stage, rename, and
    // sync beside the bare name rather than fail before writing. The sweep's
    // bare-name path is pinned separately; this covers the save side.
    #[test]
    fn file_store_saves_and_loads_a_bare_file_name_in_the_current_directory() {
        let name = format!("{}.ini", temporary_name("bare-save"));
        let store = FileStore::at(PathBuf::from(&name));

        let settings = varied_settings();
        store.save(&settings);

        assert_eq!(store.load(), Some(settings));
        assert!(
            Path::new(&name).is_file(),
            "the bare save must land in the current directory"
        );
        let temp_prefix = format!(".{name}.tmp-");
        assert!(
            !std::fs::read_dir(".").unwrap().flatten().any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with(&temp_prefix)),
            "the bare save must not leave its staged temp file behind"
        );

        std::fs::remove_file(&name).unwrap();
    }

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(temporary_name(label))
    }

    /// Build a scratch name from the test label, process ID, and current timestamp.
    fn temporary_name(label: &str) -> String {
        format!(
            "nostalgia-settings-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
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
        let store = FileStore {
            path: None,
            read_failed: Arc::new(AtomicBool::new(false)),
        };

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

    // A symlinked settings file must stay a symlink: the save replaces the
    // file it points at, and the previous file's mode carries over.
    #[cfg(unix)]
    #[test]
    fn saving_through_a_symlink_updates_its_target_and_keeps_mode() {
        use std::os::unix::fs::PermissionsExt;

        let root = temporary_root("symlink");
        std::fs::create_dir_all(&root).unwrap();
        let target = root.join("real.ini");
        std::fs::write(&target, "volume=0.25\n").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640)).unwrap();
        let link = root.join("settings.ini");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        write_atomically(&link, "volume=0.9\n").unwrap();

        assert!(std::fs::symlink_metadata(&link).unwrap().is_symlink());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "volume=0.9\n");
        let mode = std::fs::metadata(&target).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o640);
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
        write_atomically_syncing(
            &path,
            "volume=0.9\n",
            |file| {
                synced_with_old_target = std::fs::read_to_string(&path).unwrap() == "volume=0.25\n";
                file.sync_all()
            },
            |_| Ok(()),
        )
        .unwrap();
        assert!(synced_with_old_target);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "volume=0.9\n");

        let mut directory_synced = false;
        let error = write_atomically_syncing(
            &path,
            "volume=0.1\n",
            |_| Err(io::Error::other("no space left")),
            |_| {
                directory_synced = true;
                Ok(())
            },
        )
        .unwrap_err();
        assert!(!directory_synced);
        assert_eq!(error.to_string(), "no space left");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "volume=0.9\n");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }

    // The directory is synced after the rename, so the new file is already in
    // place when it runs, and its failure is reported rather than swallowed.
    #[test]
    fn the_directory_is_synced_after_the_rename_and_its_failure_is_reported() {
        let root = temporary_root("dir-sync");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.ini");
        std::fs::write(&path, "volume=0.25\n").unwrap();

        let mut contents_at_sync = String::new();
        let error = write_atomically_syncing(&path, "volume=0.9\n", File::sync_all, |target| {
            contents_at_sync = std::fs::read_to_string(target).unwrap();
            Err(io::Error::other("directory sync failed"))
        })
        .unwrap_err();

        assert_eq!(contents_at_sync, "volume=0.9\n");
        assert_eq!(error.to_string(), "directory sync failed");
        sync_parent_directory(&path).unwrap();
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

    // The directory sync after a save through a symlink targets the file the
    // rename replaced, not the link.
    #[cfg(unix)]
    #[test]
    fn the_directory_sync_follows_a_symlink_to_the_target() {
        let root = temporary_root("symlink-dir-sync");
        std::fs::create_dir_all(&root).unwrap();
        let target = root.join("real.ini");
        std::fs::write(&target, "volume=0.25\n").unwrap();
        let link = root.join("settings.ini");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let mut synced = None;
        write_atomically_syncing(&link, "volume=0.9\n", File::sync_all, |dir_target| {
            synced = Some(dir_target.to_path_buf());
            Ok(())
        })
        .unwrap();

        assert_eq!(synced, Some(std::fs::canonicalize(&target).unwrap()));
        std::fs::remove_dir_all(&root).unwrap();
    }

    // A killed save through a symlink leaves its temp beside the link's
    // target, so a load through the link must sweep there.
    #[cfg(unix)]
    #[test]
    fn the_sweep_follows_a_symlink_to_the_targets_directory() {
        let root = temporary_root("symlink-sweep");
        let target_dir = root.join("dotfiles");
        let link_dir = root.join("config");
        std::fs::create_dir_all(&target_dir).unwrap();
        std::fs::create_dir_all(&link_dir).unwrap();
        let target = target_dir.join("nostalgia.ini");
        std::fs::write(&target, "volume=0.25\n").unwrap();
        let link = link_dir.join("settings.ini");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let stale = target_dir.join(".nostalgia.ini.tmp-999-0");
        std::fs::write(&stale, "volume=0.9\nre").unwrap();
        age_file(&stale, 1200);
        let fresh = target_dir.join(".nostalgia.ini.tmp-999-1");
        std::fs::write(&fresh, "x").unwrap();

        let store = FileStore::at(link);
        assert_eq!(store.load(), Some(Settings::from_text("volume=0.25\n")));

        assert!(!stale.exists());
        assert!(fresh.exists());
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
