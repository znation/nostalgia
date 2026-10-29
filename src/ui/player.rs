//! The UI's app model: `WinampPlayer`, the state the iced loop reads and
//! mutates, its construction and settings persistence, and its timer
//! predicates. It owns the shared `AppState` handle and the loaded browse
//! lists; `presentation` resolves those into the Now Playing strings and
//! readout the views draw, and the root screen assembly lives in `view`.

use std::collections::HashMap;
use std::sync::{Arc, atomic::AtomicU64};
use std::time::Duration;

use tokio::sync::Mutex;

use crate::apple_music::AppleMusicService;
use crate::audio::{PlaybackFormat, Spectrum};
use crate::library::{Album, Artist, Playlist, Song};
use crate::settings::Settings;
use crate::settings::library_view::LibraryView;
use crate::settings::seam::SettingsWriter;
use crate::state::AppState;

use super::browse::BrowseList;
use super::genres::Genre;
use super::{transport, views};

mod presentation;

/// How many LCD ticks a key or wheel nudge of a mixer level shows its readout:
/// one second at `MARQUEE_TICK`.
pub(in crate::ui) const MIXER_FLASH_TICKS: u8 = 4;

/// How many LCD ticks Shift+V's fade-out takes before it stops: two seconds at
/// `MARQUEE_TICK`.
pub(in crate::ui) const FADE_OUT_TICKS: u8 = 8;

