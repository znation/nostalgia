//! The seam to the Apple Music API.
//!
//! This is the narrow integration point a real Apple Music backend will
//! replace. `AppleMusicService` answers a browse query from the signed-in
//! user's Apple Music library through the REST client when a session is
//! stored, and from the shared in-memory sample library
//! ([`crate::sample_library::sample_library`]) otherwise; it plays the
//! selected track's preview through an injected [`crate::audio::AudioOutput`],
//! committing the shared-state transition (`play_track` records the selected
//! track and marks it playing, `pause` pauses the audio and clears the flag).
//! The still-unimplemented stubs (`next_track` and `previous_track`) stay so a
//! real implementation has a surface to land on.
//!
//! The service also holds the authenticated `MusicKit` session
//! ([`crate::music_kit_auth::MusicKitSession`]) obtained by [`init_service`]
//! from `APPLE_MUSIC_DEVELOPER_TOKEN`; [`AppleMusicService::authenticate`]
//! stores it and [`AppleMusicService::session`] hands it to the browse
//! queries, which route through the REST client while it is stored.
//! [`init_service`] returns the one service the app shares, so the session
//! stored at startup is the session the UI later reads. Because the startup
//! sign-in runs on a background thread, the first browse query can land before
//! the session exists and answer from the sample library;
//! [`AppleMusicService::wait_for_session`] resolves when a session is stored so
//! the UI can re-issue that query against the real library.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::audio;
use crate::library::{Album, Artist, Song};
use crate::music_kit_auth::{MusicKitSession, authorize, open_in_browser};
use crate::sample_library::sample_library;
use crate::state::AppState;

pub use crate::music_error::AppleMusicError;

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
}

/// Transport stubs kept as the seam a real Apple Music implementation will
/// fill: pause, next, and previous are not yet wired to the UI (the
/// transport.rs stepping helpers drive those buttons), so `dead_code` is
/// allowed on exactly this block — a *newly* dead private field or method
/// elsewhere still triggers the compiler's `dead_code` warning.
#[allow(dead_code)]
impl AppleMusicService {
    async fn pause(&self) -> Result<(), AppleMusicError> {
        // Pause the audio first: a backend that cannot accept the request
        // reports it before shared state claims playback stopped. Only the
        // shared-state transition is owned here; a real implementation would
        // also confirm the pause with the API.
        self.audio.pause()?;

        let mut state = self.state.lock().await;
        state.is_playing = false;

        println!("Paused playback");
        Ok(())
    }

    async fn next_track(&self) -> Result<(), AppleMusicError> {
        // In a real implementation, this would:
        // 1. Get current track position
        // 2. Play next track in library

        println!("Playing next track");
        Ok(())
    }

    async fn previous_track(&self) -> Result<(), AppleMusicError> {
        // In a real implementation, this would:
        // 1. Get current track position
        // 2. Play previous track in library

        println!("Playing previous track");
        Ok(())
    }
}

