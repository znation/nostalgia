//! Repository-hygiene guard: no stray backup artifacts are committed.

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// Editor and tool backups (`*.bak`, `*.orig`, `*~`, fusion files) must not
    /// ride along in commits: they are stale duplicates of tracked text and rot.
    #[test]
    fn no_backup_artifacts_ship_in_the_repository() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut found = Vec::new();
        collect_backup_artifacts(root, &mut found);
        assert!(
            found.is_empty(),
            "stray backup artifacts in the repository: {found:?}"
        );
    }

    /// Depth-first walk collecting backup filenames, skipping `.git` and `target`.
    fn collect_backup_artifacts(dir: &Path, found: &mut Vec<String>) {
        let mut entries: Vec<_> = match std::fs::read_dir(dir) {
            Ok(entries) => entries.flatten().collect(),
            Err(_) => return,
        };
        entries.sort_unstable_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let path = entry.path();
            if path.is_dir() && (name == ".git" || name == "target") {
                continue;
            }
            if is_backup_artifact(&name) {
                found.push(path.to_string_lossy().into_owned());
            }
            if path.is_dir() {
                collect_backup_artifacts(&path, found);
            }
        }
    }

    fn is_backup_artifact(name: &str) -> bool {
        name.ends_with(".bak")
            || name.ends_with(".orig")
            || name.ends_with(".rej")
            || name.ends_with(".swp")
            || name.ends_with(".swo")
            || name.ends_with('~')
    }
}
