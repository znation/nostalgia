//! The seam to the Apple Music API.
//!
//! This is the narrow integration point through which the rest of the app
//! reaches Apple Music: `AppleMusicService` answers a browse query from the
//! signed-in user's Apple Music library through the REST client when a session
//! is stored, and from the shared in-memory sample library
//! ([`crate::sample_library::sample_library`]) otherwise; it plays the
//! selected track's preview through an injected [`crate::audio::AudioOutput`],
//! committing the shared-state transition: `play_track` records the selected
//! track and marks it playing, `pause` pauses the audio and marks the track
//! paused (or stops it when no preview is loaded), `stop` stops the audio and
//! clears both the playing and paused flags, and `resume` resumes (or
//! restarts) the preview and marks it playing. The still-unimplemented stubs
//! (`next_track` and `previous_track`) stay so a real implementation has a
//! surface to land on. Inside, `browse` holds the queries (over `rest` or the
//! `sample` fallbacks), `playback` the transport transitions, output controls,
//! and play path, `validation` the id and preview-URL guards, `address` the
//! host-address classification behind them, and `threads` the
//! background sign-in and REST seams.
//!
//! The service also holds the authenticated `MusicKit` session
//! ([`crate::music_kit_auth::MusicKitSession`]) obtained by [`init_service`]
//! from `APPLE_MUSIC_DEVELOPER_TOKEN`; [`AppleMusicService::authenticate`]
//! stores it and [`AppleMusicService::session`] hands it to the browse
//! queries, which route through the REST client while it is stored, and report
//! the sign-in failures, startup skips, and transport dead ends through an
//! injectable [`Reporter`] so a test can record what production only prints.
//! [`init_service`] returns the one service the app shares, so the session
//! stored at startup is the session the UI later reads. Because the startup
//! sign-in runs on a background thread, the first browse query can land before
//! the session exists and answer from the sample library;
//! [`AppleMusicService::wait_for_session`] resolves when a session is stored so
//! the UI can re-issue that query against the real library.

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::audio;
use crate::music_kit_auth::MusicKitSession;
use crate::state::AppState;
use playback::TransportState;
use threads::{browser_sign_in, start_sign_in, startup_token};

pub use crate::music_error::{AppleMusicError, AppleMusicErrorKind};

use crate::music_kit_auth::{validate_developer_token, validate_user_token};

/// Where in production a service report goes. Reporting kept two destinations
/// before the seam existed — `println!` for operator-visible progress and
/// `eprintln!` for failures — and the default reporter preserves both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReportChannel {
    /// Operator progress: `println!`.
    Stdout,
    /// Operator failure: `eprintln!`.
    Stderr,
}

/// The seam a service report (a sign-in failure, a startup skip, or a
/// transport dead end) is handed to at the point it is raised, so a failure
/// that once left no trace any check could read (`no-observability`) is now
/// test-visible. The production [`StdReporter`] writes to the operator's
/// stdout or stderr exactly like the direct calls it replaced; a test stub
/// records the lines instead.
pub(crate) trait Reporter: Send + Sync {
    /// Hands `line` to `channel`.
    fn report(&self, channel: ReportChannel, line: String);
}

/// The production [`Reporter`]: stdout or stderr, exactly like the
/// `println!`/`eprintln!` calls this seam replaced.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StdReporter;

impl Reporter for StdReporter {
    fn report(&self, channel: ReportChannel, line: String) {
        match channel {
            ReportChannel::Stdout => println!("{line}"),
            ReportChannel::Stderr => eprintln!("{line}"),
        }
    }
}

