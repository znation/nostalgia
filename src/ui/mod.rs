//! The iced application shell: the `WinampPlayer` app model, its `Message`
//! event vocabulary, and the `update`/`view` loop `init_ui` hands to iced.
//! The model lives in `player`, the events in `message`, and the root screen
//! assembly in `view`; widget construction lives in the `views` submodule
//! (browse lists, the title bar, Now Playing bar, seek strip, transport
//! controls, and equalizer panel), the Previous/Next stepping arithmetic in
//! `transport`, the library-fetch and error-reporting adapter in `loading`,
//! the per-level browse state machine in `browse`, the classic key bindings in
//! `shortcuts`, and the marquee, position-poll and spectrum-poll timer
//! subscriptions in `timers`; `update` is a dispatch table over the
//! per-concern message handlers in `handlers`, and this module wires those to
//! the shared `AppState` and the `AppleMusicService` seam. Its unit tests live
//! in the `tests` submodule.

use iced::Task;
use std::sync::Arc;
use tokio::sync::Mutex;

mod bevel;
mod browse;
mod handlers;
#[cfg(test)]
pub(crate) mod layout_capture;
mod loading;
mod message;
mod player;
mod shortcuts;
mod style;
mod theme;
mod ticks;
mod timers;
mod transport;
mod view;
mod views;

pub(in crate::ui) use message::Message;
pub(in crate::ui) use player::{CurrentView, SongsOrigin, WinampPlayer};
pub(in crate::ui) use view::view;

use timers::subscriptions;

use crate::settings::Settings;
use crate::settings::file_store::FileStore;
use crate::settings::writer::BackgroundWriter;
use crate::{apple_music::AppleMusicService, state::AppState};

