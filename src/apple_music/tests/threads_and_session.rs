use super::*;

// `startup_token` decides whether startup sign-in runs and, when it does not,
// which of the three skip cases the message should name. The three cases are
// otherwise indistinguishable through `init_service` (which reads the real,
// process-global environment), so the pure classifier is pinned directly. A
// present, non-blank token is returned exactly as read — trimming happens
// later, at the `authorize` boundary — so a token file's trailing newline
// still signs in rather than being dropped here.
#[test]
fn startup_token_names_the_missing_blank_and_non_utf8_cases() {
    assert_eq!(
        startup_token(Ok("dev-token".to_string())),
        Ok("dev-token".to_string())
    );
    assert_eq!(
        startup_token(Ok("  dev-token  ".to_string())),
        Ok("  dev-token  ".to_string())
    );
    assert_eq!(
        startup_token(Ok("   ".to_string())),
        Err("APPLE_MUSIC_DEVELOPER_TOKEN is set but blank")
    );
    assert_eq!(
        startup_token(Err(std::env::VarError::NotPresent)),
        Err("APPLE_MUSIC_DEVELOPER_TOKEN is not set")
    );
    // `OsString` only holds non-UTF-8 bytes on Unix; on other platforms the
    // `NotUnicode` variant cannot be constructed portably, so that one arm is
    // covered where it exists.
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let invalid = std::ffi::OsString::from_vec(vec![0xff, 0xfe]);
        assert_eq!(
            startup_token(Err(std::env::VarError::NotUnicode(invalid))),
            Err("APPLE_MUSIC_DEVELOPER_TOKEN is not valid UTF-8")
        );
    }
}

// `init_service` is the startup seam `main` calls before the UI boots.
// When `APPLE_MUSIC_DEVELOPER_TOKEN` is unset it skips sign-in and returns a
// service with no session; when it is set it spawns a sign-in thread on a
// clone that only touches the service's own session, never the shared playback
// state. Pin the contract both paths keep — the shared state is exactly as
// `init_service` found it and the returned service is the one the UI shares —
// so a regression that wired the startup hook into `AppState` (or reused
// `pause`'s clear-the-flag logic) can't ship silently. `blocking_lock` panics
// inside an async runtime, so this stays a plain test. CI leaves
// `APPLE_MUSIC_DEVELOPER_TOKEN` unset, so this takes the skip path.
#[test]
fn init_service_leaves_the_shared_state_untouched() {
    let state = Arc::new(Mutex::new(AppState::default()));
    {
        let mut state = state.blocking_lock();
        state.current_track = Some("song-1".to_string());
        state.is_playing = true;
        state.repeat = true;
        state.set_volume(0.7);
    }

    let service = init_service(state.clone(), None);

    assert!(service.session().is_none());
    let state = state.blocking_lock();
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(state.is_playing);
    assert!(state.repeat);
    assert_eq!(state.volume(), 0.7);
}

// The startup wiring: `init_service` builds one service, returns it for the
// UI, and runs the blocking sign-in on a clone. The session that sign-in
// stores must be visible through the returned service and must outlive the
// signing clone — the regression test for an earlier version that built a
// throwaway service inside the sign-in thread and dropped the session with it.
// `sign_in` is the synchronous body that thread runs; driving it directly
// exercises the startup path without the real browser flow. `init_service`
// itself reads the environment and spawns the thread, so only that thin wrapper
// is not driven here.
#[test]
fn startup_sign_in_stores_the_session_on_the_shared_service() {
    let service = test_service();
    let sign_in_service = service.clone();
    let expected = sign_in_session();

    sign_in_service.sign_in("dev-token", &|token| {
        assert_eq!(token, "dev-token");
        Ok(expected.clone())
    });
    drop(sign_in_service);

    assert_eq!(service.session(), Some(expected));
}

