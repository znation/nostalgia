use super::*;

#[test]
fn bootstrap_page_is_private_and_carries_the_sign_in_url() {
    let url = "http://127.0.0.1:4321/?state=deadbeefdeadbeef";
    let page = BootstrapPage::write(url).expect("write the bootstrap page");
    let path = page.path_string();
    let directory = std::path::Path::new(&path)
        .parent()
        .expect("the bootstrap page has a directory")
        .to_path_buf();
    let html = std::fs::read_to_string(&path).expect("read the bootstrap page");
    assert!(
        html.contains(url),
        "the bootstrap page must redirect to the sign-in URL, got: {html}"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let file_mode = std::fs::metadata(&path)
            .expect("bootstrap metadata")
            .permissions()
            .mode();
        assert_eq!(
            file_mode & 0o077,
            0,
            "the bootstrap page must be owner-only, mode {file_mode:o}"
        );
        let dir_mode = std::fs::metadata(&directory)
            .expect("bootstrap directory metadata")
            .permissions()
            .mode();
        assert_eq!(
            dir_mode & 0o077,
            0,
            "the bootstrap directory must be owner-only, mode {dir_mode:o}"
        );
    }

    drop(page);
    assert!(
        !std::path::Path::new(&path).exists(),
        "dropping the bootstrap page removes it"
    );
    assert!(
        !directory.exists(),
        "dropping the bootstrap page removes its private directory, got: {}",
        directory.display()
    );
}

// Dropping a bootstrap page whose directory holds a leftover file (for
// example one left behind by an interrupted atomic write) must still remove
// the whole directory, not fail on `remove_dir` because it is non-empty.
#[test]
fn bootstrap_page_drop_removes_directory_with_leftover_files() {
    let url = "http://127.0.0.1:4321/?state=deadbeefdeadbeef";
    let page = BootstrapPage::write(url).expect("write the bootstrap page");
    let path = page.path_string();
    let directory = std::path::Path::new(&path)
        .parent()
        .expect("the bootstrap page has a directory")
        .to_path_buf();
    // Simulate a leftover temp file from an interrupted write.
    std::fs::write(directory.join(".sign-in-leftover.tmp"), b"partial").expect("plant leftover");

    drop(page);
    assert!(
        !directory.exists(),
        "dropping the bootstrap page removes a directory containing leftover files, got: {}",
        directory.display()
    );
}

// A failed page write must not leak the private directory the guard created:
// the guard owns the directory from the moment `create_private_dir` succeeds,
// so a write error (a full disk, say) drops it and removes the directory
// rather than leaving a `nostalgia-auth-*` entry and a partial page behind.
// `write_private_file` cannot be made to fail deterministically in a test, so
// `write_with` injects the failure; the production `write` calls the same
// helper with the real writer.
#[test]
fn a_failed_bootstrap_write_removes_the_private_directory() {
    let written_to = Mutex::new(None);
    let result = BootstrapPage::write_with(
        "http://127.0.0.1:1/?state=deadbeefdeadbeef",
        &|file, _contents| {
            *written_to.lock().expect("record lock") = Some(file.to_path_buf());
            Err(io::Error::other("simulated write failure"))
        },
    );

    assert!(result.is_err(), "the write failure must propagate");
    let file = written_to
        .lock()
        .expect("record lock")
        .clone()
        .expect("the page writer was called");
    assert!(
        !file.exists(),
        "the partial bootstrap page must be removed, got: {}",
        file.display()
    );
    let directory = file.parent().expect("the page has a parent directory");
    assert!(
        !directory.exists(),
        "the private directory must be removed when the page write fails, got: {}",
        directory.display()
    );
}

// A directory left behind by a process that died mid-sign-in has no guard to
// remove it, so a later sign-in sweeps it once it is older than twice the flow
// deadline. Only the aged bootstrap directories go: a fresh one may belong to
// a flow still running in another process, and an entry with another name or a
// non-directory entry is not ours to remove.
#[cfg(unix)]
#[test]
fn the_bootstrap_sweep_removes_only_old_bootstrap_directories() {
    let root = std::env::temp_dir().join(format!("nostalgia-sweep-{}", random_nonce()));
    std::fs::create_dir(&root).expect("create the sweep scratch root");
    let old = root.join(format!("{BOOTSTRAP_DIR_PREFIX}old"));
    let fresh = root.join(format!("{BOOTSTRAP_DIR_PREFIX}fresh"));
    let unrelated = root.join("other");
    let file = root.join(format!("{BOOTSTRAP_DIR_PREFIX}file"));
    std::fs::create_dir(&old).expect("create the old bootstrap directory");
    std::fs::create_dir(&fresh).expect("create the fresh bootstrap directory");
    std::fs::create_dir(&unrelated).expect("create an unrelated directory");
    std::fs::write(&file, b"not a directory").expect("create an unrelated file");
    std::fs::write(old.join("sign-in.html"), b"stale").expect("plant a stale page");

    let now = std::time::SystemTime::now();
    std::fs::File::open(&old)
        .expect("open the old directory")
        .set_modified(now - Duration::from_secs(3600))
        .expect("age the old directory");

    sweep_stale_bootstrap_dirs(&root, now, BOOTSTRAP_STALE_AFTER);

    assert!(
        !old.exists(),
        "an aged bootstrap directory must be swept, got: {}",
        old.display()
    );
    assert!(
        fresh.exists(),
        "a fresh bootstrap directory must be left alone"
    );
    assert!(
        unrelated.exists(),
        "an unrelated directory must be left alone"
    );
    assert!(file.exists(), "a non-directory entry must be left alone");

    std::fs::remove_dir_all(&root).expect("remove the sweep scratch root");
}