/// The app model the iced loop owns: the shared playback state, the Apple
/// Music service, the browse lists and their view state, the Now Playing
/// readouts and drag state, and the window flags and settings writer.
/// Handlers mutate it and the views read it; `update` dispatches each
/// `Message` to one of them.
pub(in crate::ui) struct WinampPlayer {
    pub(in crate::ui) state: Arc<Mutex<AppState>>,
    pub(in crate::ui) apple_music_service: AppleMusicService,
    /// The app window's id, resolved at boot by `iced::window::latest()`. The
    /// custom title bar's window messages — drag, minimize, close, shade, and
    /// always-on-top — act on this window; a `None` (before the query
    /// resolves, or if it fails) makes their window calls no-ops.
    pub(in crate::ui) window_id: Option<iced::window::Id>,
    /// Whether the window is rolled up into shade mode: `view` then renders
    /// only the title bar and the compact transport strip, and the window is
    /// resized to [`views::SHADED_HEIGHT`]. The choice is saved and restored by
    /// `persist_settings_if_changed` and `adopt_saved_settings`, so a launch
    /// whose settings say shaded draws the rolled-up tree from the first view.
    pub(in crate::ui) shaded: bool,
    /// The window's last screen position reported by the window system, saved
    /// with the settings so the next launch opens there. `None` until a move
    /// event arrives (none do on Wayland).
    pub(in crate::ui) window_position: Option<(f32, f32)>,
    /// Whether Control is held, tracked from modifier events so a click on Previous or Next can
    /// jump to the list's edge instead of stepping. Not saved in settings.
    pub(in crate::ui) control_held: bool,
    /// Whether Shift is held, tracked from modifier events so a Shift-click on Stop fades out.
    /// Not saved in settings.
    pub(in crate::ui) shift_held: bool,
    /// Whether the window is pinned above other windows. The LCD strip's `A`
    /// toggle and the `A` key flip it, and `update` schedules
    /// `iced::window::set_level` to match; `view` renders the strip's control
    /// sunken while it holds. It lives here rather than in `AppState` so the
    /// shaded frame can read it without the state lock.
    pub(in crate::ui) always_on_top: bool,
    /// The window's inner size captured just before the first shade, so
    /// unshading can restore it. `None` until the measurement lands.
    pub(in crate::ui) unshaded_size: Option<iced::Size>,
    pub(in crate::ui) current_view: CurrentView,
    /// The browse levels. Each `BrowseList` owns its rows, epoch, loading
    /// flag, error report, and request counter (see `browse`).
    pub(in crate::ui) artists: BrowseList<Artist>,
    pub(in crate::ui) albums: BrowseList<Album>,
    pub(in crate::ui) songs: BrowseList<Song>,
    /// The flat whole-library list behind the Artists view's All Songs button.
    pub(in crate::ui) all_songs: BrowseList<Song>,
    /// The flat whole-library album list behind the Artists view's All Albums
    /// button.
    pub(in crate::ui) all_albums: BrowseList<Album>,
    /// The library's recently added albums behind the Artists view's Recently
    /// Added button, newest first.
    pub(in crate::ui) recently_added: BrowseList<Album>,
    /// The user's recently played songs behind the Artists view's Recently
    /// Played button, most recent first.
    pub(in crate::ui) recently_played: BrowseList<Song>,
    /// The library's playlists behind the Artists view's Playlists button.
    pub(in crate::ui) playlists: BrowseList<Playlist>,
    /// The library's songs grouped by genre behind the Artists view's Genres button.
    pub(in crate::ui) genres: BrowseList<Genre>,
    /// Which list the Songs view was opened from. `Album` returns to the
    /// per-artist Albums view on Back; `AllAlbums` returns to the flat All
    /// Albums view; `RecentlyAdded` returns to the Recently Added view; `Playlist` returns to the Playlists view. Set by the
    /// selection arm from the view that rendered the pressed row, so Back
    /// always steps to the level the row came from.
    pub(in crate::ui) songs_origin: SongsOrigin,
    /// The library search box's text, as typed. `Message::SearchChanged`
    /// stores each keystroke here and the search box renders it back; a submit
    /// trims it into the fetch's query.
    pub(in crate::ui) search_query: String,
    /// Whether the Songs view is showing the results of a library search
    /// rather than a browsed album. `Message::SearchSubmitted` sets it, and a
    /// browse step or `Back` clears it, so Back leaves the search for the
    /// artists list instead of the album the search was issued from.
    pub(in crate::ui) search_active: bool,
    /// Each played song's id mapped to its [`views::KnownTrack`] (playlist
    /// number prefix, title, artist, and formatted length), so the Now Playing
    /// bar can still name and time the playing track after the user browses to
    /// a different list (whose load replaces the visible song buffer). The
    /// `TrackSelected` arm records a track's number, title, artist, and
    /// formatted length when it is played, and `now_playing_label` /
    /// `now_playing_artist` / `now_playing_number` / `now_playing_time`
    /// resolve the per-frame bar with a single get instead
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
    pub(in crate::ui) known_tracks: HashMap<String, views::KnownTrack>,
    /// The tracks actually played, in play order, backing Previous while
    /// Shuffle is on. A committed `TrackPlayed` records the track; a shuffled
    /// Previous retraces it, and a list that no longer shows the previous
    /// entry falls back to sequential stepping.
    pub(in crate::ui) play_history: transport::PlayHistory,
    /// The songs played this Shuffle round, so a shuffled forward step
    /// plays each song of the showing list once before any repeats.
    pub(in crate::ui) shuffle_round: transport::ShuffleRound,
    /// How far an overlong Now Playing title has scrolled: the character
    /// offset `views::view_now_playing` windows the combined number, artist,
    /// and title line by.
    /// `Message::MarqueeTick` advances it, and a `TrackSelected` resets it so
    /// a new song starts at the beginning of its title. The view ignores it
    /// whenever the line fits the LCD's character budget.
    pub(in crate::ui) marquee_offset: usize,
    /// The hand-scroll drag of an overlong Now Playing title while the left button is held, or
    /// `None`. The marquee timer pauses for its duration, and a new selection clears it.
    pub(in crate::ui) title_drag: Option<crate::ui::handlers::playback::title_drag::TitleDrag>,
    /// The loaded preview's last reported playback position, or `None` when no
    /// preview is loaded or the backend cannot answer. `Message::PlaybackPosition`
    /// stores it; the Now Playing bar reads it for the large elapsed readout.
    /// Pause leaves it alone, so the bar keeps showing the paused player's
    /// frozen position.
    pub(in crate::ui) playback_position: Option<Duration>,
    /// The loaded preview's backend-reported sample rate and channel mode, or
    /// `None` when no preview is loaded or the backend cannot answer.
    /// `Message::PlaybackFormat` stores it; the LCD's small status row reads
    /// it, and a missing value omits the prefix and leaves both channel labels
    /// dim. Pause keeps it, while Stop and a new selection clear it through
    /// `reset_position_readout`.
    pub(in crate::ui) playback_format: Option<PlaybackFormat>,
    /// The loaded preview's own backend-reported length, or `None` when no
    /// preview is loaded or the backend cannot answer. `Message::PlaybackDuration`
    /// stores it; the seek bar spans this length, while the LCD's right edge
    /// keeps the catalog track's total. Stop and a new selection clear it
    /// through `reset_position_readout`.
    pub(in crate::ui) playback_duration: Option<Duration>,
    /// The paused elapsed readout's blink phase. `position_tick` toggles it on
    /// each poll while the transport is paused and holds it true whenever the
    /// transport is not paused. The view reads it only in the paused mode and
    /// draws the text transparent when it is false, keeping the row's bounds;
    /// while stopped, the view clears a current track's time readout regardless
    /// of this flag, keeping only the `--:--` placeholder.
    pub(in crate::ui) blink_visible: bool,
    /// The loaded preview's last reported live spectrum, or `None` when no
    /// preview is loaded or the backend cannot answer. `Message::Spectrum`
    /// stores it; the LCD's visualizer row reads it, and a missing value draws
    /// zero-height bars without changing the row's height. Pause keeps it, so
    /// the bars freeze at their last levels, while Stop and a new selection
    /// clear it through `reset_position_readout`.
    pub(in crate::ui) spectrum: Option<Spectrum>,
    /// The levels of the visualizer's peak markers: each band holds its
    /// highest recent level and drops back on each spectrum reply. It follows
    /// [`Self::spectrum`] (same reply guards, Pause freeze, Stop and selection
    /// clear), is not persisted, and starts as `None`.
    pub(in crate::ui) spectrum_peaks: Option<Spectrum>,
    /// Whether the LCD's indicator strip's V control lets the visualizer row
    /// draw the stored spectrum. `Message::ToggleVisualizer` flips it; hiding
    /// leaves [`Self::spectrum`] and the spectrum poll untouched, so showing
    /// the bars again draws the current ones. The choice is saved and restored
    /// by `persist_settings_if_changed` and `adopt_saved_settings`.
    pub(in crate::ui) visualizer_visible: bool,
    /// The transient "Stop after current" one-shot: `Message::ToggleStopAfterCurrent`
    /// (Ctrl+V) flips it, and a natural preview end that finds it set stops
    /// playback instead of advancing and clears it. False at launch and never
    /// persisted; manual transports neither read nor clear it.
    pub(in crate::ui) stop_after_current: bool,
    /// Whether the normal-mode `NOSTALGIA EQUALIZER` band and equalizer panel
    /// are drawn. `Message::ToggleEqualizerPanel` flips it; it is open on a
    /// first launch and is saved and restored by `persist_settings_if_changed`
    /// and `adopt_saved_settings`. It is a layout flag only: the EQ's on/off
    /// state, curve, and preset, and the audio they forward, are untouched, and
    /// hiding the section hands its height to the browse list without resizing
    /// the window.
    pub(in crate::ui) eq_panel_visible: bool,
    /// Whether the normal-mode library section — its `NOSTALGIA LIBRARY` band,
    /// browse buttons, search row, list, and footer — is drawn.
    /// `Message::ToggleLibraryPanel` flips it; it is open on a first launch and
    /// is saved and restored like [`Self::eq_panel_visible`]. It is a
    /// layout flag only: no playback, equalizer, browse, search, or selection
    /// state changes, and hiding the section does not resize the window.
    pub(in crate::ui) library_panel_visible: bool,
    /// Whether the transient About panel is drawn over the normal-mode tree.
    /// `Message::ToggleAbout` flips it; it is closed at launch and deliberately
    /// not persisted. Opening it while [`Self::shaded`] holds restores the
    /// window first, and `view` never draws it in shade mode. At most one of
    /// this and [`Self::options_visible`], [`Self::file_info_visible`],
    /// [`Self::jump_time_visible`], and [`Self::jump_file_visible`] holds.
    pub(in crate::ui) about_visible: bool,
    /// Whether the transient options menu is drawn over the normal-mode tree.
    /// `Message::ToggleOptions` flips it; it is closed at launch and
    /// deliberately not persisted, because the menu only lists existing
    /// choices. Opening it while [`Self::shaded`] holds restores the window
    /// first, and `view` never draws it in shade mode. At most one of this and
    /// [`Self::about_visible`], [`Self::file_info_visible`],
    /// [`Self::jump_time_visible`], and [`Self::jump_file_visible`] holds.
    pub(in crate::ui) options_visible: bool,
    /// Layout-only index into `OptionsRow::ALL`, reset when the menu opens.
    pub(in crate::ui) options_selected: usize,
    /// Whether the transient File info panel is drawn over the normal-mode
    /// tree. `Message::ToggleFileInfo` flips it; it is closed at launch and
    /// deliberately not persisted. At most one of this,
    /// [`Self::about_visible`], [`Self::options_visible`],
    /// [`Self::jump_time_visible`], and [`Self::jump_file_visible`] holds.
    pub(in crate::ui) file_info_visible: bool,
    /// Whether the transient Jump to time panel is drawn over the normal-mode
    /// tree. `Message::ToggleJumpTime` flips it; it is closed at launch and
    /// deliberately not persisted. At most one of this,
    /// [`Self::about_visible`], [`Self::options_visible`],
    /// [`Self::file_info_visible`], and [`Self::jump_file_visible`] holds.
    pub(in crate::ui) jump_time_visible: bool,
    /// The Jump to time panel's typed text, cleared each time it opens.
    pub(in crate::ui) jump_time_query: String,
    /// Whether the transient Jump to File panel is drawn over the normal-mode
    /// showing song list. `Message::ToggleJumpFile` flips it; it is closed at
    /// launch and deliberately not persisted. At most one of this,
    /// [`Self::about_visible`], [`Self::options_visible`],
    /// [`Self::file_info_visible`], and [`Self::jump_time_visible`] holds.
    /// Navigation that changes the showing list clears it, so a panel left open
    /// cannot reappear over the next list.
    pub(in crate::ui) jump_file_visible: bool,
    /// The Jump to File panel's typed filter text, cleared each time it opens.
    pub(in crate::ui) jump_file_query: String,
    /// The Jump to File panel's highlighted match, as the source index of the
    /// showing list. `None` means the first match, as on opening or after
    /// typing a new filter; Up and Down move it within the filtered matches.
    /// Layout-only and not persisted.
    pub(in crate::ui) jump_file_selected: Option<usize>,
    /// The seek bar's in-progress drag, or `None` while the thumb follows the
    /// reported position. `Message::SeekPreviewChanged` stores the drag and
    /// `Message::SeekPreviewReleased` seeks to and clears it, so only a release
    /// touches the backend. The held drag also previews in the large elapsed
    /// readout through [`Self::readout_position`], and clearing it falls back
    /// to the reported position. Stop and a new selection clear it too.
    pub(in crate::ui) seek_preview: Option<Duration>,
    /// Which time the large left-hand LCD readout shows: elapsed by default,
    /// or the track's remaining time after a click. Presentation-only: it
    /// changes no playback state, and the mode survives Pause, Stop,
    /// Next/Previous, and a new selection until clicked again.
    pub(in crate::ui) time_display: views::TimeDisplay,
    /// The mixer slider being dragged, whose live level the LCD shows in place
    /// of the title; transient, never saved to settings.
    pub(in crate::ui) mixer_drag: Option<views::MixerDrag>,
    /// The LCD ticks the readout still has after a key or wheel nudge of a
    /// mixer level, shown in place of the title like a drag's; 0 when none
    /// shows. Counted down by `marquee_tick`; transient, never saved to
    /// settings.
    pub(in crate::ui) mixer_flash_ticks: u8,
    /// The LCD ticks Shift+V's fade-out has left before it stops; 0 when no
    /// fade runs. Counted down by `marquee_tick`; transient, never saved to
    /// settings.
    pub(in crate::ui) fade_out_ticks: u8,
    /// The slider whose readout the flash shows; means nothing while
    /// `mixer_flash_ticks` is 0, and is never saved to settings.
    pub(in crate::ui) mixer_flash: views::MixerDrag,
    /// The last unsnapped balance value the current drag requested, so the
    /// center detent compares each move with the cursor's previous position
    /// rather than the (possibly snapped) stored balance. Cleared on release.
    pub(in crate::ui) balance_drag_raw: Option<f32>,
    /// The playback transition counter. Each backend query carries its value;
    /// `reset_position_readout` bumps it on a Stop or a new selection, so a
    /// reply that was already in flight is dropped rather than restoring the
    /// previous track's values.
    pub(in crate::ui) position_epoch: u64,
    /// Backend transport transitions whose call has not yet completed. Every
    /// handler that schedules one — Play, Pause, Play/Play-Pause, Stop, and a
    /// selected track's play — increments it before scheduling its async
    /// work, and the completion that reports that same transition
    /// (`TransportSettled` for the transports, `TrackPlayed` for the play)
    /// decrements it. The position poll waits for the count to reach zero, so
    /// it cannot query a backend that still holds the previous player, and a
    /// completion belonging to some other transition cannot close that window
    /// early.
    pub(in crate::ui) pending_position_transitions: u64,
    /// The playback-request counter. Each `TrackSelected` bumps it and
    /// carries its value into the async play as a guard: `play_track`
    /// evaluates the guard while holding the state lock and commits the
    /// track only if no newer play has been issued. Without it, two rapid
    /// selections whose plays complete out of order let the older play
    /// overwrite the newer one, so the Now Playing bar names the wrong
    /// track.
    pub(in crate::ui) plays_generation: Arc<AtomicU64>,
    /// The settings file writer, attached by `init_ui` from the store `main`
    /// resolved. `None` in tests and before boot attaches it, which makes
    /// [`Self::persist_settings_if_changed`] a no-op.
    pub(in crate::ui) settings_store: Option<Box<dyn SettingsWriter>>,
    /// The mixer and equalizer snapshot last loaded or saved. `update` compares
    /// the state's current snapshot against it after every message, so only a
    /// message that actually changed one of those values writes the file.
    pub(in crate::ui) saved_settings: Settings,
}

