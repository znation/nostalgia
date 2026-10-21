//! The iced application: the `WinampPlayer` app struct, its `Message` event
//! type, and the `update`/`view` loop `init_ui` hands to iced. Widget
//! construction lives in the `views` submodule (browse lists, Now Playing bar,
//! transport controls, equalizer panel), the Previous/Next stepping arithmetic
//! in `transport`, the library-fetch/error-reporting adapter in `loading`, the
//! per-level browse state machine in `browse`, and the classic key bindings in
//! `shortcuts`; this module wires those to the shared `AppState` and the
//! `AppleMusicService` seam. Its unit tests live in the `tests` submodule.

use iced::{Element, Task, keyboard, widget::Column};
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::sync::Mutex;

mod bevel;
mod browse;
mod loading;
mod shortcuts;
mod style;
mod theme;
mod transport;
mod views;

use browse::BrowseList;
use shortcuts::message_for;

use crate::{
    apple_music::AppleMusicService,
    audio::EqSnapshot,
    equalizer::Preset,
    library::{Album, Artist, Playlist, Song},
    state::AppState,
};
use loading::{
    PLAY_TIMEOUT, RequestGeneration, fetch_level, play_failure_report, play_into, transport_into,
};

/// How often the Now Playing title window advances one character while an
/// overlong line scrolls. The view ignores the offset whenever the line fits,
/// so the timer runs unconditionally.
const MARQUEE_TICK: Duration = Duration::from_millis(250);

/// A subscription that emits [`Message::MarqueeTick`] once per
/// [`MARQUEE_TICK`]. iced's timed subscription is gated behind `smol`/`tokio`
/// backend features the app does not enable (its default thread-pool backend
/// has no `time::every`), so this feeds the ticks from one std thread through
/// the futures channel iced re-exports. Dropping the subscription drops the
/// receiver, and the thread exits on its next send.
fn marquee_ticks() -> iced::Subscription<Message> {
    iced::Subscription::run(marquee_tick_stream)
}

/// The tick stream behind [`marquee_ticks`], split out so a test can poll it.
/// The one-slot channel deliberately drops a tick when the UI has not polled
/// the previous one yet; that keeps memory bounded without stopping the timer.
/// Only a disconnected receiver — the subscription ended — ends the thread.
fn marquee_tick_stream() -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::{StreamExt, channel::mpsc};

    let (mut sender, receiver) = mpsc::channel(1);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(MARQUEE_TICK);
            if let Err(error) = sender.try_send(())
                && error.is_disconnected()
            {
                break;
            }
        }
    });
    receiver.map(|()| Message::MarqueeTick)
}

/// Runs the UI, blocking until the window is closed.
///
/// `service` is the [`AppleMusicService`] `main` built and ran the startup
/// sign-in on; the UI stores a clone, so the session that sign-in stores is the
/// session the UI later reads. `main` must call this from a plain (non-async)
/// context: iced drives its event loop synchronously on the calling thread, and
/// `update`/`view` use `blocking_lock` on the shared state, which panics inside
/// a runtime.
///
/// # Errors
///
/// Returns the error iced reports when the application fails to start, such
/// as a window that cannot be created.
pub fn init_ui(state: Arc<Mutex<AppState>>, service: AppleMusicService) -> iced::Result {
    iced::application(move || boot(state.clone(), service.clone()), update, view)
        .title("nostalgia")
        .theme(|_: &WinampPlayer| theme::winamp_theme())
        .decorations(false)
        .resizable(false)
        .subscription(|_player| {
            iced::Subscription::batch([keyboard::listen().filter_map(message_for), marquee_ticks()])
        })
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    PlayPause,
    // The explicit Play and Pause key bindings (X and C). Unlike `PlayPause`
    // they are idempotent: pressing X always plays, C always pauses.
    Play,
    Pause,
    Stop,
    ToggleRepeat,
    // The Shuffle toggle: with it on, a forward step picks a random
    // non-current song from the loaded song list instead of the next in order.
    ToggleShuffle,
    VolumeChange(f32),
    // The balance slider's change message, clamped by `AppState::set_balance`.
    BalanceChange(f32),
    // Arrow-key volume nudges: one `views::VOLUME_STEP` up or down, clamped
    // by `AppState::nudge_volume`.
    VolumeUp,
    VolumeDown,
    // Arrow-key seek nudges: one `views::SEEK_STEP_SECS` back or forward.
    SeekBackward,
    SeekForward,
    // The marquee timer's tick: advance the Now Playing title window by one
    // character. The view ignores the offset while the line fits, so the timer
    // needs no state to start or stop.
    MarqueeTick,
    ToggleEqualizer,
    EqPreampChange(f32),
    EqBandChange(usize, f32),
    EqPresetSelected(Preset),
    NextTrack,
    PreviousTrack,
    // The selection messages carry a row's index into the list that rendered
    // it, plus that list's epoch. Building a row therefore never clones the
    // entry's `String` id (the view rebuilds on every message, so that clone
    // ran per row per refresh); `update` resolves the index back to an id only
    // when the row is actually pressed. The epoch guards the case where the
    // list is replaced between rendering and the press: a stale index would
    // otherwise resolve against the new list and select the wrong entry.
    TrackSelected { epoch: u64, index: usize },
    ArtistSelected { epoch: u64, index: usize },
    AlbumSelected { epoch: u64, index: usize },
    PlaylistSelected { epoch: u64, index: usize },
    Back,
    // The browse view's Top button: snap the current list back to its first
    // row. The arm schedules an iced scroll operation on the browse list's
    // stable widget id and changes no state.
    ScrollBrowseToTop,
    // The Artists view's All Songs button: fetch the whole library flat and
    // show it under the All Songs view.
    ShowAllSongs,
    // The Artists view's All Albums button: fetch every library album flat and
    // show it under the All Albums view.
    ShowAllAlbums,
    // The Artists view's Playlists button: fetch the library's playlists and
    // show them under the Playlists view.
    ShowPlaylists,
    // The library search box: `SearchChanged` carries each keystroke into
    // `search_query`; `SearchSubmitted` takes that stored text when Enter is
    // pressed and replaces the songs list with the search results. The submit
    // carries no payload so the per-view widget build does not clone the
    // query String just to have a message ready for a press.
    SearchChanged(String),
    SearchSubmitted,
    LoadArtists,
    ArtistsLoaded(Vec<Artist>),
    AlbumsLoaded(Vec<Album>),
    SongsLoaded(Vec<Song>),
    AllSongsLoaded(Vec<Song>),
    AllAlbumsLoaded(Vec<Album>),
    PlaylistsLoaded(Vec<Playlist>),
    // A browse fetch that returned an error. The payload is the formatted
    // report (see `loading::fetch_failure_report`): it names the failed fetch
    // and carries the backend's cause, so the browse panel can show why the
    // list is empty instead of misreading the failure as an empty library.
    ArtistsLoadFailed(String),
    AlbumsLoadFailed(String),
    SongsLoadFailed(String),
    AllSongsLoadFailed(String),
    AllAlbumsLoadFailed(String),
    PlaylistsLoadFailed(String),
    // A browse reply stamped with the generation that produced it. The
    // mapper's `RequestGeneration::is_current` check runs on iced's executor
    // thread; `update` stores the reply later on the UI thread, and a
    // navigation in that window bumps the list's generation. The arm re-checks
    // the stamp against the target list before dispatching, so a reply a newer
    // request superseded is dropped instead of overwriting the newer list (or
    // its error). Only `loading::fetch_into` constructs this, and only with
    // the `*Loaded`/`*LoadFailed` message above.
    BrowseReply { issued: u64, reply: Box<Message> },
    // A browse reply that a newer request for the same list has superseded.
    // `fetch_into` emits this instead of the `*Loaded` message so the stale
    // reply never reaches `BrowseList::store`; the arm below is a no-op.
    Ignored,
    // The play's completion carries the generation its selection issued, so
    // the arm can tell whether a newer selection has superseded it. Only the
    // latest completion may prune the title index; a superseded one must not
    // drop a still-pending selection's title.
    TrackPlayed { generation: u64 },
    // A Play/Pause/Stop transition finished (or failed; the failure was
    // logged). It carries no payload — the service already wrote shared
    // state — and exists so iced re-renders the Now Playing bar after the
    // asynchronous transition, exactly as `TrackPlayed` re-renders after a
    // play. The arm is a no-op.
    TransportSettled,
    // The app window's id, resolved once at boot by `iced::window::latest()`.
    // The custom title bar's window actions — drag, minimize, close, shade,
    // and always-on-top — need it; until the query resolves (or if it fails)
    // their window calls are no-ops.
    WindowIdResolved(Option<iced::window::Id>),
    WindowDragged,
    MinimizeWindow,
    CloseWindow,
    // Double-clicking the title bar's drag region rolls the window up into
    // shade mode, or restores it. The first shade measures the window so the
    // pre-shade inner size can be restored; later shades reuse it.
    ToggleWindowShade,
    // The clutter bar's always-on-top toggle. The arm flips the flag and
    // schedules `iced::window::set_level` for the resolved window, which is a
    // no-op until the window id resolves.
    ToggleAlwaysOnTop,
    // The window's inner size, captured just before the first shade. The
    // arm stores it and resizes the window down to the title-bar strip,
    // unless a newer toggle already left shade mode or captured the size.
    WindowShadeMeasured(iced::Size),
}

