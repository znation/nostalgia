//! The iced application: the `WinampPlayer` app struct, its `Message` event
//! type, and the `update`/`view` loop `init_ui` hands to iced. Widget
//! construction lives in the `views` submodule (browse lists, Now Playing bar,
//! transport controls, equalizer panel), the Previous/Next stepping arithmetic
//! in `transport`, and the library-fetch/error-reporting adapter in `loading`;
//! this module wires those to the shared `AppState` and the `AppleMusicService`
//! seam. Its unit tests live in the `tests` submodule.

use iced::{Element, Task, widget::Column};
use std::{borrow::Cow, collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

mod loading;
mod style;
mod theme;
mod transport;
mod views;

use crate::{
    apple_music::AppleMusicService,
    library::{Album, Artist, Song},
    state::AppState,
};
use loading::{fetch_into, play_failure_report, played_or_reported};

/// Runs the UI, blocking until the window is closed.
///
/// `main` must call this from a plain (non-async) context: iced drives its
/// event loop synchronously on the calling thread, and `update`/`view` use
/// `blocking_lock` on the shared state, which panics inside a runtime.
pub fn init_ui(state: Arc<Mutex<AppState>>) -> iced::Result {
    iced::application(move || boot(state.clone()), update, view)
        .title("nostalgia")
        .theme(|_: &WinampPlayer| theme::winamp_theme())
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    PlayPause,
    Stop,
    ToggleRepeat,
    VolumeChange(f32),
    ToggleEqualizer,
    EqPreampChange(f32),
    EqBandChange(usize, f32),
    NextTrack,
    PreviousTrack,
    TrackSelected(String),
    ArtistSelected(String),
    AlbumSelected(String),
    Back,
    LoadArtists,
    ArtistsLoaded(Vec<Artist>),
    AlbumsLoaded(Vec<Album>),
    SongsLoaded(Vec<Song>),
    TrackPlayed,
}

struct WinampPlayer {
    state: Arc<Mutex<AppState>>,
    apple_music_service: AppleMusicService,
    current_view: CurrentView,
    artists: Vec<Artist>,
    albums: Vec<Album>,
    songs: Vec<Song>,
    /// Each played song's id mapped to its title, so the Now Playing bar can
    /// still name the playing track after the user browses to a different
    /// album (whose list replaces `songs`). The `TrackSelected` arm records a
    /// track's title once, when it is played, and `now_playing_label`
    /// resolves the per-frame bar label with a single get instead of scanning
    /// a growing list on every frame. Recording on play — rather than folding
    /// every song of every browsed album into the map — keeps the map
    /// proportional to songs actually played and takes the per-album fold off
    /// the browse path.
    known_titles: HashMap<String, String>,
}

/// Which browse screen is showing. Payload-free: the albums and songs the
/// view renders come from the loaded `albums`/`songs` buffers, so carrying
/// the selected ids here (as earlier versions did) only duplicated
/// state nothing read.
#[derive(Debug, PartialEq, Eq)]
enum CurrentView {
    Artists,
    Albums,
    Songs,
}

/// The initial player over the given shared state: nothing selected, no
/// lists loaded. Both `boot` and the tests start a player this way, so the
/// starting shape lives here instead of being repeated at each site — a new
/// field has only one spot to get its startup value.
impl WinampPlayer {
    fn new(state: Arc<Mutex<AppState>>) -> Self {
        let service = AppleMusicService::new(state.clone());
        Self {
            state,
            apple_music_service: service,
            current_view: CurrentView::Artists,
            artists: Vec::new(),
            albums: Vec::new(),
            songs: Vec::new(),
            known_titles: HashMap::new(),
        }
    }

    /// The Now Playing bar's label for `current_track`, resolved against the
    /// accumulated [`Self::known_titles`] index rather than the
    /// currently-browsed album's `songs`: `view` renders the bar from this, so
    /// the bar keeps naming a playing track even after a browse to another
    /// album replaced `songs`. The browse-away regression test asserts this
    /// same path.
    fn now_playing_label<'a>(&'a self, current_track: Option<&str>) -> Cow<'a, str> {
        views::now_playing_label(&self.known_titles, current_track)
    }
}

fn boot(state: Arc<Mutex<AppState>>) -> (WinampPlayer, Task<Message>) {
    (WinampPlayer::new(state), Task::done(Message::LoadArtists))
}