// `std::thread::spawn` panics when the OS refuses a thread, which would abort
// the player at startup. `start_sign_in` spawns through the fallible
// `Builder::spawn` and reports the refusal as a seam error instead; this test
// injects a failing spawner to drive that branch without exhausting the OS.
#[test]
fn start_sign_in_reports_a_thread_spawn_failure_as_a_seam_error() {
    let result = start_sign_in_with(test_service(), "dev-token".to_string(), |_work| {
        Err(AppleMusicError::new(
            "spawning the sign-in thread failed: no more threads",
        ))
    });

    assert_eq!(
        result.unwrap_err().to_string(),
        "spawning the sign-in thread failed: no more threads"
    );
}

// The startup sign-in runs on a named thread, so a panic in the blocking
// sign-in is attributed to `nostalgia-signin` rather than `<unnamed>`. Pin the
// name on the production spawner, since the failure test above injects its own
// spawner and would not notice a rename. The name is spelled out here rather
// than read back from `SIGN_IN_THREAD_NAME`, so a rename of the constant fails
// this test instead of renaming the thread and its assertion together.
#[test]
fn spawn_sign_in_thread_names_its_work_thread() {
    let (sender, receiver) = std::sync::mpsc::channel();
    spawn_sign_in_thread(Box::new(move || {
        let _ = sender.send(std::thread::current().name().map(str::to_string));
    }))
    .unwrap();

    assert_eq!(
        receiver.recv().unwrap().as_deref(),
        Some("nostalgia-signin")
    );
}

// The session seam: `new` starts with no session, `authenticate_with`
// stores the session its flow returns, and a failed flow propagates the
// error without disturbing an already-stored session. The real
// `authenticate` wraps the browser flow and is covered by a manual check;
// these tests drive `authenticate_with` with a stub.

#[test]
fn a_new_service_has_no_session() {
    assert!(test_service().session().is_none());
}

#[test]
fn authenticate_with_stores_the_session_its_flow_returns() {
    let service = test_service();
    let expected = sign_in_session();

    service
        .authenticate_with("dev-token", &|token| {
            assert_eq!(token, "dev-token");
            Ok(expected.clone())
        })
        .unwrap();

    // `session` clones, so a second read returns the same value rather
    // than consuming the stored session.
    assert_eq!(service.session(), Some(expected.clone()));
    assert_eq!(service.session(), Some(expected));
}

// The session-ready signal: `wait_for_session` resolves once a session is
// stored, whether it was already stored before the wait began or arrives
// afterwards. The UI's boot task awaits it to re-issue the artists fetch after
// the background sign-in completes, so both orderings must resolve.

#[tokio::test]
async fn wait_for_session_returns_immediately_when_a_session_is_already_stored() {
    let service = test_service();
    service
        .authenticate_with("dev-token", &|_| Ok(sign_in_session()))
        .unwrap();

    service.wait_for_session().await;
}

#[test]
fn wait_for_session_resolves_when_a_session_is_stored_after_the_wait_starts() {
    use std::task::{Context, Poll};

    let service = test_service();
    let future = service.wait_for_session();
    futures::pin_mut!(future);

    // Poll once so the waiter registers on the notification before the session
    // is stored; the already-stored fast path therefore cannot be what
    // resolves it.
    let waker = futures::task::noop_waker();
    let mut context = Context::from_waker(&waker);
    assert!(future.as_mut().poll(&mut context).is_pending());

    service
        .authenticate_with("dev-token", &|_| Ok(sign_in_session()))
        .unwrap();

    assert!(matches!(
        future.as_mut().poll(&mut context),
        Poll::Ready(())
    ));
}

#[test]
fn a_failed_authentication_leaves_a_stored_session_unchanged() {
    let service = test_service();
    let stored = sign_in_session();
    service
        .authenticate_with("dev-token", &|_| Ok(stored.clone()))
        .unwrap();

    let error = service
        .authenticate_with("dev-token", &|_| {
            Err(AppleMusicError::new("sign-in failed"))
        })
        .unwrap_err();

    assert_eq!(error.to_string(), "sign-in failed");
    assert_eq!(service.session(), Some(stored));
}