struct WinampPlayer {
    state: Arc<Mutex<AppState>>,
    apple_music_service: AppleMusicService,
    /// The app window's id, resolved at boot by `iced::window::latest()`. The
    /// custom title bar's window messages — drag, minimize, close, shade, and
    /// always-on-top — act on this window; a `None` (before the query
    /// resolves, or if it fails) makes their window calls no-ops.
    window_id: Option<iced::window::Id>,
    /// Whether the window is rolled up into shade mode: `view` then renders
    /// only the title bar and the window is resized to
    /// [`views::TITLE_BAR_HEIGHT`].
    shaded: bool,
    /// Whether the window is pinned above other windows. The title bar's
    /// clutter toggle flips it, and `update` schedules
    /// `iced::window::set_level` to match; `view` renders the toggle sunken
    /// while it holds. It lives here rather than in `AppState` so the shaded
    /// frame can read it without the state lock.
    always_on_top: bool,
    /// The window's inner size captured just before the first shade, so
    /// unshading can restore it. `None` until the measurement lands.
    unshaded_size: Option<iced::Size>,
    current_view: CurrentView,
    /// The browse levels. Each `BrowseList` owns its rows, epoch, loading
    /// flag, error report, and request counter (see `browse`).
    artists: BrowseList<Artist>,
    albums: BrowseList<Album>,
    songs: BrowseList<Song>,
    /// The flat whole-library list behind the Artists view's All Songs button.
    all_songs: BrowseList<Song>,
    /// The flat whole-library album list behind the Artists view's All Albums
    /// button.
    all_albums: BrowseList<Album>,
    /// The library's playlists behind the Artists view's Playlists button.
    playlists: BrowseList<Playlist>,
    /// Which list the Songs view was opened from. `Album` returns to the
    /// per-artist Albums view on Back; `AllAlbums` returns to the flat All
    /// Albums view; `Playlist` returns to the Playlists view. Set by the
    /// selection arm from the view that rendered the pressed row, so Back
    /// always steps to the level the row came from.
    songs_origin: SongsOrigin,
    /// The library search box's text, as typed. `Message::SearchChanged`
    /// stores each keystroke here and the search box renders it back; a submit
    /// trims it into the fetch's query.
    search_query: String,
    /// Whether the Songs view is showing the results of a library search
    /// rather than a browsed album. `Message::SearchSubmitted` sets it, and a
    /// browse step or `Back` clears it, so Back leaves the search for the
    /// artists list instead of the album the search was issued from.
    search_active: bool,
    /// Each played song's id mapped to its [`views::KnownTrack`] (title,
    /// artist, and formatted length), so the Now Playing bar can still name and
    /// time the playing track after the user browses to a different list
    /// (whose load replaces the visible song buffer). The `TrackSelected` arm
    /// records a track's title, artist, and formatted length once, when it is
    /// played, and `now_playing_label` / `now_playing_artist` /
    /// `now_playing_time` resolve the per-frame bar with a single get instead
    /// of scanning a growing list on every frame. Recording on play — rather
    /// than folding every song of every loaded list into the map — keeps the
    /// map proportional to songs actually played and takes the per-list fold
    /// off the browse path. Only the current track's entry is ever read, so
    /// the latest play's completion prunes the map back to that entry (or
    /// clears it when nothing committed): without the prune the map would grow
    /// by one entry for every distinct track ever played. A completion whose
    /// play a newer selection superseded leaves the map alone — the newer
    /// selection's entry is still pending, and pruning to the older committed
    /// track would drop it.
    known_tracks: HashMap<String, views::KnownTrack>,
    /// The tracks actually played, in play order, backing Previous while
    /// Shuffle is on. A committed `TrackPlayed` records the track; a shuffled
    /// Previous retraces it, and a list that no longer shows the previous
    /// entry falls back to sequential stepping.
    play_history: transport::PlayHistory,
    /// How far an overlong Now Playing title has scrolled: the character
    /// offset `views::view_now_playing` windows the combined artist/title by.
    /// `Message::MarqueeTick` advances it, and a `TrackSelected` resets it so
    /// a new song starts at the beginning of its title. The view ignores it
    /// whenever the line fits the LCD's character budget.
    marquee_offset: usize,
    /// The playback-request counter. Each `TrackSelected` bumps it and
    /// carries its value into the async play as a guard: `play_track`
    /// evaluates the guard while holding the state lock and commits the
    /// track only if no newer play has been issued. Without it, two rapid
    /// selections whose plays complete out of order let the older play
    /// overwrite the newer one, so the Now Playing bar names the wrong
    /// track.
    plays_generation: Arc<AtomicU64>,
}

