//! Background-thread seams for sign-in and blocking REST work.

use super::{AppleMusicError, AppleMusicService};
use crate::music_kit_auth::{MusicKitSession, authorize, open_in_browser};

/// The name the startup sign-in thread gets, so a panic in the blocking
/// sign-in is attributed to `nostalgia-signin` rather than `<unnamed>`.
pub(super) const SIGN_IN_THREAD_NAME: &str = "nostalgia-signin";

/// Starts the blocking sign-in for `developer_token` on `service` in the
/// background, logging a thread-creation failure rather than aborting the
/// player.
///
/// `std::thread::spawn` panics when the OS refuses a thread; a background
/// sign-in is not worth crashing the player for, so [`spawn_sign_in_thread`]
/// uses the fallible `Builder::spawn` and this reports the refusal and returns.
/// The player then keeps the sample library, as it does when no token is set.
pub(super) fn start_sign_in(service: AppleMusicService, developer_token: String) {
    if let Err(error) = start_sign_in_with(service, developer_token, spawn_sign_in_thread) {
        eprintln!("Apple Music sign-in could not start: {error}; the sample library stays in use");
    }
}

/// [`start_sign_in`] with an injectable spawner, so a test can drive the
/// thread-creation-failure branch without making the OS refuse a thread. The
/// work is boxed because the seam carries a `FnOnce` value rather than a
/// generic type the caller and the test stub would have to share.
pub(super) fn start_sign_in_with(
    service: AppleMusicService,
    developer_token: String,
    spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> Result<(), AppleMusicError>,
) -> Result<(), AppleMusicError> {
    spawn(Box::new(move || {
        service.sign_in(&developer_token, &browser_sign_in);
    }))
}

/// [`start_sign_in`]'s production spawner: the named [`SIGN_IN_THREAD_NAME`]
/// thread. Maps a thread-creation failure to an [`AppleMusicError`] naming the
/// cause; `std::thread::Builder::spawn` is the fallible form
/// `std::thread::spawn` wraps with a panic.
pub(super) fn spawn_sign_in_thread(work: Box<dyn FnOnce() + Send>) -> Result<(), AppleMusicError> {
    std::thread::Builder::new()
        .name(SIGN_IN_THREAD_NAME.to_string())
        .spawn(work)
        .map(|_handle| ())
        .map_err(|error| {
            AppleMusicError::new(format!("spawning the sign-in thread failed: {error}"))
        })
}

/// Classifies the `APPLE_MUSIC_DEVELOPER_TOKEN` read: the token to sign in
/// with, or the reason startup sign-in is skipped.
///
/// `std::env::var` collapses three distinct situations into values a single
/// blank check cannot tell apart: the variable is absent (`Err(NotPresent)`),
/// it is present but not valid UTF-8 (`Err(NotUnicode)`), or it is present and
/// blank (`Ok` of only whitespace). The startup line should name the case that
/// applies, so a user who exported a blank token is not told the variable is
/// unset. A present, non-blank value is returned untouched — [`authorize`]
/// trims it — so a token read from a file with a trailing newline still signs
/// in. Pure and injectable so the three skip cases are testable without
/// mutating the process environment, which is global.
pub(super) fn startup_token(
    token: Result<String, std::env::VarError>,
) -> Result<String, &'static str> {
    match token {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        Ok(_) => Err("APPLE_MUSIC_DEVELOPER_TOKEN is set but blank"),
        Err(std::env::VarError::NotPresent) => Err("APPLE_MUSIC_DEVELOPER_TOKEN is not set"),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err("APPLE_MUSIC_DEVELOPER_TOKEN is not valid UTF-8")
        }
    }
}

/// The production sign-in flow: opens the system browser and waits for the
/// loopback callback. Single-sourced so [`AppleMusicService::authenticate`]
/// and [`crate::apple_music::init_service`] run the same flow.
pub(super) fn browser_sign_in(developer_token: &str) -> Result<MusicKitSession, AppleMusicError> {
    authorize(developer_token, &open_in_browser)
}

/// The name [`off_thread`] gives the thread it runs blocking library work on.
///
/// Naming the thread (like the audio worker's `nostalgia-audio`) means a panic
/// there is attributed to `nostalgia-rest` in its message rather than to
/// `<unnamed>`, so a crashed request is traceable to this seam.
pub(super) const REST_THREAD_NAME: &str = "nostalgia-rest";

/// Runs `work` on a `std::thread` and awaits its result, so a blocking HTTP
/// call never stalls iced's executor.
///
/// The browse queries call this when a session is stored: the REST client is
/// synchronous, and iced's executor has no tokio runtime for
/// `tokio::task::spawn_blocking`, so the work moves to an ordinary thread and
/// reports back through a `tokio::sync::oneshot`. A sender dropped without a
/// value — the thread panicked — becomes an [`AppleMusicError`] rather than
/// hanging the await. The thread is named [`REST_THREAD_NAME`], and a
/// thread-creation failure is reported as an [`AppleMusicError`] naming the
/// cause instead of letting `std::thread::spawn`'s panic abort the player.
pub(super) async fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, AppleMusicError> + Send + 'static,
) -> Result<T, AppleMusicError> {
    off_thread_with(work, spawn_rest_thread).await
}

/// Spawns `work` on the named REST thread, mapping a thread-creation failure
/// to an [`AppleMusicError`] naming the cause. `std::thread::Builder::spawn` is
/// the fallible form `std::thread::spawn` wraps with a panic; the audio
/// worker's device thread uses the same form for the same reason.
fn spawn_rest_thread(work: Box<dyn FnOnce() + Send>) -> Result<(), AppleMusicError> {
    std::thread::Builder::new()
        .name(REST_THREAD_NAME.to_string())
        .spawn(work)
        .map(|_handle| ())
        .map_err(|error| {
            AppleMusicError::new(format!(
                "spawning the library request thread failed: {error}"
            ))
        })
}

/// [`off_thread`] with an injectable spawner, so a test can drive the
/// thread-creation-failure branch without making the OS refuse a thread. The
/// work is boxed because the seam carries a `FnOnce` value rather than a
/// generic type the caller and the test stub would have to share.
pub(super) async fn off_thread_with<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, AppleMusicError> + Send + 'static,
    spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> Result<(), AppleMusicError>,
) -> Result<T, AppleMusicError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    spawn(Box::new(move || {
        let _ = sender.send(work());
    }))?;
    receiver
        .await
        .map_err(|_| AppleMusicError::new("the library request thread ended without a result"))?
}