/// Which browse screen is showing. Payload-free: the albums and songs the
/// view renders come from the loaded `albums`/`songs` lists, so carrying
/// the selected ids here would only duplicate state that nothing reads.
#[derive(Debug, PartialEq, Eq)]
pub(in crate::ui) enum CurrentView {
    Artists,
    Albums,
    Songs,
    AllSongs,
    AllAlbums,
    RecentlyAdded,
    RecentlyPlayed,
    Playlists,
    Genres,
}

/// Which browse list a Songs view was opened from, so `Back` can step to it.
#[derive(Debug, PartialEq, Eq)]
pub(in crate::ui) enum SongsOrigin {
    /// The per-artist Albums view: Back returns to `CurrentView::Albums`.
    Album,
    /// The flat whole-library All Albums view: Back returns to
    /// `CurrentView::AllAlbums`.
    AllAlbums,
    /// The Recently Added view: Back returns to `CurrentView::RecentlyAdded`.
    RecentlyAdded,
    /// The Playlists view: Back returns to `CurrentView::Playlists`.
    Playlist,
    /// The Genres view: Back returns to `CurrentView::Genres`.
    Genre,
}

impl WinampPlayer {
    /// The initial player over the given shared state and Apple Music service:
    /// nothing selected, no lists loaded. Both `boot` and the tests start a
    /// player this way, so the starting shape lives here instead of being
    /// repeated at each site — a new field has only one spot to get its startup
    /// value. The service is passed in rather than built here so the UI shares
    /// the instance `main` ran the startup sign-in on.
    pub(in crate::ui) fn new(state: Arc<Mutex<AppState>>, service: AppleMusicService) -> Self {
        Self {
            state,
            apple_music_service: service,
            window_id: None,
            shaded: false,
            window_position: None,
            control_held: false,
            shift_held: false,
            always_on_top: false,
            unshaded_size: None,
            current_view: CurrentView::Artists,
            artists: BrowseList::new(true),
            albums: BrowseList::new(false),
            songs: BrowseList::new(false),
            all_songs: BrowseList::new(false),
            all_albums: BrowseList::new(false),
            recently_added: BrowseList::new(false),
            recently_played: BrowseList::new(false),
            playlists: BrowseList::new(false),
            genres: BrowseList::new(false),
            songs_origin: SongsOrigin::Album,
            search_query: String::new(),
            search_active: false,
            known_tracks: HashMap::new(),
            play_history: transport::PlayHistory::default(),
            shuffle_round: transport::ShuffleRound::default(),
            marquee_offset: 0,
            title_drag: None,
            playback_position: None,
            playback_format: None,
            playback_duration: None,
            blink_visible: true,
            spectrum: None,
            spectrum_peaks: None,
            visualizer_visible: true,
            stop_after_current: false,
            eq_panel_visible: true,
            library_panel_visible: true,
            about_visible: false,
            options_visible: false,
            options_selected: 0,
            file_info_visible: false,
            jump_time_visible: false,
            jump_time_query: String::new(),
            jump_file_visible: false,
            jump_file_query: String::new(),
            jump_file_selected: None,
            seek_preview: None,
            time_display: views::TimeDisplay::Elapsed,
            mixer_drag: None,
            mixer_flash_ticks: 0,
            fade_out_ticks: 0,
            mixer_flash: views::MixerDrag::Volume,
            balance_drag_raw: None,
            position_epoch: 0,
            pending_position_transitions: 0,
            plays_generation: Arc::new(AtomicU64::new(0)),
            settings_store: None,
            saved_settings: Settings::default(),
        }
    }