/// Which browse screen is showing. Payload-free: the albums and songs the
/// view renders come from the loaded `albums`/`songs` lists, so carrying
/// the selected ids here (as earlier versions did) only duplicated
/// state that nothing read.
#[derive(Debug, PartialEq, Eq)]
enum CurrentView {
    Artists,
    Albums,
    Songs,
    AllSongs,
    AllAlbums,
    Playlists,
}

/// Which browse list a Songs view was opened from, so `Back` can step to it.
#[derive(Debug, PartialEq, Eq)]
enum SongsOrigin {
    /// The per-artist Albums view: Back returns to `CurrentView::Albums`.
    Album,
    /// The flat whole-library All Albums view: Back returns to
    /// `CurrentView::AllAlbums`.
    AllAlbums,
    /// The Playlists view: Back returns to `CurrentView::Playlists`.
    Playlist,
}

impl WinampPlayer {
    /// The initial player over the given shared state and Apple Music service:
    /// nothing selected, no lists loaded. Both `boot` and the tests start a
    /// player this way, so the starting shape lives here instead of being
    /// repeated at each site — a new field has only one spot to get its startup
    /// value. The service is passed in rather than built here so the UI shares
    /// the instance `main` ran the startup sign-in on.
    fn new(state: Arc<Mutex<AppState>>, service: AppleMusicService) -> Self {
        Self {
            state,
            apple_music_service: service,
            window_id: None,
            shaded: false,
            always_on_top: false,
            unshaded_size: None,
            current_view: CurrentView::Artists,
            artists: BrowseList::new(true),
            albums: BrowseList::new(false),
            songs: BrowseList::new(false),
            all_songs: BrowseList::new(false),
            all_albums: BrowseList::new(false),
            playlists: BrowseList::new(false),
            songs_origin: SongsOrigin::Album,
            search_query: String::new(),
            search_active: false,
            known_tracks: HashMap::new(),
            play_history: transport::PlayHistory::default(),
            marquee_offset: 0,
            plays_generation: Arc::new(AtomicU64::new(0)),
        }
    }

    /// The Now Playing bar's label for `current_track`, resolved against the
    /// accumulated [`Self::known_tracks`] index rather than the currently
    /// loaded song list: `view` renders the bar from this, so the bar keeps
    /// naming a playing track even after a browse replaced the visible song
    /// buffer. The browse-away regression test asserts this same path.
    fn now_playing_label<'a>(&'a self, current_track: Option<&str>) -> Cow<'a, str> {
        views::now_playing_label(&self.known_tracks, current_track)
    }

    /// The Now Playing bar's artist for `current_track`, resolved against the
    /// accumulated [`Self::known_tracks`] index like
    /// [`Self::now_playing_label`]: the artist is borrowed from the index, or
    /// the `""` literal when the track is unknown or none is current, so
    /// `view` renders the artist before the title only when one is known.
    fn now_playing_artist<'a>(&'a self, current_track: Option<&str>) -> Cow<'a, str> {
        views::now_playing_artist(&self.known_tracks, current_track)
    }

    /// The Now Playing bar's length readout for `current_track`, borrowed from
    /// the label cached in [`Self::known_tracks`]; a track the index does not
    /// know reads the static `--:--` placeholder. Like the label and artist,
    /// the known case borrows from the index, so `view` renders the bar
    /// without formatting or allocating a `String` per frame.
    fn now_playing_time<'a>(&'a self, current_track: Option<&str>) -> Cow<'a, str> {
        views::now_playing_time(&self.known_tracks, current_track)
    }
}

fn boot(state: Arc<Mutex<AppState>>, service: AppleMusicService) -> (WinampPlayer, Task<Message>) {
    // `iced` runs the boot task only after the window opens, so `latest()`
    // resolves to the app window. The three run together: the artists fetch
    // does not wait on the window id, and the sign-in wait does not block the
    // first fetch.
    let sign_in_service = service.clone();
    (
        WinampPlayer::new(state, service),
        Task::batch([
            Task::done(Message::LoadArtists),
            // The startup sign-in runs on a background thread, so the fetch
            // above can land before the session exists and answer from the
            // sample library. Re-issue it when a session arrives, replacing
            // the sample artists with the signed-in library.
            Task::perform(
                async move { sign_in_service.wait_for_session().await },
                |()| Message::LoadArtists,
            ),
            iced::window::latest().map(Message::WindowIdResolved),
        ]),
    )
}

/// The task the Next/Previous buttons schedule: step the current track
/// through the list the current view shows — `all_songs` for the flat All
/// Songs view, the browsed `songs` otherwise — in the given direction — with
/// Shuffle on a forward step picks a random non-current song via
/// `transport::shuffled_track_id`, and a backward step first retraces the
/// recorded play history via `PlayHistory::previous`, falling back to
/// `transport::previous_track_id` when no recorded track is still visible;
/// otherwise `forward` selects the
/// `transport::next_track_id`/`previous_track_id` stepping function — and
/// schedule the landed song as `TrackSelected`, or no task when there is
/// nothing to step through. Both transport update arms used to repeat this
/// lock-then-dispatch block; `forward` selects the direction so the stepping
/// arithmetic stays in `transport` and the wiring lives here once. The shared
/// Shuffle and Repeat flags are read under the same lock, so the
/// random-vs-sequential and wrap-vs-stop-at-the-edge decisions (see
/// `transport::shuffled_track_id` and `transport::next_track_id`) stay in
/// `transport` too.
fn step_track(player: &mut WinampPlayer, forward: bool) -> Task<Message> {
    // The stepped song is computed under the state lock, borrowing the
    // current track directly. The earlier version cloned the `current_track`
    // `String` only to borrow it via `as_deref` — the same clone-to-borrow
    // the per-frame now-playing path used to do — so the lock now covers the
    // pure stepping scan (fast, and the transport functions never lock
    // anything themselves).
    let state = player.state.blocking_lock();
    // Step the list the current view draws from, so the scheduled
    // `TrackSelected` index and epoch match the list the arm resolves
    // against: the flat All Songs view's `all_songs`, otherwise `songs`.
    let (items, epoch) = if player.current_view == CurrentView::AllSongs {
        (&player.all_songs.items, player.all_songs.epoch)
    } else {
        (&player.songs.items, player.songs.epoch)
    };
    let stepped = if forward && state.shuffle {
        transport::shuffled_track_id(
            items,
            state.current_track.as_deref(),
            transport::shuffle_roll(),
        )
    } else if forward {
        transport::next_track_id(items, state.current_track.as_deref(), state.repeat)
    } else if state.shuffle {
        player
            .play_history
            .previous(items, state.current_track.as_deref())
            .or_else(|| {
                transport::previous_track_id(items, state.current_track.as_deref(), state.repeat)
            })
    } else {
        transport::previous_track_id(items, state.current_track.as_deref(), state.repeat)
    };
    match stepped {
        // `transport` answers with the stepped song's id, but the selection
        // message carries the song's index and the epoch of the list that
        // rendered the row (see `TrackSelected`), so resolve the id to its
        // position in this same buffer. This scan runs once per button press,
        // not per view refresh, and the stepped id always names a song in
        // this buffer.
        Some(track_id) => match items.iter().position(|song| song.id == track_id) {
            Some(index) => Task::done(Message::TrackSelected { epoch, index }),
            None => Task::none(),
        },
        None => Task::none(),
    }
}

