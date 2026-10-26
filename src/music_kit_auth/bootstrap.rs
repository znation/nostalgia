//! The owner-only bootstrap page the browser is sent to, and the private
//! temp-directory handling around it.

use std::env;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::page::render_bootstrap_page;
use super::random_nonce;

/// A private, single-use bootstrap page that redirects the browser to the
/// sign-in URL.
///
/// The sign-in URL carries the per-flow `state` nonce. Handing it to the
/// browser as a command-line argument would expose it to every local user
/// through `/proc/<pid>/cmdline` on Linux (and `ps` on macOS). Writing the URL
/// into an owner-only file and opening *that* file keeps the nonce out of the
/// only world-readable channel to the browser: the spawned opener and browser
/// see this file's path, never the URL. The file and its directory are removed
/// when the guard is dropped, after the flow has finished with them.
pub(super) struct BootstrapPage {
    directory: PathBuf,
    file: PathBuf,
}

/// The prefix every bootstrap directory gets. A new sign-in sweeps only the
/// entries carrying it, so a cleanup cannot touch unrelated temp files.
pub(super) const BOOTSTRAP_DIR_PREFIX: &str = "nostalgia-auth-";

/// How old a bootstrap directory must be before a new sign-in removes it as
/// the leftover of a flow that never finished. Twice
/// [`crate::music_kit_auth::AUTH_TIMEOUT`]: a live flow removes its own
/// directory when it returns, so a directory still there after twice the
/// flow's whole budget belonged to a process that died (or was suspended and
/// resumed after its flow had already given up).
pub(super) const BOOTSTRAP_STALE_AFTER: Duration = Duration::from_secs(600);

/// Removes `root`'s bootstrap directories whose last modification is at or
/// before `now - stale_after`.
///
/// The [`BootstrapPage`] guard removes its directory when its flow returns, so
/// a directory left in the temp dir means the process died during sign-in and
/// the guard never ran. Removing it on the next sign-in keeps a crashed run
/// from leaving one `nostalgia-auth-*` directory (with the stale loopback URL
/// and nonce it embeds) behind per attempt. Only real directories carrying the
/// bootstrap prefix are removed; a symlink is skipped rather than followed,
/// and an entry with no readable modification time or an unrelated name is
/// left alone. A failed removal is reported and does not stop the sweep or the
/// sign-in: the new bootstrap needs a new, differently named directory.
pub(super) fn sweep_stale_bootstrap_dirs(root: &Path, now: SystemTime, stale_after: Duration) {
    sweep_stale_bootstrap_dirs_with(
        root,
        now,
        stale_after,
        |path| fs::remove_dir_all(path),
        |report| {
            eprintln!("{report}");
        },
    );
}

/// [`sweep_stale_bootstrap_dirs`] with its removal and reporter injected, so a
/// test can force a removal failure and observe the report without capturing
/// stderr.
pub(super) fn sweep_stale_bootstrap_dirs_with(
    root: &Path,
    now: SystemTime,
    stale_after: Duration,
    remove: impl Fn(&Path) -> io::Result<()>,
    report: impl Fn(String),
) {
    let Some(cutoff) = now.checked_sub(stale_after) else {
        return;
    };
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("could not read the temp directory to clean up stale sign-in pages: {error}");
            return;
        }
    };
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with(BOOTSTRAP_DIR_PREFIX) {
            continue;
        }
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !metadata.file_type().is_dir() {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if modified > cutoff {
            continue;
        }
        if let Err(error) = remove(&path) {
            report(sweep_removal_failure_report(&path, &error));
        }
    }
}

/// The report for a stale bootstrap directory the sweep could not remove.
///
/// `path` comes from an entry name in the shared temp directory, which any
/// local user can create, so it is formatted with `Debug` to escape a terminal
/// control character (`\u{1b}`) rather than pass the attacker-chosen name to
/// the terminal raw, matching the notices for server-controlled pagination
/// links in `apple_music::rest::paging`.
fn sweep_removal_failure_report(path: &Path, error: &io::Error) -> String {
    format!("could not remove the stale sign-in page directory {path:?}: {error}")
}

