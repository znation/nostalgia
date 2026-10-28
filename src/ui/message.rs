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
    // The system media Play/Pause key: toggles a loaded preview. The
    // on-screen transport row uses the explicit Play and Pause variants
    // below instead, like the X and C keys.
    PlayPause,
    // The explicit Play and Pause controls (the transport row's Play and
    // Pause buttons, the X and C keys, and the system media Play and Pause
    // keys). Unlike `PlayPause`
    // they are idempotent: pressing X always plays, while C pauses a loaded
    // preview and stops a track whose source supplied none.
    Play,
    Pause,
    Stop,
    // Ctrl+V: arms or disarms "Stop after current", so the playing song's
    // natural end stops playback instead of advancing. It changes no
    // transport state itself.
    ToggleStopAfterCurrent,
    ToggleRepeat,
    // The Shuffle toggle: with it on, a forward step picks a random
    // non-current song from the loaded song list instead of the next in order.
    ToggleShuffle,
    /// A left press on the overlong scrolling LCD title: starts a hand-scroll drag.
    TitleDragPressed,
    /// The cursor's window-space x during a title drag; scrolls the title by the distance moved.
    TitleDragMoved(f32),
    /// The left button released, or the cursor left the window: ends the title drag.
    TitleDragReleased,
    VolumeChange(f32),
    /// The release of any `chrome_slider` or equalizer slider drag: the LCD shows the title again.
    MixerReleased,
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
    // The mouse wheel's vertical direction over the balance slider: positive
    // moves it right, negative left, and zero is a no-op. Moves only the
    // balance by one `views::BALANCE_STEP` and does not set `mixer_drag`.
    BalanceWheel(f32),
    // The mouse wheel's vertical direction over the equalizer's preamp slider:
    // positive raises the gain, negative lowers it, zero is a no-op. Moves only
    // the preamp by one `views::EQ_STEP` and does not set `mixer_drag`.
    EqPreampWheel(f32),
    // The mouse wheel's vertical direction over equalizer band `usize`:
    // positive raises that band's gain, negative lowers it, zero is a no-op.
    // Moves only that band by one `views::EQ_STEP` and does not set
    // `mixer_drag`; an out-of-range index is a no-op.
    EqBandWheel(usize, f32),
    // Arrow-key seek nudges: one `views::SEEK_STEP_SECS` back or forward.
    SeekBackward,
    SeekForward,
    // The marquee timer's tick: advance the Now Playing title window by one
    // character. The timer runs only while the line is too long to fit the
    // LCD's character budget (see [`super::subscriptions::subscriptions`]), so a tick always
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
    // [`super::subscriptions::subscriptions`]), so a paused or stopped player keeps its last bars.
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
    // The equalizer panel's AUTO button: flips whether a track selection
    // applies a preset matched to its genre. It changes no curve, preset, or
    // on/off value and schedules no work.
    ToggleEqAuto,
    EqPreampChange(f32),
    EqBandChange(usize, f32),
    // From a press on an equalizer dB scale label; carries that label's gain.
    EqBandsSetAll(f32),
    EqPresetSelected(Preset),
    NextTrack,
    PreviousTrack,
    // A click on the Next button: the last song while Control is held, otherwise one step forward.
    NextButton,
    // A click on the Previous button: the first song while Control is held, otherwise one step back.
    PreviousButton,
    // Whether Control is held; false too when the window loses focus.
    ControlHeld(bool),
    // Ctrl+Z: play the first song of the showing list, whatever Shuffle and Repeat say.
    FirstTrack,
    // Ctrl+B: play the last song of the showing list, whatever Shuffle and Repeat say.
    LastTrack,
    // Keypad 1: play the song ten before the current one in the showing list, clamped to its first.
    JumpTracksBack,
    // Keypad 3: play the song ten after the current one in the showing list, clamped to its last.
    JumpTracksForward,
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
    // Ctrl+R: reverse the showing list's order, one-shot like Sort. The
    // handler bumps that list's epoch and leaves every other list and the
    // playback state alone.
    ReverseBrowse,
    // Ctrl+Shift+R: shuffle the showing list's order, one-shot like Sort. The
    // handler bumps that list's epoch and leaves every other list and the
    // playback state alone.
    RandomizeBrowse,
    // Alt+Up: move the showing list's selected row one place up, one-shot like
    // Sort. The handler bumps that list's epoch, keeps the row selected, and
    // leaves every other list and the playback state alone.
    MoveSelectedRowUp,
    // Alt+Down: like `MoveSelectedRowUp`, one place down.
    MoveSelectedRowDown,
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
    ShowRecentlyAdded,
    // The Artists view's Recently Played button: fetch the recently played
    // songs and show them flat under the Recently Played view.
    ShowRecentlyPlayed,
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
    RecentlyAddedLoaded(Vec<Album>),
    RecentlyPlayedLoaded(Vec<Song>),
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
    RecentlyAddedLoadFailed(String),
    RecentlyPlayedLoadFailed(String),
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
    // The title bar's `M` button: flips the transient About panel. Opening it
    // while the window is shaded first restores the window, so the panel never
    // draws in shade mode. The flag is not persisted.
    ToggleAbout,
    // The LCD strip's `O` or the transport row's lightning button: flips the
    // transient options menu. Opening it closes every other transient panel,
    // and opening while shaded first restores the window. The flag is not
    // persisted.
    ToggleOptions,
    /// Move the options highlight toward the first row.
    OptionsSelectionUp,
    /// Move the options highlight toward the last row.
    OptionsSelectionDown,
    /// Invoke the highlighted options row.
    OptionsActivate,
    // A row of the options menu: close the menu first, then apply the one
    // choice it names through that choice's existing toggle handler. The
    // choice itself is not a persisted setting.
    OptionsChoice(OptionsChoice),
    // Alt+3 or the LCD strip's `I`: flips the transient File info panel for the
    // current track.
    // Opening it needs a current track known to the player and closes About and
    // Options; opening while shaded first restores the window. The flag is not
    // persisted.
    ToggleFileInfo,
    // Ctrl+J: flips the transient Jump to time panel. Opening it needs a
    // loaded preview with a known length, closes About, Options, and File info,
    // and while shaded first restores the window. The flag is not persisted.
    ToggleJumpTime,
    // A keystroke in the Jump to time box: stores the typed text.
    JumpTimeChanged(String),
    // Enter or the `Jump` button: seeks to the typed time and closes the panel.
    JumpTimeSubmitted,
    // Plain J: flips the transient Jump to File panel. Opening needs a showing
    // song list (Songs or All Songs), closes About, Options, File info, and Jump
    // to time, and while shaded first restores the window. The flag is not
    // persisted.
    ToggleJumpFile,
    // A keystroke in the Jump to File box: stores the typed filter text.
    JumpFileChanged(String),
    // Enter or the `Jump` button: plays the highlighted matching row and
    // closes the panel.
    JumpFileSubmitted,
    // Up while the Jump to File panel is open: moves the highlight to the
    // previous match, stopping at the first. Changes no playback state.
    JumpFileSelectionUp,
    // Down while the Jump to File panel is open: moves the highlight to the
    // next match, stopping at the last. Changes no playback state.
    JumpFileSelectionDown,
    // Page Up while the Jump to File panel is open: moves the highlight one
    // page of visible matches back, stopping at the first. Requests only the
    // match list scroll needed to show the landed row.
    JumpFileSelectionPageUp,
    // Page Down while the Jump to File panel is open: moves the highlight one
    // page of visible matches forward, stopping at the last. Requests only the
    // match list scroll needed to show the landed row.
    JumpFileSelectionPageDown,
    // A press on a Jump to File row: plays that row through `TrackSelected`
    // and closes the panel.
    JumpFileRowSelected {
        epoch: u64,
        index: usize,
    },
    // A plain or Shift-held Escape press while a transient panel is open:
    // close the showing About, Options, File info, Jump to time, or Jump to
    // File panel by clearing only those five visibility flags. No other state
    // changes and no work is scheduled; with no panel open the arm is a no-op.
    EscapePressed,
    // The LCD strip's always-on-top toggle. The arm flips the flag and
    // schedules `iced::window::set_level` for the resolved window, which is a
    // no-op until the window id resolves.
    ToggleAlwaysOnTop,
    // The window's inner size, captured just before the first shade. The
    // arm stores it and resizes the window down to the title-bar strip,
    // unless a newer toggle already left shade mode or captured the size.
    WindowShadeMeasured(iced::Size),
}