/// Stores a freshly fetched list into the player's matching buffer, with no
/// further work. The `ArtistsLoaded`, `AlbumsLoaded`, and `SongsLoaded`
/// update arms all just store. The store-and-noop shape lives here once
/// instead of in each arm.
fn store_loaded<T>(buffer: &mut Vec<T>, items: Vec<T>) -> Task<Message> {
    *buffer = items;
    Task::none()
}

/// The task the Next/Previous buttons schedule: step the current track
/// through the player's loaded songs in the given direction — via the
/// `transport::next_track_id`/`previous_track_id` stepping function passed
/// in — and schedule the landed song as `TrackSelected`, or no task when
/// there is nothing to step through. Both transport update arms used to
/// repeat this lock-then-dispatch block; the direction comes in as a function
/// so the stepping arithmetic stays in `transport` and the wiring lives here
/// once. The shared Repeat flag is read under the same lock and passed
/// through to the stepping function, so the wrap-vs-stop-at-the-edge decision
/// (see `transport::next_track_id`) stays in `transport` too.
fn step_track(
    player: &WinampPlayer,
    step: fn(&[Song], Option<&str>, bool) -> Option<String>,
) -> Task<Message> {
    // The stepped song is computed under the state lock, borrowing the
    // current track directly. The earlier version cloned the `current_track`
    // `String` only to borrow it via `as_deref` — the same clone-to-borrow
    // the per-frame now-playing path used to do — so the lock now covers the
    // pure stepping scan (fast, and `step` never locks anything itself).
    let state = player.state.blocking_lock();
    match step(&player.songs, state.current_track.as_deref(), state.repeat) {
        Some(track_id) => Task::done(Message::TrackSelected(track_id)),
        None => Task::none(),
    }
}

/// Locks the shared playback state, applies `mutation` to it, and returns no
/// task. The Play/Pause, Stop, ToggleRepeat, and VolumeChange arms all repeat
/// the same synchronous shared-state update — `blocking_lock`, one mutation,
/// then `Task::none()` — so the lock-and-noop shape lives here once and each
/// arm only names its mutation. Asynchronous work (fetches) goes through
/// [`fetch_into`] instead.
fn mutate_state(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    let mut state = player.state.blocking_lock();
    mutation(&mut state);
    Task::none()
}