/// Locks the shared playback state, applies `mutation` to it, and returns no
/// task. The synchronous arms — `ToggleRepeat` and `ToggleShuffle` — repeat
/// the same shared-state update — `blocking_lock`, one mutation, then
/// `Task::none()` — so the lock-and-noop shape lives here once and each arm
/// only names its mutation. The transport arms and asynchronous work (fetches,
/// plays) go through the service instead (see [`transport_into`] and
/// [`fetch_level`]), and the volume, balance, and equalizer arms use
/// [`mutate_volume`], [`mutate_balance`], and [`mutate_equalizer`] because
/// they also forward their value to the backend.
fn mutate_state(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    let mut state = player.state.blocking_lock();
    mutation(&mut state);
    Task::none()
}

/// Locks the shared playback state, applies `mutation` to it, forwards the
/// resulting clamped volume to the audio backend, and returns no task. The
/// volume arms — `VolumeChange`, `VolumeUp`, and `VolumeDown` — all repeat the
/// same two-step update: `blocking_lock`, one mutation, read back the clamped
/// `state.volume()`, then hand it to
/// [`AppleMusicService::set_output_volume`]. The state write and the backend
/// forward both happen under the one lock, and the forward is a non-blocking
/// channel send, so a slider drag stays responsive and its messages stay
/// ordered. A backend error is logged; the state write has already committed.
fn mutate_volume(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    let mut state = player.state.blocking_lock();
    mutation(&mut state);
    let volume = state.volume();
    if let Err(error) = player.apple_music_service.set_output_volume(volume) {
        eprintln!("forwarding volume to the audio backend failed: {error}");
    }
    Task::none()
}

/// Locks the shared playback state, applies `mutation` to it, forwards the
/// resulting clamped balance to the audio backend, and returns no task. The
/// `BalanceChange` arm uses this so a slider drag reaches the playing preview
/// without restarting it; the state write and the backend forward both happen
/// under the one lock, exactly as [`mutate_volume`] does. A backend error is
/// logged; the state write has already committed.
fn mutate_balance(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    let mut state = player.state.blocking_lock();
    mutation(&mut state);
    let balance = state.balance();
    if let Err(error) = player.apple_music_service.set_output_balance(balance) {
        eprintln!("forwarding balance to the audio backend failed: {error}");
    }
    Task::none()
}

/// Forwards a relative seek to the audio backend and returns no task. The
/// left and right arrow arms use this so a key press moves the playing
/// preview without touching shared state: the backend owns the position, and
/// [`AppleMusicService::seek_output`] is a non-blocking channel send. A
/// backend error is logged.
fn seek_preview(player: &WinampPlayer, offset_secs: i64) -> Task<Message> {
    if let Err(error) = player.apple_music_service.seek_output(offset_secs) {
        eprintln!("seeking the preview failed: {error}");
    }
    Task::none()
}

/// Locks the shared playback state, applies `mutation` to it, forwards the
/// resulting clamped curve to the audio backend, and returns no task. The
/// equalizer arms — `ToggleEqualizer`, `EqPreampChange`, `EqBandChange`, and
/// `EqPresetSelected` — use this so a toggle, slider drag, or preset pick
/// reaches the playing preview without restarting it; the running
/// `EqSource` re-reads its shared curve between samples. The
/// state write and the backend forward both happen under the one lock, exactly
/// as [`mutate_volume`] and [`mutate_balance`] do. A backend error is logged;
/// the state write has already committed.
fn mutate_equalizer(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    let mut state = player.state.blocking_lock();
    mutation(&mut state);
    let equalizer = EqSnapshot {
        enabled: state.eq_enabled,
        preamp_db: state.eq_preamp(),
        bands_db: state.eq_bands(),
    };
    if let Err(error) = player.apple_music_service.set_output_equalizer(equalizer) {
        eprintln!("forwarding equalizer to the audio backend failed: {error}");
    }
    Task::none()
}

/// Runs `action` with the resolved window id, or returns no task when it has
/// not resolved yet. The title bar's window actions — drag, minimize, close,
/// the shade size query and resize, and the always-on-top level — all guard on
/// `WinampPlayer::window_id` the same way: a `None` (before boot's
/// `iced::window::latest()` query resolves, or if it fails) makes the action a
/// no-op. The guard lives here once, so each arm names only the
/// `iced::window` call it schedules.
fn with_window_id(
    player: &WinampPlayer,
    action: impl FnOnce(iced::window::Id) -> Task<Message>,
) -> Task<Message> {
    match player.window_id {
        Some(id) => action(id),
        None => Task::none(),
    }
}

/// Rolls the window up to the shade strip at `width`: the title-bar height,
/// keeping the window's current width so the rolled-up window matches the one
/// it replaces. Both shade paths — rolling straight up with a known size, and
/// the first shade once its measurement lands — resize this way, so the
/// expression lives here once. A no-op before the window id resolves, via
/// [`with_window_id`], like the other title-bar actions.
fn resize_to_shade(player: &WinampPlayer, width: f32) -> Task<Message> {
    with_window_id(player, |id| {
        iced::window::resize(id, iced::Size::new(width, views::TITLE_BAR_HEIGHT))
    })
}

/// Snaps the browse list back to its first row. The list carries
/// [`views::BROWSE_LIST_ID`] whichever level is showing, so one operation
/// serves the Top button and every navigation that opens a fresh list.
fn scroll_browse_to_top() -> Task<Message> {
    iced::widget::operation::snap_to(
        views::BROWSE_LIST_ID,
        iced::widget::scrollable::RelativeOffset::START,
    )
}

