//! The iced application shell: the `WinampPlayer` app model, its `Message`
//! event vocabulary, and the `update`/`view` loop `init_ui` hands to iced.
//! The model lives in `player`, the events in `message`, and the root screen
//! assembly in `view`; widget construction lives in the `views` submodule
//! (browse lists, the title bar, Now Playing bar, seek strip, transport
//! controls, and equalizer panel), the Previous/Next stepping arithmetic in
//! `transport`, the library-fetch and error-reporting adapter in `loading`,
//! the per-level browse state machine in `browse`, the classic key bindings in
//! `shortcuts`, and the iced subscription set — keyboard and mixer listeners
//! plus the marquee, position-poll, and spectrum-poll timers — in
//! `subscriptions`; `update` is a dispatch table over the
//! per-concern message handlers in `handlers`, and this module wires those to
//! the shared `AppState` and the `AppleMusicService` seam. Its unit tests live
//! in the `tests` submodule.

use iced::Task;
use std::sync::Arc;
use tokio::sync::Mutex;

mod bevel;
mod browse;
mod composers;
mod genres;
mod handlers;
#[cfg(test)]
pub(crate) mod layout_capture;
mod loading;
mod message;
mod player;
mod shortcuts;
mod song_groups;
mod style;
mod subscriptions;
mod theme;
mod transport;
mod view;
mod views;
mod years;

pub(in crate::ui) use message::{Message, OptionsChoice, OptionsRow};
pub(in crate::ui) use player::{CurrentView, SAVED_PLAYLIST_ROW_ID, SongsOrigin, WinampPlayer};
pub(in crate::ui) use view::view;

use subscriptions::subscriptions;

use crate::settings::Settings;
use crate::settings::file_store::FileStore;
use crate::settings::writer::BackgroundWriter;
use crate::{apple_music::AppleMusicService, state::AppState};