    /// Close every transient panel — About, Options, File info, Jump to time,
    /// and Jump to File — so that at most one is open (the invariant the
    /// `*_visible` fields document). A toggle handler that opens its panel
    /// calls this first, then sets its own flag, so the full set of panels
    /// lives here instead of being listed again in each opener.
    pub(in crate::ui) fn close_transient_panels(&mut self) {
        self.about_visible = false;
        self.options_visible = false;
        self.file_info_visible = false;
        self.jump_time_visible = false;
        self.jump_file_visible = false;
    }

    /// Whether any transient panel — About, Options, File info, Jump to time,
    /// or Jump to File — is open. The read side of the same five flags
    /// [`Self::close_transient_panels`] clears, so the subscription gate and
    /// the closer cannot drift.
    pub(in crate::ui) fn transient_panel_open(&self) -> bool {
        self.about_visible
            || self.options_visible
            || self.file_info_visible
            || self.jump_time_visible
            || self.jump_file_visible
    }

    /// Save the mixer and equalizer values when the current state differs from
    /// the last saved snapshot. `update` calls this once after every message,
    /// so a changed volume, balance, toggle, or equalizer value is written
    /// without a save call in each handler; an unchanged snapshot writes
    /// nothing, which keeps timer and backend poll messages from touching the
    /// file. The snapshot also carries the player-owned readout mode,
    /// visualizer choice, panel choices, always-on-top flag, shade mode, and window
    /// position, so
    /// a click on the LCD time, the strip's `V`, the LCD strip's `A`, or a shade
    /// toggle saves exactly one new snapshot.
    /// With no store attached (tests, and before `init_ui` attaches the
    /// production one) this returns before taking the state lock.
    pub(in crate::ui) fn persist_settings_if_changed(&mut self) {
        if self.settings_store.is_none() {
            return;
        }
        let mut snapshot = {
            let state = self.state.blocking_lock();
            Settings::from_state(&state)
        };
        snapshot.set_time_display_remaining(self.time_display == views::TimeDisplay::Remaining);
        snapshot.set_visualizer_visible(self.visualizer_visible);
        snapshot.set_eq_panel_visible(self.eq_panel_visible);
        snapshot.set_library_panel_visible(self.library_panel_visible);
        snapshot.set_always_on_top(self.always_on_top);
        snapshot.set_shaded(self.shaded);
        snapshot.set_window_position(self.window_position);
        snapshot.set_library_view(self.library_view());
        if snapshot == self.saved_settings {
            return;
        }
        if let Some(store) = self.settings_store.as_deref() {
            store.save(&snapshot);
        }
        self.saved_settings = snapshot;
    }