/// Runs a browse fetch and then snaps the list it fills to the top. Navigation
/// replaces the visible list, so leaving the new level at the previous list's
/// scroll offset would hide its first rows; batching the scroll with the fetch
/// keeps the two one operation from the update arm.
fn fetch_at_top(task: Task<Message>) -> Task<Message> {
    Task::batch([task, scroll_browse_to_top()])
}

fn update(player: &mut WinampPlayer, message: Message) -> Task<Message> {
    match message {
        Message::PlayPause => transport_into(
            &player.apple_music_service,
            "toggle play/pause",
            |service| async move { service.toggle_play_pause().await },
        ),
        // The keyboard's X and C keys: explicit, idempotent play/pause rather
        // than the button's toggle. X resumes (or restarts) the preview; C
        // pauses it.
        Message::Play => {
            transport_into(&player.apple_music_service, "play", |service| async move {
                service.resume().await
            })
        }
        Message::Pause => {
            transport_into(&player.apple_music_service, "pause", |service| async move {
                service.pause().await
            })
        }
        // Stop discards the preview and clears the playing flag; the service
        // remembers the last preview URL so Play can restart it.
        Message::Stop => {
            transport_into(&player.apple_music_service, "stop", |service| async move {
                service.stop().await
            })
        }
        Message::ToggleRepeat => mutate_state(player, AppState::toggle_repeat),
        Message::ToggleShuffle => mutate_state(player, AppState::toggle_shuffle),
        Message::VolumeChange(volume) => mutate_volume(player, |state| state.set_volume(volume)),
        Message::BalanceChange(balance) => {
            mutate_balance(player, |state| state.set_balance(balance))
        }
        // The arrow keys nudge the slider's value by its own step, so the
        // keyboard and the drag share one granularity; `nudge_volume` clamps.
        Message::VolumeUp => mutate_volume(player, |state| state.nudge_volume(views::VOLUME_STEP)),
        Message::VolumeDown => {
            mutate_volume(player, |state| state.nudge_volume(-views::VOLUME_STEP))
        }
        // The left and right arrows move the playing preview by Winamp's
        // five-second step; a seek while stopped is a no-op in the backend.
        Message::SeekBackward => seek_preview(player, -views::SEEK_STEP_SECS),
        Message::SeekForward => seek_preview(player, views::SEEK_STEP_SECS),
        // The equalizer mutators all store clamped gains in shared state, so
        // the sliders can never write an out-of-range value; the clamp itself
        // lives in `equalizer::clamp_gain`, called by the `AppState` setters.
        // Each arm also forwards the stored curve to the playing preview.
        Message::ToggleEqualizer => mutate_equalizer(player, AppState::toggle_equalizer),
        Message::EqPreampChange(gain) => {
            mutate_equalizer(player, |state| state.set_eq_preamp(gain))
        }
        Message::EqBandChange(band, gain) => {
            mutate_equalizer(player, |state| state.set_eq_band(band, gain))
        }
        Message::EqPresetSelected(preset) => {
            mutate_equalizer(player, |state| state.apply_eq_preset(preset))
        }
        Message::NextTrack => step_track(player, true),
        Message::PreviousTrack => step_track(player, false),
        // The marquee timer only advances the title window; the view uses the
        // offset solely for an overlong line, so a fitting line stays still.
        Message::MarqueeTick => {
            player.marquee_offset = player.marquee_offset.wrapping_add(1);
            Task::none()
        }
        Message::TrackSelected { epoch, index } => {
            // The message carries the pressed row's index plus the epoch of
            // the list that rendered it. The Songs view draws from `songs`
            // and the flat All Songs view draws from `all_songs`, so resolve
            // the press against the list the current view shows; resolving
            // it against the other list would no-op or play the wrong song.
            // A mismatched epoch means the list was replaced after the row
            // was rendered, so the index names a different song now — ignore
            // the stale press. An out-of-range index likewise names no track
            // and is a no-op rather than a bogus play. `select` applies both
            // checks.
            let selected = if player.current_view == CurrentView::AllSongs {
                player.all_songs.select(epoch, index)
            } else {
                player.songs.select(epoch, index)
            };
            let Some(song) = selected else {
                return Task::none();
            };
            let track_id = song.id.clone();
            let preview_url = song.preview_url.clone();
            // A newly selected song starts its title at character zero; the
            // previous song's scroll position must not carry over.
            player.marquee_offset = 0;
            // Record the played track's title, artist, and formatted length
            // before handing the id to the async play: the Now Playing bar
            // resolves its label, artist, and time from `known_tracks`, and the
            // entry must survive a later browse to a different list (which
            // replaces the visible song buffer). Recording once per play —
            // rather than folding every song of every loaded list into the map —
            // keeps the index proportional to songs actually played and takes
            // the fold off the browse path.
            player
                .known_tracks
                .entry(track_id.clone())
                .or_insert_with(|| views::KnownTrack {
                    title: song.title.clone(),
                    artist: song.artist.clone(),
                    time: views::format_track_time(song.duration_ms),
                });
            // A newer selection must win even when an older play's backend
            // call finishes later: issue a generation and hand its guard to
            // `play_track`, which evaluates it under the state lock so the
            // older play cannot overwrite the newer track. The browse path
            // uses the same [`RequestGeneration`] for the same reason.
            let generation = RequestGeneration::issue(&player.plays_generation);
            let id_for_report = track_id.clone();
            play_into(
                &player.apple_music_service,
                track_id,
                generation,
                PLAY_TIMEOUT,
                move |service, id, generation| async move {
                    service
                        .play_track(&id, preview_url.as_deref(), || generation.is_current())
                        .await
                },
                move |err| eprintln!("{}", play_failure_report(&id_for_report, err)),
            )
        }
        Message::ArtistSelected { epoch, index } => {
            // As with `TrackSelected`: the row message carries the artist's
            // index and its list's epoch. A mismatched epoch is a stale press
            // from a replaced list; an out-of-range index has no artist to
            // browse. `artists.select` rejects both.
            let Some(artist_id) = player
                .artists
                .select(epoch, index)
                .map(|artist| artist.id.clone())
            else {
                return Task::none();
            };
            player.current_view = CurrentView::Albums;
            // Entering a browse level leaves any active search: the songs list
            // the search filled is about to be replaced, so Back must not
            // treat it as a searched list any more.
            player.search_active = false;
            // Drop the previous artist's albums before the new fetch lands:
            // otherwise the Albums view renders them, and a press during the
            // fetch resolves against the wrong artist's list.
            fetch_at_top(fetch_level(
                &mut player.albums,
                &player.apple_music_service,
                format!("loading albums for artist {artist_id:?}"),
                move |service| async move { service.get_albums_by_artist(&artist_id).await },
                Message::AlbumsLoaded,
                Message::AlbumsLoadFailed,
            ))
        }
        Message::AlbumSelected { epoch, index } => {
            // The album twin of the artist arm above. The Songs view can be
            // opened from the per-artist Albums list or from the flat All
            // Albums list, so resolve the press against the list that view
            // rendered; `select` rejects the same stale-epoch and
            // out-of-range presses either way. The origin is recorded from the
            // same view, so Back later returns to the level the row came from.
            let from_all_albums = player.current_view == CurrentView::AllAlbums;
            let selected = if from_all_albums {
                player.all_albums.select(epoch, index)
            } else {
                player.albums.select(epoch, index)
            };
            let Some(album_id) = selected.map(|album| album.id.clone()) else {
                return Task::none();
            };
            player.songs_origin = if from_all_albums {
                SongsOrigin::AllAlbums
            } else {
                SongsOrigin::Album
            };
            player.current_view = CurrentView::Songs;
            player.search_active = false;
            // The songs twin of the artist arm above: clear the previous
            // album's songs so they cannot be shown or pressed while the new
            // album's fetch is in flight.
            fetch_at_top(fetch_level(
                &mut player.songs,
                &player.apple_music_service,
                format!("loading songs from album {album_id:?}"),
                move |service| async move { service.get_songs_from_album(&album_id).await },
                Message::SongsLoaded,
                Message::SongsLoadFailed,
            ))
        }
        Message::PlaylistSelected { epoch, index } => {
            // The playlist twin of the album arm above: resolve the press
            // against the Playlists list that rendered it, rejecting a stale
            // epoch or out-of-range index, then load the playlist's songs into
            // the shared songs buffer and record the origin for Back.
            let Some(playlist_id) = player
                .playlists
                .select(epoch, index)
                .map(|playlist| playlist.id.clone())
            else {
                return Task::none();
            };
            player.songs_origin = SongsOrigin::Playlist;
            player.current_view = CurrentView::Songs;
            player.search_active = false;
            // Clear the previous list before the fetch lands, as the album arm
            // does, so a press during the fetch cannot resolve against stale
            // rows.
            fetch_at_top(fetch_level(
                &mut player.songs,
                &player.apple_music_service,
                format!("loading songs from playlist {playlist_id:?}"),
                move |service| async move { service.get_songs_from_playlist(&playlist_id).await },
                Message::SongsLoaded,
                Message::SongsLoadFailed,
            ))
        }
        // The browse hierarchy is navigable both ways: ArtistSelected,
        // AlbumSelected, and PlaylistSelected step down (Artists → Albums →
        // Songs, or into a playlist's songs), Back steps up again. The Back
        // button is only rendered below the artist list, so this arm mostly
        // fires where a level exists to leave; the explicit
        // top-level arm keeps the no-op in the same place as the steps.
        Message::Back => {
            // Only stepping up replaces the visible list, so Back at the top
            // level stays a true no-op (the invariant docs pin that) while a
            // restored list starts at its first row.
            let stepping_up = player.search_active || player.current_view != CurrentView::Artists;
            // A search result list sits under the artists list, not under the
            // album it was issued from: leaving it returns straight to
            // Artists, and the album list it would otherwise open is empty or
            // stale. A normal browse steps up one level.
            if player.search_active {
                player.search_active = false;
                player.current_view = CurrentView::Artists;
            } else {
                player.current_view = match &player.current_view {
                    CurrentView::Songs => match player.songs_origin {
                        SongsOrigin::Album => CurrentView::Albums,
                        SongsOrigin::AllAlbums => CurrentView::AllAlbums,
                        SongsOrigin::Playlist => CurrentView::Playlists,
                    },
                    CurrentView::Albums => CurrentView::Artists,
                    CurrentView::AllSongs => CurrentView::Artists,
                    CurrentView::AllAlbums => CurrentView::Artists,
                    CurrentView::Playlists => CurrentView::Artists,
                    CurrentView::Artists => CurrentView::Artists,
                };
            }
            if stepping_up {
                scroll_browse_to_top()
            } else {
                Task::none()
            }
        }
        Message::ScrollBrowseToTop => scroll_browse_to_top(),
        Message::ShowAllSongs => {
            // The flat list hangs off the artists list, so an active search's
            // results are left behind and Back returns to Artists.
            player.search_active = false;
            player.current_view = CurrentView::AllSongs;
            // Clear the previous flat list before the new fetch lands, so a
            // press during the fetch cannot resolve against stale rows (the
            // same clear-before-fetch the navigation arms do).
            fetch_at_top(fetch_level(
                &mut player.all_songs,
                &player.apple_music_service,
                "loading all library songs".to_string(),
                |service| async move { service.get_all_songs().await },
                Message::AllSongsLoaded,
                Message::AllSongsLoadFailed,
            ))
        }
        Message::ShowAllAlbums => {
            // Like All Songs, the flat album list hangs off the artists list:
            // leave an active search behind and let Back return to Artists.
            player.search_active = false;
            player.current_view = CurrentView::AllAlbums;
            // Clear the previous flat list before the new fetch lands, so a
            // press during the fetch cannot resolve against stale rows.
            fetch_at_top(fetch_level(
                &mut player.all_albums,
                &player.apple_music_service,
                "loading all library albums".to_string(),
                |service| async move { service.get_all_albums().await },
                Message::AllAlbumsLoaded,
                Message::AllAlbumsLoadFailed,
            ))
        }
        Message::ShowPlaylists => {
            // Like the flat lists, Playlists hangs off the artists list: leave
            // an active search behind and let Back return to Artists.
            player.search_active = false;
            player.current_view = CurrentView::Playlists;
            // Clear the previous list before the fetch lands, so a press during
            // the fetch cannot resolve against stale rows.
            fetch_at_top(fetch_level(
                &mut player.playlists,
                &player.apple_music_service,
                "loading library playlists".to_string(),
                |service| async move { service.get_playlists().await },
                Message::PlaylistsLoaded,
                Message::PlaylistsLoadFailed,
            ))
        }
        Message::SearchChanged(text) => {
            player.search_query = text;
            Task::none()
        }
        Message::SearchSubmitted => {
            // A blank query has nothing to match: `search_songs` rejects it,
            // and issuing the fetch would only turn the empty box into an
            // error report. Leave the current list and view alone.
            let query = player.search_query.trim().to_string();
            if query.is_empty() {
                return Task::none();
            }
            player.search_query = query.clone();
            player.search_active = true;
            player.current_view = CurrentView::Songs;
            // The search results reuse the songs buffer, so the previous list's
            // rows are dropped and a press during the fetch cannot resolve
            // against them (the same clear-before-fetch the navigation arms do).
            fetch_at_top(fetch_level(
                &mut player.songs,
                &player.apple_music_service,
                format!("searching the library for {query:?}"),
                move |service| async move { service.search_songs(&query).await },
                Message::SongsLoaded,
                Message::SongsLoadFailed,
            ))
        }
        Message::LoadArtists => {
            // `boot` schedules this for the initial load, and again once the
            // startup sign-in stores a session; it is also the Retry button's
            // arm: clearing first drops any earlier failure report and shows
            // "Loading…" while the re-fetch is in flight, exactly as the
            // navigation arms clear their level before re-fetching.
            fetch_level(
                &mut player.artists,
                &player.apple_music_service,
                "loading favorite artists".to_string(),
                |service| async move { service.get_favorite_artists().await },
                Message::ArtistsLoaded,
                Message::ArtistsLoadFailed,
            )
        }
        Message::ArtistsLoaded(artists) => {
            player.artists.store(artists);
            Task::none()
        }
        Message::AlbumsLoaded(albums) => {
            player.albums.store(albums);
            Task::none()
        }
        Message::SongsLoaded(songs) => {
            // Cache each row's display title and formatted length once, when
            // the list loads, so a view refresh borrows them instead of
            // re-joining every artist and title and formatting every row's
            // duration again (see `BrowseList::store_with_row_text`).
            player.songs.store_with_row_text(
                songs,
                |song| song.display_title().into_owned(),
                |song| views::format_track_time(song.duration_ms),
            );
            Task::none()
        }
        Message::AllSongsLoaded(songs) => {
            player.all_songs.store_with_row_text(
                songs,
                |song| song.display_title().into_owned(),
                |song| views::format_track_time(song.duration_ms),
            );
            Task::none()
        }
        Message::AllAlbumsLoaded(albums) => {
            player.all_albums.store(albums);
            Task::none()
        }
        Message::PlaylistsLoaded(playlists) => {
            player.playlists.store(playlists);
            Task::none()
        }
        Message::ArtistsLoadFailed(report) => {
            player.artists.fail(report);
            Task::none()
        }
        Message::AlbumsLoadFailed(report) => {
            player.albums.fail(report);
            Task::none()
        }
        Message::SongsLoadFailed(report) => {
            player.songs.fail(report);
            Task::none()
        }
        Message::AllSongsLoadFailed(report) => {
            player.all_songs.fail(report);
            Task::none()
        }
        Message::AllAlbumsLoadFailed(report) => {
            player.all_albums.fail(report);
            Task::none()
        }
        Message::PlaylistsLoadFailed(report) => {
            player.playlists.fail(report);
            Task::none()
        }
        // The executor-thread check in `fetch_into` already dropped replies a
        // newer request had superseded when they were mapped. This re-checks
        // the stamp on the UI thread, where the store happens, closing the
        // window between that check and this arm: a navigation there issues a
        // newer request, and the stale reply must not repopulate the list (or
        // record its error) over it.
        Message::BrowseReply { issued, reply } => {
            let current = match reply.as_ref() {
                Message::ArtistsLoaded(_) | Message::ArtistsLoadFailed(_) => {
                    player.artists.is_current(issued)
                }
                Message::AlbumsLoaded(_) | Message::AlbumsLoadFailed(_) => {
                    player.albums.is_current(issued)
                }
                Message::SongsLoaded(_) | Message::SongsLoadFailed(_) => {
                    player.songs.is_current(issued)
                }
                Message::AllSongsLoaded(_) | Message::AllSongsLoadFailed(_) => {
                    player.all_songs.is_current(issued)
                }
                Message::AllAlbumsLoaded(_) | Message::AllAlbumsLoadFailed(_) => {
                    player.all_albums.is_current(issued)
                }
                Message::PlaylistsLoaded(_) | Message::PlaylistsLoadFailed(_) => {
                    player.playlists.is_current(issued)
                }
                _ => true,
            };
            if current {
                update(player, *reply)
            } else {
                Task::none()
            }
        }
        Message::TrackPlayed { generation } => {
            // Only the current track's entry is ever read (see
            // [`WinampPlayer::known_tracks`]), so the latest play's completion drops
            // every other entry the selections above accumulated. A completion
            // whose play a newer selection superseded is skipped: its
            // `current_track` names the older committed track, but the newer
            // selection's entry is still pending, and pruning to the older
            // track would delete it. When this completion is the latest, no
            // newer selection is pending, so reducing to `current_track` is
            // safe — and when nothing committed, clearing the map keeps it
            // bounded rather than leaving the failed selection's entry.
            if generation == player.plays_generation.load(Ordering::SeqCst) {
                let state = player.state.blocking_lock();
                if state.is_playing
                    && let Some(current) = state.current_track.as_deref()
                {
                    player.play_history.record(current);
                } else {
                    // A failed play leaves shared state naming the jump's
                    // target (the backend leaves `current_track` set), so keep
                    // the pending jump and mark it for retry: the next Previous
                    // offers that same target rather than walking past it.
                    player.play_history.retry_pending();
                }
                match state.current_track.as_deref() {
                    Some(current) => player.known_tracks.retain(|id, _| id == current),
                    None => player.known_tracks.clear(),
                }
            }
            Task::none()
        }
        Message::WindowIdResolved(id) => {
            player.window_id = id;
            Task::none()
        }
        // The custom title bar's window actions — drag, minimize, close, shade,
        // and always-on-top — all act on the resolved window id and are no-ops
        // until it exists; `with_window_id` holds that guard, so each arm names
        // only the `iced::window` call it schedules.
        Message::WindowDragged => with_window_id(player, iced::window::drag),
        Message::MinimizeWindow => with_window_id(player, |id| iced::window::minimize(id, true)),
        Message::CloseWindow => with_window_id(player, iced::window::close),
        // Shading measures the window only until the pre-shade size is
        // known; after that it rolls straight up to the strip. Unshading
        // resizes back to the captured size. `with_window_id` keeps both
        // no-ops before the window id resolves, exactly like the other
        // title-bar actions; the flag still flips so the view switches to
        // the title-bar-only tree even where the resize is ignored.
        Message::ToggleWindowShade => {
            if player.shaded {
                player.shaded = false;
                match player.unshaded_size {
                    Some(size) => with_window_id(player, |id| iced::window::resize(id, size)),
                    None => Task::none(),
                }
            } else {
                player.shaded = true;
                match player.unshaded_size {
                    // The size to restore is already known, so roll straight
                    // up; re-measuring could read the strip a prior shade left.
                    Some(size) => resize_to_shade(player, size.width),
                    // First shade: measure so the size can be restored.
                    None => with_window_id(player, |id| {
                        iced::window::size(id).map(Message::WindowShadeMeasured)
                    }),
                }
            }
        }
        Message::WindowShadeMeasured(size) => {
            // A measurement only belongs to a shade still waiting for its
            // first capture: one that lands after an unshade would clamp the
            // restored window and record the strip as the size to restore.
            if !player.shaded || player.unshaded_size.is_some() {
                return Task::none();
            }
            player.unshaded_size = Some(size);
            resize_to_shade(player, size.width)
        }
        // The clutter toggle flips the flag and schedules the window-level
        // change. `with_window_id` makes the level call a no-op before the
        // window id resolves, exactly like the other title-bar actions, while
        // the flag still flips so the view redraws the toggle.
        Message::ToggleAlwaysOnTop => {
            player.always_on_top = !player.always_on_top;
            let level = if player.always_on_top {
                iced::window::Level::AlwaysOnTop
            } else {
                iced::window::Level::Normal
            };
            with_window_id(player, move |id| iced::window::set_level(id, level))
        }
        Message::Ignored => Task::none(),
        Message::TransportSettled => Task::none(),
    }
}

