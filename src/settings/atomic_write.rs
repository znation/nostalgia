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
    write_atomically_with(&path, |temporary| {
        let mut file = File::create(temporary)?;
        file.write_all(text.as_bytes())?;
        sync(&file)
    })?;
    sync_directory(&path)
}

pub(super) fn write_atomically_with(
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