    /// The top-level browse list the current view belongs to, which the
    /// settings file saves. An active search counts as Artists, because Back
    /// from a search goes there; a Songs view counts as the list it was
    /// opened from.
    pub(in crate::ui) fn library_view(&self) -> LibraryView {
        if self.search_active {
            return LibraryView::Artists;
        }
        match self.current_view {
            CurrentView::Artists | CurrentView::Albums => LibraryView::Artists,
            CurrentView::AllSongs => LibraryView::AllSongs,
            CurrentView::AllAlbums => LibraryView::AllAlbums,
            CurrentView::RecentlyAdded => LibraryView::RecentlyAdded,
            CurrentView::RecentlyPlayed => LibraryView::RecentlyPlayed,
            CurrentView::Playlists => LibraryView::Playlists,
            // Part 2/2 gives Genres its own saved view.
            CurrentView::Genres => LibraryView::Artists,
            CurrentView::Songs => match self.songs_origin {
                SongsOrigin::Album => LibraryView::Artists,
                SongsOrigin::AllAlbums => LibraryView::AllAlbums,
                SongsOrigin::RecentlyAdded => LibraryView::RecentlyAdded,
                SongsOrigin::Playlist => LibraryView::Playlists,
                SongsOrigin::Genre => LibraryView::Artists,
            },
        }
    }