/// The music-library service. It answers a browse query from the signed-in
/// user's Apple Music library through the REST client once
/// [`AppleMusicService::authenticate`] has stored a session, and from the
/// shared [`crate::sample_library::sample_library`] otherwise, so the UI and
/// its tests agree on the same stub data before sign-in. It plays audio
/// through the injected [`crate::audio::AudioOutput`], so the production
/// `rodio` backend can be swapped for a silent or recording fake.
#[derive(Clone)]
pub struct AppleMusicService {
    session: Arc<std::sync::Mutex<Option<MusicKitSession>>>,
    /// Fires when [`AppleMusicService::authenticate_with`] stores a session.
    /// [`AppleMusicService::wait_for_session`] awaits it, so a session that
    /// lands after the UI's first browse fetch still triggers a re-fetch.
    session_ready: Arc<tokio::sync::Notify>,
    rest: Arc<rest::RestLibrary>,
    state: Arc<Mutex<AppState>>,
    audio: Arc<dyn audio::AudioOutput>,
    /// The playback the audio backend currently holds, plus the most recent
    /// preview URL, so a transport transition can tell a paused player
    /// (`loaded`) from a discarded one and restart the latter. Guarded by its
    /// own async mutex: every transport method holds it across the read,
    /// the backend call, and the state write, so two concurrent transitions
    /// (a double-clicked Play/Pause) serialize instead of both reading the
    /// same flag.
    transport: Arc<tokio::sync::Mutex<TransportState>>,
    /// The reporter every failure-path line the service or its threads raise
    /// is handed to. The production [`StdReporter`] keeps the operator's
    /// stdout and stderr; a test stub records what it is given.
    reporter: Arc<dyn Reporter>,
}

/// Initializes the service, reseeding a session from the saved sign-in when
/// one is available and otherwise starting the `MusicKit` sign-in when a
/// developer token is available.
///
/// Builds the one [`AppleMusicService`] the app shares, reads
/// `APPLE_MUSIC_DEVELOPER_TOKEN`, and when it is set and non-blank:
///
/// - with a saved user token from a previous successful sign-in
///   (`saved_user_token`, read from the settings file by `main`), tries to reseed
///   a session through [`AppleMusicService::seed_session`] instead of launching
///   the browser flow. On success the sign-in is skipped — no thread, no loopback
///   server, no browser — the skip line says the saved sign-in was reused, and
///   because the seed lands before `init_ui` the boot fetch is the first
///   signed-in one with no sample-library flash. On `Err` (a blank, malformed,
///   or absent saved value) the flow falls back to the browser, leaving the
///   saved value alone for a later success to overwrite; and
/// - after the fallback, or when no saved token exists, starts a named
///   background thread that runs the blocking sign-in on a clone, so the UI
///   thread never blocks.
///
/// Returns the service so the caller hands that same instance to the UI: a
/// clone shares the session handle, so the session stored at startup is
/// visible through [`AppleMusicService::session`] on the UI's service. When
/// the variable is unset, blank, or not valid UTF-8, sign-in is skipped, the
/// saved token is left inert, and the sample library stays in use; the skip
/// line names which of those three cases applies (see [`startup_token`]). A
/// thread the OS refuses to start is logged rather than aborting the player,
/// which then keeps the sample library (see [`start_sign_in`]).
pub fn init_service(
    state: Arc<Mutex<AppState>>,
    saved_user_token: Option<&str>,
) -> AppleMusicService {
    let output = audio::audio_with_fallback(|| {
        audio::RodioOutput::new().map(|output| Arc::new(output) as Arc<dyn audio::AudioOutput>)
    });
    init_service_with(
        state,
        output,
        Arc::new(StdReporter),
        std::env::var("APPLE_MUSIC_DEVELOPER_TOKEN"),
        saved_user_token,
    )
}

/// [`init_service`] with the audio backend, the reporter, and the environment
/// read injected, so a test records the skip and reuse lines the startup wiring
/// raises without touching the process environment.
#[allow(dead_code)]
fn init_service_with(
    state: Arc<Mutex<AppState>>,
    audio: Arc<dyn audio::AudioOutput>,
    reporter: Arc<dyn Reporter>,
    developer_token: Result<String, std::env::VarError>,
    saved_user_token: Option<&str>,
) -> AppleMusicService {
    let service = AppleMusicService::with_reporter(
        state,
        Box::new(rest::UreqTransport::new()),
        audio,
        reporter,
    );
    let developer_token = match startup_token(developer_token) {
        Ok(token) => token,
        Err(reason) => {
            service.report(
                ReportChannel::Stdout,
                format!("Apple Music sign-in skipped: {reason}"),
            );
            return service;
        }
    };
    if let Some(saved) = saved_user_token
        && service.seed_session(&developer_token, saved).is_ok()
    {
        service.report(
            ReportChannel::Stdout,
            "Apple Music sign-in skipped: the saved sign-in was reused",
        );
        return service;
    }
    start_sign_in(service.clone(), developer_token);
    service
}