fn update(player: &mut WinampPlayer, message: Message) -> Task<Message> {
    match message {
        Message::PlayPause => mutate_state(player, AppState::toggle_playing),
        // Stop is synchronous, exactly like PlayPause: the stub has no
        // playback position yet, so its only observable effect is the cleared
        // playing flag — identical to Pause today. The seam is the
        // distinction: once real playback lands, Stop also resets the track
        // position while Pause keeps it.
        Message::Stop => mutate_state(player, AppState::stop),
        Message::ToggleRepeat => mutate_state(player, AppState::toggle_repeat),
        Message::VolumeChange(volume) => mutate_state(player, |state| state.set_volume(volume)),
        // The equalizer mutators all store clamped gains in shared state, so
        // the sliders can never write an out-of-range value; the clamp itself
        // lives in `equalizer::clamp_gain`, called by the `AppState` setters.
        Message::ToggleEqualizer => mutate_state(player, AppState::toggle_equalizer),
        Message::EqPreampChange(gain) => mutate_state(player, |state| state.set_eq_preamp(gain)),
        Message::EqBandChange(band, gain) => {
            mutate_state(player, |state| state.set_eq_band(band, gain))
        }
        Message::NextTrack => step_track(player, transport::next_track_id),
        Message::PreviousTrack => step_track(player, transport::previous_track_id),
        Message::TrackSelected(track_id) => {
            // Record the played track's title before handing the id to the
            // async play: the Now Playing bar resolves its label from
            // `known_titles`, and the entry must survive a later browse to a
            // different album (which replaces `songs`). Recording once per
            // play — rather than folding every song of every browsed album
            // into the map — keeps the index proportional to songs actually
            // played and takes the fold off the browse path. The id comes
            // from a row of the currently loaded `songs`, so the lookup hits;
            // the `if let` only guards a stale id defensively.
            if let Some(song) = player.songs.iter().find(|song| song.id == track_id) {
                player
                    .known_titles
                    .entry(track_id.clone())
                    .or_insert_with(|| song.title.clone());
            }
            let service = player.apple_music_service.clone();
            let id_for_report = track_id.clone();
            Task::perform(
                async move { service.play_track(&track_id).await },
                move |result| {
                    played_or_reported(result, |err| {
                        eprintln!("{}", play_failure_report(&id_for_report, err))
                    })
                },
            )
        }
        Message::ArtistSelected(artist_id) => {
            player.current_view = CurrentView::Albums;
            fetch_into(
                &player.apple_music_service,
                format!("loading albums for artist {artist_id:?}"),
                move |service| async move { service.get_albums_by_artist(&artist_id).await },
                Message::AlbumsLoaded,
            )
        }
        Message::AlbumSelected(album_id) => {
            player.current_view = CurrentView::Songs;
            fetch_into(
                &player.apple_music_service,
                format!("loading songs from album {album_id:?}"),
                move |service| async move { service.get_songs_from_album(&album_id).await },
                Message::SongsLoaded,
            )
        }
        // The browse hierarchy is navigable both ways: ArtistSelected and
        // AlbumSelected step down (Artists → Albums → Songs), Back steps up
        // again. The Back button is only rendered below the artist list, so
        // this arm mostly fires where a level exists to leave; the explicit
        // top-level arm keeps the no-op in the same place as the steps.
        Message::Back => {
            player.current_view = match &player.current_view {
                CurrentView::Songs => CurrentView::Albums,
                CurrentView::Albums => CurrentView::Artists,
                CurrentView::Artists => CurrentView::Artists,
            };
            Task::none()
        }
        Message::LoadArtists => fetch_into(
            &player.apple_music_service,
            "loading favorite artists".to_string(),
            |service| async move { service.get_favorite_artists().await },
            Message::ArtistsLoaded,
        ),
        Message::ArtistsLoaded(artists) => store_loaded(&mut player.artists, artists),
        Message::AlbumsLoaded(albums) => store_loaded(&mut player.albums, albums),
        Message::SongsLoaded(songs) => store_loaded(&mut player.songs, songs),
        Message::TrackPlayed => Task::none(),
    }
}

/// Assembles the app screen: the Now Playing bar, transport row, and
/// equalizer panel — all built in `views.rs` from the resolved title and the
/// shared playback, volume, Repeat, and EQ state — above the current browse
/// list.
fn view(player: &WinampPlayer) -> Element<'_, Message> {
    // The now-playing title is resolved while the state lock is held, from a
    // borrowed `current_track` against the accumulated `known_titles` index
    // (see [`WinampPlayer::now_playing_label`]) — the label borrows the title
    // from `known_titles` (or the `"Nothing"` literal), so the per-frame path
    // allocates only in the unknown-id fallback, not the common cases.
    let (now_playing, is_playing, volume, repeat, eq_enabled, eq_preamp, eq_bands) = {
        let state = player.state.blocking_lock();
        (
            player.now_playing_label(state.current_track.as_deref()),
            state.is_playing,
            state.volume(),
            state.repeat,
            state.eq_enabled,
            state.eq_preamp(),
            state.eq_bands(),
        )
    };

    let main_content = match &player.current_view {
        CurrentView::Artists => views::view_artists(&player.artists),
        CurrentView::Albums => views::view_albums(&player.albums),
        // A second short lock (like the label block above) hands the current
        // track's id to the Songs view so it can mark the playing row. The
        // borrowed id is compared inside `song_row` and the guard drops at
        // the end of this arm, so no per-frame `Option<String>` clone is
        // needed.
        CurrentView::Songs => {
            let state = player.state.blocking_lock();
            views::view_songs(&player.songs, state.current_track.as_deref())
        }
    };

    let mut column = Column::new()
        .push(views::view_now_playing(now_playing))
        .push(views::view_transport_controls(is_playing, volume, repeat))
        .push(views::view_equalizer(eq_enabled, eq_preamp, &eq_bands));
    // The Back button sits above the list it navigates and exists only where
    // the hierarchy has a level above to return to (see `views::can_go_back`).
    if views::can_go_back(&player.current_view) {
        column = column.push(views::view_back_button());
    }
    column.push(main_content).into()
}

#[cfg(test)]
mod tests;