    /// Adopt the snapshot `main` loaded before the first view: restore the
    /// player-owned readout mode, visualizer choice, panel choices,
    /// always-on-top flag, shade mode, and window position and remember the snapshot so the
    /// update loop's first comparison does not re-save it. The state-backed
    /// values were already applied to `AppState` by `main`, so this touches
    /// only those seven UI-owned values. The saved browse list is left alone:
    /// `handlers::browse::restore_view` opens it as a task.
    pub(in crate::ui) fn adopt_saved_settings(&mut self, saved: Settings) {
        self.time_display = if saved.time_display_remaining() {
            views::TimeDisplay::Remaining
        } else {
            views::TimeDisplay::Elapsed
        };
        self.visualizer_visible = saved.visualizer_visible();
        self.eq_panel_visible = saved.eq_panel_visible();
        self.library_panel_visible = saved.library_panel_visible();
        self.always_on_top = saved.always_on_top();
        self.shaded = saved.shaded();
        self.window_position = saved.window_position();
        self.saved_settings = saved;
    }

    /// Whether the LCD timer must run: a mixer flash or a fade-out is counting
    /// down, or the title has to scroll ([`Self::needs_marquee`]).
    pub(in crate::ui) fn needs_lcd_tick(&self) -> bool {
        self.mixer_flash_ticks > 0 || self.fade_out_ticks > 0 || self.needs_marquee()
    }