/// Initializes the service and starts the `MusicKit` sign-in when a developer
/// token is available.
///
/// Builds the one [`AppleMusicService`] the app shares, reads
/// `APPLE_MUSIC_DEVELOPER_TOKEN`, and — when it is set and non-blank — spawns a
/// `std::thread` that runs the blocking sign-in on a clone, so the UI thread
/// never blocks. Returns the service so the caller hands that same instance to
/// the UI: a clone shares the session handle, so the session stored at startup
/// is visible through [`AppleMusicService::session`] on the UI's service. When
/// the variable is unset, blank, or not valid UTF-8, sign-in is skipped and
/// the sample library stays in use; the skip line names which of those three
/// cases applies (see [`startup_token`]).
pub fn init_service(state: Arc<Mutex<AppState>>) -> AppleMusicService {
    let service = AppleMusicService::new(state);
    let developer_token = match startup_token(std::env::var("APPLE_MUSIC_DEVELOPER_TOKEN")) {
        Ok(token) => token,
        Err(reason) => {
            println!("Apple Music sign-in skipped: {reason}");
            return service;
        }
    };
    let sign_in_service = service.clone();
    std::thread::spawn(move || sign_in_service.sign_in(&developer_token, &browser_sign_in));
    service
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
fn startup_token(token: Result<String, std::env::VarError>) -> Result<String, &'static str> {
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
/// and [`init_service`] run the same flow.
fn browser_sign_in(developer_token: &str) -> Result<MusicKitSession, AppleMusicError> {
    authorize(developer_token, &open_in_browser)
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
        Self {
            session: Arc::new(std::sync::Mutex::new(None)),
            session_ready: Arc::new(tokio::sync::Notify::new()),
            rest: Arc::new(rest::RestLibrary::new(transport)),
            state,
            audio,
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
    /// so tests can store a session without a browser. Stores the session the
    /// flow returns under the mutex; a flow that returns `Err` propagates it
    /// and leaves any previously stored session unchanged.
    pub(crate) fn authenticate_with(
        &self,
        developer_token: &str,
        authorize: &dyn Fn(&str) -> Result<MusicKitSession, AppleMusicError>,
    ) -> Result<(), AppleMusicError> {
        let session = authorize(developer_token)?;
        {
            let mut stored = self
                .session
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *stored = Some(session);
        }
        // Wake the UI's boot task, which awaits `wait_for_session` to re-issue
        // its artists fetch once the startup sign-in completes. Notified only
        // after the session is stored, so a woken waiter always observes it.
        self.session_ready.notify_one();
        Ok(())
    }

    /// Runs `authorize` for `developer_token` on this service and logs whether
    /// the session was stored.
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
            Err(error) => eprintln!("Apple Music sign-in failed: {error}"),
        }
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

    /// Resolves once a `MusicKit` session has been stored.
    ///
    /// [`init_service`] runs the blocking sign-in on a background thread and
    /// returns immediately, so the UI's first artists fetch can answer from
    /// the sample library before the session exists. The UI's boot task awaits
    /// this future and re-issues `Message::LoadArtists` when it resolves, so a
    /// signed-in user's real library replaces the sample list. Returns
    /// immediately when a session is already stored (the sign-in finished
    /// first); when sign-in is skipped or fails, no session is ever stored and
    /// this never resolves — there is no library to load from.
    pub async fn wait_for_session(&self) {
        if self.session().is_some() {
            return;
        }
        self.session_ready.notified().await;
    }

    /// Plays the given track by id, recording it as the current track and
    /// marking it playing, then starts `preview_url` through the injected
    /// [`crate::audio::AudioOutput`]. A blank `track_id` — empty or only
    /// whitespace — or one carrying a terminal control character is rejected
    /// with an [`AppleMusicError`] and leaves shared state untouched: a blank
    /// id can never name a track, and a control-character id must not reach
    /// shared state. The playback log below escapes the id with `Debug` (see
    /// [`play_log_line`]), so even a non-control Unicode format character the
    /// guard does not reject cannot reach the terminal raw. A `None`
    /// `preview_url` is a track with no playable asset: the state transition
    /// is committed but no audio starts.
    ///
    /// `is_current` guards the commit: it is evaluated while the state lock is
    /// held, and when it returns `false` the track is left uncommitted — and
    /// no audio starts — because a newer play has superseded this one. A slow
    /// backend can complete two plays out of order, and without the guard the
    /// older reply would overwrite the newer track in shared state; the UI
    /// passes the guard from its playback-request counter, exactly as the
    /// browse path does.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when `track_id` is blank (empty or only
    /// whitespace), carries a control character, or the audio backend cannot
    /// start the preview.
    pub async fn play_track(
        &self,
        track_id: &str,
        preview_url: Option<&str>,
        is_current: impl FnOnce() -> bool,
    ) -> Result<(), AppleMusicError> {
        ensure_id_is_valid(track_id, IdKind::Track)?;

        let mut state = self.state.lock().await;
        if !is_current() {
            return Ok(());
        }
        state.current_track = Some(track_id.to_string());
        state.is_playing = true;
        drop(state);

        println!("{}", play_log_line(track_id));

        if let Some(url) = preview_url {
            self.audio.play(url)?;
        }
        Ok(())
    }

    /// All favorite artists: the signed-in user's library artists when a
    /// session is stored, and every sample-library artist otherwise.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the signed-in request fails; with
    /// no session the sample-library lookup never fails.
    pub async fn get_favorite_artists(&self) -> Result<Vec<Artist>, AppleMusicError> {
        self.browse(
            |rest, session| rest.get_favorite_artists(session),
            || sample_library().artists.clone(),
        )
        .await
    }

    /// Albums by the given artist: the signed-in user's library albums when a
    /// session is stored, and the sample library's matching albums otherwise
    /// (unknown artists yield an empty list).
    ///
    /// A blank `artist_id` — empty or only whitespace — or one carrying a
    /// terminal control character is rejected with an [`AppleMusicError`]
    /// instead of looked up: no artist has a blank id, so a blank id is a
    /// caller bug, and returning the empty list would report it as the
    /// ordinary "no albums" case. The same id guard
    /// [`AppleMusicService::play_track`] applies to its track id.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when `artist_id` is blank (empty or only
    /// whitespace), carries a control character, or the signed-in request
    /// fails.
    pub async fn get_albums_by_artist(
        &self,
        artist_id: &str,
    ) -> Result<Vec<Album>, AppleMusicError> {
        ensure_id_is_valid(artist_id, IdKind::Artist)?;
        let id = artist_id.to_string();
        self.browse(
            move |rest, session| rest.get_albums_by_artist(session, &id),
            || lookup(&sample_library().albums_by_artist, artist_id),
        )
        .await
    }

    /// Songs on the given album: the signed-in user's library songs when a
    /// session is stored, and the sample library's matching songs otherwise
    /// (unknown albums yield an empty list).
    ///
    /// A blank `album_id` — empty or only whitespace — or one carrying a
    /// terminal control character is rejected with an [`AppleMusicError`], the
    /// album-query twin of [`AppleMusicService::get_albums_by_artist`].
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when `album_id` is blank (empty or only
    /// whitespace), carries a control character, or the signed-in request
    /// fails.
    pub async fn get_songs_from_album(&self, album_id: &str) -> Result<Vec<Song>, AppleMusicError> {
        ensure_id_is_valid(album_id, IdKind::Album)?;
        let id = album_id.to_string();
        self.browse(
            move |rest, session| rest.get_songs_from_album(session, &id),
            || lookup(&sample_library().songs_by_album, album_id),
        )
        .await
    }

    /// Answers a browse query from the signed-in REST library when a session
    /// is stored, and from `sample` otherwise.
    ///
    /// `call` runs on a worker thread through `off_thread`, so the REST
    /// client's blocking HTTP request never stalls iced's executor; `sample`
    /// runs on the caller's thread and only when no session is stored, so the
    /// offline path never touches the REST client.
    async fn browse<T: Send + 'static>(
        &self,
        call: impl FnOnce(&rest::RestLibrary, &MusicKitSession) -> Result<Vec<T>, AppleMusicError>
        + Send
        + 'static,
        sample: impl FnOnce() -> Vec<T>,
    ) -> Result<Vec<T>, AppleMusicError> {
        match self.session() {
            Some(session) => {
                let rest = Arc::clone(&self.rest);
                off_thread(move || call(&rest, &session)).await
            }
            None => Ok(sample()),
        }
    }
}

/// The resource an id names, so the seam's validation error can label it.
///
/// [`ensure_id_is_valid`] serves the three id-taking queries; naming the id
/// kinds as a closed enum (rather than passing a `&str`) makes a misspelled
/// label impossible to compile and keeps the three kinds the only ones the
/// guard accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdKind {
    Track,
    Artist,
    Album,
}

