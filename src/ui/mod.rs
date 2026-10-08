//! The iced application: the `WinampPlayer` app struct, its `Message` event
//! type, and the `update`/`view` loop `init_ui` hands to iced. Widget
//! construction lives in the `views` submodule (browse lists, Now Playing bar,
//! transport controls, equalizer panel), the Previous/Next stepping arithmetic
//! in `transport`, and the library-fetch/error-reporting adapter in `loading`;
//! this module wires those to the shared `AppState` and the `AppleMusicService`
//! seam. Its unit tests live in the `tests` submodule.

use iced::{Element, Task, widget::Column};
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
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
use loading::{RequestGeneration, fetch_into, play_failure_report, played_or_reported};

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
    Back,
    LoadArtists,
    ArtistsLoaded(Vec<Artist>),
    AlbumsLoaded(Vec<Album>),
    SongsLoaded(Vec<Song>),
    // A browse reply that a newer request for the same list has superseded.
    // `fetch_into` emits this instead of the `*Loaded` message so the stale
    // reply never reaches `store_loaded`; the arm below is a no-op.
    Ignored,
    // The play's completion carries the generation its selection issued, so
    // the arm can tell whether a newer selection has superseded it. Only the
    // latest completion may prune the title index; a superseded one must not
    // drop a still-pending selection's title.
    TrackPlayed { generation: u64 },
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
    /// the browse path. Only the current track's entry is ever read, so the
    /// latest play's completion prunes the map back to that entry (or clears
    /// it when nothing committed): without the prune the map would grow by one
    /// entry for every distinct track ever played. A completion whose play a
    /// newer selection superseded leaves the map alone — the newer selection's
    /// title is still pending, and pruning to the older committed track would
    /// drop it.
    known_titles: HashMap<String, String>,
    /// Counters naming the list that rendered a row, bumped whenever a fetched
    /// list replaces its buffer (see [`store_loaded`]). A selection message
    /// carries the epoch of the list that rendered its row, so `update` can
    /// reject a press that outlived that list instead of resolving its index
    /// against the new one. A navigation clears a buffer without touching its
    /// epoch (see [`clear_loaded`]), so the epoch a replacing list gets is
    /// bumped past the cleared list's rather than reusing it.
    artists_epoch: u64,
    albums_epoch: u64,
    songs_epoch: u64,
    /// True while a list is waiting on a reply: it starts true for the artists
    /// fetch [`boot`] schedules, and [`clear_loaded`] sets it when a
    /// navigation empties a list for a new fetch. [`store_loaded`] clears it.
    /// The view reads it to show "Loading…" instead of the list's empty
    /// wording (see `views::browse_placeholder`).
    artists_loading: bool,
    albums_loading: bool,
    songs_loading: bool,
    /// Per-list browse-request counters. Each list's counter is bumped when a
    /// request for that list is issued, so a reply that completes after a
    /// newer request for the same list is recognized as stale (see
    /// [`loading::RequestGeneration`]) and does not overwrite the newer list.
    /// The counters are shared (`Arc`) with the fetch tasks, which complete on
    /// iced's executor while this player's UI thread issues later requests.
    artists_generation: Arc<AtomicU64>,
    albums_generation: Arc<AtomicU64>,
    songs_generation: Arc<AtomicU64>,
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
/// view renders come from the loaded `albums`/`songs` buffers, so carrying
/// the selected ids here (as earlier versions did) only duplicated
/// state that nothing read.
#[derive(Debug, PartialEq, Eq)]
enum CurrentView {
    Artists,
    Albums,
    Songs,
}