// `off_thread` reports the thread's result through a `oneshot` channel, so a
// panic in the blocking work drops the sender without a value. That must
// surface as an error rather than leaving the browse query's `await` pending
// forever; this test drives exactly that branch. The panic prints to stderr on
// the spawned thread, which is expected here.
#[tokio::test]
async fn off_thread_reports_an_error_when_its_work_thread_panics() {
    let result: Result<(), AppleMusicError> = off_thread(|| panic!("work thread panicked")).await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "the library request thread ended without a result"
    );
}

// The REST worker runs on a named thread (like the audio worker), so a panic
// there is attributed to `nostalgia-rest` rather than `<unnamed>`. Pin the
// name: a regression back to a bare `std::thread::spawn` would leave the panic
// test above green (it checks only the error) while the log lost the
// attribution. The name is spelled out here rather than read back from
// `REST_THREAD_NAME`, so a rename of the constant fails this test instead of
// renaming the thread and its assertion together.
#[tokio::test]
async fn off_thread_names_its_work_thread() {
    let name =
        off_thread(|| Ok::<_, AppleMusicError>(std::thread::current().name().map(str::to_string)))
            .await
            .unwrap();

    assert_eq!(name.as_deref(), Some("nostalgia-rest"));
}

// `std::thread::spawn` panics when the OS refuses a thread, which would abort
// the player on a browse query. `off_thread` spawns through the fallible
// `Builder::spawn` and reports the refusal through the seam instead; this test
// injects a failing spawner to drive that branch without exhausting the OS.
#[tokio::test]
async fn off_thread_reports_a_thread_spawn_failure_as_a_seam_error() {
    let result = off_thread_with(
        || Ok::<_, AppleMusicError>(()),
        |_work| {
            Err(AppleMusicError::new(
                "spawning the library request thread failed: no more threads",
            ))
        },
    )
    .await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "spawning the library request thread failed: no more threads"
    );
}

// The saved-token reseed: `seed_session` is the seam `init_service` calls with
// the settings file's saved user token, so a run whose previous sign-in
// succeeded boots into the signed-in library with no browser flow. The pair it
// stores must have the exact shape the browser flow stores — same struct, and
// the developer token trimmed at the boundary like `authorize` trims it — so
// the REST client cannot tell a seeded launch from a signed-in one.

#[test]
fn seed_session_stores_a_valid_pair() {
    let service = test_service();

    service.seed_session(" ey1.dev.ey ", "ey1.user.ey").unwrap();

    let stored = service.session().unwrap();
    assert_eq!(stored.developer_token, "ey1.dev.ey");
    assert_eq!(stored.user_token, "ey1.user.ey");
}

#[test]
fn seed_session_rejects_a_malformed_user_token_without_storing() {
    for user_token in ["", "   ", "not-a-jwt", "only.two"] {
        let service = test_service();

        let error = service.seed_session("ey1.dev.ey", user_token).unwrap_err();

        assert_eq!(service.session(), None, "{user_token:?} must store none");
        assert_eq!(service.session_user_token(), None);
        assert!(error.to_string().contains("the user token"));
    }
}

#[test]
fn a_failed_seed_leaves_a_stored_session_unchanged() {
    let service = test_service();
    service.seed_session("ey1.dev.ey", "ey1.user.ey").unwrap();

    let error = service.seed_session("ey1.dev.ey", "bad").unwrap_err();

    assert_eq!(service.session_user_token().as_deref(), Some("ey1.user.ey"));
    assert!(error.to_string().contains("the user token"));
}

#[test]
fn session_user_token_answers_the_stored_token() {
    let service = test_service();
    assert_eq!(service.session_user_token(), None);

    service.seed_session("ey1.dev.ey", "ey1.apple.ey").unwrap();

    assert_eq!(
        service.session_user_token().as_deref(),
        Some("ey1.apple.ey")
    );
}