impl AppleMusicService {
    /// Wraps the shared [`AppState`] in a new service over the production
    /// [`rest::UreqTransport`] and [`audio::RodioOutput`]; the session starts
    /// unset until [`AppleMusicService::authenticate`] stores one. When no
    /// audio device opens, the service falls back to a silent
    /// [`audio::SilentOutput`] rather than failing to start.
    #[must_use]
    pub fn new(state: Arc<Mutex<AppState>>) -> Self {
        let output = audio::audio_with_fallback(|| {
            audio::RodioOutput::new().map(|output| Arc::new(output) as Arc<dyn audio::AudioOutput>)
        });
        Self::with_audio(state, Box::new(rest::UreqTransport::new()), output)
    }

    /// [`new`](Self::new) with an injectable [`rest::HttpTransport`], so a
    /// test drives the browse queries with a stub instead of the network. The
    /// audio backend is a silent [`audio::SilentOutput`], so a browse test
    /// never touches an audio device; use [`with_audio`](Self::with_audio) to
    /// inject a recording backend for playback tests.
    #[must_use]
    pub fn with_transport(
        state: Arc<Mutex<AppState>>,
        transport: Box<dyn rest::HttpTransport>,
    ) -> Self {
        Self::with_audio(state, transport, Arc::new(audio::SilentOutput))
    }

    /// [`with_transport`](Self::with_transport) with an injectable
    /// [`audio::AudioOutput`], so a playback test records what the service
    /// hands the backend.
    #[must_use]
    pub fn with_audio(
        state: Arc<Mutex<AppState>>,
        transport: Box<dyn rest::HttpTransport>,
        audio: Arc<dyn audio::AudioOutput>,
    ) -> Self {
        Self::with_reporter(state, transport, audio, Arc::new(StdReporter))
    }

    /// [`with_audio`](Self::with_audio) with an injectable [`Reporter`], so a
    /// test drives a failure path and records the line its default would only
    /// print.
    #[must_use]
    pub(crate) fn with_reporter(
        state: Arc<Mutex<AppState>>,
        transport: Box<dyn rest::HttpTransport>,
        audio: Arc<dyn audio::AudioOutput>,
        reporter: Arc<dyn Reporter>,
    ) -> Self {
        Self {
            session: Arc::new(std::sync::Mutex::new(None)),
            session_ready: Arc::new(tokio::sync::Notify::new()),
            rest: Arc::new(rest::RestLibrary::new(transport)),
            state,
            audio,
            reporter,
            transport: Arc::new(tokio::sync::Mutex::new(TransportState {
                loaded: false,
                last_url: None,
            })),
        }
    }

    /// Runs the blocking `MusicKit` sign-in flow and stores the resulting
    /// session.
    ///
    /// This is synchronous and blocks the calling thread — it opens the system
    /// browser and waits for the callback — so callers run it off the UI
    /// thread. [`init_service`] runs the same flow through [`Self::sign_in`],
    /// which also logs the outcome. On error the previously stored session, if
    /// any, is left in place.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the developer token is malformed,
    /// the browser cannot be opened, the callback is rejected, or no callback
    /// arrives before the flow times out.
    pub fn authenticate(&self, developer_token: &str) -> Result<(), AppleMusicError> {
        self.authenticate_with(developer_token, &browser_sign_in)
    }

    /// [`authenticate`](Self::authenticate) with an injectable sign-in flow,
    /// so tests can store a session without a browser. A flow that returns
    /// `Err` propagates it and leaves any previously stored session unchanged;
    /// a success is stored through [`Self::store_session`], so the two
    /// session-arrival paths — the browser flow and the saved-token reseed —
    /// share one insert site.
    pub(crate) fn authenticate_with(
        &self,
        developer_token: &str,
        authorize: &dyn Fn(&str) -> Result<MusicKitSession, AppleMusicError>,
    ) -> Result<(), AppleMusicError> {
        let session = authorize(developer_token)?;
        self.store_session(session);
        Ok(())
    }

    /// Stores `session` under the mutex and wakes the UI's boot task, which
    /// awaits `wait_for_session` to re-issue its artists fetch once the startup
    /// sign-in completes — or resolves immediately when a seeded launch stored
    /// its session before `init_ui` started. Notified only after the session
    /// is stored, so a woken waiter always observes it. Both session-arrival
    /// paths ([`authenticate_with`](Self::authenticate_with) and
    /// [`seed_session`](Self::seed_session)) store through this method, so the
    /// insert site and the wake-up stay single-sourced.
    pub(crate) fn store_session(&self, session: MusicKitSession) {
        let mut stored = self
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *stored = Some(session);
        self.session_ready.notify_one();
    }

