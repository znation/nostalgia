//! The UI's event vocabulary: every `Message` the iced loop routes to a
//! handler. Kept apart from the app model and the `update` dispatch table so a
//! new event and its handler can be read without the rest of the shell.

use std::time::Duration;

use crate::audio::{PlaybackFormat, Spectrum};
use crate::equalizer::Preset;
use crate::library::{Album, Artist, Playlist, Song};

/// Every event the iced loop routes to `update`: transport and playback
/// controls, LCD and equalizer changes, browse navigation, search, sort, and
/// fetch replies, track-completion reports, and window actions. Each variant
/// carries its handler's input; the comments on individual variants name the
/// state they change.
#[derive(Debug, Clone)]
pub(in crate::ui) enum Message {
    PlayPause,
    // The explicit Play and Pause bindings (X and C, and the system media
    // Play and Pause keys). Unlike `PlayPause`
    // they are idempotent: pressing X always plays, while C pauses a loaded
    // preview and stops a track whose source supplied none.
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
    // The mouse wheel's vertical direction over the player panels: positive
    // raises the volume, negative lowers it, and zero is a no-op. The browse
    // list is deliberately outside the area that publishes it, so its own
    // scrollable keeps the wheel.
    VolumeWheel(f32),
    // Arrow-key seek nudges: one `views::SEEK_STEP_SECS` back or forward.
    SeekBackward,
    SeekForward,
    // The marquee timer's tick: advance the Now Playing title window by one
    // character. The timer runs only while the line is too long to fit the
    // LCD's character budget (see [`super::timers::subscriptions`]), so a tick always
    // scrolls the bar.
    MarqueeTick,
    // The position poll's tick: schedule queries of the loaded preview's
    // position, format, length, and finished state. Keeping the queries in
    // tasks, rather than in `update`, means a slow backend cannot stall the UI
    // thread.
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
    // [`super::timers::subscriptions`]), so a paused or stopped player keeps its last bars.
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
    // The finished-state query's reply, stamped with the `position_epoch` that
    // issued it. `true` means the loaded preview reached its end while playing,
    // so the handler advances the transport as Next would; a reply from before
    // a Stop or a new selection is dropped.
    PlaybackFinished {
        epoch: u64,
        finished: bool,
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
    // The indicator strip's V control: hides the LCD visualizer, or shows it
    // again. It flips only the LCD's display flag, changes no playback state,
    // and schedules no work; the stored spectrum and its poll keep running.
    ToggleVisualizer,
    // The player row's EQ button: shows or hides the normal-mode equalizer
    // section. It flips only the panel-visibility flag — not the EQ's on/off
    // state, curve, or preset — and schedules no work.
    ToggleEqualizerPanel,
    // The player row's PL button: shows or hides the normal-mode library
    // section (its title band, browse controls, search row, list, and footer).
    // It flips only the panel-visibility flag, changes no playback, equalizer,
    // browse, search, or selection state, and schedules no work.
    ToggleLibraryPanel,
    ToggleEqualizer,
    EqPreampChange(f32),
    EqBandChange(usize, f32),
    EqPresetSelected(Preset),
    NextTrack,
    PreviousTrack,
    // A browse row's single press: select that row (the view paints the
    // selection bar) without opening it. A second press on the row that is
    // already selected opens it, so a double-click reaches the existing
    // artist/album/track/playlist action on its second press. The epoch and
    // index travel like the selection messages below, so a press that
    // outlived its list cannot select or open the wrong row.
    SelectRow {
        epoch: u64,
        index: usize,
    },
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
    // The browse view's Open button: open the row selected in the showing
    // list through the same message a second press dispatches. With nothing
    // selected it is a no-op and schedules no task.
    OpenSelected,
    Back,
    // The browse view's Top button: snap the current list back to its first
    // row. The arm schedules an iced scroll operation on the browse list's
    // stable widget id and changes no state.
    ScrollBrowseToTop,
    // The End key's counterpart to the Top button: snap the current list to
    // its last row. The arm schedules an iced scroll operation on the browse
    // list's stable widget id and changes no state.
    ScrollBrowseToBottom,
    // The Page Up key: scroll the showing browse list back by one page.
    // Like the Top and End keys it schedules only a one-page `scroll_by` on
    // the browse list's stable widget id and changes no state.
    ScrollBrowsePageUp,
    // The Page Down key: scroll the showing browse list forward by one page.
    // Like the Top and End keys it schedules only a one-page `scroll_by` on
    // the browse list's stable widget id and changes no state.
    ScrollBrowsePageDown,
    // The browse view's Sort button: reorder the showing list alphabetically
    // by its rows' rendered text. The handler bumps that list's epoch so a
    // press rendered before the sort is rejected, and leaves every other list
    // and the playback state alone.
    SortBrowse,
    // The browse view's Reload button: refetch the top-level artists list and
    // return to the Artists view, clearing an active search but not the search
    // box's typed text. It leaves playback and the other browse lists alone.
    ReloadLibrary,
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
    // query String just to have a message ready for a press. `FocusSearch`
    // moves keyboard focus to the box via its `BROWSE_SEARCH_ID` widget id,
    // changing no browse or playback state and submitting nothing.
    // `ClearSearch` is the Clear button beside the box: it empties the stored
    // query and, when a search is showing, leaves it exactly as Back does;
    // it schedules no fetch and touches no playback.
    SearchChanged(String),
    SearchSubmitted,
    FocusSearch,
    ClearSearch,
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
    // drop a still-pending selection's title. `committed` is whether the play
    // itself succeeded: shared state's flags cannot stand in, because a Pause
    // or Stop processed after the commit but before this message clears them.
    TrackPlayed {
        generation: u64,
        committed: bool,
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