#[test]
fn seed_session_wakes_a_wait_for_session_waiter() {
    use std::task::{Context, Poll};

    let service = test_service();
    let future = service.wait_for_session();
    futures::pin_mut!(future);
    let waker = futures::task::noop_waker();
    let mut context = Context::from_waker(&waker);
    assert!(future.as_mut().poll(&mut context).is_pending());

    service.seed_session("ey1.dev.ey", "ey1.user.ey").unwrap();

    assert!(matches!(
        future.as_mut().poll(&mut context),
        Poll::Ready(())
    ));
}

// The sign-in failure route: `sign_in` hands the failure line to the service's
// reporter instead of a direct `eprintln!`, which would leave the failure on
// the operator's stderr where no check could ever read it.
#[test]
fn a_sign_in_failure_is_reported_through_the_reporter_seam() {
    let (service, recorder) = test_service_with_reporter();

    service.sign_in("dev-token", &|_| {
        Err(AppleMusicError::new("the browser did not open"))
    });

    assert_eq!(
        recorder.recorded(),
        vec![(
            ReportChannel::Stderr,
            "Apple Music sign-in failed: the browser did not open".to_string(),
        )]
    );
}

// The thread-creation-refusal route: the same injectable spawner that drives
// the auxiliary-error branch also drives the report through the service's
// reporter, so the seam is pinned without making the OS refuse a thread.
#[test]
fn a_sign_in_thread_that_cannot_start_is_reported_through_the_reporter_seam() {
    let (service, recorder) = test_service_with_reporter();

    start_sign_in_with(service, "dev-token".to_string(), |_work| {
        Err(AppleMusicError::new(
            "spawning the sign-in thread failed: no more threads",
        ))
    })
    .unwrap_err();

    assert_eq!(
        recorder.recorded(),
        vec![(
            ReportChannel::Stderr,
            "Apple Music sign-in could not start: spawning the sign-in thread \
             failed: no more threads; the sample library stays in use"
                .to_string(),
        )]
    );
}

// The startup-skip route: the line naming why startup sign-in did not run is
// recorded through the reporter instead of printed to stdout alone, and the
// shared state is left exactly as the skip path expects. The environment read
// and the reporter are injected, so the process environment is untouched.
#[test]
fn a_skipped_startup_sign_in_names_its_reason_through_the_reporter_seam() {
    let recorder = Arc::new(RecordingReporter::new());
    let state = Arc::new(Mutex::new(AppState::default()));

    let service = init_service_with(
        state,
        Arc::new(crate::audio::SilentOutput) as Arc<dyn AudioOutput>,
        Arc::clone(&recorder) as Arc<dyn super::Reporter>,
        Err(std::env::VarError::NotPresent),
        None,
    );

    assert!(service.session().is_none());
    assert_eq!(
        recorder.recorded(),
        vec![(
            ReportChannel::Stdout,
            "Apple Music sign-in skipped: APPLE_MUSIC_DEVELOPER_TOKEN is not set".to_string(),
        )]
    );
}

// The saved-sign-in-reuse route: the reuse line is recorded through the
// reporter, and the session the reusable seed stored is present on the
// returned service.
#[test]
fn a_reused_saved_sign_in_is_reported_through_the_reporter_seam() {
    let recorder = Arc::new(RecordingReporter::new());
    let state = Arc::new(Mutex::new(AppState::default()));

    let service = init_service_with(
        state,
        Arc::new(crate::audio::SilentOutput) as Arc<dyn AudioOutput>,
        Arc::clone(&recorder) as Arc<dyn super::Reporter>,
        Ok("ey1.dev.ey".to_string()),
        Some("ey1.user.ey"),
    );

    assert!(service.session().is_some());
    assert_eq!(
        recorder.recorded(),
        vec![(
            ReportChannel::Stdout,
            "Apple Music sign-in skipped: the saved sign-in was reused".to_string(),
        )]
    );
}
