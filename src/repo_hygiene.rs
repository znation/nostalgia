//! Repository-hygiene guards: no stray backup artifacts are committed, and every
//! `**Pinned by.**` entry in `docs/invariants.md` names a test the file it cites defines.

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

    /// Every `**Pinned by.**` entry in `docs/invariants.md` must name a test the
    /// file it cites defines: one bullet per test, reading ``- `name` (in `path`)``.
    /// A renamed, moved, or deleted pinned test then breaks the check until the
    /// invariant file is updated in the same commit, instead of leaving a section
    /// asserting a pin that no longer exists.
    #[test]
    fn every_pinned_by_entry_names_a_test_its_file_defines() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let doc = std::fs::read_to_string(root.join("docs/invariants.md"))
            .expect("read docs/invariants.md");
        let violations = pinned_by_violations(root, &doc);
        assert!(
            violations.is_empty(),
            "stale Pinned-by pins in docs/invariants.md:\n{}",
            violations.join("\n")
        );
    }

    /// The scan fails on a pinned name no cited file defines.
    #[test]
    fn the_pinned_by_scan_fails_on_a_name_no_file_defines() {
        let doc = concat!(
            "**Pinned by.**\n",
            "\n",
            "- `no_such_test_exists_anywhere` (in `src/ui/tests/loading_tests.rs`)\n",
            "\n",
            "## Next section\n",
        );
        let violations = pinned_by_violations(Path::new(env!("CARGO_MANIFEST_DIR")), doc);
        assert!(
            violations
                .iter()
                .any(|v| v.contains("no_such_test_exists_anywhere")),
            "expected the undefined name to be flagged, got: {violations:?}"
        );
    }

    /// The scan fails on an explicit path pointing at a file that does not
    /// define the name it carries (`src/ui/bevel.rs` is real but does not
    /// define the cited test).
    #[test]
    fn the_pinned_by_scan_fails_on_a_path_that_does_not_define_the_name() {
        let doc = concat!(
            "**Pinned by.**\n",
            "\n",
            "- `stale_flat_list_replies_superseded_before_update_are_not_stored` (in `src/ui/bevel.rs`)\n",
            "\n",
            "## Next section\n",
        );
        let violations = pinned_by_violations(Path::new(env!("CARGO_MANIFEST_DIR")), doc);
        assert!(
            violations.iter().any(|v| v.contains("does not define")),
            "expected the wrong path to be flagged, got: {violations:?}"
        );
    }

    /// A resolvable entry passes; the scan reads repository text only.
    #[test]
    fn the_pinned_by_scan_accepts_a_resolvable_entry() {
        let doc = concat!(
            "**Pinned by.**\n",
            "\n",
            "- `stale_flat_list_replies_superseded_before_update_are_not_stored` (in `src/ui/tests/loading_tests.rs`)\n",
            "\n",
            "## Next section\n",
        );
        let violations = pinned_by_violations(Path::new(env!("CARGO_MANIFEST_DIR")), doc);
        assert!(
            violations.is_empty(),
            "expected no violations, got: {violations:?}"
        );
    }

    /// A list entry with trailing text is malformed even when its name and path look valid.
    #[test]
    fn the_pinned_by_scan_rejects_trailing_text_in_an_entry() {
        let doc = concat!(
            "**Pinned by.**\n",
            "\n",
            "- `stale_flat_list_replies_superseded_before_update_are_not_stored` (in `src/ui/tests/loading_tests.rs`) trailing\n",
            "\n",
            "## Next section\n",
        );
        let violations = pinned_by_violations(Path::new(env!("CARGO_MANIFEST_DIR")), doc);
        assert!(
            violations.iter().any(|v| v.contains("entry does not read")),
            "expected the malformed entry to be flagged, got: {violations:?}"
        );
    }

    /// Walk the `**Pinned by.**` lists of `doc` and report every entry that does
    /// not resolve against the repository rooted at `root`: a list line not
    /// shaped ``- `name` (in `path`)``, a name its cited file does not define, or
    /// a path that cannot be read. The scan reads repository text only; it never
    /// runs the suite.
    fn pinned_by_violations(root: &Path, doc: &str) -> Vec<String> {
        let mut violations = Vec::new();
        let mut in_list = false;
        for (index, line) in doc.lines().enumerate() {
            let line_no = index + 1;
            if line.starts_with("**Pinned by.**") {
                in_list = true;
                continue;
            }
            if line.starts_with("## ") || line.starts_with("**") {
                in_list = false;
                continue;
            }
            if !in_list || line.trim().is_empty() {
                continue;
            }
            let parsed = line.strip_prefix("- ").and_then(parse_pinned_entry);
            let Some((name, path)) = parsed else {
                violations.push(format!(
                    "line {line_no}: entry does not read `- `name` (in `path`)`: {line}"
                ));
                continue;
            };
            match std::fs::read_to_string(root.join(&path)) {
                Ok(source) => {
                    if !source.contains(&format!("fn {name}(")) {
                        violations
                            .push(format!("line {line_no}: `{path}` does not define `{name}`"));
                    }
                }
                Err(_) => violations.push(format!(
                    "line {line_no}: `{path}` is not a readable repository file"
                )),
            }
        }
        violations
    }

    /// Split `` `name` (in `path`) `` into its name and path, rejecting anything
    /// else so a malformed entry cannot slip through unparseable.
    fn parse_pinned_entry(entry: &str) -> Option<(String, String)> {
        let rest = entry.strip_prefix('`')?;
        let close = rest.find('`')?;
        let name = &rest[..close];
        let well_formed_name = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if !well_formed_name {
            return None;
        }
        let rest = rest[close + 1..].strip_prefix(" (in `")?;
        let close = rest.find('`')?;
        let path = &rest[..close];
        if rest.get(close + 1..) != Some(")") {
            return None;
        }
        Some((name.to_string(), path.to_string()))
    }
}