    /// Whether the Now Playing line is too long for the LCD's character
    /// budget and so has to scroll: the playlist number prefix plus the
    /// combined artist and title exceed `views::MARQUEE_WIDTH` characters.
    ///
    /// The marquee timer is subscribed only while this holds (see
    /// [`super::subscriptions::subscriptions`]), so the bar is not rebuilt four times a second for a
    /// line that fits whole. The label, artist, and number are resolved exactly
    /// as `view` resolves them, so the timer scrolls precisely the lines the
    /// bar windows.
    pub(in crate::ui) fn needs_marquee(&self) -> bool {
        if self.mixer_drag.is_some() || self.seek_preview.is_some() || self.title_drag.is_some() {
            return false;
        }
        let state = self.state.blocking_lock();
        let label = self.now_playing_label(state.current_track.as_deref());
        let artist = self.now_playing_artist(state.current_track.as_deref());
        let number = self.now_playing_number(state.current_track.as_deref());
        views::needs_marquee(&label, &artist, &number)
    }

    /// Whether the position poll is needed: playback is running, the audio
    /// backend still holds a preview (paused, even before its first position
    /// reply, so a seek can still move it), or a stored position is still shown
    /// because Pause kept it. A stopped player's view clears the current
    /// track's time readout (except the `--:--` placeholder), so the poll stays
    /// unsubscribed and the UI does not rebuild the whole visible screen, the
    /// library panel included while it shows, twice a second to re-render an
    /// unchanged readout (see [`super::subscriptions::subscriptions`]).
    pub(in crate::ui) fn needs_position_poll(&self) -> bool {
        if self.playback_position.is_some() || self.apple_music_service.has_loaded_preview() {
            return true;
        }
        self.state.blocking_lock().is_playing
    }

    /// Whether the spectrum poll is needed: only while playback is running.
    /// Pause and Stop drop the timer, so the bars freeze at their last levels
    /// instead of repainting the whole screen four times a second for an
    /// unchanged strip (see [`super::subscriptions::subscriptions`]).
    pub(in crate::ui) fn needs_spectrum_poll(&self) -> bool {
        self.state.blocking_lock().is_playing
    }
}