// A bootstrap directory's name comes from the shared temp directory, where any
// local user can create an aged entry carrying the prefix. A removal failure is
// reported, and the report must escape that attacker-chosen name: the sticky
// bit keeps another user's directory from being removed, so the failure path is
// reachable, and a raw control character in the name would otherwise reach the
// terminal.
#[cfg(unix)]
#[test]
fn the_bootstrap_sweep_escapes_a_hostile_directory_name_in_its_report() {
    let root = std::env::temp_dir().join(format!("nostalgia-sweep-report-{}", random_nonce()));
    std::fs::create_dir(&root).expect("create the sweep scratch root");
    let hostile = root.join(format!("{BOOTSTRAP_DIR_PREFIX}\u{1b}]0;pwned\u{7}"));
    std::fs::create_dir(&hostile).expect("create the hostile directory");
    let now = std::time::SystemTime::now();
    std::fs::File::open(&hostile)
        .expect("open the hostile directory")
        .set_modified(now - Duration::from_secs(3600))
        .expect("age the hostile directory");

    let reports = Mutex::new(Vec::new());
    sweep_stale_bootstrap_dirs_with(
        &root,
        now,
        BOOTSTRAP_STALE_AFTER,
        |_path| Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied")),
        |report| reports.lock().expect("report lock").push(report),
    );
    let reports = reports.into_inner().expect("report lock");
    assert_eq!(reports.len(), 1, "the failed removal must be reported once");
    let report = &reports[0];
    assert!(
        !report.contains('\u{1b}') && !report.contains('\u{7}'),
        "a control character in the directory name must not reach the terminal raw, got: {report:?}"
    );
    assert!(
        report.contains("\\u{1b}") && report.contains("\\u{7}"),
        "the report must show the name's control characters escaped, got: {report:?}"
    );

    std::fs::remove_dir_all(&root).expect("remove the sweep scratch root");
}

// A symlink carrying the bootstrap prefix must be skipped by its own type, not
// followed to its target's metadata: an aged target would otherwise make the
// sweep treat the link as a stale bootstrap directory and remove it.
#[cfg(unix)]
#[test]
fn the_bootstrap_sweep_skips_a_symlink_instead_of_following_it() {
    let root = std::env::temp_dir().join(format!("nostalgia-sweep-link-{}", random_nonce()));
    std::fs::create_dir(&root).expect("create the sweep scratch root");
    let target = root.join("victim");
    std::fs::create_dir(&target).expect("create the symlink target");
    std::fs::write(target.join("keep.txt"), b"keep").expect("plant the target's contents");
    let link = root.join(format!("{BOOTSTRAP_DIR_PREFIX}link"));
    std::os::unix::fs::symlink(&target, &link).expect("plant a symlink");

    let now = std::time::SystemTime::now();
    std::fs::File::open(&target)
        .expect("open the link target")
        .set_modified(now - Duration::from_secs(3600))
        .expect("age the link target");

    sweep_stale_bootstrap_dirs(&root, now, BOOTSTRAP_STALE_AFTER);

    assert!(
        std::fs::symlink_metadata(&link).is_ok(),
        "a symlink must be skipped, not removed: {}",
        link.display()
    );
    assert!(
        target.join("keep.txt").exists(),
        "the sweep must not follow the symlink into its target: {}",
        target.display()
    );

    std::fs::remove_dir_all(&root).expect("remove the sweep scratch root");
}

// The guard cannot run when the process dies, so the sweep must actually be
// wired into the next bootstrap write rather than only existing as a helper.
#[cfg(unix)]
#[test]
fn a_new_bootstrap_write_sweeps_a_stale_directory_from_a_crashed_flow() {
    let stale = std::env::temp_dir().join(format!("{BOOTSTRAP_DIR_PREFIX}{}", random_nonce()));
    std::fs::create_dir(&stale).expect("create the stale bootstrap directory");
    std::fs::write(stale.join("sign-in.html"), b"stale").expect("plant a stale page");
    std::fs::File::open(&stale)
        .expect("open the stale directory")
        .set_modified(std::time::SystemTime::now() - Duration::from_secs(3600))
        .expect("age the stale directory");

    let page = BootstrapPage::write("http://127.0.0.1:1/?state=deadbeefdeadbeef")
        .expect("write a new bootstrap page");
    assert!(
        !stale.exists(),
        "a new sign-in must sweep the crashed flow's directory, got: {}",
        stale.display()
    );
    drop(page);
}