/// Runs the UI, blocking until the window is closed.
///
/// `service` is the [`AppleMusicService`] `main` built and ran the startup sign-in on; the UI
/// stores a clone, so the session that sign-in stores is the session the UI later reads. `store`
/// and `saved` are the settings file's store and the snapshot `main` already restored into `state`;
/// the player keeps both so `update` can save every later change, with the blocking write on the
/// `BackgroundWriter` worker rather than the update thread, and `adopt_saved_settings` also
/// restores the player-owned elapsed/remaining readout mode from the snapshot. The iced window's
/// title follows the current track and its paused/stopped state: `WinampPlayer::window_title` names
/// it, falling back to `nostalgia` when none is known. `main` must call this from a plain
/// (non-async) context: iced drives its event loop synchronously on the calling thread, and
/// `update`/`view` use `blocking_lock` on the shared state, which panics inside a runtime.
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
    let position = handlers::window::launch_position(saved.window_position());
    iced::application(
        move || {
            let (mut player, task) = boot(state.clone(), service.clone());
            player.settings_store = Some(Box::new(BackgroundWriter::new(store.clone())));
            player.adopt_saved_settings(saved.clone());
            let restore = handlers::browse::lists::restore_view(&mut player, saved.library_view());
            (player, Task::batch([task, restore]))
        },
        update,
        view,
    )
    .title(|player: &WinampPlayer| player.window_title())
    .theme(|_: &WinampPlayer| theme::winamp_theme())
    // The vendored pixel faces the chrome's text will render in: the bytes
    // load at startup, so `Font::with_name` addresses them app-wide. The
    // letterform face is also the app-wide default, so no future text falls
    // back to the system face — the pixel-font split's parts 1–3 gave the
    // chrome, the browse rows and the equalizer explicit faces, so nothing
    // else relied on the old default when this landed.
    .default_font(theme::ui_font())
    .font(std::borrow::Cow::Borrowed(
        include_bytes!("../../assets/fonts/Silkscreen-Regular.ttf").as_slice(),
    ))
    .font(std::borrow::Cow::Borrowed(
        include_bytes!("../../assets/fonts/Tiny5.ttf").as_slice(),
    ))
    .decorations(false)
    .resizable(false)
    .position(position)
    .subscription(|player| {
        iced::Subscription::batch(subscriptions(
            player.needs_lcd_tick(),
            player.needs_position_poll(),
            player.needs_spectrum_poll(),
            player.mixer_drag.is_some(),
            player.jump_file_visible,
            player.options_visible,
            player.transient_panel_open(),
            player.title_drag.is_some(),
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
            // The last-played track the restored snapshot left in `current_track`
            // is named, not just shown: resolving it records its title, artist,
            // and rest into `known_tracks` so the bar and the window title name
            // it instead of the raw id. The offline sample library answers
            // immediately; an Apple Music id stays id-named until sign-in.
            Task::done(Message::ResolveCurrentTrack),
            // The signed-out Recently Played list the restored snapshot
            // carries in `recently_played=` is named back the same way: the
            // saved ids resolve into songs and record into the played list,
            // so the view reopens with them instead of the sample stand-in.
            // Runs after the init closure's `adopt_saved_settings` seeded the
            // player, exactly like the last-played resolve, and is a no-op
            // with a stored session, which fetches the service's own list.
            Task::done(Message::ResolveRecentlyPlayed),
            // The startup sign-in runs on a background thread, so the fetch
            // above can land before the session exists and answer from the
            // sample library. Refetch when a session arrives, replacing
            // the sample artists and any flat list with the signed-in library.
            Task::perform(
                async move { sign_in_service.wait_for_session().await },
                |()| Message::SessionReady,
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
/// the browse row presses in `handlers::browse::selection`, the Artist-view
/// lists and Back in `handlers::browse::lists`, the row-order edits in
/// `handlers::browse::ordering`, the library search with its Clear button in
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
        Message::FadeOutAndStop => handlers::playback::fade::fade_out_and_stop(player),
        Message::ToggleStopAfterCurrent => handlers::playback::toggle_stop_after_current(player),
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
        Message::ToggleEqualizerPanel => handlers::window::toggle_equalizer_panel(player),
        Message::ToggleLibraryPanel => handlers::window::toggle_library_panel(player),
        Message::ToggleEqualizer => handlers::mixer::toggle_equalizer(player),
        Message::ToggleEqAuto => handlers::mixer::toggle_eq_auto(player),
        Message::EqPreampChange(gain) => handlers::mixer::eq_preamp_change(player, gain),
        Message::EqBandChange(band, gain) => handlers::mixer::eq_band_change(player, band, gain),
        Message::EqBandsSetAll(gain) => handlers::mixer::eq_bands_set_all(player, gain),
        Message::EqPresetSelected(preset) => handlers::mixer::eq_preset_selected(player, preset),
        Message::NextTrack => handlers::playback::stepping::step_track(player, true),
        Message::PreviousTrack => handlers::playback::stepping::step_track(player, false),
        Message::NextButton => handlers::playback::stepping::next_button(player),
        Message::PreviousButton => handlers::playback::stepping::previous_button(player),
        Message::StopButton => handlers::playback::fade::stop_button(player),
        Message::ModifiersHeld { control, shift } => {
            player.control_held = control;
            player.shift_held = shift;
            Task::none()
        }
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
        Message::TitleDragPressed => handlers::playback::title_drag::title_drag_pressed(player),
        Message::TitleDragMoved(x) => handlers::playback::title_drag::title_drag_moved(player, x),
        Message::TitleDragReleased => handlers::playback::title_drag::title_drag_released(player),
        Message::ToggleTimeDisplay => handlers::playback::toggle_time_display(player),
        Message::ToggleVisualizer => handlers::playback::toggle_visualizer(player),
        Message::ToggleAutoscroll => handlers::playback::toggle_autoscroll(player),
        Message::TrackSelected { epoch, index } => {
            handlers::playback::track_selected(player, epoch, index)
        }
        Message::SelectRow { epoch, index } => {
            handlers::browse::selection::select_row(player, epoch, index)
        }
        Message::ArtistSelected { epoch, index } => {
            handlers::browse::selection::artist_selected(player, epoch, index)
        }
        Message::AlbumSelected { epoch, index } => {
            handlers::browse::selection::album_selected(player, epoch, index)
        }
        Message::PlaylistSelected { epoch, index } => {
            handlers::browse::selection::playlist_selected(player, epoch, index)
        }
        // (the saved-list sentinel rows route inside `playlist_selected`)
        Message::OpenSelected => handlers::browse::selection::open_selected(player),
        Message::Back => handlers::browse::lists::back(player),
        Message::ScrollBrowseToTop => handlers::browse::scroll::scroll_browse_to_top(),
        Message::ScrollBrowseToBottom => handlers::browse::scroll::scroll_browse_to_bottom(),
        Message::ScrollBrowsePageUp => handlers::browse::scroll::scroll_browse_page_up(),
        Message::ScrollBrowsePageDown => handlers::browse::scroll::scroll_browse_page_down(),
        Message::SortBrowse => handlers::browse::ordering::sort_browse(player),
        Message::ReverseBrowse => handlers::browse::ordering::reverse_browse(player),
        Message::RemoveAllBrowse => handlers::browse::ordering::remove_all(player),
        Message::RandomizeBrowse => handlers::browse::ordering::randomize_browse(player),
        Message::RemoveSelectedBrowse => handlers::browse::ordering::remove_selected(player),
        Message::MoveSelectedRowUp => handlers::browse::ordering::move_selected_row(player, false),
        Message::MoveSelectedRowDown => handlers::browse::ordering::move_selected_row(player, true),
        Message::MoveSelectedRowToTop => {
            handlers::browse::ordering::move_selected_row_to_edge(player, false)
        }
        Message::MoveSelectedRowToBottom => {
            handlers::browse::ordering::move_selected_row_to_edge(player, true)
        }
        Message::MoveBrowseSelectionUp => {
            handlers::browse::selection::move_browse_selection(player, false)
        }
        Message::MoveBrowseSelectionDown => {
            handlers::browse::selection::move_browse_selection(player, true)
        }
        Message::ReloadLibrary => handlers::browse::lists::reload_library(player),
        Message::ShowAllSongs => handlers::browse::lists::show_all_songs(player),
        Message::ShowAllAlbums => handlers::browse::lists::show_all_albums(player),
        Message::ShowRecentlyAdded => handlers::browse::lists::show_recently_added(player),
        Message::ShowRecentlyPlayed => handlers::browse::lists::show_recently_played(player),
        Message::ShowPlaylists => handlers::browse::lists::show_playlists(player),
        Message::ShowGenres => handlers::browse::lists::show_genres(player),
        Message::GenreSelected { epoch, index } => {
            handlers::browse::selection::genre_selected(player, epoch, index)
        }
        Message::GenresLoaded(genres) => handlers::browse::fetch::genres_loaded(player, genres),
        Message::GenresLoadFailed(report) => {
            handlers::browse::fetch::genres_load_failed(player, report)
        }
        Message::ShowYears => handlers::browse::lists::show_years(player),
        Message::YearSelected { epoch, index } => {
            handlers::browse::selection::year_selected(player, epoch, index)
        }
        Message::YearsLoaded(years) => handlers::browse::fetch::years_loaded(player, years),
        Message::YearsLoadFailed(report) => {
            handlers::browse::fetch::years_load_failed(player, report)
        }
        Message::ShowComposers => handlers::browse::lists::show_composers(player),
        Message::ComposerSelected { epoch, index } => {
            handlers::browse::selection::composer_selected(player, epoch, index)
        }
        Message::ComposersLoaded(composers) => {
            handlers::browse::fetch::composers_loaded(player, composers)
        }
        Message::ComposersLoadFailed(report) => {
            handlers::browse::fetch::composers_load_failed(player, report)
        }
        Message::SearchChanged(text) => handlers::browse::search::search_changed(player, text),
        Message::SearchSubmitted => handlers::browse::search::search_submitted(player),
        Message::FocusSearch => handlers::browse::search::focus_search(),
        Message::ClearSearch => handlers::browse::search::clear_search(player),
        Message::LoadArtists => handlers::browse::fetch::load_artists(player),
        Message::SessionReady => handlers::browse::lists::session_ready(player),
        Message::ResolveCurrentTrack => handlers::playback::resolve::resolve_current_track(player),
        Message::CurrentTrackResolved { id, song } => {
            handlers::playback::resolve::current_track_resolved(player, id, song)
        }
        Message::ResolveRecentlyPlayed => {
            handlers::playback::resolve::resolve_recently_played(player)
        }
        Message::RecentlyPlayedResolved { songs } => {
            handlers::playback::resolve::recently_played_resolved(player, songs)
        }
        Message::ArtistsLoaded(artists) => handlers::browse::fetch::artists_loaded(player, artists),
        Message::AlbumsLoaded(albums) => handlers::browse::fetch::albums_loaded(player, albums),
        Message::SongsLoaded(songs) => handlers::browse::fetch::songs_loaded(player, songs),
        // A resolved saved song list is wrapped in `BrowseReply` and
        // re-checked like every fetch reply before its `update` call.
        Message::AllSongsLoaded(songs) => handlers::browse::fetch::all_songs_loaded(player, songs),
        Message::AllAlbumsLoaded(albums) => {
            handlers::browse::fetch::all_albums_loaded(player, albums)
        }
        Message::RecentlyAddedLoaded(albums) => {
            handlers::browse::fetch::recently_added_loaded(player, albums)
        }
        Message::RecentlyPlayedLoaded(songs) => {
            handlers::browse::fetch::recently_played_loaded(player, songs)
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
        Message::RecentlyAddedLoadFailed(report) => {
            handlers::browse::fetch::recently_added_load_failed(player, report)
        }
        Message::RecentlyPlayedLoadFailed(report) => {
            handlers::browse::fetch::recently_played_load_failed(player, report)
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
        Message::ReplayResolved { track_id, song } => {
            handlers::playback::resolve::replay_resolved(player, &track_id, song)
        }
        Message::WindowIdResolved(id) => handlers::window::window_id_resolved(player, id),
        Message::WindowDragged => handlers::window::window_dragged(player),
        Message::MinimizeWindow => handlers::window::minimize_window(player),
        Message::CloseWindow => handlers::window::close_window(player),
        Message::ToggleWindowShade => handlers::window::toggle_window_shade(player),
        Message::WindowShadeMeasured(size) => handlers::window::window_shade_measured(player, size),
        Message::WindowMoved(point) => handlers::window::window_moved(player, point),
        Message::ToggleAbout => handlers::window::toggle_about(player),
        Message::ToggleOptions => handlers::window::toggle_options(player),
        Message::OptionsSelectionUp => handlers::window::options_selection_up(player),
        Message::OptionsSelectionDown => handlers::window::options_selection_down(player),
        Message::OptionsActivate => handlers::window::options_activate(player),
        Message::OptionsChoice(choice) => handlers::window::options_choice(player, choice),
        Message::EscapePressed => handlers::window::escape_pressed(player),
        Message::ToggleFileInfo => handlers::window::toggle_file_info(player),
        Message::ToggleJumpTime => handlers::jump::toggle_jump_time(player),
        Message::JumpTimeChanged(text) => handlers::jump::jump_time_changed(player, text),
        Message::JumpTimeSubmitted => handlers::jump::jump_time_submitted(player),
        Message::ToggleSaveList => handlers::save_list::toggle_save_list(player),
        Message::SaveListNameChanged(text) => {
            handlers::save_list::save_list_name_changed(player, text)
        }
        Message::SaveListSubmitted => handlers::save_list::save_list_submitted(player),
        Message::DeleteListSubmitted => handlers::save_list::delete_list_submitted(player),
        Message::UserPlaylistLoaded(songs) => {
            handlers::save_list::user_playlist_loaded(player, songs)
        }
        Message::ToggleSavePreset => handlers::mixer::toggle_save_preset(player),
        Message::SavePresetNameChanged(text) => {
            handlers::mixer::save_preset_name_changed(player, text)
        }
        Message::SavePresetSubmitted => handlers::mixer::save_preset_submitted(player),
        Message::DeletePresetSubmitted => handlers::mixer::delete_preset_submitted(player),
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