    /// Wraps a developer token and a saved user token into a
    /// [`MusicKitSession`] — the same struct shape the browser flow returns —
    /// validates both, and stores the session when both are valid.
    ///
    /// `init_service` calls this with the settings file's saved user token at
    /// every launch that has `APPLE_MUSIC_DEVELOPER_TOKEN` set, so a run whose
    /// previous sign-in succeeded boots straight into the signed-in library
    /// with no browser detour. The developer token is trimmed here — the same
    /// trim [`crate::music_kit_auth::authorize`] applies at its boundary — so
    /// a value read from the environment with stray whitespace still seeds.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when either token fails its shape check
    /// (see [`validate_developer_token`] and [`validate_user_token`] for the
    /// defects each names). An `Err` stores and notifies nothing, leaving any
    /// previously stored session unchanged, so a malformed saved value falls
    /// back to the browser flow rather than poisoning the session seam.
    pub fn seed_session(
        &self,
        developer_token: &str,
        user_token: &str,
    ) -> Result<(), AppleMusicError> {
        let developer_token = developer_token.trim();
        validate_developer_token(developer_token)?;
        validate_user_token(user_token)?;
        self.store_session(MusicKitSession {
            developer_token: developer_token.to_string(),
            user_token: user_token.to_string(),
        });
        Ok(())
    }

    /// Runs `authorize` for `developer_token` on this service and logs whether
    /// the session was stored — through the [`Reporter`] seam, so a test can
    /// record the failure line a direct `eprintln!` would leave on stdout only.
    /// The success progress line stays a plain `println!`: it is not a failure
    /// the seam exists for.
    ///
    /// This is the blocking body [`init_service`] runs on its own thread. It is
    /// kept synchronous and separate from the thread spawn so a test can drive
    /// the startup path with a stub flow and observe the stored session.
    fn sign_in(
        &self,
        developer_token: &str,
        authorize: &dyn Fn(&str) -> Result<MusicKitSession, AppleMusicError>,
    ) {
        match self.authenticate_with(developer_token, authorize) {
            Ok(()) => println!("Apple Music session stored"),
            Err(error) => {
                self.report(
                    ReportChannel::Stderr,
                    format!("Apple Music sign-in failed: {error}"),
                );
            }
        }
    }

    /// Hands a service report (a failure, a skip, or a dead end) to the
    /// injected [`Reporter`] — the production default prints it, a test stub
    /// records it — instead of a direct `println!`/`eprintln!` only the
    /// operator would see.
    pub(crate) fn report(&self, channel: ReportChannel, line: impl Into<String>) {
        self.reporter.report(channel, line.into());
    }

    /// The stored `MusicKit` session, or `None` before a successful sign-in.
    ///
    /// Clones the session out from under the mutex so the caller owns it; the
    /// browse queries are the caller this seam is kept for.
    pub fn session(&self) -> Option<MusicKitSession> {
        self.session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// The saved sign-in's user token, when a session is stored.
    ///
    /// The player reads this at save time to persist the token in the settings
    /// file, so the next launch can reseed the session through
    /// [`seed_session`](Self::seed_session) without the browser flow. It clones
    /// the token out from under the mutex like [`session`](Self::session), and
    /// nothing else ever reads it — the token is a secret, so no log line or
    /// error message may carry it.
    pub fn session_user_token(&self) -> Option<String> {
        self.session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(|session| session.user_token.clone())
    }

    /// Resolves once a `MusicKit` session has been stored.
    ///
    /// [`init_service`] runs the blocking sign-in on a background thread and
    /// returns immediately, so the UI's first artists fetch can answer from
    /// the sample library before the session exists. The UI's boot task awaits
    /// this future and sends `Message::SessionReady` when it resolves, which refetches the
    /// artists list and any flat list showing, so a signed-in user's real
    /// library replaces the sample rows. Returns
    /// immediately when a session is already stored (the sign-in finished
    /// first); when sign-in is skipped or fails, no session is ever stored and
    /// this never resolves — there is no library to load from.
    pub async fn wait_for_session(&self) {
        if self.session().is_some() {
            return;
        }
        self.session_ready.notified().await;
    }
}

pub mod rest;

mod address;
mod browse;
mod playback;
mod sample;
mod threads;
mod validation;

pub(crate) use address::ip_is_internal;

#[cfg(test)]
mod tests;