/// A row of the transient options menu, each naming the existing on/off
/// toggle its press reuses after closing the menu. These choices introduce
/// no additional settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui) enum OptionsChoice {
    /// Reuses [`Message::ToggleAlwaysOnTop`].
    AlwaysOnTop,
    /// Reuses [`Message::ToggleTimeDisplay`].
    TimeDisplay,
    /// Reuses [`Message::ToggleVisualizer`].
    Visualizer,
    /// Reuses [`Message::ToggleEqualizerPanel`].
    EqualizerPanel,
    /// Reuses [`Message::ToggleLibraryPanel`].
    LibraryPanel,
    /// Reuses [`Message::ToggleStopAfterCurrent`].
    StopAfterCurrent,
    /// Reuses [`Message::ToggleRepeat`].
    Repeat,
    /// Reuses [`Message::ToggleShuffle`].
    Shuffle,
    /// Reuses [`Message::JumpTracksBack`].
    JumpTracksBack,
    /// Reuses [`Message::JumpTracksForward`].
    JumpTracksForward,
    /// Opens the Jump to time panel when a preview of known length is loaded.
    JumpTime,
    /// Opens the Jump to File panel while a song list is showing.
    JumpFile,
}

/// Options rows in their shared drawing and keyboard order.
#[derive(Debug, Clone, Copy)]
pub(in crate::ui) enum OptionsRow {
    AlwaysOnTop,
    TimeDisplay,
    Visualizer,
    EqualizerPanel,
    LibraryPanel,
    StopAfterCurrent,
    Repeat,
    Shuffle,
    JumpTracksBack,
    JumpTracksForward,
    JumpTime,
    JumpFile,
    About,
    Close,
}

