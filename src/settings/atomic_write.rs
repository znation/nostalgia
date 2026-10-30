//! Crash-safe replacement of the settings file and cleanup of the temp files a
//! killed save leaves behind: [`write_atomically`] stages a hidden sibling and
//! renames it over the target, and [`sweep_stale_temporaries`] removes
//! siblings old enough to be debris.

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

/// The mode (`st_mode` permission bits) a brand-new settings file is staged
/// and published with. The file can carry the `MusicKit` sign-in's user token
/// (`music_kit_user_token=`), so it must be readable by its owner alone —
/// the same owner-only file the sign-in bootstrap directory gets — whatever
/// the process umask is; the explicit set after the staged bytes are written
/// is what makes the published file umask-proof.
///
/// [`set_staged_mode`]: fn@set_staged_mode
#[cfg(unix)]
const NEW_SETTINGS_FILE_MODE: u32 = 0o600;

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
pub(super) fn sweep_stale_temporaries(path: &Path) {
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
pub(super) fn write_atomically(path: &Path, text: &str) -> io::Result<()> {
    write_atomically_syncing(path, text, File::sync_all, sync_parent_directory)
}

/// Flush the directory entry of `path` (the rename) to disk. Directories
/// cannot be opened for syncing on non-Unix platforms, where this does nothing.
pub(super) fn sync_parent_directory(path: &Path) -> io::Result<()> {
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
pub(super) fn write_atomically_syncing(
    path: &Path,
    text: &str,
    sync: impl FnOnce(&File) -> io::Result<()>,
    sync_directory: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<()> {
    // Resolve once so the directory synced is the one the rename happened in.
    let path = resolve_symlink(path);
    write_atomically_with(&path, |file| {
        file.write_all(text.as_bytes())?;
        sync(file)?;
        Ok(())
    })?;
    sync_directory(&path)
}

/// The target's mode (`st_mode` permission bits) resolved before the staged
/// bytes are written: a metadata failure there fails the save instead of
/// publishing it with the default mode. `None` when the target does not exist.
#[cfg(unix)]
fn existing_stage_mode(path: &Path) -> Option<std::fs::Permissions> {
    std::fs::metadata(path).ok().map(|meta| meta.permissions())
}
#[cfg(not(unix))]
fn existing_stage_mode(path: &Path) -> Option<std::fs::Permissions> {
    let _ = path;
    None
}

/// Create the staged replacement file already carrying the mode it will end
/// up with — the existing target's mode, or owner-only for a new one — so no
/// interval exists between the staging and the rename where the staged
/// settings (which can carry the sign-in token) are readable beyond the mode
/// the rename publishes them under.
#[cfg(unix)]
fn open_staged_file(temporary: &Path, existing: Option<std::fs::Permissions>) -> io::Result<File> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(
            existing
                .map(|permissions| permissions.mode())
                .unwrap_or(NEW_SETTINGS_FILE_MODE),
        )
        .open(temporary)
}
/// On non-Unix platforms settings-file permission bits do not apply, so the
/// staged file is created the default way.
#[cfg(not(unix))]
fn open_staged_file(temporary: &Path, existing: Option<std::fs::Permissions>) -> io::Result<File> {
    let _ = existing;
    File::create(temporary)
}

/// Keep the existing file's mode exactly, as writing in place would have:
/// `open`'s mode argument is filtered by the process umask, so the staging
/// mode alone could have dropped a pre-existing bit, and a brand-new file is
/// made umask-proof against the default mode it would otherwise take.
#[cfg(unix)]
fn set_staged_mode(temporary: &Path, existing: Option<std::fs::Permissions>) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = existing
        .map(|permissions| permissions.mode())
        .unwrap_or(NEW_SETTINGS_FILE_MODE);
    std::fs::set_permissions(temporary, std::fs::Permissions::from_mode(mode))
}
#[cfg(not(unix))]
fn set_staged_mode(temporary: &Path, existing: Option<std::fs::Permissions>) -> io::Result<()> {
    let _ = (temporary, existing);
    Ok(())
}

pub(super) fn write_atomically_with(
    path: &Path,
    write_temporary: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<()> {
    // Rename over the file a symlink points at, not over the link itself; a
    // dangling link or a missing file keeps the path as given.
    let resolved = resolve_symlink(path);
    let path = resolved.as_path();
    let temporary = temporary_sibling(path);
    let existing = existing_stage_mode(path);
    let staged = open_staged_file(&temporary, existing.clone());
    let result = match staged {
        Ok(mut file) => write_temporary(&mut file),
        Err(error) => Err(error),
    };
    let result = result
        .and_then(|()| set_staged_mode(&temporary, existing))
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

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nostalgia-atomic-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    // A save killed mid-write must not truncate the previous file: the
    // replacement is staged aside, and a failure removes the staging file.
    #[test]
    fn a_failed_half_write_leaves_the_previous_settings_file_intact() {
        let root = temporary_root("half-write");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.ini");
        std::fs::write(&path, "volume=0.25\n").unwrap();

        let error = write_atomically_with(&path, |file| {
            // Model a disk filling up after the first few bytes.
            file.write_all(b"volume=0.9\nre")?;
            Err(io::Error::other("disk full"))
        })
        .unwrap_err();

        assert_eq!(error.to_string(), "disk full");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "volume=0.25\n");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }

    // A brand-new settings file is published owner-only, whatever the umask:
    // the file can carry the `music_kit_user_token=` sign-in token, and its
    // mode must not depend on the umask the save happened to run under.
    #[cfg(unix)]
    #[test]
    fn a_new_settings_file_is_written_owner_only_whatever_the_umask() {
        use std::os::unix::fs::PermissionsExt;

        let root = temporary_root("new-mode");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.ini");

        write_atomically(&path, "volume=0.9\nmusic_kit_user_token=x.y.z\n").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
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
}