// `render_auth_page` fills every template placeholder. The flow tests assert
// that the served page carries the developer token and the per-flow nonce, but
// nothing pins the build version the page hands MusicKit's `configure` as
// `app.build`: dropping the `{{VERSION}}` substitution, or adding a fourth
// placeholder without a matching `replace`, would serve a literal `{{...}}`
// into the browser's MusicKit config with the whole suite still green. Render
// the template directly and pin all three substitutions plus the absence of
// any surviving placeholder.
#[test]
fn render_auth_page_substitutes_every_template_placeholder() {
    let page = super::page::render_auth_page(SAMPLE_DEVELOPER_TOKEN, "deadbeefdeadbeef");

    assert!(
        page.contains(SAMPLE_DEVELOPER_TOKEN),
        "the page must embed the developer token"
    );
    assert!(
        page.contains("deadbeefdeadbeef"),
        "the page must embed the per-flow state nonce"
    );
    assert!(
        page.contains(&format!("build: \"{}\"", env!("CARGO_PKG_VERSION"))),
        "the page must hand MusicKit the crate's build version"
    );
    assert!(
        !page.contains("{{"),
        "no template placeholder may survive rendering, got: {page}"
    );
}

// The bootstrap guard's security claim is that it creates its directory and
// file with `create`/`create_new`, so an entry already at the path — a file an
// attacker pre-planted, or a symlink to a victim — is refused rather than
// adopted, followed, or overwritten. `BootstrapPage::write` names its
// directory with a fresh 64-bit nonce, so a collision cannot be forced through
// it; exercise the two guards directly, since they are what the claim rests on.
#[test]
fn bootstrap_guard_refuses_a_pre_planted_path() {
    let base = std::env::temp_dir().join(format!("nostalgia-guard-{}", random_nonce()));
    std::fs::create_dir(&base).expect("create the scratch directory");

    // `create_private_dir` must refuse a path that already exists — the
    // `AlreadyExists` signal `BootstrapPage::write` retries on — and must
    // leave the existing entry alone rather than adopt or replace it.
    let planted_dir = base.join("private");
    std::fs::write(&planted_dir, b"pre-planted").expect("plant a file at the directory path");
    let error = create_private_dir(&planted_dir).expect_err("an existing path is refused");
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(
        std::fs::read(&planted_dir).expect("read the planted file"),
        b"pre-planted",
        "the guard must not replace the entry already at the path"
    );

    // `write_private_file` opens with `create_new`, so a pre-planted symlink
    // is refused rather than followed: the link's target must stay untouched.
    #[cfg(unix)]
    {
        let victim = base.join("victim");
        std::fs::write(&victim, b"original").expect("write the symlink target");
        let planted_file = base.join("sign-in.html");
        std::os::unix::fs::symlink(&victim, &planted_file).expect("plant a symlink");
        let error = write_private_file(&planted_file, "attacker-controlled")
            .expect_err("an existing symlink is refused");
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(
            std::fs::read(&victim).expect("read the symlink target"),
            b"original",
            "the guard must not follow a pre-planted symlink"
        );
    }

    std::fs::remove_dir_all(&base).expect("remove the scratch directory");
}

#[test]
fn authorize_keeps_the_state_nonce_out_of_the_opener_argument() {
    // `/proc/<pid>/cmdline` (and `ps`) is world-readable on Unix, so the
    // sign-in URL — which carries the `state` nonce — must never be a
    // command-line argument. The flow writes the URL into an owner-only
    // bootstrap file and hands the opener only that file's path.
    let (opener, handles, calls) = background_capturing(|port, state| {
        let _ = request(port, &token_request(&state, SAMPLE_USER_TOKEN));
    });
    let session = authorize_with_timeout(SAMPLE_DEVELOPER_TOKEN, &opener, Duration::from_secs(5))
        .expect("the callback completes the flow");
    join_all(&handles);
    assert_eq!(session.user_token, SAMPLE_USER_TOKEN);

    let calls = calls.lock().expect("calls lock");
    let (path, url) = calls.first().expect("the opener was called");
    let nonce = state_of(url);
    assert!(
        !path.contains(&nonce),
        "the state nonce must not travel in the opener argument: {path}"
    );
    assert!(
        !path.contains("state=") && !path.starts_with("http"),
        "the opener must receive a local path, not the sign-in URL: {path}"
    );
}