impl WinampPlayer {
    /// The initial player over the given shared state: nothing selected, no
    /// lists loaded. Both `boot` and the tests start a player this way, so the
    /// starting shape lives here instead of being repeated at each site — a new
    /// field has only one spot to get its startup value.
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
            artists_epoch: 0,
            albums_epoch: 0,
            songs_epoch: 0,
            artists_loading: true,
            albums_loading: false,
            songs_loading: false,
            artists_generation: Arc::new(AtomicU64::new(0)),
            albums_generation: Arc::new(AtomicU64::new(0)),
            songs_generation: Arc::new(AtomicU64::new(0)),
            plays_generation: Arc::new(AtomicU64::new(0)),
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

/// Stores a freshly fetched list into the player's matching buffer, bumps
/// that buffer's epoch, and marks it loaded, with no further work. The
/// `ArtistsLoaded`, `AlbumsLoaded`, and `SongsLoaded` update arms all just
/// store. The store-and-noop shape lives here once instead of in each arm.
///
/// The epoch is what makes the index-carrying selection messages safe: a row
/// rendered from the old list carries the old epoch, so if the list is
/// replaced before the press is processed, the epoch no longer matches and
/// the stale press is ignored rather than resolving its index against the
/// new list. A navigation clears a buffer without resetting its epoch, so the
/// replacing list's epoch is bumped past the cleared one's and differs from
/// it.
fn store_loaded<T>(
    buffer: &mut Vec<T>,
    epoch: &mut u64,
    loading: &mut bool,
    items: Vec<T>,
) -> Task<Message> {
    *buffer = items;
    *epoch = epoch.wrapping_add(1);
    *loading = false;
    Task::none()
}

/// Clears a browse buffer and marks it loading before a new fetch for that
/// level is issued. The `ArtistSelected` and `AlbumSelected` arms call this
/// so the view never shows — or lets the user press — the previous
/// selection's rows while the new fetch is in flight. Clearing removes the
/// stale rows and `loading` makes the view show "Loading…" instead of the
/// list's empty wording (see `views::browse_placeholder`); the buffer's epoch
/// is deliberately left alone, so the reply that replaces it still gets a
/// bumped epoch that differs from the cleared list's and a press rendered
/// from the old rows cannot match the new list.
fn clear_loaded<T>(buffer: &mut Vec<T>, loading: &mut bool) {
    buffer.clear();
    *loading = true;
}

/// The list entry a selection message names, or `None` when the press is
/// stale. A selection message carries a row's index in the list that rendered
/// it plus that list's epoch; `epoch` must still match `current_epoch` (the
/// list was not replaced after the row was rendered) and `index` must still
/// name an entry. Either failure means the press outlived its list and must be
/// a no-op, so the epoch check and the bounds check live here once instead of
/// in each of the three selection arms.
fn selected_entry<T>(items: &[T], epoch: u64, current_epoch: u64, index: usize) -> Option<&T> {
    if epoch != current_epoch {
        return None;
    }
    items.get(index)
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
    let epoch = player.songs_epoch;
    match step(&player.songs, state.current_track.as_deref(), state.repeat) {
        // `transport` answers with the stepped song's id, but the selection
        // message carries the song's index and the epoch of the list that
        // rendered the row (see `TrackSelected`), so resolve the id to its
        // position in this same buffer. This scan runs once per button press,
        // not per view refresh, and the stepped id always names a song in
        // this buffer.
        Some(track_id) => match player.songs.iter().position(|song| song.id == track_id) {
            Some(index) => Task::done(Message::TrackSelected { epoch, index }),
            None => Task::none(),
        },
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
        Message::TrackSelected { epoch, index } => {
            // The message carries the pressed row's index into `songs` plus the
            // epoch of the list that rendered it. A mismatched epoch means the
            // list was replaced after the row was rendered, so the index names
            // a different song now — ignore the stale press. An out-of-range
            // index likewise names no track and is a no-op rather than a bogus
            // play. `selected_entry` applies both checks.
            let Some(song) = selected_entry(&player.songs, epoch, player.songs_epoch, index) else {
                return Task::none();
            };
            let track_id = song.id.clone();
            // Record the played track's title before handing the id to the
            // async play: the Now Playing bar resolves its label from
            // `known_titles`, and the entry must survive a later browse to a
            // different album (which replaces `songs`). Recording once per
            // play — rather than folding every song of every browsed album
            // into the map — keeps the index proportional to songs actually
            // played and takes the fold off the browse path.
            player
                .known_titles
                .entry(track_id.clone())
                .or_insert_with(|| song.title.clone());
            // A newer selection must win even when an older play's backend
            // call finishes later: issue a generation and hand its guard to
            // `play_track`, which evaluates it under the state lock so the
            // older play cannot overwrite the newer track. The browse path
            // uses the same [`RequestGeneration`] for the same reason.
            let generation = RequestGeneration::issue(&player.plays_generation);
            let play_generation = generation.issued();
            let service = player.apple_music_service.clone();
            let id_for_report = track_id.clone();
            Task::perform(
                async move {
                    service
                        .play_track(&track_id, || generation.is_current())
                        .await
                },
                move |result| {
                    played_or_reported(result, play_generation, |err| {
                        eprintln!("{}", play_failure_report(&id_for_report, err))
                    })
                },
            )
        }
        Message::ArtistSelected { epoch, index } => {
            // As with `TrackSelected`: the row message carries the artist's
            // index and its list's epoch. A mismatched epoch is a stale press
            // from a replaced list; an out-of-range index has no artist to
            // browse. `selected_entry` rejects both.
            let Some(artist_id) =
                selected_entry(&player.artists, epoch, player.artists_epoch, index)
                    .map(|artist| artist.id.clone())
            else {
                return Task::none();
            };
            player.current_view = CurrentView::Albums;
            // Drop the previous artist's albums before the new fetch lands:
            // otherwise the Albums view renders them, and a press during the
            // fetch resolves against the wrong artist's list.
            clear_loaded(&mut player.albums, &mut player.albums_loading);
            let generation = RequestGeneration::issue(&player.albums_generation);
            fetch_into(
                &player.apple_music_service,
                format!("loading albums for artist {artist_id:?}"),
                generation,
                move |service| async move { service.get_albums_by_artist(&artist_id).await },
                Message::AlbumsLoaded,
            )
        }
        Message::AlbumSelected { epoch, index } => {
            // The album twin of the artist arm above; `selected_entry`
            // rejects the same stale-epoch and out-of-range presses.
            let Some(album_id) = selected_entry(&player.albums, epoch, player.albums_epoch, index)
                .map(|album| album.id.clone())
            else {
                return Task::none();
            };
            player.current_view = CurrentView::Songs;
            // The songs twin of the artist arm above: clear the previous
            // album's songs so they cannot be shown or pressed while the new
            // album's fetch is in flight.
            clear_loaded(&mut player.songs, &mut player.songs_loading);
            let generation = RequestGeneration::issue(&player.songs_generation);
            fetch_into(
                &player.apple_music_service,
                format!("loading songs from album {album_id:?}"),
                generation,
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
        Message::LoadArtists => {
            let generation = RequestGeneration::issue(&player.artists_generation);
            fetch_into(
                &player.apple_music_service,
                "loading favorite artists".to_string(),
                generation,
                |service| async move { service.get_favorite_artists().await },
                Message::ArtistsLoaded,
            )
        }
        Message::ArtistsLoaded(artists) => store_loaded(
            &mut player.artists,
            &mut player.artists_epoch,
            &mut player.artists_loading,
            artists,
        ),
        Message::AlbumsLoaded(albums) => store_loaded(
            &mut player.albums,
            &mut player.albums_epoch,
            &mut player.albums_loading,
            albums,
        ),
        Message::SongsLoaded(songs) => store_loaded(
            &mut player.songs,
            &mut player.songs_epoch,
            &mut player.songs_loading,
            songs,
        ),
        Message::TrackPlayed { generation } => {
            // Only the current track's title is ever read (see
            // [`Self::known_titles`]), so the latest play's completion drops
            // every other entry the selections above accumulated. A completion
            // whose play a newer selection superseded is skipped: its
            // `current_track` names the older committed track, but the newer
            // selection's title is still pending, and pruning to the older
            // track would delete it. When this completion is the latest, no
            // newer selection is pending, so reducing to `current_track` is
            // safe — and when nothing committed, clearing the map keeps it
            // bounded rather than leaving the failed selection's entry.
            if generation == player.plays_generation.load(Ordering::SeqCst) {
                let state = player.state.blocking_lock();
                match state.current_track.as_deref() {
                    Some(current) => player.known_titles.retain(|id, _| id == current),
                    None => player.known_titles.clear(),
                }
            }
            Task::none()
        }
        Message::Ignored => Task::none(),
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
        CurrentView::Artists => views::view_artists(
            &player.artists,
            player.artists_epoch,
            player.artists_loading,
        ),
        CurrentView::Albums => {
            views::view_albums(&player.albums, player.albums_epoch, player.albums_loading)
        }
        // A second short lock (like the label block above) hands the current
        // track's id to the Songs view so it can mark the playing row. The
        // borrowed id is compared inside `song_row` and the guard drops at
        // the end of this arm, so no per-frame `Option<String>` clone is
        // needed.
        CurrentView::Songs => {
            let state = player.state.blocking_lock();
            views::view_songs(
                &player.songs,
                player.songs_epoch,
                player.songs_loading,
                state.current_track.as_deref(),
            )
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
