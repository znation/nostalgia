//! The iced application: the `WinampPlayer` app struct, its `Message` event
//! type, and the `update`/`view` loop `init_ui` hands to iced. Widget
//! construction lives in the `views` submodule (browse lists, Now Playing bar,
//! transport controls, equalizer panel), the Previous/Next stepping arithmetic
//! in `transport`, the library-fetch/error-reporting adapter in `loading`, the
//! per-level browse state machine in `browse`, the classic key bindings in
//! `shortcuts`, and the marquee, position-poll and spectrum-poll timer
//! subscriptions in `timers`; `update` is a dispatch table over the
//! per-concern message handlers in `handlers`, and this module wires those
//! to the shared `AppState` and the `AppleMusicService` seam. Its unit tests
//! live in the `tests` submodule.

use iced::{Element, Task, widget::Column};
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, atomic::AtomicU64},
    time::Duration,
};
use tokio::sync::Mutex;

mod bevel;
mod browse;
mod handlers;
mod loading;
mod shortcuts;
mod style;
mod theme;
mod ticks;
mod timers;
mod transport;
mod views;

use browse::BrowseList;
use timers::subscriptions;

use crate::{
    apple_music::AppleMusicService,
    audio::{PlaybackFormat, Spectrum},
    equalizer::Preset,
    library::{Album, Artist, Playlist, Song},
    state::AppState,
};

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
        .subscription(|player| {
            iced::Subscription::batch(subscriptions(
                player.needs_marquee(),
                player.needs_position_poll(),
                player.needs_spectrum_poll(),
            ))
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
    // character. The timer runs only while the line is too long to fit the
    // LCD's character budget (see [`subscriptions`]), so a tick always
    // scrolls the bar.
    MarqueeTick,
    // The position poll's tick: schedule queries of the loaded preview's
    // position, format, and length. Keeping the queries in tasks, rather than
    // in `update`, means a slow backend cannot stall the UI thread.
    PositionTick,
    // The position query's reply, stamped with the `position_epoch` that issued
    // it. A reply from before a Stop or a new selection is dropped so it
    // cannot restore the previous track's elapsed value.
    PlaybackPosition {
        epoch: u64,
        position: Option<Duration>,
    },
    // The format query's reply, also stamped with the `position_epoch` that
    // issued it. A reply from before a Stop or a new selection is dropped so
    // it cannot restore the previous track's sample rate and channel mode.
    PlaybackFormat {
        epoch: u64,
        format: Option<PlaybackFormat>,
    },
    // The spectrum poll's tick: schedule a query of the loaded preview's live
    // ten-band spectrum. It runs only while playback is running (see
    // [`subscriptions`]), so a paused or stopped player keeps its last bars.
    SpectrumTick,
    // The spectrum query's reply, stamped with the `position_epoch` that
    // issued it. A reply from before a Stop or a new selection is dropped so
    // it cannot restore the previous track's bars.
    Spectrum {
        epoch: u64,
        spectrum: Option<Spectrum>,
    },
    // The duration query's reply, also stamped with the `position_epoch` that
    // issued it. It carries the loaded preview's own length, which the seek
    // bar spans; a reply from before a Stop or a new selection is dropped so
    // it cannot scale the bar to the previous preview.
    PlaybackDuration {
        epoch: u64,
        duration: Option<Duration>,
    },
    // The seek bar's drag: the slider's value in seconds. It only stores the
    // in-progress drag, so dragging never reaches the backend.
    SeekPreviewChanged(f32),
    // The seek bar's release: seeks to the stored drag and clears it.
    SeekPreviewReleased,
    // The large left-hand LCD readout's mode toggle: flips between elapsed and
    // remaining time. It changes no playback state and leaves the right-edge
    // total alone.
    ToggleTimeDisplay,
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
    TrackSelected {
        epoch: u64,
        index: usize,
    },
    ArtistSelected {
        epoch: u64,
        index: usize,
    },
    AlbumSelected {
        epoch: u64,
        index: usize,
    },
    PlaylistSelected {
        epoch: u64,
        index: usize,
    },
    Back,
    // The browse view's Top button: snap the current list back to its first
    // row. The arm schedules an iced scroll operation on the browse list's
    // stable widget id and changes no state.
    ScrollBrowseToTop,
    // The browse view's Sort button: reorder the showing list alphabetically
    // by its rows' rendered text. The handler bumps that list's epoch so a
    // press rendered before the sort is rejected, and leaves every other list
    // and the playback state alone.
    SortBrowse,
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
    BrowseReply {
        issued: u64,
        reply: Box<Message>,
    },
    // A browse reply that a newer request for the same list has superseded.
    // `fetch_into` emits this instead of the `*Loaded` message so the stale
    // reply never reaches `BrowseList::store`; the handler it routes to is a
    // no-op.
    Ignored,
    // The play's completion carries the generation its selection issued, so
    // the arm can tell whether a newer selection has superseded it. Only the
    // latest completion may prune the title index; a superseded one must not
    // drop a still-pending selection's title.
    TrackPlayed {
        generation: u64,
    },
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
    /// The loaded preview's last reported playback position, or `None` when no
    /// preview is loaded or the backend cannot answer. `Message::PlaybackPosition`
    /// stores it; the Now Playing bar reads it for the large elapsed readout.
    /// Pause leaves it alone, so the bar keeps showing the paused player's
    /// frozen position.
    playback_position: Option<Duration>,
    /// The loaded preview's backend-reported sample rate and channel mode, or
    /// `None` when no preview is loaded or the backend cannot answer.
    /// `Message::PlaybackFormat` stores it; the LCD's small status row reads
    /// it, and a missing value leaves that row blank. Pause keeps it, while
    /// Stop and a new selection clear it through `reset_position_readout`.
    playback_format: Option<PlaybackFormat>,
    /// The loaded preview's own backend-reported length, or `None` when no
    /// preview is loaded or the backend cannot answer. `Message::PlaybackDuration`
    /// stores it; the seek bar spans this length, while the LCD's right edge
    /// keeps the catalog track's total. Stop and a new selection clear it
    /// through `reset_position_readout`.
    playback_duration: Option<Duration>,
    /// The loaded preview's last reported live spectrum, or `None` when no
    /// preview is loaded or the backend cannot answer. `Message::Spectrum`
    /// stores it; the LCD's visualizer row reads it, and a missing value draws
    /// zero-height bars without changing the row's height. Pause keeps it, so
    /// the bars freeze at their last levels, while Stop and a new selection
    /// clear it through `reset_position_readout`.
    spectrum: Option<Spectrum>,
    /// The seek bar's in-progress drag, or `None` while the thumb follows the
    /// reported position. `Message::SeekPreviewChanged` stores the drag and
    /// `Message::SeekPreviewReleased` seeks to and clears it, so only a release
    /// touches the backend. Stop and a new selection clear it too.
    seek_preview: Option<Duration>,
    /// Which time the large left-hand LCD readout shows: elapsed by default,
    /// or the track's remaining time after a click. Presentation-only: it
    /// changes no playback state, and the mode survives Pause, Stop,
    /// Next/Previous, and a new selection until clicked again.
    time_display: views::TimeDisplay,
    /// The playback transition counter. Each backend query carries its value;
    /// `reset_position_readout` bumps it on a Stop or a new selection, so a
    /// reply that was already in flight is dropped rather than restoring the
    /// previous track's values.
    position_epoch: u64,
    /// Backend transport transitions whose call has not yet completed. Every
    /// handler that schedules one — Play, Pause, Play/Play-Pause, Stop, and a
    /// selected track's play — increments it before scheduling its async
    /// work, and the completion that reports that same transition
    /// (`TransportSettled` for the transports, `TrackPlayed` for the play)
    /// decrements it. The position poll waits for the count to reach zero, so
    /// it cannot query a backend that still holds the previous player, and a
    /// completion belonging to some other transition cannot close that window
    /// early.
    pending_position_transitions: u64,
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
/// the selected ids here would only duplicate state that nothing reads.
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
            playback_position: None,
            playback_format: None,
            playback_duration: None,
            spectrum: None,
            seek_preview: None,
            time_display: views::TimeDisplay::Elapsed,
            position_epoch: 0,
            pending_position_transitions: 0,
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

    /// The Now Playing bar's right-edge length readout for `current_track`: the
    /// plain total borrowed from the label cached in [`Self::known_tracks`], or
    /// the static `--:--` placeholder for a track the index does not know. A
    /// reported playback position never changes it; the big elapsed readout
    /// comes from [`Self::now_playing_elapsed`] instead. Like the label and
    /// artist, the known case borrows from the index, so `view` renders the bar
    /// without formatting or allocating a `String` per frame.
    fn now_playing_time<'a>(&'a self, current_track: Option<&str>) -> Cow<'a, str> {
        views::now_playing_time(&self.known_tracks, current_track)
    }

    /// The Now Playing bar's large left-hand time readout for
    /// `current_track`, in the mode [`Self::time_display`] selects: the
    /// position [`Self::playback_position`] reports as zero-padded `MM:SS` by
    /// default, or the countdown from the known track's length when the user
    /// has clicked the readout. The placeholder cases borrow, so a per-frame
    /// refresh allocates only while an actual time is shown.
    fn now_playing_elapsed<'a>(&'a self, current_track: Option<&str>) -> Cow<'a, str> {
        views::now_playing_elapsed(
            &self.known_tracks,
            current_track,
            self.playback_position,
            self.time_display,
        )
    }

    /// Whether the Now Playing line is too long for the LCD's character
    /// budget and so has to scroll: the combined artist and title exceed
    /// `views::MARQUEE_WIDTH` characters.
    ///
    /// The marquee timer is subscribed only while this holds (see
    /// [`subscriptions`]), so the bar is not rebuilt four times a second for a
    /// line that fits whole. The label and artist are resolved exactly as
    /// `view` resolves them, so the timer scrolls precisely the lines the bar
    /// windows.
    fn needs_marquee(&self) -> bool {
        let state = self.state.blocking_lock();
        let label = self.now_playing_label(state.current_track.as_deref());
        let artist = self.now_playing_artist(state.current_track.as_deref());
        views::needs_marquee(&label, &artist)
    }

    /// Whether the position poll is needed: playback is running, the audio
    /// backend still holds a preview (paused, even before its first position
    /// reply, so a seek can still move it), or a stored position is still shown
    /// because Pause kept it. A stopped player's readout is already the `00:00`
    /// Stop cleared it to, so the poll stays unsubscribed and the UI does not
    /// rebuild the whole screen, browse list included, twice a second to
    /// re-render an unchanged readout (see [`subscriptions`]).
    fn needs_position_poll(&self) -> bool {
        if self.playback_position.is_some() || self.apple_music_service.has_loaded_preview() {
            return true;
        }
        self.state.blocking_lock().is_playing
    }

    /// Whether the spectrum poll is needed: only while playback is running.
    /// Pause and Stop drop the timer, so the bars freeze at their last levels
    /// instead of repainting the whole screen four times a second for an
    /// unchanged strip (see [`subscriptions`]).
    fn needs_spectrum_poll(&self) -> bool {
        self.state.blocking_lock().is_playing
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

/// Routes each [`Message`] to the handler that owns its behavior: the
/// transport buttons and keys, the track stepping, and the play request and
/// its completion in `handlers::playback`; the volume, balance, seek, and
/// equalizer in `handlers::mixer`; the browse hierarchy, the library search,
/// and the fetch results in `handlers::browse`; and the custom title bar's
/// window actions in `handlers::window`. The table carries no behavior of its
/// own, so a change to one concern stays in that concern's module.
fn update(player: &mut WinampPlayer, message: Message) -> Task<Message> {
    match message {
        Message::PlayPause => handlers::playback::play_pause(player),
        Message::Play => handlers::playback::play(player),
        Message::Pause => handlers::playback::pause(player),
        Message::Stop => handlers::playback::stop(player),
        Message::ToggleRepeat => handlers::playback::toggle_repeat(player),
        Message::ToggleShuffle => handlers::playback::toggle_shuffle(player),
        Message::VolumeChange(volume) => handlers::mixer::volume_change(player, volume),
        Message::BalanceChange(balance) => handlers::mixer::balance_change(player, balance),
        Message::VolumeUp => handlers::mixer::volume_up(player),
        Message::VolumeDown => handlers::mixer::volume_down(player),
        Message::SeekBackward => handlers::mixer::seek_backward(player),
        Message::SeekForward => handlers::mixer::seek_forward(player),
        Message::ToggleEqualizer => handlers::mixer::toggle_equalizer(player),
        Message::EqPreampChange(gain) => handlers::mixer::eq_preamp_change(player, gain),
        Message::EqBandChange(band, gain) => handlers::mixer::eq_band_change(player, band, gain),
        Message::EqPresetSelected(preset) => handlers::mixer::eq_preset_selected(player, preset),
        Message::NextTrack => handlers::playback::step_track(player, true),
        Message::PreviousTrack => handlers::playback::step_track(player, false),
        Message::MarqueeTick => handlers::playback::marquee_tick(player),
        // The position poll asks the backend in tasks, as the service's
        // backend queries bound their own wait; `update` stays non-blocking.
        // Each reply carries the epoch it was issued under, so a transition that
        // lands first invalidates it. While a transport transition is still
        // reaching the backend, no query is issued: the backend would still
        // report the previous preview, and the epoch no longer identifies it.
        Message::PositionTick => handlers::playback::position_tick(player),
        Message::PlaybackPosition { epoch, position } => {
            handlers::playback::playback_position(player, epoch, position)
        }
        Message::PlaybackFormat { epoch, format } => {
            handlers::playback::playback_format(player, epoch, format)
        }
        Message::PlaybackDuration { epoch, duration } => {
            handlers::playback::playback_duration(player, epoch, duration)
        }
        Message::SpectrumTick => handlers::playback::spectrum_tick(player),
        Message::Spectrum { epoch, spectrum } => {
            handlers::playback::spectrum(player, epoch, spectrum)
        }
        Message::SeekPreviewChanged(value) => {
            handlers::playback::seek_preview_changed(player, value)
        }
        Message::SeekPreviewReleased => handlers::playback::seek_preview_released(player),
        Message::ToggleTimeDisplay => handlers::playback::toggle_time_display(player),
        Message::TrackSelected { epoch, index } => {
            handlers::playback::track_selected(player, epoch, index)
        }
        Message::ArtistSelected { epoch, index } => {
            handlers::browse::artist_selected(player, epoch, index)
        }
        Message::AlbumSelected { epoch, index } => {
            handlers::browse::album_selected(player, epoch, index)
        }
        Message::PlaylistSelected { epoch, index } => {
            handlers::browse::playlist_selected(player, epoch, index)
        }
        Message::Back => handlers::browse::back(player),
        Message::ScrollBrowseToTop => handlers::browse::scroll_browse_to_top(),
        Message::SortBrowse => handlers::browse::sort_browse(player),
        Message::ShowAllSongs => handlers::browse::show_all_songs(player),
        Message::ShowAllAlbums => handlers::browse::show_all_albums(player),
        Message::ShowPlaylists => handlers::browse::show_playlists(player),
        Message::SearchChanged(text) => handlers::browse::search_changed(player, text),
        Message::SearchSubmitted => handlers::browse::search_submitted(player),
        Message::LoadArtists => handlers::browse::load_artists(player),
        Message::ArtistsLoaded(artists) => handlers::browse::artists_loaded(player, artists),
        Message::AlbumsLoaded(albums) => handlers::browse::albums_loaded(player, albums),
        Message::SongsLoaded(songs) => handlers::browse::songs_loaded(player, songs),
        Message::AllSongsLoaded(songs) => handlers::browse::all_songs_loaded(player, songs),
        Message::AllAlbumsLoaded(albums) => handlers::browse::all_albums_loaded(player, albums),
        Message::PlaylistsLoaded(playlists) => {
            handlers::browse::playlists_loaded(player, playlists)
        }
        Message::ArtistsLoadFailed(report) => handlers::browse::artists_load_failed(player, report),
        Message::AlbumsLoadFailed(report) => handlers::browse::albums_load_failed(player, report),
        Message::SongsLoadFailed(report) => handlers::browse::songs_load_failed(player, report),
        Message::AllSongsLoadFailed(report) => {
            handlers::browse::all_songs_load_failed(player, report)
        }
        Message::AllAlbumsLoadFailed(report) => {
            handlers::browse::all_albums_load_failed(player, report)
        }
        Message::PlaylistsLoadFailed(report) => {
            handlers::browse::playlists_load_failed(player, report)
        }
        Message::BrowseReply { issued, reply } => {
            handlers::browse::browse_reply(player, issued, reply)
        }
        Message::TrackPlayed { generation } => handlers::playback::track_played(player, generation),
        Message::WindowIdResolved(id) => handlers::window::window_id_resolved(player, id),
        Message::WindowDragged => handlers::window::window_dragged(player),
        Message::MinimizeWindow => handlers::window::minimize_window(player),
        Message::CloseWindow => handlers::window::close_window(player),
        Message::ToggleWindowShade => handlers::window::toggle_window_shade(player),
        Message::WindowShadeMeasured(size) => handlers::window::window_shade_measured(player, size),
        Message::ToggleAlwaysOnTop => handlers::window::toggle_always_on_top(player),
        Message::Ignored => handlers::browse::ignored(player),
        Message::TransportSettled => handlers::playback::transport_settled(player),
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

    // The now-playing title, artist, elapsed time, and total are resolved while
    // the state lock is held, from a borrowed `current_track` against the
    // accumulated `known_tracks` index (see [`WinampPlayer::now_playing_label`],
    // [`WinampPlayer::now_playing_artist`], [`WinampPlayer::now_playing_elapsed`],
    // and [`WinampPlayer::now_playing_time`]). The label, artist, and elapsed
    // placeholder borrow from `known_tracks` (or the `"Nothing"`/`""`/
    // `"00:00"`/`"--:--"` literals); the total borrows the cached length, while
    // a reported elapsed position allocates its formatted `MM:SS` string.
    let (
        now_playing,
        now_playing_artist,
        now_playing_elapsed,
        now_playing_time,
        playback_format,
        spectrum,
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
            player.now_playing_elapsed(state.current_track.as_deref()),
            player.now_playing_time(state.current_track.as_deref()),
            player.playback_format,
            player.spectrum,
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
            now_playing_elapsed,
            now_playing,
            now_playing_artist,
            now_playing_time,
            player.marquee_offset,
            playback_format,
            spectrum,
        ))
        .push(views::view_seek_bar(
            player.seek_preview.or(player.playback_position),
            player.playback_duration,
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
    // The Top and Sort buttons share one row, Sort beside Top, so the
    // current list can always be returned to its first row and reordered
    // in place.
    column = column.push(views::view_browse_nav_buttons());
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