/// Runs the UI, blocking until the window is closed.
///
/// `service` is the [`AppleMusicService`] `main` built and ran the startup
/// sign-in on; the UI stores a clone, so the session that sign-in stores is the
/// session the UI later reads. `store` and `saved` are the settings file's
/// store and the snapshot `main` already restored into `state`; the player
/// keeps both so `update` can save every later change, with the blocking write
/// on the `BackgroundWriter` worker rather than the update thread, and
/// `adopt_saved_settings` also restores the player-owned elapsed/remaining
/// readout mode from the snapshot. The iced window's title follows the current
/// track: `WinampPlayer::window_title` names it, falling back to `nostalgia`
/// when none is known. `main` must call this
/// from a plain (non-async) context: iced drives its event loop synchronously
/// on the calling thread, and `update`/`view` use `blocking_lock` on the shared
/// state, which panics inside a runtime.
///
/// # Errors
///
/// Returns the error iced reports when the application fails to start, such
/// as a window that cannot be created.
pub fn init_ui(
    state: Arc<Mutex<AppState>>,
    service: AppleMusicService,
    store: FileStore,
    saved: Settings,
) -> iced::Result {
    iced::application(
        move || {
            let (mut player, task) = boot(state.clone(), service.clone());
            player.settings_store = Some(Box::new(BackgroundWriter::new(store.clone())));
            player.adopt_saved_settings(saved.clone());
            (player, task)
        },
        update,
        view,
    )
    .title(|player: &WinampPlayer| player.window_title())
    .theme(|_: &WinampPlayer| theme::winamp_theme())
    .decorations(false)
    .resizable(false)
    .subscription(|player| {
        iced::Subscription::batch(subscriptions(
            player.needs_marquee(),
            player.needs_position_poll(),
            player.needs_spectrum_poll(),
            player.mixer_drag.is_some(),
            player.jump_file_visible,
            player.transient_panel_open(),
        ))
    })
    .run()
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
/// transport buttons and keys, the play request and its completion, the
/// elapsed/remaining toggle, and the marquee tick in `handlers::playback`; the
/// track stepping and the edge and ten-song jumps in
/// `handlers::playback::stepping`; the seek drag and release and the backend
/// polls in `handlers::playback::preview`; the volume, balance, seek nudges, the
/// equalizer, and the equalizer panel's show/hide toggle in `handlers::mixer`;
/// the browse hierarchy, the Back, Open, Top, Sort, and Reload navigation in
/// `handlers::browse`, the library search with its Clear button in
/// `handlers::browse::search`, the list's scroll operations in
/// `handlers::browse::scroll`, and the initial load and fetch results in
/// `handlers::browse::fetch`; and the custom title bar's window
/// actions in `handlers::window`. The table carries no behavior of its own,
/// so a change to one concern stays in that concern's module.
fn update(player: &mut WinampPlayer, message: Message) -> Task<Message> {
    let task = match message {
        Message::PlayPause => handlers::playback::play_pause(player),
        Message::Play => handlers::playback::play(player),
        Message::Pause => handlers::playback::pause(player),
        Message::Stop => handlers::playback::stop(player),
        Message::ToggleRepeat => handlers::playback::toggle_repeat(player),
        Message::ToggleShuffle => handlers::playback::toggle_shuffle(player),
        Message::MixerReleased => handlers::mixer::mixer_released(player),
        Message::VolumeChange(volume) => handlers::mixer::volume_change(player, volume),
        Message::BalanceChange(balance) => handlers::mixer::balance_change(player, balance),
        Message::VolumeUp => handlers::mixer::volume_up(player),
        Message::VolumeDown => handlers::mixer::volume_down(player),
        Message::VolumeWheel(direction) => handlers::mixer::volume_wheel(player, direction),
        Message::BalanceWheel(direction) => handlers::mixer::balance_wheel(player, direction),
        Message::EqPreampWheel(direction) => handlers::mixer::eq_preamp_wheel(player, direction),
        Message::EqBandWheel(band, direction) => {
            handlers::mixer::eq_band_wheel(player, band, direction)
        }
        Message::SeekBackward => handlers::mixer::seek_backward(player),
        Message::SeekForward => handlers::mixer::seek_forward(player),
        Message::ToggleEqualizerPanel => handlers::mixer::toggle_equalizer_panel(player),
        Message::ToggleLibraryPanel => handlers::mixer::toggle_library_panel(player),
        Message::ToggleEqualizer => handlers::mixer::toggle_equalizer(player),
        Message::EqPreampChange(gain) => handlers::mixer::eq_preamp_change(player, gain),
        Message::EqBandChange(band, gain) => handlers::mixer::eq_band_change(player, band, gain),
        Message::EqBandsSetAll(gain) => handlers::mixer::eq_bands_set_all(player, gain),
        Message::EqPresetSelected(preset) => handlers::mixer::eq_preset_selected(player, preset),
        Message::NextTrack => handlers::playback::stepping::step_track(player, true),
        Message::PreviousTrack => handlers::playback::stepping::step_track(player, false),
        Message::FirstTrack => handlers::playback::stepping::jump_to_edge_track(player, false),
        Message::LastTrack => handlers::playback::stepping::jump_to_edge_track(player, true),
        Message::JumpTracksBack => handlers::playback::stepping::jump_ten_tracks(player, false),
        Message::JumpTracksForward => handlers::playback::stepping::jump_ten_tracks(player, true),
        Message::MarqueeTick => handlers::playback::marquee_tick(player),
        // The position poll asks the backend in tasks, as the service's
        // backend queries bound their own wait; `update` stays non-blocking.
        // Each reply carries the epoch it was issued under, so a transition that
        // lands first invalidates it. While a transport transition is still
        // reaching the backend, no query is issued: the backend would still
        // report the previous preview, and the epoch no longer identifies it.
        Message::PositionTick => handlers::playback::preview::position_tick(player),
        Message::PlaybackPosition { epoch, position } => {
            handlers::playback::preview::playback_position(player, epoch, position)
        }
        Message::PlaybackFormat { epoch, format } => {
            handlers::playback::preview::playback_format(player, epoch, format)
        }
        Message::PlaybackDuration { epoch, duration } => {
            handlers::playback::preview::playback_duration(player, epoch, duration)
        }
        Message::PlaybackFinished { epoch, finished } => {
            handlers::playback::preview::playback_finished(player, epoch, finished)
        }
        Message::SpectrumTick => handlers::playback::preview::spectrum_tick(player),
        Message::Spectrum { epoch, spectrum } => {
            handlers::playback::preview::spectrum(player, epoch, spectrum)
        }
        Message::SeekPreviewChanged(value) => {
            handlers::playback::preview::seek_preview_changed(player, value)
        }
        Message::SeekPreviewReleased => handlers::playback::preview::seek_preview_released(player),
        Message::ToggleTimeDisplay => handlers::playback::toggle_time_display(player),
        Message::ToggleVisualizer => handlers::playback::toggle_visualizer(player),
        Message::TrackSelected { epoch, index } => {
            handlers::playback::track_selected(player, epoch, index)
        }
        Message::SelectRow { epoch, index } => handlers::browse::select_row(player, epoch, index),
        Message::ArtistSelected { epoch, index } => {
            handlers::browse::artist_selected(player, epoch, index)
        }
        Message::AlbumSelected { epoch, index } => {
            handlers::browse::album_selected(player, epoch, index)
        }
        Message::PlaylistSelected { epoch, index } => {
            handlers::browse::playlist_selected(player, epoch, index)
        }
        Message::OpenSelected => handlers::browse::open_selected(player),
        Message::Back => handlers::browse::back(player),
        Message::ScrollBrowseToTop => handlers::browse::scroll::scroll_browse_to_top(),
        Message::ScrollBrowseToBottom => handlers::browse::scroll::scroll_browse_to_bottom(),
        Message::ScrollBrowsePageUp => handlers::browse::scroll::scroll_browse_page_up(),
        Message::ScrollBrowsePageDown => handlers::browse::scroll::scroll_browse_page_down(),
        Message::SortBrowse => handlers::browse::sort_browse(player),
        Message::ReverseBrowse => handlers::browse::reverse_browse(player),
        Message::RandomizeBrowse => handlers::browse::randomize_browse(player),
        Message::MoveSelectedRowUp => handlers::browse::move_selected_row(player, false),
        Message::MoveSelectedRowDown => handlers::browse::move_selected_row(player, true),
        Message::ReloadLibrary => handlers::browse::reload_library(player),
        Message::ShowAllSongs => handlers::browse::show_all_songs(player),
        Message::ShowAllAlbums => handlers::browse::show_all_albums(player),
        Message::ShowPlaylists => handlers::browse::show_playlists(player),
        Message::SearchChanged(text) => handlers::browse::search::search_changed(player, text),
        Message::SearchSubmitted => handlers::browse::search::search_submitted(player),
        Message::FocusSearch => handlers::browse::search::focus_search(),
        Message::ClearSearch => handlers::browse::search::clear_search(player),
        Message::LoadArtists => handlers::browse::fetch::load_artists(player),
        Message::ArtistsLoaded(artists) => handlers::browse::fetch::artists_loaded(player, artists),
        Message::AlbumsLoaded(albums) => handlers::browse::fetch::albums_loaded(player, albums),
        Message::SongsLoaded(songs) => handlers::browse::fetch::songs_loaded(player, songs),
        Message::AllSongsLoaded(songs) => handlers::browse::fetch::all_songs_loaded(player, songs),
        Message::AllAlbumsLoaded(albums) => {
            handlers::browse::fetch::all_albums_loaded(player, albums)
        }
        Message::PlaylistsLoaded(playlists) => {
            handlers::browse::fetch::playlists_loaded(player, playlists)
        }
        Message::ArtistsLoadFailed(report) => {
            handlers::browse::fetch::artists_load_failed(player, report)
        }
        Message::AlbumsLoadFailed(report) => {
            handlers::browse::fetch::albums_load_failed(player, report)
        }
        Message::SongsLoadFailed(report) => {
            handlers::browse::fetch::songs_load_failed(player, report)
        }
        Message::AllSongsLoadFailed(report) => {
            handlers::browse::fetch::all_songs_load_failed(player, report)
        }
        Message::AllAlbumsLoadFailed(report) => {
            handlers::browse::fetch::all_albums_load_failed(player, report)
        }
        Message::PlaylistsLoadFailed(report) => {
            handlers::browse::fetch::playlists_load_failed(player, report)
        }
        Message::BrowseReply { issued, reply } => {
            handlers::browse::fetch::browse_reply(player, issued, reply)
        }
        Message::TrackPlayed {
            generation,
            committed,
        } => handlers::playback::track_played(player, generation, committed),
        Message::WindowIdResolved(id) => handlers::window::window_id_resolved(player, id),
        Message::WindowDragged => handlers::window::window_dragged(player),
        Message::MinimizeWindow => handlers::window::minimize_window(player),
        Message::CloseWindow => handlers::window::close_window(player),
        Message::ToggleWindowShade => handlers::window::toggle_window_shade(player),
        Message::WindowShadeMeasured(size) => handlers::window::window_shade_measured(player, size),
        Message::ToggleAbout => handlers::window::toggle_about(player),
        Message::EscapePressed => handlers::window::escape_pressed(player),
        Message::ToggleFileInfo => handlers::window::toggle_file_info(player),
        Message::ToggleJumpTime => handlers::jump::toggle_jump_time(player),
        Message::JumpTimeChanged(text) => handlers::jump::jump_time_changed(player, text),
        Message::JumpTimeSubmitted => handlers::jump::jump_time_submitted(player),
        Message::ToggleJumpFile => handlers::jump::toggle_jump_file(player),
        Message::JumpFileChanged(text) => handlers::jump::jump_file_changed(player, text),
        Message::JumpFileSubmitted => handlers::jump::jump_file_submitted(player),
        Message::JumpFileSelectionUp => handlers::jump::jump_file_selection_up(player),
        Message::JumpFileSelectionDown => handlers::jump::jump_file_selection_down(player),
        Message::JumpFileSelectionPageUp => handlers::jump::jump_file_selection_page_up(player),
        Message::JumpFileSelectionPageDown => handlers::jump::jump_file_selection_page_down(player),
        Message::JumpFileRowSelected { epoch, index } => {
            handlers::jump::jump_file_row_selected(player, epoch, index)
        }
        Message::ToggleAlwaysOnTop => handlers::window::toggle_always_on_top(player),
        Message::Ignored => handlers::browse::fetch::ignored(player),
        Message::TransportSettled => handlers::playback::transport_settled(player),
    };
    // Every message goes through this one save point, so a settings change is
    // persisted without a save call in each handler. The snapshot comparison
    // inside the helper keeps timer and poll messages from writing.
    player.persist_settings_if_changed();
    task
}
#[cfg(test)]
mod tests;