impl std::fmt::Display for IdKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Track => "track",
            Self::Artist => "artist",
            Self::Album => "album",
        })
    }
}

/// Validates an id at the seam: `Ok` when `id` names a resource, `Err` naming
/// the offending value when it is blank or carries a terminal control
/// character.
///
/// Every id in the library names a real Apple Music resource, so a blank id
/// is a caller bug rather than the ordinary "unknown id" case the browse
/// queries answer with an empty list. `kind` names the id in the message
/// (see [`IdKind`]). A whitespace-only id is rejected too:
/// like an empty one it can name nothing, and letting it through would record
/// a blank-looking Now Playing entry or report the bug as an ordinary empty
/// result. A control character (`\u{1b}`, `\n`, and the rest) is rejected
/// because `play_track` writes the id to the terminal log; without this check
/// a hostile id from the library could inject escape sequences there. The
/// message quotes the value with `{id:?}`, so both a whitespace-only id and a
/// control-character id read as escaped text rather than as invisible bytes.
fn ensure_id_is_valid(id: &str, kind: IdKind) -> Result<(), AppleMusicError> {
    if id.trim().is_empty() {
        return Err(AppleMusicError::new(format!(
            "{kind} id must not be blank (got {id:?})"
        )));
    }
    if id.chars().any(char::is_control) {
        return Err(AppleMusicError::new(format!(
            "{kind} id must not contain control characters (got {id:?})"
        )));
    }
    Ok(())
}

/// The playback log line for `track_id`, escaped so a hostile library id
/// cannot inject terminal output.
///
/// `play_track` writes this line to stdout. The id is formatted with `Debug`,
/// not `Display`: `Display` passes every byte through raw, so an id carrying a
/// control character (`\u{1b}`) or a Unicode format character — the
/// right-to-left override `\u{202e}`, say, which `char::is_control` does not
/// classify as a control — would reach the terminal verbatim, while `Debug`
/// renders both as `\u{...}` escapes. This matches the failure report, which
/// already quotes the id with `{track_id:?}`. Kept pure so the escaping
/// contract is testable without capturing stdout.
fn play_log_line(track_id: &str) -> String {
    format!("Playing track: {track_id:?}")
}

/// The group stored under `id` in `index`, or an empty list when the id is
/// unknown. The read-side twin of `index_by`: the browse queries both look
/// up their matches in the prebuilt tables, so this get-then-clone-then-
/// default chain lives here once instead of in each query method.
fn lookup<T: Clone>(index: &HashMap<String, Vec<T>>, id: &str) -> Vec<T> {
    index.get(id).cloned().unwrap_or_default()
}

/// Runs `work` on a `std::thread` and awaits its result, so a blocking HTTP
/// call never stalls iced's executor.
///
/// The browse queries call this when a session is stored: the REST client is
/// synchronous, and iced's executor has no tokio runtime for
/// `tokio::task::spawn_blocking`, so the work moves to an ordinary thread and
/// reports back through a `tokio::sync::oneshot`. A sender dropped without a
/// value — the thread panicked — becomes an [`AppleMusicError`] rather than
/// hanging the await.
async fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, AppleMusicError> + Send + 'static,
) -> Result<T, AppleMusicError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let _ = sender.send(work());
    });
    receiver
        .await
        .map_err(|_| AppleMusicError::new("the library request thread ended without a result"))?
}

pub mod rest;

#[cfg(test)]
mod tests;