impl OptionsRow {
    pub(in crate::ui) const ALL: [Self; 14] = [
        Self::AlwaysOnTop,
        Self::TimeDisplay,
        Self::Visualizer,
        Self::EqualizerPanel,
        Self::LibraryPanel,
        Self::StopAfterCurrent,
        Self::Repeat,
        Self::Shuffle,
        Self::JumpTracksBack,
        Self::JumpTracksForward,
        Self::JumpTime,
        Self::JumpFile,
        Self::About,
        Self::Close,
    ];

    pub(in crate::ui) fn message(self) -> Message {
        match self {
            Self::AlwaysOnTop => Message::OptionsChoice(OptionsChoice::AlwaysOnTop),
            Self::TimeDisplay => Message::OptionsChoice(OptionsChoice::TimeDisplay),
            Self::Visualizer => Message::OptionsChoice(OptionsChoice::Visualizer),
            Self::EqualizerPanel => Message::OptionsChoice(OptionsChoice::EqualizerPanel),
            Self::LibraryPanel => Message::OptionsChoice(OptionsChoice::LibraryPanel),
            Self::StopAfterCurrent => Message::OptionsChoice(OptionsChoice::StopAfterCurrent),
            Self::Repeat => Message::OptionsChoice(OptionsChoice::Repeat),
            Self::Shuffle => Message::OptionsChoice(OptionsChoice::Shuffle),
            Self::JumpTracksBack => Message::OptionsChoice(OptionsChoice::JumpTracksBack),
            Self::JumpTracksForward => Message::OptionsChoice(OptionsChoice::JumpTracksForward),
            Self::JumpTime => Message::OptionsChoice(OptionsChoice::JumpTime),
            Self::JumpFile => Message::OptionsChoice(OptionsChoice::JumpFile),
            Self::About => Message::ToggleAbout,
            Self::Close => Message::ToggleOptions,
        }
    }
}

#[cfg(test)]
mod options_tests {
    use super::*;

    #[test]
    fn options_rows_map_to_mouse_messages_in_draw_order() {
        let messages = OptionsRow::ALL.map(OptionsRow::message);
        assert!(matches!(
            messages,
            [
                Message::OptionsChoice(OptionsChoice::AlwaysOnTop),
                Message::OptionsChoice(OptionsChoice::TimeDisplay),
                Message::OptionsChoice(OptionsChoice::Visualizer),
                Message::OptionsChoice(OptionsChoice::EqualizerPanel),
                Message::OptionsChoice(OptionsChoice::LibraryPanel),
                Message::OptionsChoice(OptionsChoice::StopAfterCurrent),
                Message::OptionsChoice(OptionsChoice::Repeat),
                Message::OptionsChoice(OptionsChoice::Shuffle),
                Message::OptionsChoice(OptionsChoice::JumpTracksBack),
                Message::OptionsChoice(OptionsChoice::JumpTracksForward),
                Message::OptionsChoice(OptionsChoice::JumpTime),
                Message::OptionsChoice(OptionsChoice::JumpFile),
                Message::ToggleAbout,
                Message::ToggleOptions,
            ]
        ));
    }
}