/// Assembles the app screen: the Now Playing bar, transport row, and
/// equalizer panel — all built in `views` from the resolved title and the
/// shared playback, volume, Repeat, Shuffle, and EQ state — above the current
/// browse list. When the window is shaded, only [`views::view_title_bar`] is built
/// and the rest of the screen is dropped.
fn view(player: &WinampPlayer) -> Element<'_, Message> {
    // Shade mode builds only the title bar — the Now Playing bar, transport
    // row, equalizer, and browse list are all dropped, and the window itself
    // is resized down to this strip. Returning before the state lock keeps
    // the rolled-up frame cheap and free of a lock the content needs.
    if player.shaded {
        return views::view_title_bar(player.always_on_top);
    }

    // The now-playing title, artist, and time are resolved while the state
    // lock is held, from a borrowed `current_track` against the accumulated
    // `known_tracks` index (see [`WinampPlayer::now_playing_label`],
    // [`WinampPlayer::now_playing_artist`], and
    // [`WinampPlayer::now_playing_time`]). All three borrow from `known_tracks`
    // (or the `"Nothing"`/`""`/`"--:--"` literals), so those paths allocate
    // only in the unknown-id label fallback.
    let (
        now_playing,
        now_playing_artist,
        now_playing_time,
        is_playing,
        volume,
        balance,
        repeat,
        shuffle,
        eq_enabled,
        eq_preamp,
        eq_bands,
        eq_preset,
    ) = {
        let state = player.state.blocking_lock();
        (
            player.now_playing_label(state.current_track.as_deref()),
            player.now_playing_artist(state.current_track.as_deref()),
            player.now_playing_time(state.current_track.as_deref()),
            state.is_playing,
            state.volume(),
            state.balance(),
            state.repeat,
            state.shuffle,
            state.eq_enabled,
            state.eq_preamp(),
            state.eq_bands(),
            state.eq_preset(),
        )
    };

    let main_content = match &player.current_view {
        CurrentView::Artists => views::view_artists(
            &player.artists.items,
            player.artists.epoch,
            player.artists.loading,
            player.artists.error.as_deref(),
        ),
        CurrentView::Albums => views::view_albums(
            &player.albums.items,
            player.albums.epoch,
            player.albums.loading,
            player.albums.error.as_deref(),
        ),
        // A second short lock (like the label block above) hands the current
        // track's id to the Songs view so it can mark the playing row. The
        // borrowed id is compared inside `song_row` and the guard drops at
        // the end of this arm, so no per-frame `Option<String>` clone is
        // needed.
        CurrentView::Songs => {
            let state = player.state.blocking_lock();
            views::view_songs(
                &player.songs.items,
                &player.songs.titles,
                &player.songs.labels,
                player.songs.epoch,
                player.songs.loading,
                player.songs.error.as_deref(),
                state.current_track.as_deref(),
                player.search_active,
            )
        }
        // The flat whole-library view marks its playing row from the same
        // second lock the Songs arm takes.
        CurrentView::AllSongs => {
            let state = player.state.blocking_lock();
            views::view_all_songs(
                &player.all_songs.items,
                &player.all_songs.titles,
                &player.all_songs.labels,
                player.all_songs.epoch,
                player.all_songs.loading,
                player.all_songs.error.as_deref(),
                state.current_track.as_deref(),
            )
        }
        // The flat album list renders like the per-artist Albums view: album
        // rows with no current-track marker.
        CurrentView::AllAlbums => views::view_all_albums(
            &player.all_albums.items,
            player.all_albums.epoch,
            player.all_albums.loading,
            player.all_albums.error.as_deref(),
        ),
        CurrentView::Playlists => views::view_playlists(
            &player.playlists.items,
            player.playlists.epoch,
            player.playlists.loading,
            player.playlists.error.as_deref(),
        ),
    };

    let mut column = Column::new()
        .push(views::view_title_bar(player.always_on_top))
        .push(views::view_now_playing(
            now_playing,
            now_playing_artist,
            now_playing_time,
            player.marquee_offset,
        ))
        .push(views::view_transport_controls(
            is_playing, volume, balance, repeat, shuffle,
        ))
        .push(views::view_equalizer(
            eq_enabled, eq_preamp, &eq_bands, eq_preset,
        ));
    // The Back button sits above the list it navigates and exists only where
    // the hierarchy has a level above to return to (see `views::can_go_back`).
    if views::can_go_back(&player.current_view) {
        column = column.push(views::view_back_button());
    }
    // A failed top-level artists fetch is a dead end: no navigation re-issues
    // it, so the Artists view offers a Retry that re-runs `LoadArtists`.
    // Every other browse level recovers by navigating back into it, so those
    // failures need no button.
    if views::can_retry_artists(&player.current_view, player.artists.error.as_deref()) {
        column = column.push(views::view_retry_button());
    }
    // The Top button sits above every browse level, beside Back/Retry, so the
    // current list can always be returned to its first row.
    column = column.push(views::view_top_button());
    // The All Songs, All Albums, and Playlists buttons open the flat lists;
    // they live on the Artists view, beside Back/Retry, so each list is one
    // press from the top of the hierarchy.
    if matches!(player.current_view, CurrentView::Artists) {
        column = column
            .push(views::view_all_songs_button())
            .push(views::view_all_albums_button())
            .push(views::view_playlists_button());
    }
    // The search box sits on every browse screen, so a query can be typed
    // from any level of the hierarchy.
    column = column.push(views::view_search_box(&player.search_query));
    column.push(main_content).into()
}

#[cfg(test)]
mod tests;