impl BootstrapPage {
    /// Writes a redirect page carrying `url` into a fresh owner-only temp
    /// directory and returns the guard that owns it.
    ///
    /// A stale directory left by a crashed earlier flow is swept first (see
    /// [`sweep_stale_bootstrap_dirs`]); the sweep is best effort and never
    /// fails the new sign-in.
    pub(super) fn write(url: &str) -> io::Result<Self> {
        sweep_stale_bootstrap_dirs(&env::temp_dir(), SystemTime::now(), BOOTSTRAP_STALE_AFTER);
        Self::write_with(url, &write_private_file)
    }

    /// [`Self::write`] with an injectable page writer, so a test can make the
    /// write fail and prove the guard still cleans up the directory it made.
    ///
    /// The guard is built as soon as the directory exists, *before* the page
    /// is written: if the write fails, that guard is dropped as the error
    /// propagates and removes the directory, so a failed sign-in never leaves
    /// a `nostalgia-auth-*` entry (and a partial page) behind in the temp
    /// directory.
    pub(super) fn write_with(
        url: &str,
        write_page: &dyn Fn(&Path, &str) -> io::Result<()>,
    ) -> io::Result<Self> {
        // Retry on the vanishingly unlikely name collision so a stale entry
        // cannot wedge the flow; `create_dir` refuses an existing path, so a
        // pre-planted file or symlink is never followed.
        for _ in 0..8 {
            let directory =
                env::temp_dir().join(format!("{BOOTSTRAP_DIR_PREFIX}{}", random_nonce()));
            match create_private_dir(&directory) {
                Ok(()) => {
                    // Own the directory before writing, so a failed write
                    // drops this guard and removes it rather than leaking it.
                    let file = directory.join("sign-in.html");
                    let page = Self { directory, file };
                    write_page(&page.file, &render_bootstrap_page(url))?;
                    return Ok(page);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not create a private temp directory for the sign-in page",
        ))
    }

    /// The local file path handed to the browser opener.
    pub(super) fn path_string(&self) -> String {
        self.file.to_string_lossy().into_owned()
    }
}

impl Drop for BootstrapPage {
    /// Removes the page and its directory. A failure is ignored: the flow has
    /// already finished, so there is nothing left to retry or report.
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.file);
        // Remove the whole directory tree so a leftover temp file from an
        // interrupted write cannot keep the directory from being cleaned up.
        let _ = fs::remove_dir_all(&self.directory);
    }
}

/// Creates `directory` with owner-only permissions. The mode is set at
/// creation (via `mkdir`), so there is no window where another user can enter
/// the directory.
#[cfg(unix)]
pub(super) fn create_private_dir(directory: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(0o700).create(directory)
}

/// Creates `directory`. The world-readable `/proc` argument channel this guard
/// defends against is Unix-specific, so the non-Unix fallback needs no mode.
#[cfg(not(unix))]
pub(super) fn create_private_dir(directory: &Path) -> io::Result<()> {
    fs::DirBuilder::new().create(directory)
}

/// Writes `contents` to `file`, creating it with owner-only permissions and
/// refusing to overwrite an existing path.
#[cfg(unix)]
pub(super) fn write_private_file(file: &Path, contents: &str) -> io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    // Write to a sibling temp file first, then hard-link it into place: a
    // crash during the write leaves the temp file behind (along with its
    // directory) instead of a truncated page, and `hard_link` fails with
    // `AlreadyExists` when `file` already exists, preserving the refusal of
    // a pre-planted path or symlink.
    let temp = file.with_file_name(format!(".sign-in-{}.tmp", random_nonce()));
    let result = (|| {
        let mut handle = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)?;
        handle.write_all(contents.as_bytes())?;
        handle.sync_all()?;
        fs::hard_link(&temp, file)?;
        let _ = fs::remove_file(&temp);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Writes `contents` to `file`, refusing to overwrite an existing path. The
/// world-readable `/proc` argument channel this guard defends against is
/// Unix-specific, so the non-Unix fallback needs no mode.
#[cfg(not(unix))]
pub(super) fn write_private_file(file: &Path, contents: &str) -> io::Result<()> {
    let mut handle = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file)?;
    handle.write_all(contents.as_bytes())
}
